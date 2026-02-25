use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer, Mint};
use crate::state::{Market, TraderState};
use crate::errors::DexError;
use crate::events::DepositEvent;

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct Deposit<'info> {
    #[account(
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        init_if_needed,
        payer = trader,
        space = TraderState::SIZE,
        seeds = [b"trader_state", trader.key().as_ref(), market.key().as_ref()],
        bump
    )]
    pub trader_state: Account<'info, TraderState>,
    
    #[account(mut)]
    pub trader: Signer<'info>,
    
    #[account(mut)]
    pub trader_token_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,
    
    pub mint: Account<'info, Mint>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
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
    
    // Transfer tokens from trader to vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.trader_token_account.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.trader.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    anchor_spl::token::transfer(cpi_ctx, amount)?;
    
    // Update trader state
    let mut trader_state = ctx.accounts.trader_state.as_mut();
    
    if trader_state.trader == Pubkey::default() {
        // Initialize trader state
        trader_state.trader = ctx.accounts.trader.key();
        trader_state.market = market.key();
        trader_state.bump = ctx.bumps.get("trader_state").unwrap().clone();
    }
    
    if is_base {
        trader_state.base_available = trader_state.base_available
            .checked_add(amount)
            .ok_or(DexError::MathOverflow)?;
    } else {
        trader_state.quote_available = trader_state.quote_available
            .checked_add(amount)
            .ok_or(DexError::MathOverflow)?;
    }
    
    emit!(DepositEvent {
        trader: ctx.accounts.trader.key(),
        market: market.key(),
        mint: ctx.accounts.mint.key(),
        amount,
        new_balance: if is_base {
            trader_state.base_available
        } else {
            trader_state.quote_available
        },
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!("Deposit: trader={}, mint={}, amount={}", 
         ctx.accounts.trader.key(), ctx.accounts.mint.key(), amount);
    
    Ok(())
}
