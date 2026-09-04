//! Incremental, kernel-free reconstruction of per-firm state from the event
//! log (§17 A5, §22.2; ADR 0045/0046). One fold, driven one [`Event`] at a
//! time so it serves both a batch reader ([`crate::sanity::sanity_from_run`])
//! and a live tailer (`firma-tui`) from the same code — `h`, the four `g_j`,
//! `u`, and repertoire entropy are each computed in exactly one place here,
//! not re-derived per consumer.

use std::collections::BTreeMap;

use firma_config::RunConfig;
use firma_core::{DeltaKind, DeltaTarget, Event};
use firma_domain::{
    keys, margin, Aspirations, ConstraintContext, ConstraintParams, FirmAuxState, FirmState,
    LaggedRecord, ScaleFactors, WindowEntry,
};
use serde::Serialize;

/// `H_rep = −Σ p_a log₂ p_a` over a window of action indices (§14.2 R1).
#[must_use]
pub fn repertoire_entropy(window: &[WindowEntry]) -> f64 {
    if window.is_empty() {
        return 0.0;
    }
    let n = window.len() as f64;
    let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
    for e in window {
        *counts.entry(e.action).or_default() += 1;
    }
    -counts
        .values()
        .map(|&k| {
            let p = k as f64 / n;
            p * p.log2()
        })
        .sum::<f64>()
}

/// One firm's raw reconstructed ledger — inputs to the derived quantities in
/// [`FirmSnapshot`], not the quantities themselves.
#[derive(Debug, Clone)]
struct Firm {
    r_l: i64,
    r_i: i64,
    c: f64,
    q: i64,
    lambda: f64,
    window: Vec<WindowEntry>,
    action_counts: BTreeMap<u8, u64>,
    alive: bool,
}

/// A read-only snapshot of one firm's current derived state — `h`/`g_j`/`u`
/// computed on demand from the raw ledger, so there is exactly one place a
/// log-derived margin is computed from (§17 A5). `Serialize` is for the
/// Rust/Python cross-check only (Stage 7 Part D,
/// `examples/sanity_report_json.rs`).
#[derive(Debug, Clone, Serialize)]
pub struct FirmSnapshot {
    /// Agent id.
    pub id: u64,
    /// Whether the firm is currently live.
    pub alive: bool,
    /// `r^L`.
    pub liquid_capital: i64,
    /// `r^I`.
    pub input_stock: i64,
    /// `c`.
    pub capability: f64,
    /// `q`.
    pub obligation: i64,
    /// `λ`.
    pub legitimacy: f64,
    /// `u` — trailing-window regulated-activity intensity (ADR 0014).
    pub regulated_intensity: f64,
    /// The standard four-constraint viability margin (§9.2).
    pub h: f64,
    /// `[solvency, compliance, scope, obligation]` (§9.1, canonical order).
    pub g: [f64; 4],
    /// Trailing action window `W`, oldest first.
    pub window: Vec<WindowEntry>,
    /// Cumulative selected-action tally over the whole reconstructed run.
    pub action_counts: BTreeMap<u8, u64>,
}

/// One fact worth reacting to, extracted from a raw [`Event`] during
/// [`Reconstruction::apply`]. The single place "what does this raw event
/// mean" is decided for consumers — `sanity::sanity_from_run` and
/// `firma-tui` both fold these instead of re-reading `DeltaKind`
/// discriminants themselves.
#[derive(Debug, Clone)]
pub enum Notable {
    /// The log's first record.
    RunStarted {
        /// Manual version in force.
        manual_version: String,
        /// Engine semantic version.
        engine_version: String,
        /// Total ticks the run will execute.
        horizon: u64,
    },
    /// A tick's phases are about to run.
    TickStarted(u64),
    /// A tick finished; all invariants held.
    TickCompleted {
        /// The tick.
        tick: u64,
        /// Live agent count at tick end.
        live_agents: u32,
    },
    /// An agent entered the population.
    AgentBorn {
        /// The tick.
        tick: u64,
        /// The new agent.
        agent: u64,
    },
    /// An agent left the population.
    AgentDied {
        /// The tick.
        tick: u64,
        /// The departed agent.
        agent: u64,
        /// Why.
        cause: String,
    },
    /// A `decision.*` plugin selected an action for a firm (§11 canonical
    /// index 0–8).
    Decision {
        /// The tick.
        tick: u64,
        /// The deciding firm.
        agent: u64,
        /// The plugin id, e.g. `decision.satisficing`.
        plugin: String,
        /// The selected action.
        action: u8,
    },
    /// `decision.satisficing` logged its §12.3 Step-2 `Focus` for a firm
    /// (`0` = `SURVIVAL`).
    Focus {
        /// The tick.
        tick: u64,
        /// The firm.
        agent: u64,
        /// The logged focus code.
        focus: i64,
    },
    /// A `constraint.enforce` first-strike `compliance` penalty — a bind the
    /// end-of-tick `g_j` sweep alone would miss for a firm that dies before
    /// the next `TickCompleted`.
    ComplianceFirstStrike {
        /// The tick.
        tick: u64,
        /// The firm.
        agent: u64,
    },
    /// A shaping action committed in `act_shaping` (phase 5) — its success
    /// is decided at commitment (ADR 0023 Decision 3), so `applied` is
    /// already known. `None` if the committed record was a `CapabilityGain`
    /// (a market-action maturity through the same queue, not a shaping
    /// attempt).
    ShapingCommitted {
        /// The tick.
        tick: u64,
        /// The firm.
        agent: u64,
        /// Whether the attempt succeeds, if this was a shaping effect.
        applied: Option<bool>,
    },
    /// A global θ/price field moved from a `shock.*` plugin.
    Shock {
        /// The tick.
        tick: u64,
        /// The shock plugin id.
        origin: String,
        /// The field it moved.
        field: String,
    },
}

fn param_f64(cfg: &RunConfig, plugin: &str, key: &str) -> Option<f64> {
    cfg.rules
        .iter()
        .find(|r| r.id.0 == plugin)?
        .params
        .get(key)?
        .as_f64()
}
fn param_u64(cfg: &RunConfig, plugin: &str, key: &str) -> Option<u64> {
    cfg.rules
        .iter()
        .find(|r| r.id.0 == plugin)?
        .params
        .get(key)?
        .as_u64()
}

/// The `applied` flag of a matured shaping effect, or `None` for a
/// non-shaping effect (`CapabilityGain`).
fn shaping_applied(rec: &LaggedRecord) -> Option<bool> {
    match rec.effect {
        firma_domain::Effect::Lobby { applied, .. }
        | firma_domain::Effect::Contract { applied, .. }
        | firma_domain::Effect::Diversify { applied, .. } => Some(applied),
        firma_domain::Effect::CapabilityGain { .. } => None,
    }
}

/// Incremental reconstruction of every firm's `(r^L, r^I, c, q, λ, u, W)` and
/// global `θ` from a stream of [`Event`]s, with no dependency on
/// `firma-kernel` (ADR 0045/0046) — a live tailer or a batch reader can drive
/// it identically, one [`Reconstruction::apply`] call per logged event.
#[derive(Debug, Clone)]
pub struct Reconstruction {
    firms: BTreeMap<u64, Firm>,
    theta: ConstraintParams,
    h_crit: f64,
    l_w: usize,
    scales: ScaleFactors,
    tick: u64,
    horizon: Option<u64>,
}

impl Reconstruction {
    /// Seed from a run's config: initial stocks/reals/ints, global θ, and the
    /// `decision.satisficing` / `constraint.action_window` parameters the
    /// running model used, so a reconstructed `h` matches the model's own
    /// (§9.2 `scales`).
    #[must_use]
    pub fn new(cfg: &RunConfig) -> Reconstruction {
        let mut firms = BTreeMap::new();
        for a in &cfg.agents {
            firms.insert(
                a.id,
                Firm {
                    r_l: a
                        .stocks
                        .get(&firma_core::ResourceKind(keys::CAPITAL.into()))
                        .copied()
                        .unwrap_or(0),
                    r_i: a
                        .stocks
                        .get(&firma_core::ResourceKind(keys::INPUT.into()))
                        .copied()
                        .unwrap_or(0),
                    c: a.reals.get(keys::CAPABILITY).copied().unwrap_or(0.0),
                    q: a.ints.get(keys::OBLIGATION).copied().unwrap_or(0),
                    lambda: a.reals.get(keys::LEGITIMACY).copied().unwrap_or(1.0),
                    window: Vec::new(),
                    action_counts: BTreeMap::new(),
                    alive: true,
                },
            );
        }
        let theta = ConstraintParams {
            theta_limit: cfg
                .world
                .global_reals
                .get(keys::THETA_LIMIT)
                .copied()
                .unwrap_or(0.0),
            theta_cap: cfg
                .world
                .global_reals
                .get(keys::THETA_CAP)
                .copied()
                .unwrap_or(0.0),
            theta_q: cfg
                .world
                .global_ints
                .get(keys::THETA_Q)
                .copied()
                .unwrap_or(0),
        };
        let h_crit = param_f64(cfg, "decision.satisficing", "h_crit").unwrap_or(0.15);
        let l_w = param_u64(cfg, "constraint.action_window", "l_w")
            .or_else(|| param_u64(cfg, "decision.satisficing", "l_w"))
            .unwrap_or(8) as usize;
        let scales: ScaleFactors = cfg
            .rules
            .iter()
            .find(|r| r.id.0 == "decision.satisficing")
            .and_then(|r| r.params.get("scales").cloned())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        Reconstruction {
            firms,
            theta,
            h_crit,
            l_w,
            scales,
            tick: 0,
            horizon: None,
        }
    }

    /// Fold one event, returning the [`Notable`] facts it represents — most
    /// raw `DeltaApplied` records are plumbing with nothing worth surfacing,
    /// so this is usually empty.
    pub fn apply(&mut self, ev: &Event) -> Vec<Notable> {
        match ev {
            Event::RunStarted {
                manual_version,
                engine_version,
                horizon,
            } => {
                self.horizon = Some(*horizon);
                vec![Notable::RunStarted {
                    manual_version: manual_version.clone(),
                    engine_version: engine_version.clone(),
                    horizon: *horizon,
                }]
            }
            Event::TickStarted { tick } => {
                self.tick = *tick;
                vec![Notable::TickStarted(*tick)]
            }
            Event::AgentBorn { tick, agent } => vec![Notable::AgentBorn {
                tick: *tick,
                agent: agent.0,
            }],
            Event::TickCompleted { tick, live_agents } => vec![Notable::TickCompleted {
                tick: *tick,
                live_agents: *live_agents,
            }],
            Event::AgentDied { tick, agent, cause } => {
                if let Some(f) = self.firms.get_mut(&agent.0) {
                    f.alive = false;
                }
                vec![Notable::AgentDied {
                    tick: *tick,
                    agent: agent.0,
                    cause: cause.clone(),
                }]
            }
            Event::InterventionApplied { .. } | Event::PhaseCompleted { .. } => Vec::new(),
            Event::DeltaApplied {
                tick,
                phase,
                origin,
                target,
                kind,
            } => self.apply_delta(*tick, *phase, &origin.0, target, kind),
        }
    }

    fn apply_delta(
        &mut self,
        tick: u64,
        phase: u8,
        origin: &str,
        target: &DeltaTarget,
        kind: &DeltaKind,
    ) -> Vec<Notable> {
        let mut out = Vec::new();
        match (target, kind) {
            (DeltaTarget::Agent(a), DeltaKind::AdjustStock { resource, amount }) => {
                if let Some(f) = self.firms.get_mut(&a.0) {
                    if resource.0 == keys::CAPITAL {
                        f.r_l += amount;
                    } else if resource.0 == keys::INPUT {
                        f.r_i += amount;
                    }
                }
            }
            (DeltaTarget::Agent(a), DeltaKind::AdjustAgentReal { field, delta }) => {
                if let Some(f) = self.firms.get_mut(&a.0) {
                    if field == keys::CAPABILITY {
                        f.c += delta;
                    } else if field == keys::LEGITIMACY {
                        f.lambda += delta;
                    }
                }
            }
            (DeltaTarget::Agent(a), DeltaKind::AdjustAgentInt { field, delta }) => {
                if field == keys::OBLIGATION {
                    if let Some(f) = self.firms.get_mut(&a.0) {
                        f.q += delta;
                    }
                }
            }
            (DeltaTarget::Global, DeltaKind::AdjustGlobalReal { field, delta }) => {
                if field == keys::THETA_LIMIT {
                    self.theta.theta_limit += delta;
                } else if field == keys::THETA_CAP {
                    self.theta.theta_cap += delta;
                }
                if origin.starts_with("shock.") {
                    out.push(Notable::Shock {
                        tick,
                        origin: origin.to_owned(),
                        field: field.clone(),
                    });
                }
            }
            (DeltaTarget::Global, DeltaKind::AdjustGlobalInt { field, delta }) => {
                if field == keys::THETA_Q {
                    self.theta.theta_q += delta;
                }
                if origin.starts_with("shock.") {
                    out.push(Notable::Shock {
                        tick,
                        origin: origin.to_owned(),
                        field: field.clone(),
                    });
                }
            }
            (DeltaTarget::Agent(a), DeltaKind::SetAgentInt { field, .. })
                if origin == "constraint.enforce"
                    && field == keys::COMPLIANCE_LAST_VIOLATION_TICK =>
            {
                out.push(Notable::ComplianceFirstStrike { tick, agent: a.0 });
            }
            (DeltaTarget::Agent(a), DeltaKind::SetAgentInt { field, value }) => {
                if origin.starts_with("decision.") && field == keys::SELECTED_ACTION {
                    out.push(Notable::Decision {
                        tick,
                        agent: a.0,
                        plugin: origin.to_owned(),
                        action: (*value).clamp(0, u8::MAX as i64) as u8,
                    });
                }
                if origin == "decision.satisficing" && field == keys::FOCUS {
                    out.push(Notable::Focus {
                        tick,
                        agent: a.0,
                        focus: *value,
                    });
                }
            }
            (DeltaTarget::Agent(a), DeltaKind::ReplaceAgentList { list, records_json }) => {
                if list == keys::ACTION_WINDOW {
                    if let Some(f) = self.firms.get_mut(&a.0) {
                        f.window = records_json
                            .iter()
                            .filter_map(|s| WindowEntry::from_json(s).ok())
                            .collect();
                        if let Some(last) = f.window.last() {
                            *f.action_counts.entry(last.action).or_default() += 1;
                        }
                    }
                }
            }
            (DeltaTarget::Agent(a), DeltaKind::PushAgentRecord { list, record_json })
                if list == keys::LAGGED_EFFECTS && phase == 5 =>
            {
                if let Ok(rec) = LaggedRecord::from_json(record_json) {
                    if let Some(applied) = shaping_applied(&rec) {
                        out.push(Notable::ShapingCommitted {
                            tick,
                            agent: a.0,
                            applied: Some(applied),
                        });
                    }
                }
            }
            _ => {}
        }
        out
    }

    /// Every firm's derived snapshot, ascending id order (`firms()` is
    /// `BTreeMap`-backed — deterministic).
    #[must_use]
    pub fn firms(&self) -> Vec<FirmSnapshot> {
        self.firms
            .iter()
            .map(|(&id, f)| self.snapshot(id, f))
            .collect()
    }

    /// One firm's derived snapshot (alive or dead — a dead firm keeps its
    /// last-known ledger for post-mortem display).
    #[must_use]
    pub fn firm(&self, id: u64) -> Option<FirmSnapshot> {
        self.firms.get(&id).map(|f| self.snapshot(id, f))
    }

    fn snapshot(&self, id: u64, f: &Firm) -> FirmSnapshot {
        let u = if f.window.is_empty() {
            0.0
        } else {
            margin::u_from_window(&f.window, self.l_w)
        };
        let state = FirmState {
            liquid_capital: f.r_l,
            input_stock: f.r_i,
            capability: f.c,
            obligation: f.q,
        };
        let aux = FirmAuxState {
            legitimacy: f.lambda,
            regulated_intensity: u,
            aspirations: Aspirations {
                capital_growth: 0.0,
                capability: 0.0,
                obligation_clearance: 0.0,
            },
        };
        let ctx = ConstraintContext {
            state: &state,
            aux: &aux,
            theta: &self.theta,
        };
        let h = margin::standard_margin(&ctx, &self.scales);
        let g = margin::all_g(&ctx);
        FirmSnapshot {
            id,
            alive: f.alive,
            liquid_capital: f.r_l,
            input_stock: f.r_i,
            capability: f.c,
            obligation: f.q,
            legitimacy: f.lambda,
            regulated_intensity: u,
            h,
            g,
            window: f.window.clone(),
            action_counts: f.action_counts.clone(),
        }
    }

    /// The most recently observed tick (`TickStarted`).
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// The run's declared horizon, once `RunStarted` has been seen.
    #[must_use]
    pub fn horizon(&self) -> Option<u64> {
        self.horizon
    }

    /// `h_crit` in force for this run.
    #[must_use]
    pub fn h_crit(&self) -> f64 {
        self.h_crit
    }

    /// Number of currently-live firms.
    #[must_use]
    pub fn live_count(&self) -> usize {
        self.firms.values().filter(|f| f.alive).count()
    }

    /// Total firms ever seen (initial population + births).
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.firms.len()
    }
}
