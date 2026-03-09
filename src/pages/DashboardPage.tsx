import {
  Cpu, Play, Square, Layers, Coins, FileText, Activity,
} from "lucide-react";
import { useNodeStore } from "@/store/useNodeStore";
import { useUIStore } from "@/store/useUIStore";
import { useWalletStore } from "@/store/useWalletStore";
import { useStartNetwork } from "@/hooks/useStartNetwork";
import * as api from "@/services/tauri";

export function DashboardPage() {
  const nodeStatus = useNodeStore((s) => s.nodeStatus);
  const setNodeStatus = useNodeStore((s) => s.setNodeStatus);
  const minerStatus = useNodeStore((s) => s.minerStatus);
  const setMinerStatus = useNodeStore((s) => s.setMinerStatus);
  const blockHeight = useNodeStore((s) => s.blockHeight);
  const setBlockHeight = useNodeStore((s) => s.setBlockHeight);
  const hashRate = useNodeStore((s) => s.hashRate);
  const setHashRate = useNodeStore((s) => s.setHashRate);
  const setError = useUIStore((s) => s.setError);
  const { setHeadlessStatus } = useWalletStore();
  const { startNetwork, isLoading: isNetworkStarting } = useStartNetwork();

  const isLoading = isNetworkStarting || minerStatus === "starting";

  const handleStartNode = startNetwork;

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

  const networkIsLive = nodeStatus === "running" || nodeStatus === "starting";

  return (
    <>
      {/* ── Action Bar ─────────────────────────────────────────────── */}
      <div className="flex items-center justify-between mb-5">
        <div>
          <h2 className="text-[15px] font-bold text-white tracking-tight">Dashboard</h2>
          <p className="text-[10px] text-white/25 mt-0.5 uppercase tracking-[0.12em]">
            Local Hathor Network
          </p>
        </div>

        <div className="flex items-center gap-2">
          {nodeStatus === "stopped" || nodeStatus === "error" ? (
            <button
              onClick={handleStartNode}
              disabled={isLoading}
              className="flex items-center gap-2 px-4 py-1.5 rounded-lg text-[12px] font-bold transition-all duration-150 disabled:opacity-50 disabled:cursor-not-allowed"
              style={{
                background: "linear-gradient(135deg, #9cf35b 0%, #bff658 100%)",
                color: "#000f61",
                boxShadow: "0 0 20px rgba(156,243,91,0.3), 0 2px 8px rgba(0,0,0,0.4)",
              }}
            >
              <Play className="w-3 h-3" />
              Start Network
            </button>
          ) : (
            <button
              onClick={handleStopNode}
              disabled={isLoading}
              className="flex items-center gap-2 px-4 py-1.5 rounded-lg text-[12px] font-bold transition-all duration-150 disabled:opacity-40"
              style={{
                background: "rgba(244,63,94,0.08)",
                border: "1px solid rgba(244,63,94,0.3)",
                color: "#fb7185",
              }}
            >
              <Square className="w-3 h-3" />
              Stop Network
            </button>
          )}

          {nodeStatus === "running" && (minerStatus === "stopped" || minerStatus === "error") && (
            <button
              onClick={handleStartMiner}
              disabled={isLoading}
              className="flex items-center gap-2 px-4 py-1.5 rounded-lg text-[12px] font-semibold transition-all duration-150 disabled:opacity-50"
              style={{
                background: "rgba(255,255,255,0.04)",
                border: "1px solid rgba(255,255,255,0.08)",
                color: "rgba(255,255,255,0.55)",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "rgba(156,243,91,0.08)";
                e.currentTarget.style.borderColor = "rgba(156,243,91,0.2)";
                e.currentTarget.style.color = "#9cf35b";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "rgba(255,255,255,0.04)";
                e.currentTarget.style.borderColor = "rgba(255,255,255,0.08)";
                e.currentTarget.style.color = "rgba(255,255,255,0.55)";
              }}
            >
              <Cpu className="w-3 h-3" />
              Start Mining
            </button>
          )}

          {(minerStatus === "mining" || minerStatus === "starting") && (
            <button
              onClick={handleStopMiner}
              disabled={minerStatus === "starting"}
              className="flex items-center gap-2 px-4 py-1.5 rounded-lg text-[12px] font-semibold transition-all duration-150 disabled:opacity-50"
              style={{
                background: "rgba(255,255,255,0.04)",
                border: "1px solid rgba(255,255,255,0.08)",
                color: "rgba(255,255,255,0.55)",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "rgba(244,63,94,0.08)";
                e.currentTarget.style.borderColor = "rgba(244,63,94,0.25)";
                e.currentTarget.style.color = "#fb7185";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "rgba(255,255,255,0.04)";
                e.currentTarget.style.borderColor = "rgba(255,255,255,0.08)";
                e.currentTarget.style.color = "rgba(255,255,255,0.55)";
              }}
            >
              <Square className="w-3 h-3" />
              Stop Mining
            </button>
          )}
        </div>
      </div>

      {/* ── Stat Cards ─────────────────────────────────────────────── */}
      <div className="grid grid-cols-4 gap-3 mb-5">
        <StatCard
          icon={Layers}
          label="Block Height"
          value={blockHeight.toLocaleString()}
          accentColor="#9cf35b"
          live={minerStatus === "mining"}
        />
        <StatCard
          icon={Cpu}
          label="Hash Rate"
          value={hashRate}
          accentColor="#9cf35b"
          live={minerStatus === "mining"}
        />
        <StatCard
          icon={FileText}
          label="Transactions"
          value="0"
          accentColor="#9cf35b"
          live={false}
        />
        <StatCard
          icon={Coins}
          label="Tokens"
          value="1"
          sublabel="HTR"
          accentColor="#ff8000"
          live={false}
        />
      </div>

      {/* ── Recent Blocks ──────────────────────────────────────────── */}
      {networkIsLive && (
        <div
          className="rounded-xl overflow-hidden"
          style={{
            background: "#0d1117",
            border: "1px solid rgba(255,255,255,0.06)",
          }}
        >
          {/* Panel header */}
          <div
            className="px-4 py-3 flex items-center justify-between"
            style={{ borderBottom: "1px solid rgba(255,255,255,0.05)" }}
          >
            <div className="flex items-center gap-2">
              <Activity className="w-3.5 h-3.5 text-[#9cf35b]/60" />
              <span className="text-[10px] font-bold text-white/35 uppercase tracking-[0.15em]">
                Recent Blocks
              </span>
            </div>
            {minerStatus === "mining" && (
              <div className="flex items-center gap-1.5">
                <span
                  className="w-1.5 h-1.5 rounded-full bg-[#9cf35b]"
                  style={{
                    boxShadow: "0 0 6px #9cf35b",
                    animation: "pulse 2s ease-in-out infinite",
                  }}
                />
                <span className="text-[9px] font-bold text-[#9cf35b]/60 uppercase tracking-[0.12em]">
                  Live
                </span>
              </div>
            )}
          </div>

          {/* Block list */}
          <div className="divide-y divide-white/[0.03]">
            {blockHeight > 0 ? (
              Array.from({ length: Math.min(blockHeight, 10) }, (_, i) => {
                const num = blockHeight - i;
                const isLatest = i === 0;
                const isGenesis = num === 0;

                return (
                  <div
                    key={i}
                    className="flex items-center justify-between px-4 py-2.5 transition-colors duration-100 group"
                    style={{
                      background: isLatest
                        ? "linear-gradient(90deg, rgba(156,243,91,0.04) 0%, transparent 60%)"
                        : i % 2 === 0
                        ? "rgba(255,255,255,0.01)"
                        : "transparent",
                      borderLeft: isLatest ? "2px solid rgba(156,243,91,0.4)" : "2px solid transparent",
                    }}
                    onMouseEnter={(e) => {
                      if (!isLatest) e.currentTarget.style.background = "rgba(255,255,255,0.025)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = isLatest
                        ? "linear-gradient(90deg, rgba(156,243,91,0.04) 0%, transparent 60%)"
                        : i % 2 === 0
                        ? "rgba(255,255,255,0.01)"
                        : "transparent";
                    }}
                  >
                    <div className="flex items-center gap-3">
                      {/* Block icon */}
                      <div
                        className="w-6 h-6 rounded-md flex items-center justify-center shrink-0"
                        style={{
                          background: isLatest
                            ? "rgba(156,243,91,0.12)"
                            : "rgba(255,255,255,0.04)",
                        }}
                      >
                        <Layers
                          className="w-3 h-3"
                          style={{ color: isLatest ? "#9cf35b" : "rgba(255,255,255,0.2)" }}
                        />
                      </div>

                      {/* Block number */}
                      <span
                        className="font-mono text-[12px] font-semibold tabular-nums"
                        style={{
                          color: isLatest ? "rgba(255,255,255,0.9)" : "rgba(255,255,255,0.5)",
                        }}
                      >
                        #{num.toLocaleString()}
                      </span>

                      {isLatest && (
                        <span
                          className="text-[8px] font-bold uppercase tracking-[0.12em]"
                          style={{ color: "rgba(156,243,91,0.5)" }}
                        >
                          Latest
                        </span>
                      )}
                    </div>

                    {/* Right side */}
                    <div className="flex items-center gap-2">
                      {isGenesis && (
                        <span
                          className="px-2 py-0.5 rounded text-[8px] font-bold uppercase tracking-[0.1em]"
                          style={{
                            background: "rgba(156,243,91,0.1)",
                            color: "#9cf35b",
                            border: "1px solid rgba(156,243,91,0.2)",
                          }}
                        >
                          Genesis
                        </span>
                      )}
                      {isLatest && minerStatus === "mining" && (
                        <span
                          className="w-1.5 h-1.5 rounded-full"
                          style={{
                            background: "#9cf35b",
                            boxShadow: "0 0 6px #9cf35b",
                            animation: "pulse 1.5s ease-in-out infinite",
                          }}
                        />
                      )}
                    </div>
                  </div>
                );
              })
            ) : (
              <div className="flex items-center justify-between px-4 py-2.5">
                <div className="flex items-center gap-3">
                  <div className="w-6 h-6 rounded-md bg-[#9cf35b]/10 flex items-center justify-center">
                    <Layers className="w-3 h-3 text-[#9cf35b]" />
                  </div>
                  <span className="font-mono text-[12px] font-semibold text-white/50">#0</span>
                </div>
                <span
                  className="px-2 py-0.5 rounded text-[8px] font-bold uppercase tracking-[0.1em]"
                  style={{
                    background: "rgba(156,243,91,0.1)",
                    color: "#9cf35b",
                    border: "1px solid rgba(156,243,91,0.2)",
                  }}
                >
                  Genesis
                </span>
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
}

// ── StatCard component ────────────────────────────────────────────────────

function StatCard({
  icon: Icon,
  label,
  value,
  sublabel,
  accentColor,
  live,
}: {
  icon: typeof Layers;
  label: string;
  value: string;
  sublabel?: string;
  accentColor: string;
  live: boolean;
}) {
  return (
    <div
      className="rounded-xl p-4 relative overflow-hidden transition-all duration-200"
      style={{
        background: "linear-gradient(135deg, #111520 0%, #0d1117 100%)",
        border: "1px solid rgba(255,255,255,0.06)",
      }}
      onMouseEnter={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = `rgba(${hexToRgb(accentColor)}, 0.2)`;
      }}
      onMouseLeave={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = "rgba(255,255,255,0.06)";
      }}
    >
      {/* Accent top border */}
      <div
        className="absolute top-0 left-0 right-0 h-px"
        style={{
          background: `linear-gradient(90deg, transparent, ${accentColor}${live ? "80" : "30"}, transparent)`,
        }}
      />

      {/* Ambient glow behind number when live */}
      {live && (
        <div
          className="absolute bottom-0 right-0 w-16 h-16 rounded-full pointer-events-none"
          style={{
            background: `radial-gradient(circle, ${accentColor}08 0%, transparent 70%)`,
            transform: "translate(25%, 25%)",
          }}
        />
      )}

      <div className="flex items-center gap-1.5 mb-3">
        <Icon
          className="w-3 h-3 shrink-0"
          style={{ color: `${accentColor}${live ? "cc" : "55"}` }}
        />
        <span className="text-[8px] font-bold text-white/25 uppercase tracking-[0.15em]">
          {label}
        </span>
        {live && (
          <span
            className="ml-auto w-1 h-1 rounded-full shrink-0"
            style={{
              background: accentColor,
              boxShadow: `0 0 4px ${accentColor}`,
              animation: "pulse 2s ease-in-out infinite",
            }}
          />
        )}
      </div>

      <div
        className="text-[28px] font-bold font-mono leading-none tabular-nums"
        style={{ color: live ? "rgba(255,255,255,0.95)" : "rgba(255,255,255,0.75)" }}
      >
        {value}
      </div>

      {sublabel && (
        <span className="text-[10px] text-white/25 font-medium mt-1.5 block uppercase tracking-wider">
          {sublabel}
        </span>
      )}
    </div>
  );
}

function hexToRgb(hex: string): string {
  const clean = hex.replace("#", "");
  const r = parseInt(clean.substring(0, 2), 16);
  const g = parseInt(clean.substring(2, 4), 16);
  const b = parseInt(clean.substring(4, 6), 16);
  return `${r},${g},${b}`;
}
