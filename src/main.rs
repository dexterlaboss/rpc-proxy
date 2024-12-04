
use axum::{
    extract::State,
    routing::{post, get},
    Json, Router,
};
use clap::Parser;
use std::sync::Arc;
use tower::ServiceBuilder;
use tracing::{info, Level};
use tracing_subscriber;

use config::load_config_from_yaml;
use proxy::{RpcProxy, RpcRequest, RpcResponse};
use metrics::metrics_handler;

mod proxy;
mod config;
mod metrics;

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
        .with_max_level(Level::DEBUG)
        .init();

    // Parse command-line arguments
    let args = Args::parse();

    // Load configuration from YAML
    let config = load_config_from_yaml(&args.config_path);
    let proxy = Arc::new(RpcProxy::new(config));

    // Set up the HTTP server
    let app_state = AppState { proxy };

    // Set up the HTTP server
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