//! Live application state — folds tailed events through
//! [`firma_analysis::Reconstruction`] (the same reconstruction Part B
//! promoted for `firma-conformance` and `firma-py`) and keeps the small
//! amount of extra bookkeeping the panels need that a batch reader would
//! not (throughput, a shock timeline, a raw log tail).

use std::collections::{BTreeMap, VecDeque};
use std::time::Instant;

use firma_analysis::{Notable, Reconstruction};
use firma_config::RunConfig;
use firma_core::Event;

use crate::tail::{PolledLine, Tailer};

/// How many raw lines the "log tail" panel keeps.
const LOG_TAIL_LEN: usize = 12;
/// How many shock events the "shock timeline" panel keeps.
const SHOCK_TIMELINE_LEN: usize = 12;
/// No new event for this long ⇒ the "invariant status" panel calls it
/// stalled (a heuristic — a tailer has no direct signal for a writer's
/// invariant abort, only silence where activity was expected).
const STALL_WARNING: std::time::Duration = std::time::Duration::from_secs(3);

/// One shock-timeline entry.
#[derive(Debug, Clone)]
pub struct ShockEvent {
    /// The tick it fired.
    pub tick: u64,
    /// The shock plugin id.
    pub origin: String,
    /// The θ/price field it moved.
    pub field: String,
}

/// One death-list entry.
#[derive(Debug, Clone)]
pub struct DeathEvent {
    /// The tick.
    pub tick: u64,
    /// The departed agent.
    pub agent: u64,
    /// Why.
    pub cause: String,
}

/// The live monitor's whole state. `firma-tui` (this crate) MUST NOT link
/// `firma-kernel` (§23.2) — everything here is built from
/// `firma_analysis::Reconstruction`, which is itself kernel-free (ADR 0045).
pub struct App {
    recon: Reconstruction,
    tailer: Tailer,
    started_at: Instant,
    last_event_at: Instant,
    horizon: Option<u64>,
    finished: bool,
    action_totals: BTreeMap<u8, u64>,
    shock_timeline: VecDeque<ShockEvent>,
    deaths: Vec<DeathEvent>,
    raw_tail: VecDeque<String>,
    /// (wall-clock instant, tick) samples for a throughput estimate.
    throughput_samples: VecDeque<(Instant, u64)>,
    parse_errors: u64,
    should_quit: bool,
}

impl App {
    /// Build from a run's config (for the reconstruction's initial state and
    /// parameters — §9.2 `scales`, `h_crit`, `L_W`) and the path to its
    /// `events.ndjson`.
    #[must_use]
    pub fn new(cfg: &RunConfig, events_path: std::path::PathBuf) -> App {
        let now = Instant::now();
        App {
            recon: Reconstruction::new(cfg),
            tailer: Tailer::new(events_path),
            started_at: now,
            last_event_at: now,
            horizon: None,
            finished: false,
            action_totals: BTreeMap::new(),
            shock_timeline: VecDeque::new(),
            deaths: Vec::new(),
            raw_tail: VecDeque::new(),
            throughput_samples: VecDeque::new(),
            parse_errors: 0,
            should_quit: false,
        }
    }

    /// Pull whatever new lines are available and fold them in. Never errors
    /// on IO problems that just mean "nothing new yet" (`Tailer::poll`
    /// already absorbs those); a genuine IO error is logged to the raw-tail
    /// panel rather than crashing the monitor — a dashboard should degrade,
    /// not disappear.
    pub fn poll(&mut self) {
        let lines = match self.tailer.poll() {
            Ok(l) => l,
            Err(e) => {
                self.push_raw(format!("[tail error: {e}]"));
                Vec::new()
            }
        };
        for line in lines {
            match line {
                PolledLine::Parsed(ev) => self.apply(*ev),
                PolledLine::Unparsed(raw) => {
                    self.parse_errors += 1;
                    self.push_raw(format!("[unparsed: {raw}]"));
                }
            }
        }
    }

    fn apply(&mut self, ev: Event) {
        self.last_event_at = Instant::now();
        self.push_raw(raw_summary(&ev));
        for n in self.recon.apply(&ev) {
            match n {
                Notable::RunStarted { horizon, .. } => self.horizon = Some(horizon),
                Notable::TickCompleted { tick, .. } => {
                    self.throughput_samples.push_back((Instant::now(), tick));
                    while self.throughput_samples.len() > 20 {
                        self.throughput_samples.pop_front();
                    }
                    if let Some(h) = self.horizon {
                        if tick + 1 >= h {
                            self.finished = true;
                        }
                    }
                }
                Notable::AgentDied { tick, agent, cause } => {
                    self.deaths.push(DeathEvent { tick, agent, cause });
                }
                Notable::Decision { action, .. } => {
                    *self.action_totals.entry(action).or_default() += 1;
                }
                Notable::Shock {
                    tick,
                    origin,
                    field,
                } => {
                    self.shock_timeline.push_back(ShockEvent {
                        tick,
                        origin,
                        field,
                    });
                    while self.shock_timeline.len() > SHOCK_TIMELINE_LEN {
                        self.shock_timeline.pop_front();
                    }
                }
                _ => {}
            }
        }
    }

    fn push_raw(&mut self, line: String) {
        self.raw_tail.push_back(line);
        while self.raw_tail.len() > LOG_TAIL_LEN {
            self.raw_tail.pop_front();
        }
    }

    /// Request the event loop stop.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Whether the event loop should stop.
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// The reconstruction (for the margin-distribution / per-firm panels).
    #[must_use]
    pub fn reconstruction(&self) -> &Reconstruction {
        &self.recon
    }

    /// Elapsed wall-clock time since this monitor started watching.
    #[must_use]
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Declared horizon, once seen.
    #[must_use]
    pub fn horizon(&self) -> Option<u64> {
        self.horizon
    }

    /// Whether the run has reached its horizon.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Ticks/second, estimated from the last few `TickCompleted` samples'
    /// wall-clock spacing. `None` until at least two samples exist.
    #[must_use]
    pub fn throughput(&self) -> Option<f64> {
        let (t0, tick0) = *self.throughput_samples.front()?;
        let (t1, tick1) = *self.throughput_samples.back()?;
        if tick1 <= tick0 {
            return None;
        }
        let dt = t1.duration_since(t0).as_secs_f64();
        if dt <= 0.0 {
            return None;
        }
        Some((tick1 - tick0) as f64 / dt)
    }

    /// A rough ETA to the horizon, from the current throughput estimate.
    #[must_use]
    pub fn eta(&self) -> Option<std::time::Duration> {
        let horizon = self.horizon?;
        let tick = self.recon.tick();
        if tick + 1 >= horizon {
            return Some(std::time::Duration::ZERO);
        }
        let rate = self.throughput()?;
        if rate <= 0.0 {
            return None;
        }
        let remaining = (horizon - tick - 1) as f64;
        Some(std::time::Duration::from_secs_f64(remaining / rate))
    }

    /// "OK" unless nothing has happened for a while and the run has not
    /// finished — the only signal a tailing reader has for a writer that
    /// aborted (§19.5 invariant violations are fatal, so a stalled log is
    /// exactly what one would look like from the outside).
    #[must_use]
    pub fn invariant_status(&self) -> &'static str {
        if self.finished {
            "OK — run complete"
        } else if self.horizon.is_none() {
            "waiting for RunStarted"
        } else if self.last_event_at.elapsed() > STALL_WARNING {
            "STALLED — no new events (possible abort)"
        } else {
            "OK — progressing"
        }
    }

    /// Cumulative action-selection tally across the whole population.
    #[must_use]
    pub fn action_totals(&self) -> &BTreeMap<u8, u64> {
        &self.action_totals
    }

    /// The most recent shock events, oldest first.
    #[must_use]
    pub fn shock_timeline(&self) -> &VecDeque<ShockEvent> {
        &self.shock_timeline
    }

    /// Every death seen so far.
    #[must_use]
    pub fn deaths(&self) -> &[DeathEvent] {
        &self.deaths
    }

    /// The last few raw log lines.
    #[must_use]
    pub fn raw_tail(&self) -> &VecDeque<String> {
        &self.raw_tail
    }

    /// Lines that failed to parse as JSON (should stay `0` for a healthy
    /// log).
    #[must_use]
    pub fn parse_errors(&self) -> u64 {
        self.parse_errors
    }
}

fn raw_summary(ev: &Event) -> String {
    // A short, human-scannable line for the "log tail" panel — not the raw
    // JSON (too wide for a narrow panel), but every field that matters.
    match ev {
        Event::RunStarted {
            horizon,
            engine_version,
            ..
        } => {
            format!("RunStarted engine={engine_version} horizon={horizon}")
        }
        Event::TickStarted { tick } => format!("tick {tick} started"),
        Event::AgentBorn { tick, agent } => format!("t{tick}: agent {} born", agent.0),
        Event::InterventionApplied { tick, op } => format!("t{tick}: intervention {op}"),
        Event::DeltaApplied {
            tick, origin, kind, ..
        } => {
            format!("t{tick}: {} -> {}", origin.0, delta_kind_name(kind))
        }
        Event::PhaseCompleted {
            tick,
            phase,
            deltas_applied,
        } => {
            format!("t{tick}: phase {phase} done ({deltas_applied} deltas)")
        }
        Event::AgentDied { tick, agent, cause } => {
            format!("t{tick}: agent {} died ({cause})", agent.0)
        }
        Event::TickCompleted { tick, live_agents } => {
            format!("tick {tick} complete, {live_agents} live")
        }
    }
}

fn delta_kind_name(kind: &firma_core::DeltaKind) -> &'static str {
    use firma_core::DeltaKind;
    match kind {
        DeltaKind::AdjustStock { .. } => "AdjustStock",
        DeltaKind::SetAgentInt { .. } => "SetAgentInt",
        DeltaKind::AdjustAgentReal { .. } => "AdjustAgentReal",
        DeltaKind::AdjustAgentInt { .. } => "AdjustAgentInt",
        DeltaKind::AdjustGlobalReal { .. } => "AdjustGlobalReal",
        DeltaKind::AdjustGlobalInt { .. } => "AdjustGlobalInt",
        DeltaKind::PushAgentRecord { .. } => "PushAgentRecord",
        DeltaKind::PushGlobalRecord { .. } => "PushGlobalRecord",
        DeltaKind::ReplaceAgentList { .. } => "ReplaceAgentList",
        DeltaKind::RemoveAgent { .. } => "RemoveAgent",
        DeltaKind::ReplaceGlobalList { .. } => "ReplaceGlobalList",
    }
}
