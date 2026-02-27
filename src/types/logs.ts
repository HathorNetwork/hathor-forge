export type LogSource = "node" | "miner" | "headless";
export type LogLevel = "info" | "warning" | "error" | "debug";

export interface LogEntry {
  id: number;
  timestamp: Date;
  source: LogSource;
  level: LogLevel;
  message: string;
}
