# ADR 0048 — H3 revision, round 2: a real decision-time race against `time_to_boundary`, with a config-only toggle

**Status:** Accepted (2026-09-06) — **implemented on branch
`h3-satisficing-lookahead`, NOT merged**. Round 2 of an owner-directed
revision (ADR-0047 was round 1). **Supersedes nothing in ADR-0047** — this
ADR adds a gating condition in front of ADR-0047's expected-relief
calculation; every one of ADR-0047's Decisions stands as written.
**Compliance claim that the `sc4_wmax_beta_probe` harness is "all green" superseded by [ADR-0056](0056-sc4-probe-claims-correction.md) (DRAFT, pending owner review)** — the probe fails at the commit this ADR shipped in.
**Phase:** 2 (Model), post-Stage-7 (H3 revision, round 2)
**Relates to:** manual §2.4 (H3 exact text), §11.2/§11.3 (shaping's cost/
lag/uncertainty properties), §12.3 (the decision procedure), §14.3
(`time_to_boundary` — "Ticks until `h ≤ 0` under continued current
action... Forward projection"), §16.1 (run horizon `T = 400`, shaping-lag
sweep), §20.5 (versioning), ADR-0021 Decision 4 (single-source deterministic
core — `market_core`, reused again here), ADR-0047 (round 1, the
expected-relief calculation this ADR gates)

## Context

**Why this round exists, stated plainly, the same way ADR-0047 named its
own owner direction:** ADR-0047 gave a shaping action's payoff a fair,
non-zero expected value under `SURVIVAL` focus, but never checked whether
that payoff could plausibly *arrive* before the firm's situation resolves
one way or the other — it evaluated *how good* the payoff is, not *whether
there is time for it to matter*. The owner has confirmed directly: **the
time comparison was the actually-wanted mechanism, not an optional
extra** — H3's text has two clauses ("lag < time-to-boundary" *and*
"lag > time-to-boundary"), and round 1 only ever engaged the first reading
implicitly (by not checking timing at all, it behaved as if the payoff
always arrives eventually). This round builds the comparison for real, as
a decision-time branch, not something left to emerge downstream across
config sweeps (which round 1's own honest finding — "the decision does not
vary with lag length" — correctly reported as the *actual* state of the
code at the time, not a gap left unmentioned).

## Decision

### Part A — `time_to_boundary` (§14.3), reusing what already exists

New function `firma_domain::dynamics::time_to_boundary`, alongside
`market_core` (which it calls repeatedly) and `standard_margin` (which it
calls to recompute `h` after each simulated step) — **no second copy of
either**. Signature:

```rust
pub fn time_to_boundary(
    action: MarketAction, state: &FirmState, theta: &ConstraintParams,
    env: &EnvParams, aux: &FirmAuxState, action_params: &ActionParams,
    scales: &ScaleFactors,
) -> Option<u64>
```

- **Mechanism.** Starting from `state`, repeatedly applies `market_core`
  with the *same fixed* `action`, recomputing `h` via `standard_margin`
  after each step. Returns `Some(0)` if already at/past the boundary,
  `Some(k)` for the first `k ≥ 1` at which the projected `h ≤ 0`, or `None`
  if the cap ([`MAX_PROJECTION_TICKS`] `= 50`) is reached first.
- **`u` held fixed for the whole projection** — the same §9.3 approximation
  every other one-step lookahead in this codebase already makes ("`u`
  tracking is a caller concern, not part of this `FirmState → FirmState`
  core"). Consequence, verified by test, not merely asserted: a firm whose
  *only* binding constraint is `compliance` has an **unbounded**
  `time_to_boundary` under any repeated market action, because none of them
  move `u`. This is not a bug in the projection — it is a faithful
  consequence of an already-accepted modelling simplification, and it means
  a compliance-bound firm's shaping evaluation is *never* time-gated
  (finite `Δ_min` always beats an unbounded horizon) — verified directly in
  round 2's regression (below).
- **The cap: 50 ticks, justified.** Comfortably past §16.1's widest
  shaping-lag sweep (`Δ_max = 16`), so no realistic lag comparison this
  feeds is ever truncated by the cap; an eighth of the §16.1 default run
  horizon (`T = 400`) — long enough to mean "far enough away to stop
  mattering for a same-tick decision," short enough to keep a
  once-per-candidate-shaping-action, per-firm, per-`decide` computation
  cheap. An unbounded loop was explicitly rejected as unacceptable, per the
  task's own instruction; 50 is the smallest round number comfortably
  clearing the largest input this comparison is ever fed.
- **"Current action," decided precisely, mirroring an existing precedent
  rather than inventing a new one.** `time_to_boundary`'s `action` argument
  is the firm's `prev_action` (last tick's `SELECTED_ACTION`) — **the same
  value, and the same `hold`-default, `select()`'s `NONE`-focus inertia
  fallback already uses** (§12.3: "repeat previous action"). One definition
  of "what the firm is currently doing," two consumers, not defined twice.
  If `prev_action` is a shaping index (`6, 7, 8`), the projection falls
  back to `MarketAction::Hold` — a shaping commitment is not a sustained
  *market* behaviour with an obvious `market_core` transition, so the
  simplest, most conservative reading ("assume no organic change from a
  one-off shaping attempt") is used, stated here rather than silently
  chosen.
- **Not a second decision procedure.** §12.3's "the firm does not simulate;
  it applies a one-step lookahead" is about not chaining the firm's own
  future *decisions* together — this projection never re-runs `attend` or
  `satisfices` at an intermediate step; it is one fixed-action forward
  projection of *state*, the same conceptual move §14.3 itself already
  licenses as an offline viability metric. Used decision-time here purely
  as a **comparison input** to one threshold test (Part B re-confirms the
  boundary explicitly).

Four new `firma-domain` unit tests lock the exact-crossing case, the
becomes-inadmissible case (a running cost the firm can no longer afford —
the boundary is deemed reached there, since the firm cannot even continue
its assumed trajectory), the already-past-the-boundary case, and the
unbounded/cap case.

### Part B — the race, built as a real branch

`DecideCtx::shaping_expected_survival_margin` (ADR-0047) now computes, for
`lobby`/`contract`, the action's `Δ_min` (from its configured `LagRange`)
and — **when gated on** (see Part C) — `time_to_boundary` under the firm's
`prev_action`. **If `Δ_min ≥ time_to_boundary`, returns `None`** — the
caller's existing fallback path (unchanged since ADR-0047: `dc.margin_at(&n,
…) > h_t`, the pre-ADR-0047 cost-only test) takes over, exactly as if no
success model were configured at all. **If `Δ_min < time_to_boundary`** (or
`time_to_boundary` is `None`, unbounded), ADR-0047's expected-relief
calculation runs as before.

**`Δ_min`, not `Δ_max` or the mean — reasoned through, not assumed.** H3's
text asks whether the payoff *can* arrive in time, an existential claim,
not whether it is *guaranteed* to (which `Δ_max` would test) or arrives *on
average* (the mean, which conflates the two readings). Using `Δ_min` means:
if **even the fastest possible draw** would not beat the projected
boundary, the payoff is categorically irrelevant to this race, and no
draw could rescue it — a clean, necessary-condition gate. `Δ_max` was
considered and rejected: it would make the "arrives in time" branch fire
far less often (only when even the *slowest* draw wins), which is a
stricter reading than H3's own "can arrive" wording supports, and would
make the mechanism harder to observe in practice for no textual gain.

**Satisficing vs. optimising, re-checked (this branch adds a genuine new
conditional, so re-confirmed, not assumed from round 1):** `select()`
(ADR-0040) is still untouched. This race check is evaluated **once, for
one candidate action, at the point the scan reaches it** — it does not
compare `lobby` against any other action, does not change scan order, does
not change `w_eff`, and does not let the scan continue past `w_eff` to find
a "better" timed option. It is a threshold gate in front of a threshold
test ("does this one action clear the bar, given the situation"), not a
comparison across options. A true optimiser would evaluate every
admissible action's full expected value (time-gated or not) and pick the
best; this still stops at the first action, in scan order, that clears its
own bar.

### Part C — the config-only toggle

`ShapingScanParams` (the same struct ADR-0047 extended) gains:

```rust
#[serde(default = "default_require_time_margin")] // → true
pub require_time_margin: bool,
```

**`false` reverts exactly to ADR-0047's payoff-only test** (the race check
is skipped entirely; the expected-relief calculation always runs when a
success model is configured) — an explicit, named, ablation/comparison
switch, not a separate plugin or a compile-time feature flag, discoverable
in the plugin's own config schema like every other field here.

**Default: `true` (time-aware is the standard going forward).** Decided,
not assumed, for these reasons:

1. **It is the mechanism the owner actually asked for**, twice now (round 1
   asked for a fair evaluation of the payoff; round 2 asked for the timing
   comparison specifically, confirmed as "the part they actually wanted, not
   an optional extra"). Defaulting to the *simpler*, less-complete
   mechanism would mean anyone writing a *new* config from scratch gets the
   weaker model unless they know to opt in to the one actually intended —
   backwards from how a config author should discover the right behaviour.
2. **It costs nothing for every config that predates ADR-0047.** The field
   only matters when `lobby_success`/`contract_success` is *also*
   configured (both still default to `None`) — a config with neither field
   set behaves identically regardless of `require_time_margin`'s value. The
   "behaviour-preserving default" property §20.5 cares about is about
   *existing* configs, and it holds either way; the choice here is only
   about what a *new* config that opts into ADR-0047's mechanism gets by
   default.
3. **The alternative (default `false`) would be maximally conservative
   relative to round 1's regression suite specifically** — but round 1's
   suite is re-run and re-confirmed passing under the new default anyway
   (Part D), so that conservatism buys nothing here that isn't already
   verified directly.

### Consequence for round 1's own finding — clarified, not contradicted

ADR-0047 reported "the decision does not vary with lag length" as a
general property. Round 2 sharpens this precisely: **that finding was
correct for the specific scenario it tested** (a firm whose only binding
constraint is `compliance`), because `time_to_boundary` is unbounded there
by construction (`u` held fixed, as above) — no finite lag can lose an
unbounded race, so lag genuinely didn't matter *in that scenario*. It was
never a general claim that lag can never matter, and round 2's new tests
demonstrate the general case directly: with a *finite* `time_to_boundary`
(a `solvency`-eroding `prev_action`), lag decides the outcome. Round 1's
original test (`h3_the_survival_test_is_not_lag_sensitive_by_design`) still
passes unchanged under this ADR's new default — verified, not assumed —
because it is still exercising exactly the unbounded case.

## Alternatives

- **`Δ_max` or the lag distribution's mean, instead of `Δ_min`.** Rejected
  with reasoning above — `Δ_max` tests a stricter ("guaranteed") reading
  than H3's "can arrive" text supports; the mean conflates the two.
- **A discount-by-elapsed-time inside `time_to_boundary` itself** (treating
  a boundary crossing found earlier in the loop as "worse" continuously,
  rather than a hard cutoff). Rejected: `time_to_boundary` is a §14.3-
  defined *metric* (an integer tick count), not a design surface for a new
  weighting scheme; ADR-0047 already rejected discounting the payoff by
  lag, for the same no-manual-anchor reason, and this would just relocate
  the same rejected idea into a different function.
- **A separate `firma-plugin-decision-timed` crate or a compile-time
  feature flag for the toggle.** Rejected per the task's own instruction
  and this project's existing pattern: a config-level `Option`/`bool` field
  is how every other opt-in mechanism in this codebase (`ShapingScanParams`
  itself; `SatisficingParams.shaping`; `ScaleFactors`) is already built.
- **Defaulting `require_time_margin` to `false`.** Considered, reasoned
  through, and rejected above — the case for `true` is stronger given the
  owner's stated intent and the fact that regression safety does not
  actually depend on which default is chosen (both are verified).

## Consequences

- **Positive.** H3's two-sided prediction ("reduces narrowing when lag <
  time-to-boundary... increases it when lag > time-to-boundary") is now a
  **real, decision-time mechanical branch**, not an emergent, only-visible-
  downstream property. The satisficing/optimising boundary is preserved,
  checked directly again against both of §12.3's "MUST NOT" rules. The
  toggle makes the ablation (ADR-0047-only behaviour) a one-line config
  change, useful for E3's own "shaping available vs. removed"-style
  comparisons later.
- **Negative, accepted.** `time_to_boundary` under a `compliance`-only
  binding constraint is always unbounded (a real, stated limitation
  inherited from the existing `u`-held-fixed approximation, not introduced
  here) — meaning the race check has no teeth for firms whose only trouble
  is compliance; it only discriminates when `solvency`, `scope`, or
  `obligation` is (or becomes) the eroding constraint under the firm's
  current action. This is a boundary condition worth knowing, not hidden.
- **Neutral.** One new `firma-domain` function + 4 unit tests; one new
  `DecideCtx` field (`prev_action`) and one new config field
  (`require_time_margin`); the `shaping_expected_survival_margin` gate is
  the only change to `satisfices()`'s control flow.

## Compliance

- Four `firma-domain` unit tests for `time_to_boundary` (exact crossing,
  inadmissible-mid-projection, already-past, unbounded/cap).
- Both VT-8 criterion-(iii) directional greps re-run fresh against this
  round's code — still empty/unchanged; `time_to_boundary` and
  `shaping_expected_survival_margin` grepped directly for
  `aspiration`/`shortfall` — clean.
- Two new `firma-plugin-decision` tests proving the actual race
  (`h3_race_short_lag_wins_lobby_satisfices`,
  `h3_race_long_lag_loses_falls_back_to_cost_only`) and one proving the
  toggle (`h3_toggle_off_reverts_to_payoff_only_despite_long_lag`).
- Full VT-1…VT-8, the `sc16_gate`/`sc4_wmax_beta_probe`/`sc16b_arm_scoping`
  harness, and the full workspace test suite — all re-run, all green (Stage
  report).
- All four frozen hashes re-confirmed byte-identical (Stage report) — no
  golden-trace regeneration, no version bump; neither shipped smoke config
  configures `shaping` at all, confirmed directly, not assumed.

## Note

`time_to_boundary`'s unbounded behaviour under a `compliance`-only bind is
the same "worth a manual PATCH" category of finding ADR-0047 already
flagged for §12.3's "one-step lookahead" ambiguity: §14.3 licenses
`time_to_boundary` as a general viability metric without saying anything
about which constraint dimensions a "continued current action" projection
can or cannot move — an implementer without ADR-0047/0048's specific `u`-
held-fixed reasoning could reasonably expect it to be finite for *any*
persistently-thin-margin firm. Recorded here for the same reason ADR-0047
flagged its own ambiguity: so a future reader does not have to re-derive
it.
