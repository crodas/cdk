use async_trait::async_trait;
use cdk_common::saga::CompensatingAction;
use cdk_common::{Error, PublicKey};
use tracing::instrument;
use uuid::Uuid;

use crate::mint::saga::MintSagaContext;

/// Compensation action to remove swap setup (both proofs and blinded messages).
///
/// This compensation is used when blind signing fails or finalization fails after
/// the setup transaction has committed. It removes:
/// - Output blinded messages (identified by blinded_secrets)
/// - Input proofs (identified by input_ys)
/// - Saga state record
///
/// This restores the database to its pre-swap state.
pub struct RemoveSwapSetup {
    /// Blinded secrets (B values) from the output blinded messages
    pub blinded_secrets: Vec<PublicKey>,
    /// Y values (public keys) from the input proofs
    pub input_ys: Vec<PublicKey>,
    /// Operation ID (saga ID) to delete
    pub operation_id: Uuid,
}

#[async_trait]
impl<M: Send + Sync> CompensatingAction<MintSagaContext<M>> for RemoveSwapSetup {
    #[instrument(skip_all)]
    async fn execute(&self, ctx: &MintSagaContext<M>) -> Result<(), Error> {
        super::super::shared::rollback_swap_setup(
            &ctx.db,
            &ctx.pubsub,
            &self.blinded_secrets,
            &self.input_ys,
            &self.operation_id,
        )
        .await
    }

    fn name(&self) -> &'static str {
        "RemoveSwapSetup"
    }
}
