//! `firma-plugin-resource` — the `Resource` category (manual §20.3; ADR 0038).
//!
//! Both rules run in `environment` (§10.1 phase 1). The manual gives no formula
//! for what "patchy" means mechanically, so this is **designed, not
//! transcribed** — flagged `[D]`, the same posture as `b_λ` / `P_q` / the shock
//! ramp formulas.
//!
//! * [`Constant`] — the null: prices do not move on their own (only shocks and
//!   shaping move them). It emits **no delta**. It exists and is registered so
//!   that "the environment is static" is an explicit, manifest-recorded choice
//!   rather than the unstated absence of a plugin — every Stage-2–4 config was
//!   implicitly `resource.constant`.
//! * [`Patchy`] — the input price `π^I` follows a **mean-reverting random walk**
//!   around a configured baseline:
//!   `π^I(t+1) = π^I(t) + round( reversion·(baseline − π^I(t)) + σ·Normal(0,1) )`,
//!   drawn from the **`environment`** stream (§21.3: "resource dynamics").
//!   "Patchy" here is **temporal** — a fluctuating cost of acquiring input,
//!   which stresses `solvency`. It is deliberately *not* a depleting/regen
//!   availability pool: §8.3's environment vector `(π^I, π^O, Σ_t)` has no pool
//!   level, and a genuine regenerating pool would need the kernel to account
//!   sources/sinks against `initial_total` (it does not — §19.5 is an exact
//!   `live == initial` check). The manual's §20.4 example config
//!   (`resource.patchy … regen: 0.04`) points at that pool reading; it is
//!   deferred, and the trigger to build it is the kernel gaining source/sink
//!   accounting (Phase 4).

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use firma_core::{
    ComponentId, ConflictClass, Delta, DeltaKind, DeltaKindTag, DeltaTarget, Phase, PluginId,
    RngKey, Rule, StreamId, View,
};
use firma_domain::keys;

/// This crate's build version.
#[must_use]
pub fn version() -> semver::Version {
    semver::Version::new(1, 0, 0)
}

/// Plugin ids and the declared content hash.
pub mod catalog {
    /// `resource.constant`.
    pub const CONSTANT_ID: &str = "resource.constant";
    /// `resource.patchy`.
    pub const PATCHY_ID: &str = "resource.patchy";
    /// Declared content hash for the resource build.
    pub const CONTENT_HASH: &str = "phase2s5-resource-v1";
}

// --------------------------------------------------------------------------
// resource.constant
// --------------------------------------------------------------------------

/// `resource.constant` — prices are fixed; emits nothing.
pub struct Constant {
    id: PluginId,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Default for Constant {
    fn default() -> Self {
        Constant {
            id: PluginId::new(catalog::CONSTANT_ID),
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustGlobalInt],
        }
    }
}

impl Rule for Constant {
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
    fn apply(&self, _view: &dyn View, _key: RngKey) -> Vec<Delta> {
        Vec::new()
    }
    fn assumption(&self) -> &str {
        "Input and output prices are exogenously fixed for the whole run; the \
         only things that move them are regulatory/market shocks and shaping \
         actions, never any resource dynamic of their own (§20.3)."
    }
}

// --------------------------------------------------------------------------
// resource.patchy
// --------------------------------------------------------------------------

/// Parameters for [`Patchy`]. All required — **`[D]`, not calibrated**.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchyParams {
    /// The input price the walk reverts toward (`> 0`).
    pub baseline: i64,
    /// `reversion ∈ [0, 1]` — pull toward `baseline` per tick.
    pub reversion: f64,
    /// `σ ≥ 0` — standard deviation of the per-tick `Normal(0, σ)` shock to the
    /// price (before rounding).
    pub sigma: f64,
    /// Floor for `π^I` (`≥ 1`); the walk never drives the price below this.
    pub floor: i64,
}

/// `resource.patchy` — mean-reverting random walk on `π^I`.
pub struct Patchy {
    id: PluginId,
    p: PatchyParams,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Patchy {
    /// Build from parameters.
    ///
    /// # Errors
    /// `baseline < 1`, `reversion ∉ [0, 1]`, `σ` non-finite / `< 0`, or
    /// `floor < 1` / `floor > baseline`.
    pub fn new(p: PatchyParams) -> Result<Patchy, String> {
        if p.baseline < 1 {
            return Err(format!("baseline must be >= 1, got {}", p.baseline));
        }
        if !(0.0..=1.0).contains(&p.reversion) {
            return Err(format!("reversion must be in [0, 1], got {}", p.reversion));
        }
        if !(p.sigma.is_finite() && p.sigma >= 0.0) {
            return Err(format!("sigma must be finite and >= 0, got {}", p.sigma));
        }
        if p.floor < 1 || p.floor > p.baseline {
            return Err(format!(
                "floor must be in [1, baseline]=[1, {}], got {}",
                p.baseline, p.floor
            ));
        }
        Ok(Patchy {
            id: PluginId::new(catalog::PATCHY_ID),
            p,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::AdjustGlobalInt],
        })
    }
}

impl Rule for Patchy {
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
        Some(StreamId::Environment) // §21.3 — resource dynamics
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, key: RngKey) -> Vec<Delta> {
        let current = view
            .global_int(keys::INPUT_PRICE)
            .unwrap_or(self.p.baseline);
        let mut r = firma_rng::open(&key, "resource_patchy");
        let drift = self.p.reversion * (self.p.baseline - current) as f64;
        let noise = if self.p.sigma == 0.0 {
            0.0
        } else {
            r.next_normal(0.0, self.p.sigma)
        };
        let step = (drift + noise).round() as i64;
        let next = (current + step).max(self.p.floor);
        let delta = next - current;
        if delta == 0 {
            return Vec::new();
        }
        vec![Delta {
            target: DeltaTarget::Global,
            kind: DeltaKind::AdjustGlobalInt {
                field: keys::INPUT_PRICE.to_owned(),
                delta,
            },
            conflict_class: ConflictClass::Independent,
            origin: self.id.clone(),
        }]
    }
    fn assumption(&self) -> &str {
        "The cost of acquiring input fluctuates tick to tick as a mean-reverting \
         random walk around a fixed baseline, so a firm faces a variable — not \
         constant — resource environment that intermittently squeezes its \
         liquid capital (§20.3, RDT resource munificence)."
    }
}

// --------------------------------------------------------------------------
// construction + registration
// --------------------------------------------------------------------------

/// A constructor signature shared by every rule this crate registers.
pub type Ctor = fn(&serde_json::Value) -> Result<Box<dyn Rule>, String>;

/// Build `resource.constant` (no parameters).
///
/// # Errors
/// Never — kept `Result` for a uniform constructor signature.
pub fn constant(_p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(Constant::default()))
}

/// Build `resource.patchy`. All parameters **required**.
///
/// # Errors
/// Missing / invalid [`PatchyParams`].
pub fn patchy(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: PatchyParams =
        serde_json::from_value(p.clone()).map_err(|e| format!("resource.patchy params: {e}"))?;
    Ok(Box::new(Patchy::new(params)?))
}

/// Registration entries for `firma-cli` — `(id, content_hash, ctor)`.
#[must_use]
pub fn registered() -> Vec<(&'static str, &'static str, Ctor)> {
    vec![
        (catalog::CONSTANT_ID, catalog::CONTENT_HASH, constant),
        (catalog::PATCHY_ID, catalog::CONTENT_HASH, patchy),
    ]
}

#[cfg(test)]
mod tests;
