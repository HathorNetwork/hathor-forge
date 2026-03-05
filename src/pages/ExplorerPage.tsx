import { Play, Compass, Loader2 } from "lucide-react";
import { useAppStore } from "@/store/useAppStore";
import { useWalletStore } from "@/store/useWalletStore";
import * as api from "@/services/tauri";
import { PORTS } from "@/lib/constants";

export function ExplorerPage() {
  const { nodeStatus, setNodeStatus, setError } = useAppStore();
  const { setHeadlessStatus } = useWalletStore();
  const isLoading = nodeStatus === "starting";

  const handleStartNode = async () => {
    setError(null);
    setNodeStatus("starting");
    try {
      await api.startNode();
      setNodeStatus("running");
      try { await api.startExplorerServer(); } catch { /* best-effort */ }
      try {
        await api.startHeadless();
        setHeadlessStatus({ running: true, port: PORTS.WALLET_HEADLESS });
      } catch { /* best-effort */ }
    } catch (e) {
      setError(String(e));
      setNodeStatus("error");
    }
  };

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
            {isLoading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
            Start Network
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col -m-6">
      <iframe
        src={`http://localhost:${PORTS.EXPLORER}`}
        className="w-full flex-1 border-0"
        title="Hathor Explorer"
      />
    </div>
  );
}
