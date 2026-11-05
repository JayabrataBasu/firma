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
    /// Aspirations `A_1..A_3` (§12.1), `None` while a goal's aspiration has
    /// never been seeded or written — the model reads an absent aspiration as
    /// "no shortfall for that goal yet".
    aspirations: [Option<f64>; 3],
    /// `v_1`'s stored value, `realized_capital_growth` (written in `record`);
    /// `None` until first written — the model reads absent as `0`.
    realized_growth: Option<i64>,
}

/// The three per-goal aspiration keys, in §12.1 goal order.
const ASPIRATION_KEYS: [&str; 3] = [
    keys::ASPIRATION_CAPITAL_GROWTH,
    keys::ASPIRATION_CAPABILITY,
    keys::ASPIRATION_OBLIGATION_CLEARANCE,
];

/// `max_j ς_j`, reduced exactly as `decision.satisficing`'s `Focus::attend`
/// reduces the three shortfalls (first index wins a tie; the value at that
/// index is the maximum).
#[must_use]
pub fn max_shortfall(shortfalls: [f64; 3]) -> f64 {
    let mut best = 0usize;
    for j in 1..3 {
        if shortfalls[j] > shortfalls[best] {
            best = j;
        }
    }
    shortfalls[best]
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
    parse_errors: Vec<String>,
}

impl Reconstruction {
    /// Seed from a run's config: initial stocks/reals/ints, global θ, and the
    /// `decision.satisficing` / `constraint.action_window` parameters the
    /// running model used, so a reconstructed `h` matches the model's own
    /// (§9.2 `scales`).
    ///
    /// # Panics
    /// If the config's `decision.satisficing` `scales` parameter is present
    /// but malformed — see [`Reconstruction::try_new`], which returns that as
    /// an error instead. This infallible form exists for callers that
    /// cannot propagate one; it never falls back to default scales.
    #[must_use]
    pub fn new(cfg: &RunConfig) -> Reconstruction {
        Self::try_new(cfg).unwrap_or_else(|e| panic!("{e}"))
    }

    /// As [`Reconstruction::new`], returning an error instead of panicking.
    ///
    /// # Errors
    /// If `decision.satisficing` carries a `scales` parameter that is not a
    /// valid [`ScaleFactors`]. An **absent** `scales` parameter (or no
    /// `decision.satisficing` rule at all) means the §9.2 defaults, exactly
    /// as the running model applies them; a present-but-malformed one would
    /// make every reconstructed `h` silently disagree with the model's, so
    /// it is refused.
    pub fn try_new(cfg: &RunConfig) -> Result<Reconstruction, String> {
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
                    aspirations: ASPIRATION_KEYS.map(|k| a.reals.get(k).copied()),
                    realized_growth: a.ints.get(keys::REALIZED_CAPITAL_GROWTH).copied(),
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
        let scales: ScaleFactors = match cfg
            .rules
            .iter()
            .find(|r| r.id.0 == "decision.satisficing")
            .and_then(|r| r.params.get("scales").cloned())
        {
            None => ScaleFactors::default(),
            Some(v) => serde_json::from_value(v.clone()).map_err(|e| {
                format!(
                    "decision.satisficing `scales` parameter is not a valid ScaleFactors \
                     ({e}); offending value: {v}. Refusing to reconstruct with default scales."
                )
            })?,
        };
        Ok(Reconstruction {
            firms,
            theta,
            h_crit,
            l_w,
            scales,
            tick: 0,
            horizon: None,
            parse_errors: Vec::new(),
        })
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
                    } else if let Some(j) = ASPIRATION_KEYS.iter().position(|k| field == k) {
                        // The store treats an absent real as 0 and the field
                        // exists from the first adjustment on.
                        f.aspirations[j] = Some(f.aspirations[j].unwrap_or(0.0) + delta);
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
                if field == keys::REALIZED_CAPITAL_GROWTH {
                    if let Some(f) = self.firms.get_mut(&a.0) {
                        f.realized_growth = Some(*value);
                    }
                }
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
                        // An unparsable entry is recorded in `parse_errors`
                        // (agent, key, offending string), not dropped without
                        // trace: this is offline analysis, so it reports the
                        // problem rather than aborting the reader.
                        let mut window = Vec::with_capacity(records_json.len());
                        for s in records_json {
                            match WindowEntry::from_json(s) {
                                Ok(e) => window.push(e),
                                Err(err) => self.parse_errors.push(format!(
                                    "tick {tick}, agent {}: corrupt record under `{}` \
                                     ({err}); offending record: {s:?}",
                                    a.0,
                                    keys::ACTION_WINDOW
                                )),
                            }
                        }
                        f.window = window;
                        if let Some(last) = f.window.last() {
                            *f.action_counts.entry(last.action).or_default() += 1;
                        }
                    }
                }
            }
            (DeltaTarget::Agent(a), DeltaKind::PushAgentRecord { list, record_json })
                if list == keys::LAGGED_EFFECTS && phase == 5 =>
            {
                match LaggedRecord::from_json(record_json) {
                    Ok(rec) => {
                        if let Some(applied) = shaping_applied(&rec) {
                            out.push(Notable::ShapingCommitted {
                                tick,
                                agent: a.0,
                                applied: Some(applied),
                            });
                        }
                    }
                    // Counted, not skipped without trace: an unparsable
                    // record here would otherwise silently lower the
                    // shaping-commit count (SC-4/SC-5).
                    Err(err) => self.parse_errors.push(format!(
                        "tick {tick}, agent {}: corrupt record under `{}` ({err}); \
                         offending record: {record_json:?}",
                        a.0,
                        keys::LAGGED_EFFECTS
                    )),
                }
            }
            _ => {}
        }
        out
    }

    /// A firm's three shortfalls `[ς_1, ς_2, ς_3]` (§12.1: `ς_j = A_j − v_j`)
    /// from the state reconstructed so far, or `None` for an unknown firm.
    ///
    /// The same arithmetic `decision.satisficing` performs at `decide`
    /// (`Satisficing::shortfalls`, `firma-plugin-decision`): `v_1` is the
    /// stored `realized_capital_growth` (absent ⇒ `0`), `v_2 = c`,
    /// `v_3 = −q`; an **absent** aspiration gives `ς_j = 0`. This crate may
    /// not depend on a plugin (ADR 0045), so the formula is restated here
    /// from persisted state rather than called; equality with the plugin's
    /// own decision-time values is checked against its diagnostic trace
    /// (review R14). Called while folding a tick's `decision.*` record, it
    /// returns the values that firm's decision used.
    ///
    /// Not covered: Arm-A pins (`pinned_shortfall_*`, ADR 0054), which
    /// replace these values inside the plugin and are not reconstructed.
    #[must_use]
    pub fn shortfalls(&self, id: u64) -> Option<[f64; 3]> {
        let f = self.firms.get(&id)?;
        let v = [f.realized_growth.unwrap_or(0) as f64, f.c, -(f.q as f64)];
        let mut sc = [0.0; 3];
        for j in 0..3 {
            if let Some(a) = f.aspirations[j] {
                sc[j] = a - v[j];
            }
        }
        Some(sc)
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

    /// Persisted records in the event log that could not be parsed, one
    /// message per record, naming the agent, the list key and the offending
    /// string. Empty for a well-formed log. A non-empty list means every
    /// quantity derived from that list (`u`, `h`, repertoire entropy, action
    /// counts) was computed without the unparsable entries and must not be
    /// trusted — check this before using a reconstruction.
    #[must_use]
    pub fn parse_errors(&self) -> &[String] {
        &self.parse_errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use firma_core::{AgentId, DeltaKind, DeltaTarget, Event, PluginId};

    fn recon() -> Reconstruction {
        Reconstruction::new(&recon_cfg())
    }

    fn recon_cfg() -> RunConfig {
        RunConfig::from_json(
            r#"{
              "experiment": "unit", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
              "seeds": {"mechanism": 910001, "environment": 910002, "shock": 910003, "init": 910004},
              "world": {"ticks": 10, "resources": ["capital", "input"],
                "conflict_resolver": {"id": "conflict.additive", "version": "^1"}},
              "agents": [{"id": 0, "stocks": {"capital": 100}}],
              "rules": [{"id": "constraint.action_window", "version": "^1", "params": {"l_w": 4}}]
            }"#,
        )
        .unwrap()
    }

    fn window_event(records: Vec<String>) -> Event {
        Event::DeltaApplied {
            tick: 3,
            phase: 7,
            origin: PluginId::new("constraint.action_window"),
            target: DeltaTarget::Agent(AgentId(0)),
            kind: DeltaKind::ReplaceAgentList {
                list: keys::ACTION_WINDOW.to_owned(),
                records_json: records,
            },
        }
    }

    fn cfg_with_scales(scales: &str) -> RunConfig {
        RunConfig::from_json(&format!(
            r#"{{
              "experiment": "unit", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
              "seeds": {{"mechanism": 910001, "environment": 910002, "shock": 910003, "init": 910004}},
              "world": {{"ticks": 10, "resources": ["capital", "input"],
                "conflict_resolver": {{"id": "conflict.additive", "version": "^1"}}}},
              "agents": [{{"id": 0, "stocks": {{"capital": 100}}}}],
              "rules": [{{"id": "decision.satisficing", "version": "^1", "params": {{"l_w": 4{scales}}}}}]
            }}"#
        ))
        .unwrap()
    }

    #[test]
    fn absent_scales_param_still_means_defaults() {
        assert!(Reconstruction::try_new(&cfg_with_scales("")).is_ok());
        assert!(Reconstruction::try_new(&recon_cfg()).is_ok());
    }

    #[test]
    fn well_formed_scales_param_is_accepted() {
        let json = serde_json::to_string(&ScaleFactors::default()).unwrap();
        assert!(
            Reconstruction::try_new(&cfg_with_scales(&format!(r#", "scales": {json}"#))).is_ok()
        );
    }

    #[test]
    fn malformed_scales_param_is_an_error_not_silent_defaults() {
        let Err(err) = Reconstruction::try_new(&cfg_with_scales(
            r#", "scales": {"s_l": "MALFORMED-SCALE"}"#,
        )) else {
            panic!("a malformed `scales` param must be an error, not ScaleFactors::default()");
        };
        for p in ["decision.satisficing", "scales", "MALFORMED-SCALE"] {
            assert!(err.contains(p), "error does not name {p:?}: {err}");
        }
    }

    #[test]
    #[should_panic(expected = "MALFORMED-SCALE")]
    fn infallible_constructor_panics_on_a_malformed_scales_param() {
        let _ = Reconstruction::new(&cfg_with_scales(
            r#", "scales": {"s_l": "MALFORMED-SCALE"}"#,
        ));
    }

    #[test]
    fn one_corrupt_lagged_record_is_counted_and_named_not_silently_skipped() {
        let mut r = recon();
        r.apply(&Event::DeltaApplied {
            tick: 6,
            phase: 5,
            origin: PluginId::new("action.shaping.rdt_standard.lobby"),
            target: DeltaTarget::Agent(AgentId(0)),
            kind: DeltaKind::PushAgentRecord {
                list: keys::LAGGED_EFFECTS.to_owned(),
                record_json: r#"{"maturity_tick":"CORRUPT-RECORD"}"#.to_owned(),
            },
        });
        assert_eq!(
            r.parse_errors().len(),
            1,
            "one corrupt lagged record ⇒ one error"
        );
        let msg = &r.parse_errors()[0];
        for p in ["agent 0", keys::LAGGED_EFFECTS, "CORRUPT-RECORD", "tick 6"] {
            assert!(msg.contains(p), "error does not name {p:?}: {msg}");
        }
    }

    // --- shortfall (review R14) ---

    fn cfg_with_agent(reals: &str, ints: &str) -> RunConfig {
        RunConfig::from_json(&format!(
            r#"{{
              "experiment": "unit", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
              "seeds": {{"mechanism": 910001, "environment": 910002, "shock": 910003, "init": 910004}},
              "world": {{"ticks": 10, "resources": ["capital", "input"],
                "conflict_resolver": {{"id": "conflict.additive", "version": "^1"}}}},
              "agents": [{{"id": 0, "stocks": {{"capital": 100}}, "reals": {{{reals}}}, "ints": {{{ints}}}}}],
              "rules": [{{"id": "constraint.action_window", "version": "^1", "params": {{"l_w": 4}}}}]
            }}"#
        ))
        .unwrap()
    }

    fn agent_delta(origin: &str, phase: u8, kind: DeltaKind) -> Event {
        Event::DeltaApplied {
            tick: 3,
            phase,
            origin: PluginId::new(origin),
            target: DeltaTarget::Agent(AgentId(0)),
            kind,
        }
    }

    fn set_growth(v: i64) -> Event {
        agent_delta(
            "decision.aspiration_update",
            9,
            DeltaKind::SetAgentInt {
                field: keys::REALIZED_CAPITAL_GROWTH.to_owned(),
                value: v,
            },
        )
    }

    fn adjust_real(field: &str, delta: f64) -> Event {
        agent_delta(
            "decision.aspiration_update",
            9,
            DeltaKind::AdjustAgentReal {
                field: field.to_owned(),
                delta,
            },
        )
    }

    #[test]
    fn shortfalls_match_the_manual_12_1_table_on_the_15_3_numbers() {
        // Manual §15.3: A_1 = 5, A_2 = 0.6, A_3 = −20. With realised growth
        // v_1 = 2, capability c = 0.60 and obligation q = 3:
        //   ς_1 = 5 − 2 = 3;  ς_2 = 0.6 − 0.60 = 0;  ς_3 = −20 − (−3) = −17.
        let mut r = Reconstruction::new(&cfg_with_agent(
            r#""capability": 0.60, "aspiration_capital_growth": 5.0, "aspiration_capability": 0.6, "aspiration_obligation_clearance": -20.0"#,
            r#""obligation": 3"#,
        ));
        r.apply(&set_growth(2));
        let sc = r.shortfalls(0).unwrap();
        assert!((sc[0] - 3.0).abs() < 1e-12, "{sc:?}");
        assert!(sc[1].abs() < 1e-12, "{sc:?}");
        assert!((sc[2] + 17.0).abs() < 1e-12, "{sc:?}");
        assert!((max_shortfall(sc) - 3.0).abs() < 1e-12);

        // §15.3's aspiration update after tick 3: A_1 = 5 + 0.1(2 − 5) = 4.7,
        // logged by `decision.aspiration_update` as a delta of −0.3.
        r.apply(&adjust_real(
            keys::ASPIRATION_CAPITAL_GROWTH,
            0.1 * (2.0 - 5.0),
        ));
        let sc = r.shortfalls(0).unwrap();
        assert!((sc[0] - (4.7 - 2.0)).abs() < 1e-12, "{sc:?}");
    }

    #[test]
    fn an_absent_aspiration_gives_zero_shortfall_until_it_is_first_written() {
        let mut r = Reconstruction::new(&cfg_with_agent(r#""capability": 0.4"#, ""));
        assert_eq!(r.shortfalls(0).unwrap(), [0.0, 0.0, 0.0]);
        // Absent realised growth reads as 0: seeding A_1 := 7 gives ς_1 = 7.
        r.apply(&adjust_real(keys::ASPIRATION_CAPITAL_GROWTH, 7.0));
        assert_eq!(r.shortfalls(0).unwrap(), [7.0, 0.0, 0.0]);
        r.apply(&set_growth(9));
        assert_eq!(r.shortfalls(0).unwrap(), [-2.0, 0.0, 0.0]);
        // Capability aspiration seeded at 0.4 (the firm's own c): ς_2 = 0.
        r.apply(&adjust_real(keys::ASPIRATION_CAPABILITY, 0.4));
        assert_eq!(r.shortfalls(0).unwrap()[1], 0.0);
        assert!(r.shortfalls(99).is_none());
    }

    #[test]
    fn max_shortfall_is_the_value_attend_reads() {
        assert_eq!(max_shortfall([1.0, 3.0, 2.0]), 3.0);
        assert_eq!(max_shortfall([3.0, 3.0, 2.0]), 3.0);
        assert_eq!(max_shortfall([-5.0, -1.0, -2.0]), -1.0);
        assert_eq!(max_shortfall([0.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn a_well_formed_window_reports_no_parse_error() {
        let mut r = recon();
        r.apply(&window_event(vec![
            WindowEntry::new(1, 2).to_json(),
            WindowEntry::new(2, 1).to_json(),
        ]));
        assert!(r.parse_errors().is_empty());
        assert_eq!(r.firm(0).unwrap().window.len(), 2);
    }

    #[test]
    fn one_corrupt_window_record_is_counted_and_named_not_silently_dropped() {
        let mut r = recon();
        let corrupt = r#"{"tick":"CORRUPT-RECORD"}"#;
        r.apply(&window_event(vec![
            WindowEntry::new(1, 2).to_json(),
            corrupt.to_owned(),
            WindowEntry::new(2, 1).to_json(),
        ]));
        assert_eq!(
            r.parse_errors().len(),
            1,
            "exactly one corrupt record ⇒ one error"
        );
        let msg = &r.parse_errors()[0];
        for p in ["agent 0", keys::ACTION_WINDOW, "CORRUPT-RECORD", "tick 3"] {
            assert!(msg.contains(p), "error does not name {p:?}: {msg}");
        }
    }
}
