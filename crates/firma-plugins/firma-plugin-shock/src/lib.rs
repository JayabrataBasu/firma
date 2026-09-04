//! `firma-plugin-shock` — the `Shock` category (manual §13.2, §20.3; ADR 0036).
//!
//! Both rules run in `environment` (§10.1 phase 1) and move the **true** global
//! store on one of the four §13.2 channels. The channel table:
//!
//! | channel | world delta | key | kind |
//! |---|---|---|---|
//! | `Resource` | `π^I += m` | `input_price` | `AdjustGlobalInt` |
//! | `Regulatory{ThetaLimit}` | `θ_limit -= m` | `theta_limit` | `AdjustGlobalReal` |
//! | `Regulatory{ThetaCap}` | `θ_cap += m` | `theta_cap` | `AdjustGlobalReal` |
//! | `Competitive` | `π^O -= m` | `output_price` | `AdjustGlobalInt` |
//! | `Reputational` | `λ -= m` (per targeted firm) | `legitimacy` | `AdjustAgentReal` |
//!
//! The `Regulatory` channel emits the **identical** `AdjustGlobalReal` on
//! `theta_limit` that a matured `Effect::Lobby` does (§9.4: threat and response
//! share the θ surface) — not a separate path.
//!
//! **Deltas are incremental.** A shock has a *target cumulative shift* `S(t)`
//! ([`Shock::effective_shift`]); each tick the rule emits only `S(t) − S(t−1)`
//! (signed by [`Shock::channel_sign`]), tracking the shift it has applied in
//! the `ACTIVE_SHOCKS` record. A `Transient` shock's shift is reversed with one
//! step when its window closes. `i64` price channels round the *cumulative*
//! target each tick so rounding never drifts.
//!
//! * [`Scheduled`] — a config list of concrete [`Shock`]s. No RNG.
//! * [`Stochastic`] — one shock whose `onset` and `magnitude` are drawn once
//!   (tick 0) from the **`shock`** stream (§21.3 names it for exactly this).
//!
//! **A config uses at most one shock plugin** (ADR 0036) — the same rule as
//! "at most one decision plugin". Both rules own the single `ACTIVE_SHOCKS`
//! global list, and two of them rewriting it in one phase under Jacobi
//! semantics would clobber each other. A future `shock.composite` is the way
//! to combine a schedule and a stochastic source if a run needs both.
//!
//! `Σ_t` (the active-shock set, §8.3) is the `ACTIVE_SHOCKS` global list; it is
//! written for offline novelty N1/N2 (§13.3) and is not read by the running
//! model. There is no dedicated `Event::ShockFired`: a shock is fully visible
//! in the log through its `DeltaApplied` events (origin = this plugin) plus the
//! `ACTIVE_SHOCKS` rewrites, which §22.2 log-sufficiency needs.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use firma_core::{
    AgentId, ComponentId, ConflictClass, Delta, DeltaKind, DeltaKindTag, DeltaTarget, Phase,
    PluginId, RngKey, Rule, StreamId, View,
};
use firma_domain::{
    keys, Persistence, Ramp, RegulatoryTarget, Shock, ShockChannel, ShockObservability,
    ShockTargets,
};

/// This crate's build version.
#[must_use]
pub fn version() -> semver::Version {
    semver::Version::new(1, 0, 0)
}

/// Plugin ids and the declared content hash.
pub mod catalog {
    /// `shock.scheduled`.
    pub const SCHEDULED_ID: &str = "shock.scheduled";
    /// `shock.stochastic`.
    pub const STOCHASTIC_ID: &str = "shock.stochastic";
    /// Declared content hash for the shock build.
    pub const CONTENT_HASH: &str = "phase2s5-shock-v1";
}

/// One `ACTIVE_SHOCKS` record: a [`Shock`] plus the cumulative shift this
/// plugin has already applied for it, split by channel numeric type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveShock {
    /// The rule id that owns this record (so `scheduled` and `stochastic` never
    /// process each other's shocks if a config unusually has both).
    pub owner: String,
    /// The shock.
    pub shock: Shock,
    /// Cumulative shift applied on an `f64` channel (θ, λ), in world units
    /// (already signed).
    pub applied_real: f64,
    /// Cumulative shift applied on an `i64` channel (prices), in world units
    /// (already signed).
    pub applied_int: i64,
}

impl ActiveShock {
    fn to_json(&self) -> String {
        serde_json::to_string(self).expect("ActiveShock serialises")
    }
    fn from_json(s: &str) -> Option<ActiveShock> {
        serde_json::from_str(s).ok()
    }
}

/// The channel's world-effect for a target cumulative shift of `target`
/// (already signed), given the shift already applied. Returns the deltas to
/// emit and the new applied-shift bookkeeping.
fn channel_step(
    shock: &Shock,
    target_signed: f64,
    applied_real: f64,
    applied_int: i64,
    live: &[AgentId],
    origin: &PluginId,
) -> (Vec<Delta>, f64, i64) {
    let mut out = Vec::new();
    let g = |field: &str, delta: f64| Delta {
        target: DeltaTarget::Global,
        kind: DeltaKind::AdjustGlobalReal {
            field: field.to_owned(),
            delta,
        },
        conflict_class: ConflictClass::Independent,
        origin: origin.clone(),
    };
    let gi = |field: &str, delta: i64| Delta {
        target: DeltaTarget::Global,
        kind: DeltaKind::AdjustGlobalInt {
            field: field.to_owned(),
            delta,
        },
        conflict_class: ConflictClass::Independent,
        origin: origin.clone(),
    };
    match shock.channel {
        ShockChannel::Regulatory { target } => {
            let d = target_signed - applied_real;
            if d != 0.0 {
                let field = match target {
                    RegulatoryTarget::ThetaLimit => keys::THETA_LIMIT,
                    RegulatoryTarget::ThetaCap => keys::THETA_CAP,
                };
                out.push(g(field, d));
            }
            (out, target_signed, applied_int)
        }
        ShockChannel::Reputational => {
            let d = target_signed - applied_real;
            if d != 0.0 {
                for &a in live {
                    if shock.targets.hits(a.0) {
                        out.push(Delta {
                            target: DeltaTarget::Agent(a),
                            kind: DeltaKind::AdjustAgentReal {
                                field: keys::LEGITIMACY.to_owned(),
                                delta: d,
                            },
                            conflict_class: ConflictClass::Independent,
                            origin: origin.clone(),
                        });
                    }
                }
            }
            (out, target_signed, applied_int)
        }
        ShockChannel::Resource | ShockChannel::Competitive => {
            // Round the *cumulative* target so rounding never drifts.
            let target_int = target_signed.round() as i64;
            let d = target_int - applied_int;
            if d != 0 {
                let field = match shock.channel {
                    ShockChannel::Resource => keys::INPUT_PRICE,
                    _ => keys::OUTPUT_PRICE,
                };
                out.push(gi(field, d));
            }
            (out, applied_real, target_int)
        }
    }
}

/// Process one shock for `tick`: emit its incremental channel delta(s) and
/// return the updated [`ActiveShock`], or `None` if the shock has fully closed
/// and should leave `Σ_t`.
fn step_shock(
    owner: &str,
    shock: &Shock,
    prev: Option<&ActiveShock>,
    tick: u64,
    live: &[AgentId],
    origin: &PluginId,
) -> (Vec<Delta>, Option<ActiveShock>) {
    let applied_real = prev.map_or(0.0, |p| p.applied_real);
    let applied_int = prev.map_or(0, |p| p.applied_int);

    // Not started yet: keep it pending (so `stochastic`'s drawn shock survives
    // to its onset), emit nothing.
    if tick < shock.onset {
        return (
            Vec::new(),
            Some(ActiveShock {
                owner: owner.to_owned(),
                shock: shock.clone(),
                applied_real,
                applied_int,
            }),
        );
    }

    // Started and finished (window closed and reversal already emitted): drop.
    if !shock.in_sigma(tick) {
        return (Vec::new(), None);
    }

    let target_signed = shock.effective_shift(tick) * shock.channel_sign();
    let (deltas, new_real, new_int) = channel_step(
        shock,
        target_signed,
        applied_real,
        applied_int,
        live,
        origin,
    );
    (
        deltas,
        Some(ActiveShock {
            owner: owner.to_owned(),
            shock: shock.clone(),
            applied_real: new_real,
            applied_int: new_int,
        }),
    )
}

/// Assemble the phase's deltas for `owner`'s shocks `shocks`, given the current
/// `ACTIVE_SHOCKS` records and the live agents. Emits one `ReplaceGlobalList`
/// **only if** the active-shock list actually changed.
fn run_owner(owner: &str, origin: &PluginId, shocks: &[Shock], view: &dyn View) -> Vec<Delta> {
    let tick = view.tick().0;
    let live = view.live_agents();

    // **The shock rule owns `ACTIVE_SHOCKS` entirely** (ADR 0036): a config
    // uses at most one shock plugin, exactly as it uses at most one decision
    // plugin. Two shock rules both rewriting this one global list under Jacobi
    // semantics would clobber each other (last-writer-wins, and neither sees
    // the other's step) — so this rewrites the whole list from *its own*
    // shocks. The `owner` field is kept only for log legibility.
    let old: Vec<ActiveShock> = view
        .global_records(keys::ACTIVE_SHOCKS)
        .iter()
        .filter_map(|s| ActiveShock::from_json(s))
        .collect();

    let mut out = Vec::new();
    let mut new_records: Vec<ActiveShock> = Vec::new();
    for shock in shocks {
        let prev = old.iter().find(|a| a.shock.id == shock.id);
        let (deltas, updated) = step_shock(owner, shock, prev, tick, live, origin);
        out.extend(deltas);
        if let Some(a) = updated {
            new_records.push(a);
        }
    }

    if new_records != old {
        out.push(Delta {
            target: DeltaTarget::Global,
            kind: DeltaKind::ReplaceGlobalList {
                list: keys::ACTIVE_SHOCKS.to_owned(),
                records_json: new_records.iter().map(ActiveShock::to_json).collect(),
            },
            conflict_class: ConflictClass::Independent,
            origin: origin.clone(),
        });
    }
    out
}

fn writes() -> Vec<DeltaKindTag> {
    vec![
        DeltaKindTag::AdjustGlobalReal,
        DeltaKindTag::AdjustGlobalInt,
        DeltaKindTag::AdjustAgentReal,
        DeltaKindTag::ReplaceGlobalList,
    ]
}

// --------------------------------------------------------------------------
// shock.scheduled
// --------------------------------------------------------------------------

/// Parameters for [`Scheduled`]: a list of concrete shocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledParams {
    /// The shocks to schedule. Each is a full §13.2 [`Shock`] object; `onset`
    /// is the tick it fires.
    pub shocks: Vec<Shock>,
}

/// `shock.scheduled` — deterministic; every shock is spelled out in config.
pub struct Scheduled {
    id: PluginId,
    shocks: Vec<Shock>,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Scheduled {
    /// Build from parameters.
    ///
    /// # Errors
    /// Any shock failing [`Shock::validate`], or two shocks sharing an id.
    pub fn new(p: ScheduledParams) -> Result<Scheduled, String> {
        for s in &p.shocks {
            s.validate()?;
        }
        let mut ids: Vec<&str> = p.shocks.iter().map(|s| s.id.as_str()).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err("shock.scheduled: shock ids must be unique".to_string());
        }
        Ok(Scheduled {
            id: PluginId::new(catalog::SCHEDULED_ID),
            shocks: p.shocks,
            reads: vec![ComponentId::ledger()],
            writes: writes(),
        })
    }
}

impl Rule for Scheduled {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::Environment
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        run_owner(catalog::SCHEDULED_ID, &self.id, &self.shocks, view)
    }
    fn assumption(&self) -> &str {
        "Threats arrive on a fixed, pre-specified schedule — each shock's \
         channel, magnitude, onset, ramp, and persistence are given exactly — \
         so the environment is a controlled input, not a random one (§13.2)."
    }
}

// --------------------------------------------------------------------------
// shock.stochastic
// --------------------------------------------------------------------------

/// Parameters for [`Stochastic`]: a shock template whose `onset` and
/// `magnitude` are drawn once. Everything except those two is fixed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StochasticParams {
    /// Id given to the drawn shock.
    pub id: String,
    /// Channel.
    pub channel: ShockChannel,
    /// `onset ~ Uniform{onset_min ..= onset_max}` from the `shock` stream.
    pub onset_min: u64,
    /// See [`Self::onset_min`].
    pub onset_max: u64,
    /// `magnitude ~ |Normal(magnitude_mean, magnitude_sd)|` from the `shock`
    /// stream (absolute value so it stays a non-negative size). `sd = 0` ⇒
    /// fixed at `magnitude_mean`.
    pub magnitude_mean: f64,
    /// See [`Self::magnitude_mean`].
    pub magnitude_sd: f64,
    /// Ramp shape (fixed).
    pub ramp: Ramp,
    /// Persistence shape (fixed).
    pub persistence: Persistence,
    /// The shock's own observability (fixed; logged, not consumed).
    pub observability: ShockObservability,
    /// Declared novelty (fixed; logged for offline N1/N2).
    pub novelty: f64,
    /// Targets (fixed).
    pub targets: ShockTargets,
}

/// `shock.stochastic` — one shock whose timing and size are random.
pub struct Stochastic {
    id: PluginId,
    p: StochasticParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Stochastic {
    /// Build from parameters.
    ///
    /// # Errors
    /// `onset_min > onset_max`, non-finite / negative magnitude moments, or a
    /// template that cannot produce a valid [`Shock`].
    pub fn new(p: StochasticParams) -> Result<Stochastic, String> {
        if p.onset_min > p.onset_max {
            return Err("shock.stochastic: onset_min > onset_max".to_string());
        }
        if !(p.magnitude_mean.is_finite() && p.magnitude_mean >= 0.0) {
            return Err("shock.stochastic: magnitude_mean must be finite and >= 0".to_string());
        }
        if !(p.magnitude_sd.is_finite() && p.magnitude_sd >= 0.0) {
            return Err("shock.stochastic: magnitude_sd must be finite and >= 0".to_string());
        }
        if !(0.0..=1.0).contains(&p.novelty) {
            return Err("shock.stochastic: novelty must be in [0, 1]".to_string());
        }
        Ok(Stochastic {
            id: PluginId::new(catalog::STOCHASTIC_ID),
            p,
            reads: vec![ComponentId::ledger()],
            writes: writes(),
        })
    }

    /// The concrete shock, drawn deterministically from `key` (`shock` stream).
    /// Called once at tick 0; re-derivable for tests.
    fn draw(&self, key: &RngKey) -> Shock {
        let mut r = firma_rng::open(key, "shock_draw");
        let onset = r.uniform_inclusive(self.p.onset_min, self.p.onset_max);
        let magnitude = if self.p.magnitude_sd == 0.0 {
            self.p.magnitude_mean
        } else {
            r.next_normal(self.p.magnitude_mean, self.p.magnitude_sd)
                .abs()
        };
        Shock {
            id: self.p.id.clone(),
            channel: self.p.channel,
            magnitude,
            onset,
            ramp: self.p.ramp,
            persistence: self.p.persistence,
            observability: self.p.observability,
            novelty: self.p.novelty,
            targets: self.p.targets.clone(),
        }
    }
}

impl Rule for Stochastic {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::Environment
    }
    fn rng_stream(&self) -> Option<StreamId> {
        Some(StreamId::Shock) // §21.3 — shock timing and magnitude
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, key: RngKey) -> Vec<Delta> {
        // The shock is drawn **once**, at tick 0 (`key.tick == 0`), and then
        // lives in `ACTIVE_SHOCKS`. Every later tick reuses the record's shock
        // — re-`draw()`ing would use that tick's key and produce a different
        // onset/magnitude.
        let existing: Option<Shock> = view
            .global_records(keys::ACTIVE_SHOCKS)
            .iter()
            .filter_map(|s| ActiveShock::from_json(s))
            .find(|a| a.shock.id == self.p.id)
            .map(|a| a.shock);
        let shock = match existing {
            Some(s) => s,
            None if view.tick().0 == 0 => {
                let s = self.draw(&key);
                if let Err(e) = s.validate() {
                    debug_assert!(false, "stochastic drew an invalid shock: {e}");
                    return Vec::new();
                }
                s
            }
            // Past tick 0 with no record: the shock has already run its course
            // and been dropped from Σ_t. Nothing more to do.
            None => return Vec::new(),
        };
        run_owner(
            catalog::STOCHASTIC_ID,
            &self.id,
            std::slice::from_ref(&shock),
            view,
        )
    }
    fn assumption(&self) -> &str {
        "A single threat arrives at a random time within a known window and \
         with a random magnitude drawn from a declared distribution; its \
         channel and shape are fixed (§13.2)."
    }
}

// --------------------------------------------------------------------------
// construction + registration
// --------------------------------------------------------------------------

/// A constructor signature shared by every rule this crate registers.
pub type Ctor = fn(&serde_json::Value) -> Result<Box<dyn Rule>, String>;

/// Build `shock.scheduled`.
///
/// # Errors
/// Missing / invalid [`ScheduledParams`].
pub fn scheduled(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: ScheduledParams =
        serde_json::from_value(p.clone()).map_err(|e| format!("shock.scheduled params: {e}"))?;
    Ok(Box::new(Scheduled::new(params)?))
}

/// Build `shock.stochastic`.
///
/// # Errors
/// Missing / invalid [`StochasticParams`].
pub fn stochastic(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: StochasticParams =
        serde_json::from_value(p.clone()).map_err(|e| format!("shock.stochastic params: {e}"))?;
    Ok(Box::new(Stochastic::new(params)?))
}

/// Registration entries for `firma-cli` — `(id, content_hash, ctor)`.
#[must_use]
pub fn registered() -> Vec<(&'static str, &'static str, Ctor)> {
    vec![
        (catalog::SCHEDULED_ID, catalog::CONTENT_HASH, scheduled),
        (catalog::STOCHASTIC_ID, catalog::CONTENT_HASH, stochastic),
    ]
}

#[cfg(test)]
mod tests;
