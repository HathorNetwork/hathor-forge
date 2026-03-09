import {
  Activity, Settings, Terminal, Zap, Compass, Wallet, BookOpen,
} from "lucide-react";
import { useUIStore } from "@/store/useUIStore";
import { PORTS } from "@/lib/constants";
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

export function Sidebar() {
  const { currentPage, setCurrentPage, unreadLogCount } = useUIStore();

  return (
    <aside className="w-72 border-r border-slate-800/50 bg-[#0d1117] flex flex-col" aria-label="Main navigation">
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
              <span className="font-mono text-slate-300">127.0.0.1:{PORTS.FULLNODE_API}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-500">Stratum</span>
              <span className="font-mono text-slate-300">127.0.0.1:{PORTS.STRATUM}</span>
            </div>
          </div>
        </div>
      </div>
    </aside>
  );
}
