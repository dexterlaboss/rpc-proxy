
use axum::{
    extract::State,
    routing::{post, get},
    Json, Router,
};
use clap::Parser;
use std::sync::Arc;
use tracing::{info};
use tracing_subscriber::{EnvFilter};
use tracing_subscriber;

use rpc_proxy::proxy::{RpcProxy, RpcRequest, RpcResponse};
use rpc_proxy::config::load_config_from_yaml;
use rpc_proxy::metrics::metrics_handler;
use rpc_proxy::client::HttpClient;

/// Command-line arguments for the RPC Proxy.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Listening IP address (default: 0.0.0.0)
    #[arg(short, long, default_value = "0.0.0.0")]
    listen_ip: String,

    /// Listening port (default: 8899)
    #[arg(short, long, default_value_t = 8899)]
    listen_port: u16,

    /// Path to the configuration file (default: config.yaml)
    #[arg(short, long, default_value = "config.yaml")]
    config_path: String,
}

#[derive(Clone)]
struct AppState {
    proxy: Arc<RpcProxy>,
}

/// Handles incoming RPC requests
async fn handle_http_request(
    State(state): State<AppState>,
    Json(request): Json<RpcRequest>,
) -> Result<Json<RpcResponse>, String> {
    state.proxy.forward_request(request).await.map(Json)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Parse command-line arguments
    let args = Args::parse();

    // Load configuration from YAML
    let config = load_config_from_yaml(&args.config_path);

    // Create HTTP client
    // let http_client = reqwest::Client::builder()
    //     .build()
    //     .expect("Failed to build HTTP client");

    let http_client = HttpClient::new();

    // Initialize the proxy
    let proxy = Arc::new(RpcProxy::new(config, http_client));

    // Set up the HTTP server
    let app_state = AppState { proxy };

    let app = Router::new()
        .route("/", post(handle_http_request))
        .route("/metrics", get(metrics_handler))
        .with_state(app_state);

    let addr = format!("{}:{}", args.listen_ip, args.listen_port);
    info!("Server running on {}", addr);

    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await?;

    Ok(())
}