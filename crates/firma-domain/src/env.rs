//! The observed-environment snapshot (manual §8.3 `e_t = (π^I, π^O, Σ_t)`,
//! §12.2 observation).
//!
//! [`EnvSnapshot`] is the record `observation.{full,noisy,delayed}` writes,
//! per agent, under [`keys::OBSERVED_ENV`](crate::keys::OBSERVED_ENV) each
//! `observe` phase, and the record the same plugins append to the global
//! [`keys::ENV_HISTORY`](crate::keys::ENV_HISTORY) ring so `delayed(k)` can
//! reach back `k` ticks (ADR 0035). `decision.{satisficing,random}` read the
//! per-agent snapshot in `decide`, falling back to the true global store when
//! no `Observation` plugin ran (so an unobserved run is byte-identical).
//!
//! It carries only the fields a firm's decision procedure consults: the two
//! prices and the three constraint parameters θ. `Σ_t` (the active-shock set)
//! is logged separately under
//! [`keys::ACTIVE_SHOCKS`](crate::keys::ACTIVE_SHOCKS); the MVP decision
//! procedure does not read it, so it is not in this struct (§16.4:
//! anticipation is out of scope, and reasoning about *which* shocks are
//! active would be a form of it).

use serde::{Deserialize, Serialize};

/// One firm's view of the environment for one tick, or one entry of the global
/// history ring. `θ` values are `f64`; the two prices are `i64` (they must
/// stay exact for `r^L` arithmetic — §21.4). A `noisy` observation rounds its
/// perturbed prices back to `i64` before storing here.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvSnapshot {
    /// The tick this snapshot describes (the *true* tick for `full`/`noisy`;
    /// the delayed source tick for `delayed(k)`).
    pub tick: u64,
    /// `π^I` — input price as seen.
    pub input_price: i64,
    /// `π^O` — output price as seen.
    pub output_price: i64,
    /// `θ_limit` as seen.
    pub theta_limit: f64,
    /// `θ_cap` as seen.
    pub theta_cap: f64,
    /// `θ_Q` as seen.
    pub theta_q: i64,
}

impl EnvSnapshot {
    /// Serialise to the canonical JSON stored in the keyed list.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("EnvSnapshot serialises")
    }

    /// Parse one stored record.
    ///
    /// # Errors
    /// If `s` is not a valid [`EnvSnapshot`] JSON object.
    pub fn from_json(s: &str) -> Result<EnvSnapshot, String> {
        serde_json::from_str(s).map_err(|e| format!("invalid EnvSnapshot: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let s = EnvSnapshot {
            tick: 42,
            input_price: 2,
            output_price: 3,
            theta_limit: 0.45,
            theta_cap: 0.40,
            theta_q: 100,
        };
        assert_eq!(EnvSnapshot::from_json(&s.to_json()).unwrap(), s);
    }

    #[test]
    fn unknown_field_is_rejected() {
        assert!(EnvSnapshot::from_json(
            r#"{"tick":0,"input_price":1,"output_price":1,"theta_limit":0.1,"theta_cap":0.1,"theta_q":1,"x":0}"#
        )
        .is_err());
    }
}
