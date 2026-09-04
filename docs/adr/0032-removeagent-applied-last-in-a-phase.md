# ADR 0032 — `RemoveAgent` is applied in a final sub-pass of a phase, after every value delta

**Status:** Accepted (2026-09-04)
**Phase:** 2 (Model), Stage 4
**Relates to:** manual §19.4 (reconciliation, step 5 "apply resolved deltas"),
§19.5 (conservation, non-negativity), §22.2 (log sufficiency); ADR-0022
Decision 3 (`Delta`'s `Ord`:
`(target_id, conflict_class, origin, discriminant, slot)`), ADR-0029
(`enforce`: dead-firm stock transfer then `RemoveAgent`), ADR-0031
(`ResourcePool` deltas exempt from the per-rule uniqueness guard)

## Context

ADR-0029 Decision 2 stated:

> Because `AdjustStock` (disc 0) sorts before `RemoveAgent` (disc 9), the
> transfers apply first; `RemoveAgent` then removes an agent with zero stocks
> and conservation is untouched.

**That claim is wrong.** `Delta`'s `Ord` (ADR-0022 Decision 3) compares
`conflict_class` *before* `kind.discriminant()`. The dead-firm stock transfers
`enforce` emits are paired agent↔environment `AdjustStock`s of class
`ResourcePool` (rank 1); the `RemoveAgent` is class `Independent` (rank 0). So
for the dying agent's target, `RemoveAgent` sorts **before** its own
stock-transfer deltas. The first real multi-agent run aborted:

```
firma: unknown/dead agent 3
```

— `RemoveAgent { Agent(3) }` applied first, then
`AdjustStock { Agent(3), input, −1 }` hit a removed agent
(`KernelError::UnknownAgent`).

Making the transfers `Independent` instead is not a fix: two firms dying in one
phase with the same non-zero stock would then emit two `Independent`
`AdjustStock { Environment, … }` deltas and trip the per-rule uniqueness guard
(the guard's `ResourcePool` carve-out is ADR-0031, and it is deliberately
*not* extended to `Independent`).

The real invariant is simpler than any sort tweak: **an agent removal
invalidates the target of every other delta that references that agent**, so it
must come last.

## Decision

`firma-kernel::run_phase`'s apply step (§19.4 step 5) runs in **two
order-preserving sub-passes over the already-sorted `resolved` vector**:

1. every delta whose kind is **not** `RemoveAgent`, in sorted order;
2. every `RemoveAgent` delta, in sorted order.

```rust
let is_remove = |d: &&Delta| matches!(d.kind, DeltaKind::RemoveAgent { .. });
for d in resolved.iter().filter(|d| !is_remove(d))
        .chain(resolved.iter().filter(is_remove)) {
    self.apply_delta(world, d)?;
    // …DeltaApplied, then AgentDied on a RemoveAgent…
}
```

Determinism is unchanged: `resolved` is sorted once (§19.4 step 2 order), and
each sub-pass is a stable filter of it. `DeltaApplied` / `AgentDied` events are
emitted in application order, so the event log now always shows a dead firm's
stock transfers *before* its `AgentDied`, on every path.

This **supersedes ADR-0029 Decision 2's ordering rationale only** — the
mechanism ADR-0029 describes (transfers first, then removal of a zeroed agent,
conservation intact, flow visible in the log) is exactly what this produces.
ADR-0029's other decisions stand.

### Scope

`RemoveAgent` is the only kind that needs this. `ReplaceGlobalList` (the edge
rewrite) targets `Global`, not the dying agent, and is fine in sort order.
`AgentBorn` is emitted by `step()` outside the phase loop and is unaffected.

## Alternatives

- **Change `Delta`'s `Ord` to sort `RemoveAgent` last** (e.g. discriminant
  before conflict_class, or a dedicated "terminal" rank). Rejected: it
  re-touches the §19.4 step-2 comparator that the golden trace and every
  determinism test depend on, for a one-kind special case an apply-time
  partition expresses more directly and reversibly.
- **`enforce` emits transfers as `Independent`.** Rejected — see Context (trips
  the uniqueness guard for simultaneous deaths).
- **`enforce` emits a single `RemoveAgent` carrying the stock deltas as a
  payload.** Rejected: bloats the variant, and the kernel would then interpret
  stock movement inside an agent-lifecycle delta (borderline A1), duplicating
  `AdjustStock`.
- **A distinct `Phase` for removals.** Over-engineered; removal is a
  within-`enforce` consequence, and a firm must be gone before phase 9
  (`record`) reads `live_agents()`.

## Consequences

- **Positive.** Multi-agent death works, with or without residual stock. The
  rule "agent removal is the last thing that happens in its phase" is a clean,
  one-line mental model and is now true structurally, not by discriminant
  coincidence.
- **Negative, accepted.** The apply loop is no longer a single pass over
  `resolved`. The two-pass form is three lines and fully ordered; the cost is
  one extra iteration of a short vector per phase.
- **Neutral.** No shipped numerical output changes — no golden/`phase1-smoke`
  config runs `enforce` or emits `RemoveAgent`; both hashes stay byte-identical.

## Compliance

- `firma-kernel::run_phase` — the two-sub-pass apply loop, with a comment
  citing this ADR.
- Tests: a kernel test where one phase emits, for one agent, a `ResourcePool`
  `AdjustStock` *and* a `RemoveAgent` — the stock delta applies (env receives
  it), then the agent is gone, and conservation holds; a two-agent
  simultaneous-death test with residual stock (the `phase2-smoke` conformance
  run covers this end to end — agents 0 and 1 die of `compliance` at tick 2
  each holding capital 533 + input 17, all transferred, totals unchanged).

## Note

ADR-0029 was written before `Delta`'s full `Ord` was in front of me, and the
"disc 0 sorts before disc 9" line read as obviously true. It was not — the
comparator had grown a `conflict_class` term (ADR-0022 D3) that inverts the
order for exactly this pairing. The lesson for future ADRs: when an ADR leans
on sort order, quote the comparator, don't paraphrase it.
