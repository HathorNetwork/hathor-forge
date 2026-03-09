import { useEffect } from "react";
import * as api from "@/services/tauri";
import { useNodeStore } from "@/store/useNodeStore";
import { useWalletStore } from "@/store/useWalletStore";

/**
 * On app launch, check the actual status of managed services
 * instead of assuming everything is stopped.
 */
export function useInitialStateReconciliation() {
  const setNodeStatus = useNodeStore((s) => s.setNodeStatus);
  const setMinerStatus = useNodeStore((s) => s.setMinerStatus);
  const setBlockHeight = useNodeStore((s) => s.setBlockHeight);
  const setHeadlessStatus = useWalletStore((s) => s.setHeadlessStatus);

  useEffect(() => {
    let cancelled = false;

    async function reconcile() {
      // Check node status
      try {
        const nodeStatus = await api.getNodeStatus();
        if (cancelled) return;
        if (nodeStatus.running) {
          setNodeStatus("running");
          if (nodeStatus.block_height !== null) {
            setBlockHeight(nodeStatus.block_height);
          }
        }
      } catch {
        // Node status check failed — leave as stopped
      }

      // Check miner status
      try {
        const minerStatus = await api.getMinerStatus();
        if (cancelled) return;
        if (minerStatus.running) {
          setMinerStatus("mining");
        }
      } catch {
        // Miner status check failed — leave as stopped
      }

      // Check headless status
      try {
        const headlessStatus = await api.getHeadlessStatus();
        if (cancelled) return;
        if (headlessStatus.running) {
          setHeadlessStatus(headlessStatus);
        }
      } catch {
        // Headless status check failed — leave as stopped
      }
    }

    reconcile();

    return () => {
      cancelled = true;
    };
  }, [setNodeStatus, setMinerStatus, setBlockHeight, setHeadlessStatus]);
}
