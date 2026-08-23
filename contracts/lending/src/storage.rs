use crate::types::{Listing, PlatformConfig, Position};
use soroban_sdk::{contracttype, Address, Env};

/// Storage keys for contract instance and persistent storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Config,
    NextListingId,
    NextPositionId,
    Listing(u64),
    Position(u64),
    WhitelistedCurrency(Address),
}

/// Retrieve platform configuration from instance storage.
pub fn get_config(env: &Env) -> Option<PlatformConfig> {
    env.storage().instance().get(&DataKey::Config)
}

/// Store platform configuration in instance storage.
pub fn set_config(env: &Env, config: &PlatformConfig) {
    env.storage().instance().set(&DataKey::Config, config);
}

/// Check if platform configuration is initialized.
pub fn has_config(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Config)
}

/// Atomically fetch and increment the next listing ID.
pub fn next_listing_id(env: &Env) -> u64 {
    let id: u64 = env.storage().instance().get(&DataKey::NextListingId).unwrap_or(1);
    env.storage().instance().set(&DataKey::NextListingId, &(id + 1));
    id
}

/// Atomically fetch and increment the next position ID.
pub fn next_position_id(env: &Env) -> u64 {
    let id: u64 = env.storage().instance().get(&DataKey::NextPositionId).unwrap_or(1);
    env.storage().instance().set(&DataKey::NextPositionId, &(id + 1));
    id
}

/// Retrieve a listing by ID from persistent storage.
pub fn get_listing(env: &Env, id: u64) -> Option<Listing> {
    env.storage().persistent().get(&DataKey::Listing(id))
}

/// Store a listing in persistent storage.
pub fn set_listing(env: &Env, listing: &Listing) {
    env.storage().persistent().set(&DataKey::Listing(listing.id), listing);
}

/// Check if a listing exists in persistent storage.
pub fn has_listing(env: &Env, id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Listing(id))
}

/// Retrieve a position by ID from persistent storage.
pub fn get_position(env: &Env, id: u64) -> Option<Position> {
    env.storage().persistent().get(&DataKey::Position(id))
}

/// Store a position in persistent storage.
pub fn set_position(env: &Env, position: &Position) {
    env.storage().persistent().set(&DataKey::Position(position.id), position);
}

/// Check if a position exists in persistent storage.
pub fn has_position(env: &Env, id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Position(id))
}

/// Whitelist a currency token address.
pub fn set_whitelisted_currency(env: &Env, currency: &Address, whitelisted: bool) {
    if whitelisted {
        env.storage().persistent().set(&DataKey::WhitelistedCurrency(currency.clone()), &true);
    } else {
        env.storage().persistent().remove(&DataKey::WhitelistedCurrency(currency.clone()));
    }
}

/// Check if a currency is whitelisted for collateral.
pub fn is_currency_whitelisted(env: &Env, currency: &Address) -> bool {
    env.storage().persistent().has(&DataKey::WhitelistedCurrency(currency.clone()))
}
