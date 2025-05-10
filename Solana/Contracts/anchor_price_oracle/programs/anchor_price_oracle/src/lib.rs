use anchor_lang::prelude::*;

declare_id!("EUeWpzuAwmY13VNbVumpTJVZwmoUkrf1cujqKVGr7rRE"); 

// Seed for PDA
const PRICE_FEED_PDA_SEED: &[u8] = b"price_feed_v1"; // Using a specific seed

#[program]
pub mod anchor_price_oracle {
    use super::*;

    /// Initialize the price oracle account
    /// @param initial_price: e.g. 2197 (representing $21.97)
    /// @param decimals: e.g. 2 (meaning price has 2 decimal places)
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

    /// Update the price
    /// Only the authority set during initialization can call this method
    /// @param new_price: New price in same format as initial_price
    pub fn update_price(ctx: Context<UpdateOraclePrice>, new_price: u64) -> Result<()> {
        let price_account = &mut ctx.accounts.price_data_account;
        // Permission check is handled by `has_one = authority` constraint
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

    /// Get current price information
    /// This method primarily demonstrates reading or triggering events via instruction.
    /// Clients can typically read and deserialize price_data_account data directly.
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
        init, // Initialize this account
        payer = authority, // authority will pay rent
        space = 8 + PriceData::LEN, // 8 bytes for Anchor discriminator + PriceData length
        seeds = [PRICE_FEED_PDA_SEED], // Use constant seed to create PDA
        bump // Anchor will find and pass the bump seed
    )]
    pub price_data_account: Account<'info, PriceData>, // Account storing price data
    #[account(mut)] // authority needs to be mutable as it's the payer
    pub authority: Signer<'info>, // Signer for initialization, becomes price controller
    pub system_program: Program<'info, System>, // System program required for account creation
}

#[derive(Accounts)]
pub struct UpdateOraclePrice<'info> {
    #[account(
        mut, // Need mutable access to modify data
        seeds = [PRICE_FEED_PDA_SEED],
        bump = price_data_account.bump, // Use stored bump for verification
        has_one = authority @ PriceOracleError::InvalidAuthority // Critical: ensure signer is the authority stored in price_data_account
    )]
    pub price_data_account: Account<'info, PriceData>,
    pub authority: Signer<'info>, // Update must be signed by preset authority
}

#[derive(Accounts)]
pub struct GetOraclePrice<'info> {
    #[account(
        seeds = [PRICE_FEED_PDA_SEED],
        bump = price_data_account.bump // Use stored bump for verification
    )]
    pub price_data_account: Account<'info, PriceData>,
}

// --------- STATE ACCOUNT ---------

#[account] // Define on-chain state account structure
pub struct PriceData {
    pub authority: Pubkey, // Public key authorized to update price
    pub price: u64,        // Price, e.g. 2197
    pub decimals: u8,      // Decimal places, e.g. 2
    pub bump: u8,          // Bump seed for PDA verification
}

impl PriceData {
    // Calculate required account space
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

