use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{GlobalConfig, Market};
use crate::errors::DexError;
use crate::events::MarketCreated;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateMarketParams {
    pub market_id: u64,
    pub tick_size: u64,
    pub lot_size: u64,
}

#[derive(Accounts)]
#[instruction(params: CreateMarketParams)]
pub struct CreateMarket<'info> {
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(
        init,
        payer = authority,
        space = Market::SIZE,
        seeds = [b"market", params.market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,
    
    pub base_mint: Account<'info, Mint>,
    pub quote_mint: Account<'info, Mint>,
    
    #[account(
        init,
        payer = authority,
        token::mint = base_mint,
        token::authority = market,
        seeds = [b"base_vault", market.key().as_ref()],
        bump
    )]
    pub base_vault: Account<'info, TokenAccount>,
    
    #[account(
        init,
        payer = authority,
        token::mint = quote_mint,
        token::authority = market,
        seeds = [b"quote_vault", market.key().as_ref()],
        bump
    )]
    pub quote_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<CreateMarket>, params: CreateMarketParams) -> Result<()> {
    let global_config = &ctx.accounts.global_config;
    
    // Check if market creation is allowed
    if !global_config.permissionless_markets {
        require!(
            ctx.accounts.authority.key() == global_config.authority,
            DexError::MarketCreationNotAllowed
        );
    }
    
    // Validate parameters
    require!(params.tick_size > 0, DexError::InvalidMarketParams);
    require!(params.lot_size > 0, DexError::InvalidMarketParams);
    require!(
        params.tick_size <= 1_000_000_000, // Reasonable upper bound
        DexError::InvalidMarketParams
    );
    require!(
        params.lot_size <= 1_000_000_000_000, // Reasonable upper bound
        DexError::InvalidMarketParams
    );
    
    let market = &mut ctx.accounts.market;
    market.market_id = params.market_id;
    market.base_mint = ctx.accounts.base_mint.key();
    market.quote_mint = ctx.accounts.quote_mint.key();
    market.base_vault = ctx.accounts.base_vault.key();
    market.quote_vault = ctx.accounts.quote_vault.key();
    market.tick_size = params.tick_size;
    market.lot_size = params.lot_size;
    market.authority = ctx.accounts.authority.key();
    market.paused = false;
    market.best_bid = 0;
    market.best_ask = 0;
    market.order_count = 0;
    market.total_volume = 0;
    market.bump = ctx.bumps.get("market").unwrap().clone();
    
    emit!(MarketCreated {
        market: market.key(),
        base_mint: market.base_mint,
        quote_mint: market.quote_mint,
        tick_size: market.tick_size,
        lot_size: market.lot_size,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!("Market created: id={}, base={}, quote={}", 
         params.market_id, market.base_mint, market.quote_mint);
    
    Ok(())
}
