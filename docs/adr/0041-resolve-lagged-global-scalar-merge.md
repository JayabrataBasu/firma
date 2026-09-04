# ADR 0041 — `resolve_lagged` combines multiple firms' global-scalar effects into one delta

**Status:** Accepted (2026-09-06)
**Phase:** 2 (Model), Stage 6 (discovered, not part of the VT-8/SC scope)
**Relates to:** manual §10.1 phase 6 (`resolve_lagged`), §19.4 (reconciliation
/ the per-rule uniqueness guard), §11.2 (`lobby` / `contract` maturity),
§9.4 (θ is a shared surface); **ADR-0016** (simultaneous lobbying is
additive), ADR-0023 (`Effect`, `LaggedEffectResolver`), ADR-0031 / ADR-0033
(the reconciler's per-rule `(target, kind, slot)` guard and its narrow
`Environment`-pool carve-out)

## Context

Running an Arm-C-style config (`decision.random` with shaping in the
repertoire — needed for the SC-4 comparison in ADR-0040) aborted:

```
firma: plugin "action.shaping.rdt_standard.resolve_lagged" emitted two deltas
for one (target, kind) in a phase
```

`LaggedEffectResolver::apply` iterates live agents and, for each, calls
`Effect::deltas_at_maturity`. A matured `Effect::Lobby` yields
`AdjustGlobalReal { field: theta_limit, delta: δ_θ }` (target `Global`); a
matured `Effect::Contract` yields `AdjustGlobalInt { field: theta_q, … }`.
When **two firms' lobby effects mature in the same tick**, the one rule emits
two `AdjustGlobalReal` deltas to the same `(Global, theta_limit)` cell — and
the reconciler's per-rule uniqueness guard rejects the second.

This is the **same class** of bug ADR-0031 fixed for the shock plugin
(a rule acting for `N` agents making `N` claims on one global cell). ADR-0033
deliberately narrowed that carve-out to `Environment`-targeted `ResourcePool`
deltas only — `Independent` `Global`-scalar deltas stay under the guard — so
`resolve_lagged` is not covered and must fix it itself.

The bug was latent because no shipped config produces two simultaneous
maturing global effects: `decision.satisficing` almost never selects a shaping
action (ADR-0040 SC finding), and the smoke configs have at most one lobbying
firm.

## Decision

`LaggedEffectResolver::apply` ends by passing its delta vector through
`merge_global_scalar_adjusts`, which **sums every `AdjustGlobalReal` /
`AdjustGlobalInt` delta targeting the same `(Global, field)` into a single
delta**, preserving the order and identity of every other delta (per-agent
`AdjustAgentReal { capability }`, `AdjustAgentInt { obligation }`,
`PushGlobalRecord { relation_edges }` (an append — `allows_repeat`), and the
one `ReplaceAgentList { lagged_effects }` drain per agent).

Summing is the **correct semantics**, not a workaround: ADR-0016 already
states "simultaneous lobbying is additive" — `θ_limit += 2·δ_θ` when two firms
succeed in the same tick. The reconciler simply rejected the two separate
deltas before they could be added; folding them in the plugin produces the
identical world change ADR-0016 prescribes.

The merge lives in the plugin (not a wider reconciler carve-out) because:

- It is domain-shaped — "these two θ shifts are the same additive quantity"
  is a §9.4 fact, and A1 keeps such facts out of the kernel.
- ADR-0033 already decided the reconciler guard stays strict for
  `Independent` `Global` deltas; loosening it would re-open the
  same-agent-double-write hole ADR-0033 closed.
- Only `resolve_lagged` has this shape among current rules (the shock plugin
  handles its own `ACTIVE_SHOCKS` rewrite as one delta; `enforce` targets
  `Environment` for pooled transfers, covered by ADR-0031).

## Alternatives

- **Extend ADR-0031's carve-out to `Independent` `Global`-scalar deltas from
  one rule.** Re-opens the ADR-0033 hole (a buggy rule emitting two
  `AdjustGlobalInt` to one field would pass silently). Rejected for the same
  reason ADR-0033 gave.
- **`Effect::deltas_at_maturity` returns per-agent deltas only; a separate
  pass aggregates.** That *is* this decision — the aggregation pass is
  `merge_global_scalar_adjusts`.
- **Emit one `AdjustGlobalReal` per firm but on distinct synthetic field
  names, summed offline.** Breaks every consumer that reads `theta_limit`.
  Absurd.

## Consequences

- **Positive.** Multi-firm shaping runs work. The θ surface stays additive
  across simultaneous lobbyists exactly as ADR-0016 specifies.
- **Negative, accepted.** `resolve_lagged` is no longer a pure map over agents
  — it has a fold at the end. ~20 lines, and it only touches `Global` scalar
  adjusts.
- **Neutral.** No shipped numerical output changes — no golden /
  `phase1-smoke` / `phase2-smoke` / `phase2-stage5-smoke` config matures two
  global effects in one tick (verified: all four `run_id` /
  `event_log_sha256` byte-identical).

## Compliance

- `firma_plugin_action_shaping::merge_global_scalar_adjusts`, called at the
  end of `LaggedEffectResolver::apply`.
- Test `two_firms_maturing_lobby_combine_into_one_theta_delta`: two agents'
  matured `Lobby { theta_limit_delta: 0.10 / 0.05 }` ⇒ **one**
  `AdjustGlobalReal { theta_limit, +0.15 }`, plus one drain per agent.

## Note

ADR-0031's Note said the conflict *class* was the right place to draw the line
— `Independent` means "these must not collide", `ResourcePool` means "a
resolver sorts it out". This case is a third thing: `Independent` deltas that
*are* the same additive quantity by domain semantics (§9.4, ADR-0016). The
plugin, which knows that, does the sum; the guard, which does not, stays
strict.
