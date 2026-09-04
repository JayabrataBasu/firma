//! Kernel-level tests with in-crate mock rules (the kernel cannot depend on a
//! plugin crate — §18.1 — so it carries its own fixtures). The full DT/VT gate
//! lives in the `tests/` conformance package.

use std::collections::BTreeMap;

use firma_core::{
    AgentId, ComponentId, ConflictClass, ConflictResolver, Delta, DeltaKind, DeltaKindTag,
    DeltaTarget, Intervention, Phase, PluginId, ResourceKind, RngKey, Rule, View,
};
use semver::Version;

use crate::{AgentState, ExecMode, Kernel, RunSeeds, Schedule, World};

fn res(name: &str) -> ResourceKind {
    ResourceKind(name.to_owned())
}

fn world_with(agents: &[(u64, i64)], env_capital: i64) -> World {
    let mut map = BTreeMap::new();
    for (id, cap) in agents {
        let mut stocks = BTreeMap::new();
        stocks.insert(res("capital"), *cap);
        map.insert(
            AgentId(*id),
            AgentState {
                birth_tick: firma_core::Tick(0),
                stocks,
            },
        );
    }
    let mut env = BTreeMap::new();
    env.insert(res("capital"), env_capital);
    World::new(
        vec![res("capital")],
        RunSeeds {
            mechanism: 42,
            environment: 7,
            shock: 9,
            init: 1,
        },
        map,
        env,
    )
}

/// Moves one unit of capital from agent `from` to agent `to` every tick.
struct Push {
    id: PluginId,
    from: AgentId,
    to: AgentId,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Push {
    fn new(id: &str, from: u64, to: u64) -> Push {
        Push {
            id: PluginId::new(id),
            from: AgentId(from),
            to: AgentId(to),
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock],
        }
    }
}

impl Rule for Push {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> Version {
        Version::new(1, 0, 0)
    }
    fn phase(&self) -> Phase {
        Phase::ActMarket
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        if view.agent_stock(self.from, &res("capital")) < 1 {
            return Vec::new();
        }
        vec![
            Delta {
                target: DeltaTarget::Agent(self.from),
                kind: DeltaKind::AdjustStock {
                    resource: res("capital"),
                    amount: -1,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
            Delta {
                target: DeltaTarget::Agent(self.to),
                kind: DeltaKind::AdjustStock {
                    resource: res("capital"),
                    amount: 1,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
        ]
    }
    fn assumption(&self) -> &str {
        "moves one unit per tick; kernel test fixture"
    }
}

/// A keyed rule that draws each tick and moves `draw % 2` units.
struct KeyedPush {
    id: PluginId,
    from: AgentId,
    to: AgentId,
    purpose: String,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl KeyedPush {
    fn new(id: &str, purpose: &str, from: u64, to: u64) -> KeyedPush {
        KeyedPush {
            id: PluginId::new(id),
            from: AgentId(from),
            to: AgentId(to),
            purpose: purpose.to_owned(),
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock],
        }
    }
}

impl Rule for KeyedPush {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> Version {
        Version::new(1, 0, 0)
    }
    fn phase(&self) -> Phase {
        Phase::ActMarket
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, key: RngKey) -> Vec<Delta> {
        let mut rng = firma_rng::open(&key, &self.purpose);
        let amount = (rng.next_u64() % 2) as i64;
        if amount == 0 || view.agent_stock(self.from, &res("capital")) < amount {
            return Vec::new();
        }
        vec![
            Delta {
                target: DeltaTarget::Agent(self.from),
                kind: DeltaKind::AdjustStock {
                    resource: res("capital"),
                    amount: -amount,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
            Delta {
                target: DeltaTarget::Agent(self.to),
                kind: DeltaKind::AdjustStock {
                    resource: res("capital"),
                    amount,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
        ]
    }
    fn assumption(&self) -> &str {
        "keyed draw each tick; DT-6 fixture"
    }
}

struct NoResolver;
impl ConflictResolver for NoResolver {
    fn id(&self) -> PluginId {
        PluginId::new("conflict.none")
    }
    fn version(&self) -> Version {
        Version::new(1, 0, 0)
    }
    fn handles(&self) -> ConflictClass {
        ConflictClass::ResourcePool
    }
    fn resolve(&self, group: &[Delta]) -> Vec<Delta> {
        group.to_vec()
    }
    fn assumption(&self) -> &str {
        "identity resolver; test fixture"
    }
}

fn run(kernel: &Kernel, world: &mut World, schedule: &Schedule, ticks: u64) -> Vec<crate::Event> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        kernel.step(world, schedule, &mut events).expect("step");
    }
    events
}

#[test]
fn conserves_over_a_run() {
    let kernel = Kernel::new();
    let mut world = world_with(&[(0, 100), (1, 0)], 0);
    let schedule = Schedule::new(
        vec![Box::new(Push::new("t.push", 0, 1))],
        Box::new(NoResolver),
    );
    run(&kernel, &mut world, &schedule, 50);
    assert_eq!(world.agent_stock(AgentId(0), &res("capital")), 50);
    assert_eq!(world.agent_stock(AgentId(1), &res("capital")), 50);
    assert_eq!(world.live_total(&res("capital")), 100);
}

/// Unconditionally transfers one unit of capital from agent 0 to agent 1 every
/// tick, with no affordability check — used to drive agent 0 below zero.
struct ForceDrain {
    id: PluginId,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}
impl Rule for ForceDrain {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> Version {
        Version::new(1, 0, 0)
    }
    fn phase(&self) -> Phase {
        Phase::ActMarket
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, _view: &dyn View, _key: RngKey) -> Vec<Delta> {
        vec![
            Delta {
                target: DeltaTarget::Agent(AgentId(0)),
                kind: DeltaKind::AdjustStock {
                    resource: res("capital"),
                    amount: -1,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
            Delta {
                target: DeltaTarget::Agent(AgentId(1)),
                kind: DeltaKind::AdjustStock {
                    resource: res("capital"),
                    amount: 1,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
        ]
    }
    fn assumption(&self) -> &str {
        "unconditional drain; test fixture for the non-negativity abort"
    }
}

#[test]
fn non_negativity_aborts_the_run() {
    let kernel = Kernel::new();
    let mut world = world_with(&[(0, 2), (1, 0)], 0);
    let schedule = Schedule::new(
        vec![Box::new(ForceDrain {
            id: PluginId::new("t.drain"),
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock],
        })],
        Box::new(NoResolver),
    );
    let mut events = Vec::new();
    let mut err = None;
    for _ in 0..10 {
        if let Err(e) = kernel.step(&mut world, &schedule, &mut events) {
            err = Some(e);
            break;
        }
    }
    // After ticks 0 and 1 agent 0 is at 0; tick 2 would take it to -1 and abort.
    assert!(matches!(
        err,
        Some(crate::KernelError::InvariantViolation(_))
    ));
}

#[test]
fn dt2_agent_storage_order_does_not_matter() {
    let kernel = Kernel::new();
    let schedule = || {
        Schedule::new(
            vec![Box::new(Push::new("t.push", 0, 2))],
            Box::new(NoResolver),
        )
    };
    let mut a = world_with(&[(0, 100), (2, 0), (1, 5)], 0);
    let mut b = world_with(&[(1, 5), (0, 100), (2, 0)], 0);
    let ea = run(&kernel, &mut a, &schedule(), 30);
    let eb = run(&kernel, &mut b, &schedule(), 30);
    assert_eq!(ea, eb);
    assert_eq!(a, b);
}

#[test]
fn dt3_sequential_and_parallel_agree() {
    let seq = Kernel::new();
    let par = Kernel::parallel();
    assert_eq!(par.mode(), ExecMode::Parallel);
    let sched = || {
        Schedule::new(
            vec![
                Box::new(Push::new("t.push", 0, 1)) as Box<dyn Rule>,
                Box::new(KeyedPush::new("t.keyed.a", "a", 1, 0)),
                Box::new(KeyedPush::new("t.keyed.b", "b", 0, 1)),
            ],
            Box::new(NoResolver),
        )
    };
    let mut w1 = world_with(&[(0, 500), (1, 500)], 0);
    let mut w2 = world_with(&[(0, 500), (1, 500)], 0);
    let e1 = run(&seq, &mut w1, &sched(), 100);
    let e2 = run(&par, &mut w2, &sched(), 100);
    assert_eq!(e1, e2);
    assert_eq!(w1, w2);
}

#[test]
fn dt4_null_fork_equals_continuation() {
    let kernel = Kernel::new();
    let sched = || {
        Schedule::new(
            vec![Box::new(Push::new("t.push", 0, 1))],
            Box::new(NoResolver),
        )
    };
    let mut base = world_with(&[(0, 100), (1, 0)], 0);
    run(&kernel, &mut base, &sched(), 10);
    let snap = kernel.snapshot(&base);

    // continuation
    let mut cont = kernel.restore(&snap);
    let cont_events = run(&kernel, &mut cont, &sched(), 20);

    // null fork
    let mut forked = kernel.fork(&snap, &Intervention::Null).unwrap();
    let fork_events = run(&kernel, &mut forked, &sched(), 20);

    assert_eq!(cont_events, fork_events);
    assert_eq!(cont, forked);
}

#[test]
fn dt5_snapshot_restore_continue_equals_uninterrupted() {
    let kernel = Kernel::new();
    let sched = || {
        Schedule::new(
            vec![Box::new(Push::new("t.push", 0, 1))],
            Box::new(NoResolver),
        )
    };

    let mut straight = world_with(&[(0, 100), (1, 0)], 0);
    let straight_events = run(&kernel, &mut straight, &sched(), 40);

    let mut broken = world_with(&[(0, 100), (1, 0)], 0);
    let mut broken_events = run(&kernel, &mut broken, &sched(), 15);
    let snap = kernel.snapshot(&broken);
    let mut resumed = kernel.restore(&snap);
    broken_events.extend(run(&kernel, &mut resumed, &sched(), 25));

    assert_eq!(straight_events, broken_events);
    assert_eq!(straight, resumed);
}

#[test]
fn dt6_adding_a_keyed_rule_does_not_perturb_another_rules_deltas() {
    let kernel = Kernel::new();
    let deltas_from = |events: &[crate::Event], origin: &str| -> Vec<crate::Event> {
        events
            .iter()
            .filter(|e| {
                matches!(e, crate::Event::DeltaApplied { origin: o, .. } if o.as_str() == origin)
            })
            .cloned()
            .collect()
    };

    let base_sched = || {
        Schedule::new(
            vec![
                Box::new(Push::new("t.push", 0, 1)) as Box<dyn Rule>,
                Box::new(KeyedPush::new("t.keyed.a", "purpose-a", 1, 0)),
            ],
            Box::new(NoResolver),
        )
    };
    let extended_sched = || {
        Schedule::new(
            vec![
                Box::new(Push::new("t.push", 0, 1)) as Box<dyn Rule>,
                Box::new(KeyedPush::new("t.keyed.a", "purpose-a", 1, 0)),
                Box::new(KeyedPush::new("t.keyed.b", "purpose-b", 0, 1)),
            ],
            Box::new(NoResolver),
        )
    };

    let mut w_base = world_with(&[(0, 1000), (1, 1000)], 0);
    let mut w_ext = world_with(&[(0, 1000), (1, 1000)], 0);
    let e_base = run(&kernel, &mut w_base, &base_sched(), 200);
    let e_ext = run(&kernel, &mut w_ext, &extended_sched(), 200);

    // The extra keyed rule must not shift rule A's draws (hence its deltas) …
    assert_eq!(
        deltas_from(&e_base, "t.keyed.a"),
        deltas_from(&e_ext, "t.keyed.a")
    );
    // … nor the non-keyed rule's.
    assert_eq!(
        deltas_from(&e_base, "t.push"),
        deltas_from(&e_ext, "t.push")
    );
}

#[test]
fn freeze_rule_intervention_silences_a_rule() {
    let kernel = Kernel::new();
    let sched = || {
        Schedule::new(
            vec![Box::new(Push::new("t.push", 0, 1))],
            Box::new(NoResolver),
        )
    };
    let mut world = world_with(&[(0, 100), (1, 0)], 0);
    run(&kernel, &mut world, &sched(), 10);
    let snap = kernel.snapshot(&world);
    let mut frozen = kernel
        .fork(&snap, &Intervention::FreezeRule(PluginId::new("t.push")))
        .unwrap();
    run(&kernel, &mut frozen, &sched(), 20);
    // No further movement after the freeze.
    assert_eq!(frozen.agent_stock(AgentId(0), &res("capital")), 90);
}

// --------------------------------------------------------------------------
// Phase 2 Stage 2 — the eight new DeltaKind variants (ADR 0022).
// --------------------------------------------------------------------------

/// Emits one of every string-keyed domain-state delta once, in `act_market`.
struct DomainWriter {
    id: PluginId,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}
impl DomainWriter {
    fn new() -> DomainWriter {
        DomainWriter {
            id: PluginId::new("t.domain_writer"),
            reads: vec![ComponentId::ledger()],
            writes: vec![
                DeltaKindTag::SetAgentInt,
                DeltaKindTag::AdjustAgentReal,
                DeltaKindTag::AdjustAgentInt,
                DeltaKindTag::AdjustGlobalReal,
                DeltaKindTag::AdjustGlobalInt,
                DeltaKindTag::PushAgentRecord,
                DeltaKindTag::PushGlobalRecord,
                DeltaKindTag::ReplaceAgentList,
            ],
        }
    }
}
impl Rule for DomainWriter {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> Version {
        Version::new(1, 0, 0)
    }
    fn phase(&self) -> Phase {
        Phase::ActMarket
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        let a = AgentId(0);
        let ind = ConflictClass::Independent;
        let g = |kind| Delta {
            target: DeltaTarget::Global,
            kind,
            conflict_class: ind,
            origin: self.id.clone(),
        };
        let ag = |kind| Delta {
            target: DeltaTarget::Agent(a),
            kind,
            conflict_class: ind,
            origin: self.id.clone(),
        };
        // Only act while the queue is short, so the test converges.
        let round = view.agent_records(a, "lagged_effects").len();
        if round >= 2 {
            // Drain the per-agent list back to one entry.
            return vec![ag(DeltaKind::ReplaceAgentList {
                list: "lagged_effects".to_owned(),
                records_json: vec!["{\"kept\":true}".to_owned()],
            })];
        }
        vec![
            // An opaque per-agent int slot — the kernel never names it; the
            // decide→act hand-off happens to use `keys::SELECTED_ACTION`.
            ag(DeltaKind::SetAgentInt {
                field: "attn".to_owned(),
                value: 3,
            }),
            ag(DeltaKind::AdjustAgentReal {
                field: "capability".to_owned(),
                delta: 0.25,
            }),
            ag(DeltaKind::AdjustAgentInt {
                field: "obligation".to_owned(),
                delta: 2,
            }),
            g(DeltaKind::AdjustGlobalReal {
                field: "theta_limit".to_owned(),
                delta: 0.1,
            }),
            g(DeltaKind::AdjustGlobalInt {
                field: "theta_q".to_owned(),
                delta: 5,
            }),
            ag(DeltaKind::PushAgentRecord {
                list: "lagged_effects".to_owned(),
                record_json: "{\"n\":1}".to_owned(),
            }),
            g(DeltaKind::PushGlobalRecord {
                list: "relation_edges".to_owned(),
                record_json: "{\"e\":1}".to_owned(),
            }),
        ]
    }
    fn assumption(&self) -> &str {
        "writes one of every string-keyed domain delta; kernel test fixture"
    }
}

#[test]
fn new_delta_kinds_round_trip_through_the_reconciler() {
    let kernel = Kernel::new();
    let mut world = world_with(&[(0, 100), (1, 0)], 0);
    let schedule = Schedule::new(vec![Box::new(DomainWriter::new())], Box::new(NoResolver));

    // Tick 1: the seven writes apply additively.
    kernel
        .step(&mut world, &schedule, &mut Vec::new())
        .expect("step");
    assert_eq!(world.agent_int(AgentId(0), "attn"), Some(3));
    assert_eq!(world.agent_real(AgentId(0), "capability"), Some(0.25));
    assert_eq!(world.agent_int(AgentId(0), "obligation"), Some(2));
    assert_eq!(world.global_real("theta_limit"), Some(0.1));
    assert_eq!(world.global_int("theta_q"), Some(5));
    assert_eq!(
        world.agent_records(AgentId(0), "lagged_effects"),
        &["{\"n\":1}".to_owned()]
    );
    assert_eq!(
        world.global_records("relation_edges"),
        &["{\"e\":1}".to_owned()]
    );

    // Tick 2: additive accumulation; `set` semantics for SetAgentInt.
    kernel
        .step(&mut world, &schedule, &mut Vec::new())
        .expect("step");
    assert_eq!(world.agent_int(AgentId(0), "attn"), Some(3)); // set, not +3
    assert_eq!(world.agent_real(AgentId(0), "capability"), Some(0.50)); // 0.25 + 0.25
    assert_eq!(world.global_int("theta_q"), Some(10));
    assert_eq!(world.agent_records(AgentId(0), "lagged_effects").len(), 2);

    // Tick 3: the queue has 2 entries → the rule drains it with ReplaceAgentList.
    kernel
        .step(&mut world, &schedule, &mut Vec::new())
        .expect("step");
    assert_eq!(
        world.agent_records(AgentId(0), "lagged_effects"),
        &["{\"kept\":true}".to_owned()]
    );
}

#[test]
fn domain_state_survives_a_snapshot_round_trip() {
    let kernel = Kernel::new();
    let mut world = world_with(&[(0, 100), (1, 0)], 0);
    let schedule = Schedule::new(vec![Box::new(DomainWriter::new())], Box::new(NoResolver));
    kernel
        .step(&mut world, &schedule, &mut Vec::new())
        .expect("step");

    let snap = kernel.snapshot(&world);
    let restored = kernel.restore(&snap);
    assert_eq!(world, restored);
    assert_eq!(restored.global_int("theta_q"), Some(5));
    assert_eq!(restored.agent_real(AgentId(0), "capability"), Some(0.25));
    assert_eq!(
        restored.agent_records(AgentId(0), "lagged_effects"),
        &["{\"n\":1}".to_owned()]
    );
}

#[test]
fn a_repeated_push_record_is_allowed_but_a_repeated_set_is_not() {
    // Two PushAgentRecord to the same (agent, list) in one phase is fine
    // (allows_repeat); two SetAgentInt to the same (agent, field) is a
    // DuplicateDelta abort.
    struct Twice {
        id: PluginId,
        repeat_push: bool,
        reads: Vec<ComponentId>,
        writes: Vec<DeltaKindTag>,
    }
    impl Rule for Twice {
        fn id(&self) -> PluginId {
            self.id.clone()
        }
        fn version(&self) -> Version {
            Version::new(1, 0, 0)
        }
        fn phase(&self) -> Phase {
            Phase::ActMarket
        }
        fn reads(&self) -> &[ComponentId] {
            &self.reads
        }
        fn writes(&self) -> &[DeltaKindTag] {
            &self.writes
        }
        fn apply(&self, _v: &dyn View, _k: RngKey) -> Vec<Delta> {
            let a = DeltaTarget::Agent(AgentId(0));
            let mk = |kind| Delta {
                target: a.clone(),
                kind,
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            };
            if self.repeat_push {
                vec![
                    mk(DeltaKind::PushAgentRecord {
                        list: "l".to_owned(),
                        record_json: "1".to_owned(),
                    }),
                    mk(DeltaKind::PushAgentRecord {
                        list: "l".to_owned(),
                        record_json: "2".to_owned(),
                    }),
                ]
            } else {
                vec![
                    mk(DeltaKind::SetAgentInt {
                        field: "attn".to_owned(),
                        value: 1,
                    }),
                    mk(DeltaKind::SetAgentInt {
                        field: "attn".to_owned(),
                        value: 2,
                    }),
                ]
            }
        }
        fn assumption(&self) -> &str {
            "emits two same-kind deltas; test fixture"
        }
    }

    let kernel = Kernel::new();
    let writes = vec![DeltaKindTag::PushAgentRecord, DeltaKindTag::SetAgentInt];

    let mut ok_world = world_with(&[(0, 10)], 0);
    let ok = Schedule::new(
        vec![Box::new(Twice {
            id: PluginId::new("t.twice"),
            repeat_push: true,
            reads: vec![ComponentId::ledger()],
            writes: writes.clone(),
        })],
        Box::new(NoResolver),
    );
    kernel
        .step(&mut ok_world, &ok, &mut Vec::new())
        .expect("repeated push is allowed");
    assert_eq!(ok_world.agent_records(AgentId(0), "l").len(), 2);

    let mut bad_world = world_with(&[(0, 10)], 0);
    let bad = Schedule::new(
        vec![Box::new(Twice {
            id: PluginId::new("t.twice"),
            repeat_push: false,
            reads: vec![ComponentId::ledger()],
            writes,
        })],
        Box::new(NoResolver),
    );
    assert!(kernel.step(&mut bad_world, &bad, &mut Vec::new()).is_err());
}

/// The §19.5 uniqueness check, widened in Stage 2 to key on
/// `(target, discriminant, slot)` (ADR 0022 Decision 3):
/// * two `AdjustStock` deltas to the *same agent* for *different resources*
///   (`acquire_input`'s shape) are now **accepted** — Phase 1's `(target,
///   discriminant)` key would have rejected them as a duplicate;
/// * two `AdjustStock` for the *same* resource to the same agent are still
///   **rejected**.
#[test]
fn widened_uniqueness_check_distinguishes_by_slot() {
    /// Emits two `AdjustStock` to agent 0: either different resources
    /// (`capital` + `input`) or the same resource twice.
    struct TwoStocks {
        id: PluginId,
        same_resource: bool,
        reads: Vec<ComponentId>,
        writes: Vec<DeltaKindTag>,
    }
    impl Rule for TwoStocks {
        fn id(&self) -> PluginId {
            self.id.clone()
        }
        fn version(&self) -> Version {
            Version::new(1, 0, 0)
        }
        fn phase(&self) -> Phase {
            Phase::ActMarket
        }
        fn reads(&self) -> &[ComponentId] {
            &self.reads
        }
        fn writes(&self) -> &[DeltaKindTag] {
            &self.writes
        }
        fn apply(&self, _v: &dyn View, _k: RngKey) -> Vec<Delta> {
            let second = if self.same_resource {
                "capital"
            } else {
                "input"
            };
            let mk = |target, resource: &str, amount| Delta {
                target,
                kind: DeltaKind::AdjustStock {
                    resource: res(resource),
                    amount,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            };
            // Two agent-side AdjustStock (the pair the check must allow /
            // reject), each paired with the env pool so the run conserves.
            vec![
                mk(DeltaTarget::Agent(AgentId(0)), "capital", -1),
                mk(DeltaTarget::Environment, "capital", 1),
                mk(DeltaTarget::Agent(AgentId(0)), second, 1),
                mk(DeltaTarget::Environment, second, -1),
            ]
        }
        fn assumption(&self) -> &str {
            "emits two AdjustStock to one agent; slot-uniqueness fixture"
        }
    }

    fn world_two_res(cap: i64, inp: i64) -> World {
        let mut stocks = BTreeMap::new();
        stocks.insert(res("capital"), cap);
        stocks.insert(res("input"), inp);
        let mut agents = BTreeMap::new();
        agents.insert(
            AgentId(0),
            AgentState {
                birth_tick: firma_core::Tick(0),
                stocks,
            },
        );
        let mut env = BTreeMap::new();
        env.insert(res("capital"), 10);
        env.insert(res("input"), 10);
        World::new(
            vec![res("capital"), res("input")],
            RunSeeds {
                mechanism: 1,
                environment: 1,
                shock: 1,
                init: 1,
            },
            agents,
            env,
        )
    }

    let kernel = Kernel::new();
    let writes = vec![DeltaKindTag::AdjustStock];

    // Different slots ⇒ accepted; both applied.
    let mut ok_world = world_two_res(5, 5);
    let ok = Schedule::new(
        vec![Box::new(TwoStocks {
            id: PluginId::new("t.two_stocks"),
            same_resource: false,
            reads: vec![ComponentId::ledger()],
            writes: writes.clone(),
        })],
        Box::new(NoResolver),
    );
    kernel
        .step(&mut ok_world, &ok, &mut Vec::new())
        .expect("two different-slot AdjustStock to one agent is accepted");
    assert_eq!(ok_world.agent_stock(AgentId(0), &res("capital")), 4);
    assert_eq!(ok_world.agent_stock(AgentId(0), &res("input")), 6);

    // Same slot twice ⇒ genuine duplicate ⇒ rejected.
    let mut bad_world = world_two_res(5, 5);
    let bad = Schedule::new(
        vec![Box::new(TwoStocks {
            id: PluginId::new("t.two_stocks"),
            same_resource: true,
            reads: vec![ComponentId::ledger()],
            writes,
        })],
        Box::new(NoResolver),
    );
    let err = kernel
        .step(&mut bad_world, &bad, &mut Vec::new())
        .expect_err("two same-slot AdjustStock to one agent is a DuplicateDelta");
    assert!(
        matches!(err, crate::KernelError::DuplicateDelta { .. }),
        "expected DuplicateDelta, got {err:?}"
    );
}

/// Stage-2 review Fix 1 (§21.4; ADR 0022 Decision 2): a real-valued delta with
/// a non-finite payload is rejected with `KernelError::NonFiniteDelta` in the
/// same pre-apply validation loop that catches undeclared / duplicate deltas,
/// so it goes through the identical abort path — **nothing** from that phase is
/// applied, and `step()` returns `Err` (the orchestrator then never writes the
/// tick's events, exactly as for an `InvariantViolation`; see
/// `tests/tests/determinism.rs::tick_is_atomic_wrt_event_log_on_invariant_abort`).
#[test]
fn non_finite_real_delta_is_rejected_atomically() {
    /// Emits, to agent 0 in one phase: a *valid* `AdjustAgentReal` on
    /// `capability` **and** a non-finite one on `legitimacy` (`NaN` or `+∞`).
    struct BadReal {
        id: PluginId,
        infinite: bool,
        reads: Vec<ComponentId>,
        writes: Vec<DeltaKindTag>,
    }
    impl Rule for BadReal {
        fn id(&self) -> PluginId {
            self.id.clone()
        }
        fn version(&self) -> Version {
            Version::new(1, 0, 0)
        }
        fn phase(&self) -> Phase {
            Phase::ActMarket
        }
        fn reads(&self) -> &[ComponentId] {
            &self.reads
        }
        fn writes(&self) -> &[DeltaKindTag] {
            &self.writes
        }
        fn apply(&self, _v: &dyn View, _k: RngKey) -> Vec<Delta> {
            let bad = if self.infinite {
                f64::INFINITY
            } else {
                f64::NAN
            };
            let mk = |field: &str, delta| Delta {
                target: DeltaTarget::Agent(AgentId(0)),
                kind: DeltaKind::AdjustAgentReal {
                    field: field.to_owned(),
                    delta,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            };
            vec![mk("capability", 0.1), mk("legitimacy", bad)]
        }
        fn assumption(&self) -> &str {
            "emits one valid and one non-finite real delta; Fix-1 fixture"
        }
    }

    for infinite in [false, true] {
        let kernel = Kernel::new();
        let mut world = world_with(&[(0, 10)], 0);
        let schedule = Schedule::new(
            vec![Box::new(BadReal {
                id: PluginId::new("t.bad_real"),
                infinite,
                reads: vec![ComponentId::ledger()],
                writes: vec![DeltaKindTag::AdjustAgentReal],
            })],
            Box::new(NoResolver),
        );

        let err = kernel
            .step(&mut world, &schedule, &mut Vec::new())
            .expect_err("a non-finite real delta must abort the step");
        match err {
            crate::KernelError::NonFiniteDelta { plugin, field } => {
                assert_eq!(plugin, "t.bad_real");
                assert_eq!(field, "legitimacy");
            }
            other => panic!("expected NonFiniteDelta, got {other:?}"),
        }

        // Atomicity: the *valid* delta from the same phase was NOT applied —
        // the loop returned Err before `apply_delta` ran for anything.
        assert_eq!(
            world.agent_real(AgentId(0), "capability"),
            None,
            "the phase's valid delta leaked past a rejected non-finite sibling"
        );
        assert_eq!(world.agent_real(AgentId(0), "legitimacy"), None);
        // World was not advanced past tick 0.
        assert_eq!(world.tick, firma_core::Tick(0));
    }
}

/// One rule, in one phase, transferring several agents' residual stock to the
/// environment pool and then removing those agents — the `enforce` shape (ADR
/// 0029). Exercises **ADR 0031** (two `ResourcePool` `AdjustStock`s to the same
/// `(Environment, "capital")` cell from one rule are accepted, not a
/// `DuplicateDelta`) and **ADR 0032** (each `RemoveAgent` — `Independent`, so
/// it sorts *before* its own `ResourcePool` transfer — is nonetheless applied
/// last, so the transfer lands before the agent is gone). Conservation holds
/// throughout.
#[test]
fn resourcepool_transfers_then_removeagent_conserves() {
    struct DrainAndRemove {
        id: PluginId,
        reads: Vec<ComponentId>,
        writes: Vec<DeltaKindTag>,
    }
    impl Rule for DrainAndRemove {
        fn id(&self) -> PluginId {
            self.id.clone()
        }
        fn version(&self) -> Version {
            Version::new(1, 0, 0)
        }
        fn phase(&self) -> Phase {
            Phase::Enforce
        }
        fn reads(&self) -> &[ComponentId] {
            &self.reads
        }
        fn writes(&self) -> &[DeltaKindTag] {
            &self.writes
        }
        fn apply(&self, view: &dyn View, _k: RngKey) -> Vec<Delta> {
            let mut out = Vec::new();
            for &agent in view.live_agents() {
                let q = view.agent_stock(agent, &res("capital"));
                // paired ResourcePool transfer: agent -> env pool
                out.push(Delta {
                    target: DeltaTarget::Agent(agent),
                    kind: DeltaKind::AdjustStock {
                        resource: res("capital"),
                        amount: -q,
                    },
                    conflict_class: ConflictClass::ResourcePool,
                    origin: self.id.clone(),
                });
                out.push(Delta {
                    target: DeltaTarget::Environment,
                    kind: DeltaKind::AdjustStock {
                        resource: res("capital"),
                        amount: q,
                    },
                    conflict_class: ConflictClass::ResourcePool,
                    origin: self.id.clone(),
                });
                out.push(Delta {
                    target: DeltaTarget::Agent(agent),
                    kind: DeltaKind::RemoveAgent {
                        reason: "test".to_owned(),
                    },
                    conflict_class: ConflictClass::Independent,
                    origin: self.id.clone(),
                });
            }
            out
        }
        fn assumption(&self) -> &str {
            "transfers residual stock to the pool then removes the agent"
        }
    }

    let kernel = Kernel::new();
    let mut world = world_with(&[(0, 40), (1, 25)], 100);
    let before = world.live_total(&res("capital"));
    let schedule = Schedule::new(
        vec![Box::new(DrainAndRemove {
            id: PluginId::new("t.drain_and_remove"),
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock, DeltaKindTag::RemoveAgent],
        })],
        Box::new(NoResolver),
    );

    let mut events = Vec::new();
    kernel
        .step(&mut world, &schedule, &mut events)
        .expect("ResourcePool transfers + RemoveAgent must not abort");

    assert!(world.live_agents().is_empty(), "both agents removed");
    assert_eq!(
        world.env_stock(&res("capital")),
        100 + 40 + 25,
        "both agents' stock reached the pool before removal"
    );
    assert_eq!(
        world.live_total(&res("capital")),
        before,
        "conservation held across the transfer + removal phase"
    );
    let deaths = events
        .iter()
        .filter(|e| matches!(e, crate::Event::AgentDied { .. }))
        .count();
    assert_eq!(deaths, 2, "one AgentDied per RemoveAgent");
}

/// ADR 0033 (narrows ADR 0031). A single rule invocation, for **one agent**,
/// emitting two `ResourcePool` `AdjustStock` deltas to *that agent* for the
/// *same resource* in one phase is a duplicate-emission **bug**, not
/// legitimate multi-agent pool contention (`DeltaTarget::Agent` carries the
/// agent id, so two distinct agents never collide on the `(target, disc, slot)`
/// key — only `DeltaTarget::Environment`, whose `sort_key()` is a fixed
/// sentinel, does). The uniqueness guard must still reject it.
#[test]
fn same_agent_duplicate_resourcepool_delta_is_rejected() {
    /// Emits, for agent 0 in one phase, the paired pool transfer `−5` **twice**
    /// (agent↔env), same resource — a deliberate double-debit bug.
    struct DoubleDebit {
        id: PluginId,
        reads: Vec<ComponentId>,
        writes: Vec<DeltaKindTag>,
    }
    impl Rule for DoubleDebit {
        fn id(&self) -> PluginId {
            self.id.clone()
        }
        fn version(&self) -> Version {
            Version::new(1, 0, 0)
        }
        fn phase(&self) -> Phase {
            Phase::ActMarket
        }
        fn reads(&self) -> &[ComponentId] {
            &self.reads
        }
        fn writes(&self) -> &[DeltaKindTag] {
            &self.writes
        }
        fn apply(&self, _v: &dyn View, _k: RngKey) -> Vec<Delta> {
            let pair = || {
                [
                    Delta {
                        target: DeltaTarget::Agent(AgentId(0)),
                        kind: DeltaKind::AdjustStock {
                            resource: res("capital"),
                            amount: -5,
                        },
                        conflict_class: ConflictClass::ResourcePool,
                        origin: self.id.clone(),
                    },
                    Delta {
                        target: DeltaTarget::Environment,
                        kind: DeltaKind::AdjustStock {
                            resource: res("capital"),
                            amount: 5,
                        },
                        conflict_class: ConflictClass::ResourcePool,
                        origin: self.id.clone(),
                    },
                ]
            };
            let mut out = Vec::new();
            out.extend(pair()); // the intended transfer
            out.extend(pair()); // the accidental duplicate
            out
        }
        fn assumption(&self) -> &str {
            "emits the same agent-side ResourcePool debit twice; ADR-0033 fixture"
        }
    }

    let kernel = Kernel::new();
    let mut world = world_with(&[(0, 100)], 0);
    let schedule = Schedule::new(
        vec![Box::new(DoubleDebit {
            id: PluginId::new("t.double_debit"),
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock],
        })],
        Box::new(NoResolver),
    );

    let result = kernel.step(&mut world, &schedule, &mut Vec::new());

    // If the guard let both through, `step` succeeds and agent 0 was debited
    // 10 instead of 5 — the silent double-debit. Report that explicitly.
    assert!(
        result.is_err(),
        "same-agent duplicate ResourcePool delta was NOT rejected: step() = Ok, \
         agent 0 capital is now {} (should be 95 after one −5 transfer; 90 ⇒ \
         both applied silently), env capital {}",
        world.agent_stock(AgentId(0), &res("capital")),
        world.env_stock(&res("capital")),
    );
    assert!(
        matches!(result, Err(crate::KernelError::DuplicateDelta { .. })),
        "expected DuplicateDelta, got {result:?}"
    );
}
