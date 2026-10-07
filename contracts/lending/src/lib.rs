#![no_std]
#![allow(clippy::too_many_arguments)]

// ------------------------------------------------------------
// lib.rs — Afristore NFT Lending contract root
//
// This file wires together the sub-modules.
// Do NOT implement business logic here — each module has a
// dedicated issue in issues.md at the repository root.
//

pub mod types;

pub mod storage;

pub mod oracle;

mod interest;

mod events;

pub mod settlement;

mod contract;

#[cfg(test)]
mod test;
