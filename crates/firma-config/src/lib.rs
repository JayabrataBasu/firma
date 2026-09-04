//! `firma-config` — typed, schema-validated, content-hashed run configuration
//! (manual §20.4, ADR 0002).
//!
//! Phase 1 uses **JSON** (`serde_json`, `deny_unknown_fields`) rather than the
//! YAML shown in §20.4 — see `PROGRESS.md` OQ-2 and ADR 0010. "Unknown keys are
//! an error, not a warning" (§20.4) is enforced by `#[serde(deny_unknown_fields)]`
//! on every struct.
//!
//! A config is **data, never code** (ADR 0002 "config must be data"): no
//! conditionals, loops, or expressions. Its [`content_hash`](RunConfig::content_hash)
//! is SHA-256 over canonical JSON and goes into the run manifest (§22.3).

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use firma_core::{ConfigError, Intervention, PluginRef, ResourceKind};

/// A complete description of one run (manual §5.1 "Run", §20.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfig {
    /// Human label for the experiment this run belongs to.
    pub experiment: String,
    /// Config schema version (manual §20.4 `schema_version`).
    pub schema_version: Version,
    /// Acceptable engine versions (manual §20.4 `engine`). The engine refuses a
    /// config it does not satisfy (§20.4, §20.5).
    #[serde(with = "version_req_string")]
    pub engine: VersionReq,
    /// Per-stream run seeds (manual §21.3).
    pub seeds: StreamSeeds,
    /// World-level settings.
    pub world: WorldConfig,
    /// Initial agent population. Ids must be unique.
    pub agents: Vec<AgentConfig>,
    /// Initial environment state.
    #[serde(default)]
    pub environment: EnvConfig,
    /// Rules to load, resolved by `firma-registry`.
    pub rules: Vec<PluginRef>,
    /// Interventions, each stamped with the tick it applies at (manual §6.5).
    #[serde(default)]
    pub interventions: Vec<TimedIntervention>,
}

/// The four independent stream seeds (manual §21.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamSeeds {
    /// Decisions, action outcomes, shaping success.
    pub mechanism: u64,
    /// Resource dynamics, exogenous variation.
    pub environment: u64,
    /// Shock timing and magnitude.
    pub shock: u64,
    /// Initial conditions.
    pub init: u64,
}

/// World-level configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldConfig {
    /// Run horizon in ticks (manual §16.1 default 400).
    pub ticks: u64,
    /// The conserved resource kinds in this run. Non-empty; deduplicated and
    /// sorted on validation.
    pub resources: Vec<ResourceKind>,
    /// The conflict resolver plugin (manual §19.4, §20.3).
    pub conflict_resolver: PluginRef,
    /// Snapshot cadence in ticks (manual §22.1 default 25).
    #[serde(default = "default_snapshot_every")]
    pub snapshot_every: u64,
    /// Initial global-store real scalars, seeded before tick 0 (ADR 0030) —
    /// e.g. `θ_limit`, `θ_cap`. Opaque keys (`firma_domain::keys`).
    ///
    /// `skip_serializing_if` empty so a config that omits it canonicalises
    /// **byte-identically** to a pre-ADR-0030 config — the golden trace and
    /// `phase1-smoke` `run_id` (SHA-256 over canonical JSON) are unaffected.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub global_reals: BTreeMap<String, f64>,
    /// Initial global-store integer scalars (ADR 0030) — e.g. `θ_Q`, `π^I`,
    /// `π^O`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub global_ints: BTreeMap<String, i64>,
}

fn default_snapshot_every() -> u64 {
    25
}

/// One agent's initial state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    /// Stable agent id (manual §6.1). Unique within a config.
    pub id: u64,
    /// Birth tick (manual §7.2 primitive 1).
    #[serde(default)]
    pub birth_tick: u64,
    /// Opening integer stock balances by resource (ADR 0004).
    #[serde(default)]
    pub stocks: BTreeMap<ResourceKind, i64>,
    /// Initial per-agent real scalars, seeded before tick 0 (ADR 0030) — e.g.
    /// `capability`, `legitimacy`, the three `aspiration_*`. `skip_serializing_if`
    /// empty (see [`WorldConfig::global_reals`]).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reals: BTreeMap<String, f64>,
    /// Initial per-agent integer scalars (ADR 0030) — e.g. `obligation`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ints: BTreeMap<String, i64>,
}

/// Initial environment state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvConfig {
    /// Opening integer balances of the shared resource pool.
    #[serde(default)]
    pub stocks: BTreeMap<ResourceKind, i64>,
}

/// An [`Intervention`] plus the tick it fires at (manual §6.5 "with a tick stamp").
///
/// `deny_unknown_fields` is intentionally **absent** here: the `intervention`
/// field is `#[serde(flatten)]`ed, and serde cannot combine `flatten` with
/// `deny_unknown_fields`. The flattened [`Intervention`] is itself an
/// internally-tagged enum, so an unknown `op` is still rejected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimedIntervention {
    /// The tick at which the intervention is applied, before that tick's phases
    /// run.
    pub at: u64,
    /// The operation.
    #[serde(flatten)]
    pub intervention: Intervention,
}

impl RunConfig {
    /// Parse a config from a JSON file.
    ///
    /// # Errors
    /// [`ConfigError::Io`] if the file cannot be read; [`ConfigError::Parse`]
    /// on malformed JSON or an unknown key.
    pub fn load(path: &Path) -> Result<RunConfig, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(e.to_string()))?;
        Self::from_json(&text)
    }

    /// Parse a config from a JSON string.
    ///
    /// # Errors
    /// [`ConfigError::Parse`] on malformed JSON or an unknown key.
    pub fn from_json(text: &str) -> Result<RunConfig, ConfigError> {
        serde_json::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Serialise back to canonical JSON (for the manifest's "fully resolved
    /// config after defaults", §22.3).
    ///
    /// # Errors
    /// [`ConfigError::Parse`] if serialisation fails (should not happen for a
    /// valid config).
    pub fn to_json(&self) -> Result<String, ConfigError> {
        firma_core::to_pretty_json(self).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// SHA-256 (hex) over canonical JSON — the config's content hash (manual
    /// §20.4, §22.3). Call [`normalise`](Self::normalise) first so equivalent
    /// configs hash equally.
    ///
    /// # Errors
    /// [`ConfigError::Parse`] if serialisation fails.
    pub fn content_hash(&self) -> Result<String, ConfigError> {
        let json =
            firma_core::to_canonical_json(self).map_err(|e| ConfigError::Parse(e.to_string()))?;
        let mut h = Sha256::new();
        h.update(json.as_bytes());
        Ok(hex(&h.finalize()))
    }

    /// Canonicalise in place: sort and dedup `world.resources`. Idempotent.
    pub fn normalise(&mut self) {
        self.world.resources.sort();
        self.world.resources.dedup();
    }

    /// Validate against the schema rules the type system cannot express (manual
    /// §20.4). Call [`normalise`](Self::normalise) first.
    ///
    /// # Errors
    /// [`ConfigError::EngineMismatch`] if `engine` excludes `engine_version`;
    /// [`ConfigError::Invalid`] for any structural problem (empty resource list,
    /// duplicate agent id, unknown resource in a stock map, negative opening
    /// balance, `ticks == 0`).
    pub fn validate(&self, engine_version: &Version) -> Result<(), ConfigError> {
        if !self.engine.matches(engine_version) {
            return Err(ConfigError::EngineMismatch {
                required: self.engine.to_string(),
                actual: engine_version.to_string(),
            });
        }
        if self.world.ticks == 0 {
            return Err(ConfigError::Invalid("world.ticks must be > 0".into()));
        }
        if self.world.resources.is_empty() {
            return Err(ConfigError::Invalid(
                "world.resources must be non-empty".into(),
            ));
        }
        let known: std::collections::BTreeSet<&ResourceKind> =
            self.world.resources.iter().collect();

        let mut seen_ids = std::collections::BTreeSet::new();
        for a in &self.agents {
            if !seen_ids.insert(a.id) {
                return Err(ConfigError::Invalid(format!("duplicate agent id {}", a.id)));
            }
            check_stocks(&format!("agent {}", a.id), &a.stocks, &known)?;
        }
        check_stocks("environment", &self.environment.stocks, &known)?;

        for iv in &self.interventions {
            if iv.at >= self.world.ticks {
                return Err(ConfigError::Invalid(format!(
                    "intervention at tick {} is beyond the horizon {}",
                    iv.at, self.world.ticks
                )));
            }
        }
        Ok(())
    }
}

fn check_stocks(
    who: &str,
    stocks: &BTreeMap<ResourceKind, i64>,
    known: &std::collections::BTreeSet<&ResourceKind>,
) -> Result<(), ConfigError> {
    for (res, qty) in stocks {
        if !known.contains(res) {
            return Err(ConfigError::Invalid(format!(
                "{who}: stock references unknown resource {res:?}"
            )));
        }
        if *qty < 0 {
            return Err(ConfigError::Invalid(format!(
                "{who}: opening balance for {res:?} is negative ({qty})"
            )));
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// (De)serialise a [`VersionReq`] as its canonical string.
mod version_req_string {
    use semver::VersionReq;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(v: &VersionReq, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<VersionReq, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        r#"{
          "experiment": "unit",
          "schema_version": "1.0.0",
          "engine": ">=0.1.0, <0.2.0",
          "seeds": {"mechanism": 1, "environment": 2, "shock": 3, "init": 4},
          "world": {
            "ticks": 10,
            "resources": ["input", "capital"],
            "conflict_resolver": {"id": "conflict.additive", "version": "^1"}
          },
          "agents": [
            {"id": 0, "stocks": {"capital": 100}},
            {"id": 1}
          ],
          "rules": [
            {"id": "testkit.transfer", "version": "^1",
             "params": {"resource": "capital", "from": {"agent": 0}, "to": {"agent": 1}, "amount": 1}}
          ]
        }"#
    }

    #[test]
    fn parses_normalises_validates() {
        let mut c = RunConfig::from_json(sample()).unwrap();
        c.normalise();
        assert_eq!(
            c.world.resources,
            vec![ResourceKind("capital".into()), ResourceKind("input".into())]
        );
        assert_eq!(c.world.snapshot_every, 25); // default applied
        c.validate(&Version::new(0, 1, 0)).unwrap();
    }

    #[test]
    fn rejects_unknown_key() {
        let bad = sample().replace("\"ticks\": 10", "\"ticks\": 10, \"bogus\": 1");
        let err = RunConfig::from_json(&bad).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    #[test]
    fn rejects_engine_mismatch() {
        let mut c = RunConfig::from_json(sample()).unwrap();
        c.normalise();
        let err = c.validate(&Version::new(0, 5, 0)).unwrap_err();
        assert!(matches!(err, ConfigError::EngineMismatch { .. }));
    }

    #[test]
    fn rejects_unknown_resource_in_stock() {
        let bad = sample().replace("\"capital\": 100", "\"gold\": 100");
        let mut c = RunConfig::from_json(&bad).unwrap();
        c.normalise();
        assert!(matches!(
            c.validate(&Version::new(0, 1, 0)),
            Err(ConfigError::Invalid(_))
        ));
    }

    #[test]
    fn content_hash_is_stable_and_order_independent() {
        let mut a = RunConfig::from_json(sample()).unwrap();
        let reordered = sample().replace(
            "\"resources\": [\"input\", \"capital\"]",
            "\"resources\": [\"capital\", \"input\"]",
        );
        let mut b = RunConfig::from_json(&reordered).unwrap();
        a.normalise();
        b.normalise();
        assert_eq!(a.content_hash().unwrap(), b.content_hash().unwrap());
        assert_eq!(a.content_hash().unwrap().len(), 64);
    }
}
