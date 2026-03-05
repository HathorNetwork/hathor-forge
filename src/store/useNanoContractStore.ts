import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { NanoContract } from '@/types/nano-contracts';

interface NanoContractState {
  contracts: NanoContract[];
  addContract: (contract: NanoContract) => void;
  removeContract: (id: string) => void;
  updateContract: (id: string, updates: Partial<NanoContract>) => void;
  clearContracts: () => void;
}

export const useNanoContractStore = create<NanoContractState>()(
  persist(
    (set) => ({
      contracts: [],
      addContract: (contract) =>
        set((state) => ({
          contracts: state.contracts.some(c => c.id === contract.id) 
            ? state.contracts 
            : [...state.contracts, contract],
        })),
      removeContract: (id) =>
        set((state) => ({
          contracts: state.contracts.filter((c) => c.id !== id),
        })),
      updateContract: (id, updates) =>
        set((state) => ({
          contracts: state.contracts.map((c) =>
            c.id === id ? { ...c, ...updates } : c
          ),
        })),
      clearContracts: () => set({ contracts: [] }),
    }),
    {
      name: 'hathor-forge-nano-contracts',
    }
  )
);
