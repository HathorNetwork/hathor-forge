import { useState } from "react";
import { SwaggerUIComponent } from "@/components/SwaggerUI";
import 'swagger-ui-react/swagger-ui.css';

export function ApiExplorerPage() {
  const [selectedApi, setSelectedApi] = useState<"fullnode" | "wallet">("fullnode");

  return (
    <div className="p-8 space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">API Explorer</h2>
        <p className="text-slate-500">Interactive API documentation powered by Swagger UI</p>
      </div>

      {/* API Selector Tabs */}
      <div className="flex gap-2 border-b border-slate-800/50 pb-4">
        <button
          onClick={() => setSelectedApi("fullnode")}
          className={`px-4 py-2 rounded-lg transition-colors ${
            selectedApi === "fullnode"
              ? "bg-amber-500/10 text-amber-400 border border-amber-500/30"
              : "text-slate-400 hover:text-slate-200"
          }`}
        >
          Fullnode API
        </button>
        <button
          onClick={() => setSelectedApi("wallet")}
          className={`px-4 py-2 rounded-lg transition-colors ${
            selectedApi === "wallet"
              ? "bg-amber-500/10 text-amber-400 border border-amber-500/30"
              : "text-slate-400 hover:text-slate-200"
          }`}
        >
          Wallet Headless API
        </button>
      </div>

      <div className="bg-[#0d1117] rounded-xl border border-slate-800 overflow-hidden flex-1">
        <SwaggerUIComponent apiType={selectedApi} />
      </div>
    </div>
  );
}
