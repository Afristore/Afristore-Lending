//! Typed Soroban event emitters for lending contract state transitions.
//!
//! Each helper publishes a two-topic event (`"lending"`, action) so indexers and
//! keepers can react without polling contract storage.
#![allow(dead_code)]

use soroban_sdk::{contracttype, Address, Env, Symbol};

// ---------------------------------------------------------------------------
// Event data payloads (#[contracttype] for stable XDR encoding)
// ---------------------------------------------------------------------------

/// Payload for `create_listing` / `listing_created`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListingCreatedEvent {
    pub listing_id: u64,
    pub lender: Address,
    pub nft_contract: Address,
    pub token_id: u128,
    pub declared_price_usd: i128,
}

/// Payload for `cancel_listing` / `listing_cancelled`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListingCancelledEvent {
    pub listing_id: u64,
    pub lender: Address,
}

/// Payload for `borrow` / `position_opened`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionOpenedEvent {
    pub position_id: u64,
    pub listing_id: u64,
    pub borrower: Address,
    pub collateral_amount: i128,
}

/// Payload for `add_collateral` / `collateral_added`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollateralAddedEvent {
    pub position_id: u64,
    pub borrower: Address,
    pub amount: i128,
    pub new_total: i128,
}

/// Payload for `return_nft` / `position_returned`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionReturnedEvent {
    pub position_id: u64,
    pub interest_paid: i128,
    pub platform_fee: i128,
    pub borrower_refund: i128,
}

/// Payload for `liquidate` / `position_liquidated`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionLiquidatedEvent {
    pub position_id: u64,
    pub liquidator: Address,
    pub lender_payout: i128,
    pub liquidator_bounty: i128,
    pub borrower_refund: i128,
}

// ---------------------------------------------------------------------------
// Emit helpers
// ---------------------------------------------------------------------------

fn topic_lending(env: &Env) -> Symbol {
    Symbol::new(env, "lending")
}

/// Emit when a new NFT lending listing is created.
pub fn emit_listing_created(env: &Env, data: ListingCreatedEvent) {
    env.events()
        .publish((topic_lending(env), Symbol::new(env, "listing_created")), data);
}

/// Emit when a lender cancels an open listing.
pub fn emit_listing_cancelled(env: &Env, data: ListingCancelledEvent) {
    env.events().publish(
        (topic_lending(env), Symbol::new(env, "listing_cancelled")),
        data,
    );
}

/// Emit when a borrower opens a position against a listing.
pub fn emit_position_opened(env: &Env, data: PositionOpenedEvent) {
    env.events().publish(
        (topic_lending(env), Symbol::new(env, "position_opened")),
        data,
    );
}

/// Emit when a borrower adds collateral to an active position.
pub fn emit_collateral_added(env: &Env, data: CollateralAddedEvent) {
    env.events().publish(
        (topic_lending(env), Symbol::new(env, "collateral_added")),
        data,
    );
}

/// Emit when a position is returned (loan repaid, NFT released).
pub fn emit_position_returned(env: &Env, data: PositionReturnedEvent) {
    env.events().publish(
        (topic_lending(env), Symbol::new(env, "position_returned")),
        data,
    );
}

/// Emit when a position is liquidated.
pub fn emit_position_liquidated(env: &Env, data: PositionLiquidatedEvent) {
    env.events().publish(
        (topic_lending(env), Symbol::new(env, "position_liquidated")),
        data,
    );
}
