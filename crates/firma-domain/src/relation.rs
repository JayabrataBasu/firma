//! The typed directed relation graph `G_t` (manual §8.3, §7.2 primitive 6).
//!
//! **Minimal for Stage 2.** `contract` (§11.2 action 7) and `diversify`
//! (action 8) need exactly two queries — "does this firm already have a
//! `supply` partner?" and "how many distinct `supply` sources does it have?"
//! (dependence concentration, §13.1) — plus the ability to add an edge. Edge
//! *severance* (on an `obligation` violation, §9.1) and realised-inflow weights
//! for the dependence metric (§13.1) are later work; this type carries the
//! `weight` / `age` fields (§8.3 `(i,j,ℓ,w,age)`) so the shape is right, but
//! nothing in Stage 2 reads them.

use serde::{Deserialize, Serialize};

use crate::AgentId;

/// `ℓ` — the edge type (§8.3). `supply` edges additionally carry realised
/// inflows used for dependence (§13.1); `alliance` / `rivalry` are structural.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// A supplier relationship: `source` supplies `target`.
    Supply,
    /// A cooperative tie.
    Alliance,
    /// A competitive tie.
    Rivalry,
}

/// One directed edge `(source, target, kind, weight, age)` (§8.3).
///
/// `source` / `target` are `u64` rather than [`AgentId`] because `diversify`'s
/// partner is a **synthetic** supply-channel id, not a modelled agent (§16.4
/// forbids agent entry in the MVP; ADR 0023).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    /// The `i` endpoint — who the relationship is *from*.
    pub source: u64,
    /// The `j` endpoint — who the relationship is *to*.
    pub target: u64,
    /// `ℓ` — the edge type.
    pub kind: EdgeKind,
    /// `w` — realised weight (e.g. cumulative `supply` inflow, §13.1). Fixed at
    /// `contract` maturity (§11.2 "edge weight fixed"); `0` for a fresh
    /// `diversify` edge until inflows accrue (later work).
    pub weight: i64,
    /// `age` — ticks since the edge was created. Advanced by the environment
    /// phase (later work); `0` at creation.
    pub age: u64,
}

impl Edge {
    /// A fresh `supply` edge `source → target` at creation (`weight`, `age` 0).
    #[must_use]
    pub fn supply(source: u64, target: u64, weight: i64) -> Edge {
        Edge {
            source,
            target,
            kind: EdgeKind::Supply,
            weight,
            age: 0,
        }
    }
}

/// `G_t` — the relation graph, as the ordered edge list the kernel stores under
/// [`keys::RELATION_EDGES`](crate::keys::RELATION_EDGES).
///
/// Constructed per-query from the stored records; not itself persisted. Edge
/// order is insertion order (deterministic — append-only via `PushGlobalRecord`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RelationGraph {
    edges: Vec<Edge>,
}

impl RelationGraph {
    /// Build from stored canonical-JSON edge records (one [`Edge`] per string).
    ///
    /// # Errors
    /// The first record that is not a valid [`Edge`] JSON object, as a message.
    pub fn from_records(records: &[String]) -> Result<RelationGraph, String> {
        let mut edges = Vec::with_capacity(records.len());
        for (i, r) in records.iter().enumerate() {
            let e: Edge = serde_json::from_str(r)
                .map_err(|e| format!("relation_edges[{i}] is not a valid Edge: {e}"))?;
            edges.push(e);
        }
        Ok(RelationGraph { edges })
    }

    /// Every edge, in insertion order.
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Whether `firm` has at least one **incoming** `supply` edge — the
    /// `contract` precondition "has `supply` partner" (§11.2 action 7).
    #[must_use]
    pub fn has_supply_partner(&self, firm: AgentId) -> bool {
        self.edges
            .iter()
            .any(|e| e.kind == EdgeKind::Supply && e.target == firm.0)
    }

    /// The number of **distinct** `supply` sources feeding `firm` — the
    /// dependence-concentration input (§13.1). `diversify` raises this.
    #[must_use]
    pub fn supply_source_count(&self, firm: AgentId) -> usize {
        let mut sources: Vec<u64> = self
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Supply && e.target == firm.0)
            .map(|e| e.source)
            .collect();
        sources.sort_unstable();
        sources.dedup();
        sources.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supply_queries() {
        let g = RelationGraph {
            edges: vec![
                Edge::supply(7, 1, 0),
                Edge::supply(9, 1, 0),
                Edge::supply(7, 1, 0), // duplicate source
                Edge::supply(3, 2, 0),
            ],
        };
        assert!(g.has_supply_partner(AgentId(1)));
        assert!(!g.has_supply_partner(AgentId(5)));
        assert_eq!(g.supply_source_count(AgentId(1)), 2);
        assert_eq!(g.supply_source_count(AgentId(2)), 1);
        assert_eq!(g.supply_source_count(AgentId(5)), 0);
    }

    #[test]
    fn record_roundtrip() {
        let e = Edge::supply(1_000_000_042, 3, 5);
        let j = serde_json::to_string(&e).unwrap();
        let g = RelationGraph::from_records(&[j]).unwrap();
        assert_eq!(g.edges(), &[e]);
    }

    #[test]
    fn bad_record_is_an_error() {
        assert!(RelationGraph::from_records(&["not json".to_string()]).is_err());
    }
}
