//! Generic conformance suite for the wallet's HTTP mint connector.
//!
//! Mirrors the `mint_db_test!` pattern: a set of generic `pub async fn`s plus a
//! macro that emits one `#[tokio::test]` per function. Each test configures a
//! [`MockMintServer`] (the server counterpart of the transport client), builds
//! the real [`HttpClient`](super::http_client::HttpClient) over an
//! [`InProcessTransport`], drives a [`MintConnector`] method, and asserts on both
//! the decoded response and the request the client actually put on the wire.
//!
//! The suite is parameterized by a connector factory, so the same tests can run
//! over any wiring that turns an [`HttpServer`] into a connector. The default
//! wiring is in-process (no socket).
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use cdk_common::{Method, MintQuoteRequest, MintQuoteResponse, MeltQuoteResponse};
use cdk_http_client::{HttpServer, InProcessTransport, ServerRequest, ServerResponse};
use serde::Serialize;

use self::mock_server::MockMintServer;
use super::http_client::HttpClient;
use super::MintConnector;
use crate::nuts::nut00::KnownMethod;
use crate::nuts::{
    CheckStateRequest, CheckStateResponse, CurrencyUnit, KeysResponse, KeysetResponse,
    MeltQuoteBolt11Response,
    MeltQuoteState, MeltRequest, MintQuoteBolt11Request, MintQuoteBolt11Response, MintQuoteState,
    MintRequest, MintResponse, PaymentMethod, RestoreRequest, RestoreResponse, SecretKey,
    SwapRequest, SwapResponse,
};
use crate::wallet::test_utils::{test_keyset, test_keyset_id, test_mint_info, test_mint_url, test_proof};
use crate::Amount;

mod mock_server;

/// Connector factory: turn a mock server into a connector under test.
type ConnectorFactory = fn(Arc<dyn HttpServer>) -> Arc<dyn MintConnector + Send + Sync>;

/// Default wiring: the real `HttpClient` over an in-process bridge to the server.
pub fn in_process_connector(server: Arc<dyn HttpServer>) -> Arc<dyn MintConnector + Send + Sync> {
    Arc::new(HttpClient::with_transport(
        test_mint_url(),
        InProcessTransport::new(server),
        None,
    ))
}

fn assert_json_body<T: Serialize>(request: &ServerRequest, expected: &T) {
    let sent: serde_json::Value = serde_json::from_slice(&request.body).expect("request body json");
    let expected = serde_json::to_value(expected).expect("serialize expected request");
    assert_eq!(sent, expected, "request body sent by the client differs");
}

fn mint_quote_response(quote: &str) -> MintQuoteBolt11Response<String> {
    MintQuoteBolt11Response {
        quote: quote.to_string(),
        request: "lnbc100n1ptest".to_string(),
        amount: Some(Amount::from(100)),
        unit: Some(CurrencyUnit::Sat),
        method: PaymentMethod::Known(KnownMethod::Bolt11),
        amount_paid: Amount::from(0),
        amount_issued: Amount::from(0),
        updated_at: 0,
        state: MintQuoteState::Unpaid,
        expiry: Some(9_999_999_999),
        pubkey: None,
    }
}

fn melt_quote_response(quote: &str) -> MeltQuoteBolt11Response<String> {
    MeltQuoteBolt11Response {
        quote: quote.to_string(),
        amount: Amount::from(100),
        fee_reserve: Amount::from(1),
        state: MeltQuoteState::Unpaid,
        expiry: 9_999_999_999,
        payment_preimage: None,
        change: None,
        request: Some("lnbc100n1ptest".to_string()),
        unit: Some(CurrencyUnit::Sat),
        method: PaymentMethod::Known(KnownMethod::Bolt11),
    }
}

// === Generic tests: one per MintConnector method ===

/// `get_mint_info` GETs `/v1/info` and decodes a [`MintInfo`](crate::nuts::MintInfo).
pub async fn get_mint_info(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond_json(Method::Get, "/v1/info", &test_mint_info());
    let connector = connector_for(server.clone());

    let info = connector.get_mint_info().await.expect("get_mint_info");
    assert_eq!(info.name, test_mint_info().name);

    let request = server.last_request().expect("recorded request");
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.path, "/v1/info");
    assert!(request.body.is_empty());
}

/// `get_mint_keys` GETs `/v1/keys` and returns the keyset list.
pub async fn get_mint_keys(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    let response = KeysResponse {
        keysets: vec![test_keyset()],
    };
    server.respond_json(Method::Get, "/v1/keys", &response);
    let connector = connector_for(server.clone());

    let keys = connector.get_mint_keys().await.expect("get_mint_keys");
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].id, test_keyset_id());
    assert_eq!(server.last_request().expect("request").path, "/v1/keys");
}

/// `get_mint_keyset` GETs `/v1/keys/{id}` and returns the single keyset.
pub async fn get_mint_keyset(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    let response = KeysResponse {
        keysets: vec![test_keyset()],
    };
    let path = format!("/v1/keys/{}", test_keyset_id());
    server.respond_json(Method::Get, path.clone(), &response);
    let connector = connector_for(server.clone());

    let keyset = connector
        .get_mint_keyset(test_keyset_id())
        .await
        .expect("get_mint_keyset");
    assert_eq!(keyset.id, test_keyset_id());
    assert_eq!(server.last_request().expect("request").path, path);
}

/// `get_mint_keysets` GETs `/v1/keysets`.
pub async fn get_mint_keysets(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    let response = KeysetResponse {
        keysets: Vec::new(),
    };
    server.respond_json(Method::Get, "/v1/keysets", &response);
    let connector = connector_for(server.clone());

    connector.get_mint_keysets().await.expect("get_mint_keysets");
    assert_eq!(server.last_request().expect("request").path, "/v1/keysets");
}

/// `post_swap` POSTs the `SwapRequest` to `/v1/swap`.
pub async fn post_swap(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond_json(Method::Post, "/v1/swap", &SwapResponse::new(vec![]));
    let connector = connector_for(server.clone());

    let request = SwapRequest::new(vec![test_proof(test_keyset_id(), 8)], vec![]);
    connector.post_swap(request.clone()).await.expect("post_swap");

    let sent = server.last_request().expect("request");
    assert_eq!(sent.method, Method::Post);
    assert_eq!(sent.path, "/v1/swap");
    assert_json_body(&sent, &request);
}

/// `post_check_state` POSTs `CheckStateRequest` to `/v1/checkstate`.
pub async fn post_check_state(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond_json(
        Method::Post,
        "/v1/checkstate",
        &CheckStateResponse { states: vec![] },
    );
    let connector = connector_for(server.clone());

    let request = CheckStateRequest {
        ys: vec![SecretKey::generate().public_key()],
    };
    connector
        .post_check_state(request.clone())
        .await
        .expect("post_check_state");

    let sent = server.last_request().expect("request");
    assert_eq!(sent.path, "/v1/checkstate");
    assert_json_body(&sent, &request);
}

/// `post_restore` POSTs `RestoreRequest` to `/v1/restore`.
pub async fn post_restore(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond_json(
        Method::Post,
        "/v1/restore",
        &RestoreResponse {
            outputs: vec![],
            signatures: vec![],
        },
    );
    let connector = connector_for(server.clone());

    let request = RestoreRequest { outputs: vec![] };
    connector
        .post_restore(request.clone())
        .await
        .expect("post_restore");

    let sent = server.last_request().expect("request");
    assert_eq!(sent.path, "/v1/restore");
    assert_json_body(&sent, &request);
}

/// `post_mint` (bolt11) POSTs `MintRequest` to `/v1/mint/bolt11`.
pub async fn post_mint(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond_json(
        Method::Post,
        "/v1/mint/bolt11",
        &MintResponse { signatures: vec![] },
    );
    let connector = connector_for(server.clone());

    let request = MintRequest {
        quote: "quote-id".to_string(),
        outputs: vec![],
        signature: None,
    };
    connector
        .post_mint(&PaymentMethod::Known(KnownMethod::Bolt11), request.clone())
        .await
        .expect("post_mint");

    let sent = server.last_request().expect("request");
    assert_eq!(sent.path, "/v1/mint/bolt11");
    assert_json_body(&sent, &request);
}

/// `post_mint_quote` (bolt11) POSTs to `/v1/mint/quote/bolt11`.
pub async fn post_mint_quote(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond_json(
        Method::Post,
        "/v1/mint/quote/bolt11",
        &mint_quote_response("quote-id"),
    );
    let connector = connector_for(server.clone());

    let request = MintQuoteBolt11Request {
        amount: Amount::from(100),
        unit: CurrencyUnit::Sat,
        description: None,
        pubkey: None,
    };
    let response = connector
        .post_mint_quote(MintQuoteRequest::Bolt11(request.clone()))
        .await
        .expect("post_mint_quote");
    assert!(matches!(response, MintQuoteResponse::Bolt11(_)));

    let sent = server.last_request().expect("request");
    assert_eq!(sent.path, "/v1/mint/quote/bolt11");
    assert_json_body(&sent, &request);
}

/// `get_mint_quote_status` (bolt11) GETs `/v1/mint/quote/bolt11/{id}`.
pub async fn get_mint_quote_status(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond_json(
        Method::Get,
        "/v1/mint/quote/bolt11/quote-id",
        &mint_quote_response("quote-id"),
    );
    let connector = connector_for(server.clone());

    let response = connector
        .get_mint_quote_status(PaymentMethod::Known(KnownMethod::Bolt11), "quote-id")
        .await
        .expect("get_mint_quote_status");
    assert!(matches!(response, MintQuoteResponse::Bolt11(_)));
    assert_eq!(
        server.last_request().expect("request").path,
        "/v1/mint/quote/bolt11/quote-id"
    );
}

/// `get_melt_quote_status` (bolt11) GETs `/v1/melt/quote/bolt11/{id}`.
pub async fn get_melt_quote_status(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond_json(
        Method::Get,
        "/v1/melt/quote/bolt11/quote-id",
        &melt_quote_response("quote-id"),
    );
    let connector = connector_for(server.clone());

    let response = connector
        .get_melt_quote_status(PaymentMethod::Known(KnownMethod::Bolt11), "quote-id")
        .await
        .expect("get_melt_quote_status");
    assert!(matches!(response, MeltQuoteResponse::Bolt11(_)));
    assert_eq!(
        server.last_request().expect("request").path,
        "/v1/melt/quote/bolt11/quote-id"
    );
}

/// `post_melt` (bolt11) POSTs `MeltRequest` to `/v1/melt/bolt11`.
pub async fn post_melt(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond_json(
        Method::Post,
        "/v1/melt/bolt11",
        &melt_quote_response("quote-id"),
    );
    let connector = connector_for(server.clone());

    let request = MeltRequest::new(
        "quote-id".to_string(),
        vec![test_proof(test_keyset_id(), 8)],
        None,
    );
    let response = connector
        .post_melt(&PaymentMethod::Known(KnownMethod::Bolt11), request.clone())
        .await
        .expect("post_melt");
    assert!(matches!(response, MeltQuoteResponse::Bolt11(_)));

    let sent = server.last_request().expect("request");
    assert_eq!(sent.path, "/v1/melt/bolt11");
    assert_json_body(&sent, &request);
}

/// A non-2xx response maps to [`Error::HttpError`](crate::Error::HttpError).
pub async fn error_maps_to_http_error(connector_for: ConnectorFactory) {
    let server = Arc::new(MockMintServer::new());
    server.respond(Method::Get, "/v1/info", ServerResponse::text(400, "boom"));
    let connector = connector_for(server.clone());

    let error = connector
        .get_mint_info()
        .await
        .expect_err("non-2xx should be an error");
    assert!(
        matches!(error, crate::Error::HttpError(Some(400), _)),
        "expected HttpError(400), got {error:?}"
    );
}

/// Generate one `#[tokio::test]` per suite function, wired to `$factory`.
macro_rules! mint_connector_test {
    ($factory:expr) => {
        mint_connector_test!(
            $factory,
            get_mint_info,
            get_mint_keys,
            get_mint_keyset,
            get_mint_keysets,
            post_swap,
            post_check_state,
            post_restore,
            post_mint,
            post_mint_quote,
            get_mint_quote_status,
            get_melt_quote_status,
            post_melt,
            error_maps_to_http_error,
        );
    };
    ($factory:expr, $($name:ident),+ $(,)?) => {
        $(
            #[tokio::test]
            async fn $name() {
                crate::wallet::mint_connector::test::$name($factory).await;
            }
        )+
    };
}

mod in_process_suite {
    mint_connector_test!(super::in_process_connector);
}
