//! Supervision for long-lived streaming subscriptions.
//!
//! [`supervise_stream`] owns the reconnect, backoff, and shutdown loop that
//! every consumer of a long-lived stream (a gRPC server-stream, a payment
//! backend's event stream) would otherwise hand-roll. It is transport-agnostic:
//! the caller supplies a `connect` closure that opens a fresh stream and a
//! handler for each item.

use std::fmt;

use futures::{pin_mut, Stream, StreamExt};

/// Capped exponential backoff for [`supervise_stream`] reconnect attempts.
#[derive(Debug, Clone, Copy)]
pub struct BackoffPolicy {
    /// Delay before the first reconnect, and the floor the delay resets to
    /// once a message has been delivered.
    pub initial: std::time::Duration,
    /// Upper bound the delay is capped at while backing off.
    pub max: std::time::Duration,
}

/// Keep a streaming subscription alive across reconnects.
///
/// `connect` opens a fresh stream of items. Every item it yields is handed to
/// `on_message`, which is awaited to completion (item processing is sequential,
/// and a slow handler holds the loop, so a handler that must not block the read
/// should offload its work). When the stream ends (a clean close or a per-item
/// error) or a connect attempt fails, the supervisor waits out a capped
/// exponential backoff and reconnects. The backoff resets once a message is
/// delivered, so a healthy connection that later drops reconnects quickly while
/// an endpoint that keeps failing is not hammered.
///
/// `shutdown` stops the supervisor promptly whenever it is waiting: to connect,
/// for the next item, or during a backoff. It does not interrupt an in-flight
/// `on_message`.
///
/// This owns only the reconnect, backoff, and shutdown machinery. Opening the
/// stream, decoding items, and acting on them stay with the caller. Any
/// teardown that must run when the subscription stops (cancelling a token, say)
/// belongs on the line after this call returns.
pub async fn supervise_stream<T, E1, E2, C, CFut, St, F, Fut, S>(
    policy: BackoffPolicy,
    shutdown: S,
    mut connect: C,
    mut on_message: F,
) where
    C: FnMut() -> CFut,
    CFut: std::future::Future<Output = Result<St, E1>>,
    St: Stream<Item = Result<T, E2>>,
    E1: fmt::Display,
    E2: fmt::Display,
    F: FnMut(T) -> Fut,
    Fut: std::future::Future<Output = ()>,
    S: std::future::Future<Output = ()>,
{
    tokio::pin!(shutdown);
    let mut backoff = policy.initial;

    loop {
        let connect_result = tokio::select! {
            biased;
            _ = &mut shutdown => return,
            result = connect() => result,
        };

        match connect_result {
            Ok(stream) => {
                pin_mut!(stream);
                loop {
                    let next = tokio::select! {
                        biased;
                        _ = &mut shutdown => return,
                        next = stream.next() => next,
                    };

                    match next {
                        Some(Ok(item)) => {
                            // A delivered message proves the connection is
                            // productive, so a later drop reconnects quickly.
                            backoff = policy.initial;
                            on_message(item).await;
                        }
                        Some(Err(err)) => {
                            tracing::warn!("Stream error: {err}");
                            break;
                        }
                        None => {
                            tracing::debug!("Stream closed by the server");
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                tracing::warn!("Could not open stream: {err}");
            }
        }

        // Wait before reconnecting so a persistently failing endpoint is not
        // hammered. Shutdown during the wait ends the loop immediately.
        tokio::select! {
            biased;
            _ = &mut shutdown => return,
            _ = tokio::time::sleep(backoff) => {}
        }
        backoff = (backoff * 2).min(policy.max);
    }
}
