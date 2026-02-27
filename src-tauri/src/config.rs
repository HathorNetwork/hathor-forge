use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub api_port: u16,
    pub stratum_port: u16,
    pub data_dir: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("hathor-forge")
            .join("data");
        Self {
            api_port: 8080,
            stratum_port: 8000,
            data_dir: data_dir.to_string_lossy().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MinerConfig {
    pub stratum_port: u16,
    pub address: String,
    pub threads: u32,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            stratum_port: 8003,
            address: "WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb".to_string(),
            threads: 1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadlessConfig {
    pub port: u16,
    pub fullnode_url: String,
    pub network: String,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            port: 8001,
            fullnode_url: "http://localhost:8080/v1a/".to_string(),
            network: "privatenet".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxMiningConfig {
    pub api_port: u16,
    pub stratum_port: u16,
    pub fullnode_url: String,
    pub address: String,
}

impl Default for TxMiningConfig {
    fn default() -> Self {
        Self {
            api_port: 8002,
            stratum_port: 8003,
            fullnode_url: "http://localhost:8080".to_string(),
            address: "WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb".to_string(),
        }
    }
}
