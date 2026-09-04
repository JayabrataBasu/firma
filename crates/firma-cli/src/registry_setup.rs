//! Compile-time plugin registration (manual §20.1, ADR 0008).
//!
//! This is the one place that depends on both `firma-registry` and a plugin
//! crate — permitted because `firma-cli ← everything` (§18.1). The kernel and
//! the registry themselves never see a plugin crate.

use firma_plugin_testkit::catalog;
use firma_plugin_testkit::{
    AdditiveResolver, ForceAdjust, ForceAdjustParams, KeyedNudge, KeyedNudgeParams, KeyedPerAgent,
    KeyedPerAgentParams, Transfer, TransferParams,
};
use firma_registry::{RegisteredResolver, RegisteredRule, Registry};

/// Build the Phase 1 registry: the testkit rules and the additive resolver.
#[must_use]
pub fn standard_registry() -> Registry {
    let mut r = Registry::new();

    r.register_rule(RegisteredRule::new(
        catalog::TRANSFER_ID,
        catalog::version(),
        catalog::TRANSFER_HASH,
        Box::new(|params| {
            let p: TransferParams = serde_json::from_value(params.clone())
                .map_err(|e| format!("testkit.transfer params: {e}"))?;
            Transfer::new(p).map(|t| Box::new(t) as Box<dyn firma_core::Rule>)
        }),
    ));

    r.register_rule(RegisteredRule::new(
        catalog::KEYED_NUDGE_ID,
        catalog::version(),
        catalog::KEYED_NUDGE_HASH,
        Box::new(|params| {
            let p: KeyedNudgeParams = serde_json::from_value(params.clone())
                .map_err(|e| format!("testkit.keyed_nudge params: {e}"))?;
            KeyedNudge::new(p).map(|k| Box::new(k) as Box<dyn firma_core::Rule>)
        }),
    ));

    r.register_rule(RegisteredRule::new(
        catalog::KEYED_PER_AGENT_ID,
        catalog::version(),
        catalog::KEYED_PER_AGENT_HASH,
        Box::new(|params| {
            let p: KeyedPerAgentParams = serde_json::from_value(params.clone())
                .map_err(|e| format!("testkit.keyed_per_agent params: {e}"))?;
            KeyedPerAgent::new(p).map(|k| Box::new(k) as Box<dyn firma_core::Rule>)
        }),
    ));

    r.register_rule(RegisteredRule::new(
        catalog::FORCE_ADJUST_ID,
        catalog::version(),
        catalog::FORCE_ADJUST_HASH,
        Box::new(|params| {
            let p: ForceAdjustParams = serde_json::from_value(params.clone())
                .map_err(|e| format!("testkit.force_adjust params: {e}"))?;
            ForceAdjust::new(p).map(|f| Box::new(f) as Box<dyn firma_core::Rule>)
        }),
    ));

    r.register_resolver(RegisteredResolver::new(
        catalog::RESOLVER_ID,
        catalog::version(),
        catalog::RESOLVER_HASH,
        Box::new(|_params| {
            Ok(Box::new(AdditiveResolver::new()) as Box<dyn firma_core::ConflictResolver>)
        }),
    ));

    r
}

/// The **full Phase-2 model registry**: every `Rule` built so far — the six
/// market actions (`action.market.standard.*`), the three shaping actions +
/// `resolve_lagged` (`action.shaping.rdt_standard.*`), the decision procedure +
/// its null + the aspiration update (`decision.*`), the phase-7/8 rules
/// (`constraint.action_window`, `constraint.enforce`), and — Stage 5 — the
/// environment/observe-phase rules (`observation.{full,noisy,delayed}`,
/// `shock.{scheduled,stochastic}`, `resource.{constant,patchy}`) — plus the
/// additive resolver. The four constraint `MarginTerm`s do **not** register
/// (they are wired via `standard_constraints`); `locality.*` is **not** a
/// `Rule` (ADR 0037) and is reached through [`locality_catalogue`].
#[must_use]
pub fn model_registry() -> Registry {
    let mut r = Registry::new();
    let v = || semver::Version::new(1, 0, 0);

    // --- every `Rule` built so far: (id, hash, ctor: fn) tuples, all the same
    //     `fn(&serde_json::Value) -> Result<Box<dyn Rule>, String>` shape ---
    let entries = firma_plugin_action_market::registered()
        .into_iter()
        .chain(firma_plugin_action_shaping::registered())
        .chain(firma_plugin_decision::registered())
        .chain(firma_plugin_constraint::registered_rules())
        .chain(firma_plugin_observation::registered())
        .chain(firma_plugin_shock::registered())
        .chain(firma_plugin_resource::registered());
    for (id, hash, ctor) in entries {
        r.register_rule(RegisteredRule::new(id, v(), hash, Box::new(ctor)));
    }

    r.register_resolver(RegisteredResolver::new(
        catalog::RESOLVER_ID,
        catalog::version(),
        catalog::RESOLVER_HASH,
        Box::new(|_params| {
            Ok(Box::new(AdditiveResolver::new()) as Box<dyn firma_core::ConflictResolver>)
        }),
    ));

    r
}

/// The `Locality` catalogue (ADR 0037). `Locality` is a query service, not a
/// `Rule`, so it is not in [`model_registry`]; this is how a config's chosen
/// interaction structure is constructed and checked. Nothing consumes
/// `neighbours()` in the MVP.
#[must_use]
pub fn locality_catalogue() -> Vec<(&'static str, &'static str, firma_plugin_locality::Ctor)> {
    firma_plugin_locality::catalogue()
}

#[cfg(test)]
mod tests {
    use super::{model_registry, standard_registry};

    #[test]
    fn model_registry_assembles_every_phase2_rule() {
        // Construction exercises the ADR-0013 `numeric()` uniqueness guard
        // across *all* model rule ids at once — a collision would panic here.
        let r = model_registry();
        let ids: Vec<_> = r.rules().map(|x| x.id.0.clone()).collect();
        for want in [
            "decision.satisficing",
            "decision.random",
            "decision.aspiration_update",
            "action.market.standard.produce_regulated",
            "action.market.standard.invest_capability",
            "action.shaping.rdt_standard.resolve_lagged",
            "constraint.action_window",
            "constraint.enforce",
            "observation.full",
            "observation.noisy",
            "observation.delayed",
            "shock.scheduled",
            "shock.stochastic",
            "resource.constant",
            "resource.patchy",
        ] {
            assert!(ids.contains(&want.to_string()), "missing {want}");
        }
        // 6 market + 4 shaping (incl. resolve_lagged) + 3 decision + 2 constraint
        // + 3 observation + 2 shock + 2 resource = 22 (Stage 5). `locality.*` is
        // not a `Rule` (ADR 0037).
        assert_eq!(r.rules().count(), 22);
        assert_eq!(r.resolvers().count(), 1);
    }

    #[test]
    fn registry_has_the_phase1_plugins() {
        // Constructing the registry also exercises the ADR-0013 numeric()
        // uniqueness guard across every registered rule id.
        let r = standard_registry();
        let ids: Vec<_> = r.rules().map(|x| x.id.0.clone()).collect();
        assert!(ids.contains(&"testkit.transfer".to_string()));
        assert!(ids.contains(&"testkit.keyed_nudge".to_string()));
        assert!(ids.contains(&"testkit.keyed_per_agent".to_string()));
        assert!(ids.contains(&"testkit.force_adjust".to_string()));
        assert_eq!(r.rules().count(), 4);
        assert_eq!(r.resolvers().count(), 1);
    }
}
