//! The `constrain` (phase 7) and `enforce` (phase 8) `Rule`s (manual §10.1,
//! §9.1; ADR 0028, ADR 0029).
//!
//! * [`ActionWindow`] (`constraint.action_window`) — phase `Constrain`. Appends
//!   this tick's selected action to the per-agent action window `W` and trims
//!   it to `L_W` entries (ADR 0014, ADR 0028). Phase 7 writes **nothing** to
//!   `θ` (all θ-writers are phase-1 shocks or phase-6 matured effects — ADR
//!   0028 Part A) and does **not** persist `u` (`u`, like `h`, is computed on
//!   demand from `W` by its consumers).
//! * [`Enforce`] (`constraint.enforce`) — phase `Enforce`. Detects §9.1
//!   violations for every live agent and applies each semantic's consequence,
//!   dispatching on `ViolationSemantic` with an explicit no-op arm for
//!   `AdmissibilityGate` (`scope` is a decide-phase gate — ADR 0021). Deaths
//!   transfer the firm's stocks to the environment (conservation, §22.2) and
//!   emit `RemoveAgent`; edge severances are collected into a single
//!   `ReplaceGlobalList` for the whole phase (ADR 0029).

use firma_core::{
    AgentId, ComponentId, ConflictClass, Delta, DeltaKind, DeltaKindTag, DeltaTarget, Phase,
    PluginId, ResourceKind, RngKey, Rule, View,
};
use firma_domain::{
    keys, margin, Aspirations, Constraint, ConstraintContext, ConstraintParams, Edge, EdgeKind,
    FirmAuxState, FirmState, ViolationSemantic, WindowEntry,
};
use serde::{Deserialize, Serialize};

use crate::{
    catalog, default_delta_lambda, default_p_c, default_s_q, default_t_c, Compliance,
    ComplianceParams, Obligation, ObligationParams, Scope, ScopeParams, Solvency, SolvencyParams,
};

fn cap() -> ResourceKind {
    ResourceKind(keys::CAPITAL.to_owned())
}

/// Read a firm's `d ≤ 4` constraint-carrying state from the view.
fn firm_state(view: &dyn View, agent: AgentId) -> FirmState {
    FirmState {
        liquid_capital: view.agent_stock(agent, &cap()),
        input_stock: view.agent_stock(agent, &ResourceKind(keys::INPUT.to_owned())),
        capability: view.agent_real(agent, keys::CAPABILITY).unwrap_or(0.0),
        obligation: view.agent_int(agent, keys::OBLIGATION).unwrap_or(0),
    }
}

/// `u` for one firm — derived from the action window `W`, falling back to a
/// seeded `keys::REGULATED_INTENSITY` while `W` is empty (tick 0). ADR 0028.
fn firm_u(view: &dyn View, agent: AgentId, l_w: usize) -> f64 {
    let w = view.agent_records(agent, keys::ACTION_WINDOW);
    if w.is_empty() {
        view.agent_real(agent, keys::REGULATED_INTENSITY)
            .unwrap_or(0.0)
    } else {
        let entries: Vec<WindowEntry> = w
            .iter()
            .filter_map(|s| WindowEntry::from_json(s).ok())
            .collect();
        margin::u_from_window(&entries, l_w)
    }
}

fn theta(view: &dyn View) -> ConstraintParams {
    ConstraintParams {
        theta_limit: view.global_real(keys::THETA_LIMIT).unwrap_or(0.0),
        theta_cap: view.global_real(keys::THETA_CAP).unwrap_or(0.0),
        theta_q: view.global_int(keys::THETA_Q).unwrap_or(0),
    }
}

// ==========================================================================
// constrain (phase 7) — the action window W
// ==========================================================================

/// Parameters for [`ActionWindow`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionWindowParams {
    /// `L_W` — window length (§16.1 default `8`; sweep `{4, 8, 16}`).
    #[serde(default = "default_l_w")]
    pub l_w: u32,
}

fn default_l_w() -> u32 {
    8
}

impl Default for ActionWindowParams {
    fn default() -> Self {
        ActionWindowParams { l_w: default_l_w() }
    }
}

/// `constraint.action_window` — phase `Constrain` (§10.1 phase 7). Maintains
/// `W`; that is its entire job (ADR 0028).
pub struct ActionWindow {
    id: PluginId,
    l_w: usize,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl ActionWindow {
    /// Build from parameters.
    ///
    /// # Errors
    /// `l_w == 0`.
    pub fn new(p: ActionWindowParams) -> Result<ActionWindow, String> {
        if p.l_w == 0 {
            return Err("l_w must be >= 1 (§16.1)".to_string());
        }
        Ok(ActionWindow {
            id: PluginId::new(catalog::ACTION_WINDOW_ID),
            l_w: p.l_w as usize,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::ReplaceAgentList],
        })
    }
}

impl Rule for ActionWindow {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        crate::version()
    }
    fn phase(&self) -> Phase {
        Phase::Constrain
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
            // This tick's action (set in `decide`, reconciled — ADR 0022).
            // A firm with no decision this tick is recorded as `hold` (0).
            let action = view
                .agent_int(agent, keys::SELECTED_ACTION)
                .and_then(|v| u8::try_from(v).ok())
                .filter(|a| *a <= 8)
                .unwrap_or(0);

            let mut w: Vec<WindowEntry> = view
                .agent_records(agent, keys::ACTION_WINDOW)
                .iter()
                .filter_map(|s| WindowEntry::from_json(s).ok())
                .collect();
            w.push(WindowEntry::new(tick, action));
            if w.len() > self.l_w {
                let drop = w.len() - self.l_w;
                w.drain(0..drop);
            }

            out.push(Delta {
                target: DeltaTarget::Agent(agent),
                kind: DeltaKind::ReplaceAgentList {
                    list: keys::ACTION_WINDOW.to_owned(),
                    records_json: w.iter().map(WindowEntry::to_json).collect(),
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            });
        }
        out
    }
    fn assumption(&self) -> &str {
        "A firm's regulated-activity intensity u is the fraction of its last L_W \
         actions that were regulated production; this rule keeps that window \
         current (§8.1, §9.1 compliance; ADR 0014)."
    }
}

// ==========================================================================
// enforce (phase 8) — §9.1 violation processing
// ==========================================================================

/// Parameters for [`Enforce`] (§16.1). `P_q` has **no §16.1 default** (ADR
/// 0021) and is required.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnforceParams {
    /// `T_c` — ticks within which a second `compliance` violation is fatal
    /// (§16.1 default `4`). The boundary is **inclusive** (ADR 0029).
    #[serde(default = "default_t_c")]
    pub t_c: u64,
    /// `P_c` — `compliance` first-violation penalty (§16.1 default `30`).
    #[serde(default = "default_p_c")]
    pub p_c: i64,
    /// `δ_λ` — legitimacy lost on a first `compliance` violation (§16.1 default
    /// `0.15`).
    #[serde(default = "default_delta_lambda")]
    pub delta_lambda: f64,
    /// `P_q` — `obligation` penalty. **Required** — §16.1 gives no value
    /// (ADR 0021).
    pub p_q: i64,
    /// `L_W` — action-window length for the `u` recompute in `g_2`.
    #[serde(default = "default_l_w")]
    pub l_w: u32,
}

/// `constraint.enforce` — phase `Enforce` (§10.1 phase 8). Runs for every live
/// agent, checking all four §9.1 constraints.
pub struct Enforce {
    id: PluginId,
    p: EnforceParams,
    semantics: [ViolationSemantic; 4],
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Enforce {
    /// Build from parameters.
    ///
    /// # Errors
    /// `P_c < 0`, `δ_λ ∉ [0, 1]`, `P_q < 0`, `T_c == 0`, or `L_W == 0`.
    pub fn new(p: EnforceParams) -> Result<Enforce, String> {
        if p.p_c < 0 {
            return Err(format!("p_c must be >= 0, got {}", p.p_c));
        }
        if !(0.0..=1.0).contains(&p.delta_lambda) {
            return Err(format!(
                "delta_lambda must be in [0, 1], got {}",
                p.delta_lambda
            ));
        }
        if p.p_q < 0 {
            return Err(format!("p_q must be >= 0, got {}", p.p_q));
        }
        if p.t_c == 0 || p.l_w == 0 {
            return Err("t_c and l_w must be >= 1".to_string());
        }
        // The four §9.1 violation semantics — from the real `Constraint`
        // objects, so a change to any of them is caught here.
        let semantics = [
            Solvency::new(SolvencyParams::default())?.violation(),
            Compliance::new(ComplianceParams {
                penalty: p.p_c,
                window_ticks: p.t_c,
                legitimacy_loss: p.delta_lambda,
                ..Default::default()
            })?
            .violation(),
            Scope::new(ScopeParams::default())?.violation(),
            Obligation::new(ObligationParams {
                scale: default_s_q(),
                penalty: p.p_q,
            })?
            .violation(),
        ];
        Ok(Enforce {
            id: PluginId::new(catalog::ENFORCE_ID),
            p,
            semantics,
            reads: vec![ComponentId::ledger()],
            writes: vec![
                DeltaKindTag::AdjustStock,
                DeltaKindTag::AdjustAgentReal,
                DeltaKindTag::SetAgentInt,
                DeltaKindTag::RemoveAgent,
                DeltaKindTag::ReplaceGlobalList,
            ],
        })
    }

    /// Is constraint `j` (`g_j = g_val`) violated? `Death` semantics fire at
    /// the boundary (`g ≥ 0` — `r^L == 0` is insolvency, and `g_1 > 0` is
    /// unreachable under the non-negativity invariant); the other three fire
    /// strictly (`g > 0`) — ADR 0029.
    fn violated(semantic: ViolationSemantic, g_val: f64) -> bool {
        match semantic {
            ViolationSemantic::Death => g_val >= 0.0,
            _ => g_val > 0.0,
        }
    }

    /// A paired `AdjustStock`: `-amount` from `agent`, `+amount` to the env
    /// pool (a penalty / a dead firm's assets flow to the market). `ResourcePool`.
    fn to_env(&self, agent: AgentId, resource: &ResourceKind, amount: i64) -> [Delta; 2] {
        [
            Delta {
                target: DeltaTarget::Agent(agent),
                kind: DeltaKind::AdjustStock {
                    resource: resource.clone(),
                    amount: -amount,
                },
                conflict_class: ConflictClass::ResourcePool,
                origin: self.id.clone(),
            },
            Delta {
                target: DeltaTarget::Environment,
                kind: DeltaKind::AdjustStock {
                    resource: resource.clone(),
                    amount,
                },
                conflict_class: ConflictClass::ResourcePool,
                origin: self.id.clone(),
            },
        ]
    }
}

impl Rule for Enforce {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        crate::version()
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
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        let tick = view.tick().0;
        let theta = theta(view);
        let l_w = self.p.l_w as usize;

        // Current relation edges, parsed once. Severances mutate this in place;
        // one `ReplaceGlobalList` is emitted at the end iff it changed.
        let mut edges: Vec<Edge> = view
            .global_records(keys::RELATION_EDGES)
            .iter()
            .filter_map(|s| serde_json::from_str::<Edge>(s).ok())
            .collect();
        let mut edges_changed = false;

        let mut out: Vec<Delta> = Vec::new();
        let mut deaths: Vec<(AgentId, String)> = Vec::new();

        for &agent in view.live_agents() {
            // (§19.2: live_agents() is ascending, BTreeMap-backed — deterministic.)
            let state = firm_state(view, agent);
            let lambda = view.agent_real(agent, keys::LEGITIMACY).unwrap_or(1.0);
            let aux = FirmAuxState {
                legitimacy: lambda,
                regulated_intensity: firm_u(view, agent, l_w),
                aspirations: Aspirations {
                    capital_growth: 0.0,
                    capability: 0.0,
                    obligation_clearance: 0.0,
                },
            };
            let ctx = ConstraintContext {
                state: &state,
                aux: &aux,
                theta: &theta,
            };
            let g = margin::all_g(&ctx); // [g_1, g_2, g_3, g_4]

            let mut lethal_causes: Vec<&str> = Vec::new();
            let mut first_strike_compliance = false;
            let mut obligation_violated = false;

            for (semantic, &g_val) in self.semantics.iter().copied().zip(g.iter()) {
                if !Self::violated(semantic, g_val) {
                    continue;
                }
                match semantic {
                    ViolationSemantic::Death => lethal_causes.push("solvency"),
                    ViolationSemantic::Graduated { .. } => {
                        // compliance — first or second strike within T_c?
                        match view.agent_int(agent, keys::COMPLIANCE_LAST_VIOLATION_TICK) {
                            Some(last) if tick.saturating_sub(last.max(0) as u64) <= self.p.t_c => {
                                lethal_causes.push("compliance"); // second strike ⇒ death
                            }
                            _ => first_strike_compliance = true,
                        }
                    }
                    ViolationSemantic::AdmissibilityGate => {
                        // Explicit, deliberate no-op: `scope` is checked during
                        // `decide` (§11.4 / ADR 0021), never here.
                    }
                    ViolationSemantic::Relational { .. } => obligation_violated = true,
                }
            }

            if !lethal_causes.is_empty() {
                // A dying firm pays no fine and severs no edges of its own —
                // `RemoveAgent` + the death edge cleanup below cover it.
                deaths.push((agent, lethal_causes.join("+")));
                continue;
            }

            // `compliance` first-strike (`P_c`) and `obligation` (`P_q`) are
            // independent, non-lethal §9.1 consequences — nothing prevents
            // both firing for the same agent in the same tick (a firm can be
            // over-regulated *and* over-obligated at once). Each used to call
            // `to_env(agent, capital, ..)` on its own, so a simultaneous case
            // produced two same-agent `AdjustStock{capital}` deltas from this
            // one rule — exactly what the per-rule uniqueness guard
            // (ADR-0033) rejects as `DuplicateDelta`. Accumulate one capital
            // penalty per agent instead, and emit it once, jointly capped by
            // the agent's starting-of-tick capital (ADR-0043 Decision 2 —
            // supersedes no prior ADR; this path was never previously
            // reachable in a shipped or tested config).
            let mut capital_penalty = 0i64;

            if first_strike_compliance {
                capital_penalty += self.p.p_c.max(0);
                let d_lam = self.p.delta_lambda.min(lambda.max(0.0));
                if d_lam > 0.0 {
                    out.push(Delta {
                        target: DeltaTarget::Agent(agent),
                        kind: DeltaKind::AdjustAgentReal {
                            field: keys::LEGITIMACY.to_owned(),
                            delta: -d_lam,
                        },
                        conflict_class: ConflictClass::Independent,
                        origin: self.id.clone(),
                    });
                }
                out.push(Delta {
                    target: DeltaTarget::Agent(agent),
                    kind: DeltaKind::SetAgentInt {
                        field: keys::COMPLIANCE_LAST_VIOLATION_TICK.to_owned(),
                        value: tick as i64,
                    },
                    conflict_class: ConflictClass::Independent,
                    origin: self.id.clone(),
                });
            }

            if obligation_violated {
                let before = edges.len();
                edges.retain(|e| {
                    !(e.kind == EdgeKind::Supply && (e.source == agent.0 || e.target == agent.0))
                });
                if edges.len() != before {
                    edges_changed = true;
                }
                capital_penalty += self.p.p_q.max(0);
                // The agent survives (§9.1 `obligation` is never lethal).
            }

            let pen = capital_penalty.min(state.liquid_capital).max(0);
            if pen > 0 {
                out.extend(self.to_env(agent, &cap(), pen));
            }
        }

        // --- deaths: full edge cleanup + stock transfer + RemoveAgent ---
        for (agent, cause) in &deaths {
            let before = edges.len();
            edges.retain(|e| e.source != agent.0 && e.target != agent.0); // all edge types (ADR 0017)
            if edges.len() != before {
                edges_changed = true;
            }
            for r in view.resource_kinds() {
                let q = view.agent_stock(*agent, r);
                if q != 0 {
                    out.extend(self.to_env(*agent, r, q)); // assets → the market
                }
            }
            out.push(Delta {
                target: DeltaTarget::Agent(*agent),
                kind: DeltaKind::RemoveAgent {
                    reason: cause.clone(),
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            });
        }

        if edges_changed {
            out.push(Delta {
                target: DeltaTarget::Global,
                kind: DeltaKind::ReplaceGlobalList {
                    list: keys::RELATION_EDGES.to_owned(),
                    records_json: edges
                        .iter()
                        .map(|e| serde_json::to_string(e).expect("Edge serialises"))
                        .collect(),
                },
                conflict_class: ConflictClass::Independent,
                origin: self.id.clone(),
            });
        }

        out
    }
    fn assumption(&self) -> &str {
        "Each of the four constraints has its own consequence on violation: \
         running out of capital is fatal; exceeding the regulated-activity limit \
         is penalised then, on repetition within T_c ticks, fatal; falling below \
         the capability threshold only bars regulated production; and exceeding \
         the obligation ceiling severs supply ties and levies a fine but is \
         survivable (§9.1)."
    }
}

/// A constructor signature shared by the phase-7/8 rules.
pub type Ctor = fn(&serde_json::Value) -> Result<Box<dyn Rule>, String>;

/// Build `constraint.action_window` from JSON (all fields optional; §16.1
/// default `L_W = 8`).
///
/// # Errors
/// Invalid [`ActionWindowParams`].
pub fn build_action_window(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: ActionWindowParams = if p.is_null() {
        ActionWindowParams::default()
    } else {
        serde_json::from_value(p.clone()).map_err(|e| e.to_string())?
    };
    Ok(Box::new(ActionWindow::new(params)?))
}

/// Build `constraint.enforce` from JSON. `p_q` is **required** (ADR 0021).
///
/// # Errors
/// Missing / invalid [`EnforceParams`].
pub fn build_enforce(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: EnforceParams =
        serde_json::from_value(p.clone()).map_err(|e| format!("enforce params: {e}"))?;
    Ok(Box::new(Enforce::new(params)?))
}

/// Registration entries for `firma-cli` — `(id, content_hash, ctor)`.
#[must_use]
pub fn registered_rules() -> Vec<(&'static str, &'static str, Ctor)> {
    vec![
        (
            catalog::ACTION_WINDOW_ID,
            catalog::PHASE_RULES_HASH,
            build_action_window,
        ),
        (
            catalog::ENFORCE_ID,
            catalog::PHASE_RULES_HASH,
            build_enforce,
        ),
    ]
}

#[cfg(test)]
mod tests;
