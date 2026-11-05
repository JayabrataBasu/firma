//! The deterministic core of §11 (manual §11.1; ADR 0021 decision 4).
//!
//! These pure functions are the single source of the §11 transition formulas.
//! `firma-viability`'s Stage-1 FIRMA `Dynamics` calls them for the exact-kernel
//! computation; Stage 2's `action.market.standard` `Rule` calls the *same*
//! functions and layers cost accounting, the drawn lag, and success probability
//! (§11.2–11.3) on top. Nothing is written twice.
//!
//! ## §9.3 approximation (recorded per §9.3's requirement)
//!
//! When these feed the viability-kernel computation, the transition `f` is "the
//! deterministic core of §11 with stochastic terms at expectation — an
//! approximation that MUST be recorded as such". Concretely for [`market_core`]:
//!
//! * `invest_capability`'s lag (`Δ_cap` ticks, §11.1) is collapsed to an
//!   immediate `c += δ_c`. The lagged effect still arrives, so it does not
//!   change *whether* a state can survive indefinitely.
//! * `u` tracking (§8.1: `produce_regulated` "contributes to `u`") is a
//!   caller concern (the action window `W` / [`FirmAuxState`](crate::FirmAuxState)),
//!   not part of this `FirmState → FirmState` core.

use serde::{Deserialize, Serialize};

use crate::constraint::ConstraintContext;
use crate::effect::LaggedRecord;
use crate::margin::standard_margin;
use crate::params::{ConstraintParams, ScaleFactors};
use crate::state::{Aspirations, FirmAuxState, FirmState};
use crate::window::{advance_window, WindowEntry};

/// §8.3 environment prices `(π^I, π^O)`. `i64` — `r^L` is `i64` and
/// `r^L += π^O · y_O(c)` must be exact. No §16.1 defaults exist, so no `Default`
/// impl — configuration supplies them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvParams {
    /// `π^I` — input price (also `acquire_input`'s cost, §11.1).
    pub input_price: i64,
    /// `π^O` — output price.
    pub output_price: i64,
}

/// The §11.1 market-action parameters the deterministic core needs. Defaults are
/// the §16.1 "fixed" values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionParams {
    /// `y_0` — base yield. §16.1 default `4`.
    pub base_yield: i64,
    /// `η` — capability yield elasticity. §16.1 default `0.8`.
    pub capability_elasticity: f64,
    /// `γ_R` — regulated yield premium (`> 0`). §16.1 default `0.5`.
    pub regulated_premium: f64,
    /// `R^I_max` — input storage cap. §16.1 default `40`.
    pub input_stock_max: i64,
    /// `δ_c` — capability step per `invest_capability`. §16.1 default `0.05`.
    pub capability_step: f64,
    /// `κ_c` — `invest_capability` cost. §16.1 default `20`.
    pub capability_cost: i64,
}

impl Default for ActionParams {
    /// The §16.1 "fixed" defaults.
    fn default() -> Self {
        ActionParams {
            base_yield: 4,
            capability_elasticity: 0.8,
            regulated_premium: 0.5,
            input_stock_max: 40,
            capability_step: 0.05,
            capability_cost: 20,
        }
    }
}

/// `y_O(c) = ⌊y_0 (1 + η c)⌋` (manual §11.1).
#[must_use]
pub fn y_o(c: f64, p: &ActionParams) -> i64 {
    (f64::from(i32::try_from(p.base_yield).unwrap_or(i32::MAX))
        * (1.0 + p.capability_elasticity * c))
        .floor() as i64
}

/// `y_R(c) = ⌊y_0 (1 + η c)(1 + γ_R)⌋` (manual §11.1). Regulated production
/// yields more (`γ_R > 0`) but consumes compliance headroom — the model's core
/// operating tension (§11.1).
#[must_use]
pub fn y_r(c: f64, p: &ActionParams) -> i64 {
    (f64::from(i32::try_from(p.base_yield).unwrap_or(i32::MAX))
        * (1.0 + p.capability_elasticity * c)
        * (1.0 + p.regulated_premium))
        .floor() as i64
}

/// The six §11.1 market actions, in the canonical (normative) order — the order
/// that defines scan order (§12.4) and RNG `purpose_tag` indices (§11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum MarketAction {
    /// 0 — `hold`.
    Hold = 0,
    /// 1 — `produce_ordinary`.
    ProduceOrdinary = 1,
    /// 2 — `produce_regulated`.
    ProduceRegulated = 2,
    /// 3 — `acquire_input`.
    AcquireInput = 3,
    /// 4 — `invest_capability`.
    InvestCapability = 4,
    /// 5 — `deliver`.
    Deliver = 5,
}

/// The six market actions in canonical order.
pub const MARKET_ACTIONS: [MarketAction; 6] = [
    MarketAction::Hold,
    MarketAction::ProduceOrdinary,
    MarketAction::ProduceRegulated,
    MarketAction::AcquireInput,
    MarketAction::InvestCapability,
    MarketAction::Deliver,
];

impl MarketAction {
    /// The §11 canonical index.
    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// Whether `produce_regulated` — the one action gated by `scope` (§11.1,
    /// §11.4; ADR 0021 decision 2).
    #[must_use]
    pub const fn is_scope_gated(self) -> bool {
        matches!(self, MarketAction::ProduceRegulated)
    }
}

/// Deterministic core of one §11.1 market action. Returns the post-state, or
/// `None` if the §11.1 precondition is not met at `s`.
///
/// See the module docs for the §9.3 approximations this embodies when used for
/// kernel computation.
#[must_use]
pub fn market_core(
    action: MarketAction,
    s: &FirmState,
    theta: &ConstraintParams,
    env: &EnvParams,
    p: &ActionParams,
) -> Option<FirmState> {
    let mut n = *s;
    match action {
        MarketAction::Hold => {}
        MarketAction::ProduceOrdinary => {
            if s.input_stock < 1 {
                return None;
            }
            n.input_stock -= 1;
            n.liquid_capital += env.output_price * y_o(s.capability, p);
        }
        MarketAction::ProduceRegulated => {
            // §11.1 precondition: r^I ≥ 1 and c ≥ θ_cap (the `scope` gate).
            if s.input_stock < 1 || s.capability < theta.theta_cap {
                return None;
            }
            n.input_stock -= 1;
            n.liquid_capital += env.output_price * y_r(s.capability, p);
        }
        MarketAction::AcquireInput => {
            if s.liquid_capital < env.input_price || s.input_stock >= p.input_stock_max {
                return None;
            }
            n.liquid_capital -= env.input_price;
            n.input_stock += 1;
        }
        MarketAction::InvestCapability => {
            if s.liquid_capital < p.capability_cost {
                return None;
            }
            // §9.3 approximation: lag collapsed to immediate. Clamp to [0, 1]
            // (§8.1 domain; ADR 0018 — no decay, hard ceiling at 1).
            n.liquid_capital -= p.capability_cost;
            n.capability = (s.capability + p.capability_step).min(1.0);
        }
        MarketAction::Deliver => {
            if s.obligation < 1 || s.input_stock < 1 {
                return None;
            }
            n.obligation -= 1;
            n.input_stock -= 1;
        }
    }
    Some(n)
}

/// The one-step deterministic core of a §11.2 constraint-shaping action for
/// decision **lookahead** (§12.3 Step 5, `satisfices`).
///
/// Only the certain, immediate cost `r^L −= spend` is felt at `t + 1`. The
/// θ / edge / obligation payoff is lagged (`≥ 1` tick, §11.3 property 2) and
/// probabilistic (`p_max < 1`, property 3), so it is **invisible to a one-step
/// lookahead** — "the firm does not simulate; it applies a one-step lookahead"
/// (§12.3). A firm evaluating a shaping action for a positive shortfall
/// therefore sees only its cost and (correctly, per the one-step horizon)
/// never finds it satisficing; shaping is selected only via the Step-5 fallback
/// or `decision.random`. This is a faithful consequence of §12.3, and an SC-4
/// watch item (§16.2) for the full model.
///
/// Returns `None` if `s` cannot afford `spend` (the §11.2 affordability
/// precondition).
#[must_use]
pub fn shaping_cost_step(spend: i64, s: &FirmState) -> Option<FirmState> {
    if spend < 0 || s.liquid_capital < spend {
        return None;
    }
    Some(FirmState {
        liquid_capital: s.liquid_capital - spend,
        ..*s
    })
}

/// Cap for [`time_to_boundary`]'s forward projection, in ticks. Comfortably
/// past §16.1's widest shaping-lag sweep (`Δ_max = 16`), so no realistic
/// lag-vs-boundary comparison this cap feeds is ever truncated by it; an
/// eighth of the §16.1 default run horizon (`T = 400`), which keeps the
/// projection "far enough away to stop mattering for a same-tick decision"
/// without simulating anywhere near the rest of the run.
pub const MAX_PROJECTION_TICKS: u64 = 50;

/// §14.3 `time_to_boundary` — "Ticks until `h ≤ 0` under continued current
/// action... Forward projection" (manual). Repeatedly applies [`market_core`]
/// — the *same* deterministic core every other lookahead in this codebase
/// reuses (ADR 0021 Decision 4) — with a **fixed** `action`, recomputing `h`
/// via [`standard_margin`] after each simulated step, until `h ≤ 0` or
/// [`MAX_PROJECTION_TICKS`] is reached.
///
/// **ADR 0049 (fixing a round-2 defect, not documenting a new
/// approximation): every deterministically-certain future event is
/// advanced, not just `FirmState` via `market_core`.** Two more things move
/// each simulated tick, both reusing the single existing implementation
/// rather than a second copy of either:
///
/// * **The action window `W`, and `u` with it** — [`crate::advance_window`]
///   (the *same* function `constraint.action_window`'s real `apply` calls)
///   appends the fixed `action` and trims to `l_w`; `u` is recomputed via
///   [`crate::margin::u_from_window`] on the projected window, not held
///   fixed. A firm whose only real danger is a `compliance` violation (`u`
///   trending toward `θ_limit`) now gets a **finite** projection instead of
///   an incorrect "no danger foreseeable."
/// * **Already-enqueued `Λ` effects** — `pending` is the firm's *real*,
///   already-committed lagged-effect queue at the tick the projection
///   starts (not hypothetical). At each simulated tick, any entry whose
///   `maturity_tick` has arrived is applied via
///   [`crate::Effect::deltas_at_maturity`] (the same function
///   `resolve_lagged` calls) — folding its `AdjustAgentReal`/
///   `AdjustAgentInt`/`AdjustGlobalReal`/`AdjustGlobalInt` deltas into the
///   projected state/`θ`; `PushGlobalRecord` (a new `supply` edge) is
///   ignored, since no `g_j` reads edge topology. This is how a firm
///   already mid-flight on a capability investment, or a maturing
///   `lobby`/`contract`, is correctly projected as receiving that benefit
///   (or `contract`'s double-edged cost) at the right future tick.
///
/// This is still **one** fixed-action forward projection of *state*, not
/// the firm re-deciding at each simulated step — it never re-runs
/// `attend`/`satisfices` at an intermediate tick, so it does not chain
/// future *decisions* together, and it never assumes any *hypothetical*
/// future decision (a shaping action not yet taken, a shock not yet
/// scheduled-and-observed) — only facts already true at the start of the
/// projection. §12.3's "the firm does not simulate; it applies a one-step
/// lookahead" is about not projecting its own future *choices* — this
/// projects fixed, already-determined facts forward, the same conceptual
/// move §14.3 itself already licenses as an *offline* viability metric;
/// ADR 0048 uses it decision-time, purely as a **comparison input**, never
/// as a second decision procedure.
///
/// `λ` (legitimacy) is held fixed — nothing changes it before a
/// `constraint.enforce` violation penalty, which only fires *after* a
/// violation this projection would already have stopped at.
///
/// `initial_u` is the caller's own current `u` (matching whatever `h_t`
/// used, including the tick-0 fallback to a seeded `REGULATED_INTENSITY`
/// real while `window` is genuinely empty) — used for the tick-0 check
/// only; from tick 1 on `u` is always recomputed from the growing
/// projected `window`.
///
/// Returns `Some(0)` if `state` is already at or past the boundary,
/// `Some(k)` for the first `k ≥ 1` at which the projected `h ≤ 0`, or
/// `None` if the cap is reached without crossing zero (callers should treat
/// this as "far enough away to not matter"). If `action` becomes
/// inadmissible partway through the projection (e.g. a running cost the
/// firm can no longer afford), the boundary is deemed reached at that step
/// — a firm that can no longer even continue its assumed trajectory has,
/// for this projection's purposes, hit a wall.
#[must_use]
#[allow(clippy::too_many_arguments)] // every input is a distinct, named fact this projection needs
pub fn time_to_boundary(
    action: MarketAction,
    state: &FirmState,
    theta: &ConstraintParams,
    env: &EnvParams,
    legitimacy: f64,
    initial_u: f64,
    window: &[WindowEntry],
    l_w: usize,
    pending: &[LaggedRecord],
    start_tick: u64,
    action_params: &ActionParams,
    scales: &ScaleFactors,
) -> Option<u64> {
    let margin_of = |s: &FirmState, th: &ConstraintParams, u: f64| -> f64 {
        let aux = FirmAuxState {
            legitimacy,
            regulated_intensity: u,
            aspirations: Aspirations {
                capital_growth: 0.0,
                capability: 0.0,
                obligation_clearance: 0.0,
            },
        };
        let ctx = ConstraintContext {
            state: s,
            aux: &aux,
            theta: th,
        };
        standard_margin(&ctx, scales)
    };

    let mut s = *state;
    let mut th = *theta;
    let mut w: Vec<WindowEntry> = window.to_vec();
    // `initial_u` (not re-derived from `window`) matches whatever the
    // *caller's* own `h_t` used — including the tick-0 fallback to a seeded
    // `REGULATED_INTENSITY` real while `W` is still genuinely empty (ADR
    // 0014/0028's tick-0 special case). From tick 1 on, `u` is always
    // recomputed fresh from the growing projected window — the seed is a
    // tick-0-only fact in the real model too (`firm_u`'s fallback only
    // fires while the *real* window is empty, which happens only once).
    let mut u = initial_u;

    // Tick 0: apply any pending `Λ` effect that matures *this* tick before
    // checking whether the firm is already at/past the boundary. At
    // `decide` (phase 3) `resolve_lagged` (phase 6) has not run yet this
    // tick, but it is *guaranteed* to — regardless of what gets decided
    // right now — so a firm about to be rescued (or pushed under) by an
    // already-committed effect maturing today is not misjudged from a
    // stale snapshot.
    apply_maturing(&mut s, &mut th, pending, start_tick);
    if margin_of(&s, &th, u) <= 0.0 {
        return Some(0);
    }

    for tick in 1..=MAX_PROJECTION_TICKS {
        let Some(next) = market_core(action, &s, &th, env, action_params) else {
            return Some(tick); // can no longer continue ⇒ boundary reached
        };
        s = next;

        let real_tick = start_tick + tick;
        apply_maturing(&mut s, &mut th, pending, real_tick);

        w = advance_window(&w, real_tick, action.index(), l_w);
        u = crate::margin::u_from_window(&w, l_w);

        if margin_of(&s, &th, u) <= 0.0 {
            return Some(tick);
        }
    }
    None
}

/// Apply every `pending` `Λ` effect whose `maturity_tick` equals `tick`,
/// via [`crate::Effect::deltas_at_maturity`] — the same function
/// `resolve_lagged` calls — folding the resulting deltas into the
/// projection's `(FirmState, θ)` through [`apply_projected_delta`].
fn apply_maturing(
    state: &mut FirmState,
    theta: &mut ConstraintParams,
    pending: &[LaggedRecord],
    tick: u64,
) {
    // Placeholders for `deltas_at_maturity`'s signature — this projection
    // only reads the resulting deltas' *kinds*, never their `target` agent
    // or `origin`, so any fixed value is safe here.
    let placeholder_agent = firma_core::AgentId(0);
    let placeholder_origin = firma_core::PluginId::new("firma_domain::dynamics::time_to_boundary");
    for rec in pending {
        if rec.matures_at(tick) {
            for d in rec.effect.deltas_at_maturity(
                placeholder_agent,
                &placeholder_origin,
                state.capability,
            ) {
                apply_projected_delta(state, theta, &d.kind);
            }
        }
    }
}

/// Fold one matured effect's delta into the projection's `(FirmState, θ)` —
/// the same fields `resolve_lagged`'s real deltas would mutate through the
/// kernel's reconciler, applied directly here since the projection has no
/// kernel to reconcile through. `PushGlobalRecord` (a new `supply` edge) is
/// silently ignored: no `g_j` reads edge topology.
fn apply_projected_delta(
    state: &mut FirmState,
    theta: &mut ConstraintParams,
    kind: &firma_core::DeltaKind,
) {
    use firma_core::DeltaKind;
    match kind {
        DeltaKind::AdjustAgentReal { field, delta } if field == crate::keys::CAPABILITY => {
            state.capability += delta; // already clamped by `deltas_at_maturity`
        }
        DeltaKind::AdjustAgentInt { field, delta } if field == crate::keys::OBLIGATION => {
            state.obligation += delta;
        }
        DeltaKind::AdjustGlobalReal { field, delta } if field == crate::keys::THETA_LIMIT => {
            theta.theta_limit += delta;
        }
        DeltaKind::AdjustGlobalInt { field, delta } if field == crate::keys::THETA_Q => {
            theta.theta_q += delta;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Effect;

    fn theta() -> ConstraintParams {
        ConstraintParams {
            theta_limit: 0.9,
            theta_cap: 0.4,
            theta_q: 100,
        }
    }

    #[test]
    fn yields_match_section_11_1_formula() {
        let p = ActionParams::default(); // y_0 = 4, η = 0.8, γ_R = 0.5
                                         // c = 0.55: y_O = ⌊4·(1 + 0.8·0.55)⌋ = ⌊4·1.44⌋ = ⌊5.76⌋ = 5
        assert_eq!(y_o(0.55, &p), 5);
        // y_R = ⌊4·1.44·1.5⌋ = ⌊8.64⌋ = 8
        assert_eq!(y_r(0.55, &p), 8);
        // c = 0: y_O = 4, y_R = ⌊4·1.5⌋ = 6
        assert_eq!(y_o(0.0, &p), 4);
        assert_eq!(y_r(0.0, &p), 6);
    }

    #[test]
    fn produce_regulated_gated_by_scope() {
        let p = ActionParams::default();
        let env = EnvParams {
            input_price: 2,
            output_price: 1,
        };
        let below = FirmState {
            liquid_capital: 10,
            input_stock: 3,
            capability: 0.30,
            obligation: 0,
        };
        // c = 0.30 < θ_cap = 0.40 → inadmissible.
        assert!(market_core(MarketAction::ProduceRegulated, &below, &theta(), &env, &p).is_none());
        let ok = FirmState {
            capability: 0.45,
            ..below
        };
        assert!(market_core(MarketAction::ProduceRegulated, &ok, &theta(), &env, &p).is_some());
    }

    #[test]
    fn hold_is_identity_and_always_admissible() {
        let p = ActionParams::default();
        let env = EnvParams {
            input_price: 2,
            output_price: 1,
        };
        let s = FirmState {
            liquid_capital: 0,
            input_stock: 0,
            capability: 0.0,
            obligation: 0,
        };
        assert_eq!(
            market_core(MarketAction::Hold, &s, &theta(), &env, &p),
            Some(s)
        );
    }

    #[test]
    fn invest_capability_clamps_at_one() {
        let p = ActionParams::default(); // δ_c = 0.05, κ_c = 20
        let env = EnvParams {
            input_price: 2,
            output_price: 1,
        };
        let s = FirmState {
            liquid_capital: 100,
            input_stock: 0,
            capability: 0.98,
            obligation: 0,
        };
        let n = market_core(MarketAction::InvestCapability, &s, &theta(), &env, &p).unwrap();
        assert!((n.capability - 1.0).abs() < 1e-12);
        assert_eq!(n.liquid_capital, 80);
    }

    #[test]
    fn shaping_cost_step_is_cost_only_and_gated_by_affordability() {
        let s = FirmState {
            liquid_capital: 30,
            input_stock: 4,
            capability: 0.5,
            obligation: 2,
        };
        // affordable: only r^L moves; c / r^I / q untouched (the payoff is lagged)
        let n = shaping_cost_step(25, &s).unwrap();
        assert_eq!(n.liquid_capital, 5);
        assert_eq!(n.input_stock, s.input_stock);
        assert_eq!(n.obligation, s.obligation);
        assert!((n.capability - s.capability).abs() < 1e-12);
        // cannot afford ⇒ None (the §11.2 precondition)
        assert!(shaping_cost_step(31, &s).is_none());
        assert!(shaping_cost_step(-1, &s).is_none());
    }

    // --- time_to_boundary (§14.3; ADR 0048, ADR 0049) ---

    // theta chosen so only `solvency` can ever bind in these fixtures.
    fn solvency_only_theta() -> ConstraintParams {
        ConstraintParams {
            theta_limit: 100.0, // u=0 always ⇒ g_compliance deeply negative
            theta_cap: 0.0,     // any capability ≥ 0 ⇒ g_scope deeply negative
            theta_q: 1_000_000, // obligation=0 ⇒ g_obligation deeply negative
        }
    }

    #[test]
    fn time_to_boundary_zero_when_already_past() {
        let s = FirmState {
            liquid_capital: 0, // g_solvency = -0 = 0 ⇒ h = 0 ⇒ already at the boundary
            input_stock: 0,
            capability: 0.5,
            obligation: 0,
        };
        let env = EnvParams {
            input_price: 2,
            output_price: 3,
        };
        let got = time_to_boundary(
            MarketAction::Hold,
            &s,
            &solvency_only_theta(),
            &env,
            1.0,
            0.0,
            &[],
            8,
            &[],
            0,
            &ActionParams::default(),
            &ScaleFactors::default(),
        );
        assert_eq!(got, Some(0));
    }

    #[test]
    fn time_to_boundary_counts_an_exact_crossing() {
        // capital = κ_c exactly ⇒ one invest_capability lands at capital = 0
        // (h = 0, "≤ 0") via a *valid* transition, not inadmissibility.
        let p = ActionParams::default(); // κ_c = 20
        let s = FirmState {
            liquid_capital: 20,
            input_stock: 0,
            capability: 0.5,
            obligation: 0,
        };
        let env = EnvParams {
            input_price: 2,
            output_price: 3,
        };
        let got = time_to_boundary(
            MarketAction::InvestCapability,
            &s,
            &solvency_only_theta(),
            &env,
            1.0,
            0.0,
            &[],
            8,
            &[],
            0,
            &p,
            &ScaleFactors::default(),
        );
        assert_eq!(got, Some(1));
    }

    #[test]
    fn time_to_boundary_counts_ticks_when_action_becomes_inadmissible() {
        // capital = 90, κ_c = 20/tick: 90→70→50→30→10, then the 5th attempt
        // (capital=10 < κ_c=20) is inadmissible — the boundary is deemed
        // reached there, one tick after the last *valid* step, even though
        // capital (10) never actually reached ≤ 0 via a completed transition.
        let p = ActionParams::default();
        let s = FirmState {
            liquid_capital: 90,
            input_stock: 0,
            capability: 0.1, // room to keep "investing" without early-capping
            obligation: 0,
        };
        let env = EnvParams {
            input_price: 2,
            output_price: 3,
        };
        let got = time_to_boundary(
            MarketAction::InvestCapability,
            &s,
            &solvency_only_theta(),
            &env,
            1.0,
            0.0,
            &[],
            8,
            &[],
            0,
            &p,
            &ScaleFactors::default(),
        );
        assert_eq!(got, Some(5));
    }

    #[test]
    fn time_to_boundary_is_none_under_hold_when_margin_never_moves() {
        // `hold` is the identity transition (§11.1); it is never
        // `produce_regulated`, so the projected window never gains a
        // regulated-production entry and `u` stays 0 (ADR 0049 — `u` is no
        // longer *unconditionally* frozen, but it only moves when the
        // projected action actually is `produce_regulated`). With
        // `solvency_only_theta()` (deeply slack everywhere else) and no
        // pending effects, nothing here ever moves ⇒ `h` never changes ⇒
        // the cap is reached.
        let s = FirmState {
            liquid_capital: 500,
            input_stock: 0,
            capability: 0.5,
            obligation: 0,
        };
        let env = EnvParams {
            input_price: 2,
            output_price: 3,
        };
        let got = time_to_boundary(
            MarketAction::Hold,
            &s,
            &solvency_only_theta(),
            &env,
            1.0,
            0.0,
            &[],
            8,
            &[],
            0,
            &ActionParams::default(),
            &ScaleFactors::default(),
        );
        assert_eq!(
            got, None,
            "an unmoving margin must hit the cap, not loop forever"
        );
    }

    // --- ADR 0049: the two round-3 fixes, each verified by hand ---

    // theta chosen so only `compliance` can ever bind.
    fn compliance_only_theta() -> ConstraintParams {
        ConstraintParams {
            theta_limit: 0.5,
            theta_cap: 0.0,     // any capability ≥ 0 ⇒ g_scope deeply negative
            theta_q: 1_000_000, // obligation=0 ⇒ g_obligation deeply negative
        }
    }

    #[test]
    fn time_to_boundary_sees_compliance_danger_the_round_2_projection_missed() {
        // A firm with ample cash/input/capability — no solvency or scope
        // danger whatsoever — but repeatedly `produce_regulated` (2) drives
        // `u` toward `θ_limit` as the projected window fills.
        //
        // By hand: l_w = 4, empty starting window, θ_limit = 0.5.
        // u_from_window divides by the *configured* l_w, not how many
        // entries exist yet (§14.2 R1 / ADR 0014's window formula):
        //   tick 1: window=[2]        ⇒ u = 1/4 = 0.25 ⇒ g_compliance=-0.25 (slack)
        //   tick 2: window=[2,2]      ⇒ u = 2/4 = 0.50 ⇒ g_compliance= 0.00 ⇒ h ≤ 0.
        // Round 2 (u held fixed at its start value, 0) would have returned
        // `None` here — incorrectly reporting no danger foreseeable.
        let s = FirmState {
            liquid_capital: 1000,
            input_stock: 100,
            capability: 0.5,
            obligation: 0,
        };
        let env = EnvParams {
            input_price: 2,
            output_price: 3,
        };
        let got = time_to_boundary(
            MarketAction::ProduceRegulated,
            &s,
            &compliance_only_theta(),
            &env,
            1.0,
            0.0,
            &[], // empty starting window
            4,   // l_w
            &[], // no pending effects
            0,
            &ActionParams::default(),
            &ScaleFactors::default(),
        );
        assert_eq!(
            got,
            Some(2),
            "a compliance-only danger must now be seen (finite), matching \
             the hand-computed u=0.25→0.50 crossing at tick 2"
        );
    }

    #[test]
    fn time_to_boundary_sees_an_already_pending_rescue() {
        // A firm *currently* below θ_cap (§9.1 scope already violated, so
        // WITHOUT the pending effect this is `Some(0)` — verified below,
        // both sides of the contrast) — but with a `CapabilityGain` already
        // committed (a past `invest_capability`) maturing *this* tick
        // (`start_tick`), raising capability back above θ_cap before the
        // boundary is ever checked.
        //
        // By hand: θ_cap=0.5, capability=0.4 ⇒ g_scope = 0.5-0.4 = 0.1 ≥ 0
        // ⇒ violated *now*. Pending `CapabilityGain{delta:0.2}` matures at
        // tick 10 (== start_tick) ⇒ capability → 0.6 ⇒ g_scope = 0.5-0.6 =
        // -0.1 (slack) ⇒ h > 0 after the rescue is applied, *before* the
        // tick-0 boundary check runs.
        let s = FirmState {
            liquid_capital: 1000,
            input_stock: 0,
            capability: 0.4,
            obligation: 0,
        };
        let theta = ConstraintParams {
            theta_limit: 100.0, // slack — never binds
            theta_cap: 0.5,
            theta_q: 1_000_000,
        };
        let env = EnvParams {
            input_price: 2,
            output_price: 3,
        };
        let scales = ScaleFactors::default();

        // Without the pending rescue: already violating ⇒ Some(0).
        let without_rescue = time_to_boundary(
            MarketAction::Hold,
            &s,
            &theta,
            &env,
            1.0,
            0.0,
            &[],
            8,
            &[], // no pending effects
            10,
            &ActionParams::default(),
            &scales,
        );
        assert_eq!(
            without_rescue,
            Some(0),
            "capability 0.4 < θ_cap 0.5 ⇒ already violating scope, with no rescue pending"
        );

        // With the pending rescue maturing exactly at `start_tick`:
        let pending = [LaggedRecord::new(10, Effect::CapabilityGain { delta: 0.2 })];
        let with_rescue = time_to_boundary(
            MarketAction::Hold,
            &s,
            &theta,
            &env,
            1.0,
            0.0,
            &[],
            8,
            &pending,
            10, // start_tick == the rescue's maturity_tick
            &ActionParams::default(),
            &scales,
        );
        assert_eq!(
            with_rescue, None,
            "the already-pending capability gain must be applied before the \
             tick-0 boundary check — the firm is rescued, not already violating"
        );
    }
}
