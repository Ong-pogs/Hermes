import { Connection, PublicKey } from "@solana/web3.js";
import { Program, AnchorProvider } from "@coral-xyz/anchor";
import type { Escrow as EscrowProgram } from "./idl/escrow_types";
import { EscrowModule } from "./escrow";
import { SmartContractAPIConfig } from "./types";
import escrowIdl from "./idl/escrow.json";

// Default program ID — update after deployment
const DEFAULT_PROGRAM_ID = new PublicKey(
  "8Qu8qouNV7CZ4MUEX7rDpAruLFXhvaUruBQRoSViewDY"
);

export class SmartContractAPI {
  /** Escrow operations: create, release, refund, cancel, list */
  public escrow: EscrowModule;

  private connection: Connection;
  private program: Program<EscrowProgram>;

  /**
   * Initialize the SmartContractAPI client.
   *
   * @example
   * ```ts
   * import { SmartContractAPI } from '@smartcontractapi/solana';
   * import { Connection } from '@solana/web3.js';
   *
   * const api = new SmartContractAPI({
   *   connection: new Connection('https://api.devnet.solana.com'),
   *   wallet: phantomWallet, // any wallet adapter
   * });
   *
   * // Create an escrow
   * const result = await api.escrow.create({
   *   recipient: 'RecipientAddress...',
   *   amount: 100,           // 100 tokens (human-readable)
   *   token: 'MintAddress...', // SPL token mint
   *   expiresIn: 604800,     // 7 days in seconds
   * });
   * ```
   */
  constructor(config: SmartContractAPIConfig) {
    this.connection = config.connection;

    const commitment = config.commitment || "confirmed";

    const provider = new AnchorProvider(
      this.connection,
      config.wallet as any,
      { commitment }
    );

    this.program = new Program<EscrowProgram>(escrowIdl as any, provider);

    this.escrow = new EscrowModule(
      this.program,
      this.connection,
      config.wallet
    );
  }
}
