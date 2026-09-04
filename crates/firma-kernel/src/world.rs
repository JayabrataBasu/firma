//! The state store (manual §19.2) and its serialisable [`Snapshot`] form.
//!
//! Component-oriented and keyed by [`AgentId`]. **All agent iteration is
//! ascending by `AgentId`** (§19.2); [`World::live_agents`] returns a slice that
//! is already in that order.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use firma_core::{AgentId, PluginId, ResourceKind, StreamId, Tick};

/// The four run seeds, one per independent stream (manual §21.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSeeds {
    /// Decisions, action outcomes, shaping success.
    pub mechanism: u64,
    /// Resource dynamics, exogenous variation.
    pub environment: u64,
    /// Shock timing and magnitude.
    pub shock: u64,
    /// Initial conditions.
    pub init: u64,
}

impl RunSeeds {
    /// The seed for `stream`.
    #[must_use]
    pub fn for_stream(&self, stream: StreamId) -> u64 {
        match stream {
            StreamId::Mechanism => self.mechanism,
            StreamId::Environment => self.environment,
            StreamId::Shock => self.shock,
            StreamId::Init => self.init,
        }
    }
}

/// One agent's state. Phase 1: birth tick + the integer ledger (ADR 0004).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentState {
    /// Tick the agent was created.
    pub birth_tick: Tick,
    /// Integer stock balances, `BTreeMap` so iteration is deterministic (§19.2).
    pub stocks: BTreeMap<ResourceKind, i64>,
}

impl AgentState {
    /// Balance of `resource` (0 if unheld).
    #[must_use]
    pub fn stock(&self, resource: &ResourceKind) -> i64 {
        self.stocks.get(resource).copied().unwrap_or(0)
    }
}

/// The complete mutable simulation state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
    /// Current tick.
    pub tick: Tick,
    /// The conserved resource kinds, ascending by name.
    pub resources: Vec<ResourceKind>,
    /// Run seeds.
    pub seeds: RunSeeds,
    /// Live agents, keyed and iterated by ascending id.
    agents: BTreeMap<AgentId, AgentState>,
    /// The shared environment resource pool.
    env: BTreeMap<ResourceKind, i64>,
    /// Rules currently frozen by a `freeze_rule` intervention (§6.5): loaded,
    /// in the manifest, but emitting no deltas.
    pub frozen_rules: BTreeSet<PluginId>,
    /// Total of each resource at run start — the conservation reference
    /// (§19.5). Phase 1 has no sources or sinks, so live total MUST always
    /// equal this.
    initial_totals: BTreeMap<ResourceKind, i64>,
    /// The highest agent id ever live, so a removed id is never reissued
    /// (§7.2).
    max_agent_id: Option<u64>,

    // --- Phase 2 Stage 2: opaque domain-state store (ADR 0022) ---
    //
    // The kernel never interprets these string keys — they are as opaque to it
    // as a `ResourceKind`. The meaning lives in `firma-domain::keys` and the
    // plugins. All `#[serde(default)]` so a Phase-1 snapshot round-trips.
    /// Per-agent real scalars (e.g. `"capability"`).
    #[serde(default)]
    agent_reals: BTreeMap<AgentId, BTreeMap<String, f64>>,
    /// Per-agent integer scalars (e.g. `"obligation"`).
    #[serde(default)]
    agent_ints: BTreeMap<AgentId, BTreeMap<String, i64>>,
    /// Global real scalars (e.g. `"theta_limit"`, `"theta_cap"`).
    #[serde(default)]
    global_reals: BTreeMap<String, f64>,
    /// Global integer scalars (e.g. `"theta_q"`).
    #[serde(default)]
    global_ints: BTreeMap<String, i64>,
    /// Per-agent lists of canonical-JSON records (e.g. `"lagged_effects"`).
    #[serde(default)]
    agent_lists: BTreeMap<AgentId, BTreeMap<String, Vec<String>>>,
    /// Global lists of canonical-JSON records (e.g. `"relation_edges"`).
    #[serde(default)]
    global_lists: BTreeMap<String, Vec<String>>,

    /// Cached ascending live-id list backing [`live_agents`](Self::live_agents).
    #[serde(skip)]
    live_cache: Vec<AgentId>,
}

impl World {
    /// Build the initial world. `agents` and `env` give opening balances; every
    /// referenced resource MUST appear in `resources` (the config validator
    /// guarantees this).
    #[must_use]
    pub fn new(
        resources: Vec<ResourceKind>,
        seeds: RunSeeds,
        agents: BTreeMap<AgentId, AgentState>,
        env: BTreeMap<ResourceKind, i64>,
    ) -> World {
        let mut initial_totals: BTreeMap<ResourceKind, i64> = BTreeMap::new();
        for r in &resources {
            initial_totals.insert(r.clone(), 0);
        }
        for st in agents.values() {
            for (r, q) in &st.stocks {
                *initial_totals.entry(r.clone()).or_insert(0) += *q;
            }
        }
        for (r, q) in &env {
            *initial_totals.entry(r.clone()).or_insert(0) += *q;
        }
        let max_agent_id = agents.keys().map(|a| a.0).max();
        let mut w = World {
            tick: Tick(0),
            resources,
            seeds,
            agents,
            env,
            frozen_rules: BTreeSet::new(),
            initial_totals,
            max_agent_id,
            agent_reals: BTreeMap::new(),
            agent_ints: BTreeMap::new(),
            global_reals: BTreeMap::new(),
            global_ints: BTreeMap::new(),
            agent_lists: BTreeMap::new(),
            global_lists: BTreeMap::new(),
            live_cache: Vec::new(),
        };
        w.rebuild_live();
        w
    }

    /// Seed a global real scalar (`ConstraintParams`, prices) before a run —
    /// the orchestrator calls this from config; the reconciler adjusts it via
    /// [`DeltaKind::AdjustGlobalReal`](firma_core::DeltaKind).
    pub fn set_global_real(&mut self, field: impl Into<String>, value: f64) {
        self.global_reals.insert(field.into(), value);
    }

    /// Seed a global integer scalar (`theta_q`) before a run.
    pub fn set_global_int(&mut self, field: impl Into<String>, value: i64) {
        self.global_ints.insert(field.into(), value);
    }

    /// Seed a per-agent real scalar (`capability`, `legitimacy`) before a run.
    pub fn set_agent_real(&mut self, agent: AgentId, field: impl Into<String>, value: f64) {
        self.agent_reals
            .entry(agent)
            .or_default()
            .insert(field.into(), value);
    }

    /// Seed a per-agent integer scalar (`obligation`) before a run.
    pub fn set_agent_int_value(&mut self, agent: AgentId, field: impl Into<String>, value: i64) {
        self.agent_ints
            .entry(agent)
            .or_default()
            .insert(field.into(), value);
    }

    // --- domain-state reads (backing `WorldView`) ---

    /// A per-agent real scalar, or `None`.
    #[must_use]
    pub fn agent_real(&self, agent: AgentId, field: &str) -> Option<f64> {
        self.agent_reals.get(&agent)?.get(field).copied()
    }

    /// A per-agent integer scalar, or `None`.
    #[must_use]
    pub fn agent_int(&self, agent: AgentId, field: &str) -> Option<i64> {
        self.agent_ints.get(&agent)?.get(field).copied()
    }

    /// A global real scalar, or `None`.
    #[must_use]
    pub fn global_real(&self, field: &str) -> Option<f64> {
        self.global_reals.get(field).copied()
    }

    /// A global integer scalar, or `None`.
    #[must_use]
    pub fn global_int(&self, field: &str) -> Option<i64> {
        self.global_ints.get(field).copied()
    }

    /// A per-agent record list (empty if unset).
    #[must_use]
    pub fn agent_records(&self, agent: AgentId, list: &str) -> &[String] {
        self.agent_lists
            .get(&agent)
            .and_then(|m| m.get(list))
            .map_or(&[], Vec::as_slice)
    }

    /// A global record list (empty if unset).
    #[must_use]
    pub fn global_records(&self, list: &str) -> &[String] {
        self.global_lists.get(list).map_or(&[], Vec::as_slice)
    }

    // --- domain-state mutation (reconciler only) ---

    pub(crate) fn adjust_agent_real(
        &mut self,
        agent: AgentId,
        field: &str,
        delta: f64,
    ) -> Option<()> {
        if !self.agents.contains_key(&agent) {
            return None;
        }
        *self
            .agent_reals
            .entry(agent)
            .or_default()
            .entry(field.to_owned())
            .or_insert(0.0) += delta;
        Some(())
    }

    pub(crate) fn adjust_agent_int(
        &mut self,
        agent: AgentId,
        field: &str,
        delta: i64,
    ) -> Option<i64> {
        if !self.agents.contains_key(&agent) {
            return None;
        }
        let e = self
            .agent_ints
            .entry(agent)
            .or_default()
            .entry(field.to_owned())
            .or_insert(0);
        *e += delta;
        Some(*e)
    }

    pub(crate) fn set_agent_int(&mut self, agent: AgentId, field: &str, value: i64) -> Option<()> {
        if !self.agents.contains_key(&agent) {
            return None;
        }
        self.agent_ints
            .entry(agent)
            .or_default()
            .insert(field.to_owned(), value);
        Some(())
    }

    pub(crate) fn adjust_global_real(&mut self, field: &str, delta: f64) {
        *self.global_reals.entry(field.to_owned()).or_insert(0.0) += delta;
    }

    pub(crate) fn adjust_global_int(&mut self, field: &str, delta: i64) {
        *self.global_ints.entry(field.to_owned()).or_insert(0) += delta;
    }

    pub(crate) fn push_agent_record(
        &mut self,
        agent: AgentId,
        list: &str,
        record: String,
    ) -> Option<()> {
        if !self.agents.contains_key(&agent) {
            return None;
        }
        self.agent_lists
            .entry(agent)
            .or_default()
            .entry(list.to_owned())
            .or_default()
            .push(record);
        Some(())
    }

    pub(crate) fn push_global_record(&mut self, list: &str, record: String) {
        self.global_lists
            .entry(list.to_owned())
            .or_default()
            .push(record);
    }

    pub(crate) fn replace_agent_list(
        &mut self,
        agent: AgentId,
        list: &str,
        records: Vec<String>,
    ) -> Option<()> {
        if !self.agents.contains_key(&agent) {
            return None;
        }
        self.agent_lists
            .entry(agent)
            .or_default()
            .insert(list.to_owned(), records);
        Some(())
    }

    pub(crate) fn replace_global_list(&mut self, list: &str, records: Vec<String>) {
        self.global_lists.insert(list.to_owned(), records);
    }

    /// Remove an agent (§9.1 death; ADR 0029) — its `AgentState` **and** its
    /// per-agent keyed subtrees (`agent_reals` / `agent_ints` / `agent_lists`).
    /// Does **not** touch the relation-edge global list (the `enforce` rule
    /// rewrites it with `ReplaceGlobalList`) and does **not** rebase
    /// conservation (the `enforce` rule transfers the agent's stocks to the
    /// environment with paired `AdjustStock` deltas that sort before this).
    /// Returns `true` if the agent was live. Id is retired forever (§7.2).
    pub(crate) fn remove_agent_and_keyed_state(&mut self, agent: AgentId) -> bool {
        let existed = self.agents.remove(&agent).is_some();
        if existed {
            self.agent_reals.remove(&agent);
            self.agent_ints.remove(&agent);
            self.agent_lists.remove(&agent);
            self.rebuild_live();
        }
        existed
    }

    /// Reconstruct the transient live-id cache — call after deserialising a
    /// snapshot or mutating the population.
    pub fn rebuild_live(&mut self) {
        self.live_cache = self.agents.keys().copied().collect();
    }

    /// Live agent ids, ascending (§19.2).
    #[must_use]
    pub fn live_agents(&self) -> &[AgentId] {
        &self.live_cache
    }

    /// Whether `agent` is live.
    #[must_use]
    pub fn is_live(&self, agent: AgentId) -> bool {
        self.agents.contains_key(&agent)
    }

    /// Read an agent's stock (0 if the agent is absent).
    #[must_use]
    pub fn agent_stock(&self, agent: AgentId, resource: &ResourceKind) -> i64 {
        self.agents.get(&agent).map_or(0, |s| s.stock(resource))
    }

    /// Read the environment pool's balance of `resource`.
    #[must_use]
    pub fn env_stock(&self, resource: &ResourceKind) -> i64 {
        self.env.get(resource).copied().unwrap_or(0)
    }

    /// Borrow an agent's state.
    #[must_use]
    pub fn agent(&self, agent: AgentId) -> Option<&AgentState> {
        self.agents.get(&agent)
    }

    /// Iterate `(id, state)` in ascending id order.
    pub fn agents_iter(&self) -> impl Iterator<Item = (&AgentId, &AgentState)> {
        self.agents.iter()
    }

    // --- mutation: only the reconciler / intervention path calls these ---

    /// Add `amount` (may be negative) to a live agent's stock. Returns the new
    /// balance, or `None` if the agent is not live.
    pub(crate) fn adjust_agent_stock(
        &mut self,
        agent: AgentId,
        resource: &ResourceKind,
        amount: i64,
    ) -> Option<i64> {
        let st = self.agents.get_mut(&agent)?;
        let e = st.stocks.entry(resource.clone()).or_insert(0);
        *e += amount;
        Some(*e)
    }

    /// Add `amount` to the environment pool. Returns the new balance.
    pub(crate) fn adjust_env_stock(&mut self, resource: &ResourceKind, amount: i64) -> i64 {
        let e = self.env.entry(resource.clone()).or_insert(0);
        *e += amount;
        *e
    }

    /// Remove an agent (id retired forever). Returns `true` if it was live.
    pub(crate) fn remove_agent(&mut self, agent: AgentId) -> bool {
        let existed = self.agents.remove(&agent).is_some();
        if existed {
            self.rebuild_live();
        }
        existed
    }

    /// Force an agent's stock to an absolute value (`set_state`, §6.5).
    pub(crate) fn set_agent_stock(
        &mut self,
        agent: AgentId,
        resource: &ResourceKind,
        value: i64,
    ) -> Option<()> {
        let st = self.agents.get_mut(&agent)?;
        st.stocks.insert(resource.clone(), value);
        Some(())
    }

    /// The conservation reference total for `resource`.
    #[must_use]
    pub fn initial_total(&self, resource: &ResourceKind) -> i64 {
        self.initial_totals.get(resource).copied().unwrap_or(0)
    }

    /// The current live total of `resource` (all agents + environment).
    #[must_use]
    pub fn live_total(&self, resource: &ResourceKind) -> i64 {
        let agents: i64 = self.agents.values().map(|s| s.stock(resource)).sum();
        agents + self.env_stock(resource)
    }

    /// Adjust the conservation reference when the population is manipulated by
    /// intervention (`remove_agent` / `set_state` move resources in or out of
    /// the modelled system — a deliberate, logged act, §6.5). Without this the
    /// invariant would (correctly) abort after such an intervention.
    pub(crate) fn rebase_conservation(&mut self) {
        for r in self.resources.clone() {
            let total = self.live_total(&r);
            self.initial_totals.insert(r, total);
        }
    }

    /// Highest id ever assigned (for id-uniqueness checks).
    #[must_use]
    pub fn max_agent_id(&self) -> Option<u64> {
        self.max_agent_id
    }
}

/// A serialised full-state checkpoint (manual §22.1). Phase 1: a clone of
/// [`World`]. Round-trips exactly (DT-5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// The captured world.
    pub world: World,
}
