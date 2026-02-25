use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::{Market, TraderState, Orderbook};
use crate::orderbook::Order;
use crate::errors::DexError;
use crate::events::OrderCancelled;

#[derive(Accounts)]
#[instruction(order_id: u128)]
pub struct CancelOrder<'info> {
    #[account(
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    
    /// CHECK: Orderbook account
    #[account(mut)]
    pub orderbook: UncheckedAccount<'info>,
    
    #[account(
        seeds = [b"trader_state", trader.key().as_ref(), market.key().as_ref()],
        bump = trader_state.bump,
        constraint = trader_state.trader == trader.key() @ DexError::Unauthorized
    )]
    pub trader_state: Account<'info, TraderState>,
    
    #[account(mut)]
    pub trader: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CancelOrder>, order_id: u128) -> Result<()> {
    let market = &ctx.accounts.market;
    
    // Load orderbook
    let orderbook_account_info = &ctx.accounts.orderbook;
    require!(
        orderbook_account_info.data_len() >= Orderbook::HEADER_SIZE,
        DexError::InvalidOrderbookState
    );
    
    let mut orderbook_data = orderbook_account_info.try_borrow_mut_data()?;
    let mut orderbook = Account::<Orderbook>::try_deserialize(
        &mut &orderbook_account_info.data.borrow()[..Orderbook::HEADER_SIZE]
    )?;
    
    // Find order in orderbook
    let mut found_slot = None;
    let mut found_order = None;
    
    for i in 0..Orderbook::MAX_ORDERS {
        if let Some(order) = orderbook.get_order(&orderbook_data, i as u64) {
            if order.order_id == order_id && order.trader == ctx.accounts.trader.key() {
                found_slot = Some(i as u64);
                found_order = Some(order);
                break;
            }
        }
    }
    
    let (slot, order) = found_slot
        .zip(found_order)
        .ok_or(DexError::OrderNotFound)?;
    
    require!(
        !order.is_filled(),
        DexError::OrderAlreadyFilled
    );
    
    // Unlock tokens
    let mut trader_state = ctx.accounts.trader_state.clone();
    
    if order.is_bid() {
        // Unlock quote tokens
        let quote_locked = order.price
            .checked_mul(order.remaining_size)
            .and_then(|v| v.checked_div(market.lot_size))
            .ok_or(DexError::MathOverflow)?;
        
        trader_state.unlock_quote(quote_locked)?;
    } else {
        // Unlock base tokens
        trader_state.unlock_base(order.remaining_size)?;
    }
    
    // Remove order from orderbook
    orderbook.free_slot(&mut orderbook_data, slot)?;
    orderbook.order_count = orderbook.order_count
        .checked_sub(1)
        .ok_or(DexError::MathUnderflow)?;
    orderbook.update_best_prices(&orderbook_data);
    
    // Save orderbook
    orderbook.try_serialize(&mut &mut orderbook_data[..Orderbook::HEADER_SIZE])?;
    
    // Update trader state
    ctx.accounts.trader_state.base_available = trader_state.base_available;
    ctx.accounts.trader_state.quote_available = trader_state.quote_available;
    ctx.accounts.trader_state.base_locked = trader_state.base_locked;
    ctx.accounts.trader_state.quote_locked = trader_state.quote_locked;
    ctx.accounts.trader_state.open_order_count = ctx.accounts.trader_state.open_order_count
        .checked_sub(1)
        .ok_or(DexError::MathUnderflow)? as u16;
    
    // Update market
    let mut market_mut = ctx.accounts.market.as_mut();
    market_mut.best_bid = orderbook.best_bid;
    market_mut.best_ask = orderbook.best_ask;
    market_mut.order_count = orderbook.order_count;
    
    emit!(OrderCancelled {
        market: market.key(),
        trader: ctx.accounts.trader.key(),
        order_id,
        remaining_size: order.remaining_size,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!("Order cancelled: id={}, remaining_size={}", order_id, order.remaining_size);
    
    Ok(())
}
