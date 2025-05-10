import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
// 1. 更新类型导入以匹配您的程序名
import { AnchorPriceOracle } from "../target/types/anchor_price_oracle"; 
import { assert } from "chai";

// 2. 更新 describe 块的名称（可选，但推荐）
describe("anchor-price-oracle", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // 3. 更新程序名称和类型
  //    确保 'AnchorPriceOracle' 与您在 `anchor.workspace` 中的程序名称匹配
  //    这通常是您 lib.rs 中 `#[program] pub mod your_program_name` 的 PascalCase 形式。
  const program = anchor.workspace.AnchorPriceOracle as Program<AnchorPriceOracle>;

  // PDA for the price data account
  let priceDataPDA: anchor.web3.PublicKey;
  let pdaBump: number; // To store the bump for the PDA

  // 使用合约中定义的种子来派生PDA
  const PRICE_FEED_SEED = Buffer.from("price_feed_v1");

  before(async () => {
    // 在测试开始前派生PDA地址和bump
    [priceDataPDA, pdaBump] =
      await anchor.web3.PublicKey.findProgramAddressSync(
        [PRICE_FEED_SEED],
        program.programId
      );
  });

  it("Is initialized with $21.97!", async () => {
    // 定义初始化参数
    const initialPrice = new anchor.BN(2197); // $21.97 (2197)
    const decimals = 2;
    const authority = provider.wallet.publicKey;

    // 调用 initialize 指令
    const tx = await program.methods
      .initialize(initialPrice, decimals)
      .accounts({
        priceDataAccount: priceDataPDA,
        authority: authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    
    console.log("Initialize transaction signature", tx);

    // 获取并验证已创建账户的数据
    const accountData = await program.account.priceData.fetch(priceDataPDA);
    assert.ok(accountData.authority.equals(authority), "Authority mismatch");
    assert.ok(accountData.price.eq(initialPrice), "Initial price mismatch");
    assert.equal(accountData.decimals, decimals, "Decimals mismatch");
    assert.equal(accountData.bump, pdaBump, "PDA bump mismatch"); // 验证存储的bump是否正确
    console.log("Price data account initialized successfully:", {
        authority: accountData.authority.toBase58(),
        price: accountData.price.toString(),
        decimals: accountData.decimals,
        bump: accountData.bump,
    });
  });

  it("Updates the price to $25.50!", async () => {
    const newPrice = new anchor.BN(2550); // $25.50
    const authority = provider.wallet.publicKey; // 假设权限方未改变

    // 调用 updatePrice 指令
    const tx = await program.methods
      .updatePrice(newPrice)
      .accounts({
        priceDataAccount: priceDataPDA,
        authority: authority,
      })
      .rpc();
    console.log("Update price transaction signature", tx);

    // 获取并验证更新后的价格
    const accountData = await program.account.priceData.fetch(priceDataPDA);
    assert.ok(accountData.price.eq(newPrice), "Price was not updated correctly");
    console.log("Price updated successfully. New price:", accountData.price.toString());
  });

  it("Gets the current price!", async () => {
    // 调用 getPrice 指令
    // 这个测试主要验证指令是否可以被调用，以及事件是否按预期发出
    // 客户端通常直接 fetch 账户数据
    
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
        .catch(reject); //确保RPC调用错误能被捕获
    });

    if (listener) {
        await program.removeEventListener(listener);
    }

    assert.isNotNull(eventData, "PriceInfo event was not emitted");
    console.log("PriceInfo event emitted:", eventData);
    console.log("Event slot:", slot);

    // 将事件数据与链上账户数据进行比较
    const accountData = await program.account.priceData.fetch(priceDataPDA);
    const event = eventData as any; // 类型断言以便访问字段

    assert.ok(accountData.price.eq(event.price), "Event price does not match account price");
    assert.equal(accountData.decimals, event.decimals, "Event decimals do not match account decimals");
    assert.ok(accountData.authority.equals(event.authority), "Event authority does not match account authority");

    console.log("Fetched Price Data for get_price check:", {
        price: accountData.price.toString(),
        decimals: accountData.decimals,
    });
  });

  it("Fails to update price with wrong authority", async () => {
    const wrongAuthority = anchor.web3.Keypair.generate(); // 生成一个新的密钥对作为错误的权限方
    const priceBeforeAttempt = (await program.account.priceData.fetch(priceDataPDA)).price;
    const attemptPrice = new anchor.BN(9999);

    // 给错误的权限方空投一些 SOL 以支付交易费（如果它需要签名交易的话）
    // 对于这个特定的测试，由于约束，交易会在链上失败，但错误的权限方仍然需要签名。
    // 如果您的本地测试网配置为允许费用由默认payer支付，则此步骤可能不是严格必需的。
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
        .signers([wrongAuthority]) // 错误的权限方签名
        .rpc();
      assert.fail("Transaction should have failed due to invalid authority!");
    } catch (error) {
      // console.log(JSON.stringify(error)); // 打印完整错误信息用于调试
      assert.isNotNull(error, "Error was not thrown");
      // 检查错误是否与 'InvalidAuthority' 相关
      // Anchor 错误通常包含 error.msg 或 error.error.errorMessage
      // 并且 error.error.errorCode.code 应该是 "InvalidAuthority" (错误号通常是 6000)
      const anchorError = error as any;
      assert.equal(anchorError.error?.errorCode?.code, "InvalidAuthority", "Error code mismatch");
      assert.equal(anchorError.error?.errorCode?.number, 6000, "Error number mismatch for InvalidAuthority");
      console.log("Successfully caught expected error for invalid authority.");
    }

    // 验证价格没有被错误的操作改变
    const accountData = await program.account.priceData.fetch(priceDataPDA);
    assert.ok(accountData.price.eq(priceBeforeAttempt), "Price should not have changed after failed update attempt.");
  });
});