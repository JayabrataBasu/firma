//! `firma-kernel` — the deterministic substrate (manual §19).
//!
//! Scope (§19.1), and nothing else: a **state store** ([`World`]), a **phase
//! scheduler** (the fixed nine phases of §10.1), a **delta reconciler**
//! (§19.4), an **invariant enforcer** (§19.5), an **RNG key derivator**
//! (§21.2, via `firma-rng`), and an **event writer** (it fills `Vec<Event>`;
//! `firma-cli` persists them).
//!
//! **No domain logic** (§17 A1). There is no `Firm`, no constraint, no action,
//! no theory here. The kernel moves integers between ledgers when a [`Rule`]
//! proposes it and the reconciler allows it — that is the whole job (§26.3:
//! "The kernel is finished when it can move integers around deterministically").
//!
//! Determinism is structural (§17 A3):
//! * agents live in a `BTreeMap` and are always iterated ascending by id (DT-2);
//! * every delta is sorted by the §19.4 total order before it is applied, so
//!   rule execution order — sequential or multi-threaded — cannot change the
//!   result (DT-3);
//! * all randomness is derived from the keyed counter-based RNG (DT-6);
//! * a fork under a null intervention is a bit-identical clone (DT-4);
//! * snapshot → restore → continue equals an uninterrupted run (DT-5).

#![forbid(unsafe_code)]

mod world;

/// The kernel's event-log record type. Re-exported from `firma-core` (ADR
/// 0045): `Event` depends only on other `firma-core` wire types, so it moved
/// there to let kernel-free analysis crates (`firma-analysis`, `firma-tui` —
/// §23.2's "MUST NOT link against the kernel") read and construct it without
/// pulling in this crate. Every existing `firma_kernel::Event` call site is
/// unaffected.
pub use firma_core::Event;
pub use world::{AgentState, RunSeeds, Snapshot, World};

use std::collections::BTreeSet;
use std::fmt;

use firma_core::{
    ConflictClass, ConflictResolver, Delta, DeltaKind, DeltaTarget, Intervention, Phase,
    ResourceKind, RngKey, Rule, StreamId, Tick, View, PHASE_ORDER,
};

/// What went wrong in the kernel. Every variant is fatal: there is no
/// repair-and-continue path (§19.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// An invariant of §19.5 was violated. Carries a diagnostic.
    InvariantViolation(String),
    /// A rule emitted a [`DeltaKind`] it did not declare in `writes()` (§20.2).
    UndeclaredDeltaKind {
        /// The offending plugin.
        plugin: String,
        /// The kind it emitted without declaring.
        kind: String,
    },
    /// A rule emitted two deltas for the same `(target, kind)` in one phase
    /// (§19.5 "Delta uniqueness").
    DuplicateDelta {
        /// The offending plugin.
        plugin: String,
    },
    /// A rule emitted an `AdjustAgentReal` / `AdjustGlobalReal` whose `f64`
    /// payload is not finite (`NaN` / `±∞`). Rejected at the boundary so the
    /// manual `Eq` impl on [`Delta`](firma_core::Delta) rests on an enforced
    /// invariant, not an assumption (§21.4; ADR 0022 Decision 2).
    NonFiniteDelta {
        /// The offending plugin.
        plugin: String,
        /// The domain field key it tried to write.
        field: String,
    },
    /// A delta or intervention referenced an agent that is not live.
    UnknownAgent(u64),
    /// A `ResourcePool` delta was proposed but no registered resolver handles
    /// that class.
    NoResolverForClass,
    /// This intervention changes the rule set and must be applied by the
    /// orchestrator (via `firma-registry`), not the kernel (§18.1 forbids the
    /// kernel depending on the registry). See `PROGRESS.md`.
    InterventionNotKernelScoped(String),
    /// A delta targeted `Global`, which carries no stock in Phase 1.
    UnsupportedTarget,
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::InvariantViolation(m) => write!(f, "invariant violation: {m}"),
            KernelError::UndeclaredDeltaKind { plugin, kind } => {
                write!(f, "plugin {plugin:?} emitted undeclared delta kind {kind}")
            }
            KernelError::DuplicateDelta { plugin } => {
                write!(
                    f,
                    "plugin {plugin:?} emitted two deltas for one (target, kind) in a phase"
                )
            }
            KernelError::NonFiniteDelta { plugin, field } => {
                write!(
                    f,
                    "plugin {plugin:?} emitted a non-finite (NaN/inf) real delta for field {field:?}"
                )
            }
            KernelError::UnknownAgent(a) => write!(f, "unknown/dead agent {a}"),
            KernelError::NoResolverForClass => {
                write!(
                    f,
                    "a ResourcePool delta was proposed but no resolver handles that class"
                )
            }
            KernelError::InterventionNotKernelScoped(op) => {
                write!(
                    f,
                    "intervention {op} changes the rule set; apply it via the orchestrator"
                )
            }
            KernelError::UnsupportedTarget => {
                write!(f, "delta target Global is unsupported in Phase 1")
            }
        }
    }
}

impl std::error::Error for KernelError {}

/// The active rule set and conflict resolver for a run (manual §6.4 `R_p`).
///
/// Interventions that change *which rules exist* (`add_rule`, `remove_rule`) are
/// modelled as building a different `Schedule`; the kernel itself only mutates
/// state (§18.1).
pub struct Schedule {
    rules: Vec<Box<dyn Rule>>,
    resolver: Box<dyn ConflictResolver>,
}

impl Schedule {
    /// Build a schedule from resolved plugins.
    #[must_use]
    pub fn new(rules: Vec<Box<dyn Rule>>, resolver: Box<dyn ConflictResolver>) -> Schedule {
        Schedule { rules, resolver }
    }

    /// The rules, in registration order.
    #[must_use]
    pub fn rules(&self) -> &[Box<dyn Rule>] {
        &self.rules
    }

    /// The conflict resolver.
    #[must_use]
    pub fn resolver(&self) -> &dyn ConflictResolver {
        self.resolver.as_ref()
    }

    /// Decompose into owned parts, so the orchestrator can apply an
    /// `add_rule` / `remove_rule` intervention (§6.5) by rebuilding.
    #[must_use]
    pub fn into_parts(self) -> (Vec<Box<dyn Rule>>, Box<dyn ConflictResolver>) {
        (self.rules, self.resolver)
    }
}

/// Per-tick execution summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickReport {
    /// The tick that just executed.
    pub tick: u64,
    /// Total resolved deltas applied across all nine phases.
    pub deltas_applied: u32,
    /// Live agents at tick end.
    pub live_agents: u32,
}

/// How rules within a phase are executed. Both modes MUST produce identical
/// output — that is DT-3 (§25.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecMode {
    /// One thread, rules in registration order.
    Sequential,
    /// One scoped thread per rule; results merged and sorted afterwards.
    Parallel,
}

/// The engine. Holds no mutable state — every method takes the [`World`] it acts
/// on.
#[derive(Debug, Clone, Copy)]
pub struct Kernel {
    mode: ExecMode,
}

impl Default for Kernel {
    fn default() -> Self {
        Kernel {
            mode: ExecMode::Sequential,
        }
    }
}

impl Kernel {
    /// A kernel that executes phases sequentially.
    #[must_use]
    pub fn new() -> Kernel {
        Kernel::default()
    }

    /// A kernel that executes each phase's rules on scoped threads (§21.5:
    /// parallelism is permitted only within a phase).
    #[must_use]
    pub fn parallel() -> Kernel {
        Kernel {
            mode: ExecMode::Parallel,
        }
    }

    /// This kernel's execution mode.
    #[must_use]
    pub fn mode(&self) -> ExecMode {
        self.mode
    }

    /// Advance the world by one tick: run all nine phases in order, reconciling
    /// and enforcing invariants after each (manual §10.1, §19.3–19.5). Appends
    /// events to `events`.
    ///
    /// # Errors
    /// [`KernelError`] on any invariant violation or plugin contract breach.
    /// The world may be left partially advanced; callers MUST treat the run as
    /// aborted (§19.5).
    ///
    /// # Panics
    /// Never in normal operation. (Internal `expect`s guard thread joins.)
    pub fn step(
        &self,
        world: &mut World,
        schedule: &Schedule,
        events: &mut Vec<Event>,
    ) -> Result<TickReport, KernelError> {
        let tick = world.tick.0;
        events.push(Event::TickStarted { tick });

        // Births declared for this tick (Phase 1: only the initial population at
        // tick 0; no birth mechanism exists).
        for (id, st) in world.agents_iter() {
            if st.birth_tick.0 == tick {
                events.push(Event::AgentBorn { tick, agent: *id });
            }
        }

        let mut total_deltas = 0u32;
        for phase in PHASE_ORDER {
            let applied = self.run_phase(world, schedule, phase, tick, events)?;
            total_deltas += applied;
            events.push(Event::PhaseCompleted {
                tick,
                phase: phase.as_byte(),
                deltas_applied: applied,
            });
            self.check_invariants(world).map_err(|m| {
                KernelError::InvariantViolation(format!("after phase {}: {m}", phase.as_byte()))
            })?;
        }

        world.tick = Tick(tick + 1); // tick monotonicity (§19.5)
        let live = u32::try_from(world.live_agents().len()).unwrap_or(u32::MAX);
        events.push(Event::TickCompleted {
            tick,
            live_agents: live,
        });
        Ok(TickReport {
            tick,
            deltas_applied: total_deltas,
            live_agents: live,
        })
    }

    /// Run one phase: collect proposed deltas, sort, resolve, apply.
    fn run_phase(
        &self,
        world: &mut World,
        schedule: &Schedule,
        phase: Phase,
        tick: u64,
        events: &mut Vec<Event>,
    ) -> Result<u32, KernelError> {
        let active: Vec<&dyn Rule> = schedule
            .rules
            .iter()
            .map(AsRef::as_ref)
            .filter(|r| r.phase() == phase && !world.frozen_rules.contains(&r.id()))
            .collect();

        // --- collect (Jacobi: every rule reads pre-phase state — we do not
        //     mutate `world` until all deltas are gathered) ---
        let view = WorldView { world, phase };
        let per_rule: Vec<Vec<Delta>> = match self.mode {
            ExecMode::Sequential => active
                .iter()
                .map(|&r| {
                    let key = rng_key(world, rng_stream_for(r, phase), r, phase, tick);
                    r.apply(&view, key)
                })
                .collect(),
            ExecMode::Parallel => std::thread::scope(|scope| {
                let handles: Vec<_> = active
                    .iter()
                    .map(|&r| {
                        let key = rng_key(world, rng_stream_for(r, phase), r, phase, tick);
                        let view_ref = &view;
                        scope.spawn(move || r.apply(view_ref, key))
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|h| h.join().expect("rule panicked on a worker thread"))
                    .collect()
            }),
        };

        let mut proposed: Vec<Delta> = Vec::new();
        for (rule, deltas) in active.iter().zip(per_rule) {
            let declared: BTreeSet<_> = rule.writes().iter().copied().collect();
            // §19.5 delta uniqueness, widened to include `slot` (ADR 0022
            // Decision 3): two `AdjustStock`s to one agent for different
            // resources are allowed; append kinds are exempt entirely.
            // `ResourcePool` deltas that target the shared `Environment` pool
            // are also exempt (ADR 0031, narrowed by ADR 0033): one rule
            // distributing a paired transfer across N agents legitimately
            // proposes N deltas against the pool cell — whose `sort_key()` is a
            // fixed sentinel with no agent in it, so the N claims are otherwise
            // indistinguishable — and the §19.4 step-4 resolver is precisely
            // what aggregates them. Determinism is preserved because the rule
            // emits them in ascending-`AgentId` order and the step-2 sort is
            // stable over `Delta`'s (payload-blind) `Ord`. The exemption is
            // *only* for the `Environment` target: a `ResourcePool` delta to a
            // specific `Agent` carries that agent's id in its `sort_key()`, so
            // two of them for one agent + resource in one phase is a
            // duplicate-emission bug (silent double-debit), not contention —
            // the guard must still catch it.
            let mut seen: BTreeSet<((u8, u64), u16, String)> = BTreeSet::new();
            for d in deltas {
                if !declared.contains(&d.kind.tag()) {
                    return Err(KernelError::UndeclaredDeltaKind {
                        plugin: rule.id().0.clone(),
                        kind: format!("{:?}", d.kind.tag()),
                    });
                }
                // §21.4 / ADR 0022 Decision 2: real-valued deltas must be
                // finite. This is what makes `Delta`'s manual `Eq` sound — the
                // "never NaN" property is enforced here, not merely assumed.
                match &d.kind {
                    DeltaKind::AdjustAgentReal { field, delta }
                    | DeltaKind::AdjustGlobalReal { field, delta }
                        if !delta.is_finite() =>
                    {
                        return Err(KernelError::NonFiniteDelta {
                            plugin: rule.id().0.clone(),
                            field: field.clone(),
                        });
                    }
                    _ => {}
                }
                let pooled_env_claim = d.conflict_class == ConflictClass::ResourcePool
                    && matches!(d.target, DeltaTarget::Environment);
                if !d.kind.allows_repeat() && !pooled_env_claim {
                    let key = (
                        d.target.sort_key(),
                        d.kind.discriminant(),
                        d.kind.slot().to_owned(),
                    );
                    if !seen.insert(key) {
                        return Err(KernelError::DuplicateDelta {
                            plugin: rule.id().0.clone(),
                        });
                    }
                }
                proposed.push(d);
            }
        }

        // --- sort (§19.4 step 2) ---
        proposed.sort();

        // --- group by conflict_class, resolve (§19.4 steps 3–4) ---
        let mut resolved: Vec<Delta> = Vec::with_capacity(proposed.len());
        for (class, group) in group_by_class(&proposed) {
            match class {
                ConflictClass::Independent => resolved.extend(group.iter().cloned()),
                ConflictClass::ResourcePool => {
                    if schedule.resolver.handles() != ConflictClass::ResourcePool {
                        return Err(KernelError::NoResolverForClass);
                    }
                    resolved.extend(schedule.resolver.resolve(&group));
                }
            }
        }
        resolved.sort(); // deterministic application order regardless of resolver

        // --- apply (§19.4 step 5) ---
        // `RemoveAgent` is applied in a final sub-pass, after every value delta,
        // regardless of its position in the sort (ADR 0032, superseding ADR
        // 0029 Decision 2's ordering claim): removing an agent invalidates the
        // target of every other delta that references it — in particular the
        // paired `AdjustStock` transfers that move a dead firm's stock to the
        // environment, which are `ResourcePool`-class and therefore sort *after*
        // the `Independent`-class `RemoveAgent` for the same agent. Both
        // sub-passes keep the sorted order, so application stays deterministic.
        let mut applied = 0u32;
        let is_remove = |d: &&Delta| matches!(d.kind, DeltaKind::RemoveAgent { .. });
        for d in resolved
            .iter()
            .filter(|d| !is_remove(d))
            .chain(resolved.iter().filter(is_remove))
        {
            self.apply_delta(world, d)?;
            events.push(Event::DeltaApplied {
                tick,
                phase: phase.as_byte(),
                origin: d.origin.clone(),
                target: d.target.clone(),
                kind: d.kind.clone(),
            });
            // A `RemoveAgent` delta is an agent-lifecycle event — emit the
            // death record, symmetric with the `AgentBorn` in `step()` (ADR
            // 0029). The kernel reads only `reason`, as an opaque log string.
            if let (DeltaTarget::Agent(a), DeltaKind::RemoveAgent { reason }) = (&d.target, &d.kind)
            {
                events.push(Event::AgentDied {
                    tick,
                    agent: *a,
                    cause: reason.clone(),
                });
            }
            applied += 1;
        }
        Ok(applied)
    }

    fn apply_delta(&self, world: &mut World, d: &Delta) -> Result<(), KernelError> {
        let agent_of = |t: &DeltaTarget| match t {
            DeltaTarget::Agent(a) => Ok(*a),
            _ => Err(KernelError::UnsupportedTarget),
        };
        match &d.kind {
            DeltaKind::AdjustStock { resource, amount } => {
                let new_balance = match &d.target {
                    DeltaTarget::Agent(a) => world
                        .adjust_agent_stock(*a, resource, *amount)
                        .ok_or(KernelError::UnknownAgent(a.0))?,
                    DeltaTarget::Environment => world.adjust_env_stock(resource, *amount),
                    DeltaTarget::Global => return Err(KernelError::UnsupportedTarget),
                };
                if new_balance < 0 {
                    return Err(KernelError::InvariantViolation(format!(
                        "non-negativity: {} stock of {resource} would be {new_balance}",
                        target_label(&d.target)
                    )));
                }
                Ok(())
            }
            // --- Stage 2 domain-state deltas (ADR 0022). Opaque keyed writes;
            //     the kernel does not interpret the field/list names. ---
            DeltaKind::SetAgentInt { field, value } => {
                let a = agent_of(&d.target)?;
                world
                    .set_agent_int(a, field, *value)
                    .ok_or(KernelError::UnknownAgent(a.0))
            }
            DeltaKind::AdjustAgentReal { field, delta } => {
                let a = agent_of(&d.target)?;
                world
                    .adjust_agent_real(a, field, *delta)
                    .ok_or(KernelError::UnknownAgent(a.0))
            }
            DeltaKind::AdjustAgentInt { field, delta } => {
                let a = agent_of(&d.target)?;
                world
                    .adjust_agent_int(a, field, *delta)
                    .map(|_| ())
                    .ok_or(KernelError::UnknownAgent(a.0))
            }
            DeltaKind::AdjustGlobalReal { field, delta } => {
                if !matches!(d.target, DeltaTarget::Global) {
                    return Err(KernelError::UnsupportedTarget);
                }
                world.adjust_global_real(field, *delta);
                Ok(())
            }
            DeltaKind::AdjustGlobalInt { field, delta } => {
                if !matches!(d.target, DeltaTarget::Global) {
                    return Err(KernelError::UnsupportedTarget);
                }
                world.adjust_global_int(field, *delta);
                Ok(())
            }
            DeltaKind::PushAgentRecord { list, record_json } => {
                let a = agent_of(&d.target)?;
                world
                    .push_agent_record(a, list, record_json.clone())
                    .ok_or(KernelError::UnknownAgent(a.0))
            }
            DeltaKind::PushGlobalRecord { list, record_json } => {
                if !matches!(d.target, DeltaTarget::Global) {
                    return Err(KernelError::UnsupportedTarget);
                }
                world.push_global_record(list, record_json.clone());
                Ok(())
            }
            DeltaKind::ReplaceAgentList { list, records_json } => {
                let a = agent_of(&d.target)?;
                world
                    .replace_agent_list(a, list, records_json.clone())
                    .ok_or(KernelError::UnknownAgent(a.0))
            }
            DeltaKind::RemoveAgent { reason: _ } => {
                // ADR 0029: pure agent removal. Edge cleanup + stock transfer
                // are done by preceding deltas from the same phase; the death
                // event is emitted by `run_phase`.
                let a = agent_of(&d.target)?;
                if world.remove_agent_and_keyed_state(a) {
                    Ok(())
                } else {
                    Err(KernelError::UnknownAgent(a.0))
                }
            }
            DeltaKind::ReplaceGlobalList { list, records_json } => {
                if !matches!(d.target, DeltaTarget::Global) {
                    return Err(KernelError::UnsupportedTarget);
                }
                world.replace_global_list(list, records_json.clone());
                Ok(())
            }
        }
    }

    /// Check every §19.5 invariant that applies in Phase 1.
    fn check_invariants(&self, world: &World) -> Result<(), String> {
        // Resource conservation — exact (integers, §21.4).
        for r in &world.resources {
            let live = world.live_total(r);
            let expected = world.initial_total(r);
            if live != expected {
                return Err(format!(
                    "conservation: {r} total is {live}, expected {expected}"
                ));
            }
        }
        // Non-negativity (a second sweep; `apply_delta` also guards inline).
        for (id, st) in world.agents_iter() {
            for (r, q) in &st.stocks {
                if *q < 0 {
                    return Err(format!("non-negativity: {id} stock of {r} is {q}"));
                }
            }
        }
        for r in &world.resources {
            if world.env_stock(r) < 0 {
                return Err(format!(
                    "non-negativity: environment stock of {r} is negative"
                ));
            }
        }
        // ID uniqueness: the `BTreeMap` key set is unique by construction; also
        // no live id may exceed the high-water mark (no reissue, §7.2).
        if let Some(max) = world.max_agent_id() {
            if let Some(top) = world.live_agents().last() {
                if top.0 > max {
                    return Err(format!(
                        "id uniqueness: live id {} exceeds high-water {max}",
                        top.0
                    ));
                }
            }
        }
        // Ledger balance (§19.5): in Phase 1 an agent's ledger *is* its stock
        // map — there is no separate transaction log — so this holds by
        // construction. Phase 2 adds the log and a real check here.
        Ok(())
    }

    /// A full-state checkpoint (manual §18.2).
    #[must_use]
    pub fn snapshot(&self, world: &World) -> Snapshot {
        Snapshot {
            world: world.clone(),
        }
    }

    /// Restore a world from a checkpoint (manual §18.2). Rebuilds the transient
    /// live-id cache so a restored run is byte-identical to an uninterrupted one
    /// (DT-5).
    #[must_use]
    pub fn restore(&self, snap: &Snapshot) -> World {
        let mut w = snap.world.clone();
        w.rebuild_live();
        w
    }

    /// Fork a world from a checkpoint and apply one intervention (manual §6.5,
    /// §18.2). A `Null` intervention yields a bit-identical clone (DT-4).
    ///
    /// # Errors
    /// [`KernelError`] if the intervention is not kernel-scoped or references a
    /// missing agent.
    pub fn fork(&self, snap: &Snapshot, iv: &Intervention) -> Result<World, KernelError> {
        let mut w = self.restore(snap);
        self.apply_intervention(&mut w, iv)?;
        Ok(w)
    }

    /// Apply a state-scoped intervention in place (manual §6.5). Rule-set
    /// interventions (`add_rule`, `remove_rule`) return
    /// [`KernelError::InterventionNotKernelScoped`] — the orchestrator handles
    /// those by rebuilding the [`Schedule`].
    ///
    /// # Errors
    /// [`KernelError::UnknownAgent`] for a missing target;
    /// [`KernelError::InterventionNotKernelScoped`] for `add_rule`/`remove_rule`.
    pub fn apply_intervention(
        &self,
        world: &mut World,
        iv: &Intervention,
    ) -> Result<(), KernelError> {
        match iv {
            Intervention::Null => Ok(()),
            Intervention::SetStock {
                agent,
                resource,
                value,
            } => {
                world
                    .set_agent_stock(firma_core::AgentId(agent.0), resource, *value)
                    .ok_or(KernelError::UnknownAgent(agent.0))?;
                world.rebase_conservation();
                Ok(())
            }
            Intervention::RemoveAgent(a) => {
                if world.remove_agent(firma_core::AgentId(a.0)) {
                    world.rebase_conservation();
                    Ok(())
                } else {
                    Err(KernelError::UnknownAgent(a.0))
                }
            }
            Intervention::FreezeRule(id) => {
                world.frozen_rules.insert(id.clone());
                Ok(())
            }
            Intervention::AddRule(spec) => Err(KernelError::InterventionNotKernelScoped(format!(
                "add_rule({})",
                spec.id
            ))),
            Intervention::RemoveRule(id) => Err(KernelError::InterventionNotKernelScoped(format!(
                "remove_rule({id})"
            ))),
        }
    }
}

/// The RNG stream a phase draws from **by default** (manual §21.3): the
/// environment phase uses the `environment` stream; every other phase uses
/// `mechanism`.
fn stream_for_phase(phase: Phase) -> StreamId {
    match phase {
        Phase::Environment => StreamId::Environment,
        _ => StreamId::Mechanism,
    }
}

/// The RNG stream a specific rule draws from (ADR 0034): the rule's own
/// [`Rule::rng_stream`] if it declares one, else [`stream_for_phase`]. This is
/// how `shock.stochastic` reaches the `shock` stream from phase 1 while
/// `resource.patchy` in the same phase keeps `environment`.
fn rng_stream_for(rule: &dyn Rule, phase: Phase) -> StreamId {
    rule.rng_stream().unwrap_or_else(|| stream_for_phase(phase))
}

fn rng_key(world: &World, stream: StreamId, rule: &dyn Rule, phase: Phase, tick: u64) -> RngKey {
    RngKey {
        run_seed: world.seeds.for_stream(stream),
        stream,
        plugin_id: rule.id().numeric(),
        phase,
        tick,
        agent_id: None, // phase scope; a rule specialises per-agent via firma_rng::open_for
    }
}

fn target_label(t: &DeltaTarget) -> String {
    match t {
        DeltaTarget::Agent(a) => a.to_string(),
        DeltaTarget::Environment => "environment".to_string(),
        DeltaTarget::Global => "global".to_string(),
    }
}

/// Group a §19.4-sorted delta slice by `conflict_class`, preserving order.
fn group_by_class(sorted: &[Delta]) -> Vec<(ConflictClass, Vec<Delta>)> {
    let mut out: Vec<(ConflictClass, Vec<Delta>)> = Vec::new();
    for d in sorted {
        match out.last_mut() {
            Some((c, v)) if *c == d.conflict_class => v.push(d.clone()),
            _ => out.push((d.conflict_class, vec![d.clone()])),
        }
    }
    out
}

#[cfg(test)]
mod tests;

/// Read-only [`View`] over a [`World`] at a phase boundary (Jacobi, §19.3).
struct WorldView<'a> {
    world: &'a World,
    phase: Phase,
}

impl View for WorldView<'_> {
    fn tick(&self) -> Tick {
        self.world.tick
    }
    fn phase(&self) -> Phase {
        self.phase
    }
    fn run_seed(&self, stream: StreamId) -> u64 {
        self.world.seeds.for_stream(stream)
    }
    fn live_agents(&self) -> &[firma_core::AgentId] {
        self.world.live_agents()
    }
    fn is_live(&self, agent: firma_core::AgentId) -> bool {
        self.world.is_live(agent)
    }
    fn agent_stock(&self, agent: firma_core::AgentId, resource: &ResourceKind) -> i64 {
        self.world.agent_stock(agent, resource)
    }
    fn env_stock(&self, resource: &ResourceKind) -> i64 {
        self.world.env_stock(resource)
    }
    fn resource_kinds(&self) -> &[ResourceKind] {
        &self.world.resources
    }

    // --- Phase 2 Stage 2: opaque domain-state reads (ADR 0022). Straight
    //     delegation to `World`; the kernel does not interpret the keys. ---
    fn agent_real(&self, agent: firma_core::AgentId, field: &str) -> Option<f64> {
        self.world.agent_real(agent, field)
    }
    fn agent_int(&self, agent: firma_core::AgentId, field: &str) -> Option<i64> {
        self.world.agent_int(agent, field)
    }
    fn global_real(&self, field: &str) -> Option<f64> {
        self.world.global_real(field)
    }
    fn global_int(&self, field: &str) -> Option<i64> {
        self.world.global_int(field)
    }
    fn agent_records(&self, agent: firma_core::AgentId, list: &str) -> &[String] {
        self.world.agent_records(agent, list)
    }
    fn global_records(&self, list: &str) -> &[String] {
        self.world.global_records(list)
    }
}
