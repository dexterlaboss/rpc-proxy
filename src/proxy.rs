use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};
use tracing::{debug, error};

use crate::config::{EndpointConfig, MethodEndpointCollection, RpcConfig};
use crate::metrics::{REQUESTS_TOTAL, REQUESTS_SUCCESS, REQUESTS_FAILURE, ENDPOINT_RETRIES, REQUEST_LATENCY};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub id: serde_json::Value,
}

pub struct RpcProxy {
    routes: Vec<MethodEndpointCollection>,
    http_client: Client,
}

impl RpcProxy {
    pub fn new(config: RpcConfig) -> Self {
        let http_client = Client::builder()
            .build()
            .expect("Failed to build HTTP client");
        Self {
            routes: config.routes,
            http_client,
        }
    }

    /// Forwards a JSON-RPC request to the appropriate endpoints based on the method.
    pub async fn forward_request(
        &self,
        request: RpcRequest,
    ) -> Result<RpcResponse, String> {
        REQUESTS_TOTAL.inc();
        let timer = REQUEST_LATENCY.start_timer();

        debug!(method = %request.method, "Forwarding JSON-RPC request");

        let route = self
            .routes
            .iter()
            .find(|c| c.methods.contains(&request.method))
            .ok_or_else(|| format!("No endpoints configured for method: {}", request.method))?;

        for endpoint in &route.endpoints {
            match self.process_request_with_endpoint(endpoint, &request).await {
                Ok(Some(response)) => {
                    REQUESTS_SUCCESS.inc();
                    timer.observe_duration();
                    return Ok(response);
                }
                Ok(None) => {
                    debug!(endpoint = %endpoint.address, "Null result; moving to next endpoint");
                }
                Err(e) => {
                    REQUESTS_FAILURE.inc();
                    error!(endpoint = %endpoint.address, %e, "Error processing request");
                }
            }
        }

        timer.observe_duration();
        Err("Failed to process request after retries.".to_string())
    }

    async fn process_request_with_endpoint(
        &self,
        endpoint: &EndpointConfig,
        request: &RpcRequest,
    ) -> Result<Option<RpcResponse>, String> {
        let timeout_duration = Duration::from_secs(endpoint.timeout_secs);

        for attempt in 1..=endpoint.retries {
            debug!(endpoint = %endpoint.address, attempt, "Attempting to send request");

            match timeout(timeout_duration, self.send_request_to_endpoint(endpoint, request)).await {
                Ok(Ok(Some(response))) => return Ok(Some(response)),
                Ok(Ok(None)) => return Ok(None), // Null result; try next endpoint
                Ok(Err(e)) => {
                    if attempt == endpoint.retries {
                        error!(endpoint = %endpoint.address, %e, "Max retries reached for endpoint");
                        return Err(format!("Error after retries: {}", e));
                    }
                    debug!(endpoint = %endpoint.address, attempt, "Retrying due to error");
                }
                Err(_) => {
                    error!(endpoint = %endpoint.address, "Request timed out");
                    if attempt == endpoint.retries {
                        return Err("Timeout after retries".to_string());
                    }
                }
            }
        }

        Ok(None)
    }

    async fn send_request_to_endpoint(
        &self,
        endpoint: &EndpointConfig,
        request: &RpcRequest,
    ) -> Result<Option<RpcResponse>, String> {
        let response = self
            .http_client
            .post(&endpoint.address)
            .json(request)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if response.status().is_server_error() {
            return Err(format!(
                "HTTP server error: {}",
                response.status()
            ));
        }

        if !response.status().is_success() {
            return Err(format!(
                "HTTP client error: {}",
                response.status()
            ));
        }

        let rpc_response: RpcResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

        if rpc_response.result.is_none() {
            debug!(endpoint = %endpoint.address, "Received null result");
            return Ok(None);
        }

        Ok(Some(rpc_response))
    }
}