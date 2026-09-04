//! Test support: an in-memory [`View`] for exercising a `Rule` in isolation.
//!
//! Public (not `#[cfg(test)]`) so the cross-crate conformance suite (VT-4 /
//! VT-5) can use the same fixture the crate's own tests do. It is a plain
//! key-value view — no domain logic — mirroring the kernel's opaque store
//! (ADR 0022). The `View` trait's default impls (`None` / `&[]`) mean only the
//! fields a test sets need to be populated.

use std::collections::BTreeMap;

use firma_core::{AgentId, Phase, ResourceKind, RngKey, StreamId, Tick, View};

/// A mutable in-memory [`View`]. Build with [`MockView::new`], then use the
/// `with_*` setters.
#[derive(Debug, Clone)]
pub struct MockView {
    tick: u64,
    phase: Phase,
    run_seed: u64,
    live: Vec<AgentId>,
    stocks: BTreeMap<(u64, String), i64>,
    env_stocks: BTreeMap<String, i64>,
    resources: Vec<ResourceKind>,
    agent_reals: BTreeMap<(u64, String), f64>,
    agent_ints: BTreeMap<(u64, String), i64>,
    global_reals: BTreeMap<String, f64>,
    global_ints: BTreeMap<String, i64>,
    agent_lists: BTreeMap<(u64, String), Vec<String>>,
    global_lists: BTreeMap<String, Vec<String>>,
}

impl MockView {
    /// A view at `tick` in `phase` with run seed `run_seed` and the given live
    /// agents (sorted ascending, as the kernel guarantees — §19.2).
    #[must_use]
    pub fn new(tick: u64, phase: Phase, run_seed: u64, live: &[u64]) -> MockView {
        let mut live: Vec<AgentId> = live.iter().copied().map(AgentId).collect();
        live.sort_unstable();
        MockView {
            tick,
            phase,
            run_seed,
            live,
            stocks: BTreeMap::new(),
            env_stocks: BTreeMap::new(),
            resources: vec![
                ResourceKind("capital".to_owned()),
                ResourceKind("input".to_owned()),
            ],
            agent_reals: BTreeMap::new(),
            agent_ints: BTreeMap::new(),
            global_reals: BTreeMap::new(),
            global_ints: BTreeMap::new(),
            agent_lists: BTreeMap::new(),
            global_lists: BTreeMap::new(),
        }
    }

    /// Set an agent's stock of `resource`.
    #[must_use]
    pub fn with_stock(mut self, agent: u64, resource: &str, amount: i64) -> Self {
        self.stocks.insert((agent, resource.to_owned()), amount);
        self
    }
    /// Set the env pool's stock of `resource`.
    #[must_use]
    pub fn with_env_stock(mut self, resource: &str, amount: i64) -> Self {
        self.env_stocks.insert(resource.to_owned(), amount);
        self
    }
    /// Set a per-agent real scalar.
    #[must_use]
    pub fn with_agent_real(mut self, agent: u64, field: &str, v: f64) -> Self {
        self.agent_reals.insert((agent, field.to_owned()), v);
        self
    }
    /// Set a per-agent integer scalar.
    #[must_use]
    pub fn with_agent_int(mut self, agent: u64, field: &str, v: i64) -> Self {
        self.agent_ints.insert((agent, field.to_owned()), v);
        self
    }
    /// Set a global real scalar.
    #[must_use]
    pub fn with_global_real(mut self, field: &str, v: f64) -> Self {
        self.global_reals.insert(field.to_owned(), v);
        self
    }
    /// Set a global integer scalar.
    #[must_use]
    pub fn with_global_int(mut self, field: &str, v: i64) -> Self {
        self.global_ints.insert(field.to_owned(), v);
        self
    }
    /// Append a record to a per-agent list.
    #[must_use]
    pub fn with_agent_record(mut self, agent: u64, list: &str, record: String) -> Self {
        self.agent_lists
            .entry((agent, list.to_owned()))
            .or_default()
            .push(record);
        self
    }
    /// Append a record to a global list.
    #[must_use]
    pub fn with_global_record(mut self, list: &str, record: String) -> Self {
        self.global_lists
            .entry(list.to_owned())
            .or_default()
            .push(record);
        self
    }

    /// Build an [`RngKey`] as the kernel would hand one to a rule in this
    /// view's phase (`Mechanism` stream, `agent_id: None`, plugin id 0).
    #[must_use]
    pub fn rng_key(&self) -> RngKey {
        RngKey {
            run_seed: self.run_seed,
            stream: StreamId::Mechanism,
            plugin_id: 0,
            phase: self.phase,
            tick: self.tick,
            agent_id: None,
        }
    }
}

impl View for MockView {
    fn tick(&self) -> Tick {
        Tick(self.tick)
    }
    fn phase(&self) -> Phase {
        self.phase
    }
    fn run_seed(&self, _stream: StreamId) -> u64 {
        self.run_seed
    }
    fn live_agents(&self) -> &[AgentId] {
        &self.live
    }
    fn is_live(&self, agent: AgentId) -> bool {
        self.live.binary_search(&agent).is_ok()
    }
    fn agent_stock(&self, agent: AgentId, resource: &ResourceKind) -> i64 {
        self.stocks
            .get(&(agent.0, resource.0.clone()))
            .copied()
            .unwrap_or(0)
    }
    fn env_stock(&self, resource: &ResourceKind) -> i64 {
        self.env_stocks.get(&resource.0).copied().unwrap_or(0)
    }
    fn resource_kinds(&self) -> &[ResourceKind] {
        &self.resources
    }
    fn agent_real(&self, agent: AgentId, field: &str) -> Option<f64> {
        self.agent_reals.get(&(agent.0, field.to_owned())).copied()
    }
    fn agent_int(&self, agent: AgentId, field: &str) -> Option<i64> {
        self.agent_ints.get(&(agent.0, field.to_owned())).copied()
    }
    fn global_real(&self, field: &str) -> Option<f64> {
        self.global_reals.get(field).copied()
    }
    fn global_int(&self, field: &str) -> Option<i64> {
        self.global_ints.get(field).copied()
    }
    fn agent_records(&self, agent: AgentId, list: &str) -> &[String] {
        self.agent_lists
            .get(&(agent.0, list.to_owned()))
            .map_or(&[], Vec::as_slice)
    }
    fn global_records(&self, list: &str) -> &[String] {
        self.global_lists.get(list).map_or(&[], Vec::as_slice)
    }
}
