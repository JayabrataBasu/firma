//! `firma-plugin-observation` — the `Observation` category (manual §12.2,
//! §20.3; ADR 0035).
//!
//! **The firm observes its own state — `x`, and its own `h` — exactly and
//! always.** That is not configurable and needs no plugin (§12.2, §16.4). What
//! an `Observation` plugin governs is only the firm's view of the *environment
//! and* `θ` (`π^I`, `π^O`, `θ_limit`, `θ_cap`, `θ_Q`).
//!
//! Each rule runs in `observe` (§10.1 phase 2, before `decide`) and writes one
//! [`EnvSnapshot`] per live agent under
//! [`keys::OBSERVED_ENV`](firma_domain::keys::OBSERVED_ENV) via
//! `ReplaceAgentList`. `decision.{satisficing,random}` read that snapshot in
//! `decide`; with **no** `Observation` plugin configured the key is absent and
//! the decision procedure reads the true global store, so an unobserved run is
//! byte-identical (ADR 0035).
//!
//! * [`full`] — lossless passthrough. Explicit and registered (like
//!   `resource.constant`) so the observation channel is visible in the log
//!   even when it introduces no error.
//! * [`noisy`] — `Normal(0, σ)` added to every observed field; the two prices
//!   are rounded back to `i64`. `σ` is **required** (no §16.1 value — same
//!   discipline as `P_q`, `b_λ`). Draws from the **`environment`** stream
//!   (ADR 0035): the perceptual channel is exogenous variation the firm faces,
//!   and a matched-environment sweep (§21.3 — "same `environment` and `shock`,
//!   different `mechanism`") must hold it fixed while `β` varies.
//! * [`delayed`] — the firm sees the environment/θ state from `k` ticks ago.
//!   Maintains a global [`ENV_HISTORY`](firma_domain::keys::ENV_HISTORY) ring
//!   of the last `k + 1` true snapshots. `k` is **required**. Draws nothing.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use firma_core::{
    AgentId, ComponentId, ConflictClass, Delta, DeltaKind, DeltaKindTag, DeltaTarget, Phase,
    PluginId, RngKey, Rule, StreamId, View,
};
use firma_domain::{keys, EnvSnapshot};

/// This crate's build version.
#[must_use]
pub fn version() -> semver::Version {
    semver::Version::new(1, 0, 0)
}

/// Plugin ids and the declared content hash.
pub mod catalog {
    /// `observation.full`.
    pub const FULL_ID: &str = "observation.full";
    /// `observation.noisy`.
    pub const NOISY_ID: &str = "observation.noisy";
    /// `observation.delayed`.
    pub const DELAYED_ID: &str = "observation.delayed";
    /// Declared content hash for the observation build.
    pub const CONTENT_HASH: &str = "phase2s5-observation-v1";
}

/// Read the true environment/θ from the global store into a snapshot for
/// `tick`. Absent scalars default to `0` — the same fallback the decision
/// procedure already uses (an unseeded θ makes the constraint bind at once).
fn true_env(view: &dyn View, tick: u64) -> EnvSnapshot {
    EnvSnapshot {
        tick,
        input_price: view.global_int(keys::INPUT_PRICE).unwrap_or(0),
        output_price: view.global_int(keys::OUTPUT_PRICE).unwrap_or(0),
        theta_limit: view.global_real(keys::THETA_LIMIT).unwrap_or(0.0),
        theta_cap: view.global_real(keys::THETA_CAP).unwrap_or(0.0),
        theta_q: view.global_int(keys::THETA_Q).unwrap_or(0),
    }
}

fn per_agent_snapshot(agent: AgentId, snap: &EnvSnapshot, origin: &PluginId) -> Delta {
    Delta {
        target: DeltaTarget::Agent(agent),
        kind: DeltaKind::ReplaceAgentList {
            list: keys::OBSERVED_ENV.to_owned(),
            records_json: vec![snap.to_json()],
        },
        conflict_class: ConflictClass::Independent,
        origin: origin.clone(),
    }
}

// --------------------------------------------------------------------------
// observation.full
// --------------------------------------------------------------------------

/// `observation.full` — every firm sees the true environment/θ.
pub struct Full {
    id: PluginId,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Default for Full {
    fn default() -> Self {
        Full {
            id: PluginId::new(catalog::FULL_ID),
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::ReplaceAgentList],
        }
    }
}

impl Rule for Full {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::Observe
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        let snap = true_env(view, view.tick().0);
        view.live_agents()
            .iter()
            .map(|&a| per_agent_snapshot(a, &snap, &self.id))
            .collect()
    }
    fn assumption(&self) -> &str {
        "Every firm perceives the current input and output prices and the \
         constraint parameters θ exactly and without lag; only the firm's view \
         of the environment is modelled here — its own state and its own \
         viability margin h are always observed without error (§12.2)."
    }
}

// --------------------------------------------------------------------------
// observation.noisy
// --------------------------------------------------------------------------

/// Parameters for [`Noisy`]. `σ` has **no §16.1 default** and is required.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoisyParams {
    /// `σ` — standard deviation of the `Normal(0, σ)` perception error added to
    /// every observed field (`≥ 0`, finite). `[D]`, not calibrated.
    pub sigma: f64,
}

/// `observation.noisy` — each firm's view of environment/θ carries independent
/// `Normal(0, σ)` error, drawn per firm from the `environment` stream.
pub struct Noisy {
    id: PluginId,
    sigma: f64,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Noisy {
    /// Build from parameters.
    ///
    /// # Errors
    /// `σ` non-finite or `< 0`.
    pub fn new(p: NoisyParams) -> Result<Noisy, String> {
        if !(p.sigma.is_finite() && p.sigma >= 0.0) {
            return Err(format!("sigma must be finite and >= 0, got {}", p.sigma));
        }
        Ok(Noisy {
            id: PluginId::new(catalog::NOISY_ID),
            sigma: p.sigma,
            reads: vec![ComponentId::ledger()],
            writes: vec![DeltaKindTag::ReplaceAgentList],
        })
    }
}

impl Rule for Noisy {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::Observe
    }
    fn rng_stream(&self) -> Option<StreamId> {
        Some(StreamId::Environment) // ADR 0035 — exogenous perceptual variation
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, key: RngKey) -> Vec<Delta> {
        let tick = view.tick().0;
        let truth = true_env(view, tick);
        let mut out = Vec::new();
        for &agent in view.live_agents() {
            let mut r = firma_rng::open_for(&key, Some(agent.0), "obs_noise");
            // Field order is fixed (price, price, θ, θ, θ) so the draw sequence
            // is deterministic per firm.
            let ip = truth.input_price as f64 + r.next_normal(0.0, self.sigma);
            let op = truth.output_price as f64 + r.next_normal(0.0, self.sigma);
            let tl = truth.theta_limit + r.next_normal(0.0, self.sigma);
            let tc = truth.theta_cap + r.next_normal(0.0, self.sigma);
            let tq = truth.theta_q as f64 + r.next_normal(0.0, self.sigma);
            let observed = EnvSnapshot {
                tick,
                input_price: ip.round() as i64,
                output_price: op.round() as i64,
                theta_limit: tl,
                theta_cap: tc,
                theta_q: tq.round() as i64,
            };
            out.push(per_agent_snapshot(agent, &observed, &self.id));
        }
        out
    }
    fn assumption(&self) -> &str {
        "Each firm perceives prices and the constraint parameters θ with \
         independent zero-mean Gaussian error of fixed standard deviation, and \
         acts on the perceived values; its own state and its own viability \
         margin h remain observed exactly (§12.2)."
    }
}

// --------------------------------------------------------------------------
// observation.delayed
// --------------------------------------------------------------------------

/// Parameters for [`Delayed`]. `k` has **no §16.1 default** and is required.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DelayedParams {
    /// `k` — how many ticks stale the observed environment/θ is. `k = 0` is
    /// equivalent to `full`. `[D]`, not calibrated.
    pub k: u64,
}

/// `observation.delayed` — every firm sees the environment/θ state from `k`
/// ticks ago (the same lag for all firms). Maintains a global history ring.
pub struct Delayed {
    id: PluginId,
    k: u64,
    reads: Vec<ComponentId>,
    writes: Vec<DeltaKindTag>,
}

impl Delayed {
    /// Build from parameters.
    #[must_use]
    pub fn new(p: DelayedParams) -> Delayed {
        Delayed {
            id: PluginId::new(catalog::DELAYED_ID),
            k: p.k,
            reads: vec![ComponentId::ledger()],
            writes: vec![
                DeltaKindTag::ReplaceAgentList,
                DeltaKindTag::ReplaceGlobalList,
            ],
        }
    }
}

impl Rule for Delayed {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    fn version(&self) -> semver::Version {
        version()
    }
    fn phase(&self) -> Phase {
        Phase::Observe
    }
    fn reads(&self) -> &[ComponentId] {
        &self.reads
    }
    fn writes(&self) -> &[DeltaKindTag] {
        &self.writes
    }
    fn apply(&self, view: &dyn View, _key: RngKey) -> Vec<Delta> {
        let tick = view.tick().0;
        let truth = true_env(view, tick);

        // Append this tick's truth to the history ring; keep the last k + 1.
        let mut hist: Vec<EnvSnapshot> = view
            .global_records(keys::ENV_HISTORY)
            .iter()
            .filter_map(|s| EnvSnapshot::from_json(s).ok())
            .collect();
        hist.push(truth);
        let cap = (self.k + 1) as usize;
        if hist.len() > cap {
            let drop = hist.len() - cap;
            hist.drain(0..drop);
        }

        // The observed snapshot is `k` back from the newest; if history is
        // shorter than that (early ticks), the oldest available is used.
        let idx = hist.len().saturating_sub(1 + self.k as usize);
        let observed = hist[idx];

        let mut out: Vec<Delta> = view
            .live_agents()
            .iter()
            .map(|&a| per_agent_snapshot(a, &observed, &self.id))
            .collect();
        out.push(Delta {
            target: DeltaTarget::Global,
            kind: DeltaKind::ReplaceGlobalList {
                list: keys::ENV_HISTORY.to_owned(),
                records_json: hist.iter().map(EnvSnapshot::to_json).collect(),
            },
            conflict_class: ConflictClass::Independent,
            origin: self.id.clone(),
        });
        out
    }
    fn assumption(&self) -> &str {
        "Every firm acts on a stale picture of prices and the constraint \
         parameters θ — the state as it was a fixed number of ticks earlier — \
         while still observing its own state and its own viability margin h \
         without error or lag (§12.2)."
    }
}

// --------------------------------------------------------------------------
// construction + registration
// --------------------------------------------------------------------------

/// A constructor signature shared by every rule this crate registers.
pub type Ctor = fn(&serde_json::Value) -> Result<Box<dyn Rule>, String>;

/// Build `observation.full` (no parameters).
///
/// # Errors
/// Never — kept `Result` for a uniform constructor signature.
pub fn full(_p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(Full::default()))
}

/// Build `observation.noisy`. `sigma` is **required**.
///
/// # Errors
/// Missing / invalid [`NoisyParams`].
pub fn noisy(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: NoisyParams =
        serde_json::from_value(p.clone()).map_err(|e| format!("observation.noisy params: {e}"))?;
    Ok(Box::new(Noisy::new(params)?))
}

/// Build `observation.delayed`. `k` is **required**.
///
/// # Errors
/// Missing / invalid [`DelayedParams`].
pub fn delayed(p: &serde_json::Value) -> Result<Box<dyn Rule>, String> {
    let params: DelayedParams = serde_json::from_value(p.clone())
        .map_err(|e| format!("observation.delayed params: {e}"))?;
    Ok(Box::new(Delayed::new(params)))
}

/// Registration entries for `firma-cli` — `(id, content_hash, ctor)`.
#[must_use]
pub fn registered() -> Vec<(&'static str, &'static str, Ctor)> {
    vec![
        (catalog::FULL_ID, catalog::CONTENT_HASH, full),
        (catalog::NOISY_ID, catalog::CONTENT_HASH, noisy),
        (catalog::DELAYED_ID, catalog::CONTENT_HASH, delayed),
    ]
}

#[cfg(test)]
mod tests;
