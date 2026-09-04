//! `firma-analysis` — offline, kernel-free reconstruction of simulation
//! state and metrics from an event log (manual §14, §22.2, §17 A5; ADR 0045,
//! ADR 0046).
//!
//! **No domain logic is invented here** — every formula (`standard_margin`,
//! the four `g_j`, `u_from_window`, repertoire entropy) is read from
//! `firma-domain`, the single source ADR 0021/0026 established. This crate's
//! only job is folding a log's `DeltaApplied` records back into the typed
//! state those formulas need (§17 A5: "the kernel writes an event log; every
//! metric is computed afterward").
//!
//! Promoted from `tests/src/replay.rs` (Stage 6's SC-1…6 harness) so the same
//! reconstruction serves three consumers without three implementations
//! (ADR 0046): `firma-conformance`'s tests (unchanged behaviour — see
//! [`sanity::sanity_from_run`]), `firma-tui`'s live panels (via
//! [`Reconstruction`]'s incremental `apply`), and `firma-py`'s `metrics`
//! binding (Stage 7 Part C).
//!
//! Dependency graph (ADR 0045): `firma-analysis ← firma-core, firma-domain,
//! firma-config, firma-io` — never `firma-kernel`, never a plugin. This is
//! what lets `firma-tui` depend on it without violating §23.2's "MUST NOT
//! link against the kernel."

#![forbid(unsafe_code)]

mod reconstruct;
pub mod sanity;

pub use reconstruct::{repertoire_entropy, FirmSnapshot, Notable, Reconstruction};
