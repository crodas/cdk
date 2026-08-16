use std::fmt::Debug;

use async_trait::async_trait;
use cdk_common::{AuthToken, ProtectedEndpoint};

use super::AuthWallet;
use crate::error::Error;

/// Supplies auth tokens for the endpoints a mint protects.
///
/// This is the only auth surface a [`MintConnector`](crate::wallet::MintConnector)
/// sees, so the wallet's auth state stays internal to the crate.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait AuthTokenProvider: Debug + Send + Sync {
    /// Token to attach to a request for `endpoint`, or `None` when the endpoint
    /// is not protected.
    async fn auth_for_request(
        &self,
        endpoint: &ProtectedEndpoint,
    ) -> Result<Option<AuthToken>, Error>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AuthTokenProvider for AuthWallet {
    async fn auth_for_request(
        &self,
        endpoint: &ProtectedEndpoint,
    ) -> Result<Option<AuthToken>, Error> {
        self.get_auth_for_request(endpoint).await
    }
}
