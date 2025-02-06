//! CDK Database

use std::collections::HashMap;

use async_trait::async_trait;
use cashu::MintInfo;
use uuid::Uuid;

use super::Error;
use crate::common::{LnKey, QuoteTTL};
use crate::mint::{self, MintKeySetInfo, MintQuote as MintMintQuote};
use crate::nuts::{
    BlindSignature, CurrencyUnit, Id, MeltBolt11Request, MeltQuoteState, MintQuoteState, Proof,
    Proofs, PublicKey, State,
};

/// Database Writer
///
/// This trait is the only way to update the database, in a atomic way, from the Rust side, making
/// sure that on commit all changes happen or none.
///
/// Every record read or updated by this Writer should be locked exclusively until the Writer is
/// consumed, either by commit or rollback.
///
/// On Drop, if unless commit() was called explicitly, the changes are expected to be rolled back.
#[async_trait]
pub trait Transaction: Send + Sync {
    /// Add [`MintMintQuote`]
    async fn add_mint_quote(&mut self, quote: MintMintQuote) -> Result<(), Error>;

    /// Get [`MintMintQuote`]
    ///
    /// While this Writer object is in scope the quote should be locked exclusively
    async fn get_mint_quote(&mut self, quote_id: &Uuid) -> Result<Option<MintMintQuote>, Error>;

    /// Get all [`MintMintQuote`]s
    async fn get_mint_quote_by_request(
        &self,
        request: &str,
    ) -> Result<Option<MintMintQuote>, Error>;

    /// Get all [`MintMintQuote`]s
    async fn get_mint_quote_by_request_lookup_id(
        &mut self,
        request_lookup_id: &str,
    ) -> Result<Option<MintMintQuote>, Error>;

    /// Update state of [`MintMintQuote`]
    async fn update_mint_quote_state(
        &mut self,
        quote_id: &Uuid,
        state: MintQuoteState,
    ) -> Result<MintQuoteState, Error>;

    /// Add  [`Proofs`]
    async fn add_proofs(&mut self, proof: Proofs, quote_id: Option<Uuid>) -> Result<(), Error>;

    /// Get [`Proofs`] state
    async fn update_proofs_states(
        &mut self,
        ys: &[PublicKey],
        proofs_state: State,
    ) -> Result<Vec<Option<State>>, Error>;

    /// Get [`BlindSignature`]s and lock them exclusively until the Writer is dropped
    async fn get_blind_signatures(
        &mut self,
        blinded_messages: &[PublicKey],
    ) -> Result<Vec<Option<BlindSignature>>, Error>;

    /// Add [`BlindSignature`]
    async fn add_blind_signatures(
        &mut self,
        blinded_messages: &[PublicKey],
        blind_signatures: &[BlindSignature],
        quote_id: Option<Uuid>,
    ) -> Result<(), Error>;

    /// Get melt request
    async fn get_melt_request(
        &mut self,
        quote_id: &Uuid,
    ) -> Result<Option<(MeltBolt11Request<Uuid>, LnKey)>, Error>;

    /// Get [`mint::MeltQuote`]
    ///
    /// While this Writer object is in scope the quote should be locked exclusively
    async fn get_melt_quote(&mut self, quote_id: &Uuid) -> Result<Option<mint::MeltQuote>, Error>;

    /// Update [`mint::MeltQuote`] state
    async fn update_melt_quote_state(
        &mut self,
        quote_id: &Uuid,
        state: MeltQuoteState,
    ) -> Result<MeltQuoteState, Error>;

    /// Consumes the Writer and commit the changes
    async fn commit(self: Box<Self>) -> Result<(), Error>;

    /// Consumes the Writer and rollback the changes
    async fn rollback(self: Box<Self>) -> Result<(), Error>;
}

/// Mint Database trait
#[async_trait]
pub trait Database {
    /// Mint Database Error
    type Err: Into<Error> + From<Error>;

    /// Get a Database Writer
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>, Self::Err>;

    /// Add Active Keyset
    async fn set_active_keyset(&self, unit: CurrencyUnit, id: Id) -> Result<(), Self::Err>;
    /// Get Active Keyset
    ///
    /// TODO: Refactor code to use `SignatoryManager` instead of the database
    async fn get_active_keyset_id(&self, unit: &CurrencyUnit) -> Result<Option<Id>, Self::Err>;
    /// Get all Active Keyset
    async fn get_active_keysets(&self) -> Result<HashMap<CurrencyUnit, Id>, Self::Err>;

    /// Get [`MintMintQuote`]
    async fn get_mint_quote(&self, quote_id: &Uuid) -> Result<Option<MintMintQuote>, Self::Err>;
    /// Get all [`MintMintQuote`]s
    async fn get_mint_quote_by_request(
        &self,
        request: &str,
    ) -> Result<Option<MintMintQuote>, Self::Err>;
    /// Get all [`MintMintQuote`]s
    async fn get_mint_quote_by_request_lookup_id(
        &self,
        request_lookup_id: &str,
    ) -> Result<Option<MintMintQuote>, Self::Err>;
    /// Get Mint Quotes
    async fn get_mint_quotes(&self) -> Result<Vec<MintMintQuote>, Self::Err>;

    /// Remove [`MintMintQuote`]
    async fn remove_mint_quote(&self, quote_id: &Uuid) -> Result<(), Self::Err>;

    /// Add [`mint::MeltQuote`]
    async fn add_melt_quote(&self, quote: mint::MeltQuote) -> Result<(), Self::Err>;
    /// Get [`mint::MeltQuote`]
    async fn get_melt_quote(&self, quote_id: &Uuid) -> Result<Option<mint::MeltQuote>, Self::Err>;
    /// Get all [`mint::MeltQuote`]s
    async fn get_melt_quotes(&self) -> Result<Vec<mint::MeltQuote>, Self::Err>;
    /// Remove [`mint::MeltQuote`]
    async fn remove_melt_quote(&self, quote_id: &Uuid) -> Result<(), Self::Err>;

    /// Add melt request
    async fn add_melt_request(
        &self,
        melt_request: MeltBolt11Request<Uuid>,
        ln_key: LnKey,
    ) -> Result<(), Self::Err>;
    /// Get melt request
    async fn get_melt_request(
        &self,
        quote_id: &Uuid,
    ) -> Result<Option<(MeltBolt11Request<Uuid>, LnKey)>, Self::Err>;

    /// Add [`MintKeySetInfo`]
    async fn add_keyset_info(&self, keyset: MintKeySetInfo) -> Result<(), Self::Err>;
    /// Get [`MintKeySetInfo`]
    /// TODO: Refactor code to use `SignatoryManager` instead of the database
    async fn get_keyset_info(&self, id: &Id) -> Result<Option<MintKeySetInfo>, Self::Err>;
    /// Get [`MintKeySetInfo`]s
    /// TODO: Refactor code to use `SignatoryManager` instead of the database
    async fn get_keyset_infos(&self) -> Result<Vec<MintKeySetInfo>, Self::Err>;

    /// Get [`Proofs`] by ys
    async fn get_proofs_by_ys(&self, ys: &[PublicKey]) -> Result<Vec<Option<Proof>>, Self::Err>;
    /// Get ys by quote id
    async fn get_proof_ys_by_quote_id(&self, quote_id: &Uuid) -> Result<Vec<PublicKey>, Self::Err>;
    /// Get [`Proofs`] state
    async fn get_proofs_states(&self, ys: &[PublicKey]) -> Result<Vec<Option<State>>, Self::Err>;
    /// Get [`Proofs`] by state
    async fn get_proofs_by_keyset_id(
        &self,
        keyset_id: &Id,
    ) -> Result<(Proofs, Vec<Option<State>>), Self::Err>;

    /// Get [`BlindSignature`]s
    async fn get_blind_signatures(
        &self,
        blinded_messages: &[PublicKey],
    ) -> Result<Vec<Option<BlindSignature>>, Self::Err>;
    /// Get [`BlindSignature`]s for keyset_id
    async fn get_blind_signatures_for_keyset(
        &self,
        keyset_id: &Id,
    ) -> Result<Vec<BlindSignature>, Self::Err>;
    /// Get [`BlindSignature`]s for quote
    async fn get_blind_signatures_for_quote(
        &self,
        quote_id: &Uuid,
    ) -> Result<Vec<BlindSignature>, Self::Err>;

    /// Set [`MintInfo`]
    async fn set_mint_info(&self, mint_info: MintInfo) -> Result<(), Self::Err>;
    /// Get [`MintInfo`]
    async fn get_mint_info(&self) -> Result<MintInfo, Self::Err>;

    /// Set [`QuoteTTL`]
    async fn set_quote_ttl(&self, quote_ttl: QuoteTTL) -> Result<(), Self::Err>;
    /// Get [`QuoteTTL`]
    async fn get_quote_ttl(&self) -> Result<QuoteTTL, Self::Err>;
}
