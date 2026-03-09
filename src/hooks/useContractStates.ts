import { useQuery } from "@tanstack/react-query";
import * as api from "@/services/tauri";
import { useNodeStore } from "@/store/useNodeStore";
import { useNanoContractStore } from "@/store/useNanoContractStore";
import { POLLING_INTERVALS } from "@/lib/constants";

export function useContractStatesPolling() {
  const nodeStatus = useNodeStore((s) => s.nodeStatus);
  const contracts = useNanoContractStore((s) => s.contracts);
  const updateContract = useNanoContractStore((s) => s.updateContract);

  return useQuery({
    queryKey: ["contractStates", contracts.map((c) => c.id)],
    queryFn: async () => {
      for (const contract of contracts) {
        try {
          const state = await api.getNanoContractState(contract.id);
          if (state.success) {
            updateContract(contract.id, {
              blueprintId: state.blueprint_name || state.blueprint_id || contract.blueprintId,
              balance: state.balance ?? contract.balance,
            });
          }
        } catch {
          // Non-critical
        }
      }
      return true;
    },
    enabled: nodeStatus === "running" && contracts.length > 0,
    refetchInterval: POLLING_INTERVALS.CONTRACT_STATES,
  });
}
