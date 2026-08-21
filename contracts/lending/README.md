# `contracts/lending`

Soroban NFT lending contract for the Afristore protocol.

## Module Layout

```text
src/
├── lib.rs          # Contract entry-point declarations  ← you are here
├── contract.rs     # All #[contractimpl] functions      (Issues L-7 → L-16)
├── types.rs        # Listing, Position, PlatformConfig  (Issue L-1)
├── storage.rs      # DataKey + read/write helpers       (Issue L-2)
├── oracle.rs       # Reflector client + staleness guard (Issue L-3)
├── interest.rs     # Per-month accrual math             (Issue L-4)
├── settlement.rs   # Shared settle() waterfall          (Issue L-5)
├── events.rs       # Typed event emitters               (Issue L-6)
└── test.rs         # Unit & integration tests           (Issues L-17, L-21)
```

## Getting Started (once the contract is implemented)

```bash
# Build WASM
cargo build --target wasm32-unknown-unknown --release

# Run tests
cargo test --features testutils

# Lint
cargo fmt --check
cargo clippy -- -D warnings

# Optimize
stellar contract optimize --wasm target/wasm32-unknown-unknown/release/lending.wasm

# Deploy to testnet
stellar contract deploy --wasm target/wasm32-unknown-unknown/release/lending.optimized.wasm --network testnet
```

## Contributing

See [`issues.md`](../../issues.md) at the repo root for the full list of independently
claimable issues. Issues marked `good-first-issue` are safe entry points — start with
**L-1** (types) and **L-2** (storage) as they unblock everything else.

All PRs must pass:
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test --features testutils`
- `cargo build --target wasm32-unknown-unknown --release`
