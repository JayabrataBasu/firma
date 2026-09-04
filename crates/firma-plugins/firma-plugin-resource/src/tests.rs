//! Unit tests for the resource rules against an in-memory `View`.

use std::collections::BTreeMap;

use firma_core::{AgentId, DeltaKind, Phase, ResourceKind, RngKey, StreamId, Tick, View};
use firma_domain::keys;

use super::*;

struct Mv {
    tick: u64,
    gi: BTreeMap<String, i64>,
}
impl Mv {
    fn new(tick: u64, input_price: Option<i64>) -> Mv {
        let mut gi = BTreeMap::new();
        if let Some(p) = input_price {
            gi.insert(keys::INPUT_PRICE.to_owned(), p);
        }
        Mv { tick, gi }
    }
    fn key(&self) -> RngKey {
        RngKey {
            run_seed: 4242,
            stream: StreamId::Environment,
            plugin_id: 9,
            phase: Phase::Environment,
            tick: self.tick,
            agent_id: None,
        }
    }
}
impl View for Mv {
    fn tick(&self) -> Tick {
        Tick(self.tick)
    }
    fn phase(&self) -> Phase {
        Phase::Environment
    }
    fn run_seed(&self, _s: StreamId) -> u64 {
        4242
    }
    fn live_agents(&self) -> &[AgentId] {
        &[]
    }
    fn is_live(&self, _a: AgentId) -> bool {
        false
    }
    fn agent_stock(&self, _a: AgentId, _r: &ResourceKind) -> i64 {
        0
    }
    fn env_stock(&self, _r: &ResourceKind) -> i64 {
        0
    }
    fn resource_kinds(&self) -> &[ResourceKind] {
        &[]
    }
    fn global_int(&self, f: &str) -> Option<i64> {
        self.gi.get(f).copied()
    }
}

#[test]
fn constant_emits_nothing() {
    let v = Mv::new(0, Some(2));
    assert!(Constant::default().apply(&v, v.key()).is_empty());
    assert_eq!(Constant::default().rng_stream(), None);
}

#[test]
fn patchy_reverts_toward_baseline() {
    // Price far below baseline, no noise: the walk steps up toward baseline.
    let rule = Patchy::new(PatchyParams {
        baseline: 10,
        reversion: 0.5,
        sigma: 0.0,
        floor: 1,
    })
    .unwrap();
    let v = Mv::new(1, Some(2));
    let d = rule.apply(&v, v.key());
    match &d[0].kind {
        DeltaKind::AdjustGlobalInt { field, delta } => {
            assert_eq!(field, keys::INPUT_PRICE);
            // drift = 0.5 * (10 - 2) = 4 ⇒ +4
            assert_eq!(*delta, 4);
        }
        _ => panic!(),
    }
    assert_eq!(rule.rng_stream(), Some(StreamId::Environment));
}

#[test]
fn patchy_at_baseline_with_no_noise_is_silent() {
    let rule = Patchy::new(PatchyParams {
        baseline: 5,
        reversion: 0.3,
        sigma: 0.0,
        floor: 1,
    })
    .unwrap();
    let v = Mv::new(3, Some(5));
    assert!(rule.apply(&v, v.key()).is_empty());
}

#[test]
fn patchy_respects_the_floor_and_is_deterministic() {
    let rule = Patchy::new(PatchyParams {
        baseline: 8,
        reversion: 0.1,
        sigma: 3.0,
        floor: 4,
    })
    .unwrap();
    // Same key ⇒ identical delta.
    let v = Mv::new(2, Some(4));
    let a = rule.apply(&v, v.key());
    let b = rule.apply(&v, v.key());
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    // Never below the floor: simulate many ticks.
    let mut price = 4i64;
    for t in 0..200 {
        let mut vv = Mv::new(t, Some(price));
        for x in rule.apply(&vv, vv.key()) {
            if let DeltaKind::AdjustGlobalInt { delta, .. } = x.kind {
                price += delta;
            }
        }
        let _ = &mut vv;
        assert!(price >= 4, "tick {t}: price {price} below floor");
    }
}

#[test]
fn params_required_and_validated() {
    assert!(patchy(&serde_json::json!({})).is_err());
    assert!(patchy(
        &serde_json::json!({ "baseline": 0, "reversion": 0.1, "sigma": 0.0, "floor": 1 })
    )
    .is_err());
    assert!(patchy(
        &serde_json::json!({ "baseline": 5, "reversion": 2.0, "sigma": 0.0, "floor": 1 })
    )
    .is_err());
    assert!(patchy(
        &serde_json::json!({ "baseline": 5, "reversion": 0.1, "sigma": 0.5, "floor": 2 })
    )
    .is_ok());
    for (id, _h, ctor) in registered() {
        let params = if id == "resource.patchy" {
            serde_json::json!({ "baseline": 3, "reversion": 0.1, "sigma": 0.0, "floor": 1 })
        } else {
            serde_json::json!({})
        };
        let rule = ctor(&params).unwrap();
        assert_eq!(rule.phase(), Phase::Environment);
        assert!(!rule.assumption().trim().is_empty());
    }
}
