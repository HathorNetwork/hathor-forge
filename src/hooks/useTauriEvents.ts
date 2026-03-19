import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { useNodeStore } from "@/store/useNodeStore";
import { useUIStore } from "@/store/useUIStore";
import { useWalletStore } from "@/store/useWalletStore";

export function useTauriEvents() {
  const addLog = useUIStore((s) => s.addLog);
  const setNodeStatus = useNodeStore((s) => s.setNodeStatus);
  const setMinerStatus = useNodeStore((s) => s.setMinerStatus);
  const setHashRate = useNodeStore((s) => s.setHashRate);
  const setError = useUIStore((s) => s.setError);
  const setMiningStale = useUIStore((s) => s.setMiningStale);
  const setHeadlessStatus = useWalletStore((s) => s.setHeadlessStatus);
  const setHeadlessWallets = useWalletStore((s) => s.setHeadlessWallets);
  const queryClient = useQueryClient();

  const mountedRef = useRef(true);
  const unlistenFns = useRef<(() => void)[]>([]);

  useEffect(() => {
    mountedRef.current = true;
    unlistenFns.current = [];

    const register = async (promise: Promise<() => void>) => {
      const unlisten = await promise;
      if (mountedRef.current) {
        unlistenFns.current.push(unlisten);
      } else {
        unlisten();
      }
    };

    register(listen<void>("node-started", () => {
      setNodeStatus("running");
    }));

    register(listen<string>("node-log", (event) => {
      addLog("node", event.payload);
    }));

    register(listen<string>("node-error", (event) => {
      addLog("node", event.payload);
    }));

    register(listen<number | null>("node-terminated", (event) => {
      setNodeStatus("stopped");
      setMinerStatus("stopped");
      setMiningStale(false);
      if (event.payload !== 0 && event.payload !== null) {
        setError(`Node exited with code ${event.payload}`);
      }
    }));

    register(listen<void>("miner-started", () => {
      setMinerStatus("mining");
    }));

    register(listen<string>("miner-log", (event) => {
      addLog("miner", event.payload);
    }));

    register(listen<string>("miner-stats", (event) => {
      const match = event.payload.match(/(\d+\.?\d*)\s*(k|M|G|T)?hash\/s/i);
      if (match) {
        const value = match[1];
        const unit = match[2] ? match[2].toUpperCase() + "H/s" : "H/s";
        setHashRate(`${value} ${unit}`);
      }
      addLog("miner", event.payload);
    }));

    register(listen<number | null>("miner-terminated", () => {
      setMinerStatus("stopped");
      setHashRate("0 H/s");
      setMiningStale(false);
    }));

    register(listen<void>("mining-stale", () => {
      setMiningStale(true);
    }));

    register(listen<void>("headless-started", () => {
      setHeadlessStatus({ running: true, port: null });
    }));

    register(listen<string>("headless-log", (event) => {
      addLog("headless", event.payload);
    }));

    register(listen<number | null>("headless-terminated", () => {
      setHeadlessStatus({ running: false, port: null });
      setHeadlessWallets([]);
    }));

    // MCP → frontend state sync events
    register(listen<string>("wallet-created", (event) => {
      try {
        const data = JSON.parse(event.payload);
        setHeadlessWallets((prev) => {
          // Don't add if already exists
          if (prev.some((w) => w.wallet_id === data.wallet_id)) return prev;
          return [...prev, {
            wallet_id: data.wallet_id,
            status: "starting",
            status_code: null,
            seed: data.seed,
          }];
        });
        // Poll until wallet is ready, then load details
        import("@/services/tauri").then((api) => {
          const poll = async () => {
            for (let i = 0; i < 30; i++) {
              if (!mountedRef.current) return;
              await new Promise((r) => setTimeout(r, 1000));
              if (!mountedRef.current) return;
              try {
                const status = await api.getHeadlessWalletStatus(data.wallet_id);
                setHeadlessWallets((prev) =>
                  prev.map((w) =>
                    w.wallet_id === data.wallet_id
                      ? { ...w, status: status.status, status_code: status.status_code }
                      : w
                  )
                );
                if (status.status_code === 3) {
                  const [balance, addresses] = await Promise.all([
                    api.getHeadlessWalletBalance(data.wallet_id),
                    api.getHeadlessWalletAddresses(data.wallet_id),
                  ]);
                  setHeadlessWallets((prev) =>
                    prev.map((w) =>
                      w.wallet_id === data.wallet_id ? { ...w, balance, addresses } : w
                    )
                  );
                  break;
                }
              } catch { break; }
            }
          };
          poll();
        });
      } catch { /* ignore parse errors */ }
    }));

    register(listen<string>("wallet-closed", (event) => {
      try {
        const data = JSON.parse(event.payload);
        setHeadlessWallets((prev) => prev.filter((w) => w.wallet_id !== data.wallet_id));
      } catch { /* ignore parse errors */ }
    }));

    register(listen<string>("wallet-funded", (event) => {
      try {
        const data = JSON.parse(event.payload);
        // Trigger a refresh of the wallet's balance/addresses via the Tauri command
        import("@/services/tauri").then((api) => {
          Promise.all([
            api.getHeadlessWalletBalance(data.wallet_id),
            api.getHeadlessWalletAddresses(data.wallet_id),
          ]).then(([balance, addresses]) => {
            setHeadlessWallets((prev) =>
              prev.map((w) =>
                w.wallet_id === data.wallet_id ? { ...w, balance, addresses } : w
              )
            );
          }).catch(() => { /* wallet may not be ready yet */ });
        });
      } catch { /* ignore parse errors */ }
    }));

    // Nano contract / blueprint events from MCP → invalidate React Query caches
    register(listen<string>("blueprint-published", () => {
      queryClient.invalidateQueries({ queryKey: ["blueprints"] });
    }));

    register(listen<string>("nano-contract-created", () => {
      queryClient.invalidateQueries({ queryKey: ["blueprints"] });
      queryClient.invalidateQueries({ queryKey: ["contractStates"] });
    }));

    register(listen<string>("nano-contract-executed", () => {
      queryClient.invalidateQueries({ queryKey: ["contractStates"] });
    }));

    return () => {
      mountedRef.current = false;
      unlistenFns.current.forEach((fn) => fn());
    };
  }, [addLog, setNodeStatus, setMinerStatus, setHashRate, setError, setMiningStale, setHeadlessStatus, setHeadlessWallets, queryClient]);
}
