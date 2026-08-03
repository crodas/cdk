//! Swap rollback shared by the saga's compensation and by startup recovery.

use cdk_common::database::DynMintDatabase;
use cdk_common::{Error, PublicKey, State};
use tracing::instrument;
use uuid::Uuid;

use crate::mint::subscription::PubSubManager;

/// Undoes a committed swap setup, restoring the database to its pre-swap state.
///
/// Within a single database transaction:
/// 1. Removes output blinded messages
/// 2. Removes input proofs
/// 3. Deletes the saga record
///
/// Used by both [`super::swap_saga::compensation::RemoveSwapSetup`] when a swap
/// fails in process and by `start_up_check::recover_from_incomplete_sagas` when
/// recovering after a crash, so the two paths cannot drift.
#[instrument(skip_all)]
pub async fn rollback_swap_setup(
    db: &DynMintDatabase,
    pubsub: &PubSubManager,
    blinded_secrets: &[PublicKey],
    input_ys: &[PublicKey],
    operation_id: &Uuid,
) -> Result<(), Error> {
    if blinded_secrets.is_empty() && input_ys.is_empty() {
        return Ok(());
    }

    tracing::info!(
        "Rolling back swap setup ({} blinded messages, {} proofs, saga {})",
        blinded_secrets.len(),
        input_ys.len(),
        operation_id
    );

    let mut tx = db.begin_transaction().await?;

    if !blinded_secrets.is_empty() {
        tx.delete_blinded_messages(blinded_secrets).await?;
    }

    if !input_ys.is_empty() {
        tx.remove_proofs(input_ys, None).await?;
    }

    if let Err(e) = tx.delete_saga(operation_id).await {
        tracing::warn!(
            "Failed to delete saga {} during compensation: {}",
            operation_id,
            e
        );
        // Continue anyway - saga cleanup is best-effort
    }

    tx.commit().await?;

    // setup_swap published Pending for these proofs; without this subscribers
    // would never see the reversal.
    for y in input_ys {
        pubsub.proof_state((*y, State::Unspent));
    }

    Ok(())
}
