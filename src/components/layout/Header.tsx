import { Cpu, Database } from "lucide-react";
import { useNodeStore } from "@/store/useNodeStore";

export function Header() {
  const { nodeStatus, minerStatus, blockHeight, hashRate } = useNodeStore();

  return (
    <header className="h-16 border-b border-slate-800/50 bg-[#0d1117] flex items-center justify-between px-6" role="banner">
      <div className="flex items-center gap-8">
        <div className="flex items-center gap-3">
          <Database className="w-4 h-4 text-slate-500" aria-hidden="true" />
          <div>
            <span className="text-[10px] font-semibold text-slate-500 uppercase tracking-wider block">Block Height</span>
            <span className="text-lg font-bold font-mono text-white">{blockHeight.toLocaleString()}</span>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <Cpu className="w-4 h-4 text-slate-500" aria-hidden="true" />
          <div>
            <span className="text-[10px] font-semibold text-slate-500 uppercase tracking-wider block">Hash Rate</span>
            <span className="text-lg font-bold font-mono text-white">{hashRate}</span>
          </div>
        </div>
      </div>

      <div className="flex items-center gap-4">
        {/* Node Status */}
        <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-800/50" role="status" aria-live="polite" aria-label={`Node status: ${nodeStatus}`}>
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
            aria-hidden="true"
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
        <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-800/50" role="status" aria-live="polite" aria-label={`Miner status: ${minerStatus}`}>
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
            aria-hidden="true"
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
  );
}
