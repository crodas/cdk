//! [`HttpServer`] implementation backed by the mint's axum [`Router`].
//!
//! This is the server-side counterpart of the wallet transport client. It lets
//! the real mint be driven through the same [`HttpServer`] abstraction a mock
//! uses, including in-process (no socket) via
//! [`InProcessTransport`](cdk_http_client::InProcessTransport). `cdk-mintd` still
//! serves the router over TCP with `axum::serve`; this adapter is an additional
//! entry point, not a replacement.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use axum::body::Body;
use axum::Router;
use cdk::mint::Mint;
use cdk::nuts::Method;
use cdk_http_client::{HttpServer, ServerRequest, ServerResponse};
use tower::ServiceExt;

use crate::create_mint_router;

/// An [`HttpServer`] that dispatches requests through the mint's axum router.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct RouterServer {
    router: Router,
}

impl RouterServer {
    /// Wrap an already-built mint router.
    pub fn new(router: Router) -> Self {
        Self { router }
    }
}

/// Build a [`RouterServer`] over the mint's default router.
pub async fn create_mint_server(
    mint: Arc<Mint>,
    custom_methods: Vec<String>,
) -> Result<RouterServer> {
    Ok(RouterServer::new(
        create_mint_router(mint, custom_methods).await?,
    ))
}

fn server_error(message: String) -> ServerResponse {
    ServerResponse::text(500, message)
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl HttpServer for RouterServer {
    async fn process(&self, request: ServerRequest) -> ServerResponse {
        let method = match request.method {
            Method::Get => axum::http::Method::GET,
            Method::Post => axum::http::Method::POST,
        };

        let mut builder = axum::http::Request::builder()
            .method(method)
            .uri(&request.path);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        let http_request = match builder.body(Body::from(request.body)) {
            Ok(request) => request,
            Err(err) => return server_error(format!("invalid request: {err}")),
        };

        // The router's service is infallible; `match err {}` discharges the
        // `Infallible` error without an unwrap.
        let response = match self.router.clone().oneshot(http_request).await {
            Ok(response) => response,
            Err(infallible) => match infallible {},
        };

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();

        let body = match axum::body::to_bytes(response.into_body(), usize::MAX).await {
            Ok(bytes) => bytes.to_vec(),
            Err(err) => return server_error(format!("failed to read response body: {err}")),
        };

        ServerResponse {
            status,
            headers,
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use bip39::Mnemonic;
    use cdk::mint::{MintBuilder, MintMeltLimits};
    use cdk::nuts::nut00::KnownMethod;
    use cdk::nuts::{CurrencyUnit, MintInfo, PaymentMethod};
    use cdk::types::{FeeReserve, QuoteTTL};
    use cdk_fake_wallet::FakeWallet;
    use cdk_http_client::ServerRequest;

    use super::*;

    async fn test_mint() -> Arc<Mint> {
        let db = Arc::new(cdk_sqlite::mint::memory::empty().await.unwrap());
        let mut builder = MintBuilder::new(db.clone());
        let fake = FakeWallet::new(
            FeeReserve {
                min_fee_reserve: 1.into(),
                percent_fee_reserve: 0.0,
            },
            HashMap::default(),
            HashSet::default(),
            0,
            CurrencyUnit::Sat,
        );
        builder
            .add_payment_processor(
                CurrencyUnit::Sat,
                PaymentMethod::Known(KnownMethod::Bolt11),
                MintMeltLimits::new(1, 10_000),
                Arc::new(fake),
            )
            .await
            .unwrap();

        let mnemonic = Mnemonic::generate(12).unwrap();
        let mint = builder
            .build_with_seed(db, &mnemonic.to_seed_normalized(""))
            .await
            .unwrap();
        mint.set_quote_ttl(QuoteTTL::new(10_000, 10_000))
            .await
            .unwrap();
        mint.start().await.unwrap();
        Arc::new(mint)
    }

    #[tokio::test]
    async fn get_info_through_http_server() {
        let server = create_mint_server(test_mint().await, vec![])
            .await
            .expect("router builds");

        let response = server
            .process(ServerRequest::new(Method::Get, "/v1/info"))
            .await;

        assert_eq!(response.status, 200);
        let _info: MintInfo =
            serde_json::from_slice(&response.body).expect("body is a MintInfo document");
    }
}
