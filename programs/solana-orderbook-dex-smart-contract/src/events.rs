use anchor_lang::prelude::*;

/// Event emitted when a new market is created
#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub tick_size: u64,
    pub lot_size: u64,
    pub timestamp: i64,
}

/// Event emitted when an order is placed
#[event]
pub struct OrderPlaced {
    pub market: Pubkey,
    pub trader: Pubkey,
    pub order_id: u128,
    pub side: u8, // 0 = bid, 1 = ask
    pub price: u64,
    pub size: u64,
    pub time_in_force: u8,
    pub timestamp: i64,
}

/// Event emitted when an order is cancelled
#[event]
pub struct OrderCancelled {
    pub market: Pubkey,
    pub trader: Pubkey,
    pub order_id: u128,
    pub remaining_size: u64,
    pub timestamp: i64,
}

/// Event emitted when orders are matched
#[event]
pub struct OrderMatched {
    pub market: Pubkey,
    pub bid_order_id: u128,
    pub ask_order_id: u128,
    pub price: u64,
    pub size: u64,
    pub bid_trader: Pubkey,
    pub ask_trader: Pubkey,
    pub fill_id: u128,
    pub timestamp: i64,
}

/// Event emitted when a fill is settled
#[event]
pub struct FillSettled {
    pub market: Pubkey,
    pub fill_id: u128,
    pub bid_trader: Pubkey,
    pub ask_trader: Pubkey,
    pub base_amount: u64,
    pub quote_amount: u64,
    pub maker_fee: u64,
    pub taker_fee: u64,
    pub timestamp: i64,
}

/// Event emitted when a trader deposits funds
#[event]
pub struct DepositEvent {
    pub trader: Pubkey,
    pub market: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

/// Event emitted when a trader withdraws funds
#[event]
pub struct WithdrawEvent {
    pub trader: Pubkey,
    pub market: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

/// Event emitted when market parameters are updated
#[event]
pub struct MarketParamsUpdated {
    pub market: Pubkey,
    pub tick_size: Option<u64>,
    pub lot_size: Option<u64>,
    pub timestamp: i64,
}

/// Event emitted when a market is paused/unpaused
#[event]
pub struct MarketPauseUpdated {
    pub market: Pubkey,
    pub paused: bool,
    pub timestamp: i64,
}
