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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_config_defaults() {
        let config = NodeConfig::default();
        assert_eq!(config.api_port, 8080);
        assert_eq!(config.stratum_port, 8000);
        assert!(config.data_dir.contains("hathor-forge"));
    }

    #[test]
    fn miner_config_defaults() {
        let config = MinerConfig::default();
        assert_eq!(config.stratum_port, 8003);
        assert_eq!(config.address, "WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb");
        assert_eq!(config.threads, 1);
    }

    #[test]
    fn headless_config_defaults() {
        let config = HeadlessConfig::default();
        assert_eq!(config.port, 8001);
        assert_eq!(config.fullnode_url, "http://localhost:8080/v1a/");
        assert_eq!(config.network, "privatenet");
    }

    #[test]
    fn tx_mining_config_defaults() {
        let config = TxMiningConfig::default();
        assert_eq!(config.api_port, 8002);
        assert_eq!(config.stratum_port, 8003);
        assert_eq!(config.fullnode_url, "http://localhost:8080");
        assert_eq!(config.address, "WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb");
    }

    #[test]
    fn node_config_serialization_roundtrip() {
        let config = NodeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: NodeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.api_port, config.api_port);
        assert_eq!(deserialized.stratum_port, config.stratum_port);
        assert_eq!(deserialized.data_dir, config.data_dir);
    }
}
