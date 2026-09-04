//! Constraint-shaping parameters and the shared success / lag model
//! (manual §11.2, §11.3, §16.1; ADR 0023).
//!
//! The `p_success` formula and the lag *distribution* ([`LagRange`]) live here,
//! once, so that `action.shaping.rdt_standard`'s three rules and VT-7's generic
//! property check read the same code (mirrors `firma-domain::dynamics` for
//! §11.1). The draw itself belongs to the rule — it needs `firma-rng`, which
//! this crate deliberately does not depend on.

use serde::{Deserialize, Serialize};

/// `b_λ` for `lobby` — the value `LobbyParams::default()` supplies explicitly.
/// **`[D]`, not calibrated** — the manual gives no §16.1 value (ADR 0023). It
/// is **not** a serde default: `SuccessModel` requires `b_lambda` on every
/// path (Stage-2 review Fix 2), the same way §16.1's silent-`P_q` gap was
/// closed in Stage 1. `contract` / `diversify` configs MUST state their own.
pub const LOBBY_B_LAMBDA: f64 = 0.2;

/// `b_κ` for `lobby` — see [`LOBBY_B_LAMBDA`]. **`[D]`, not calibrated**; not a
/// serde default.
pub const LOBBY_B_KAPPA: f64 = 0.1;

/// The §11.2 success model for one shaping action, exposed in config as §11.3
/// property 3 requires ("model declared in plugin docs and exposed in config").
///
/// `p_success(a) = clip(p0 + b_λ·λ + b_κ·(spend − κ_a)/κ_a, 0, p_max)`, with
/// `p_max < 1` **strictly** (§11.2, §11.3 property 3).
///
/// **Every field is required — no serde defaults** (Stage-2 review Fix 2).
/// `b_lambda` / `b_kappa` in particular have no §16.1 value, so a config that
/// omits them is rejected rather than silently given `0.2` / `0.1`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuccessModel {
    /// `p^a_0` — base success probability.
    pub p0: f64,
    /// `b_λ` — legitimacy coefficient (≥ 0). Required; no §16.1 value.
    pub b_lambda: f64,
    /// `b_κ` — overspend coefficient (≥ 0). Required; no §16.1 value.
    pub b_kappa: f64,
    /// `p^a_max` — the ceiling. **MUST be `< 1`** (§11.3 property 3).
    pub p_max: f64,
    /// `κ_a` — the minimum commitment (also the §11.2 affordability
    /// precondition and the `spend` baseline). `> 0`.
    pub kappa_min: i64,
}

impl SuccessModel {
    /// Validate against §11.2 / §11.3: `0 ≤ p0`, `p_max < 1` strictly,
    /// `0 ≤ p0 ≤ p_max`, `b_λ, b_κ ≥ 0`, `κ_a > 0`.
    ///
    /// # Errors
    /// A message naming the first violated condition.
    pub fn validate(&self) -> Result<(), String> {
        if !self.p_max.is_finite() || self.p_max >= 1.0 {
            return Err(format!(
                "p_max must be < 1 strictly (§11.3 property 3), got {}",
                self.p_max
            ));
        }
        if !(0.0..=self.p_max).contains(&self.p0) {
            return Err(format!(
                "p0 must be in [0, p_max]={:?}, got {}",
                (0.0, self.p_max),
                self.p0
            ));
        }
        if self.b_lambda < 0.0 || self.b_kappa < 0.0 {
            return Err(format!(
                "b_lambda and b_kappa must be >= 0 (§11.2), got {}, {}",
                self.b_lambda, self.b_kappa
            ));
        }
        if self.kappa_min <= 0 {
            return Err(format!("kappa_min must be > 0, got {}", self.kappa_min));
        }
        Ok(())
    }

    /// `p_success` for a commitment of `spend` capital by a firm with legitimacy
    /// `legitimacy` (§11.2). `spend ≥ κ_a` is assumed (the affordability
    /// precondition); the overspend term is then `≥ 0`.
    #[must_use]
    pub fn p_success(&self, legitimacy: f64, spend: i64) -> f64 {
        let overspend = (spend - self.kappa_min) as f64 / self.kappa_min as f64;
        let raw = self.p0 + self.b_lambda * legitimacy + self.b_kappa * overspend;
        clip(raw, 0.0, self.p_max)
    }
}

/// `clip(x, l, u) = min(max(x, l), u)` (§11.2).
#[must_use]
pub fn clip(x: f64, l: f64, u: f64) -> f64 {
    x.max(l).min(u)
}

/// `Δ_a ∼ Uniform{Δ^min_a, …, Δ^max_a}` (§11.2). `min ≥ 1` enforces §11.3
/// property 2 ("lag ≥ 1 tick").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LagRange {
    /// `Δ^min_a` — **MUST be `≥ 1`** (§11.3 property 2).
    pub min: u64,
    /// `Δ^max_a` — `≥ min`.
    pub max: u64,
}

impl LagRange {
    /// Validate against §11.3 property 2: `1 ≤ min ≤ max`.
    ///
    /// # Errors
    /// A message if `min < 1` or `max < min`.
    pub fn validate(&self) -> Result<(), String> {
        if self.min < 1 {
            return Err(format!(
                "lag min must be >= 1 (§11.3 property 2), got {}",
                self.min
            ));
        }
        if self.max < self.min {
            return Err(format!(
                "lag max ({}) must be >= min ({})",
                self.max, self.min
            ));
        }
        Ok(())
    }
}

/// `lobby` parameters (§11.2 action 6, §16.1). All fields default — `lag`,
/// `p0`, `p_max`, `kappa_min`, `delta_theta` to their §16.1 values, and
/// `b_lambda` / `b_kappa` to the `[D]`-not-calibrated
/// [`LOBBY_B_LAMBDA`] / [`LOBBY_B_KAPPA`] (the manual gives no value; supplied
/// here explicitly, not as a `SuccessModel` serde fallback).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LobbyParams {
    /// `Δ_ℓ` — lag range. §16.1 default `{2, …, 6}`.
    #[serde(default = "default_lobby_lag")]
    pub lag: LagRange,
    /// Success model. §16.1: `p^ℓ_0 = 0.25`, `p^ℓ_max = 0.75`, `κ_ℓ = 25`;
    /// `b_λ` / `b_κ` are `[D]`. A *partial* `success` object in config is still
    /// rejected — the default applies only when `success` is absent entirely.
    #[serde(default = "default_lobby_success")]
    pub success: SuccessModel,
    /// `δ_θ` — the shift applied to `θ_limit` on success. §16.1 default `0.10`.
    #[serde(default = "default_delta_theta")]
    pub delta_theta: f64,
}

fn default_lobby_lag() -> LagRange {
    LagRange { min: 2, max: 6 }
}
fn default_lobby_success() -> SuccessModel {
    SuccessModel {
        p0: 0.25,                 // §16.1 "p^ℓ_0 Lobby base success 0.25"
        b_lambda: LOBBY_B_LAMBDA, // [D] not calibrated — no §16.1 value (ADR 0023)
        b_kappa: LOBBY_B_KAPPA,   // [D] not calibrated — no §16.1 value (ADR 0023)
        p_max: 0.75,              // §16.1 "p^ℓ_max Lobby ceiling 0.75"
        kappa_min: 25,            // §16.1 "κ_ℓ Lobby cost 25"
    }
}
fn default_delta_theta() -> f64 {
    0.10
}

impl Default for LobbyParams {
    fn default() -> Self {
        LobbyParams {
            lag: default_lobby_lag(),
            success: default_lobby_success(),
            delta_theta: default_delta_theta(),
        }
    }
}

/// `contract` parameters (§11.2 action 7). **No §16.1 values exist** (manual
/// gap, ADR 0023) — every field is required config.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractParams {
    /// `Δ_k` — lag range.
    pub lag: LagRange,
    /// Success model (`κ_k` is `success.kappa_min`).
    pub success: SuccessModel,
    /// `δ_Q` — the shift applied to `θ_Q` on success.
    pub delta_q: i64,
    /// `q_0` — obligation added to the acting firm on success (the double
    /// edge, §11.2).
    pub q0: i64,
}

/// `diversify` parameters (§11.2 action 8). **No §16.1 values exist** (manual
/// gap, ADR 0023) — every field is required config.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiversifyParams {
    /// `Δ_d` — lag range.
    pub lag: LagRange,
    /// Success model (`κ_d` is `success.kappa_min`).
    pub success: SuccessModel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lobby_defaults_match_section_16_1() {
        let p = LobbyParams::default();
        assert_eq!(p.lag, LagRange { min: 2, max: 6 });
        assert!((p.success.p0 - 0.25).abs() < 1e-12);
        assert!((p.success.p_max - 0.75).abs() < 1e-12);
        assert_eq!(p.success.kappa_min, 25);
        assert!((p.delta_theta - 0.10).abs() < 1e-12);
    }

    #[test]
    fn p_max_ge_one_is_rejected() {
        let mut m = default_lobby_success();
        m.p_max = 1.0;
        assert!(m.validate().is_err());
        m.p_max = 1.5;
        assert!(m.validate().is_err());
    }

    #[test]
    fn lag_min_zero_is_rejected() {
        assert!(LagRange { min: 0, max: 4 }.validate().is_err());
        assert!(LagRange { min: 1, max: 1 }.validate().is_ok());
    }

    #[test]
    fn p_success_is_clipped_to_p_max() {
        let m = default_lobby_success(); // p0=0.25, b_λ=0.2, b_κ=0.1, p_max=0.75
                                         // λ=1, spend=κ → 0.25 + 0.2 = 0.45
        assert!((m.p_success(1.0, 25) - 0.45).abs() < 1e-12);
        // huge overspend → clipped at 0.75
        assert!((m.p_success(1.0, 100_000) - 0.75).abs() < 1e-12);
        // λ=0, spend=κ → 0.25
        assert!((m.p_success(0.0, 25) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn contract_params_are_all_required() {
        // no defaults: an empty object fails
        assert!(serde_json::from_str::<ContractParams>("{}").is_err());
    }

    #[test]
    fn success_model_b_coefficients_have_no_serde_default() {
        // Stage-2 review Fix 2: b_lambda / b_kappa are required — a success
        // model that omits either is a deserialization error, not `0.2` / `0.1`.
        let full = r#"{"p0":0.2,"b_lambda":0.2,"b_kappa":0.1,"p_max":0.6,"kappa_min":30}"#;
        assert!(serde_json::from_str::<SuccessModel>(full).is_ok());

        let no_b_lambda = r#"{"p0":0.2,"b_kappa":0.1,"p_max":0.6,"kappa_min":30}"#;
        let e = serde_json::from_str::<SuccessModel>(no_b_lambda).unwrap_err();
        assert!(e.to_string().contains("b_lambda"), "{e}");

        let no_b_kappa = r#"{"p0":0.2,"b_lambda":0.2,"p_max":0.6,"kappa_min":30}"#;
        let e = serde_json::from_str::<SuccessModel>(no_b_kappa).unwrap_err();
        assert!(e.to_string().contains("b_kappa"), "{e}");
    }
}
