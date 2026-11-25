use anchor_lang::prelude::*;
use anchor_lang::Key;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::MarkError;
use crate::events::{DepositEvent, WithdrawEvent};
use crate::state::{Exchange, User};
use crate::token_util::transfer_as_exchange;

use super::exchange::mint_to_exchange;

#[derive(Accounts)]
pub struct InitializeUser<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = 8 + User::INIT_SPACE,
        seeds = [b"user", owner.key().as_ref()],
        bump,
    )]
    pub user: Account<'info, User>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_user(ctx: Context<InitializeUser>) -> Result<()> {
    let user = &mut ctx.accounts.user;
    user.owner = ctx.accounts.owner.key();
    user.free_collateral = 0;
    user.realised_pnl = 0;
    user.faucet_claimed = 0;
    user.bump = ctx.bumps.user;
    Ok(())
}

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [b"exchange"], bump = exchange.bump)]
    pub exchange: Account<'info, Exchange>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user.bump,
    )]
    pub user: Account<'info, User>,
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = owner,
    )]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(mut, address = exchange.vault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(address = exchange.collateral_mint)]
    pub collateral_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handle_deposit(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
    if amount == 0 {
        return err!(MarkError::ZeroSize);
    }
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.user_ata.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
    )?;
    let user = &mut ctx.accounts.user;
    user.free_collateral = user
        .free_collateral
        .checked_add(amount)
        .ok_or(error!(MarkError::Math))?;
    emit!(DepositEvent {
        owner: user.owner,
        amount,
        free_after: user.free_collateral,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [b"exchange"], bump = exchange.bump)]
    pub exchange: Account<'info, Exchange>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user.bump,
    )]
    pub user: Account<'info, User>,
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = owner,
    )]
    pub user_ata: Account<'info, TokenAccount>,
    #[account(mut, address = exchange.vault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(address = exchange.collateral_mint)]
    pub collateral_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handle_withdraw(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
    if amount == 0 {
        return err!(MarkError::ZeroSize);
    }
    let user = &mut ctx.accounts.user;
    if amount > user.free_collateral {
        return err!(MarkError::WithdrawExceedsFree);
    }
    user.free_collateral -= amount;
    transfer_as_exchange(
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.vault.to_account_info(),
        ctx.accounts.user_ata.to_account_info(),
        ctx.accounts.exchange.to_account_info(),
        ctx.accounts.exchange.bump,
        amount,
    )?;
    emit!(WithdrawEvent {
        owner: user.owner,
        amount,
        free_after: user.free_collateral,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct Faucet<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [b"exchange"], bump = exchange.bump)]
    pub exchange: Account<'info, Exchange>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user.bump,
    )]
    pub user: Account<'info, User>,
    #[account(mut, address = exchange.collateral_mint)]
    pub collateral_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = collateral_mint,
        associated_token::authority = owner,
    )]
    pub user_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_faucet(ctx: Context<Faucet>, amount: u64) -> Result<()> {
    let exchange = &ctx.accounts.exchange;
    if exchange.faucet_enabled == 0 {
        return err!(MarkError::FaucetOff);
    }
    if amount == 0 || amount > exchange.faucet_per_tx {
        return err!(MarkError::FaucetCap);
    }
    let user = &mut ctx.accounts.user;
    let next = user
        .faucet_claimed
        .checked_add(amount)
        .ok_or(error!(MarkError::Math))?;
    if next > exchange.faucet_per_user {
        return err!(MarkError::FaucetCap);
    }
    mint_to_exchange(
        &ctx.accounts.token_program,
        &ctx.accounts.collateral_mint,
        &ctx.accounts.user_ata,
        exchange,
        amount,
    )?;
    user.faucet_claimed = next;
    Ok(())
}
