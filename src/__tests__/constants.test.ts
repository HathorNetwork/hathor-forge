import { describe, it, expect } from "vitest";
import {
  PORTS,
  POLLING_INTERVALS,
  MAX_LOG_ENTRIES,
  DEFAULT_FAUCET_ADDRESS,
} from "@/lib/constants";

describe("PORTS", () => {
  it("has expected port values", () => {
    expect(PORTS.FULLNODE_API).toBe(8080);
    expect(PORTS.STRATUM).toBe(8000);
    expect(PORTS.WALLET_HEADLESS).toBe(8001);
    expect(PORTS.TX_MINING_API).toBe(8002);
    expect(PORTS.TX_MINING_STRATUM).toBe(8003);
    expect(PORTS.EXPLORER).toBe(3001);
    expect(PORTS.MCP_SERVER).toBe(9876);
    expect(PORTS.VITE_DEV).toBe(1420);
  });

  it("has all expected keys", () => {
    const keys = Object.keys(PORTS);
    expect(keys).toContain("FULLNODE_API");
    expect(keys).toContain("STRATUM");
    expect(keys).toContain("WALLET_HEADLESS");
    expect(keys).toContain("TX_MINING_API");
    expect(keys).toContain("TX_MINING_STRATUM");
    expect(keys).toContain("EXPLORER");
    expect(keys).toContain("MCP_SERVER");
    expect(keys).toContain("VITE_DEV");
    expect(keys).toHaveLength(8);
  });

  it("all ports are positive integers", () => {
    for (const port of Object.values(PORTS)) {
      expect(port).toBeGreaterThan(0);
      expect(Number.isInteger(port)).toBe(true);
    }
  });
});

describe("POLLING_INTERVALS", () => {
  it("has expected interval values", () => {
    expect(POLLING_INTERVALS.NODE_STATUS).toBe(3000);
    expect(POLLING_INTERVALS.FAUCET_BALANCE).toBe(3000);
    expect(POLLING_INTERVALS.BLUEPRINTS).toBe(10000);
    expect(POLLING_INTERVALS.CONTRACT_STATES).toBe(5000);
    expect(POLLING_INTERVALS.WALLET_STATUS).toBe(1000);
  });

  it("all intervals are positive", () => {
    for (const interval of Object.values(POLLING_INTERVALS)) {
      expect(interval).toBeGreaterThan(0);
    }
  });
});

describe("MAX_LOG_ENTRIES", () => {
  it("is 1000", () => {
    expect(MAX_LOG_ENTRIES).toBe(1000);
  });
});

describe("DEFAULT_FAUCET_ADDRESS", () => {
  it("is the expected address", () => {
    expect(DEFAULT_FAUCET_ADDRESS).toBe("WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb");
  });
});
