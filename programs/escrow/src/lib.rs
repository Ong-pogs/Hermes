use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    self, Mint, TokenAccount, TokenInterface, TransferChecked,
};

declare_id!("8Qu8qouNV7CZ4MUEX7rDpAruLFXhvaUruBQRoSViewDY");

// Seeds
pub const CONFIG_SEED: &[u8] = b"config";
pub const ESCROW_SEED: &[u8] = b"escrow";
pub const VAULT_SEED: &[u8] = b"vault";

#[program]
pub mod escrow {
    use super::*;

    /// One-time initialization of protocol configuration.
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        fee_bps: u16,
    ) -> Result<()> {
        require!(fee_bps <= 10_000, EscrowError::InvalidFeeBps);

        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.fee_bps = fee_bps;
        config.fee_recipient = ctx.accounts.fee_recipient.key();
        config.escrow_count = 0;
        config.bump = ctx.bumps.config;

        emit!(ConfigInitialized {
            authority: config.authority,
            fee_bps,
            fee_recipient: config.fee_recipient,
        });

        Ok(())
    }

    /// Create a new escrow: deposit SPL tokens into a program-owned vault.
    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        amount: u64,
        expires_at: Option<i64>,
    ) -> Result<()> {
        require!(amount > 0, EscrowError::ZeroAmount);

        if let Some(exp) = expires_at {
            let clock = Clock::get()?;
            require!(exp > clock.unix_timestamp, EscrowError::ExpiryInPast);
        }

        let config = &mut ctx.accounts.config;
        let escrow_id = config.escrow_count;
        config.escrow_count = config.escrow_count.checked_add(1).unwrap();

        // Calculate protocol fee
        let fee_amount = (amount as u128)
            .checked_mul(config.fee_bps as u128)
            .unwrap()
            .checked_div(10_000)
            .unwrap() as u64;
        let deposit_amount = amount.checked_sub(fee_amount).unwrap();

        // Transfer deposit to vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.creator_token_account.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.creator.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
        );
        token_interface::transfer_checked(
            transfer_ctx,
            deposit_amount,
            ctx.accounts.mint.decimals,
        )?;

        // Transfer fee to fee recipient (if any)
        if fee_amount > 0 {
            let fee_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.creator_token_account.to_account_info(),
                    to: ctx.accounts.fee_token_account.to_account_info(),
                    authority: ctx.accounts.creator.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
            );
            token_interface::transfer_checked(fee_ctx, fee_amount, ctx.accounts.mint.decimals)?;
        }

        // Initialize escrow state
        let escrow = &mut ctx.accounts.escrow;
        escrow.creator = ctx.accounts.creator.key();
        escrow.recipient = ctx.accounts.recipient.key();
        escrow.mint = ctx.accounts.mint.key();
        escrow.amount = deposit_amount;
        escrow.status = EscrowStatus::Active;
        escrow.escrow_id = escrow_id;
        escrow.created_at = Clock::get()?.unix_timestamp;
        escrow.expires_at = expires_at;
        escrow.bump = ctx.bumps.escrow;

        emit!(EscrowCreated {
            escrow: escrow.key(),
            creator: escrow.creator,
            recipient: escrow.recipient,
            mint: escrow.mint,
            amount: deposit_amount,
            fee: fee_amount,
            escrow_id,
            expires_at,
        });

        Ok(())
    }

    /// Creator releases escrowed funds to the recipient.
    pub fn release_escrow(ctx: Context<ReleaseEscrow>) -> Result<()> {
        let escrow = &ctx.accounts.escrow;
        require!(
            escrow.status == EscrowStatus::Active,
            EscrowError::NotActive
        );

        let escrow_key = escrow.key();
        let seeds = &[
            VAULT_SEED,
            escrow_key.as_ref(),
            &[ctx.bumps.vault],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            signer_seeds,
        );
        token_interface::transfer_checked(
            transfer_ctx,
            escrow.amount,
            ctx.accounts.mint.decimals,
        )?;

        let escrow = &mut ctx.accounts.escrow;
        escrow.status = EscrowStatus::Released;

        emit!(EscrowReleased {
            escrow: escrow.key(),
            recipient: escrow.recipient,
            amount: escrow.amount,
        });

        Ok(())
    }

    /// Creator refunds escrowed funds back to themselves.
    /// Only allowed if escrow is active and either: creator initiated, or escrow expired.
    pub fn refund_escrow(ctx: Context<RefundEscrow>) -> Result<()> {
        let escrow = &ctx.accounts.escrow;
        require!(
            escrow.status == EscrowStatus::Active,
            EscrowError::NotActive
        );

        let escrow_key = escrow.key();
        let seeds = &[
            VAULT_SEED,
            escrow_key.as_ref(),
            &[ctx.bumps.vault],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.creator_token_account.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            signer_seeds,
        );
        token_interface::transfer_checked(
            transfer_ctx,
            escrow.amount,
            ctx.accounts.mint.decimals,
        )?;

        let escrow = &mut ctx.accounts.escrow;
        escrow.status = EscrowStatus::Refunded;

        emit!(EscrowRefunded {
            escrow: escrow.key(),
            creator: escrow.creator,
            amount: escrow.amount,
        });

        Ok(())
    }

    /// Cancel an active escrow — alias for refund but only by creator.
    pub fn cancel_escrow(ctx: Context<CancelEscrow>) -> Result<()> {
        let escrow = &ctx.accounts.escrow;
        require!(
            escrow.status == EscrowStatus::Active,
            EscrowError::NotActive
        );

        let escrow_key = escrow.key();
        let seeds = &[
            VAULT_SEED,
            escrow_key.as_ref(),
            &[ctx.bumps.vault],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.creator_token_account.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            signer_seeds,
        );
        token_interface::transfer_checked(
            transfer_ctx,
            escrow.amount,
            ctx.accounts.mint.decimals,
        )?;

        let escrow = &mut ctx.accounts.escrow;
        escrow.status = EscrowStatus::Cancelled;

        emit!(EscrowCancelled {
            escrow: escrow.key(),
            creator: escrow.creator,
            amount: escrow.amount,
        });

        Ok(())
    }

    /// Update protocol fee (admin only).
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_fee_bps: Option<u16>,
        new_fee_recipient: Option<Pubkey>,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;

        if let Some(fee_bps) = new_fee_bps {
            require!(fee_bps <= 10_000, EscrowError::InvalidFeeBps);
            config.fee_bps = fee_bps;
        }

        if let Some(recipient) = new_fee_recipient {
            config.fee_recipient = recipient;
        }

        Ok(())
    }
}

// ============================================================
// Accounts
// ============================================================

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub authority: Pubkey,
    pub fee_bps: u16,
    pub fee_recipient: Pubkey,
    pub escrow_count: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct EscrowState {
    pub creator: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub status: EscrowStatus,
    pub escrow_id: u64,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum EscrowStatus {
    Active,
    Released,
    Refunded,
    Cancelled,
}

// ============================================================
// Instruction Contexts
// ============================================================

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + ProtocolConfig::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump,
    )]
    pub config: Account<'info, ProtocolConfig>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: fee recipient, validated by admin
    pub fee_recipient: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateEscrow<'info> {
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        init,
        payer = creator,
        space = 8 + EscrowState::INIT_SPACE,
        seeds = [ESCROW_SEED, creator.key().as_ref(), &config.escrow_count.to_le_bytes()],
        bump,
    )]
    pub escrow: Account<'info, EscrowState>,

    #[account(
        init,
        payer = creator,
        seeds = [VAULT_SEED, escrow.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = creator,
    )]
    pub creator_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = config.fee_recipient,
    )]
    pub fee_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub creator: Signer<'info>,
    /// CHECK: recipient pubkey, stored in escrow state
    pub recipient: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReleaseEscrow<'info> {
    #[account(
        mut,
        seeds = [ESCROW_SEED, escrow.creator.as_ref(), &escrow.escrow_id.to_le_bytes()],
        bump = escrow.bump,
        has_one = creator,
        has_one = mint,
    )]
    pub escrow: Account<'info, EscrowState>,

    #[account(
        mut,
        seeds = [VAULT_SEED, escrow.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = creator,
        associated_token::mint = mint,
        associated_token::authority = recipient,
    )]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,
    /// CHECK: validated via escrow.recipient
    #[account(address = escrow.recipient)]
    pub recipient: UncheckedAccount<'info>,
    #[account(mut)]
    pub creator: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RefundEscrow<'info> {
    #[account(
        mut,
        seeds = [ESCROW_SEED, escrow.creator.as_ref(), &escrow.escrow_id.to_le_bytes()],
        bump = escrow.bump,
        has_one = creator,
        has_one = mint,
    )]
    pub escrow: Account<'info, EscrowState>,

    #[account(
        mut,
        seeds = [VAULT_SEED, escrow.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = creator,
    )]
    pub creator_token_account: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub creator: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelEscrow<'info> {
    #[account(
        mut,
        seeds = [ESCROW_SEED, escrow.creator.as_ref(), &escrow.escrow_id.to_le_bytes()],
        bump = escrow.bump,
        has_one = creator,
        has_one = mint,
    )]
    pub escrow: Account<'info, EscrowState>,

    #[account(
        mut,
        seeds = [VAULT_SEED, escrow.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = creator,
    )]
    pub creator_token_account: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub creator: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = authority,
    )]
    pub config: Account<'info, ProtocolConfig>,
    pub authority: Signer<'info>,
}

// ============================================================
// Events
// ============================================================

#[event]
pub struct ConfigInitialized {
    pub authority: Pubkey,
    pub fee_bps: u16,
    pub fee_recipient: Pubkey,
}

#[event]
pub struct EscrowCreated {
    pub escrow: Pubkey,
    pub creator: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub fee: u64,
    pub escrow_id: u64,
    pub expires_at: Option<i64>,
}

#[event]
pub struct EscrowReleased {
    pub escrow: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
}

#[event]
pub struct EscrowRefunded {
    pub escrow: Pubkey,
    pub creator: Pubkey,
    pub amount: u64,
}

#[event]
pub struct EscrowCancelled {
    pub escrow: Pubkey,
    pub creator: Pubkey,
    pub amount: u64,
}

// ============================================================
// Errors
// ============================================================

#[error_code]
pub enum EscrowError {
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Escrow is not active")]
    NotActive,
    #[msg("Expiry must be in the future")]
    ExpiryInPast,
    #[msg("Fee basis points must be <= 10000")]
    InvalidFeeBps,
}
