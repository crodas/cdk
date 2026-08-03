//! Compensation actions for the mint (issue) saga.
//!
//! When a saga step fails, compensating actions are executed in reverse order (LIFO)
//! to undo all completed steps and restore the database to its pre-saga state.
//!
//! Note: For mint operations, the primary side effect before the API call is
//! incrementing the keyset counter. Counter increments are not reversed because:
//! 1. They don't cause data loss (just potentially unused counter values)
//! 2. The secrets can be recovered via the restore process
//! 3. Reversing could cause issues if concurrent operations used adjacent counters

use async_trait::async_trait;
use tracing::instrument;
use uuid::Uuid;

use crate::wallet::saga::{CompensatingAction, WalletSagaContext};
// The mint flow's only other rollback is deleting the saga record: counter
// increments are deliberately left in place (see the module docs).
pub(crate) use crate::wallet::saga::DeleteSaga;
use crate::Error;

/// Compensation action to release a mint quote reservation.
/// Clears the used_by_operation field on the quote.
pub struct ReleaseMintQuote {
    /// Operation ID that reserved the quote
    pub operation_id: Uuid,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<'a> CompensatingAction<WalletSagaContext<'a>> for ReleaseMintQuote {
    #[instrument(skip_all)]
    async fn execute(&self, ctx: &WalletSagaContext<'a>) -> Result<(), Error> {
        tracing::info!(
            "Compensation: Releasing mint quote reserved by operation {}",
            self.operation_id
        );

        ctx.localstore()
            .release_mint_quote(&self.operation_id)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ReleaseMintQuote"
    }
}

#[cfg(test)]
mod tests {
    use cdk_common::nut00::KnownMethod;
    use cdk_common::nuts::CurrencyUnit;
    use cdk_common::wallet::{
        MintQuote, OperationData, SwapOperationData, SwapSagaState, WalletSaga, WalletSagaState,
    };
    use cdk_common::{Amount, PaymentMethod};

    use super::*;
    use crate::wallet::saga::test_utils::*;
    use crate::wallet::saga::CompensatingAction;
    use crate::wallet::saga::WalletSagaContext;

    /// Create a test wallet saga for issue operations
    fn test_issue_saga(mint_url: cdk_common::mint_url::MintUrl) -> WalletSaga {
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

    /// Create a test mint quote
    fn test_mint_quote(mint_url: cdk_common::mint_url::MintUrl) -> MintQuote {
        MintQuote::new(
            format!("test_quote_{}", uuid::Uuid::new_v4()),
            mint_url,
            PaymentMethod::Known(KnownMethod::Bolt11),
            Some(Amount::from(1000)),
            CurrencyUnit::Sat,
            "lnbc1000...".to_string(),
            9999999999,
            None,
        )
    }

    // =========================================================================
    // ReleaseMintQuote Tests
    // =========================================================================

    #[tokio::test]
    async fn test_release_mint_quote_is_idempotent() {
        let db = create_test_db().await;
        let wallet = test_wallet(db.clone()).await;
        let ctx = WalletSagaContext::new(&wallet);
        let mint_url = test_mint_url();
        let operation_id = uuid::Uuid::new_v4();

        let mut quote = test_mint_quote(mint_url);
        quote.used_by_operation = Some(operation_id.to_string());
        db.add_mint_quote(quote.clone()).await.unwrap();

        let compensation = ReleaseMintQuote { operation_id };

        // Execute twice
        compensation.execute(&ctx).await.unwrap();
        compensation.execute(&ctx).await.unwrap();

        let retrieved_quote = db.get_mint_quote(&quote.id).await.unwrap().unwrap();
        assert!(retrieved_quote.used_by_operation.is_none());
    }

    #[tokio::test]
    async fn test_release_mint_quote_handles_no_matching_quote() {
        let db = create_test_db().await;
        let wallet = test_wallet(db.clone()).await;
        let ctx = WalletSagaContext::new(&wallet);
        let operation_id = uuid::Uuid::new_v4();

        // Don't add any quote - compensation should still succeed
        let compensation = ReleaseMintQuote { operation_id };

        // Should not error even with no matching quote
        let result = compensation.execute(&ctx).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // DeleteSaga Tests
    // =========================================================================

    #[tokio::test]
    async fn test_mint_compensation_is_idempotent() {
        let db = create_test_db().await;
        let wallet = test_wallet(db.clone()).await;
        let ctx = WalletSagaContext::new(&wallet);
        let mint_url = test_mint_url();

        let saga = test_issue_saga(mint_url);
        let saga_id = saga.id;
        db.add_saga(saga).await.unwrap();

        let compensation = DeleteSaga { saga_id };

        // Execute twice - should succeed both times
        compensation.execute(&ctx).await.unwrap();
        compensation.execute(&ctx).await.unwrap();

        assert!(db.get_saga(&saga_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mint_compensation_handles_missing_saga() {
        let db = create_test_db().await;
        let wallet = test_wallet(db.clone()).await;
        let ctx = WalletSagaContext::new(&wallet);
        let saga_id = uuid::Uuid::new_v4();

        let compensation = DeleteSaga { saga_id };

        // Should succeed even without saga
        let result = compensation.execute(&ctx).await;
        assert!(result.is_ok());
    }
}
