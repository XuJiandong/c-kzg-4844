#![cfg_attr(not(feature = "std"), no_std)]

#[cfg_attr(not(feature = "verification-only"), macro_use)]
extern crate alloc;

// This `extern crate` invocation tells `rustc` that we actually need the symbols from `blst`.
// Without it, the compiler won't link to `blst` when compiling this crate.
// See: https://kornel.ski/rust-sys-crate#linking
#[cfg(not(target_arch = "riscv64"))]
extern crate blst;

#[cfg(not(feature = "verification-only"))]
mod bindings;

#[cfg(feature = "verification-only")]
mod verification;
#[cfg(feature = "verification-only")]
pub use verification::*;
#[cfg(all(feature = "verification-only", target_arch = "riscv64"))]
mod ckb_alloc;

#[cfg(all(feature = "ethereum_kzg_settings", not(feature = "verification-only")))]
mod ethereum_kzg_settings;

// Expose relevant types with idiomatic names.
#[cfg(not(feature = "verification-only"))]
pub use bindings::{
    KZGCommitment as KzgCommitment, KZGProof as KzgProof, KZGSettings as KzgSettings,
    C_KZG_RET as CkzgError,
};

// Expose the default settings.
#[cfg(all(feature = "ethereum_kzg_settings", not(feature = "verification-only")))]
pub use ethereum_kzg_settings::{ethereum_kzg_settings, ethereum_kzg_settings_arc};

// Expose the constants.
#[cfg(not(feature = "verification-only"))]
pub use bindings::{
    BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT, BYTES_PER_PROOF,
    CELLS_PER_EXT_BLOB, FIELD_ELEMENTS_PER_BLOB, FIELD_ELEMENTS_PER_CELL,
};
// Expose the remaining relevant types.
#[cfg(not(feature = "verification-only"))]
pub use bindings::{Blob, Bytes32, Bytes48, Cell, Error};
