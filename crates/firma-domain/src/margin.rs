//! The four §9.1 constraint functions `g_j` and the standard-four viability
//! margin `h` (manual §9.1, §9.2; ADR 0026).
//!
//! **Single source.** `firma-plugin-constraint`'s four `Constraint` plugins
//! delegate their `MarginTerm::g` here, and `decision.satisficing` (§12.3
//! Step 1) computes `h` here — without a cross-plugin dependency and without
//! writing the formulas twice (the same discipline ADR 0021 applied to §11's
//! `market_core`). `firma-viability::margin` stays the *generic* fold over an
//! arbitrary `&[&dyn MarginTerm]` that the kernel solver needs; it is
//! unchanged, and it now agrees with these by construction.

use crate::constraint::ConstraintContext;
use crate::params::ScaleFactors;
use crate::window::WindowEntry;

/// `produce_regulated`'s §11 canonical action index — the one action that
/// contributes to `u` (§9.1, ADR 0014).
pub const PRODUCE_REGULATED: u8 = 2;

/// `u = (1/L_W) · Σ_{k=1..L_W} ι(W[k])`, `ι(a) = 1` iff
/// `a == produce_regulated` (ADR 0014). The denominator is **always** `l_w`:
/// window slots not yet filled early in a run count as `ι = 0` (equivalently,
/// pre-filled with `hold`), so a fresh firm has `u = 0`. Only the last `l_w`
/// entries of `window` are considered; `u ∈ {0, 1/l_w, …, 1}`.
///
/// # Panics
/// If `l_w == 0`.
#[must_use]
pub fn u_from_window(window: &[WindowEntry], l_w: usize) -> f64 {
    assert!(l_w > 0, "L_W must be >= 1 (§16.1)");
    let start = window.len().saturating_sub(l_w);
    let regulated = window[start..]
        .iter()
        .filter(|e| e.action == PRODUCE_REGULATED)
        .count();
    regulated as f64 / l_w as f64
}

/// `g_1 = −r^L` — §9.1 `solvency`. `g_1 > 0` ⇔ insolvent.
#[must_use]
pub fn g_solvency(ctx: &ConstraintContext<'_>) -> f64 {
    -(ctx.state.liquid_capital as f64)
}

/// `g_2 = u − θ_limit` — §9.1 `compliance` (`u` = regulated-activity intensity,
/// the trailing-window mean of ADR 0014).
#[must_use]
pub fn g_compliance(ctx: &ConstraintContext<'_>) -> f64 {
    ctx.aux.regulated_intensity - ctx.theta.theta_limit
}

/// `g_3 = θ_cap − c` — §9.1 `scope` (the `produce_regulated` gate, §11.4).
#[must_use]
pub fn g_scope(ctx: &ConstraintContext<'_>) -> f64 {
    ctx.theta.theta_cap - ctx.state.capability
}

/// `g_4 = q − θ_Q` — §9.1 `obligation`. Integer arithmetic under the hood
/// (`q`, `θ_Q` are `i64`); cast once for the scaled fold.
#[must_use]
pub fn g_obligation(ctx: &ConstraintContext<'_>) -> f64 {
    ctx.state.obligation as f64 - ctx.theta.theta_q as f64
}

/// The four `g_j` in canonical order `[solvency, compliance, scope, obligation]`.
#[must_use]
pub fn all_g(ctx: &ConstraintContext<'_>) -> [f64; 4] {
    [
        g_solvency(ctx),
        g_compliance(ctx),
        g_scope(ctx),
        g_obligation(ctx),
    ]
}

/// The standard four-constraint viability margin `h = −max_j (g_j / s_j)`
/// (§9.2), with `s_j` from `scales` (`s_L`, `s_u`, `s_c`, `s_q`).
///
/// `h > 0` ⇔ every one of the four constraints is currently satisfied. Same
/// §9.2 caveat as [`firma_viability::margin`](../../firma_viability/): `h`
/// measures distance to *current* infeasibility, not viability-kernel
/// membership.
#[must_use]
pub fn standard_margin(ctx: &ConstraintContext<'_>, scales: &ScaleFactors) -> f64 {
    let worst = [
        g_solvency(ctx) / scales.s_l,
        g_compliance(ctx) / scales.s_u,
        g_scope(ctx) / scales.s_c,
        g_obligation(ctx) / scales.s_q,
    ]
    .into_iter()
    .fold(f64::NEG_INFINITY, f64::max);
    -worst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Aspirations, FirmAuxState, FirmState};
    use crate::ConstraintParams;

    /// Manual §15.1 (Example A) — the direct fixture. `h = 0.180`, binding
    /// `compliance`.
    #[test]
    fn matches_section_15_1() {
        let state = FirmState {
            liquid_capital: 40,
            input_stock: 12,
            capability: 0.55,
            obligation: 30,
        };
        let aux = FirmAuxState {
            legitimacy: 1.0,
            regulated_intensity: 0.72,
            aspirations: Aspirations {
                capital_growth: 0.0,
                capability: 0.0,
                obligation_clearance: 0.0,
            },
        };
        let theta = ConstraintParams {
            theta_limit: 0.90,
            theta_cap: 0.40,
            theta_q: 100,
        };
        let ctx = ConstraintContext {
            state: &state,
            aux: &aux,
            theta: &theta,
        };

        // §15.1 table: raw g_j
        assert!((g_solvency(&ctx) - (-40.0)).abs() < 1e-12);
        assert!((g_compliance(&ctx) - (-0.18)).abs() < 1e-12);
        assert!((g_scope(&ctx) - (-0.15)).abs() < 1e-12);
        assert!((g_obligation(&ctx) - (-70.0)).abs() < 1e-12);

        // scaled g_j / s_j = -0.400, -0.180, -0.300, -1.400
        let scales = ScaleFactors::default();
        let scaled = [
            g_solvency(&ctx) / scales.s_l,
            g_compliance(&ctx) / scales.s_u,
            g_scope(&ctx) / scales.s_c,
            g_obligation(&ctx) / scales.s_q,
        ];
        for (got, want) in scaled.iter().zip([-0.400, -0.180, -0.300, -1.400]) {
            assert!((got - want).abs() < 1e-12, "{scaled:?}");
        }

        // h = -max = 0.180, binding = compliance (index 1)
        let h = standard_margin(&ctx, &scales);
        assert!((h - 0.180).abs() < 1e-12, "h = {h}");
        let binding = scaled
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert_eq!(binding, 1);
    }

    #[test]
    fn u_from_window_matches_adr_0014() {
        let w = |acts: &[u8]| -> Vec<WindowEntry> {
            acts.iter()
                .enumerate()
                .map(|(i, &a)| WindowEntry::new(i as u64, a))
                .collect()
        };
        // empty / short window: denominator is always L_W, unfilled ⇒ ι=0
        assert!((u_from_window(&[], 8) - 0.0).abs() < 1e-12);
        assert!((u_from_window(&w(&[2, 2]), 8) - 2.0 / 8.0).abs() < 1e-12);
        // full window of 8, three regulated ⇒ 3/8
        assert!((u_from_window(&w(&[2, 0, 2, 1, 0, 2, 5, 0]), 8) - 3.0 / 8.0).abs() < 1e-12);
        // only the last L_W count — an old `produce_regulated` ages out
        assert!((u_from_window(&w(&[2, 0, 0, 0, 0]), 4) - 0.0).abs() < 1e-12);
        // grid: u is always a multiple of 1/L_W
        for k in 0..=4 {
            let acts: Vec<u8> = (0..4).map(|i| if i < k { 2 } else { 0 }).collect();
            assert!((u_from_window(&w(&acts), 4) - k as f64 / 4.0).abs() < 1e-12);
        }
    }

    #[test]
    fn positive_margin_means_all_satisfied() {
        let state = FirmState {
            liquid_capital: 100,
            input_stock: 5,
            capability: 0.9,
            obligation: 0,
        };
        let aux = FirmAuxState {
            legitimacy: 1.0,
            regulated_intensity: 0.1,
            aspirations: Aspirations {
                capital_growth: 0.0,
                capability: 0.0,
                obligation_clearance: 0.0,
            },
        };
        let theta = ConstraintParams {
            theta_limit: 0.9,
            theta_cap: 0.4,
            theta_q: 50,
        };
        let ctx = ConstraintContext {
            state: &state,
            aux: &aux,
            theta: &theta,
        };
        assert!(all_g(&ctx).iter().all(|g| *g <= 0.0));
        assert!(standard_margin(&ctx, &ScaleFactors::default()) > 0.0);
    }
}
