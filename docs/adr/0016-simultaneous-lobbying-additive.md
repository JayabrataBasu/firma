# ADR 0016 — Simultaneous lobbying is additive; shaping deltas are `Independent` class

**Status:** Accepted (2026-09-03)
**Phase:** 2 (Model), Stage 0
**Resolves:** manual §16.3 item 3 ("Simultaneous lobbying. Additive assumed;
contested resolution is more realistic (Phase 4).")
**Relates to:** §9.4 (γ properties), §11.2 (`lobby`, `contract`, `diversify`),
§11.3 (mandatory shaping properties), §13.2, §19.4 (`ConflictClass`,
reconciliation), §20.3 (`ConflictResolver` category), §35.3 TC-003, ADR 0015,
ADR 0003

## Context

§16.3 item 3 asks how two firms' `lobby` actions combine when both succeed in
the same tick. `lobby` on success does `θ_limit += δ_θ` (§11.2); with global θ
(ADR 0015) both successes target the *same* parameter. The manual's stated
default is additive; "contested resolution" is flagged as "more realistic
(Phase 4)".

This needs settling before Stage 2 writes `action.shaping.rdt_standard`,
because the reconciler (§19.4) needs to know which `ConflictClass` a shaping
delta carries, and whether a `ConflictResolver` (§20.3) is involved.

**Tracing against §19.4's `ConflictClass` definitions (from Phase 1).** The
Phase 1 `firma-core::ConflictClass` enum has:

- `Independent` — "No contention possible; apply as proposed." The kernel
  applies these additively, with no resolver (`firma-kernel` `run_phase`:
  `ConflictClass::Independent => resolved.extend(group)`).
- `ResourcePool` — "Draws on a shared pool; the registered resolver settles
  contention" (proportional rationing, priority, keyed-random — §20.3).

A shaping delta `θ_limit += δ_θ` is **not drawing from a finite pool**. Two
firms each pushing `θ_limit` up by `δ_θ` are not competing for a scarce
resource that must be rationed; under the additive assumption the result is
simply `θ_limit += 2·δ_θ`. That is *exactly* what the kernel already does for
`Independent`-class deltas — it sums them (§19.4 step 5, additive apply). So
**additive simultaneous lobbying is the `Independent` semantics; it needs no
new `ConflictClass` and no `ConflictResolver`.**

The "contested" alternative is a genuinely different mechanism — e.g. the
regulator only moves `θ_limit` so far regardless of how many firms lobby
(a cap on aggregate shaping per tick), or firms lobbying in opposite
directions net out, or lobbying effort is rationed against a regulator
"attention" pool. *That* would need a new class (`ShapingContest` or similar)
and a resolver. It is Phase 4 (§20.6 `regulator.strategic`).

## Decision

**Phase 2 (and Phase 3) use additive simultaneous shaping.**

1. Each successful `lobby` enqueues, and later (`resolve_lagged`, §10.1 phase
   6) matures into, a delta `θ_limit += δ_θ` tagged **`ConflictClass::Independent`**.
2. Each successful `contract` matures into `θ_Q += δ_Q` (global cap, ADR 0015),
   also **`Independent`**, plus a per-firm `q += q_0` and a supply-edge fix
   (also `Independent` — different targets).
3. `diversify` on success adds a `supply` edge — a graph mutation, `Independent`.
4. The `constrain` phase (§10.1 phase 7) folds every matured shaping delta and
   every regulatory shock into θ by summation, then recomputes `u` and `h`.
5. **No new `ConflictClass` variant. No `ConflictResolver` for shaping.**
   Additivity *is* the `Independent` reconciler path.

Contested/capped shaping (a `ShapingContest` class + resolver, opposed-direction
netting, or a regulator attention pool) is deferred to Phase 4, bundled with
`regulator.strategic` (§20.6) and the two-sided lobbying experiment E9 (§28.2).

## Rejected alternatives

- **Route shaping deltas through `ConflictClass::ResourcePool` + the
  `proportional` resolver now.** Rejected: there is no pool. Proportional
  rationing scales claims so their sum fits a capacity; θ has no capacity, and
  scaling two `+δ_θ` claims to `+δ_θ` total would be the *contested* model
  smuggled in without a decision or an `assumption()` string saying so.

- **Add a `ShapingContest` class in Phase 2 but have its resolver be the
  identity (additive) for now.** Rejected: a class whose only resolver is the
  identity is `Independent` with extra ceremony (same reasoning as ADR 0006's
  rejected "two subunits" middle path), and it would create a false impression
  that contested lobbying is "handled".

- **Cap aggregate `Δθ_limit` per tick now** (the simplest "contested" flavour).
  Rejected for Phase 2: the cap value is an unspecified parameter with
  research-design weight (it directly bounds H3's "shaping availability"
  mechanism), and §16.3 explicitly files contested resolution under Phase 4.

## Consequences

**Positive.**
- The reconciler is unchanged from Phase 1 — shaping deltas ride the existing
  `Independent` path. One fewer moving part in the Phase 2 kernel.
- γ's three mandatory properties (§9.4: costly, lagged, uncertain) are
  orthogonal to this decision — they live in the shaping *action* plugin
  (cost at commitment, drawn lag, `p_max < 1`), not in reconciliation.
- Composes cleanly with ADR 0015: additive combination only *matters* because
  θ is shared; with per-firm θ there would be nothing to combine.

**Negative, accepted, and a TC-003 (§35.3) scope line.**
- Additive lobbying means shaping effect scales linearly with the number of
  firms lobbying in the same direction — "successful lobbying always relaxes
  in the intended direction" (TC-003 *Introduced*: "Monotone effect"). Real
  regulatory processes saturate, reverse, and are contested. **Any Phase 2–4
  claim about lobbying MUST cite TC-003 and MUST NOT be presented as a claim
  about competitive lobbying** (§35.3 scope condition).
- Under rivalry with additive shaping, coordinated lobbying is unrealistically
  effective. E8/H5 results are interpreted with that caveat; E9 (Phase 4)
  exists to relax it.

**Neutral.**
- **No shipped numerical output changes.** No shaping action, no `resolve_lagged`
  or `constrain` phase logic, no new `ConflictClass` exists yet. Phase 1 golden
  trace, `phase1-smoke` run id, and all 55 workspace tests are unaffected.
- `firma-core::ConflictClass` is **not** modified by this ADR (a new variant
  would be a breaking change requiring its own ADR per §18.2 — this ADR
  deliberately avoids that by using the existing `Independent`).

## Compliance

- Stage 2's `action.shaping.rdt_standard` emits θ-shift deltas with
  `conflict_class: ConflictClass::Independent`; its `assumption()` states
  "simultaneous shaping combines additively; contested resolution is not
  modelled (Phase 4)".
- VT-7 (§25.4) verifies costly/lagged/uncertain for the shaping plugin —
  unaffected by this ADR.
- A Stage 2 test: two firms both successfully lobby in one tick → `θ_limit`
  increases by exactly `2·δ_θ`.
- No `firma-core::ConflictClass` change; the `no-*` lint set and the Phase 1
  `Delta` total-order tests are untouched.

## Note

The load-bearing sentence here is the `ConflictClass` trace: *additivity is
already what `Independent` does*, so "additive simultaneous lobbying" costs
zero new machinery. The moment someone wants the regulator to push back —
a cap, a contest, opposed directions netting — that is a new `ConflictClass`
+ resolver + ADR, and it is Phase 4. This ADR's job is to make sure Stage 2
does not accidentally build the Phase 4 version, or accidentally route shaping
through the rationing resolver and call it "contested" without deciding to.
