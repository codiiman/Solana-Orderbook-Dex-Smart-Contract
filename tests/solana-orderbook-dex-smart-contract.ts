import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaOrderbookDex } from "../target/types/solana_orderbook_dex";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo } from "@solana/spl-token";
import { expect } from "chai";

describe("solana-orderbook-dex", () => {
  // Configure the client
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaOrderbookDex as Program<SolanaOrderbookDex>;
  
  let globalConfig: PublicKey;
  let globalConfigBump: number;
  let authority: Keypair;
  let feeRecipient: Keypair;
  
  let baseMint: PublicKey;
  let quoteMint: PublicKey;
  let market: PublicKey;
  let marketBump: number;
  let marketId: anchor.BN;
  
  before(async () => {
    authority = Keypair.generate();
    feeRecipient = Keypair.generate();
    
    // Airdrop SOL to authority
    const signature = await provider.connection.requestAirdrop(
      authority.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(signature);
    
    // Derive global config PDA
    [globalConfig, globalConfigBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("global_config")],
      program.programId
    );
    
    // Create test mints
    baseMint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      9 // 9 decimals
    );
    
    quoteMint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      6 // 6 decimals (USDC-like)
    );
    
    marketId = new anchor.BN(1);
    [market, marketBump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("market"),
        marketId.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );
  });

  it("Initializes global config", async () => {
    const tx = await program.methods
      .initialize({
        makerFeeBps: new anchor.BN(2), // 0.02%
        takerFeeBps: new anchor.BN(4), // 0.04%
        permissionlessMarkets: true,
        marketCreationFee: new anchor.BN(0),
      })
      .accounts({
        globalConfig,
        authority: authority.publicKey,
        feeRecipient: feeRecipient.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authority])
      .rpc();
    
    console.log("Initialize transaction:", tx);
    
    const config = await program.account.globalConfig.fetch(globalConfig);
    expect(config.makerFeeBps.toNumber()).to.equal(2);
    expect(config.takerFeeBps.toNumber()).to.equal(4);
    expect(config.permissionlessMarkets).to.be.true;
  });

  it("Creates a market", async () => {
    const [baseVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("base_vault"), market.toBuffer()],
      program.programId
    );
    
    const [quoteVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("quote_vault"), market.toBuffer()],
      program.programId
    );
    
    const tx = await program.methods
      .createMarket({
        marketId,
        tickSize: new anchor.BN(100), // $0.0001 for 6-decimal quote
        lotSize: new anchor.BN(1000000), // 0.001 base units for 9-decimal base
      })
      .accounts({
        globalConfig,
        market,
        baseMint,
        quoteMint,
        baseVault,
        quoteVault,
        authority: authority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authority])
      .rpc();
    
    console.log("Create market transaction:", tx);
    
    const marketAccount = await program.account.market.fetch(market);
    expect(marketAccount.marketId.toNumber()).to.equal(1);
    expect(marketAccount.baseMint.toString()).to.equal(baseMint.toString());
    expect(marketAccount.quoteMint.toString()).to.equal(quoteMint.toString());
    expect(marketAccount.paused).to.be.false;
  });

  it("Deposits tokens", async () => {
    const trader = Keypair.generate();
    
    // Airdrop SOL to trader
    const sig = await provider.connection.requestAirdrop(
      trader.publicKey,
      5 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);
    
    // Create token accounts
    const traderBaseAccount = await createAccount(
      provider.connection,
      trader,
      baseMint,
      trader.publicKey
    );
    
    const traderQuoteAccount = await createAccount(
      provider.connection,
      trader,
      quoteMint,
      trader.publicKey
    );
    
    // Mint tokens
    await mintTo(
      provider.connection,
      trader,
      baseMint,
      traderBaseAccount,
      authority,
      1000000000 // 1 base token
    );
    
    await mintTo(
      provider.connection,
      trader,
      quoteMint,
      traderQuoteAccount,
      authority,
      100000000 // 100 quote tokens
    );
    
    const [baseVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("base_vault"), market.toBuffer()],
      program.programId
    );
    
    const [traderState] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("trader_state"),
        trader.publicKey.toBuffer(),
        market.toBuffer(),
      ],
      program.programId
    );
    
    // Deposit base
    const depositBaseTx = await program.methods
      .deposit(new anchor.BN(100000000)) // 0.1 base
      .accounts({
        market,
        traderState,
        trader: trader.publicKey,
        traderTokenAccount: traderBaseAccount,
        vault: baseVault,
        mint: baseMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([trader])
      .rpc();
    
    console.log("Deposit base transaction:", depositBaseTx);
    
    const state = await program.account.traderState.fetch(traderState);
    expect(state.baseAvailable.toNumber()).to.equal(100000000);
  });

  it("Places an order", async () => {
    // This test would require setting up the orderbook account
    // For now, it's a placeholder showing the structure
    console.log("Place order test - requires orderbook account setup");
  });

  it("Matches orders", async () => {
    // This test would require multiple traders and orders
    console.log("Match orders test - requires multiple orders");
  });

  it("Cancels an order", async () => {
    // This test would require an existing order
    console.log("Cancel order test - requires existing order");
  });
});
