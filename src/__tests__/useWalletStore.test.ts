import { describe, it, expect, beforeEach } from "vitest";
import { useWalletStore } from "@/store/useWalletStore";

describe("useWalletStore", () => {
  beforeEach(() => {
    useWalletStore.setState({
      headlessStatus: { running: false, port: null },
      headlessWallets: [],
      faucetAddress: "",
      faucetAmount: "100",
      faucetBalance: null,
      sendingTx: false,
      txResult: null,
      showCreateWallet: false,
      newSeed: null,
      newWalletId: "",
      importSeed: "",
      creatingWallet: false,
      expandedWallet: null,
      copiedAddress: null,
    });
  });

  describe("initial state", () => {
    it("has correct defaults", () => {
      const state = useWalletStore.getState();
      expect(state.headlessStatus).toEqual({ running: false, port: null });
      expect(state.headlessWallets).toEqual([]);
      expect(state.faucetAddress).toBe("");
      expect(state.faucetAmount).toBe("100");
      expect(state.faucetBalance).toBeNull();
      expect(state.sendingTx).toBe(false);
      expect(state.txResult).toBeNull();
      expect(state.showCreateWallet).toBe(false);
      expect(state.newSeed).toBeNull();
      expect(state.newWalletId).toBe("");
      expect(state.importSeed).toBe("");
      expect(state.creatingWallet).toBe(false);
      expect(state.expandedWallet).toBeNull();
      expect(state.copiedAddress).toBeNull();
    });
  });

  describe("setHeadlessStatus", () => {
    it("updates headless status", () => {
      useWalletStore.getState().setHeadlessStatus({ running: true, port: 8001 });
      expect(useWalletStore.getState().headlessStatus).toEqual({
        running: true,
        port: 8001,
      });
    });
  });

  describe("setHeadlessWallets", () => {
    it("sets wallets directly", () => {
      const wallets = [
        { wallet_id: "w1", status: "ready", status_code: 3 },
      ];
      useWalletStore.getState().setHeadlessWallets(wallets);
      expect(useWalletStore.getState().headlessWallets).toEqual(wallets);
    });

    it("accepts an updater function", () => {
      const wallet1 = { wallet_id: "w1", status: "ready", status_code: 3 };
      const wallet2 = { wallet_id: "w2", status: "ready", status_code: 3 };
      useWalletStore.getState().setHeadlessWallets([wallet1]);
      useWalletStore.getState().setHeadlessWallets((prev) => [...prev, wallet2]);
      expect(useWalletStore.getState().headlessWallets).toHaveLength(2);
    });
  });

  describe("faucet actions", () => {
    it("sets faucet address", () => {
      useWalletStore.getState().setFaucetAddress("WXk123");
      expect(useWalletStore.getState().faucetAddress).toBe("WXk123");
    });

    it("sets faucet amount", () => {
      useWalletStore.getState().setFaucetAmount("500");
      expect(useWalletStore.getState().faucetAmount).toBe("500");
    });

    it("sets faucet balance", () => {
      const balance = { available: 1000, locked: 0 };
      useWalletStore.getState().setFaucetBalance(balance);
      expect(useWalletStore.getState().faucetBalance).toEqual(balance);
    });

    it("sets sendingTx", () => {
      useWalletStore.getState().setSendingTx(true);
      expect(useWalletStore.getState().sendingTx).toBe(true);
    });

    it("sets txResult", () => {
      useWalletStore.getState().setTxResult({ type: "success", message: "Sent!" });
      expect(useWalletStore.getState().txResult).toEqual({
        type: "success",
        message: "Sent!",
      });
    });

    it("clears txResult", () => {
      useWalletStore.getState().setTxResult({ type: "error", message: "fail" });
      useWalletStore.getState().setTxResult(null);
      expect(useWalletStore.getState().txResult).toBeNull();
    });
  });

  describe("UI state actions", () => {
    it("toggles showCreateWallet", () => {
      useWalletStore.getState().setShowCreateWallet(true);
      expect(useWalletStore.getState().showCreateWallet).toBe(true);
    });

    it("sets expandedWallet", () => {
      useWalletStore.getState().setExpandedWallet("w1");
      expect(useWalletStore.getState().expandedWallet).toBe("w1");
    });

    it("sets copiedAddress", () => {
      useWalletStore.getState().setCopiedAddress("addr123");
      expect(useWalletStore.getState().copiedAddress).toBe("addr123");
    });
  });
});
