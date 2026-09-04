# ADR 0031 — `ResourcePool` deltas are exempt from the per-rule delta-uniqueness guard

**Status:** Accepted (2026-09-04). The exemption is **narrowed** by
[ADR-0033](0033-resourcepool-uniqueness-exemption-narrowed-to-environment.md) —
it applies only to `ResourcePool` deltas targeting `Environment`, not to those
targeting a specific `Agent`. The `Environment`-target case this ADR's Context
diagnoses is unchanged; the over-broad part (agent-targeted `ResourcePool`
deltas) is restored to the guard.
**Phase:** 2 (Model), Stage 4
**Relates to:** manual §19.4 (the reconciliation algorithm, step 2 "ties
impossible… a plugin emits at most one delta per (target, kind) per phase"),
§19.5 (the "Delta uniqueness" invariant), §21.4 (exact integer conservation),
§15.6 / §34.4 (rationing is a resolver's job); ADR-0016 (shaping deltas are
`Independent`, pooled market/enforce deltas are `ResourcePool`), ADR-0022
Decision 3 (the guard widened to include `slot`), ADR-0029 (`enforce`
transfers a dead firm's stocks to the environment pool)

## Context

The first real multi-agent, multi-plugin run (Stage 4 Part E) aborted at
tick 0:

```
plugin "action.market.standard.produce_regulated" emitted two deltas
for one (target, kind) in a phase
```

`action.market.standard.*` moves resources between an agent and the shared
**environment pool** with a *paired* `AdjustStock` (`+x` to the agent, `−x` to
the pool), conflict class `ResourcePool`, so that the transfer conserves by
construction and a rationing resolver can settle contention (§19.4 step 4,
§15.6). When **two** agents take `produce_regulated` in the same tick, the one
rule instance emits two deltas against the same pool cell —
`(Environment, AdjustStock, "capital")` — and the per-rule uniqueness guard
(`firma-kernel::run_phase`, widened by ADR-0022 Decision 3 to
`(target, discriminant, slot)`) rejects the second.

This is not a plugin bug. Distributing a paired transfer across `N` agents
*is* `N` legitimate claims on the shared pool. The same shape appears in
`action.shaping.rdt_standard.*` (`cost_deltas` → pool, per acting agent) and
in `constraint.enforce` (ADR-0029: each dying or fined firm transfers stock to
the pool). The `enforce` test `two_agents_dying_share_one_replace_global_list`
only passed because its fixture agents held **zero** stock, so no transfer
deltas were emitted — the guard was never exercised on this path.

The manual's stated rationale for the uniqueness rule is narrow: §19.4 step 2
needs the sort to be **total and tie-free**. It is a determinism device, not a
semantic-correctness device — and `ResourcePool` contention has its own,
explicitly-plugin-owned settlement mechanism downstream.

## Decision

**In `firma-kernel::run_phase`, the per-rule delta-uniqueness guard is skipped
for any delta whose `conflict_class` is `ResourcePool`.** The guard still
applies in full to `Independent` deltas (two `Independent` writes to one cell
from one rule remain a `DuplicateDelta` abort).

```rust
if !d.kind.allows_repeat() && d.conflict_class != ConflictClass::ResourcePool {
    // …existing (target, discriminant, slot) uniqueness check…
}
```

Determinism is preserved without the guard because:

1. Every rule that emits pooled deltas iterates `view.live_agents()` —
   ascending `AgentId`, `BTreeMap`-backed (§19.2) — so the deltas are pushed
   in a deterministic order.
2. The §19.4 step-2 sort (`slice::sort`, **stable**) orders by `Delta`'s
   `Ord`, which is **payload-blind**
   (`(target_id, conflict_class, origin, discriminant, slot)` — ADR-0022 D3).
   Two pooled deltas from one rule to one cell are `Ord`-equal, and the stable
   sort keeps them in ascending-`AgentId` emission order.
3. The registered `ConflictResolver` for the `ResourcePool` group then
   aggregates them (§19.4 step 4). The Phase-1 `conflict.additive` resolver
   applies them additively; any conserving resolver (§34.4) is order-insensitive
   on a commutative-sum pool by construction.

Conservation (§21.4, §19.5) is unaffected: each pooled delta is still one half
of a `+x / −x` pair, and the post-reconciliation invariant sweep is unchanged.

## Alternatives

- **Aggregate pool-side deltas inside each rule** — emit per-agent agent-side
  deltas plus **one** summed `Environment` delta per resource per phase.
  Rejected: it pushes a fiddly, easy-to-forget accumulation into every current
  and future action/enforcement plugin (market, shaping, enforce, and the
  Stage-5 shock plugin), duplicating what the resolver exists to do, for no
  determinism or conservation gain over the stable-sort argument above.
- **Exempt only `AdjustStock` + `ResourcePool`.** Narrower, but the class is
  already the precise signal ("this contends for a shared pool, a resolver
  settles it"); keying on the kind as well adds a special case without
  removing a real risk (a rule emitting two pooled `AdjustGlobalInt` to one
  field is still resolver-aggregated and stable-sorted).
- **Leave the guard, make the run fail.** Rejected: it makes the paired-pool
  pattern — which the manual itself prescribes for conserving transfers —
  unusable the moment two agents act alike, i.e. always.

## Consequences

- **Positive.** Multi-agent runs work. The paired-transfer pattern is now
  uniformly correct across market, shaping, and enforce without per-plugin
  bookkeeping.
- **Negative, accepted.** A rule can now emit two *different* pooled writes to
  one cell (e.g. `+30` and `+50` to the pool) without a kernel diagnostic.
  These are summed by the resolver and the result is deterministic, but a
  genuine plugin bug of this exact shape would no longer be caught at the
  guard — it would surface (if at all) as a conservation or non-negativity
  abort. Judged acceptable: the pattern is intentional and conserving by
  construction, and `Independent` writes — where a double-write really is a
  bug — keep the strict guard.
- **Manual discrepancy (flagged).** Manual §19.4 step 2 and the §19.5 "Delta
  uniqueness" invariant state the guard unconditionally ("a plugin emits at
  most one delta per (target, kind) per phase"). They need a PATCH-level
  clarification: the guard is `(target, kind, slot)` per rule for `Independent`
  deltas; `ResourcePool` deltas from one rule are permitted to repeat and are
  aggregated at step 4, with pre-sort order fixed by ascending-`AgentId`
  emission + the stable sort. Logged as a PROGRESS.md open question for the
  owner; per CLAUDE.md the manual is authoritative and this ADR records the
  deviation pending that PATCH.

## Compliance

- `firma-kernel::run_phase` — the uniqueness guard carries the
  `&& d.conflict_class != ConflictClass::ResourcePool` condition and a comment
  citing this ADR.
- `no-hashmap-iteration` already covers every pooled-delta emitter's
  `live_agents()` loop (all are in `SIM_PATH_SRC`), so the ascending-`AgentId`
  emission premise is lint-enforced.
- Tests: a kernel test that one rule emitting two same-cell `ResourcePool`
  `AdjustStock` deltas is accepted and both are applied (sum lands, conservation
  holds); the existing `Independent` double-write test still aborts with
  `DuplicateDelta`; the Stage-4 `phase2-smoke` conformance run (two agents
  doing `produce_regulated`, two dying with non-zero stock) executes and
  conserves.

## Note

The uniqueness guard was designed in Phase 1, when every rule was a
single-transfer testkit rule and "one delta per cell" and "one claim per
contended pool" were the same statement. They stop being the same the moment a
single rule acts for a whole population — which is exactly what the real
decision/action/enforcement rules do. The conflict *class* was the right place
to have drawn the line all along: `Independent` means "these must not collide",
`ResourcePool` means "these are expected to, and a resolver will sort it out".
