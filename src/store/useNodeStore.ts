import { create } from "zustand";
import type { NodeStatusType, MinerStatusType } from "@/types";

interface NodeState {
  nodeStatus: NodeStatusType;
  setNodeStatus: (status: NodeStatusType) => void;
  minerStatus: MinerStatusType;
  setMinerStatus: (status: MinerStatusType) => void;
  blockHeight: number;
  setBlockHeight: (height: number) => void;
  hashRate: string;
  setHashRate: (rate: string) => void;
}

export const useNodeStore = create<NodeState>()((set) => ({
  nodeStatus: "stopped",
  setNodeStatus: (status) => set({ nodeStatus: status }),
  minerStatus: "stopped",
  setMinerStatus: (status) => set({ minerStatus: status }),
  blockHeight: 0,
  setBlockHeight: (height) => set({ blockHeight: height }),
  hashRate: "0 H/s",
  setHashRate: (rate) => set({ hashRate: rate }),
}));
