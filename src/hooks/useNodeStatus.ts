import { useQuery } from "@tanstack/react-query";
import * as api from "@/services/tauri";
import { useAppStore } from "@/store/useAppStore";
import { useWalletStore } from "@/store/useWalletStore";
import { POLLING_INTERVALS } from "@/lib/constants";

export function useNodeStatusPolling() {
  const nodeStatus = useAppStore((s) => s.nodeStatus);
  const setBlockHeight = useAppStore((s) => s.setBlockHeight);
  const setFaucetBalance = useWalletStore((s) => s.setFaucetBalance);

  return useQuery({
    queryKey: ["nodeStatus"],
    queryFn: async () => {
      const status = await api.getNodeStatus();
      if (status.block_height !== null) {
        setBlockHeight(status.block_height);
      }

      try {
        const balance = await api.getFullnodeBalance();
        setFaucetBalance(balance);
      } catch {
        // Non-critical
      }

      return status;
    },
    enabled: nodeStatus === "running",
    refetchInterval: POLLING_INTERVALS.NODE_STATUS,
  });
}
