use anchor_lang::prelude::*;

// TODO: 部署后请将此替换为您的实际程序ID
// 您可以在第一次成功构建和部署后，或通过 target/deploy/your_program_name-keypair.json 找到它
// 使用 `solana address -k target/deploy/your_program_name-keypair.json`
declare_id!("EUeWpzuAwmY13VNbVumpTJVZwmoUkrf1cujqKVGr7rRE"); // 这是一个占位符

// 用于PDA的种子
const PRICE_FEED_PDA_SEED: &[u8] = b"price_feed_v1"; // 使用一个特定的种子

#[program]
pub mod anchor_price_oracle {
    use super::*;

    /// 初始化价格预言机账户
    /// @param initial_price: 例如 2197 (表示 $21.97)
    /// @param decimals: 例如 2 (表示价格有2位小数)
    pub fn initialize(ctx: Context<InitializeOracle>, initial_price: u64, decimals: u8) -> Result<()> {
        let price_account = &mut ctx.accounts.price_data_account;
        
        price_account.authority = *ctx.accounts.authority.key;
        price_account.price = initial_price;
        price_account.decimals = decimals;
        // CORRECTED LINE: Directly access the bump for the account
        price_account.bump = ctx.bumps.price_data_account; 

        msg!("Price Oracle Initialized!");
        msg!("Authority: {}", price_account.authority);
        msg!("Initial Price: {}", price_account.price);
        msg!("Decimals: {}", price_account.decimals);
        msg!("Bump: {}", price_account.bump);

        emit!(OracleInitialized {
            authority: price_account.authority,
            price: price_account.price,
            decimals: price_account.decimals,
        });
        Ok(())
    }

    /// 更新价格
    /// 只有初始化时设定的 authority 才能调用此方法
    /// @param new_price: 新的价格，格式与 initial_price 相同
    pub fn update_price(ctx: Context<UpdateOraclePrice>, new_price: u64) -> Result<()> {
        let price_account = &mut ctx.accounts.price_data_account;
        // 权限检查已通过 `has_one = authority` 约束完成
        price_account.price = new_price;

        msg!("Price Updated!");
        msg!("New Price: {}", price_account.price);
        
        emit!(PriceChanged {
            price: price_account.price,
            decimals: price_account.decimals,
            updater: price_account.authority,
        });
        Ok(())
    }

    /// 获取当前价格信息
    /// 此方法主要用于演示通过指令读取或触发事件。
    /// 客户端通常可以直接读取和反序列化 price_data_account 的数据。
    pub fn get_price(ctx: Context<GetOraclePrice>) -> Result<()> {
        let price_account = &ctx.accounts.price_data_account;
        
        msg!("Current Price Information:");
        msg!("Price: {}", price_account.price);
        msg!("Decimals: {}", price_account.decimals);
        msg!("Authority: {}", price_account.authority);

        emit!(PriceInfo {
            price: price_account.price,
            decimals: price_account.decimals,
            authority: price_account.authority,
        });
        Ok(())
    }
}

// --------- ACCOUNTS STRUCTS ---------

#[derive(Accounts)]
pub struct InitializeOracle<'info> {
    #[account(
        init, // 初始化这个账户
        payer = authority, // authority 将支付租金
        space = 8 + PriceData::LEN, // 8 字节的 Anchor discriminator + PriceData 的长度
        seeds = [PRICE_FEED_PDA_SEED], // 使用常量种子创建PDA
        bump // Anchor 会找到并传入 bump 种子
    )]
    pub price_data_account: Account<'info, PriceData>, // 存储价格数据的账户
    #[account(mut)] // authority 需要是可变的，因为它是 payer
    pub authority: Signer<'info>, // 初始化操作的签名者，将成为价格的控制者
    pub system_program: Program<'info, System>, // 系统程序，创建账户时需要
}

#[derive(Accounts)]
pub struct UpdateOraclePrice<'info> {
    #[account(
        mut, // 需要可变权限来修改数据
        seeds = [PRICE_FEED_PDA_SEED],
        bump = price_data_account.bump, // 使用存储的 bump 进行校验
        has_one = authority @ PriceOracleError::InvalidAuthority // 关键：确保签名者是 price_data_account 中存储的 authority
    )]
    pub price_data_account: Account<'info, PriceData>,
    pub authority: Signer<'info>, // 更新操作必须由预设的 authority 签名
}

#[derive(Accounts)]
pub struct GetOraclePrice<'info> {
    #[account(
        seeds = [PRICE_FEED_PDA_SEED],
        bump = price_data_account.bump // 使用存储的 bump 进行校验
    )]
    pub price_data_account: Account<'info, PriceData>,
}

// --------- STATE ACCOUNT ---------

#[account] // 定义链上状态账户的结构
pub struct PriceData {
    pub authority: Pubkey, // 可以更新价格的公钥
    pub price: u64,        // 价格，例如 2197
    pub decimals: u8,      // 小数位数，例如 2
    pub bump: u8,          // 用于PDA验证的bump种子
}

impl PriceData {
    // 计算账户所需空间
    // authority (Pubkey): 32 bytes
    // price (u64): 8 bytes
    // decimals (u8): 1 byte
    // bump (u8): 1 byte
    pub const LEN: usize = 32 + 8 + 1 + 1;
}

// --------- ERRORS ---------

#[error_code]
pub enum PriceOracleError {
    #[msg("Invalid authority to perform this action.")]
    InvalidAuthority,
    // REMOVED: BumpSeedNotInHashMap, as direct bump access makes it obsolete
}

// --------- EVENTS ---------

#[event]
pub struct OracleInitialized {
    authority: Pubkey,
    price: u64,
    decimals: u8,
}

#[event]
pub struct PriceChanged {
    price: u64,
    decimals: u8,
    updater: Pubkey,
}

#[event]
pub struct PriceInfo {
    price: u64,
    decimals: u8,
    authority: Pubkey,
}

