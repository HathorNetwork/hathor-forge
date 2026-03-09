import { useState } from "react";
import { SwaggerUIComponent } from "@/components/SwaggerUI";
import 'swagger-ui-react/swagger-ui.css';

export function ApiExplorerPage() {
  const [selectedApi, setSelectedApi] = useState<"fullnode" | "wallet">("fullnode");

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">API Explorer</h2>
        <p className="text-white/30">Interactive API documentation powered by Swagger UI</p>
      </div>

      {/* API Selector Tabs */}
      <div className="flex gap-2 border-b border-white/5 pb-4">
        <button
          onClick={() => setSelectedApi("fullnode")}
          className={`px-4 py-2 rounded-lg transition-colors ${
            selectedApi === "fullnode"
              ? "bg-[#9cf35b]/10 text-[#9cf35b] border border-[#9cf35b]/30"
              : "text-white/30 hover:text-white"
          }`}
        >
          Fullnode API
        </button>
        <button
          onClick={() => setSelectedApi("wallet")}
          className={`px-4 py-2 rounded-lg transition-colors ${
            selectedApi === "wallet"
              ? "bg-[#9cf35b]/10 text-[#9cf35b] border border-[#9cf35b]/30"
              : "text-white/30 hover:text-white"
          }`}
        >
          Wallet Headless API
        </button>
      </div>

      <div className="bg-[#0b0a12] rounded-xl border border-white/5 overflow-hidden flex-1">
        <SwaggerUIComponent apiType={selectedApi} />
      </div>
    </div>
  );
}
