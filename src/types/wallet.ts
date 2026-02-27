export interface WalletAddress {
  address: string;
  index: number;
  balance: number | null;
}

export interface HeadlessWallet {
  wallet_id: string;
  status: string;
  status_code: number | null;
  balance?: { available: number; locked: number };
  addresses?: string[];
  seed?: string;
}

export interface HeadlessStatus {
  running: boolean;
  port: number | null;
}

export interface SendTxRequest {
  address: string;
  amount: number;
}

export interface CreateHeadlessWalletRequest {
  wallet_id: string;
  seed: string;
}

export interface HeadlessWalletSendTxRequest {
  wallet_id: string;
  address: string;
  amount: number;
}

export interface WalletBalance {
  available: number;
  locked: number;
}
