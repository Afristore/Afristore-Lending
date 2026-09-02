#![cfg(test)]

// ------------------------------------------------------------
// test.rs — Afristore NFT Lending contract tests
//

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::LendingContract;
use crate::storage::get_config;

fn setup() -> (Env, Address) {
    let env = Env::default();
    let contract_id = env.register(LendingContract, ());
    env.mock_all_auths();
    (env, contract_id)
}

fn init_config(env: &Env, contract_id: &Address) {
    let client = crate::contract::LendingContractClient::new(env, contract_id);
    let admin = Address::generate(env);
    let fee_receiver = Address::generate(env);
    let oracle = Address::generate(env);

    client.initialize(
        &admin,
        &fee_receiver,
        &oracle,
        &100,  // platform_fee_bps
        &200,  // liquidator_fee_bps
        &1000, // min_buffer_bps
        &5000, // max_buffer_bps
        &100,  // min_liq_threshold_bps
        &900,  // max_liq_threshold_bps
        &3600, // max_price_staleness_secs
    );
}

#[test]
fn test_initialize_success() {
    let (env, contract_id) = setup();
    init_config(&env, &contract_id);

    let cfg = env.as_contract(&contract_id, || get_config(&env)).expect("config should be set");
    assert_eq!(cfg.platform_fee_bps, 100);
    assert_eq!(cfg.min_buffer_bps, 1000);
    assert_eq!(cfg.max_liq_threshold_bps, 900);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_double_init_panics() {
    let (env, contract_id) = setup();
    init_config(&env, &contract_id);
    init_config(&env, &contract_id);
}

#[test]
#[should_panic(expected = "min_buffer_bps must be less than max_buffer_bps")]
fn test_initialize_bad_buffer_bounds_panics() {
    let (env, contract_id) = setup();
    let client = crate::contract::LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let fee_receiver = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.initialize(
        &admin,
        &fee_receiver,
        &oracle,
        &100,
        &200,
        &5000, // min > max
        &1000,
        &100,
        &900,
        &3600,
    );
}

#[test]
#[should_panic(expected = "max_liq_threshold_bps must be less than min_buffer_bps")]
fn test_initialize_liq_threshold_above_buffer_panics() {
    let (env, contract_id) = setup();
    let client = crate::contract::LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let fee_receiver = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.initialize(
        &admin,
        &fee_receiver,
        &oracle,
        &100,
        &200,
        &1000,
        &5000,
        &100,
        &1500, // max_liq_threshold > min_buffer
        &3600,
    );
}

#[test]
#[should_panic(expected = "combined fees must be less than 10000 bps")]
fn test_initialize_combined_fees_too_high_panics() {
    let (env, contract_id) = setup();
    let client = crate::contract::LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let fee_receiver = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.initialize(
        &admin,
        &fee_receiver,
        &oracle,
        &9_000,
        &1_000,
        &1000,
        &5000,
        &100,
        &900,
        &3600,
    );
}
