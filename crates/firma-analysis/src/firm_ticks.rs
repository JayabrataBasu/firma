//! Per-firm-per-tick reconstruction for offline analysis (review R14):
//! margin `h` and the decision-time shortfalls for every *live firm-tick* of
//! a run, read from the event log.
//!
//! A **live firm-tick** is a `(tick, firm)` pair in which the firm made a
//! decision and is still alive when the tick completes — the same unit
//! [`crate::sanity`] counts for SC-2/SC-6. For each one this module records:
//!
//! * `shortfalls` / `shortfall_max` — the firm's `[ς_1, ς_2, ς_3]` and
//!   `max_j ς_j` **as of its decision in that tick** (the state the fold has
//!   reached when the tick's `decision.*` record appears: everything up to
//!   and including phases 1–2 of that tick, nothing later). See
//!   [`Reconstruction::shortfalls`] for the formula and its limits.
//! * `h` — the reconstructed viability margin (§9.2, true `θ`) **at the end
//!   of that tick**, exactly as [`crate::sanity`] samples it. It is *not* the
//!   margin the firm perceived at decision time (which uses the observed
//!   `θ`, ADR 0035).
//! * `action` — the action selected in that tick.

use std::collections::BTreeMap;

use firma_config::RunConfig;
use firma_core::Event;
use serde::Serialize;

use crate::reconstruct::{max_shortfall, Notable, Reconstruction};
use crate::sanity::refuse_parse_errors;

/// One live firm-tick. See the module docs for what each field is sampled at.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FirmTick {
    /// The tick.
    pub tick: u64,
    /// The firm.
    pub agent: u64,
    /// The action selected in this tick (§11 canonical index).
    pub action: u8,
    /// Reconstructed margin `h` at the end of the tick.
    pub h: f64,
    /// `[ς_1, ς_2, ς_3]` at the firm's decision in this tick.
    pub shortfalls: [f64; 3],
    /// `max_j ς_j` of the above ([`max_shortfall`]).
    pub shortfall_max: f64,
}

/// Every live firm-tick of a run, in `(tick, agent)` order.
///
/// # Errors
/// If the config's `scales` parameter is malformed
/// ([`Reconstruction::try_new`]) or the log contains an unparsable persisted
/// record ([`Reconstruction::parse_errors`]) — the same refusals as
/// [`crate::sanity::reconstruct_checked`].
pub fn firm_ticks<'a>(
    cfg: &RunConfig,
    events: impl IntoIterator<Item = &'a Event>,
) -> Result<Vec<FirmTick>, String> {
    let mut r = Reconstruction::try_new(cfg)?;
    let mut out = Vec::new();
    // Decisions of the tick in progress: agent -> (action, decision-time ς).
    let mut pending: BTreeMap<u64, (u8, [f64; 3])> = BTreeMap::new();
    for ev in events {
        for n in r.apply(ev) {
            match n {
                Notable::TickStarted(_) => pending.clear(),
                Notable::Decision { agent, action, .. } => {
                    if let Some(sc) = r.shortfalls(agent) {
                        pending.insert(agent, (action, sc));
                    }
                }
                Notable::TickCompleted { tick, .. } => {
                    for (&agent, &(action, sc)) in &pending {
                        let Some(f) = r.firm(agent) else { continue };
                        if !f.alive {
                            continue; // died this tick: not a live firm-tick
                        }
                        out.push(FirmTick {
                            tick,
                            agent,
                            action,
                            h: f.h,
                            shortfalls: sc,
                            shortfall_max: max_shortfall(sc),
                        });
                    }
                    pending.clear();
                }
                _ => {}
            }
        }
    }
    refuse_parse_errors(&r)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use firma_core::{AgentId, DeltaKind, DeltaTarget, PluginId};
    use firma_domain::keys;

    fn cfg() -> RunConfig {
        RunConfig::from_json(
            r#"{
              "experiment": "unit", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
              "seeds": {"mechanism": 910001, "environment": 910002, "shock": 910003, "init": 910004},
              "world": {"ticks": 3, "resources": ["capital", "input"],
                "conflict_resolver": {"id": "conflict.additive", "version": "^1"},
                "global_reals": {"theta_limit": 0.9, "theta_cap": 0.2},
                "global_ints": {"theta_q": 100}},
              "agents": [
                {"id": 0, "stocks": {"capital": 100}, "reals": {"capability": 0.6, "aspiration_capital_growth": 5.0}},
                {"id": 1, "stocks": {"capital": 100}, "reals": {"capability": 0.6}}
              ],
              "rules": [{"id": "constraint.action_window", "version": "^1", "params": {"l_w": 4}}]
            }"#,
        )
        .unwrap()
    }

    fn delta(tick: u64, phase: u8, origin: &str, agent: u64, kind: DeltaKind) -> Event {
        Event::DeltaApplied {
            tick,
            phase,
            origin: PluginId::new(origin),
            target: DeltaTarget::Agent(AgentId(agent)),
            kind,
        }
    }

    fn decide(tick: u64, agent: u64, action: i64) -> Event {
        delta(
            tick,
            3,
            "decision.satisficing",
            agent,
            DeltaKind::SetAgentInt {
                field: keys::SELECTED_ACTION.to_owned(),
                value: action,
            },
        )
    }

    fn growth(tick: u64, agent: u64, v: i64) -> Event {
        delta(
            tick,
            9,
            "decision.aspiration_update",
            agent,
            DeltaKind::SetAgentInt {
                field: keys::REALIZED_CAPITAL_GROWTH.to_owned(),
                value: v,
            },
        )
    }

    fn tick_end(tick: u64, live: u32) -> Event {
        Event::TickCompleted {
            tick,
            live_agents: live,
        }
    }

    /// Two firms; firm 1 dies in tick 1. Realised growth is written in the
    /// `record` phase of tick 0, i.e. *after* tick 0's decisions.
    fn log() -> Vec<Event> {
        vec![
            Event::TickStarted { tick: 0 },
            decide(0, 0, 2),
            decide(0, 1, 1),
            growth(0, 0, 2),
            tick_end(0, 2),
            Event::TickStarted { tick: 1 },
            decide(1, 0, 1),
            decide(1, 1, 1),
            Event::AgentDied {
                tick: 1,
                agent: AgentId(1),
                cause: "solvency".into(),
            },
            tick_end(1, 1),
            Event::TickStarted { tick: 2 },
            decide(2, 0, 0),
            tick_end(2, 1),
        ]
    }

    #[test]
    fn one_record_per_live_firm_tick_and_the_death_tick_is_excluded() {
        let ft = firm_ticks(&cfg(), &log()).unwrap();
        let ids: Vec<(u64, u64)> = ft.iter().map(|f| (f.tick, f.agent)).collect();
        // firm 1 decided in tick 1 but died in it: no (1, 1) record, and none after.
        assert_eq!(ids, vec![(0, 0), (0, 1), (1, 0), (2, 0)]);
        assert_eq!(ft[0].action, 2);
    }

    #[test]
    fn shortfall_is_the_value_at_the_decision_not_after_the_record_phase() {
        let ft = firm_ticks(&cfg(), &log()).unwrap();
        // Tick 0: A_1 = 5, realised growth not yet written (reads 0): ς_1 = 5.
        assert_eq!(ft[0].shortfalls, [5.0, 0.0, 0.0]);
        assert_eq!(ft[0].shortfall_max, 5.0);
        // Tick 1: growth 2 was written in tick 0's record phase: ς_1 = 3.
        assert_eq!(ft[2].shortfalls, [3.0, 0.0, 0.0]);
        // Firm 1 has no aspiration at all: every shortfall is 0.
        assert_eq!(ft[1].shortfalls, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn h_is_the_reconstructed_margin_at_the_end_of_the_tick() {
        let ft = firm_ticks(&cfg(), &log()).unwrap();
        let mut r = Reconstruction::new(&cfg());
        for ev in &log()[..5] {
            r.apply(ev);
        }
        assert_eq!(ft[0].h, r.firm(0).unwrap().h);
    }

    #[test]
    fn a_corrupt_record_is_refused() {
        let mut evs = log();
        evs.insert(
            4,
            delta(
                0,
                7,
                "constraint.action_window",
                0,
                DeltaKind::ReplaceAgentList {
                    list: keys::ACTION_WINDOW.to_owned(),
                    records_json: vec![r#"{"tick":"CORRUPT-RECORD"}"#.to_owned()],
                },
            ),
        );
        let Err(err) = firm_ticks(&cfg(), &evs) else {
            panic!("a corrupt record must not yield firm-ticks");
        };
        assert!(err.contains("CORRUPT-RECORD"), "{err}");
    }
}
