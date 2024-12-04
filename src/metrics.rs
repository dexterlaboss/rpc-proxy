use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};
use tracing::info;

/// Metrics registration
lazy_static::lazy_static! {
    pub static ref REQUESTS_TOTAL: IntCounter = prometheus::register_int_counter!(
        "rpc_requests_total",
        "Total number of RPC requests received"
    ).unwrap();

    pub static ref REQUESTS_SUCCESS: IntCounter = prometheus::register_int_counter!(
        "rpc_requests_success_total",
        "Total number of successful RPC responses"
    ).unwrap();

    pub static ref REQUESTS_FAILURE: IntCounter = prometheus::register_int_counter!(
        "rpc_requests_failure_total",
        "Total number of failed RPC responses"
    ).unwrap();

    pub static ref ENDPOINT_RETRIES: IntCounterVec = prometheus::register_int_counter_vec!(
        "rpc_endpoint_retries_total",
        "Total number of retries per endpoint",
        &["endpoint"]
    ).unwrap();

    pub static ref REQUEST_LATENCY: Histogram = {
        let opts = HistogramOpts::new("rpc_request_latency_seconds", "Request latency in seconds")
            .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]);
        Histogram::with_opts(opts).unwrap()
    };
}

/// Metrics handler for Prometheus
pub async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}