//! `Λ` lagged-effect entries and their maturity deltas (manual §8.1, §10.1
//! phase 6, §11; ADR 0023).
//!
//! An [`Effect`] is enqueued — with its success already decided (ADR 0023
//! Decision 2) — at commitment (`act_market` / `act_shaping`) and applied at
//! `maturity_tick` by the `resolve_lagged` phase. [`Effect::deltas_at_maturity`]
//! is the single place the maturity mutation is expressed, so the resolver rule
//! carries no per-variant logic.

use serde::{Deserialize, Serialize};

use firma_core::{ConflictClass, Delta, DeltaKind, DeltaTarget, PluginId};

use crate::relation::Edge;
use crate::{keys, AgentId};

/// A matured effect. Every variant that can *fail* carries an `applied` flag
/// frozen at commitment (ADR 0023 Decision 2); a failed attempt is still
/// enqueued so it is visible in the event log (§22.2), and applies nothing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    /// `invest_capability` maturity (§11.1 action 4). Always "succeeds" — a
    /// market action, not a probabilistic shaping one — so no `applied` flag.
    CapabilityGain {
        /// `δ_c` — the capability increment (§16.1 default `0.05`).
        delta: f64,
    },
    /// `lobby` maturity (§11.2 action 6): `θ_limit += δ_θ` on success.
    Lobby {
        /// Whether the attempt succeeded (decided at commitment).
        applied: bool,
        /// `δ_θ` — the shift applied to `θ_limit` on success.
        theta_limit_delta: f64,
    },
    /// `contract` maturity (§11.2 action 7): `θ_Q += δ_Q`, `q += q_0`, and a
    /// `supply` edge to `source` with fixed weight, on success.
    Contract {
        /// Whether the attempt succeeded.
        applied: bool,
        /// `δ_Q` — the shift applied to `θ_Q`.
        theta_q_delta: i64,
        /// `q_0` — obligation added to the acting firm (the double edge, §11.2).
        q0: i64,
        /// The supply partner the edge is fixed to.
        source: u64,
    },
    /// `diversify` maturity (§11.2 action 8): a new `supply` edge from a fresh
    /// synthetic `source` (ADR 0023), on success.
    Diversify {
        /// Whether the attempt succeeded.
        applied: bool,
        /// The fresh synthetic supply-channel id.
        source: u64,
    },
}

impl Effect {
    /// The deltas that fire when this effect matures for `agent`, proposed by
    /// `origin`. Empty for a failed shaping attempt.
    ///
    /// `capability_now` is the acting firm's current `c`, used only by
    /// [`Effect::CapabilityGain`] to clamp the increment at the `c ≤ 1` ceiling
    /// (§8.1 domain; ADR 0018 — no decay, hard ceiling). All other variants
    /// ignore it.
    #[must_use]
    pub fn deltas_at_maturity(
        &self,
        agent: AgentId,
        origin: &PluginId,
        capability_now: f64,
    ) -> Vec<Delta> {
        let agent_d = |kind| Delta {
            target: DeltaTarget::Agent(agent),
            kind,
            conflict_class: ConflictClass::Independent,
            origin: origin.clone(),
        };
        let global_d = |kind| Delta {
            target: DeltaTarget::Global,
            kind,
            conflict_class: ConflictClass::Independent,
            origin: origin.clone(),
        };
        match *self {
            Effect::CapabilityGain { delta } => {
                let clamped = (capability_now + delta).clamp(0.0, 1.0) - capability_now;
                if clamped == 0.0 {
                    return Vec::new();
                }
                vec![agent_d(DeltaKind::AdjustAgentReal {
                    field: keys::CAPABILITY.to_owned(),
                    delta: clamped,
                })]
            }
            Effect::Lobby {
                applied,
                theta_limit_delta,
            } => {
                if !applied {
                    return Vec::new();
                }
                vec![global_d(DeltaKind::AdjustGlobalReal {
                    field: keys::THETA_LIMIT.to_owned(),
                    delta: theta_limit_delta,
                })]
            }
            Effect::Contract {
                applied,
                theta_q_delta,
                q0,
                source,
            } => {
                if !applied {
                    return Vec::new();
                }
                vec![
                    global_d(DeltaKind::AdjustGlobalInt {
                        field: keys::THETA_Q.to_owned(),
                        delta: theta_q_delta,
                    }),
                    agent_d(DeltaKind::AdjustAgentInt {
                        field: keys::OBLIGATION.to_owned(),
                        delta: q0,
                    }),
                    global_d(DeltaKind::PushGlobalRecord {
                        list: keys::RELATION_EDGES.to_owned(),
                        record_json: edge_json(Edge::supply(source, agent.0, q0)),
                    }),
                ]
            }
            Effect::Diversify { applied, source } => {
                if !applied {
                    return Vec::new();
                }
                vec![global_d(DeltaKind::PushGlobalRecord {
                    list: keys::RELATION_EDGES.to_owned(),
                    record_json: edge_json(Edge::supply(source, agent.0, 0)),
                })]
            }
        }
    }
}

fn edge_json(e: Edge) -> String {
    // `Edge` is plain data with no float NaN risk; serialisation cannot fail.
    serde_json::to_string(&e).expect("Edge serialises")
}

/// One `Λ` queue entry: an [`Effect`] and the tick it fires
/// (§8.1 `(maturity_tick, Effect)`). Stored as canonical JSON under
/// [`keys::LAGGED_EFFECTS`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaggedRecord {
    /// The tick at which [`Self::effect`] fires (§10.1 phase 6: `== t`).
    pub maturity_tick: u64,
    /// The effect, with success already decided (ADR 0023).
    pub effect: Effect,
}

impl LaggedRecord {
    /// Construct.
    #[must_use]
    pub fn new(maturity_tick: u64, effect: Effect) -> LaggedRecord {
        LaggedRecord {
            maturity_tick,
            effect,
        }
    }

    /// Whether this entry fires at `tick` (§10.1 phase 6 — exact equality; the
    /// MVP never skips a tick).
    #[must_use]
    pub fn matures_at(&self, tick: u64) -> bool {
        self.maturity_tick == tick
    }

    /// Serialise to the canonical JSON stored in the queue.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("LaggedRecord serialises")
    }

    /// Parse one stored queue entry.
    ///
    /// # Errors
    /// If `s` is not a valid [`LaggedRecord`] JSON object.
    pub fn from_json(s: &str) -> Result<LaggedRecord, String> {
        serde_json::from_str(s).map_err(|e| format!("invalid LaggedRecord: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_shaping_applies_nothing() {
        let id = PluginId::new("action.shaping.rdt_standard.lobby");
        let e = Effect::Lobby {
            applied: false,
            theta_limit_delta: 0.1,
        };
        assert!(e.deltas_at_maturity(AgentId(1), &id, 0.5).is_empty());
    }

    #[test]
    fn successful_lobby_shifts_theta_limit() {
        let id = PluginId::new("action.shaping.rdt_standard.lobby");
        let e = Effect::Lobby {
            applied: true,
            theta_limit_delta: 0.1,
        };
        let d = e.deltas_at_maturity(AgentId(1), &id, 0.5);
        assert_eq!(d.len(), 1);
        assert!(matches!(
            &d[0].kind,
            DeltaKind::AdjustGlobalReal { field, delta }
                if field == keys::THETA_LIMIT && (*delta - 0.1).abs() < 1e-12
        ));
    }

    #[test]
    fn capability_gain_clamps_at_one() {
        let id = PluginId::new("action.market.standard.invest_capability");
        let e = Effect::CapabilityGain { delta: 0.05 };
        // Below the ceiling: full increment.
        let d = e.deltas_at_maturity(AgentId(1), &id, 0.90);
        match &d[0].kind {
            DeltaKind::AdjustAgentReal { delta, .. } => assert!((*delta - 0.05).abs() < 1e-12),
            _ => panic!(),
        }
        // At the ceiling: no delta at all.
        assert!(e.deltas_at_maturity(AgentId(1), &id, 1.0).is_empty());
        // Straddling: only the part below 1.0.
        match &e.deltas_at_maturity(AgentId(1), &id, 0.98)[0].kind {
            DeltaKind::AdjustAgentReal { delta, .. } => assert!((*delta - 0.02).abs() < 1e-12),
            _ => panic!(),
        }
    }

    #[test]
    fn contract_is_double_edged() {
        let id = PluginId::new("action.shaping.rdt_standard.contract");
        let e = Effect::Contract {
            applied: true,
            theta_q_delta: 20,
            q0: 5,
            source: 3,
        };
        let d = e.deltas_at_maturity(AgentId(1), &id, 0.5);
        // θ_Q up, obligation up, supply edge added — all three.
        assert_eq!(d.len(), 3);
        assert!(d.iter().any(|x| matches!(&x.kind,
            DeltaKind::AdjustAgentInt { field, delta } if field == keys::OBLIGATION && *delta == 5)));
    }

    #[test]
    fn record_json_roundtrip() {
        let r = LaggedRecord::new(
            9,
            Effect::Diversify {
                applied: true,
                source: 1_000_000_042,
            },
        );
        assert_eq!(LaggedRecord::from_json(&r.to_json()).unwrap(), r);
    }
}
