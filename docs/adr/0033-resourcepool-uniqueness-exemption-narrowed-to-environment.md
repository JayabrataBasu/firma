# ADR 0033 — The `ResourcePool` delta-uniqueness exemption is narrowed to the `Environment` target

**Status:** Accepted (2026-09-04)
**Phase:** 2 (Model), Stage 4 (follow-up)
**Relates to:** manual §19.4 (reconciliation, step 2), §19.5 ("Delta
uniqueness" invariant), §21.4 (exact integer conservation); ADR-0022
Decision 3 (`Delta`'s `Ord` / the guard key widened to include `slot`),
**ADR-0031** (which this narrows), ADR-0029 (`enforce` transfers a dead firm's
stocks to the pool)

## Context

ADR-0031 fixed a real bug: when one rule invocation distributes a paired
`ResourcePool` transfer across `N` agents, it emits `N` deltas against the
shared pool cell, and the per-rule uniqueness guard
(`(target.sort_key(), discriminant, slot)`, widened by ADR-0022 D3) rejected
the 2nd because `DeltaTarget::Environment::sort_key()` is a **fixed sentinel
with no agent in it** — the `N` claims are otherwise identical. The diagnosis
was correct and the `Environment` case genuinely needs the exemption.

**But the fix was written too broadly.** It exempted *every* `ResourcePool`
delta from the guard, including `ResourcePool` deltas whose target is a
specific `Agent`. That reopened a different hole the diagnosis never
mentioned: a single rule invocation, for **one agent**, that accidentally
emits two `ResourcePool` `AdjustStock` deltas to *that agent* for the *same
resource* in one phase — a duplicate-emission bug, not multi-agent
contention. `DeltaTarget::Agent(a)::sort_key()` carries `a`, so two different
agents never collide there; only a genuine double-emission for one agent
does, and under ADR-0031-as-applied both deltas passed the guard and were
applied.

### Evidence (the test, run against ADR-0031-as-applied)

`crates/firma-kernel/src/tests.rs::same_agent_duplicate_resourcepool_delta_is_rejected`
— one rule, phase `ActMarket`, agent 0 starting at `capital = 100`, emits the
paired pool transfer `−5` (agent) / `+5` (env) **twice**:

```
thread '…::same_agent_duplicate_resourcepool_delta_is_rejected' panicked:
same-agent duplicate ResourcePool delta was NOT rejected: step() = Ok,
agent 0 capital is now 90 (should be 95 after one −5 transfer; 90 ⇒ both
applied silently), env capital 10
```

`step()` returned `Ok`. Nothing downstream caught it: conservation held
(`100 = 90 + 10`), non-negativity held (`90 ≥ 0`), and the resolver
(`conflict.additive` / the test `NoResolver`) is a pass-through that never
inspects for duplicates. The agent was silently debited `10` instead of `5`.
**The gap is real.**

## Decision

The uniqueness-guard exemption for `ResourcePool` deltas applies **only when
the target is `DeltaTarget::Environment`**:

```rust
let pooled_env_claim = d.conflict_class == ConflictClass::ResourcePool
    && matches!(d.target, DeltaTarget::Environment);
if !d.kind.allows_repeat() && !pooled_env_claim {
    // …existing (target.sort_key(), discriminant, slot) uniqueness check…
}
```

- A `ResourcePool` delta to `DeltaTarget::Agent(a)` is back under the strict
  guard: a rule emitting two of them for one `(agent, resource)` in one phase
  is a `DuplicateDelta` abort, exactly as before ADR-0031.
- A `ResourcePool` delta to `DeltaTarget::Environment` remains exempt — this
  is the sentinel-collision case ADR-0031's Context actually diagnosed, and
  the §19.4 step-4 resolver aggregates those claims. Determinism is preserved
  by the same argument as ADR-0031 (ascending-`AgentId` emission + stable
  step-2 sort over the payload-blind `Ord`).
- `DeltaTarget::Global` is not exempted. `AdjustStock` cannot target `Global`
  (`apply_delta` → `UnsupportedTarget`), and the global scalar/list kinds are
  `Independent` in every rule that emits them; no `ResourcePool` + `Global`
  delta exists. If one is ever needed, it gets its own ADR.

### What ADR-0031 got wrong

Over-broad. Its Context, Decision alternatives, and Note are all about the
shared pool cell and multi-agent contention; the `Environment` predicate was
the intended scope throughout. Writing the guard condition as
`!= ConflictClass::ResourcePool` rather than "not an `Environment`-targeted
`ResourcePool` claim" silently also dropped the guard for agent-targeted
pooled deltas, which the ADR never argued for and which reintroduces a
silent-corruption path the guard existed to close.

## Alternatives

- **Keep ADR-0031 broad; rely on conservation + non-negativity to catch
  double-debits.** Rejected — the evidence above shows they do not: a paired
  transfer emitted twice is conserving and non-negative, so both invariants
  pass while the agent's balance is wrong. The guard is the only check that
  catches emission-shape bugs before they become state.
- **Drop the `conflict_class` term from `Delta`'s `Ord` so agent-side
  `ResourcePool` deltas sort by discriminant and the guard key is naturally
  unique per agent.** Rejected — re-touches the §19.4 step-2 comparator the
  golden trace depends on, and does not address the `Environment` sentinel
  collision (which is the case that actually needs help).
- **Give `DeltaTarget::Environment` a per-origin or per-claim `sort_key()` so
  the `N` multi-agent claims differ.** Rejected — the pool is deliberately one
  cell; distinguishing claims there just to satisfy the guard, then
  re-merging them in the resolver, is circular. Exempting the one target
  whose sentinel key makes the guard meaningless is the direct statement.

## Consequences

- **Positive.** The silent same-agent double-debit path is closed again. The
  exemption now says exactly what it means: "the shared pool cell's key
  cannot distinguish legitimate concurrent claims, so the guard does not
  apply *there*" — nothing wider.
- **Negative, accepted.** The guard condition is a two-part predicate rather
  than one comparison. Three lines, and it names the real reason.
- **Neutral.** No shipped numerical output changes. The `phase2-smoke` run is
  byte-identical (`run_id 0529c6bb…`, `event_log_sha256 9a476938…`) before and
  after the narrowing — its only repeated pool deltas are the `enforce`
  death-stock transfers, which target `Environment` and stay exempt. Golden
  trace and `phase1-smoke` unaffected (neither emits a repeated `ResourcePool`
  delta at all).

## Compliance

- `firma-kernel::run_phase` — the guard carries the `pooled_env_claim`
  two-part predicate and a comment citing this ADR.
- Tests:
  - `same_agent_duplicate_resourcepool_delta_is_rejected` (new) — one rule,
    one agent, two `ResourcePool` `AdjustStock` to the same `(agent,
    resource)` in one phase ⇒ `KernelError::DuplicateDelta`.
  - `resourcepool_transfers_then_removeagent_conserves` (ADR-0031/0032) — two
    *different* agents' `Environment`-targeted pool transfers in one phase are
    still accepted and both applied; conservation holds.
  - `phase2-smoke` conformance run — two agents dying of `compliance` in one
    `enforce` phase, each transferring residual stock to the pool; run
    completes, `run_id` / `event_log_sha256` unchanged from before the
    narrowing.
  - `widened_uniqueness_check_distinguishes_by_slot` (ADR-0022 D3) — the
    `Independent` double-write is still a `DuplicateDelta`.

## Note

ADR-0031's Context is a precise description of the `Environment`-target
sentinel collision; the bug was purely in translating that into the guard
condition — `!= ConflictClass::ResourcePool` is a wider statement than "the
pooled `Environment` claim", and the extra breadth was invisible until a test
went looking for it. The lesson matches ADR-0032's: when a one-line predicate
encodes an ADR's scope, the predicate has to say the same thing the prose
does, and a test has to pin the boundary on *both* sides — the case that must
pass and the case that must still fail.
