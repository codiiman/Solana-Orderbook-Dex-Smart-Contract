use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

/// Order side: Bid (buy) or Ask (sell)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Side {
    Bid = 0,
    Ask = 1,
}

impl Side {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Side::Bid),
            1 => Some(Side::Ask),
            _ => None,
        }
    }
}

/// Time-in-force options for orders
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TimeInForce {
    /// Good-till-cancelled (default)
    GTC = 0,
    /// Immediate-or-cancel (fill immediately or cancel)
    IOC = 1,
    /// Fill-or-kill (fill completely or cancel)
    FOK = 2,
    /// Post-only (maker only, reject if would cross)
    PostOnly = 3,
}

impl TimeInForce {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(TimeInForce::GTC),
            1 => Some(TimeInForce::IOC),
            2 => Some(TimeInForce::FOK),
            3 => Some(TimeInForce::PostOnly),
            _ => None,
        }
    }
}

/// Order structure stored in the orderbook
/// Uses a slab-based data structure for efficient insertion/deletion
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Order {
    /// Unique order identifier (128-bit for collision resistance)
    pub order_id: u128,
    
    /// Trader's public key
    pub trader: Pubkey,
    
    /// Order side (0 = bid, 1 = ask)
    pub side: u8,
    
    /// Limit price (in quote units, must be on tick)
    pub price: u64,
    
    /// Order size (in base units, must be multiple of lot_size)
    pub size: u64,
    
    /// Remaining size (decreases as order is filled)
    pub remaining_size: u64,
    
    /// Time-in-force
    pub time_in_force: u8,
    
    /// Timestamp when order was placed
    pub timestamp: i64,
    
    /// Next order at same price (linked list for price level)
    pub next_at_price: u64,
    
    /// Previous order at same price
    pub prev_at_price: u64,
    
    /// Next order in price-sorted order (for orderbook traversal)
    pub next_in_book: u64,
    
    /// Previous order in price-sorted order
    pub prev_in_book: u64,
}

unsafe impl Pod for Order {}
unsafe impl Zeroable for Order {}

impl Order {
    pub const SIZE: usize = 16 + // order_id
        32 + // trader
        1 +  // side
        8 +  // price
        8 +  // size
        8 +  // remaining_size
        1 +  // time_in_force
        8 +  // timestamp
        8 +  // next_at_price
        8 +  // prev_at_price
        8 +  // next_in_book
        8;   // prev_in_book
    
    /// Create a new order
    pub fn new(
        order_id: u128,
        trader: Pubkey,
        side: Side,
        price: u64,
        size: u64,
        time_in_force: TimeInForce,
        timestamp: i64,
    ) -> Self {
        Self {
            order_id,
            trader,
            side: side as u8,
            price,
            size,
            remaining_size: size,
            time_in_force: time_in_force as u8,
            timestamp,
            next_at_price: 0,
            prev_at_price: 0,
            next_in_book: 0,
            prev_in_book: 0,
        }
    }
    
    /// Check if order is a bid
    pub fn is_bid(&self) -> bool {
        self.side == Side::Bid as u8
    }
    
    /// Check if order is an ask
    pub fn is_ask(&self) -> bool {
        self.side == Side::Ask as u8
    }
    
    /// Check if order can match with another order
    pub fn can_match(&self, other: &Order) -> bool {
        if self.trader == other.trader {
            return false; // Self-trade prevention
        }
        
        match (self.is_bid(), other.is_bid()) {
            (true, false) => self.price >= other.price, // Bid can match if price >= ask
            (false, true) => self.price <= other.price, // Ask can match if price <= bid
            _ => false, // Same side cannot match
        }
    }
    
    /// Fill this order by a given size
    pub fn fill(&mut self, fill_size: u64) -> Result<()> {
        require!(
            fill_size <= self.remaining_size,
            crate::errors::DexError::InvalidOrderParams
        );
        self.remaining_size = self.remaining_size
            .checked_sub(fill_size)
            .ok_or(crate::errors::DexError::MathUnderflow)?;
        Ok(())
    }
    
    /// Check if order is fully filled
    pub fn is_filled(&self) -> bool {
        self.remaining_size == 0
    }
}

/// Orderbook data structure
/// Uses a slab allocator pattern stored in account data
/// 
/// Structure:
/// - Header: metadata (best_bid, best_ask, order_count, free_list_head)
/// - Slab: array of orders indexed by slot number
/// - Price levels: linked lists of orders at each price point
/// 
/// Matching algorithm:
/// 1. For bids: highest price first (price-time priority)
/// 2. For asks: lowest price first (price-time priority)
/// 3. Within same price: FIFO (first-in-first-out)
#[account]
pub struct Orderbook {
    /// Market this orderbook belongs to
    pub market: Pubkey,
    
    /// Best bid price (0 if empty)
    pub best_bid: u64,
    
    /// Best ask price (u64::MAX if empty)
    pub best_ask: u64,
    
    /// Number of active orders
    pub order_count: u64,
    
    /// Head of free list (for slab allocation)
    pub free_list_head: u64,
    
    /// Reserved space for future extensions
    pub _reserved: [u8; 64],
    
    /// Order slab data follows (stored as raw bytes)
    /// Each order is 128 bytes, max ~5000 orders per orderbook
    /// (limited by account size constraints)
}

impl Orderbook {
    pub const HEADER_SIZE: usize = 8 + // discriminator
        32 + // market
        8 +  // best_bid
        8 +  // best_ask
        8 +  // order_count
        8 +  // free_list_head
        64;  // reserved
    
    pub const MAX_ORDERS: usize = 1000; // Conservative limit for account size
    pub const ORDER_SIZE: usize = Order::SIZE;
    pub const MAX_SIZE: usize = Self::HEADER_SIZE + (Self::MAX_ORDERS * Self::ORDER_SIZE);
    
    /// Get order at a specific slot index
    /// Returns None if slot is free or invalid
    pub fn get_order(&self, data: &[u8], slot: u64) -> Option<Order> {
        if slot as usize >= Self::MAX_ORDERS {
            return None;
        }
        
        let offset = Self::HEADER_SIZE + (slot as usize * Self::ORDER_SIZE);
        if offset + Self::ORDER_SIZE > data.len() {
            return None;
        }
        
        let order_bytes = &data[offset..offset + Self::ORDER_SIZE];
        if order_bytes.iter().all(|&b| b == 0) {
            return None; // Free slot
        }
        
        bytemuck::try_from_bytes::<Order>(order_bytes).ok().copied()
    }
    
    /// Write order to a specific slot
    pub fn set_order(&mut self, data: &mut [u8], slot: u64, order: &Order) -> Result<()> {
        require!(
            slot as usize < Self::MAX_ORDERS,
            crate::errors::DexError::OrderbookFull
        );
        
        let offset = Self::HEADER_SIZE + (slot as usize * Self::ORDER_SIZE);
        require!(
            offset + Self::ORDER_SIZE <= data.len(),
            crate::errors::DexError::OrderbookFull
        );
        
        let order_bytes = bytemuck::bytes_of(order);
        data[offset..offset + Self::ORDER_SIZE].copy_from_slice(order_bytes);
        Ok(())
    }
    
    /// Allocate a new slot for an order
    pub fn allocate_slot(&mut self, data: &mut [u8]) -> Result<u64> {
        // Try free list first
        if self.free_list_head != 0 && self.free_list_head < Self::MAX_ORDERS as u64 {
            let slot = self.free_list_head;
            // Read next free slot from order's next_at_price field (repurposed for free list)
            let offset = Self::HEADER_SIZE + (slot as usize * Self::ORDER_SIZE);
            if offset + 8 <= data.len() {
                let next_free = u64::from_le_bytes(
                    data[offset..offset + 8].try_into().unwrap_or([0; 8])
                );
                self.free_list_head = next_free;
            } else {
                self.free_list_head = 0;
            }
            return Ok(slot);
        }
        
        // Allocate new slot
        require!(
            self.order_count < Self::MAX_ORDERS as u64,
            crate::errors::DexError::OrderbookFull
        );
        
        // Find first free slot by scanning
        for i in 0..Self::MAX_ORDERS {
            let offset = Self::HEADER_SIZE + (i * Self::ORDER_SIZE);
            if offset + Self::ORDER_SIZE <= data.len() {
                if data[offset..offset + Self::ORDER_SIZE].iter().all(|&b| b == 0) {
                    return Ok(i as u64);
                }
            }
        }
        
        Err(crate::errors::DexError::OrderbookFull.into())
    }
    
    /// Free a slot (add to free list)
    pub fn free_slot(&mut self, data: &mut [u8], slot: u64) -> Result<()> {
        require!(
            slot as usize < Self::MAX_ORDERS,
            crate::errors::DexError::InvalidOrderbookState
        );
        
        let offset = Self::HEADER_SIZE + (slot as usize * Self::ORDER_SIZE);
        require!(
            offset + Self::ORDER_SIZE <= data.len(),
            crate::errors::DexError::InvalidOrderbookState
        );
        
        // Clear the slot
        data[offset..offset + Self::ORDER_SIZE].fill(0);
        
        // Add to free list
        if self.free_list_head != 0 {
            // Write current free_list_head to slot's next_at_price
            data[offset..offset + 8].copy_from_slice(&self.free_list_head.to_le_bytes());
        }
        self.free_list_head = slot;
        
        Ok(())
    }
    
    /// Find best matching order for a given order
    /// Returns (slot, order) if match found
    pub fn find_best_match(
        &self,
        data: &[u8],
        order: &Order,
    ) -> Option<(u64, Order)> {
        if order.is_bid() {
            // For bids, find best ask (lowest price)
            self.find_best_ask(data)
        } else {
            // For asks, find best bid (highest price)
            self.find_best_bid(data)
        }
    }
    
    /// Find best bid (highest price)
    fn find_best_bid(&self, data: &[u8]) -> Option<(u64, Order)> {
        if self.best_bid == 0 {
            return None;
        }
        
        // Start from best_bid price level and find first order
        // In a full implementation, we'd maintain price level pointers
        // For now, we scan (inefficient but functional)
        let mut best_price = 0u64;
        let mut best_slot = None;
        let mut best_order = None;
        
        for i in 0..Self::MAX_ORDERS {
            if let Some(order) = self.get_order(data, i as u64) {
                if order.is_bid() && order.remaining_size > 0 {
                    if order.price > best_price {
                        best_price = order.price;
                        best_slot = Some(i as u64);
                        best_order = Some(order);
                    }
                }
            }
        }
        
        best_slot.zip(best_order)
    }
    
    /// Find best ask (lowest price)
    fn find_best_ask(&self, data: &[u8]) -> Option<(u64, Order)> {
        if self.best_ask == u64::MAX {
            return None;
        }
        
        let mut best_price = u64::MAX;
        let mut best_slot = None;
        let mut best_order = None;
        
        for i in 0..Self::MAX_ORDERS {
            if let Some(order) = self.get_order(data, i as u64) {
                if order.is_ask() && order.remaining_size > 0 {
                    if order.price < best_price {
                        best_price = order.price;
                        best_slot = Some(i as u64);
                        best_order = Some(order);
                    }
                }
            }
        }
        
        best_slot.zip(best_order)
    }
    
    /// Update best bid/ask after order changes
    pub fn update_best_prices(&mut self, data: &[u8]) {
        let mut best_bid = 0u64;
        let mut best_ask = u64::MAX;
        
        for i in 0..Self::MAX_ORDERS {
            if let Some(order) = self.get_order(data, i as u64) {
                if order.remaining_size > 0 {
                    if order.is_bid() && order.price > best_bid {
                        best_bid = order.price;
                    } else if order.is_ask() && order.price < best_ask {
                        best_ask = order.price;
                    }
                }
            }
        }
        
        self.best_bid = best_bid;
        self.best_ask = if best_ask == u64::MAX { 0 } else { best_ask };
    }
}

/// Orderbook side enumeration for clarity
pub enum OrderbookSide {
    Bid,
    Ask,
}
