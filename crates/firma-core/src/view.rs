//! The read-only [`View`] handed to a rule (manual §6.4, §20.2).

use crate::ids::{AgentId, ResourceKind, Tick};
use crate::rng_key::Phase;

/// A read-only window onto state as of the start of the current phase (Jacobi
/// semantics, manual §10.1 / §19.3). A rule receives `&dyn View` and cannot
/// mutate anything through it — the type is the enforcement of §17 A2.
///
/// Phase 1 exposes only what a "move one integer" rule needs: the tick, the
/// phase, the live-agent list (ascending `AgentId` — §19.2), and integer stock
/// balances. Phase 2 widens this (constraints, environment prices, relations)
/// behind the same trait.
pub trait View {
    /// The current tick.
    fn tick(&self) -> Tick;

    /// The phase currently executing.
    fn phase(&self) -> Phase;

    /// The run seed for a given stream (manual §21.3). Used by the kernel to
    /// build [`RngKey`](crate::RngKey)s; exposed here so a rule can be tested in
    /// isolation.
    fn run_seed(&self, stream: crate::rng_key::StreamId) -> u64;

    /// Live agent ids in **ascending** order (§19.2). The returned slice is
    /// already sorted; callers MUST NOT re-sort into a different order.
    fn live_agents(&self) -> &[AgentId];

    /// Whether `agent` is currently live.
    fn is_live(&self, agent: AgentId) -> bool;

    /// The integer balance of `resource` held by `agent`, or `0` if the agent
    /// holds none / is not live.
    fn agent_stock(&self, agent: AgentId, resource: &ResourceKind) -> i64;

    /// The integer balance of `resource` in the shared environment pool.
    fn env_stock(&self, resource: &ResourceKind) -> i64;

    /// All resource kinds known to this run, ascending by name.
    fn resource_kinds(&self) -> &[ResourceKind];

    // --- Phase 2 Stage 2: opaque domain-state reads (ADR 0022) ---
    //
    // The kernel stores domain state (`capability`, `theta_limit`, Λ, edges, …)
    // as string-keyed opaque values it never interprets. Plugins that depend on
    // `firma-domain` deserialise the JSON records. Default impls return
    // "absent" so a test `View` only overrides what it exercises.

    /// A named per-agent real scalar (e.g. `"capability"`), or `None` if unset.
    fn agent_real(&self, _agent: AgentId, _field: &str) -> Option<f64> {
        None
    }

    /// A named per-agent integer scalar (e.g. `"obligation"`,
    /// `"selected_action"`), or `None` if unset.
    fn agent_int(&self, _agent: AgentId, _field: &str) -> Option<i64> {
        None
    }

    /// A named global real scalar (e.g. `"theta_limit"`), or `None` if unset.
    fn global_real(&self, _field: &str) -> Option<f64> {
        None
    }

    /// A named global integer scalar (e.g. `"theta_q"`), or `None` if unset.
    fn global_int(&self, _field: &str) -> Option<i64> {
        None
    }

    /// A named per-agent list of canonical-JSON records (e.g.
    /// `"lagged_effects"`), in insertion order. Empty if unset.
    fn agent_records(&self, _agent: AgentId, _list: &str) -> &[String] {
        &[]
    }

    /// A named global list of canonical-JSON records (e.g. `"relation_edges"`),
    /// in insertion order. Empty if unset.
    fn global_records(&self, _list: &str) -> &[String] {
        &[]
    }
}
