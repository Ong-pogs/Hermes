import {
  PublicKey,
  SystemProgram,
  Connection,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getMint,
} from "@solana/spl-token";
import { Program, AnchorProvider, BN } from "@coral-xyz/anchor";
import type { Escrow as EscrowProgram } from "./idl/escrow_types";
import {
  CreateEscrowParams,
  ReleaseEscrowParams,
  RefundEscrowParams,
  CancelEscrowParams,
  EscrowInfo,
  TransactionResult,
  ListEscrowsParams,
  WalletAdapter,
} from "./types";

const CONFIG_SEED = Buffer.from("config");
const ESCROW_SEED = Buffer.from("escrow");
const VAULT_SEED = Buffer.from("vault");

export class EscrowModule {
  private program: Program<EscrowProgram>;
  private connection: Connection;
  private wallet: WalletAdapter;

  constructor(
    program: Program<EscrowProgram>,
    connection: Connection,
    wallet: WalletAdapter
  ) {
    this.program = program;
    this.connection = connection;
    this.wallet = wallet;
  }

  private getConfigPda(): PublicKey {
    const [pda] = PublicKey.findProgramAddressSync(
      [CONFIG_SEED],
      this.program.programId
    );
    return pda;
  }

  private getEscrowPda(creator: PublicKey, escrowId: number): PublicKey {
    const [pda] = PublicKey.findProgramAddressSync(
      [
        ESCROW_SEED,
        creator.toBuffer(),
        new BN(escrowId).toArrayLike(Buffer, "le", 8),
      ],
      this.program.programId
    );
    return pda;
  }

  private getVaultPda(escrowPda: PublicKey): PublicKey {
    const [pda] = PublicKey.findProgramAddressSync(
      [VAULT_SEED, escrowPda.toBuffer()],
      this.program.programId
    );
    return pda;
  }

  private toPublicKey(value: string | PublicKey): PublicKey {
    return typeof value === "string" ? new PublicKey(value) : value;
  }

  private parseStatus(status: any): EscrowInfo["status"] {
    if ("active" in status) return "active";
    if ("released" in status) return "released";
    if ("refunded" in status) return "refunded";
    if ("cancelled" in status) return "cancelled";
    return "active";
  }

  /**
   * Create a new escrow. Deposits tokens into a program-owned vault.
   *
   * @example
   * ```ts
   * const result = await api.escrow.create({
   *   recipient: "RecipientBase58Address...",
   *   amount: 100,
   *   token: "TokenMintBase58Address...",
   *   expiresIn: 7 * 24 * 60 * 60, // 7 days
   * });
   * console.log("Escrow created:", result.escrowAddress.toBase58());
   * ```
   */
  async create(params: CreateEscrowParams): Promise<TransactionResult> {
    const recipientKey = this.toPublicKey(params.recipient);
    const mintKey = this.toPublicKey(params.token);

    // Get mint info for decimal conversion
    const mintInfo = await getMint(this.connection, mintKey);
    const rawAmount = new BN(
      Math.floor(params.amount * 10 ** mintInfo.decimals)
    );

    // Calculate expiry
    let expiresAt: BN | null = null;
    if (params.expiresIn) {
      const now = Math.floor(Date.now() / 1000);
      expiresAt = new BN(now + params.expiresIn);
    }

    // Get current escrow count for PDA derivation
    const configPda = this.getConfigPda();
    const config = await this.program.account.protocolConfig.fetch(configPda);
    const escrowId = config.escrowCount.toNumber();

    const escrowPda = this.getEscrowPda(this.wallet.publicKey, escrowId);
    const creatorAta = getAssociatedTokenAddressSync(
      mintKey,
      this.wallet.publicKey
    );
    const feeRecipientAta = getAssociatedTokenAddressSync(
      mintKey,
      config.feeRecipient
    );

    const signature = await this.program.methods
      .createEscrow(rawAmount, expiresAt)
      .accountsPartial({
        config: configPda,
        mint: mintKey,
        creatorTokenAccount: creatorAta,
        feeTokenAccount: feeRecipientAta,
        creator: this.wallet.publicKey,
        recipient: recipientKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    return { signature, escrowAddress: escrowPda };
  }

  /**
   * Release escrowed funds to the recipient.
   *
   * @example
   * ```ts
   * await api.escrow.release({ escrowAddress: "EscrowBase58Address..." });
   * ```
   */
  async release(params: ReleaseEscrowParams): Promise<TransactionResult> {
    const escrowPda = this.toPublicKey(params.escrowAddress);
    const escrow = await this.program.account.escrowState.fetch(escrowPda);

    const recipientAta = getAssociatedTokenAddressSync(
      escrow.mint,
      escrow.recipient
    );

    const signature = await this.program.methods
      .releaseEscrow()
      .accountsPartial({
        escrow: escrowPda,
        recipientTokenAccount: recipientAta,
        mint: escrow.mint,
        recipient: escrow.recipient,
        creator: this.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    return { signature };
  }

  /**
   * Refund escrowed funds back to the creator.
   *
   * @example
   * ```ts
   * await api.escrow.refund({ escrowAddress: "EscrowBase58Address..." });
   * ```
   */
  async refund(params: RefundEscrowParams): Promise<TransactionResult> {
    const escrowPda = this.toPublicKey(params.escrowAddress);
    const escrow = await this.program.account.escrowState.fetch(escrowPda);

    const creatorAta = getAssociatedTokenAddressSync(
      escrow.mint,
      escrow.creator
    );

    const signature = await this.program.methods
      .refundEscrow()
      .accountsPartial({
        escrow: escrowPda,
        creatorTokenAccount: creatorAta,
        mint: escrow.mint,
        creator: this.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    return { signature };
  }

  /**
   * Cancel an active escrow and return funds to creator.
   *
   * @example
   * ```ts
   * await api.escrow.cancel({ escrowAddress: "EscrowBase58Address..." });
   * ```
   */
  async cancel(params: CancelEscrowParams): Promise<TransactionResult> {
    const escrowPda = this.toPublicKey(params.escrowAddress);
    const escrow = await this.program.account.escrowState.fetch(escrowPda);

    const creatorAta = getAssociatedTokenAddressSync(
      escrow.mint,
      escrow.creator
    );

    const signature = await this.program.methods
      .cancelEscrow()
      .accountsPartial({
        escrow: escrowPda,
        creatorTokenAccount: creatorAta,
        mint: escrow.mint,
        creator: this.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    return { signature };
  }

  /**
   * Fetch a single escrow's state.
   *
   * @example
   * ```ts
   * const info = await api.escrow.get("EscrowBase58Address...");
   * console.log(info.status, info.amountFormatted);
   * ```
   */
  async get(escrowAddress: string | PublicKey): Promise<EscrowInfo> {
    const address = this.toPublicKey(escrowAddress);
    const escrow = await this.program.account.escrowState.fetch(address);
    const mintInfo = await getMint(this.connection, escrow.mint);

    return {
      address,
      creator: escrow.creator,
      recipient: escrow.recipient,
      mint: escrow.mint,
      amount: escrow.amount,
      amountFormatted: escrow.amount.toNumber() / 10 ** mintInfo.decimals,
      status: this.parseStatus(escrow.status),
      escrowId: escrow.escrowId.toNumber(),
      createdAt: new Date(escrow.createdAt.toNumber() * 1000),
      expiresAt: escrow.expiresAt
        ? new Date(escrow.expiresAt.toNumber() * 1000)
        : null,
    };
  }

  /**
   * List all escrows for a given creator.
   *
   * @example
   * ```ts
   * const escrows = await api.escrow.list({ creator: wallet.publicKey });
   * escrows.forEach(e => console.log(e.escrowId, e.status, e.amountFormatted));
   * ```
   */
  async list(params?: ListEscrowsParams): Promise<EscrowInfo[]> {
    const creator = params?.creator
      ? this.toPublicKey(params.creator)
      : this.wallet.publicKey;

    const accounts = await this.program.account.escrowState.all([
      {
        memcmp: {
          offset: 8, // after discriminator
          bytes: creator.toBase58(),
        },
      },
    ]);

    const results: EscrowInfo[] = [];
    for (const acc of accounts) {
      const mintInfo = await getMint(this.connection, acc.account.mint);
      results.push({
        address: acc.publicKey,
        creator: acc.account.creator,
        recipient: acc.account.recipient,
        mint: acc.account.mint,
        amount: acc.account.amount,
        amountFormatted:
          acc.account.amount.toNumber() / 10 ** mintInfo.decimals,
        status: this.parseStatus(acc.account.status),
        escrowId: acc.account.escrowId.toNumber(),
        createdAt: new Date(acc.account.createdAt.toNumber() * 1000),
        expiresAt: acc.account.expiresAt
          ? new Date(acc.account.expiresAt.toNumber() * 1000)
          : null,
      });
    }

    return results.sort((a, b) => a.escrowId - b.escrowId);
  }
}
