import { useNodeStore } from "@/store/useNodeStore";
import { useUIStore } from "@/store/useUIStore";
import { useWalletStore } from "@/store/useWalletStore";
import { usePortsStore } from "@/store/usePortsStore";
import * as api from "@/services/tauri";

export function useStartNetwork() {
  const { nodeStatus, setNodeStatus } = useNodeStore();
  const { setError } = useUIStore();
  const { setHeadlessStatus } = useWalletStore();
  const ports = usePortsStore((s) => s.ports);

  const isLoading = nodeStatus === "starting";

  const startNetwork = async () => {
    setError(null);
    setNodeStatus("starting");
    try {
      await api.startNode();
      setNodeStatus("running");
      try {
        await api.startExplorerServer();
      } catch (e) {
        console.warn("Explorer server failed to start:", e);
      }
      try {
        await api.startHeadless();
        setHeadlessStatus({ running: true, port: ports.WALLET_HEADLESS });
      } catch (e) {
        console.warn("Wallet-headless failed to start:", e);
      }
    } catch (e) {
      setError(String(e));
      setNodeStatus("error");
    }
  };

  return { startNetwork, isLoading };
}
