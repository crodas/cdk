use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use cdk_common::{database, AuthToken};
use tokio::sync::RwLock as TokioRwLock;
use zeroize::Zeroize;

use crate::cdk_database::WalletDatabase;
use crate::error::Error;
use crate::mint_url::MintUrl;
use crate::nuts::CurrencyUnit;
use crate::wallet::auth::AuthWallet;
use crate::wallet::mint_connector::transport::{Async, RateLimitedTransport};
use crate::wallet::mint_connector::RateLimitedHttpClient;
use crate::wallet::mint_metadata_cache::MintMetadataCache;
use crate::wallet::{
    HttpClient, MintConnector, RateLimitConfig, RateLimiterManager, SubscriptionManager, Wallet,
};

/// Builder for creating a new [`Wallet`]
///
/// Rate limiting: unless a limiter is injected with
/// [`WalletBuilder::with_rate_limiter`], `build()` constructs the wallet's own
/// [`RateLimiterManager`]. Budgets are keyed by the host each request is
/// addressed to, so the wallet's mint, an LNURL service, and an OIDC provider
/// each pace separately. Two wallets built independently do not share a live
/// in-memory budget, only the persisted per-host budget in the KV store. To
/// share one live budget, build them through a
/// [`WalletRepository`](crate::wallet::WalletRepository), which injects one
/// manager into every wallet it creates.
///
/// A limiter only paces what it wraps. `build()` wraps the client it builds
/// itself; when a custom client is supplied with [`WalletBuilder::client`], the
/// caller must build that client over a
/// [`RateLimitedTransport`](crate::wallet::RateLimitedTransport) for the
/// injected limiter to have any effect.
pub struct WalletBuilder {
    mint_url: Option<MintUrl>,
    unit: Option<CurrencyUnit>,
    localstore: Option<Arc<dyn WalletDatabase<database::Error> + Send + Sync>>,
    target_proof_count: Option<usize>,
    seed: Option<[u8; 64]>,
    use_http_subscription: bool,
    client: Option<Arc<dyn MintConnector + Send + Sync>>,
    metadata_cache_ttl: Option<Duration>,
    metadata_cache: Option<Arc<MintMetadataCache>>,
    metadata_caches: HashMap<MintUrl, Arc<MintMetadataCache>>,
    rate_limit: Option<RateLimitConfig>,
    rate_limiter: Option<RateLimiterManager>,
    auth_cat: Option<String>,
}

impl std::fmt::Debug for WalletBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletBuilder")
            .field("mint_url", &self.mint_url)
            .field("unit", &self.unit)
            .field("target_proof_count", &self.target_proof_count)
            .finish_non_exhaustive()
    }
}

impl Default for WalletBuilder {
    fn default() -> Self {
        Self {
            mint_url: None,
            unit: None,
            localstore: None,
            target_proof_count: Some(3),
            seed: None,
            client: None,
            metadata_cache_ttl: Some(Duration::from_secs(3600)),
            use_http_subscription: false,
            metadata_cache: None,
            metadata_caches: HashMap::new(),
            rate_limit: Some(RateLimitConfig::default()),
            rate_limiter: None,
            auth_cat: None,
        }
    }
}

impl Drop for WalletBuilder {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}

impl WalletBuilder {
    /// Create a new WalletBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Use HTTP for wallet subscriptions to mint events
    pub fn use_http_subscription(mut self) -> Self {
        self.use_http_subscription = true;
        self
    }

    /// Set metadata_cache_ttl
    ///
    /// The TTL determines how often the wallet checks the mint for new keysets and information.
    ///
    /// If `None`, the cache will never expire and the wallet will use cached data indefinitely
    /// (unless manually refreshed).
    ///
    /// The default value is 1 hour (3600 seconds).
    pub fn set_metadata_cache_ttl(mut self, metadata_cache_ttl: Option<Duration>) -> Self {
        self.metadata_cache_ttl = metadata_cache_ttl;
        self
    }

    /// If WS is preferred (with fallback to HTTP is it is not supported by the mint) for the wallet
    /// subscriptions to mint events
    pub fn prefer_ws_subscription(mut self) -> Self {
        self.use_http_subscription = false;
        self
    }

    /// Set the mint URL
    pub fn mint_url(mut self, mint_url: MintUrl) -> Self {
        self.mint_url = Some(mint_url);
        self
    }

    /// Set the currency unit
    pub fn unit(mut self, unit: CurrencyUnit) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Set the local storage backend
    pub fn localstore(
        mut self,
        localstore: Arc<dyn WalletDatabase<database::Error> + Send + Sync>,
    ) -> Self {
        self.localstore = Some(localstore);
        self
    }

    /// Set the target proof count
    pub fn target_proof_count(mut self, count: usize) -> Self {
        self.target_proof_count = Some(count);
        self
    }

    /// Set the seed bytes
    pub fn seed(mut self, seed: [u8; 64]) -> Self {
        self.seed.zeroize();
        self.seed = Some(seed);
        self
    }

    /// Set a custom client connector
    pub fn client<C: MintConnector + 'static + Send + Sync>(mut self, client: C) -> Self {
        self.client = Some(Arc::new(client));
        self
    }

    /// Set a custom client connector from Arc
    pub fn shared_client(mut self, client: Arc<dyn MintConnector + Send + Sync>) -> Self {
        self.client = Some(client);
        self
    }

    /// Set a shared MintMetadataCache
    ///
    /// This allows multiple wallets to share the same metadata cache instance for
    /// optimal performance and memory usage. If not provided, a new cache
    /// will be created for each wallet.
    pub fn metadata_cache(mut self, metadata_cache: Arc<MintMetadataCache>) -> Self {
        self.metadata_cache = Some(metadata_cache);
        self
    }

    /// Set a HashMap of MintMetadataCaches for reusing across multiple wallets
    ///
    /// This allows the builder to reuse existing cache instances or create new ones.
    /// Useful when creating multiple wallets that share metadata caches.
    pub fn metadata_caches(
        mut self,
        metadata_caches: HashMap<MintUrl, Arc<MintMetadataCache>>,
    ) -> Self {
        self.metadata_caches = metadata_caches;
        self
    }

    /// Set the rate-limiting configuration.
    ///
    /// Rate limiting is enabled by default with [`RateLimitConfig::default`].
    /// This config is only used when `build()` constructs the wallet's own
    /// limiter; a limiter injected with [`Self::with_rate_limiter`] carries its
    /// own config and overrides this, regardless of call order.
    pub fn with_rate_limiting_config(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit = Some(config);
        self
    }

    /// Use a pre-built, possibly shared [`RateLimiterManager`] for pacing.
    ///
    /// An injected limiter takes precedence over
    /// [`Self::with_rate_limiting_config`]: `build()` uses it verbatim instead of
    /// constructing a per-wallet one, so several wallets can share one live set
    /// of per-host budgets. [`Self::without_rate_limiting`] still clears it.
    pub fn with_rate_limiter(mut self, limiter: RateLimiterManager) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    /// Disable client-side rate limiting.
    pub fn without_rate_limiting(mut self) -> Self {
        self.rate_limit = None;
        self.rate_limiter = None;
        self
    }

    /// Set auth CAT (Clear Auth Token)
    ///
    /// The auth wallet is constructed in [`WalletBuilder::build`], from the
    /// wallet's own mint connector, so auth requests go out over the same
    /// transport as everything else.
    ///
    /// # Errors
    ///
    /// Returns an error if `mint_url` or `localstore` have not been set on the builder.
    pub fn set_auth_cat(mut self, cat: String) -> Result<Self, Error> {
        if self.mint_url.is_none() {
            return Err(Error::Custom("Mint URL required".to_string()));
        }
        if self.localstore.is_none() {
            return Err(Error::Custom("Localstore required".to_string()));
        }

        self.auth_cat = Some(cat);
        Ok(self)
    }

    /// Build the wallet
    pub fn build(mut self) -> Result<Wallet, Error> {
        let mint_url = self
            .mint_url
            .take()
            .ok_or(Error::Custom("Mint url required".to_string()))?;
        let unit = self
            .unit
            .take()
            .ok_or(Error::Custom("Unit required".to_string()))?;
        let localstore = self
            .localstore
            .take()
            .ok_or(Error::Custom("Localstore required".to_string()))?;
        let seed: [u8; 64] = self
            .seed
            .ok_or(Error::Custom("Seed required".to_string()))?;

        let metadata_cache = self.metadata_cache.take().unwrap_or_else(|| {
            // Check if we already have a cache for this mint in the HashMap
            if let Some(cache) = self.metadata_caches.get(&mint_url) {
                cache.clone()
            } else {
                // Create a new one
                Arc::new(MintMetadataCache::new(mint_url.clone()))
            }
        });

        metadata_cache.set_ttl(self.metadata_cache_ttl);

        // A single rate-limited transport, shared by the main client and the
        // blind-auth client so both draw down one persisted budget per host and
        // reuse one connection pool. An injected limiter (e.g. the one
        // WalletRepository shares across all its wallets) wins over building a
        // per-wallet one.
        let injected_limiter = self.rate_limiter.is_some();
        let rate_limiter = match self.rate_limiter.take() {
            Some(limiter) => Some(limiter),
            None => self
                .rate_limit
                .take()
                .map(|config| RateLimiterManager::new(config, Some(localstore.clone()))),
        };
        let shared_transport = rate_limiter.clone().map(|limiter| {
            Arc::new(RateLimitedTransport::with_manager(
                Async::default(),
                limiter,
            ))
        });

        // A limiter the builder constructs only paces the client the builder
        // itself builds around `shared_transport`, so a custom client leaves it
        // wired to nothing. An injected limiter is recorded either way: whoever
        // injected it is also responsible for the transport the custom client
        // runs on, as `WalletRepository` does for its proxy and Tor clients.
        let limiter_is_wired = injected_limiter || self.client.is_none();

        let client = match self.client.take() {
            Some(client) => client,
            None => match shared_transport {
                Some(transport) => Arc::new(RateLimitedHttpClient::with_shared_transport(
                    mint_url.clone(),
                    transport,
                )) as Arc<dyn MintConnector + Send + Sync>,
                None => Arc::new(HttpClient::new(mint_url.clone()))
                    as Arc<dyn MintConnector + Send + Sync>,
            },
        };

        // Deriving the auth wallet from the client is what keeps blind-auth
        // traffic on the same transport, and so the same proxy, Tor circuit and
        // rate-limit budget, as everything else the wallet sends.
        let auth_wallet = self.auth_cat.take().map(|cat| {
            let auth_wallet = AuthWallet::with_auth_client(
                mint_url.clone(),
                localstore.clone(),
                metadata_cache.clone(),
                HashMap::new(),
                None,
                client.auth_connector(mint_url.clone(), Some(AuthToken::ClearAuth(cat))),
            );
            client.set_auth_provider(Some(Arc::new(auth_wallet.clone())));
            auth_wallet
        });

        Ok(Wallet {
            mint_url,
            unit,
            localstore,
            metadata_cache,
            target_proof_count: self.target_proof_count.unwrap_or(3),
            auth_wallet: Arc::new(TokioRwLock::new(auth_wallet)),
            #[cfg(feature = "npubcash")]
            npubcash_client: Arc::new(TokioRwLock::new(None)),
            seed,
            client: client.clone(),
            subscription: SubscriptionManager::new(client, self.use_http_subscription),
            rate_limiter: if limiter_is_wired { rate_limiter } else { None },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_default_ttl() {
        let builder = WalletBuilder::default();
        assert_eq!(builder.metadata_cache_ttl, Some(Duration::from_secs(3600)));
    }

    #[test]
    fn rate_limiting_on_by_default() {
        let builder = WalletBuilder::default();
        assert!(builder.rate_limit.is_some());
    }

    #[test]
    fn without_rate_limiting_clears_it() {
        let builder = WalletBuilder::default().without_rate_limiting();
        assert!(builder.rate_limit.is_none());
    }

    #[tokio::test]
    async fn set_auth_cat_defers_construction() {
        let mint_url = MintUrl::from_str("https://mint.example.com").unwrap();
        let store = Arc::new(cdk_sqlite::wallet::memory::empty().await.unwrap());
        let builder = WalletBuilder::default()
            .mint_url(mint_url)
            .localstore(store)
            .set_auth_cat("cat".to_string())
            .unwrap();
        // Construction is deferred to build(): only the raw CAT is stored.
        assert_eq!(builder.auth_cat.as_deref(), Some("cat"));
    }

    #[test]
    fn set_auth_cat_requires_mint_and_store() {
        let err = WalletBuilder::default().set_auth_cat("cat".to_string());
        assert!(err.is_err());
    }

    async fn base_builder() -> WalletBuilder {
        let store = Arc::new(cdk_sqlite::wallet::memory::empty().await.unwrap());
        WalletBuilder::default()
            .mint_url(MintUrl::from_str("https://mint.example.com").unwrap())
            .unit(crate::nuts::CurrencyUnit::Sat)
            .localstore(store)
            .seed([0u8; 64])
    }

    #[tokio::test]
    async fn build_with_rate_limiting_and_auth_cat() {
        // Exercises the shared-bucket path: a rate-limited auth client plus a
        // rate-limited main client, both built in build().
        let wallet = base_builder()
            .await
            .set_auth_cat("cat".to_string())
            .unwrap()
            .build()
            .unwrap();
        assert!(wallet.auth_wallet.read().await.is_some());
    }

    #[tokio::test]
    async fn build_without_rate_limiting_and_auth_cat() {
        // Exercises the plain path: a plain auth client plus a plain main client.
        let wallet = base_builder()
            .await
            .without_rate_limiting()
            .set_auth_cat("cat".to_string())
            .unwrap()
            .build()
            .unwrap();
        assert!(wallet.auth_wallet.read().await.is_some());
    }

    #[tokio::test]
    async fn default_build_keeps_the_rate_limiter() {
        // No custom client: the limiter paces the main client, so it is retained
        // and the runtime setters have something to act on.
        let wallet = base_builder().await.build().unwrap();
        assert!(wallet.rate_limiter.is_some());
    }

    #[tokio::test]
    async fn custom_client_drops_a_builder_owned_rate_limiter() {
        // A custom client replaces the transport the builder would have paced,
        // so a limiter the builder constructed itself is wired to nothing. The
        // wallet must not keep it, otherwise the runtime setters would silently
        // mutate a disconnected limiter.
        use crate::wallet::test_utils::MockMintConnector;

        let wallet = base_builder()
            .await
            .shared_client(Arc::new(MockMintConnector::new()))
            .build()
            .unwrap();
        assert!(wallet.rate_limiter.is_none());
    }

    #[tokio::test]
    async fn custom_client_keeps_an_injected_rate_limiter() {
        // Whoever injects a limiter also owns the transport the custom client
        // runs on, as WalletRepository does for its proxy and Tor clients, so
        // the wallet keeps it and the runtime setters reach that transport.
        use crate::wallet::test_utils::MockMintConnector;

        let store = Arc::new(cdk_sqlite::wallet::memory::empty().await.unwrap());
        let limiter = RateLimiterManager::new(RateLimitConfig::default(), Some(store));

        let wallet = base_builder()
            .await
            .shared_client(Arc::new(MockMintConnector::new()))
            .with_rate_limiter(limiter)
            .build()
            .unwrap();
        assert!(wallet.rate_limiter.is_some());
    }

    #[tokio::test]
    async fn auth_cat_derives_the_auth_client_from_the_connector() {
        // The CAT path must not build its own auth client: it has to come from
        // the wallet's connector, carrying the CAT, so auth requests inherit
        // that connector's transport.
        use crate::wallet::test_utils::MockMintConnector;

        let client = Arc::new(MockMintConnector::new());

        let wallet = base_builder()
            .await
            .shared_client(client.clone())
            .set_auth_cat("cat".to_string())
            .unwrap()
            .build()
            .unwrap();

        assert!(wallet.auth_wallet.read().await.is_some());
        assert_eq!(
            *client.auth_connector_calls.lock().unwrap(),
            vec![Some(AuthToken::ClearAuth("cat".to_string()))]
        );
    }
}
