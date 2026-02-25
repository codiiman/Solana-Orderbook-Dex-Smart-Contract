use anchor_lang::prelude::*;
use crate::state::Market;
use crate::errors::DexError;
use crate::events::MarketPauseUpdated;

#[derive(Accounts)]
#[instruction(paused: bool)]
pub struct PauseMarket<'info> {
    #[account(
        mut,
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump,
        constraint = authority.key() == market.authority || 
                     authority.key() == global_config.authority @ DexError::Unauthorized
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, crate::state::GlobalConfig>,
    
    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<PauseMarket>, paused: bool) -> Result<()> {
    let market = &mut ctx.accounts.market;
    market.paused = paused;
    
    emit!(MarketPauseUpdated {
        market: market.key(),
        paused,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!("Market {}: market={}", if paused { "paused" } else { "unpaused" }, market.key());
    
    Ok(())
}
