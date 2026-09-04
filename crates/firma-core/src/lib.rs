//! `firma-core` — the shared vocabulary of FIRMA.
//!
//! Per manual §18.2 this crate holds **types only, no behaviour**: `AgentId`,
//! `Tick`, `ResourceKind`, `Delta`, `View`, `RngKey`, `PluginId`, the plugin
//! traits, `Event` (ADR 0045 — moved from `firma-kernel`, which re-exports it,
//! so kernel-free analysis crates can read the event log), and the error
//! enums. It depends on nothing else in the workspace (§18.1).
//!
//! What lives here is deliberately small. The rule-purity contract of §17 (A2)
//! is expressed by the [`Rule`] trait: a rule receives a read-only [`View`] and
//! an [`RngKey`] and returns `Vec<Delta>` — it cannot reach the state store, so
//! "rules propose; only the reconciler mutates" is a type-level fact, not a
//! convention.
//!
//! Determinism notes that touch every consumer:
//! * every map that can influence a result is a `BTreeMap` (§19.2 / §21.1 D1);
//! * conserved resource quantities are `i64` (§21.4 / ADR 0004);
//! * [`Delta`] has a total order matching §19.4 step 2.

#![forbid(unsafe_code)]

mod delta;
mod error;
mod event;
mod ids;
mod intervention;
mod rng_key;
mod rule;
mod view;

pub use delta::{ConflictClass, Delta, DeltaKind, DeltaKindTag, DeltaTarget};
pub use error::{ConfigError, CoreError, RegistryError};
pub use event::Event;
pub use ids::{AgentId, ComponentId, PluginId, ResourceKind, Tick};
pub use intervention::{Intervention, PluginRef};
pub use rng_key::{Phase, RngKey, StreamId, PHASE_ORDER};
pub use rule::{ConflictResolver, Rule};
pub use view::View;

/// The manual version this engine build implements (manual §0.6, §22.3 — the
/// manual version in force MUST be recorded in every run manifest).
pub const MANUAL_VERSION: &str = "1.0.0";

/// Canonical JSON serialisation used wherever bytes must be reproducible
/// (content hashing, the event log, snapshots — ADR 0012).
///
/// `serde_json` emits struct fields in declaration order and `BTreeMap` in
/// sorted-key order, both deterministic. Callers MUST only pass types that obey
/// the "`BTreeMap` everywhere" rule.
///
/// # Errors
/// Propagates any `serde_json` failure (e.g. a map with non-string keys).
pub fn to_canonical_json<T: serde::Serialize>(value: &T) -> Result<String, CoreError> {
    serde_json::to_string(value).map_err(|e| CoreError::Serialize(e.to_string()))
}

/// Pretty canonical JSON — same guarantees as [`to_canonical_json`], used for
/// human-facing artefacts (the manifest, snapshots).
///
/// # Errors
/// Propagates any `serde_json` failure.
pub fn to_pretty_json<T: serde::Serialize>(value: &T) -> Result<String, CoreError> {
    serde_json::to_string_pretty(value).map_err(|e| CoreError::Serialize(e.to_string()))
}
