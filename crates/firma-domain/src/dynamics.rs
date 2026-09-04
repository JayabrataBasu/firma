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

use crate::params::ConstraintParams;
use crate::state::FirmState;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
