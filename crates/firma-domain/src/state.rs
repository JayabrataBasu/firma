//! Firm-agent state (manual §8.1).
//!
//! §8.1 splits state into two kinds, and the split is load-bearing for §9.3:
//!
//! * **constraint-carrying** — [`FirmState`], `d = 4`. These four fields, and
//!   *only* these, may enter a constraint function `g_j` (§9.3: "The MVP state
//!   space MUST have `d ≤ 4` for kernel-carrying components. Additional state
//!   may exist but MUST NOT enter `g_j`.").
//! * **auxiliary** — [`FirmAuxState`]. Does not count toward `d ≤ 4`; feeds
//!   the decision procedure, shaping success, and goal evaluation, but never a
//!   `g_j`.
//!
//! Keeping them as distinct types makes that boundary a compile-time fact.

use serde::{Deserialize, Serialize};

/// Number of goals a firm satisfices against (manual §12.1: capital growth,
/// capability, obligation clearance).
pub const GOAL_COUNT: usize = 3;

/// §8.1 — the constraint-carrying firm-agent state, `x_{i,t} = (r^L, r^I, c, q)`,
/// `d = 4`.
///
/// `r^L` and `r^I` are also the run's conserved resources (§7.2 primitive 3,
/// §27.2 "3 types, integer, conserved"); in the running model they live in the
/// kernel's resource-keyed conserved ledger, and this struct is the domain-side
/// view of them plus the two non-conserved fields. `q` (outstanding obligation)
/// is a per-firm counter, **not** a conserved resource — it rises on `contract`
/// and falls on `deliver` (§11).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirmState {
    /// `r^L` — liquid capital. `i64`, domain `[0, R^L_max]` (§16.1). Zero ⇒
    /// insolvency (§9.1 `solvency`, `g_1 = -r^L`).
    pub liquid_capital: i64,
    /// `r^I` — input stock. `i64`, domain `[0, R^I_max]`, storage-capped.
    pub input_stock: i64,
    /// `c` — capability. `f64`, domain `[0, 1]`. Determines in-scope activities
    /// (§9.1 `scope`, `g_3 = θ_cap - c`). Monotone non-decreasing in the MVP
    /// (ADR 0018).
    pub capability: f64,
    /// `q` — outstanding (undelivered contracted) obligation. `i64`, domain
    /// `[0, Q_max]` (§16.1). §9.1 `obligation`, `g_4 = q - θ_Q`.
    pub obligation: i64,
}

impl FirmState {
    /// Constraint-carrying dimensionality (§8.1, §9.3). The exact viability
    /// kernel is tractable only while this holds `≤ 4`.
    pub const DIM: usize = 4;
}

/// §8.1 — auxiliary firm-agent state. Does **not** count toward `d ≤ 4` (§9.3)
/// and MUST NOT enter any `g_j`.
///
/// The structured auxiliary state of §8.1 — memory `M`, the lagged-effect queue
/// `Λ`, and the action window `W` — is deferred to Stage 1 (see the crate
/// docs); this Stage-0 type carries the scalar auxiliary fields the constraint
/// and goal layers need first.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirmAuxState {
    /// `λ` — legitimacy. `f64 ∈ [0, 1]`. Modulates constraint-shaping success
    /// (§11.3); decremented by `δ_λ` on a first `compliance` violation (§9.1).
    pub legitimacy: f64,
    /// `u` — regulated-activity intensity. `f64 ≥ 0`. A **derived** trailing
    /// statistic: the simple mean over the last `L_W` actions of the
    /// per-action regulated-production indicator (ADR 0014). §8.1 permits
    /// caching it here for efficiency, but any cached value MUST equal the
    /// value recomputed from the action window `W` bit-for-bit. It is
    /// recomputed in the `constrain` phase (§10.1 phase 7).
    pub regulated_intensity: f64,
    /// `A` — the three aspiration levels (§12.1).
    pub aspirations: Aspirations,
}

/// `A_{i,t}` — one aspiration level per goal (manual §12.1). `f64³`.
///
/// Adaptive: `A_{j,t+1} = A_{j,t} + α (v_{j,t} − A_{j,t})` (§12.1) — that update
/// is decision-layer behaviour (Stage 2), not defined here.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Aspirations {
    /// `A_1` — capital-growth aspiration. Realised value `v_1 = r^L_t − r^L_{t-1}`.
    pub capital_growth: f64,
    /// `A_2` — capability aspiration. Realised value `v_2 = c_t`.
    pub capability: f64,
    /// `A_3` — obligation-clearance aspiration. Realised value `v_3 = -q_t`.
    pub obligation_clearance: f64,
}

impl Aspirations {
    /// The three levels in goal order `[A_1, A_2, A_3]` (§12.1), for iteration.
    #[must_use]
    pub fn as_array(&self) -> [f64; GOAL_COUNT] {
        [
            self.capital_growth,
            self.capability,
            self.obligation_clearance,
        ]
    }

    /// Construct from `[A_1, A_2, A_3]`.
    #[must_use]
    pub fn from_array(a: [f64; GOAL_COUNT]) -> Aspirations {
        Aspirations {
            capital_growth: a[0],
            capability: a[1],
            obligation_clearance: a[2],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dim_is_four() {
        assert_eq!(FirmState::DIM, 4);
    }

    #[test]
    fn aspirations_array_roundtrip() {
        let a = Aspirations {
            capital_growth: 5.0,
            capability: 0.6,
            obligation_clearance: -20.0,
        };
        assert_eq!(Aspirations::from_array(a.as_array()), a);
        assert_eq!(a.as_array(), [5.0, 0.6, -20.0]);
    }

    #[test]
    fn firm_state_json_roundtrip() {
        let s = FirmState {
            liquid_capital: 40,
            input_stock: 12,
            capability: 0.55,
            obligation: 30,
        };
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<FirmState>(&j).unwrap(), s);
    }
}
