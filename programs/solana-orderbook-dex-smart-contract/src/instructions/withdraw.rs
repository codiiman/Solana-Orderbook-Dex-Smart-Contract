use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer, Mint};
use crate::state::{Market, TraderState};
use crate::errors::DexError;
use crate::events::WithdrawEvent;

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct Withdraw<'info> {
    #[account(
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        seeds = [b"trader_state", trader.key().as_ref(), market.key().as_ref()],
        bump = trader_state.bump,
        constraint = trader_state.trader == trader.key() @ DexError::Unauthorized
    )]
    pub trader_state: Account<'info, TraderState>,
    
    #[account(mut)]
    pub trader: Signer<'info>,
    
    #[account(mut)]
    pub trader_token_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    /// CHECK: Market authority for vault signer
    pub market_authority: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, DexError::InvalidOrderParams);
    
    let market = &ctx.accounts.market;
    
    // Validate mint matches market
    let is_base = ctx.accounts.mint.key() == market.base_mint;
    let is_quote = ctx.accounts.mint.key() == market.quote_mint;
    require!(is_base || is_quote, DexError::InvalidMint);
    
    // Validate vault matches mint
    let expected_vault = if is_base {
        market.base_vault
    } else {
        market.quote_vault
    };
    require!(
        ctx.accounts.vault.key() == expected_vault,
        DexError::InvalidMint
    );
    
    // Check available balance
    let trader_state = &ctx.accounts.trader_state;
    let available = if is_base {
        trader_state.base_available
    } else {
        trader_state.quote_available
    };
    
    require!(available >= amount, DexError::InsufficientFunds);
    
    // Update trader state
    let mut trader_state_mut = ctx.accounts.trader_state.as_mut();
    
    if is_base {
        trader_state_mut.base_available = trader_state_mut.base_available
            .checked_sub(amount)
            .ok_or(DexError::MathUnderflow)?;
    } else {
        trader_state_mut.quote_available = trader_state_mut.quote_available
            .checked_sub(amount)
            .ok_or(DexError::MathUnderflow)?;
    }
    
    // Transfer tokens from vault to trader
    let seeds = &[
        b"market",
        &market.market_id.to_le_bytes(),
        &[market.bump],
    ];
    let signer = &[&seeds[..]];
    
    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.trader_token_account.to_account_info(),
        authority: ctx.accounts.market_authority.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    anchor_spl::token::transfer(cpi_ctx, amount)?;
    
    emit!(WithdrawEvent {
        trader: ctx.accounts.trader.key(),
        market: market.key(),
        mint: ctx.accounts.mint.key(),
        amount,
        new_balance: if is_base {
            trader_state_mut.base_available
        } else {
            trader_state_mut.quote_available
        },
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!("Withdraw: trader={}, mint={}, amount={}", 
         ctx.accounts.trader.key(), ctx.accounts.mint.key(), amount);
    
    Ok(())
}
