import { useAppStore } from "@/store/useAppStore";
import { useWalletStore } from "@/store/useWalletStore";
import * as api from "@/services/tauri";
import { PORTS } from "@/lib/constants";

export function useStartNetwork() {
  const { nodeStatus, setNodeStatus, setError } = useAppStore();
  const { setHeadlessStatus } = useWalletStore();

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
        setHeadlessStatus({ running: true, port: PORTS.WALLET_HEADLESS });
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
