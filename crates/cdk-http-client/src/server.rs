//! Server-side counterpart of the [`Transport`](crate::Transport) trait.
//!
//! Where a `Transport` turns a wallet call into an HTTP request and sends it, an
//! [`HttpServer`] receives that request and produces the response. The two sides
//! share the same wire vocabulary, so a client can be pointed at any server
//! implementation: the real mint router, or a mock, without a socket in between
//! (see [`InProcessTransport`](crate::InProcessTransport)).

use async_trait::async_trait;
use cashu::nuts::Method;

/// An HTTP request as seen by the server side.
#[derive(Debug, Clone)]
pub struct ServerRequest {
    /// Request method (GET or POST).
    pub method: Method,
    /// Request target: absolute path plus any query string, e.g. `/v1/keys`.
    pub path: String,
    /// Header name/value pairs.
    pub headers: Vec<(String, String)>,
    /// Raw request body.
    pub body: Vec<u8>,
}

impl ServerRequest {
    /// Build a request with no headers or body.
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Append a header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the body.
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Case-insensitive header lookup.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// An HTTP response produced by the server side.
#[derive(Debug, Clone)]
pub struct ServerResponse {
    /// HTTP status code.
    pub status: u16,
    /// Header name/value pairs.
    pub headers: Vec<(String, String)>,
    /// Raw response body.
    pub body: Vec<u8>,
}

impl ServerResponse {
    /// A response with the given status, no headers, empty body.
    pub fn status(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// A JSON response (`content-type: application/json`).
    pub fn json(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: body.into(),
        }
    }

    /// A plain-text response (`content-type: text/plain`).
    pub fn text(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: vec![("content-type".to_string(), "text/plain".to_string())],
            body: body.into(),
        }
    }
}

/// The server side of an HTTP exchange.
///
/// Implementors receive a [`ServerRequest`] and return a [`ServerResponse`]. The
/// real mint provides one by adapting its axum router; tests provide a mock.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait HttpServer: Send + Sync {
    /// Handle a request and produce a response.
    async fn process(&self, request: ServerRequest) -> ServerResponse;
}
