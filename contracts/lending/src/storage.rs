//! Storage helpers for the Afristore Lending contract.
//!
//! This module provides typed read/write helpers for all on-chain storage,
//! mapping each `DataKey` variant to the correct storage class (Instance or Persistent)
//! and automatically extending TTL on persistent writes.

use soroban_sdk::{contracttype, Address, Env};
use crate::types::{PlatformConfig as Config, Listing, Position};

// ----------------------------------------------------------------------------
// DataKey Enum
// ----------------------------------------------------------------------------

/// Storage keys used by the lending contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Contract configuration (Instance storage).
    Config,
    /// Auto-incrementing counter for listing IDs (Instance storage).
    NextListingId,
    /// Auto-incrementing counter for position IDs (Instance storage).
    NextPositionId,
    /// A specific listing by ID (Persistent storage).
    Listing(u64),
    /// A specific position by ID (Persistent storage).
    Position(u64),
    /// Address of a whitelisted currency/token (Persistent storage).
    WhitelistedCurrency(Address),
}

// ----------------------------------------------------------------------------
// TTL Constants
// ----------------------------------------------------------------------------

/// Threshold in ledgers before we extend TTL (~14 days at 5s/ledger).
const TTL_THRESHOLD_LEDGERS: u32 = 14 * 17280;

/// Amount to extend TTL to (~30 days at 5s/ledger).
const TTL_EXTEND_LEDGERS: u32 = 30 * 17280;

// ----------------------------------------------------------------------------
// Helper: Extend TTL on Persistent Entries
// ----------------------------------------------------------------------------

/// Extends TTL for a persistent entry using the DataKey.
fn extend_persistent_ttl(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD_LEDGERS, TTL_EXTEND_LEDGERS);
}

// ----------------------------------------------------------------------------
// Config Helpers (Instance Storage)
// ----------------------------------------------------------------------------

/// Stores the contract configuration.
pub fn set_config(env: &Env, config: &Config) {
    env.storage().instance().set(&DataKey::Config, config);
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_LEDGERS);
}

/// Retrieves the contract configuration.
pub fn get_config(env: &Env) -> Option<Config> {
    env.storage().instance().get(&DataKey::Config)
}

/// Checks if the contract configuration exists.
pub fn has_config(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Config)
}

// ----------------------------------------------------------------------------
// NextListingId Helpers (Instance Storage)
// ----------------------------------------------------------------------------

/// Returns the next listing ID and increments the counter atomically.
pub fn next_listing_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextListingId)
        .unwrap_or(0);
    let next_id = id + 1;
    env.storage()
        .instance()
        .set(&DataKey::NextListingId, &next_id);
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_LEDGERS);
    next_id
}

/// Gets the current next listing ID without incrementing.
pub fn get_next_listing_id(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextListingId)
        .unwrap_or(0)
}

// ----------------------------------------------------------------------------
// NextPositionId Helpers (Instance Storage)
// ----------------------------------------------------------------------------

/// Returns the next position ID and increments the counter atomically.
pub fn next_position_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextPositionId)
        .unwrap_or(0);
    let next_id = id + 1;
    env.storage()
        .instance()
        .set(&DataKey::NextPositionId, &next_id);
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_LEDGERS);
    next_id
}

/// Gets the current next position ID without incrementing.
pub fn get_next_position_id(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextPositionId)
        .unwrap_or(0)
}

// ----------------------------------------------------------------------------
// Listing Helpers (Persistent Storage)
// ----------------------------------------------------------------------------

/// Stores a listing by ID.
pub fn set_listing(env: &Env, listing_id: u64, listing: &Listing) {
    let key = DataKey::Listing(listing_id);
    env.storage().persistent().set(&key, listing);
    extend_persistent_ttl(env, &key);
}

/// Retrieves a listing by ID.
pub fn get_listing(env: &Env, listing_id: u64) -> Option<Listing> {
    let key = DataKey::Listing(listing_id);
    let listing: Option<Listing> = env.storage().persistent().get(&key);
    if listing.is_some() {
        extend_persistent_ttl(env, &key);
    }
    listing
}

/// Checks if a listing exists by ID.
pub fn has_listing(env: &Env, listing_id: u64) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Listing(listing_id))
}

// ----------------------------------------------------------------------------
// Position Helpers (Persistent Storage)
// ----------------------------------------------------------------------------

/// Stores a position by ID.
pub fn set_position(env: &Env, position_id: u64, position: &Position) {
    let key = DataKey::Position(position_id);
    env.storage().persistent().set(&key, position);
    extend_persistent_ttl(env, &key);
}

/// Retrieves a position by ID.
pub fn get_position(env: &Env, position_id: u64) -> Option<Position> {
    let key = DataKey::Position(position_id);
    let position: Option<Position> = env.storage().persistent().get(&key);
    if position.is_some() {
        extend_persistent_ttl(env, &key);
    }
    position
}

/// Checks if a position exists by ID.
pub fn has_position(env: &Env, position_id: u64) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Position(position_id))
}

// ----------------------------------------------------------------------------
// Whitelisted Currency Helpers (Persistent Storage)
// ----------------------------------------------------------------------------

/// Adds a currency to the whitelist.
pub fn set_whitelisted_currency(env: &Env, currency: &Address) {
    let key = DataKey::WhitelistedCurrency(currency.clone());
    env.storage().persistent().set(&key, &true);
    extend_persistent_ttl(env, &key);
}

/// Removes a currency from the whitelist.
pub fn remove_whitelisted_currency(env: &Env, currency: &Address) {
    let key = DataKey::WhitelistedCurrency(currency.clone());
    env.storage().persistent().remove(&key);
}

/// Checks if a currency is whitelisted.
pub fn is_currency_whitelisted(env: &Env, currency: &Address) -> bool {
    let key = DataKey::WhitelistedCurrency(currency.clone());
    env.storage().persistent().get(&key).unwrap_or(false)
}
