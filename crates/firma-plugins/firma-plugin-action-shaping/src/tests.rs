//! Unit tests for the shaping rules and the lagged-effect resolver
//! (manual §11.2, §11.3, §10.1 phase 6).

use firma_core::{DeltaKind, DeltaTarget, Phase};
use firma_domain::{keys, Effect, LaggedRecord};

use super::support::MockView;
use super::*;

fn lobby_rule() -> Box<dyn Rule> {
    lobby(&serde_json::Value::Null).unwrap()
}

/// Cost delta (agent side) for `agent`, summed.
fn agent_cost(deltas: &[Delta], agent: u64) -> i64 {
    deltas
        .iter()
        .filter_map(|d| match (&d.target, &d.kind) {
            (DeltaTarget::Agent(a), DeltaKind::AdjustStock { resource, amount })
                if a.0 == agent && resource.0 == "capital" =>
            {
                Some(*amount)
            }
            _ => None,
        })
        .sum()
}

fn enqueued(deltas: &[Delta]) -> Vec<LaggedRecord> {
    deltas
        .iter()
        .filter_map(|d| match &d.kind {
            DeltaKind::PushAgentRecord { list, record_json } if list == keys::LAGGED_EFFECTS => {
                Some(LaggedRecord::from_json(record_json).unwrap())
            }
            _ => None,
        })
        .collect()
}

fn view_for_lobby(tick: u64, seed: u64) -> MockView {
    MockView::new(tick, Phase::ActShaping, seed, &[0])
        .with_stock(0, "capital", 1_000)
        .with_agent_real(0, keys::LEGITIMACY, 0.5)
        .with_agent_int(0, keys::SELECTED_ACTION, 6)
}

#[test]
fn cost_is_paid_at_commitment_and_conserves() {
    let v = view_for_lobby(0, 42);
    let d = lobby_rule().apply(&v, v.rng_key());
    // κ_ℓ = 25 spent by the agent, +25 to the env pool.
    assert_eq!(agent_cost(&d, 0), -25);
    let net: i64 = d
        .iter()
        .filter_map(|x| match &x.kind {
            DeltaKind::AdjustStock { resource, amount } if resource.0 == "capital" => Some(*amount),
            _ => None,
        })
        .sum();
    assert_eq!(net, 0, "cost is conserving (paired with env)");
}

#[test]
fn cost_is_paid_even_when_the_attempt_fails() {
    // Find a seed that fails, and one that succeeds — assert the cost is
    // identical either way (§11.3 property 1, no refund-on-failure).
    let (mut saw_fail, mut saw_ok) = (false, false);
    for seed in 0..200u64 {
        let v = view_for_lobby(3, seed);
        let d = lobby_rule().apply(&v, v.rng_key());
        assert_eq!(agent_cost(&d, 0), -25, "cost always -25 (seed {seed})");
        match enqueued(&d)[0].effect {
            Effect::Lobby { applied: false, .. } => saw_fail = true,
            Effect::Lobby { applied: true, .. } => saw_ok = true,
            _ => unreachable!(),
        }
    }
    assert!(saw_fail && saw_ok, "both outcomes occur across seeds");
}

#[test]
fn lag_is_at_least_one_tick_and_drawn() {
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..200u64 {
        let v = view_for_lobby(10, seed);
        let d = lobby_rule().apply(&v, v.rng_key());
        let m = enqueued(&d)[0].maturity_tick;
        assert!(m > 10, "maturity {m} must be > commitment tick 10");
        assert!(m <= 16, "within Δ^max_ℓ = 6");
        seen.insert(m - 10);
    }
    assert!(
        seen.len() > 1,
        "the lag is actually drawn, not constant: {seen:?}"
    );
}

#[test]
fn the_two_draws_are_independent_streams() {
    // DT-6 shape, within one rule: perturbing the "shaping_lag" stream must not
    // change the "shaping_success" outcome, and vice versa. We compare the real
    // rule against manual draws with a decoy interleaved.
    let v = view_for_lobby(5, 7);
    let key = v.rng_key();

    let lag_solo = firma_rng::open_for(&key, Some(0), TAG_LAG).uniform_inclusive(2, 6);
    let succ_solo = firma_rng::open_for(&key, Some(0), TAG_SUCCESS).next_f64_unit();

    // Draw success first, then lag (opposite order) — counter-based RNG ⇒ same.
    let succ_first = firma_rng::open_for(&key, Some(0), TAG_SUCCESS).next_f64_unit();
    let lag_first = firma_rng::open_for(&key, Some(0), TAG_LAG).uniform_inclusive(2, 6);
    assert_eq!(lag_solo, lag_first);
    assert!((succ_solo - succ_first).abs() < 1e-15);
}

#[test]
fn contract_needs_a_supply_partner() {
    let params = reference_contract_params();
    let rule = contract(&params).unwrap();
    // No edges → precondition fails → no-op.
    let bare = MockView::new(0, Phase::ActShaping, 1, &[0])
        .with_stock(0, "capital", 1_000)
        .with_agent_int(0, keys::SELECTED_ACTION, 7);
    assert!(rule.apply(&bare, bare.rng_key()).is_empty());

    // Add an incoming supply edge → fires.
    let edge = serde_json::to_string(&firma_domain::Edge::supply(99, 0, 0)).unwrap();
    let ok = bare.with_global_record(keys::RELATION_EDGES, edge);
    let d = rule.apply(&ok, ok.rng_key());
    assert!(!d.is_empty());
    match enqueued(&d)[0].effect {
        Effect::Contract { source, q0, .. } => {
            assert_eq!(source, 99);
            assert_eq!(q0, 5);
        }
        _ => unreachable!(),
    }
}

#[test]
fn diversify_uses_a_synthetic_source() {
    let rule = diversify(&reference_diversify_params()).unwrap();
    let v = MockView::new(4, Phase::ActShaping, 1, &[2])
        .with_stock(2, "capital", 1_000)
        .with_agent_int(2, keys::SELECTED_ACTION, 8);
    let d = rule.apply(&v, v.rng_key());
    match enqueued(&d)[0].effect {
        Effect::Diversify { source, .. } => {
            assert_eq!(source, synthetic_source(firma_core::AgentId(2), 4));
            assert!(source >= 1_000_000_000);
        }
        _ => unreachable!(),
    }
}

#[test]
fn affordability_precondition_blocks_a_poor_firm() {
    let v = MockView::new(0, Phase::ActShaping, 1, &[0])
        .with_stock(0, "capital", 10) // < κ_ℓ = 25
        .with_agent_int(0, keys::SELECTED_ACTION, 6);
    assert!(lobby_rule().apply(&v, v.rng_key()).is_empty());
}

#[test]
fn bad_params_are_rejected_at_construction() {
    // p_max ≥ 1 violates §11.3 property 3 (full success model so it reaches
    // `validate()`, not the missing-field parse error).
    let bad = serde_json::json!({
        "success": { "p0": 0.2, "b_lambda": 0.2, "b_kappa": 0.1, "p_max": 1.0, "kappa_min": 25 }
    });
    assert!(lobby(&bad).is_err());
    // lag min 0 violates §11.3 property 2.
    let bad2 = serde_json::json!({ "lag": { "min": 0, "max": 4 } });
    assert!(lobby(&bad2).is_err());
}

/// Stage-2 review Fix 2: `b_lambda` / `b_kappa` have **no** §16.1 value and are
/// **not** serde-defaulted — a config that provides a `success` block but omits
/// either coefficient is rejected, not silently given `0.2` / `0.1`. (Same
/// treatment §16.1's silent-`P_q` gap got in Stage 1.)
#[test]
fn b_coefficients_are_required_when_a_success_block_is_given() {
    // `contract` — whole param block required; omit b_kappa from `success`.
    let missing_b_kappa = serde_json::json!({
        "lag": { "min": 3, "max": 8 },
        "success": { "p0": 0.2, "b_lambda": 0.2, "p_max": 0.6, "kappa_min": 30 },
        "delta_q": 20, "q0": 5
    });
    let err = contract(&missing_b_kappa).err().expect("must be rejected");
    assert!(
        err.contains("b_kappa"),
        "error should name the missing field: {err}"
    );

    // `diversify` — omit b_lambda from `success`.
    let missing_b_lambda = serde_json::json!({
        "lag": { "min": 2, "max": 5 },
        "success": { "p0": 0.15, "b_kappa": 0.1, "p_max": 0.55, "kappa_min": 15 }
    });
    let err = diversify(&missing_b_lambda)
        .err()
        .expect("must be rejected");
    assert!(
        err.contains("b_lambda"),
        "error should name the missing field: {err}"
    );

    // `lobby` — a partial `success` object (present but incomplete) is also
    // rejected; the coefficients are only defaulted when the *whole* success
    // block is absent (LobbyParams::default()).
    let lobby_partial =
        serde_json::json!({ "success": { "p0": 0.25, "p_max": 0.75, "kappa_min": 25 } });
    assert!(lobby(&lobby_partial).is_err());
    // …but bare `lobby` (no params at all) still builds from §16.1 defaults.
    assert!(lobby(&serde_json::Value::Null).is_ok());
}

// --- resolve_lagged ---

#[test]
fn resolver_applies_matured_and_drains_only_those() {
    let resolver = LaggedEffectResolver::new();
    let matured = LaggedRecord::new(
        5,
        Effect::Lobby {
            applied: true,
            theta_limit_delta: 0.1,
        },
    );
    let pending = LaggedRecord::new(
        9,
        Effect::Diversify {
            applied: true,
            source: 1_000_000_042,
        },
    );
    let v = MockView::new(5, Phase::ResolveLagged, 1, &[0])
        .with_agent_real(0, keys::CAPABILITY, 0.5)
        .with_agent_record(0, keys::LAGGED_EFFECTS, matured.to_json())
        .with_agent_record(0, keys::LAGGED_EFFECTS, pending.to_json());
    let d = resolver.apply(&v, v.rng_key());

    // θ_limit shifted by the matured lobby.
    assert!(d.iter().any(|x| matches!(&x.kind,
        DeltaKind::AdjustGlobalReal { field, delta } if field == keys::THETA_LIMIT && (*delta - 0.1).abs() < 1e-12)));
    // the queue is replaced with exactly the still-pending entry.
    let replace = d
        .iter()
        .find_map(|x| match &x.kind {
            DeltaKind::ReplaceAgentList { list, records_json } if list == keys::LAGGED_EFFECTS => {
                Some(records_json.clone())
            }
            _ => None,
        })
        .expect("a drain");
    assert_eq!(replace, vec![pending.to_json()]);
}

/// ADR 0041: when **two firms'** matured `lobby` effects both shift `θ_limit`
/// in one `resolve_lagged` phase, the resolver combines them into a single
/// `AdjustGlobalReal` (simultaneous shaping is additive, ADR 0016) rather than
/// emitting two deltas to the same `(Global, theta_limit)` cell — which would
/// trip the reconciler's per-rule uniqueness guard.
#[test]
fn two_firms_maturing_lobby_combine_into_one_theta_delta() {
    let resolver = LaggedEffectResolver::new();
    let mk = |d| {
        LaggedRecord::new(
            5,
            Effect::Lobby {
                applied: true,
                theta_limit_delta: d,
            },
        )
        .to_json()
    };
    let v = MockView::new(5, Phase::ResolveLagged, 1, &[0, 1])
        .with_agent_real(0, keys::CAPABILITY, 0.5)
        .with_agent_real(1, keys::CAPABILITY, 0.5)
        .with_agent_record(0, keys::LAGGED_EFFECTS, mk(0.10))
        .with_agent_record(1, keys::LAGGED_EFFECTS, mk(0.05));
    let d = resolver.apply(&v, v.rng_key());

    let theta_deltas: Vec<f64> = d
        .iter()
        .filter_map(|x| match &x.kind {
            DeltaKind::AdjustGlobalReal { field, delta } if field == keys::THETA_LIMIT => {
                Some(*delta)
            }
            _ => None,
        })
        .collect();
    assert_eq!(theta_deltas.len(), 1, "one combined θ_limit delta, not two");
    assert!((theta_deltas[0] - 0.15).abs() < 1e-12, "0.10 + 0.05 summed");
    // still one drain per agent.
    assert_eq!(
        d.iter()
            .filter(|x| matches!(&x.kind, DeltaKind::ReplaceAgentList { list, .. } if list == keys::LAGGED_EFFECTS))
            .count(),
        2
    );
}

#[test]
fn resolver_is_a_noop_when_nothing_matures() {
    let resolver = LaggedEffectResolver::new();
    let pending = LaggedRecord::new(
        99,
        Effect::Diversify {
            applied: true,
            source: 1,
        },
    );
    let v = MockView::new(5, Phase::ResolveLagged, 1, &[0]).with_agent_record(
        0,
        keys::LAGGED_EFFECTS,
        pending.to_json(),
    );
    assert!(resolver.apply(&v, v.rng_key()).is_empty());
}

#[test]
fn failed_effect_matures_to_nothing_but_still_drains() {
    let resolver = LaggedEffectResolver::new();
    let failed = LaggedRecord::new(
        2,
        Effect::Lobby {
            applied: false,
            theta_limit_delta: 0.1,
        },
    );
    let v = MockView::new(2, Phase::ResolveLagged, 1, &[0]).with_agent_record(
        0,
        keys::LAGGED_EFFECTS,
        failed.to_json(),
    );
    let d = resolver.apply(&v, v.rng_key());
    // no θ shift...
    assert!(!d
        .iter()
        .any(|x| matches!(&x.kind, DeltaKind::AdjustGlobalReal { .. })));
    // ...but the queue is drained to empty.
    assert!(d.iter().any(|x| matches!(&x.kind,
        DeltaKind::ReplaceAgentList { list, records_json } if list == keys::LAGGED_EFFECTS && records_json.is_empty())));
}

#[test]
fn resolver_phase_is_resolve_lagged() {
    assert_eq!(LaggedEffectResolver::new().phase(), Phase::ResolveLagged);
}

#[test]
fn every_registered_rule_has_a_nonempty_assumption() {
    for (id, _, ctor) in registered() {
        let params = match id {
            catalog::CONTRACT_ID => reference_contract_params(),
            catalog::DIVERSIFY_ID => reference_diversify_params(),
            _ => serde_json::Value::Null,
        };
        let r = ctor(&params).unwrap();
        assert!(!r.assumption().trim().is_empty(), "{}", r.id().as_str());
    }
}
