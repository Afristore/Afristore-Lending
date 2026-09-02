// ------------------------------------------------------------
// contract.rs — Afristore NFT Lending contract implementation
//

use soroban_sdk::{contract, contractimpl, Address, Env};

use crate::storage::{has_config, set_config};
use crate::types::PlatformConfig;

/// NFT lending contract.
#[contract]
pub struct LendingContract;

#[contractimpl]
impl LendingContract {
    /// Initializes the contract with the platform configuration.
    ///
    /// Panics if the contract has already been initialized or if any
    /// invariant is violated.
    pub fn initialize(
        env: Env,
        admin: Address,
        fee_receiver: Address,
        oracle_address: Address,
        platform_fee_bps: u32,
        liquidator_fee_bps: u32,
        min_buffer_bps: u32,
        max_buffer_bps: u32,
        min_liq_threshold_bps: u32,
        max_liq_threshold_bps: u32,
        max_price_staleness_secs: u64,
    ) {
        assert!(!has_config(&env), "already initialized");
        assert!(
            min_buffer_bps < max_buffer_bps,
            "min_buffer_bps must be less than max_buffer_bps"
        );
        assert!(
            min_liq_threshold_bps < max_liq_threshold_bps,
            "min_liq_threshold_bps must be less than max_liq_threshold_bps"
        );
        assert!(
            max_liq_threshold_bps < min_buffer_bps,
            "max_liq_threshold_bps must be less than min_buffer_bps"
        );
        assert!(
            platform_fee_bps + liquidator_fee_bps < 10_000,
            "combined fees must be less than 10000 bps"
        );

        admin.require_auth();

        set_config(
            &env,
            &PlatformConfig {
                admin,
                fee_receiver,
                platform_fee_bps,
                liquidator_fee_bps,
                min_buffer_bps,
                max_buffer_bps,
                min_liq_threshold_bps,
                max_liq_threshold_bps,
                oracle_address,
                max_price_staleness_secs,
            },
        );
    }
}
