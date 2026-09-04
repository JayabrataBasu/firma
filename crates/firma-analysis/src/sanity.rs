//! SC-1…SC-6 (manual §16.2) computed from a completed run's event log by
//! folding [`crate::Reconstruction`] over it (§17 A5: measurement is
//! offline). Promoted from `tests/src/replay.rs` (ADR 0046) — the batch
//! entry point `firma-conformance`'s tests and `firma-py`'s `metrics`
//! binding both call.

use std::path::Path;

use firma_config::RunConfig;
use firma_core::Event;
use serde::Serialize;

use crate::reconstruct::{repertoire_entropy, Notable, Reconstruction};

/// The SC-1…SC-6 outcome for one run. `Serialize` (Stage 7 Part D) is for
/// the Rust/Python cross-check only (`examples/sanity_report_json.rs`) —
/// nothing in the sim path reads this back.
#[derive(Debug, Clone, Serialize)]
pub struct SanityReport {
    /// n firms at `t = 0`.
    pub initial_firms: usize,
    /// n firms alive at the horizon.
    pub survivors: usize,
    /// **SC-1** — survivor fraction. Target 0.60–0.90.
    pub sc1_survival: f64,
    /// **SC-2** — fraction of firm-decide-ticks with `focus == SURVIVAL`
    /// (⇔ `h < h_crit`), read from the logged `focus`. Target 0.05–0.25.
    pub sc2_survival_attention: f64,
    /// SC-2 cross-check: fraction of (firm, end-of-tick) with reconstructed
    /// `h < h_crit`.
    pub sc2_reconstructed_h_below_crit: f64,
    /// **SC-3** — `[solvency, compliance, scope, obligation]` each bound at
    /// least once (reconstructed `g_j` threshold). All four must be `true`.
    pub sc3_binds: [bool; 4],
    /// SC-3 first tick each constraint bound (for the report).
    pub sc3_first_bind_tick: [Option<u64>; 4],
    /// **SC-4** — shaping attempts (`selected_action ∈ {6,7,8}`) as a fraction
    /// of all logged decisions. Target `> 0.05`.
    pub sc4_shaping_fraction: f64,
    /// **SC-5** — matured-shaping success rate (`Effect.applied == true`).
    /// `None` if no shaping was ever committed. Target 0.10–0.60.
    pub sc5_shaping_success: Option<f64>,
    /// **SC-6** — variance of repertoire entropy `H_rep`
    /// (`−Σ p_a log₂ p_a` over the trailing window `W`) across all
    /// (firm, tick). Target: non-degenerate (`> SC6_MIN_VARIANCE`).
    pub sc6_entropy_variance: f64,
    /// SC-6 mean entropy (context for the variance).
    pub sc6_entropy_mean: f64,
    /// Total logged decisions (denominator for SC-4).
    pub decisions: usize,
    /// Shaping commits seen (denominator for SC-5).
    pub shaping_commits: usize,
}

/// "Non-degenerate" for SC-6: the entropy distribution has variance at least
/// this. `0.01 bits²` ⇒ a standard deviation of `0.1 bits` — a firm-tick
/// population where repertoire concentration genuinely varies, not one where
/// every firm-tick has the same entropy.
pub const SC6_MIN_VARIANCE: f64 = 0.01;

impl SanityReport {
    /// Whether all six conditions hold.
    #[must_use]
    pub fn all_pass(&self) -> bool {
        self.sc1_pass()
            && self.sc2_pass()
            && self.sc3_pass()
            && self.sc4_pass()
            && self.sc5_pass()
            && self.sc6_pass()
    }
    /// SC-1: 0.60 ≤ survival ≤ 0.90.
    #[must_use]
    pub fn sc1_pass(&self) -> bool {
        (0.60..=0.90).contains(&self.sc1_survival)
    }
    /// SC-2: 0.05 ≤ survival-attention ≤ 0.25.
    #[must_use]
    pub fn sc2_pass(&self) -> bool {
        (0.05..=0.25).contains(&self.sc2_survival_attention)
    }
    /// SC-3: all four constraints bind.
    #[must_use]
    pub fn sc3_pass(&self) -> bool {
        self.sc3_binds.iter().all(|&b| b)
    }
    /// SC-4: shaping fraction > 0.05.
    #[must_use]
    pub fn sc4_pass(&self) -> bool {
        self.sc4_shaping_fraction > 0.05
    }
    /// SC-5: 0.10 ≤ success ≤ 0.60 (and shaping was attempted).
    #[must_use]
    pub fn sc5_pass(&self) -> bool {
        self.sc5_shaping_success
            .is_some_and(|r| (0.10..=0.60).contains(&r))
    }
    /// SC-6: entropy variance ≥ `SC6_MIN_VARIANCE`.
    #[must_use]
    pub fn sc6_pass(&self) -> bool {
        self.sc6_entropy_variance >= SC6_MIN_VARIANCE
    }
}

fn ratio(num: usize, den: usize) -> f64 {
    if den == 0 {
        0.0
    } else {
        num as f64 / den as f64
    }
}

/// Reconstruct SC-1…SC-6 from a run directory (`events.ndjson` + the config).
///
/// # Panics
/// If the event log cannot be read or a record fails to parse — a conformance
/// harness should fail loudly.
#[must_use]
pub fn sanity_from_run(cfg: &RunConfig, run_dir: &Path) -> SanityReport {
    let mut r = Reconstruction::new(cfg);
    let initial_firms = cfg.agents.len();

    let mut focus_total = 0usize;
    let mut focus_survival = 0usize;
    let mut h_ticks_total = 0usize;
    let mut h_ticks_below = 0usize;
    let mut binds = [false; 4];
    let mut first_bind: [Option<u64>; 4] = [None; 4];
    let mut decisions = 0usize;
    let mut shaping_decisions = 0usize;
    let mut shaping_commits = 0usize;
    let mut shaping_ok = 0usize;
    let mut entropies: Vec<f64> = Vec::new();

    let events: Vec<Event> = firma_io::read_events::<Event>(&run_dir.join("events.ndjson"))
        .expect("read events")
        .map(|e| e.expect("parse event"))
        .collect();

    for ev in &events {
        for n in r.apply(ev) {
            match n {
                Notable::TickCompleted { tick, .. } => {
                    for f in r.firms() {
                        if !f.alive {
                            continue;
                        }
                        h_ticks_total += 1;
                        if f.h < r.h_crit() {
                            h_ticks_below += 1;
                        }
                        // "binds": solvency at g ≥ 0 (r^L == 0 is insolvency,
                        // ADR 0029); the other three strictly.
                        let over = [f.g[0] >= 0.0, f.g[1] > 0.0, f.g[2] > 0.0, f.g[3] > 0.0];
                        for (j, &o) in over.iter().enumerate() {
                            if o {
                                binds[j] = true;
                                first_bind[j].get_or_insert(tick);
                            }
                        }
                        if !f.window.is_empty() {
                            entropies.push(repertoire_entropy(&f.window));
                        }
                    }
                }
                Notable::AgentDied { tick, cause, .. } => {
                    // A death *is* the binding of its constraint — even
                    // though the firm is gone by `TickCompleted` so the
                    // reconstructed-`g_j` sweep above never sees it.
                    if cause.contains("solvency") {
                        binds[0] = true;
                        first_bind[0].get_or_insert(tick);
                    }
                    if cause.contains("compliance") {
                        binds[1] = true;
                        first_bind[1].get_or_insert(tick);
                    }
                }
                Notable::ComplianceFirstStrike { tick, .. } => {
                    binds[1] = true;
                    first_bind[1].get_or_insert(tick);
                }
                Notable::Decision { plugin, action, .. } => {
                    // any `decision.*` plugin (`satisficing` under test,
                    // `random` for Arm C) counts toward SC-4's denominator.
                    decisions += 1;
                    let _ = &plugin;
                    if (6..=8).contains(&action) {
                        shaping_decisions += 1;
                    }
                }
                Notable::Focus { focus, .. } => {
                    // only `decision.satisficing` writes `focus`.
                    focus_total += 1;
                    if focus == 0 {
                        focus_survival += 1;
                    }
                }
                Notable::ShapingCommitted {
                    applied: Some(applied),
                    ..
                } => {
                    shaping_commits += 1;
                    if applied {
                        shaping_ok += 1;
                    }
                }
                _ => {}
            }
        }
    }

    let survivors = r.live_count();
    let ent_mean = if entropies.is_empty() {
        0.0
    } else {
        entropies.iter().sum::<f64>() / entropies.len() as f64
    };
    let ent_var = if entropies.len() < 2 {
        0.0
    } else {
        entropies
            .iter()
            .map(|e| (e - ent_mean).powi(2))
            .sum::<f64>()
            / entropies.len() as f64
    };

    SanityReport {
        initial_firms,
        survivors,
        sc1_survival: survivors as f64 / initial_firms.max(1) as f64,
        sc2_survival_attention: ratio(focus_survival, focus_total),
        sc2_reconstructed_h_below_crit: ratio(h_ticks_below, h_ticks_total),
        sc3_binds: binds,
        sc3_first_bind_tick: first_bind,
        sc4_shaping_fraction: ratio(shaping_decisions, decisions),
        sc5_shaping_success: if shaping_commits == 0 {
            None
        } else {
            Some(shaping_ok as f64 / shaping_commits as f64)
        },
        sc6_entropy_variance: ent_var,
        sc6_entropy_mean: ent_mean,
        decisions,
        shaping_commits,
    }
}
