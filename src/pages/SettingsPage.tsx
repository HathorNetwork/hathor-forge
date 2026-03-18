import { useState, useCallback } from "react";
import { AlertTriangle, Trash2, Loader2, Copy, Check, Plug } from "lucide-react";
import { useNodeStore } from "@/store/useNodeStore";
import { useNanoContractStore } from "@/store/useNanoContractStore";
import { useUIStore } from "@/store/useUIStore";
import { Modal } from "@/components/ui/Modal";
import { usePortsStore } from "@/store/usePortsStore";
import * as api from "@/services/tauri";

export function SettingsPage() {
  const nodeStatus = useNodeStore((s) => s.nodeStatus);
  const PORTS = usePortsStore((s) => s.ports);
  const [showResetConfirm, setShowResetConfirm] = useState(false);
  const [resetStatus, setResetStatus] = useState<"idle" | "resetting" | "success" | "error">("idle");
  const [resetMessage, setResetMessage] = useState("");
  const [mcpCopied, setMcpCopied] = useState(false);
  const [mcpError, setMcpError] = useState<string | null>(null);
  const clearContracts = useNanoContractStore((s) => s.clearContracts);
  const clearLogs = useUIStore((s) => s.clearLogs);

  const closeResetModal = useCallback(() => setShowResetConfirm(false), []);

  const handleResetData = async () => {
    if (nodeStatus === "running") {
      setResetMessage("Stop the node before resetting data");
      setResetStatus("error");
      return;
    }
    setResetStatus("resetting");
    try {
      const result = await api.resetData();
      clearContracts();
      clearLogs();
      setResetMessage(result);
      setResetStatus("success");
      setShowResetConfirm(false);
    } catch (error) {
      setResetMessage(String(error));
      setResetStatus("error");
    }
  };

  const handleCopyMcpConfig = async () => {
    setMcpError(null);
    try {
      const config = await api.getMcpConfig();
      const json = JSON.stringify({ mcpServers: config }, null, 2);
      await navigator.clipboard.writeText(json);
      setMcpCopied(true);
      setTimeout(() => setMcpCopied(false), 2000);
    } catch (error) {
      setMcpError(String(error));
    }
  };

  const mcpHttpConfig = JSON.stringify({
    mcpServers: {
      "hathor-forge": {
        type: "http",
        url: `http://127.0.0.1:${PORTS.MCP_SERVER}/mcp`,
      },
    },
  }, null, 2);

  const handleCopyHttpConfig = async () => {
    await navigator.clipboard.writeText(mcpHttpConfig);
    setMcpCopied(true);
    setTimeout(() => setMcpCopied(false), 2000);
  };

  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">Settings</h2>
        <p className="text-white/30">Configure your local development environment</p>
      </div>

      {/* MCP Integration */}
      <div className="border border-blue-500/30 rounded-xl bg-blue-500/5 p-6">
        <div className="flex items-center gap-3 mb-4">
          <Plug className="w-5 h-5 text-blue-400" aria-hidden="true" />
          <h3 className="text-lg font-semibold text-blue-400">MCP Integration</h3>
        </div>

        <p className="text-sm text-white/30 mb-4">
          Connect AI assistants like Claude to control your local blockchain environment.
        </p>

        <div className="space-y-4">
          {/* Claude Code / HTTP config */}
          <div className="p-4 bg-white/3 rounded-lg border border-white/5">
            <div className="flex items-center justify-between mb-2">
              <h4 className="font-medium text-white">Claude Code / .mcp.json</h4>
              <button
                onClick={handleCopyHttpConfig}
                className="px-3 py-1.5 text-sm bg-blue-500/10 text-blue-400 border border-blue-500/30 rounded-lg hover:bg-blue-500/20 transition-colors flex items-center gap-1.5"
              >
                {mcpCopied ? (
                  <><Check className="w-3.5 h-3.5" /> Copied</>
                ) : (
                  <><Copy className="w-3.5 h-3.5" /> Copy Config</>
                )}
              </button>
            </div>
            <p className="text-xs text-white/30 mb-2">
              Paste into your project's <code className="text-white/70">.mcp.json</code> or run: <code className="text-white/70">claude mcp add --transport http hathor-forge http://127.0.0.1:{PORTS.MCP_SERVER}/mcp</code>
            </p>
            <pre className="text-xs text-white/70 font-mono bg-black/40 rounded p-3 overflow-x-auto">{mcpHttpConfig}</pre>
          </div>

          {/* Claude Desktop / stdio config */}
          <div className="p-4 bg-white/3 rounded-lg border border-white/5">
            <div className="flex items-center justify-between mb-2">
              <h4 className="font-medium text-white">Claude Desktop (stdio)</h4>
              <button
                onClick={handleCopyMcpConfig}
                className="px-3 py-1.5 text-sm bg-blue-500/10 text-blue-400 border border-blue-500/30 rounded-lg hover:bg-blue-500/20 transition-colors flex items-center gap-1.5"
              >
                {mcpCopied ? (
                  <><Check className="w-3.5 h-3.5" /> Copied</>
                ) : (
                  <><Copy className="w-3.5 h-3.5" /> Copy Config</>
                )}
              </button>
            </div>
            <p className="text-xs text-white/30">
              Copies a config snippet with resolved paths to the bundled Node.js binary and stdio bridge. Paste into Claude Desktop's config file.
            </p>
            {mcpError && (
              <div className="mt-2 p-2 rounded text-xs bg-red-500/10 text-red-400 border border-red-500/30">
                {mcpError}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Danger Zone */}
      <div className="border border-red-500/30 rounded-xl bg-red-500/5 p-6">
        <div className="flex items-center gap-3 mb-4">
          <AlertTriangle className="w-5 h-5 text-red-400" aria-hidden="true" />
          <h3 className="text-lg font-semibold text-red-400">Danger Zone</h3>
        </div>

        <div className="space-y-4">
          <div className="flex items-center justify-between p-4 bg-white/3 rounded-lg border border-white/5">
            <div>
              <h4 className="font-medium text-white">Reset Blockchain Data</h4>
              <p className="text-sm text-white/30 mt-1">
                Delete all blockchain data and start fresh. This will remove all blocks, transactions, and wallet history.
              </p>
            </div>
            <button
              onClick={() => setShowResetConfirm(true)}
              disabled={nodeStatus === "running" || resetStatus === "resetting"}
              className="px-4 py-2 bg-red-500/10 text-red-400 border border-red-500/30 rounded-lg hover:bg-red-500/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              <Trash2 className="w-4 h-4" aria-hidden="true" />
              Reset Data
            </button>
          </div>

          {resetStatus !== "idle" && (
            <div role="status" aria-live="polite" className={`p-3 rounded-lg text-sm ${
              resetStatus === "success" ? "bg-green-500/10 text-green-400 border border-green-500/30" :
              resetStatus === "error" ? "bg-red-500/10 text-red-400 border border-red-500/30" :
              "bg-white/5 text-white/30"
            }`}>
              {resetStatus === "resetting" ? "Resetting data..." : resetMessage}
            </div>
          )}
        </div>
      </div>

      {/* Reset Confirmation Modal */}
      {showResetConfirm && (
        <Modal
          title="Reset Blockchain Data?"
          onClose={closeResetModal}
          icon={
            <div className="w-10 h-10 rounded-full bg-red-500/20 flex items-center justify-center">
              <AlertTriangle className="w-5 h-5 text-red-400" aria-hidden="true" />
            </div>
          }
        >
            <p className="text-white/30 mb-6">
              This will permanently delete all blockchain data including blocks, transactions, and wallet history.
              You will need to mine from block 0 again. This action cannot be undone.
            </p>
            <div className="flex gap-3 justify-end">
              <button
                onClick={closeResetModal}
                className="px-4 py-2 text-white/30 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleResetData}
                disabled={resetStatus === "resetting"}
                className="px-4 py-2 bg-red-500 text-white rounded-lg hover:bg-red-600 transition-colors disabled:opacity-50 flex items-center gap-2"
              >
                {resetStatus === "resetting" ? (
                  <><Loader2 className="w-4 h-4 animate-spin" />Resetting...</>
                ) : (
                  <><Trash2 className="w-4 h-4" aria-hidden="true" />Yes, Reset Data</>
                )}
              </button>
            </div>
        </Modal>
      )}

    </div>
  );
}
