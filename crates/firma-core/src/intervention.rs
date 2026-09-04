//! The `do(·)` intervention algebra (manual §6.5).

use semver::VersionReq;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::ids::{AgentId, PluginId, ResourceKind};

/// A reference to a plugin to be resolved by `firma-registry` (manual §20.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginRef {
    /// `"<category>.<name>"`.
    pub id: PluginId,
    /// Acceptable versions. The engine refuses a MAJOR mismatch (§20.5).
    #[serde(with = "crate::intervention::version_req_string")]
    pub version: VersionReq,
    /// Optional expected content hash. When present, `firma-registry` refuses
    /// the plugin if the registered build's hash differs (manual §18.2: "hash
    /// mismatch rejected").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    /// Plugin-specific parameters, schema-validated by the plugin.
    #[serde(default)]
    pub params: JsonValue,
}

/// A `do(·)` operation applied at a specified tick (manual §6.5, §7.2 primitive
/// 9). Phase 1 implements the subset the determinism gate exercises; the rest
/// land in Phase 2 with the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Intervention {
    /// No change. A fork under `Null` MUST be byte-identical to the unforked
    /// continuation (manual §25.2 DT-4).
    Null,

    /// `set_state(i, stock(resource), value)` — force an agent's integer stock.
    SetStock {
        /// Target agent.
        agent: AgentId,
        /// Which resource.
        resource: ResourceKind,
        /// New absolute value.
        value: i64,
    },

    /// `add_rule(spec)` — register an additional rule mid-run. Used by DT-6:
    /// adding a randomness-consuming rule MUST NOT perturb any other rule's
    /// draws (§21.2 property 4).
    AddRule(PluginRef),

    /// `remove_rule(id)` — ablate a rule entirely.
    RemoveRule(PluginId),

    /// `freeze_rule(id)` — rule stays loaded (and in the manifest) but emits no
    /// deltas; distinguishes "absent" from "present but inactive" (§6.5).
    FreezeRule(PluginId),

    /// `remove_agent(i)` — population manipulation.
    RemoveAgent(AgentId),
}

impl Intervention {
    /// Whether this is the identity operation (fast path for DT-4).
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Intervention::Null)
    }
}

/// Serialise [`VersionReq`] as its canonical string form so config and manifest
/// round-trip identically.
pub(crate) mod version_req_string {
    use semver::VersionReq;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(v: &VersionReq, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<VersionReq, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
