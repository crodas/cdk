//! gRPC helpers: protocol version checking and server-stream supervision.

use tonic::metadata::AsciiMetadataValue;
use tonic::service::Interceptor;
use tonic::{Request, Status};

/// Capped exponential backoff for [`supervise_stream`] reconnect attempts.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy)]
pub struct BackoffPolicy {
    /// Delay before the first reconnect, and the floor the delay resets to
    /// after a connection succeeds.
    pub initial: std::time::Duration,
    /// Upper bound the delay is capped at while backing off.
    pub max: std::time::Duration,
}

/// Keep a gRPC server-stream subscription alive.
///
/// `connect` opens a fresh [`tonic::Streaming`]. Every message it yields is
/// handed to `on_message`. When the stream closes cleanly or errors, the
/// supervisor reconnects with capped exponential backoff. `shutdown` stops the
/// supervisor promptly, even while it is blocked waiting for the next message,
/// and returning ends the loop.
///
/// This owns only the reconnect, backoff, and shutdown machinery. Decoding a
/// message and deciding what to do with it stays with the caller in
/// `on_message`. The callback is synchronous on purpose: a consumer that must
/// await (a database write, say) should forward each message into a channel it
/// drains elsewhere, so processing latency never stalls the transport read.
#[cfg(not(target_arch = "wasm32"))]
pub async fn supervise_stream<T, C, CFut, F, S>(
    policy: BackoffPolicy,
    shutdown: S,
    mut connect: C,
    mut on_message: F,
) where
    C: FnMut() -> CFut,
    CFut: std::future::Future<Output = Result<tonic::Streaming<T>, Status>>,
    F: FnMut(T),
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
            Ok(mut stream) => {
                // A live connection: reset the backoff so the next drop retries
                // quickly, and drain until the stream ends or shutdown fires.
                backoff = policy.initial;
                loop {
                    let message = tokio::select! {
                        biased;
                        _ = &mut shutdown => return,
                        message = stream.message() => message,
                    };

                    match message {
                        Ok(Some(item)) => on_message(item),
                        Ok(None) => {
                            tracing::debug!("Server closed the stream");
                            break;
                        }
                        Err(status) => {
                            tracing::warn!("Stream error: {status}");
                            break;
                        }
                    }
                }
            }
            Err(status) => {
                tracing::warn!("Could not subscribe to stream: {status}");
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

/// Header name for protocol version
pub const VERSION_HEADER: &str = "x-cdk-protocol-version";
/// Header for version of the signatory protofile
pub const VERSION_SIGNATORY_HEADER: &str = "x-signatory-schema-version";

/// A client-side interceptor that injects a protocol version header into every
/// outgoing gRPC request.
///
/// # Panics
/// [`VersionInterceptor::new`] panics if the version string is not a valid gRPC
/// metadata ASCII value.
#[derive(Debug, Clone)]
pub struct VersionInterceptor {
    header: &'static str,
    value: AsciiMetadataValue,
}

impl VersionInterceptor {
    /// Create a new `VersionInterceptor`.
    ///
    /// # Panics
    /// Panics if `version` is not a valid gRPC metadata ASCII value.
    pub fn new(header: &'static str, version: impl AsRef<str>) -> Self {
        Self {
            header,
            value: version.as_ref().parse().expect("Invalid protocol version"),
        }
    }
}

impl Interceptor for VersionInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request
            .metadata_mut()
            .insert(self.header, self.value.clone());
        Ok(request)
    }
}

/// Creates a server-side interceptor that validates a specific protocol version on incoming requests
pub fn create_version_check_interceptor(
    header: &'static str,
    expected_version: &'static str,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    move |request: Request<()>| match request.metadata().get(header) {
        Some(version) => {
            let version = version
                .to_str()
                .map_err(|_| Status::invalid_argument("Invalid protocol version header"))?;
            if version != expected_version {
                return Err(Status::failed_precondition(format!(
                    "Protocol version mismatch: server={}, client={}",
                    expected_version, version
                )));
            }
            Ok(request)
        }
        None => Err(Status::failed_precondition(
            "Missing x-cdk-protocol-version header",
        )),
    }
}
