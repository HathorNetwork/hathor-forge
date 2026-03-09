import { Cpu, Database } from "lucide-react";
import { useNodeStore } from "@/store/useNodeStore";

export function Header() {
  const { nodeStatus, minerStatus, blockHeight, hashRate } = useNodeStore();

  return (
    <header
      className="h-12 border-b border-white/[0.06] bg-[#080a10] flex items-center justify-between px-5"
      role="banner"
      style={{ boxShadow: "0 1px 0 rgba(156,243,91,0.04)" }}
    >
      {/* Live telemetry */}
      <div className="flex items-center gap-0">
        <div className="flex items-center gap-2.5 pr-5 border-r border-white/[0.06]">
          <Database className="w-3 h-3 text-[#9cf35b]/40" aria-hidden="true" />
          <div>
            <span className="text-[8px] font-bold text-white/20 uppercase tracking-[0.15em] block leading-none mb-0.5">
              Block Height
            </span>
            <span className="text-[13px] font-bold font-mono text-white leading-none tabular-nums">
              {blockHeight.toLocaleString()}
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2.5 pl-5">
          <Cpu className="w-3 h-3 text-[#9cf35b]/40" aria-hidden="true" />
          <div>
            <span className="text-[8px] font-bold text-white/20 uppercase tracking-[0.15em] block leading-none mb-0.5">
              Hash Rate
            </span>
            <span className="text-[13px] font-bold font-mono text-white leading-none tabular-nums">
              {hashRate}
            </span>
          </div>
        </div>
      </div>

      {/* Status capsules */}
      <div className="flex items-center gap-1.5">
        <StatusCapsule
          label="Node"
          status={nodeStatus}
          activeStatus="running"
          activeColor="#9cf35b"
          startingColor="#ff8000"
        />
        <StatusCapsule
          label="Miner"
          status={minerStatus}
          activeStatus="mining"
          activeColor="#9cf35b"
          startingColor="#ff8000"
        />
      </div>
    </header>
  );
}

function StatusCapsule({
  label,
  status,
  activeStatus,
  activeColor,
  startingColor,
}: {
  label: string;
  status: string;
  activeStatus: string;
  activeColor: string;
  startingColor: string;
}) {
  const isActive = status === activeStatus;
  const isStarting = status === "starting";
  const isError = status === "error";

  const color = isActive
    ? activeColor
    : isStarting
    ? startingColor
    : isError
    ? "#f43f5e"
    : "rgba(255,255,255,0.2)";

  const bgOpacity = isActive || isStarting ? "0.08" : isError ? "0.08" : "0.03";
  const borderOpacity = isActive || isStarting ? "0.25" : isError ? "0.25" : "0.08";

  return (
    <div
      className="flex items-center gap-2 px-2.5 py-1 rounded-md text-[9px] font-bold uppercase tracking-[0.12em] transition-all duration-300"
      style={{
        background: `rgba(${hexToRgb(color)}, ${bgOpacity})`,
        border: `1px solid rgba(${hexToRgb(color)}, ${borderOpacity})`,
        color: isActive || isStarting || isError ? color : "rgba(255,255,255,0.25)",
      }}
      role="status"
      aria-label={`${label} status: ${status}`}
    >
      <span
        className="w-1.5 h-1.5 rounded-full shrink-0"
        style={{
          background: color,
          boxShadow: isActive ? `0 0 6px ${color}, 0 0 12px ${color}40` : "none",
          animation: isStarting ? "pulse 1.5s ease-in-out infinite" : "none",
        }}
      />
      <span style={{ color: "rgba(255,255,255,0.35)" }}>{label}</span>
      <span>{status}</span>
    </div>
  );
}

function hexToRgb(hex: string): string {
  if (hex.startsWith("rgba")) return "255,255,255";
  const clean = hex.replace("#", "");
  const r = parseInt(clean.substring(0, 2), 16);
  const g = parseInt(clean.substring(2, 4), 16);
  const b = parseInt(clean.substring(4, 6), 16);
  return `${r},${g},${b}`;
}
