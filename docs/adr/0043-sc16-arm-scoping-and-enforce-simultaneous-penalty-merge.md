# ADR 0043 — SC-1…6 is unscoped to a decision plugin (§16.2/§27.3); and a `constraint.enforce` simultaneous-penalty merge

**Status:** Accepted (2026-09-06). **Decision 1's "the Phase-2 gate's
criterion 8 is met" framing is superseded by
[ADR-0044](0044-sc4-sc5-gate-readiness-is-per-hypothesis-h3-untestable.md)**,
which reads gate-readiness per-hypothesis: H1a/H1b/H1c/H2/H4 are gate-clear
(their DVs never depend on SC-4/SC-5), but H3 is untestable as built —
shaping never gets selected under `decision.satisficing`, in any arm, at
any tested lag, so its predicted comparison has no computable data. This
Decision's underlying empirical result (`decision.random` jointly satisfies
SC-1…6, robustly across 5 seeds) is unchanged and not retracted — only its
use as grounds for an unqualified whole-gate claim is superseded. **Decision
2 (the `constraint.enforce` fix) is untouched by ADR-0044 and stands as
written.**
**Phase:** 2 (Model), Stage 6b
**Relates to:** manual §16.2 (SC-1…6), §27.3 criterion 8, §30.4 (Arm A/B/C
design table), ADR-0040 (VT-8 + the original SC-1…6 finding), ADR-0042
(SC-4/SC-5 confirmed width-independent), ADR-0033 (`ResourcePool`
uniqueness-guard scope)

## Context

Stage 6 (ADR-0040) found VT-8 passes and SC-1…6 cannot be jointly satisfied
under `decision.satisficing` in a no-shock baseline, and reported this as a
§16.2 Phase-2 failure condition blocking the gate pending a research-design
decision. This ADR resolves that decision by testing, rather than assuming,
whether SC-1…6 is scoped to `decision.satisficing` specifically.

**The manual text, quoted exactly.** §16.2's table has no decision-plugin
qualifier on any row except SC-1 ("Firms surviving to `T` **under no
shock**"). SC-2 through SC-6 carry no qualifier of any kind. §27.3 criterion
8: "**A parameter regime exists in which constraints genuinely bind** —
firms neither all die immediately nor never approach a boundary (§16.2).
**Criterion 8 is not a software test. It is the check that the model is
scientifically alive.**" §30.4's factor table lists `Decision plugin |
satisficing, random | C` as a swept factor of the model, on the same footing
as `β`, shock magnitude, and shaping lag.

Nothing in either passage names `decision.satisficing`. Read literally, "a
parameter regime exists" is existential over the model's full parameter
space — and the decision plugin is formally part of that space (§30.4). This
motivated **Path 1**: test whether an already-specified plugin
(`decision.random`, Arm C's plugin) can jointly satisfy all six in some
regime, which would mean §16.2/criterion 8 is a check on the model's
plumbing (are all four constraints wired to bind, does the shaping machinery
resolve at a plausible rate, is repertoire entropy non-degenerate) rather
than a claim about `decision.satisficing`'s own emergent behaviour.

## Decision 1 — SC-1…6, read as unscoped to any decision plugin, is jointly satisfiable; the Phase-2 gate criterion 8 is met under this reading

**`decision.random` (`tests::sc16b_arm_scoping::cfg_random_binding`), a
12-firm, `T = 400` config, jointly satisfies SC-1…6, robustly across five
independent mechanism seeds.**

Economy: `output_price = 7`, `input_price = 2` (so a `c ≈ 0.5` firm's
expected capital drift is near zero — the same knife-edge SC-1/SC-2 tension
ADR-0040 found for `decision.satisficing`, now engineered for `decision.random`
instead of stumbled into). `θ_cap = 0.45`, `θ_limit = 0.25`, `θ_Q = 10`.
Population: 3 low-capability firms (`c ∈ {0.22, 0.30, 0.40}`, thin capital)
that bleed out under random production choices — `solvency` binds; 7
comfortably-capitalised firms (`c ∈ {0.50 … 0.85}`) that mostly survive; 2
firms seeded `q ∈ {90, 70} > θ_Q` — `obligation` binds and is non-lethal, so
they survive stressed. `lobby` + `diversify` in the repertoire (§16.1
defaults except `κ_ℓ = κ_d = 25`). One deliberate non-default: **`t_c = 1`**,
not §16.1's `4` — under uniform-random action choice, `u`'s tick-to-tick
draws are uncorrelated, so the default `T_c = 4` window turns almost every
`compliance` first strike into a near-certain lethal second strike within a
few ticks, conflating SC-1 (survival) with SC-3 (compliance need only bind
*once*, not repeatedly kill). `t_c = 1` keeps the same violation semantics
(`g_2 = u − θ_limit`, Graduated) while decoupling "strikes at least once"
from "keeps striking and dies."

| SC | target | seed 4242 | seed 1 | seed 77 | seed 900001 | seed 31337 |
|---|---|---|---|---|---|---|
| SC-1 survival | 0.60–0.90 | 0.667 | 0.667 | 0.833 | 0.833 | 0.667 |
| SC-2 `h<h_crit` (reconstructed) | 0.05–0.25 | 0.081 | 0.056 | 0.074 | 0.050 | 0.062 |
| SC-3 all four bind | — | ✓ | ✓ | ✓ | ✓ | ✓ |
| SC-4 shaping fraction | > 0.05 | 0.298 | 0.299 | 0.301 | 0.297 | 0.311 |
| SC-5 shaping success | 0.10–0.60 | 0.47 | 0.49 | 0.47 | 0.48 | 0.47 |
| SC-6 entropy variance | ≥ 0.01 | 0.173 | 0.152 | 0.154 | 0.142 | 0.155 |
| **all six** | | **PASS** | **PASS** | **PASS** | **PASS** | **PASS** |

**SC-2 for a plugin that writes no `focus`.** `sanity_from_run`'s SC-2 metric
(`focus == SURVIVAL` fraction) is `decision.satisficing`-only — `decision.
random` writes no `focus`. The manual's SC-2 text is literally "firm-ticks
with `h < h_crit`," which `replay.rs` already reconstructs directly
(`sc2_reconstructed_h_below_crit`) as a cross-check under `decision.
satisficing`. This ADR reads SC-2 via that reconstructed metric for
`decision.random` — the manual's literal definition, not a
`decision.satisficing`-specific proxy for it.

**Reading adopted, and why.** The manual scopes only SC-1; criterion 8's own
gloss ("the check that the model is scientifically alive," "firms neither
all die immediately nor never approach a boundary") is a plumbing framing —
are the four constraint types wired to actually bind, does the shaping
resolution mechanism (`p_success`, the `Λ` queue) produce plausible outcomes
when invoked, is the repertoire-entropy computation non-degenerate — not an
emergent-behaviour claim about one theory-bearing plugin. §30.4 formally
includes the decision plugin in the model's swept-factor table, on equal
footing with everything else "the model" is parameterised by. Given a
demonstrated regime, **this ADR reads §16.2/criterion 8 as satisfied**, and
the Phase-2 gate's criterion 8 (§27.3) as **met** under this reading.

**The caveat this reading does not remove — flagged, not resolved, here.**
Path 1 shows the *mechanics* work. It does **not** show that E1's actual
pre-registered arms will exercise them: Arms A and B (§30.4, where H1a/H1b/
H1c live) both run under `decision.satisficing`; only the null Arm C uses
`decision.random`. ADR-0040/ADR-0042's finding — shaping is
width-independently unreachable under `decision.satisficing` in a no-shock
regime — is **unaffected by this ADR** and stands exactly as those ADRs
state it. A criterion-8 pass via Path 1 tells us the shaping machinery is
correctly wired and produces sane numbers *when invoked*; it does not tell
us Arms A/B will ever invoke it. That gap is a distinct, still-open Phase-3
question — addressed at most partially by ADR-0042's Path-2-shaped probe
below — and **this ADR recommends a §16.2 PATCH** at the next manual
revision to state explicitly which of the two readings (plumbing-check vs.
`decision.satisficing`-specific) was intended, so a future reader does not
have to re-derive this from the sanity-condition wording alone.

### Path 2, run for completeness (not required — Path 1 already resolves it)

An Arm-B-shaped config (`cfg_arm_b_satisficing`: `decision.satisficing`
under a regulatory ramp + a sustained resource-price shock, shaping lag
`(2, 6)`, one of §30.4's three levels) was also run, checking SC-1/4/5
against the shocked run and SC-2/3/6 against the Stage-6 no-shock
`cfg_threaded` baseline:

| SC | shocked / baseline value | pass |
|---|---|---|
| SC-1 (shocked) | 0.300 | fail (shock too harsh — over-tuned toward stress) |
| SC-2 (no-shock) | 1.000 | fail |
| SC-3 (no-shock) | `[true, false, true, true]` | fail |
| SC-4 (shocked) | 0.0000 | fail |
| SC-5 (shocked) | none | fail |
| SC-6 (no-shock) | 0.0000 | fail |

**SC-4 = 0.0000 under shock reconfirms ADR-0042: an applied shock does not
open the shaping channel for `decision.satisficing`** — consistent with
ADR-0042's mechanism (shaping never satisfices; the fallback needs every
market action ahead of it inadmissible, which a shock does not by itself
force for a firm still able to hold and occasionally produce). Per the
instruction this ADR was written under, Path 2 was **not tuned further**
once Path 1 resolved the question — its role here is a confirmatory data
point, not the primary deliverable.

## Decision 2 — `constraint.enforce`: merge simultaneous same-agent capital penalties into one delta

**Found while constructing Decision 1's config**, not part of the research
question: `Enforce::apply` called `to_env(agent, capital, pen)` — which
emits `AdjustStock{capital}` on `DeltaTarget::Agent(agent)` — once for a
`compliance` first-strike penalty (`P_c`) and, independently, once for an
`obligation` penalty (`P_q`). Nothing in §9.1 makes these two non-lethal
consequences mutually exclusive within one tick, and once `cfg_random_binding`
combined a moderate `θ_limit` (so `compliance` strikes sometimes) with
seeded-high-obligation firms, one agent legitimately violated both in the
same tick — producing **two** same-agent, same-resource `AdjustStock`
deltas from the one rule, origin `constraint.enforce`. The kernel's per-rule
uniqueness guard (ADR-0033, which exists *precisely* to catch a same-agent
double-debit) correctly rejected this as `DuplicateDelta { plugin:
"constraint.enforce" }`, aborting the run.

**Reproduced first, in isolation** (`firma-plugin-constraint::phase_rules::
tests::simultaneous_compliance_first_strike_and_obligation_violation_is_one_
capital_delta`), driving `Enforce::apply` directly with `u = 0.75 > θ_limit
= 0.50` (first strike) and `q = 120 > θ_Q = 100` (obligation) for one agent,
before any fix:

```
capital deltas on agent 0: [-30, -7]
thread '...' panicked: assertion `left == right` failed: enforce must emit
exactly one capital-penalty delta per agent per tick (P_c + P_q merged), ...
  left: 2
 right: 1
```

**Fix:** accumulate `capital_penalty += p_c` / `+= p_q` across the two
(still independently-evaluated) violation branches, and emit **one**
`to_env` call per agent per tick, capped by the agent's starting-of-tick
`liquid_capital` (`capital_penalty.min(state.liquid_capital).max(0)`) — the
same cap the two separate calls used individually, so a single-violation
tick is unaffected. The legitimacy loss and `COMPLIANCE_LAST_VIOLATION_TICK`
write, and the obligation branch's supply-edge severing, are untouched —
only the two capital-penalty emission sites are merged.

**Byte-identical for every shipped config**: this combination was never
previously reachable in any Phase-1/2 smoke config or the golden trace (all
four frozen hashes unchanged before/after). Re-run after the fix:

```
capital deltas on agent 0: [-37]
test result: ok. 20 passed; 0 failed
```

`20 passed` — the reproduction test plus the pre-existing 19, none regressed.

## Alternatives (Decision 1)

- **Path 2 as the primary resolution** (formalise an Arm-B scoping for
  SC-1/4/5, no-shock for SC-2/3/6). Not needed: Path 1 already demonstrates a
  single config jointly satisfying all six, and does so more directly
  (one config, one reading, matching the literal manual text) than a
  two-config split that itself requires inventing a scoping rule the manual
  never states.
- **Declaring the gate unmet regardless of Path 1** (treat SC-1…6 as
  implicitly `decision.satisficing`-scoped despite the text). Rejected as
  the *primary* reading — it requires reading a qualifier into five of six
  rows that the manual does not carry, when §30.4 explicitly treats the
  decision plugin as a swept model parameter. Recorded instead as the
  caveat above, for a §16.2 PATCH to settle explicitly.
- **(Decision 2) Cap the two penalties independently, in the order
  encountered, letting the second one see the post-first-penalty capital.**
  Rejected: introduces an arbitrary compliance-before-obligation ordering
  dependency for no modelling reason (§9.1 does not order the four
  constraints), where a single joint cap does not.

## Consequences

- **Positive.** The Phase-2 gate's criterion 8 is met under a reading that
  quotes the manual's own words rather than assuming a scope the text does
  not state; the finding is falsifiable (five-seed table) rather than a
  single lucky run. A previously-undiscovered `constraint.enforce`
  correctness gap is closed with a demonstrating test before the fix.
- **Negative, accepted.** The reading in Decision 1 leaves open whether
  Arms A/B (the arms E1's hypotheses actually depend on) will exercise
  SC-4/SC-5-like behaviour — flagged, not resolved, pending a §16.2 PATCH
  and owner review. `cfg_random_binding` uses `t_c = 1`, a deviation from
  the §16.1 default, needed to decouple SC-1 from SC-3 under uncorrelated
  random action choice; this is a property of `decision.random`'s lack of
  persistence, not evidence about `decision.satisficing`.
- **Neutral.** One new plugin-level unit test (`firma-plugin-constraint`,
  20 total); one new conformance test (`sanity::sc16b_arm_scoping`, asserts
  the Path-1 joint pass across five seeds and locks `decision.satisficing`'s
  `SC-4 = 0.0` under shock as a regression per ADR-0042).

## Compliance

- `tests/tests/sanity.rs::sc16b_arm_scoping` — locks Path 1's five-seed joint
  pass and Path 2's confirmatory `SC-4 = 0.0` under shock.
- `crates/firma-plugins/firma-plugin-constraint/src/phase_rules/tests.rs::
  simultaneous_compliance_first_strike_and_obligation_violation_is_one_
  capital_delta` — locks the merged single-delta behaviour.
- `PROGRESS.md` Stage 6b section and the SC finding table updated; this
  ADR flagged for owner review before being treated as settled, per this
  Stage's explicit instruction (same posture as ADR-0015).

## Note

This is the second time this project has found a same-agent duplicate-delta
gap by *constructing* a config that finally exercises a combination no
previous config happened to hit (the first was ADR-0031/0033's
`ResourcePool` exemption). Neither was caught by unit tests in isolation,
because each rule's own tests exercised its violation paths one at a time.
Worth a standing note for future plugin work: whenever a rule can emit more
than one delta to the same `(target, resource-or-field)` from *independent*
internal branches — not just independent invocations — a test that
deliberately triggers more than one branch at once is the only thing that
catches this before a config does.
