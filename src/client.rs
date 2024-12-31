use tokio::time::{timeout, Duration};
use reqwest::Client;

use crate::proxy::{RpcRequest, RpcResponse};

/// Represents outcomes of a request.
pub enum CallResult {
    Success(RpcResponse),
    NullResult(RpcResponse),
    EmptyBody,
}

/// Errors that may occur during request processing.
#[derive(Debug, thiserror::Error)]
pub enum RpcProxyError {
    #[error("HTTP server error: {0}")]
    HttpServerError(String),
    #[error("HTTP client error: {0}")]
    HttpClientError(String),
    #[error("HTTP request failed: {0}")]
    HttpRequestFailed(String),
    #[error("Request timed out")]
    Timeout,
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    /// Creates a new `HttpClient` instance.
    pub fn new() -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to build HTTP client");
        Self { client }
    }

    /// Sends an HTTP request to the given address.
    pub async fn send_http_request(
        &self,
        address: &str,
        request: &RpcRequest,
    ) -> Result<CallResult, RpcProxyError> {
        let response = self
            .client
            .post(address)
            .json(request)
            .send()
            .await
            .map_err(|e| RpcProxyError::HttpRequestFailed(e.to_string()))?;

        if response.status().is_server_error() {
            return Err(RpcProxyError::HttpServerError(format!(
                "Server error: {}",
                response.status()
            )));
        }

        if response.status().is_client_error() {
            return Err(RpcProxyError::HttpClientError(format!(
                "Client error: {}",
                response.status()
            )));
        }

        let text_body = response
            .text()
            .await
            .map_err(|e| RpcProxyError::ParseError(format!("Failed to read body: {}", e)))?;

        if text_body.trim().is_empty() {
            return Ok(CallResult::EmptyBody);
        }

        let parsed: RpcResponse = serde_json::from_str(&text_body)
            .map_err(|e| RpcProxyError::ParseError(format!("JSON parse error: {}", e)))?;

        if parsed.result.is_none() {
            return Ok(CallResult::NullResult(parsed));
        }

        Ok(CallResult::Success(parsed))
    }

    /// Wraps the HTTP request in a timeout.
    pub async fn send_request_with_timeout(
        &self,
        address: &str,
        request: &RpcRequest,
        timeout_duration: Duration,
    ) -> Result<CallResult, RpcProxyError> {
        timeout(timeout_duration, self.send_http_request(address, request))
            .await
            .unwrap_or_else(|_| Err(RpcProxyError::Timeout))
    }
}