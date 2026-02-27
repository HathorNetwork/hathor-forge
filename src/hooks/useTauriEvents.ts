import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "@/store/useAppStore";
import { useWalletStore } from "@/store/useWalletStore";

export function useTauriEvents() {
  const addLog = useAppStore((s) => s.addLog);
  const setNodeStatus = useAppStore((s) => s.setNodeStatus);
  const setMinerStatus = useAppStore((s) => s.setMinerStatus);
  const setHashRate = useAppStore((s) => s.setHashRate);
  const setError = useAppStore((s) => s.setError);
  const setHeadlessStatus = useWalletStore((s) => s.setHeadlessStatus);
  const setHeadlessWallets = useWalletStore((s) => s.setHeadlessWallets);

  useEffect(() => {
    const unlistenLog = listen<string>("node-log", (event) => {
      addLog("node", event.payload);
    });

    const unlistenError = listen<string>("node-error", (event) => {
      addLog("node", event.payload);
    });

    const unlistenTerminated = listen<number | null>("node-terminated", (event) => {
      setNodeStatus("stopped");
      setMinerStatus("stopped");
      if (event.payload !== 0 && event.payload !== null) {
        setError(`Node exited with code ${event.payload}`);
      }
    });

    const unlistenMinerLog = listen<string>("miner-log", (event) => {
      addLog("miner", event.payload);
    });

    const unlistenMinerStats = listen<string>("miner-stats", (event) => {
      const match = event.payload.match(/(\d+\.?\d*)\s*(k|M|G|T)?hash\/s/i);
      if (match) {
        const value = match[1];
        const unit = match[2] ? match[2].toUpperCase() + "H/s" : "H/s";
        setHashRate(`${value} ${unit}`);
      }
      addLog("miner", event.payload);
    });

    const unlistenMinerTerminated = listen<number | null>("miner-terminated", () => {
      setMinerStatus("stopped");
      setHashRate("0 H/s");
    });

    const unlistenHeadlessLog = listen<string>("headless-log", (event) => {
      addLog("headless", event.payload);
    });

    const unlistenHeadlessTerminated = listen<number | null>("headless-terminated", () => {
      setHeadlessStatus({ running: false, port: null });
      setHeadlessWallets([]);
    });

    return () => {
      unlistenLog.then((f) => f());
      unlistenError.then((f) => f());
      unlistenTerminated.then((f) => f());
      unlistenMinerLog.then((f) => f());
      unlistenMinerStats.then((f) => f());
      unlistenMinerTerminated.then((f) => f());
      unlistenHeadlessLog.then((f) => f());
      unlistenHeadlessTerminated.then((f) => f());
    };
  }, [addLog, setNodeStatus, setMinerStatus, setHashRate, setError, setHeadlessStatus, setHeadlessWallets]);
}
