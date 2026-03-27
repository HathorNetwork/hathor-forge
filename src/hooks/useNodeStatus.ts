import { useQuery } from "@tanstack/react-query";
import * as api from "@/services/tauri";
import { useNodeStore } from "@/store/useNodeStore";
import { useUIStore } from "@/store/useUIStore";
import { useWalletStore } from "@/store/useWalletStore";
import { POLLING_INTERVALS } from "@/lib/constants";

export function useNodeStatusPolling() {
  const nodeStatus = useNodeStore((s) => s.nodeStatus);
  const setBlockHeight = useNodeStore((s) => s.setBlockHeight);
  const setFaucetBalance = useWalletStore((s) => s.setFaucetBalance);
  const addLog = useUIStore((s) => s.addLog);

  return useQuery({
    queryKey: ["nodeStatus"],
    queryFn: async () => {
      const status = await api.getNodeStatus();
      if (status.block_height !== null) {
        setBlockHeight(status.block_height);
      }

      try {
        const balance = await api.getFullnodeBalance();
        addLog("node", `[FAUCET-DEBUG] statusPoll got: available=${balance.available}, locked=${balance.locked}`);
        setFaucetBalance(balance);
      } catch (e) {
        addLog("node", `[FAUCET-DEBUG] statusPoll balance FAILED: ${e}`);
      }

      return status;
    },
    enabled: nodeStatus === "running",
    refetchInterval: POLLING_INTERVALS.NODE_STATUS,
  });
}
