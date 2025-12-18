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
} from "lucide-react";

type NodeStatusType = "stopped" | "starting" | "running" | "error";
type MinerStatusType = "stopped" | "starting" | "mining" | "error";
type PageType = "dashboard" | "explorer" | "blocks" | "transactions" | "tokens" | "mining" | "logs" | "settings";

interface NodeStatus {
  running: boolean;
  block_height: number | null;
  hash_rate: number | null;
  peer_count: number | null;
}

interface LogEntry {
  id: number;
  timestamp: Date;
  source: "node" | "miner";
  level: "info" | "warning" | "error" | "debug";
  message: string;
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
  const [nodeStatus, setNodeStatus] = useState<NodeStatusType>("stopped");
  const [minerStatus, setMinerStatus] = useState<MinerStatusType>("stopped");
  const [blockHeight, setBlockHeight] = useState(0);
  const [hashRate, setHashRate] = useState("0 H/s");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const logIdRef = useRef(0);

  const addLog = (source: "node" | "miner", message: string) => {
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

  // Poll node status when running
  useEffect(() => {
    if (nodeStatus !== "running") return;

    const interval = setInterval(async () => {
      try {
        const status = await invoke<NodeStatus>("get_node_status");
        if (status.block_height !== null) {
          setBlockHeight(status.block_height);
        }
      } catch (e) {
        console.error("Failed to get node status:", e);
      }
    }, 2000);

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

    return () => {
      unlistenLog.then((f) => f());
      unlistenError.then((f) => f());
      unlistenTerminated.then((f) => f());
      unlistenMinerLog.then((f) => f());
      unlistenMinerStats.then((f) => f());
      unlistenMinerTerminated.then((f) => f());
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
    } catch (e) {
      setError(String(e));
      setNodeStatus("error");
    }
  };

  const handleStopNode = async () => {
    try {
      await invoke("stop_miner").catch(() => {});
      await invoke("stop_explorer_server").catch(() => {});
      await invoke("stop_node");
      setNodeStatus("stopped");
      setMinerStatus("stopped");
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
    { icon: Layers, label: "Blocks", page: "blocks" },
    { icon: FileText, label: "Transactions", page: "transactions" },
    { icon: Coins, label: "Tokens", page: "tokens" },
    { icon: Cpu, label: "Mining", page: "mining" },
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

  const renderLogs = () => (
    <>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-2xl font-bold text-white">Logs</h2>
          <p className="text-sm text-slate-500 mt-1">Real-time node and miner output</p>
        </div>
        <button
          onClick={() => setLogs([])}
          className="px-4 py-2 rounded-lg bg-slate-800 border border-slate-700 text-slate-300 text-sm font-medium hover:bg-slate-700 transition-colors"
        >
          Clear Logs
        </button>
      </div>

      <div className="rounded-xl bg-[#0d1117] border border-slate-800/50 overflow-hidden flex-1 flex flex-col">
        <div className="flex items-center gap-3 px-5 py-4 border-b border-slate-800/50">
          <Terminal className="w-4 h-4 text-amber-400" />
          <h3 className="text-sm font-semibold text-white">Live Output</h3>
          <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-slate-800 text-slate-400">
            {logs.length} entries
          </span>
        </div>
        <div className="flex-1 overflow-auto bg-[#080b10] p-4 font-mono text-sm min-h-0" style={{ maxHeight: 'calc(100vh - 280px)' }}>
          {logs.length > 0 ? (
            <div className="space-y-1">
              {logs.map((log) => (
                <div key={log.id} className="flex gap-3 leading-relaxed hover:bg-slate-900/30 px-2 py-1 rounded">
                  <span className="text-slate-600 text-xs shrink-0">
                    {log.timestamp.toLocaleTimeString()}
                  </span>
                  <span
                    className={`text-xs font-semibold uppercase shrink-0 w-12 ${
                      log.source === "miner" ? "text-purple-400" : "text-blue-400"
                    }`}
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
                <p>No logs yet. Start the network to see activity.</p>
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

  const renderContent = () => {
    switch (currentPage) {
      case "dashboard":
        return renderDashboard();
      case "explorer":
        return renderExplorer();
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
