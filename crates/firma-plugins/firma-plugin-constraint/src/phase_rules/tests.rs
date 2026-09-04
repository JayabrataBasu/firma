//! Tests for the `constrain` / `enforce` phase rules — one per §9.1 violation
//! path plus the `T_c` boundary and simultaneous-violation idempotency cases
//! (ADR 0028, ADR 0029).

use std::collections::BTreeMap;

use firma_core::{
    AgentId, Delta, DeltaKind, DeltaTarget, Phase, ResourceKind, RngKey, StreamId, Tick, View,
};
use firma_domain::{keys, Edge, WindowEntry};

use super::*;

// --- a minimal in-memory View (this crate cannot dep another plugin) ---
#[derive(Default)]
struct Mv {
    tick: u64,
    live: Vec<AgentId>,
    stocks: BTreeMap<(u64, String), i64>,
    reals: BTreeMap<(u64, String), f64>,
    ints: BTreeMap<(u64, String), i64>,
    greals: BTreeMap<String, f64>,
    gints: BTreeMap<String, i64>,
    alists: BTreeMap<(u64, String), Vec<String>>,
    glists: BTreeMap<String, Vec<String>>,
    resources: Vec<ResourceKind>,
}
impl Mv {
    fn new(tick: u64, live: &[u64]) -> Self {
        Mv {
            tick,
            live: live.iter().copied().map(AgentId).collect(),
            resources: vec![ResourceKind("capital".into()), ResourceKind("input".into())],
            ..Default::default()
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
    fn gr(mut self, f: &str, v: f64) -> Self {
        self.greals.insert(f.into(), v);
        self
    }
    fn gi(mut self, f: &str, v: i64) -> Self {
        self.gints.insert(f.into(), v);
        self
    }
    fn window(mut self, a: u64, actions: &[u8]) -> Self {
        self.alists.insert(
            (a, keys::ACTION_WINDOW.into()),
            actions
                .iter()
                .enumerate()
                .map(|(i, &x)| WindowEntry::new(i as u64, x).to_json())
                .collect(),
        );
        self
    }
    fn edge(mut self, e: Edge) -> Self {
        self.glists
            .entry(keys::RELATION_EDGES.into())
            .or_default()
            .push(serde_json::to_string(&e).unwrap());
        self
    }
    fn theta(self) -> Self {
        // slack θ unless a test overrides
        self.gr(keys::THETA_LIMIT, 0.90)
            .gr(keys::THETA_CAP, 0.40)
            .gi(keys::THETA_Q, 100)
    }
}
impl View for Mv {
    fn tick(&self) -> Tick {
        Tick(self.tick)
    }
    fn phase(&self) -> Phase {
        Phase::Enforce
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
    fn agent_records(&self, a: AgentId, l: &str) -> &[String] {
        self.alists
            .get(&(a.0, l.to_owned()))
            .map_or(&[], Vec::as_slice)
    }
    fn global_records(&self, l: &str) -> &[String] {
        self.glists.get(l).map_or(&[], Vec::as_slice)
    }
}

fn key() -> RngKey {
    RngKey {
        run_seed: 1,
        stream: StreamId::Mechanism,
        plugin_id: 0,
        phase: Phase::Enforce,
        tick: 0,
        agent_id: None,
    }
}

fn enforce(p_q: i64) -> Enforce {
    Enforce::new(EnforceParams {
        t_c: 4,
        p_c: 30,
        delta_lambda: 0.15,
        p_q,
        l_w: 8,
    })
    .unwrap()
}

fn removed(deltas: &[Delta], agent: u64) -> Option<String> {
    deltas.iter().find_map(|d| match (&d.target, &d.kind) {
        (DeltaTarget::Agent(a), DeltaKind::RemoveAgent { reason }) if a.0 == agent => {
            Some(reason.clone())
        }
        _ => None,
    })
}
fn agent_capital_delta(deltas: &[Delta], agent: u64) -> i64 {
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
fn is_conserving(deltas: &[Delta], resource: &str) -> bool {
    let t: i64 = deltas
        .iter()
        .filter_map(|d| match &d.kind {
            DeltaKind::AdjustStock {
                resource: r,
                amount,
            } if r.0 == resource => Some(*amount),
            _ => None,
        })
        .sum();
    t == 0
}

// ==================== constrain (ActionWindow) ====================

#[test]
fn action_window_appends_and_trims_to_l_w() {
    let rule = ActionWindow::new(ActionWindowParams { l_w: 4 }).unwrap();
    // W already has 4 entries [0,1,2,3]; this tick's action is 2.
    let v = Mv::new(9, &[0])
        .window(0, &[0, 1, 2, 3])
        .int(0, keys::SELECTED_ACTION, 2);
    let d = rule.apply(&v, key());
    let new_w: Vec<WindowEntry> = match &d[0].kind {
        DeltaKind::ReplaceAgentList { list, records_json } if list == keys::ACTION_WINDOW => {
            records_json
                .iter()
                .map(|s| WindowEntry::from_json(s).unwrap())
                .collect()
        }
        _ => panic!(),
    };
    // oldest dropped, newest appended, length == L_W
    assert_eq!(new_w.len(), 4);
    assert_eq!(
        new_w.iter().map(|e| e.action).collect::<Vec<_>>(),
        vec![1, 2, 3, 2]
    );
    assert_eq!(new_w.last().unwrap().tick, 9);
}

#[test]
fn action_window_no_decision_records_hold() {
    let rule = ActionWindow::new(ActionWindowParams::default()).unwrap();
    let v = Mv::new(0, &[0]); // no SELECTED_ACTION
    let d = rule.apply(&v, key());
    match &d[0].kind {
        DeltaKind::ReplaceAgentList { records_json, .. } => {
            let e = WindowEntry::from_json(&records_json[0]).unwrap();
            assert_eq!(e.action, 0);
        }
        _ => panic!(),
    }
}

#[test]
fn u_moves_from_produce_regulated_then_ages_out() {
    use firma_domain::margin::u_from_window;
    let w1 = vec![WindowEntry::new(0, 2)]; // one regulated in an 8-window
    assert!((u_from_window(&w1, 8) - 1.0 / 8.0).abs() < 1e-12);
    // 8 ticks later it has aged past the L_W boundary
    let w2: Vec<WindowEntry> = (0..8).map(|i| WindowEntry::new(i + 1, 0)).collect();
    assert!((u_from_window(&w2, 8) - 0.0).abs() < 1e-12);
}

// ==================== enforce: solvency (Death) ====================

#[test]
fn solvency_death_at_zero_capital() {
    // r^L == 0 ⇒ g_1 = 0 ⇒ Death fires at the boundary (ADR 0029).
    let v = Mv::new(5, &[0])
        .theta()
        .stock(0, "capital", 0)
        .stock(0, "input", 7) // held input → transferred to env on death
        .real(0, keys::CAPABILITY, 0.9)
        .int(0, keys::OBLIGATION, 0);
    let d = enforce(10).apply(&v, key());
    assert_eq!(removed(&d, 0).as_deref(), Some("solvency"));
    // input (7) transferred to the market, conserving
    assert!(is_conserving(&d, "input"));
    let inp: i64 = d
        .iter()
        .filter_map(|x| match (&x.target, &x.kind) {
            (DeltaTarget::Agent(a), DeltaKind::AdjustStock { resource, amount })
                if a.0 == 0 && resource.0 == "input" =>
            {
                Some(*amount)
            }
            _ => None,
        })
        .sum();
    assert_eq!(inp, -7);
}

#[test]
fn positive_capital_is_not_a_solvency_death() {
    let v = Mv::new(5, &[0])
        .theta()
        .stock(0, "capital", 1)
        .real(0, keys::CAPABILITY, 0.9);
    assert!(removed(&enforce(10).apply(&v, key()), 0).is_none());
}

// ==================== enforce: compliance (Graduated) ====================

#[test]
fn compliance_first_strike_penalty_and_legitimacy_loss() {
    // u = 6/8 = 0.75, θ_limit = 0.5 ⇒ g_2 = 0.25 > 0. No prior violation.
    let v = Mv::new(20, &[0])
        .gr(keys::THETA_LIMIT, 0.50)
        .gr(keys::THETA_CAP, 0.40)
        .gi(keys::THETA_Q, 100)
        .stock(0, "capital", 200)
        .real(0, keys::CAPABILITY, 0.9)
        .real(0, keys::LEGITIMACY, 0.8)
        .int(0, keys::OBLIGATION, 0)
        .window(0, &[2, 2, 2, 2, 2, 2, 0, 0]);
    let d = enforce(10).apply(&v, key());
    assert!(removed(&d, 0).is_none(), "first strike is not lethal");
    assert_eq!(agent_capital_delta(&d, 0), -30, "P_c = 30 fined");
    assert!(is_conserving(&d, "capital"));
    // λ -= δ_λ (0.15)
    let dlam = d.iter().find_map(|x| match (&x.target, &x.kind) {
        (DeltaTarget::Agent(a), DeltaKind::AdjustAgentReal { field, delta })
            if a.0 == 0 && field == keys::LEGITIMACY =>
        {
            Some(*delta)
        }
        _ => None,
    });
    assert!((dlam.unwrap() - (-0.15)).abs() < 1e-12);
    // last-violation tick recorded = 20
    let t = d.iter().find_map(|x| match (&x.target, &x.kind) {
        (DeltaTarget::Agent(a), DeltaKind::SetAgentInt { field, value })
            if a.0 == 0 && field == keys::COMPLIANCE_LAST_VIOLATION_TICK =>
        {
            Some(*value)
        }
        _ => None,
    });
    assert_eq!(t, Some(20));
}

#[test]
fn compliance_second_strike_within_tc_is_death() {
    // prior violation at tick 18, now tick 22 → gap 4 == T_c → INCLUSIVE ⇒ death.
    let base = || {
        Mv::new(22, &[0])
            .gr(keys::THETA_LIMIT, 0.50)
            .gr(keys::THETA_CAP, 0.40)
            .gi(keys::THETA_Q, 100)
            .stock(0, "capital", 200)
            .real(0, keys::CAPABILITY, 0.9)
            .real(0, keys::LEGITIMACY, 0.8)
            .int(0, keys::OBLIGATION, 0)
            .window(0, &[2, 2, 2, 2, 2, 2, 0, 0])
    };
    let d = enforce(10).apply(
        &base().int(0, keys::COMPLIANCE_LAST_VIOLATION_TICK, 18),
        key(),
    );
    assert_eq!(removed(&d, 0).as_deref(), Some("compliance"));
    // no penalty — the firm is dying
    assert!(!d
        .iter()
        .any(|x| matches!(&x.kind, DeltaKind::SetAgentInt { field, .. } if field == keys::COMPLIANCE_LAST_VIOLATION_TICK)));
}

#[test]
fn compliance_restrike_after_tc_is_a_fresh_first_strike() {
    // prior at tick 10, now tick 15 → gap 5 > T_c=4 → first strike again.
    let v = Mv::new(15, &[0])
        .gr(keys::THETA_LIMIT, 0.50)
        .gr(keys::THETA_CAP, 0.40)
        .gi(keys::THETA_Q, 100)
        .stock(0, "capital", 200)
        .real(0, keys::CAPABILITY, 0.9)
        .real(0, keys::LEGITIMACY, 0.8)
        .int(0, keys::OBLIGATION, 0)
        .window(0, &[2, 2, 2, 2, 2, 2, 0, 0])
        .int(0, keys::COMPLIANCE_LAST_VIOLATION_TICK, 10);
    let d = enforce(10).apply(&v, key());
    assert!(removed(&d, 0).is_none(), "gap > T_c ⇒ not a second strike");
    assert_eq!(agent_capital_delta(&d, 0), -30);
    // tick updated to 15
    let t = d.iter().find_map(|x| match &x.kind {
        DeltaKind::SetAgentInt { field, value }
            if field == keys::COMPLIANCE_LAST_VIOLATION_TICK =>
        {
            Some(*value)
        }
        _ => None,
    });
    assert_eq!(t, Some(15));
}

// ==================== enforce: scope (AdmissibilityGate — no-op) ====================

#[test]
fn scope_violation_does_nothing_in_enforce() {
    // c = 0.2 < θ_cap = 0.4 ⇒ g_3 = 0.2 > 0. Nothing else violated.
    let v = Mv::new(5, &[0])
        .theta()
        .stock(0, "capital", 200)
        .real(0, keys::CAPABILITY, 0.2)
        .int(0, keys::OBLIGATION, 0);
    let d = enforce(10).apply(&v, key());
    assert!(
        d.is_empty(),
        "scope is a decide-phase gate — enforce emits nothing"
    );
}

// ==================== enforce: obligation (Relational) ====================

#[test]
fn obligation_severs_supply_edges_and_fines_but_survives() {
    // q = 120 > θ_Q = 100 ⇒ g_4 = 20 > 0.
    let v = Mv::new(5, &[0, 9])
        .theta()
        .stock(0, "capital", 200)
        .real(0, keys::CAPABILITY, 0.9)
        .int(0, keys::OBLIGATION, 120)
        .stock(9, "capital", 200)
        .real(9, keys::CAPABILITY, 0.9)
        .int(9, keys::OBLIGATION, 0)
        .edge(Edge::supply(9, 0, 5)) // 9 → 0 supply (severed)
        .edge(Edge {
            source: 0,
            target: 9,
            kind: firma_domain::EdgeKind::Alliance,
            weight: 0,
            age: 0,
        }); // alliance (NOT severed — only supply)
    let d = enforce(7).apply(&v, key());
    assert!(removed(&d, 0).is_none(), "obligation is never lethal");
    assert_eq!(agent_capital_delta(&d, 0), -7, "P_q = 7");
    // one ReplaceGlobalList; the supply edge gone, the alliance kept
    let survivors: Vec<Edge> = d
        .iter()
        .find_map(|x| match &x.kind {
            DeltaKind::ReplaceGlobalList { list, records_json } if list == keys::RELATION_EDGES => {
                Some(
                    records_json
                        .iter()
                        .map(|s| serde_json::from_str(s).unwrap())
                        .collect(),
                )
            }
            _ => None,
        })
        .expect("a ReplaceGlobalList");
    assert_eq!(survivors.len(), 1);
    assert_eq!(survivors[0].kind, firma_domain::EdgeKind::Alliance);
}

// ==================== ordering / simultaneity / idempotency ====================

#[test]
fn simultaneous_solvency_and_compliance_death_is_one_removal_no_penalty() {
    // r^L == 0 (solvency) AND u > θ_limit with a prior violation in-window
    // (compliance second strike). One RemoveAgent, cause "solvency+compliance",
    // no fine, no last-violation-tick write.
    let v = Mv::new(12, &[0])
        .gr(keys::THETA_LIMIT, 0.50)
        .gr(keys::THETA_CAP, 0.40)
        .gi(keys::THETA_Q, 100)
        .stock(0, "capital", 0)
        .stock(0, "input", 3)
        .real(0, keys::CAPABILITY, 0.9)
        .real(0, keys::LEGITIMACY, 0.8)
        .int(0, keys::OBLIGATION, 0)
        .window(0, &[2, 2, 2, 2, 2, 2, 0, 0])
        .int(0, keys::COMPLIANCE_LAST_VIOLATION_TICK, 10); // gap 2 ≤ T_c
    let d = enforce(10).apply(&v, key());
    let reasons: Vec<_> = d
        .iter()
        .filter(|x| matches!(&x.kind, DeltaKind::RemoveAgent { .. }))
        .collect();
    assert_eq!(reasons.len(), 1, "exactly one RemoveAgent");
    assert_eq!(removed(&d, 0).as_deref(), Some("solvency+compliance"));
    // no compliance penalty / λ / tick — the firm is dead
    assert!(!d.iter().any(
        |x| matches!(&x.kind, DeltaKind::AdjustAgentReal { field, .. } if field == keys::LEGITIMACY)
    ));
    assert!(!d
        .iter()
        .any(|x| matches!(&x.kind, DeltaKind::SetAgentInt { field, .. } if field == keys::COMPLIANCE_LAST_VIOLATION_TICK)));
    // only the input (3) is transferred (capital is already 0)
    assert!(is_conserving(&d, "input"));
    assert!(is_conserving(&d, "capital"));
}

#[test]
fn simultaneous_compliance_first_strike_and_obligation_violation_is_one_capital_delta() {
    // u = 6/8 = 0.75 > θ_limit = 0.50 ⇒ compliance first strike (P_c = 30).
    // q = 120 > θ_Q = 100 ⇒ obligation violation (P_q = 7). Neither is lethal
    // on its own, and nothing in §9.1 says the four constraints are mutually
    // exclusive within one tick — so both fire for the same agent at once.
    // Found via the Stage-6b `decision.random` arm-scoping probe: a
    // `DuplicateDelta { plugin: "constraint.enforce" }` kernel abort, because
    // `Enforce::apply` called `to_env(agent, capital, ..)` once per violated
    // semantic instead of once per agent.
    let v = Mv::new(20, &[0])
        .gr(keys::THETA_LIMIT, 0.50)
        .gr(keys::THETA_CAP, 0.40)
        .gi(keys::THETA_Q, 100)
        .stock(0, "capital", 200)
        .real(0, keys::CAPABILITY, 0.9)
        .real(0, keys::LEGITIMACY, 1.0)
        .int(0, keys::OBLIGATION, 120)
        .window(0, &[2, 2, 2, 2, 2, 2, 0, 0]);
    let d = enforce(7).apply(&v, key()); // p_c = 30 (fixed by `enforce()`), p_q = 7

    let capital_deltas: Vec<i64> = d
        .iter()
        .filter_map(|x| match (&x.target, &x.kind) {
            (DeltaTarget::Agent(a), DeltaKind::AdjustStock { resource, amount })
                if a.0 == 0 && resource.0 == "capital" =>
            {
                Some(*amount)
            }
            _ => None,
        })
        .collect();
    eprintln!("capital deltas on agent 0: {capital_deltas:?}");

    assert_eq!(
        capital_deltas.len(),
        1,
        "enforce must emit exactly one capital-penalty delta per agent per tick \
         (P_c + P_q merged), not one per violated semantic — two same-agent, \
         same-resource deltas from one rule is exactly what the kernel's \
         per-rule uniqueness guard (ADR-0033) rejects as DuplicateDelta"
    );
    assert_eq!(
        capital_deltas[0], -37,
        "P_c(30) + P_q(7) = 37, jointly capped by the starting capital (200)"
    );
    assert!(is_conserving(&d, "capital"));
    assert!(removed(&d, 0).is_none(), "neither violation is lethal");
    // the compliance side-effects (λ loss, last-violation-tick) still fire
    assert!(d.iter().any(
        |x| matches!(&x.kind, DeltaKind::AdjustAgentReal { field, .. } if field == keys::LEGITIMACY)
    ));
    assert!(d.iter().any(
        |x| matches!(&x.kind, DeltaKind::SetAgentInt { field, .. } if field == keys::COMPLIANCE_LAST_VIOLATION_TICK)
    ));
    // the obligation side-effect (supply-edge severing) is not exercised here
    // (no edges configured) — covered by `obligation_severs_supply_edges_and_fines_but_survives`.
}

#[test]
fn two_agents_dying_share_one_replace_global_list() {
    let v = Mv::new(5, &[0, 1])
        .theta()
        .stock(0, "capital", 0)
        .stock(1, "capital", 0)
        .real(0, keys::CAPABILITY, 0.9)
        .real(1, keys::CAPABILITY, 0.9)
        .edge(Edge::supply(0, 1, 3)); // 0 → 1
    let d = enforce(10).apply(&v, key());
    assert_eq!(
        d.iter()
            .filter(|x| matches!(&x.kind, DeltaKind::RemoveAgent { .. }))
            .count(),
        2
    );
    assert_eq!(
        d.iter()
            .filter(|x| matches!(&x.kind, DeltaKind::ReplaceGlobalList { .. }))
            .count(),
        1,
        "one edge-list rewrite for the whole phase"
    );
}

#[test]
fn enforce_p_q_is_required_and_params_validated() {
    assert!(serde_json::from_str::<EnforceParams>("{}").is_err()); // no p_q
    assert!(serde_json::from_str::<EnforceParams>(r#"{"p_q":25}"#).is_ok());
    assert!(Enforce::new(EnforceParams {
        t_c: 0,
        p_c: 30,
        delta_lambda: 0.15,
        p_q: 25,
        l_w: 8
    })
    .is_err());
}

#[test]
fn phase_rules_register_and_have_the_right_phases() {
    for (id, _, ctor) in registered_rules() {
        let params = if id == catalog::ENFORCE_ID {
            serde_json::json!({ "p_q": 25 })
        } else {
            serde_json::Value::Null
        };
        let r = ctor(&params).unwrap();
        assert!(!r.assumption().trim().is_empty());
        let want = if id == catalog::ENFORCE_ID {
            Phase::Enforce
        } else {
            Phase::Constrain
        };
        assert_eq!(r.phase(), want, "{id}");
    }
}
