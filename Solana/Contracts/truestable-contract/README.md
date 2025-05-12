# TrueStable

## Features

- Deposit ORE tokens as collateral
- Borrow USDC against your collateral
- Repay borrowed USDC
- Withdraw collateral when loan-to-value ratio is healthy
- Liquidation system to manage unhealthy positions

## Smart Contract Architecture

The protocol implements the following main functions:

- `initialize_lending_pool`: Create a lending pool with the specified token mints
- `deposit_collateral`: Deposit ORE tokens as collateral
- `borrow_usdc`: Borrow USDC against deposited collateral
- `repay_usdc`: Repay borrowed USDC
- `withdraw_collateral`: Withdraw ORE collateral when position is healthy
- `liquidate_position`: Liquidate unhealthy positions (positions with collateral ratio below threshold)

## Development Setup

### Prerequisites

- Solana CLI tools
- Anchor framework
- Node.js and yarn
- Solana wallet (for deploying and testing)

### Installation

1. Clone the repository
```bash
git clone https://github.com/yourusername/truestable-contract.git
cd truestable
```

2. Install dependencies
```bash
yarn install
```

3. Build the program
```bash
anchor build
```

4. Deploy to localnet or testnet
```bash
anchor deploy
```

## Testing

The protocol has comprehensive test coverage for all major functions including liquidation mechanics.

To run tests:
```bash
anchor test
```

### Test Results

Latest test results:

```
Setting up test environment...
Derived lending pool address: 2ahGe2R37QUg9ca9H8z7DFYwxqVLaTJJSCuLr6EW4FnD
Derived user position address: CpSFVRgGMCQFKcBcmPH8pW7RVHHagc31miqqM4tPUsGR
Lending pool already initialized with:
  ORE mint: JAw9ietAEYZ3cZHztJQqCRSRFRiCfcKJSaKqcBQ2BZJb
  USDC mint: YVgnW2Z5ezkRJ4G7xh1q8cyBBWNVuh2MQGX3yVXTZYM
  Treasury: HreLtkLTgZaGG2mi4AjFoHKQEzfJfqS7JgNSsRBAdyVP
Verified treasury account is initialized
Created ORE vault: DzKHkfcTXxrDsnHYwqYbGV7MhKMhjbJSsTDXv56EnN8b
Created user ORE account: E4vu3Bs2GPKFennsjtyuYTKZcPYihzayaNSEjZFu6XTi
Created user USDC account: HreLtkLTgZaGG2mi4AjFoHKQEzfJfqS7JgNSsRBAdyVP
Minted 1000000000 ORE tokens to user
Using existing keypair as liquidator: Bnv2vC2GxzuY9m676FPLTJ65nFdWrMduVZpajtGe8pB3
Created liquidator ORE account: EjfpmGR3KBaBhrQ4tdYjJPH6SzkhtgdqPKDx1Yh2FpEC
Created liquidator USDC account: B4t4ab4aGibFLkmab6oNwUCPe5mzt3XpoMPS1UMkHiL7
Minted 750000000 USDC tokens to liquidator

Initializing lending pool...
Lending pool already initialized with: {
  oreMint: 'JAw9ietAEYZ3cZHztJQqCRSRFRiCfcKJSaKqcBQ2BZJb',
  usdcMint: 'YVgnW2Z5ezkRJ4G7xh1q8cyBBWNVuh2MQGX3yVXTZYM',
  treasury: 'HreLtkLTgZaGG2mi4AjFoHKQEzfJfqS7JgNSsRBAdyVP',
  ltvRatio: 75,
  liquidationThreshold: 80
}

Depositing collateral...
Transaction signature: 4GFfMajshzKpcrixS6rgsyzUtiXdHchX9f7pT3e6WdtXtWTSCfndTC8MTJyj765Rx9AMonnWAaRxWW9SKzXYoFE2
User position after deposit: {
  owner: 'FEjRfGokWnz3tpHj38p8ktgWsPGiemn5sNcgubS8xsp8',
  collateralAmount: '19000000000',
  borrowedAmount: '12599999997'
}

Borrowing USDC...
Transaction signature: 5wcvW569gLNmxNvyWZhxjg64uNfUc3huoAWYXcwhaHVW2oW3FuoPU61U1PYnfb9T6jmGtxG9bvxczpAS6SYTXWLo
User position after borrow: { 
  collateralAmount: '19000000000', 
  borrowedAmount: '13349999997' 
}

Repaying USDC...
Transaction signature: Ey1k97bCyZRgwbhQNLd73ni68jPRUsPFRuSTF2k2kccWk6ny4tvdFMSMG8rmk8zrjMxqbg877PdYuNNQzWhXqn5
User position after repay: { 
  collateralAmount: '19000000000', 
  borrowedAmount: '12599999997' 
}

Withdrawing collateral...
Transaction signature: 3ZsgmrA84sg7zVnQQhPnKBERf4W74R8aTapcz8CoiJxsouwLKNvnuQw81xjb3sG6efiuzDj7PF9h8CFyY5ZTfqQA
User position after withdrawal: { 
  collateralAmount: '18000000000', 
  borrowedAmount: '12599999997' 
}

Created position for liquidation test
Position details: { 
  collateral: '6000000000', 
  borrowed: '4199999999', 
  ratio: '142.86%' 
}
✅ Correctly rejected liquidation of healthy position

Position not liquidatable, but still testing amount validation

Current position: {
  collateralRatio: '142.86%',
  liquidationThreshold: '80%',
  isLiquidatable: false
}
Liquidation calculation: {
  liquidationAmount: 3149999999,
  bonus: 157499999,
  totalCollateralToReceive: 3307499998
}
✅ Liquidation threshold properly enforced
```

## Liquidation Mechanics

The protocol uses the following parameters for liquidation:

- LTV Ratio: 75% (loan-to-value limit for borrowing)
- Liquidation Threshold: 80% (positions with LTV above this can be liquidated)
- Liquidation Bonus: 5% (liquidators receive 5% extra collateral as incentive)

When a position's collateral value decreases or debt value increases such that the LTV ratio exceeds the liquidation threshold, the position becomes eligible for liquidation. Liquidators can repay a portion of the borrowed amount in exchange for collateral plus the bonus.

## License

[MIT License](LICENSE) 