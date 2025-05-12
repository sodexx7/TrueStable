use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

declare_id!("CZVdyKQKuK3peBnfWtqXWF2AM6MKK2qneNhZuB6UMive");

#[program]
pub mod truestable {
    use super::*;

    pub fn initialize_lending_pool(
        ctx: Context<InitializeLendingPool>,
        bump: u8,
    ) -> Result<()> {
        let lending_pool = &mut ctx.accounts.lending_pool;
        lending_pool.bump = bump;
        lending_pool.ore_mint = ctx.accounts.ore_mint.key();
        lending_pool.usdc_mint = ctx.accounts.usdc_mint.key();
        lending_pool.treasury = ctx.accounts.treasury.key();
        lending_pool.ltv_ratio = 75; // 75% LTV ratio
        lending_pool.liquidation_threshold = 80; // 80% liquidation threshold
        Ok(())
    }

    pub fn update_treasury(
        ctx: Context<UpdateTreasury>
    ) -> Result<()> {
        let lending_pool = &mut ctx.accounts.lending_pool;
        require!(
            ctx.accounts.treasury.mint == lending_pool.usdc_mint,
            ErrorCode::InvalidTreasury
        );
        lending_pool.treasury = ctx.accounts.treasury.key();
        Ok(())
    }

    pub fn deposit_collateral(
        ctx: Context<DepositCollateral>,
        amount: u64,
    ) -> Result<()> {
        // Transfer ORE tokens from user to vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_ore_account.to_account_info(),
                to: ctx.accounts.ore_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.owner = ctx.accounts.user.key();
        position.collateral_amount = position.collateral_amount.checked_add(amount)
            .ok_or(ErrorCode::NumericalOverflow)?;
        
        Ok(())
    }

    pub fn borrow_usdc(
        ctx: Context<BorrowUsdc>,
        amount: u64,
    ) -> Result<()> {
        let position = &ctx.accounts.user_position;
        let lending_pool = &ctx.accounts.lending_pool;

        // Calculate maximum borrow amount based on collateral
        let max_borrow = (position.collateral_amount as u128)
            .checked_mul(lending_pool.ltv_ratio as u128)
            .ok_or(ErrorCode::NumericalOverflow)?
            .checked_div(100)
            .ok_or(ErrorCode::NumericalOverflow)? as u64;

        require!(amount <= max_borrow, ErrorCode::BorrowTooLarge);
        
        // Verify we're using the correct treasury
        require!(
            ctx.accounts.treasury.key() == lending_pool.treasury,
            ErrorCode::InvalidTreasury
        );

        // Use SPL Token program directly with the treasury account
        let ix = anchor_spl::token::Transfer {
            from: ctx.accounts.treasury.to_account_info(),
            to: ctx.accounts.user_usdc_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            ix,
        );
        
        anchor_spl::token::transfer(cpi_ctx, amount)?;

        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.borrowed_amount = position.borrowed_amount.checked_add(amount)
            .ok_or(ErrorCode::NumericalOverflow)?;

        Ok(())
    }

    pub fn repay_usdc(
        ctx: Context<RepayUsdc>,
        amount: u64,
    ) -> Result<()> {
        let position = &ctx.accounts.user_position;
        require!(
            amount <= position.borrowed_amount,
            ErrorCode::RepayAmountTooLarge
        );
        
        // Verify we're using the correct treasury
        require!(
            ctx.accounts.treasury.key() == ctx.accounts.lending_pool.treasury,
            ErrorCode::InvalidTreasury
        );

        // Use SPL Token program directly with the treasury account
        let ix = anchor_spl::token::Transfer {
            from: ctx.accounts.user_usdc_account.to_account_info(),
            to: ctx.accounts.treasury.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            ix,
        );
        
        anchor_spl::token::transfer(cpi_ctx, amount)?;

        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.borrowed_amount = position.borrowed_amount.checked_sub(amount)
            .ok_or(ErrorCode::NumericalOverflow)?;

        Ok(())
    }

    pub fn withdraw_collateral(
        ctx: Context<WithdrawCollateral>,
        amount: u64,
    ) -> Result<()> {
        let position = &ctx.accounts.user_position;
        let lending_pool = &ctx.accounts.lending_pool;

        // Calculate remaining collateral after withdrawal
        let remaining_collateral = position.collateral_amount.checked_sub(amount)
            .ok_or(ErrorCode::InsufficientCollateral)?;

        // Check if remaining collateral is sufficient for current borrow
        let min_required_collateral = (position.borrowed_amount as u128)
            .checked_mul(100)
            .ok_or(ErrorCode::NumericalOverflow)?
            .checked_div(lending_pool.ltv_ratio as u128)
            .ok_or(ErrorCode::NumericalOverflow)? as u64;

        require!(
            remaining_collateral >= min_required_collateral,
            ErrorCode::InsufficientCollateral
        );

        // Transfer ORE tokens from vault to user using PDA signer
        let seeds = &[
            b"lending_pool".as_ref(),
            &[lending_pool.bump],
        ];
        let signer = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ore_vault.to_account_info(),
                to: ctx.accounts.user_ore_account.to_account_info(),
                authority: ctx.accounts.lending_pool.to_account_info(),
            },
            signer,
        );
        token::transfer(transfer_ctx, amount)?;

        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.collateral_amount = remaining_collateral;

        Ok(())
    }

    pub fn liquidate_position(
        ctx: Context<LiquidatePosition>,
        liquidation_amount: u64,
    ) -> Result<()> {
        let position = &ctx.accounts.user_position;
        let lending_pool = &ctx.accounts.lending_pool;

        // Verify treasury account
        require!(
            ctx.accounts.treasury.key() == lending_pool.treasury,
            ErrorCode::InvalidTreasury
        );

        // Calculate current collateral ratio
        let collateral_ratio = (position.collateral_amount as u128)
            .checked_mul(100)
            .ok_or(ErrorCode::NumericalOverflow)?
            .checked_div(position.borrowed_amount as u128)
            .ok_or(ErrorCode::NumericalOverflow)? as u8;

        // Check if position is liquidatable
        require!(
            collateral_ratio <= lending_pool.liquidation_threshold,
            ErrorCode::PositionNotLiquidatable
        );

        // Check if liquidation amount is valid
        require!(
            liquidation_amount <= position.borrowed_amount,
            ErrorCode::LiquidationAmountTooLarge
        );

        // Calculate ORE tokens to receive (with 5% bonus)
        let ore_to_receive = (liquidation_amount as u128)
            .checked_mul(105)
            .ok_or(ErrorCode::NumericalOverflow)?
            .checked_div(100)
            .ok_or(ErrorCode::NumericalOverflow)? as u64;

        require!(
            ore_to_receive <= position.collateral_amount,
            ErrorCode::InsufficientCollateral
        );

        // Transfer USDC from liquidator to treasury
        let transfer_usdc_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.liquidator_usdc_account.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
                authority: ctx.accounts.liquidator.to_account_info(),
            },
        );
        token::transfer(transfer_usdc_ctx, liquidation_amount)?;

        // Transfer ORE from vault to liquidator using PDA signer
        let seeds = &[
            b"lending_pool".as_ref(),
            &[lending_pool.bump],
        ];
        let signer = &[&seeds[..]];

        let transfer_ore_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ore_vault.to_account_info(),
                to: ctx.accounts.liquidator_ore_account.to_account_info(),
                authority: ctx.accounts.lending_pool.to_account_info(),
            },
            signer,
        );
        token::transfer(transfer_ore_ctx, ore_to_receive)?;

        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.borrowed_amount = position.borrowed_amount.checked_sub(liquidation_amount)
            .ok_or(ErrorCode::NumericalOverflow)?;
        position.collateral_amount = position.collateral_amount.checked_sub(ore_to_receive)
            .ok_or(ErrorCode::NumericalOverflow)?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeLendingPool<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 32 + 1 + 1 + 1,
        seeds = [b"lending_pool"],
        bump
    )]
    pub lending_pool: Account<'info, LendingPool>,
    pub ore_mint: Account<'info, Mint>,
    pub usdc_mint: Account<'info, Mint>,
    /// CHECK: Treasury account that holds USDC
    pub treasury: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub lending_pool: Account<'info, LendingPool>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 32 + 8 + 8,
        seeds = [b"user_position", user.key().as_ref()],
        bump
    )]
    pub user_position: Account<'info, UserPosition>,
    #[account(mut)]
    pub user_ore_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub ore_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BorrowUsdc<'info> {
    #[account(mut)]
    pub lending_pool: Account<'info, LendingPool>,
    #[account(
        mut,
        seeds = [b"user_position", user.key().as_ref()],
        bump,
        constraint = user_position.owner == user.key()
    )]
    pub user_position: Account<'info, UserPosition>,
    /// CHECK: Treasury account that holds USDC, validated through lending pool
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,
    #[account(mut)]
    pub user_usdc_account: Account<'info, TokenAccount>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RepayUsdc<'info> {
    #[account(mut)]
    pub lending_pool: Account<'info, LendingPool>,
    #[account(
        mut,
        seeds = [b"user_position", user.key().as_ref()],
        bump,
        constraint = user_position.owner == user.key()
    )]
    pub user_position: Account<'info, UserPosition>,
    /// CHECK: Treasury account that holds USDC, validated through lending pool
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,
    #[account(mut)]
    pub user_usdc_account: Account<'info, TokenAccount>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(
        mut,
        seeds = [b"lending_pool"],
        bump = lending_pool.bump,
    )]
    pub lending_pool: Account<'info, LendingPool>,
    #[account(
        mut,
        seeds = [b"user_position", user.key().as_ref()],
        bump,
        constraint = user_position.owner == user.key()
    )]
    pub user_position: Account<'info, UserPosition>,
    #[account(
        mut,
        constraint = ore_vault.mint == lending_pool.ore_mint,
        constraint = ore_vault.owner == lending_pool.key()
    )]
    pub ore_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_ore_account: Account<'info, TokenAccount>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateTreasury<'info> {
    #[account(mut)]
    pub lending_pool: Account<'info, LendingPool>,
    #[account(
        constraint = treasury.owner == authority.key(),
        constraint = treasury.mint == lending_pool.usdc_mint
    )]
    pub treasury: Account<'info, TokenAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct LiquidatePosition<'info> {
    #[account(
        mut,
        seeds = [b"lending_pool"],
        bump = lending_pool.bump,
    )]
    pub lending_pool: Account<'info, LendingPool>,
    #[account(
        mut,
        seeds = [b"user_position", position_owner.key().as_ref()],
        bump,
        constraint = user_position.owner == position_owner.key()
    )]
    pub user_position: Account<'info, UserPosition>,
    /// CHECK: The owner of the position being liquidated
    pub position_owner: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = ore_vault.mint == lending_pool.ore_mint,
        constraint = ore_vault.owner == lending_pool.key()
    )]
    pub ore_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = treasury.mint == lending_pool.usdc_mint,
        constraint = treasury.key() == lending_pool.treasury
    )]
    pub treasury: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = liquidator_usdc_account.mint == lending_pool.usdc_mint,
        constraint = liquidator_usdc_account.owner == liquidator.key()
    )]
    pub liquidator_usdc_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = liquidator_ore_account.mint == lending_pool.ore_mint,
        constraint = liquidator_ore_account.owner == liquidator.key()
    )]
    pub liquidator_ore_account: Account<'info, TokenAccount>,
    pub liquidator: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct LendingPool {
    pub ore_mint: Pubkey,
    pub usdc_mint: Pubkey,
    pub treasury: Pubkey,
    pub bump: u8,
    pub ltv_ratio: u8,
    pub liquidation_threshold: u8,
}

#[account]
pub struct UserPosition {
    pub owner: Pubkey,
    pub collateral_amount: u64,
    pub borrowed_amount: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Numerical overflow occurred")]
    NumericalOverflow,
    #[msg("Borrow amount exceeds maximum allowed")]
    BorrowTooLarge,
    #[msg("Insufficient collateral")]
    InsufficientCollateral,
    #[msg("Unauthorized access")]
    UnauthorizedAccess,
    #[msg("Repay amount exceeds borrowed amount")]
    RepayAmountTooLarge,
    #[msg("The treasury provided doesn't match the one in the lending pool")]
    InvalidTreasury,
    #[msg("Position is not liquidatable")]
    PositionNotLiquidatable,
    #[msg("Liquidation amount is too large")]
    LiquidationAmountTooLarge,
}
