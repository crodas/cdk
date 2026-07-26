//! Cross-process pub/sub bus backed by PostgreSQL `LISTEN`/`NOTIFY`.
//!
//! The mint's NUT-17 notifications are in-process by default: an event
//! published on one instance only reaches subscribers connected to that same
//! instance. [`PostgresBus`] implements [`cdk_common::pub_sub::Bus`] so several
//! mint instances sharing a Postgres database also share notifications.
//!
//! On publish the event is delivered to local subscribers immediately and, in
//! the background, sent to peers with `pg_notify`. Each instance keeps a
//! dedicated connection in `LISTEN`, deserializes inbound events, and injects
//! them into its own local fan-out. Messages carry the origin instance id so a
//! bus skips the copy of its own event that Postgres echoes back.
//!
//! Wire format is JSON. `NOTIFY` payloads are capped by Postgres at 8000 bytes;
//! oversized events are delivered locally but not forwarded (a warning is
//! logged). Mint events are small; this only concerns unusually large melts.
//!
//! Inbound notifications flow through a bounded queue: if a peer floods faster
//! than they can be dispatched, the excess is dropped (those subscribers
//! recover on their next backfill) rather than growing memory without bound.
//!
//! The background connection and its tasks are tied to the bus lifetime: when
//! the built [`PostgresBus`] (or an unbuilt [`PostgresBusConnector`]) is
//! dropped, the driver and dispatcher stop and the connection is released.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use cdk_common::database::Error;
use cdk_common::pub_sub::{Bus, LocalDelivery, Spec};
use cdk_sql_common::pool::DatabaseConfig;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_postgres::{AsyncMessage, Client, Connection};

use crate::connection::{connect_and_drive, DriveConnection};
use crate::PgConfig;

/// Backoff before the first reconnect attempt.
const INITIAL_BACKOFF: Duration = Duration::from_millis(500);

/// Upper bound for the reconnect backoff.
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Maximum `NOTIFY` payload Postgres accepts is 8000 bytes; stay under it.
const MAX_NOTIFY_PAYLOAD: usize = 7999;

/// Bound on buffered inbound notifications awaiting dispatch. Mirrors the local
/// pub/sub channel size; excess is dropped rather than buffered without bound.
const INBOUND_CHANNEL_SIZE: usize = 10_000;

/// A live Postgres client shared between the publisher and the connection
/// driver. `None` while disconnected; the driver swaps it on (re)connect.
type SharedClient = Arc<RwLock<Option<Arc<Client>>>>;

/// Envelope written to the wire. Borrows the event to avoid a clone.
#[derive(Serialize)]
struct OutEnvelope<'a, E> {
    origin: &'a str,
    event: &'a E,
}

/// Envelope read from the wire.
#[derive(Deserialize)]
struct InEnvelope<E> {
    origin: String,
    event: E,
}

/// Outcome of classifying an inbound wire payload.
enum Inbound<E> {
    /// A peer event to deliver locally.
    Deliver(E),
    /// This instance's own event echoed back by Postgres; ignore it.
    SelfEcho,
    /// Undecodable payload; ignore it.
    Malformed,
}

/// Decode an inbound payload and decide what to do with it.
///
/// Pure, so the self-echo and malformed-payload handling are unit-testable
/// without a live connection.
fn classify<E: DeserializeOwned>(payload: &str, our_origin: &str) -> Inbound<E> {
    match serde_json::from_str::<InEnvelope<E>>(payload) {
        Ok(envelope) if envelope.origin == our_origin => Inbound::SelfEcho,
        Ok(envelope) => Inbound::Deliver(envelope.event),
        Err(_) => Inbound::Malformed,
    }
}

/// A connected Postgres bus, not yet bound to a subscriber set.
///
/// [`PostgresBusConnector::connect`] establishes the connection and starts
/// listening; [`PostgresBusConnector::build`] then attaches it to a
/// [`Pubsub`](cdk_common::pub_sub::Pubsub) via its [`LocalDelivery`] handle.
#[allow(missing_debug_implementations)]
pub struct PostgresBusConnector {
    inbound: mpsc::Receiver<String>,
    client: SharedClient,
    channel: Arc<str>,
    origin: Arc<str>,
    /// Dropping this sender signals the background tasks to stop. Held here
    /// until `build` moves it into the [`PostgresBus`], so a connector dropped
    /// before `build` also shuts the connection down.
    shutdown: watch::Sender<()>,
}

impl PostgresBusConnector {
    /// Connect to Postgres and start listening on `channel`.
    ///
    /// Returns once the first connection is established and the `LISTEN` is in
    /// place, so a failure to reach Postgres surfaces here rather than silently
    /// later. After that the connection is maintained in the background and
    /// reconnects with backoff until the bus is dropped.
    ///
    /// `channel` must be a valid Postgres identifier (letters, digits and
    /// underscores, not starting with a digit, at most 63 bytes) because it is
    /// interpolated into the `LISTEN` statement, which cannot be parameterized.
    pub async fn connect(config: PgConfig, channel: &str) -> Result<Self, Error> {
        let channel = validate_channel(channel)?;
        let connect_timeout = config.default_timeout();

        let (inbound_tx, inbound) = mpsc::channel(INBOUND_CHANNEL_SIZE);
        let (ready_tx, ready_rx) = oneshot::channel();
        let (shutdown, driver_shutdown) = watch::channel(());
        let client: SharedClient = Arc::new(RwLock::new(None));

        let driver_channel = channel.clone();
        let driver_client = client.clone();

        cdk_common::task::spawn(run_driver(
            config,
            driver_channel,
            inbound_tx,
            driver_client,
            ready_tx,
            driver_shutdown,
        ));

        match timeout(connect_timeout, ready_rx).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(err))) => return Err(Error::Internal(err)),
            Ok(Err(_)) => return Err(Error::Internal("postgres bus driver stopped".to_string())),
            Err(_) => {
                return Err(Error::Internal(
                    "timeout connecting postgres bus".to_string(),
                ))
            }
        }

        Ok(Self {
            inbound,
            client,
            channel: Arc::from(channel),
            origin: Arc::from(new_origin_id().as_str()),
            shutdown,
        })
    }

    /// Attach the bus to a subscriber set and return it as a [`Bus`].
    ///
    /// Spawns the inbound dispatcher that delivers peer events through `local`,
    /// skipping this instance's own echoed events.
    pub fn build<S>(self, local: LocalDelivery<S>) -> Arc<dyn Bus<S>>
    where
        S: Spec + 'static,
    {
        let PostgresBusConnector {
            mut inbound,
            client,
            channel,
            origin,
            shutdown,
        } = self;

        let dispatch_local = local.clone();
        let dispatch_origin = origin.clone();
        let mut dispatch_shutdown = shutdown.subscribe();
        cdk_common::task::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = dispatch_shutdown.changed() => break,
                    payload = inbound.recv() => match payload {
                        Some(payload) => match classify::<S::Event>(&payload, &dispatch_origin) {
                            Inbound::Deliver(event) => dispatch_local.deliver(event),
                            Inbound::SelfEcho => {}
                            Inbound::Malformed => {
                                tracing::warn!("postgres bus: dropping malformed payload");
                            }
                        },
                        None => break,
                    },
                }
            }
        });

        Arc::new(PostgresBus {
            local,
            client,
            channel,
            origin,
            _shutdown: shutdown,
        })
    }
}

/// Postgres-backed [`Bus`]. See the module docs.
#[allow(missing_debug_implementations)]
pub struct PostgresBus<S>
where
    S: Spec + 'static,
{
    local: LocalDelivery<S>,
    client: SharedClient,
    channel: Arc<str>,
    origin: Arc<str>,
    /// Dropped when the bus is dropped, which stops the background tasks.
    _shutdown: watch::Sender<()>,
}

impl<S> Bus<S> for PostgresBus<S>
where
    S: Spec + 'static,
{
    fn publish(&self, event: S::Event) {
        // Serialize before delivering locally, since delivery consumes the event.
        let payload = match serde_json::to_string(&OutEnvelope {
            origin: self.origin.as_ref(),
            event: &event,
        }) {
            Ok(payload) => Some(payload),
            Err(err) => {
                tracing::warn!("postgres bus: failed to serialize event: {err}");
                None
            }
        };

        self.local.deliver(event);

        let Some(payload) = payload else { return };
        if payload.len() > MAX_NOTIFY_PAYLOAD {
            tracing::warn!(
                "postgres bus: event of {} bytes exceeds NOTIFY limit, not forwarded to peers",
                payload.len()
            );
            return;
        }

        let client = self.client.clone();
        let channel = self.channel.clone();
        cdk_common::task::spawn(async move {
            let client = client.read().ok().and_then(|guard| guard.clone());
            match client {
                Some(client) => {
                    let channel: &str = &channel;
                    if let Err(err) = client
                        .execute("SELECT pg_notify($1, $2)", &[&channel, &payload])
                        .await
                    {
                        tracing::warn!("postgres bus: pg_notify failed: {err}");
                    }
                }
                None => {
                    tracing::warn!("postgres bus: disconnected, event not forwarded to peers");
                }
            }
        });
    }
}

/// Drives a bus connection by forwarding its notification payloads. Bridges the
/// shared [`connect_and_drive`] helper to [`drive_connection`].
struct NotifyDrive {
    inbound_tx: mpsc::Sender<String>,
}

impl DriveConnection for NotifyDrive {
    fn drive<S, T>(self, connection: Connection<S, T>) -> Pin<Box<dyn Future<Output = ()> + Send>>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        Box::pin(drive_connection(connection, self.inbound_tx))
    }
}

/// Maintain a `LISTEN` connection: connect, listen, forward notifications, and
/// reconnect with capped backoff when the connection drops. Returns when
/// `shutdown` fires (the bus was dropped).
async fn run_driver(
    config: PgConfig,
    channel: String,
    inbound_tx: mpsc::Sender<String>,
    client_slot: SharedClient,
    ready_tx: oneshot::Sender<Result<(), String>>,
    mut shutdown: watch::Receiver<()>,
) {
    let listen_sql = format!("LISTEN \"{channel}\"");
    let mut ready_tx = Some(ready_tx);
    let mut backoff = INITIAL_BACKOFF;

    loop {
        // `connect_and_drive` spawns the connection driver, so the connection is
        // already being polled and requests like LISTEN make progress.
        let drive = NotifyDrive {
            inbound_tx: inbound_tx.clone(),
        };
        let outcome = tokio::select! {
            biased;
            _ = shutdown.changed() => break,
            res = connect_and_drive(&config, drive) => res,
        };

        match outcome {
            Ok((client, driver)) => {
                let session = serve_connection(
                    Arc::new(client),
                    driver,
                    &listen_sql,
                    &client_slot,
                    &mut ready_tx,
                    &mut shutdown,
                )
                .await;
                match session {
                    Session::Shutdown => break,
                    // A listening session ran and the connection later ended.
                    // The endpoint was healthy, so retry from the base backoff.
                    Session::Disconnected => backoff = INITIAL_BACKOFF,
                    // Setup failed on this connection. Keep growing the backoff
                    // so repeated LISTEN failures do not hammer the database.
                    Session::ListenFailed => {}
                }
            }
            Err(err) => {
                if let Some(tx) = ready_tx.take() {
                    let _ = tx.send(Err(err.to_string()));
                }
                tracing::warn!("postgres bus: connect failed: {err}");
            }
        }

        tokio::select! {
            biased;
            _ = shutdown.changed() => break,
            _ = tokio::time::sleep(backoff) => {}
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

/// Outcome of serving a single connection in [`run_driver`].
enum Session {
    /// The bus was dropped; stop the driver loop.
    Shutdown,
    /// A listening session ran and the connection later ended; reconnect.
    Disconnected,
    /// The connection came up but `LISTEN` never took effect; reconnect.
    ListenFailed,
}

/// Serve one connection: install the client, issue `LISTEN`, then run until the
/// connection ends or shutdown fires.
///
/// A SQL-level `LISTEN` failure does not necessarily close an otherwise healthy
/// Postgres connection. Treat it as a failed setup for this connection: clear
/// the installed client, abort the driver, and return [`Session::ListenFailed`]
/// so the caller reconnects. Serving such a connection would leave the bus
/// connected and still publishing outbound `pg_notify` while never receiving
/// inbound peer notifications until the connection dropped for some unrelated
/// reason.
async fn serve_connection(
    client: Arc<Client>,
    mut driver: JoinHandle<()>,
    listen_sql: &str,
    client_slot: &SharedClient,
    ready_tx: &mut Option<oneshot::Sender<Result<(), String>>>,
    shutdown: &mut watch::Receiver<()>,
) -> Session {
    // LISTEN needs an installed client, so install it first and uninstall it on
    // failure below, so publishers never use a connection that is not listening.
    if let Ok(mut slot) = client_slot.write() {
        *slot = Some(client.clone());
    }

    let listen_result = client.batch_execute(listen_sql).await;
    if let Some(tx) = ready_tx.take() {
        let _ = tx.send(
            listen_result
                .as_ref()
                .map(|_| ())
                .map_err(|e| e.to_string()),
        );
    }

    if let Err(err) = listen_result {
        tracing::warn!("postgres bus: LISTEN failed: {err}, dropping connection to reconnect");
        driver.abort();
        clear_client(client_slot);
        return Session::ListenFailed;
    }

    tokio::select! {
        biased;
        _ = shutdown.changed() => {
            driver.abort();
            clear_client(client_slot);
            Session::Shutdown
        }
        // Runs until the connection ends.
        _ = &mut driver => {
            clear_client(client_slot);
            tracing::warn!("postgres bus: connection lost, reconnecting");
            Session::Disconnected
        }
    }
}

/// Uninstall the shared client so publishers stop using a dead or deaf
/// connection.
fn clear_client(client_slot: &SharedClient) {
    if let Ok(mut slot) = client_slot.write() {
        *slot = None;
    }
}

/// Poll a Postgres connection, forwarding notification payloads. Returns when
/// the connection closes or errors. When the inbound queue is full the
/// notification is dropped rather than stalling the connection.
async fn drive_connection<S, T>(connection: Connection<S, T>, inbound_tx: mpsc::Sender<String>)
where
    S: AsyncRead + AsyncWrite + Unpin,
    T: AsyncRead + AsyncWrite + Unpin,
{
    tokio::pin!(connection);
    loop {
        match std::future::poll_fn(|cx| connection.as_mut().poll_message(cx)).await {
            Some(Ok(AsyncMessage::Notification(notification))) => {
                match inbound_tx.try_send(notification.payload().to_string()) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        tracing::warn!("postgres bus: inbound queue full, dropping notification");
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => return,
                }
            }
            Some(Ok(_)) => {}
            Some(Err(err)) => {
                tracing::warn!("postgres bus: connection error: {err}");
                return;
            }
            None => return,
        }
    }
}

/// Generate a per-instance origin id used to skip self-echoed events.
fn new_origin_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Validate that `channel` is a safe Postgres identifier for interpolation into
/// `LISTEN`.
fn validate_channel(channel: &str) -> Result<String, Error> {
    let valid = !channel.is_empty()
        && channel.len() <= 63
        && channel
            .bytes()
            .next()
            .map(|b| b.is_ascii_alphabetic() || b == b'_')
            .unwrap_or(false)
        && channel
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_');

    if valid {
        Ok(channel.to_string())
    } else {
        Err(Error::Internal(format!(
            "invalid postgres bus channel name: {channel:?}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use cdk_common::bus_test;
    use cdk_common::pub_sub::test::{CustomPubSub, Message, SubscriptionReq};
    use cdk_common::pub_sub::Pubsub;

    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Ev {
        foo: u64,
    }

    /// Connection string for the test database, matching the generic database
    /// tests: `CDK_MINTD_DATABASE_URL`, then `PG_DB_URL`, then a local default.
    fn test_db_url() -> String {
        std::env::var("CDK_MINTD_DATABASE_URL")
            .or_else(|_| std::env::var("PG_DB_URL"))
            .unwrap_or_else(|_| {
                "host=localhost user=cdk_user password=cdk_password dbname=cdk_mint port=5432"
                    .to_owned()
            })
    }

    /// Derive a valid, unique `LISTEN`/`NOTIFY` channel from a test id.
    ///
    /// The id from [`bus_test!`] begins with `test_<nanos>_`, so truncating to
    /// the 63-byte Postgres identifier limit keeps the unique `<nanos>` prefix.
    /// A unique channel per test means one test never receives another's
    /// `NOTIFY`.
    fn pg_channel(test_id: &str) -> String {
        test_id.chars().take(63).collect()
    }

    /// Connect a Postgres bus on `channel` and wrap it in a `Pubsub`.
    async fn pg_pubsub(channel: &str) -> Pubsub<CustomPubSub> {
        let connector =
            PostgresBusConnector::connect(PgConfig::from(test_db_url().as_str()), channel)
                .await
                .expect("connect postgres bus");
        Pubsub::new_with_bus(CustomPubSub::new_instance(()), move |local| {
            connector.build(local)
        })
    }

    /// Factory for the generic bus suite: one Postgres-backed node per test.
    async fn provide_pg_bus(test_id: String) -> Pubsub<CustomPubSub> {
        pg_pubsub(&pg_channel(&test_id)).await
    }

    bus_test!(provide_pg_bus);

    /// Two mint instances sharing one Postgres database and channel: an event
    /// published on instance A is delivered through `LISTEN`/`NOTIFY` to a
    /// subscriber on instance B, and A's own subscriber receives it exactly
    /// once (the echo Postgres sends back is dropped as a self-echo).
    #[tokio::test]
    async fn event_crosses_instances_through_postgres() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let channel = pg_channel(&format!("test_{nanos}_cross_node"));

        let instance_a = pg_pubsub(&channel).await;
        let instance_b = pg_pubsub(&channel).await;

        let mut sub_a = instance_a.subscribe(SubscriptionReq::Foo(2)).unwrap();
        let mut sub_b = instance_b.subscribe(SubscriptionReq::Foo(2)).unwrap();

        instance_a.publish(Message { foo: 2, bar: 7 });

        // Delivered to the other instance through Postgres.
        let received_b = timeout(Duration::from_secs(5), sub_b.recv())
            .await
            .expect("event delivered to instance B before timeout");
        assert_eq!(received_b.map(|m| m.bar), Some(7));

        // Delivered locally on the publishing instance as well.
        let received_a = timeout(Duration::from_secs(5), sub_a.recv())
            .await
            .expect("event delivered to instance A before timeout");
        assert_eq!(received_a.map(|m| m.bar), Some(7));

        // The self-echo Postgres sends back is dropped: A sees the event once.
        // Wait long enough for a round-trip through the database.
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(sub_a.try_recv().is_none());
    }

    /// A `LISTEN` that fails on an otherwise healthy connection must not leave
    /// the bus connected-but-deaf: `serve_connection` has to abort the driver,
    /// clear the installed client, and return `ListenFailed` so the driver loop
    /// reconnects, rather than parking on the still-open connection.
    ///
    /// The failure is forced by putting the session in an aborted transaction:
    /// the socket stays open, but every later statement (including `LISTEN`)
    /// errors until the transaction ends.
    #[tokio::test]
    async fn listen_failure_on_live_connection_reconnects() {
        let config = PgConfig::from(test_db_url().as_str());

        // A real connection whose driver keeps running, so the connection stays
        // open. `_inbound` keeps the notification channel alive.
        let (inbound_tx, _inbound) = mpsc::channel(16);
        let (client, driver) = connect_and_drive(&config, NotifyDrive { inbound_tx })
            .await
            .expect("connect");
        let client = Arc::new(client);

        // Abort the transaction: LISTEN will now fail while the socket is up.
        let _ = client.batch_execute("BEGIN; SELECT 1 / 0;").await;

        let client_slot: SharedClient = Arc::new(RwLock::new(None));
        let (_shutdown_tx, mut shutdown_rx) = watch::channel(());
        let mut ready_tx = None;

        let session = timeout(
            Duration::from_secs(5),
            serve_connection(
                client,
                driver,
                "LISTEN \"cdk_bus_listen_failure_test\"",
                &client_slot,
                &mut ready_tx,
                &mut shutdown_rx,
            ),
        )
        .await
        .expect("serve_connection must not hang when LISTEN fails on a live connection");

        assert!(matches!(session, Session::ListenFailed));
        // The client is uninstalled, so publishers stop using the deaf connection.
        assert!(client_slot.read().unwrap().is_none());
    }

    #[test]
    fn accepts_valid_channel_names() {
        assert!(validate_channel("cdk_mint_events").is_ok());
        assert!(validate_channel("_private").is_ok());
        assert!(validate_channel("a1").is_ok());
    }

    #[test]
    fn rejects_invalid_channel_names() {
        assert!(validate_channel("").is_err());
        assert!(validate_channel("1leading_digit").is_err());
        assert!(validate_channel("has space").is_err());
        assert!(validate_channel("drop;table").is_err());
        assert!(validate_channel(&"x".repeat(64)).is_err());
    }

    #[test]
    fn envelope_roundtrips_as_json() {
        let json = serde_json::to_string(&OutEnvelope {
            origin: "abc",
            event: &Ev { foo: 7 },
        })
        .expect("serialize");

        let parsed: InEnvelope<Ev> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.origin, "abc");
        assert_eq!(parsed.event, Ev { foo: 7 });
    }

    #[test]
    fn classify_delivers_peer_events() {
        let json = serde_json::to_string(&OutEnvelope {
            origin: "peer",
            event: &Ev { foo: 9 },
        })
        .expect("serialize");

        match classify::<Ev>(&json, "self") {
            Inbound::Deliver(event) => assert_eq!(event, Ev { foo: 9 }),
            _ => panic!("expected Deliver"),
        }
    }

    #[test]
    fn classify_skips_self_echo() {
        let json = serde_json::to_string(&OutEnvelope {
            origin: "self",
            event: &Ev { foo: 1 },
        })
        .expect("serialize");

        assert!(matches!(classify::<Ev>(&json, "self"), Inbound::SelfEcho));
    }

    #[test]
    fn classify_rejects_malformed_payload() {
        assert!(matches!(
            classify::<Ev>("not json", "self"),
            Inbound::Malformed
        ));
    }
}
