use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TxMiningStatus {
    pub running: bool,
    pub port: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStatus {
    pub running: bool,
    pub block_height: Option<u64>,
    pub hash_rate: Option<f64>,
    pub peer_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MinerStatus {
    pub running: bool,
    pub hash_rate: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadlessStatus {
    pub running: bool,
    pub port: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletAddress {
    pub address: String,
    pub index: u32,
    pub balance: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendTxRequest {
    pub address: String,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FullnodeBalance {
    pub available: i64,
    pub locked: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeadlessWallet {
    pub wallet_id: String,
    pub status: String,
    pub status_code: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateHeadlessWalletRequest {
    pub wallet_id: String,
    pub seed: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadlessWalletBalance {
    pub available: u64,
    pub locked: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadlessWalletSendTxRequest {
    pub wallet_id: String,
    pub address: String,
    pub amount: u64,
}
