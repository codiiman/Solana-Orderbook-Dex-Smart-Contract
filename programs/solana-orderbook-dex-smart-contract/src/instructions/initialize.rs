use anchor_lang::prelude::*;
use crate::state::GlobalConfig;
use crate::errors::DexError;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeParams {
    pub maker_fee_bps: u16,
    pub taker_fee_bps: u16,
    pub permissionless_markets: bool,
    pub market_creation_fee: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = GlobalConfig::SIZE,
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: Fee recipient can be any account
    pub fee_recipient: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
    require!(
        params.maker_fee_bps <= 1000, // Max 10%
        DexError::InvalidFeeCalculation
    );
    require!(
        params.taker_fee_bps <= 1000, // Max 10%
        DexError::InvalidFeeCalculation
    );
    
    let global_config = &mut ctx.accounts.global_config;
    global_config.authority = ctx.accounts.authority.key();
    global_config.fee_recipient = ctx.accounts.fee_recipient.key();
    global_config.maker_fee_bps = params.maker_fee_bps;
    global_config.taker_fee_bps = params.taker_fee_bps;
    global_config.permissionless_markets = params.permissionless_markets;
    global_config.market_creation_fee = params.market_creation_fee;
    global_config.bump = ctx.bumps.get("global_config").unwrap().clone();
    
    msg!("Global config initialized: maker_fee={}bps, taker_fee={}bps", 
         params.maker_fee_bps, params.taker_fee_bps);
    
    Ok(())
}
