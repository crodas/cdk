mod auth_connector;
mod auth_provider;
mod auth_wallet;

use std::collections::HashMap;
use std::sync::Arc;

pub use auth_connector::AuthMintConnector;
pub use auth_provider::AuthTokenProvider;
pub use auth_wallet::AuthWallet;
use cdk_common::{Amount, AuthProof, AuthToken, Proofs};
use tracing::instrument;

use super::Wallet;
use crate::error::Error;

impl Wallet {
    /// Mint blind auth tokens
    #[instrument(skip_all)]
    pub async fn mint_blind_auth(&self, amount: Amount) -> Result<Proofs, Error> {
        self.auth_wallet
            .read()
            .await
            .as_ref()
            .ok_or(Error::AuthSettingsUndefined)?
            .mint_blind_auth(amount)
            .await
    }

    /// Get unspent auth proofs
    #[instrument(skip_all)]
    pub async fn get_unspent_auth_proofs(&self) -> Result<Vec<AuthProof>, Error> {
        self.auth_wallet
            .read()
            .await
            .as_ref()
            .ok_or(Error::AuthSettingsUndefined)?
            .get_unspent_auth_proofs()
            .await
    }

    /// Total balance of unspent blind auth proofs
    #[instrument(skip_all)]
    pub async fn total_blind_auth_balance(&self) -> Result<Amount, Error> {
        self.auth_wallet
            .read()
            .await
            .as_ref()
            .ok_or(Error::AuthSettingsUndefined)?
            .total_blind_auth_balance()
            .await
    }

    /// Set Clear Auth Token (CAT) for authentication
    ///
    /// The auth wallet is created on demand when the mint has not been queried
    /// yet, so the token takes effect whatever the call order.
    #[instrument(skip_all)]
    pub async fn set_cat(&self, cat: String) -> Result<(), Error> {
        let token = AuthToken::ClearAuth(cat);
        let mut auth_wallet = self.auth_wallet.write().await;

        match auth_wallet.as_ref() {
            Some(auth_wallet) => auth_wallet.set_auth_token(token).await?,
            None => {
                let new_auth_wallet = AuthWallet::with_auth_client(
                    self.mint_url.clone(),
                    self.localstore.clone(),
                    self.metadata_cache.clone(),
                    HashMap::new(),
                    None,
                    self.client
                        .auth_connector(self.mint_url.clone(), Some(token)),
                );

                self.client
                    .set_auth_provider(Some(Arc::new(new_auth_wallet.clone())));
                *auth_wallet = Some(new_auth_wallet);
            }
        }

        Ok(())
    }

    /// Set refresh for authentication
    #[instrument(skip_all)]
    pub async fn set_refresh_token(&self, refresh_token: String) -> Result<(), Error> {
        let auth_wallet = self.auth_wallet.read().await;
        if let Some(auth_wallet) = auth_wallet.as_ref() {
            auth_wallet.set_refresh_token(Some(refresh_token)).await;
        }
        Ok(())
    }

    /// Refresh CAT token
    #[instrument(skip(self))]
    pub async fn refresh_access_token(&self) -> Result<(), Error> {
        let auth_wallet = self.auth_wallet.read().await;
        if let Some(auth_wallet) = auth_wallet.as_ref() {
            auth_wallet.refresh_access_token().await?;
        }
        Ok(())
    }
}
