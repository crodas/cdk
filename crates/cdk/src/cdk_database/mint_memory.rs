//! Mint in memory database

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use cdk_common::common::QuoteTTL;
use cdk_common::database::{Error, MintDatabase, MintTransaction};
use cdk_common::mint::MintKeySetInfo;
use cdk_common::nut00::ProofsMethods;
use cdk_common::MintInfo;
use tokio::sync::RwLock;
use tokio::time::sleep;
use uuid::Uuid;

use crate::dhke::hash_to_curve;
use crate::mint::{self, MintQuote};
use crate::nuts::nut07::State;
use crate::nuts::{
    nut07, BlindSignature, CurrencyUnit, Id, MeltBolt11Request, MeltQuoteState, MintQuoteState,
    Proof, Proofs, PublicKey,
};
use crate::types::LnKey;

/// Macro to merge two `Arc<RwLock<HashMap<K, V>>>` where `map2` is drained into `map1`
macro_rules! merge {
    ($map1:expr, $map2:expr) => {{
        let mut map1_lock = $map1.write().await;
        let mut map2_lock = $map2.write().await;

        for (k, v) in map2_lock.drain() {
            map1_lock.insert(k, v);
        }
    }};
}

#[derive(Debug, Default)]
#[allow(clippy::type_complexity)]
struct MemoryStorage {
    active_keysets: RwLock<HashMap<CurrencyUnit, Id>>,
    keysets: RwLock<HashMap<Id, MintKeySetInfo>>,
    mint_quotes: RwLock<HashMap<Uuid, MintQuote>>,
    melt_quotes: RwLock<HashMap<Uuid, mint::MeltQuote>>,
    proofs: RwLock<HashMap<[u8; 33], Proof>>,
    proof_state: RwLock<HashMap<[u8; 33], nut07::State>>,
    quote_proofs: RwLock<HashMap<Uuid, Vec<PublicKey>>>,
    blinded_signatures: RwLock<HashMap<[u8; 33], BlindSignature>>,
    quote_signatures: RwLock<HashMap<Uuid, Vec<BlindSignature>>>,
    melt_requests: RwLock<HashMap<Uuid, (MeltBolt11Request<Uuid>, LnKey)>>,
    mint_info: RwLock<MintInfo>,
    quote_ttl: RwLock<QuoteTTL>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AnyId {
    MintQuote(Uuid),
    MeltQuote(Uuid),
    BlindSignature(PublicKey),
}

/// Poor man's concurrent access manager
#[derive(Debug, Default)]
struct AccessManager(RwLock<HashMap<AnyId, u64>>);

impl AccessManager {
    /// Lock a resource for exclusive access
    ///
    /// If the resource is already locked, it will wait until it is unlocked. Since this
    /// implementation is mainly for testing, it is not optimized for performance. In a real-world
    /// scenario, a more sophisticated releasing mechanism should be used to avoid CPU overhead.
    pub async fn lock(&self, resource_id: AnyId, writer_id: u64) {
        loop {
            let mut write = self.0.write().await;
            match write.get(&resource_id) {
                Some(lock_writer_id) if *lock_writer_id == writer_id => break,
                None => {
                    write.insert(resource_id.clone(), writer_id);
                    break;
                }
                _ => {}
            }
            drop(write);
            sleep(Duration::from_nanos(10)).await;
        }
    }

    /// Access a resource for reading, if it is locked, it will wait until it is unlocked.
    ///
    /// Since this implementation is mainly for testing, it will not add a read-lock to the
    /// resource. In a real-world scenario an Read-Write lock should be used.
    pub async fn access(&self, resource_id: AnyId) {
        loop {
            let read = self.0.read().await;
            let lock_reader_id = read.get(&resource_id).cloned();
            if lock_reader_id.is_none() {
                break;
            }
            drop(read);
            sleep(Duration::from_nanos(10)).await;
        }
    }

    pub async fn release(&self, writer_id: u64) {
        let mut write = self.0.write().await;
        write.retain(|_, v| *v != writer_id);
    }
}

/// Mint Memory Database
#[derive(Debug, Clone, Default)]
pub struct MintMemoryDatabase {
    /// Storage
    inner: Arc<MemoryStorage>,
    /// Exclusive access list, where transaction can lock Ids for exclusive access
    /// until they either commit or rollback
    exclusive_access_manager: Arc<AccessManager>,
    writer_index: Arc<AtomicU64>,
}

/// Writer for the [`MintMemoryDatabase`]
pub struct MintMemoryWriter {
    exclusive_access_manager: Arc<AccessManager>,
    inner: Arc<MemoryStorage>,
    changes: MemoryStorage,
    id: u64,
}

#[async_trait]
impl MintTransaction for MintMemoryWriter {
    async fn get_mint_quote(&mut self, quote_id: &Uuid) -> Result<Option<MintQuote>, Error> {
        self.exclusive_access_manager
            .lock(AnyId::MintQuote(quote_id.to_owned()), self.id)
            .await;

        if let Some(quote) = self.changes.mint_quotes.read().await.get(quote_id) {
            return Ok(Some(quote.clone()));
        }

        Ok(self.inner.mint_quotes.read().await.get(quote_id).cloned())
    }

    async fn add_mint_quote(&mut self, quote: MintQuote) -> Result<(), Error> {
        self.exclusive_access_manager
            .lock(AnyId::MintQuote(quote.id.clone()), self.id)
            .await;
        self.changes
            .mint_quotes
            .write()
            .await
            .insert(quote.id, quote);
        Ok(())
    }

    async fn get_melt_request(
        &mut self,
        quote_id: &Uuid,
    ) -> Result<Option<(MeltBolt11Request<Uuid>, LnKey)>, Error> {
        let melt_requests = self.inner.melt_requests.read().await;
        let melt_request = melt_requests.get(quote_id).cloned();

        if let Some((request, _)) = &melt_request {
            self.exclusive_access_manager
                .lock(AnyId::MeltQuote(request.quote), self.id)
                .await;
        }

        Ok(melt_request)
    }

    async fn get_blind_signatures(
        &mut self,
        blinded_messages: &[PublicKey],
    ) -> Result<Vec<Option<BlindSignature>>, Error> {
        let mut signatures = Vec::with_capacity(blinded_messages.len());

        let blinded_signatures = self.inner.blinded_signatures.read().await;

        for blinded_message in blinded_messages {
            let signature = blinded_signatures.get(&blinded_message.to_bytes()).cloned();

            self.exclusive_access_manager
                .lock(AnyId::BlindSignature(*blinded_message), self.id)
                .await;

            signatures.push(signature)
        }

        Ok(signatures)
    }

    async fn get_mint_quote_by_request_lookup_id(
        &mut self,
        request: &str,
    ) -> Result<Option<MintQuote>, Error> {
        let result = self
            .inner
            .mint_quotes
            .read()
            .await
            .values()
            .filter(|q| q.request_lookup_id.eq(request))
            .next()
            .cloned();

        if let Some(quote) = &result {
            self.exclusive_access_manager
                .lock(AnyId::MintQuote(quote.id), self.id)
                .await;
        }

        Ok(result)
    }

    async fn get_mint_quote_by_request(&self, request: &str) -> Result<Option<MintQuote>, Error> {
        let result = self
            .inner
            .mint_quotes
            .read()
            .await
            .values()
            .filter(|q| q.request.eq(request))
            .next()
            .cloned();

        if let Some(quote) = &result {
            self.exclusive_access_manager
                .lock(AnyId::MintQuote(quote.id), self.id)
                .await;
        }

        Ok(result)
    }

    async fn update_mint_quote_state(
        &mut self,
        quote_id: &Uuid,
        state: MintQuoteState,
    ) -> Result<MintQuoteState, Error> {
        let mut quote = self
            .get_mint_quote(quote_id)
            .await?
            .ok_or(Error::UnknownQuote)?;

        let current_state = quote.state;
        quote.state = state;

        self.changes
            .mint_quotes
            .write()
            .await
            .insert(*quote_id, quote.clone());

        Ok(current_state)
    }

    async fn add_blind_signatures(
        &mut self,
        blinded_message: &[PublicKey],
        blind_signatures: &[BlindSignature],
        quote_id: Option<Uuid>,
    ) -> Result<(), Error> {
        let mut current_blinded_signatures = self.changes.blinded_signatures.write().await;

        for (blinded_message, blind_signature) in blinded_message.iter().zip(blind_signatures) {
            current_blinded_signatures.insert(blinded_message.to_bytes(), blind_signature.clone());
        }

        if let Some(quote_id) = quote_id {
            let mut current_quote_signatures = self.inner.quote_signatures.write().await;
            current_quote_signatures.insert(quote_id, blind_signatures.to_vec());
        }

        Ok(())
    }

    async fn add_proofs(&mut self, proofs: Proofs, quote_id: Option<Uuid>) -> Result<(), Error> {
        let mut db_proofs = self.inner.proofs.write().await;

        let mut ys = Vec::with_capacity(proofs.capacity());

        for proof in proofs {
            let y = hash_to_curve(&proof.secret.to_bytes())?;
            ys.push(y);

            let y = y.to_bytes();

            db_proofs.insert(y, proof);
        }

        if let Some(quote_id) = quote_id {
            let mut db_quote_proofs = self.inner.quote_proofs.write().await;

            db_quote_proofs.insert(quote_id, ys);
        }

        Ok(())
    }

    async fn update_melt_quote_state(
        &mut self,
        quote_id: &Uuid,
        state: MeltQuoteState,
    ) -> Result<MeltQuoteState, Error> {
        let mut melt_quotes = self.inner.melt_quotes.write().await;

        let mut quote = melt_quotes
            .get(quote_id)
            .cloned()
            .ok_or(Error::UnknownQuote)?;

        let current_state = quote.state;

        quote.state = state;

        melt_quotes.insert(*quote_id, quote.clone());

        Ok(current_state)
    }

    async fn get_melt_quote(&mut self, quote_id: &Uuid) -> Result<Option<mint::MeltQuote>, Error> {
        let melt_quote = self.inner.melt_quotes.read().await.get(quote_id).cloned();
        if let Some(quote) = &melt_quote {
            self.exclusive_access_manager
                .lock(AnyId::MeltQuote(quote.id), self.id)
                .await;
        }
        Ok(melt_quote)
    }

    async fn update_proofs_states(
        &mut self,
        ys: &[PublicKey],
        proof_state: State,
    ) -> Result<Vec<Option<State>>, Error> {
        let mut proofs_states = self.inner.proof_state.write().await;

        let mut states = Vec::new();

        for y in ys {
            let state = proofs_states.insert(y.to_bytes(), proof_state);
            states.push(state);
        }

        Ok(states)
    }

    /// Consumes the Writer and commit the changes
    async fn commit(mut self: Box<Self>) -> Result<(), Error> {
        merge!(self.inner.keysets, self.changes.keysets);
        merge!(self.inner.mint_quotes, self.changes.mint_quotes);
        merge!(self.inner.melt_quotes, self.changes.melt_quotes);
        merge!(self.inner.proofs, self.changes.proofs);
        merge!(
            self.inner.blinded_signatures,
            self.changes.blinded_signatures
        );
        merge!(self.inner.quote_proofs, self.changes.quote_proofs);
        merge!(self.inner.quote_signatures, self.changes.quote_signatures);
        merge!(self.inner.melt_requests, self.changes.melt_requests);

        self.exclusive_access_manager.release(self.id).await;
        todo!()
    }

    /// Consumes the Writer and rollback the changes
    async fn rollback(self: Box<Self>) -> Result<(), Error> {
        self.exclusive_access_manager.release(self.id).await;
        Ok(())
    }
}

impl MintMemoryDatabase {
    /// Create new [`MintMemoryDatabase`]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        active_keysets: HashMap<CurrencyUnit, Id>,
        keysets: Vec<MintKeySetInfo>,
        mint_quotes: Vec<MintQuote>,
        melt_quotes: Vec<mint::MeltQuote>,
        pending_proofs: Proofs,
        spent_proofs: Proofs,
        quote_proofs: HashMap<Uuid, Vec<PublicKey>>,
        blinded_signatures: HashMap<[u8; 33], BlindSignature>,
        quote_signatures: HashMap<Uuid, Vec<BlindSignature>>,
        melt_request: Vec<(MeltBolt11Request<Uuid>, LnKey)>,
        mint_info: MintInfo,
        quote_ttl: QuoteTTL,
    ) -> Result<Self, Error> {
        let mut proofs = HashMap::new();
        let mut proof_states = HashMap::new();

        for proof in pending_proofs {
            let y = hash_to_curve(&proof.secret.to_bytes())?.to_bytes();
            proofs.insert(y, proof);
            proof_states.insert(y, State::Pending);
        }

        for proof in spent_proofs {
            let y = hash_to_curve(&proof.secret.to_bytes())?.to_bytes();
            proofs.insert(y, proof);
            proof_states.insert(y, State::Spent);
        }

        let melt_requests = melt_request
            .into_iter()
            .map(|(request, ln_key)| (request.quote, (request, ln_key)))
            .collect();

        Ok(Self {
            writer_index: Arc::new(0.into()),
            exclusive_access_manager: Arc::new(AccessManager::default()),
            inner: Arc::new(MemoryStorage {
                active_keysets: RwLock::new(active_keysets),
                keysets: RwLock::new(keysets.into_iter().map(|k| (k.id, k)).collect()),
                mint_quotes: RwLock::new(mint_quotes.into_iter().map(|q| (q.id, q)).collect()),
                melt_quotes: RwLock::new(melt_quotes.into_iter().map(|q| (q.id, q)).collect()),
                proofs: RwLock::new(proofs),
                proof_state: RwLock::new(proof_states),
                blinded_signatures: RwLock::new(blinded_signatures),
                quote_proofs: RwLock::new(quote_proofs),
                quote_signatures: RwLock::new(quote_signatures),
                melt_requests: RwLock::new(melt_requests),
                mint_info: RwLock::new(mint_info),
                quote_ttl: RwLock::new(quote_ttl),
            }),
        })
    }
}

#[async_trait]
impl MintDatabase for MintMemoryDatabase {
    type Err = Error;

    async fn begin_transaction(&self) -> Result<Box<dyn MintTransaction>, Self::Err> {
        Ok(Box::new(MintMemoryWriter {
            inner: self.inner.clone(),
            exclusive_access_manager: self.exclusive_access_manager.clone(),
            changes: MemoryStorage::default(),
            id: self
                .writer_index
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }))
    }

    async fn set_active_keyset(&self, unit: CurrencyUnit, id: Id) -> Result<(), Self::Err> {
        self.inner.active_keysets.write().await.insert(unit, id);
        Ok(())
    }

    async fn get_active_keyset_id(&self, unit: &CurrencyUnit) -> Result<Option<Id>, Self::Err> {
        Ok(self.inner.active_keysets.read().await.get(unit).cloned())
    }

    async fn get_active_keysets(&self) -> Result<HashMap<CurrencyUnit, Id>, Self::Err> {
        Ok(self.inner.active_keysets.read().await.clone())
    }

    async fn add_keyset_info(&self, keyset: MintKeySetInfo) -> Result<(), Self::Err> {
        self.inner.keysets.write().await.insert(keyset.id, keyset);
        Ok(())
    }

    async fn get_keyset_info(&self, keyset_id: &Id) -> Result<Option<MintKeySetInfo>, Self::Err> {
        Ok(self.inner.keysets.read().await.get(keyset_id).cloned())
    }

    async fn get_keyset_infos(&self) -> Result<Vec<MintKeySetInfo>, Self::Err> {
        Ok(self.inner.keysets.read().await.values().cloned().collect())
    }

    async fn get_mint_quote(&self, quote_id: &Uuid) -> Result<Option<MintQuote>, Self::Err> {
        self.exclusive_access_manager
            .access(AnyId::MintQuote(quote_id.to_owned()))
            .await;
        Ok(self.inner.mint_quotes.read().await.get(quote_id).cloned())
    }

    async fn get_mint_quote_by_request_lookup_id(
        &self,
        request: &str,
    ) -> Result<Option<MintQuote>, Self::Err> {
        let quotes = self.get_mint_quotes().await?;

        let quote = quotes
            .into_iter()
            .filter(|q| q.request_lookup_id.eq(request))
            .collect::<Vec<MintQuote>>()
            .first()
            .cloned();

        Ok(quote)
    }
    async fn get_mint_quote_by_request(
        &self,
        request: &str,
    ) -> Result<Option<MintQuote>, Self::Err> {
        let quotes = self.get_mint_quotes().await?;

        let quote = quotes
            .into_iter()
            .filter(|q| q.request.eq(request))
            .collect::<Vec<MintQuote>>()
            .first()
            .cloned();

        Ok(quote)
    }

    async fn get_mint_quotes(&self) -> Result<Vec<MintQuote>, Self::Err> {
        Ok(self
            .inner
            .mint_quotes
            .read()
            .await
            .values()
            .cloned()
            .collect())
    }

    async fn remove_mint_quote(&self, quote_id: &Uuid) -> Result<(), Self::Err> {
        self.inner.mint_quotes.write().await.remove(quote_id);

        Ok(())
    }

    async fn add_melt_quote(&self, quote: mint::MeltQuote) -> Result<(), Self::Err> {
        self.inner.melt_quotes.write().await.insert(quote.id, quote);
        Ok(())
    }

    async fn get_melt_quote(&self, quote_id: &Uuid) -> Result<Option<mint::MeltQuote>, Self::Err> {
        Ok(self.inner.melt_quotes.read().await.get(quote_id).cloned())
    }

    async fn get_melt_quotes(&self) -> Result<Vec<mint::MeltQuote>, Self::Err> {
        Ok(self
            .inner
            .melt_quotes
            .read()
            .await
            .values()
            .cloned()
            .collect())
    }

    async fn remove_melt_quote(&self, quote_id: &Uuid) -> Result<(), Self::Err> {
        self.inner.melt_quotes.write().await.remove(quote_id);

        Ok(())
    }

    async fn add_melt_request(
        &self,
        melt_request: MeltBolt11Request<Uuid>,
        ln_key: LnKey,
    ) -> Result<(), Self::Err> {
        let mut melt_requests = self.inner.melt_requests.write().await;
        melt_requests.insert(melt_request.quote, (melt_request, ln_key));
        Ok(())
    }

    async fn get_melt_request(
        &self,
        quote_id: &Uuid,
    ) -> Result<Option<(MeltBolt11Request<Uuid>, LnKey)>, Self::Err> {
        let melt_requests = self.inner.melt_requests.read().await;

        let melt_request = melt_requests.get(quote_id);

        Ok(melt_request.cloned())
    }

    async fn get_proofs_by_ys(&self, ys: &[PublicKey]) -> Result<Vec<Option<Proof>>, Self::Err> {
        let spent_proofs = self.inner.proofs.read().await;

        let mut proofs = Vec::with_capacity(ys.len());

        for y in ys {
            let proof = spent_proofs.get(&y.to_bytes()).cloned();

            proofs.push(proof);
        }

        Ok(proofs)
    }

    async fn get_proof_ys_by_quote_id(&self, quote_id: &Uuid) -> Result<Vec<PublicKey>, Self::Err> {
        let quote_proofs = &self.inner.quote_proofs.write().await;

        match quote_proofs.get(quote_id) {
            Some(ys) => Ok(ys.clone()),
            None => Ok(vec![]),
        }
    }

    async fn get_proofs_states(&self, ys: &[PublicKey]) -> Result<Vec<Option<State>>, Self::Err> {
        let proofs_states = self.inner.proof_state.write().await;

        let mut states = Vec::new();

        for y in ys {
            let state = proofs_states.get(&y.to_bytes()).cloned();
            states.push(state);
        }

        Ok(states)
    }

    async fn get_proofs_by_keyset_id(
        &self,
        keyset_id: &Id,
    ) -> Result<(Proofs, Vec<Option<State>>), Self::Err> {
        let proofs = self.inner.proofs.read().await;

        let proofs_for_id: Proofs = proofs
            .iter()
            .filter_map(|(_, p)| match &p.keyset_id == keyset_id {
                true => Some(p),
                false => None,
            })
            .cloned()
            .collect();

        let proof_ys = proofs_for_id.ys()?;

        assert_eq!(proofs_for_id.len(), proof_ys.len());

        let states = self.get_proofs_states(&proof_ys).await?;

        Ok((proofs_for_id, states))
    }

    async fn get_blind_signatures(
        &self,
        blinded_messages: &[PublicKey],
    ) -> Result<Vec<Option<BlindSignature>>, Self::Err> {
        let mut signatures = Vec::with_capacity(blinded_messages.len());

        let blinded_signatures = self.inner.blinded_signatures.read().await;

        for blinded_message in blinded_messages {
            let signature = blinded_signatures.get(&blinded_message.to_bytes()).cloned();

            self.exclusive_access_manager
                .access(AnyId::BlindSignature(*blinded_message))
                .await;

            signatures.push(signature)
        }

        Ok(signatures)
    }

    async fn get_blind_signatures_for_keyset(
        &self,
        keyset_id: &Id,
    ) -> Result<Vec<BlindSignature>, Self::Err> {
        let blinded_signatures = self.inner.blinded_signatures.read().await;

        Ok(blinded_signatures
            .values()
            .filter(|b| &b.keyset_id == keyset_id)
            .cloned()
            .collect())
    }

    /// Get [`BlindSignature`]s for quote
    async fn get_blind_signatures_for_quote(
        &self,
        quote_id: &Uuid,
    ) -> Result<Vec<BlindSignature>, Self::Err> {
        let ys = self.inner.quote_signatures.read().await;

        Ok(ys.get(quote_id).cloned().unwrap_or_default())
    }

    async fn set_mint_info(&self, mint_info: MintInfo) -> Result<(), Self::Err> {
        let mut current_mint_info = self.inner.mint_info.write().await;

        *current_mint_info = mint_info;

        Ok(())
    }
    async fn get_mint_info(&self) -> Result<MintInfo, Self::Err> {
        let mint_info = self.inner.mint_info.read().await;

        Ok(mint_info.clone())
    }

    async fn set_quote_ttl(&self, quote_ttl: QuoteTTL) -> Result<(), Self::Err> {
        let mut current_quote_ttl = self.inner.quote_ttl.write().await;

        *current_quote_ttl = quote_ttl;

        Ok(())
    }
    async fn get_quote_ttl(&self) -> Result<QuoteTTL, Self::Err> {
        let quote_ttl = self.inner.quote_ttl.read().await;

        Ok(*quote_ttl)
    }
}
