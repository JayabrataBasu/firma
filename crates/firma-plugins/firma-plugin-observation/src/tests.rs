//! Unit tests for the three `Observation` rules against an in-memory `View`.

use std::collections::BTreeMap;

use firma_core::{AgentId, Delta, DeltaKind, Phase, ResourceKind, RngKey, StreamId, Tick, View};
use firma_domain::{keys, EnvSnapshot};

use super::*;

/// Minimal opaque key-value `View` (mirrors the kernel store; ADR 0022).
struct Mv {
    tick: u64,
    live: Vec<AgentId>,
    global_reals: BTreeMap<String, f64>,
    global_ints: BTreeMap<String, i64>,
    global_lists: BTreeMap<String, Vec<String>>,
}

impl Mv {
    fn new(tick: u64, live: &[u64]) -> Mv {
        Mv {
            tick,
            live: live.iter().copied().map(AgentId).collect(),
            global_reals: BTreeMap::new(),
            global_ints: BTreeMap::new(),
            global_lists: BTreeMap::new(),
        }
    }
    fn gr(mut self, k: &str, v: f64) -> Self {
        self.global_reals.insert(k.into(), v);
        self
    }
    fn gi(mut self, k: &str, v: i64) -> Self {
        self.global_ints.insert(k.into(), v);
        self
    }
    fn rec(mut self, list: &str, r: String) -> Self {
        self.global_lists.entry(list.into()).or_default().push(r);
        self
    }
    fn seeded_env(self) -> Self {
        self.gi(keys::INPUT_PRICE, 2)
            .gi(keys::OUTPUT_PRICE, 3)
            .gi(keys::THETA_Q, 100)
            .gr(keys::THETA_LIMIT, 0.45)
            .gr(keys::THETA_CAP, 0.40)
    }
    fn key(&self) -> RngKey {
        RngKey {
            run_seed: 20260905,
            stream: StreamId::Environment,
            plugin_id: 7,
            phase: Phase::Observe,
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
        Phase::Observe
    }
    fn run_seed(&self, _s: StreamId) -> u64 {
        20260905
    }
    fn live_agents(&self) -> &[AgentId] {
        &self.live
    }
    fn is_live(&self, a: AgentId) -> bool {
        self.live.contains(&a)
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
    fn global_real(&self, f: &str) -> Option<f64> {
        self.global_reals.get(f).copied()
    }
    fn global_int(&self, f: &str) -> Option<i64> {
        self.global_ints.get(f).copied()
    }
    fn global_records(&self, l: &str) -> &[String] {
        self.global_lists.get(l).map_or(&[], Vec::as_slice)
    }
}

fn observed(deltas: &[Delta], agent: u64) -> EnvSnapshot {
    let d = deltas
        .iter()
        .find(|d| matches!(&d.target, firma_core::DeltaTarget::Agent(a) if a.0 == agent))
        .expect("a per-agent snapshot delta");
    match &d.kind {
        DeltaKind::ReplaceAgentList { list, records_json } => {
            assert_eq!(list, keys::OBSERVED_ENV);
            EnvSnapshot::from_json(&records_json[0]).unwrap()
        }
        _ => panic!("expected ReplaceAgentList"),
    }
}

#[test]
fn full_is_lossless_and_per_agent() {
    let v = Mv::new(5, &[0, 1]).seeded_env();
    let d = Full::default().apply(&v, v.key());
    assert_eq!(d.len(), 2);
    for a in [0, 1] {
        let o = observed(&d, a);
        assert_eq!(
            (
                o.input_price,
                o.output_price,
                o.theta_q,
                o.theta_limit,
                o.theta_cap
            ),
            (2, 3, 100, 0.45, 0.40)
        );
        assert_eq!(o.tick, 5);
    }
}

#[test]
fn noisy_perturbs_each_field_and_is_deterministic() {
    let v = Mv::new(3, &[0, 1, 2]).seeded_env();
    let rule = Noisy::new(NoisyParams { sigma: 0.05 }).unwrap();
    let d1 = rule.apply(&v, v.key());
    let d2 = rule.apply(&v, v.key());
    // CRN: identical apply ⇒ identical deltas.
    assert_eq!(format!("{d1:?}"), format!("{d2:?}"));

    // θ_limit is perturbed away from 0.45 for at least one firm, and the firms
    // differ (independent per-agent draws).
    let tls: Vec<f64> = [0, 1, 2]
        .iter()
        .map(|&a| observed(&d1, a).theta_limit)
        .collect();
    assert!(tls.iter().any(|t| (t - 0.45).abs() > 1e-9));
    assert!(tls[0] != tls[1] && tls[1] != tls[2]);
    // Prices stay integers.
    for a in [0, 1, 2] {
        let _ = observed(&d1, a).input_price; // typed i64 — compiles ⇒ rounded
    }
    assert_eq!(rng_stream_of(&rule), Some(StreamId::Environment));
}

fn rng_stream_of(r: &dyn Rule) -> Option<StreamId> {
    r.rng_stream()
}

#[test]
fn noisy_sigma_is_required_and_validated() {
    assert!(noisy(&serde_json::json!({})).is_err()); // missing sigma
    assert!(noisy(&serde_json::json!({ "sigma": -1.0 })).is_err());
    assert!(noisy(&serde_json::json!({ "sigma": 0.1 })).is_ok());
}

#[test]
fn delayed_reaches_back_k_ticks_and_maintains_the_ring() {
    let k = 2u64;
    // History already holds ticks 0 and 1 (θ_limit 0.30, 0.35); current truth
    // is tick 2 at 0.45. With k = 2 the firm should see tick 0's 0.30.
    let h0 = EnvSnapshot {
        tick: 0,
        input_price: 2,
        output_price: 3,
        theta_limit: 0.30,
        theta_cap: 0.4,
        theta_q: 100,
    };
    let h1 = EnvSnapshot {
        tick: 1,
        theta_limit: 0.35,
        ..h0
    };
    let v = Mv::new(2, &[0])
        .seeded_env()
        .rec(keys::ENV_HISTORY, h0.to_json())
        .rec(keys::ENV_HISTORY, h1.to_json());
    let d = Delayed::new(DelayedParams { k }).apply(&v, v.key());

    let o = observed(&d, 0);
    assert!((o.theta_limit - 0.30).abs() < 1e-12, "sees tick-0 θ_limit");
    assert_eq!(o.tick, 0);

    // The ring is rewritten to the last k + 1 = 3 entries: ticks 0, 1, 2.
    let ring = d
        .iter()
        .find_map(|x| match &x.kind {
            DeltaKind::ReplaceGlobalList { list, records_json } if list == keys::ENV_HISTORY => {
                Some(records_json.clone())
            }
            _ => None,
        })
        .expect("ENV_HISTORY rewrite");
    assert_eq!(ring.len(), 3);
    assert_eq!(EnvSnapshot::from_json(&ring[2]).unwrap().tick, 2);
}

#[test]
fn delayed_early_ticks_see_the_oldest_available() {
    // No history yet, tick 0, k = 5 ⇒ the firm sees the current truth.
    let v = Mv::new(0, &[0]).seeded_env();
    let d = Delayed::new(DelayedParams { k: 5 }).apply(&v, v.key());
    assert!((observed(&d, 0).theta_limit - 0.45).abs() < 1e-12);
}

#[test]
fn registered_ids_and_phases() {
    for (id, _hash, ctor) in registered() {
        let params = match id {
            "observation.noisy" => serde_json::json!({ "sigma": 0.1 }),
            "observation.delayed" => serde_json::json!({ "k": 1 }),
            _ => serde_json::json!({}),
        };
        let rule = ctor(&params).expect("constructs");
        assert_eq!(rule.phase(), Phase::Observe);
        assert!(!rule.assumption().trim().is_empty());
    }
}
