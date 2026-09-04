//! The event stream the kernel emits (manual §22.1). The kernel builds
//! `Vec<Event>`; `firma-cli` writes them through `firma-io` (the kernel never
//! links `firma-io` — §18.1).
//!
//! Log sufficiency (§22.2, ADR 0005): every applied delta, birth, death, and
//! intervention is recorded, so any offline metric — and the conservation check
//! of VT-6 — can be reconstructed from the log alone.
//!
//! Lives in `firma-core`, not `firma-kernel` (ADR 0045): `Event` depends only
//! on other `firma-core` wire types ([`AgentId`], [`DeltaKind`],
//! [`DeltaTarget`], [`PluginId`]) and nothing kernel-internal, so an
//! offline/analysis crate that must stay kernel-free (`firma-analysis`,
//! `firma-tui` — §23.2's explicit "MUST NOT link against the kernel") can read
//! and construct `Event`s without depending on `firma-kernel`.
//! `firma-kernel` re-exports this type (`pub use firma_core::Event;`) so no
//! existing call site (`firma_kernel::Event`) changes.

use serde::{Deserialize, Serialize};

use crate::{AgentId, DeltaKind, DeltaTarget, PluginId};

/// One entry in the event log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// The first record of the log: identifies the engine and manual version
    /// the run was produced under.
    RunStarted {
        /// Manual version in force (§0.6).
        manual_version: String,
        /// Engine semantic version.
        engine_version: String,
        /// Total ticks the run will execute.
        horizon: u64,
    },
    /// A tick's phases are about to run.
    TickStarted {
        /// The tick.
        tick: u64,
    },
    /// An agent entered the population (born at this tick, per config).
    AgentBorn {
        /// The tick.
        tick: u64,
        /// The new agent.
        agent: AgentId,
    },
    /// An intervention was applied before this tick's phases.
    InterventionApplied {
        /// The tick.
        tick: u64,
        /// A short description of the `do(·)` operation (§6.5).
        op: String,
    },
    /// A resolved delta was applied to state (the workhorse record).
    DeltaApplied {
        /// The tick.
        tick: u64,
        /// The §10.1 phase number.
        phase: u8,
        /// The plugin that proposed the delta.
        origin: PluginId,
        /// What it acted on.
        target: DeltaTarget,
        /// The change.
        kind: DeltaKind,
    },
    /// A phase finished reconciling.
    PhaseCompleted {
        /// The tick.
        tick: u64,
        /// The §10.1 phase number.
        phase: u8,
        /// Number of resolved deltas applied in the phase.
        deltas_applied: u32,
    },
    /// An agent left the population. Id is never reused (§7.2).
    AgentDied {
        /// The tick.
        tick: u64,
        /// The departed agent.
        agent: AgentId,
        /// Why (Phase 1: only `"intervention:remove_agent"`).
        cause: String,
    },
    /// A tick completed; all invariants held (§19.5).
    TickCompleted {
        /// The tick.
        tick: u64,
        /// Live agent count at tick end.
        live_agents: u32,
    },
}
