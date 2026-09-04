# ADR 0038 — `resource.constant` (explicit null) and `resource.patchy` (mean-reverting π^I)

**Status:** Accepted (2026-09-05)
**Phase:** 2 (Model), Stage 5
**Relates to:** manual §20.3 (`Resource` category — "resource dynamics";
`patchy` / `constant`, no formula given), §8.3 (`e_t = (π^I, π^O, Σ_t)` — no
pool-level field), §10.1 (phase 1 `environment`), §19.5 (conservation is an
exact `live == initial` check), §21.3 (`environment` stream), §20.4 (the
example config's `resource.patchy … regen: 0.04`); ADR-0034 (`rng_stream`)

## Context

§20.3 lists `resource.{patchy,constant}` as the `Resource`-category MVP
implementations with **no formula anywhere** for what "patchy" means. This is
the same underspecified-category situation as `Constraint`, `decision.random`,
and `Observation` — it needs a designed, stated mechanic.

The §20.4 example config shows `resource.patchy` with `params: {types:
[capital, input, legitimacy], regen: 0.04}`, which points at a *regenerating
availability pool*. But §8.3's environment vector has **no pool-level state**
distinct from price, and a genuine regenerating pool would need the kernel to
account sources/sinks against `initial_total` — it does not (§19.5 is an exact
`live_total == initial_total` check; an unpaired addition to the environment
pool aborts the run).

## Decision

### 1. `resource.constant` — the explicit null

A `Rule` in `environment` that **emits nothing**. Prices move only via shocks
and shaping. It exists and registers so that "the environment is static" is a
manifest-recorded choice, not the unstated absence of a plugin — every
Stage-2–4 config was implicitly `resource.constant`.

### 2. `resource.patchy` — a mean-reverting random walk on `π^I`

```
π^I(t+1) = π^I(t) + round( reversion·(baseline − π^I(t)) + σ·Normal(0,1) )
π^I(t+1) := max(π^I(t+1), floor)
```

drawn from the **`environment`** stream (§21.3: "resource dynamics"),
`purpose_tag = "resource_patchy"`. It emits one `AdjustGlobalInt {
input_price, Δ }` when `Δ ≠ 0`.

**"Patchy" here is temporal** — a fluctuating *cost of acquiring input*, which
stresses `solvency` (an expensive-input run of ticks drains `r^L`). Parameters
`baseline`, `reversion ∈ [0,1]`, `σ ≥ 0`, `floor ∈ [1, baseline]` — all
**required**, all **`[D]`, not calibrated** (same posture as `b_λ`, `P_q`).

**It is deliberately *not* a depleting/regenerating availability pool.** The
`regen`-pool reading of the §20.4 example is **deferred**; the trigger to
build it is the kernel gaining source/sink accounting (Phase 4). Only `π^I` is
walked (not `π^O`): the input-cost channel is the one RDT resource-munificence
story the MVP needs, and adding `π^O` doubles the parameter surface for no new
mechanism.

## Alternatives

- **A genuine availability pool with `regen` (the §20.4 example reading).**
  Needs `firma-domain` state that does not exist and kernel source/sink
  accounting that does not exist (`AdjustStock` to the env pool from nowhere
  aborts on the conservation check). A real Phase-4 item.
- **Walk both `π^I` and `π^O`.** More parameters, same mechanism. `π^I` alone
  is the minimal version that creates resource stress.
- **Spatially-varying price (a price per locality).** Requires `locality` to
  be a live consumer, which it is not (ADR-0037). Phase 4.
- **Leave `resource.constant` unregistered (absence = constant).** Same
  argument as ADR-0035 rejected for `observation.full`: an unstated default is
  worse than a manifest-recorded one.

## Consequences

- **Positive.** The environment can now be non-static without a shock —
  `resource.patchy` gives a background resource-munificence dynamic that
  stresses `solvency`, and `resource.constant` makes the static choice
  explicit. Grounded entirely in existing state (`π^I`, a `global_int`);
  conserving trivially (price is not a conserved resource).
- **Negative, accepted.** The mean-reverting-walk mechanic is invented and
  `[D]`; stated in `firma-plugin-resource` rustdoc. The `regen`-pool reading
  the manual's example implies is not built.
- **Neutral.** No new DeltaKind, no `firma-domain` state, no kernel change. No
  shipped numerical output changes (no golden/`phase1-smoke`/`phase2-smoke`
  config runs a resource plugin).

## Compliance

- `firma-plugin-resource` — `Constant` (emits nothing) / `Patchy`, phase
  `Environment`; `Patchy::rng_stream() == Some(Environment)`; all `Patchy`
  params required; the `floor` is enforced.
- Tests: `firma-plugin-resource` (constant silent; patchy reverts toward
  baseline, is silent at baseline with `σ = 0`, respects the floor over 200
  ticks, CRN-deterministic, params required); conformance
  `phase2_stage5_smoke_*` (`resource.patchy` moves `π^I` in a real run).

## Note

The manual's own example config is the strongest hint at what `patchy` "should"
be — a regenerating pool — and it is exactly the reading the current kernel
cannot support without new accounting machinery. Recording that gap here, and
shipping the temporal-price version that *is* supportable, is the honest split.
