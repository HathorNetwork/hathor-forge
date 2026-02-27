import { invoke } from "@tauri-apps/api/core";
import type {
  NodeStatus,
  MinerStatus,
  HeadlessStatus,
  HeadlessWallet,
  WalletAddress,
  WalletBalance,
  SendTxRequest,
  CreateHeadlessWalletRequest,
  HeadlessWalletSendTxRequest,
  ContractState,
  BlueprintInfo,
  CreateNanoContractRequest,
  CallNanoContractMethodRequest,
} from "@/types";

// ─── Node ───────────────────────────────────────────────────────────────────

export function startNode(config: unknown = null) {
  return invoke<void>("start_node", { config });
}

export function stopNode() {
  return invoke<void>("stop_node");
}

export function getNodeStatus() {
  return invoke<NodeStatus>("get_node_status");
}

export function getMinerStatus() {
  return invoke<MinerStatus>("get_miner_status");
}

export function resetData() {
  return invoke<string>("reset_data");
}

// ─── Miner ──────────────────────────────────────────────────────────────────

export function startMiner(config: unknown = null) {
  return invoke<void>("start_miner", { config });
}

export function stopMiner() {
  return invoke<void>("stop_miner");
}

// ─── Fullnode Wallet (Faucet) ───────────────────────────────────────────────

export function getFullnodeBalance() {
  return invoke<WalletBalance>("get_fullnode_balance");
}

export function getWalletAddresses() {
  return invoke<WalletAddress[]>("get_wallet_addresses");
}

export function sendTx(request: SendTxRequest) {
  return invoke<string>("send_tx", { request });
}

// ─── Wallet Headless ────────────────────────────────────────────────────────

export function startHeadless(config: unknown = null) {
  return invoke<void>("start_headless", { config });
}

export function stopHeadless() {
  return invoke<void>("stop_headless");
}

export function getHeadlessStatus() {
  return invoke<HeadlessStatus>("get_headless_status");
}

export function createHeadlessWallet(request: CreateHeadlessWalletRequest) {
  return invoke<void>("create_headless_wallet", { request });
}

export function getHeadlessWalletStatus(walletId: string) {
  return invoke<HeadlessWallet>("get_headless_wallet_status", { walletId });
}

export function getHeadlessWalletBalance(walletId: string) {
  return invoke<WalletBalance>("get_headless_wallet_balance", { walletId });
}

export function getHeadlessWalletAddresses(walletId: string) {
  return invoke<string[]>("get_headless_wallet_addresses", { walletId });
}

export function headlessWalletSendTx(request: HeadlessWalletSendTxRequest) {
  return invoke<string>("headless_wallet_send_tx", { request });
}

export function closeHeadlessWallet(walletId: string) {
  return invoke<void>("close_headless_wallet", { walletId });
}

// ─── Explorer ───────────────────────────────────────────────────────────────

export function startExplorerServer() {
  return invoke<void>("start_explorer_server");
}

export function stopExplorerServer() {
  return invoke<void>("stop_explorer_server");
}

// ─── Nano Contracts ─────────────────────────────────────────────────────────

export function getNanoContractState(id: string) {
  return invoke<ContractState>("get_nano_contract_state", { id });
}

export function getNanoContractHistory(id: string) {
  return invoke<unknown>("get_nano_contract_history", { id });
}

export function listBlueprints() {
  return invoke<{ success: boolean; blueprints: Array<{ id: string; name: string; timestamp: number }> }>("list_blueprints");
}

export function getBlueprintInformation(id: string) {
  return invoke<BlueprintInfo>("get_blueprint_information", { id });
}

export function headlessWalletCreateNanoContract(request: CreateNanoContractRequest) {
  return invoke<{ success: boolean; hash: string; message?: string }>("headless_wallet_create_nano_contract", { request });
}

export function headlessWalletCallNanoContractMethod(request: CallNanoContractMethodRequest) {
  return invoke<{ success: boolean; hash: string; message?: string }>("headless_wallet_call_nano_contract_method", { request });
}

// ─── Utilities ──────────────────────────────────────────────────────────────

export function generateSeed() {
  return invoke<string>("generate_seed");
}

export function getState() {
  return invoke<unknown>("get_state");
}

export function gracefulShutdown() {
  return invoke<void>("graceful_shutdown");
}
