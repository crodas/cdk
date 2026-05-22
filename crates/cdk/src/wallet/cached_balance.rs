//! Wallet balance cache decorator.
//!
//! `CachedBalance<T>` wraps any [`WalletDatabase`] implementation. It intercepts
//! proof-mutating methods to maintain balance deltas in the KV store and serves
//! `get_balance` from KV instead of scanning the proof table.

use std::collections::HashMap;
use std::fmt::Debug;

use async_trait::async_trait;
use bitcoin::bip32::DerivationPath;
use cdk_common::database::{self, WalletDatabase};
use cdk_common::mint_url::MintUrl;
use cdk_common::nuts::{
    CurrencyUnit, Id, KeySet, KeySetInfo, Keys, MintInfo, PublicKey, SpendingConditions, State,
};
use cdk_common::wallet::{
    self, MintQuote, ProofInfo, Transaction, TransactionDirection, TransactionId,
};

const KV_NS: &str = "balance_cache";
const KV_SUB: &str = "";

/// Decorator that maintains per-(mint_url, unit, state) balance totals in KV
/// storage and serves `get_balance` from those totals.
pub struct CachedBalance<T> {
    inner: T,
}

impl<T: Debug> Debug for CachedBalance<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedBalance")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<T> CachedBalance<T>
where
    T: WalletDatabase<database::Error> + Send + Sync,
{
    /// Wrap a store. Rebuilds the cache from proofs if no KV entries exist.
    pub async fn new(inner: T) -> Result<Self, database::Error> {
        let cached = Self { inner };
        let keys = cached.inner.kv_list(KV_NS, KV_SUB).await?;
        if keys.is_empty() {
            cached.rebuild().await?;
        }
        Ok(cached)
    }

    /// Rebuild cache from the proof table and persist to KV.
    pub async fn rebuild(&self) -> Result<(), database::Error> {
        // Clear existing KV entries
        let old_keys = self.inner.kv_list(KV_NS, KV_SUB).await?;
        for key in old_keys {
            self.inner.kv_remove(KV_NS, KV_SUB, &key).await?;
        }

        // Sum all proofs by (mint_url, unit, state)
        let proofs = self.inner.get_proofs(None, None, None, None).await?;
        let mut totals: HashMap<String, u64> = HashMap::new();
        for p in &proofs {
            let key = kv_key(&p.mint_url, &p.unit, &p.state);
            *totals.entry(key).or_default() += u64::from(p.proof.amount);
        }

        for (key, total) in totals {
            self.inner
                .kv_write(KV_NS, KV_SUB, &key, total.to_string().as_bytes())
                .await?;
        }

        Ok(())
    }

    /// Apply signed deltas to KV entries.
    async fn apply_deltas(
        &self,
        deltas: Vec<(String, i64)>,
    ) -> Result<(), database::Error> {
        // Merge deltas for the same key
        let mut merged: HashMap<String, i64> = HashMap::new();
        for (key, delta) in deltas {
            *merged.entry(key).or_default() += delta;
        }

        for (key, delta) in merged {
            if delta == 0 {
                continue;
            }
            let current = self
                .inner
                .kv_read(KV_NS, KV_SUB, &key)
                .await?
                .and_then(|b| String::from_utf8_lossy(&b).parse::<u64>().ok())
                .unwrap_or(0);
            let new_val = (current as i64).saturating_add(delta).max(0) as u64;
            self.inner
                .kv_write(KV_NS, KV_SUB, &key, new_val.to_string().as_bytes())
                .await?;
        }
        Ok(())
    }
}

fn kv_key(mint_url: &MintUrl, unit: &CurrencyUnit, state: &State) -> String {
    format!("{}:{}:{}", mint_url, unit, state)
}

/// Parse a KV key back into (mint_url, unit, state) strings.
fn parse_kv_key(s: &str) -> Option<(String, String, String)> {
    let mut parts = s.splitn(3, ':');
    let mint_url = parts.next()?.to_string();
    let unit = parts.next()?.to_string();
    let state = parts.next()?.to_string();
    Some((mint_url, unit, state))
}

// =============================================================================
// WalletDatabase delegation with overrides
// =============================================================================

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T> WalletDatabase<database::Error> for CachedBalance<T>
where
    T: WalletDatabase<database::Error> + Send + Sync + Debug,
{
    // ---- Overridden: balance from KV cache ----

    async fn get_balance(
        &self,
        mint_url: Option<MintUrl>,
        unit: Option<CurrencyUnit>,
        states: Option<Vec<State>>,
    ) -> Result<u64, database::Error> {
        let keys = self.inner.kv_list(KV_NS, KV_SUB).await?;
        let states: Vec<String> = states
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut total: u64 = 0;
        for key in &keys {
            let Some((m, u, s)) = parse_kv_key(key) else {
                continue;
            };
            if let Some(ref mint_url) = mint_url {
                if m != mint_url.to_string() {
                    continue;
                }
            }
            if let Some(ref unit) = unit {
                if u != unit.to_string() {
                    continue;
                }
            }
            if !states.is_empty() && !states.contains(&s) {
                continue;
            }
            let val = self
                .inner
                .kv_read(KV_NS, KV_SUB, key)
                .await?
                .and_then(|b| String::from_utf8_lossy(&b).parse::<u64>().ok())
                .unwrap_or(0);
            total = total.saturating_add(val);
        }
        Ok(total)
    }

    // ---- Overridden: proof mutations with delta tracking ----

    async fn update_proofs(
        &self,
        added: Vec<ProofInfo>,
        removed_ys: Vec<PublicKey>,
    ) -> Result<(), database::Error> {
        let mut deltas: Vec<(String, i64)> = Vec::new();

        if !removed_ys.is_empty() {
            let to_remove = self.inner.get_proofs_by_ys(removed_ys.clone()).await?;
            for p in &to_remove {
                deltas.push((
                    kv_key(&p.mint_url, &p.unit, &p.state),
                    -(u64::from(p.proof.amount) as i64),
                ));
            }
        }

        // For ON CONFLICT overwrites, subtract old values
        if !added.is_empty() {
            let existing = self
                .inner
                .get_proofs_by_ys(added.iter().map(|p| p.y).collect())
                .await?;
            for p in &existing {
                deltas.push((
                    kv_key(&p.mint_url, &p.unit, &p.state),
                    -(u64::from(p.proof.amount) as i64),
                ));
            }
        }

        for p in &added {
            deltas.push((
                kv_key(&p.mint_url, &p.unit, &p.state),
                u64::from(p.proof.amount) as i64,
            ));
        }

        self.inner.update_proofs(added, removed_ys).await?;
        self.apply_deltas(deltas).await?;
        Ok(())
    }

    async fn update_proofs_state(
        &self,
        ys: Vec<PublicKey>,
        state: State,
    ) -> Result<(), database::Error> {
        let proofs = self.inner.get_proofs_by_ys(ys.clone()).await?;
        let mut deltas: Vec<(String, i64)> = Vec::new();
        for p in &proofs {
            if p.state == state {
                continue;
            }
            let amount = u64::from(p.proof.amount) as i64;
            deltas.push((kv_key(&p.mint_url, &p.unit, &p.state), -amount));
            deltas.push((kv_key(&p.mint_url, &p.unit, &state), amount));
        }

        self.inner.update_proofs_state(ys, state).await?;
        self.apply_deltas(deltas).await?;
        Ok(())
    }

    async fn reserve_proofs(
        &self,
        ys: Vec<PublicKey>,
        operation_id: &uuid::Uuid,
    ) -> Result<(), database::Error> {
        let proofs = self.inner.get_proofs_by_ys(ys.clone()).await?;
        let mut deltas: Vec<(String, i64)> = Vec::new();
        for p in &proofs {
            let amount = u64::from(p.proof.amount) as i64;
            deltas.push((kv_key(&p.mint_url, &p.unit, &State::Unspent), -amount));
            deltas.push((kv_key(&p.mint_url, &p.unit, &State::Reserved), amount));
        }

        self.inner.reserve_proofs(ys, operation_id).await?;
        self.apply_deltas(deltas).await?;
        Ok(())
    }

    async fn release_proofs(&self, operation_id: &uuid::Uuid) -> Result<(), database::Error> {
        let proofs = self.inner.get_reserved_proofs(operation_id).await?;
        let mut deltas: Vec<(String, i64)> = Vec::new();
        for p in &proofs {
            let amount = u64::from(p.proof.amount) as i64;
            deltas.push((kv_key(&p.mint_url, &p.unit, &State::Reserved), -amount));
            deltas.push((kv_key(&p.mint_url, &p.unit, &State::Unspent), amount));
        }

        self.inner.release_proofs(operation_id).await?;
        self.apply_deltas(deltas).await?;
        Ok(())
    }

    // ---- Pure delegation for everything else ----

    async fn get_mint(&self, mint_url: MintUrl) -> Result<Option<MintInfo>, database::Error> {
        self.inner.get_mint(mint_url).await
    }
    async fn get_mints(&self) -> Result<HashMap<MintUrl, Option<MintInfo>>, database::Error> {
        self.inner.get_mints().await
    }
    async fn get_mint_keysets(
        &self,
        mint_url: MintUrl,
    ) -> Result<Option<Vec<KeySetInfo>>, database::Error> {
        self.inner.get_mint_keysets(mint_url).await
    }
    async fn get_keyset_by_id(
        &self,
        keyset_id: &Id,
    ) -> Result<Option<KeySetInfo>, database::Error> {
        self.inner.get_keyset_by_id(keyset_id).await
    }
    async fn get_mint_quote(
        &self,
        quote_id: &str,
    ) -> Result<Option<MintQuote>, database::Error> {
        self.inner.get_mint_quote(quote_id).await
    }
    async fn get_mint_quotes(&self) -> Result<Vec<MintQuote>, database::Error> {
        self.inner.get_mint_quotes().await
    }
    async fn get_unissued_mint_quotes(&self) -> Result<Vec<MintQuote>, database::Error> {
        self.inner.get_unissued_mint_quotes().await
    }
    async fn get_melt_quote(
        &self,
        quote_id: &str,
    ) -> Result<Option<wallet::MeltQuote>, database::Error> {
        self.inner.get_melt_quote(quote_id).await
    }
    async fn get_melt_quotes(&self) -> Result<Vec<wallet::MeltQuote>, database::Error> {
        self.inner.get_melt_quotes().await
    }
    async fn get_keys(&self, id: &Id) -> Result<Option<Keys>, database::Error> {
        self.inner.get_keys(id).await
    }
    async fn get_proofs(
        &self,
        mint_url: Option<MintUrl>,
        unit: Option<CurrencyUnit>,
        state: Option<Vec<State>>,
        spending_conditions: Option<Vec<SpendingConditions>>,
    ) -> Result<Vec<ProofInfo>, database::Error> {
        self.inner
            .get_proofs(mint_url, unit, state, spending_conditions)
            .await
    }
    async fn get_proofs_by_ys(
        &self,
        ys: Vec<PublicKey>,
    ) -> Result<Vec<ProofInfo>, database::Error> {
        self.inner.get_proofs_by_ys(ys).await
    }
    async fn get_transaction(
        &self,
        transaction_id: TransactionId,
    ) -> Result<Option<Transaction>, database::Error> {
        self.inner.get_transaction(transaction_id).await
    }
    async fn list_transactions(
        &self,
        mint_url: Option<MintUrl>,
        direction: Option<TransactionDirection>,
        unit: Option<CurrencyUnit>,
    ) -> Result<Vec<Transaction>, database::Error> {
        self.inner.list_transactions(mint_url, direction, unit).await
    }
    async fn add_transaction(
        &self,
        transaction: Transaction,
    ) -> Result<(), database::Error> {
        self.inner.add_transaction(transaction).await
    }
    async fn update_mint_url(
        &self,
        old_mint_url: MintUrl,
        new_mint_url: MintUrl,
    ) -> Result<(), database::Error> {
        self.inner.update_mint_url(old_mint_url, new_mint_url).await
    }
    async fn increment_keyset_counter(
        &self,
        keyset_id: &Id,
        count: u32,
    ) -> Result<u32, database::Error> {
        self.inner.increment_keyset_counter(keyset_id, count).await
    }
    async fn add_mint(
        &self,
        mint_url: MintUrl,
        mint_info: Option<MintInfo>,
    ) -> Result<(), database::Error> {
        self.inner.add_mint(mint_url, mint_info).await
    }
    async fn remove_mint(&self, mint_url: MintUrl) -> Result<(), database::Error> {
        self.inner.remove_mint(mint_url).await
    }
    async fn add_mint_keysets(
        &self,
        mint_url: MintUrl,
        keysets: Vec<KeySetInfo>,
    ) -> Result<(), database::Error> {
        self.inner.add_mint_keysets(mint_url, keysets).await
    }
    async fn add_mint_quote(&self, quote: MintQuote) -> Result<(), database::Error> {
        self.inner.add_mint_quote(quote).await
    }
    async fn remove_mint_quote(&self, quote_id: &str) -> Result<(), database::Error> {
        self.inner.remove_mint_quote(quote_id).await
    }
    async fn add_melt_quote(&self, quote: wallet::MeltQuote) -> Result<(), database::Error> {
        self.inner.add_melt_quote(quote).await
    }
    async fn remove_melt_quote(&self, quote_id: &str) -> Result<(), database::Error> {
        self.inner.remove_melt_quote(quote_id).await
    }
    async fn add_keys(&self, keyset: KeySet) -> Result<(), database::Error> {
        self.inner.add_keys(keyset).await
    }
    async fn remove_keys(&self, id: &Id) -> Result<(), database::Error> {
        self.inner.remove_keys(id).await
    }
    async fn remove_transaction(
        &self,
        transaction_id: TransactionId,
    ) -> Result<(), database::Error> {
        self.inner.remove_transaction(transaction_id).await
    }
    async fn add_saga(&self, saga: wallet::WalletSaga) -> Result<(), database::Error> {
        self.inner.add_saga(saga).await
    }
    async fn get_saga(
        &self,
        id: &uuid::Uuid,
    ) -> Result<Option<wallet::WalletSaga>, database::Error> {
        self.inner.get_saga(id).await
    }
    async fn update_saga(&self, saga: wallet::WalletSaga) -> Result<bool, database::Error> {
        self.inner.update_saga(saga).await
    }
    async fn delete_saga(&self, id: &uuid::Uuid) -> Result<(), database::Error> {
        self.inner.delete_saga(id).await
    }
    async fn get_incomplete_sagas(&self) -> Result<Vec<wallet::WalletSaga>, database::Error> {
        self.inner.get_incomplete_sagas().await
    }
    async fn get_reserved_proofs(
        &self,
        operation_id: &uuid::Uuid,
    ) -> Result<Vec<ProofInfo>, database::Error> {
        self.inner.get_reserved_proofs(operation_id).await
    }
    async fn reserve_melt_quote(
        &self,
        quote_id: &str,
        operation_id: &uuid::Uuid,
    ) -> Result<(), database::Error> {
        self.inner.reserve_melt_quote(quote_id, operation_id).await
    }
    async fn release_melt_quote(
        &self,
        operation_id: &uuid::Uuid,
    ) -> Result<(), database::Error> {
        self.inner.release_melt_quote(operation_id).await
    }
    async fn reserve_mint_quote(
        &self,
        quote_id: &str,
        operation_id: &uuid::Uuid,
    ) -> Result<(), database::Error> {
        self.inner.reserve_mint_quote(quote_id, operation_id).await
    }
    async fn release_mint_quote(
        &self,
        operation_id: &uuid::Uuid,
    ) -> Result<(), database::Error> {
        self.inner.release_mint_quote(operation_id).await
    }
    async fn kv_read(
        &self,
        primary_namespace: &str,
        secondary_namespace: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, database::Error> {
        self.inner
            .kv_read(primary_namespace, secondary_namespace, key)
            .await
    }
    async fn kv_list(
        &self,
        primary_namespace: &str,
        secondary_namespace: &str,
    ) -> Result<Vec<String>, database::Error> {
        self.inner
            .kv_list(primary_namespace, secondary_namespace)
            .await
    }
    async fn kv_write(
        &self,
        primary_namespace: &str,
        secondary_namespace: &str,
        key: &str,
        value: &[u8],
    ) -> Result<(), database::Error> {
        self.inner
            .kv_write(primary_namespace, secondary_namespace, key, value)
            .await
    }
    async fn kv_remove(
        &self,
        primary_namespace: &str,
        secondary_namespace: &str,
        key: &str,
    ) -> Result<(), database::Error> {
        self.inner
            .kv_remove(primary_namespace, secondary_namespace, key)
            .await
    }
    async fn add_p2pk_key(
        &self,
        pubkey: &PublicKey,
        derivation_path: DerivationPath,
        derivation_index: u32,
    ) -> Result<(), database::Error> {
        self.inner
            .add_p2pk_key(pubkey, derivation_path, derivation_index)
            .await
    }
    async fn get_p2pk_key(
        &self,
        pubkey: &PublicKey,
    ) -> Result<Option<wallet::P2PKSigningKey>, database::Error> {
        self.inner.get_p2pk_key(pubkey).await
    }
    async fn list_p2pk_keys(&self) -> Result<Vec<wallet::P2PKSigningKey>, database::Error> {
        self.inner.list_p2pk_keys().await
    }
    async fn latest_p2pk(&self) -> Result<Option<wallet::P2PKSigningKey>, database::Error> {
        self.inner.latest_p2pk().await
    }
}
