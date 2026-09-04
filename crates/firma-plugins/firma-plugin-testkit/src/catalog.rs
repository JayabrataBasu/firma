//! Registration metadata for the test plugins (manual §20.5, ADR 0008).
//!
//! `firma-cli` builds a `firma-registry::Registry` from these entries. The
//! `*_HASH` constants are the Phase 1 **declared** content hashes (there is no
//! compiled-artefact hashing yet — see `firma-registry` docs and `PROGRESS.md`).
//! Bump the hash string and the [`version`] together whenever a rule's
//! behaviour changes (§20.5: a MAJOR bump needs a golden-trace update).

use semver::Version;

/// Plugin id of [`Transfer`](crate::Transfer).
pub const TRANSFER_ID: &str = "testkit.transfer";
/// Plugin id of [`KeyedNudge`](crate::KeyedNudge).
pub const KEYED_NUDGE_ID: &str = "testkit.keyed_nudge";
/// Plugin id of [`KeyedPerAgent`](crate::KeyedPerAgent).
pub const KEYED_PER_AGENT_ID: &str = "testkit.keyed_per_agent";
/// Plugin id of [`ForceAdjust`](crate::ForceAdjust).
pub const FORCE_ADJUST_ID: &str = "testkit.force_adjust";
/// Plugin id of [`SelectAction`](crate::SelectAction).
pub const SELECT_ACTION_ID: &str = "testkit.select_action";
/// Plugin id of [`AdditiveResolver`](crate::AdditiveResolver).
pub const RESOLVER_ID: &str = "conflict.additive";

/// Declared content hash for [`Transfer`](crate::Transfer).
pub const TRANSFER_HASH: &str = "phase1-testkit-transfer-v1";
/// Declared content hash for [`KeyedNudge`](crate::KeyedNudge).
pub const KEYED_NUDGE_HASH: &str = "phase1-testkit-keyed-nudge-v1";
/// Declared content hash for [`KeyedPerAgent`](crate::KeyedPerAgent).
pub const KEYED_PER_AGENT_HASH: &str = "phase1-testkit-keyed-per-agent-v1";
/// Declared content hash for [`ForceAdjust`](crate::ForceAdjust).
pub const FORCE_ADJUST_HASH: &str = "phase1-testkit-force-adjust-v1";
/// Declared content hash for [`SelectAction`](crate::SelectAction).
pub const SELECT_ACTION_HASH: &str = "phase2s2-testkit-select-action-v1";
/// Declared content hash for [`AdditiveResolver`](crate::AdditiveResolver).
pub const RESOLVER_HASH: &str = "phase1-testkit-additive-resolver-v1";

/// The shared build version of the testkit plugins.
#[must_use]
pub fn version() -> Version {
    Version::new(1, 0, 0)
}
