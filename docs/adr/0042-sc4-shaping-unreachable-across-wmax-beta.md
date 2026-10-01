# ADR 0042 — SC-4/SC-5: shaping is unreachable under `decision.satisficing` across the full `w_max × β` sweep

**Status:** Accepted (2026-09-06). **Evidence (a) — the `0 / 400` claim and its "`h` stays well above `h_crit`" premise — superseded by [ADR-0056](0056-sc4-probe-claims-correction.md) (DRAFT, pending owner review).** The Decision's `GOAL`-branch reasoning is not superseded.
**Phase:** 2 (Model), Stage 6 (follow-up)
**Relates to:** **ADR-0040** (VT-8 + the SC-1…6 finding — this supplements its
SC-4/SC-5 row), manual §12.3 Step 5 (satisficing selection, the fallback),
§16.1 (`β ∈ {0, 0.5, 1, 2, 4}`, `w_max ∈ {3, 6, 9}` sweeps), §16.2 SC-4/SC-5,
§30.4 Arm B

## Context

ADR-0040 reported SC-4 (shaping attempted > 5 % of decisions) and SC-5
(shaping success 0.10–0.60) unreachable under `decision.satisficing` —
**0 shaping selections across ~50 000 decisions** — over ~24 configs that
varied `θ_limit`, `θ_cap`, `θ_Q`, `L_W`, `n`, and shocks. Its reasoning was
correct ("shaping never satisfices … it is only ever the scan *fallback*, and
the fallback reaches an admissible earlier market action first") but it also
quoted the manual's `with small w_eff they are never reached`, and it did
**not sweep `β` or `w_max`** — the two parameters §16.1 lists for exactly this
kind of search, and the two a "with *small* `w_eff`" framing points at. This
ADR closes that gap with a targeted probe.

## Decision

**SC-4/SC-5's "structurally unreachable" conclusion stands, now confirmed
across the full §16.1 `β × w_max` sweep. The effect is width-*independent*,
not a small-`w_eff` artefact.** ADR-0040's SC-4/SC-5 row is unchanged in
substance; this ADR is the additional evidence, and ADR-0040's `Status` line
points here.

### Why `w_max` and `β` cannot open the shaping channel

Read `firma_plugin_decision::select` (ADR-0040): `pick` is set **only** when
`satisfices(a, focus)` is true; otherwise the returned action is
`order.iter().find(admissible)` — the first admissible action in the **full**
focus priority order, **unbounded by `w`**.

1. **A shaping action never satisfices any `GOAL`.** `satisfices(a, GOAL(j))`
   for `a ∈ {6,7,8}` evaluates `dc.lookahead(a) = shaping_cost_step(κ_a, s)`,
   which subtracts the cost and changes nothing else: `Δv_1 = −κ_a < 0 ≤ ς_1`;
   `Δv_2 = Δv_3 = 0 < ς_j` (the shortfall is positive by definition of
   `GOAL(j)`). So a shaping action can never be the satisficing `pick`,
   regardless of how deep the scan goes.
2. **`β` is inert for a `GOAL` firm.** `focus = GOAL(j)` requires
   `h ≥ h_crit`, and `ψ(h) = 1` for `h ≥ h_crit` **for every `β`** (the
   formula, not an approximation). So `w_eff = w_max` for any `GOAL` firm at
   any `β` — there is no "small `w_eff`" to be had in `GOAL` focus.
3. **`w_max` only changes which *market* action satisfices first**, never
   whether shaping is picked — because shaping is never the `pick`, and the
   fallback ignores `w`.
4. Therefore shaping is selected **iff** it is the fallback, which requires
   **every market action ahead of it in the `GOAL` order to be inadmissible**.
   Under `SURVIVAL`, `hold` (always admissible) precedes every shaping action
   in the order `[1,3,5,2,0,4,6,7,8]`, so `SURVIVAL` can never fall back to
   shaping. Under `GOAL(1)` `[2,1,3,6,…]`, `GOAL(2)` `[4,1,3,2,6,…]`,
   `GOAL(3)` `[5,3,1,7,…]`, shaping precedes `hold` — but only if the market
   actions before it are all inadmissible.

### The evidence

**`validation::sc4_shaping_selected_only_as_the_goal_fallback_never_by_the_scan`**
— drives `select` directly across `β ∈ {0, 0.5, 1, 2, 4} × w_max ∈ {3, 6, 9}`:

| firm | `GOAL(1)` | `GOAL(2)` | `GOAL(3)` |
|---|---|---|---|
| healthy (all 9 admissible) | picks `{2}` at every (β, w_max) | `{4}` | `{5}` |
| input-starved (only 6/7/8 admissible) | `{6}` (lobby) | `{6}` | `{7}` (contract) |

**`sanity::sc4_wmax_beta_probe`** — full runs:

- **(a)** a healthy `GOAL(1)` firm (`aspiration_capital_growth = 100 000`, so
  `ς_1` stays enormous and no market action satisfices; `θ_limit = 0.90`,
  `θ_cap = 0.15`, so `h` stays well above `h_crit`), full shaping repertoire:
  **`0 / 400` shaping decisions at *every* one of the 15 `(w_max, β)` cells**,
  including `w_max = 9, β = 0` (widest possible search, zero narrowing).
- **(b)** the degenerate channel — an input-starved firm (`r^I = 0`,
  `r^L ∈ {26, 27, 28}`, `π^I = 30 > κ_ℓ = 25`, `w_max = 9`): reaches `lobby`,
  but **`3 / 600` decisions (0.5 %)** — each firm lobbies **exactly once**,
  then holds forever, because a firm that cannot operate cannot afford a
  second `κ_ℓ`. SC-4's 5 % is not met even here.

## Alternatives

- **A full `w_max × β`-inclusive re-run of `sc16_search`.** Unnecessary: (a)
  above shows `0` shaping at every `(w_max, β)` for a firm *engineered* to
  force the scan deep into the priority order, so no `(w_max, β)` value opens
  the channel and a broader re-sweep would only reproduce `SC-4 = 0`
  everywhere. The `sc16_search` exploratory test gains a small `(w_max, β)`
  loop on the healthy config to make this visible, but the gate test's
  assertions are unchanged.
- **Treating the degenerate one-shot channel as "SC-4 reachable".** A
  can't-operate firm lobbying once before idling is not "shaping attempted
  > 5 % of decisions" in any regime with more than a handful of ticks; and
  the price (`π^I > κ_ℓ`) is a degenerate config, not a sanity regime.

## Consequences

- **Positive.** The SC-4/SC-5 finding is now robust to the two parameters
  most directly relevant to it, and the *reason* is stated precisely
  (never-satisfices ⇒ fallback-only ⇒ needs-earlier-inadmissible), not via
  the manual's "small `w_eff`" shorthand.
- **Negative, accepted.** None — no code or shipped-output change.
- **Neutral.** Two new tests; ADR-0040's `Status` line annotated.

## Compliance

- `tests/tests/validation.rs::sc4_shaping_selected_only_as_the_goal_fallback_never_by_the_scan`
  — structural, drives `select` across the §16.1 sweep.
- `tests/tests/sanity.rs::sc4_wmax_beta_probe` — full runs: (a) healthy firm,
  0 shaping at all 15 `(w_max, β)`; (b) degenerate channel, one-shot only.
- ADR-0040 `Status` line references this ADR; `PROGRESS.md` Stage-6 section
  and the SC finding table updated.

## Note

There is a substantively interesting connection here, worth stating: the
*only* thing that would make a `decision.satisficing` firm shape is exactly
what H1a predicts as the driver of *wider* search — a large, unmet goal
shortfall. But H1a's channel (`ς → w_eff` via focus) and the shaping channel
are disjoint in this model: a firm with a large `ς_1` gets `GOAL(1)` focus
and a wide `w_eff`, and then takes the *first admissible market action* in the
`GOAL(1)` order (`produce_regulated`), because shaping never satisfices and
that market action is always there. Shaping is reached only when the firm has
been cut off from operating entirely. Whether that is the intended
relationship between the BTOF search-widening mechanism and the RDT shaping
repertoire — or a gap to close before Phase 3 — is the same spec question
ADR-0040 raised, and this result sharpens rather than changes it.
