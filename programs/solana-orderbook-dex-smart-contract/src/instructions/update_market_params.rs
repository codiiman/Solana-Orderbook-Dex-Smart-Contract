use anchor_lang::prelude::*;
use crate::state::Market;
use crate::errors::DexError;
use crate::events::MarketParamsUpdated;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateMarketParamsParams {
    pub tick_size: Option<u64>,
    pub lot_size: Option<u64>,
}

#[derive(Accounts)]
#[instruction(params: UpdateMarketParamsParams)]
pub struct UpdateMarketParams<'info> {
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

pub fn handler(ctx: Context<UpdateMarketParams>, params: UpdateMarketParamsParams) -> Result<()> {
    let market = &mut ctx.accounts.market;
    
    if let Some(tick_size) = params.tick_size {
        require!(tick_size > 0, DexError::InvalidMarketParams);
        require!(
            tick_size <= 1_000_000_000,
            DexError::InvalidMarketParams
        );
        market.tick_size = tick_size;
    }
    
    if let Some(lot_size) = params.lot_size {
        require!(lot_size > 0, DexError::InvalidMarketParams);
        require!(
            lot_size <= 1_000_000_000_000,
            DexError::InvalidMarketParams
        );
        market.lot_size = lot_size;
    }
    
    emit!(MarketParamsUpdated {
        market: market.key(),
        tick_size: params.tick_size,
        lot_size: params.lot_size,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!("Market params updated: market={}", market.key());
    
    Ok(())
}
