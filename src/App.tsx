import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Cpu,
  Database,
  Play,
  Square,
  Activity,
  Layers,
  Coins,
  FileText,
  Settings,
  Loader2,
  Terminal,
  Zap,
  Compass,
  AlertTriangle,
  Trash2,
  Wallet,
  Copy,
  Send,
  Check,
  BookOpen,
} from "lucide-react";
import { SwaggerUIComponent } from "@/components/SwaggerUI";
import 'swagger-ui-react/swagger-ui.css';

type NodeStatusType = "stopped" | "starting" | "running" | "error";
type MinerStatusType = "stopped" | "starting" | "mining" | "error";
type PageType = "dashboard" | "explorer" | "wallet" | "blocks" | "transactions" | "tokens" | "mining" | "logs" | "settings" | "api-explorer";

interface NodeStatus {
  running: boolean;
  block_height: number | null;
  hash_rate: number | null;
  peer_count: number | null;
}

type LogSource = "node" | "miner" | "headless";

interface LogEntry {
  id: number;
  timestamp: Date;
  source: LogSource;
  level: "info" | "warning" | "error" | "debug";
  message: string;
}

interface WalletAddress {
  address: string;
  index: number;
  balance: number | null;
}

interface HeadlessWallet {
  wallet_id: string;
  status: string;
  status_code: number | null;
  balance?: { available: number; locked: number };
  addresses?: string[];
  seed?: string; // Stored in memory for dev convenience (not persisted)
}

interface HeadlessStatus {
  running: boolean;
  port: number | null;
}

function parseLogLevel(line: string): "info" | "warning" | "error" | "debug" {
  const lower = line.toLowerCase();
  if (lower.includes("[error]") || lower.includes("error:")) return "error";
  if (lower.includes("[warn") || lower.includes("warning")) return "warning";
  if (lower.includes("[debug]")) return "debug";
  return "info";
}

function stripAnsi(str: string): string {
  return str.replace(/\x1B\[[0-9;]*[mK]/g, "");
}

function App() {
  const [currentPage, setCurrentPage] = useState<PageType>("dashboard");
  const [selectedApi, setSelectedApi] = useState<"fullnode" | "wallet">("fullnode");
  const [nodeStatus, setNodeStatus] = useState<NodeStatusType>("stopped");
  const [minerStatus, setMinerStatus] = useState<MinerStatusType>("stopped");
  const [blockHeight, setBlockHeight] = useState(0);
  const [hashRate, setHashRate] = useState("0 H/s");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [logFilters, setLogFilters] = useState<Set<LogSource>>(new Set(["node", "miner", "headless"]));
  const logsEndRef = useRef<HTMLDivElement>(null);
  const logIdRef = useRef(0);

  const toggleLogFilter = (source: LogSource) => {
    setLogFilters((prev) => {
      const next = new Set(prev);
      if (next.has(source)) {
        next.delete(source);
      } else {
        next.add(source);
      }
      return next;
    });
  };

  const addLog = (source: LogSource, message: string) => {
    const cleanMessage = stripAnsi(message);
    if (!cleanMessage.trim()) return;

    const entry: LogEntry = {
      id: logIdRef.current++,
      timestamp: new Date(),
      source,
      level: parseLogLevel(cleanMessage),
      message: cleanMessage,
    };
    setLogs((prev) => [...prev.slice(-1000), entry]);
  };

  // Auto-scroll logs only on logs page
  useEffect(() => {
    if (currentPage === "logs") {
      logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, currentPage]);

  // Poll node status and faucet balance when running
  useEffect(() => {
    if (nodeStatus !== "running") return;

    const fetchStatus = async () => {
      try {
        const status = await invoke<NodeStatus>("get_node_status");
        if (status.block_height !== null) {
          setBlockHeight(status.block_height);
        }
      } catch (e) {
        console.error("Failed to get node status:", e);
      }

      // Also fetch faucet balance
      try {
        const balance = await invoke<{ available: number; locked: number }>("get_fullnode_balance");
        setFaucetBalance(balance);
      } catch (e) {
        console.error("Failed to fetch faucet balance:", e);
      }
    };

    // Fetch immediately
    fetchStatus();

    const interval = setInterval(fetchStatus, 3000);

    return () => clearInterval(interval);
  }, [nodeStatus]);

  // Listen for events from the backend
  useEffect(() => {
    const unlistenLog = listen<string>("node-log", (event) => {
      addLog("node", event.payload);
    });

    const unlistenError = listen<string>("node-error", (event) => {
      addLog("node", event.payload);
    });

    const unlistenTerminated = listen<number | null>("node-terminated", (event) => {
      setNodeStatus("stopped");
      setMinerStatus("stopped");
      if (event.payload !== 0 && event.payload !== null) {
        setError(`Node exited with code ${event.payload}`);
      }
    });

    const unlistenMinerLog = listen<string>("miner-log", (event) => {
      addLog("miner", event.payload);
    });

    const unlistenMinerStats = listen<string>("miner-stats", (event) => {
      // Match formats like "1423 khash/s" or "1.5 MH/s"
      const match = event.payload.match(/(\d+\.?\d*)\s*(k|M|G|T)?hash\/s/i);
      if (match) {
        const value = match[1];
        const unit = match[2] ? match[2].toUpperCase() + "H/s" : "H/s";
        setHashRate(`${value} ${unit}`);
      }
      addLog("miner", event.payload);
    });

    const unlistenMinerTerminated = listen<number | null>("miner-terminated", () => {
      setMinerStatus("stopped");
      setHashRate("0 H/s");
    });

    const unlistenHeadlessLog = listen<string>("headless-log", (event) => {
      addLog("headless", event.payload);
    });

    const unlistenHeadlessTerminated = listen<number | null>("headless-terminated", () => {
      setHeadlessStatus({ running: false, port: null });
      setHeadlessWallets([]);
    });

    return () => {
      unlistenLog.then((f) => f());
      unlistenError.then((f) => f());
      unlistenTerminated.then((f) => f());
      unlistenMinerLog.then((f) => f());
      unlistenMinerStats.then((f) => f());
      unlistenMinerTerminated.then((f) => f());
      unlistenHeadlessLog.then((f) => f());
      unlistenHeadlessTerminated.then((f) => f());
    };
  }, []);

  const handleStartNode = async () => {
    setError(null);
    setNodeStatus("starting");
    try {
      await invoke("start_node", { config: null });
      setNodeStatus("running");
      // Auto-start explorer server
      try {
        await invoke("start_explorer_server");
      } catch (e) {
        console.warn("Explorer server failed to start:", e);
      }
      // Auto-start wallet-headless service
      try {
        await invoke("start_headless", { config: null });
        setHeadlessStatus({ running: true, port: 8001 });
      } catch (e) {
        console.warn("Wallet-headless failed to start:", e);
      }
    } catch (e) {
      setError(String(e));
      setNodeStatus("error");
    }
  };

  const handleStopNode = async () => {
    try {
      await invoke("stop_miner").catch(() => {});
      await invoke("stop_headless").catch(() => {});
      await invoke("stop_explorer_server").catch(() => {});
      await invoke("stop_node");
      setNodeStatus("stopped");
      setMinerStatus("stopped");
      setHeadlessStatus({ running: false, port: null });
      setHeadlessWallets([]);
      setBlockHeight(0);
      setHashRate("0 H/s");
    } catch (e) {
      setError(String(e));
    }
  };

  const handleStartMiner = async () => {
    if (nodeStatus !== "running") return;
    setMinerStatus("starting");
    try {
      await invoke("start_miner", { config: null });
      setMinerStatus("mining");
    } catch (e) {
      setError(String(e));
      setMinerStatus("error");
    }
  };

  const handleStopMiner = async () => {
    try {
      await invoke("stop_miner");
      setMinerStatus("stopped");
      setHashRate("0 H/s");
    } catch (e) {
      setError(String(e));
    }
  };

  const isLoading = nodeStatus === "starting" || minerStatus === "starting";

  const navItems: { icon: typeof Activity; label: string; page: PageType }[] = [
    { icon: Activity, label: "Dashboard", page: "dashboard" },
    { icon: Compass, label: "Explorer", page: "explorer" },
    { icon: Wallet, label: "Wallet", page: "wallet" },
    { icon: Layers, label: "Blocks", page: "blocks" },
    { icon: FileText, label: "Transactions", page: "transactions" },
    { icon: Coins, label: "Tokens", page: "tokens" },
    { icon: Cpu, label: "Mining", page: "mining" },
    { icon: BookOpen, label: "API Explorer", page: "api-explorer" },
    { icon: Terminal, label: "Logs", page: "logs" },
    { icon: Settings, label: "Settings", page: "settings" },
  ];

  const getLogLevelStyle = (level: LogEntry["level"]) => {
    switch (level) {
      case "error":
        return "text-rose-400";
      case "warning":
        return "text-amber-400";
      case "debug":
        return "text-slate-500";
      default:
        return "text-emerald-400";
    }
  };

  const renderDashboard = () => (
    <>
      {/* Action Bar */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-2xl font-bold text-white">Dashboard</h2>
          <p className="text-sm text-slate-500 mt-1">Manage your local Hathor network</p>
        </div>
        <div className="flex gap-3">
          {nodeStatus === "stopped" || nodeStatus === "error" ? (
            <button
              onClick={handleStartNode}
              disabled={isLoading}
              className="flex items-center gap-2 px-5 py-2.5 rounded-lg bg-gradient-to-r from-emerald-500 to-emerald-600 text-white font-semibold text-sm shadow-lg shadow-emerald-500/25 hover:shadow-emerald-500/40 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {nodeStatus === "starting" ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Play className="w-4 h-4" />
              )}
              Start Network
            </button>
          ) : (
            <button
              onClick={handleStopNode}
              disabled={isLoading}
              className="flex items-center gap-2 px-5 py-2.5 rounded-lg bg-gradient-to-r from-rose-500 to-rose-600 text-white font-semibold text-sm shadow-lg shadow-rose-500/25 hover:shadow-rose-500/40 transition-all duration-200 disabled:opacity-50"
            >
              <Square className="w-4 h-4" />
              Stop Network
            </button>
          )}
          {nodeStatus === "running" && minerStatus === "stopped" && (
            <button
              onClick={handleStartMiner}
              disabled={isLoading}
              className="flex items-center gap-2 px-5 py-2.5 rounded-lg bg-slate-800 border border-slate-700 text-slate-200 font-semibold text-sm hover:bg-slate-700 transition-all duration-200 disabled:opacity-50"
            >
              {minerStatus === "starting" ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Cpu className="w-4 h-4" />
              )}
              Start Mining
            </button>
          )}
          {(minerStatus === "mining" || minerStatus === "starting") && (
            <button
              onClick={handleStopMiner}
              disabled={minerStatus === "starting"}
              className="flex items-center gap-2 px-5 py-2.5 rounded-lg bg-slate-800 border border-slate-700 text-slate-200 font-semibold text-sm hover:bg-slate-700 transition-all duration-200 disabled:opacity-50"
            >
              <Square className="w-4 h-4" />
              Stop Mining
            </button>
          )}
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-4 gap-4 mb-6">
        {[
          { icon: Layers, label: "Block Height", value: blockHeight.toLocaleString(), color: "amber" },
          { icon: Cpu, label: "Hash Rate", value: hashRate, color: "emerald" },
          { icon: FileText, label: "Transactions", value: "0", color: "blue" },
          { icon: Coins, label: "Tokens", value: "1", sublabel: "HTR", color: "purple" },
        ].map((stat) => (
          <div
            key={stat.label}
            className="rounded-xl bg-[#0d1117] border border-slate-800/50 p-5 hover:border-slate-700/50 transition-colors"
          >
            <div className="flex items-center gap-2 mb-3">
              <stat.icon className={`w-4 h-4 text-${stat.color}-400`} />
              <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider">{stat.label}</span>
            </div>
            <div className="text-3xl font-bold text-white font-mono">{stat.value}</div>
            {stat.sublabel && <span className="text-xs text-slate-500 font-medium">{stat.sublabel}</span>}
          </div>
        ))}
      </div>

      {/* Recent Blocks Section */}
      {nodeStatus === "running" && (
        <div className="rounded-xl bg-[#0d1117] border border-slate-800/50 overflow-hidden">
          <div className="px-5 py-4 border-b border-slate-800/50">
            <div className="flex items-center gap-3">
              <Layers className="w-4 h-4 text-amber-400" />
              <h3 className="text-sm font-semibold text-white">Recent Blocks</h3>
            </div>
          </div>
          <div className="p-4">
            <div className="space-y-2">
              {blockHeight > 0 ? (
                Array.from({ length: Math.min(blockHeight, 10) }, (_, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between p-3 rounded-lg bg-slate-900/50 border border-slate-800/30"
                  >
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-lg bg-amber-500/10 flex items-center justify-center">
                        <Layers className="w-4 h-4 text-amber-400" />
                      </div>
                      <span className="font-mono font-semibold text-white">Block #{blockHeight - i}</span>
                    </div>
                    {blockHeight - i === 0 && (
                      <span className="px-2 py-1 rounded text-[10px] font-bold bg-amber-500/20 text-amber-400 uppercase">
                        Genesis
                      </span>
                    )}
                  </div>
                ))
              ) : (
                <div className="flex items-center justify-between p-3 rounded-lg bg-slate-900/50 border border-slate-800/30">
                  <div className="flex items-center gap-3">
                    <div className="w-8 h-8 rounded-lg bg-amber-500/10 flex items-center justify-center">
                      <Layers className="w-4 h-4 text-amber-400" />
                    </div>
                    <span className="font-mono font-semibold text-white">Block #0</span>
                  </div>
                  <span className="px-2 py-1 rounded text-[10px] font-bold bg-amber-500/20 text-amber-400 uppercase">
                    Genesis
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );

  const getSourceStyle = (source: LogSource) => {
    switch (source) {
      case "node":
        return "text-blue-400 bg-blue-400/10";
      case "miner":
        return "text-purple-400 bg-purple-400/10";
      case "headless":
        return "text-amber-400 bg-amber-400/10";
    }
  };

  const filteredLogs = logs.filter((log) => logFilters.has(log.source));

  const renderLogs = () => (
    <>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-2xl font-bold text-white">Logs</h2>
          <p className="text-sm text-slate-500 mt-1">Real-time service output</p>
        </div>
        <button
          onClick={() => setLogs([])}
          className="px-4 py-2 rounded-lg bg-slate-800 border border-slate-700 text-slate-300 text-sm font-medium hover:bg-slate-700 transition-colors"
        >
          Clear Logs
        </button>
      </div>

      <div className="rounded-xl bg-[#0d1117] border border-slate-800/50 overflow-hidden flex-1 flex flex-col">
        <div className="flex items-center justify-between px-5 py-4 border-b border-slate-800/50">
          <div className="flex items-center gap-3">
            <Terminal className="w-4 h-4 text-amber-400" />
            <h3 className="text-sm font-semibold text-white">Live Output</h3>
            <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-slate-800 text-slate-400">
              {filteredLogs.length} / {logs.length}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-slate-500 mr-2">Filter:</span>
            {(["node", "miner", "headless"] as LogSource[]).map((source) => (
              <button
                key={source}
                onClick={() => toggleLogFilter(source)}
                className={`px-3 py-1 rounded text-xs font-semibold uppercase transition-all ${
                  logFilters.has(source)
                    ? getSourceStyle(source)
                    : "text-slate-600 bg-slate-800/50 opacity-50"
                }`}
              >
                {source}
              </button>
            ))}
          </div>
        </div>
        <div className="flex-1 overflow-auto bg-[#080b10] p-4 font-mono text-sm min-h-0" style={{ maxHeight: 'calc(100vh - 280px)' }}>
          {filteredLogs.length > 0 ? (
            <div className="space-y-1">
              {filteredLogs.map((log) => (
                <div key={log.id} className="flex gap-3 leading-relaxed hover:bg-slate-900/30 px-2 py-1 rounded">
                  <span className="text-slate-600 text-xs shrink-0">
                    {log.timestamp.toLocaleTimeString()}
                  </span>
                  <span
                    className={`text-xs font-semibold uppercase shrink-0 w-16 px-1.5 py-0.5 rounded ${getSourceStyle(log.source)}`}
                  >
                    {log.source}
                  </span>
                  <span className={`${getLogLevelStyle(log.level)} break-all`}>{log.message}</span>
                </div>
              ))}
              <div ref={logsEndRef} />
            </div>
          ) : (
            <div className="h-full flex items-center justify-center text-slate-600">
              <div className="text-center">
                <Terminal className="w-8 h-8 mx-auto mb-2 opacity-50" />
                <p>{logs.length > 0 ? "No logs match the current filter." : "No logs yet. Start the network to see activity."}</p>
              </div>
            </div>
          )}
        </div>
      </div>
    </>
  );

  const renderExplorer = () => {
    if (nodeStatus !== "running") {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <Compass className="w-12 h-12 mx-auto mb-4 text-slate-600" />
            <h2 className="text-2xl font-bold text-white mb-2">Explorer</h2>
            <p className="text-slate-500 mb-4">Start the network to use the explorer</p>
            <button
              onClick={handleStartNode}
              disabled={isLoading}
              className="flex items-center gap-2 px-5 py-2.5 rounded-lg bg-gradient-to-r from-emerald-500 to-emerald-600 text-white font-semibold text-sm shadow-lg shadow-emerald-500/25 hover:shadow-emerald-500/40 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed mx-auto"
            >
              {nodeStatus === "starting" ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Play className="w-4 h-4" />
              )}
              Start Network
            </button>
          </div>
        </div>
      );
    }

    return (
      <div className="h-full flex flex-col -m-6">
        <iframe
          src="http://localhost:3001"
          className="w-full flex-1 border-0"
          title="Hathor Explorer"
        />
      </div>
    );
  };

  const renderPlaceholder = (title: string) => (
    <div className="flex items-center justify-center h-full">
      <div className="text-center">
        <h2 className="text-2xl font-bold text-white mb-2">{title}</h2>
        <p className="text-slate-500">Coming soon</p>
      </div>
    </div>
  );

  const [showResetConfirm, setShowResetConfirm] = useState(false);
  const [resetStatus, setResetStatus] = useState<"idle" | "resetting" | "success" | "error">("idle");
  const [resetMessage, setResetMessage] = useState("");

  const handleResetData = async () => {
    if (nodeStatus === "running") {
      setResetMessage("Stop the node before resetting data");
      setResetStatus("error");
      return;
    }

    setResetStatus("resetting");
    try {
      const result = await invoke<string>("reset_data");
      setResetMessage(result);
      setResetStatus("success");
      setShowResetConfirm(false);
    } catch (error) {
      setResetMessage(String(error));
      setResetStatus("error");
    }
  };

  const renderApiExplorer = () => (
    <div className="p-8 space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">API Explorer</h2>
        <p className="text-slate-500">Interactive API documentation powered by Swagger UI</p>
      </div>

      {/* API Selector Tabs */}
      <div className="flex gap-2 border-b border-slate-800/50 pb-4">
        <button
          onClick={() => setSelectedApi("fullnode")}
          className={`px-4 py-2 rounded-lg transition-colors ${
            selectedApi === "fullnode"
              ? "bg-amber-500/10 text-amber-400 border border-amber-500/30"
              : "bg-slate-900/50 text-slate-400 border border-slate-800 hover:bg-slate-900/80"
          }`}
        >
          Fullnode API (Port 8080)
        </button>
        <button
          onClick={() => setSelectedApi("wallet")}
          className={`px-4 py-2 rounded-lg transition-colors ${
            selectedApi === "wallet"
              ? "bg-amber-500/10 text-amber-400 border border-amber-500/30"
              : "bg-slate-900/50 text-slate-400 border border-slate-800 hover:bg-slate-900/80"
          }`}
        >
          Wallet Headless API (Port 8001)
        </button>
      </div>

      {/* Service Status Warnings */}
      {selectedApi === "fullnode" && nodeStatus !== "running" && (
        <div className="bg-amber-500/10 border border-amber-500/30 rounded-lg p-4 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-amber-400 mt-0.5 flex-shrink-0" />
          <div className="flex-1">
            <h4 className="font-medium text-amber-400 mb-1">Fullnode Not Running</h4>
            <p className="text-sm text-slate-400">
              The fullnode is not currently running. Start it from the Dashboard to test API endpoints.
            </p>
          </div>
        </div>
      )}

      {selectedApi === "wallet" && !headlessStatus.running && (
        <div className="bg-amber-500/10 border border-amber-500/30 rounded-lg p-4 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-amber-400 mt-0.5 flex-shrink-0" />
          <div className="flex-1">
            <h4 className="font-medium text-amber-400 mb-1">Wallet Headless Not Running</h4>
            <p className="text-sm text-slate-400">
              The wallet-headless service is not currently running. Start it from the Wallet page to test API endpoints.
            </p>
          </div>
        </div>
      )}

      {/* Swagger UI Component */}
      <div className="bg-[#0d1117] border border-slate-800/50 rounded-xl overflow-hidden">
        <SwaggerUIComponent apiType={selectedApi} />
      </div>
    </div>
  );

  const renderSettings = () => (
    <div className="p-8 space-y-8">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">Settings</h2>
        <p className="text-slate-500">Configure your local development environment</p>
      </div>

      {/* Danger Zone */}
      <div className="border border-red-500/30 rounded-xl bg-red-500/5 p-6">
        <div className="flex items-center gap-3 mb-4">
          <AlertTriangle className="w-5 h-5 text-red-400" />
          <h3 className="text-lg font-semibold text-red-400">Danger Zone</h3>
        </div>

        <div className="space-y-4">
          <div className="flex items-center justify-between p-4 bg-slate-900/50 rounded-lg border border-slate-800">
            <div>
              <h4 className="font-medium text-white">Reset Blockchain Data</h4>
              <p className="text-sm text-slate-500 mt-1">
                Delete all blockchain data and start fresh. This will remove all blocks, transactions, and wallet history.
              </p>
            </div>
            <button
              onClick={() => setShowResetConfirm(true)}
              disabled={nodeStatus === "running" || resetStatus === "resetting"}
              className="px-4 py-2 bg-red-500/10 text-red-400 border border-red-500/30 rounded-lg hover:bg-red-500/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              <Trash2 className="w-4 h-4" />
              Reset Data
            </button>
          </div>

          {resetStatus !== "idle" && (
            <div className={`p-3 rounded-lg text-sm ${
              resetStatus === "success" ? "bg-green-500/10 text-green-400 border border-green-500/30" :
              resetStatus === "error" ? "bg-red-500/10 text-red-400 border border-red-500/30" :
              "bg-slate-800 text-slate-400"
            }`}>
              {resetStatus === "resetting" ? "Resetting data..." : resetMessage}
            </div>
          )}
        </div>
      </div>

      {/* Reset Confirmation Modal */}
      {showResetConfirm && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
          <div className="bg-[#0d1117] border border-slate-800 rounded-xl p-6 max-w-md w-full mx-4 shadow-2xl">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 rounded-full bg-red-500/20 flex items-center justify-center">
                <AlertTriangle className="w-5 h-5 text-red-400" />
              </div>
              <h3 className="text-lg font-semibold text-white">Reset Blockchain Data?</h3>
            </div>
            <p className="text-slate-400 mb-6">
              This will permanently delete all blockchain data including blocks, transactions, and wallet history.
              You will need to mine from block 0 again. This action cannot be undone.
            </p>
            <div className="flex gap-3 justify-end">
              <button
                onClick={() => setShowResetConfirm(false)}
                className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleResetData}
                disabled={resetStatus === "resetting"}
                className="px-4 py-2 bg-red-500 text-white rounded-lg hover:bg-red-600 transition-colors disabled:opacity-50 flex items-center gap-2"
              >
                {resetStatus === "resetting" ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    Resetting...
                  </>
                ) : (
                  <>
                    <Trash2 className="w-4 h-4" />
                    Yes, Reset Data
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );

  const [walletAddresses, setWalletAddresses] = useState<WalletAddress[]>([]);
  const [loadingAddresses, setLoadingAddresses] = useState(false);
  const [copiedAddress, setCopiedAddress] = useState<string | null>(null);
  const [faucetAddress, setFaucetAddress] = useState("");
  const [faucetAmount, setFaucetAmount] = useState("100");
  const [faucetBalance, setFaucetBalance] = useState<{ available: number; locked: number } | null>(null);
  const [sendingTx, setSendingTx] = useState(false);
  const [txResult, setTxResult] = useState<{ type: "success" | "error"; message: string } | null>(null);

  // Headless wallet state
  const [headlessStatus, setHeadlessStatus] = useState<HeadlessStatus>({ running: false, port: null });
  const [headlessWallets, setHeadlessWallets] = useState<HeadlessWallet[]>([]);
  const [showCreateWallet, setShowCreateWallet] = useState(false);
  const [newSeed, setNewSeed] = useState<string | null>(null);
  const [newWalletId, setNewWalletId] = useState("");
  const [importSeed, setImportSeed] = useState("");
  const [creatingWallet, setCreatingWallet] = useState(false);
  const [expandedWallet, setExpandedWallet] = useState<string | null>(null);
  const [selectedWalletForSend, setSelectedWalletForSend] = useState<string | null>(null);

  const loadWalletAddresses = async () => {
    if (nodeStatus !== "running") return;

    setLoadingAddresses(true);
    try {
      const addresses = await invoke<WalletAddress[]>("get_wallet_addresses");
      setWalletAddresses(addresses);
    } catch (error) {
      console.error("Failed to load wallet addresses:", error);
    } finally {
      setLoadingAddresses(false);
    }
  };

  const copyAddress = async (address: string) => {
    await navigator.clipboard.writeText(address);
    setCopiedAddress(address);
    setTimeout(() => setCopiedAddress(null), 2000);
  };

  const fetchFaucetBalance = async () => {
    if (nodeStatus !== "running") return;
    try {
      const balance = await invoke<{ available: number; locked: number }>("get_fullnode_balance");
      setFaucetBalance(balance);
    } catch (e) {
      console.error("Failed to fetch faucet balance:", e);
    }
  };

  const handleSendTx = async () => {
    if (!faucetAddress || !faucetAmount) {
      setTxResult({ type: "error", message: "Please enter address and amount" });
      return;
    }

    setSendingTx(true);
    setTxResult(null);

    try {
      const amountInCents = Math.floor(parseFloat(faucetAmount) * 100);
      const result = await invoke<string>("send_tx", {
        request: {
          address: faucetAddress,
          amount: amountInCents,
        },
      });
      setTxResult({ type: "success", message: result });
      setFaucetAddress("");
      setFaucetAmount("100");
      // Reload addresses to update balances
      setTimeout(loadWalletAddresses, 1000);
    } catch (error) {
      setTxResult({ type: "error", message: String(error) });
    } finally {
      setSendingTx(false);
    }
  };

  // Headless wallet functions
  const checkHeadlessStatus = async () => {
    try {
      const status = await invoke<HeadlessStatus>("get_headless_status");
      setHeadlessStatus(status);
    } catch (e) {
      console.error("Failed to get headless status:", e);
    }
  };

  const startHeadless = async () => {
    try {
      await invoke("start_headless", { config: null });
      setHeadlessStatus({ running: true, port: 8001 });
    } catch (e) {
      setError(String(e));
    }
  };

  const stopHeadless = async () => {
    try {
      await invoke("stop_headless");
      setHeadlessStatus({ running: false, port: null });
      setHeadlessWallets([]);
    } catch (e) {
      setError(String(e));
    }
  };

  const generateNewSeed = async () => {
    try {
      const seed = await invoke<string>("generate_seed");
      setNewSeed(seed);
      setImportSeed("");
    } catch (e) {
      setError(String(e));
    }
  };

  const createWallet = async () => {
    const seed = newSeed || importSeed;
    if (!seed || !newWalletId) {
      setError("Please provide wallet ID and seed phrase");
      return;
    }

    setCreatingWallet(true);
    try {
      await invoke("create_headless_wallet", {
        request: {
          wallet_id: newWalletId,
          seed: seed,
        },
      });

      // Add to local state (store seed for dev convenience)
      setHeadlessWallets((prev) => [
        ...prev,
        {
          wallet_id: newWalletId,
          status: "starting",
          status_code: null,
          seed: seed,
        },
      ]);

      // Close modal and reset
      setShowCreateWallet(false);
      setNewSeed(null);
      setNewWalletId("");
      setImportSeed("");

      // Poll for wallet ready status
      pollWalletStatus(newWalletId);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreatingWallet(false);
    }
  };

  const pollWalletStatus = async (walletId: string) => {
    const maxAttempts = 30;
    for (let i = 0; i < maxAttempts; i++) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      try {
        const status = await invoke<HeadlessWallet>("get_headless_wallet_status", {
          walletId: walletId,
        });

        setHeadlessWallets((prev) =>
          prev.map((w) =>
            w.wallet_id === walletId
              ? { ...w, status: status.status, status_code: status.status_code }
              : w
          )
        );

        // Status code 3 means "Ready"
        if (status.status_code === 3) {
          // Load balance and addresses
          await loadWalletDetails(walletId);
          break;
        }
      } catch {
        break;
      }
    }
  };

  const loadWalletDetails = async (walletId: string) => {
    try {
      const [balance, addresses] = await Promise.all([
        invoke<{ available: number; locked: number }>("get_headless_wallet_balance", {
          walletId: walletId,
        }),
        invoke<string[]>("get_headless_wallet_addresses", { walletId: walletId }),
      ]);

      setHeadlessWallets((prev) =>
        prev.map((w) =>
          w.wallet_id === walletId ? { ...w, balance, addresses } : w
        )
      );
    } catch (e) {
      console.error("Failed to load wallet details:", e);
    }
  };

  const closeWallet = async (walletId: string) => {
    try {
      await invoke("close_headless_wallet", { walletId: walletId });
      setHeadlessWallets((prev) => prev.filter((w) => w.wallet_id !== walletId));
    } catch (e) {
      setError(String(e));
    }
  };

  const sendFromHeadlessWallet = async (walletId: string, address: string, amount: number) => {
    try {
      const result = await invoke<string>("headless_wallet_send_tx", {
        request: {
          wallet_id: walletId,
          address,
          amount: Math.floor(amount * 100), // Convert to cents
        },
      });
      setTxResult({ type: "success", message: result });
      // Reload wallet details
      await loadWalletDetails(walletId);
    } catch (e) {
      setTxResult({ type: "error", message: String(e) });
    }
  };

  const fundWallet = async (walletId: string) => {
    const wallet = headlessWallets.find((w) => w.wallet_id === walletId);
    if (!wallet?.addresses?.length) {
      setError("Wallet has no addresses. Wait for it to sync.");
      return;
    }

    // Fetch fresh balance right before sending to avoid race conditions
    let available = 0;
    try {
      const freshBalance = await invoke<{ available: number; locked: number }>("get_fullnode_balance");
      available = freshBalance.available;
      setFaucetBalance(freshBalance); // Update UI state too
    } catch {
      setError("Failed to fetch faucet balance. Try again.");
      return;
    }

    if (available <= 0) {
      setError("Faucet has no available funds. Wait for blocks to be mined and confirmed.");
      return;
    }

    // Use 10% of available, capped at 100 HTR (10000 cents), minimum 1 HTR (100 cents)
    const tenPercent = Math.floor(available * 0.1);
    const amount = Math.max(100, Math.min(tenPercent, 10000)); // Between 1 HTR and 100 HTR

    const firstAddress = wallet.addresses[0];
    try {
      await invoke("send_tx", { request: { address: firstAddress, amount } });
      setTxResult({ type: "success", message: `Sent ${(amount / 100).toFixed(2)} HTR to ${walletId}` });
      // Reload wallet details after a short delay
      setTimeout(() => loadWalletDetails(walletId), 1000);
    } catch (e) {
      setError(String(e));
    }
  };

  const copySeed = (wallet: HeadlessWallet) => {
    if (wallet.seed) {
      navigator.clipboard.writeText(wallet.seed);
      setCopiedAddress(wallet.wallet_id + "-seed"); // Reuse copiedAddress state for feedback
      setTimeout(() => setCopiedAddress(null), 2000);
    }
  };

  const renderWallet = () => {
    if (nodeStatus !== "running") {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <Wallet className="w-12 h-12 text-slate-600 mx-auto mb-4" />
            <h2 className="text-xl font-bold text-white mb-2">Node Not Running</h2>
            <p className="text-slate-500">Start the node to access wallets</p>
          </div>
        </div>
      );
    }

    return (
      <div className="p-8 space-y-8">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold text-white mb-2">Wallet Manager</h2>
            <p className="text-slate-500">Create and manage multiple wallets</p>
          </div>
          <div className="flex gap-3">
            {!headlessStatus.running ? (
              <button
                onClick={startHeadless}
                className="px-4 py-2 bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 rounded-lg hover:bg-emerald-500/20 transition-colors flex items-center gap-2"
              >
                <Play className="w-4 h-4" />
                Start Wallet Service
              </button>
            ) : (
              <>
                <button
                  onClick={() => setShowCreateWallet(true)}
                  className="px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors flex items-center gap-2"
                >
                  <Wallet className="w-4 h-4" />
                  New Wallet
                </button>
                <button
                  onClick={stopHeadless}
                  className="px-4 py-2 bg-slate-700 text-slate-300 rounded-lg hover:bg-slate-600 transition-colors flex items-center gap-2"
                >
                  <Square className="w-4 h-4" />
                  Stop Service
                </button>
              </>
            )}
          </div>
        </div>

        {/* Headless Status */}
        <div className={`p-4 rounded-lg border ${headlessStatus.running ? "bg-emerald-500/10 border-emerald-500/30" : "bg-slate-800/50 border-slate-700"}`}>
          <div className="flex items-center gap-3">
            <div className={`w-2 h-2 rounded-full ${headlessStatus.running ? "bg-emerald-400" : "bg-slate-500"}`} />
            <span className="text-sm font-medium text-slate-300">
              Wallet Service: {headlessStatus.running ? `Running on port ${headlessStatus.port}` : "Stopped"}
            </span>
          </div>
        </div>

        {/* Wallets List */}
        {headlessStatus.running && (
          <div className="space-y-4">
            {headlessWallets.length === 0 ? (
              <div className="border border-slate-800 rounded-xl bg-slate-900/30 p-8 text-center">
                <Wallet className="w-12 h-12 text-slate-600 mx-auto mb-4" />
                <p className="text-slate-500">No wallets yet. Click "New Wallet" to create one.</p>
              </div>
            ) : (
              headlessWallets.map((wallet) => (
                <div key={wallet.wallet_id} className="border border-slate-800 rounded-xl bg-slate-900/30 overflow-hidden">
                  <div className="p-4 border-b border-slate-800 bg-slate-900/50">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <Wallet className="w-5 h-5 text-amber-400" />
                        <span className="font-semibold text-white">{wallet.wallet_id}</span>
                        <span className={`px-2 py-0.5 rounded text-xs font-medium ${
                          wallet.status_code === 3 ? "bg-emerald-500/20 text-emerald-400" :
                          "bg-amber-500/20 text-amber-400"
                        }`}>
                          {wallet.status}
                        </span>
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => setExpandedWallet(expandedWallet === wallet.wallet_id ? null : wallet.wallet_id)}
                          className="px-3 py-1 text-sm text-slate-400 hover:text-white transition-colors"
                        >
                          {expandedWallet === wallet.wallet_id ? "Collapse" : "Expand"}
                        </button>
                        <button
                          onClick={() => fundWallet(wallet.wallet_id)}
                          disabled={wallet.status_code !== 3 || !faucetBalance?.available}
                          className="px-3 py-1 text-sm bg-emerald-500/10 text-emerald-400 rounded hover:bg-emerald-500/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                          title={!faucetBalance?.available ? "Wait for blocks to be mined" : "Send funds from faucet"}
                        >
                          Fund
                        </button>
                        {wallet.seed && (
                          <button
                            onClick={() => copySeed(wallet)}
                            className="px-3 py-1 text-sm bg-amber-500/10 text-amber-400 rounded hover:bg-amber-500/20 transition-colors flex items-center gap-1"
                          >
                            {copiedAddress === wallet.wallet_id + "-seed" ? (
                              <><Check className="w-3 h-3" /> Copied</>
                            ) : (
                              <><Copy className="w-3 h-3" /> Seed</>
                            )}
                          </button>
                        )}
                        <button
                          onClick={() => loadWalletDetails(wallet.wallet_id)}
                          className="px-3 py-1 text-sm bg-slate-700 text-slate-300 rounded hover:bg-slate-600 transition-colors"
                        >
                          Refresh
                        </button>
                        <button
                          onClick={() => closeWallet(wallet.wallet_id)}
                          className="px-3 py-1 text-sm bg-red-500/10 text-red-400 rounded hover:bg-red-500/20 transition-colors"
                        >
                          Close
                        </button>
                      </div>
                    </div>
                    {wallet.balance && (
                      <div className="mt-2 text-sm text-slate-400">
                        Balance: <span className="text-white font-semibold">{(wallet.balance.available / 100).toFixed(2)} HTR</span>
                        {wallet.balance.locked > 0 && (
                          <span className="text-amber-400 ml-2">({(wallet.balance.locked / 100).toFixed(2)} locked)</span>
                        )}
                      </div>
                    )}
                  </div>

                  {expandedWallet === wallet.wallet_id && (
                    <div className="p-4 space-y-4">
                      {/* Addresses */}
                      {wallet.addresses && wallet.addresses.length > 0 && (
                        <div>
                          <h4 className="text-sm font-medium text-slate-400 mb-2">Addresses</h4>
                          <div className="space-y-1 max-h-48 overflow-y-auto">
                            {wallet.addresses.slice(0, 10).map((addr, i) => (
                              <div key={addr} className="flex items-center gap-2 p-2 bg-slate-800/50 rounded">
                                <span className="text-xs text-slate-500">#{i}</span>
                                <code className="text-xs text-slate-300 font-mono flex-1 truncate">{addr}</code>
                                <button
                                  onClick={() => copyAddress(addr)}
                                  className="p-1 hover:bg-slate-700 rounded transition-colors"
                                >
                                  {copiedAddress === addr ? (
                                    <Check className="w-3 h-3 text-green-400" />
                                  ) : (
                                    <Copy className="w-3 h-3 text-slate-500" />
                                  )}
                                </button>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                      {/* Send from this wallet */}
                      {wallet.status_code === 3 && (
                        <div className="border-t border-slate-800 pt-4">
                          <h4 className="text-sm font-medium text-slate-400 mb-2">Send HTR</h4>
                          <div className="flex gap-2">
                            <input
                              type="text"
                              placeholder="Destination address"
                              className="flex-1 px-3 py-2 bg-slate-800 border border-slate-700 rounded text-sm text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
                              id={`send-addr-${wallet.wallet_id}`}
                            />
                            <input
                              type="number"
                              placeholder="Amount"
                              className="w-24 px-3 py-2 bg-slate-800 border border-slate-700 rounded text-sm text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
                              id={`send-amount-${wallet.wallet_id}`}
                            />
                            <button
                              onClick={() => {
                                const addrEl = document.getElementById(`send-addr-${wallet.wallet_id}`) as HTMLInputElement;
                                const amountEl = document.getElementById(`send-amount-${wallet.wallet_id}`) as HTMLInputElement;
                                if (addrEl?.value && amountEl?.value) {
                                  sendFromHeadlessWallet(wallet.wallet_id, addrEl.value, parseFloat(amountEl.value));
                                }
                              }}
                              className="px-4 py-2 bg-amber-500 text-white rounded hover:bg-amber-600 transition-colors flex items-center gap-1"
                            >
                              <Send className="w-4 h-4" />
                              Send
                            </button>
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        )}

        {/* Transaction Result */}
        {txResult && (
          <div className={`p-3 rounded-lg text-sm ${
            txResult.type === "success"
              ? "bg-green-500/10 text-green-400 border border-green-500/30"
              : "bg-red-500/10 text-red-400 border border-red-500/30"
          }`}>
            {txResult.message}
          </div>
        )}

        {/* Fullnode Wallet (Faucet) */}
        <div className="border border-amber-500/30 rounded-xl bg-amber-500/5 p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-3">
              <Send className="w-5 h-5 text-amber-400" />
              <h3 className="text-lg font-semibold text-amber-400">Fullnode Faucet</h3>
              <span className="text-xs text-slate-500">(Send HTR from fullnode's built-in wallet)</span>
            </div>
            {faucetBalance && (
              <div className="text-sm">
                <span className="text-slate-400">Available: </span>
                <span className="text-amber-400 font-semibold">{(faucetBalance.available / 100).toFixed(2)} HTR</span>
                {faucetBalance.locked > 0 && (
                  <span className="text-slate-500 ml-2">({(faucetBalance.locked / 100).toFixed(2)} locked)</span>
                )}
              </div>
            )}
          </div>

          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Destination Address
                </label>
                <input
                  type="text"
                  value={faucetAddress}
                  onChange={(e) => setFaucetAddress(e.target.value)}
                  placeholder="Enter Hathor address..."
                  className="w-full px-4 py-2 bg-slate-900 border border-slate-700 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Amount (HTR)
                </label>
                <input
                  type="number"
                  value={faucetAmount}
                  onChange={(e) => setFaucetAmount(e.target.value)}
                  min="0.01"
                  step="0.01"
                  className="w-full px-4 py-2 bg-slate-900 border border-slate-700 rounded-lg text-white focus:outline-none focus:border-amber-500/50"
                />
              </div>
            </div>

            <button
              onClick={handleSendTx}
              disabled={sendingTx || !faucetAddress || !faucetAmount}
              className="px-6 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              {sendingTx ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Sending...
                </>
              ) : (
                <>
                  <Send className="w-4 h-4" />
                  Send from Faucet
                </>
              )}
            </button>
          </div>
        </div>

        {/* Create Wallet Modal */}
        {showCreateWallet && (
          <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
            <div className="bg-[#0d1117] border border-slate-800 rounded-xl p-6 max-w-lg w-full mx-4 shadow-2xl">
              <div className="flex items-center gap-3 mb-6">
                <div className="w-10 h-10 rounded-full bg-amber-500/20 flex items-center justify-center">
                  <Wallet className="w-5 h-5 text-amber-400" />
                </div>
                <h3 className="text-lg font-semibold text-white">Create New Wallet</h3>
              </div>

              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">
                    Wallet ID
                  </label>
                  <input
                    type="text"
                    value={newWalletId}
                    onChange={(e) => setNewWalletId(e.target.value)}
                    placeholder="e.g., my-wallet"
                    className="w-full px-4 py-2 bg-slate-900 border border-slate-700 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
                  />
                </div>

                {/* Seed Options */}
                <div className="space-y-3">
                  <div className="flex gap-2">
                    <button
                      onClick={generateNewSeed}
                      className="flex-1 px-4 py-2 bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 rounded-lg hover:bg-emerald-500/20 transition-colors"
                    >
                      Generate New Seed
                    </button>
                    <button
                      onClick={() => { setNewSeed(null); }}
                      className={`flex-1 px-4 py-2 border rounded-lg transition-colors ${!newSeed ? "bg-amber-500/10 text-amber-400 border-amber-500/30" : "bg-slate-800 text-slate-400 border-slate-700"}`}
                    >
                      Import Existing
                    </button>
                  </div>

                  {newSeed ? (
                    <div className="p-4 bg-emerald-500/10 border border-emerald-500/30 rounded-lg">
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-sm font-medium text-emerald-400">Generated Seed (Save this!)</span>
                        <button
                          onClick={() => navigator.clipboard.writeText(newSeed)}
                          className="p-1 hover:bg-emerald-500/20 rounded transition-colors"
                        >
                          <Copy className="w-4 h-4 text-emerald-400" />
                        </button>
                      </div>
                      <p className="text-sm text-emerald-300 font-mono leading-relaxed break-all">{newSeed}</p>
                    </div>
                  ) : (
                    <div>
                      <label className="block text-sm font-medium text-slate-300 mb-2">
                        Seed Phrase (24 words)
                      </label>
                      <textarea
                        value={importSeed}
                        onChange={(e) => setImportSeed(e.target.value)}
                        placeholder="Enter your 24-word seed phrase..."
                        rows={3}
                        className="w-full px-4 py-2 bg-slate-900 border border-slate-700 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50 font-mono text-sm"
                      />
                    </div>
                  )}
                </div>
              </div>

              <div className="flex gap-3 justify-end mt-6">
                <button
                  onClick={() => {
                    setShowCreateWallet(false);
                    setNewSeed(null);
                    setNewWalletId("");
                    setImportSeed("");
                  }}
                  className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={createWallet}
                  disabled={creatingWallet || !newWalletId || (!newSeed && !importSeed)}
                  className="px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors disabled:opacity-50 flex items-center gap-2"
                >
                  {creatingWallet ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      Creating...
                    </>
                  ) : (
                    <>
                      <Wallet className="w-4 h-4" />
                      Create Wallet
                    </>
                  )}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    );
  };

  const renderContent = () => {
    switch (currentPage) {
      case "dashboard":
        return renderDashboard();
      case "explorer":
        return renderExplorer();
      case "wallet":
        return renderWallet();
      case "logs":
        return renderLogs();
      case "blocks":
        return renderPlaceholder("Blocks");
      case "transactions":
        return renderPlaceholder("Transactions");
      case "tokens":
        return renderPlaceholder("Tokens");
      case "mining":
        return renderPlaceholder("Mining");
      case "api-explorer":
        return renderApiExplorer();
      case "settings":
        return renderSettings();
      default:
        return renderDashboard();
    }
  };

  return (
    <div className="flex h-screen bg-[#0a0e14] text-slate-200 font-sans">
      {/* Sidebar */}
      <aside className="w-72 border-r border-slate-800/50 bg-[#0d1117] flex flex-col">
        <div className="p-6 border-b border-slate-800/50">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-amber-500 to-orange-600 flex items-center justify-center">
              <Zap className="w-5 h-5 text-white" />
            </div>
            <div>
              <h1 className="text-xl font-bold tracking-tight text-white">Hathor Forge</h1>
              <p className="text-xs text-slate-500 font-medium tracking-wide uppercase">Local Development</p>
            </div>
          </div>
        </div>

        <nav className="flex-1 p-4 space-y-1">
          {navItems.map((item) => (
            <button
              key={item.label}
              onClick={() => setCurrentPage(item.page)}
              className={`flex w-full items-center gap-3 rounded-lg px-4 py-3 text-sm font-medium transition-all duration-200 ${
                currentPage === item.page
                  ? "bg-amber-500/10 text-amber-400 border border-amber-500/20"
                  : "text-slate-400 hover:bg-slate-800/50 hover:text-slate-200 border border-transparent"
              }`}
            >
              <item.icon className="h-4 w-4" />
              {item.label}
              {item.page === "logs" && logs.length > 0 && (
                <span className="ml-auto px-1.5 py-0.5 rounded text-[10px] font-bold bg-slate-700 text-slate-300">
                  {logs.length}
                </span>
              )}
            </button>
          ))}
        </nav>

        {/* Network Status Card */}
        <div className="p-4 border-t border-slate-800/50">
          <div className="rounded-xl bg-slate-900/50 border border-slate-800/50 p-4">
            <div className="flex items-center justify-between mb-3">
              <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider">Network</span>
              <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/20 text-amber-400 uppercase tracking-wide">
                Localnet
              </span>
            </div>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-slate-500">RPC</span>
                <span className="font-mono text-slate-300">127.0.0.1:8080</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Stratum</span>
                <span className="font-mono text-slate-300">127.0.0.1:8000</span>
              </div>
            </div>
          </div>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* Top Bar */}
        <header className="h-16 border-b border-slate-800/50 bg-[#0d1117] flex items-center justify-between px-6">
          <div className="flex items-center gap-8">
            <div className="flex items-center gap-3">
              <Database className="w-4 h-4 text-slate-500" />
              <div>
                <span className="text-[10px] font-semibold text-slate-500 uppercase tracking-wider block">Block Height</span>
                <span className="text-lg font-bold font-mono text-white">{blockHeight.toLocaleString()}</span>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <Cpu className="w-4 h-4 text-slate-500" />
              <div>
                <span className="text-[10px] font-semibold text-slate-500 uppercase tracking-wider block">Hash Rate</span>
                <span className="text-lg font-bold font-mono text-white">{hashRate}</span>
              </div>
            </div>
          </div>

          <div className="flex items-center gap-4">
            {/* Node Status */}
            <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-800/50">
              <div
                className={`w-2 h-2 rounded-full ${
                  nodeStatus === "running"
                    ? "bg-emerald-400 shadow-lg shadow-emerald-400/50"
                    : nodeStatus === "starting"
                    ? "bg-amber-400 animate-pulse"
                    : nodeStatus === "error"
                    ? "bg-rose-400"
                    : "bg-slate-600"
                }`}
              />
              <span className="text-xs font-semibold text-slate-400 uppercase tracking-wide">Node</span>
              <span
                className={`text-xs font-bold uppercase tracking-wide ${
                  nodeStatus === "running"
                    ? "text-emerald-400"
                    : nodeStatus === "starting"
                    ? "text-amber-400"
                    : nodeStatus === "error"
                    ? "text-rose-400"
                    : "text-slate-500"
                }`}
              >
                {nodeStatus}
              </span>
            </div>

            {/* Miner Status */}
            <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-800/50">
              <div
                className={`w-2 h-2 rounded-full ${
                  minerStatus === "mining"
                    ? "bg-emerald-400 shadow-lg shadow-emerald-400/50"
                    : minerStatus === "starting"
                    ? "bg-amber-400 animate-pulse"
                    : minerStatus === "error"
                    ? "bg-rose-400"
                    : "bg-slate-600"
                }`}
              />
              <span className="text-xs font-semibold text-slate-400 uppercase tracking-wide">Miner</span>
              <span
                className={`text-xs font-bold uppercase tracking-wide ${
                  minerStatus === "mining"
                    ? "text-emerald-400"
                    : minerStatus === "starting"
                    ? "text-amber-400"
                    : minerStatus === "error"
                    ? "text-rose-400"
                    : "text-slate-500"
                }`}
              >
                {minerStatus}
              </span>
            </div>
          </div>
        </header>

        {/* Error Banner */}
        {error && (
          <div className="bg-rose-500/10 border-b border-rose-500/30 px-6 py-3">
            <p className="text-sm text-rose-400 font-medium">{error}</p>
          </div>
        )}

        {/* Page Content */}
        <div className="flex-1 overflow-auto p-6">
          {renderContent()}
        </div>
      </main>
    </div>
  );
}

export default App;
