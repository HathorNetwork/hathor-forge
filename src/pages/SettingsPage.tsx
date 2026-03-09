import { useState, useCallback } from "react";
import { AlertTriangle, Trash2, Loader2 } from "lucide-react";
import { useNodeStore } from "@/store/useNodeStore";
import { useNanoContractStore } from "@/store/useNanoContractStore";
import { useEscapeKey } from "@/hooks/useEscapeKey";
import * as api from "@/services/tauri";

export function SettingsPage() {
  const nodeStatus = useNodeStore((s) => s.nodeStatus);
  const [showResetConfirm, setShowResetConfirm] = useState(false);
  const [resetStatus, setResetStatus] = useState<"idle" | "resetting" | "success" | "error">("idle");
  const [resetMessage, setResetMessage] = useState("");
  const clearContracts = useNanoContractStore((s) => s.clearContracts);

  const closeResetModal = useCallback(() => setShowResetConfirm(false), []);
  useEscapeKey(closeResetModal, showResetConfirm);

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
      setResetMessage(result);
      setResetStatus("success");
      setShowResetConfirm(false);
    } catch (error) {
      setResetMessage(String(error));
      setResetStatus("error");
    }
  };

  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-2xl font-bold text-white mb-2">Settings</h2>
        <p className="text-slate-500">Configure your local development environment</p>
      </div>

      {/* Danger Zone */}
      <div className="border border-red-500/30 rounded-xl bg-red-500/5 p-6">
        <div className="flex items-center gap-3 mb-4">
          <AlertTriangle className="w-5 h-5 text-red-400" />
          <h3 className="text-lg font-semibold text-red-400">Danger Zone</h3>
        </div>

        <div className="space-y-4">
          <div className="flex items-center justify-between p-4 bg-slate-900/50 rounded-lg border border-slate-800">
            <div>
              <h4 className="font-medium text-white">Reset Blockchain Data</h4>
              <p className="text-sm text-slate-500 mt-1">
                Delete all blockchain data and start fresh. This will remove all blocks, transactions, and wallet history.
              </p>
            </div>
            <button
              onClick={() => setShowResetConfirm(true)}
              disabled={nodeStatus === "running" || resetStatus === "resetting"}
              className="px-4 py-2 bg-red-500/10 text-red-400 border border-red-500/30 rounded-lg hover:bg-red-500/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              <Trash2 className="w-4 h-4" />
              Reset Data
            </button>
          </div>

          {resetStatus !== "idle" && (
            <div className={`p-3 rounded-lg text-sm ${
              resetStatus === "success" ? "bg-green-500/10 text-green-400 border border-green-500/30" :
              resetStatus === "error" ? "bg-red-500/10 text-red-400 border border-red-500/30" :
              "bg-slate-800 text-slate-400"
            }`}>
              {resetStatus === "resetting" ? "Resetting data..." : resetMessage}
            </div>
          )}
        </div>
      </div>

      {/* Reset Confirmation Modal */}
      {showResetConfirm && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
          <div className="bg-[#0d1117] border border-slate-800 rounded-xl p-6 max-w-md w-full mx-4 shadow-2xl">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 rounded-full bg-red-500/20 flex items-center justify-center">
                <AlertTriangle className="w-5 h-5 text-red-400" />
              </div>
              <h3 className="text-lg font-semibold text-white">Reset Blockchain Data?</h3>
            </div>
            <p className="text-slate-400 mb-6">
              This will permanently delete all blockchain data including blocks, transactions, and wallet history.
              You will need to mine from block 0 again. This action cannot be undone.
            </p>
            <div className="flex gap-3 justify-end">
              <button
                onClick={() => setShowResetConfirm(false)}
                className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
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
                  <><Trash2 className="w-4 h-4" />Yes, Reset Data</>
                )}
              </button>
            </div>
          </div>
        </div>
      )}

    </div>
  );
}
