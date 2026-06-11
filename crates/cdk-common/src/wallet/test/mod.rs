//! Generic wallet trait tests.
//!
//! This module provides test functions parameterized over `W: Wallet` and
//! `DB: WalletDatabase`, following the same pattern as `crate::database::wallet::test`.
//! Each backend invokes the [`wallet_test!`] macro with a provider function that
//! returns a [`WalletTestContext`], ensuring every `Wallet` implementation is
//! tested against the same suite.

#![allow(clippy::unwrap_used)]
#![allow(missing_docs)]
#![allow(missing_debug_implementations)]

use std::str::FromStr;
use std::sync::Arc;

use crate::database::{self, WalletDatabase};
use crate::mint_url::MintUrl;
use crate::nuts::{CurrencyUnit, Id, SecretKey, State};
use crate::secret::Secret;
use crate::wallet::{
    IssueSagaState, MeltOperationData, MeltSagaState, MeltQuote, MintOperationData, MintQuote,
    OperationData, ProofInfo, ReceiveOperationData, ReceiveSagaState, RecoveryReport, Wallet,
    WalletSaga, WalletSagaState,
};
use crate::{Amount, PaymentMethod, Proof};

/// Type alias for the database handle used in wallet tests.
pub type TestDb = Arc<dyn WalletDatabase<database::Error> + Send + Sync>;

// ---------------------------------------------------------------------------
// Test context
// ---------------------------------------------------------------------------

/// Context provided to every generic wallet test.
pub struct WalletTestContext<W> {
    /// The wallet under test.
    pub wallet: W,
    /// Direct database handle for test setup and verification.
    pub db: TestDb,
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn test_mint_url() -> MintUrl {
    MintUrl::from_str("https://test-mint.example.com").unwrap()
}

fn test_keyset_id() -> Id {
    Id::from_str("0094d5a774c40a32").unwrap()
}

fn test_proof(keyset_id: Id, amount: u64) -> Proof {
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

fn test_proof_info(keyset_id: Id, amount: u64, mint_url: MintUrl) -> ProofInfo {
    let proof = test_proof(keyset_id, amount);
    ProofInfo::new(proof, mint_url, State::Unspent, CurrencyUnit::Sat).unwrap()
}

fn test_melt_quote() -> MeltQuote {
    use crate::nut00::KnownMethod;

    MeltQuote {
        id: format!("test_melt_quote_{}", uuid::Uuid::new_v4()),
        mint_url: Some(test_mint_url()),
        unit: CurrencyUnit::Sat,
        amount: Amount::from(1000),
        request: "lnbc1000...".to_string(),
        fee_reserve: Amount::from(10),
        state: crate::nuts::MeltQuoteState::Unpaid,
        expiry: 9999999999,
        payment_proof: None,
        estimated_blocks: None,
        fee_index: None,
        payment_method: PaymentMethod::Known(KnownMethod::Bolt11),
        used_by_operation: None,
        version: 0,
    }
}

fn test_mint_quote(mint_url: MintUrl) -> MintQuote {
    use crate::nut00::KnownMethod;

    MintQuote::new(
        format!("test_mint_quote_{}", uuid::Uuid::new_v4()),
        mint_url,
        PaymentMethod::Known(KnownMethod::Bolt11),
        Some(Amount::from(1000)),
        CurrencyUnit::Sat,
        "lnbc1000...".to_string(),
        9999999999,
        None,
    )
}

// ---------------------------------------------------------------------------
// Trait-surface tests (fresh wallet, no setup)
// ---------------------------------------------------------------------------

pub async fn total_balance_empty<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<Amount = Amount, RecoveryReport = RecoveryReport>,
{
    let balance = ctx.wallet.total_balance().await.unwrap();
    assert_eq!(balance, Amount::ZERO);
}

pub async fn total_pending_balance_empty<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<Amount = Amount, RecoveryReport = RecoveryReport>,
{
    let balance = ctx.wallet.total_pending_balance().await.unwrap();
    assert_eq!(balance, Amount::ZERO);
}

pub async fn total_reserved_balance_empty<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<Amount = Amount, RecoveryReport = RecoveryReport>,
{
    let balance = ctx.wallet.total_reserved_balance().await.unwrap();
    assert_eq!(balance, Amount::ZERO);
}

pub async fn list_transactions_empty<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let txs = ctx.wallet.list_transactions(None).await.unwrap();
    assert!(txs.is_empty());
}

pub async fn get_proofs_by_states_empty<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let proofs = ctx
        .wallet
        .get_proofs_by_states(vec![State::Unspent])
        .await
        .unwrap();
    assert!(proofs.is_empty());
}

pub async fn generate_and_get_public_key<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let pubkey = ctx.wallet.generate_public_key().await.unwrap();
    let retrieved = ctx.wallet.get_public_key(&pubkey).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().pubkey, pubkey);
}

pub async fn generate_multiple_public_keys<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let pk1 = ctx.wallet.generate_public_key().await.unwrap();
    let pk2 = ctx.wallet.generate_public_key().await.unwrap();
    assert_ne!(pk1, pk2);

    let keys = ctx.wallet.get_public_keys().await.unwrap();
    assert_eq!(keys.len(), 2);

    let latest = ctx.wallet.get_latest_public_key().await.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().pubkey, pk2);
}

// ---------------------------------------------------------------------------
// Recovery tests (need DB setup, no mock connector)
// ---------------------------------------------------------------------------

pub async fn recover_no_incomplete_sagas<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let report = ctx.wallet.recover_incomplete_sagas().await.unwrap();
    assert_eq!(report.recovered, 0);
    assert_eq!(report.compensated, 0);
    assert_eq!(report.skipped, 0);
    assert_eq!(report.failed, 0);
}

pub async fn recover_receive_proofs_pending<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let mint_url = test_mint_url();
    let saga_id = uuid::Uuid::new_v4();

    let saga = WalletSaga::new(
        saga_id,
        WalletSagaState::Receive(ReceiveSagaState::ProofsPending),
        Amount::from(100),
        mint_url.clone(),
        CurrencyUnit::Sat,
        OperationData::Receive(ReceiveOperationData {
            token: Some("cashu...".to_string()),
            counter_start: None,
            counter_end: None,
            amount: Some(Amount::from(100)),
            blinded_messages: None,
        }),
    );
    ctx.db.add_saga(saga).await.unwrap();

    let report = ctx.wallet.recover_incomplete_sagas().await.unwrap();
    assert_eq!(report.compensated, 1);

    assert!(ctx.db.get_saga(&saga_id).await.unwrap().is_none());
}

pub async fn recover_issue_secrets_prepared<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let mint_url = test_mint_url();
    let saga_id = uuid::Uuid::new_v4();

    let mut quote = test_mint_quote(mint_url.clone());
    quote.used_by_operation = Some(saga_id.to_string());
    ctx.db.add_mint_quote(quote.clone()).await.unwrap();

    let saga = WalletSaga::new(
        saga_id,
        WalletSagaState::Issue(IssueSagaState::SecretsPrepared),
        Amount::from(1000),
        mint_url.clone(),
        CurrencyUnit::Sat,
        OperationData::Mint(MintOperationData::new_single(
            quote.id.clone(),
            Amount::from(1000),
            Some(0),
            Some(10),
            None,
        )),
    );
    ctx.db.add_saga(saga).await.unwrap();

    let report = ctx.wallet.recover_incomplete_sagas().await.unwrap();
    assert_eq!(report.compensated, 1);

    let retrieved_quote = ctx.db.get_mint_quote(&quote.id).await.unwrap().unwrap();
    assert!(retrieved_quote.used_by_operation.is_none());

    assert!(ctx.db.get_saga(&saga_id).await.unwrap().is_none());
}

pub async fn recover_melt_proofs_reserved<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let mint_url = test_mint_url();
    let keyset_id = test_keyset_id();
    let saga_id = uuid::Uuid::new_v4();

    let proof_info = test_proof_info(keyset_id, 100, mint_url.clone());
    let proof_y = proof_info.y;
    ctx.db.update_proofs(vec![proof_info], vec![]).await.unwrap();
    ctx.db
        .reserve_proofs(vec![proof_y], &saga_id)
        .await
        .unwrap();

    let mut quote = test_melt_quote();
    quote.used_by_operation = Some(saga_id.to_string());
    ctx.db.add_melt_quote(quote.clone()).await.unwrap();

    let saga = WalletSaga::new(
        saga_id,
        WalletSagaState::Melt(MeltSagaState::ProofsReserved),
        Amount::from(100),
        mint_url.clone(),
        CurrencyUnit::Sat,
        OperationData::Melt(MeltOperationData {
            quote_id: quote.id.clone(),
            amount: Amount::from(100),
            fee_reserve: Amount::from(10),
            counter_start: None,
            counter_end: None,
            change_amount: None,
            change_blinded_messages: None,
        }),
    );
    ctx.db.add_saga(saga).await.unwrap();

    let report = ctx.wallet.recover_incomplete_sagas().await.unwrap();
    assert_eq!(report.compensated, 1);

    let proofs = ctx
        .db
        .get_proofs(None, None, Some(vec![State::Unspent]), None)
        .await
        .unwrap();
    assert_eq!(proofs.len(), 1);

    let retrieved_quote = ctx.db.get_melt_quote(&quote.id).await.unwrap().unwrap();
    assert!(retrieved_quote.used_by_operation.is_none());

    assert!(ctx.db.get_saga(&saga_id).await.unwrap().is_none());
}

pub async fn recover_multiple_sagas<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let mint_url = test_mint_url();
    let keyset_id = test_keyset_id();

    for i in 0..3 {
        let saga_id = uuid::Uuid::new_v4();

        let proof_info = test_proof_info(keyset_id, 100, mint_url.clone());
        let proof_y = proof_info.y;
        ctx.db.update_proofs(vec![proof_info], vec![]).await.unwrap();
        ctx.db
            .reserve_proofs(vec![proof_y], &saga_id)
            .await
            .unwrap();

        let mut quote = test_melt_quote();
        quote.id = format!("quote_{}", i);
        quote.used_by_operation = Some(saga_id.to_string());
        ctx.db.add_melt_quote(quote.clone()).await.unwrap();

        let saga = WalletSaga::new(
            saga_id,
            WalletSagaState::Melt(MeltSagaState::ProofsReserved),
            Amount::from(100),
            mint_url.clone(),
            CurrencyUnit::Sat,
            OperationData::Melt(MeltOperationData {
                quote_id: quote.id.clone(),
                amount: Amount::from(100),
                fee_reserve: Amount::from(10),
                counter_start: None,
                counter_end: None,
                change_amount: None,
                change_blinded_messages: None,
            }),
        );
        ctx.db.add_saga(saga).await.unwrap();
    }

    let report = ctx.wallet.recover_incomplete_sagas().await.unwrap();
    assert_eq!(report.compensated, 3);

    let proofs = ctx
        .db
        .get_proofs(None, None, Some(vec![State::Unspent]), None)
        .await
        .unwrap();
    assert_eq!(proofs.len(), 3);

    let sagas = ctx.db.get_incomplete_sagas().await.unwrap();
    assert!(sagas.is_empty());
}

pub async fn cleanup_orphaned_melt_quote_reservation<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let operation_id = uuid::Uuid::new_v4();

    let mut quote = test_melt_quote();
    quote.used_by_operation = Some(operation_id.to_string());
    ctx.db.add_melt_quote(quote.clone()).await.unwrap();

    let _report = ctx.wallet.recover_incomplete_sagas().await.unwrap();

    let retrieved_quote = ctx.db.get_melt_quote(&quote.id).await.unwrap().unwrap();
    assert!(retrieved_quote.used_by_operation.is_none());
}

pub async fn cleanup_orphaned_mint_quote_reservation<W>(ctx: WalletTestContext<W>)
where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let mint_url = test_mint_url();
    let operation_id = uuid::Uuid::new_v4();

    let mut quote = test_mint_quote(mint_url);
    quote.used_by_operation = Some(operation_id.to_string());
    ctx.db.add_mint_quote(quote.clone()).await.unwrap();

    let _report = ctx.wallet.recover_incomplete_sagas().await.unwrap();

    let retrieved_quote = ctx.db.get_mint_quote(&quote.id).await.unwrap().unwrap();
    assert!(retrieved_quote.used_by_operation.is_none());
}

pub async fn recover_incomplete_sagas_filters_by_mint_and_unit<W>(
    ctx: WalletTestContext<W>,
) where
    W: Wallet<RecoveryReport = RecoveryReport>,

{
    let mint_url = test_mint_url();
    let other_mint_url = MintUrl::from_str("https://other-mint.example.com").unwrap();
    let saga_id_1 = uuid::Uuid::new_v4();
    let saga_id_2 = uuid::Uuid::new_v4();
    let saga_id_3 = uuid::Uuid::new_v4();

    // 1. Saga for our mint and unit (should be recovered/compensated)
    let saga_1 = WalletSaga::new(
        saga_id_1,
        WalletSagaState::Receive(ReceiveSagaState::ProofsPending),
        Amount::from(100),
        mint_url.clone(),
        CurrencyUnit::Sat,
        OperationData::Receive(ReceiveOperationData {
            token: Some("cashu...".to_string()),
            counter_start: None,
            counter_end: None,
            amount: Some(Amount::from(100)),
            blinded_messages: None,
        }),
    );
    ctx.db.add_saga(saga_1).await.unwrap();

    // 2. Saga for OTHER mint (should be skipped)
    let saga_2 = WalletSaga::new(
        saga_id_2,
        WalletSagaState::Receive(ReceiveSagaState::ProofsPending),
        Amount::from(100),
        other_mint_url.clone(),
        CurrencyUnit::Sat,
        OperationData::Receive(ReceiveOperationData {
            token: Some("cashu...".to_string()),
            counter_start: None,
            counter_end: None,
            amount: Some(Amount::from(100)),
            blinded_messages: None,
        }),
    );
    ctx.db.add_saga(saga_2).await.unwrap();

    // 3. Saga for our mint but OTHER unit (should be skipped)
    let saga_3 = WalletSaga::new(
        saga_id_3,
        WalletSagaState::Receive(ReceiveSagaState::ProofsPending),
        Amount::from(100),
        mint_url.clone(),
        CurrencyUnit::Usd,
        OperationData::Receive(ReceiveOperationData {
            token: Some("cashu...".to_string()),
            counter_start: None,
            counter_end: None,
            amount: Some(Amount::from(100)),
            blinded_messages: None,
        }),
    );
    ctx.db.add_saga(saga_3).await.unwrap();

    let report = ctx.wallet.recover_incomplete_sagas().await.unwrap();

    assert_eq!(report.compensated, 1);
    assert_eq!(report.recovered, 0);
    assert_eq!(report.skipped, 0);

    assert!(ctx.db.get_saga(&saga_id_1).await.unwrap().is_none());
    assert!(ctx.db.get_saga(&saga_id_2).await.unwrap().is_some());
    assert!(ctx.db.get_saga(&saga_id_3).await.unwrap().is_some());
}

// ---------------------------------------------------------------------------
// Macro
// ---------------------------------------------------------------------------

/// Generates `#[tokio::test]` wrappers for every generic wallet test function.
///
/// Usage (in the crate that provides a concrete `Wallet` implementation):
///
/// ```ignore
/// use cdk_common::wallet_test;
///
/// async fn provide_wallet(_name: String) -> WalletTestContext<MyWallet, MyDb> { .. }
///
/// wallet_test!(provide_wallet);
/// ```
#[macro_export]
macro_rules! wallet_test {
    ($make_ctx_fn:ident) => {
        wallet_test!(
            $make_ctx_fn,
            // trait-surface
            total_balance_empty,
            total_pending_balance_empty,
            total_reserved_balance_empty,
            list_transactions_empty,
            get_proofs_by_states_empty,
            generate_and_get_public_key,
            generate_multiple_public_keys,
            // recovery
            recover_no_incomplete_sagas,
            recover_receive_proofs_pending,
            recover_issue_secrets_prepared,
            recover_melt_proofs_reserved,
            recover_multiple_sagas,
            cleanup_orphaned_melt_quote_reservation,
            cleanup_orphaned_mint_quote_reservation,
            recover_incomplete_sagas_filters_by_mint_and_unit
        );
    };
    ($make_ctx_fn:ident, $($name:ident),+ $(,)?) => {
        ::paste::paste! {
            $(
                #[tokio::test]
                async fn [<wallet_ $name>]() {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards");

                    cdk_common::wallet::test::$name(
                        $make_ctx_fn(format!("test_{}_{}", now.as_nanos(), stringify!($name))).await
                    ).await;
                }
            )+
        }
    };
}
