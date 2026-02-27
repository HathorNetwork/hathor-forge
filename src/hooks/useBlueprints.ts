import { useQuery } from "@tanstack/react-query";
import * as api from "@/services/tauri";
import { useAppStore } from "@/store/useAppStore";
import { POLLING_INTERVALS } from "@/lib/constants";

export function useBlueprints() {
  const nodeStatus = useAppStore((s) => s.nodeStatus);

  return useQuery({
    queryKey: ["blueprints"],
    queryFn: async () => {
      const response = await api.listBlueprints();
      if (response.success && Array.isArray(response.blueprints)) {
        return response.blueprints;
      }
      return [];
    },
    enabled: nodeStatus === "running",
    refetchInterval: POLLING_INTERVALS.BLUEPRINTS,
    initialData: [],
  });
}
