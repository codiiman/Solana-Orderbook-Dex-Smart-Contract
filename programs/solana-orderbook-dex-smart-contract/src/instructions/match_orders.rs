use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::{Market, Orderbook, PendingFill};
use crate::orderbook::Order;
use crate::errors::DexError;
use crate::events::OrderMatched;
use crate::state::GlobalConfig;

#[derive(Accounts)]
pub struct MatchOrders<'info> {
    #[account(
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    
    /// CHECK: Orderbook account
    #[account(mut)]
    pub orderbook: UncheckedAccount<'info>,
    
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// CHECK: Pending fills account (can be any account, we'll create fills)
    #[account(mut)]
    pub pending_fills: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<MatchOrders>, max_iterations: u8) -> Result<()> {
    let market = &ctx.accounts.market;
    
    require!(!market.paused, DexError::MarketPaused);
    
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
    
    let global_config = &ctx.accounts.global_config;
    let mut iterations = 0u8;
    
    // Matching loop
    while iterations < max_iterations {
        // Find best bid and best ask
        let best_bid_opt = orderbook.find_best_bid(&orderbook_data);
        let best_ask_opt = orderbook.find_best_ask(&orderbook_data);
        
        let (bid_slot, mut bid_order) = match best_bid_opt {
            Some((slot, order)) => (slot, order),
            None => break, // No bids
        };
        
        let (ask_slot, mut ask_order) = match best_ask_opt {
            Some((slot, order)) => (slot, order),
            None => break, // No asks
        };
        
        // Check if orders can match
        if !bid_order.can_match(&ask_order) {
            break; // No more matches possible
        }
        
        // Calculate match price (use bid price for simplicity, could use mid-price)
        let match_price = bid_order.price.min(ask_order.price);
        
        // Calculate fill size (minimum of remaining sizes)
        let fill_size = bid_order.remaining_size.min(ask_order.remaining_size);
        
        // Fill orders
        bid_order.fill(fill_size)?;
        ask_order.fill(fill_size)?;
        
        // Calculate fees
        let quote_amount = match_price
            .checked_mul(fill_size)
            .and_then(|v| v.checked_div(market.lot_size))
            .ok_or(DexError::MathOverflow)?;
        
        // Determine maker/taker (older order is maker)
        let is_bid_maker = bid_order.timestamp <= ask_order.timestamp;
        let maker_fee = if is_bid_maker {
            quote_amount
                .checked_mul(global_config.maker_fee_bps as u64)
                .and_then(|v| v.checked_div(10000))
                .unwrap_or(0)
        } else {
            quote_amount
                .checked_mul(global_config.taker_fee_bps as u64)
                .and_then(|v| v.checked_div(10000))
                .unwrap_or(0)
        };
        
        let taker_fee = if is_bid_maker {
            quote_amount
                .checked_mul(global_config.taker_fee_bps as u64)
                .and_then(|v| v.checked_div(10000))
                .unwrap_or(0)
        } else {
            quote_amount
                .checked_mul(global_config.maker_fee_bps as u64)
                .and_then(|v| v.checked_div(10000))
                .unwrap_or(0)
        };
        
        // Generate fill ID
        let clock = Clock::get()?;
        let fill_id = u128::from(clock.unix_timestamp)
            .checked_mul(1_000_000)
            .and_then(|v| v.checked_add(u128::from(clock.slot)))
            .and_then(|v| v.checked_add(u128::from(iterations)))
            .ok_or(DexError::MathOverflow)?;
        
        // Update orders in orderbook
        orderbook.set_order(&mut orderbook_data, bid_slot, &bid_order)?;
        orderbook.set_order(&mut orderbook_data, ask_slot, &ask_order)?;
        
        // Remove filled orders
        if bid_order.is_filled() {
            orderbook.free_slot(&mut orderbook_data, bid_slot)?;
            orderbook.order_count = orderbook.order_count
                .checked_sub(1)
                .ok_or(DexError::MathUnderflow)?;
        }
        
        if ask_order.is_filled() {
            orderbook.free_slot(&mut orderbook_data, ask_slot)?;
            orderbook.order_count = orderbook.order_count
                .checked_sub(1)
                .ok_or(DexError::MathUnderflow)?;
        }
        
        // Update best prices
        orderbook.update_best_prices(&orderbook_data);
        
        // Emit match event
        emit!(OrderMatched {
            market: market.key(),
            bid_order_id: bid_order.order_id,
            ask_order_id: ask_order.order_id,
            price: match_price,
            size: fill_size,
            bid_trader: bid_order.trader,
            ask_trader: ask_order.trader,
            fill_id,
            timestamp: clock.unix_timestamp,
        });
        
        msg!("Orders matched: bid={}, ask={}, price={}, size={}", 
             bid_order.order_id, ask_order.order_id, match_price, fill_size);
        
        iterations = iterations.checked_add(1).ok_or(DexError::MathOverflow)?;
    }
    
    // Save orderbook
    orderbook.try_serialize(&mut &mut orderbook_data[..Orderbook::HEADER_SIZE])?;
    
    // Update market
    let mut market_mut = ctx.accounts.market.as_mut();
    market_mut.best_bid = orderbook.best_bid;
    market_mut.best_ask = orderbook.best_ask;
    market_mut.order_count = orderbook.order_count;
    
    Ok(())
}
