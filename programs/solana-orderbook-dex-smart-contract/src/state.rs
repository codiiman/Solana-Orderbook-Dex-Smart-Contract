use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::orderbook::{Orderbook, OrderbookSide};

/// Global DEX configuration account
/// Stores protocol-wide settings, fee parameters, and authority
#[account]
#[derive(Default)]
pub struct GlobalConfig {
    /// Protocol authority (can update fees, pause markets, etc.)
    pub authority: Pubkey,
    
    /// Protocol fee recipient (treasury)
    pub fee_recipient: Pubkey,
    
    /// Maker fee in basis points (e.g., 2 = 0.02%)
    pub maker_fee_bps: u16,
    
    /// Taker fee in basis points (e.g., 4 = 0.04%)
    pub taker_fee_bps: u16,
    
    /// Whether market creation is permissionless (true) or permissioned (false)
    pub permissionless_markets: bool,
    
    /// Market creation fee (in lamports) if permissioned
    pub market_creation_fee: u64,
    
    /// Bump seed for PDA derivation
    pub bump: u8,
    
    /// Reserved space for future upgrades
    pub _reserved: [u8; 64],
}

impl GlobalConfig {
    pub const SIZE: usize = 8 + // discriminator
        32 + // authority
        32 + // fee_recipient
        2 +  // maker_fee_bps
        2 +  // taker_fee_bps
        1 +  // permissionless_markets
        8 +  // market_creation_fee
        1 +  // bump
        64;  // reserved
}

/// Market account storing spot market configuration and orderbook state
#[account]
pub struct Market {
    /// Market identifier (unique)
    pub market_id: u64,
    
    /// Base asset mint (e.g., SOL)
    pub base_mint: Pubkey,
    
    /// Quote asset mint (e.g., USDC)
    pub quote_mint: Pubkey,
    
    /// Base asset vault (holds base tokens for settlement)
    pub base_vault: Pubkey,
    
    /// Quote asset vault (holds quote tokens for settlement)
    pub quote_vault: Pubkey,
    
    /// Minimum price increment (in quote units, e.g., 100 = $0.01 for USDC quote)
    pub tick_size: u64,
    
    /// Minimum order size (in base units, e.g., 1000000 = 0.001 SOL if 9 decimals)
    pub lot_size: u64,
    
    /// Market authority (can update params, pause market)
    pub authority: Pubkey,
    
    /// Whether market is paused (no new orders allowed)
    pub paused: bool,
    
    /// Current best bid price (0 if no bids)
    pub best_bid: u64,
    
    /// Current best ask price (0 if no asks, or u64::MAX if no asks)
    pub best_ask: u64,
    
    /// Total number of orders in orderbook
    pub order_count: u64,
    
    /// Total volume traded (in quote units)
    pub total_volume: u128,
    
    /// Bump seed for PDA derivation
    pub bump: u8,
    
    /// Reserved space for future extensions (perp, AMM, etc.)
    pub _reserved: [u8; 128],
}

impl Market {
    pub const SIZE: usize = 8 + // discriminator
        8 +  // market_id
        32 + // base_mint
        32 + // quote_mint
        32 + // base_vault
        32 + // quote_vault
        8 +  // tick_size
        8 +  // lot_size
        32 + // authority
        1 +  // paused
        8 +  // best_bid
        8 +  // best_ask
        8 +  // order_count
        16 + // total_volume
        1 +  // bump
        128; // reserved
    
    /// Validate that a price is on a valid tick
    pub fn is_valid_tick(&self, price: u64) -> bool {
        price >= self.tick_size && price % self.tick_size == 0
    }
    
    /// Validate that a size is a valid lot
    pub fn is_valid_lot(&self, size: u64) -> bool {
        size >= self.lot_size && size % self.lot_size == 0
    }
    
    /// Calculate the minimum price increment
    pub fn next_tick_up(&self, price: u64) -> Option<u64> {
        price.checked_add(self.tick_size)
    }
    
    /// Calculate the maximum price decrement
    pub fn next_tick_down(&self, price: u64) -> Option<u64> {
        price.checked_sub(self.tick_size)
    }
}

/// Trader position account storing balances and open orders per market
#[account]
pub struct TraderState {
    /// Trader's wallet address
    pub trader: Pubkey,
    
    /// Market this position is for
    pub market: Pubkey,
    
    /// Available base balance (not locked in orders)
    pub base_available: u64,
    
    /// Available quote balance (not locked in orders)
    pub quote_available: u64,
    
    /// Base balance locked in open orders
    pub base_locked: u64,
    
    /// Quote balance locked in open orders
    pub quote_locked: u64,
    
    /// Number of open orders
    pub open_order_count: u16,
    
    /// Bump seed for PDA derivation
    pub bump: u8,
    
    /// Reserved space
    pub _reserved: [u8; 32],
}

impl TraderState {
    pub const SIZE: usize = 8 + // discriminator
        32 + // trader
        32 + // market
        8 +  // base_available
        8 +  // quote_available
        8 +  // base_locked
        8 +  // quote_locked
        2 +  // open_order_count
        1 +  // bump
        32;  // reserved
    
    /// Get total base balance (available + locked)
    pub fn total_base(&self) -> u64 {
        self.base_available
            .checked_add(self.base_locked)
            .unwrap_or(u64::MAX)
    }
    
    /// Get total quote balance (available + locked)
    pub fn total_quote(&self) -> u64 {
        self.quote_available
            .checked_add(self.quote_locked)
            .unwrap_or(u64::MAX)
    }
    
    /// Lock base tokens for an order
    pub fn lock_base(&mut self, amount: u64) -> Result<()> {
        require!(
            self.base_available >= amount,
            crate::errors::DexError::InsufficientFunds
        );
        self.base_available = self.base_available
            .checked_sub(amount)
            .ok_or(crate::errors::DexError::MathUnderflow)?;
        self.base_locked = self.base_locked
            .checked_add(amount)
            .ok_or(crate::errors::DexError::MathOverflow)?;
        Ok(())
    }
    
    /// Lock quote tokens for an order
    pub fn lock_quote(&mut self, amount: u64) -> Result<()> {
        require!(
            self.quote_available >= amount,
            crate::errors::DexError::InsufficientFunds
        );
        self.quote_available = self.quote_available
            .checked_sub(amount)
            .ok_or(crate::errors::DexError::MathUnderflow)?;
        self.quote_locked = self.quote_locked
            .checked_add(amount)
            .ok_or(crate::errors::DexError::MathOverflow)?;
        Ok(())
    }
    
    /// Unlock base tokens from a cancelled/filled order
    pub fn unlock_base(&mut self, amount: u64) -> Result<()> {
        require!(
            self.base_locked >= amount,
            crate::errors::DexError::InvalidAccountState
        );
        self.base_locked = self.base_locked
            .checked_sub(amount)
            .ok_or(crate::errors::DexError::MathUnderflow)?;
        self.base_available = self.base_available
            .checked_add(amount)
            .ok_or(crate::errors::DexError::MathOverflow)?;
        Ok(())
    }
    
    /// Unlock quote tokens from a cancelled/filled order
    pub fn unlock_quote(&mut self, amount: u64) -> Result<()> {
        require!(
            self.quote_locked >= amount,
            crate::errors::DexError::InvalidAccountState
        );
        self.quote_locked = self.quote_locked
            .checked_sub(amount)
            .ok_or(crate::errors::DexError::MathUnderflow)?;
        self.quote_available = self.quote_available
            .checked_add(amount)
            .ok_or(crate::errors::DexError::MathOverflow)?;
        Ok(())
    }
}

/// Pending fill account storing matched orders awaiting settlement
#[account]
pub struct PendingFill {
    /// Unique fill identifier
    pub fill_id: u128,
    
    /// Market this fill is for
    pub market: Pubkey,
    
    /// Bid order ID
    pub bid_order_id: u128,
    
    /// Ask order ID
    pub ask_order_id: u128,
    
    /// Bid trader
    pub bid_trader: Pubkey,
    
    /// Ask trader
    pub ask_trader: Pubkey,
    
    /// Fill price
    pub price: u64,
    
    /// Fill size (in base units)
    pub size: u64,
    
    /// Quote amount (price * size)
    pub quote_amount: u64,
    
    /// Maker fee (paid by maker)
    pub maker_fee: u64,
    
    /// Taker fee (paid by taker)
    pub taker_fee: u64,
    
    /// Whether this fill has been settled
    pub settled: bool,
    
    /// Timestamp of fill creation
    pub timestamp: i64,
    
    /// Reserved space
    pub _reserved: [u8; 32],
}

impl PendingFill {
    pub const SIZE: usize = 8 + // discriminator
        16 + // fill_id
        32 + // market
        16 + // bid_order_id
        16 + // ask_order_id
        32 + // bid_trader
        32 + // ask_trader
        8 +  // price
        8 +  // size
        8 +  // quote_amount
        8 +  // maker_fee
        8 +  // taker_fee
        1 +  // settled
        8 +  // timestamp
        32;  // reserved
}
