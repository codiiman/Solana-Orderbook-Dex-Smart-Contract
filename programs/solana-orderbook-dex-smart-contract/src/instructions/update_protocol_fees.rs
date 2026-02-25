use anchor_lang::prelude::*;
use crate::state::GlobalConfig;
use crate::errors::DexError;

#[derive(Accounts)]
pub struct UpdateProtocolFees<'info> {
    #[account(
        mut,
        seeds = [b"global_config"],
        bump = global_config.bump,
        constraint = authority.key() == global_config.authority @ DexError::Unauthorized
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    pub authority: Signer<'info>,
}

pub fn handler(
    ctx: Context<UpdateProtocolFees>,
    maker_fee_bps: Option<u16>,
    taker_fee_bps: Option<u16>,
) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    
    if let Some(fee) = maker_fee_bps {
        require!(fee <= 1000, DexError::InvalidFeeCalculation); // Max 10%
        global_config.maker_fee_bps = fee;
    }
    
    if let Some(fee) = taker_fee_bps {
        require!(fee <= 1000, DexError::InvalidFeeCalculation); // Max 10%
        global_config.taker_fee_bps = fee;
    }
    
    msg!("Protocol fees updated: maker={}bps, taker={}bps", 
         global_config.maker_fee_bps, global_config.taker_fee_bps);
    
    Ok(())
}
