//! Unit tests for the six market-action rules (manual §11.1).

use std::collections::BTreeMap;

use firma_core::{
    AgentId, DeltaKind, DeltaTarget, Phase, ResourceKind, RngKey, StreamId, Tick, View,
};

use super::*;

// --- a minimal in-memory View (this crate cannot depend on another plugin
//     crate for a shared one — §18.1) ---
struct Mv {
    tick: u64,
    live: Vec<AgentId>,
    stocks: BTreeMap<(u64, String), i64>,
    reals: BTreeMap<(u64, String), f64>,
    ints: BTreeMap<(u64, String), i64>,
    greals: BTreeMap<String, f64>,
    gints: BTreeMap<String, i64>,
    resources: Vec<ResourceKind>,
}
impl Mv {
    fn new(tick: u64, live: &[u64]) -> Self {
        Mv {
            tick,
            live: live.iter().copied().map(AgentId).collect(),
            stocks: BTreeMap::new(),
            reals: BTreeMap::new(),
            ints: BTreeMap::new(),
            greals: BTreeMap::new(),
            gints: BTreeMap::new(),
            resources: vec![ResourceKind("capital".into()), ResourceKind("input".into())],
        }
    }
    fn stock(mut self, a: u64, r: &str, v: i64) -> Self {
        self.stocks.insert((a, r.into()), v);
        self
    }
    fn real(mut self, a: u64, f: &str, v: f64) -> Self {
        self.reals.insert((a, f.into()), v);
        self
    }
    fn int(mut self, a: u64, f: &str, v: i64) -> Self {
        self.ints.insert((a, f.into()), v);
        self
    }
    fn gint(mut self, f: &str, v: i64) -> Self {
        self.gints.insert(f.into(), v);
        self
    }
    fn greal(mut self, f: &str, v: f64) -> Self {
        self.greals.insert(f.into(), v);
        self
    }
    fn select(self, a: u64, action: u8) -> Self {
        self.int(a, keys::SELECTED_ACTION, i64::from(action))
    }
}
impl View for Mv {
    fn tick(&self) -> Tick {
        Tick(self.tick)
    }
    fn phase(&self) -> Phase {
        Phase::ActMarket
    }
    fn run_seed(&self, _s: StreamId) -> u64 {
        1
    }
    fn live_agents(&self) -> &[AgentId] {
        &self.live
    }
    fn is_live(&self, a: AgentId) -> bool {
        self.live.contains(&a)
    }
    fn agent_stock(&self, a: AgentId, r: &ResourceKind) -> i64 {
        self.stocks.get(&(a.0, r.0.clone())).copied().unwrap_or(0)
    }
    fn env_stock(&self, _r: &ResourceKind) -> i64 {
        0
    }
    fn resource_kinds(&self) -> &[ResourceKind] {
        &self.resources
    }
    fn agent_real(&self, a: AgentId, f: &str) -> Option<f64> {
        self.reals.get(&(a.0, f.to_owned())).copied()
    }
    fn agent_int(&self, a: AgentId, f: &str) -> Option<i64> {
        self.ints.get(&(a.0, f.to_owned())).copied()
    }
    fn global_real(&self, f: &str) -> Option<f64> {
        self.greals.get(f).copied()
    }
    fn global_int(&self, f: &str) -> Option<i64> {
        self.gints.get(f).copied()
    }
}

fn key() -> RngKey {
    RngKey {
        run_seed: 1,
        stream: StreamId::Mechanism,
        plugin_id: 0,
        phase: Phase::ActMarket,
        tick: 0,
        agent_id: None,
    }
}

fn net_agent_stock(deltas: &[Delta], agent: u64, resource: &str) -> i64 {
    deltas
        .iter()
        .filter_map(|d| match (&d.target, &d.kind) {
            (
                DeltaTarget::Agent(a),
                DeltaKind::AdjustStock {
                    resource: r,
                    amount,
                },
            ) if a.0 == agent && r.0 == resource => Some(*amount),
            _ => None,
        })
        .sum()
}

fn conserves(deltas: &[Delta], resource: &str) -> bool {
    let total: i64 = deltas
        .iter()
        .filter_map(|d| match &d.kind {
            DeltaKind::AdjustStock {
                resource: r,
                amount,
            } if r.0 == resource => Some(*amount),
            _ => None,
        })
        .sum();
    total == 0
}

#[test]
fn only_the_selected_action_fires() {
    let v = Mv::new(0, &[0, 1]).stock(0, "input", 5).select(0, 1); // agent 0 → produce_ordinary
    let rule = produce_ordinary(&serde_json::Value::Null).unwrap();
    let d = rule.apply(&v, key());
    // agent 0 acts; agent 1 (no selection) does not.
    assert!(d
        .iter()
        .any(|x| matches!(&x.target, DeltaTarget::Agent(a) if a.0 == 0)));
    assert!(!d
        .iter()
        .any(|x| matches!(&x.target, DeltaTarget::Agent(a) if a.0 == 1)));
    // hold rule with agent 0 selected for produce_ordinary → nothing.
    assert!(hold(&serde_json::Value::Null)
        .unwrap()
        .apply(&v, key())
        .is_empty());
}

#[test]
fn produce_ordinary_pairs_with_env_and_conserves() {
    // y_0=4, η=0.8, c=0.5 → y_O = ⌊4·1.4⌋ = 5; π^O = 2 → revenue 10.
    let v = Mv::new(0, &[0])
        .stock(0, "input", 3)
        .real(0, "capability", 0.5)
        .gint("output_price", 2)
        .select(0, 1);
    let d = produce_ordinary(&serde_json::Value::Null)
        .unwrap()
        .apply(&v, key());
    assert_eq!(net_agent_stock(&d, 0, "capital"), 10);
    assert_eq!(net_agent_stock(&d, 0, "input"), -1);
    assert!(conserves(&d, "capital"));
    assert!(conserves(&d, "input"));
}

#[test]
fn none_from_market_core_is_a_silent_noop() {
    // produce_ordinary needs input ≥ 1; agent has none.
    let v = Mv::new(0, &[0]).stock(0, "input", 0).select(0, 1);
    let d = produce_ordinary(&serde_json::Value::Null)
        .unwrap()
        .apply(&v, key());
    assert!(d.is_empty());
}

#[test]
fn invest_capability_is_lagged_not_immediate() {
    // κ_c = 20; agent affords it.
    let v = Mv::new(7, &[0])
        .stock(0, "capital", 50)
        .real(0, "capability", 0.3)
        .select(0, 4);
    let d = invest_capability(&serde_json::Value::Null)
        .unwrap()
        .apply(&v, key());
    // capital spent now, paired with env:
    assert_eq!(net_agent_stock(&d, 0, "capital"), -20);
    assert!(conserves(&d, "capital"));
    // NO immediate capability delta; instead a lagged record maturing at t+Δ_cap.
    assert!(!d
        .iter()
        .any(|x| matches!(&x.kind, DeltaKind::AdjustAgentReal { .. })));
    let rec = d
        .iter()
        .find_map(|x| match &x.kind {
            DeltaKind::PushAgentRecord { list, record_json } if list == keys::LAGGED_EFFECTS => {
                Some(firma_domain::LaggedRecord::from_json(record_json).unwrap())
            }
            _ => None,
        })
        .expect("a lagged record");
    assert_eq!(rec.maturity_tick, 7 + CAPABILITY_LAG);
    assert!(
        matches!(rec.effect, firma_domain::Effect::CapabilityGain { delta } if (delta - 0.05).abs() < 1e-12)
    );
}

#[test]
fn deliver_reduces_obligation_unpaired() {
    let v = Mv::new(0, &[0])
        .stock(0, "input", 2)
        .int(0, "obligation", 3)
        .select(0, 5);
    let d = deliver(&serde_json::Value::Null).unwrap().apply(&v, key());
    assert_eq!(net_agent_stock(&d, 0, "input"), -1);
    assert!(conserves(&d, "input"));
    // obligation is a per-firm counter, not conserved: one unpaired AdjustAgentInt.
    let oblig: Vec<_> = d
        .iter()
        .filter(|x| matches!(&x.kind, DeltaKind::AdjustAgentInt { field, .. } if field == keys::OBLIGATION))
        .collect();
    assert_eq!(oblig.len(), 1);
    assert!(matches!(&oblig[0].kind, DeltaKind::AdjustAgentInt { delta, .. } if *delta == -1));
}

#[test]
fn produce_regulated_respects_the_scope_gate() {
    // θ_cap = 0.4; c = 0.3 → inadmissible → no-op.
    let v = Mv::new(0, &[0])
        .stock(0, "input", 2)
        .real(0, "capability", 0.3)
        .greal("theta_cap", 0.4)
        .gint("output_price", 1)
        .select(0, 2);
    assert!(produce_regulated(&serde_json::Value::Null)
        .unwrap()
        .apply(&v, key())
        .is_empty());
    // c = 0.5 ≥ θ_cap → fires.
    let v2 = Mv::new(0, &[0])
        .stock(0, "input", 2)
        .real(0, "capability", 0.5)
        .greal("theta_cap", 0.4)
        .gint("output_price", 1)
        .select(0, 2);
    assert!(!produce_regulated(&serde_json::Value::Null)
        .unwrap()
        .apply(&v2, key())
        .is_empty());
}

#[test]
fn every_rule_has_a_nonempty_assumption() {
    for (_, _, ctor) in registered() {
        let r = ctor(&serde_json::Value::Null).unwrap();
        assert!(!r.assumption().trim().is_empty(), "{}", r.id().as_str());
    }
}

#[test]
fn ids_have_distinct_numeric_projections() {
    let mut seen = std::collections::BTreeSet::new();
    for (id, _, _) in registered() {
        assert!(
            seen.insert(PluginId::new(id).numeric()),
            "collision on {id}"
        );
    }
}
