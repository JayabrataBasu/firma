//! `firma-domain` — the shared domain vocabulary of FIRMA's model layer.
//!
//! **Types plus the shared deterministic core** (ADR 0020, ADR 0021, ADR 0023).
//! This crate defines the data shapes of manual §8.1 (firm-agent state), §8.2
//! (constraint parameters θ), §9.2 (scale factors `s_j`), §16.1 (state bounds),
//! §8.3 ([`RelationGraph`]), and §8.1's `Λ` queue ([`Effect`], [`LaggedRecord`]);
//! plus the single copy of the §11 transition formulas ([`dynamics`]) and the
//! §11.2 shaping success / lag model ([`shaping`]), so no formula is written
//! twice. Every Stage 1+ domain crate — the constraint plugins, the decision
//! plugin, the action plugins, and `firma-viability`'s `margin` layer — reads
//! these; the kernel never does. [`keys`] holds the canonical string names for
//! the kernel's opaque domain-state store (ADR 0022).
//!
//! # Why a separate crate, not `firma-core` (ADR 0020)
//!
//! `firma-core` is pure substrate: `firma-kernel` depends on it (§18.1), so
//! anything `pub` in `firma-core` is in the kernel's dependency surface.
//! Putting `FirmState` / `Capability` / `ConstraintParams` there would put
//! firm-shaped domain types in the kernel's reachable graph, violating the
//! §17 A1 / §7.3 principle "there is no `Firm` type in the kernel" — a
//! violation the name-matching `no-kernel-domain-deps` lint (§25.6) would not
//! catch. `firma-domain` is a sibling of `firma-core` (`← firma-core` only) so
//! that the kernel *cannot* reach it.
//!
//! # Still deferred
//!
//! Memory `M` (ring buffer) and the action window `W` land with the decision
//! component that owns them (Stage 3). Environment state (§8.3: prices `π^I`,
//! `π^O` beyond [`dynamics::EnvParams`], active shocks `Σ_t`) is a
//! shock/environment-plugin concern (Stage 4+). Relation-edge *severance* and
//! realised-inflow dependence weights (§13.1) are later work — [`RelationGraph`]
//! carries only the queries `contract` / `diversify` need (ADR 0023).

#![forbid(unsafe_code)]

mod constraint;
pub mod dynamics;
mod effect;
mod env;
pub mod keys;
pub mod margin;
mod params;
mod relation;
pub mod shaping;
pub mod shock;
mod state;
mod window;

pub use constraint::{Constraint, ConstraintContext, MarginTerm, ViolationSemantic};
pub use effect::{Effect, LaggedRecord};
pub use env::EnvSnapshot;
pub use params::{ConstraintParams, ScaleFactors, StateBounds};
pub use relation::{Edge, EdgeKind, RelationGraph};
pub use shock::{
    Persistence, Ramp, RegulatoryTarget, Shock, ShockChannel, ShockObservability, ShockTargets,
};
pub use state::{Aspirations, FirmAuxState, FirmState, GOAL_COUNT};
pub use window::{advance_window, WindowEntry};

/// Shared identifier vocabulary, re-exported from `firma-core` (§5) so domain
/// crates have a single import surface. Stage 1's constraint-instance,
/// relation-edge, and lagged-effect types are keyed by these.
pub use firma_core::{AgentId, PluginId, ResourceKind, Tick};
