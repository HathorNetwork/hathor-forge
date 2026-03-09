import { describe, it, expect, beforeEach } from "vitest";
import { useUIStore } from "@/store/useUIStore";
import { useNodeStore } from "@/store/useNodeStore";

describe("useUIStore", () => {
  beforeEach(() => {
    useUIStore.setState({
      currentPage: "dashboard",
      error: null,
      logs: [],
      logFilters: new Set(["node", "miner", "headless"]),
      _logIdCounter: 0,
      unreadLogCount: 0,
    });
  });

  describe("initial state", () => {
    it("has correct default values", () => {
      const state = useUIStore.getState();
      expect(state.currentPage).toBe("dashboard");
      expect(state.error).toBeNull();
      expect(state.logs).toEqual([]);
      expect(state.unreadLogCount).toBe(0);
    });

    it("has default log filters", () => {
      const state = useUIStore.getState();
      expect(state.logFilters.has("node")).toBe(true);
      expect(state.logFilters.has("miner")).toBe(true);
      expect(state.logFilters.has("headless")).toBe(true);
    });
  });

  describe("setCurrentPage", () => {
    it("updates the current page", () => {
      useUIStore.getState().setCurrentPage("wallet");
      expect(useUIStore.getState().currentPage).toBe("wallet");
    });
  });

  describe("setError", () => {
    it("sets an error", () => {
      useUIStore.getState().setError("something broke");
      expect(useUIStore.getState().error).toBe("something broke");
    });

    it("clears an error", () => {
      useUIStore.getState().setError("err");
      useUIStore.getState().setError(null);
      expect(useUIStore.getState().error).toBeNull();
    });
  });

  describe("addLog", () => {
    it("adds a log entry", () => {
      useUIStore.getState().addLog("node", "Node started");
      const state = useUIStore.getState();
      expect(state.logs).toHaveLength(1);
      expect(state.logs[0].source).toBe("node");
      expect(state.logs[0].message).toBe("Node started");
      expect(state.logs[0].level).toBe("info");
      expect(state.logs[0].id).toBe(0);
    });

    it("increments unread count", () => {
      useUIStore.getState().addLog("node", "msg1");
      useUIStore.getState().addLog("miner", "msg2");
      expect(useUIStore.getState().unreadLogCount).toBe(2);
    });

    it("strips ANSI codes from messages", () => {
      useUIStore.getState().addLog("node", "\x1B[31mred\x1B[0m");
      expect(useUIStore.getState().logs[0].message).toBe("red");
    });

    it("ignores empty messages", () => {
      useUIStore.getState().addLog("node", "  ");
      expect(useUIStore.getState().logs).toHaveLength(0);
    });

    it("detects error log level", () => {
      useUIStore.getState().addLog("node", "[ERROR] failed");
      expect(useUIStore.getState().logs[0].level).toBe("error");
    });

    it("increments log id counter", () => {
      useUIStore.getState().addLog("node", "first");
      useUIStore.getState().addLog("node", "second");
      expect(useUIStore.getState().logs[0].id).toBe(0);
      expect(useUIStore.getState().logs[1].id).toBe(1);
    });
  });

  describe("clearLogs", () => {
    it("clears all logs and resets unread count", () => {
      useUIStore.getState().addLog("node", "msg");
      useUIStore.getState().clearLogs();
      const state = useUIStore.getState();
      expect(state.logs).toEqual([]);
      expect(state.unreadLogCount).toBe(0);
    });
  });

  describe("markLogsRead", () => {
    it("resets unread count to zero", () => {
      useUIStore.getState().addLog("node", "msg");
      useUIStore.getState().markLogsRead();
      expect(useUIStore.getState().unreadLogCount).toBe(0);
    });
  });

  describe("toggleLogFilter", () => {
    it("removes a filter that exists", () => {
      useUIStore.getState().toggleLogFilter("node");
      expect(useUIStore.getState().logFilters.has("node")).toBe(false);
    });

    it("adds a filter that was removed", () => {
      useUIStore.getState().toggleLogFilter("node");
      useUIStore.getState().toggleLogFilter("node");
      expect(useUIStore.getState().logFilters.has("node")).toBe(true);
    });
  });
});

describe("useNodeStore", () => {
  beforeEach(() => {
    useNodeStore.setState({
      nodeStatus: "stopped",
      minerStatus: "stopped",
      blockHeight: 0,
      hashRate: "0 H/s",
    });
  });

  describe("initial state", () => {
    it("has correct default values", () => {
      const state = useNodeStore.getState();
      expect(state.nodeStatus).toBe("stopped");
      expect(state.minerStatus).toBe("stopped");
      expect(state.blockHeight).toBe(0);
      expect(state.hashRate).toBe("0 H/s");
    });
  });

  describe("setNodeStatus", () => {
    it("updates the node status", () => {
      useNodeStore.getState().setNodeStatus("running");
      expect(useNodeStore.getState().nodeStatus).toBe("running");
    });
  });

  describe("setMinerStatus", () => {
    it("updates the miner status", () => {
      useNodeStore.getState().setMinerStatus("mining");
      expect(useNodeStore.getState().minerStatus).toBe("mining");
    });
  });

  describe("setBlockHeight", () => {
    it("updates the block height", () => {
      useNodeStore.getState().setBlockHeight(42);
      expect(useNodeStore.getState().blockHeight).toBe(42);
    });
  });

  describe("setHashRate", () => {
    it("updates the hash rate", () => {
      useNodeStore.getState().setHashRate("1.5 kH/s");
      expect(useNodeStore.getState().hashRate).toBe("1.5 kH/s");
    });
  });
});
