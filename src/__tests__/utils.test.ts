import { describe, it, expect } from "vitest";
import {
  truncateHash,
  truncateAddress,
  formatHTR,
  formatHashRate,
  parseLogLevel,
  stripAnsi,
} from "@/lib/utils";

describe("truncateHash", () => {
  it("returns short hashes unchanged", () => {
    expect(truncateHash("abcdef")).toBe("abcdef");
  });

  it("truncates long hashes with ellipsis", () => {
    const hash =
      "00000000000000001e8d6829a8a21adc5d38d0a473b144b6765798e61f98bd1d";
    const result = truncateHash(hash);
    expect(result).toMatch(/^.{8}\.\.\..{8}$/);
  });

  it("respects custom char count", () => {
    const hash =
      "00000000000000001e8d6829a8a21adc5d38d0a473b144b6765798e61f98bd1d";
    const result = truncateHash(hash, 4);
    expect(result).toBe("0000...bd1d");
  });
});

describe("truncateAddress", () => {
  it("returns short addresses unchanged", () => {
    expect(truncateAddress("short")).toBe("short");
  });

  it("truncates long addresses", () => {
    const addr = "WXkMhVgRVmTXTVh47wauPKm1xcrW8Qf3Vb";
    const result = truncateAddress(addr);
    expect(result).toMatch(/^.{6}\.\.\..{6}$/);
  });
});

describe("formatHTR", () => {
  it("formats value dividing by 100", () => {
    const result = formatHTR(10000);
    // 10000 / 100 = 100.00
    expect(result).toContain("100");
  });

  it("handles zero", () => {
    const result = formatHTR(0);
    expect(result).toContain("0");
  });
});

describe("formatHashRate", () => {
  it("formats H/s for small values", () => {
    expect(formatHashRate(42)).toBe("42.00 H/s");
  });

  it("formats KH/s", () => {
    expect(formatHashRate(1500)).toBe("1.50 KH/s");
  });

  it("formats MH/s", () => {
    expect(formatHashRate(2_500_000)).toBe("2.50 MH/s");
  });

  it("formats GH/s", () => {
    expect(formatHashRate(3_000_000_000)).toBe("3.00 GH/s");
  });

  it("formats TH/s", () => {
    expect(formatHashRate(1_200_000_000_000)).toBe("1.20 TH/s");
  });
});

describe("parseLogLevel", () => {
  it("detects error level", () => {
    expect(parseLogLevel("[ERROR] something failed")).toBe("error");
    expect(parseLogLevel("error: connection refused")).toBe("error");
  });

  it("detects warning level", () => {
    expect(parseLogLevel("[WARN] deprecated API")).toBe("warning");
    expect(parseLogLevel("Warning: slow query")).toBe("warning");
  });

  it("detects debug level", () => {
    expect(parseLogLevel("[DEBUG] trace info")).toBe("debug");
  });

  it("defaults to info", () => {
    expect(parseLogLevel("normal log line")).toBe("info");
  });
});

describe("stripAnsi", () => {
  it("removes ANSI escape codes", () => {
    expect(stripAnsi("\x1B[31mred text\x1B[0m")).toBe("red text");
  });

  it("leaves plain text unchanged", () => {
    expect(stripAnsi("hello world")).toBe("hello world");
  });
});
