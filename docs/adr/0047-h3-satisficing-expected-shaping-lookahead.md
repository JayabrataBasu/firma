# ADR 0047 — H3 model revision: `satisfices()`'s SURVIVAL branch evaluates a shaping action's expected effect on `h`, not just its cost

**Status:** Accepted (2026-09-06) — **implemented on branch
`h3-satisficing-lookahead`, NOT merged to `main`**. The decision to attempt
a targeted model revision (rather than descope H3, ADR-0044's other named
option) was made explicitly by the project owner, with the risk stated up
front. This ADR and its implementation are reported here for review before
merge — see the Stage report for the retraction plan (`git tag
pre-h3-revision`).
**Phase:** 2 (Model), post-Stage-7 (H3 revision)
**Relates to:** manual §2.4 (H3 exact text), §11.2/§11.3 (shaping's cost/
lag/uncertainty properties), §12.3 (the decision procedure — Steps 1–5,
`satisfices()`), §20.5 (versioning), §28.2 (E7 — satisficing/optimising/
random comparison, Phase 4), ADR-0040 (VT-8 seam, the `select()`
extraction), ADR-0042 (the structural finding this revision addresses),
ADR-0044 (the per-hypothesis reading; names this as one of two options for
H3), ADR-0023 (the "no silent default for an uncalibrated `[D]` parameter"
precedent this ADR follows again)

## Context

Stage 6/6b/6c established, and this ADR does not relitigate: `decision.
satisficing`'s `satisfices()` evaluates a shaping action's lookahead via
`shaping_cost_step` — cost only, **never** the action's declared,
config-exposed success probability or its lagged payoff (§11.2's table:
`p_success`, lag, on-success effect). A shaping action can therefore never
satisfice any test (`Expected h_{t+1} > h_t` for `SURVIVAL`; `Expected Δv_j
≥ ς_j` for `GOAL(j)`) — its lookahead only ever shows a cost, which fails
both tests by construction. It is only ever reached via the Step-5
**fallback**, which needs every action ahead of it in priority order to be
inadmissible (ADR-0042). H3 ("Shaping availability reduces narrowing when
shaping lag < time-to-boundary, and increases it when lag > time-to-
boundary") predicts a comparison that has no data to compute under this
build, in any arm, at any tested lag (ADR-0044).

**The manual's own text is in tension with the fix this ADR makes**, and
that tension is named here rather than papered over. §12.3 states, of
`satisfices()` generally: "Expectations use the deterministic core with
stochastic terms at expectation. **The firm does not simulate; it applies a
one-step lookahead.**" Read narrowly, "one-step" could mean "the lookahead
sees only what changes by tick `t+1`" — which would foreclose ever
accounting for a shaping action's payoff, since §11.3 property 2 requires
lag `≥ 1` tick and the payoff is therefore, by construction, never visible
at `t+1` for any shaping action. Read as this ADR reads it: "does not
simulate" contrasts with *chaining multiple future decisions together*
(the firm does not project its own future choices forward) — not with
*evaluating one action's own full, declared, probabilistic consequence in
a single expectation*, which is exactly what "stochastic terms at
expectation" already instructs for the probabilistic part of any action's
outcome. **Textual support for the wider reading:** the GOAL(j) row of the
`satisfices()` table says "Expected `Δv_j` under `a`" — with no `t+1`
subscript, unlike SURVIVAL's row, which is written "Expected `h_{t+1}`
under `a`" explicitly. The asymmetry is in the manual's own table. This ADR
adopts the wider reading and flags the narrower one as a live alternative a
reviewer might prefer — **flagged for a §12.3 PATCH** to state explicitly
which reading is intended, regardless of which way this ADR's revision is
judged.

## Decision

### The mechanism

`satisfices()`'s **`SURVIVAL` branch only**, for a shaping action `a ∈
{6,7}` whose success model is explicitly configured (new, opt-in fields —
see below), now computes:

$$E[h_{t+1} \mid a] = p_{\text{success}}(a) \cdot h(\text{state}_{\text{cost}} \oplus \text{payoff}_a,\ \theta \oplus \Delta\theta_a) \;+\; (1 - p_{\text{success}}(a)) \cdot h(\text{state}_{\text{cost}},\ \theta)$$

— a proper expectation over the Bernoulli success draw, both branches
computed with the **same** `standard_margin` function every other margin
computation in this codebase uses (ADR-0026: one source). `p_success` is
`firma_domain::shaping::SuccessModel::p_success(legitimacy, spend)` — the
**exact function** `action.shaping.rdt_standard`'s own `apply()` calls at
commitment, with the **exact same `legitimacy` input** (read from the same
tick's view; nothing between `decide` (phase 3) and `act_shaping` (phase 5)
writes `legitimacy`, so the value is identical — verified by reading
`crates/firma-plugins/firma-plugin-action-shaping/src/lib.rs`'s `apply()`
directly, not assumed). This is "reuse the plugin's own declared
`p_success`," not a second implementation of it.

- **`lobby` (action 6):** on success, `θ_limit += δ_θ` — directly reduces
  `g_compliance`, so it is representable as a `θ` shift in the existing
  `ConstraintContext`.
- **`contract` (action 7):** on success, `θ_Q += δ_Q` **and** `q += q_0`
  (the manual's own "double edge," §11.2: "It should not be uniformly
  beneficial"). Both are modelled — the obligation increase is not dropped
  just because it is inconvenient; a `contract` whose `q_0` outweighs `δ_Q`
  legitimately has a *negative* expected value under this formula, exactly
  matching the manual's warning.
- **`diversify` (action 8): unchanged, cost-only.** Its payoff (a new
  `supply` edge) has no representation in `(FirmState, θ)` — the `d ≤ 4`
  state this model's margin is defined over (§9.3) has no dependence term.
  This is not an oversight; it is the same boundary §13.1 already draws
  ("Dependence is... a *measured consequence*... not assigned" — nothing
  reads `Edge.weight` in the running model, per §13.2's own note on the
  deferred `Reputational` sub-effect). Stated plainly rather than silently
  narrowed.
- **`GOAL(j)` branch: unchanged, cost-only, for all three shaping
  actions.** None of the three declared payoffs move `v_1` (capital),
  `v_2` (capability), or `v_3` (obligation cleared) — `lobby`'s and
  `contract`'s payoffs move `θ` (a constraint boundary), and `contract`'s
  `q_0` moves obligation the *wrong* direction for `GOAL(3)`. `E[Δv_j] =
  p_{\text{success}} \cdot 0 - \text{cost} = -\text{cost}` for every `j` —
  algebraically identical to today's cost-only result, so there is nothing
  to change in that branch; it was already computing the right (trivial)
  answer.

**A substantive, unforced finding, stated here rather than left implicit:**
this means the fix's channel is `SURVIVAL`-only. H3's own text is about
narrowing "when shaping lag < time-to-boundary" — `time-to-boundary` is a
§14.3 viability metric, and `SURVIVAL` focus (`h < h_crit`) is precisely
the near-boundary regime. The fix opening exactly the focus branch closest
to H3's own framing, and not the goal-directed one, is a good sign that the
revision is tracking the theory rather than being shoehorned to pass a
test.

### Config shape — additive, opt-in, backward-compatible

`ShapingScanParams` (in both `SatisficingParams` and `RandomParams`) gains
two new fields, both `#[serde(default)]` → `None`:

```rust
pub struct ShapingScanParams {
    pub lobby_cost: i64,                              // unchanged
    pub contract_cost: Option<i64>,                    // unchanged
    pub diversify_cost: Option<i64>,                   // unchanged
    #[serde(default)]
    pub lobby_success: Option<firma_domain::shaping::LobbyParams>,     // NEW
    #[serde(default)]
    pub contract_success: Option<firma_domain::shaping::ContractParams>, // NEW
}
```

Reusing `firma_domain::shaping::LobbyParams`/`ContractParams` directly (not
a parallel struct) — the same types `action.shaping.rdt_standard.lobby`/
`.contract` are configured with. **`None` (the default, and every existing
config) ⇒ exactly today's cost-only lookahead** — this is precisely §20.5's
"new optional parameter with behaviour-preserving default" row: a MINOR
change in isolation. Whether the *workspace* version needs to move, and by
how much, is decided empirically in Part C of the Stage report (whether any
*existing* config sets these new fields and therefore whether any existing
config's output actually changes) — not asserted here in advance.

**Convention, matching `SatisficingParams.action: ActionParams`'s existing
precedent:** the firm's belief about its own shaping success model is
config it carries independently of the action plugin's config — the same
way `decision.satisficing` already carries its own copy of `ActionParams`
(`y_0`, `η`, `γ_R`, …) to predict market actions without depending on
`firma-plugin-action-market` (a cross-plugin dependency, forbidden by
§18.1). A config author sets `decision.satisficing.params.shaping.
lobby_success` to the same values as `action.shaping.rdt_standard.lobby`'s
own params for the lookahead to be a faithful prediction of what will
actually happen — the **formula** is shared (`SuccessModel::p_success`,
one function in `firma-domain`); the **numbers** are config-duplicated by
the same established pattern, not by this ADR inventing a new one.

### Preserving the satisficing/optimising distinction (§28.2, E7)

Checked directly against the two rules §12.3 marks "MUST NOT be optimised
away":

1. **First satisficing, not argmax.** Unchanged. `select()` (ADR-0040) is
   untouched by this revision — it still calls `satisfices(a, focus)` as a
   closure and returns on the **first** `a`, in scan order, for which it
   returns `true`. This revision changes what `satisfices()` computes for
   one class of `a` under one `focus`; it does not touch how many
   candidates get evaluated, whether the scan continues after a pass, or
   whether outcomes are ever compared against each other. A true optimiser
   would evaluate every admissible action and return the best; this still
   evaluates admissible actions **in priority order** and returns the
   first that clears its own threshold — bounded rationality's essential
   character (stop at "good enough," not "best") is intact.
2. **Inadmissible actions do not consume scan budget.** Unaffected —
   `admissible()` is untouched by this ADR.

### Discounting by lag: decided against, reasoning stated

The manual gives no discount rate for satisficing's lookahead, and this ADR
does **not** introduce one. Two readings were weighed:

- **Discount by expected lag** (e.g. divide the expected payoff by
  `E[Δ_a]`, the midpoint of the drawn `Uniform{Δ_min, Δ_max}` range).
  Superficially plausible — a payoff 8 ticks out "feels" less valuable than
  one 2 ticks out — but introduces a genuinely new `[D]` parameter (a
  discount rate or an implicit assumption that "ticks of delay" and
  "probability of success" trade off linearly, which nothing in §9–§14
  states) with no calibration anchor anywhere in the manual.
- **No discount — the payoff counts at full value whenever it lands
  (adopted).** This matches the **existing** precedent for exactly this
  situation elsewhere in the same lookahead: `market_core`'s
  `InvestCapability` arm is documented "§9.3 approximation: lag collapsed
  to immediate" — the model already treats a *different* action's lagged
  effect (`invest_capability`'s own §9.3 lag) as landing at full value with
  no discount, for lookahead purposes. Applying the same treatment to
  shaping's lag is the *consistent* choice, not a new one. Bounded-
  rationality literature (Cyert & March, the manual's own cited tradition)
  does not require temporal discounting as a defining feature — satisficing
  is about the *threshold test*, not about *how future value is weighted*;
  nothing about "stop at good enough" logically implies "and discount the
  future while doing so."

**If a reviewer prefers the discounted reading, that is a different ADR**,
not a revision of this one (immutability) — this ADR states the choice
made and why, so a disagreement is a citable, supersedable claim rather
than a silent default.

### Scope boundary, confirmed against the actual diff

Every other satisficing/random code path is untouched:
`Focus::attend`, narrowing (`ψ`, `w_eff`), the scan-order tables, `select()`
itself, market actions' `lookahead`/`satisfices` arms, `admissible()`,
`decision.random`, `decision.aspiration_update`. The diff (Stage report,
Part B) touches exactly: `DecideCtx` (one new method), `ShapingScanParams`
(two new opt-in fields), and `satisfices`'s `SURVIVAL` arm (one new
conditional branch, falling through to the existing cost-only comparison
when no success model is configured for that action).

## Alternatives

- **Extend the `GOAL(j)` branch too**, treating a shaping action's `θ`
  improvement as if it contributed to whichever goal is active. Rejected:
  `θ` is not `v_1`/`v_2`/`v_3` — conflating "the constraint boundary moved"
  with "the firm's own performance level rose" would be inventing a
  cross-construct exchange rate the manual never states, a materially
  larger and less textually grounded change than the one made.
- **Model `diversify`'s dependence-reduction benefit via a synthetic
  proxy** (e.g. treat a fresh supply edge as worth some assumed capital
  value). Rejected: no such proxy is declared anywhere in the manual;
  inventing one would be a second undeclared `[D]` parameter with no
  anchor, for a channel (`GOAL`-focus dependence concerns) H3's own text
  does not obviously live in anyway (H3 is a `SURVIVAL`/boundary-proximity
  claim).
- **Discount by lag** — see above; rejected with reasoning stated, not
  merely dismissed.
- **A required (non-optional) config shape**, forcing every existing
  `shaping` config to declare success models. Rejected: breaks every
  existing test config for no behavioural gain over an opt-in default that
  achieves the same end state once configs actually set it, and violates
  §20.5's own preference for behaviour-preserving additions where one is
  available.

## Consequences

- **Positive.** H3's `SURVIVAL`-focus channel is no longer structurally
  foreclosed — whether it is *practically* reachable at realistic parameter
  values is Part B2's empirical question, not asserted here. The
  satisficing/optimising distinction (§28.2, E7) is preserved by
  construction, checked directly against both of §12.3's "MUST NOT" rules
  above. No existing config's behaviour changes unless it opts in.
- **Negative, accepted.** `contract`'s expected value can now be genuinely
  negative (by design — the manual's own warning), which a config author
  must understand when setting `contract_success`; `diversify` remains
  permanently cost-only under this design, a real, stated limitation of
  what H3-relevant behaviour this revision can produce for that specific
  action. The "one-step lookahead" sentence's ambiguity is now
  load-bearing for this implementation rather than moot — flagged for a
  manual PATCH, not resolved unilaterally.
- **Neutral.** Two new opt-in config fields; one new `DecideCtx` method;
  the `SURVIVAL` arm of `satisfices` gains one conditional. Whether any
  shipped config's numerical output changes, and the version-bump
  consequence if so, is reported empirically in the Stage report (Part C),
  not predicted here.

## Compliance

- The two VT-8 criterion-(iii) directional greps (ADR-0040), re-run fresh
  against this change (Stage report Part B1) — not assumed to still pass.
- A targeted scenario demonstrating the channel opens (short lag relative
  to a constructed time-to-boundary) and a mirror scenario demonstrating it
  does not spuriously open (long lag) — Stage report Part B2.
- Full VT-1…VT-8, the `sc16_gate`/`sc16_search` harness, ADR-0044's
  H1a/H1b/H1c/H2/H4 hypothesis table re-checked, and the full workspace
  test suite — Stage report Part C.
- The adversarial-literature-check house rule applies to any *contribution
  claim* about H3 now being testable (a working mechanism "would
  substantially raise the project's apparent value") — this ADR records
  the code change and its verification only; no claim that H3 is confirmed,
  supported, or even reliably testable in practice appears here or should
  be inferred from a passing regression suite alone.

## Note

The asymmetry in §12.3's own `satisfices()` table — `SURVIVAL`'s row
explicit about `t+1`, `GOAL(j)`'s row silent on timing — is worth a
manual PATCH regardless of how this ADR is ultimately judged, because it is
exactly the kind of textual gap that let two defensible readings ("one-step
means no lagged payoff, ever" vs. "one-step means no chained future
decisions, but a single action's full expectation is fine") both survive a
close reading. Recorded so a future reviewer does not have to rediscover
the ambiguity from scratch.
