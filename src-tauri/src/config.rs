use serde::{Deserialize, Serialize};

// ============================================================================
// Default Port Constants (preferred/fallback values)
// ============================================================================

/// Fullnode HTTP API port (public, served by CORS proxy).
pub const DEFAULT_FULLNODE_API_PORT: u16 = 49080;
/// Fullnode internal port (hathor-core binds here, behind the CORS proxy).
pub const DEFAULT_FULLNODE_INTERNAL_PORT: u16 = 49180;
/// Stratum mining protocol port.
pub const DEFAULT_STRATUM_PORT: u16 = 49000;
/// Wallet-headless service port.
pub const DEFAULT_WALLET_HEADLESS_PORT: u16 = 49001;
/// Tx-mining-service API port.
pub const DEFAULT_TX_MINING_API_PORT: u16 = 49002;
/// Tx-mining-service stratum port.
pub const DEFAULT_TX_MINING_STRATUM_PORT: u16 = 49003;
/// Block explorer HTTP port.
pub const DEFAULT_EXPLORER_PORT: u16 = 49081;
/// MCP server port.
pub const DEFAULT_MCP_SERVER_PORT: u16 = 49876;

// ============================================================================
// Fixed port allocation
// ============================================================================

/// Fixed ports for all services. Using the 49xxx range to avoid conflicts with
/// common services. Ports are deterministic so that external integrations (e.g.
/// MCP config registered in Claude Desktop) remain stable across app restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPorts {
    /// Public-facing fullnode API port (CORS proxy listens here).
    pub fullnode_api: u16,
    /// Internal fullnode port (hathor-core binds here, behind proxy).
    pub fullnode_internal: u16,
    pub stratum: u16,
    pub wallet_headless: u16,
    pub tx_mining_api: u16,
    pub tx_mining_stratum: u16,
    pub explorer: u16,
    pub mcp_server: u16,
}

impl Default for ResolvedPorts {
    fn default() -> Self {
        Self {
            fullnode_api: DEFAULT_FULLNODE_API_PORT,
            fullnode_internal: DEFAULT_FULLNODE_INTERNAL_PORT,
            stratum: DEFAULT_STRATUM_PORT,
            wallet_headless: DEFAULT_WALLET_HEADLESS_PORT,
            tx_mining_api: DEFAULT_TX_MINING_API_PORT,
            tx_mining_stratum: DEFAULT_TX_MINING_STRATUM_PORT,
            explorer: DEFAULT_EXPLORER_PORT,
            mcp_server: DEFAULT_MCP_SERVER_PORT,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub api_port: u16,
    pub stratum_port: u16,
    pub data_dir: String,
}

impl NodeConfig {
    pub fn from_ports(ports: &ResolvedPorts) -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("hathor-forge")
            .join("data");
        Self {
            api_port: ports.fullnode_internal,
            stratum_port: ports.stratum,
            data_dir: data_dir.to_string_lossy().to_string(),
        }
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("hathor-forge")
            .join("data");
        Self {
            api_port: DEFAULT_FULLNODE_API_PORT,
            stratum_port: DEFAULT_STRATUM_PORT,
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
            stratum_port: DEFAULT_TX_MINING_STRATUM_PORT,
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
            port: DEFAULT_WALLET_HEADLESS_PORT,
            fullnode_url: format!("http://127.0.0.1:{}/v1a/", DEFAULT_FULLNODE_API_PORT),
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
            api_port: DEFAULT_TX_MINING_API_PORT,
            stratum_port: DEFAULT_TX_MINING_STRATUM_PORT,
            fullnode_url: format!("http://127.0.0.1:{}", DEFAULT_FULLNODE_API_PORT),
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
        assert_eq!(config.api_port, DEFAULT_FULLNODE_API_PORT);
        assert_eq!(config.stratum_port, DEFAULT_STRATUM_PORT);
        assert!(config.data_dir.contains("hathor-forge"));
    }

    #[test]
    fn miner_config_defaults() {
        let config = MinerConfig::default();
        assert_eq!(config.stratum_port, DEFAULT_TX_MINING_STRATUM_PORT);
        assert_eq!(config.address, "WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb");
        assert_eq!(config.threads, 1);
    }

    #[test]
    fn headless_config_defaults() {
        let config = HeadlessConfig::default();
        assert_eq!(config.port, DEFAULT_WALLET_HEADLESS_PORT);
        assert_eq!(
            config.fullnode_url,
            format!("http://127.0.0.1:{}/v1a/", DEFAULT_FULLNODE_API_PORT)
        );
        assert_eq!(config.network, "privatenet");
    }

    #[test]
    fn tx_mining_config_defaults() {
        let config = TxMiningConfig::default();
        assert_eq!(config.api_port, DEFAULT_TX_MINING_API_PORT);
        assert_eq!(config.stratum_port, DEFAULT_TX_MINING_STRATUM_PORT);
        assert_eq!(
            config.fullnode_url,
            format!("http://127.0.0.1:{}", DEFAULT_FULLNODE_API_PORT)
        );
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
