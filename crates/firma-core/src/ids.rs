//! Identifier newtypes (manual §6.1, §7.2).

use serde::{Deserialize, Serialize};

/// A discrete simulation tick. One fiscal quarter by default (manual §6.2).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct Tick(pub u64);

impl Tick {
    /// The tick immediately after this one.
    #[must_use]
    pub const fn next(self) -> Tick {
        Tick(self.0 + 1)
    }
}

impl std::fmt::Display for Tick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Stable agent identifier. Monotonically increasing, **never reused** (§7.2
/// primitive 1, §19.2). Agents are always iterated in ascending `AgentId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(pub u64);

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "agent:{}", self.0)
    }
}

/// A conserved, transferable resource kind (§7.2 primitive 3). Config-named;
/// ordered by name so any grouping or reduction over resources is deterministic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceKind(pub String);

impl ResourceKind {
    /// Borrow the underlying name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A named slice of agent state (§7.2 primitive 2). Phase 1 uses only
/// [`ComponentId::ledger`]; the type is a `String` newtype so Phase 2 can add
/// components without touching the kernel.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComponentId(pub String);

impl ComponentId {
    /// The integer resource ledger component (ADR 0004).
    #[must_use]
    pub fn ledger() -> ComponentId {
        ComponentId("ledger".to_owned())
    }
}

/// Stable plugin identifier, `"<category>.<name>"` (manual §24.2). Stable across
/// plugin versions (§20.2).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginId(pub String);

impl PluginId {
    /// Construct from any string-like value.
    pub fn new(s: impl Into<String>) -> PluginId {
        PluginId(s.into())
    }

    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A stable 64-bit projection of the id, used as the `plugin_id` field of
    /// the RNG key tuple (manual §21.2). Derived by FNV-1a over the UTF-8 bytes
    /// — deterministic, dependency-free, and stable for the life of the id
    /// string. (The cryptographic hash in the key derivation itself lives in
    /// `firma-rng`; this is only a compaction of the identifier.)
    #[must_use]
    pub fn numeric(&self) -> u64 {
        // FNV-1a, 64-bit. Constants are the published FNV parameters, not tunables.
        const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut h = FNV_OFFSET;
        for b in self.0.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(FNV_PRIME);
        }
        h
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
