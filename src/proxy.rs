use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{Duration};
use tracing::{debug, error, info};

use crate::config::{RpcConfig};
use crate::metrics::{REQUESTS_TOTAL, REQUESTS_SUCCESS, REQUESTS_FAILURE, REQUEST_LATENCY};
use crate::endpoint::{Endpoint, EndpointResult};
use crate::client::HttpClient;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub id: serde_json::Value,
}

pub struct RpcProxy {
    routes: Vec<Route>,
}

struct Route {
    methods: Vec<String>,
    endpoints: Vec<Endpoint>,
}

impl Serialize for RpcResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;

        // Common fields
        map.serialize_entry("jsonrpc", &self.jsonrpc)?;
        map.serialize_entry("id", &self.id)?;

        // Include `result` only for non-error responses
        if self.error.is_none() {
            map.serialize_entry("result", &self.result)?;
        }

        // Include `error` only for error responses
        if let Some(error) = &self.error {
            map.serialize_entry("error", error)?;
        }

        map.end()
    }
}

impl RpcProxy {
    /// Creates a new RpcProxy instance.
    pub fn new(config: RpcConfig, http_client: HttpClient) -> Self {
        let routes = config
            .routes
            .into_iter()
            .map(|collection| {
                let endpoints = collection
                    .endpoints
                    .into_iter()
                    .map(|endpoint_config| Endpoint::new(endpoint_config, http_client.clone()))
                    .collect();
                Route {
                    methods: collection.methods,
                    endpoints,
                }
            })
            .collect();

        Self { routes }
    }

    /// Forwards a JSON-RPC request to the appropriate endpoints based on the method.
    pub async fn forward_request(&self, request: RpcRequest) -> Result<RpcResponse, String> {
        REQUESTS_TOTAL.inc();
        let timer = REQUEST_LATENCY.start_timer();
        let start_time = tokio::time::Instant::now();

        let route = self
            .routes
            .iter()
            .find(|route| route.methods.contains(&request.method));

        if route.is_none() {
            REQUESTS_FAILURE.inc();
            timer.observe_duration();
            return Ok(RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(json!({
                    "code": -32601,
                    "message": "Method not found"
                })),
                id: request.id.clone(),
            });
        }

        let route = route.unwrap();
        let result = self.try_endpoints(&route.endpoints, &request).await;

        let duration = start_time.elapsed();
        timer.observe_duration();

        if duration > Duration::from_secs(1) {
            info!(
                method = %request.method,
                params = %serde_json::to_string(&request.params)
                    .unwrap_or_else(|_| "Failed to serialize params".to_string()),
                duration = ?duration,
                "Request took longer than 1 second"
            );
        }

        result
    }

    /// Tries a list of endpoints for the given request.
    async fn try_endpoints(
        &self,
        endpoints: &[Endpoint],
        request: &RpcRequest,
    ) -> Result<RpcResponse, String> {
        let mut last_response: Option<RpcResponse> = None;

        for endpoint in endpoints {
            match endpoint.send_request_with_retry(request).await {
                Ok(EndpointResult::Response(response)) => {
                    REQUESTS_SUCCESS.inc();

                    // Got a non-empty result, return it immediately.
                    if response.result.is_some() {
                        return Ok(response);
                    }

                    last_response = Some(response);
                }
                Ok(EndpointResult::SkipToNext(None)) => {
                    REQUESTS_FAILURE.inc();
                    debug!(
                        endpoint = %endpoint.config().address,
                        "Skipping endpoint due to empty body"
                    );
                }
                Ok(EndpointResult::SkipToNext(Some(null_response))) => {
                    REQUESTS_FAILURE.inc();
                    last_response = Some(null_response);
                    debug!(
                        endpoint = %endpoint.config().address,
                        "Skipping endpoint due to null result"
                    );
                }
                Ok(EndpointResult::Error(err)) => {
                    REQUESTS_FAILURE.inc();
                    error!(
                        endpoint = %endpoint.config().address,
                        %err,
                        "Error processing request"
                    );

                    last_response = Some(RpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(json!({
                            "code": -32000,
                            "message": err.to_string()
                        })),
                        id: request.id.clone(),
                    });
                }
                Err(_) => continue,
            }
        }

        if let Some(response) = last_response {
            return Ok(response);
        }

        Ok(RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(json!({
                "code": -32000,
                "message": "Failed to process request"
            })),
            id: request.id.clone(),
        })
    }
}

