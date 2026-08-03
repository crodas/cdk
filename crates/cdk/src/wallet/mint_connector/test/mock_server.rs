//! A mock mint implementing [`HttpServer`], the server counterpart of the
//! wallet transport client.
//!
//! Tests register a canned [`ServerResponse`] per `(method, path)` and drive the
//! real [`HttpClient`](super::super::http_client::HttpClient) at it through an
//! [`InProcessTransport`](cdk_http_client::InProcessTransport). Every request is
//! recorded so tests can assert on exactly what the client serialized and where
//! it sent it. This mirrors the canned-response style of
//! [`MockMintConnector`](crate::wallet::test_utils::MockMintConnector), one layer
//! lower: at the wire, not the connector method.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use cdk_common::Method;
use cdk_http_client::{HttpServer, ServerRequest, ServerResponse};
use serde::Serialize;

/// Canned mint server driven by registered responses.
#[derive(Debug, Default)]
pub struct MockMintServer {
    responses: Mutex<HashMap<(Method, String), ServerResponse>>,
    requests: Mutex<Vec<ServerRequest>>,
}

impl MockMintServer {
    /// A server with no responses configured (every request 404s until one is).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the response returned for an exact `(method, path)`.
    pub fn respond(&self, method: Method, path: impl Into<String>, response: ServerResponse) {
        self.responses
            .lock()
            .expect("responses lock")
            .insert((method, path.into()), response);
    }

    /// Register a `200` JSON response by serializing `body`.
    pub fn respond_json<T: Serialize>(&self, method: Method, path: impl Into<String>, body: &T) {
        let bytes = serde_json::to_vec(body).expect("serialize canned response");
        self.respond(method, path, ServerResponse::json(200, bytes));
    }

    /// The most recent request received, if any.
    pub fn last_request(&self) -> Option<ServerRequest> {
        self.requests.lock().expect("requests lock").last().cloned()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl HttpServer for MockMintServer {
    async fn process(&self, request: ServerRequest) -> ServerResponse {
        self.requests
            .lock()
            .expect("requests lock")
            .push(request.clone());

        let path = request
            .path
            .split('?')
            .next()
            .unwrap_or(&request.path)
            .to_string();

        self.responses
            .lock()
            .expect("responses lock")
            .get(&(request.method, path))
            .cloned()
            .unwrap_or_else(|| {
                ServerResponse::text(404, format!("no canned response for {}", request.path))
            })
    }
}
