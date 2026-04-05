import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

export interface WalletAdapter {
  publicKey: PublicKey;
  signTransaction: (tx: any) => Promise<any>;
  signAllTransactions: (txs: any[]) => Promise<any[]>;
}

export interface SmartContractAPIConfig {
  /** Solana RPC connection */
  connection: any; // Connection from @solana/web3.js
  /** Wallet adapter (Phantom, Solflare, etc.) */
  wallet: WalletAdapter;
  /** Program ID override (defaults to deployed address) */
  programId?: PublicKey;
  /** Commitment level */
  commitment?: "processed" | "confirmed" | "finalized";
}

export interface CreateEscrowParams {
  /** Recipient's wallet address (base58 string or PublicKey) */
  recipient: string | PublicKey;
  /** Amount in human-readable units (e.g. 1.5 for 1.5 tokens) */
  amount: number;
  /** SPL token mint address (base58 string or PublicKey) */
  token: string | PublicKey;
  /** Expiry in seconds from now (optional) */
  expiresIn?: number;
}

export interface ReleaseEscrowParams {
  /** The escrow account address */
  escrowAddress: string | PublicKey;
}

export interface RefundEscrowParams {
  /** The escrow account address */
  escrowAddress: string | PublicKey;
}

export interface CancelEscrowParams {
  /** The escrow account address */
  escrowAddress: string | PublicKey;
}

export interface EscrowInfo {
  /** Escrow account address */
  address: PublicKey;
  /** Creator's wallet */
  creator: PublicKey;
  /** Recipient's wallet */
  recipient: PublicKey;
  /** Token mint */
  mint: PublicKey;
  /** Amount in raw token units */
  amount: BN;
  /** Human-readable amount */
  amountFormatted: number;
  /** Escrow status */
  status: "active" | "released" | "refunded" | "cancelled";
  /** Unique escrow ID */
  escrowId: number;
  /** Creation timestamp */
  createdAt: Date;
  /** Expiry timestamp (null if no expiry) */
  expiresAt: Date | null;
}

export interface TransactionResult {
  /** Transaction signature */
  signature: string;
  /** Escrow account address (for create operations) */
  escrowAddress?: PublicKey;
}

export interface ListEscrowsParams {
  /** Filter by creator wallet */
  creator?: string | PublicKey;
}
