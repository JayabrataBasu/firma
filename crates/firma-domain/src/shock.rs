//! The `Shock` formal object and its per-tick magnitude schedule (manual
//! §13.2, §13.3; ADR 0036).
//!
//! **The manual gives the variant *names* of [`Ramp`] and [`Persistence`], not
//! their per-tick formulas.** This module fixes concrete, stated formulas —
//! flagged `[D]`, the same posture as `b_λ` / `P_q` / the `patchy` mechanic —
//! and [`Shock::effective_shift`] is the single place they live so
//! `shock.scheduled` and `shock.stochastic` compute the identical schedule.
//!
//! The **channel** deltas (which field moves, and the sign) are the plugin's
//! job — building a `Delta` needs an `origin` [`PluginId`] and the live-agent
//! list — but [`ShockChannel`] and [`Shock::channel_sign`] pin the direction so
//! that too is written once.

use serde::{Deserialize, Serialize};

/// §13.2 channel. `Regulatory` additionally names which θ field it moves — the
/// table lists both `θ_limit -= m` and `θ_cap += m`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShockChannel {
    /// `π^I += m` — stresses `solvency` (§13.2).
    Resource,
    /// `θ_limit -= m` (`ThetaLimit`) or `θ_cap += m` (`ThetaCap`) — stresses
    /// `compliance`. **This is the channel that shares θ's surface with
    /// `lobby`** (§9.4, §13.2): it emits the *same* `AdjustGlobalReal` on the
    /// same key `Effect::Lobby` uses.
    Regulatory {
        /// Which θ real the shock moves.
        target: RegulatoryTarget,
    },
    /// `π^O -= m` — stresses `solvency`, `obligation` (§13.2).
    Competitive,
    /// `λ -= m` for every targeted firm — stresses `obligation` (§13.2). The
    /// table's companion "`supply` weights reduced" sub-effect is **deferred**:
    /// nothing in the running model reads `Edge.weight` (only the not-yet-built
    /// §13.1 dependence metric does), so reducing it would be inert. Trigger to
    /// build it: §13.1 dependence landing with a live consumer.
    Reputational,
}

/// Which θ real a `Regulatory` shock moves (§13.2 table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegulatoryTarget {
    /// `θ_limit -= m`.
    ThetaLimit,
    /// `θ_cap += m`.
    ThetaCap,
}

/// §13.2 ramp. **`[D]` formulas (manual gives names only):**
/// - `Instant` — full magnitude from `onset`.
/// - `Linear(d)` — the shift grows linearly `0 → 1` over `d` ticks: factor
///   `clamp((t − onset) / d, 0, 1)`, so `0` at `onset` and `1` at `onset + d`.
/// - `Exponential(rate)` — **read as ramp-*up*** (the name is ambiguous;
///   this is the reading): factor `1 − exp(−rate · (t − onset))`, approaching
///   `1` asymptotically, `rate > 0` the approach speed. A decay reading would
///   need a separate "shift then fade" object; `Transient` already covers
///   fade-out.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Ramp {
    /// Full magnitude at `onset`.
    Instant,
    /// Linear `0 → full` over `duration` ticks.
    Linear {
        /// Ticks from `onset` to full magnitude. `0` ⇒ treated as `Instant`.
        duration: u64,
    },
    /// Asymptotic approach at rate `rate` (`> 0`).
    Exponential {
        /// Approach speed.
        rate: f64,
    },
}

/// §13.2 persistence. **`[D]` semantics (manual gives names only):**
/// - `Transient(d)` — the shift applies (per `ramp`) while
///   `onset ≤ t < onset + d`, then the accumulated shift is **reversed** in one
///   step and the shock leaves `Σ_t`.
/// - `Permanent` — the shift applies (per `ramp`) and stays; the shock stays in
///   `Σ_t` for the rest of the run.
/// - `Recurring(period)` — **read as a staircase**: a fresh `magnitude` step is
///   added every `period` ticks from `onset`, permanently (ramp ignored — each
///   step is instant). `S(t) = magnitude · (1 + ⌊(t − onset) / period⌋)`. A
///   one-tick-spike reading is the alternative; the staircase models repeated
///   tightening, which is the §13-relevant case.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Persistence {
    /// Active for `duration` ticks from `onset`, then reversed.
    Transient {
        /// Active window length in ticks (`> 0`).
        duration: u64,
    },
    /// Active from `onset` for the rest of the run.
    Permanent,
    /// A fresh `magnitude` step every `period` ticks (`> 0`).
    Recurring {
        /// Ticks between steps.
        period: u64,
    },
}

/// §13.2 observability — **the shock's own** observability, distinct from the
/// general [`Observation`](crate) plugin. See ADR 0036 for how the two compose:
/// a shock's `θ`/price move lands in the *true* global store, then whatever
/// `Observation` plugin is configured perturbs the firm's *view* of that store.
/// `Hidden` / `Noisy` / `Delayed` here are **logged** (for offline
/// reconstruction of what a firm could have known) but the MVP decision
/// procedure reads only the `Observation`-plugin view, so this field does not
/// change dynamics on its own this Stage. Recorded, not consumed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShockObservability {
    /// Perceived exactly and immediately.
    Full,
    /// Perceived `ticks` ticks late.
    Delayed {
        /// Lag in ticks.
        ticks: u64,
    },
    /// Perceived with `Normal(0, sigma)` error.
    Noisy {
        /// Noise standard deviation.
        sigma: f64,
    },
    /// Not perceived at all.
    Hidden,
}

/// §13.2 targets. `Predicate` is deferred (no predicate language in the MVP
/// config — ADR 0002); `All` and an explicit `Set` cover the E1 needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShockTargets {
    /// Every live firm.
    All,
    /// An explicit set of agent ids.
    Set {
        /// The targeted agent ids.
        ids: Vec<u64>,
    },
}

impl ShockTargets {
    /// Whether `agent` is targeted.
    #[must_use]
    pub fn hits(&self, agent: u64) -> bool {
        match self {
            ShockTargets::All => true,
            ShockTargets::Set { ids } => ids.contains(&agent),
        }
    }
}

/// The §13.2 formal object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shock {
    /// Stable id, for the event log and for N1 memory matching (§13.3).
    pub id: String,
    /// Which surface the shock stresses.
    pub channel: ShockChannel,
    /// `m` — full magnitude (always given as a non-negative size; the channel
    /// sets the sign of the world delta).
    pub magnitude: f64,
    /// `onset` — the first tick the shock is active.
    pub onset: u64,
    /// Ramp shape.
    pub ramp: Ramp,
    /// Persistence shape.
    pub persistence: Persistence,
    /// The shock's own observability (logged, not consumed this Stage).
    pub observability: ShockObservability,
    /// `novelty ∈ [0, 1]` — declared novelty. **Logged for offline N1/N2
    /// (§13.3); the running model does not consume it** (§14.1 measurement is
    /// offline, same as `h`). N1 needs the firm's memory `M` and the shock's
    /// parameter vector `z`; both are reconstructable from the event log
    /// (the shock record is logged in full under `ACTIVE_SHOCKS`), so the
    /// memory ring buffer stays deferred until an online consumer needs it.
    pub novelty: f64,
    /// Which firms the shock hits.
    pub targets: ShockTargets,
}

impl Shock {
    /// Validate: `magnitude ≥ 0` and finite, `novelty ∈ [0, 1]`, positive
    /// `Transient` / `Recurring` durations, `Exponential` rate `> 0` and finite,
    /// `Noisy` sigma `≥ 0`.
    ///
    /// # Errors
    /// A message naming the first violated condition.
    pub fn validate(&self) -> Result<(), String> {
        if !self.magnitude.is_finite() || self.magnitude < 0.0 {
            return Err(format!(
                "shock {}: magnitude must be finite and >= 0",
                self.id
            ));
        }
        if !(0.0..=1.0).contains(&self.novelty) {
            return Err(format!("shock {}: novelty must be in [0, 1]", self.id));
        }
        match self.ramp {
            Ramp::Exponential { rate } if !(rate.is_finite() && rate > 0.0) => {
                return Err(format!(
                    "shock {}: Exponential rate must be finite and > 0",
                    self.id
                ));
            }
            _ => {}
        }
        match self.persistence {
            Persistence::Transient { duration } | Persistence::Recurring { period: duration }
                if duration == 0 =>
            {
                return Err(format!(
                    "shock {}: Transient/Recurring duration must be > 0",
                    self.id
                ));
            }
            _ => {}
        }
        if let ShockObservability::Noisy { sigma } = self.observability {
            if !(sigma.is_finite() && sigma >= 0.0) {
                return Err(format!(
                    "shock {}: Noisy sigma must be finite and >= 0",
                    self.id
                ));
            }
        }
        Ok(())
    }

    fn ramp_factor(&self, dt: u64) -> f64 {
        match self.ramp {
            Ramp::Instant => 1.0,
            Ramp::Linear { duration } => {
                if duration == 0 {
                    1.0
                } else {
                    (dt as f64 / duration as f64).min(1.0)
                }
            }
            Ramp::Exponential { rate } => 1.0 - (-rate * dt as f64).exp(),
        }
    }

    /// The **target cumulative shift** `S(t)` in magnitude units (always `≥ 0`;
    /// the sign of the world delta comes from [`Self::channel_sign`]). `0`
    /// before `onset`, after a `Transient` window, and between `Recurring`
    /// steps only in the one-tick-spike reading (not used — see [`Persistence`]).
    #[must_use]
    pub fn effective_shift(&self, tick: u64) -> f64 {
        if tick < self.onset {
            return 0.0;
        }
        let dt = tick - self.onset;
        match self.persistence {
            Persistence::Permanent => self.magnitude * self.ramp_factor(dt),
            Persistence::Transient { duration } => {
                if dt < duration {
                    self.magnitude * self.ramp_factor(dt)
                } else {
                    0.0 // window closed — the plugin emits the reversing step
                }
            }
            Persistence::Recurring { period } => {
                let steps = 1 + dt / period.max(1);
                self.magnitude * steps as f64
            }
        }
    }

    /// Whether the shock is still in `Σ_t` at `tick` (so the plugin keeps its
    /// `ACTIVE_SHOCKS` record). A `Transient` shock leaves one tick after its
    /// window (once the reversing step is emitted).
    #[must_use]
    pub fn in_sigma(&self, tick: u64) -> bool {
        if tick < self.onset {
            return false;
        }
        match self.persistence {
            Persistence::Permanent | Persistence::Recurring { .. } => true,
            Persistence::Transient { duration } => tick <= self.onset + duration,
        }
    }

    /// The sign the channel applies to `effective_shift` when it moves the
    /// world: `+1` for `Resource` (`π^I` up) and `Regulatory{ThetaCap}`
    /// (`θ_cap` up), `−1` for `Regulatory{ThetaLimit}` (`θ_limit` down),
    /// `Competitive` (`π^O` down), and `Reputational` (`λ` down).
    #[must_use]
    pub fn channel_sign(&self) -> f64 {
        match self.channel {
            ShockChannel::Resource
            | ShockChannel::Regulatory {
                target: RegulatoryTarget::ThetaCap,
            } => 1.0,
            ShockChannel::Regulatory {
                target: RegulatoryTarget::ThetaLimit,
            }
            | ShockChannel::Competitive
            | ShockChannel::Reputational => -1.0,
        }
    }

    /// Serialise to canonical JSON (stored under `ACTIVE_SHOCKS`).
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Shock serialises")
    }

    /// Parse one stored / configured shock.
    ///
    /// # Errors
    /// If `s` is not a valid [`Shock`] JSON object.
    pub fn from_json(s: &str) -> Result<Shock, String> {
        serde_json::from_str(s).map_err(|e| format!("invalid Shock: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Shock {
        Shock {
            id: "s1".into(),
            channel: ShockChannel::Regulatory {
                target: RegulatoryTarget::ThetaLimit,
            },
            magnitude: 0.3,
            onset: 10,
            ramp: Ramp::Instant,
            persistence: Persistence::Permanent,
            observability: ShockObservability::Full,
            novelty: 0.8,
            targets: ShockTargets::All,
        }
    }

    #[test]
    fn instant_permanent_is_a_single_step() {
        let s = base();
        assert_eq!(s.effective_shift(9), 0.0);
        assert!((s.effective_shift(10) - 0.3).abs() < 1e-12);
        assert!((s.effective_shift(200) - 0.3).abs() < 1e-12);
        assert!((s.channel_sign() - (-1.0)).abs() < 1e-12);
    }

    #[test]
    fn linear_ramp_reaches_full_at_onset_plus_duration() {
        let s = Shock {
            ramp: Ramp::Linear { duration: 4 },
            ..base()
        };
        assert!((s.effective_shift(10) - 0.0).abs() < 1e-12); // dt=0
        assert!((s.effective_shift(11) - 0.075).abs() < 1e-12); // dt=1 → 1/4
        assert!((s.effective_shift(14) - 0.3).abs() < 1e-12); // dt=4 → full
        assert!((s.effective_shift(50) - 0.3).abs() < 1e-12); // clamped
    }

    #[test]
    fn transient_closes_and_leaves_sigma() {
        let s = Shock {
            persistence: Persistence::Transient { duration: 5 },
            ..base()
        };
        assert!((s.effective_shift(12) - 0.3).abs() < 1e-12); // dt=2, inside
        assert_eq!(s.effective_shift(15), 0.0); // dt=5, window closed
        assert!(s.in_sigma(15)); // still logged the tick the reversal fires
        assert!(!s.in_sigma(16)); // gone
    }

    #[test]
    fn recurring_is_a_staircase() {
        let s = Shock {
            magnitude: 0.1,
            persistence: Persistence::Recurring { period: 20 },
            ..base()
        };
        assert!((s.effective_shift(10) - 0.1).abs() < 1e-12); // step 1
        assert!((s.effective_shift(29) - 0.1).abs() < 1e-12); // still step 1
        assert!((s.effective_shift(30) - 0.2).abs() < 1e-12); // step 2
        assert!((s.effective_shift(70) - 0.4).abs() < 1e-12); // step 4
    }

    #[test]
    fn validation_rejects_the_bad_cases() {
        assert!(Shock {
            magnitude: -1.0,
            ..base()
        }
        .validate()
        .is_err());
        assert!(Shock {
            novelty: 1.5,
            ..base()
        }
        .validate()
        .is_err());
        assert!(Shock {
            persistence: Persistence::Transient { duration: 0 },
            ..base()
        }
        .validate()
        .is_err());
        assert!(Shock {
            ramp: Ramp::Exponential { rate: 0.0 },
            ..base()
        }
        .validate()
        .is_err());
        assert!(base().validate().is_ok());
    }

    #[test]
    fn roundtrip() {
        let s = Shock {
            channel: ShockChannel::Reputational,
            targets: ShockTargets::Set { ids: vec![1, 3] },
            observability: ShockObservability::Delayed { ticks: 2 },
            ..base()
        };
        assert_eq!(Shock::from_json(&s.to_json()).unwrap(), s);
    }
}
