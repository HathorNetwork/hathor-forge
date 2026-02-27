export type NodeStatusType = "stopped" | "starting" | "running" | "error";
export type MinerStatusType = "stopped" | "starting" | "mining" | "error";

export interface NodeStatus {
  running: boolean;
  block_height: number | null;
  hash_rate: number | null;
  peer_count: number | null;
}

export interface MinerStatus {
  running: boolean;
  hash_rate: number | null;
}
