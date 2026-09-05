//! Unit tests for the decision plugins (manual §12.1, §12.3; ADR 0027).

use firma_core::{DeltaKind, DeltaTarget, Phase};
use firma_domain::keys;

use super::support::MockView;
use super::*;

// --- §12.3 Step 3: ψ and w_eff ---

#[test]
fn psi_matches_section_12_3_piecewise() {
    // h >= h_crit  ⇒ ψ = 1  (any β)
    assert!((psi(0.20, 0.15, 1.0) - 1.0).abs() < 1e-12);
    assert!((psi(0.15, 0.15, 4.0) - 1.0).abs() < 1e-12);
    // 0 <= h < h_crit ⇒ (h/h_crit)^β
    assert!((psi(0.075, 0.15, 1.0) - 0.5).abs() < 1e-12); // (0.5)^1
    assert!((psi(0.075, 0.15, 2.0) - 0.25).abs() < 1e-12); // (0.5)^2
                                                           // β = 0 ⇒ ψ ≡ 1 with NO special-case branch — falls out of x^0 = 1
    assert!((psi(0.075, 0.15, 0.0) - 1.0).abs() < 1e-12);
    assert!((psi(0.0, 0.15, 0.0) - 1.0).abs() < 1e-12);
    // h < 0 (past the boundary), β > 0 ⇒ max(h,0)=0 ⇒ ψ = 0
    assert!(psi(-0.3, 0.15, 1.0).abs() < 1e-12);
    // h < 0, β = 0 ⇒ 0^0 = 1 (IEEE) ⇒ still the null
    assert!((psi(-0.3, 0.15, 0.0) - 1.0).abs() < 1e-12);
}

#[test]
fn w_eff_is_bounded_1_to_wmax() {
    assert_eq!(w_eff(1.0, 6), 6); // no narrowing
    assert_eq!(w_eff(0.5, 6), 3); // ⌈3⌉
    assert_eq!(w_eff(0.1, 6), 1); // ⌈0.6⌉ = 1
    assert_eq!(w_eff(0.0, 6), 1); // max(1, 0)
    assert_eq!(w_eff(1.0, 3), 3);
    assert_eq!(w_eff(0.34, 9), 4); // ⌈3.06⌉
}

// --- §12.3 Step 4: scan-order table transcribed exactly ---

#[test]
fn scan_orders_are_the_section_12_3_table() {
    assert_eq!(Focus::Survival.scan_order(), &[1, 3, 5, 2, 0, 4, 6, 7, 8]);
    assert_eq!(Focus::Goal(1).scan_order(), &[2, 1, 3, 6, 8, 5, 4, 0, 7]);
    assert_eq!(Focus::Goal(2).scan_order(), &[4, 1, 3, 2, 6, 5, 0, 8, 7]);
    assert_eq!(Focus::Goal(3).scan_order(), &[5, 3, 1, 7, 2, 8, 6, 0, 4]);
    assert!(Focus::None.scan_order().is_empty());
    // every market order is a permutation of 0..=8
    for f in [
        Focus::Survival,
        Focus::Goal(1),
        Focus::Goal(2),
        Focus::Goal(3),
    ] {
        let mut o = f.scan_order().to_vec();
        o.sort_unstable();
        assert_eq!(o, (0..=8).collect::<Vec<_>>());
    }
    // shaping (6,7,8) sit last under SURVIVAL — the §12.3 R3 note
    assert_eq!(&Focus::Survival.scan_order()[6..], &[6, 7, 8]);
}

// --- helpers for the rule tests ---

fn seed_common(v: MockView) -> MockView {
    // θ seeded (ADR 0015): slack — nothing binds.
    v.with_global_real(keys::THETA_LIMIT, 0.90)
        .with_global_real(keys::THETA_CAP, 0.40)
        .with_global_int(keys::THETA_Q, 100)
        .with_global_int(keys::INPUT_PRICE, 2)
        .with_global_int(keys::OUTPUT_PRICE, 3)
}

fn selected(deltas: &[Delta], agent: u64) -> Option<i64> {
    deltas.iter().find_map(|d| match (&d.target, &d.kind) {
        (DeltaTarget::Agent(a), DeltaKind::SetAgentInt { field, value })
            if a.0 == agent && field == keys::SELECTED_ACTION =>
        {
            Some(*value)
        }
        _ => None,
    })
}

fn field_value(deltas: &[Delta], agent: u64, field: &str) -> Option<i64> {
    deltas.iter().find_map(|d| match (&d.target, &d.kind) {
        (DeltaTarget::Agent(a), DeltaKind::SetAgentInt { field: f, value })
            if a.0 == agent && f == field =>
        {
            Some(*value)
        }
        _ => None,
    })
}

// --- §12.3 Step 2: Attend ---

#[test]
fn attend_survival_when_margin_thin() {
    // h_crit default 0.15. Make g_1 = -r^L bind: r^L small so h < h_crit.
    // s_L = 100 ⇒ g_1/s_1 = -r^L/100. h ≈ -max(...) ; with r^L = 5, θ slack,
    // capability high ⇒ scope slack ⇒ h ≈ 5/100 = 0.05 < 0.15.
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 5)
        .with_stock(0, "input", 3)
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 0);
    let d = Satisficing::new(SatisficingParams::default())
        .unwrap()
        .apply(&v, v.rng_key());
    assert_eq!(
        field_value(&d, 0, keys::FOCUS),
        Some(Focus::Survival.code())
    );
    assert_eq!(field_value(&d, 0, keys::FOCUS), Some(0));
}

#[test]
fn attend_none_when_no_shortfall_and_margin_fat() {
    // Fat margin, aspirations met (absent ⇒ ς = 0) ⇒ focus NONE ⇒ repeat prev.
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 400)
        .with_stock(0, "input", 10)
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 0)
        .with_agent_int(0, keys::SELECTED_ACTION, 2); // previously produce_regulated
    let d = Satisficing::new(SatisficingParams::default())
        .unwrap()
        .apply(&v, v.rng_key());
    assert_eq!(field_value(&d, 0, keys::FOCUS), Some(Focus::None.code()));
    assert_eq!(selected(&d, 0), Some(2), "NONE ⇒ repeat previous action");
    assert_eq!(
        field_value(&d, 0, keys::W_EFF),
        Some(0),
        "no scan on a NONE tick"
    );
}

#[test]
fn attend_goal_argmax_ties_to_lowest_j() {
    // Fat margin. Give an obligation shortfall (goal 3) and a capital shortfall
    // (goal 1) that are equal ⇒ tie ⇒ lowest j = GOAL(1).
    // ς_1 = A_1 - v_1 ; v_1 = REALIZED_CAPITAL_GROWTH (absent ⇒ 0) ⇒ ς_1 = A_1.
    // ς_3 = A_3 - v_3 ; v_3 = -q. q=10 ⇒ v_3 = -10 ⇒ ς_3 = A_3 + 10.
    // Want ς_1 == ς_3 > 0: A_1 = 5, A_3 = -5 ⇒ ς_1 = 5, ς_3 = 5.
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 400)
        .with_stock(0, "input", 10)
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 10)
        .with_agent_real(0, keys::ASPIRATION_CAPITAL_GROWTH, 5.0)
        .with_agent_real(0, keys::ASPIRATION_OBLIGATION_CLEARANCE, -5.0);
    let d = Satisficing::new(SatisficingParams::default())
        .unwrap()
        .apply(&v, v.rng_key());
    assert_eq!(field_value(&d, 0, keys::FOCUS), Some(1), "tie → GOAL(1)");
}

// --- §12.3 Step 5: satisficing selection ---

#[test]
fn goal1_first_satisficing_is_not_argmax() {
    // GOAL(1) capital, order [2,1,3,6,8,5,4,0,7]. Big shortfall so nothing
    // trivially satisfices; produce_regulated (2) yields more than
    // produce_ordinary (1), but the firm must take the FIRST that satisfices in
    // scan order, and 2 comes first anyway — so instead make 2 inadmissible
    // (no input) and check it picks 1 (next), not 3/6/8 with bigger Δ.
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 400)
        .with_stock(0, "input", 1) // exactly one input
        .with_agent_real(0, keys::CAPABILITY, 0.5)
        .with_agent_int(0, keys::OBLIGATION, 0)
        .with_agent_real(0, keys::ASPIRATION_CAPITAL_GROWTH, 3.0); // ς_1 = 3
                                                                   // produce_ordinary (1): y_O(0.5)=⌊4·1.4⌋=5, π^O=3 ⇒ Δr^L = 15 ≥ 3 ✓
    let d = Satisficing::new(SatisficingParams::default())
        .unwrap()
        .apply(&v, v.rng_key());
    assert_eq!(field_value(&d, 0, keys::FOCUS), Some(1));
    // order is 2,1,3,...; 2 needs input≥1 AND c≥θ_cap(0.4): c=0.5 ok, input=1 ok
    // ⇒ 2 IS admissible and satisfices (Δ = π^O·y_R = 3·8 = 24 ≥ 3) ⇒ picks 2.
    assert_eq!(selected(&d, 0), Some(2));
}

#[test]
fn inadmissible_actions_do_not_consume_scan_budget() {
    // GOAL(3) obligation, order [5,3,1,7,2,8,6,0,4]. w_eff small (narrow).
    // deliver (5) needs q≥1 AND input≥1. Give q≥1 but input=0 ⇒ 5 inadmissible.
    // With β large and thin-ish margin, w_eff = 1: if 5 consumed the budget the
    // scan would stop with the fallback; instead 5 is skipped (not counted) and
    // 3 (acquire_input) is scanned. acquire_input needs r^L ≥ π^I and storage
    // room — give both ⇒ admissible. It does not clear obligation so it won't
    // *satisfice* GOAL(3) (Δv_3 = 0 < ς_3), but it IS the first admissible ⇒
    // fallback picks it. The point: the run doesn't error and 5 didn't burn the
    // single budget slot on nothing.
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 400)
        .with_stock(0, "input", 0)
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 5)
        .with_agent_real(0, keys::ASPIRATION_OBLIGATION_CLEARANCE, 0.0); // ς_3 = 0 - (-5) = 5
    let params = SatisficingParams {
        beta: 4.0,
        ..Default::default()
    };
    let d = Satisficing::new(params).unwrap().apply(&v, v.rng_key());
    assert_eq!(field_value(&d, 0, keys::FOCUS), Some(3));
    // scanned within w_eff=? margin is fat here (r^L=400) so h ≥ h_crit ⇒
    // ψ=1 ⇒ w_eff = w_max = 6. order 5(inadm),3(adm, doesn't satisfice),
    // 1(inadm no input),7(shaping off),2(inadm no input),8(off) ... fallback
    // = first admissible = 3.
    assert_eq!(selected(&d, 0), Some(3));
}

#[test]
fn narrowing_shrinks_the_scanned_window() {
    // Same firm, β=0 vs β=4, thin margin. With β=0 the full order is scanned;
    // with β=4 and thin h, w_eff collapses to 1 and only the top-priority
    // survival action is considered.
    let base = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 8) // thin: h ≈ 0.08 < 0.15
        .with_stock(0, "input", 0) // produce_ordinary (order[0]=1) inadmissible
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 0);
    // SURVIVAL order [1,3,5,2,0,4,...]. 1 inadmissible (no input).
    //   β=4, h≈0.08 ⇒ ψ = (0.08/0.15)^4 ≈ 0.08 ⇒ w_eff = 1.
    //     scan: 1 inadmissible (skip, not counted) → 3 acquire_input: needs
    //     r^L ≥ π^I=2 (ok) ⇒ admissible, counted (scanned=1). Does it satisfice
    //     SURVIVAL (h_next > h_t)? acquire_input: r^L 8→6, input 0→1. g_1
    //     worsens slightly ⇒ h_next < h_t ⇒ NOT satisficing. Budget spent ⇒
    //     fallback = first admissible = 3.
    let d4 = Satisficing::new(SatisficingParams {
        beta: 4.0,
        ..Default::default()
    })
    .unwrap()
    .apply(&base, base.rng_key());
    assert_eq!(field_value(&d4, 0, keys::W_EFF), Some(1));
    //   β=0 ⇒ ψ=1 ⇒ w_eff = 6. Wider scan reaches `hold` (0) and `deliver`
    //   etc.; `hold` keeps h flat (not > h_t) so still no SURVIVAL-satisficer
    //   here → fallback first admissible = 3 as well, but w_eff differs.
    let d0 = Satisficing::new(SatisficingParams {
        beta: 0.0,
        ..Default::default()
    })
    .unwrap()
    .apply(&base, base.rng_key());
    assert_eq!(field_value(&d0, 0, keys::W_EFF), Some(6));
}

// --- decision.random (ADR 0027) ---

#[test]
fn random_draws_only_from_the_admissible_set() {
    // A firm that can only `hold` (0 capital, 0 input, q=0): admissible = {0}.
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 0)
        .with_stock(0, "input", 0)
        .with_agent_real(0, keys::CAPABILITY, 0.0)
        .with_agent_int(0, keys::OBLIGATION, 0);
    for seed in 0..50u64 {
        let vs = seed_common(MockView::new(3, Phase::Decide, seed, &[0]))
            .with_stock(0, "capital", 0)
            .with_stock(0, "input", 0)
            .with_agent_real(0, keys::CAPABILITY, 0.0)
            .with_agent_int(0, keys::OBLIGATION, 0);
        let d = DecisionRandom::new(RandomParams::default())
            .unwrap()
            .apply(&vs, vs.rng_key());
        assert_eq!(selected(&d, 0), Some(0), "hold-only firm is deterministic");
    }
    // no FOCUS / W_EFF emitted by decision.random
    let d = DecisionRandom::new(RandomParams::default())
        .unwrap()
        .apply(&v, v.rng_key());
    assert!(field_value(&d, 0, keys::FOCUS).is_none());
    assert!(field_value(&d, 0, keys::W_EFF).is_none());
}

#[test]
fn random_covers_the_admissible_set_across_seeds() {
    // A well-provisioned firm: hold(0), produce_ordinary(1), acquire_input(3)
    // at least are admissible; produce_regulated(2) needs c ≥ θ_cap.
    use std::collections::BTreeSet;
    let mut seen = BTreeSet::new();
    for seed in 0..400u64 {
        let v = seed_common(MockView::new(1, Phase::Decide, seed, &[0]))
            .with_stock(0, "capital", 500)
            .with_stock(0, "input", 10)
            .with_agent_real(0, keys::CAPABILITY, 0.9)
            .with_agent_int(0, keys::OBLIGATION, 3);
        let d = DecisionRandom::new(RandomParams::default())
            .unwrap()
            .apply(&v, v.rng_key());
        seen.insert(selected(&d, 0).unwrap());
    }
    // admissible here: 0 (hold), 1 (produce_ordinary), 2 (produce_regulated,
    // c=0.9≥0.4), 3 (acquire_input), 4 (invest_capability, r^L≥κ_c=20),
    // 5 (deliver, q=3≥1, input≥1). Six actions — all should appear.
    assert_eq!(seen, [0, 1, 2, 3, 4, 5].into_iter().collect());
}

// --- decision.aspiration_update (§12.1) ---

#[test]
fn aspiration_update_moves_toward_realised_value() {
    // A_2 (capability) present at 0.5; v_2 = c = 0.8; α = 0.1
    // ⇒ ΔA_2 = 0.1·(0.8 − 0.5) = 0.03.
    let v = MockView::new(5, Phase::Record, 1, &[0])
        .with_stock(0, "capital", 100)
        .with_agent_real(0, keys::CAPABILITY, 0.8)
        .with_agent_int(0, keys::OBLIGATION, 4)
        .with_agent_real(0, keys::ASPIRATION_CAPABILITY, 0.5)
        .with_agent_int(0, keys::PREV_TICK_CAPITAL, 90); // v_1 = 100 - 90 = 10
    let d = AspirationUpdate::new(AspirationUpdateParams::default())
        .unwrap()
        .apply(&v, v.rng_key());

    let dcap = d.iter().find_map(|x| match (&x.target, &x.kind) {
        (DeltaTarget::Agent(a), DeltaKind::AdjustAgentReal { field, delta })
            if a.0 == 0 && field == keys::ASPIRATION_CAPABILITY =>
        {
            Some(*delta)
        }
        _ => None,
    });
    assert!((dcap.unwrap() - 0.03).abs() < 1e-12);

    // A_1 absent ⇒ seed to v_1 = 10 (add 10 to the 0 store).
    let dcg = d.iter().find_map(|x| match (&x.target, &x.kind) {
        (DeltaTarget::Agent(a), DeltaKind::AdjustAgentReal { field, delta })
            if a.0 == 0 && field == keys::ASPIRATION_CAPITAL_GROWTH =>
        {
            Some(*delta)
        }
        _ => None,
    });
    assert!((dcg.unwrap() - 10.0).abs() < 1e-12);

    // persists v_1 and the new baseline
    assert_eq!(field_value(&d, 0, keys::REALIZED_CAPITAL_GROWTH), Some(10));
    assert_eq!(field_value(&d, 0, keys::PREV_TICK_CAPITAL), Some(100));
}

#[test]
fn aspiration_update_rejects_bad_alpha() {
    assert!(AspirationUpdate::new(AspirationUpdateParams { alpha: 0.0 }).is_err());
    assert!(AspirationUpdate::new(AspirationUpdateParams { alpha: 1.0 }).is_err());
    assert!(AspirationUpdate::new(AspirationUpdateParams { alpha: 0.1 }).is_ok());
}

#[test]
fn bad_satisficing_params_rejected() {
    assert!(Satisficing::new(SatisficingParams {
        h_crit: 0.0,
        ..Default::default()
    })
    .is_err());
    assert!(Satisficing::new(SatisficingParams {
        beta: -1.0,
        ..Default::default()
    })
    .is_err());
    assert!(Satisficing::new(SatisficingParams {
        w_max: 0,
        ..Default::default()
    })
    .is_err());
}

/// §12.3 is **fully deterministic** — Steps 1–5 draw no random numbers. The
/// only `firma_rng` use in the whole crate is `decision.random`'s single
/// per-agent `open_for(_, _, "decision_random")`.
#[test]
fn rng_is_used_only_by_decision_random() {
    let src = include_str!("lib.rs");
    let rng_lines: Vec<&str> = src
        .lines()
        .filter(|l| l.contains("firma_rng::") && !l.trim_start().starts_with("//"))
        .collect();
    assert_eq!(
        rng_lines.len(),
        1,
        "expected exactly one firma_rng call site, found: {rng_lines:?}"
    );
    assert!(rng_lines[0].contains("\"decision_random\""));
    // and it lives inside DecisionRandom, after Satisficing's impl ends.
    let sat_end = src.find("impl Rule for DecisionRandom").unwrap();
    let rng_at = src.find("firma_rng::open_for").unwrap();
    assert!(
        rng_at > sat_end,
        "the RNG call must be in DecisionRandom, not Satisficing"
    );
}

#[test]
fn every_registered_rule_has_a_nonempty_assumption_and_right_phase() {
    for (id, _, ctor) in registered() {
        let r = ctor(&serde_json::Value::Null).unwrap();
        assert!(!r.assumption().trim().is_empty(), "{id}");
        let expected = if id == catalog::ASPIRATION_UPDATE_ID {
            Phase::Record
        } else {
            Phase::Decide
        };
        assert_eq!(r.phase(), expected, "{id}");
    }
}

// --------------------------------------------------------------------------
// ADR 0047 (H3 revision) — Part B2: the channel actually opens, responds to
// the action's real economics, and — reported honestly — does NOT read lag
// length (a deliberate design choice, not an oversight; see the ADR).
// --------------------------------------------------------------------------

use firma_domain::shaping::{LagRange, SuccessModel};

/// A `SURVIVAL`-focus firm whose *binding* constraint is `compliance`
/// (`u` close to `θ_limit`), everything else slack, so every market action
/// ahead of `lobby` in the `SURVIVAL` order `[1,3,5,2,0,4,6,7,8]` is
/// admissible but does not satisfice (none of them move `u`, which the
/// one-step lookahead holds fixed — §9.3) — the scan genuinely *reaches*
/// `lobby` rather than falling back to it. `w_max=9`, `beta` small enough
/// that `w_eff ≥ 7` (needed to even consider position 6 after scanning the
/// six actions ahead of it).
fn cfg_compliance_bound_survival() -> MockView {
    seed_common(MockView::new(5, Phase::Decide, 1, &[0]))
        .with_global_real(keys::THETA_LIMIT, 0.5) // overrides seed_common's 0.90
        .with_stock(0, "capital", 500)
        .with_stock(0, "input", 20) // < R^I_max(40): keeps `acquire_input` admissible too
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 3) // > 0: keeps `deliver` admissible
        .with_agent_real(0, keys::REGULATED_INTENSITY, 0.4) // u; empty window ⇒ read directly
        .with_agent_real(0, keys::LEGITIMACY, 1.0)
    // h = -max(g_solvency/100, g_compliance/1, g_scope/0.5, g_obligation/50)
    //   g_solvency = -500 (slack), g_scope = 0.4-0.9=-0.5/0.5=-1.0 (slack),
    //   g_obligation = (3-100)/50 ≈ -1.94 (slack), g_compliance = 0.4-0.5 = -0.1
    //   ⇒ h_t = 0.10 < h_crit(0.15) ⇒ SURVIVAL.
}

fn lobby_shaping(delta_theta: f64, p0: f64, lag: LagRange) -> ShapingScanParams {
    lobby_shaping_with_time_margin(delta_theta, p0, lag, true)
}

fn lobby_shaping_with_time_margin(
    delta_theta: f64,
    p0: f64,
    lag: LagRange,
    require_time_margin: bool,
) -> ShapingScanParams {
    ShapingScanParams {
        lobby_cost: 25,
        contract_cost: None,
        diversify_cost: None,
        lobby_success: Some(LobbyParams {
            lag,
            success: SuccessModel {
                p0,
                b_lambda: 0.2,
                b_kappa: 0.1,
                p_max: 0.75,
                kappa_min: 25,
            },
            delta_theta,
        }),
        contract_success: None,
        require_time_margin,
    }
}

/// **The channel opens.** `p0=0.25` (§16.1 default) ⇒ `p_success =
/// 0.25+0.2·1.0 = 0.45`; `δ_θ=0.10` (§16.1 default) ⇒
/// `E[h] = 0.45·h_success + 0.55·h_failure`. Computed by hand in the ADR:
/// `h_success=0.20` (compliance relief: `u-θ_limit = 0.4-0.6=-0.2`),
/// `h_failure=0.10` (cost only, `u-θ_limit` unchanged) ⇒
/// `E[h]=0.45·0.20+0.55·0.10=0.145 > h_t=0.10` ⇒ **lobby satisfices**,
/// reached genuinely within `w_eff`, not via the fallback (verified
/// separately below).
#[test]
fn h3_lobby_satisfices_survival_when_expected_relief_clears_h_t() {
    let v = cfg_compliance_bound_survival();
    let params = SatisficingParams {
        beta: 0.5, // small: ψ=(0.10/0.15)^0.5≈0.816 ⇒ w_eff=⌈9·0.816⌉=8 ≥ 7
        w_max: 9,
        shaping: Some(lobby_shaping(0.10, 0.25, LagRange { min: 2, max: 6 })),
        ..Default::default()
    };
    let d = Satisficing::new(params).unwrap().apply(&v, v.rng_key());
    assert_eq!(
        field_value(&d, 0, keys::FOCUS),
        Some(0),
        "expected SURVIVAL"
    );
    assert_eq!(field_value(&d, 0, keys::W_EFF), Some(8));
    assert_eq!(
        selected(&d, 0),
        Some(6),
        "lobby should satisfice SURVIVAL once its expected relief is modelled (ADR 0047)"
    );
}

/// **The mirror case: the channel does NOT open when the economics don't
/// support it.** Same scenario, `δ_θ=0.0` (a lobby that, even on success,
/// does nothing) ⇒ `h_success = h_failure = h_t` exactly ⇒
/// `E[h] = h_t`, which fails the *strict* `>` test — lobby does not
/// satisfice, and the scan falls through to the fallback (first
/// admissible in order = `1`, `produce_ordinary`). This is the check that
/// the fix is a genuine threshold test responsive to the action's actual
/// merit, not "shaping is reachable somehow now" regardless of it.
#[test]
fn h3_lobby_does_not_satisfice_when_its_payoff_is_worthless() {
    let v = cfg_compliance_bound_survival();
    let params = SatisficingParams {
        beta: 0.5,
        w_max: 9,
        shaping: Some(lobby_shaping(0.0, 0.25, LagRange { min: 2, max: 6 })),
        ..Default::default()
    };
    let d = Satisficing::new(params).unwrap().apply(&v, v.rng_key());
    assert_eq!(
        field_value(&d, 0, keys::FOCUS),
        Some(0),
        "expected SURVIVAL"
    );
    assert_eq!(
        selected(&d, 0),
        Some(1),
        "a worthless lobby (δ_θ=0) must not satisfice — fallback picks the \
         first admissible action (produce_ordinary), same as pre-ADR-0047"
    );
}

/// **Reported honestly, not glossed over: the mechanism does not read lag
/// length.** ADR-0047 deliberately does not discount the expected payoff by
/// `Δ_a` (reasoning in the ADR: no manual anchor for a discount rate; the
/// existing `invest_capability` lag-collapse precedent, §9.3, is also
/// undiscounted). The *same* clears-the-threshold scenario, run with a
/// short lag range `(1,1)` and a long one `(8,16)`, selects `lobby`
/// identically either way — the decision procedure is not lag-sensitive.
/// H3's lag-vs-time-to-boundary comparison is therefore an **emergent,
/// downstream** property (whether a firm survives long enough to actually
/// receive a matured, lagged effect — the existing, unchanged
/// `resolve_lagged`/`LaggedRecord` machinery already handles that;
/// ADR-0041), measured across `Δ_min/Δ_max` config settings via §30.4's
/// own experimental design (a swept factor across *runs*), not a property
/// this ADR builds into a single decision.
#[test]
fn h3_the_survival_test_is_not_lag_sensitive_by_design() {
    let short = LagRange { min: 1, max: 1 };
    let long = LagRange { min: 8, max: 16 };

    let v_short = cfg_compliance_bound_survival();
    let params_short = SatisficingParams {
        beta: 0.5,
        w_max: 9,
        shaping: Some(lobby_shaping(0.10, 0.25, short)),
        ..Default::default()
    };
    let d_short = Satisficing::new(params_short)
        .unwrap()
        .apply(&v_short, v_short.rng_key());

    let v_long = cfg_compliance_bound_survival();
    let params_long = SatisficingParams {
        beta: 0.5,
        w_max: 9,
        shaping: Some(lobby_shaping(0.10, 0.25, long)),
        ..Default::default()
    };
    let d_long = Satisficing::new(params_long)
        .unwrap()
        .apply(&v_long, v_long.rng_key());

    assert_eq!(selected(&d_short, 0), Some(6));
    assert_eq!(
        selected(&d_long, 0),
        selected(&d_short, 0),
        "the decision does not vary with lag length — expected and documented \
         (ADR 0047): lag's consequence is downstream (survival to maturity), \
         not decision-time"
    );
}

// --------------------------------------------------------------------------
// ADR 0048 (H3 revision, round 2) — Part D: the actual race, and the toggle.
// --------------------------------------------------------------------------

/// Same shape as `cfg_compliance_bound_survival` (entry via `compliance`,
/// `h_t = 0.10 < h_crit`), but with a **finite** `time_to_boundary`: `prev`
/// is seeded to `invest_capability` (4, `κ_c = 20`/tick), so the projection
/// drains `capital` (200 → 180 → … → 0) until `solvency` overtakes
/// `compliance` as the binding constraint at exactly `capital = 0`
/// (`g_solvency = 0 ≥ g_compliance = -0.10`) — `time_to_boundary = 10`
/// (verified independently by `firma-domain`'s own
/// `time_to_boundary_counts_an_exact_crossing`-style unit tests). Same
/// scan-reaches-`lobby` shape as round 1 (`w_eff = 8`).
fn cfg_solvency_race_survival() -> MockView {
    seed_common(MockView::new(5, Phase::Decide, 1, &[0]))
        .with_global_real(keys::THETA_LIMIT, 0.5)
        .with_stock(0, "capital", 200) // 200/20 = 10 ticks of invest_capability to hit 0
        .with_stock(0, "input", 20)
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 3)
        .with_agent_real(0, keys::REGULATED_INTENSITY, 0.4)
        .with_agent_real(0, keys::LEGITIMACY, 1.0)
        .with_agent_int(0, keys::SELECTED_ACTION, 4) // prev = invest_capability
}

/// **The race, won**: `Δ_min = 2 < time_to_boundary = 10` ⇒ the payoff is
/// evaluated (ADR 0047's expected-relief calculation) ⇒ `lobby` satisfices,
/// exactly as round 1, because the payoff genuinely *can* arrive before the
/// projected boundary.
#[test]
fn h3_race_short_lag_wins_lobby_satisfices() {
    let v = cfg_solvency_race_survival();
    let params = SatisficingParams {
        beta: 0.5,
        w_max: 9,
        shaping: Some(lobby_shaping(0.10, 0.25, LagRange { min: 2, max: 6 })),
        ..Default::default()
    };
    let d = Satisficing::new(params).unwrap().apply(&v, v.rng_key());
    assert_eq!(
        field_value(&d, 0, keys::FOCUS),
        Some(0),
        "expected SURVIVAL"
    );
    assert_eq!(field_value(&d, 0, keys::W_EFF), Some(8));
    assert_eq!(
        selected(&d, 0),
        Some(6),
        "short lag (min=2) comfortably beats time_to_boundary=10 ⇒ lobby should satisfice"
    );
}

/// **The race, lost**: `Δ_min = 12 ≥ time_to_boundary = 10` ⇒ the payoff
/// cannot possibly arrive before the projected boundary ⇒ falls back to the
/// pre-ADR-0047 cost-only evaluation for `lobby` (which never satisfices,
/// same as ADR-0042's original finding) ⇒ the scan continues past `lobby`
/// and the true fallback lands on the first admissible action in the full
/// `SURVIVAL` order — `produce_ordinary` (1), a cheaper, cost-only-viable
/// action — **not** `lobby`. This is H3's "increases narrowing" direction
/// as a real mechanical consequence, not an emergent one.
#[test]
fn h3_race_long_lag_loses_falls_back_to_cost_only() {
    let v = cfg_solvency_race_survival();
    let params = SatisficingParams {
        beta: 0.5,
        w_max: 9,
        shaping: Some(lobby_shaping(0.10, 0.25, LagRange { min: 12, max: 16 })),
        ..Default::default()
    };
    let d = Satisficing::new(params).unwrap().apply(&v, v.rng_key());
    assert_eq!(
        field_value(&d, 0, keys::FOCUS),
        Some(0),
        "expected SURVIVAL"
    );
    assert_eq!(
        selected(&d, 0),
        Some(1),
        "long lag (min=12) cannot beat time_to_boundary=10 ⇒ lobby must NOT \
         satisfice ⇒ fallback picks produce_ordinary, not lobby"
    );
}

/// **The toggle reverts to ADR-0047's payoff-only behaviour**: the exact
/// same long-lag scenario that just lost the race above, but with
/// `require_time_margin = false` — the timing check is skipped entirely,
/// the payoff is evaluated regardless of `time_to_boundary`, and `lobby`
/// satisfices again (identical arithmetic to the short-lag case) despite
/// the long lag. Proves the switch is a genuine behavioural toggle, not a
/// cosmetic config field.
#[test]
fn h3_toggle_off_reverts_to_payoff_only_despite_long_lag() {
    let v = cfg_solvency_race_survival();
    let params = SatisficingParams {
        beta: 0.5,
        w_max: 9,
        shaping: Some(lobby_shaping_with_time_margin(
            0.10,
            0.25,
            LagRange { min: 12, max: 16 }, // the same "losing" lag as above
            false,                         // require_time_margin = false
        )),
        ..Default::default()
    };
    let d = Satisficing::new(params).unwrap().apply(&v, v.rng_key());
    assert_eq!(
        field_value(&d, 0, keys::FOCUS),
        Some(0),
        "expected SURVIVAL"
    );
    assert_eq!(
        selected(&d, 0),
        Some(6),
        "require_time_margin=false must ignore the race and restore ADR-0047's \
         payoff-only satisficing test — a config-only switch, no code change"
    );
}

// --------------------------------------------------------------------------
// ADR 0054 — Arm A direct manipulation (`PINNED_MARGIN`/`PINNED_SHORTFALL_*`)
// --------------------------------------------------------------------------
// Test plan items 2 (independence), 6 (rare-quadrant construction), and 7
// (shortfall-mapping cross-check against VT-8's own construction), plus the
// partial-pin validation contract (ADR-0054 Part A item 1). Items 1
// (RNG-non-interference), 3 (persistence across ticks), and 4 (real-state
// independence) are kernel/conformance-level and live in
// `crates/firma-kernel/src/tests.rs` and `tests/tests/*.rs` respectively —
// not duplicated here. Item 5 (backward compatibility) is this crate's
// existing 22-test suite (none of which sets a `PINNED_*` key) staying
// green, re-confirmed, not a new test.

/// Item 2 — independence: a "fat margin, no shortfall" *real* state (the
/// same shape as `attend_none_when_no_shortfall_and_margin_fat`, which
/// would resolve to `Focus::None` unpinned) with all four pins set to
/// SURVIVAL-triggering values must resolve to `Focus::Survival` — proving
/// the override is *total*, not blended with the real state.
#[test]
fn adr0054_pins_override_real_state_totally() {
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 400) // real state: healthy, no shortfall
        .with_stock(0, "input", 10)
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 0)
        .with_agent_real(0, keys::PINNED_MARGIN, 0.02) // pinned: thin margin
        .with_agent_real(0, keys::PINNED_SHORTFALL_CAPITAL_GROWTH, 0.0)
        .with_agent_real(0, keys::PINNED_SHORTFALL_CAPABILITY, 0.0)
        .with_agent_real(0, keys::PINNED_SHORTFALL_OBLIGATION_CLEARANCE, 0.0);
    let d = Satisficing::new(SatisficingParams::default())
        .unwrap()
        .apply(&v, v.rng_key());
    assert_eq!(
        field_value(&d, 0, keys::FOCUS),
        Some(Focus::Survival.code()),
        "real state alone (fat margin) would give Focus::None — the pin must win totally"
    );
}

/// Item 6 — rare-quadrant construction: a *healthy* pinned margin
/// (`h = 0.40`, comfortably above `h_crit = 0.15`) combined with a *high*
/// pinned shortfall (`ς_1 = 1.0`) — a combination real organic dynamics
/// would rarely produce (a firm doing well on viability but badly on
/// goals) — must be directly constructible and resolve to `Focus::Goal(1)`.
#[test]
fn adr0054_rare_quadrant_healthy_margin_high_shortfall_is_constructible() {
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 5) // real state: actually thin (irrelevant once pinned)
        .with_stock(0, "input", 3)
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 0)
        .with_agent_real(0, keys::PINNED_MARGIN, 0.40)
        .with_agent_real(0, keys::PINNED_SHORTFALL_CAPITAL_GROWTH, 1.0)
        .with_agent_real(0, keys::PINNED_SHORTFALL_CAPABILITY, 0.0)
        .with_agent_real(0, keys::PINNED_SHORTFALL_OBLIGATION_CLEARANCE, 0.0);
    let d = Satisficing::new(SatisficingParams::default())
        .unwrap()
        .apply(&v, v.rng_key());
    assert_eq!(
        field_value(&d, 0, keys::FOCUS),
        Some(Focus::Goal(1).code()),
        "healthy pinned h (0.40 >= h_crit) + high pinned ς_1 (1.0) must reach GOAL(1), \
         not SURVIVAL — a real-state margin of 5/100 would give SURVIVAL unpinned"
    );
}

/// Item 7 — shortfall-mapping cross-check against
/// `vt8_orthogonal_manipulation_of_h_and_shortfall`'s own construction
/// (`tests/tests/validation.rs`, `select(h, [s1, 0.0, 0.0], ...)`): the
/// accepted Option-B mapping (ADR-0054) pins `ς_1` to the swept value and
/// `ς_2`/`ς_3` at `0.0` — confirm `Satisficing::apply()`'s pin-read path
/// produces the *same* `Focus`/`w_eff` as calling `select()` directly with
/// VT-8's own `[s1, 0.0, 0.0]` shape, for representative values drawn from
/// VT-8's actual grid (`h_levels`/`s_levels` in that test).
#[test]
fn adr0054_shortfall_mapping_matches_vt8_construction() {
    let h_crit = default_h_crit();
    let beta = default_beta();
    let w_max = default_w_max();
    // A sample from VT-8's own h_levels x s_levels grid, spanning both
    // SURVIVAL and GOAL(1) regimes.
    for &(h, s1) in &[
        (0.02_f64, 0.25_f64),
        (0.05, -0.25),
        (0.20, 0.50),
        (0.20, -1.0),
        (0.40, 2.0),
    ] {
        let direct = select(
            h,
            [s1, 0.0, 0.0],
            beta,
            h_crit,
            w_max,
            0,
            |a| a <= 5,
            |_a, _f| false,
        );

        let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
            .with_stock(0, "capital", 400)
            .with_stock(0, "input", 10)
            .with_agent_real(0, keys::CAPABILITY, 0.9)
            .with_agent_int(0, keys::OBLIGATION, 0)
            .with_agent_real(0, keys::PINNED_MARGIN, h)
            .with_agent_real(0, keys::PINNED_SHORTFALL_CAPITAL_GROWTH, s1)
            .with_agent_real(0, keys::PINNED_SHORTFALL_CAPABILITY, 0.0)
            .with_agent_real(0, keys::PINNED_SHORTFALL_OBLIGATION_CLEARANCE, 0.0);
        let d = Satisficing::new(SatisficingParams::default())
            .unwrap()
            .apply(&v, v.rng_key());

        assert_eq!(
            field_value(&d, 0, keys::FOCUS),
            Some(direct.focus.code()),
            "h={h}, ς_1={s1}: Focus diverged from VT-8's own [s1,0,0] construction"
        );
        assert_eq!(
            field_value(&d, 0, keys::W_EFF),
            Some(i64::from(direct.w_eff)),
            "h={h}, ς_1={s1}: w_eff diverged from VT-8's own [s1,0,0] construction"
        );
    }
}

/// Partial-pin contract (ADR-0054 Part A item 1): three of the four
/// `PINNED_*` keys present, one missing, must panic loudly rather than
/// silently compute a mix of pinned and real values.
#[test]
#[should_panic(expected = "partial Arm-A pin state")]
fn adr0054_partial_pin_panics() {
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 400)
        .with_stock(0, "input", 10)
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 0)
        .with_agent_real(0, keys::PINNED_MARGIN, 0.02)
        .with_agent_real(0, keys::PINNED_SHORTFALL_CAPITAL_GROWTH, 0.0)
        .with_agent_real(0, keys::PINNED_SHORTFALL_CAPABILITY, 0.0);
    // PINNED_SHORTFALL_OBLIGATION_CLEARANCE deliberately omitted.
    let _ = Satisficing::new(SatisficingParams::default())
        .unwrap()
        .apply(&v, v.rng_key());
}

/// The complementary partial-pin case — exactly one of four present —
/// confirmed to panic too (not just the three-of-four case above).
#[test]
#[should_panic(expected = "partial Arm-A pin state")]
fn adr0054_single_pin_panics() {
    let v = seed_common(MockView::new(3, Phase::Decide, 1, &[0]))
        .with_stock(0, "capital", 400)
        .with_stock(0, "input", 10)
        .with_agent_real(0, keys::CAPABILITY, 0.9)
        .with_agent_int(0, keys::OBLIGATION, 0)
        .with_agent_real(0, keys::PINNED_MARGIN, 0.02);
    let _ = Satisficing::new(SatisficingParams::default())
        .unwrap()
        .apply(&v, v.rng_key());
}
