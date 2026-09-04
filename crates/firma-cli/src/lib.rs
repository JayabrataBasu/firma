//! `firma_cli` — orchestration for the FIRMA engine, and the library the
//! `firma` binary and the conformance suite both drive.
//!
//! The kernel (`firma-kernel`) only moves state; it does not resolve plugins,
//! build manifests, or write files (§18.1, §19.1). This crate is where those
//! responsibilities live:
//!
//! * [`standard_registry`] — the compile-time [`Registry`] of Phase 1 plugins
//!   (the testkit rules and the additive resolver, ADR 0008).
//! * [`execute_run`] — config → resolved schedule → kernel loop → event log,
//!   snapshots, and manifest on disk (manual §22).
//! * [`replay_run`] — re-execute from a run directory's manifest and confirm the
//!   event log is byte-identical (the DT-1 check, §25.2).
//! * [`verify_run`] — recompute the run id, replay, and confirm every stored
//!   snapshot matches the re-executed state (§18.2 firma-io tests, DT-5).

#![forbid(unsafe_code)]

mod constraints;
mod error;
mod orchestrator;
mod registry_setup;

pub use constraints::standard_constraints;
pub use error::CliError;
pub use orchestrator::{
    execute_run, prepare_run, replay_run, verify_run, ReplayReport, RunOptions, RunReport,
    VerifyReport,
};
pub use registry_setup::{locality_catalogue, model_registry, standard_registry};

pub use firma_registry::Registry;

/// The engine semantic version (from the workspace `Cargo.toml`), recorded in
/// every manifest (manual §22.3).
#[must_use]
pub fn engine_version() -> semver::Version {
    env!("CARGO_PKG_VERSION")
        .parse()
        .expect("crate version is valid semver")
}

/// Identifier of the RNG scheme, recorded in the manifest identity (manual
/// §21.2). The field encoding is pinned per `PROGRESS.md` OQ-1.
pub const RNG_SCHEME: &str =
    "philox4x32-10; SHA-256 hierarchical keys; fixed-width big-endian field encoding (OQ-1)";
