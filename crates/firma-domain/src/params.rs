//! Constraint parameters θ (§8.2), scale factors `s_j` (§9.2), and state bounds
//! (§16.1).

use serde::{Deserialize, Serialize};

/// §8.2 — the constraint parameter vector `θ_t = (θ_limit, θ_cap, θ_Q)`.
///
/// **Global** — one shared instance per world, not indexed by `AgentId`
/// (ADR 0015; §8.2 "Global by default … Per-firm variant is Phase 4"). This is
/// the object that makes the viability kernel endogenous (§9.4): shaping
/// actions and regulatory shocks both move it, and every firm's
/// `compliance` / `scope` / `obligation` constraint reads it.
///
/// No `Default` impl: §16.1 gives no defaults for θ, and §15.1's values
/// (`0.90`, `0.40`, `100`) are worked-example values, not defaults —
/// configuration MUST supply θ explicitly (avoids an unspecified default,
/// §32.1 risk 2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstraintParams {
    /// `θ_limit` — permitted regulated-activity intensity. `f64`. Compared
    /// against `u` in §9.1 `compliance` (`g_2 = u - θ_limit`). Moved down by
    /// `Regulatory` shocks (§13.2) and up by successful `lobby` (§11.2),
    /// additively (ADR 0016).
    pub theta_limit: f64,
    /// `θ_cap` — minimum capability for regulated production. `f64`. Compared
    /// against `c` in §9.1 `scope` (`g_3 = θ_cap - c`). Moved by `Regulatory`
    /// shocks.
    pub theta_cap: f64,
    /// `θ_Q` — maximum permitted outstanding obligation. `i64` — compared
    /// against `q` (`i64`) in §9.1 `obligation` (`g_4 = q - θ_Q`), kept integer
    /// so that constraint is exact integer arithmetic (ADR 0015; §15.1's
    /// `g_4 = 30 - 100 = -70` holds). Moved up by successful `contract` (§11.2).
    pub theta_q: i64,
}

/// §9.2 — the per-constraint scale factors `s_j` used to commensurate the four
/// constraints in the viability margin `h = -max_j (g_j / s_j)`.
///
/// Defaults are the §9.2 table values. `s_j` are `f64` (they divide `g_j`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScaleFactors {
    /// `s_L` — solvency scale. §9.2 default `100` ("median start-of-run liquid
    /// capital").
    pub s_l: f64,
    /// `s_u` — compliance scale. §9.2 default `1.0` ("already a normalised
    /// intensity scale").
    pub s_u: f64,
    /// `s_c` — scope scale. §9.2 default `0.5` ("half the capability range").
    pub s_c: f64,
    /// `s_q` — obligation scale. §9.2 default `50` ("half of Q_max").
    pub s_q: f64,
}

impl Default for ScaleFactors {
    /// The §9.2 table defaults.
    fn default() -> Self {
        ScaleFactors {
            s_l: 100.0,
            s_u: 1.0,
            s_c: 0.5,
            s_q: 50.0,
        }
    }
}

/// §16.1 — fixed bounds on the constraint-carrying state fields. `[0, *_max]`
/// per §8.1.
///
/// Defaults are the §16.1 table values, listed there as "fixed".
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateBounds {
    /// `R^L_max` — upper bound on `r^L`. §16.1 default `500`.
    pub r_l_max: i64,
    /// `R^I_max` — upper bound on `r^I` (storage cap). §16.1 default `40`.
    pub r_i_max: i64,
    /// `Q_max` — upper bound on `q`. §16.1 default `100`.
    pub q_max: i64,
}

impl Default for StateBounds {
    /// The §16.1 table defaults (marked "fixed").
    fn default() -> Self {
        StateBounds {
            r_l_max: 500,
            r_i_max: 40,
            q_max: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_factor_defaults_match_section_9_2() {
        let s = ScaleFactors::default();
        assert_eq!((s.s_l, s.s_u, s.s_c, s.s_q), (100.0, 1.0, 0.5, 50.0));
    }

    #[test]
    fn state_bound_defaults_match_section_16_1() {
        let b = StateBounds::default();
        assert_eq!((b.r_l_max, b.r_i_max, b.q_max), (500, 40, 100));
    }

    #[test]
    fn constraint_params_json_roundtrip() {
        // §15.1 worked-example values (not defaults).
        let p = ConstraintParams {
            theta_limit: 0.90,
            theta_cap: 0.40,
            theta_q: 100,
        };
        let j = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<ConstraintParams>(&j).unwrap(), p);
    }

    #[test]
    fn deny_unknown_fields_is_enforced() {
        let bad = r#"{"s_l":1.0,"s_u":1.0,"s_c":1.0,"s_q":1.0,"bogus":0}"#;
        assert!(serde_json::from_str::<ScaleFactors>(bad).is_err());
    }
}
