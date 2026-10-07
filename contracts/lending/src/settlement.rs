//! Shared settlement waterfall implementation for Afristore NFT Lending.
//!
//! This module provides the single shared settlement function called by both
//! `return_nft()` (voluntary borrower repayment) and `liquidate()` (health-factor
//! or expiration liquidation). Having a single entrypoint guarantees that payout math
//! cannot drift between voluntary and liquidation settlement paths.
//!
//! # Waterfall:
//! ```text
//! owed_usd          = declared_price_usd + accrued_interest_usd
//! platform_fee_usd  = owed_usd * platform_fee_bps / 10_000
//! liquidator_fee_usd= owed_usd * liquidator_fee_bps / 10_000  // 0 on voluntary return
//!
//! debit_tokens      = usd_to_token_amount(owed_usd + fees, price, decimals)
//! lender_payout     = usd_to_token_amount(owed_usd, ...)
//! platform_payout   = usd_to_token_amount(platform_fee_usd, ...)
//! liquidator_payout = usd_to_token_amount(liquidator_fee_usd, ...)
//! borrower_rem      = collateral_amount - debit_tokens  // floor at 0, never negative
//! ```

use crate::oracle::{get_price, usd_to_token_amount};
use crate::types::{PlatformConfig, Position};
use soroban_sdk::{contracttype, Address, Env, Symbol};

// ----------------------------------------------------------------------------
// Types & Storage Keys
// ----------------------------------------------------------------------------

/// Storage keys for settlement-related configuration and metadata.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Mapping of collateral currency address to its Reflector oracle symbol.
    WhitelistedCurrency(Address),
    /// Explicit symbol mapping for an asset.
    CurrencySymbol(Address),
    /// Explicit decimals override for a token address.
    CurrencyDecimals(Address),
}

/// Resulting payout amounts from the settlement waterfall.
///
/// Captured by callers (`return_nft()` and `liquidate()`) to emit typed
/// on-chain events (`PositionReturnedEvent` and `PositionLiquidatedEvent`).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettleResult {
    /// Total amount owed in USD (fixed-point 7 decimals): declared price + accrued interest.
    pub owed_usd: i128,
    /// Accrued interest portion in USD.
    pub accrued_interest_usd: i128,
    /// Protocol fee portion in USD.
    pub platform_fee_usd: i128,
    /// Liquidator bounty portion in USD (0 on voluntary return).
    pub liquidator_fee_usd: i128,
    /// Total debit converted into collateral token units.
    pub debit_tokens: i128,
    /// Collateral token amount transferred to the lender.
    pub lender_payout: i128,
    /// Collateral token amount transferred to the platform fee receiver.
    pub platform_payout: i128,
    /// Collateral token amount transferred to the liquidator (0 if voluntary).
    pub liquidator_payout: i128,
    /// Remaining collateral token amount refunded to the borrower (clamped to 0).
    pub borrower_rem: i128,
}

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

/// Registers an oracle symbol mapping for a collateral token.
pub fn set_currency_symbol(env: &Env, currency: &Address, symbol: &Symbol) {
    env.storage()
        .persistent()
        .set(&DataKey::CurrencySymbol(currency.clone()), symbol);
}

/// Registers an explicit decimal override for a collateral token.
pub fn set_currency_decimals(env: &Env, currency: &Address, decimals: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::CurrencyDecimals(currency.clone()), &decimals);
}

/// Calculates accrued interest in USD (7 decimals fixed point) according to the
/// monthly tiered schedule defined on the position.
pub fn calculate_accrued_interest_usd(position: &Position, now: u64) -> i128 {
    if now <= position.start_time || position.interest_schedule_bps.is_empty() {
        return 0;
    }

    let elapsed_secs = now - position.start_time;
    let elapsed_days = elapsed_secs / 86_400;
    let full_months = elapsed_days / 30;
    let partial_days = elapsed_days % 30;

    let len = position.interest_schedule_bps.len();
    let last = len - 1;

    let mut total: i128 = 0;
    for month in 0..full_months {
        let idx = if month >= last as u64 {
            last
        } else {
            month as u32
        };
        let rate = position.interest_schedule_bps.get(idx).unwrap();
        let month_interest = position
            .declared_price_usd
            .checked_mul(rate as i128)
            .unwrap_or(0)
            / 10_000;
        total = total.checked_add(month_interest).unwrap_or(total);
    }

    let partial_idx = if full_months >= last as u64 {
        last
    } else {
        full_months as u32
    };
    let partial_rate = position.interest_schedule_bps.get(partial_idx).unwrap();
    let partial_month_full_rate = position
        .declared_price_usd
        .checked_mul(partial_rate as i128)
        .unwrap_or(0)
        / 10_000;
    let partial_interest = partial_month_full_rate
        .checked_mul(partial_days as i128)
        .unwrap_or(0)
        / 30;

    total.checked_add(partial_interest).unwrap_or(total)
}

// ----------------------------------------------------------------------------
// Main Settlement Waterfall
// ----------------------------------------------------------------------------

/// Executes the shared settlement waterfall for both voluntary loan repayments
/// and liquidations.
///
/// # Requirements:
/// 1. Fetches the oracle price internally using Reflector.
/// 2. Executes all token transfers via `soroban_sdk::token::Client`.
/// 3. Returns a `SettleResult` struct with all amounts (used by events).
/// 4. Clamps `borrower_rem` to 0 with no underflow.
/// 5. Does NOT transfer the NFT — it is already with the borrower by design.
pub fn settle(
    env: &Env,
    position: &Position,
    liquidator: Option<Address>,
    config: &PlatformConfig,
) -> SettleResult {
    // 1. Resolve asset symbol & decimals for the collateral currency
    let asset_symbol: Symbol = env
        .storage()
        .persistent()
        .get(&DataKey::CurrencySymbol(
            position.collateral_currency.clone(),
        ))
        .or_else(|| {
            env.storage()
                .persistent()
                .get(&DataKey::WhitelistedCurrency(
                    position.collateral_currency.clone(),
                ))
        })
        .unwrap_or_else(|| Symbol::new(env, "XLM"));

    let decimals: u32 = env
        .storage()
        .persistent()
        .get(&DataKey::CurrencyDecimals(
            position.collateral_currency.clone(),
        ))
        .unwrap_or_else(|| {
            let token_client = soroban_sdk::token::Client::new(env, &position.collateral_currency);
            token_client.decimals()
        });

    // 2. Fetch oracle price internally
    let price = get_price(env, config, asset_symbol);

    // 3. Compute interest & USD debts
    let now = env.ledger().timestamp();
    let accrued_interest_usd = calculate_accrued_interest_usd(position, now);
    let owed_usd = position
        .declared_price_usd
        .checked_add(accrued_interest_usd)
        .expect("Overflow in owed_usd calculation");

    let platform_fee_usd = owed_usd
        .checked_mul(config.platform_fee_bps as i128)
        .expect("Overflow in platform_fee_usd calculation")
        / 10_000;

    // Liquidator bounty is strictly 0 on voluntary return (liquidator is None)
    let liquidator_fee_usd = if liquidator.is_some() {
        owed_usd
            .checked_mul(config.liquidator_fee_bps as i128)
            .expect("Overflow in liquidator_fee_usd calculation")
            / 10_000
    } else {
        0
    };

    let total_fees_usd = platform_fee_usd
        .checked_add(liquidator_fee_usd)
        .expect("Overflow in total fees calculation");
    let total_debit_usd = owed_usd
        .checked_add(total_fees_usd)
        .expect("Overflow in total debit calculation");

    // 4. Convert USD values to collateral token units
    let debit_tokens = usd_to_token_amount(total_debit_usd, price, decimals);
    let lender_payout = usd_to_token_amount(owed_usd, price, decimals);
    let platform_payout = usd_to_token_amount(platform_fee_usd, price, decimals);
    let liquidator_payout = if liquidator.is_some() {
        usd_to_token_amount(liquidator_fee_usd, price, decimals)
    } else {
        0
    };

    // 5. Calculate borrower remainder with strict underflow protection:
    // When the collateral value depreciates significantly, total debit_tokens can
    // exceed the initial collateral_amount deposited. In such an under-collateralized
    // or fully consumed scenario, the borrower forfeits their entire collateral deposit.
    // borrower_rem is explicitly floored at 0 and must never underflow or become negative.
    let borrower_rem = if position.collateral_amount > debit_tokens {
        position.collateral_amount - debit_tokens
    } else {
        0
    };

    // 6. Execute token transfers via soroban_sdk::token::Client
    let token_client = soroban_sdk::token::Client::new(env, &position.collateral_currency);
    let contract_address = env.current_contract_address();

    if lender_payout > 0 {
        token_client.transfer(&contract_address, &position.lender, &lender_payout);
    }

    if platform_payout > 0 {
        token_client.transfer(&contract_address, &config.fee_receiver, &platform_payout);
    }

    if let Some(ref liquidator_addr) = liquidator {
        if liquidator_payout > 0 {
            token_client.transfer(&contract_address, liquidator_addr, &liquidator_payout);
        }
    }

    if borrower_rem > 0 {
        token_client.transfer(&contract_address, &position.borrower, &borrower_rem);
    }

    // 7. Note on NFT: Do NOT transfer the NFT — it's already with the borrower by design.

    SettleResult {
        owed_usd,
        accrued_interest_usd,
        platform_fee_usd,
        liquidator_fee_usd,
        debit_tokens,
        lender_payout,
        platform_payout,
        liquidator_payout,
        borrower_rem,
    }
}

// ----------------------------------------------------------------------------
// Unit Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle::PriceData;
    use crate::types::PositionStatus;
    use soroban_sdk::testutils::{Address as AddressTestUtils, Ledger};
    use soroban_sdk::{token, vec, Address, Env};

    #[soroban_sdk::contract]
    pub struct MockReflector;

    #[soroban_sdk::contractimpl]
    impl MockReflector {
        pub fn lastprice(env: Env, asset: Symbol) -> Option<PriceData> {
            if asset == Symbol::new(&env, "XLM") {
                Some(PriceData {
                    price: 1_000_000, // $0.10 (7 decimals)
                    timestamp: env.ledger().timestamp(),
                })
            } else if asset == Symbol::new(&env, "USDC") {
                Some(PriceData {
                    price: 10_000_000, // $1.00 (7 decimals)
                    timestamp: env.ledger().timestamp(),
                })
            } else {
                None
            }
        }
    }

    #[soroban_sdk::contract]
    pub struct SettlementContract;

    #[soroban_sdk::contractimpl]
    impl SettlementContract {
        pub fn test_settle(
            env: Env,
            position: Position,
            liquidator: Option<Address>,
            config: PlatformConfig,
        ) -> SettleResult {
            settle(&env, &position, liquidator, &config)
        }
    }

    #[allow(dead_code)]
    struct TestContext {
        env: Env,
        contract_id: Address,
        oracle_id: Address,
        token_address: Address,
        token_client: token::Client<'static>,
        token_asset_client: token::StellarAssetClient<'static>,
        config: PlatformConfig,
        lender: Address,
        borrower: Address,
        liquidator: Address,
    }

    impl TestContext {
        fn setup() -> Self {
            let env = Env::default();
            env.mock_all_auths_allowing_non_root_auth();

            let contract_id = env.register(SettlementContract, ());
            let oracle_id = env.register(MockReflector, ());

            let token_admin = Address::generate(&env);
            let token_contract = env.register_stellar_asset_contract_v2(token_admin);
            let token_address = token_contract.address();

            let token_client = token::Client::new(&env, &token_address);
            let token_asset_client = token::StellarAssetClient::new(&env, &token_address);

            let admin = Address::generate(&env);
            let fee_receiver = Address::generate(&env);
            let lender = Address::generate(&env);
            let borrower = Address::generate(&env);
            let liquidator = Address::generate(&env);

            let config = PlatformConfig {
                admin,
                fee_receiver,
                platform_fee_bps: 100,   // 1%
                liquidator_fee_bps: 200, // 2%
                min_buffer_bps: 1100,
                max_buffer_bps: 5000,
                min_liq_threshold_bps: 1050,
                max_liq_threshold_bps: 1200,
                oracle_address: oracle_id.clone(),
                max_price_staleness_secs: 3600,
            };

            // Set ledger timestamp
            let mut ledger_info = env.ledger().get();
            ledger_info.timestamp = 10_000_000;
            env.ledger().set(ledger_info);

            TestContext {
                env,
                contract_id,
                oracle_id,
                token_address,
                token_client,
                token_asset_client,
                config,
                lender,
                borrower,
                liquidator,
            }
        }

        fn create_position(
            &self,
            declared_price_usd: i128,
            collateral_amount: i128,
            schedule: soroban_sdk::Vec<u32>,
            start_time: u64,
        ) -> Position {
            Position {
                id: 1,
                listing_id: 10,
                lender: self.lender.clone(),
                borrower: self.borrower.clone(),
                nft_contract: Address::generate(&self.env),
                token_id: 42,
                declared_price_usd,
                collateral_currency: self.token_address.clone(),
                collateral_amount,
                interest_schedule_bps: schedule,
                liquidation_threshold_bps: 1100,
                start_time,
                max_duration_secs: 86_400 * 60,
                status: PositionStatus::Active,
            }
        }
    }

    #[test]
    fn test_voluntary_return_zero_liquidator_fee_partial_remaining() {
        let ctx = TestContext::setup();
        let client = SettlementContractClient::new(&ctx.env, &ctx.contract_id);

        // Price = $0.10 (price = 1_000_000)
        // Declared price = $10.00 (100_000_000 in 7 decimals)
        // At start time = now (0 elapsed days -> 0 interest)
        // Collateral = 200 tokens (2_000_000_000 in 7 decimals, worth $200.00)
        let schedule = vec![&ctx.env, 500u32]; // 5% per month
        let position = ctx.create_position(100_000_000, 2_000_000_000, schedule, 10_000_000);

        // Fund the contract with collateral
        ctx.token_asset_client
            .mint(&ctx.contract_id, &2_000_000_000);

        // Voluntary return -> liquidator is None
        let result = client.test_settle(&position, &None, &ctx.config);

        // Owed USD = 100_000_000
        assert_eq!(result.owed_usd, 100_000_000);
        assert_eq!(result.accrued_interest_usd, 0);

        // Platform fee = 1% = 1_000_000 USD
        assert_eq!(result.platform_fee_usd, 1_000_000);
        // Liquidator fee = 0 on voluntary return
        assert_eq!(result.liquidator_fee_usd, 0);

        // Tokens:
        // Lender payout = $10 / $0.10 = 100 tokens (1_000_000_000)
        assert_eq!(result.lender_payout, 1_000_000_000);
        // Platform payout = $0.10 / $0.10 = 1 token (10_000_000)
        assert_eq!(result.platform_payout, 10_000_000);
        // Liquidator payout = 0
        assert_eq!(result.liquidator_payout, 0);

        // Total debit tokens = 101 tokens (1_010_000_000)
        assert_eq!(result.debit_tokens, 1_010_000_000);

        // Partial remaining collateral refunded to borrower = 200 - 101 = 99 tokens
        assert_eq!(result.borrower_rem, 990_000_000);

        // Verify balances transferred
        assert_eq!(ctx.token_client.balance(&ctx.lender), 1_000_000_000);
        assert_eq!(
            ctx.token_client.balance(&ctx.config.fee_receiver),
            10_000_000
        );
        assert_eq!(ctx.token_client.balance(&ctx.borrower), 990_000_000);
        assert_eq!(ctx.token_client.balance(&ctx.contract_id), 0);
    }

    #[test]
    fn test_liquidation_with_liquidator_fee() {
        let ctx = TestContext::setup();
        let client = SettlementContractClient::new(&ctx.env, &ctx.contract_id);

        // 30 days elapsed -> full month interest: 500 bps = 5% of $100 = $5.00
        let start_time = 10_000_000 - (30 * 86_400);
        let schedule = vec![&ctx.env, 500u32];
        let position = ctx.create_position(1_000_000_000, 20_000_000_000, schedule, start_time);

        ctx.token_asset_client
            .mint(&ctx.contract_id, &20_000_000_000);

        // Liquidation -> liquidator is Some
        let result = client.test_settle(&position, &Some(ctx.liquidator.clone()), &ctx.config);

        // Declared $100 + $5 interest = $105 owed
        assert_eq!(result.owed_usd, 1_050_000_000);
        assert_eq!(result.accrued_interest_usd, 50_000_000);

        // Platform fee = 1% of $105 = $1.05 (10_500_000)
        assert_eq!(result.platform_fee_usd, 10_500_000);
        // Liquidator fee = 2% of $105 = $2.10 (21_000_000)
        assert_eq!(result.liquidator_fee_usd, 21_000_000);

        // Token payouts at $0.10 / token
        assert_eq!(result.lender_payout, 10_500_000_000);
        assert_eq!(result.platform_payout, 105_000_000);
        assert_eq!(result.liquidator_payout, 210_000_000);

        // Total debit = 108.15 tokens = 10_815_000_000
        assert_eq!(result.debit_tokens, 10_815_000_000);
        assert_eq!(result.borrower_rem, 20_000_000_000 - 10_815_000_000);

        assert_eq!(ctx.token_client.balance(&ctx.liquidator), 210_000_000);
        assert_eq!(ctx.token_client.balance(&ctx.lender), 10_500_000_000);
        assert_eq!(
            ctx.token_client.balance(&ctx.config.fee_receiver),
            105_000_000
        );
        assert_eq!(ctx.token_client.balance(&ctx.borrower), 9_185_000_000);
    }

    #[test]
    fn test_full_collateral_consumed_clamps_borrower_rem_to_zero() {
        let ctx = TestContext::setup();
        let client = SettlementContractClient::new(&ctx.env, &ctx.contract_id);

        // Position where debit_tokens exceeds collateral:
        // Declared price = $100, collateral deposited = only 500 tokens ($50 worth)
        let schedule = vec![&ctx.env, 0u32];
        let position = ctx.create_position(1_000_000_000, 5_000_000_000, schedule, 10_000_000);

        ctx.token_asset_client
            .mint(&ctx.contract_id, &20_000_000_000);

        let result = client.test_settle(&position, &Some(ctx.liquidator.clone()), &ctx.config);

        // debit_tokens exceeds collateral_amount (10_300_000_000 > 5_000_000_000)
        assert!(result.debit_tokens > position.collateral_amount);
        // borrower_rem must be strictly clamped to 0 without underflow
        assert_eq!(result.borrower_rem, 0);
    }

    #[test]
    fn test_accrued_interest_math() {
        let ctx = TestContext::setup();
        // 45 days elapsed: month 1 (500 bps) + 15 days of month 2 (600 bps)
        // $100 declared:
        // month 1 = $5.00
        // month 2 half = $6.00 * 15 / 30 = $3.00
        // Total interest = $8.00 (80_000_000)
        let start_time = 10_000_000 - (45 * 86_400);
        let schedule = vec![&ctx.env, 500u32, 600u32];
        let position = ctx.create_position(1_000_000_000, 20_000_000_000, schedule, start_time);

        let accrued = calculate_accrued_interest_usd(&position, 10_000_000);
        assert_eq!(accrued, 80_000_000);
    }
}
