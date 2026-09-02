use soroban_sdk::{contracttype, Env};

use crate::types::PlatformConfig;

/// Storage keys for the lending contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Global platform configuration set once at initialization.
    Config,
}

/// Persists the platform configuration in instance storage.
pub fn set_config(env: &Env, config: &PlatformConfig) {
    env.storage().instance().set(&DataKey::Config, config);
}

/// Reads the platform configuration, if it has been set.
pub fn get_config(env: &Env) -> Option<PlatformConfig> {
    env.storage().instance().get(&DataKey::Config)
}

/// Returns `true` when the contract has already been initialized.
pub fn has_config(env: &Env) -> bool {
    get_config(env).is_some()
}
