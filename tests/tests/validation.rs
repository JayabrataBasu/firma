//! Validation tests (manual §25.4).
//!
//! * **VT-2, VT-6** — Phase 1 gate (§26.3).
//! * **VT-1** (`vt1_kernel_analytic_1d_and_2d`) — Phase 2 gate; the exact
//!   kernel matches the analytic answer for a 1-D and the §15.2 2-D linear box
//!   system.
//! * **VT-3** (`vt3_margin_vs_exact_kernel_membership`) — Phase 2 gate; a
//!   *reporting* test (§25.4 "Report correlation"), not a pass/fail threshold:
//!   the correlation between `h > 0` and exact FIRMA-kernel membership.
//!
//! * **VT-7** (`vt7_shaping_actions_are_costly_lagged_and_uncertain`) — Phase 2
//!   Stage 2; §11.3's three mandatory properties, checked generically for every
//!   entry in `firma_plugin_action_shaping::shaping_catalog()`.
//! * **VT-4** (`vt4_beta_zero_no_shortfall_action_never_changes`) — Phase 2
//!   Stage 3; the inertia principle (§3.1): focus = NONE ⇒ `decision.satisficing`
//!   repeats last tick's action forever, for every β.
//! * **VT-5** (`vt5_decision_random_establishes_the_null`) — Phase 2 Stage 3;
//!   a *reporting* test (ADR 0027): under `decision.random` the realised choice
//!   is uniform-over-admissible at every margin, so `h` ↔ (narrowing) rigidity
//!   has a bootstrap 95 % CI containing zero.
//!
//! * **VT-8** (`vt8_orthogonal_manipulation_of_h_and_shortfall`) — Phase 2
//!   Stage 6 (ADR 0040); `h` and `ς` are independently manipulable —
//!   (i) empirical `r(h, ς) ≈ 0` across a constructed grid, (ii) all four
//!   `(h-side × ς-side)` quadrants populated, (iii) the pure `select` function
//!   computes neither quantity from the other (a signature-level fact).

use firma_cli::{execute_run, standard_constraints, standard_registry, verify_run, RunOptions};
use firma_conformance::{active_config, scratch};
use firma_domain::dynamics::EnvParams;
use firma_domain::{
    Aspirations, ConstraintContext, ConstraintParams, FirmAuxState, FirmState, MarginTerm,
};
use firma_viability::{firma_kernel, in_kernel, kernel, margin, Dynamics, FirmaDynamics, Grid};

/// Manual §15.2 Example B — the P/B transition system on a 5×5 integer grid.
/// Defined here, in the test, not in `firma-viability` (ADR 0011): the solver
/// sees only an opaque [`Dynamics`].
struct ExampleB;

impl Dynamics for ExampleB {
    fn successors(&self, point: &[i64]) -> Vec<Vec<i64>> {
        let (rl, ri) = (point[0], point[1]);
        let mut out = Vec::new();
        // P (produce): (rL+1, rI-1), admissible when rL <= 3 and rI >= 1.
        if rl <= 3 && ri >= 1 {
            out.push(vec![rl + 1, ri - 1]);
        }
        // B (buy): (rL-2, rI+2), admissible when rL >= 2 and rI <= 2.
        if rl >= 2 && ri <= 2 {
            out.push(vec![rl - 2, ri + 2]);
        }
        out
    }
}

/// VT-2 — backward iteration is monotone decreasing and reaches a fixed point
/// (manual §25.4, theorem `[E]`). Ground truth: §15.2 (|K^0..| = 25, 21, 19, 19).
#[test]
fn vt2_backward_iteration_monotone_to_fixed_point() {
    let grid = Grid::new(vec![0..=4, 0..=4]).unwrap();
    let report = kernel(&grid, &ExampleB);

    assert!(
        report.is_monotone_to_fixed_point(),
        "VT-2: size sequence is not monotone-decreasing to a fixed point: {:?}",
        report.sizes
    );
    // Explicit monotonicity check on the reported sequence.
    for w in report.sizes.windows(2) {
        assert!(w[0] >= w[1], "VT-2: |K| increased: {:?}", report.sizes);
    }
    // The §15.2 hand computation.
    assert_eq!(
        report.sizes,
        vec![25, 21, 19, 19],
        "VT-2: iteration sizes differ from §15.2"
    );
    assert_eq!(report.iterations, 3);

    assert_eq!(report.kernel.count(), 19);
}

/// Manual §15.1 (Example A) — the direct, non-negotiable `margin` fixture, using
/// the *real* `firma-plugin-constraint` plugins. `h = 0.180`, binding
/// `compliance` (ADR 0021 Part E). Lives here (not in `firma-viability`'s unit
/// tests) so the §15.1 reference literals stay out of a sim-path crate.
#[test]
fn margin_matches_section_15_1() {
    let state = FirmState {
        liquid_capital: 40,
        input_stock: 12,
        capability: 0.55,
        obligation: 30,
    };
    let aux = FirmAuxState {
        legitimacy: 1.0,
        regulated_intensity: 0.72, // u
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

    let cs = standard_constraints(30); // [solvency, compliance, scope, obligation]
    let terms: Vec<&dyn MarginTerm> = cs.iter().map(|c| c.as_ref() as &dyn MarginTerm).collect();

    // §15.1 table: g_j / s_j = -0.400, -0.180, -0.300, -1.400
    let scaled: Vec<f64> = terms.iter().map(|t| t.g(&ctx) / t.scale()).collect();
    for (got, want) in scaled.iter().zip([-0.400, -0.180, -0.300, -1.400]) {
        assert!((got - want).abs() < 1e-12, "§15.1: g/s = {scaled:?}");
    }

    let h = firma_viability::margin(&ctx, &terms);
    assert!(
        (h - 0.180).abs() < 1e-12,
        "§15.1: expected h = 0.180, got {h}"
    );

    // Binding = compliance (index 1) — the max of g/s, not the largest raw slack.
    let binding = scaled
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap()
        .0;
    assert_eq!(binding, 1, "§15.1: binding constraint should be compliance");
    assert_eq!(cs[binding].id().0, "constraint.compliance");
}

/// 1-D linear box: `x ∈ [0, 3]`, single action `x → x − 1` (admissible `x ≥ 1`).
/// Monotone decay with no way back ⇒ `Viab(K) = ∅`.
struct Decay1D;
impl Dynamics for Decay1D {
    fn successors(&self, point: &[i64]) -> Vec<Vec<i64>> {
        if point[0] >= 1 {
            vec![vec![point[0] - 1]]
        } else {
            Vec::new()
        }
    }
}

/// VT-1 — the exact viability kernel matches the analytic answer for a 1-D and
/// the §15.2 2-D linear box system (manual §25.4; ground truth §15.2).
#[test]
fn vt1_kernel_analytic_1d_and_2d() {
    // 1-D: pure monotone decay ⇒ empty kernel; sizes shrink to 0 and hold.
    // (`Grid::new` takes an owned `Vec` of ranges; a one-element vec is
    //  intentional here, not a mistaken range-to-elements init.)
    #[allow(clippy::single_range_in_vec_init)]
    let g1 = Grid::new(vec![0..=3]).unwrap();
    let r1 = kernel(&g1, &Decay1D);
    assert_eq!(
        r1.sizes,
        vec![4, 3, 2, 1, 0, 0],
        "VT-1 (1-D): iteration sizes"
    );
    assert_eq!(r1.kernel.count(), 0, "VT-1 (1-D): |Viab| = 0");
    assert!((r1.volume() - 0.0).abs() < 1e-12);

    // 2-D: §15.2 Example B — |Viab(K)| = 19, volume 19/25 = 0.76, fixed point at n = 2.
    let g2 = Grid::new(vec![0..=4, 0..=4]).unwrap();
    let r2 = kernel(&g2, &ExampleB);
    assert_eq!(
        r2.sizes,
        vec![25, 21, 19, 19],
        "VT-1 (2-D): iteration sizes = §15.2"
    );
    assert_eq!(r2.iterations, 3);
    assert_eq!(r2.kernel.count(), 19, "VT-1 (2-D): |Viab(K)| = 19");
    assert!(
        (r2.volume() - 0.76).abs() < 1e-12,
        "VT-1 (2-D): volume = 0.76"
    );

    let idx = |rl: i64, ri: i64| g2.index_of(&[rl, ri]).unwrap();
    // §15.2's excluded corners: low-resource (can't produce) and high-resource
    // (storage cap blocks the only admissible action).
    for (rl, ri) in [(0, 0), (1, 0), (4, 3), (4, 4), (0, 1), (3, 4)] {
        assert!(
            !r2.kernel.contains_index(idx(rl, ri)),
            "VT-1 (2-D): ({rl},{ri}) excluded"
        );
    }
    assert!(
        r2.kernel.contains_index(idx(2, 2)),
        "VT-1 (2-D): (2,2) viable"
    );
    // §15.2's key note: every excluded state has h > 0 for that abstract
    // system — the VT-3 divergence, demonstrated concretely there.
}

/// VT-3 — margin proxy vs. exact FIRMA-kernel membership (manual §25.4,
/// "Report correlation"; §9.2 caveat). **A reporting test, not a threshold.**
///
/// Runs `margin()` (all four §9.1 `g_j`) and the exact kernel (bounded by the
/// *lethal* constraints only — `solvency`, `compliance` — via
/// `ViolationSemantic::bounds_viability_kernel`) over a small FIRMA `d ≤ 4`
/// grid whose `c` range straddles `θ_cap`, and reports the contingency table
/// and φ-correlation between `h > 0` and kernel membership, at two `u_context`
/// values.
#[test]
fn vt3_margin_vs_exact_kernel_membership() {
    let theta = ConstraintParams {
        theta_limit: 0.90,
        theta_cap: 0.40,
        theta_q: 100,
    };
    let env = EnvParams {
        input_price: 3,
        output_price: 2,
    };
    let dynamics = FirmaDynamics::with_defaults(theta, env);

    // d ≤ 4 grid: r^L ∈ [0,12], r^I ∈ [0,6], c_index ∈ [0,12] (c ∈ {0.00 … 0.60},
    // straddling θ_cap = 0.40), q ∈ [0,2].
    let grid = Grid::new(vec![0..=12, 0..=6, 0..=12, 0..=2]).unwrap();

    let cs = standard_constraints(30);
    let lethal: Vec<&dyn MarginTerm> = cs
        .iter()
        .filter(|c| c.violation().bounds_viability_kernel())
        .map(|c| c.as_ref() as &dyn MarginTerm)
        .collect();
    let all: Vec<&dyn MarginTerm> = cs.iter().map(|c| c.as_ref() as &dyn MarginTerm).collect();

    println!("VT-3 — margin proxy vs. exact FIRMA kernel (§25.4, §9.2)");
    println!(
        "  grid ................... {} points  (r^L 0..12, r^I 0..6, c 0.00..0.60, q 0..2)",
        grid.size()
    );
    println!(
        "  θ ..................... limit={:.2} cap={:.2} Q={}",
        theta.theta_limit, theta.theta_cap, theta.theta_q
    );
    println!("  kernel-bounding constraints (ViolationSemantic::bounds_viability_kernel): solvency, compliance");

    for u_context in [0.0_f64, 0.95_f64] {
        let report = firma_kernel(&grid, &dynamics, &lethal, &theta, u_context);
        let kset = &report.kernel;
        let aux = FirmAuxState {
            legitimacy: 1.0,
            regulated_intensity: u_context,
            aspirations: Aspirations {
                capital_growth: 0.0,
                capability: 0.0,
                obligation_clearance: 0.0,
            },
        };

        let (mut hp_in, mut hp_out, mut hn_in, mut hn_out) = (0u64, 0u64, 0u64, 0u64);
        let mut ex_hpos_out: Option<FirmState> = None;
        let mut ex_hneg_in: Option<FirmState> = None;
        for i in 0..grid.size() {
            let s = dynamics.point_to_state(&grid.point(i));
            let ctx = ConstraintContext {
                state: &s,
                aux: &aux,
                theta: &theta,
            };
            let hpos = margin(&ctx, &all) > 0.0;
            let ink = in_kernel(kset, &grid, &dynamics, &s);
            match (hpos, ink) {
                (true, true) => hp_in += 1,
                (true, false) => {
                    hp_out += 1;
                    ex_hpos_out.get_or_insert(s);
                }
                (false, true) => {
                    hn_in += 1;
                    ex_hneg_in.get_or_insert(s);
                }
                (false, false) => hn_out += 1,
            }
        }
        let (a, b, c, d) = (hp_in as f64, hp_out as f64, hn_out as f64, hn_in as f64);
        let denom = ((a + b) * (c + d) * (a + d) * (b + c)).sqrt();
        let phi = if denom == 0.0 {
            f64::NAN
        } else {
            (a * c - b * d) / denom
        };

        println!(
            "  ── u_context = {u_context:.2} ({}) ──",
            if u_context <= theta.theta_limit {
                "compliance satisfiable"
            } else {
                "compliance violated everywhere"
            }
        );
        println!(
            "     kernel size ......... {} / {}  (volume {:.3}), backward-iter sizes {:?}",
            kset.count(),
            grid.size(),
            report.volume(),
            report.sizes
        );
        println!("     h>0 & in-kernel ..... {hp_in}");
        println!("     h>0 & OUT-of-kernel . {hp_out}");
        println!("     h<=0 & in-kernel .... {hn_in}");
        println!("     h<=0 & out-of-kernel  {hn_out}");
        println!("     φ(h>0, in-kernel) ... {phi:.4}");
        if let Some(s) = &ex_hneg_in {
            let ctx = ConstraintContext {
                state: s,
                aux: &aux,
                theta: &theta,
            };
            println!(
                "     ex. h<=0 but IN kernel: {s:?}  h={:.3}  (scope g_3 = {:.3} > 0, non-lethal)",
                margin(&ctx, &all),
                theta.theta_cap - s.capability
            );
        }
        match &ex_hpos_out {
            Some(s) => println!("     ex. h>0 but OUT of kernel: {s:?}"),
            None => println!("     (no h>0 state lies outside the kernel)"),
        }
        assert_eq!(hp_in + hp_out + hn_in + hn_out, grid.size() as u64);
    }

    println!(
        "  FINDING: at Stage 1 the market-only exact kernel is either the whole grid\n\
         \x20  (u_context <= θ_limit) or empty (u_context > θ_limit) — no backward-iteration\n\
         \x20  pruning occurs, because `hold` (§11.1 action 0) costs 0 and nothing in the\n\
         \x20  §11.1 core decays, so any state satisfying the lethal constraints (solvency,\n\
         \x20  compliance) survives indefinitely by holding. So `in_kernel <=>\n\
         \x20  lethal-constraints-satisfied`, and the φ-correlation with `h>0` is degenerate\n\
         \x20  at this stage. The divergence that IS present is the §9.2 caveat from the\n\
         \x20  other side: states with h<=0 (scope violated) that are perfectly viable,\n\
         \x20  because `scope` is never lethal (§9.1). The §15.2-direction divergence\n\
         \x20  (large h, outside the kernel) requires shock dynamics that can move θ out\n\
         \x20  from under a firm — that arrives in Stage 2 and VT-3 becomes a meaningful\n\
         \x20  correlation then. Reported per §25.4 ('Report correlation', not a threshold)."
    );
}

/// VT-6 — Conservation across a 10,000-tick run with all rules active
/// (manual §25.4). Exact (integers, §21.4). Also checks the Phase 1 gate line
/// "a 10,000-tick run with test plugins is byte-reproducible and conserves
/// exactly" (§26.3).
#[test]
fn vt6_conservation_over_10k_ticks() {
    let cfg = active_config(10_000);
    let reg = standard_registry();

    let d1 = scratch("vt6-a");
    let d2 = scratch("vt6-b");
    let r1 = execute_run(&cfg, &reg, &d1, &RunOptions::default()).unwrap();
    execute_run(
        &cfg,
        &reg,
        &d2,
        &RunOptions {
            parallel: true,
            write_snapshots: false,
        },
    )
    .unwrap();

    assert_eq!(r1.ticks, 10_000);
    assert!(r1.conservation_ok, "VT-6: kernel reported non-conservation");

    // Byte-reproducible (sequential vs parallel, DT-1 + DT-3 at 10k scale).
    assert_eq!(
        std::fs::read(d1.join("events.ndjson")).unwrap(),
        std::fs::read(d2.join("events.ndjson")).unwrap(),
        "VT-6 gate: the 10,000-tick run is not byte-reproducible"
    );

    // Conservation reconstructed from the event log alone (§22.2 log
    // sufficiency): fold every AdjustStock and confirm totals are unchanged.
    let v = verify_run(&d1).unwrap();
    assert!(
        v.conservation_from_log_ok,
        "VT-6: conservation does not hold when reconstructed from the event log"
    );
    assert!(v.ok(), "VT-6: full verification failed: {v:?}");

    let _ = std::fs::remove_dir_all(&d1);
    let _ = std::fs::remove_dir_all(&d2);
}

/// VT-7 — **every registered constraint-shaping plugin** is costly at
/// commitment, lagged by a drawn ≥ 1 amount, and probabilistic with
/// `p_max < 1` (manual §11.3; ADR 0023). The loop iterates
/// [`firma_plugin_action_shaping::shaping_catalog`] — a fourth shaping action is
/// covered automatically once it is added there.
#[test]
fn vt7_shaping_actions_are_costly_lagged_and_uncertain() {
    use std::collections::BTreeSet;

    use firma_core::{DeltaKind, DeltaTarget};
    use firma_domain::{keys, Edge, Effect, LaggedRecord};
    use firma_plugin_action_shaping::shaping_catalog;
    use firma_plugin_action_shaping::support::MockView;

    const COMMIT_TICK: u64 = 20;
    const SEEDS: u64 = 400;

    let mut any = false;
    for entry in shaping_catalog() {
        any = true;
        let params = (entry.reference_params)();

        // --- §11.3 property 3, *declared*: p_max < 1 strictly ---
        let model = (entry.success_model)(&params).expect("success model parses");
        assert!(
            model.p_max < 1.0,
            "VT-7 [{}]: declared p_max = {} is not < 1",
            entry.id,
            model.p_max
        );

        // --- §11.3 property 2, *declared*: lag drawn, Δ_min ≥ 1 ---
        let lag = (entry.lag_range)(&params).expect("lag range parses");
        assert!(
            lag.min >= 1,
            "VT-7 [{}]: declared Δ_min = {} is not ≥ 1",
            entry.id,
            lag.min
        );

        let rule = (entry.ctor)(&params).expect("rule builds");
        let idx = i64::from(entry.index);

        let (mut successes, mut failures) = (0usize, 0usize);
        let mut lags_seen: BTreeSet<u64> = BTreeSet::new();

        for seed in 0..SEEDS {
            let v = MockView::new(COMMIT_TICK, firma_core::Phase::ActShaping, seed, &[0])
                .with_stock(0, "capital", 1_000_000)
                .with_agent_real(0, keys::LEGITIMACY, 0.5)
                .with_agent_int(0, keys::SELECTED_ACTION, idx)
                // a supply partner, so `contract`'s precondition holds too
                .with_global_record(
                    keys::RELATION_EDGES,
                    serde_json::to_string(&Edge::supply(777, 0, 0)).unwrap(),
                );
            let deltas = rule.apply(&v, v.rng_key());

            // --- §11.3 property 1: cost paid at commitment, unconditionally ---
            let agent_cost: i64 = deltas
                .iter()
                .filter_map(|d| match (&d.target, &d.kind) {
                    (DeltaTarget::Agent(a), DeltaKind::AdjustStock { resource, amount })
                        if a.0 == 0 && resource.0 == "capital" =>
                    {
                        Some(*amount)
                    }
                    _ => None,
                })
                .sum();
            assert_eq!(
                agent_cost, -model.kappa_min,
                "VT-7 [{}]: cost not paid at commitment (seed {seed})",
                entry.id
            );
            let net_capital: i64 = deltas
                .iter()
                .filter_map(|d| match &d.kind {
                    DeltaKind::AdjustStock { resource, amount } if resource.0 == "capital" => {
                        Some(*amount)
                    }
                    _ => None,
                })
                .sum();
            assert_eq!(
                net_capital, 0,
                "VT-7 [{}]: cost is not conserving",
                entry.id
            );

            let rec = deltas
                .iter()
                .find_map(|d| match &d.kind {
                    DeltaKind::PushAgentRecord { list, record_json }
                        if list == keys::LAGGED_EFFECTS =>
                    {
                        Some(LaggedRecord::from_json(record_json).unwrap())
                    }
                    _ => None,
                })
                .expect("an enqueued lagged effect");

            // --- §11.3 property 2: maturity strictly after commitment ---
            assert!(
                rec.maturity_tick > COMMIT_TICK,
                "VT-7 [{}]: lag < 1 tick (seed {seed})",
                entry.id
            );
            assert!(
                rec.maturity_tick - COMMIT_TICK <= lag.max,
                "VT-7 [{}]: drawn lag exceeds Δ_max (seed {seed})",
                entry.id
            );
            lags_seen.insert(rec.maturity_tick - COMMIT_TICK);

            let applied = match rec.effect {
                Effect::Lobby { applied, .. }
                | Effect::Contract { applied, .. }
                | Effect::Diversify { applied, .. } => applied,
                Effect::CapabilityGain { .. } => unreachable!("not a shaping effect"),
            };
            if applied {
                successes += 1;
            } else {
                failures += 1;
            }
        }

        // --- §11.3 property 3, *empirical*: both outcomes occur, and the
        //     realised success rate never exceeds p_max ---
        assert!(
            successes > 0 && failures > 0,
            "VT-7 [{}]: not probabilistic over {SEEDS} seeds (success {successes}, fail {failures})",
            entry.id
        );
        let rate = successes as f64 / (successes + failures) as f64;
        assert!(
            rate <= model.p_max,
            "VT-7 [{}]: empirical success rate {rate:.3} exceeds p_max {}",
            entry.id,
            model.p_max
        );
        let p_expected = model.p_success(0.5, model.kappa_min);
        assert!(
            (rate - p_expected).abs() < 0.10,
            "VT-7 [{}]: empirical success {rate:.3} far from p_success {p_expected:.3}",
            entry.id
        );

        // --- §11.3 property 2, *empirical*: the lag is genuinely drawn ---
        if lag.min != lag.max {
            assert!(
                lags_seen.len() > 1,
                "VT-7 [{}]: lag never varied across {SEEDS} seeds — not drawn",
                entry.id
            );
        }

        println!(
            "VT-7 [{}]: {} seeds → {} success / {} fail (rate {:.3}, p_success {:.3}, p_max {}); \
             lags observed {:?} within [{}, {}]",
            entry.id,
            SEEDS,
            successes,
            failures,
            rate,
            p_expected,
            model.p_max,
            lags_seen,
            lag.min,
            lag.max,
        );
    }
    assert!(any, "VT-7: the shaping catalog is empty");
}

// ==========================================================================
// VT-4 — the inertia principle (§3.1, §12.3 Step 4 NONE row)
// ==========================================================================

/// VT-4 (§25.4, ground truth §3.1 "controls change only when required to
/// maintain viability"): with **no shortfall** (`max_j ς_j ≤ 0` ⇒ focus =
/// NONE) `decision.satisficing` repeats the previous action every tick, for
/// **every** `β` — NONE bypasses narrowing (Step 3) and the scan entirely
/// (Step 4's "repeat previous action"), so the inertia is `β`-independent.
///
/// Also checks the manual's literal statement (`β = 0`) as one row of that.
#[test]
fn vt4_beta_zero_no_shortfall_action_never_changes() {
    use firma_core::{DeltaKind, DeltaTarget, Rule};
    use firma_domain::keys;
    use firma_plugin_decision::support::MockView;
    use firma_plugin_decision::{Satisficing, SatisficingParams};

    // A comfortable firm: fat margin (r^L large, capability high, q = 0), and
    // NO aspirations seeded ⇒ every ς_j = 0 ⇒ max_j ς_j = 0 (not > 0) ⇒ NONE.
    // θ seeded slack (ADR 0015).
    fn view_at(tick: u64, prev_action: Option<i64>) -> MockView {
        let mut v = MockView::new(tick, firma_core::Phase::Decide, 1, &[0])
            .with_global_real(keys::THETA_LIMIT, 0.90)
            .with_global_real(keys::THETA_CAP, 0.40)
            .with_global_int(keys::THETA_Q, 100)
            .with_global_int(keys::INPUT_PRICE, 2)
            .with_global_int(keys::OUTPUT_PRICE, 3)
            .with_stock(0, "capital", 400)
            .with_stock(0, "input", 10)
            .with_agent_real(0, keys::CAPABILITY, 0.9)
            .with_agent_int(0, keys::OBLIGATION, 0);
        if let Some(a) = prev_action {
            v = v.with_agent_int(0, keys::SELECTED_ACTION, a);
        }
        v
    }

    fn selected(deltas: &[firma_core::Delta]) -> i64 {
        deltas
            .iter()
            .find_map(|d| match (&d.target, &d.kind) {
                (DeltaTarget::Agent(a), DeltaKind::SetAgentInt { field, value })
                    if a.0 == 0 && field == keys::SELECTED_ACTION =>
                {
                    Some(*value)
                }
                _ => None,
            })
            .expect("decision.satisficing selects an action")
    }

    const TICKS: u64 = 60;
    let betas = [0.0_f64, 0.5, 1.0, 2.0, 4.0];

    println!("VT-4 — inertia principle: focus=NONE ⇒ repeat previous, for every β");
    for &beta in &betas {
        let rule = Satisficing::new(SatisficingParams {
            beta,
            ..Default::default()
        })
        .unwrap();

        // Case A: no previous action at all ⇒ tick 0 falls back to `hold` (0),
        // then repeats it forever.
        let mut prev: Option<i64> = None;
        let mut seq = Vec::new();
        for t in 0..TICKS {
            let v = view_at(t, prev);
            let a = selected(&rule.apply(&v, v.rng_key()));
            seq.push(a);
            prev = Some(a);
        }
        assert!(
            seq.iter().all(|&a| a == 0),
            "VT-4 (β={beta}, no prior): expected all hold(0), got {seq:?}"
        );

        // Case B: seed a non-trivial previous action (produce_regulated = 2).
        // Inertia = it keeps doing exactly that, unchanged, every tick.
        let mut prev = Some(2_i64);
        let mut seq = Vec::new();
        for t in 0..TICKS {
            let v = view_at(t, prev);
            let a = selected(&rule.apply(&v, v.rng_key()));
            seq.push(a);
            prev = Some(a);
        }
        assert!(
            seq.iter().all(|&a| a == 2),
            "VT-4 (β={beta}, prior=2): action changed: {seq:?}"
        );
        let changes = seq.windows(2).filter(|w| w[0] != w[1]).count();
        println!(
            "  β={beta:<3}  no-prior → constant hold(0) for {TICKS} ticks; \
             prior=2 → constant produce_regulated(2), {changes} changes"
        );
    }
}

// ==========================================================================
// VT-5 — decision.random establishes the null (§25.4; ADR 0027)
// ==========================================================================

/// VT-5 (§25.4, "Establishes the null"): under `decision.random` the realised
/// choice is **uniform over the admissible set at every margin**. So the
/// choice carries no information about `h` *beyond which actions are available*
/// — and admissible-set composition is the §28.3 alternative explanation the
/// experiment wants visible, not something the decision plugin should mask.
///
/// A **reporting** test (like VT-3). Per grid cell it measures the total
/// variation `d_TV` between the realised action distribution and the
/// uniform-over-admissible distribution; the null holds iff (a) `d_TV` is
/// finite-sample small at *every* margin and (b) the `h ↔ d_TV` Pearson
/// correlation's bootstrap 95 % CI contains zero. "Indistinguishable from
/// zero" ≡ that CI contains zero (2000 resamples, deterministic RNG). It also
/// reports `r(h, |A_adm|)` — the admissibility channel — which is allowed to
/// be non-zero and is the point of §28.3.
#[test]
fn vt5_decision_random_establishes_the_null() {
    use firma_core::{DeltaKind, DeltaTarget, Rule};
    use firma_domain::keys;
    use firma_domain::margin::standard_margin;
    use firma_domain::{
        Aspirations, ConstraintContext, ConstraintParams, FirmAuxState, FirmState, ScaleFactors,
    };
    use firma_plugin_decision::support::MockView;
    use firma_plugin_decision::{DecisionRandom, RandomParams};

    // Grid spanning slack↔tight: vary r^L (solvency-h), u vs θ_limit
    // (compliance-h), and capability (scope-h + invest_capability
    // affordability), so both `h` and the admissible-set composition move.
    let capitals: [i64; 5] = [3, 12, 30, 90, 300];
    let u_gaps: [f64; 3] = [0.02, 0.12, 0.30]; // θ_limit − u
    let caps: [f64; 2] = [0.30, 0.85]; // below / above θ_cap = 0.40

    fn margin_of(r_l: i64, u: f64, cap: f64, theta_limit: f64) -> f64 {
        let s = FirmState {
            liquid_capital: r_l,
            input_stock: 8,
            capability: cap,
            obligation: 2,
        };
        let aux = FirmAuxState {
            legitimacy: 1.0,
            regulated_intensity: u,
            aspirations: Aspirations {
                capital_growth: 0.0,
                capability: 0.0,
                obligation_clearance: 0.0,
            },
        };
        let theta = ConstraintParams {
            theta_limit,
            theta_cap: 0.40,
            theta_q: 100,
        };
        standard_margin(
            &ConstraintContext {
                state: &s,
                aux: &aux,
                theta: &theta,
            },
            &ScaleFactors::default(),
        )
    }

    fn view_cell(
        tick: u64,
        cell_seed: u64,
        r_l: i64,
        u: f64,
        cap: f64,
        theta_limit: f64,
    ) -> MockView {
        MockView::new(tick, firma_core::Phase::Decide, cell_seed, &[0])
            .with_global_real(keys::THETA_LIMIT, theta_limit)
            .with_global_real(keys::THETA_CAP, 0.40)
            .with_global_int(keys::THETA_Q, 100)
            .with_global_int(keys::INPUT_PRICE, 2)
            .with_global_int(keys::OUTPUT_PRICE, 3)
            .with_stock(0, "capital", r_l)
            .with_stock(0, "input", 8)
            .with_agent_real(0, keys::CAPABILITY, cap)
            .with_agent_real(0, keys::REGULATED_INTENSITY, u)
            .with_agent_int(0, keys::OBLIGATION, 2)
    }

    fn selected(deltas: &[firma_core::Delta]) -> u8 {
        deltas
            .iter()
            .find_map(|d| match (&d.target, &d.kind) {
                (DeltaTarget::Agent(a), DeltaKind::SetAgentInt { field, value })
                    if a.0 == 0 && field == keys::SELECTED_ACTION =>
                {
                    u8::try_from(*value).ok()
                }
                _ => None,
            })
            .expect("decision.random selects an action")
    }

    fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
        let n = xs.len() as f64;
        let mx = xs.iter().sum::<f64>() / n;
        let my = ys.iter().sum::<f64>() / n;
        let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
        for (x, y) in xs.iter().zip(ys) {
            sxy += (x - mx) * (y - my);
            sxx += (x - mx).powi(2);
            syy += (y - my).powi(2);
        }
        if sxx == 0.0 || syy == 0.0 {
            0.0
        } else {
            sxy / (sxx.sqrt() * syy.sqrt())
        }
    }

    const DRAWS: u64 = 2000;
    let rule = DecisionRandom::new(RandomParams::default()).unwrap();

    struct Cell {
        h: f64,
        adm: usize,
        d_tv: f64,
    }
    let mut cells: Vec<Cell> = Vec::new();

    println!("VT-5 — decision.random null (ADR 0027); {DRAWS} draws/cell");
    println!(
        "  {:>8}  {:>7}  {:>8}   realised counts over 0..8",
        "h", "|A_adm|", "d_TV"
    );

    let mut cell_seed = 0x5EED_5000_u64;
    for &r_l in &capitals {
        for &gap in &u_gaps {
            for &cap in &caps {
                cell_seed = cell_seed.wrapping_add(0x9E37_79B9);
                let theta_limit = 0.90;
                let u = theta_limit - gap;
                let h = margin_of(r_l, u, cap, theta_limit);

                let mut counts = [0u64; 9];
                for t in 0..DRAWS {
                    let v = view_cell(t, cell_seed, r_l, u, cap, theta_limit);
                    counts[selected(&rule.apply(&v, v.rng_key())) as usize] += 1;
                }
                let n = DRAWS as f64;
                let adm = counts.iter().filter(|&&c| c > 0).count().max(1);
                // d_TV( realised , uniform-over-realised-support )
                let uniform = 1.0 / adm as f64;
                let d_tv: f64 = counts
                    .iter()
                    .filter(|&&c| c > 0)
                    .map(|&c| (c as f64 / n - uniform).abs())
                    .sum::<f64>()
                    * 0.5;

                println!("  {h:8.3}  {adm:7}  {d_tv:8.4}   {counts:?}");
                cells.push(Cell { h, adm, d_tv });
            }
        }
    }

    let hs: Vec<f64> = cells.iter().map(|c| c.h).collect();
    let tvs: Vec<f64> = cells.iter().map(|c| c.d_tv).collect();
    let adms: Vec<f64> = cells.iter().map(|c| c.adm as f64).collect();

    let r_tv = pearson(&hs, &tvs);
    let r_adm = pearson(&hs, &adms);

    // Bootstrap 95% CI on r(h, d_TV): resample cells with replacement,
    // deterministic RNG, 2000 iterations.
    let mut rng = firma_rng::open(
        &firma_rng::key(
            0x5EED_0005,
            firma_core::StreamId::Mechanism,
            0,
            firma_core::Phase::Decide,
            0,
            None,
        ),
        "vt5_bootstrap",
    );
    let m = cells.len();
    let mut rs: Vec<f64> = Vec::with_capacity(2000);
    for _ in 0..2000 {
        let mut bx = Vec::with_capacity(m);
        let mut by = Vec::with_capacity(m);
        for _ in 0..m {
            let idx = rng.next_below(m as u64) as usize;
            bx.push(hs[idx]);
            by.push(tvs[idx]);
        }
        rs.push(pearson(&bx, &by));
    }
    rs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo = rs[(0.025 * rs.len() as f64) as usize];
    let hi = rs[(0.975 * rs.len() as f64) as usize];
    let max_tv = tvs.iter().cloned().fold(0.0_f64, f64::max);

    println!("\n  {} cells", cells.len());
    println!(
        "  max d_TV over all cells   = {max_tv:.4}  (choice is uniform-over-admissible at every h)"
    );
    println!("  r(h, d_TV)                = {r_tv:+.3}   bootstrap 95% CI [{lo:+.3}, {hi:+.3}]  ← the null");
    println!("  r(h, |A_adm|)             = {r_adm:+.3}   (§28.3 admissibility channel — allowed to be non-zero)");

    // (a) uniform-over-admissible at every margin: finite-sample TV small.
    //     2000 draws over ≤6 categories ⇒ expected TV ~ 0.02–0.05.
    for c in &cells {
        assert!(
            c.d_tv < 0.08,
            "VT-5: choice not uniform-over-admissible at h={:.3} (d_TV={:.4})",
            c.h,
            c.d_tv
        );
    }
    // (b) no h ↔ (narrowing) rigidity: bootstrap CI of r(h, d_TV) contains 0.
    assert!(
        lo <= 0.0 && hi >= 0.0,
        "VT-5: r(h, d_TV) 95% CI [{lo:+.3}, {hi:+.3}] excludes 0 — not a clean null"
    );
}

// --------------------------------------------------------------------------
// VT-8 — independence of h and ς (manual §25.4; ADR 0040)
// --------------------------------------------------------------------------

/// VT-8 (§25.4): across a constructed grid, `h` and `ς` can be set to any
/// combination and neither mechanically determines the other. This is the
/// **precondition** for E1 — "without VT-8, H1a and H1b are guaranteed by
/// construction" (§2.5).
///
/// The harness feeds grid-constructed `(h, ς)` pairs — which never touch real
/// firm state — into the **same** `firma_plugin_decision::select` the running
/// model calls (ADR 0040). Criterion (iii)'s structural half is the signature
/// of that function (`h: f64` and `shortfalls: [f64; 3]` are independent
/// parameters, no path between them in the body); this test covers (i) and
/// (ii) empirically, and re-states (iii) with the two directional greps from
/// the ADR.
#[test]
fn vt8_orthogonal_manipulation_of_h_and_shortfall() {
    use firma_plugin_decision::{select, Focus};

    fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
        let n = xs.len() as f64;
        let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
        let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
        for (x, y) in xs.iter().zip(ys) {
            sxy += (x - mx) * (y - my);
            sxx += (x - mx).powi(2);
            syy += (y - my).powi(2);
        }
        if sxx == 0.0 || syy == 0.0 {
            0.0
        } else {
            sxy / (sxx.sqrt() * syy.sqrt())
        }
    }

    // Grid: h straddles h_crit = 0.15 on both sides and goes below 0;
    // ς_1 is positive and non-positive. ς_2 = ς_3 = 0 so max_j ς_j = max(ς_1, 0).
    let h_levels = [-0.05_f64, 0.02, 0.05, 0.10, 0.15, 0.20, 0.40];
    let s_levels = [-1.0_f64, -0.25, 0.0, 0.25, 0.50, 1.0, 2.0];
    let h_crit = 0.15_f64;
    let beta = 1.0_f64;
    let w_max = 6_u32;

    // A `satisfices` stub that reintroduces **no** h ↔ ς coupling: nothing
    // satisfices, so `select` returns the scan-order fallback — the output is
    // then a pure function of (focus, w_eff), i.e. of (h, ς) alone.
    let admissible = |a: u8| a <= 5; // the six market actions, all admissible
    let satisfices = |_a: u8, _f: Focus| false;

    struct Cell {
        h: f64,
        smax: f64,
        focus_code: i64,
        w_eff: u32,
    }
    let mut cells = Vec::new();
    for &h in &h_levels {
        for &s1 in &s_levels {
            let sel = select(
                h,
                [s1, 0.0, 0.0],
                beta,
                h_crit,
                w_max,
                0,
                admissible,
                satisfices,
            );
            cells.push(Cell {
                h,
                smax: s1.max(0.0),
                focus_code: sel.focus.code(),
                w_eff: sel.w_eff,
            });
        }
    }

    let hs: Vec<f64> = cells.iter().map(|c| c.h).collect();
    let ss: Vec<f64> = cells.iter().map(|c| c.smax).collect();
    let we: Vec<f64> = cells.iter().map(|c| f64::from(c.w_eff)).collect();
    let fc: Vec<f64> = cells.iter().map(|c| c.focus_code as f64).collect();

    let r_hs = pearson(&hs, &ss);
    let r_h_we = pearson(&hs, &we);
    let r_s_we = pearson(&ss, &we);
    let r_h_fc = pearson(&hs, &fc);
    let r_s_fc = pearson(&ss, &fc);

    // The **narrowing width** — Step 3's ψ(h) → w_eff formula, taken directly,
    // not the reported `Selection.w_eff` (which is 0 on a NONE tick, a Step-2
    // branch). This is the quantity that must be a pure function of h.
    let mut width_by_h: std::collections::BTreeMap<i64, std::collections::BTreeSet<u32>> =
        std::collections::BTreeMap::new();
    for &h in &h_levels {
        let raw_width =
            firma_plugin_decision::w_eff(firma_plugin_decision::psi(h, h_crit, beta), w_max);
        for &s1 in &s_levels {
            let sel = select(
                h,
                [s1, 0.0, 0.0],
                beta,
                h_crit,
                w_max,
                0,
                admissible,
                satisfices,
            );
            // when a scan happens, the reported w_eff IS the Step-3 width.
            if sel.focus != Focus::None {
                assert_eq!(
                    sel.w_eff, raw_width,
                    "VT-8: reported w_eff diverged from the ς-independent Step-3 width at h={h}, ς={s1}"
                );
            }
            width_by_h
                .entry((h * 1000.0) as i64)
                .or_default()
                .insert(raw_width);
        }
    }
    // For each h, the narrowing width is a single value across all ς.
    for (h_key, widths) in &width_by_h {
        assert_eq!(
            widths.len(),
            1,
            "VT-8: narrowing width varies with ς at h={}: {widths:?}",
            *h_key as f64 / 1000.0
        );
    }

    // Criterion (ii): the 2×2 partition of §12.3 Step 2 — (h < h_crit) × (max_j ς_j > 0).
    let mut quad = [0usize; 4];
    for c in &cells {
        let lo_h = (c.h < h_crit) as usize;
        let pos_s = (c.smax > 0.0) as usize;
        quad[lo_h * 2 + pos_s] += 1;
    }

    println!(
        "\n  VT-8 grid: {} cells ({} h-levels × {} ς-levels)",
        cells.len(),
        h_levels.len(),
        s_levels.len()
    );
    println!(
        "  (i)  r(h, ς)            = {r_hs:+.6}   ← MUST be ≈ 0 by construction (the two INPUTS)"
    );
    println!("       narrowing width ψ(h)→w_eff is a single value per h-level across all ς  ✓");
    println!("  descriptive (a real H1a needs BOTH channels present, NEITHER mechanical):");
    println!("       r(ς, Selection.w_eff) = {r_s_we:+.3}   (ς → focus → whether a scan happens)");
    println!("       r(h, Selection.w_eff) = {r_h_we:+.3}   (h → narrowing width)");
    println!("       r(ς, focus_code)      = {r_s_fc:+.3}");
    println!("       r(h, focus_code)      = {r_h_fc:+.3}");
    println!(
        "  (ii) quadrant populations (h≥h_crit,ς≤0)/(h≥h_crit,ς>0)/(h<h_crit,ς≤0)/(h<h_crit,ς>0)"
    );
    println!("       = {}/{}/{}/{}", quad[0], quad[1], quad[2], quad[3]);

    // (i) — a full factorial of two independent factors has exactly zero
    // sample correlation between the factors (the two INPUTS are what must be
    // independently settable — not the outputs, which H1a says ς and h both
    // legitimately move).
    assert!(
        r_hs.abs() < 1e-9,
        "VT-8 (i): r(h, ς) = {r_hs:+.9}, not ≈ 0 — the grid is not orthogonal"
    );
    // (ii) — every quadrant populated.
    assert!(
        quad.iter().all(|&n| n > 0),
        "VT-8 (ii): a quadrant is empty: {quad:?}"
    );
    // The mechanism must actually be present, or the grid proves nothing: h
    // genuinely drives the narrowing width, ς genuinely drives focus — both
    // non-zero, so H1a/H1b are real claims and not vacuous.
    assert!(
        r_h_we.abs() > 0.2 && r_s_we.abs() > 0.2,
        "VT-8 sanity: a channel is missing (r(h,w)={r_h_we:+.3}, r(ς,w)={r_s_we:+.3}) — grid uninformative"
    );
}

// --------------------------------------------------------------------------
// SC-4/SC-5 follow-up — does a wide `w_max` / any `β` open the shaping channel
// under `decision.satisficing`? (Stage-6 follow-up; ADR 0040 §"SC finding")
// --------------------------------------------------------------------------

/// **Structural check.** A shaping action (6/7/8) is selected by
/// `decision.satisficing` **iff** it is the scan *fallback* — the first
/// admissible action in the focus priority order — because a shaping action
/// **never satisfices any GOAL** (its one-step lookahead, `shaping_cost_step`,
/// only subtracts cost: `Δv_1 = −κ_a < 0 ≤ ς_1`, `Δv_2 = Δv_3 = 0 < ς_j`).
/// `select`'s fallback (`order.iter().find(admissible)`) scans the **full**
/// order, unbounded by `w`. So `w_max` and `β`:
///   * do nothing for a GOAL firm's `w_eff` (`ψ(h) = 1` for `h ≥ h_crit`, so
///     `w_eff = w_max` regardless of `β`);
///   * change only *which market action satisfices first*, never whether
///     shaping is picked.
///
/// Shaping is therefore reachable only when every market action **ahead of it
/// in the GOAL order is inadmissible** — an input-starved, can't-afford-input
/// firm. This test drives `select` directly across the full §16.1
/// `β × w_max` sweep to show that.
#[test]
fn sc4_shaping_selected_only_as_the_goal_fallback_never_by_the_scan() {
    use firma_plugin_decision::{select, Focus};

    let betas = [0.0_f64, 0.5, 1.0, 2.0, 4.0]; // §16.1 sweep
    let w_maxes = [3_u32, 6, 9]; // §16.1 sweep
    let h_crit = 0.15;
    let h_goal = 0.40; // comfortably ≥ h_crit ⇒ ψ = 1 ⇒ w_eff = w_max for every β

    // ς shapes: GOAL(1), GOAL(2), GOAL(3), each with a shortfall too large for
    // any single action to close.
    let goal_cases: [(&str, [f64; 3]); 3] = [
        ("GOAL(1)", [5.0, 0.0, 0.0]),
        ("GOAL(2)", [0.0, 5.0, 0.0]),
        ("GOAL(3)", [0.0, 0.0, 5.0]),
    ];
    // "satisfices" stub: nothing ever satisfices (shortfall too big).
    let never = |_a: u8, _f: Focus| false;

    // (a) HEALTHY firm — every action admissible, including shaping 6/7/8.
    println!("\n  (a) healthy firm (all 9 actions admissible):");
    for (name, sc) in goal_cases {
        let mut picks = std::collections::BTreeSet::new();
        for &b in &betas {
            for &wm in &w_maxes {
                let s = select(h_goal, sc, b, h_crit, wm, 0, |_| true, never);
                assert_eq!(s.w_eff, wm, "{name}: ψ=1 for h≥h_crit ⇒ w_eff = w_max");
                picks.insert(s.action);
            }
        }
        let first_market = *Focus::attend(h_goal, sc, h_crit)
            .scan_order()
            .first()
            .unwrap();
        println!(
            "    {name}: picks over all (β,w_max) = {picks:?}  (first-in-order = {first_market})"
        );
        assert_eq!(
            picks,
            std::collections::BTreeSet::from([first_market]),
            "{name}: a healthy firm always takes the first-in-order action, never shaping, at any β/w_max"
        );
        assert!(
            !picks.iter().any(|a| (6..=8).contains(a)),
            "{name}: no shaping for a healthy firm"
        );
    }

    // (b) INPUT-STARVED firm — market actions 0..=5 all inadmissible, shaping
    //     6/7/8 admissible. Now the GOAL fallback CAN be shaping.
    println!("\n  (b) input-starved firm (only shaping 6/7/8 admissible):");
    let only_shaping = |a: u8| (6..=8).contains(&a);
    for (name, sc) in goal_cases {
        let mut picks = std::collections::BTreeSet::new();
        for &b in &betas {
            for &wm in &w_maxes {
                picks.insert(select(h_goal, sc, b, h_crit, wm, 0, only_shaping, never).action);
            }
        }
        // fallback = first admissible in the focus order.
        let order = Focus::attend(h_goal, sc, h_crit).scan_order();
        let expected = *order.iter().find(|a| only_shaping(**a)).unwrap();
        println!(
            "    {name}: order={order:?}  → picks = {picks:?}  (expected fallback = {expected})"
        );
        assert_eq!(
            picks,
            std::collections::BTreeSet::from([expected]),
            "{name}: fallback is the first admissible in the order (a shaping action here)"
        );
        assert!(picks.iter().all(|a| (6..=8).contains(a)));
    }

    println!(
        "\n  ⇒ w_max/β do not open the shaping channel: a shaping action is picked only when\n\
         \x20   every market action ahead of it in the GOAL order is inadmissible (input-starved firm)."
    );
}
