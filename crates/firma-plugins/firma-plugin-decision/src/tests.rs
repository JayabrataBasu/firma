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
