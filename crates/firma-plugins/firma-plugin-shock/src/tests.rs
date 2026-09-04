//! Unit tests for the shock rules against an in-memory `View`.

use std::collections::BTreeMap;

use firma_core::{
    AgentId, DeltaKind, DeltaTarget, Phase, ResourceKind, RngKey, StreamId, Tick, View,
};
use firma_domain::{
    keys, Persistence, Ramp, RegulatoryTarget, Shock, ShockChannel, ShockObservability,
    ShockTargets,
};

use super::*;

struct Mv {
    tick: u64,
    live: Vec<AgentId>,
    gr: BTreeMap<String, f64>,
    gi: BTreeMap<String, i64>,
    gl: BTreeMap<String, Vec<String>>,
}
impl Mv {
    fn new(tick: u64, live: &[u64]) -> Mv {
        Mv {
            tick,
            live: live.iter().copied().map(AgentId).collect(),
            gr: BTreeMap::new(),
            gi: BTreeMap::new(),
            gl: BTreeMap::new(),
        }
    }
    fn rec(mut self, list: &str, r: String) -> Self {
        self.gl.entry(list.into()).or_default().push(r);
        self
    }
    fn key(&self, stream: StreamId) -> RngKey {
        RngKey {
            run_seed: 999,
            stream,
            plugin_id: 5,
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
        999
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
        self.gr.get(f).copied()
    }
    fn global_int(&self, f: &str) -> Option<i64> {
        self.gi.get(f).copied()
    }
    fn global_records(&self, l: &str) -> &[String] {
        self.gl.get(l).map_or(&[], Vec::as_slice)
    }
}

fn reg_shock(id: &str, onset: u64, m: f64, persistence: Persistence, ramp: Ramp) -> Shock {
    Shock {
        id: id.into(),
        channel: ShockChannel::Regulatory {
            target: RegulatoryTarget::ThetaLimit,
        },
        magnitude: m,
        onset,
        ramp,
        persistence,
        observability: ShockObservability::Full,
        novelty: 0.8,
        targets: ShockTargets::All,
    }
}

/// Drive `scheduled` forward tick by tick, threading `ACTIVE_SHOCKS` by hand,
/// and collect the θ_limit deltas per tick.
fn run_scheduled(shocks: Vec<Shock>, ticks: u64) -> Vec<(u64, Vec<f64>)> {
    let rule = Scheduled::new(ScheduledParams { shocks }).unwrap();
    let mut active: Vec<String> = Vec::new();
    let mut trace = Vec::new();
    for t in 0..ticks {
        let mut v = Mv::new(t, &[0, 1]);
        for r in &active {
            v = v.rec(keys::ACTIVE_SHOCKS, r.clone());
        }
        let d = rule.apply(&v, v.key(StreamId::Environment));
        let mut tl = Vec::new();
        for x in &d {
            match &x.kind {
                DeltaKind::AdjustGlobalReal { field, delta } if field == keys::THETA_LIMIT => {
                    tl.push(*delta);
                }
                DeltaKind::ReplaceGlobalList { list, records_json }
                    if list == keys::ACTIVE_SHOCKS =>
                {
                    active = records_json.clone();
                }
                _ => {}
            }
        }
        trace.push((t, tl));
    }
    trace
}

#[test]
fn instant_permanent_regulatory_is_one_step_then_silent() {
    let trace = run_scheduled(
        vec![reg_shock(
            "s",
            3,
            0.3,
            Persistence::Permanent,
            Ramp::Instant,
        )],
        8,
    );
    // No delta before onset.
    for (t, tl) in &trace {
        if *t < 3 {
            assert!(tl.is_empty(), "tick {t} moved θ_limit");
        }
    }
    // One −0.3 step at onset.
    assert_eq!(trace[3].1, vec![-0.3]);
    // Silent afterward (target reached, no more deltas).
    for (t, tl) in &trace {
        if *t > 3 {
            assert!(tl.is_empty(), "tick {t} kept moving θ_limit");
        }
    }
}

#[test]
fn linear_ramp_spreads_the_shift_over_duration() {
    let trace = run_scheduled(
        vec![reg_shock(
            "s",
            0,
            0.4,
            Persistence::Permanent,
            Ramp::Linear { duration: 4 },
        )],
        7,
    );
    // dt=0: 0; dt=1..4: −0.1 each; then silent.
    let steps: Vec<f64> = trace.iter().flat_map(|(_, tl)| tl.clone()).collect();
    assert_eq!(steps.len(), 4);
    for s in &steps {
        assert!((s - (-0.1)).abs() < 1e-9, "step {s}");
    }
    assert!((steps.iter().sum::<f64>() - (-0.4)).abs() < 1e-9);
}

#[test]
fn transient_reverses_when_the_window_closes() {
    let trace = run_scheduled(
        vec![reg_shock(
            "s",
            2,
            0.3,
            Persistence::Transient { duration: 3 },
            Ramp::Instant,
        )],
        8,
    );
    // onset tick 2: −0.3; ticks 3,4 inside (silent); tick 5 (dt=3, closed): +0.3 reversal.
    assert_eq!(trace[2].1, vec![-0.3]);
    assert!(trace[3].1.is_empty());
    assert!(trace[4].1.is_empty());
    assert_eq!(trace[5].1, vec![0.3]);
    assert!(trace[6].1.is_empty());
    // Net effect over the run: zero.
    let net: f64 = trace.iter().flat_map(|(_, tl)| tl.clone()).sum();
    assert!(net.abs() < 1e-12);
}

#[test]
fn reputational_hits_only_targeted_live_agents() {
    let shock = Shock {
        channel: ShockChannel::Reputational,
        targets: ShockTargets::Set { ids: vec![1] },
        ..reg_shock("rep", 0, 0.2, Persistence::Permanent, Ramp::Instant)
    };
    let rule = Scheduled::new(ScheduledParams {
        shocks: vec![shock],
    })
    .unwrap();
    let v = Mv::new(0, &[0, 1, 2]);
    let d = rule.apply(&v, v.key(StreamId::Environment));
    let hits: Vec<u64> = d
        .iter()
        .filter_map(|x| match (&x.target, &x.kind) {
            (DeltaTarget::Agent(a), DeltaKind::AdjustAgentReal { field, delta })
                if field == keys::LEGITIMACY =>
            {
                assert!((delta - (-0.2)).abs() < 1e-12);
                Some(a.0)
            }
            _ => None,
        })
        .collect();
    assert_eq!(hits, vec![1]);
}

#[test]
fn resource_channel_rounds_the_cumulative_target() {
    // magnitude 1.6 on a Linear(4) ramp: cumulative target 0.4, 0.8, 1.2, 1.6
    // ⇒ rounded 0, 1, 1, 2 ⇒ integer steps +0, +1, +0, +1.
    let shock = Shock {
        channel: ShockChannel::Resource,
        ..reg_shock(
            "res",
            0,
            1.6,
            Persistence::Permanent,
            Ramp::Linear { duration: 4 },
        )
    };
    let rule = Scheduled::new(ScheduledParams {
        shocks: vec![shock],
    })
    .unwrap();
    let mut active: Vec<String> = Vec::new();
    let mut steps = Vec::new();
    for t in 0..6 {
        let mut v = Mv::new(t, &[0]);
        for r in &active {
            v = v.rec(keys::ACTIVE_SHOCKS, r.clone());
        }
        for x in rule.apply(&v, v.key(StreamId::Environment)) {
            match &x.kind {
                DeltaKind::AdjustGlobalInt { field, delta } if field == keys::INPUT_PRICE => {
                    steps.push(*delta);
                }
                DeltaKind::ReplaceGlobalList { records_json, .. } => active = records_json.clone(),
                _ => {}
            }
        }
    }
    assert_eq!(steps.iter().sum::<i64>(), 2, "cumulative rounds to +2");
    assert!(
        steps.iter().all(|s| *s == 1),
        "each emitted step is +1 (no drift)"
    );
}

#[test]
fn stochastic_draws_within_window_and_is_deterministic() {
    let p = StochasticParams {
        id: "z".into(),
        channel: ShockChannel::Regulatory {
            target: RegulatoryTarget::ThetaLimit,
        },
        onset_min: 50,
        onset_max: 60,
        magnitude_mean: 0.3,
        magnitude_sd: 0.0,
        ramp: Ramp::Instant,
        persistence: Persistence::Permanent,
        observability: ShockObservability::Full,
        novelty: 0.5,
        targets: ShockTargets::All,
    };
    let a = Stochastic::new(p.clone()).unwrap();
    let v = Mv::new(0, &[0]);
    let k = v.key(StreamId::Shock);
    let s1 = a.draw(&k);
    let s2 = a.draw(&k);
    assert_eq!(s1, s2);
    assert!((50..=60).contains(&s1.onset));
    assert!((s1.magnitude - 0.3).abs() < 1e-12);
    assert_eq!(a.rng_stream(), Some(StreamId::Shock));

    // A different onset window ⇒ (almost surely) a different onset.
    let b = Stochastic::new(StochasticParams {
        onset_min: 200,
        onset_max: 260,
        ..p
    })
    .unwrap();
    assert!(b.draw(&k).onset >= 200);
}

#[test]
fn params_validated() {
    assert!(scheduled(&serde_json::json!({ "shocks": [] })).is_ok());
    assert!(stochastic(&serde_json::json!({
        "id": "z", "channel": {"kind":"resource"}, "onset_min": 10, "onset_max": 5,
        "magnitude_mean": 0.1, "magnitude_sd": 0.0, "ramp": {"kind":"instant"},
        "persistence": {"kind":"permanent"}, "observability": {"kind":"full"},
        "novelty": 0.5, "targets": {"kind":"all"}
    }))
    .is_err()); // onset_min > onset_max
    for (id, _h, ctor) in registered() {
        let _ = ctor(&serde_json::json!({ "shocks": [] })).or_else(|_| {
            ctor(&serde_json::json!({
                "id": "z", "channel": {"kind":"competitive"}, "onset_min": 1, "onset_max": 2,
                "magnitude_mean": 0.1, "magnitude_sd": 0.0, "ramp": {"kind":"instant"},
                "persistence": {"kind":"permanent"}, "observability": {"kind":"full"},
                "novelty": 0.5, "targets": {"kind":"all"}
            }))
        });
        let _ = id;
    }
}
