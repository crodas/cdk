//! Wallet trait implementations
//!
//! This module implements the wallet traits from `cdk_common::wallet::traits`
//! for the CDK Wallet struct.

use std::collections::HashMap;

use cdk_common::amount::SplitTarget;
use cdk_common::nut00::KnownMethod;
use cdk_common::nut04::MintMethodOptions;
use cdk_common::wallet::traits::{
    WalletBalance, WalletMelt, WalletMint, WalletMintInfo, WalletProofs, WalletReceive, WalletTypes,
};
use cdk_common::wallet::MintQuote;
use cdk_common::PaymentMethod;
use tracing::instrument;

use crate::mint_url::MintUrl;
use crate::nuts::nut00::ProofsMethods;
use crate::nuts::{
    CheckStateRequest, CurrencyUnit, KeySetInfo, MintInfo, MintQuoteBolt11Request, Proof, Proofs,
    SecretKey, State,
};
use crate::types::Melted;
use crate::util::unix_time;
use crate::wallet::{MeltQuote, ReceiveOptions};
use crate::{Amount, Error, Wallet};

impl WalletTypes for Wallet {
    type Amount = Amount;
    type Proofs = Proofs;
    type Proof = Proof;
    type MintQuote = MintQuote;
    type MeltQuote = MeltQuote;
    type Token = crate::nuts::Token;
    type CurrencyUnit = CurrencyUnit;
    type MintUrl = MintUrl;
    type MintInfo = MintInfo;
    type KeySetInfo = KeySetInfo;
    type Error = Error;

    fn mint_url(&self) -> Self::MintUrl {
        self.mint_url.clone()
    }

    fn unit(&self) -> Self::CurrencyUnit {
        self.unit.clone()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl WalletBalance for Wallet {
    #[instrument(skip(self))]
    async fn total_balance(&self) -> Result<Self::Amount, Self::Error> {
        // Use the efficient balance query instead of fetching all proofs
        let balance = self
            .localstore
            .get_balance(
                Some(self.mint_url.clone()),
                Some(self.unit.clone()),
                Some(vec![State::Unspent]),
            )
            .await?;
        Ok(Amount::from(balance))
    }

    #[instrument(skip(self))]
    async fn total_pending_balance(&self) -> Result<Self::Amount, Self::Error> {
        Ok(self.get_pending_proofs().await?.total_amount()?)
    }

    #[instrument(skip(self))]
    async fn total_reserved_balance(&self) -> Result<Self::Amount, Self::Error> {
        Ok(self.get_reserved_proofs().await?.total_amount()?)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl WalletMintInfo for Wallet {
    /// Query mint for current mint information
    #[instrument(skip(self))]
    async fn fetch_mint_info(&self) -> Result<Option<Self::MintInfo>, Self::Error> {
        let mint_info = self
            .metadata_cache
            .load_from_mint(&self.localstore, &self.client)
            .await?
            .mint_info
            .clone();

        // If mint provides time make sure it is accurate
        if let Some(mint_unix_time) = mint_info.time {
            let current_unix_time = unix_time();
            if current_unix_time.abs_diff(mint_unix_time) > 30 {
                tracing::warn!(
                    "Mint time does match wallet time. Mint: {}, Wallet: {}",
                    mint_unix_time,
                    current_unix_time
                );
                return Err(Error::MintTimeExceedsTolerance);
            }
        }

        // Create or update auth wallet
        #[cfg(feature = "auth")]
        {
            let mut auth_wallet = self.auth_wallet.write().await;
            match &*auth_wallet {
                Some(auth_wallet) => {
                    let mut protected_endpoints = auth_wallet.protected_endpoints.write().await;
                    *protected_endpoints = mint_info.protected_endpoints();

                    if let Some(oidc_client) = mint_info
                        .openid_discovery()
                        .map(|url| crate::OidcClient::new(url, None))
                    {
                        auth_wallet.set_oidc_client(Some(oidc_client)).await;
                    }
                }
                None => {
                    tracing::info!("Mint has auth enabled creating auth wallet");

                    let oidc_client = mint_info
                        .openid_discovery()
                        .map(|url| crate::OidcClient::new(url, None));
                    let new_auth_wallet = crate::wallet::AuthWallet::new(
                        self.mint_url.clone(),
                        None,
                        self.localstore.clone(),
                        self.metadata_cache.clone(),
                        mint_info.protected_endpoints(),
                        oidc_client,
                    );
                    *auth_wallet = Some(new_auth_wallet.clone());

                    self.client
                        .set_auth_wallet(Some(new_auth_wallet.clone()))
                        .await;

                    if let Err(e) = new_auth_wallet.refresh_keysets().await {
                        tracing::error!("Could not fetch auth keysets: {}", e);
                    }
                }
            }
        }

        tracing::trace!("Mint info updated for {}", self.mint_url);

        Ok(Some(mint_info))
    }

    /// Load mint info from cache
    #[instrument(skip(self))]
    async fn load_mint_info(&self) -> Result<Self::MintInfo, Self::Error> {
        let mint_info = self
            .metadata_cache
            .load(&self.localstore, &self.client, {
                let ttl = self.metadata_cache_ttl.read();
                *ttl
            })
            .await?
            .mint_info
            .clone();

        Ok(mint_info)
    }

    /// Get the active keyset with the lowest fees from cache
    #[instrument(skip(self))]
    async fn get_active_keyset(&self) -> Result<Self::KeySetInfo, Self::Error> {
        self.metadata_cache
            .load(&self.localstore, &self.client, {
                let ttl = self.metadata_cache_ttl.read();
                *ttl
            })
            .await?
            .active_keysets
            .iter()
            .min_by_key(|k| k.input_fee_ppk)
            .map(|ks| (**ks).clone())
            .ok_or(Error::NoActiveKeyset)
    }

    /// Refresh keysets by fetching the latest from mint
    #[instrument(skip(self))]
    async fn refresh_keysets(&self) -> Result<Vec<Self::KeySetInfo>, Self::Error> {
        tracing::debug!("Refreshing keysets from mint");

        let keysets = self
            .metadata_cache
            .load_from_mint(&self.localstore, &self.client)
            .await?
            .keysets
            .iter()
            .filter_map(|(_, keyset)| {
                if keyset.unit == self.unit && keyset.active {
                    Some((*keyset.clone()).clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if !keysets.is_empty() {
            Ok(keysets)
        } else {
            Err(Error::UnknownKeySet)
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl WalletMint for Wallet {
    #[instrument(skip(self))]
    async fn mint_quote(
        &self,
        amount: Self::Amount,
        description: Option<String>,
    ) -> Result<Self::MintQuote, Self::Error> {
        let mint_info = self.load_mint_info().await?;

        let mint_url = self.mint_url.clone();
        let unit = self.unit.clone();

        // If we have a description, we check that the mint supports it.
        if description.is_some() {
            let settings = mint_info
                .nuts
                .nut04
                .get_settings(
                    &unit,
                    &crate::nuts::PaymentMethod::Known(KnownMethod::Bolt11),
                )
                .ok_or(Error::UnsupportedUnit)?;

            match settings.options {
                Some(MintMethodOptions::Bolt11 { description }) if description => (),
                _ => return Err(Error::InvoiceDescriptionUnsupported),
            }
        }

        let secret_key = SecretKey::generate();

        let request = MintQuoteBolt11Request {
            amount,
            unit: unit.clone(),
            description,
            pubkey: Some(secret_key.public_key()),
        };

        let quote_res = self.client.post_mint_quote(request).await?;

        let quote = MintQuote::new(
            quote_res.quote,
            mint_url,
            PaymentMethod::Known(KnownMethod::Bolt11),
            Some(amount),
            unit,
            quote_res.request,
            quote_res.expiry.unwrap_or(0),
            Some(secret_key),
        );

        self.localstore.add_mint_quote(quote.clone()).await?;

        Ok(quote)
    }

    async fn mint(&self, quote_id: &str) -> Result<Self::Proofs, Self::Error> {
        self.mint(quote_id, SplitTarget::default(), None).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl WalletMelt for Wallet {
    type MeltResult = Melted;

    async fn melt_quote(&self, request: String) -> Result<Self::MeltQuote, Self::Error> {
        self.melt_quote(request, None).await
    }

    /// Melt
    #[instrument(skip(self))]
    async fn melt(&self, quote_id: &str) -> Result<Self::MeltResult, Self::Error> {
        self.melt_with_metadata(quote_id, HashMap::new()).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl WalletReceive for Wallet {
    async fn receive(&self, encoded_token: &str) -> Result<Self::Amount, Self::Error> {
        self.receive(encoded_token, ReceiveOptions::default()).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl WalletProofs for Wallet {
    async fn check_proofs_spent(&self, proofs: Self::Proofs) -> Result<Vec<bool>, Self::Error> {
        let proof_states = self.check_proofs_spent(proofs).await?;
        Ok(proof_states
            .into_iter()
            .map(|ps| matches!(ps.state, State::Spent | State::PendingSpent))
            .collect())
    }

    /// Reclaim unspent proofs
    ///
    /// Checks the stats of [`Proofs`] swapping for a new [`Proof`] if unspent
    #[instrument(skip(self, proofs))]
    async fn reclaim_unspent(&self, proofs: Self::Proofs) -> Result<(), Self::Error> {
        use cdk_common::wallet::TransactionId;

        let proof_ys = proofs.ys()?;

        let transaction_id = TransactionId::new(proof_ys.clone());

        let spendable = self
            .client
            .post_check_state(CheckStateRequest { ys: proof_ys })
            .await?
            .states;

        let unspent: Proofs = proofs
            .into_iter()
            .zip(spendable)
            .filter_map(|(p, s)| (s.state == State::Unspent).then_some(p))
            .collect();

        self.swap(None, SplitTarget::default(), unspent, None, false)
            .await?;

        let _ = self
            .localstore
            .remove_transaction(transaction_id)
            .await
            .inspect_err(|err| {
                tracing::warn!("Failed to remove transaction: {:?}", err);
            });

        Ok(())
    }
}
