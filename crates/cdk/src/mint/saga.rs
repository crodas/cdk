//! Saga context for mint operations.

use std::fmt;
use std::sync::Arc;

use cdk_common::database::DynMintDatabase;
use cdk_common::saga::SagaContext;
#[cfg(feature = "prometheus")]
use cdk_prometheus::MintMetricGuard;

use crate::mint::subscription::PubSubManager;

/// Handles the mint's saga steps and compensating actions operate against.
///
/// Generic over how the mint is held: the swap saga runs inside the request and
/// borrows it, while the melt saga is moved into a spawned task and needs an
/// owned `Arc`.
pub struct MintSagaContext<M> {
    /// The mint the saga belongs to.
    pub mint: M,
    /// Database the saga's transactions run against.
    pub db: DynMintDatabase,
    /// Publishes proof and quote state changes, including those a rollback undoes.
    pub pubsub: Arc<PubSubManager>,
    /// Taken and recorded once the operation's outcome is known.
    #[cfg(feature = "prometheus")]
    pub metrics: Option<MintMetricGuard>,
}

impl<M: Send + Sync> SagaContext for MintSagaContext<M> {}

/// The mint, database and pubsub handles are not `Debug`, so there is nothing
/// useful to print.
impl<M> fmt::Debug for MintSagaContext<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MintSagaContext")
    }
}

impl<M> MintSagaContext<M> {
    /// Record the operation as failed, if metrics are enabled.
    ///
    /// The guard is taken so a later drop does not record the operation twice.
    pub fn record_failure(&mut self) {
        #[cfg(feature = "prometheus")]
        if let Some(metrics) = self.metrics.take() {
            metrics.record(false);
        }
    }

    /// Build a context for a saga that has not started yet.
    pub fn new(
        mint: M,
        db: DynMintDatabase,
        pubsub: Arc<PubSubManager>,
        #[cfg(feature = "prometheus")] metrics: Option<MintMetricGuard>,
    ) -> Self {
        Self {
            mint,
            db,
            pubsub,
            #[cfg(feature = "prometheus")]
            metrics,
        }
    }
}
