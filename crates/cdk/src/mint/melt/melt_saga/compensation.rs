//! Compensation actions for the melt saga pattern.
//!
//! When a saga step fails, compensating actions are executed in reverse order (LIFO)
//! to undo all completed steps and restore the database to its pre-saga state.

use async_trait::async_trait;
use cdk_common::saga::CompensatingAction;
use cdk_common::{Error, PublicKey, QuoteId};
use tracing::instrument;
use uuid::Uuid;

use crate::mint::saga::MintSagaContext;

/// Compensation action to remove melt setup and reset quote state.
///
/// This compensation is used when payment fails or finalization fails after
/// the setup transaction has committed. It removes:
/// - Input proofs (identified by input_ys)
/// - Output blinded messages (identified by blinded_secrets)
/// - Melt request tracking record
/// - Saga state record
///
///   And resets:
/// - Quote state from Pending back to Unpaid
///
/// This restores the database to its pre-melt state, allowing the user to retry.
pub struct RemoveMeltSetup {
    /// Y values (public keys) from the input proofs
    pub input_ys: Vec<PublicKey>,
    /// Blinded secrets (B values) from the change output blinded messages
    pub blinded_secrets: Vec<PublicKey>,
    /// Quote ID to reset state
    pub quote_id: QuoteId,
    /// Operation ID (saga ID) to delete
    pub operation_id: Uuid,
}

#[async_trait]
impl<M: Send + Sync> CompensatingAction<MintSagaContext<M>> for RemoveMeltSetup {
    #[instrument(skip_all)]
    async fn execute(&self, ctx: &MintSagaContext<M>) -> Result<(), Error> {
        super::super::shared::rollback_melt_quote(
            &ctx.db,
            &ctx.pubsub,
            &self.quote_id,
            &self.input_ys,
            &self.blinded_secrets,
            &self.operation_id,
        )
        .await
    }

    fn name(&self) -> &'static str {
        "RemoveMeltSetup"
    }
}
