import { useState } from "react";
import { Play, Compass, Loader2 } from "lucide-react";
import { useNodeStore } from "@/store/useNodeStore";
import { useStartNetwork } from "@/hooks/useStartNetwork";
import { usePortsStore } from "@/store/usePortsStore";

export function ExplorerPage() {
  const nodeStatus = useNodeStore((s) => s.nodeStatus);
  const { startNetwork, isLoading } = useStartNetwork();
  const PORTS = usePortsStore((s) => s.ports);
  // Cache-busting: unique key per mount so WebKit doesn't serve stale iframe content
  const [cacheKey] = useState(() => Date.now());

  const handleStartNode = startNetwork;

  if (nodeStatus !== "running") {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <Compass className="w-12 h-12 mx-auto mb-4 text-white/15" />
          <h2 className="text-2xl font-bold text-white mb-2">Explorer</h2>
          <p className="text-white/30 mb-4">Start the network to use the explorer</p>
          <button
            onClick={handleStartNode}
            disabled={isLoading}
            className="flex items-center gap-2 px-5 py-2.5 rounded-lg bg-[#9cf35b] text-[#000f61] font-bold text-sm shadow-lg shadow-[#9cf35b]/25 hover:bg-[#bff658] transition-colors disabled:opacity-50 disabled:cursor-not-allowed mx-auto"
          >
            {isLoading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
            Start Network
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col -m-6" style={{ height: 'calc(100% + 48px)' }}>
      <iframe
        src={`http://localhost:${PORTS.EXPLORER}?_cb=${cacheKey}`}
        className="w-full flex-1 border-0"
        title="Hathor Explorer"
      />
    </div>
  );
}
