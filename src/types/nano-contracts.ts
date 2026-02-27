export interface NanoContract {
  id: string;
  blueprintId: string;
  balance: number;
  status: string;
}

export interface Blueprint {
  id: string;
  name: string;
  timestamp: number;
}

export interface BlueprintInfo {
  id: string;
  success: boolean;
  plan: {
    name: string;
    methods: BlueprintMethod[];
  };
}

export interface BlueprintMethod {
  name: string;
  args: BlueprintArg[];
}

export interface BlueprintArg {
  name: string;
  type: string;
}

export interface ContractState {
  success: boolean;
  blueprint_name?: string;
  blueprint_id?: string;
  balance?: number;
  state?: Record<string, unknown>;
}

export interface CreateNanoContractRequest {
  wallet_id: string;
  blueprint_id: string;
  args: unknown[];
}

export interface CallNanoContractMethodRequest {
  wallet_id: string;
  nc_id: string;
  method: string;
  args: unknown[];
}
