import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import type { LogLevel } from "@/types";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function truncateHash(hash: string, chars = 8): string {
  if (hash.length <= chars * 2 + 3) return hash;
  return `${hash.slice(0, chars)}...${hash.slice(-chars)}`;
}

export function truncateAddress(address: string, chars = 6): string {
  if (address.length <= chars * 2 + 3) return address;
  return `${address.slice(0, chars)}...${address.slice(-chars)}`;
}

export function formatHTR(value: number, decimals = 2): string {
  const htr = value / 100; // Hathor uses 2 decimal places
  return htr.toLocaleString(undefined, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

export function formatHashRate(hashRate: number): string {
  if (hashRate >= 1e12) return `${(hashRate / 1e12).toFixed(2)} TH/s`;
  if (hashRate >= 1e9) return `${(hashRate / 1e9).toFixed(2)} GH/s`;
  if (hashRate >= 1e6) return `${(hashRate / 1e6).toFixed(2)} MH/s`;
  if (hashRate >= 1e3) return `${(hashRate / 1e3).toFixed(2)} KH/s`;
  return `${hashRate.toFixed(2)} H/s`;
}

export function formatTimestamp(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString();
}

export function formatTimeAgo(timestamp: number): string {
  const seconds = Math.floor(Date.now() / 1000 - timestamp);

  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

export function parseLogLevel(line: string): LogLevel {
  const lower = line.toLowerCase();
  if (lower.includes("[error]") || lower.includes("error:")) return "error";
  if (lower.includes("[warn") || lower.includes("warning")) return "warning";
  if (lower.includes("[debug]")) return "debug";
  return "info";
}

export function stripAnsi(str: string): string {
  return str.replace(/\x1B\[[0-9;]*[mK]/g, "");
}
