import { useState, useCallback } from "react";
import {
  Zap, Plus, Search, Copy, Clock, FileCode, RefreshCw,
  Trash2, Loader2, Play, Square,
} from "lucide-react";
import { useUIStore } from "@/store/useUIStore";
import { useWalletStore } from "@/store/useWalletStore";
import { useNanoContractStore } from "@/store/useNanoContractStore";
import { useBlueprints } from "@/hooks/useBlueprints";
import { useEscapeKey } from "@/hooks/useEscapeKey";
import * as api from "@/services/tauri";
import { formatHTR } from "@/lib/utils";
import type { BlueprintInfo, BlueprintMethod, BlueprintArg, NanoContract } from "@/types/nano-contracts";

export function NanoContractsPage() {
  const setError = useUIStore((s) => s.setError);
  const { headlessWallets } = useWalletStore();
  const { contracts, addContract, removeContract } = useNanoContractStore();
  const { data: blueprints = [], refetch: refetchBlueprints } = useBlueprints();
  const fetchBlueprints = () => { refetchBlueprints(); };

  const { setTxResult } = useWalletStore();

  // Local state
  const [showRegisterContract, setShowRegisterContract] = useState(false);
  const [registerContractId, setRegisterContractId] = useState("");
  const [nanoSearch, setNanoSearch] = useState("");
  const [showInitWizard, setShowInitWizard] = useState(false);
  const [selectedBlueprint, setSelectedBlueprint] = useState<BlueprintInfo | null>(null);
  const [initArgs, setInitArgs] = useState<Record<string, string>>({});
  const [selectedWalletForInit, setSelectedWalletForInit] = useState("");
  const [selectedContractForInteract, setSelectedContractForInteract] = useState<(NanoContract & { blueprint: BlueprintInfo; state: Record<string, unknown> }) | null>(null);
  const [selectedMethod, setSelectedMethod] = useState<BlueprintMethod | null>(null);
  const [methodArgs, setMethodArgs] = useState<Record<string, string>>({});
  const [selectedWalletForInteract, setSelectedWalletForInteract] = useState("");
  const [isInitializing, setIsInitializing] = useState(false);
  const [isSubmittingMethod, setIsSubmittingMethod] = useState(false);

  // ── Escape-to-close modals ────────────────────────────────────────────────
  const closeRegisterModal = useCallback(() => {
    setShowRegisterContract(false);
    setRegisterContractId("");
  }, []);
  const closeInitWizard = useCallback(() => {
    setShowInitWizard(false);
    setSelectedBlueprint(null);
  }, []);
  const closeInteractionModal = useCallback(() => {
    setSelectedContractForInteract(null);
  }, []);

  const anyModalOpen = showRegisterContract || showInitWizard || !!selectedContractForInteract;
  const escapeHandler = useCallback(() => {
    if (selectedContractForInteract) closeInteractionModal();
    else if (showInitWizard) closeInitWizard();
    else if (showRegisterContract) closeRegisterModal();
  }, [selectedContractForInteract, showInitWizard, showRegisterContract, closeInteractionModal, closeInitWizard, closeRegisterModal]);
  useEscapeKey(escapeHandler, anyModalOpen);

  // ── Handlers ──────────────────────────────────────────────────────────────

  const handleSelectBlueprint = async (id: string) => {
    try {
      const info = await api.getBlueprintInformation(id);
      if (info.success) {
        setSelectedBlueprint(info);
        const initialArgs: Record<string, string> = {};
        const constructor = info.plan.methods.find((m: BlueprintMethod) => m.name === "initialize");
        if (constructor) {
          constructor.args.forEach((arg: BlueprintArg) => {
            initialArgs[arg.name] = "";
          });
        }
        setInitArgs(initialArgs);
      }
    } catch (e) {
      console.error("Failed to fetch blueprint information:", e);
    }
  };

  const handleInitializeContract = async () => {
    if (!selectedBlueprint || !selectedWalletForInit) return;

    setIsInitializing(true);
    try {
      const constructor = selectedBlueprint.plan.methods.find((m: BlueprintMethod) => m.name === "initialize");
      const args = constructor!.args.map((arg: BlueprintArg) => {
        const val = initArgs[arg.name];
        if (arg.type === "int" || arg.type === "float") {
          return Number(val);
        }
        return val;
      });

      const response = await api.headlessWalletCreateNanoContract({
        wallet_id: selectedWalletForInit,
        blueprint_id: selectedBlueprint.id,
        args: args,
      });

      if (response.success) {
        addContract({
          id: response.hash,
          blueprintId: selectedBlueprint.plan.name,
          balance: 0,
          status: "initializing",
        });
        setShowInitWizard(false);
        setSelectedBlueprint(null);
        setInitArgs({});
      } else {
        setError(response.message || "Failed to initialize nano contract");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsInitializing(false);
    }
  };

  const handleOpenInteract = async (contract: NanoContract) => {
    try {
      const state = await api.getNanoContractState(contract.id);
      if (state.success) {
        const info = await api.getBlueprintInformation(state.blueprint_id!);
        if (info.success) {
          setSelectedContractForInteract({ ...contract, blueprint: info, state: state.state ?? {} });
          setSelectedMethod(null);
          setMethodArgs({});
        }
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const handleCallMethod = async () => {
    if (!selectedContractForInteract || !selectedMethod || !selectedWalletForInteract) return;

    setIsSubmittingMethod(true);
    try {
      const args = selectedMethod.args.map((arg: BlueprintArg) => {
        const val = methodArgs[arg.name];
        if (arg.type === "int" || arg.type === "float") {
          return Number(val);
        }
        return val;
      });

      const response = await api.headlessWalletCallNanoContractMethod({
        wallet_id: selectedWalletForInteract,
        nc_id: selectedContractForInteract.id,
        method: selectedMethod.name,
        args: args,
      });

      if (response.success) {
        setTxResult({ type: "success", message: `Method called! Hash: ${response.hash}` });
        setSelectedContractForInteract(null);
        setSelectedMethod(null);
      } else {
        setError(response.message || "Failed to call method");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsSubmittingMethod(false);
    }
  };

  // ── Render ────────────────────────────────────────────────────────────────

  const filteredContracts = contracts.filter(c =>
    c.id.toLowerCase().includes(nanoSearch.toLowerCase()) ||
    c.blueprintId.toLowerCase().includes(nanoSearch.toLowerCase())
  );

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white mb-2">Nano Contracts</h2>
          <p className="text-slate-500">Deploy, manage, and interact with nano contracts</p>
        </div>
        <div className="flex gap-3">
          <button
            onClick={() => setShowRegisterContract(true)}
            className="px-4 py-2 bg-slate-800 border border-slate-700 text-slate-300 rounded-lg hover:bg-slate-700 transition-colors flex items-center gap-2"
          >
            <Plus className="w-4 h-4" />
            Register Contract
          </button>
          <button
            onClick={() => {
              setShowInitWizard(true);
              setSelectedBlueprint(null);
              setInitArgs({});
            }}
            className="px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors flex items-center gap-2"
          >
            <Zap className="w-4 h-4" />
            Initialize (OCB)
          </button>
        </div>
      </div>

      {/* Search and Filters */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
        <input
          type="text"
          placeholder="Search by Contract ID or Blueprint ID..."
          value={nanoSearch}
          onChange={(e) => setNanoSearch(e.target.value)}
          className="w-full pl-10 pr-4 py-2 bg-[#0d1117] border border-slate-800 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
        />
      </div>

      {/* Blueprints on Network */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wider">Blueprints on Network</h3>
          <button
            onClick={fetchBlueprints}
            className="p-1.5 text-slate-500 hover:text-slate-300 transition-colors rounded-lg hover:bg-slate-800"
            title="Refresh blueprints"
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>
        {blueprints.length === 0 ? (
          <div className="border border-slate-800 rounded-lg bg-slate-900/30 p-6 text-center">
            <FileCode className="w-8 h-8 text-slate-600 mx-auto mb-2" />
            <p className="text-sm text-slate-500">No blueprints published on the network yet.</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {blueprints.map((bp) => (
              <div key={bp.id} className="border border-slate-800 rounded-lg bg-slate-900/30 p-4 hover:border-amber-500/30 transition-colors">
                <div className="flex items-start gap-3">
                  <div className="w-9 h-9 rounded-lg bg-amber-500/10 flex items-center justify-center shrink-0 mt-0.5">
                    <FileCode className="w-4 h-4 text-amber-400" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="font-semibold text-white text-sm">{bp.name}</div>
                    <div className="flex items-center gap-1.5 mt-1">
                      <span className="text-[11px] text-slate-500 font-mono truncate">{bp.id.slice(0, 20)}...</span>
                      <button onClick={() => navigator.clipboard.writeText(bp.id)} className="p-0.5 hover:bg-slate-800 rounded shrink-0">
                        <Copy className="w-2.5 h-2.5 text-slate-500" />
                      </button>
                    </div>
                    {bp.timestamp > 0 && (
                      <div className="flex items-center gap-1 mt-1.5 text-[10px] text-slate-600">
                        <Clock className="w-3 h-3" />
                        {new Date(bp.timestamp * 1000).toLocaleString()}
                      </div>
                    )}
                  </div>
                </div>
                <button
                  onClick={() => {
                    setShowInitWizard(true);
                    handleSelectBlueprint(bp.id);
                  }}
                  className="mt-3 w-full px-3 py-1.5 text-xs font-medium bg-slate-800 text-slate-300 rounded-md hover:bg-slate-700 hover:text-white transition-colors flex items-center justify-center gap-1.5"
                >
                  <Zap className="w-3 h-3" />
                  Initialize Contract
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Contracts List */}
      <div className="space-y-4">
        {filteredContracts.length === 0 ? (
          <div className="border border-slate-800 rounded-xl bg-slate-900/30 p-12 text-center">
            <Zap className="w-12 h-12 text-slate-600 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">No Nano Contracts found</h3>
            <p className="text-slate-500 max-w-md mx-auto mb-6">
              You haven't registered any nano contracts yet. Register an existing one or initialize a new one from a blueprint.
            </p>
            <div className="flex justify-center gap-3">
              <button
                onClick={() => setShowRegisterContract(true)}
                className="px-4 py-2 bg-slate-800 text-white rounded-lg hover:bg-slate-700 transition-colors"
              >
                Register Contract
              </button>
              <button
                onClick={() => {
                  setShowInitWizard(true);
                  setSelectedBlueprint(null);
                  setInitArgs({});
                }}
                className="px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors"
              >
                Initialize OCB
              </button>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-4">
            {filteredContracts.map((contract) => (
              <div key={contract.id} className="border border-slate-800 rounded-xl bg-slate-900/30 overflow-hidden hover:border-slate-700 transition-colors">
                <div className="p-5">
                  <div className="flex items-center justify-between mb-4">
                    <div className="flex items-center gap-3">
                      <div className="w-10 h-10 rounded-lg bg-amber-500/10 flex items-center justify-center">
                        <Zap className="w-5 h-5 text-amber-400" />
                      </div>
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="font-mono font-semibold text-white">{contract.id.slice(0, 16)}...</span>
                          <button onClick={() => navigator.clipboard.writeText(contract.id)} className="p-1 hover:bg-slate-800 rounded">
                            <Copy className="w-3 h-3 text-slate-500" />
                          </button>
                        </div>
                        <span className="text-xs text-slate-500">Blueprint: {contract.blueprintId}</span>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <div className="text-right mr-4">
                        <div className="text-sm font-semibold text-white">{formatHTR(contract.balance)} HTR</div>
                        <div className="text-[10px] text-slate-500 uppercase font-bold tracking-wider">{contract.status}</div>
                      </div>
                      <button
                        onClick={() => handleOpenInteract(contract)}
                        className="px-4 py-2 bg-amber-500 text-white text-sm font-semibold rounded-lg hover:bg-amber-600 transition-colors"
                      >
                        Interact
                      </button>
                      <button
                        onClick={() => removeContract(contract.id)}
                        className="p-2 text-slate-500 hover:text-rose-400 transition-colors"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Register Contract Modal */}
      {showRegisterContract && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
          <div className="bg-[#0d1117] border border-slate-800 rounded-xl p-6 max-w-lg w-full mx-4 shadow-2xl">
            <div className="flex items-center gap-3 mb-6">
              <div className="w-10 h-10 rounded-full bg-amber-500/20 flex items-center justify-center">
                <Plus className="w-5 h-5 text-amber-400" />
              </div>
              <h3 className="text-lg font-semibold text-white">Register Nano Contract</h3>
            </div>
            <p className="text-slate-400 mb-6">
              Enter the ID of an existing nano contract to start tracking and interacting with it.
            </p>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Contract ID
                </label>
                <input
                  type="text"
                  value={registerContractId}
                  onChange={(e) => setRegisterContractId(e.target.value)}
                  placeholder="Enter contract ID (hex)..."
                  className="w-full px-4 py-2 bg-slate-900 border border-slate-700 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
                />
              </div>
            </div>
            <div className="flex gap-3 justify-end mt-8">
              <button
                onClick={() => {
                  setShowRegisterContract(false);
                  setRegisterContractId("");
                }}
                className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={() => {
                  if (registerContractId) {
                    addContract({
                      id: registerContractId,
                      blueprintId: "Checking...",
                      balance: 0,
                      status: "active",
                    });
                    setShowRegisterContract(false);
                    setRegisterContractId("");
                  }
                }}
                disabled={!registerContractId}
                className="px-6 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors disabled:opacity-50"
              >
                Register
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Initialization Wizard Modal */}
      {showInitWizard && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
          <div className="bg-[#0d1117] border border-slate-800 rounded-xl p-6 max-w-2xl w-full mx-4 shadow-2xl max-h-[90vh] overflow-y-auto">
            <div className="flex items-center gap-3 mb-6">
              <div className="w-10 h-10 rounded-full bg-amber-500/20 flex items-center justify-center">
                <Zap className="w-5 h-5 text-amber-400" />
              </div>
              <h3 className="text-lg font-semibold text-white">Initialize Nano Contract (OCB)</h3>
            </div>

            {!selectedBlueprint ? (
              <div className="space-y-4">
                <p className="text-slate-400 mb-4">Select a blueprint to initialize:</p>
                <div className="grid grid-cols-1 gap-2">
                  {blueprints.length > 0 ? (
                    blueprints.map((bp) => (
                      <button
                        key={bp.id}
                        onClick={() => handleSelectBlueprint(bp.id)}
                        className="flex items-center justify-between p-4 rounded-lg bg-slate-900 border border-slate-800 hover:border-amber-500/50 transition-colors text-left"
                      >
                        <div>
                          <div className="font-semibold text-white">{bp.name}</div>
                          <div className="text-xs text-slate-500 font-mono">{bp.id}</div>
                        </div>
                        <Plus className="w-4 h-4 text-slate-500" />
                      </button>
                    ))
                  ) : (
                    <div className="text-center py-8 text-slate-500">
                      No blueprints available on the network.
                    </div>
                  )}
                </div>
              </div>
            ) : (
              <div className="space-y-6">
                <div>
                  <button
                    onClick={() => setSelectedBlueprint(null)}
                    className="text-xs text-amber-400 hover:text-amber-300 mb-2 flex items-center gap-1"
                  >
                    &larr; Back to blueprints
                  </button>
                  <h4 className="font-bold text-white text-xl">{selectedBlueprint.plan.name}</h4>
                  <p className="text-sm text-slate-500 font-mono">{selectedBlueprint.id}</p>
                </div>

                {/* Initialization Arguments */}
                <div className="space-y-4 border-t border-slate-800 pt-4">
                  <h5 className="text-sm font-semibold text-slate-300 uppercase tracking-wider">Initialization Arguments</h5>
                  {selectedBlueprint.plan.methods.find((m: BlueprintMethod) => m.name === "initialize")?.args.map((arg: BlueprintArg) => (
                    <div key={arg.name}>
                      <label className="block text-sm font-medium text-slate-400 mb-1">
                        {arg.name} <span className="text-[10px] text-slate-600 uppercase">({arg.type})</span>
                      </label>
                      <input
                        type="text"
                        value={initArgs[arg.name] || ""}
                        onChange={(e) => setInitArgs({ ...initArgs, [arg.name]: e.target.value })}
                        placeholder={`Enter ${arg.name}...`}
                        className="w-full px-4 py-2 bg-slate-900 border border-slate-700 rounded-lg text-white placeholder-slate-600 focus:outline-none focus:border-amber-500/50"
                      />
                    </div>
                  )) || <p className="text-sm text-slate-500 italic">No initialization arguments required.</p>}
                </div>

                {/* Wallet Selection */}
                <div className="space-y-4 border-t border-slate-800 pt-4">
                  <h5 className="text-sm font-semibold text-slate-300 uppercase tracking-wider">Signing Wallet</h5>
                  <select
                    value={selectedWalletForInit}
                    onChange={(e) => setSelectedWalletForInit(e.target.value)}
                    className="w-full px-4 py-2 bg-slate-900 border border-slate-700 rounded-lg text-white focus:outline-none focus:border-amber-500/50"
                  >
                    <option value="">Select a wallet...</option>
                    {headlessWallets.filter(w => w.status_code === 3).map(w => (
                      <option key={w.wallet_id} value={w.wallet_id}>
                        {w.wallet_id} ({(w.balance?.available || 0) / 100} HTR)
                      </option>
                    ))}
                  </select>
                  {headlessWallets.filter(w => w.status_code === 3).length === 0 && (
                    <p className="text-xs text-rose-400">No ready wallets found. Create or start a wallet in the Wallet Manager.</p>
                  )}
                </div>

                <div className="flex gap-3 justify-end mt-8">
                  <button
                    onClick={() => {
                      setShowInitWizard(false);
                      setSelectedBlueprint(null);
                    }}
                    className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleInitializeContract}
                    disabled={isInitializing || !selectedWalletForInit}
                    className="px-6 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors disabled:opacity-50 flex items-center gap-2"
                  >
                    {isInitializing ? (
                      <>
                        <Loader2 className="w-4 h-4 animate-spin" />
                        Initializing...
                      </>
                    ) : (
                      <>
                        <Zap className="w-4 h-4" />
                        Initialize Contract
                      </>
                    )}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Interaction Modal */}
      {selectedContractForInteract && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
          <div className="bg-[#0d1117] border border-slate-800 rounded-xl p-6 max-w-4xl w-full mx-4 shadow-2xl max-h-[90vh] overflow-hidden flex flex-col">
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-amber-500/20 flex items-center justify-center">
                  <Zap className="w-5 h-5 text-amber-400" />
                </div>
                <div>
                  <h3 className="text-lg font-semibold text-white">Interact with Nano Contract</h3>
                  <p className="text-xs text-slate-500 font-mono">{selectedContractForInteract.id}</p>
                </div>
              </div>
              <button
                onClick={() => setSelectedContractForInteract(null)}
                className="p-2 text-slate-500 hover:text-white transition-colors"
              >
                <Square className="w-5 h-5" />
              </button>
            </div>

            <div className="flex-1 overflow-hidden flex gap-6">
              {/* Left Panel: Methods and State */}
              <div className="w-1/3 flex flex-col gap-6 overflow-y-auto pr-2">
                <div>
                  <h4 className="text-sm font-semibold text-slate-300 uppercase tracking-wider mb-3">Methods</h4>
                  <div className="space-y-2">
                    {selectedContractForInteract.blueprint.plan.methods
                      .filter((m: BlueprintMethod) => m.name !== "initialize")
                      .map((method: BlueprintMethod) => (
                        <button
                          key={method.name}
                          onClick={() => {
                            setSelectedMethod(method);
                            setMethodArgs({});
                          }}
                          className={`w-full text-left px-4 py-2 rounded-lg border transition-all ${
                            selectedMethod?.name === method.name
                              ? "bg-amber-500/10 border-amber-500/50 text-amber-400"
                              : "bg-slate-900 border-slate-800 text-slate-400 hover:border-slate-700"
                          }`}
                        >
                          <div className="font-semibold">{method.name}</div>
                        </button>
                      ))}
                  </div>
                </div>

                <div>
                  <h4 className="text-sm font-semibold text-slate-300 uppercase tracking-wider mb-3">Current State</h4>
                  <div className="bg-slate-950 rounded-lg p-4 font-mono text-xs text-emerald-400 overflow-x-auto">
                    <pre>{JSON.stringify(selectedContractForInteract.state, null, 2)}</pre>
                  </div>
                </div>
              </div>

              {/* Right Panel: Call Form */}
              <div className="flex-1 bg-slate-900/50 rounded-xl border border-slate-800 p-6 overflow-y-auto">
                {selectedMethod ? (
                  <div className="space-y-6">
                    <div>
                      <h4 className="text-xl font-bold text-white mb-1">{selectedMethod.name}</h4>
                      <p className="text-sm text-slate-500">Enter arguments to call this method</p>
                    </div>

                    <div className="space-y-4">
                      {selectedMethod.args.map((arg: BlueprintArg) => (
                        <div key={arg.name}>
                          <label className="block text-sm font-medium text-slate-400 mb-1">
                            {arg.name} <span className="text-[10px] text-slate-600 uppercase">({arg.type})</span>
                          </label>
                          <input
                            type="text"
                            value={methodArgs[arg.name] || ""}
                            onChange={(e) => setMethodArgs({ ...methodArgs, [arg.name]: e.target.value })}
                            placeholder={`Enter ${arg.name}...`}
                            className="w-full px-4 py-2 bg-slate-900 border border-slate-700 rounded-lg text-white placeholder-slate-600 focus:outline-none focus:border-amber-500/50"
                          />
                        </div>
                      ))}

                      {selectedMethod.args.length === 0 && (
                        <p className="text-sm text-slate-500 italic">No arguments required for this method.</p>
                      )}
                    </div>

                    {/* Wallet Selection */}
                    <div className="space-y-4 border-t border-slate-800 pt-6">
                      <h5 className="text-sm font-semibold text-slate-300 uppercase tracking-wider">Signing Wallet</h5>
                      <select
                        value={selectedWalletForInteract}
                        onChange={(e) => setSelectedWalletForInteract(e.target.value)}
                        className="w-full px-4 py-2 bg-slate-900 border border-slate-700 rounded-lg text-white focus:outline-none focus:border-amber-500/50"
                      >
                        <option value="">Select a wallet...</option>
                        {headlessWallets.filter(w => w.status_code === 3).map(w => (
                          <option key={w.wallet_id} value={w.wallet_id}>
                            {w.wallet_id} ({(w.balance?.available || 0) / 100} HTR)
                          </option>
                        ))}
                      </select>
                    </div>

                    <div className="flex gap-3 justify-end pt-6">
                      <button
                        onClick={() => handleCallMethod()}
                        disabled={isSubmittingMethod || !selectedWalletForInteract}
                        className="px-8 py-3 bg-amber-500 text-white font-bold rounded-lg hover:bg-amber-600 transition-colors shadow-lg shadow-amber-500/20 disabled:opacity-50 flex items-center gap-2"
                      >
                        {isSubmittingMethod ? (
                          <>
                            <Loader2 className="w-4 h-4 animate-spin" />
                            Calling Method...
                          </>
                        ) : (
                          <>
                            <Play className="w-4 h-4" />
                            Call {selectedMethod.name}
                          </>
                        )}
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="h-full flex flex-col items-center justify-center text-center">
                    <Zap className="w-12 h-12 text-slate-700 mb-4" />
                    <h4 className="text-lg font-semibold text-slate-400">Select a method</h4>
                    <p className="text-sm text-slate-600 max-w-xs">
                      Select one of the available methods from the left panel to call it on this contract.
                    </p>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
