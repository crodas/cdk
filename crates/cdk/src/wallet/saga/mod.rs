//! Wallet Saga Pattern Implementation
//!
//! Wallet operations use the saga machinery in [`crate::saga`], with the wallet
//! itself as the context its compensating actions run against.
//!
//! # Type State Pattern
//!
//! The type state pattern uses Rust's type system to enforce valid state transitions
//! at compile-time. Each operation state is a distinct type, and operations are only
//! available on the appropriate type.
//!
//! # Compensation Pattern
//!
//! When a saga step fails, compensating actions are executed in reverse order (LIFO)
//! to undo all completed steps and restore the database to its pre-saga state.

use std::sync::Arc;

use async_trait::async_trait;
use cdk_common::database::{self, WalletDatabase};
pub(crate) use cdk_common::saga::CompensatingAction;
use cdk_common::saga::SagaContext;
use tracing::instrument;

use crate::{Error, Wallet};

/// Handles the wallet's saga steps and compensating actions operate against.
pub(crate) struct WalletSagaContext<'a> {
    /// The wallet the saga belongs to.
    pub wallet: &'a Wallet,
}

impl SagaContext for WalletSagaContext<'_> {}

impl<'a> WalletSagaContext<'a> {
    /// Build a context for a saga running against `wallet`.
    pub fn new(wallet: &'a Wallet) -> Self {
        Self { wallet }
    }

    /// The wallet's local store, which every compensating action writes through.
    pub fn localstore(&self) -> &Arc<dyn WalletDatabase<database::Error> + Send + Sync> {
        &self.wallet.localstore
    }
}

/// A boxed wallet compensating action.
pub(crate) type WalletCompensation<'a> = Box<dyn CompensatingAction<WalletSagaContext<'a>>>;

impl Wallet {
    /// Roll back a saga that cannot be completed, running `actions` in order.
    ///
    /// Every action is attempted even if an earlier one fails: leaving the
    /// remaining steps un-reversed would strand more state than the one failure
    /// already has. The first failure is returned once they have all run, so
    /// recovery retries the saga rather than reporting it as rolled back.
    pub(crate) async fn compensate_saga<'a>(
        &'a self,
        actions: Vec<WalletCompensation<'a>>,
    ) -> Result<(), Error> {
        let ctx = WalletSagaContext::new(self);
        let mut first_error = None;

        for action in actions {
            if let Err(e) = action.execute(&ctx).await {
                tracing::warn!("Compensation {} failed: {}. Continuing.", action.name(), e);
                first_error.get_or_insert(e);
            }
        }

        match first_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// The `y` values of every proof still linked to this operation.
    pub(crate) async fn saga_proof_ys(
        &self,
        saga_id: &uuid::Uuid,
    ) -> Result<Vec<crate::nuts::PublicKey>, Error> {
        Ok(self
            .localstore
            .get_reserved_proofs(saga_id)
            .await?
            .iter()
            .map(|p| p.y)
            .collect())
    }
}

/// Releases the proofs an operation reserved, then deletes the saga record.
///
/// Only proofs still held by this operation are released, so a proof that was
/// spent in the meantime (by a swap that completed before a crash, say) is left
/// alone.
pub(crate) struct RevertProofReservation {
    pub saga_id: uuid::Uuid,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<'a> CompensatingAction<WalletSagaContext<'a>> for RevertProofReservation {
    #[instrument(skip_all)]
    async fn execute(&self, ctx: &WalletSagaContext<'a>) -> Result<(), Error> {
        tracing::info!(
            "Compensation: Releasing proofs reserved by operation {}",
            self.saga_id
        );

        ctx.localstore()
            .release_proofs(&self.saga_id)
            .await
            .map_err(Error::Database)?;

        if let Err(e) = ctx.localstore().delete_saga(&self.saga_id).await {
            tracing::warn!(
                "Compensation: Failed to delete saga {}: {}. Will be cleaned up on recovery.",
                self.saga_id,
                e
            );
            // Don't fail compensation if saga deletion fails - orphaned saga is harmless
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "RevertProofReservation"
    }
}

/// Deletes the saga record without touching any proofs.
///
/// Used by operations that reserve a quote rather than proofs, where there is
/// nothing else to undo.
pub(crate) struct DeleteSaga {
    pub saga_id: uuid::Uuid,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<'a> CompensatingAction<WalletSagaContext<'a>> for DeleteSaga {
    #[instrument(skip_all)]
    async fn execute(&self, ctx: &WalletSagaContext<'a>) -> Result<(), Error> {
        if let Err(e) = ctx.localstore().delete_saga(&self.saga_id).await {
            tracing::warn!(
                "Compensation: Failed to delete saga {}: {}. Will be cleaned up on recovery.",
                self.saga_id,
                e
            );
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "DeleteSaga"
    }
}

/// Test utilities shared across wallet saga compensation tests.
#[cfg(test)]
pub mod test_utils {
    use std::str::FromStr;
    use std::sync::Arc;

    use cdk_common::database::WalletDatabase;

    use crate::wallet::test_utils::{create_test_wallet_with_mock, MockMintConnector};
    use crate::Wallet;

    /// A wallet backed by `db`, for running compensations against a context.
    pub async fn test_wallet(
        db: Arc<dyn WalletDatabase<cdk_common::database::Error> + Send + Sync>,
    ) -> Wallet {
        create_test_wallet_with_mock(db, Arc::new(MockMintConnector::new())).await
    }
    use cdk_common::nuts::{CurrencyUnit, Id, Proof, State};
    use cdk_common::secret::Secret;
    use cdk_common::wallet::ProofInfo;
    use cdk_common::{Amount, SecretKey};

    /// Create an in-memory test database
    pub async fn create_test_db(
    ) -> Arc<dyn WalletDatabase<cdk_common::database::Error> + Send + Sync> {
        Arc::new(cdk_sqlite::wallet::memory::empty().await.unwrap())
    }

    /// Create a test keyset ID
    pub fn test_keyset_id() -> Id {
        Id::from_str("00916bbf7ef91a36").unwrap()
    }

    /// Create a test mint URL
    pub fn test_mint_url() -> cdk_common::mint_url::MintUrl {
        cdk_common::mint_url::MintUrl::from_str("https://test-mint.example.com").unwrap()
    }

    /// Create a test proof with the given keyset ID and amount
    pub fn test_proof(keyset_id: Id, amount: u64) -> Proof {
        Proof {
            amount: Amount::from(amount),
            keyset_id,
            secret: Secret::generate(),
            c: SecretKey::generate().public_key(),
            witness: None,
            dleq: None,
            p2pk_e: None,
        }
    }

    /// Create a test proof info with the given parameters
    pub fn test_proof_info(
        keyset_id: Id,
        amount: u64,
        mint_url: cdk_common::mint_url::MintUrl,
        state: State,
    ) -> ProofInfo {
        let proof = test_proof(keyset_id, amount);
        ProofInfo::new(proof, mint_url, state, CurrencyUnit::Sat).unwrap()
    }

    /// Create a test wallet saga for testing compensations
    pub fn test_simple_saga(
        mint_url: cdk_common::mint_url::MintUrl,
    ) -> cdk_common::wallet::WalletSaga {
        use cdk_common::wallet::{
            OperationData, SwapOperationData, SwapSagaState, WalletSaga, WalletSagaState,
        };
        use cdk_common::Amount;

        WalletSaga::new(
            uuid::Uuid::new_v4(),
            WalletSagaState::Swap(SwapSagaState::ProofsReserved),
            Amount::from(1000),
            mint_url,
            CurrencyUnit::Sat,
            OperationData::Swap(SwapOperationData {
                input_amount: Amount::from(1000),
                output_amount: Amount::from(990),
                counter_start: Some(0),
                counter_end: Some(10),
                blinded_messages: None,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use cdk_common::nuts::State;

    use super::*;

    /// Reserve `proofs` for `saga_id` and return a wallet to compensate against.
    async fn reserved_setup(
        amounts: &[u64],
    ) -> (
        Wallet,
        Arc<dyn WalletDatabase<database::Error> + Send + Sync>,
        uuid::Uuid,
        Vec<crate::nuts::PublicKey>,
    ) {
        let db = test_utils::create_test_db().await;
        let mint_url = test_utils::test_mint_url();
        let keyset_id = test_utils::test_keyset_id();

        let proofs: Vec<_> = amounts
            .iter()
            .map(|a| test_utils::test_proof_info(keyset_id, *a, mint_url.clone(), State::Unspent))
            .collect();
        let ys: Vec<_> = proofs.iter().map(|p| p.y).collect();
        db.update_proofs(proofs, vec![]).await.unwrap();

        let saga = test_utils::test_simple_saga(mint_url);
        let saga_id = saga.id;
        db.add_saga(saga).await.unwrap();
        db.reserve_proofs(ys.clone(), &saga_id).await.unwrap();

        let wallet = test_utils::test_wallet(db.clone()).await;
        (wallet, db, saga_id, ys)
    }

    #[tokio::test]
    async fn revert_proof_reservation_is_idempotent() {
        let (wallet, db, saga_id, _) = reserved_setup(&[100]).await;
        let ctx = WalletSagaContext::new(&wallet);
        let compensation = RevertProofReservation { saga_id };

        compensation.execute(&ctx).await.unwrap();
        compensation.execute(&ctx).await.unwrap();

        let unspent = db
            .get_proofs(None, None, Some(vec![State::Unspent]), None)
            .await
            .unwrap();
        assert_eq!(unspent.len(), 1);
        assert_eq!(unspent[0].used_by_operation, None);
    }

    #[tokio::test]
    async fn revert_proof_reservation_handles_missing_saga() {
        let (wallet, db, _, _) = reserved_setup(&[100]).await;
        let ctx = WalletSagaContext::new(&wallet);

        // A saga id that was never persisted still has to succeed.
        let compensation = RevertProofReservation {
            saga_id: uuid::Uuid::new_v4(),
        };
        compensation.execute(&ctx).await.unwrap();

        let reserved = db
            .get_proofs(None, None, Some(vec![State::Reserved]), None)
            .await
            .unwrap();
        assert_eq!(reserved.len(), 1, "another saga's proofs must be untouched");
    }

    #[tokio::test]
    async fn revert_proof_reservation_only_affects_its_own_operation() {
        let (wallet, db, saga_id, ys) = reserved_setup(&[100, 200]).await;

        // Hand the second proof to a different operation.
        let other_saga_id = uuid::Uuid::new_v4();
        let mut moved = db.get_proofs_by_ys(vec![ys[1]]).await.unwrap();
        moved[0].used_by_operation = Some(other_saga_id);
        db.update_proofs(moved, vec![]).await.unwrap();

        let ctx = WalletSagaContext::new(&wallet);
        RevertProofReservation { saga_id }
            .execute(&ctx)
            .await
            .unwrap();

        let unspent = db
            .get_proofs(None, None, Some(vec![State::Unspent]), None)
            .await
            .unwrap();
        assert_eq!(unspent.len(), 1);
        assert_eq!(unspent[0].y, ys[0]);

        let still_reserved = db.get_proofs_by_ys(vec![ys[1]]).await.unwrap();
        assert_eq!(still_reserved[0].state, State::Reserved);
        assert_eq!(still_reserved[0].used_by_operation, Some(other_saga_id));
    }

    #[tokio::test]
    async fn delete_saga_removes_the_record_and_leaves_proofs_alone() {
        let (wallet, db, saga_id, ys) = reserved_setup(&[100]).await;
        let ctx = WalletSagaContext::new(&wallet);

        DeleteSaga { saga_id }.execute(&ctx).await.unwrap();

        assert!(db.get_saga(&saga_id).await.unwrap().is_none());
        let proofs = db.get_proofs_by_ys(ys).await.unwrap();
        assert_eq!(proofs[0].state, State::Reserved);
    }
}
