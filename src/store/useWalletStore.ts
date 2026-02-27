import { create } from "zustand";
import type { HeadlessWallet, HeadlessStatus, WalletBalance } from "@/types";

interface WalletState {
  // Headless service
  headlessStatus: HeadlessStatus;
  setHeadlessStatus: (status: HeadlessStatus) => void;
  headlessWallets: HeadlessWallet[];
  setHeadlessWallets: (wallets: HeadlessWallet[] | ((prev: HeadlessWallet[]) => HeadlessWallet[])) => void;

  // Faucet
  faucetAddress: string;
  setFaucetAddress: (addr: string) => void;
  faucetAmount: string;
  setFaucetAmount: (amount: string) => void;
  faucetBalance: WalletBalance | null;
  setFaucetBalance: (balance: WalletBalance | null) => void;
  sendingTx: boolean;
  setSendingTx: (sending: boolean) => void;
  txResult: { type: "success" | "error"; message: string } | null;
  setTxResult: (result: { type: "success" | "error"; message: string } | null) => void;

  // UI state
  showCreateWallet: boolean;
  setShowCreateWallet: (show: boolean) => void;
  newSeed: string | null;
  setNewSeed: (seed: string | null) => void;
  newWalletId: string;
  setNewWalletId: (id: string) => void;
  importSeed: string;
  setImportSeed: (seed: string) => void;
  creatingWallet: boolean;
  setCreatingWallet: (creating: boolean) => void;
  expandedWallet: string | null;
  setExpandedWallet: (id: string | null) => void;
  copiedAddress: string | null;
  setCopiedAddress: (addr: string | null) => void;
}

export const useWalletStore = create<WalletState>()((set, get) => ({
  // Headless service
  headlessStatus: { running: false, port: null },
  setHeadlessStatus: (status) => set({ headlessStatus: status }),
  headlessWallets: [],
  setHeadlessWallets: (wallets) => {
    if (typeof wallets === "function") {
      set({ headlessWallets: wallets(get().headlessWallets) });
    } else {
      set({ headlessWallets: wallets });
    }
  },

  // Faucet
  faucetAddress: "",
  setFaucetAddress: (addr) => set({ faucetAddress: addr }),
  faucetAmount: "100",
  setFaucetAmount: (amount) => set({ faucetAmount: amount }),
  faucetBalance: null,
  setFaucetBalance: (balance) => set({ faucetBalance: balance }),
  sendingTx: false,
  setSendingTx: (sending) => set({ sendingTx: sending }),
  txResult: null,
  setTxResult: (result) => set({ txResult: result }),

  // UI state
  showCreateWallet: false,
  setShowCreateWallet: (show) => set({ showCreateWallet: show }),
  newSeed: null,
  setNewSeed: (seed) => set({ newSeed: seed }),
  newWalletId: "",
  setNewWalletId: (id) => set({ newWalletId: id }),
  importSeed: "",
  setImportSeed: (seed) => set({ importSeed: seed }),
  creatingWallet: false,
  setCreatingWallet: (creating) => set({ creatingWallet: creating }),
  expandedWallet: null,
  setExpandedWallet: (id) => set({ expandedWallet: id }),
  copiedAddress: null,
  setCopiedAddress: (addr) => set({ copiedAddress: addr }),
}));
