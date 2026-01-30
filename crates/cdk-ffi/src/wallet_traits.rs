//! Wallet trait implementations for FFI Wallet
//!
//! This module implements the wallet traits from `cdk_common::wallet::traits`
//! for the FFI Wallet struct. All trait methods call through to the inner
//! CdkWallet directly (with type conversion). The `#[uniffi::export]` inherent
//! methods in `wallet.rs` delegate to these trait impls.

use cdk::wallet::{
    WalletBalance as CdkWalletBalance, WalletMelt as CdkWalletMelt, WalletMint as CdkWalletMint,
    WalletMintInfo as CdkWalletMintInfo,
};
use cdk_common::wallet::traits::{
    WalletBalance, WalletMelt, WalletMint, WalletMintInfo, WalletProofs, WalletReceive, WalletTypes,
};

use crate::error::FfiError;
use crate::token::Token;
use crate::types::{
    Amount, CurrencyUnit, KeySetInfo, MeltQuote, Melted, MintInfo, MintQuote, MintUrl, Proof,
    Proofs,
};
use crate::wallet::Wallet;

impl WalletTypes for Wallet {
    type Amount = Amount;
    type Proofs = Proofs;
    type Proof = Proof;
    type MintQuote = MintQuote;
    type MeltQuote = MeltQuote;
    type Token = Token;
    type CurrencyUnit = CurrencyUnit;
    type MintUrl = MintUrl;
    type MintInfo = MintInfo;
    type KeySetInfo = KeySetInfo;
    type Error = FfiError;

    fn mint_url(&self) -> Self::MintUrl {
        self.inner().mint_url.clone().into()
    }

    fn unit(&self) -> Self::CurrencyUnit {
        self.inner().unit.clone().into()
    }
}

#[async_trait::async_trait]
impl WalletBalance for Wallet {
    async fn total_balance(&self) -> Result<Self::Amount, Self::Error> {
        Ok(CdkWalletBalance::total_balance(self.inner().as_ref())
            .await?
            .into())
    }

    async fn total_pending_balance(&self) -> Result<Self::Amount, Self::Error> {
        Ok(
            CdkWalletBalance::total_pending_balance(self.inner().as_ref())
                .await?
                .into(),
        )
    }

    async fn total_reserved_balance(&self) -> Result<Self::Amount, Self::Error> {
        Ok(
            CdkWalletBalance::total_reserved_balance(self.inner().as_ref())
                .await?
                .into(),
        )
    }
}

#[async_trait::async_trait]
impl WalletMintInfo for Wallet {
    async fn fetch_mint_info(&self) -> Result<Option<Self::MintInfo>, Self::Error> {
        Ok(CdkWalletMintInfo::fetch_mint_info(self.inner().as_ref())
            .await?
            .map(Into::into))
    }

    async fn load_mint_info(&self) -> Result<Self::MintInfo, Self::Error> {
        Ok(CdkWalletMintInfo::load_mint_info(self.inner().as_ref())
            .await?
            .into())
    }

    async fn get_active_keyset(&self) -> Result<Self::KeySetInfo, Self::Error> {
        Ok(CdkWalletMintInfo::get_active_keyset(self.inner().as_ref())
            .await?
            .into())
    }

    async fn refresh_keysets(&self) -> Result<Vec<Self::KeySetInfo>, Self::Error> {
        Ok(CdkWalletMintInfo::refresh_keysets(self.inner().as_ref())
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}

#[async_trait::async_trait]
impl WalletMint for Wallet {
    async fn mint_quote(
        &self,
        amount: Self::Amount,
        description: Option<String>,
    ) -> Result<Self::MintQuote, Self::Error> {
        Ok(
            CdkWalletMint::mint_quote(self.inner().as_ref(), amount.into(), description)
                .await?
                .into(),
        )
    }

    async fn mint(&self, quote_id: &str) -> Result<Self::Proofs, Self::Error> {
        let proofs = self
            .inner()
            .mint(quote_id, Default::default(), None)
            .await?;
        Ok(proofs.into_iter().map(|p| p.into()).collect())
    }
}

#[async_trait::async_trait]
impl WalletMelt for Wallet {
    type MeltResult = Melted;

    async fn melt_quote(&self, request: String) -> Result<Self::MeltQuote, Self::Error> {
        Ok(self.inner().melt_quote(request, None).await?.into())
    }

    async fn melt(&self, quote_id: &str) -> Result<Self::MeltResult, Self::Error> {
        Ok(CdkWalletMelt::melt(self.inner().as_ref(), quote_id)
            .await?
            .into())
    }
}

#[async_trait::async_trait]
impl WalletReceive for Wallet {
    async fn receive(&self, encoded_token: &str) -> Result<Self::Amount, Self::Error> {
        Ok(self
            .inner()
            .receive(encoded_token, cdk::wallet::ReceiveOptions::default())
            .await?
            .into())
    }
}

#[async_trait::async_trait]
impl WalletProofs for Wallet {
    async fn check_proofs_spent(&self, proofs: Self::Proofs) -> Result<Vec<bool>, Self::Error> {
        let cdk_proofs: Result<Vec<cdk::nuts::Proof>, _> =
            proofs.into_iter().map(|p| p.try_into()).collect();
        let cdk_proofs = cdk_proofs?;

        let proof_states = self.inner().check_proofs_spent(cdk_proofs).await?;
        Ok(proof_states
            .into_iter()
            .map(|proof_state| {
                matches!(
                    proof_state.state,
                    cdk::nuts::State::Spent | cdk::nuts::State::PendingSpent
                )
            })
            .collect())
    }

    async fn reclaim_unspent(&self, proofs: Self::Proofs) -> Result<(), Self::Error> {
        let cdk_proofs: Result<Vec<cdk::nuts::Proof>, _> =
            proofs.iter().map(|p| p.clone().try_into()).collect();
        let cdk_proofs = cdk_proofs?;
        self.inner().reclaim_unspent(cdk_proofs).await?;
        Ok(())
    }
}
