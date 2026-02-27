import {
  Cpu, Play, Square, Layers, Coins, FileText,
} from "lucide-react";
import { useAppStore } from "@/store/useAppStore";
import { useWalletStore } from "@/store/useWalletStore";
import * as api from "@/services/tauri";

export function DashboardPage() {
  const {
    nodeStatus, setNodeStatus,
    minerStatus, setMinerStatus,
    blockHeight, setBlockHeight,
    hashRate, setHashRate,
    setError,
  } = useAppStore();
  const { setHeadlessStatus } = useWalletStore();

  const isLoading = nodeStatus === "starting" || minerStatus === "starting";

  const handleStartNode = async () => {
    setError(null);
    setNodeStatus("starting");
    try {
      await api.startNode();
      setNodeStatus("running");
      try {
        await api.startExplorerServer();
      } catch (e) {
        console.warn("Explorer server failed to start:", e);
      }
      try {
        await api.startHeadless();
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
      await api.stopMiner().catch(() => {});
      await api.stopHeadless().catch(() => {});
      await api.stopExplorerServer().catch(() => {});
      await api.stopNode();
      setNodeStatus("stopped");
      setMinerStatus("stopped");
      setHeadlessStatus({ running: false, port: null });
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
      await api.startMiner();
      setMinerStatus("mining");
    } catch (e) {
      setError(String(e));
      setMinerStatus("error");
    }
  };

  const handleStopMiner = async () => {
    try {
      await api.stopMiner();
      setMinerStatus("stopped");
      setHashRate("0 H/s");
    } catch (e) {
      setError(String(e));
    }
  };

  return (
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
              <Play className="w-4 h-4" />
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
              <Cpu className="w-4 h-4" />
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
}
