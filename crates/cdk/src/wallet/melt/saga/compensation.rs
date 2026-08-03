//! Compensation actions for the melt saga.
//!
//! When a saga step fails, compensating actions are executed in reverse order (LIFO)
//! to undo all completed steps and restore the database to its pre-saga state.

use async_trait::async_trait;
use tracing::instrument;
use uuid::Uuid;

use crate::wallet::saga::{CompensatingAction, WalletSagaContext};
// Re-export shared compensation actions used by melt saga
pub(crate) use crate::wallet::saga::RevertProofReservation;
use crate::Error;

/// Compensation action to release a melt quote reservation.
pub struct ReleaseMeltQuote {
    /// Operation ID that reserved the quote
    pub operation_id: Uuid,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<'a> CompensatingAction<WalletSagaContext<'a>> for ReleaseMeltQuote {
    #[instrument(skip_all)]
    async fn execute(&self, ctx: &WalletSagaContext<'a>) -> Result<(), Error> {
        tracing::info!(
            "Compensation: Releasing melt quote reserved by operation {}",
            self.operation_id
        );

        ctx.localstore()
            .release_melt_quote(&self.operation_id)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ReleaseMeltQuote"
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cdk_common::nut00::KnownMethod;
    use cdk_common::nuts::{CurrencyUnit, MeltQuoteState};
    use cdk_common::wallet::MeltQuote;
    use cdk_common::{Amount, PaymentMethod};

    use super::*;
    use crate::wallet::saga::test_utils::*;
    use crate::wallet::saga::CompensatingAction;
    use crate::wallet::saga::WalletSagaContext;

    /// Create a test melt quote
    fn test_melt_quote() -> MeltQuote {
        MeltQuote {
            id: format!("test_melt_quote_{}", uuid::Uuid::new_v4()),
            mint_url: Some(
                cdk_common::mint_url::MintUrl::from_str("https://test-mint.example.com").unwrap(),
            ),
            unit: CurrencyUnit::Sat,
            amount: Amount::from(1000),
            request: "lnbc1000...".to_string(),
            fee_reserve: Amount::from(10),
            state: MeltQuoteState::Unpaid,
            expiry: 9999999999,
            payment_proof: None,
            estimated_blocks: None,
            fee_index: None,
            payment_method: PaymentMethod::Known(KnownMethod::Bolt11),
            used_by_operation: None,
            version: 0,
        }
    }

    // =========================================================================
    // ReleaseMeltQuote Tests
    // =========================================================================

    #[tokio::test]
    async fn test_release_melt_quote_is_idempotent() {
        let db = create_test_db().await;
        let wallet = test_wallet(db.clone()).await;
        let ctx = WalletSagaContext::new(&wallet);
        let operation_id = uuid::Uuid::new_v4();

        let mut quote = test_melt_quote();
        quote.used_by_operation = Some(operation_id.to_string());
        db.add_melt_quote(quote.clone()).await.unwrap();

        let compensation = ReleaseMeltQuote { operation_id };

        // Execute twice
        compensation.execute(&ctx).await.unwrap();
        compensation.execute(&ctx).await.unwrap();

        let retrieved_quote = db.get_melt_quote(&quote.id).await.unwrap().unwrap();
        assert!(retrieved_quote.used_by_operation.is_none());
    }

    #[tokio::test]
    async fn test_release_melt_quote_handles_no_matching_quote() {
        let db = create_test_db().await;
        let wallet = test_wallet(db.clone()).await;
        let ctx = WalletSagaContext::new(&wallet);
        let operation_id = uuid::Uuid::new_v4();

        // Don't add any quote - compensation should still succeed
        let compensation = ReleaseMeltQuote { operation_id };

        // Should not error even with no matching quote
        let result = compensation.execute(&ctx).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // Isolation Tests
    // =========================================================================
}
