import { create } from "zustand";
import type { PageType, LogSource, LogEntry } from "@/types";
import { parseLogLevel, stripAnsi } from "@/lib/utils";
import { MAX_LOG_ENTRIES } from "@/lib/constants";

interface UIState {
  // Navigation
  currentPage: PageType;
  setCurrentPage: (page: PageType) => void;

  // Error
  error: string | null;
  setError: (error: string | null) => void;

  // Logs
  logs: LogEntry[];
  logFilters: Set<LogSource>;
  _logIdCounter: number;
  unreadLogCount: number;
  addLog: (source: LogSource, message: string) => void;
  clearLogs: () => void;
  markLogsRead: () => void;
  toggleLogFilter: (source: LogSource) => void;
}

export const useUIStore = create<UIState>()((set, get) => ({
  // Navigation
  currentPage: "dashboard",
  setCurrentPage: (page) => set({ currentPage: page }),

  // Error
  error: null,
  setError: (error) => set({ error }),

  // Logs
  logs: [],
  logFilters: new Set<LogSource>(["node", "miner", "headless"]),
  _logIdCounter: 0,
  unreadLogCount: 0,
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
      unreadLogCount: state.unreadLogCount + 1,
    });
  },
  clearLogs: () => set({ logs: [], unreadLogCount: 0 }),
  markLogsRead: () => set({ unreadLogCount: 0 }),
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
