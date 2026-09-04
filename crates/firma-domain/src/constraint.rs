//! The `Constraint` plugin interface (manual §9.1, §9.2; ADR 0021).
//!
//! Split in two on purpose (ADR 0021 decision 3):
//!
//! * [`MarginTerm`] — `g_j` and `s_j`, the *only* thing `firma-viability`'s
//!   `margin` / `kernel` touch (§9.2's `h` formula uses nothing else).
//! * [`Constraint`] — the full plugin: id, version, [`ViolationSemantic`], and
//!   `assumption()`. The violation semantics are an `enforce`-phase /
//!   `Admissibility`-service concern; `firma-viability` never sees them, which
//!   keeps that crate free of death/graduated/relational knowledge (ADR 0020).

use serde::{Deserialize, Serialize};

use firma_core::PluginId;

use crate::params::ConstraintParams;
use crate::state::{FirmAuxState, FirmState};

/// Everything a constraint needs to evaluate `g_j` (manual §9.1).
///
/// `compliance` reads `aux.regulated_intensity` (`u`); the other three read
/// [`state`](Self::state) and [`theta`](Self::theta).
#[derive(Debug, Clone, Copy)]
pub struct ConstraintContext<'a> {
    /// §8.1 constraint-carrying state `(r^L, r^I, c, q)`.
    pub state: &'a FirmState,
    /// §8.1 auxiliary state (`λ`, `u`, aspirations).
    pub aux: &'a FirmAuxState,
    /// §8.2 constraint parameters θ — global (ADR 0015).
    pub theta: &'a ConstraintParams,
}

/// The narrow interface `firma-viability` depends on: the constraint function
/// `g_j(x, θ)` and its scale factor `s_j` (manual §9.1, §9.2).
///
/// `g_j ≤ 0` ⇒ the constraint is satisfied at that state.
pub trait MarginTerm: Send + Sync {
    /// `g_j(x, θ)` (§9.1). Real-valued.
    fn g(&self, ctx: &ConstraintContext<'_>) -> f64;
    /// `s_j` — the §9.2 scale factor. MUST be `> 0`.
    fn scale(&self) -> f64;
}

/// What happens when a constraint is violated (manual §9.1). A dispatch tag for
/// the Stage-2 `enforce` phase (§10.1 phase 8) and the `Admissibility` service
/// (§11.4) — **never** consulted by `firma-viability`.
///
/// The four semantics are deliberately different (§9.1 `[D]`): "Making all
/// lethal would collapse the model to a single survival constraint and destroy
/// the distinction between kinds of pressure."
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ViolationSemantic {
    /// §9.1 `solvency`: the agent is removed at the end of `enforce`.
    Death,
    /// §9.1 `compliance`: first violation → penalty `penalty` and
    /// `λ −= legitimacy_loss`; a *second* violation within `window_ticks` →
    /// death.
    Graduated {
        /// `P_c` (§16.1 default 30).
        penalty: i64,
        /// `T_c` (§16.1 default 4).
        window_ticks: u64,
        /// `δ_λ` (§16.1 default 0.15).
        legitimacy_loss: f64,
    },
    /// §9.1 `scope`: never lethal. The constraint only gates action
    /// admissibility (§11.4) — `enforce` does nothing with it. The
    /// `Admissibility` service calls this constraint's `g` directly when
    /// deciding whether a scope-gated action (`produce_regulated`) is
    /// admissible (ADR 0021 decision 2).
    AdmissibilityGate,
    /// §9.1 `obligation`: all of the firm's `supply` edges are severed and
    /// `penalty` (`P_q`) is applied. Never lethal. The edge-severance handler
    /// lands in Stage 2 with the relation graph and the shaping actions that
    /// create `supply` edges (ADR 0021).
    Relational {
        /// `P_q` — **not** given in §16.1 (a manual gap); the `obligation`
        /// plugin takes it as a required parameter.
        penalty: i64,
    },
}

impl ViolationSemantic {
    /// Whether a chronic violation of this constraint eventually terminates the
    /// firm — i.e. whether the constraint bounds the *viability kernel*
    /// `K(θ)` (manual §9.3), as opposed to only gating an action (`scope`) or
    /// severing relationships (`obligation`).
    ///
    /// `firma-viability` never calls this (ADR 0021 decision 3): the *caller*
    /// of `firma_kernel`, holding the full [`Constraint`] objects, uses it to
    /// decide which subset of `g_j` defines `K^(0)`.
    ///
    /// * `Death` — yes (immediate).
    /// * `Graduated` — yes: a state that chronically violates `compliance`
    ///   accrues repeated penalties and, on the "second within `T_c`", dies.
    /// * `AdmissibilityGate` (`scope`) — no: "Never lethal" (§9.1).
    /// * `Relational` (`obligation`) — no: "Never lethal" (§9.1).
    #[must_use]
    pub fn bounds_viability_kernel(&self) -> bool {
        matches!(
            self,
            ViolationSemantic::Death | ViolationSemantic::Graduated { .. }
        )
    }
}

/// A constraint plugin (manual §20.3 category `Constraint`). One implementation
/// per row of §9.1's table.
pub trait Constraint: MarginTerm {
    /// Stable id, `"constraint.<name>"` (§24.2).
    fn id(&self) -> PluginId;
    /// This build's semantic version (§20.5).
    fn version(&self) -> semver::Version;
    /// The §9.1 violation semantic for this constraint.
    fn violation(&self) -> ViolationSemantic;
    /// Plain-language statement of `g_j` (§20.2 — MUST NOT be empty; §25.6
    /// `assumption-nonempty`).
    fn assumption(&self) -> &str;
}
