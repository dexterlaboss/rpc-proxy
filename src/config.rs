use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndpointConfig {
    pub address: String,
    pub retries: usize,
    pub timeout_secs: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MethodEndpointCollection {
    pub methods: Vec<String>,
    pub endpoints: Vec<EndpointConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcConfig {
    pub routes: Vec<MethodEndpointCollection>,
}

/// Loads configuration from a YAML file.
pub fn load_config_from_yaml(file_path: &str) -> RpcConfig {
    let yaml_data = fs::read_to_string(file_path).expect("Failed to read YAML file");
    serde_yaml::from_str(&yaml_data).expect("Failed to parse YAML")
}