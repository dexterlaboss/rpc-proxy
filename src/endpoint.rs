use tokio::time::{Duration};
use tracing::{debug, error};

use crate::proxy::{RpcRequest, RpcResponse};
use crate::config::EndpointConfig;
use crate::client::{HttpClient, CallResult, RpcProxyError};

pub enum EndpointResult {
    Response(RpcResponse),
    SkipToNext(Option<RpcResponse>),
    Error(RpcProxyError),
}

pub struct Endpoint {
    config: EndpointConfig,
    http_client: HttpClient,
}

impl Endpoint {
    pub fn new(config: EndpointConfig, http_client: HttpClient) -> Self {
        Self { config, http_client }
    }

    pub fn config(&self) -> &EndpointConfig {
        &self.config
    }

    /// Retries sending the request up to `config.retries` times.
    pub async fn send_request_with_retry(
        &self,
        request: &RpcRequest,
    ) -> Result<EndpointResult, String> {
        let timeout_duration = Duration::from_secs(self.config.timeout_secs);

        for attempt in 1..=self.config.retries {
            debug!(endpoint = %self.config.address, attempt, "Attempting to send request");

            let result = self
                .send_request(request, timeout_duration, attempt)
                .await;

            match result {
                Ok(endpoint_result) => return Ok(endpoint_result),
                Err(true) => continue, // Retryable error; continue to next attempt
                Err(false) => break,  // Non-retryable error; exit retry loop
            }
        }

        Err("Exhausted retries".to_string())
    }

    /// Sends a single request to this endpoint.
    async fn send_request(
        &self,
        request: &RpcRequest,
        timeout_duration: Duration,
        attempt: usize,
    ) -> Result<EndpointResult, bool> {
        let result = self
            .http_client
            .send_request_with_timeout(&self.config.address, request, timeout_duration)
            .await;

        match result {
            Ok(CallResult::Success(response)) => Ok(EndpointResult::Response(response)),
            Ok(CallResult::NullResult(null_response)) => {
                Ok(EndpointResult::SkipToNext(Some(null_response)))
            }
            Ok(CallResult::EmptyBody) => Ok(EndpointResult::SkipToNext(None)),
            Err(err) => {
                let (should_retry, endpoint_result) =
                    self.handle_error(err, attempt, self.config.retries);

                if should_retry {
                    Err(true) // Retryable error
                } else {
                    Ok(endpoint_result) // Fatal or skip
                }
            }
        }
    }

    /// Handles errors and retry.
    fn handle_error(
        &self,
        err: RpcProxyError,
        attempt: usize,
        max_attempts: usize,
    ) -> (bool, EndpointResult) {
        let retryable = matches!(err, RpcProxyError::HttpServerError(_) | RpcProxyError::Timeout);

        if retryable && attempt < max_attempts {
            debug!(?err, "Retryable error -> retrying");
            (true, EndpointResult::SkipToNext(None))
        } else if retryable && attempt == max_attempts {
            error!(?err, "Exhausted retries on a retryable error");
            (false, EndpointResult::Error(err))
        } else {
            debug!(?err, "Non-retryable error -> skipping endpoint");
            (false, EndpointResult::SkipToNext(None))
        }
    }
}