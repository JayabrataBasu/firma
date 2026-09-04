//! `firma-registry` — resolve config plugin references to compiled builds
//! (manual §18.2, §20, ADR 0008).
//!
//! Phase 1 uses **compile-time registration** (ADR 0008): plugin crates provide
//! ordinary constructors, and the binary (`firma-cli`) populates a [`Registry`]
//! at start-up by calling [`Registry::register_rule`] /
//! [`Registry::register_resolver`]. This crate never depends on a plugin crate
//! (§18.1) — it only defines the mechanism and the resolution/verification
//! rules.
//!
//! [`Registry::resolve_rule`] enforces the three §18.2 rejections: unknown
//! plugin, version mismatch, content-hash mismatch (plus a fourth, bad params).
//!
//! ## Content hashes in Phase 1
//!
//! There is no compiled-artefact hashing yet. Each registered build carries a
//! **declared** `content_hash` string (a constant the plugin keeps in step with
//! its own source). A config's [`PluginRef::content_hash`] is checked against
//! it when present. Real artefact hashing is a later concern; see `PROGRESS.md`.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use semver::Version;
use serde_json::Value as JsonValue;

use firma_core::{ConflictResolver, PluginId, PluginRef, RegistryError, Rule};

/// Builds a [`Rule`] trait object from validated parameters, or returns a
/// human-readable reason it could not.
pub type RuleCtor = Box<dyn Fn(&JsonValue) -> Result<Box<dyn Rule>, String> + Send + Sync>;

/// Builds a [`ConflictResolver`] trait object from validated parameters.
pub type ResolverCtor =
    Box<dyn Fn(&JsonValue) -> Result<Box<dyn ConflictResolver>, String> + Send + Sync>;

/// A registered rule build.
pub struct RegisteredRule {
    /// Stable id.
    pub id: PluginId,
    /// This build's version.
    pub version: Version,
    /// Declared content hash (see module docs).
    pub content_hash: String,
    ctor: RuleCtor,
}

impl RegisteredRule {
    /// Construct a registration entry.
    pub fn new(
        id: impl Into<String>,
        version: Version,
        content_hash: impl Into<String>,
        ctor: RuleCtor,
    ) -> RegisteredRule {
        RegisteredRule {
            id: PluginId::new(id),
            version,
            content_hash: content_hash.into(),
            ctor,
        }
    }
}

/// A registered conflict-resolver build.
pub struct RegisteredResolver {
    /// Stable id.
    pub id: PluginId,
    /// This build's version.
    pub version: Version,
    /// Declared content hash.
    pub content_hash: String,
    ctor: ResolverCtor,
}

impl RegisteredResolver {
    /// Construct a registration entry.
    pub fn new(
        id: impl Into<String>,
        version: Version,
        content_hash: impl Into<String>,
        ctor: ResolverCtor,
    ) -> RegisteredResolver {
        RegisteredResolver {
            id: PluginId::new(id),
            version,
            content_hash: content_hash.into(),
            ctor,
        }
    }
}

/// The set of compiled plugin builds available to a run.
#[derive(Default)]
pub struct Registry {
    rules: BTreeMap<String, RegisteredRule>,
    resolvers: BTreeMap<String, RegisteredResolver>,
    /// `PluginId::numeric()` → the string id that owns that value. Enforced
    /// injective across rule registrations so two distinct plugins can never
    /// share an `RngKey.plugin_id` (manual §21.2 property 4, §17 A3; ADR 0013).
    rule_numeric_ids: BTreeMap<u64, String>,
}

impl Registry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Register a rule build.
    ///
    /// # Panics
    /// - If a rule with the same id string is already registered.
    /// - If the rule's [`PluginId::numeric()`] value is already claimed by a
    ///   *different* id string — a collision that would make the two plugins'
    ///   `RngKey.plugin_id` indistinguishable and silently correlate their
    ///   draws (manual §21.2 property 4, §17 A3; ADR 0013).
    ///
    /// Both are start-up wiring bugs, not runtime conditions.
    pub fn register_rule(&mut self, r: RegisteredRule) -> &mut Self {
        let key = r.id.0.clone();
        let numeric = r.id.numeric();
        if let Some(existing) = self.rule_numeric_ids.get(&numeric) {
            assert!(
                existing == &key,
                "PluginId::numeric() collision: {key:?} and {existing:?} both hash to \
                 {numeric:#018x} — RngKey.plugin_id would be indistinguishable between them \
                 (manual §21.2 property 4, §17 A3; ADR 0013)"
            );
        }
        self.rule_numeric_ids.insert(numeric, key.clone());
        assert!(
            self.rules.insert(key.clone(), r).is_none(),
            "duplicate rule registration for {key:?}"
        );
        self
    }

    /// Register a conflict-resolver build.
    ///
    /// # Panics
    /// If a resolver with the same id is already registered.
    pub fn register_resolver(&mut self, r: RegisteredResolver) -> &mut Self {
        let key = r.id.0.clone();
        assert!(
            self.resolvers.insert(key.clone(), r).is_none(),
            "duplicate resolver registration for {key:?}"
        );
        self
    }

    /// Resolve a config [`PluginRef`] to a live [`Rule`], verifying id, version,
    /// and (if declared) content hash (manual §18.2, §20.5).
    ///
    /// # Errors
    /// [`RegistryError::UnknownPlugin`], [`RegistryError::VersionMismatch`],
    /// [`RegistryError::HashMismatch`], or [`RegistryError::BadParams`].
    pub fn resolve_rule(&self, spec: &PluginRef) -> Result<Box<dyn Rule>, RegistryError> {
        let reg = self
            .rules
            .get(spec.id.as_str())
            .ok_or_else(|| RegistryError::UnknownPlugin(spec.id.0.clone()))?;
        check_version(&spec.id, &spec.version, &reg.version)?;
        check_hash(&spec.id, spec.content_hash.as_deref(), &reg.content_hash)?;
        (reg.ctor)(&spec.params).map_err(|detail| RegistryError::BadParams {
            id: spec.id.0.clone(),
            detail,
        })
    }

    /// Resolve a config [`PluginRef`] to a live [`ConflictResolver`].
    ///
    /// # Errors
    /// As [`resolve_rule`](Self::resolve_rule).
    pub fn resolve_resolver(
        &self,
        spec: &PluginRef,
    ) -> Result<Box<dyn ConflictResolver>, RegistryError> {
        let reg = self
            .resolvers
            .get(spec.id.as_str())
            .ok_or_else(|| RegistryError::UnknownPlugin(spec.id.0.clone()))?;
        check_version(&spec.id, &spec.version, &reg.version)?;
        check_hash(&spec.id, spec.content_hash.as_deref(), &reg.content_hash)?;
        (reg.ctor)(&spec.params).map_err(|detail| RegistryError::BadParams {
            id: spec.id.0.clone(),
            detail,
        })
    }

    /// Registered rule builds, ascending by id — for manifest listing and
    /// `firma-cli`'s plugin catalogue.
    pub fn rules(&self) -> impl Iterator<Item = &RegisteredRule> {
        self.rules.values()
    }

    /// Registered resolver builds, ascending by id.
    pub fn resolvers(&self) -> impl Iterator<Item = &RegisteredResolver> {
        self.resolvers.values()
    }
}

fn check_version(
    id: &PluginId,
    required: &semver::VersionReq,
    available: &Version,
) -> Result<(), RegistryError> {
    if required.matches(available) {
        Ok(())
    } else {
        Err(RegistryError::VersionMismatch {
            id: id.0.clone(),
            required: required.to_string(),
            available: available.to_string(),
        })
    }
}

fn check_hash(
    id: &PluginId,
    declared: Option<&str>,
    registered: &str,
) -> Result<(), RegistryError> {
    match declared {
        Some(h) if h != registered => Err(RegistryError::HashMismatch { id: id.0.clone() }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use firma_core::{ComponentId, Delta, DeltaKindTag, Phase, RngKey, View};
    use semver::VersionReq;

    struct Noop;
    impl Rule for Noop {
        fn id(&self) -> PluginId {
            PluginId::new("testkit.noop")
        }
        fn version(&self) -> Version {
            Version::new(1, 2, 0)
        }
        fn phase(&self) -> Phase {
            Phase::ActMarket
        }
        fn reads(&self) -> &[ComponentId] {
            &[]
        }
        fn writes(&self) -> &[DeltaKindTag] {
            &[]
        }
        fn apply(&self, _v: &dyn View, _k: RngKey) -> Vec<Delta> {
            Vec::new()
        }
        fn assumption(&self) -> &str {
            "nothing happens; used only to exercise the registry"
        }
    }

    fn registry() -> Registry {
        let mut r = Registry::new();
        r.register_rule(RegisteredRule::new(
            "testkit.noop",
            Version::new(1, 2, 0),
            "hash-noop-v1",
            Box::new(|_p| Ok(Box::new(Noop) as Box<dyn Rule>)),
        ));
        r
    }

    fn spec(id: &str, req: &str, hash: Option<&str>) -> PluginRef {
        PluginRef {
            id: PluginId::new(id),
            version: VersionReq::parse(req).unwrap(),
            content_hash: hash.map(str::to_owned),
            params: JsonValue::Null,
        }
    }

    #[test]
    fn resolves_matching() {
        let r = registry();
        let rule = r
            .resolve_rule(&spec("testkit.noop", "^1.2", Some("hash-noop-v1")))
            .unwrap();
        assert_eq!(rule.id().as_str(), "testkit.noop");
    }

    fn resolve_err(spec: &PluginRef) -> RegistryError {
        match registry().resolve_rule(spec) {
            Ok(_) => panic!("expected an error"),
            Err(e) => e,
        }
    }

    #[test]
    fn unknown_rejected() {
        assert!(matches!(
            resolve_err(&spec("testkit.ghost", "*", None)),
            RegistryError::UnknownPlugin(_)
        ));
    }

    #[test]
    fn version_mismatch_rejected() {
        assert!(matches!(
            resolve_err(&spec("testkit.noop", ">=2.0.0", None)),
            RegistryError::VersionMismatch { .. }
        ));
    }

    #[test]
    fn hash_mismatch_rejected() {
        assert!(matches!(
            resolve_err(&spec("testkit.noop", "*", Some("wrong"))),
            RegistryError::HashMismatch { .. }
        ));
    }

    #[test]
    fn resolution_is_deterministic() {
        let r = registry();
        let a = r.resolve_rule(&spec("testkit.noop", "*", None)).unwrap();
        let b = r.resolve_rule(&spec("testkit.noop", "*", None)).unwrap();
        assert_eq!(a.id(), b.id());
        assert_eq!(a.version(), b.version());
    }

    fn noop_rule(id: &str) -> RegisteredRule {
        RegisteredRule::new(
            id,
            Version::new(1, 2, 0),
            "hash-noop-v1",
            Box::new(|_p| Ok(Box::new(Noop) as Box<dyn Rule>)),
        )
    }

    /// ADR 0013 — a `PluginId::numeric()` collision between two distinct plugins
    /// aborts registration. A real FNV-1a collision is computationally out of
    /// reach, so we seed the private `numeric()` table directly to simulate a
    /// different plugin having already claimed the slot, then register a real
    /// rule that hashes to it.
    #[test]
    #[should_panic(expected = "PluginId::numeric() collision")]
    fn numeric_collision_rejected() {
        let mut r = Registry::new();
        let claimed = PluginId::new("testkit.noop").numeric();
        r.rule_numeric_ids
            .insert(claimed, "some.other.plugin".to_string());
        // Same numeric() value, different id string -> collision panic.
        r.register_rule(noop_rule("testkit.noop"));
    }

    /// The ADR 0013 check must not fire on the case it is not meant to catch:
    /// registering the *same* id string twice still trips the pre-existing
    /// duplicate-id assertion, not the collision one.
    #[test]
    #[should_panic(expected = "duplicate rule registration")]
    fn duplicate_string_id_still_rejected_by_the_original_check() {
        let mut r = Registry::new();
        r.register_rule(noop_rule("testkit.noop"));
        r.register_rule(noop_rule("testkit.noop"));
    }

    /// Two genuinely distinct ids register cleanly — the guard does not
    /// false-positive on the normal path.
    #[test]
    fn distinct_ids_register_without_collision() {
        let mut r = Registry::new();
        r.register_rule(noop_rule("testkit.transfer"));
        r.register_rule(noop_rule("testkit.keyed_nudge"));
        assert_eq!(r.rules().count(), 2);
    }
}
