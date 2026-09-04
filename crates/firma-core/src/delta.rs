//! [`Delta`] and its total order (manual §6.4, §19.4).
//!
//! A rule proposes deltas; only the reconciler applies them (§17 A2). The
//! ordering below is the one the reconciler sorts by in §19.4 step 2:
//! `(target_id, conflict_class, origin_plugin_id, kind_discriminant, slot)` —
//! the `slot` key was added in Stage 2 (ADR 0022) so a rule may emit two
//! `AdjustStock` deltas to one agent for different resources. It is a **total**
//! order (lexicographic over totally-ordered fields), hence also a strict weak
//! ordering — checked by `tests::order_is_a_strict_weak_ordering` (§25.3).

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, PluginId, ResourceKind};

/// What a [`Delta`] acts on (manual §19.4 `DeltaTarget`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaTarget {
    /// A specific live agent.
    Agent(AgentId),
    /// The shared environment (e.g. a resource pool).
    Environment,
    /// Global state not owned by any agent or the environment.
    Global,
}

impl DeltaTarget {
    /// The `target_id` used as the first sort key in §19.4 step 2.
    ///
    /// Agents sort by their id; `Environment` and `Global` are given fixed
    /// sentinels above every real agent id so their relative order is stable and
    /// they never collide with an agent.
    #[must_use]
    pub fn sort_key(&self) -> (u8, u64) {
        match self {
            DeltaTarget::Agent(a) => (0, a.0),
            DeltaTarget::Environment => (1, 0),
            DeltaTarget::Global => (2, 0),
        }
    }
}

/// The contention group a delta belongs to (manual §19.4). Deltas are grouped by
/// this and the registered [`ConflictResolver`](crate::ConflictResolver) for the
/// group decides the outcome.
///
/// Phase 1 has two: [`ConflictClass::Independent`] deltas cannot contend and are
/// applied additively; [`ConflictClass::ResourcePool`] deltas go through the
/// resolver. Phase 2 is expected to add classes (this stays an enum until then).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConflictClass {
    /// No contention possible; apply as proposed.
    Independent,
    /// Draws on a shared pool; the registered resolver settles contention
    /// (proportional rationing, priority, keyed-random, … — §20.3).
    ResourcePool,
}

impl ConflictClass {
    fn rank(self) -> u8 {
        match self {
            ConflictClass::Independent => 0,
            ConflictClass::ResourcePool => 1,
        }
    }
}

/// The payload of a [`Delta`] (manual §19.4). Phase 1 had one variant
/// (`AdjustStock`, ADR 0004); Phase 2 Stage 2 adds eight for the decide→act
/// hand-off and the §11 domain-state mutations (ADR 0022). New variants are
/// breaking and require an ADR (§18.2, §34.0).
///
/// The kernel stores the string-keyed variants **opaquely** — it never
/// interprets `"capability"` / `"theta_limit"` / `"lagged_effects"`, exactly as
/// it never interprets a `ResourceKind` string (§7.2). The meaning lives in the
/// plugins and in `firma-domain::keys`.
///
/// `Eq` is implemented manually (below): the `f64` `delta` fields would block
/// the derive. The invariant that makes the manual impl sound — every real
/// payload is finite (never `NaN` / `±∞`) — is **enforced by the kernel**, not
/// merely assumed: `firma-kernel`'s `run_phase` rejects a non-finite
/// `AdjustAgentReal` / `AdjustGlobalReal` with `KernelError::NonFiniteDelta`
/// before it is sorted or applied (§21.4; ADR 0022 Decision 2). `Delta` needs
/// `Eq` for its `Ord` impl (§19.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeltaKind {
    /// disc 0 — add `amount` (may be negative) to the target's conserved stock
    /// of `resource` (§21.4).
    AdjustStock {
        /// Which resource.
        resource: ResourceKind,
        /// Signed change, integer units.
        amount: i64,
    },
    /// disc 1 — **set** a named per-agent integer scalar (the `set` counterpart
    /// to [`AdjustAgentInt`](Self::AdjustAgentInt)'s `add`). `Agent` target.
    /// The decide→act hand-off uses this to write
    /// `firma_domain::keys::SELECTED_ACTION` (§12.3 → §10.1 phases 4–5;
    /// ADR 0022); the kernel routes by the carried `field` like every other
    /// keyed variant and never names the key.
    SetAgentInt {
        /// Opaque field key (see `firma-domain::keys`).
        field: String,
        /// The new value (last write wins).
        value: i64,
    },
    /// disc 2 — add `delta` to a named per-agent real scalar (e.g.
    /// `"capability"`). `Agent` target.
    AdjustAgentReal {
        /// Opaque field key (see `firma-domain::keys`).
        field: String,
        /// Signed change.
        delta: f64,
    },
    /// disc 3 — add `delta` to a named per-agent integer scalar (e.g.
    /// `"obligation"`). `Agent` target.
    AdjustAgentInt {
        /// Opaque field key.
        field: String,
        /// Signed change.
        delta: i64,
    },
    /// disc 4 — add `delta` to a named global real scalar (e.g.
    /// `"theta_limit"`). Additive combination (ADR 0016). `Global` target.
    AdjustGlobalReal {
        /// Opaque field key.
        field: String,
        /// Signed change.
        delta: f64,
    },
    /// disc 5 — add `delta` to a named global integer scalar (e.g.
    /// `"theta_q"`). `Global` target.
    AdjustGlobalInt {
        /// Opaque field key.
        field: String,
        /// Signed change.
        delta: i64,
    },
    /// disc 6 — append an opaque canonical-JSON record to a named per-agent
    /// list (e.g. `"lagged_effects"`). `Agent` target. Repeats allowed.
    PushAgentRecord {
        /// Opaque list key.
        list: String,
        /// Canonical JSON, schema owned by the plugin.
        record_json: String,
    },
    /// disc 7 — append an opaque canonical-JSON record to a named global list
    /// (e.g. `"relation_edges"`). `Global` target. Repeats allowed.
    PushGlobalRecord {
        /// Opaque list key.
        list: String,
        /// Canonical JSON.
        record_json: String,
    },
    /// disc 8 — replace a named per-agent list wholesale (e.g. `resolve_lagged`
    /// draining `"lagged_effects"`). Set-list semantics; `Agent` target.
    ReplaceAgentList {
        /// Opaque list key.
        list: String,
        /// The new contents, canonical JSON per entry.
        records_json: Vec<String>,
    },
    /// disc 9 — remove the target agent and its per-agent keyed state (§9.1
    /// death; §10.1 phase 8; ADR 0029). `Agent` target. Carries **no** edge
    /// logic and does **no** conservation rebase — the `enforce` rule
    /// transfers the agent's stocks with paired `AdjustStock` deltas (which
    /// sort first) and rewrites the relation-edge list with `ReplaceGlobalList`
    /// beforehand (ADR 0017). The kernel emits `AgentDied { cause: reason }`.
    RemoveAgent {
        /// Why (e.g. `"solvency"`, `"compliance"`, `"solvency+compliance"`) —
        /// an opaque string for the event log.
        reason: String,
    },
    /// disc 10 — replace a named global list wholesale (the `Global` sibling of
    /// [`ReplaceAgentList`](Self::ReplaceAgentList)). `Global` target. Used by
    /// `enforce` to rewrite `"relation_edges"` after severances / deaths in one
    /// delta for the whole phase (ADR 0029).
    ReplaceGlobalList {
        /// Opaque list key.
        list: String,
        /// The new contents, canonical JSON per entry.
        records_json: Vec<String>,
    },
}

impl DeltaKind {
    /// The discriminant used as the fourth sort key in §19.4 step 2 (0–10).
    #[must_use]
    pub fn discriminant(&self) -> u16 {
        match self {
            DeltaKind::AdjustStock { .. } => 0,
            DeltaKind::SetAgentInt { .. } => 1,
            DeltaKind::AdjustAgentReal { .. } => 2,
            DeltaKind::AdjustAgentInt { .. } => 3,
            DeltaKind::AdjustGlobalReal { .. } => 4,
            DeltaKind::AdjustGlobalInt { .. } => 5,
            DeltaKind::PushAgentRecord { .. } => 6,
            DeltaKind::PushGlobalRecord { .. } => 7,
            DeltaKind::ReplaceAgentList { .. } => 8,
            DeltaKind::RemoveAgent { .. } => 9,
            DeltaKind::ReplaceGlobalList { .. } => 10,
        }
    }

    /// The sub-key that distinguishes deltas sharing `(target, discriminant)`
    /// (ADR 0022 Decision 3) — the resource / field / list name, or `"agent"`
    /// for `RemoveAgent` (one per agent per phase). Two `AdjustStock`s to one
    /// agent for different resources differ here.
    #[must_use]
    pub fn slot(&self) -> &str {
        match self {
            DeltaKind::AdjustStock { resource, .. } => resource.as_str(),
            DeltaKind::SetAgentInt { field, .. }
            | DeltaKind::AdjustAgentReal { field, .. }
            | DeltaKind::AdjustAgentInt { field, .. }
            | DeltaKind::AdjustGlobalReal { field, .. }
            | DeltaKind::AdjustGlobalInt { field, .. } => field,
            DeltaKind::PushAgentRecord { list, .. }
            | DeltaKind::PushGlobalRecord { list, .. }
            | DeltaKind::ReplaceAgentList { list, .. }
            | DeltaKind::ReplaceGlobalList { list, .. } => list,
            DeltaKind::RemoveAgent { .. } => "agent",
        }
    }

    /// Whether a rule may emit more than one of this `(target, disc, slot)` in
    /// one phase (ADR 0022 Decision 3) — `true` only for the append kinds,
    /// which are exempt from the §19.5 delta-uniqueness check.
    #[must_use]
    pub fn allows_repeat(&self) -> bool {
        matches!(
            self,
            DeltaKind::PushAgentRecord { .. } | DeltaKind::PushGlobalRecord { .. }
        )
    }

    /// The payload-free tag, for a rule's [`Rule::writes`](crate::Rule::writes)
    /// declaration.
    #[must_use]
    pub fn tag(&self) -> DeltaKindTag {
        match self {
            DeltaKind::AdjustStock { .. } => DeltaKindTag::AdjustStock,
            DeltaKind::SetAgentInt { .. } => DeltaKindTag::SetAgentInt,
            DeltaKind::AdjustAgentReal { .. } => DeltaKindTag::AdjustAgentReal,
            DeltaKind::AdjustAgentInt { .. } => DeltaKindTag::AdjustAgentInt,
            DeltaKind::AdjustGlobalReal { .. } => DeltaKindTag::AdjustGlobalReal,
            DeltaKind::AdjustGlobalInt { .. } => DeltaKindTag::AdjustGlobalInt,
            DeltaKind::PushAgentRecord { .. } => DeltaKindTag::PushAgentRecord,
            DeltaKind::PushGlobalRecord { .. } => DeltaKindTag::PushGlobalRecord,
            DeltaKind::ReplaceAgentList { .. } => DeltaKindTag::ReplaceAgentList,
            DeltaKind::RemoveAgent { .. } => DeltaKindTag::RemoveAgent,
            DeltaKind::ReplaceGlobalList { .. } => DeltaKindTag::ReplaceGlobalList,
        }
    }
}

/// A payload-free [`DeltaKind`] discriminant. A rule declares which tags it may
/// emit; emitting an undeclared kind is a kernel error (manual §20.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DeltaKindTag {
    /// [`DeltaKind::AdjustStock`].
    AdjustStock,
    /// [`DeltaKind::SetAgentInt`].
    SetAgentInt,
    /// [`DeltaKind::AdjustAgentReal`].
    AdjustAgentReal,
    /// [`DeltaKind::AdjustAgentInt`].
    AdjustAgentInt,
    /// [`DeltaKind::AdjustGlobalReal`].
    AdjustGlobalReal,
    /// [`DeltaKind::AdjustGlobalInt`].
    AdjustGlobalInt,
    /// [`DeltaKind::PushAgentRecord`].
    PushAgentRecord,
    /// [`DeltaKind::PushGlobalRecord`].
    PushGlobalRecord,
    /// [`DeltaKind::ReplaceAgentList`].
    ReplaceAgentList,
    /// [`DeltaKind::RemoveAgent`].
    RemoveAgent,
    /// [`DeltaKind::ReplaceGlobalList`].
    ReplaceGlobalList,
}

// SAFETY-of-contract: the `f64` fields in `DeltaKind` are finite model deltas,
// never `NaN`, so derived `PartialEq` is reflexive/total. `Delta`'s `Ord` impl
// (§19.4) requires the `Eq` marker.
impl Eq for DeltaKind {}
impl Eq for Delta {}

/// A proposed state change (manual §19.4). Rules return `Vec<Delta>`; they never
/// mutate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    /// What it acts on.
    pub target: DeltaTarget,
    /// The change.
    pub kind: DeltaKind,
    /// Its contention group.
    pub conflict_class: ConflictClass,
    /// The plugin that proposed it.
    pub origin: PluginId,
}

impl PartialOrd for Delta {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Delta {
    /// The §19.4 step-2 order, with a `slot` tie-breaker added in Stage 2
    /// (ADR 0022 Decision 3):
    /// `(target_id, conflict_class, origin_plugin_id, kind_discriminant, slot)`.
    ///
    /// `origin` is compared by its **string** (not the FNV projection) so the
    /// order is total — no hash collision can make two distinct plugins compare
    /// equal. `slot` distinguishes e.g. two `AdjustStock` deltas from one rule
    /// to one agent for different resources (which Phase 1 could not produce).
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.target
            .sort_key()
            .cmp(&other.target.sort_key())
            .then_with(|| self.conflict_class.rank().cmp(&other.conflict_class.rank()))
            .then_with(|| self.origin.0.cmp(&other.origin.0))
            .then_with(|| self.kind.discriminant().cmp(&other.kind.discriminant()))
            .then_with(|| self.kind.slot().cmp(other.kind.slot()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AgentId, PluginId};

    fn d(target: DeltaTarget, kind: DeltaKind, origin: &str) -> Delta {
        Delta {
            target,
            kind,
            conflict_class: ConflictClass::Independent,
            origin: PluginId::new(origin),
        }
    }

    fn stock(name: &str, amount: i64) -> DeltaKind {
        DeltaKind::AdjustStock {
            resource: ResourceKind(name.to_owned()),
            amount,
        }
    }

    #[test]
    fn discriminants_are_the_documented_range_and_unique() {
        let kinds = [
            stock("x", 1),
            DeltaKind::SetAgentInt {
                field: "f".into(),
                value: 0,
            },
            DeltaKind::AdjustAgentReal {
                field: "f".into(),
                delta: 1.0,
            },
            DeltaKind::AdjustAgentInt {
                field: "f".into(),
                delta: 1,
            },
            DeltaKind::AdjustGlobalReal {
                field: "f".into(),
                delta: 1.0,
            },
            DeltaKind::AdjustGlobalInt {
                field: "f".into(),
                delta: 1,
            },
            DeltaKind::PushAgentRecord {
                list: "l".into(),
                record_json: "{}".into(),
            },
            DeltaKind::PushGlobalRecord {
                list: "l".into(),
                record_json: "{}".into(),
            },
            DeltaKind::ReplaceAgentList {
                list: "l".into(),
                records_json: vec![],
            },
            DeltaKind::RemoveAgent {
                reason: "solvency".into(),
            },
            DeltaKind::ReplaceGlobalList {
                list: "l".into(),
                records_json: vec![],
            },
        ];
        let discs: Vec<u16> = kinds.iter().map(DeltaKind::discriminant).collect();
        assert_eq!(discs, (0..=10).collect::<Vec<_>>());
    }

    #[test]
    fn allows_repeat_only_for_the_append_kinds() {
        assert!(DeltaKind::PushAgentRecord {
            list: "l".into(),
            record_json: "{}".into()
        }
        .allows_repeat());
        assert!(DeltaKind::PushGlobalRecord {
            list: "l".into(),
            record_json: "{}".into()
        }
        .allows_repeat());
        assert!(!stock("x", 1).allows_repeat());
        assert!(!DeltaKind::ReplaceAgentList {
            list: "l".into(),
            records_json: vec![]
        }
        .allows_repeat());
        assert!(!DeltaKind::SetAgentInt {
            field: "selected_action".into(),
            value: 3
        }
        .allows_repeat());
    }

    #[test]
    fn slot_distinguishes_two_adjuststock_to_one_agent() {
        // `acquire_input` proposes AdjustStock(capital) and AdjustStock(input)
        // for the same agent from the same rule — Phase 1's (target, disc) key
        // could not tell them apart; `slot` does.
        let a = DeltaTarget::Agent(AgentId(1));
        let cap = d(
            a.clone(),
            stock("capital", -2),
            "action.market.standard.acquire_input",
        );
        let inp = d(a, stock("input", 1), "action.market.standard.acquire_input");
        assert_ne!(cap.cmp(&inp), std::cmp::Ordering::Equal);
        // deterministic: "capital" < "input" lexically.
        assert!(cap < inp);
    }

    #[test]
    fn adjuststock_sorts_before_setagentint_by_discriminant() {
        let a = DeltaTarget::Agent(AgentId(1));
        let s = d(a.clone(), stock("capital", 1), "p");
        let sel = d(
            a,
            DeltaKind::SetAgentInt {
                field: "selected_action".into(),
                value: 4,
            },
            "p",
        );
        assert!(s < sel); // disc 0 < disc 1
    }

    #[test]
    fn phase1_pair_order_is_unchanged() {
        // Two AdjustStock deltas that already differ on `origin` sort by origin,
        // exactly as in Phase 1 — the new `slot` key is never consulted.
        let a = DeltaTarget::Agent(AgentId(2));
        let from_x = d(a.clone(), stock("capital", -1), "x.rule");
        let from_y = d(a, stock("capital", 1), "y.rule");
        assert!(from_x < from_y);
    }

    #[test]
    fn f64_payload_equality_is_reflexive_for_finite_values() {
        // Two deltas with bit-identical finite f64 payloads must be equal, and
        // must also compare Equal under the sort order. This is the property the
        // manual `impl Eq for Delta` claims (delta.rs) and that DT-4/DT-5's
        // `assert_eq!` on event lists / `World` relies on for the new variants.
        let a = d(
            DeltaTarget::Global,
            DeltaKind::AdjustGlobalReal {
                field: "theta_limit".into(),
                delta: 0.1_f64,
            },
            "action.shaping.rdt_standard.lobby",
        );
        let b = a.clone();
        assert_eq!(a, b);
        assert!(!(a != b));
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);

        // A different finite payload is `!=` (so the event log / uniqueness
        // reconstruction can still tell two writes apart) …
        let c = d(
            DeltaTarget::Global,
            DeltaKind::AdjustGlobalReal {
                field: "theta_limit".into(),
                delta: 0.2_f64,
            },
            "action.shaping.rdt_standard.lobby",
        );
        assert_ne!(a, c);
        // … but note it compares Equal under `cmp` (same target/class/origin/
        // disc/slot). `Ord`'s "cmp==Equal iff ==" contract is therefore not
        // strictly satisfied for the f64-payload variants; this is documented
        // and is harmless only because the reconciler's uniqueness rule stops
        // two such deltas from coexisting and `sort` is stable. See the
        // Stage-2 review notes.
        assert_eq!(a.cmp(&c), std::cmp::Ordering::Equal);
    }

    #[test]
    fn nan_payload_breaks_partialeq_reflexivity_but_ordering_is_immune() {
        // LAYER NOTE: this is a `firma-core`-level unit property — it pins what
        // is *constructible* in isolation (a `DeltaKind` value literally can
        // hold a NaN, and if it does, `PartialEq` is not reflexive). It does
        // NOT contradict the kernel-level guarantee added in the Stage-2 review
        // (Fix 1): `firma-kernel::run_phase` now *rejects* any non-finite
        // `AdjustAgentReal` / `AdjustGlobalReal` with `KernelError::
        // NonFiniteDelta` before it is sorted or applied, so a NaN never
        // reaches a `sort()` / `World` / event-log comparison in a real run.
        // The two tests guard different layers: constructible-in-principle here,
        // rejected-at-the-boundary there
        // (`firma-kernel::tests::non_finite_real_delta_is_rejected_atomically`).
        let nan = d(
            DeltaTarget::Agent(AgentId(1)),
            DeltaKind::AdjustAgentReal {
                field: "capability".into(),
                delta: f64::NAN,
            },
            "p",
        );

        // (1) `impl Eq` is a lie under NaN: equality is NOT reflexive.
        #[allow(clippy::eq_op)]
        let reflexive = nan == nan;
        assert!(!reflexive, "NaN payload: `nan == nan` is false");

        // (2) The sort order is NaN-immune: `Delta::cmp` never inspects the
        //     f64 payload, so it always returns a real `Ordering` and `sort()`
        //     stays total and deterministic even with a NaN present.
        let other = d(
            DeltaTarget::Agent(AgentId(1)),
            DeltaKind::SetAgentInt {
                field: "selected_action".into(),
                value: 0,
            },
            "p",
        );
        assert_eq!(nan.cmp(&nan), std::cmp::Ordering::Equal);
        // disc 2 (AdjustAgentReal) vs disc 1 (SetAgentInt), same
        // target/class/origin ⇒ ordered by discriminant, NaN irrelevant.
        assert_eq!(nan.cmp(&other), std::cmp::Ordering::Greater);
        assert_eq!(other.cmp(&nan), std::cmp::Ordering::Less);
        let proj = |v: &[Delta]| v.iter().map(|d| d.kind.discriminant()).collect::<Vec<_>>();
        let mut v = vec![nan.clone(), other.clone()];
        v.sort(); // must not panic
        let mut v2 = vec![other.clone(), nan.clone()];
        v2.sort();
        // (can't `assert_eq!` the vecs directly — they hold a NaN — so compare
        //  the ordering projection.)
        assert_eq!(proj(&v), vec![1, 2]);
        assert_eq!(
            proj(&v),
            proj(&v2),
            "sort deterministic regardless of input order"
        );

        // (3) Consequence: a NaN in a `Delta` makes any `assert_eq!` over a
        //     containing `World` / event list FAIL loudly (never a false
        //     green), and `Delta`/`DeltaKind` must never be used as a hash-set
        //     or btree-set element while this impl stands.
        assert!(nan != nan.clone());
    }

    #[test]
    fn order_is_a_strict_weak_ordering() {
        let a = DeltaTarget::Agent(AgentId(1));
        let items = [
            d(a.clone(), stock("capital", 1), "a"),
            d(a.clone(), stock("input", 1), "a"),
            d(
                a.clone(),
                DeltaKind::SetAgentInt {
                    field: "selected_action".into(),
                    value: 0,
                },
                "a",
            ),
            d(a.clone(), stock("capital", 1), "b"),
            d(DeltaTarget::Environment, stock("capital", 1), "a"),
            d(
                a,
                DeltaKind::AdjustAgentInt {
                    field: "obligation".into(),
                    delta: -1,
                },
                "a",
            ),
        ];
        for x in &items {
            assert_eq!(x.cmp(x), std::cmp::Ordering::Equal, "reflexive");
            for y in &items {
                assert_eq!(
                    x.cmp(y) == std::cmp::Ordering::Equal,
                    y.cmp(x) == std::cmp::Ordering::Equal,
                    "antisymmetric equality",
                );
                for z in &items {
                    if x < y && y < z {
                        assert!(x < z, "transitive");
                    }
                }
            }
        }
    }
}
