//! In-process transport that bridges a [`Transport`] client to an
//! [`HttpServer`] without a network socket.
//!
//! This lets the real wallet [`HttpClient`](crate::HttpClient) run against any
//! `HttpServer` (a mock mint in tests, or the real mint router) while exercising
//! the genuine request serialization and response decoding paths.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use cashu::nuts::nut22::AuthToken;
use cashu::nuts::Method;
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::Url;

use super::Transport;
use crate::server::{HttpServer, ServerRequest, ServerResponse};
use crate::{HttpError, RawResponse};

/// A [`Transport`] backed by an [`HttpServer`] rather than a network connection.
#[derive(Clone)]
pub struct InProcessTransport {
    server: Arc<dyn HttpServer>,
}

impl InProcessTransport {
    /// Bridge requests to the given server.
    pub fn new(server: Arc<dyn HttpServer>) -> Self {
        Self { server }
    }
}

impl fmt::Debug for InProcessTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InProcessTransport").finish()
    }
}

/// Absolute path plus query string from a URL, e.g. `/v1/keys?foo=bar`.
fn path_and_query(url: &Url) -> String {
    match url.query() {
        Some(query) => format!("{}?{}", url.path(), query),
        None => url.path().to_string(),
    }
}

fn auth_header(request: ServerRequest, auth: Option<AuthToken>) -> ServerRequest {
    match auth {
        Some(token) => request.with_header(token.header_key(), token.to_string()),
        None => request,
    }
}

fn into_raw(response: ServerResponse) -> RawResponse {
    RawResponse::new(response.status, response.body)
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Transport for InProcessTransport {
    fn with_proxy(
        &mut self,
        _proxy: Url,
        _host_matcher: Option<&str>,
        _accept_invalid_certs: bool,
    ) -> Result<(), HttpError> {
        // An in-process transport has no network path to proxy.
        Ok(())
    }

    async fn http_get<R>(&self, url: Url, auth: Option<AuthToken>) -> Result<R, HttpError>
    where
        R: DeserializeOwned,
    {
        self.http_get_raw(url, auth).await?.json_or_status_error()
    }

    async fn http_get_raw(
        &self,
        url: Url,
        auth: Option<AuthToken>,
    ) -> Result<RawResponse, HttpError> {
        let request = auth_header(ServerRequest::new(Method::Get, path_and_query(&url)), auth);
        Ok(into_raw(self.server.process(request).await))
    }

    async fn http_post<P, R>(
        &self,
        url: Url,
        auth_token: Option<AuthToken>,
        payload: &P,
    ) -> Result<R, HttpError>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        let body = serde_json::to_vec(payload)?;
        let request = auth_header(
            ServerRequest::new(Method::Post, path_and_query(&url))
                .with_header("content-type", "application/json")
                .with_body(body),
            auth_token,
        );
        into_raw(self.server.process(request).await).json_or_status_error()
    }

    async fn http_post_form_raw<P>(
        &self,
        url: Url,
        auth_token: Option<AuthToken>,
        payload: &P,
    ) -> Result<RawResponse, HttpError>
    where
        P: Serialize + Send + Sync,
    {
        let body = serde_urlencoded::to_string(payload)
            .map_err(|e| HttpError::Serialization(e.to_string()))?;
        let request = auth_header(
            ServerRequest::new(Method::Post, path_and_query(&url))
                .with_header("content-type", "application/x-www-form-urlencoded")
                .with_body(body.into_bytes()),
            auth_token,
        );
        Ok(into_raw(self.server.process(request).await))
    }
}
