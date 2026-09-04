//! RNG key material and the phase / stream enumerations (manual §10.1, §21.2–21.3).
//!
//! [`RngKey`] carries the six tuple fields the kernel knows when it invokes a
//! rule. The rule folds in a `purpose_tag` and opens a counter-based stream via
//! `firma-rng` (which owns the SHA-256). Splitting it this way keeps this crate
//! free of any hashing or generator behaviour (§18.2).

use serde::{Deserialize, Serialize};

/// One of the four independent, separately seeded RNG streams (manual §21.3).
/// Discriminants are stable and part of the key derivation (§15.5 uses
/// `mechanism` = 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum StreamId {
    /// Decisions, action outcomes, shaping success.
    Mechanism = 0,
    /// Resource dynamics, exogenous variation.
    Environment = 1,
    /// Shock timing and magnitude.
    Shock = 2,
    /// Initial conditions.
    Init = 3,
}

impl StreamId {
    /// The stable byte used in the key tuple.
    #[must_use]
    pub const fn as_byte(self) -> u8 {
        self as u8
    }
}

/// The nine simulation phases, in the fixed order of manual §10.1. The
/// discriminant is the §10.1 phase number (1-based); §15.5 uses `decide` = 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Phase {
    /// 1 — prices update; scheduled shocks fire.
    Environment = 1,
    /// 2 — agents build observations.
    Observe = 2,
    /// 3 — decision procedure selects one action per firm.
    Decide = 3,
    /// 4 — market actions emit deltas.
    ActMarket = 4,
    /// 5 — shaping actions emit deltas, enqueue lagged effects.
    ActShaping = 5,
    /// 6 — effects with `maturity_tick == t` fire.
    ResolveLagged = 6,
    /// 7 — constraint parameters update; `u` and `h` recomputed.
    Constrain = 7,
    /// 8 — violations processed; deaths recorded.
    Enforce = 8,
    /// 9 — aspirations update; events flushed.
    Record = 9,
}

impl Phase {
    /// The stable byte used in the key tuple (the §10.1 phase number).
    #[must_use]
    pub const fn as_byte(self) -> u8 {
        self as u8
    }
}

/// The nine phases in execution order (manual §10.1). The scheduler iterates
/// exactly this slice, every tick, forever (§19.3).
pub const PHASE_ORDER: [Phase; 9] = [
    Phase::Environment,
    Phase::Observe,
    Phase::Decide,
    Phase::ActMarket,
    Phase::ActShaping,
    Phase::ResolveLagged,
    Phase::Constrain,
    Phase::Enforce,
    Phase::Record,
];

/// The six tuple fields of the RNG key known to the kernel at rule-invocation
/// time (manual §21.2):
///
/// ```text
/// key = H(run_seed ‖ stream_id ‖ plugin_id ‖ phase_id ‖ tick ‖ agent_id ‖ purpose_tag)
/// ```
///
/// The `purpose_tag` is supplied by the rule at each draw site (via
/// `firma_rng::open`), not stored here, because a rule may draw for several
/// distinct purposes within one `apply` call and each must be an independent
/// stream (§21.2, ADR 0003).
///
/// `agent_id` is `None` for phase-global draws (e.g. an environment rule that
/// draws once per tick, not once per agent).
///
/// The exact byte encoding is pinned in `firma-rng` and recorded as open
/// question OQ-1 in `PROGRESS.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngKey {
    /// The run seed for `stream`.
    pub run_seed: u64,
    /// Which of the four streams (§21.3).
    pub stream: StreamId,
    /// `PluginId::numeric()` of the rule that will draw.
    pub plugin_id: u64,
    /// The phase in which the draw happens.
    pub phase: Phase,
    /// The current tick.
    pub tick: u64,
    /// The agent the draw pertains to, or `None` for a phase-global draw.
    pub agent_id: Option<u64>,
}
