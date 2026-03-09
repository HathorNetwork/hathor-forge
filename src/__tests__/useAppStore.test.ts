import { describe, it, expect, beforeEach } from "vitest";
import { useAppStore } from "@/store/useAppStore";

describe("useAppStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useAppStore.setState({
      currentPage: "dashboard",
      nodeStatus: "stopped",
      minerStatus: "stopped",
      blockHeight: 0,
      hashRate: "0 H/s",
      error: null,
      logs: [],
      logFilters: new Set(["node", "miner", "headless"]),
      _logIdCounter: 0,
      unreadLogCount: 0,
    });
  });

  describe("initial state", () => {
    it("has correct default values", () => {
      const state = useAppStore.getState();
      expect(state.currentPage).toBe("dashboard");
      expect(state.nodeStatus).toBe("stopped");
      expect(state.minerStatus).toBe("stopped");
      expect(state.blockHeight).toBe(0);
      expect(state.hashRate).toBe("0 H/s");
      expect(state.error).toBeNull();
      expect(state.logs).toEqual([]);
      expect(state.unreadLogCount).toBe(0);
    });

    it("has default log filters", () => {
      const state = useAppStore.getState();
      expect(state.logFilters.has("node")).toBe(true);
      expect(state.logFilters.has("miner")).toBe(true);
      expect(state.logFilters.has("headless")).toBe(true);
    });
  });

  describe("setCurrentPage", () => {
    it("updates the current page", () => {
      useAppStore.getState().setCurrentPage("wallet");
      expect(useAppStore.getState().currentPage).toBe("wallet");
    });
  });

  describe("setNodeStatus", () => {
    it("updates the node status", () => {
      useAppStore.getState().setNodeStatus("running");
      expect(useAppStore.getState().nodeStatus).toBe("running");
    });
  });

  describe("setMinerStatus", () => {
    it("updates the miner status", () => {
      useAppStore.getState().setMinerStatus("mining");
      expect(useAppStore.getState().minerStatus).toBe("mining");
    });
  });

  describe("setBlockHeight", () => {
    it("updates the block height", () => {
      useAppStore.getState().setBlockHeight(42);
      expect(useAppStore.getState().blockHeight).toBe(42);
    });
  });

  describe("setError", () => {
    it("sets an error", () => {
      useAppStore.getState().setError("something broke");
      expect(useAppStore.getState().error).toBe("something broke");
    });

    it("clears an error", () => {
      useAppStore.getState().setError("err");
      useAppStore.getState().setError(null);
      expect(useAppStore.getState().error).toBeNull();
    });
  });

  describe("addLog", () => {
    it("adds a log entry", () => {
      useAppStore.getState().addLog("node", "Node started");
      const state = useAppStore.getState();
      expect(state.logs).toHaveLength(1);
      expect(state.logs[0].source).toBe("node");
      expect(state.logs[0].message).toBe("Node started");
      expect(state.logs[0].level).toBe("info");
      expect(state.logs[0].id).toBe(0);
    });

    it("increments unread count", () => {
      useAppStore.getState().addLog("node", "msg1");
      useAppStore.getState().addLog("miner", "msg2");
      expect(useAppStore.getState().unreadLogCount).toBe(2);
    });

    it("strips ANSI codes from messages", () => {
      useAppStore.getState().addLog("node", "\x1B[31mred\x1B[0m");
      expect(useAppStore.getState().logs[0].message).toBe("red");
    });

    it("ignores empty messages", () => {
      useAppStore.getState().addLog("node", "  ");
      expect(useAppStore.getState().logs).toHaveLength(0);
    });

    it("detects error log level", () => {
      useAppStore.getState().addLog("node", "[ERROR] failed");
      expect(useAppStore.getState().logs[0].level).toBe("error");
    });

    it("increments log id counter", () => {
      useAppStore.getState().addLog("node", "first");
      useAppStore.getState().addLog("node", "second");
      expect(useAppStore.getState().logs[0].id).toBe(0);
      expect(useAppStore.getState().logs[1].id).toBe(1);
    });
  });

  describe("clearLogs", () => {
    it("clears all logs and resets unread count", () => {
      useAppStore.getState().addLog("node", "msg");
      useAppStore.getState().clearLogs();
      const state = useAppStore.getState();
      expect(state.logs).toEqual([]);
      expect(state.unreadLogCount).toBe(0);
    });
  });

  describe("markLogsRead", () => {
    it("resets unread count to zero", () => {
      useAppStore.getState().addLog("node", "msg");
      useAppStore.getState().markLogsRead();
      expect(useAppStore.getState().unreadLogCount).toBe(0);
    });
  });

  describe("toggleLogFilter", () => {
    it("removes a filter that exists", () => {
      useAppStore.getState().toggleLogFilter("node");
      expect(useAppStore.getState().logFilters.has("node")).toBe(false);
    });

    it("adds a filter that was removed", () => {
      useAppStore.getState().toggleLogFilter("node");
      useAppStore.getState().toggleLogFilter("node");
      expect(useAppStore.getState().logFilters.has("node")).toBe(true);
    });
  });
});
