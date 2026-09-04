//! `action.shaping.rdt_standard` — the three §11.2 constraint-shaping `Rule`
//! plugins plus the `resolve_lagged` resolver (manual §11.2, §11.3, §10.1
//! phases 5–6; ADR 0022 hand-off, ADR 0023 Λ queue / success timing).
//!
//! The name follows §5's `<category>.<theory>` convention: these actions
//! operationalise **resource-dependence theory** (RDT, manual §31.2) — the firm
//! acts on its constraint set by lobbying, contracting, and diversifying supply.
//!
//! | idx | rule | id | §11.2 effect on success |
//! |---|---|---|---|
//! | 6 | [`lobby`] | `action.shaping.rdt_standard.lobby` | `θ_limit += δ_θ` |
//! | 7 | [`contract`] | `…contract` | `θ_Q += δ_Q`; `q += q_0`; fix a `supply` edge |
//! | 8 | [`diversify`] | `…diversify` | new `supply` edge to a fresh source |
//! | — | [`resolve_lagged`] | `action.shaping.rdt_standard.resolve_lagged` | applies matured Λ entries |
//!
//! ## The three §11.3 mandatory properties (VT-7 verifies these)
//!
//! 1. **Cost at commitment.** Every shaping rule emits the cost `AdjustStock`
//!    (`−κ_a`, paired with the env pool) *unconditionally* once the
//!    precondition holds — before, and regardless of, the success draw. A
//!    failed attempt still pays.
//! 2. **Lag ≥ 1, drawn.** `Δ_a ∼ Uniform{Δ^min_a … Δ^max_a}` from the
//!    `mechanism` stream, `purpose_tag = "shaping_lag"`, per acting agent via
//!    [`firma_rng::open_for`]. [`LagRange::validate`](firma_domain::shaping::LagRange::validate)
//!    rejects `Δ^min < 1` at construction.
//! 3. **Probabilistic, `p_max < 1`.** The success coin is a second, independent
//!    draw (`purpose_tag = "shaping_success"`) compared against
//!    [`SuccessModel::p_success`](firma_domain::shaping::SuccessModel::p_success);
//!    [`SuccessModel::validate`](firma_domain::shaping::SuccessModel::validate)
//!    rejects `p_max ≥ 1`. The model is config-exposed
//!    ([`firma_domain::shaping`]).
//!
//! ## Success is decided at commitment (ADR 0023 Decision 2)
//!
//! The coin is flipped here, in `act_shaping`, using the firm's legitimacy
//! *now*; the outcome is frozen into the enqueued [`Effect`]'s `applied` flag.
//! [`resolve_lagged`] just applies whatever the matured record says. The lag is
//! still real — the effect does not fire until `maturity_tick` — and the firm
//! cannot observe the outcome early (no shaping feedback in the MVP, §16.4).

#![forbid(unsafe_code)]

pub mod support;

use firma_core::{
    ComponentId, ConflictClass, Delta, DeltaKind, DeltaKindTag, DeltaTarget, Phase, PluginId,
    ResourceKind, RngKey, Rule, View,
};
use firma_domain::shaping::{ContractParams, DiversifyParams, LagRange, LobbyParams, SuccessModel};
use firma_domain::{keys, Effect, LaggedRecord, RelationGraph};

/// `purpose_tag` for the lag draw (§11.2, §15.5 Example E).
pub const TAG_LAG: &str = "shaping_lag";
/// `purpose_tag` for the success draw (ADR 0023 Decision 3). Distinct from
/// [`TAG_LAG`] so the two draws are independent streams (§21.2).
pub const TAG_SUCCESS: &str = "shaping_success";

/// This crate's build version (ADR 0022: new crate, `1.0.0`).
#[must_use]
pub fn version() -> semver::Version {
    semver::Version::new(1, 0, 0)
}

/// Plugin ids and the declared content hash (Phase-1 style — no artefact
/// hashing yet).
pub mod catalog {
    /// `action.shaping.rdt_standard.lobby`.
    pub const LOBBY_ID: &str = "action.shaping.rdt_standard.lobby";
    /// `action.shaping.rdt_standard.contract`.
    pub const CONTRACT_ID: &str = "action.shaping.rdt_standard.contract";
    /// `action.shaping.rdt_standard.diversify`.
    pub const DIVERSIFY_ID: &str = "action.shaping.rdt_standard.diversify";
    /// `action.shaping.rdt_standard.resolve_lagged`.
    pub const RESOLVE_LAGGED_ID: &str = "action.shaping.rdt_standard.resolve_lagged";

    /// Declared content hash for the shaping build.
    pub const CONTENT_HASH: &str = "phase2s2-action-shaping-rdt-standard-v1";
}

/// The §11 canonical index of a shaping action (`lobby` 6, `contract` 7,
/// `diversify` 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapingKind {
    /// Action 6.
    Lobby,
    /// Action 7.
    Contract,
    /// Action 8.
    Diversify,
}

impl ShapingKind {
    /// The §11 canonical index.
    #[must_use]
    pub fn index(self) -> u8 {
        match self {
            ShapingKind::Lobby => 6,
            ShapingKind::Contract => 7,
            ShapingKind::Diversify => 8,
        }
    }
}

/// Resolved parameters for one shaping rule.
#[derive(Debug, Clone, Copy)]
enum Params {
    Lobby(LobbyParams),
    Contract(ContractParams),
    Diversify(DiversifyParams),
}

impl Params {
    fn kind(&self) -> ShapingKind {
        match self {
            Params::Lobby(_) => ShapingKind::Lobby,
            Params::Contract(_) => ShapingKind::Contract,
            Params::Diversify(_) => ShapingKind::Diversify,
        }
    }
    fn success(&self) -> SuccessModel {
        match self {
            Params::Lobby(p) => p.success,
            Params::Contract(p) => p.success,
            Params::Diversify(p) => p.success,
        }
    }
    fn lag(&self) -> LagRange {
        match self {
            Params::Lobby(p) => p.lag,
            Params::Contract(p) => p.lag,
            Params::Diversify(p) => p.lag,
        }
    }
    fn validate(&self) -> Result<(), String> {
        self.success().validate()?;
        self.lag().validate()?;
        Ok(())
    }
}

fn capital() -> ResourceKind {
    ResourceKind(keys::CAPITAL.to_owned())
}

/// The cost `AdjustStock` at commitment: `−spend` to the agent, `+spend` to the
/// env pool (§11.3 property 1). `ResourcePool` class — money leaves the firm
/// into the shared pool (mirrors the market rules).
fn cost_deltas(agent: firma_core::AgentId, spend: i64, origin: &PluginId) -> [Delta; 2] {
    [
        Delta {
            target: DeltaTarget::Agent(agent),
            kind: DeltaKind::AdjustStock {
                resource: capital(),
                amount: -spend,
            },
            conflict_class: ConflictClass::ResourcePool,
            origin: origin.clone(),
        },
        Delta {
            target: DeltaTarget::Environment,
            kind: DeltaKind::AdjustStock {
                resource: capital(),
                amount: spend,
            },
            conflict_class: ConflictClass::ResourcePool,
            origin: origin.clone(),
        },
    ]
}

/// `diversify`'s synthetic supply-channel id (ADR 0023): deterministic, disjoint
/// from any real `AgentId`, and distinct per `(agent, tick)`.
#[must_use]
pub fn synthetic_source(agent: firma_core::AgentId, tick: u64) -> u64 {
    1_000_000_000 + agent.0 * 100_000 + tick
}

/// One shaping rule (lobby / contract / diversify). Shares its body; differs by
/// [`Params`], [`PluginId`], and `assumption()`.
pub struct ShapingRule {
    params: Params,
    id: PluginId,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
    assumption: &'static str,
}

impl ShapingRule {
    fn new(params: Params, id: &str, assumption: &'static str) -> Result<Self, String> {
        params.validate()?;
        Ok(ShapingRule {
            params,
            id: PluginId::new(id),
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustStock, DeltaKindTag::PushAgentRecord],
            assumption,
        })
    }

    /// The success model this rule was configured with (§11.3 property 3 —
    /// "exposed in config"). Used by VT-7.
    #[must_use]
    pub fn success_model(&self) -> SuccessModel {
        self.params.success()
    }

    /// The lag distribution this rule was configured with (§11.3 property 2).
    #[must_use]
    pub fn lag_range(&self) -> LagRange {
        self.params.lag()
    }

    /// The pinned commitment `spend` = `κ_a` (§11.2 precondition / baseline).
    /// Variable overspend is a Stage-3 decision-procedure concern; Stage 2
    /// always commits the minimum.
    fn spend(&self) -> i64 {
        self.params.success().kappa_min
    }
}

impl Rule for ShapingRule {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::ActShaping
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, key: RngKey) -> Vec<Delta> {
        let kind = self.params.kind();
        let idx = i64::from(kind.index());
        let model = self.params.success();
        let lag = self.params.lag();
        let spend = self.spend();
        let tick = view.tick().0;

        // The relation graph is only consulted by `contract`; parse once.
        let graph = match RelationGraph::from_records(view.global_records(keys::RELATION_EDGES)) {
            Ok(g) => g,
            Err(_) => return Vec::new(), // corrupt store — do nothing rather than guess
        };

        let mut out = Vec::new();
        for &agent in view.live_agents() {
            if view.agent_int(agent, keys::SELECTED_ACTION) != Some(idx) {
                continue;
            }
            // §11.2 affordability precondition: r^L ≥ κ_a.
            if view.agent_stock(agent, &capital()) < spend {
                continue;
            }
            // `contract` also requires an existing `supply` partner.
            let source = match kind {
                ShapingKind::Contract => {
                    let mut sources: Vec<u64> = graph
                        .edges()
                        .iter()
                        .filter(|e| e.kind == firma_domain::EdgeKind::Supply && e.target == agent.0)
                        .map(|e| e.source)
                        .collect();
                    sources.sort_unstable();
                    match sources.first() {
                        Some(s) => *s,
                        None => continue, // no partner → cannot contract
                    }
                }
                ShapingKind::Diversify => synthetic_source(agent, tick),
                ShapingKind::Lobby => 0,
            };

            let legitimacy = view.agent_real(agent, keys::LEGITIMACY).unwrap_or(0.0);
            let p = model.p_success(legitimacy, spend);

            // Two independent per-agent draws (ADR 0023 Decision 3).
            let drawn_lag = {
                let mut r = firma_rng::open_for(&key, Some(agent.0), TAG_LAG);
                r.uniform_inclusive(lag.min, lag.max)
            };
            let applied = {
                let mut r = firma_rng::open_for(&key, Some(agent.0), TAG_SUCCESS);
                r.next_f64_unit() < p
            };

            // §11.3 property 1: cost paid now, unconditionally.
            out.extend(cost_deltas(agent, spend, &self.id));

            let effect = match kind {
                ShapingKind::Lobby => {
                    let Params::Lobby(lp) = self.params else {
                        unreachable!("kind/params agree")
                    };
                    Effect::Lobby {
                        applied,
                        theta_limit_delta: lp.delta_theta,
                    }
                }
                ShapingKind::Contract => {
                    let Params::Contract(cp) = self.params else {
                        unreachable!("kind/params agree")
                    };
                    Effect::Contract {
                        applied,
                        theta_q_delta: cp.delta_q,
                        q0: cp.q0,
                        source,
                    }
                }
                ShapingKind::Diversify => Effect::Diversify { applied, source },
            };
            let rec = LaggedRecord::new(tick + drawn_lag, effect);
            out.push(Delta {
                target: DeltaTarget::Agent(agent),
                kind: DeltaKind::PushAgentRecord {
                    list: keys::LAGGED_EFFECTS.to_owned(),
                    record_json: rec.to_json(),
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            });
        }
        out
    }
    fn assumption(&self) -> &str {
        self.assumption
    }
}

// --------------------------------------------------------------------------
// resolve_lagged
// --------------------------------------------------------------------------

/// Sum `AdjustGlobalReal` / `AdjustGlobalInt` deltas that target the same
/// `(Global, field)` into one, preserving every other delta in order (ADR
/// 0041). When two firms' matured `lobby` / `contract` effects both shift the
/// same θ field in one `resolve_lagged` phase, they must combine into a single
/// delta — simultaneous shaping is additive (ADR 0016) — rather than trip the
/// reconciler's per-rule `(target, kind, slot)` uniqueness guard.
fn merge_global_scalar_adjusts(deltas: Vec<Delta>, origin: &PluginId) -> Vec<Delta> {
    use std::collections::BTreeMap;
    let mut real_sum: BTreeMap<String, f64> = BTreeMap::new();
    let mut int_sum: BTreeMap<String, i64> = BTreeMap::new();
    let mut real_order: Vec<String> = Vec::new();
    let mut int_order: Vec<String> = Vec::new();
    let mut rest: Vec<Delta> = Vec::new();
    for d in deltas {
        match (&d.target, &d.kind) {
            (DeltaTarget::Global, DeltaKind::AdjustGlobalReal { field, delta }) => {
                if real_sum
                    .insert(
                        field.clone(),
                        real_sum.get(field).copied().unwrap_or(0.0) + delta,
                    )
                    .is_none()
                {
                    real_order.push(field.clone());
                }
            }
            (DeltaTarget::Global, DeltaKind::AdjustGlobalInt { field, delta }) => {
                if int_sum
                    .insert(
                        field.clone(),
                        int_sum.get(field).copied().unwrap_or(0) + delta,
                    )
                    .is_none()
                {
                    int_order.push(field.clone());
                }
            }
            _ => rest.push(d),
        }
    }
    let g = |kind| Delta {
        target: DeltaTarget::Global,
        kind,
        conflict_class: ConflictClass::Independent,
        origin: origin.clone(),
    };
    for f in real_order {
        rest.push(g(DeltaKind::AdjustGlobalReal {
            field: f.clone(),
            delta: real_sum[&f],
        }));
    }
    for f in int_order {
        rest.push(g(DeltaKind::AdjustGlobalInt {
            field: f.clone(),
            delta: int_sum[&f],
        }));
    }
    rest
}

/// The `resolve_lagged` phase resolver (§10.1 phase 6; ADR 0023 Decision 5).
/// For each live agent it applies every Λ entry whose `maturity_tick == t` via
/// [`Effect::deltas_at_maturity`], then drains those entries with one
/// [`DeltaKind::ReplaceAgentList`]. Carries no per-`Effect` logic. Lives in this
/// crate (not the market crate) so the `CapabilityGain` case needs no
/// cross-plugin dependency — [`Effect`] is in `firma-domain`, which both action
/// crates depend on.
pub struct LaggedEffectResolver {
    id: PluginId,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Default for LaggedEffectResolver {
    fn default() -> Self {
        LaggedEffectResolver {
            id: PluginId::new(catalog::RESOLVE_LAGGED_ID),
            reads: vec![ComponentId::ledger()],
            writes: vec![
                DeltaKindTag::AdjustAgentReal,
                DeltaKindTag::AdjustAgentInt,
                DeltaKindTag::AdjustGlobalReal,
                DeltaKindTag::AdjustGlobalInt,
                DeltaKindTag::PushGlobalRecord,
                DeltaKindTag::ReplaceAgentList,
            ],
        }
    }
}

impl LaggedEffectResolver {
    /// Construct.
    #[must_use]
    pub fn new() -> LaggedEffectResolver {
        LaggedEffectResolver::default()
    }
}

impl Rule for LaggedEffectResolver {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::ResolveLagged
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        let tick = view.tick().0;
        let mut out = Vec::new();
        for &agent in view.live_agents() {
            let records = view.agent_records(agent, keys::LAGGED_EFFECTS);
            if records.is_empty() {
                continue;
            }
            let mut matured = Vec::new();
            let mut pending: Vec<String> = Vec::new();
            for raw in records {
                match LaggedRecord::from_json(raw) {
                    Ok(rec) if rec.matures_at(tick) => matured.push(rec),
                    // Unmatured, or unparseable — keep it in the queue verbatim.
                    _ => pending.push(raw.clone()),
                }
            }
            if matured.is_empty() {
                continue;
            }
            let cap_now = view.agent_real(agent, keys::CAPABILITY).unwrap_or(0.0);
            for rec in &matured {
                out.extend(rec.effect.deltas_at_maturity(agent, &self.id, cap_now));
            }
            out.push(Delta {
                target: DeltaTarget::Agent(agent),
                kind: DeltaKind::ReplaceAgentList {
                    list: keys::LAGGED_EFFECTS.to_owned(),
                    records_json: pending,
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            });
        }
        merge_global_scalar_adjusts(out, &self.id)
    }
    fn assumption(&self) -> &str {
        "A constraint-shaping or capability-investment effect committed earlier \
         takes hold only when its drawn maturity tick arrives; until then it is \
         inert (§10.1 phase 6, §11.3 lag; ADR 0023)."
    }
}

// --------------------------------------------------------------------------
// constructors + catalog
// --------------------------------------------------------------------------

fn parse_or_default<T>(
    params: &serde_json::Value,
    default: impl FnOnce() -> Option<T>,
) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    if params.is_null() {
        default().ok_or_else(|| "parameters are required for this shaping action".to_string())
    } else {
        serde_json::from_value(params.clone()).map_err(|e| e.to_string())
    }
}

/// Build the `lobby` rule (§11.2 action 6). Parameters optional — §16.1 defaults.
///
/// # Errors
/// Invalid [`LobbyParams`], or a model failing §11.2/§11.3 validation.
pub fn lobby(params: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let p: LobbyParams = parse_or_default(params, || Some(LobbyParams::default()))?;
    Ok(Box::new(ShapingRule::new(
        Params::Lobby(p),
        catalog::LOBBY_ID,
        "At a cost paid now, the firm petitions for a higher permitted \
         regulated-activity limit; the shift θ_limit += δ_θ arrives after a \
         drawn lag and only with probability p_success < 1 (§11.2 action 6, RDT \
         political action).",
    )?))
}

/// Build the `contract` rule (§11.2 action 7). Parameters **required** (no
/// §16.1 values — ADR 0023).
///
/// # Errors
/// Missing/invalid [`ContractParams`].
pub fn contract(params: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let p: ContractParams = parse_or_default::<ContractParams>(params, || None)?;
    Ok(Box::new(ShapingRule::new(
        Params::Contract(p),
        catalog::CONTRACT_ID,
        "At a cost paid now, the firm negotiates a long-term supply contract \
         that raises its permitted obligation ceiling but also adds q_0 to its \
         obligation and fixes a supply edge, raising dependence; deliberately \
         double-edged (§11.2 action 7).",
    )?))
}

/// Build the `diversify` rule (§11.2 action 8). Parameters **required** (no
/// §16.1 values — ADR 0023).
///
/// # Errors
/// Missing/invalid [`DiversifyParams`].
pub fn diversify(params: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let p: DiversifyParams = parse_or_default::<DiversifyParams>(params, || None)?;
    Ok(Box::new(ShapingRule::new(
        Params::Diversify(p),
        catalog::DIVERSIFY_ID,
        "At a cost paid now, the firm opens a new supply channel; after a drawn \
         lag and with probability p_success < 1 it gains a fresh supply edge, \
         reducing dependence concentration (§11.2 action 8).",
    )?))
}

/// Build the `resolve_lagged` resolver rule. Parameters ignored.
///
/// # Errors
/// Never — signature matches the other constructors.
pub fn resolve_lagged(_params: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(LaggedEffectResolver::new()))
}

/// A constructor signature shared by every rule this crate registers.
pub type Ctor = fn(&serde_json::Value) -> Result<Box<dyn Rule>, String>;

/// Registration entries for `firma-cli` — `(id, content_hash, ctor)`, no
/// dependency on `firma-registry`. Includes `resolve_lagged`.
#[must_use]
pub fn registered() -> Vec<(&'static str, &'static str, Ctor)> {
    vec![
        (catalog::LOBBY_ID, catalog::CONTENT_HASH, lobby),
        (catalog::CONTRACT_ID, catalog::CONTENT_HASH, contract),
        (catalog::DIVERSIFY_ID, catalog::CONTENT_HASH, diversify),
        (
            catalog::RESOLVE_LAGGED_ID,
            catalog::CONTENT_HASH,
            resolve_lagged,
        ),
    ]
}

/// One shaping action for VT-7's **generic** §11.3 property check. Iterating
/// this list is iterating "every registered shaping plugin" (§11.3): a fourth
/// shaping action is covered by VT-7 as soon as it is added here.
pub struct ShapingCatalogEntry {
    /// Stable plugin id.
    pub id: &'static str,
    /// The §11 canonical action index (6/7/8) this rule executes.
    pub index: u8,
    /// Constructor (parameters as JSON).
    pub ctor: Ctor,
    /// Reference parameters for VT-7 (`contract` / `diversify` have no §16.1
    /// defaults, so the suite must supply some; `lobby` uses `Null`).
    pub reference_params: fn() -> serde_json::Value,
    /// The §11.3 property-3 success model for `reference_params` — so VT-7 can
    /// assert `p_max < 1` directly, not only empirically.
    pub success_model: fn(&serde_json::Value) -> Result<SuccessModel, String>,
    /// The §11.3 property-2 lag range for `reference_params`.
    pub lag_range: fn(&serde_json::Value) -> Result<LagRange, String>,
}

fn lobby_model(p: &serde_json::Value) -> Result<SuccessModel, String> {
    let lp: LobbyParams = parse_or_default(p, || Some(LobbyParams::default()))?;
    Ok(lp.success)
}
fn lobby_lag(p: &serde_json::Value) -> Result<LagRange, String> {
    let lp: LobbyParams = parse_or_default(p, || Some(LobbyParams::default()))?;
    Ok(lp.lag)
}
fn contract_model(p: &serde_json::Value) -> Result<SuccessModel, String> {
    let cp: ContractParams = parse_or_default::<ContractParams>(p, || None)?;
    Ok(cp.success)
}
fn contract_lag(p: &serde_json::Value) -> Result<LagRange, String> {
    let cp: ContractParams = parse_or_default::<ContractParams>(p, || None)?;
    Ok(cp.lag)
}
fn diversify_model(p: &serde_json::Value) -> Result<SuccessModel, String> {
    let dp: DiversifyParams = parse_or_default::<DiversifyParams>(p, || None)?;
    Ok(dp.success)
}
fn diversify_lag(p: &serde_json::Value) -> Result<LagRange, String> {
    let dp: DiversifyParams = parse_or_default::<DiversifyParams>(p, || None)?;
    Ok(dp.lag)
}

/// Reference (VT-7) parameters for `contract` / `diversify`, which have no
/// §16.1 defaults. Chosen to satisfy §11.2/§11.3; **not calibrated**. `b_lambda`
/// / `b_kappa` are stated explicitly (Stage-2 review Fix 2 — no serde default)
/// at the same `[D]` values `lobby` uses, so VT-7's numbers are unchanged.
#[must_use]
pub fn reference_contract_params() -> serde_json::Value {
    serde_json::json!({
        "lag": { "min": 3, "max": 8 },
        "success": { "p0": 0.2, "b_lambda": 0.2, "b_kappa": 0.1, "p_max": 0.6, "kappa_min": 30 },
        "delta_q": 20,
        "q0": 5
    })
}

/// See [`reference_contract_params`].
#[must_use]
pub fn reference_diversify_params() -> serde_json::Value {
    serde_json::json!({
        "lag": { "min": 2, "max": 5 },
        "success": { "p0": 0.15, "b_lambda": 0.2, "b_kappa": 0.1, "p_max": 0.55, "kappa_min": 15 }
    })
}

/// The shaping actions VT-7 iterates.
#[must_use]
pub fn shaping_catalog() -> Vec<ShapingCatalogEntry> {
    vec![
        ShapingCatalogEntry {
            id: catalog::LOBBY_ID,
            index: ShapingKind::Lobby.index(),
            ctor: lobby,
            reference_params: || serde_json::Value::Null,
            success_model: lobby_model,
            lag_range: lobby_lag,
        },
        ShapingCatalogEntry {
            id: catalog::CONTRACT_ID,
            index: ShapingKind::Contract.index(),
            ctor: contract,
            reference_params: reference_contract_params,
            success_model: contract_model,
            lag_range: contract_lag,
        },
        ShapingCatalogEntry {
            id: catalog::DIVERSIFY_ID,
            index: ShapingKind::Diversify.index(),
            ctor: diversify,
            reference_params: reference_diversify_params,
            success_model: diversify_model,
            lag_range: diversify_lag,
        },
    ]
}

#[cfg(test)]
mod tests;
