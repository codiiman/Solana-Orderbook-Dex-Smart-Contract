use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::{Market, TraderState, Orderbook};
use crate::orderbook::{Order, Side, TimeInForce};
use crate::errors::DexError;
use crate::events::OrderPlaced;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct PlaceOrderParams {
    pub side: u8, // 0 = bid, 1 = ask
    pub price: u64,
    pub size: u64,
    pub time_in_force: u8, // 0 = GTC, 1 = IOC, 2 = FOK, 3 = PostOnly
}

#[derive(Accounts)]
#[instruction(params: PlaceOrderParams)]
pub struct PlaceOrder<'info> {
    #[account(
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    
    /// CHECK: Orderbook account (we'll validate it's initialized)
    #[account(mut)]
    pub orderbook: UncheckedAccount<'info>,
    
    #[account(
        seeds = [b"trader_state", trader.key().as_ref(), market.key().as_ref()],
        bump = trader_state.bump
    )]
    pub trader_state: Account<'info, TraderState>,
    
    #[account(mut)]
    pub trader: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<PlaceOrder>, params: PlaceOrderParams) -> Result<()> {
    let market = &ctx.accounts.market;
    
    // Check if market is paused
    require!(!market.paused, DexError::MarketPaused);
    
    // Validate side
    let side = Side::from_u8(params.side)
        .ok_or(DexError::InvalidOrderParams)?;
    
    // Validate time-in-force
    let tif = TimeInForce::from_u8(params.time_in_force)
        .ok_or(DexError::InvalidTimeInForce)?;
    
    // Validate price is on tick
    require!(market.is_valid_tick(params.price), DexError::PriceNotOnTick);
    
    // Validate size is valid lot
    require!(market.is_valid_lot(params.size), DexError::OrderSizeTooSmall);
    
    // Validate size bounds
    require!(params.size >= market.lot_size, DexError::OrderSizeTooSmall);
    require!(
        params.size <= 1_000_000_000_000, // Reasonable upper bound
        DexError::OrderSizeTooLarge
    );
    
    // Load orderbook
    let orderbook_account_info = &ctx.accounts.orderbook;
    require!(
        orderbook_account_info.data_len() >= Orderbook::HEADER_SIZE,
        DexError::InvalidOrderbookState
    );
    
    let mut orderbook_data = orderbook_account_info.try_borrow_mut_data()?;
    let orderbook = Account::<Orderbook>::try_deserialize(
        &mut &orderbook_account_info.data.borrow()[..Orderbook::HEADER_SIZE]
    )?;
    
    // Check if order would cross spread (for PostOnly)
    if tif == TimeInForce::PostOnly {
        if side == Side::Bid && orderbook.best_ask > 0 && params.price >= orderbook.best_ask {
            return Err(DexError::PostOnlyWouldCross.into());
        }
        if side == Side::Ask && orderbook.best_bid > 0 && params.price <= orderbook.best_bid {
            return Err(DexError::PostOnlyWouldCross.into());
        }
    }
    
    // Calculate required tokens and lock them
    let mut trader_state = ctx.accounts.trader_state.clone();
    
    if side == Side::Bid {
        // Bids need quote tokens: price * size
        let quote_required = params.price
            .checked_mul(params.size)
            .and_then(|v| v.checked_div(market.lot_size))
            .ok_or(DexError::MathOverflow)?;
        
        trader_state.lock_quote(quote_required)?;
    } else {
        // Asks need base tokens: size
        trader_state.lock_base(params.size)?;
    }
    
    // Generate order ID (in production, use a more sophisticated method)
    let clock = Clock::get()?;
    let order_id = u128::from(clock.unix_timestamp)
        .checked_mul(1_000_000)
        .and_then(|v| v.checked_add(u128::from(clock.slot)))
        .ok_or(DexError::MathOverflow)?;
    
    // Create order
    let order = Order::new(
        order_id,
        ctx.accounts.trader.key(),
        side,
        params.price,
        params.size,
        tif,
        clock.unix_timestamp,
    );
    
    // Allocate slot in orderbook
    let mut orderbook_mut = Account::<Orderbook>::try_deserialize(
        &mut &orderbook_account_info.data.borrow()[..Orderbook::HEADER_SIZE]
    )?;
    
    let slot = orderbook_mut.allocate_slot(&mut orderbook_data)?;
    orderbook_mut.set_order(&mut orderbook_data, slot, &order)?;
    
    // Update orderbook metadata
    orderbook_mut.order_count = orderbook_mut.order_count
        .checked_add(1)
        .ok_or(DexError::MathOverflow)?;
    orderbook_mut.update_best_prices(&orderbook_data);
    orderbook_mut.market = market.key();
    
    // Save orderbook
    orderbook_mut.try_serialize(&mut &mut orderbook_data[..Orderbook::HEADER_SIZE])?;
    
    // Update trader state
    ctx.accounts.trader_state.base_available = trader_state.base_available;
    ctx.accounts.trader_state.quote_available = trader_state.quote_available;
    ctx.accounts.trader_state.base_locked = trader_state.base_locked;
    ctx.accounts.trader_state.quote_locked = trader_state.quote_locked;
    ctx.accounts.trader_state.open_order_count = ctx.accounts.trader_state.open_order_count
        .checked_add(1)
        .ok_or(DexError::MathOverflow)? as u16;
    
    // Update market
    let mut market_mut = ctx.accounts.market.as_mut();
    market_mut.best_bid = orderbook_mut.best_bid;
    market_mut.best_ask = orderbook_mut.best_ask;
    market_mut.order_count = orderbook_mut.order_count;
    
    emit!(OrderPlaced {
        market: market.key(),
        trader: ctx.accounts.trader.key(),
        order_id,
        side: params.side,
        price: params.price,
        size: params.size,
        time_in_force: params.time_in_force,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Order placed: id={}, side={:?}, price={}, size={}", 
         order_id, side, params.price, params.size);
    
    // If IOC or FOK, try to match immediately
    if tif == TimeInForce::IOC || tif == TimeInForce::FOK {
        // In a full implementation, we'd call match_orders here
        // For now, we'll let the match_orders instruction handle it
    }
    
    Ok(())
}
