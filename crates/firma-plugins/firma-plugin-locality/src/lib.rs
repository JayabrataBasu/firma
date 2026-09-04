//! `firma-plugin-locality` — the `Locality` category (manual §7.6, §34.7;
//! ADR 0037).
//!
//! One interface: `neighbours(agent, live, tick) -> Vec<AgentId>` (ascending,
//! deduplicated). Two MVP implementations:
//!
//! * [`WellMixed`] — every live agent is every other's neighbour. The null:
//!   `wellmixed` vs `network` is the clean "does interaction structure matter?"
//!   ablation (§34.7).
//! * [`Network`] — a configured **undirected** adjacency list, intersected with
//!   the live set each tick.
//!
//! ## Not a `Rule`, and not the relation graph (ADR 0037)
//!
//! `Locality` emits no [`Delta`](firma_core) and does not run in a phase — it
//! is a **query service**, so it does not implement `firma_core::Rule` and is
//! not registered in the kernel schedule. It is also a *different structure*
//! from `G_t` (`firma_domain::RelationGraph`): `G_t`'s `supply` / `alliance` /
//! `rivalry` edges are **realised economic relationships** that the model
//! creates and severs as dynamics unfold (§8.3, §13.1); a locality network is
//! the fixed map of **who can potentially interact at all** (§7.6). Conflating
//! them would make "sever a supply tie" also mean "can no longer interact",
//! which is not the model.
//!
//! **Nothing in the MVP consumes `neighbours()` yet.** No constraint, action,
//! decision, or shock filters or routes by locality — multi-firm rivalry and
//! H5 externality mechanics that need it are Phase 4 (§26.6). The interface and
//! both implementations are built and tested now because §27.2 lists Locality
//! as an MVP dimension (ADR 0019); wiring a consumer waits for one to exist.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use firma_core::AgentId;

/// This crate's build version.
#[must_use]
pub fn version() -> semver::Version {
    semver::Version::new(1, 0, 0)
}

/// Plugin ids and the declared content hash.
pub mod catalog {
    /// `locality.wellmixed`.
    pub const WELLMIXED_ID: &str = "locality.wellmixed";
    /// `locality.network`.
    pub const NETWORK_ID: &str = "locality.network";
    /// Declared content hash for the locality build.
    pub const CONTENT_HASH: &str = "phase2s5-locality-v1";
}

/// The §7.6 interface. Deliberately **not** `firma_core::Rule` (see module
/// docs). `neighbours` MUST return an ascending, deduplicated list and MUST NOT
/// include `agent` itself.
pub trait Locality: Send + Sync {
    /// Stable id, `"locality.<name>"`.
    fn id(&self) -> &'static str;

    /// `neighbours(i, t)` — the live agents `agent` can interact with this tick.
    fn neighbours(&self, agent: AgentId, live: &[AgentId], tick: u64) -> Vec<AgentId>;

    /// One plain-language sentence stating the interaction-structure assumption.
    /// MUST NOT be empty (§25.6 parity with `Rule`).
    fn assumption(&self) -> &str;
}

// --------------------------------------------------------------------------
// locality.wellmixed
// --------------------------------------------------------------------------

/// `locality.wellmixed` — the null interaction structure.
#[derive(Debug, Default, Clone, Copy)]
pub struct WellMixed;

impl Locality for WellMixed {
    fn id(&self) -> &'static str {
        catalog::WELLMIXED_ID
    }
    fn neighbours(&self, agent: AgentId, live: &[AgentId], _tick: u64) -> Vec<AgentId> {
        live.iter().copied().filter(|&a| a != agent).collect()
    }
    fn assumption(&self) -> &str {
        "Every firm can interact with every other firm; there is no interaction \
         structure, so any effect attributed to structure in a `network` run \
         is measured against this as the baseline (§7.6, §34.7)."
    }
}

// --------------------------------------------------------------------------
// locality.network
// --------------------------------------------------------------------------

/// Parameters for [`Network`]: an undirected edge list over agent ids.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkParams {
    /// Undirected edges `[a, b]`. Order within a pair does not matter; a
    /// self-loop `[a, a]` is ignored.
    pub edges: Vec<[u64; 2]>,
}

/// `locality.network` — a fixed configured adjacency, intersected with the live
/// set each tick.
pub struct Network {
    /// Sorted, deduplicated adjacency: for each agent, its neighbour ids.
    adjacency: std::collections::BTreeMap<u64, Vec<u64>>,
}

impl Network {
    /// Build from parameters.
    #[must_use]
    pub fn new(p: NetworkParams) -> Network {
        let mut adjacency: std::collections::BTreeMap<u64, Vec<u64>> =
            std::collections::BTreeMap::new();
        for [a, b] in p.edges {
            if a == b {
                continue;
            }
            adjacency.entry(a).or_default().push(b);
            adjacency.entry(b).or_default().push(a);
        }
        for v in adjacency.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
        Network { adjacency }
    }
}

impl Locality for Network {
    fn id(&self) -> &'static str {
        catalog::NETWORK_ID
    }
    fn neighbours(&self, agent: AgentId, live: &[AgentId], _tick: u64) -> Vec<AgentId> {
        let Some(adj) = self.adjacency.get(&agent.0) else {
            return Vec::new();
        };
        adj.iter()
            .copied()
            .map(AgentId)
            .filter(|a| *a != agent && live.binary_search(a).is_ok())
            .collect()
    }
    fn assumption(&self) -> &str {
        "Firms interact only along a fixed configured network of potential \
         ties; the structure is exogenous and does not change as economic \
         relationships form or break (§7.6)."
    }
}

// --------------------------------------------------------------------------
// construction
// --------------------------------------------------------------------------

/// A constructor from JSON parameters to a boxed [`Locality`].
pub type Ctor = fn(&serde_json::Value) -> Result<Box<dyn Locality>, String>;

/// Build `locality.wellmixed` (no parameters).
///
/// # Errors
/// Never — kept `Result` for a uniform signature.
pub fn wellmixed(_p: &serde_json::Value) -> Result<Box<dyn Locality>, String> {
    Ok(Box::new(WellMixed))
}

/// Build `locality.network`.
///
/// # Errors
/// Invalid [`NetworkParams`].
pub fn network(p: &serde_json::Value) -> Result<Box<dyn Locality>, String> {
    let params: NetworkParams = if p.is_null() {
        NetworkParams { edges: Vec::new() }
    } else {
        serde_json::from_value(p.clone()).map_err(|e| format!("locality.network params: {e}"))?
    };
    Ok(Box::new(Network::new(params)))
}

/// Catalogue entries — `(id, content_hash, ctor)`. Not a `Rule` registry;
/// `firma-cli` uses this only to construct-and-check a configured locality.
#[must_use]
pub fn catalogue() -> Vec<(&'static str, &'static str, Ctor)> {
    vec![
        (catalog::WELLMIXED_ID, catalog::CONTENT_HASH, wellmixed),
        (catalog::NETWORK_ID, catalog::CONTENT_HASH, network),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: Vec<AgentId>) -> Vec<u64> {
        v.into_iter().map(|a| a.0).collect()
    }

    #[test]
    fn wellmixed_is_everyone_else() {
        let live = [AgentId(0), AgentId(2), AgentId(5)];
        assert_eq!(ids(WellMixed.neighbours(AgentId(2), &live, 0)), vec![0, 5]);
        assert_eq!(
            ids(WellMixed.neighbours(AgentId(9), &live, 0)),
            vec![0, 2, 5]
        );
    }

    #[test]
    fn network_is_configured_adjacency_intersected_with_live() {
        let n = Network::new(NetworkParams {
            edges: vec![[0, 1], [1, 2], [2, 0], [3, 3]],
        });
        let live = [AgentId(0), AgentId(1)]; // 2 has died
        assert_eq!(ids(n.neighbours(AgentId(0), &live, 0)), vec![1]); // 2 filtered out
        assert_eq!(ids(n.neighbours(AgentId(1), &live, 0)), vec![0]);
        assert!(n.neighbours(AgentId(3), &live, 0).is_empty()); // self-loop ignored
        assert!(n.neighbours(AgentId(7), &live, 0).is_empty()); // not in graph
    }

    #[test]
    fn catalogue_constructs_and_has_assumptions() {
        for (id, _h, ctor) in catalogue() {
            let params = if id == "locality.network" {
                serde_json::json!({ "edges": [[0, 1]] })
            } else {
                serde_json::json!({})
            };
            let loc = ctor(&params).unwrap();
            assert_eq!(loc.id(), id);
            assert!(!loc.assumption().trim().is_empty());
        }
    }
}
