import { create } from "zustand";
import type {
  NodeStatusType,
  MinerStatusType,
  PageType,
  LogSource,
  LogEntry,
} from "@/types";
import { parseLogLevel, stripAnsi } from "@/lib/utils";
import { MAX_LOG_ENTRIES } from "@/lib/constants";

interface AppState {
  // Navigation
  currentPage: PageType;
  setCurrentPage: (page: PageType) => void;

  // Node & Miner status
  nodeStatus: NodeStatusType;
  setNodeStatus: (status: NodeStatusType) => void;
  minerStatus: MinerStatusType;
  setMinerStatus: (status: MinerStatusType) => void;
  blockHeight: number;
  setBlockHeight: (height: number) => void;
  hashRate: string;
  setHashRate: (rate: string) => void;

  // Error
  error: string | null;
  setError: (error: string | null) => void;

  // Logs
  logs: LogEntry[];
  logFilters: Set<LogSource>;
  _logIdCounter: number;
  addLog: (source: LogSource, message: string) => void;
  clearLogs: () => void;
  toggleLogFilter: (source: LogSource) => void;
}

export const useAppStore = create<AppState>()((set, get) => ({
  // Navigation
  currentPage: "dashboard",
  setCurrentPage: (page) => set({ currentPage: page }),

  // Node & Miner status
  nodeStatus: "stopped",
  setNodeStatus: (status) => set({ nodeStatus: status }),
  minerStatus: "stopped",
  setMinerStatus: (status) => set({ minerStatus: status }),
  blockHeight: 0,
  setBlockHeight: (height) => set({ blockHeight: height }),
  hashRate: "0 H/s",
  setHashRate: (rate) => set({ hashRate: rate }),

  // Error
  error: null,
  setError: (error) => set({ error }),

  // Logs
  logs: [],
  logFilters: new Set<LogSource>(["node", "miner", "headless"]),
  _logIdCounter: 0,
  addLog: (source, message) => {
    const cleanMessage = stripAnsi(message);
    if (!cleanMessage.trim()) return;

    const state = get();
    const entry: LogEntry = {
      id: state._logIdCounter,
      timestamp: new Date(),
      source,
      level: parseLogLevel(cleanMessage),
      message: cleanMessage,
    };
    set({
      logs: [...state.logs.slice(-MAX_LOG_ENTRIES), entry],
      _logIdCounter: state._logIdCounter + 1,
    });
  },
  clearLogs: () => set({ logs: [] }),
  toggleLogFilter: (source) => {
    const filters = new Set(get().logFilters);
    if (filters.has(source)) {
      filters.delete(source);
    } else {
      filters.add(source);
    }
    set({ logFilters: filters });
  },
}));
