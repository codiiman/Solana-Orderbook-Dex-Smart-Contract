use anchor_lang::prelude::*;

#[error_code]
pub enum DexError {
    // Market errors (0x1000-0x10FF)
    #[msg("Market not found")]
    MarketNotFound,
    #[msg("Market is paused")]
    MarketPaused,
    #[msg("Market already exists")]
    MarketAlreadyExists,
    #[msg("Invalid market parameters")]
    InvalidMarketParams,
    #[msg("Invalid base or quote mint")]
    InvalidMint,

    // Order errors (0x1100-0x11FF)
    #[msg("Order not found")]
    OrderNotFound,
    #[msg("Invalid order parameters")]
    InvalidOrderParams,
    #[msg("Order size too small")]
    OrderSizeTooSmall,
    #[msg("Order size too large")]
    OrderSizeTooLarge,
    #[msg("Invalid price")]
    InvalidPrice,
    #[msg("Price not on tick")]
    PriceNotOnTick,
    #[msg("Self-trade prevention triggered")]
    SelfTradePrevention,
    #[msg("Order already filled or cancelled")]
    OrderAlreadyFilled,
    #[msg("Invalid time-in-force")]
    InvalidTimeInForce,
    #[msg("Post-only order would cross spread")]
    PostOnlyWouldCross,

    // Orderbook errors (0x1200-0x12FF)
    #[msg("Orderbook is full")]
    OrderbookFull,
    #[msg("Orderbook is empty")]
    OrderbookEmpty,
    #[msg("Invalid orderbook state")]
    InvalidOrderbookState,
    #[msg("Orderbook depth exceeded")]
    OrderbookDepthExceeded,

    // Matching errors (0x1300-0x13FF)
    #[msg("No matching orders available")]
    NoMatchingOrders,
    #[msg("Matching iteration limit exceeded")]
    MatchingIterationLimit,
    #[msg("Invalid match price")]
    InvalidMatchPrice,
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,

    // Settlement errors (0x1400-0x14FF)
    #[msg("Settlement failed")]
    SettlementFailed,
    #[msg("Insufficient balance for settlement")]
    InsufficientBalance,
    #[msg("Invalid fill ID")]
    InvalidFillId,
    #[msg("Fill already settled")]
    FillAlreadySettled,

    // Account errors (0x1500-0x15FF)
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Invalid account owner")]
    InvalidAccountOwner,
    #[msg("Account not initialized")]
    AccountNotInitialized,
    #[msg("Invalid account state")]
    InvalidAccountState,

    // Authority errors (0x1600-0x16FF)
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid authority")]
    InvalidAuthority,
    #[msg("Market creation not allowed")]
    MarketCreationNotAllowed,

    // Math errors (0x1700-0x17FF)
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Math underflow")]
    MathUnderflow,
    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("Invalid fee calculation")]
    InvalidFeeCalculation,

    // Oracle errors (0x1800-0x18FF)
    #[msg("Oracle price not available")]
    OraclePriceNotAvailable,
    #[msg("Oracle price stale")]
    OraclePriceStale,
    #[msg("Oracle price deviation too large")]
    OraclePriceDeviationTooLarge,

    // General errors (0x1900-0x19FF)
    #[msg("Invalid instruction")]
    InvalidInstruction,
    #[msg("Operation not supported")]
    OperationNotSupported,
    #[msg("Reentrancy detected")]
    ReentrancyDetected,
}
