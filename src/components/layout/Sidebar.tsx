import { useState, useEffect, useRef } from "react";
import {
  Activity, Settings, Terminal, Zap, Compass, Wallet, BookOpen, Copy, Check, Smartphone, X,
} from "lucide-react";
import { useUIStore } from "@/store/useUIStore";
import { usePortsStore } from "@/store/usePortsStore";
import { getLocalIp } from "@/services/tauri";
import type { PageType } from "@/types";

const navItems: { icon: typeof Activity; label: string; page: PageType }[] = [
  { icon: Activity, label: "Dashboard", page: "dashboard" },
  { icon: Compass, label: "Explorer", page: "explorer" },
  { icon: Wallet, label: "Wallet", page: "wallet" },
  { icon: Zap, label: "Nano Contracts", page: "nano-contracts" },
  { icon: BookOpen, label: "API Explorer", page: "api-explorer" },
  { icon: Terminal, label: "Logs", page: "logs" },
  { icon: Settings, label: "Settings", page: "settings" },
];

function CopyableValue({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <button
      onClick={copy}
      title="Copy to clipboard"
      className="flex items-center gap-1 font-mono text-[11px] text-white/35 hover:text-[#9cf35b] transition-colors duration-150 group"
    >
      <span className="truncate">{value}</span>
      {copied
        ? <Check className="w-2.5 h-2.5 text-[#9cf35b] shrink-0" />
        : <Copy className="w-2.5 h-2.5 opacity-0 group-hover:opacity-50 shrink-0 transition-opacity" />
      }
    </button>
  );
}

function WalletConnectModal({ localIp, onClose }: { localIp: string; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const PORTS = usePortsStore((s) => s.ports);

  useEffect(() => {
    function handler(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    }
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  const items = [
    { label: "Full Node", value: `http://${localIp}:${PORTS.FULLNODE_API}/v1a` },
    { label: "Tx Mining", value: `http://${localIp}:${PORTS.STRATUM}` },
    { label: "Explorer", value: `http://${localIp}:${PORTS.EXPLORER}` },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-end justify-start p-4" style={{ pointerEvents: "none" }}>
      <div
        ref={ref}
        style={{ pointerEvents: "auto", background: "linear-gradient(135deg, #0a1628 0%, #060d1f 100%)" }}
        className="w-72 rounded-xl border border-[#9cf35b]/15 shadow-2xl shadow-black/80 p-4 mb-16 ml-1"
      >
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <div className="w-1.5 h-1.5 rounded-full bg-[#9cf35b] shadow-[0_0_6px_#9cf35b]" />
            <span className="text-xs font-semibold text-white">Connect Mobile Wallet</span>
          </div>
          <button onClick={onClose} className="text-white/20 hover:text-white/60 transition-colors">
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
        <div className="space-y-3">
          {items.map(({ label, value }) => (
            <div key={label} className="flex items-center justify-between gap-2">
              <span className="text-[9px] font-bold text-white/25 uppercase tracking-[0.12em] shrink-0">{label}</span>
              <CopyableValue value={value} />
            </div>
          ))}
        </div>
        <p className="mt-4 text-[10px] text-white/20 leading-relaxed">Point Hathor mobile wallet at these URLs to connect to your local node.</p>
      </div>
    </div>
  );
}

export function Sidebar() {
  const { currentPage, setCurrentPage, unreadLogCount } = useUIStore();
  const PORTS = usePortsStore((s) => s.ports);
  const [localIp, setLocalIp] = useState<string>("...");
  const [showWalletConnect, setShowWalletConnect] = useState(false);

  useEffect(() => {
    getLocalIp().then(setLocalIp).catch(() => setLocalIp("127.0.0.1"));
  }, []);

  return (
    <aside
      className="w-[220px] border-r border-white/[0.05] flex flex-col"
      aria-label="Main navigation"
      style={{ background: "linear-gradient(180deg, #000f61 0%, #000c52 100%)" }}
    >
      {/* Logo — pt-8 clears macOS traffic lights */}
      <div className="px-4 pt-8 pb-4 border-b border-white/[0.05]">
        <div className="flex items-center gap-2.5">
          <img src="/app-icon.png" alt="Hathor Forge" className="w-8 h-8 rounded-lg" />
          <div>
            <h1 className="text-[13px] font-bold tracking-tight text-white leading-none">Hathor Forge</h1>
            <p className="text-[9px] text-white/25 font-medium tracking-[0.15em] uppercase mt-0.5">Local Dev</p>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-2 pt-3 space-y-px" aria-label="Main navigation">
        {navItems.map((item) => {
          const isActive = currentPage === item.page;
          return (
            <button
              key={item.label}
              onClick={() => setCurrentPage(item.page)}
              aria-current={isActive ? "page" : undefined}
              className="flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-[13px] font-medium transition-all duration-150 relative group"
              style={{
                background: isActive ? "#9cf35b" : "transparent",
                color: isActive ? "#000f61" : "rgba(255,255,255,0.4)",
              }}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.background = "rgba(255,255,255,0.06)";
                  e.currentTarget.style.color = "rgba(255,255,255,0.85)";
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "rgba(255,255,255,0.4)";
                }
              }}
            >
              <item.icon
                className="h-[15px] w-[15px] shrink-0"
                aria-hidden="true"
                style={{ opacity: isActive ? 1 : 0.7 }}
              />
              <span className={isActive ? "font-bold" : ""}>{item.label}</span>
              {item.page === "logs" && unreadLogCount > 0 && (
                <span
                  className="ml-auto px-1.5 py-0.5 rounded text-[9px] font-bold"
                  style={{
                    background: isActive ? "rgba(0,15,97,0.2)" : "rgba(156,243,91,0.15)",
                    color: isActive ? "#000f61" : "#9cf35b",
                  }}
                >
                  {unreadLogCount}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      {/* Network card */}
      <div className="p-2.5 border-t border-white/[0.05]">
        <div
          className="rounded-lg border border-white/[0.06] p-3 space-y-3"
          style={{ background: "rgba(0,0,0,0.35)" }}
        >
          {/* Header row */}
          <div className="flex items-center justify-between">
            <span className="text-[9px] font-bold text-white/25 uppercase tracking-[0.15em]">Network</span>
            <span
              className="px-1.5 py-0.5 rounded text-[9px] font-bold uppercase tracking-wide"
              style={{
                background: "rgba(156,243,91,0.12)",
                color: "#9cf35b",
                border: "1px solid rgba(156,243,91,0.2)",
              }}
            >
              Localnet
            </span>
          </div>

          {/* Endpoints */}
          <div className="space-y-1.5">
            <div className="flex justify-between items-center gap-2">
              <span className="text-[9px] font-bold text-white/20 uppercase tracking-[0.1em] shrink-0">RPC</span>
              <CopyableValue value={`127.0.0.1:${PORTS.FULLNODE_API}`} />
            </div>
            <div className="flex justify-between items-center gap-2">
              <span className="text-[9px] font-bold text-white/20 uppercase tracking-[0.1em] shrink-0">Stratum</span>
              <CopyableValue value={`127.0.0.1:${PORTS.STRATUM}`} />
            </div>
          </div>

          {/* Connect wallet button */}
          <button
            onClick={() => setShowWalletConnect((v) => !v)}
            className="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-md text-[11px] font-medium transition-all duration-150"
            style={{
              background: showWalletConnect ? "rgba(156,243,91,0.08)" : "rgba(255,255,255,0.03)",
              border: `1px solid ${showWalletConnect ? "rgba(156,243,91,0.2)" : "rgba(255,255,255,0.06)"}`,
              color: showWalletConnect ? "#9cf35b" : "rgba(255,255,255,0.35)",
            }}
            onMouseEnter={(e) => {
              if (!showWalletConnect) {
                e.currentTarget.style.background = "rgba(255,255,255,0.06)";
                e.currentTarget.style.color = "rgba(255,255,255,0.7)";
              }
            }}
            onMouseLeave={(e) => {
              if (!showWalletConnect) {
                e.currentTarget.style.background = "rgba(255,255,255,0.03)";
                e.currentTarget.style.color = "rgba(255,255,255,0.35)";
              }
            }}
          >
            <Smartphone className="w-3 h-3 shrink-0" />
            <span>Connect Wallet</span>
          </button>
        </div>
      </div>

      {showWalletConnect && (
        <WalletConnectModal localIp={localIp} onClose={() => setShowWalletConnect(false)} />
      )}
    </aside>
  );
}
