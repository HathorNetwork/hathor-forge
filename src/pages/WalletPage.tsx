import { useState, useEffect, useRef, useCallback } from "react";
import {
  Wallet, Play, Square, Send, Copy, Check, Loader2,
} from "lucide-react";
import { useNodeStore } from "@/store/useNodeStore";
import { useUIStore } from "@/store/useUIStore";
import { useWalletStore } from "@/store/useWalletStore";
import { Modal } from "@/components/ui/Modal";
import * as api from "@/services/tauri";
import { formatHTR } from "@/lib/utils";
import { usePortsStore } from "@/store/usePortsStore";
import type { HeadlessWallet } from "@/types";

export function WalletPage() {
  const nodeStatus = useNodeStore((s) => s.nodeStatus);
  const setError = useUIStore((s) => s.setError);
  const PORTS = usePortsStore((s) => s.ports);
  const [walletSendForms, setWalletSendForms] = useState<Record<string, { address: string; amount: string }>>({});
  const {
    headlessStatus, setHeadlessStatus,
    headlessWallets, setHeadlessWallets,
    faucetAddress, setFaucetAddress,
    faucetAmount, setFaucetAmount,
    faucetBalance, setFaucetBalance,
    sendingTx, setSendingTx,
    txResult, setTxResult,
    showCreateWallet, setShowCreateWallet,
    newSeed, setNewSeed,
    newWalletId, setNewWalletId,
    importSeed, setImportSeed,
    creatingWallet, setCreatingWallet,
    expandedWallet, setExpandedWallet,
    copiedAddress, setCopiedAddress,
  } = useWalletStore();

  const closeCreateWalletModal = useCallback(() => {
    setShowCreateWallet(false);
    setNewSeed(null);
    setNewWalletId("");
    setImportSeed("");
  }, [setShowCreateWallet, setNewSeed, setNewWalletId, setImportSeed]);

  // Abort controller for cancelling pollWalletStatus on unmount
  const pollAbortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    return () => {
      pollAbortRef.current?.abort();
    };
  }, []);

  // ── Handlers ──────────────────────────────────────────────────────────────

  const startHeadless = async () => {
    try {
      await api.startHeadless();
      setHeadlessStatus({ running: true, port: PORTS.WALLET_HEADLESS });
    } catch (e) {
      setError(String(e));
    }
  };

  const stopHeadless = async () => {
    try {
      await api.stopHeadless();
      setHeadlessStatus({ running: false, port: null });
      setHeadlessWallets([]);
    } catch (e) {
      setError(String(e));
    }
  };

  const generateNewSeed = async () => {
    try {
      const seed = await api.generateSeed();
      setNewSeed(seed);
      setImportSeed("");
    } catch (e) {
      setError(String(e));
    }
  };

  const loadWalletDetails = async (walletId: string) => {
    try {
      const [balance, addresses] = await Promise.all([
        api.getHeadlessWalletBalance(walletId),
        api.getHeadlessWalletAddresses(walletId),
      ]);

      setHeadlessWallets((prev) =>
        prev.map((w) =>
          w.wallet_id === walletId ? { ...w, balance, addresses } : w
        )
      );
    } catch (e) {
      console.error("Failed to load wallet details:", e);
    }
  };

  const pollWalletStatus = async (walletId: string) => {
    // Cancel any previous polling loop
    pollAbortRef.current?.abort();
    const controller = new AbortController();
    pollAbortRef.current = controller;

    const maxAttempts = 30;
    for (let i = 0; i < maxAttempts; i++) {
      if (controller.signal.aborted) return;
      await new Promise((resolve) => setTimeout(resolve, 1000));
      if (controller.signal.aborted) return;
      try {
        const status = await api.getHeadlessWalletStatus(walletId);
        if (controller.signal.aborted) return;

        setHeadlessWallets((prev) =>
          prev.map((w) =>
            w.wallet_id === walletId
              ? { ...w, status: status.status, status_code: status.status_code }
              : w
          )
        );

        // Status code 3 means "Ready"
        if (status.status_code === 3) {
          await loadWalletDetails(walletId);
          break;
        }
      } catch {
        break;
      }
    }
  };

  const createWallet = async () => {
    const seed = newSeed || importSeed;
    if (!seed || !newWalletId) {
      setError("Please provide wallet ID and seed phrase");
      return;
    }

    setCreatingWallet(true);
    try {
      await api.createHeadlessWallet({
        wallet_id: newWalletId,
        seed: seed,
      });

      // Add to local state (store seed for dev convenience)
      setHeadlessWallets((prev) => [
        ...prev,
        {
          wallet_id: newWalletId,
          status: "starting",
          status_code: null,
          seed: seed,
        },
      ]);

      // Close modal and reset
      setShowCreateWallet(false);
      setNewSeed(null);
      setNewWalletId("");
      setImportSeed("");

      // Poll for wallet ready status
      pollWalletStatus(newWalletId);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreatingWallet(false);
    }
  };

  const closeWallet = async (walletId: string) => {
    try {
      await api.closeHeadlessWallet(walletId);
      setHeadlessWallets((prev) => prev.filter((w) => w.wallet_id !== walletId));
    } catch (e) {
      setError(String(e));
    }
  };

  const sendFromHeadlessWallet = async (walletId: string, address: string, amount: number) => {
    try {
      const result = await api.headlessWalletSendTx({
        wallet_id: walletId,
        address,
        amount: Math.floor(amount * 100), // Convert to cents
      });
      setTxResult({ type: "success", message: result });
      // Reload wallet details
      await loadWalletDetails(walletId);
    } catch (e) {
      setTxResult({ type: "error", message: String(e) });
    }
  };

  const fundWallet = async (walletId: string) => {
    const wallet = headlessWallets.find((w) => w.wallet_id === walletId);
    if (!wallet?.addresses?.length) {
      setError("Wallet has no addresses. Wait for it to sync.");
      return;
    }

    // Fetch fresh balance right before sending to avoid race conditions
    let available = 0;
    try {
      const freshBalance = await api.getFullnodeBalance();
      available = freshBalance.available;
      setFaucetBalance(freshBalance); // Update UI state too
    } catch {
      setError("Failed to fetch faucet balance. Try again.");
      return;
    }

    if (available <= 0) {
      setError("Faucet has no available funds. Wait for blocks to be mined and confirmed.");
      return;
    }

    // Use 10% of available, capped at 100 HTR (10000 cents), minimum 1 HTR (100 cents)
    const tenPercent = Math.floor(available * 0.1);
    const amount = Math.max(100, Math.min(tenPercent, 10000)); // Between 1 HTR and 100 HTR

    const firstAddress = wallet.addresses[0];
    try {
      await api.sendTx({ address: firstAddress, amount });
      setTxResult({ type: "success", message: `Sent ${formatHTR(amount)} HTR to ${walletId}` });
      // Reload wallet details after a short delay
      setTimeout(() => loadWalletDetails(walletId), 1000);
    } catch (e) {
      setError(String(e));
    }
  };

  const copySeed = (wallet: HeadlessWallet) => {
    if (wallet.seed) {
      navigator.clipboard.writeText(wallet.seed);
      setCopiedAddress(wallet.wallet_id + "-seed"); // Reuse copiedAddress state for feedback
      setTimeout(() => setCopiedAddress(null), 2000);
    }
  };

  const copyAddress = async (address: string) => {
    await navigator.clipboard.writeText(address);
    setCopiedAddress(address);
    setTimeout(() => setCopiedAddress(null), 2000);
  };

  const handleSendTx = async () => {
    if (!faucetAddress || !faucetAmount) {
      setTxResult({ type: "error", message: "Please enter address and amount" });
      return;
    }

    setSendingTx(true);
    setTxResult(null);

    try {
      const amountInCents = Math.floor(parseFloat(faucetAmount) * 100);
      const result = await api.sendTx({
        address: faucetAddress,
        amount: amountInCents,
      });
      setTxResult({ type: "success", message: result });
      setFaucetAddress("");
      setFaucetAmount("100");
    } catch (error) {
      setTxResult({ type: "error", message: String(error) });
    } finally {
      setSendingTx(false);
    }
  };

  // ── Render ────────────────────────────────────────────────────────────────

  if (nodeStatus !== "running") {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <Wallet className="w-12 h-12 text-white/15 mx-auto mb-4" />
          <h2 className="text-xl font-bold text-white mb-2">Node Not Running</h2>
          <p className="text-white/30">Start the node to access wallets</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white mb-2">Wallet Manager</h2>
          <p className="text-white/30">Create and manage multiple wallets</p>
        </div>
        <div className="flex gap-3">
          {!headlessStatus.running ? (
            <button
              onClick={startHeadless}
              className="px-4 py-2 bg-[#9cf35b]/10 text-[#9cf35b] border border-[#9cf35b]/30 rounded-lg hover:bg-[#9cf35b]/20 transition-colors flex items-center gap-2"
            >
              <Play className="w-4 h-4" />
              Start Wallet Service
            </button>
          ) : (
            <>
              <button
                onClick={() => setShowCreateWallet(true)}
                className="px-4 py-2 bg-[#9cf35b] text-[#000f61] rounded-lg hover:bg-[#bff658] transition-colors flex items-center gap-2"
              >
                <Wallet className="w-4 h-4" />
                New Wallet
              </button>
              <button
                onClick={stopHeadless}
                className="px-4 py-2 bg-white/5 text-white/70 rounded-lg hover:bg-white/5 transition-colors flex items-center gap-2"
              >
                <Square className="w-4 h-4" />
                Stop Service
              </button>
            </>
          )}
        </div>
      </div>

      {/* Headless Status */}
      <div className={`p-4 rounded-lg border ${headlessStatus.running ? "bg-[#9cf35b]/8 border-[#9cf35b]/25" : "bg-white/3 border-white/5"}`}>
        <div className="flex items-center gap-3">
          <div className={`w-2 h-2 rounded-full ${headlessStatus.running ? "bg-[#9cf35b] shadow-[0_0_6px_#9cf35b]" : "bg-white/20"}`} />
          <span className="text-sm font-medium text-white/70">
            Wallet Service: {headlessStatus.running ? `Running on port ${headlessStatus.port}` : "Stopped"}
          </span>
        </div>
      </div>

      {/* Wallets List */}
      {headlessStatus.running && (
        <div className="space-y-4">
          {headlessWallets.length === 0 ? (
            <div className="border border-white/5 rounded-xl bg-white/3 p-8 text-center">
              <Wallet className="w-12 h-12 text-white/15 mx-auto mb-4" />
              <p className="text-white/30">No wallets yet. Click "New Wallet" to create one.</p>
            </div>
          ) : (
            headlessWallets.map((wallet) => (
              <div key={wallet.wallet_id} className="border border-white/5 rounded-xl bg-white/3 overflow-hidden">
                <div className="p-4 border-b border-white/5 bg-white/3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <Wallet className="w-5 h-5 text-[#9cf35b]" />
                      <span className="font-semibold text-white">{wallet.wallet_id}</span>
                      <span className={`px-2 py-0.5 rounded text-xs font-medium ${
                        wallet.status_code === 3 ? "bg-[#9cf35b]/20 text-[#9cf35b]" :
                        "bg-[#ff8000]/20 text-[#ff8000]"
                      }`}>
                        {wallet.status}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => setExpandedWallet(expandedWallet === wallet.wallet_id ? null : wallet.wallet_id)}
                        className="px-3 py-1 text-sm text-white/30 hover:text-white transition-colors"
                      >
                        {expandedWallet === wallet.wallet_id ? "Collapse" : "Expand"}
                      </button>
                      <button
                        onClick={() => fundWallet(wallet.wallet_id)}
                        disabled={wallet.status_code !== 3 || !faucetBalance?.available}
                        className="px-3 py-1 text-sm bg-[#9cf35b]/10 text-[#9cf35b] rounded hover:bg-[#9cf35b]/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                        title={!faucetBalance?.available ? "Wait for blocks to be mined" : "Send funds from faucet"}
                      >
                        Fund
                      </button>
                      {wallet.seed && (
                        <button
                          onClick={() => copySeed(wallet)}
                          className="px-3 py-1 text-sm bg-[#9cf35b]/10 text-[#9cf35b] rounded hover:bg-[#9cf35b]/20 transition-colors flex items-center gap-1"
                        >
                          {copiedAddress === wallet.wallet_id + "-seed" ? (
                            <><Check className="w-3 h-3" /> Copied</>
                          ) : (
                            <><Copy className="w-3 h-3" /> Seed</>
                          )}
                        </button>
                      )}
                      <button
                        onClick={() => loadWalletDetails(wallet.wallet_id)}
                        className="px-3 py-1 text-sm bg-white/5 text-white/70 rounded hover:bg-white/5 transition-colors"
                      >
                        Refresh
                      </button>
                      <button
                        onClick={() => closeWallet(wallet.wallet_id)}
                        className="px-3 py-1 text-sm bg-red-500/10 text-red-400 rounded hover:bg-red-500/20 transition-colors"
                      >
                        Close
                      </button>
                    </div>
                  </div>
                  {wallet.balance && (
                    <div className="mt-2 text-sm text-white/30">
                      Balance: <span className="text-white font-semibold">{formatHTR(wallet.balance.available)} HTR</span>
                      {wallet.balance.locked > 0 && (
                        <span className="text-[#9cf35b] ml-2">({formatHTR(wallet.balance.locked)} locked)</span>
                      )}
                    </div>
                  )}
                </div>

                {expandedWallet === wallet.wallet_id && (
                  <div className="p-4 space-y-4">
                    {/* Addresses */}
                    {wallet.addresses && wallet.addresses.length > 0 && (
                      <div>
                        <h4 className="text-sm font-medium text-white/30 mb-2">Addresses</h4>
                        <div className="space-y-1 max-h-48 overflow-y-auto">
                          {wallet.addresses.slice(0, 10).map((addr, i) => (
                            <div key={addr} className="flex items-center gap-2 p-2 bg-white/5 rounded">
                              <span className="text-xs text-white/30">#{i}</span>
                              <code className="text-xs text-white/70 font-mono flex-1 truncate">{addr}</code>
                              <button
                                onClick={() => copyAddress(addr)}
                                className="p-1 hover:bg-white/5 rounded transition-colors"
                                aria-label={copiedAddress === addr ? "Address copied" : "Copy address"}
                              >
                                {copiedAddress === addr ? (
                                  <Check className="w-3 h-3 text-green-400" aria-hidden="true" />
                                ) : (
                                  <Copy className="w-3 h-3 text-white/30" aria-hidden="true" />
                                )}
                              </button>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Send from this wallet */}
                    {wallet.status_code === 3 && (
                      <div className="border-t border-white/5 pt-4">
                        <h4 className="text-sm font-medium text-white/30 mb-2">Send HTR</h4>
                        <div className="flex gap-2">
                          <input
                            type="text"
                            placeholder="Destination address"
                            aria-label="Destination address"
                            className="flex-1 px-3 py-2 bg-white/5 border border-white/5 rounded text-sm text-white placeholder-white/20 focus:outline-none focus:border-[#9cf35b]/30"
                            value={walletSendForms[wallet.wallet_id]?.address ?? ""}
                            onChange={(e) => setWalletSendForms((prev) => ({
                              ...prev,
                              [wallet.wallet_id]: { ...prev[wallet.wallet_id], address: e.target.value, amount: prev[wallet.wallet_id]?.amount ?? "" },
                            }))}
                          />
                          <input
                            type="number"
                            placeholder="Amount"
                            aria-label="Amount in HTR"
                            className="w-24 px-3 py-2 bg-white/5 border border-white/5 rounded text-sm text-white placeholder-white/20 focus:outline-none focus:border-[#9cf35b]/30"
                            value={walletSendForms[wallet.wallet_id]?.amount ?? ""}
                            onChange={(e) => setWalletSendForms((prev) => ({
                              ...prev,
                              [wallet.wallet_id]: { ...prev[wallet.wallet_id], amount: e.target.value, address: prev[wallet.wallet_id]?.address ?? "" },
                            }))}
                          />
                          <button
                            onClick={() => {
                              const form = walletSendForms[wallet.wallet_id];
                              if (form?.address && form?.amount) {
                                sendFromHeadlessWallet(wallet.wallet_id, form.address, parseFloat(form.amount));
                              }
                            }}
                            className="px-4 py-2 bg-[#9cf35b] text-[#000f61] rounded hover:bg-[#bff658] transition-colors flex items-center gap-1"
                          >
                            <Send className="w-4 h-4" />
                            Send
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      )}

      {/* Transaction Result */}
      {txResult && (
        <div role="alert" aria-live="assertive" className={`p-3 rounded-lg text-sm ${
          txResult.type === "success"
            ? "bg-green-500/10 text-green-400 border border-green-500/30"
            : "bg-red-500/10 text-red-400 border border-red-500/30"
        }`}>
          {txResult.message}
        </div>
      )}

      {/* Fullnode Wallet (Faucet) */}
      <div className="border border-[#9cf35b]/30 rounded-xl bg-[#9cf35b]/5 p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <Send className="w-5 h-5 text-[#9cf35b]" />
            <h3 className="text-lg font-semibold text-[#9cf35b]">Fullnode Faucet</h3>
            <span className="text-xs text-white/30">(Send HTR from fullnode's built-in wallet)</span>
          </div>
          {faucetBalance && (
            <div className="text-sm">
              <span className="text-white/30">Available: </span>
              <span className="text-[#9cf35b] font-semibold">{formatHTR(faucetBalance.available)} HTR</span>
              {faucetBalance.locked > 0 && (
                <span className="text-white/30 ml-2">({formatHTR(faucetBalance.locked)} locked)</span>
              )}
            </div>
          )}
        </div>

        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label htmlFor="faucet-address" className="block text-sm font-medium text-white/70 mb-2">
                Destination Address
              </label>
              <input
                id="faucet-address"
                type="text"
                value={faucetAddress}
                onChange={(e) => setFaucetAddress(e.target.value)}
                placeholder="Enter Hathor address..."
                className="w-full px-4 py-2 bg-white/3 border border-white/5 rounded-lg text-white placeholder-white/20 focus:outline-none focus:border-[#9cf35b]/30"
              />
            </div>
            <div>
              <label htmlFor="faucet-amount" className="block text-sm font-medium text-white/70 mb-2">
                Amount (HTR)
              </label>
              <input
                id="faucet-amount"
                type="number"
                value={faucetAmount}
                onChange={(e) => setFaucetAmount(e.target.value)}
                min="0.01"
                step="0.01"
                className="w-full px-4 py-2 bg-white/3 border border-white/5 rounded-lg text-white focus:outline-none focus:border-[#9cf35b]/30"
              />
            </div>
          </div>

          <button
            onClick={handleSendTx}
            disabled={sendingTx || !faucetAddress || !faucetAmount}
            className="px-6 py-2 bg-[#9cf35b] text-[#000f61] rounded-lg hover:bg-[#bff658] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            {sendingTx ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Sending...
              </>
            ) : (
              <>
                <Send className="w-4 h-4" />
                Send from Faucet
              </>
            )}
          </button>
        </div>
      </div>

      {/* Create Wallet Modal */}
      {showCreateWallet && (
        <Modal
          title="Create New Wallet"
          onClose={closeCreateWalletModal}
          maxWidth="max-w-lg"
          icon={
            <div className="w-10 h-10 rounded-full bg-[#9cf35b]/20 flex items-center justify-center">
              <Wallet className="w-5 h-5 text-[#9cf35b]" aria-hidden="true" />
            </div>
          }
        >
            <div className="space-y-4">
              <div>
                <label htmlFor="new-wallet-id" className="block text-sm font-medium text-white/70 mb-2">
                  Wallet ID
                </label>
                <input
                  id="new-wallet-id"
                  type="text"
                  value={newWalletId}
                  onChange={(e) => setNewWalletId(e.target.value)}
                  placeholder="e.g., my-wallet"
                  className="w-full px-4 py-2 bg-white/3 border border-white/5 rounded-lg text-white placeholder-white/20 focus:outline-none focus:border-[#9cf35b]/30"
                />
              </div>

              {/* Seed Options */}
              <div className="space-y-3">
                <div className="flex gap-2">
                  <button
                    onClick={generateNewSeed}
                    className="flex-1 px-4 py-2 bg-[#9cf35b]/10 text-[#9cf35b] border border-[#9cf35b]/30 rounded-lg hover:bg-[#9cf35b]/20 transition-colors"
                  >
                    Generate New Seed
                  </button>
                  <button
                    onClick={() => { setNewSeed(null); }}
                    className={`flex-1 px-4 py-2 border rounded-lg transition-colors ${!newSeed ? "bg-[#9cf35b]/10 text-[#9cf35b] border-[#9cf35b]/30" : "bg-white/5 text-white/30 border-white/5"}`}
                  >
                    Import Existing
                  </button>
                </div>

                {newSeed ? (
                  <div className="p-4 bg-[#9cf35b]/8 border border-[#9cf35b]/25 rounded-lg">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm font-medium text-[#9cf35b]">Generated Seed (Save this!)</span>
                      <button
                        onClick={() => navigator.clipboard.writeText(newSeed)}
                        className="p-1 hover:bg-[#9cf35b]/15 rounded transition-colors"
                        aria-label="Copy seed phrase"
                      >
                        <Copy className="w-4 h-4 text-[#9cf35b]" aria-hidden="true" />
                      </button>
                    </div>
                    <p className="text-sm text-[#9cf35b]/80 font-mono leading-relaxed break-all">{newSeed}</p>
                  </div>
                ) : (
                  <div>
                    <label htmlFor="import-seed" className="block text-sm font-medium text-white/70 mb-2">
                      Seed Phrase (24 words)
                    </label>
                    <textarea
                      id="import-seed"
                      value={importSeed}
                      onChange={(e) => setImportSeed(e.target.value)}
                      placeholder="Enter your 24-word seed phrase..."
                      rows={3}
                      className="w-full px-4 py-2 bg-white/3 border border-white/5 rounded-lg text-white placeholder-white/20 focus:outline-none focus:border-[#9cf35b]/30 font-mono text-sm"
                    />
                  </div>
                )}
              </div>
            </div>

            <div className="flex gap-3 justify-end mt-6">
              <button
                onClick={closeCreateWalletModal}
                className="px-4 py-2 text-white/30 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={createWallet}
                disabled={creatingWallet || !newWalletId || (!newSeed && !importSeed)}
                className="px-4 py-2 bg-[#9cf35b] text-[#000f61] rounded-lg hover:bg-[#bff658] transition-colors disabled:opacity-50 flex items-center gap-2"
              >
                {creatingWallet ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    Creating...
                  </>
                ) : (
                  <>
                    <Wallet className="w-4 h-4" />
                    Create Wallet
                  </>
                )}
              </button>
            </div>
        </Modal>
      )}
    </div>
  );
}
