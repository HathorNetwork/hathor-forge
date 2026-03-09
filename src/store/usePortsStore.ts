import { create } from "zustand";
import { PORTS as DEFAULT_PORTS } from "@/lib/constants";

interface ResolvedPorts {
  fullnode_api: number;
  stratum: number;
  wallet_headless: number;
  tx_mining_api: number;
  tx_mining_stratum: number;
  explorer: number;
  mcp_server: number;
}

interface PortsState {
  ports: {
    FULLNODE_API: number;
    STRATUM: number;
    WALLET_HEADLESS: number;
    TX_MINING_API: number;
    TX_MINING_STRATUM: number;
    EXPLORER: number;
    MCP_SERVER: number;
  };
  setPorts: (resolved: ResolvedPorts) => void;
}

export const usePortsStore = create<PortsState>()((set) => ({
  ports: {
    FULLNODE_API: DEFAULT_PORTS.FULLNODE_API,
    STRATUM: DEFAULT_PORTS.STRATUM,
    WALLET_HEADLESS: DEFAULT_PORTS.WALLET_HEADLESS,
    TX_MINING_API: DEFAULT_PORTS.TX_MINING_API,
    TX_MINING_STRATUM: DEFAULT_PORTS.TX_MINING_STRATUM,
    EXPLORER: DEFAULT_PORTS.EXPLORER,
    MCP_SERVER: DEFAULT_PORTS.MCP_SERVER,
  },
  setPorts: (resolved) =>
    set({
      ports: {
        FULLNODE_API: resolved.fullnode_api,
        STRATUM: resolved.stratum,
        WALLET_HEADLESS: resolved.wallet_headless,
        TX_MINING_API: resolved.tx_mining_api,
        TX_MINING_STRATUM: resolved.tx_mining_stratum,
        EXPLORER: resolved.explorer,
        MCP_SERVER: resolved.mcp_server,
      },
    }),
}));
