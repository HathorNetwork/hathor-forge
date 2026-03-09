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
      className="flex items-center gap-1 font-mono text-slate-300 hover:text-amber-400 transition-colors group"
    >
      <span>{value}</span>
      {copied
        ? <Check className="w-3 h-3 text-green-400 shrink-0" />
        : <Copy className="w-3 h-3 opacity-0 group-hover:opacity-60 shrink-0 transition-opacity" />
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
      <div ref={ref} style={{ pointerEvents: "auto" }} className="w-80 rounded-xl bg-[#0d1117] border border-slate-700/60 shadow-2xl p-5 mb-16 ml-2">
        <div className="flex items-center justify-between mb-4">
          <span className="text-sm font-semibold text-slate-200">Connect Wallet</span>
          <button onClick={onClose} className="text-slate-500 hover:text-slate-300 transition-colors">
            <X className="w-4 h-4" />
          </button>
        </div>
        <div className="space-y-3">
          {items.map(({ label, value }) => (
            <div key={label}>
              <span className="text-xs text-slate-500 uppercase tracking-wider">{label}</span>
              <div className="mt-1">
                <CopyableValue value={value} />
              </div>
            </div>
          ))}
        </div>
        <p className="mt-4 text-xs text-slate-600">Use these URLs to connect the Hathor mobile wallet to your local node.</p>
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
    <aside className="w-72 border-r border-slate-800/50 bg-[#0d1117] flex flex-col" aria-label="Main navigation">
      <div className="px-6 pt-8 pb-6 border-b border-slate-800/50">
        <div className="flex items-center gap-3">
          <img src="/app-icon.png" alt="Hathor Forge" className="w-10 h-10" />
          <div>
            <h1 className="text-xl font-bold tracking-tight text-white">Hathor Forge</h1>
            <p className="text-xs text-slate-500 font-medium tracking-wide uppercase">Local Development</p>
          </div>
        </div>
      </div>

      <nav className="flex-1 p-4 space-y-1" aria-label="Main navigation">
        {navItems.map((item) => (
          <button
            key={item.label}
            onClick={() => setCurrentPage(item.page)}
            aria-current={currentPage === item.page ? "page" : undefined}
            className={`flex w-full items-center gap-3 rounded-lg px-4 py-3 text-sm font-medium transition-all duration-200 ${
              currentPage === item.page
                ? "bg-amber-500/10 text-amber-400 border border-amber-500/20"
                : "text-slate-400 hover:bg-slate-800/50 hover:text-slate-200 border border-transparent"
            }`}
          >
            <item.icon className="h-4 w-4" aria-hidden="true" />
            {item.label}
            {item.page === "logs" && unreadLogCount > 0 && (
              <span className="ml-auto px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-500/20 text-amber-400" aria-label={`${unreadLogCount} unread logs`}>
                {unreadLogCount}
              </span>
            )}
          </button>
        ))}
      </nav>

      {/* Network Status Card */}
      <div className="p-4 border-t border-slate-800/50">
        <div className="rounded-xl bg-slate-900/50 border border-slate-800/50 p-4 space-y-4">
          {/* Local services */}
          <div>
            <div className="flex items-center justify-between mb-3">
              <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider">Network</span>
              <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/20 text-amber-400 uppercase tracking-wide">
                Localnet
              </span>
            </div>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between items-center">
                <span className="text-slate-500">RPC</span>
                <CopyableValue value={`127.0.0.1:${PORTS.FULLNODE_API}`} />
              </div>
              <div className="flex justify-between items-center">
                <span className="text-slate-500">Stratum</span>
                <CopyableValue value={`127.0.0.1:${PORTS.STRATUM}`} />
              </div>
            </div>
          </div>

          {/* Mobile wallet connect */}
          <div className="border-t border-slate-800/50 pt-3">
            <button
              onClick={() => setShowWalletConnect((v) => !v)}
              className="w-full flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium text-slate-400 hover:bg-slate-800/50 hover:text-slate-200 border border-slate-800/50 hover:border-slate-700/50 transition-all"
            >
              <Smartphone className="w-4 h-4 shrink-0" />
              <span>Connect Wallet</span>
            </button>
          </div>
          {showWalletConnect && (
            <WalletConnectModal localIp={localIp} onClose={() => setShowWalletConnect(false)} />
          )}
        </div>
      </div>
    </aside>
  );
}
