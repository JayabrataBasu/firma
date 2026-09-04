//! The FIRMA domain surface of `firma-viability` (manual §9.2, §9.3; §18.2).
//!
//! `firma-viability` depends on `firma-domain::MarginTerm` — `g_j` and `s_j`
//! only — and never on `Constraint` / `ViolationSemantic` (ADR 0021 decision
//! 3), so it stays domain-*state*-aware but domain-*policy*-free.

use firma_domain::dynamics::{market_core, ActionParams, EnvParams, MARKET_ACTIONS};
use firma_domain::{
    Aspirations, ConstraintContext, ConstraintParams, FirmAuxState, FirmState, MarginTerm,
};

use crate::{kernel_from, Dynamics, Grid, KernelReport, KernelSet};

/// §9.3: "21 steps of 0.05 for `c`" — the default capability grid resolution.
pub const DEFAULT_C_STEP: f64 = 0.05;

/// §9.2 — the viability margin `h(x, θ) = −max_j (g_j(x, θ) / s_j)`.
///
/// Takes `&[&dyn MarginTerm]` (`g_j`, `s_j`) rather than §18.2's
/// `cs: &[Constraint]`, and folds `x` + auxiliary state + θ into a
/// [`ConstraintContext`] (ADR 0021 decision 3).
///
/// `h > 0` ⇒ every constraint satisfied (§9.2). **Mandatory §9.2 caveat:** `h`
/// measures distance to *current* infeasibility; a state can have large `h` and
/// still lie outside the viability kernel — VT-3 quantifies the divergence.
///
/// # Panics
/// If `terms` is empty (`max` over an empty set is undefined; §27.2 requires 4).
#[must_use]
pub fn margin(ctx: &ConstraintContext<'_>, terms: &[&dyn MarginTerm]) -> f64 {
    assert!(
        !terms.is_empty(),
        "margin needs at least one constraint term (§9.2)"
    );
    let worst = terms
        .iter()
        .map(|t| t.g(ctx) / t.scale())
        .fold(f64::NEG_INFINITY, f64::max);
    -worst
}

/// The FIRMA-specific transition relation for exact viability-kernel
/// computation (manual §9.3; ADR 0021 decision 4).
///
/// Grid dimensions are `(r^L, r^I, c_index, q)` where `c = c_index · c_step`
/// (§9.3). Successors come from the §11.1 deterministic core
/// ([`firma_domain::dynamics::market_core`]).
///
/// **§9.3 approximation, recorded per §9.3's own requirement:** "The transition
/// `f` used for kernel computation is the deterministic core of §11 with
/// stochastic terms at expectation — an approximation that MUST be recorded as
/// such." Additionally (ADR 0021 decision 4): `invest_capability`'s lag is
/// collapsed to immediate (see [`firma_domain::dynamics`]), and `u` — not a
/// kernel-carrying dimension (§8.1: "d = 4, not 5") — is held at a fixed
/// `u_context` throughout the backward iteration (see [`firma_kernel`]). The
/// exact FIRMA kernel is therefore a slice `K(θ, u_context)`.
#[derive(Debug, Clone)]
pub struct FirmaDynamics {
    theta: ConstraintParams,
    env: EnvParams,
    action_params: ActionParams,
    c_step: f64,
}

impl FirmaDynamics {
    /// Construct with an explicit capability grid step.
    #[must_use]
    pub fn new(
        theta: ConstraintParams,
        env: EnvParams,
        action_params: ActionParams,
        c_step: f64,
    ) -> FirmaDynamics {
        assert!(c_step > 0.0, "c_step must be > 0");
        FirmaDynamics {
            theta,
            env,
            action_params,
            c_step,
        }
    }

    /// Construct with the §9.3 default `c_step = 0.05` and §16.1 default
    /// [`ActionParams`].
    #[must_use]
    pub fn with_defaults(theta: ConstraintParams, env: EnvParams) -> FirmaDynamics {
        FirmaDynamics::new(theta, env, ActionParams::default(), DEFAULT_C_STEP)
    }

    /// The capability grid step.
    #[must_use]
    pub fn c_step(&self) -> f64 {
        self.c_step
    }

    /// Decode a grid point `(r^L, r^I, c_index, q)` to a [`FirmState`].
    #[must_use]
    pub fn point_to_state(&self, p: &[i64]) -> FirmState {
        assert_eq!(p.len(), 4, "FIRMA grid is 4-D (r^L, r^I, c_index, q)");
        FirmState {
            liquid_capital: p[0],
            input_stock: p[1],
            capability: p[2] as f64 * self.c_step,
            obligation: p[3],
        }
    }

    /// Encode a [`FirmState`] to its grid point. `c` is snapped to the nearest
    /// `c_index`; `δ_c` (§16.1 `0.05`) is exactly one step, so the snap is exact
    /// for states reached through `invest_capability`.
    #[must_use]
    pub fn state_to_point(&self, s: &FirmState) -> Vec<i64> {
        vec![
            s.liquid_capital,
            s.input_stock,
            (s.capability / self.c_step).round() as i64,
            s.obligation,
        ]
    }
}

impl Dynamics for FirmaDynamics {
    fn successors(&self, point: &[i64]) -> Vec<Vec<i64>> {
        let s = self.point_to_state(point);
        MARKET_ACTIONS
            .iter()
            .filter_map(|&a| {
                market_core(a, &s, &self.theta, &self.env, &self.action_params)
                    .map(|n| self.state_to_point(&n))
            })
            .collect()
    }
}

/// A [`FirmAuxState`] whose only constraint-relevant field is `u` (fixed at
/// `u_context`); `legitimacy` and `aspirations` do not enter any `g_j` (§9.1)
/// so their values here are inert.
fn aux_at(u_context: f64) -> FirmAuxState {
    FirmAuxState {
        legitimacy: 1.0,
        regulated_intensity: u_context,
        aspirations: Aspirations {
            capital_growth: 0.0,
            capability: 0.0,
            obligation_clearance: 0.0,
        },
    }
}

/// Compute the exact FIRMA viability kernel `K(θ, u_context)` over `grid`
/// (manual §9.3; ADR 0021 decision 4):
///
/// 1. `K^(0)` = grid points where every constraint term is satisfied
///    (`g_j ≤ 0`) — this is `K(θ)` (§9.3), evaluated at `u = u_context`.
/// 2. backward-iterate with [`FirmaDynamics`] via [`kernel_from`].
///
/// `terms` are the four constraint plugins as [`MarginTerm`]s.
#[must_use]
pub fn firma_kernel(
    grid: &Grid,
    dynamics: &FirmaDynamics,
    terms: &[&dyn MarginTerm],
    theta: &ConstraintParams,
    u_context: f64,
) -> KernelReport {
    let aux = aux_at(u_context);
    let k0 = KernelSet::from_predicate(grid, |p| {
        let s = dynamics.point_to_state(p);
        let ctx = ConstraintContext {
            state: &s,
            aux: &aux,
            theta,
        };
        terms.iter().all(|t| t.g(&ctx) <= 0.0)
    });
    kernel_from(grid, dynamics, k0)
}

/// Whether `s` lies in the kernel set `k` (computed over `grid` with
/// `dynamics`' encoding). §18.2 `in_kernel`.
#[must_use]
pub fn in_kernel(k: &KernelSet, grid: &Grid, dynamics: &FirmaDynamics, s: &FirmState) -> bool {
    grid.index_of(&dynamics.state_to_point(s))
        .is_some_and(|i| k.contains_index(i))
}

/// `|K| / |grid|` (manual §9.3). §18.2 `volume`. Thin wrapper over
/// [`KernelSet::count`] and [`Grid::size`].
#[must_use]
pub fn volume(k: &KernelSet, grid: &Grid) -> f64 {
    if grid.size() == 0 {
        0.0
    } else {
        k.count() as f64 / grid.size() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One synthetic `MarginTerm` per (g, s) pair — enough to check that
    /// `margin` computes `-max_j(g_j/s_j)` and picks the right binding term.
    /// The §15.1 (Example A) exact-value fixture, using the *real*
    /// `firma-plugin-constraint` plugins, lives in the conformance suite
    /// (`tests/tests/validation.rs::margin_matches_section_15_1`) to keep the
    /// §15.1 reference literals out of a sim-path crate's source.
    struct Term(f64, f64); // (g value, scale)
    impl MarginTerm for Term {
        fn g(&self, _c: &ConstraintContext<'_>) -> f64 {
            self.0
        }
        fn scale(&self) -> f64 {
            self.1
        }
    }

    #[test]
    fn margin_is_negative_max_of_scaled_g() {
        let state = FirmState {
            liquid_capital: 1,
            input_stock: 0,
            capability: 0.0,
            obligation: 0,
        };
        let aux = FirmAuxState {
            legitimacy: 1.0,
            regulated_intensity: 0.0,
            aspirations: Aspirations {
                capital_growth: 0.0,
                capability: 0.0,
                obligation_clearance: 0.0,
            },
        };
        let theta = ConstraintParams {
            theta_limit: 1.0,
            theta_cap: 0.0,
            theta_q: 1,
        };
        let ctx = ConstraintContext {
            state: &state,
            aux: &aux,
            theta: &theta,
        };

        // g/s values: -0.4, -0.18, -0.3, -1.4  → max = -0.18 → h = 0.18
        let (a, b, c, d) = (
            Term(-4.0, 10.0),
            Term(-0.18, 1.0),
            Term(-0.15, 0.5),
            Term(-70.0, 50.0),
        );
        let terms: [&dyn MarginTerm; 4] = [&a, &b, &c, &d];
        let h = margin(&ctx, &terms);
        assert!((h - 0.18).abs() < 1e-12, "h = -max(g/s); got {h}");

        // A single positive term makes h negative (constraint violated).
        let bad = Term(0.5, 1.0);
        assert!(margin(&ctx, &[&bad]) < 0.0);
    }
}
