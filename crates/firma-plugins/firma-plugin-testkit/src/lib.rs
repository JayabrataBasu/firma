//! `firma-plugin-testkit` — the trivial plugins Phase 1 needs to exercise the
//! kernel (manual §26.3: "the trivial test plugin ... a rule that moves one
//! integer. Nothing domain-specific yet").
//!
//! **No domain logic.** Nothing here represents a firm, a constraint, an
//! action, or a theory. Each rule's `assumption()` says so explicitly. These
//! exist only to make the kernel's determinism and conservation guarantees
//! testable (DT-1…DT-6, VT-6).
//!
//! * [`Transfer`] — moves a fixed integer quantity of one resource between two
//!   holders every tick. The "move one integer" rule.
//! * [`KeyedNudge`] — moves a keyed pseudo-random quantity every tick (one
//!   phase-global draw); its only purpose is to test RNG stream isolation (DT-6).
//! * [`KeyedPerAgent`] — every live agent draws its own keyed value via
//!   `open_for` and its ring net is applied; exercises the per-agent RNG key
//!   path through the kernel (manual §21.2 property 1).
//! * [`ForceAdjust`] — an unguarded conserving transfer, used only to force a
//!   deliberate non-negativity abort for event-log atomicity tests (§19.5).
//! * [`SelectAction`] — stands in for the Stage-3 decision procedure: emits
//!   `SetAgentInt { field: keys::SELECTED_ACTION, .. }` deltas from a fixed
//!   `(agent → action index)` map so the action plugins can be exercised before
//!   `decision.satisficing` exists (ADR 0022 Part F). Not a theory — a wire.
//!   (This is the only place the testkit touches `firma-domain`.)
//! * [`AdditiveResolver`] — a neutral [`ConflictResolver`] that applies pooled
//!   claims without rationing.
//!
//! Plugin ids, versions, and the declared content hashes registered with
//! `firma-registry` live in [`catalog`].

#![forbid(unsafe_code)]

pub mod catalog;

use serde::{Deserialize, Serialize};

use firma_core::{
    ComponentId, ConflictClass, ConflictResolver, Delta, DeltaKind, DeltaKindTag, DeltaTarget,
    Phase, PluginId, ResourceKind, RngKey, Rule, View,
};

/// A transfer endpoint: the shared environment pool or a specific agent.
/// Serialises as `"env"` or `{"agent": <id>}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Endpoint {
    /// The environment resource pool.
    Env,
    /// A specific agent.
    Agent(u64),
}

impl Endpoint {
    fn balance(self, view: &dyn View, resource: &ResourceKind) -> i64 {
        match self {
            Endpoint::Env => view.env_stock(resource),
            Endpoint::Agent(a) => view.agent_stock(firma_core::AgentId(a), resource),
        }
    }

    fn target(self) -> DeltaTarget {
        match self {
            Endpoint::Env => DeltaTarget::Environment,
            Endpoint::Agent(a) => DeltaTarget::Agent(firma_core::AgentId(a)),
        }
    }
}

/// Which phase a test rule runs in (defaults to `act_market`). Serialises as the
/// §10.1 phase name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseName {
    /// §10.1 phase 1.
    Environment,
    /// §10.1 phase 2.
    Observe,
    /// §10.1 phase 3.
    Decide,
    /// §10.1 phase 4.
    ActMarket,
    /// §10.1 phase 5.
    ActShaping,
    /// §10.1 phase 6.
    ResolveLagged,
    /// §10.1 phase 7.
    Constrain,
    /// §10.1 phase 8.
    Enforce,
    /// §10.1 phase 9.
    Record,
}

impl From<PhaseName> for Phase {
    fn from(p: PhaseName) -> Phase {
        match p {
            PhaseName::Environment => Phase::Environment,
            PhaseName::Observe => Phase::Observe,
            PhaseName::Decide => Phase::Decide,
            PhaseName::ActMarket => Phase::ActMarket,
            PhaseName::ActShaping => Phase::ActShaping,
            PhaseName::ResolveLagged => Phase::ResolveLagged,
            PhaseName::Constrain => Phase::Constrain,
            PhaseName::Enforce => Phase::Enforce,
            PhaseName::Record => Phase::Record,
        }
    }
}

fn default_phase() -> PhaseName {
    PhaseName::ActMarket
}

fn default_class() -> ClassName {
    ClassName::Independent
}

/// Which [`ConflictClass`] a rule tags its deltas with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassName {
    /// [`ConflictClass::Independent`].
    Independent,
    /// [`ConflictClass::ResourcePool`] — routed through the resolver.
    ResourcePool,
}

impl From<ClassName> for ConflictClass {
    fn from(c: ClassName) -> ConflictClass {
        match c {
            ClassName::Independent => ConflictClass::Independent,
            ClassName::ResourcePool => ConflictClass::ResourcePool,
        }
    }
}

// --------------------------------------------------------------------------
// Transfer
// --------------------------------------------------------------------------

/// Parameters for [`Transfer`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferParams {
    /// Resource to move.
    pub resource: String,
    /// Source holder.
    pub from: Endpoint,
    /// Destination holder.
    pub to: Endpoint,
    /// Units per tick (must be `> 0`).
    pub amount: i64,
    /// Phase to run in.
    #[serde(default = "default_phase")]
    pub phase: PhaseName,
    /// Conflict class for the emitted deltas.
    #[serde(default = "default_class")]
    pub conflict_class: ClassName,
}

/// Moves `amount` units of one resource from `from` to `to` every tick, when the
/// source can afford it. Emits two [`DeltaKind::AdjustStock`] deltas (`-amount`
/// at the source, `+amount` at the destination), so it is conserving by
/// construction (§21.4). This is the "move one integer" rule of §26.3.
pub struct Transfer {
    id: PluginId,
    resource: ResourceKind,
    params: TransferParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Transfer {
    /// Build from parameters.
    ///
    /// # Errors
    /// A message if `amount <= 0` or `from == to`.
    pub fn new(params: TransferParams) -> Result<Transfer, String> {
        if params.amount <= 0 {
            return Err(format!("amount must be > 0, got {}", params.amount));
        }
        if params.from == params.to {
            return Err("from and to must differ".to_string());
        }
        Ok(Transfer {
            id: PluginId::new(catalog::TRANSFER_ID),
            resource: ResourceKind(params.resource.clone()),
            params,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock],
        })
    }
}

impl Rule for Transfer {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        catalog::version()
    }
    fn phase(&self) -> Phase {
        self.params.phase.into()
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        if self.params.from.balance(view, &self.resource) < self.params.amount {
            return Vec::new();
        }
        let class = ConflictClass::from(self.params.conflict_class);
        vec![
            Delta {
                target: self.params.from.target(),
                kind: DeltaKind::AdjustStock {
                    resource: self.resource.clone(),
                    amount: -self.params.amount,
                },
                conflict_class: class,
                origin: self.id.clone(),
            },
            Delta {
                target: self.params.to.target(),
                kind: DeltaKind::AdjustStock {
                    resource: self.resource.clone(),
                    amount: self.params.amount,
                },
                conflict_class: class,
                origin: self.id.clone(),
            },
        ]
    }
    fn assumption(&self) -> &str {
        "A fixed integer quantity of one resource moves between two holders every \
         tick; a Phase 1 test fixture with no theoretical content."
    }
}

// --------------------------------------------------------------------------
// KeyedNudge
// --------------------------------------------------------------------------

/// Parameters for [`KeyedNudge`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyedNudgeParams {
    /// Resource to move.
    pub resource: String,
    /// Source agent.
    pub from: u64,
    /// Destination agent.
    pub to: u64,
    /// The per-tick draw is reduced modulo this (must be `>= 2`).
    pub modulus: i64,
    /// `purpose_tag` for the RNG stream (manual §21.2). Distinct instances MUST
    /// use distinct purposes.
    pub purpose: String,
    /// Phase to run in.
    #[serde(default = "default_phase")]
    pub phase: PhaseName,
}

/// Moves a keyed pseudo-random quantity (`draw % modulus`) of one resource from
/// `from` to `to` every tick. Its sole purpose is DT-6: adding this rule must
/// not perturb any other rule's draws, and its own draw sequence must be
/// identical whether or not other keyed rules are present (§21.2 property 4).
pub struct KeyedNudge {
    id: PluginId,
    resource: ResourceKind,
    params: KeyedNudgeParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl KeyedNudge {
    /// Build from parameters.
    ///
    /// # Errors
    /// A message if `modulus < 2` or `from == to`.
    pub fn new(params: KeyedNudgeParams) -> Result<KeyedNudge, String> {
        if params.modulus < 2 {
            return Err(format!("modulus must be >= 2, got {}", params.modulus));
        }
        if params.from == params.to {
            return Err("from and to must differ".to_string());
        }
        Ok(KeyedNudge {
            id: PluginId::new(catalog::KEYED_NUDGE_ID),
            resource: ResourceKind(params.resource.clone()),
            params,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock],
        })
    }
}

impl Rule for KeyedNudge {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        catalog::version()
    }
    fn phase(&self) -> Phase {
        self.params.phase.into()
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, key: RngKey) -> Vec<Delta> {
        let mut rng = firma_rng::open(&key, &self.params.purpose);
        let draw = rng.next_u64();
        let amount = (draw % u64::try_from(self.params.modulus).unwrap_or(2)) as i64;
        if amount == 0 {
            return Vec::new();
        }
        let from = firma_core::AgentId(self.params.from);
        let to = firma_core::AgentId(self.params.to);
        if view.agent_stock(from, &self.resource) < amount {
            return Vec::new();
        }
        vec![
            Delta {
                target: DeltaTarget::Agent(from),
                kind: DeltaKind::AdjustStock {
                    resource: self.resource.clone(),
                    amount: -amount,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
            Delta {
                target: DeltaTarget::Agent(to),
                kind: DeltaKind::AdjustStock {
                    resource: self.resource.clone(),
                    amount,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
        ]
    }
    fn assumption(&self) -> &str {
        "Each tick a keyed pseudo-random integer quantity of one resource moves \
         between two agents; used only to test RNG stream isolation (DT-6)."
    }
}

// --------------------------------------------------------------------------
// KeyedPerAgent
// --------------------------------------------------------------------------

/// Parameters for [`KeyedPerAgent`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyedPerAgentParams {
    /// Resource to move.
    pub resource: String,
    /// Each per-agent draw is reduced modulo this (must be `>= 2`).
    pub modulus: i64,
    /// `purpose_tag` for the per-agent RNG streams (manual §21.2).
    pub purpose: String,
    /// Phase to run in.
    #[serde(default = "default_phase")]
    pub phase: PhaseName,
}

/// Every tick, **each live agent** draws its own keyed value via
/// [`firma_rng::open_for`] (the agent id in its proper key slot, manual §21.2 —
/// not smuggled into `purpose`). The agents are treated as a ring in ascending
/// `AgentId` order: agent *k* "sends" `draw_k % modulus` units to agent *k+1*,
/// and the **net** per agent (`in − out`) is applied as a single delta, so the
/// rule emits at most one delta per agent (no §19.5 delta-uniqueness breach)
/// and conserves exactly (the ring's sends sum to its receives).
///
/// Its purpose is to exercise the per-agent RNG key path — `agent_id: Some(_)`
/// — end-to-end through the kernel, which `testkit.keyed_nudge` does not
/// (it draws one phase-global value). This is what makes §21.2 property 1
/// ("order invariance") testable for a draw that actually depends on which
/// agent is being evaluated.
pub struct KeyedPerAgent {
    id: PluginId,
    resource: ResourceKind,
    params: KeyedPerAgentParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl KeyedPerAgent {
    /// Build from parameters.
    ///
    /// # Errors
    /// A message if `modulus < 2`.
    pub fn new(params: KeyedPerAgentParams) -> Result<KeyedPerAgent, String> {
        if params.modulus < 2 {
            return Err(format!("modulus must be >= 2, got {}", params.modulus));
        }
        Ok(KeyedPerAgent {
            id: PluginId::new(catalog::KEYED_PER_AGENT_ID),
            resource: ResourceKind(params.resource.clone()),
            params,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock],
        })
    }
}

impl Rule for KeyedPerAgent {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        catalog::version()
    }
    fn phase(&self) -> Phase {
        self.params.phase.into()
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, key: RngKey) -> Vec<Delta> {
        let agents = view.live_agents();
        let n = agents.len();
        if n < 2 {
            return Vec::new();
        }
        let modulus = u64::try_from(self.params.modulus).unwrap_or(2);

        // Per-agent draw: a pure function of the agent id, via open_for.
        let draws: Vec<i64> = agents
            .iter()
            .map(|a| {
                let mut r = firma_rng::open_for(&key, Some(a.0), &self.params.purpose);
                (r.next_u64() % modulus) as i64
            })
            .collect();

        // Ring net: agent k receives draws[k-1], sends draws[k].
        let nets: Vec<i64> = (0..n).map(|k| draws[(k + n - 1) % n] - draws[k]).collect();

        // If any net would drive a balance negative, do nothing this tick — this
        // fixture must never itself trip the non-negativity invariant.
        for (k, net) in nets.iter().enumerate() {
            if view.agent_stock(agents[k], &self.resource) + net < 0 {
                return Vec::new();
            }
        }

        agents
            .iter()
            .zip(nets)
            .filter(|(_, net)| *net != 0)
            .map(|(a, net)| Delta {
                target: DeltaTarget::Agent(*a),
                kind: DeltaKind::AdjustStock {
                    resource: self.resource.clone(),
                    amount: net,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            })
            .collect()
    }
    fn assumption(&self) -> &str {
        "Each tick every live agent draws a per-agent keyed value and its net over \
         a ring transfer is applied; used to test that per-agent RNG draws are \
         independent of agent processing order (manual §21.2 property 1)."
    }
}

// --------------------------------------------------------------------------
// ForceAdjust
// --------------------------------------------------------------------------

/// Parameters for [`ForceAdjust`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForceAdjustParams {
    /// Resource to move.
    pub resource: String,
    /// Agent debited each tick (may be driven negative — that is the point).
    pub agent: u64,
    /// Agent credited each tick.
    pub other: u64,
    /// Units per tick (must be `> 0`).
    pub amount: i64,
    /// Phase to run in.
    #[serde(default = "default_phase")]
    pub phase: PhaseName,
}

/// Unconditionally moves `amount` units from `agent` to `other` every tick with
/// **no affordability check**. Conserving (so it never trips the conservation
/// invariant), but it *will* drive `agent` below zero once its balance runs
/// out, tripping the non-negativity invariant (§9.1 `solvency` semantics /
/// §19.5). Its only use is to force a deliberate mid-phase abort so the
/// event-log atomicity guarantee (§19.4 step 6, §19.5 "no repair-and-continue
/// path") can be tested. `testkit.transfer` and `testkit.keyed_nudge` both
/// guard affordability and so can never reach the abort path.
pub struct ForceAdjust {
    id: PluginId,
    resource: ResourceKind,
    params: ForceAdjustParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl ForceAdjust {
    /// Build from parameters.
    ///
    /// # Errors
    /// A message if `amount <= 0` or `agent == other`.
    pub fn new(params: ForceAdjustParams) -> Result<ForceAdjust, String> {
        if params.amount <= 0 {
            return Err(format!("amount must be > 0, got {}", params.amount));
        }
        if params.agent == params.other {
            return Err("agent and other must differ".to_string());
        }
        Ok(ForceAdjust {
            id: PluginId::new(catalog::FORCE_ADJUST_ID),
            resource: ResourceKind(params.resource.clone()),
            params,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock],
        })
    }
}

impl Rule for ForceAdjust {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        catalog::version()
    }
    fn phase(&self) -> Phase {
        self.params.phase.into()
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
                target: DeltaTarget::Agent(firma_core::AgentId(self.params.agent)),
                kind: DeltaKind::AdjustStock {
                    resource: self.resource.clone(),
                    amount: -self.params.amount,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
            Delta {
                target: DeltaTarget::Agent(firma_core::AgentId(self.params.other)),
                kind: DeltaKind::AdjustStock {
                    resource: self.resource.clone(),
                    amount: self.params.amount,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            },
        ]
    }
    fn assumption(&self) -> &str {
        "Unconditionally moves a fixed quantity between two agents every tick with \
         no affordability check; used only to force a deliberate non-negativity \
         violation for event-log abort-atomicity tests (manual §19.5)."
    }
}

// --------------------------------------------------------------------------
// SelectAction
// --------------------------------------------------------------------------

/// One `(agent, action index)` assignment for [`SelectActionParams`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    /// The agent whose selected action to set.
    pub agent: u64,
    /// The §11 canonical action index (0–8).
    pub action: u8,
}

/// Parameters for [`SelectAction`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectActionParams {
    /// The assignments applied every tick. An agent absent from this list is
    /// left alone (its selected action stays whatever it was).
    pub selections: Vec<Selection>,
    /// Phase to run in (defaults to `decide`, phase 3 — where the real
    /// decision procedure runs).
    #[serde(default = "default_decide_phase")]
    pub phase: PhaseName,
}

fn default_decide_phase() -> PhaseName {
    PhaseName::Decide
}

/// A test stand-in for the Stage-3 decision procedure (§12.3): every tick it
/// emits a `DeltaKind::SetAgentInt { field: keys::SELECTED_ACTION, .. }` for
/// each configured `(agent, action)` whose agent is live. It embodies **no**
/// decision theory — it is the hand-off wire (ADR 0022) with a hard-coded
/// choice, so the `action.*` plugins and VT-7 can run before
/// `decision.satisficing` exists. The one `firma-domain` import in the testkit
/// (the `SELECTED_ACTION` key constant) is here and only here.
pub struct SelectAction {
    id: PluginId,
    params: SelectActionParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl SelectAction {
    /// Build from parameters.
    ///
    /// # Errors
    /// A message if any `action` index exceeds `8` (§11 has nine actions) or
    /// two selections name the same agent.
    pub fn new(params: SelectActionParams) -> Result<SelectAction, String> {
        let mut seen = std::collections::BTreeSet::new();
        for s in &params.selections {
            if s.action > 8 {
                return Err(format!("action index {} out of range 0..=8", s.action));
            }
            if !seen.insert(s.agent) {
                return Err(format!("agent {} selected twice", s.agent));
            }
        }
        Ok(SelectAction {
            id: PluginId::new(catalog::SELECT_ACTION_ID),
            params,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::SetAgentInt],
        })
    }
}

impl Rule for SelectAction {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        catalog::version()
    }
    fn phase(&self) -> Phase {
        self.params.phase.into()
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        self.params
            .selections
            .iter()
            .filter(|s| view.is_live(firma_core::AgentId(s.agent)))
            .map(|s| Delta {
                target: DeltaTarget::Agent(firma_core::AgentId(s.agent)),
                kind: DeltaKind::SetAgentInt {
                    field: firma_domain::keys::SELECTED_ACTION.to_owned(),
                    value: i64::from(s.action),
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            })
            .collect()
    }
    fn assumption(&self) -> &str {
        "A fixed action index is assigned to each configured agent every tick; a \
         test stand-in for the decision procedure with no theoretical content."
    }
}

// --------------------------------------------------------------------------
// AdditiveResolver
// --------------------------------------------------------------------------

/// A neutral conflict resolver: pooled claims are applied exactly as proposed,
/// with no rationing. Conserves trivially (it changes nothing). The Phase 1
/// default; proportional rationing (§15.6) is a Phase 2 plugin.
pub struct AdditiveResolver {
    id: PluginId,
}

impl Default for AdditiveResolver {
    fn default() -> Self {
        AdditiveResolver {
            id: PluginId::new(catalog::RESOLVER_ID),
        }
    }
}

impl AdditiveResolver {
    /// Construct.
    #[must_use]
    pub fn new() -> AdditiveResolver {
        AdditiveResolver::default()
    }
}

impl ConflictResolver for AdditiveResolver {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        catalog::version()
    }
    fn handles(&self) -> ConflictClass {
        ConflictClass::ResourcePool
    }
    fn resolve(&self, group: &[Delta]) -> Vec<Delta> {
        group.to_vec()
    }
    fn assumption(&self) -> &str {
        "Contested claims on a shared pool are applied additively without \
         rationing; the neutral Phase 1 resolver."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_rejects_bad_params() {
        assert!(Transfer::new(TransferParams {
            resource: "capital".into(),
            from: Endpoint::Agent(0),
            to: Endpoint::Agent(0),
            amount: 1,
            phase: PhaseName::ActMarket,
            conflict_class: ClassName::Independent,
        })
        .is_err());
    }

    #[test]
    fn endpoint_serde_shapes() {
        assert_eq!(serde_json::to_string(&Endpoint::Env).unwrap(), "\"env\"");
        assert_eq!(
            serde_json::to_string(&Endpoint::Agent(3)).unwrap(),
            "{\"agent\":3}"
        );
    }

    #[test]
    fn assumptions_non_empty() {
        let t = Transfer::new(TransferParams {
            resource: "capital".into(),
            from: Endpoint::Env,
            to: Endpoint::Agent(0),
            amount: 1,
            phase: PhaseName::ActMarket,
            conflict_class: ClassName::Independent,
        })
        .unwrap();
        assert!(!t.assumption().is_empty());
        assert!(!AdditiveResolver::new().assumption().is_empty());
    }

    #[test]
    fn keyed_per_agent_rejects_bad_modulus() {
        assert!(KeyedPerAgent::new(KeyedPerAgentParams {
            resource: "capital".into(),
            modulus: 1,
            purpose: "p".into(),
            phase: PhaseName::ActMarket,
        })
        .is_err());
        let ok = KeyedPerAgent::new(KeyedPerAgentParams {
            resource: "capital".into(),
            modulus: 2,
            purpose: "p".into(),
            phase: PhaseName::ActMarket,
        })
        .unwrap();
        assert!(!ok.assumption().is_empty());
    }

    #[test]
    fn force_adjust_rejects_bad_params() {
        assert!(ForceAdjust::new(ForceAdjustParams {
            resource: "capital".into(),
            agent: 0,
            other: 0,
            amount: 1,
            phase: PhaseName::ActMarket,
        })
        .is_err());
        assert!(ForceAdjust::new(ForceAdjustParams {
            resource: "capital".into(),
            agent: 0,
            other: 1,
            amount: 0,
            phase: PhaseName::ActMarket,
        })
        .is_err());
    }

    struct TwoLive;
    impl View for TwoLive {
        fn tick(&self) -> firma_core::Tick {
            firma_core::Tick(0)
        }
        fn phase(&self) -> Phase {
            Phase::Decide
        }
        fn run_seed(&self, _s: firma_core::StreamId) -> u64 {
            0
        }
        fn live_agents(&self) -> &[firma_core::AgentId] {
            &[firma_core::AgentId(0), firma_core::AgentId(2)]
        }
        fn is_live(&self, a: firma_core::AgentId) -> bool {
            a.0 == 0 || a.0 == 2
        }
        fn agent_stock(&self, _a: firma_core::AgentId, _r: &ResourceKind) -> i64 {
            0
        }
        fn env_stock(&self, _r: &ResourceKind) -> i64 {
            0
        }
        fn resource_kinds(&self) -> &[ResourceKind] {
            &[]
        }
    }

    #[test]
    fn select_action_emits_set_deltas_for_live_agents_only() {
        let rule = SelectAction::new(SelectActionParams {
            selections: vec![
                Selection {
                    agent: 0,
                    action: 3,
                },
                Selection {
                    agent: 1,
                    action: 6,
                }, // not live → skipped
                Selection {
                    agent: 2,
                    action: 8,
                },
            ],
            phase: PhaseName::Decide,
        })
        .unwrap();
        let key = RngKey {
            run_seed: 0,
            stream: firma_core::StreamId::Mechanism,
            plugin_id: 0,
            phase: Phase::Decide,
            tick: 0,
            agent_id: None,
        };
        let d = rule.apply(&TwoLive, key);
        assert_eq!(d.len(), 2);
        let sel = firma_domain::keys::SELECTED_ACTION;
        assert!(d.iter().any(|x| matches!((&x.target, &x.kind),
            (DeltaTarget::Agent(a), DeltaKind::SetAgentInt { field, value })
                if a.0 == 0 && field == sel && *value == 3)));
        assert!(d.iter().any(|x| matches!((&x.target, &x.kind),
            (DeltaTarget::Agent(a), DeltaKind::SetAgentInt { field, value })
                if a.0 == 2 && field == sel && *value == 8)));
    }

    #[test]
    fn select_action_rejects_bad_params() {
        assert!(SelectAction::new(SelectActionParams {
            selections: vec![Selection {
                agent: 0,
                action: 9
            }], // > 8
            phase: PhaseName::Decide,
        })
        .is_err());
        assert!(SelectAction::new(SelectActionParams {
            selections: vec![
                Selection {
                    agent: 0,
                    action: 1
                },
                Selection {
                    agent: 0,
                    action: 2
                }, // dup agent
            ],
            phase: PhaseName::Decide,
        })
        .is_err());
    }
}
