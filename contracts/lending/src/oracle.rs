use crate::types::PlatformConfig;
use soroban_sdk::{contractclient, contracttype, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contractclient(name = "ReflectorClient")]
pub trait ReflectorOracle {
    fn lastprice(env: Env, asset: Symbol) -> Option<PriceData>;
}

pub fn get_price(env: &Env, config: &PlatformConfig, asset_symbol: Symbol) -> i128 {
    let client = ReflectorClient::new(env, &config.oracle_address);
    let price_data_opt = client.lastprice(&asset_symbol);

    if price_data_opt.is_none() {
        panic!("Oracle price not found for asset");
    }

    let price_data = price_data_opt.unwrap();
    let current_time = env.ledger().timestamp();

    if current_time >= price_data.timestamp {
        let age = current_time - price_data.timestamp;
        if age > config.max_price_staleness_secs {
            panic!("Oracle price is too stale");
        }
    } else {
        panic!("Oracle price timestamp in the future");
    }

    price_data.price
}

/// Converts USD value to Token amount.
/// Both `usd` and `price` are 7-decimal fixed-point values.
/// `decimals` is the decimals of the token.
/// Formula: amount = (usd * 10^decimals) / price
pub fn usd_to_token_amount(usd: i128, price: i128, decimals: u32) -> i128 {
    if price <= 0 {
        panic!("Invalid price");
    }
    if usd < 0 {
        panic!("Negative USD amount");
    }
    if usd == 0 {
        return 0;
    }

    let multiplier: i128 = 10_i128.pow(decimals);

    let numerator = usd
        .checked_mul(multiplier)
        .expect("Overflow in usd_to_token multiplication");
    numerator / price
}

/// Converts Token amount to USD value.
/// `tokens` is the token amount with `decimals`.
/// `price` is 7-decimal fixed-point.
/// Returns USD value as 7-decimal fixed-point.
/// Formula: usd = (tokens * price) / 10^decimals
pub fn token_to_usd(tokens: i128, price: i128, decimals: u32) -> i128 {
    if price <= 0 {
        panic!("Invalid price");
    }
    if tokens < 0 {
        panic!("Negative token amount");
    }
    if tokens == 0 {
        return 0;
    }

    let divisor: i128 = 10_i128.pow(decimals);

    let numerator = tokens
        .checked_mul(price)
        .expect("Overflow in token_to_usd multiplication");
    numerator / divisor
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::PlatformConfig;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env};

    #[soroban_sdk::contract]
    pub struct MockOracle;

    #[soroban_sdk::contractimpl]
    impl MockOracle {
        pub fn lastprice(env: Env, asset: Symbol) -> Option<PriceData> {
            if asset == Symbol::new(&env, "XLM") {
                Some(PriceData {
                    price: 1_000_000, // $0.10
                    timestamp: 1000,
                })
            } else if asset == Symbol::new(&env, "STALE") {
                Some(PriceData {
                    price: 1_000_000,
                    timestamp: 500,
                })
            } else {
                None
            }
        }
    }

    #[test]
    fn test_usd_to_token_amount() {
        let amount = usd_to_token_amount(50_000_000, 10_000_000, 7);
        assert_eq!(amount, 50_000_000);

        let amount_eth = usd_to_token_amount(10_000_000_000, 20_000_000_000, 18);
        assert_eq!(amount_eth, 500_000_000_000_000_000);

        assert_eq!(usd_to_token_amount(0, 10_000_000, 7), 0);
    }

    #[test]
    #[should_panic(expected = "Invalid price")]
    fn test_usd_to_token_zero_price() {
        usd_to_token_amount(100, 0, 7);
    }

    #[test]
    #[should_panic(expected = "Negative USD amount")]
    fn test_usd_to_token_negative_usd() {
        usd_to_token_amount(-100, 10_000_000, 7);
    }

    #[test]
    #[should_panic(expected = "Overflow in usd_to_token multiplication")]
    fn test_usd_to_token_overflow() {
        usd_to_token_amount(i128::MAX, 1, 18);
    }

    #[test]
    fn test_token_to_usd() {
        let usd = token_to_usd(50_000_000, 10_000_000, 7);
        assert_eq!(usd, 50_000_000);

        let usd_eth = token_to_usd(500_000_000_000_000_000, 20_000_000_000, 18);
        assert_eq!(usd_eth, 10_000_000_000);

        assert_eq!(token_to_usd(0, 10_000_000, 7), 0);
    }

    #[test]
    #[should_panic(expected = "Invalid price")]
    fn test_token_to_usd_zero_price() {
        token_to_usd(100, 0, 7);
    }

    #[test]
    #[should_panic(expected = "Negative token amount")]
    fn test_token_to_usd_negative_tokens() {
        token_to_usd(-100, 10_000_000, 7);
    }

    #[test]
    #[should_panic(expected = "Overflow in token_to_usd multiplication")]
    fn test_token_to_usd_overflow() {
        token_to_usd(i128::MAX, 2, 7);
    }

    #[test]
    fn test_get_price_success() {
        let env = Env::default();
        let oracle_id = env.register(MockOracle, ());
        let config = PlatformConfig {
            admin: Address::generate(&env),
            fee_receiver: Address::generate(&env),
            platform_fee_bps: 100,
            liquidator_fee_bps: 100,
            min_buffer_bps: 100,
            max_buffer_bps: 1000,
            min_liq_threshold_bps: 100,
            max_liq_threshold_bps: 1000,
            oracle_address: oracle_id,
            max_price_staleness_secs: 100,
        };

        let mut ledger_info = env.ledger().get();
        ledger_info.timestamp = 1050;
        env.ledger().set(ledger_info);

        let price = get_price(&env, &config, Symbol::new(&env, "XLM"));
        assert_eq!(price, 1_000_000);
    }

    #[test]
    #[should_panic(expected = "Oracle price not found for asset")]
    fn test_get_price_not_found() {
        let env = Env::default();
        let oracle_id = env.register(MockOracle, ());
        let config = PlatformConfig {
            admin: Address::generate(&env),
            fee_receiver: Address::generate(&env),
            platform_fee_bps: 100,
            liquidator_fee_bps: 100,
            min_buffer_bps: 100,
            max_buffer_bps: 1000,
            min_liq_threshold_bps: 100,
            max_liq_threshold_bps: 1000,
            oracle_address: oracle_id,
            max_price_staleness_secs: 100,
        };

        get_price(&env, &config, Symbol::new(&env, "NOT_FOUND"));
    }

    #[test]
    #[should_panic(expected = "Oracle price is too stale")]
    fn test_get_price_stale() {
        let env = Env::default();
        let oracle_id = env.register(MockOracle, ());
        let config = PlatformConfig {
            admin: Address::generate(&env),
            fee_receiver: Address::generate(&env),
            platform_fee_bps: 100,
            liquidator_fee_bps: 100,
            min_buffer_bps: 100,
            max_buffer_bps: 1000,
            min_liq_threshold_bps: 100,
            max_liq_threshold_bps: 1000,
            oracle_address: oracle_id,
            max_price_staleness_secs: 100,
        };

        let mut ledger_info = env.ledger().get();
        ledger_info.timestamp = 1050;
        env.ledger().set(ledger_info);

        get_price(&env, &config, Symbol::new(&env, "STALE"));
    }
}
