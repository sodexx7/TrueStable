import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
// 1. Update type imports to match your program name
import { AnchorPriceOracle } from "../target/types/anchor_price_oracle"; 
import { assert } from "chai";

// 2. Update the describe block name (optional but recommended)
describe("anchor-price-oracle", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // 3. Update program name and type
  //    Ensure 'AnchorPriceOracle' matches your program name in `anchor.workspace`
  //    This is typically the PascalCase version of your `#[program] pub mod your_program_name` in lib.rs.
  const program = anchor.workspace.AnchorPriceOracle as Program<AnchorPriceOracle>;

  // PDA for the price data account
  let priceDataPDA: anchor.web3.PublicKey;
  let pdaBump: number; // To store the bump for the PDA

  // Use the seed defined in the contract to derive the PDA
  const PRICE_FEED_SEED = Buffer.from("price_feed_v1");

  before(async () => {
    // Derive PDA address and bump before tests begin
    [priceDataPDA, pdaBump] =
      await anchor.web3.PublicKey.findProgramAddressSync(
        [PRICE_FEED_SEED],
        program.programId
      );
  });

  it("Is initialized with $21.97!", async () => {
    // Define initialization parameters
    const initialPrice = new anchor.BN(2197); // $21.97 (2197)
    const decimals = 2;
    const authority = provider.wallet.publicKey;

    // Call the initialize instruction
    const tx = await program.methods
      .initialize(initialPrice, decimals)
      .accounts({
        priceDataAccount: priceDataPDA,
        authority: authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    
    console.log("Initialize transaction signature", tx);

    // Fetch and verify the created account's data
    const accountData = await program.account.priceData.fetch(priceDataPDA);
    assert.ok(accountData.authority.equals(authority), "Authority mismatch";
    assert.ok(accountData.price.eq(initialPrice), "Initial price mismatch";
    assert.equal(accountData.decimals, decimals, "Decimals mismatch");
    assert.equal(accountData.bump, pdaBump, "PDA bump mismatch"); // Verify stored bump is correct
    console.log("Price data account initialized successfully:", {
        authority: accountData.authority.toBase58(),
        price: accountData.price.toString(),
        decimals: accountData.decimals,
        bump: accountData.bump,
    });
  });

  it("Updates the price to $25.50!", async () => {
    const newPrice = new anchor.BN(2550); // $25.50
    const authority = provider.wallet.publicKey; // Assuming authority hasn't changed

    // Call the updatePrice instruction
    const tx = await program.methods
      .updatePrice(newPrice)
      .accounts({
        priceDataAccount: priceDataPDA,
        authority: authority,
      })
      .rpc();
    console.log("Update price transaction signature", tx);

    // Fetch and verify the updated price
    const accountData = await program.account.priceData.fetch(priceDataPDA);
    assert.ok(accountData.price.eq(newPrice), "Price was not updated correctly");
    console.log("Price updated successfully. New price:", accountData.price.toString());
  });

  it("Gets the current price!", async () => {
    // Call the getPrice instruction
    // This test primarily verifies that the instruction can be called and events are emitted as expected
    // Clients typically fetch account data directly
    
    let listener = null;
    let [eventData, slot] = await new Promise((resolve, reject) => {
      listener = program.addEventListener("PriceInfo", (event, slot) => {
        resolve([event, slot]);
      });
      program.methods
        .getPrice()
        .accounts({
          priceDataAccount: priceDataPDA,
        })
        .rpc()
        .catch(reject); // Ensure RPC call errors are caught
    });

    if (listener) {
        await program.removeEventListener(listener);
    }

    assert.isNotNull(eventData, "PriceInfo event was not emitted");
    console.log("PriceInfo event emitted:", eventData);
    console.log("Event slot:", slot);

    // Compare event data with on-chain account data
    const accountData = await program.account.priceData.fetch(priceDataPDA);
    const event = eventData as any; // Type assertion to access fields

    assert.ok(accountData.price.eq(event.price), "Event price does not match account price");
    assert.equal(accountData.decimals, event.decimals, "Event decimals do not match account decimals");
    assert.ok(accountData.authority.equals(event.authority), "Event authority does not match account authority");

    console.log("Fetched Price Data for get_price check:", {
        price: accountData.price.toString(),
        decimals: accountData.decimals,
    });
  });

  it("Fails to update price with wrong authority", async () => {
    const wrongAuthority = anchor.web3.Keypair.generate(); // Generate a new keypair as wrong authority
    const priceBeforeAttempt = (await program.account.priceData.fetch(priceDataPDA)).price;
    const attemptPrice = new anchor.BN(9999);

    // Airdrop some SOL to wrong authority to pay for transaction fees (if it needs to sign)
    // For this specific test, the transaction will fail on-chain due to constraints,
    // but the wrong authority still needs to sign.
    // If your local testnet is configured to allow fees to be paid by default payer, this step may not be strictly necessary.
    // await provider.connection.confirmTransaction(
    //   await provider.connection.requestAirdrop(wrongAuthority.publicKey, anchor.web3.LAMPORTS_PER_SOL)
    // );

    try {
      await program.methods
        .updatePrice(attemptPrice)
        .accounts({
          priceDataAccount: priceDataPDA,
          authority: wrongAuthority.publicKey,
        })
        .signers([wrongAuthority]) // Wrong authority signs
        .rpc();
      assert.fail("Transaction should have failed due to invalid authority!");
    } catch (error) {
      // console.log(JSON.stringify(error)); // Print full error for debugging
      assert.isNotNull(error, "Error was not thrown");
      // Check if error relates to 'InvalidAuthority'
      // Anchor errors typically contain error.msg or error.error.errorMessage
      // and error.error.errorCode.code should be "InvalidAuthority" (error number is typically 6000)
      const anchorError = error as any;
      assert.equal(anchorError.error?.errorCode?.code, "InvalidAuthority", "Error code mismatch");
      assert.equal(anchorError.error?.errorCode?.number, 6000, "Error number mismatch for InvalidAuthority");
      console.log("Successfully caught expected error for invalid authority.");
    }

    // Verify price wasn't changed by the invalid operation
    const accountData = await program.account.priceData.fetch(priceDataPDA);
    assert.ok(accountData.price.eq(priceBeforeAttempt), "Price should not have changed after failed update attempt.");
  });
});