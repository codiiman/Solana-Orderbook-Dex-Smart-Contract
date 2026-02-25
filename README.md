# Phoenix-Inspired Orderbook DEX on Solana

[![Anchor](https://img.shields.io/badge/Anchor-0.30.1-000000?logo=anchor)](https://www.anchor-lang.com/)
[![Solana](https://img.shields.io/badge/Solana-1.18-9945FF?logo=solana)](https://solana.com/)
[![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

A high-performance, self-custodial orderbook DEX smart contract for Solana, inspired by Phoenix V1. This implementation provides a complete central limit orderbook (CLOB) system with efficient order matching, multiple markets support, and comprehensive fee management.

## 🎯 Overview

This project implements a production-grade orderbook DEX on Solana, featuring:

- **Central Limit Orderbook (CLOB)**: Efficient price-time priority matching
- **Multiple Markets**: Support for creating and managing multiple spot trading pairs
- **Flexible Order Types**: Limit orders with GTC, IOC, FOK, and Post-only time-in-force options
- **Maker/Taker Fee Model**: Configurable fee structure with protocol treasury
- **Self-Custodial**: Traders maintain control of their funds
- **High Throughput**: Optimized for Solana's high-performance architecture

## ✨ Features

### Core Functionality

- ✅ **Market Creation**: Permissioned or permissionless market creation with configurable parameters
- ✅ **Order Placement**: Limit and market orders with multiple time-in-force options
- ✅ **Order Cancellation**: Cancel open orders and unlock locked funds
- ✅ **Order Matching**: Price-time priority matching engine with self-trade prevention
- ✅ **Settlement**: Atomic token swaps with fee collection
- ✅ **Deposit/Withdraw**: Self-custodial fund management

### Advanced Features

- ✅ **Admin Controls**: Pause markets, update parameters, manage protocol fees
- ✅ **Event System**: Comprehensive event emission for all operations
- ✅ **Error Handling**: Detailed custom errors for debugging
- ✅ **Security**: Reentrancy protection, overflow checks, authority validation
- ✅ **Extensibility**: Reserved space for future features (perps, AMM integration, oracles)

## 🏗️ Architecture

### Account Structure

```
GlobalConfig
├── Protocol authority
├── Fee recipient
├── Maker/taker fee rates
└── Market creation settings

Market
├── Market ID
├── Base/Quote mints
├── Base/Quote vaults
├── Tick size & Lot size
├── Best bid/ask prices
└── Order count & volume

Orderbook
├── Market reference
├── Best bid/ask
├── Order count
└── Slab allocator for orders

TraderState
├── Trader & Market
├── Available balances (base/quote)
└── Locked balances (in orders)

PendingFill
├── Fill ID
├── Bid/Ask order IDs
├── Price & Size
├── Fees
└── Settlement status
```

### Instruction Flow

```
1. Initialize
   └─> Create GlobalConfig

2. Create Market
   └─> Create Market account
   └─> Create Base/Quote vaults
   └─> Initialize Orderbook

3. Deposit
   └─> Transfer tokens to vault
   └─> Update TraderState

4. Place Order
   └─> Validate order params
   └─> Lock tokens
   └─> Add to Orderbook
   └─> Update best prices

5. Match Orders
   └─> Find matching orders
   └─> Calculate fills
   └─> Update orderbook
   └─> Create PendingFill

6. Settle
   └─> Transfer tokens
   └─> Collect fees
   └─> Update balances

7. Cancel Order
   └─> Remove from Orderbook
   └─> Unlock tokens
   └─> Update TraderState
```

## 🔑 Key Algorithms

### Order Matching Algorithm

The matching engine uses a **price-time priority** system:

1. **Price Priority**: Best bid (highest) matches with best ask (lowest)
2. **Time Priority**: Within the same price level, orders are matched FIFO
3. **Self-Trade Prevention**: Orders from the same trader cannot match
4. **Partial Fills**: Orders can be partially filled, remaining size stays in orderbook

**Matching Logic:**
```rust
while iterations < max_iterations {
    best_bid = find_highest_bid()
    best_ask = find_lowest_ask()
    
    if best_bid.price >= best_ask.price {
        match_price = min(best_bid.price, best_ask.price)
        fill_size = min(best_bid.remaining, best_ask.remaining)
        
        fill_orders(best_bid, best_ask, match_price, fill_size)
        create_pending_fill(...)
    } else {
        break // No more matches
    }
}
```

### Fee Calculation

Fees are calculated based on maker/taker status:

- **Maker**: Order that adds liquidity (resting order)
- **Taker**: Order that removes liquidity (immediate match)

```rust
maker_fee = quote_amount * maker_fee_bps / 10000
taker_fee = quote_amount * taker_fee_bps / 10000
```

The older order in a match is considered the maker.

### Orderbook Data Structure

The orderbook uses a **slab allocator** pattern:

- Orders stored in a fixed-size array (max 1000 orders per orderbook)
- Free list for efficient slot reuse
- Price-sorted linked lists for efficient traversal
- O(1) insertion/deletion with free list
- O(n) best price lookup (can be optimized with price level pointers)

## 📦 Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) (v1.18+)
- [Anchor](https://www.anchor-lang.com/docs/installation) (v0.30.1+)
- [Node.js](https://nodejs.org/) (v18+)
- [Yarn](https://yarnpkg.com/) or npm

### Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd Solana-Orderbook-Dex-Smart-Contract
   ```

2. **Install dependencies**
   ```bash
   anchor build
   npm install
   # or
   yarn install
   ```

3. **Configure Solana CLI**
   ```bash
   solana config set --url localhost
   solana-keygen new  # If you don't have a keypair
   ```

## 🚀 Build & Test

### Build

```bash
# Build the program
anchor build

# Build only (no IDL generation)
anchor build -- --no-default-features
```

### Test

```bash
# Run all tests
anchor test

# Run tests with logs
anchor test -- --nocapture

# Run specific test file
anchor test tests/solana-orderbook-dex-smart-contract.ts
```

### Deploy

```bash
# Deploy to localnet
anchor deploy

# Deploy to devnet
solana config set --url devnet
anchor deploy

# Deploy to mainnet (use with caution!)
solana config set --url mainnet-beta
anchor deploy
```

## 💻 Usage Examples

### Initialize the DEX

```typescript
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

// Derive global config PDA
const [globalConfig] = PublicKey.findProgramAddressSync(
  [Buffer.from("global_config")],
  program.programId
);

// Initialize
await program.methods
  .initialize({
    makerFeeBps: new BN(2),      // 0.02%
    takerFeeBps: new BN(4),      // 0.04%
    permissionlessMarkets: true,
    marketCreationFee: new BN(0),
  })
  .accounts({
    globalConfig,
    authority: authority.publicKey,
    feeRecipient: feeRecipient.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### Create a Market

```typescript
const marketId = new BN(1);
const [market] = PublicKey.findProgramAddressSync(
  [Buffer.from("market"), marketId.toArrayLike(Buffer, "le", 8)],
  program.programId
);

await program.methods
  .createMarket({
    marketId,
    tickSize: new BN(100),      // $0.0001 for 6-decimal quote
    lotSize: new BN(1000000),   // 0.001 base units
  })
  .accounts({
    globalConfig,
    market,
    baseMint: solMint,
    quoteMint: usdcMint,
    // ... vault accounts
  })
  .rpc();
```

### Place an Order

```typescript
// Deposit first
await program.methods
  .deposit(new BN(100000000)) // 0.1 SOL
  .accounts({
    market,
    traderState,
    trader: trader.publicKey,
    // ... token accounts
  })
  .rpc();

// Place bid order
await program.methods
  .placeOrder({
    side: 0,                    // 0 = bid, 1 = ask
    price: new BN(50000000),    // $50.00
    size: new BN(100000000),    // 0.1 base
    timeInForce: 0,             // 0 = GTC
  })
  .accounts({
    market,
    orderbook,
    traderState,
    trader: trader.publicKey,
  })
  .rpc();
```

### Match Orders

```typescript
// Anyone can call match_orders
await program.methods
  .matchOrders(10) // Max 10 iterations
  .accounts({
    market,
    orderbook,
    globalConfig,
    pendingFills,
  })
  .rpc();
```

### Cancel Order

```typescript
await program.methods
  .cancelOrder(orderId)
  .accounts({
    market,
    orderbook,
    traderState,
    trader: trader.publicKey,
  })
  .rpc();
```

## 📚 Project Structure

```
Solana-Orderbook-Dex-Smart-Contract-1/
├── Anchor.toml                 # Anchor configuration
├── Cargo.toml                 # Rust dependencies
├── programs/
│   └── solana-orderbook-dex-smart-contract/
│       └── src/
│           ├── lib.rs         # Program entry point
│           ├── state.rs       # Account structures
│           ├── errors.rs      # Custom errors
│           ├── events.rs      # Event definitions
│           ├── orderbook.rs   # Orderbook logic
│           └── instructions/  # Instruction handlers
│               ├── initialize.rs
│               ├── create_market.rs
│               ├── place_order.rs
│               ├── cancel_order.rs
│               ├── match_orders.rs
│               ├── settle.rs
│               ├── deposit.rs
│               ├── withdraw.rs
│               └── ...
├── tests/
│   └── solana-orderbook-dex-smart-contract.ts
└── README.md
```

## 📞 Contact & Support

- telegram: https://t.me/codiiman
- twitter:  https://x.com/codiiman_
