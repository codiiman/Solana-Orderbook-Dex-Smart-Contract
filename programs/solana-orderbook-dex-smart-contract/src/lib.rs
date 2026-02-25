use anchor_lang::prelude::*;

pub mod errors;
pub mod events;
pub mod instructions;
pub mod orderbook;
pub mod state;

use instructions::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

/// Phoenix-inspired Orderbook DEX Program
/// 
/// A high-performance, self-custodial orderbook DEX for Solana supporting:
/// - Central Limit Orderbook (CLOB) with efficient matching
/// - Multiple spot markets with configurable parameters
/// - Limit and market orders with various time-in-force options
/// - Maker/taker fee model with protocol treasury
/// - Optional oracle integration for price protection
#[program]
pub mod solana_orderbook_dex {
    use super::*;

    /// Initialize the global DEX configuration
    /// Sets up the protocol authority, fee parameters, and default settings
    pub fn initialize(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
        instructions::initialize::handler(ctx, params)
    }

    /// Create a new spot market
    /// Permissioned or permissionless based on global config
    pub fn create_market(
        ctx: Context<CreateMarket>,
        params: CreateMarketParams,
    ) -> Result<()> {
        instructions::create_market::handler(ctx, params)
    }

    /// Place a limit or market order
    /// Supports IOC, FOK, Post-only, and GTC time-in-force options
    pub fn place_order(
        ctx: Context<PlaceOrder>,
        params: PlaceOrderParams,
    ) -> Result<()> {
        instructions::place_order::handler(ctx, params)
    }

    /// Cancel an existing order
    /// Returns unfilled portion to trader's account
    pub fn cancel_order(
        ctx: Context<CancelOrder>,
        order_id: u128,
    ) -> Result<()> {
        instructions::cancel_order::handler(ctx, order_id)
    }

    /// Match orders in the orderbook
    /// Can be called by anyone to trigger matching engine
    pub fn match_orders(
        ctx: Context<MatchOrders>,
        max_iterations: u8,
    ) -> Result<()> {
        instructions::match_orders::handler(ctx, max_iterations)
    }

    /// Settle matched orders and transfer tokens
    /// Handles atomic token swaps and fee collection
    pub fn settle(ctx: Context<Settle>, fill_ids: Vec<u128>) -> Result<()> {
        instructions::settle::handler(ctx, fill_ids)
    }

    /// Deposit tokens into the DEX for trading
    /// Creates or updates trader's position account
    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }

    /// Withdraw tokens from the DEX
    /// Transfers available balance back to trader
    pub fn withdraw(
        ctx: Context<Withdraw>,
        amount: u64,
    ) -> Result<()> {
        instructions::withdraw::handler(ctx, amount)
    }

    /// Admin: Update market parameters
    /// Only callable by market or protocol authority
    pub fn update_market_params(
        ctx: Context<UpdateMarketParams>,
        params: UpdateMarketParamsParams,
    ) -> Result<()> {
        instructions::update_market_params::handler(ctx, params)
    }

    /// Admin: Pause/unpause a market
    /// Prevents new orders during pause
    pub fn pause_market(
        ctx: Context<PauseMarket>,
        paused: bool,
    ) -> Result<()> {
        instructions::pause_market::handler(ctx, paused)
    }

    /// Admin: Update protocol fees
    /// Only callable by protocol authority
    pub fn update_protocol_fees(
        ctx: Context<UpdateProtocolFees>,
        maker_fee_bps: Option<u16>,
        taker_fee_bps: Option<u16>,
    ) -> Result<()> {
        instructions::update_protocol_fees::handler(ctx, maker_fee_bps, taker_fee_bps)
    }
}
