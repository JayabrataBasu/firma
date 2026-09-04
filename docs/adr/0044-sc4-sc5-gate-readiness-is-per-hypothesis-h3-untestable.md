# ADR 0044 — SC-4/SC-5 gate-readiness is per-hypothesis: H1a/H1b/H1c/H2/H4 are gate-clear; H3 is untestable as built — supersedes ADR-0043 Decision 1's framing

**Status:** Accepted (2026-09-06) — **research-design decision, flagged for
owner review before being treated as settled (same posture as ADR-0015 and
ADR-0043)**
**Phase:** 2 (Model), Stage 6c
**Supersedes:** ADR-0043 **Decision 1 only** — its "the Phase-2 gate's
criterion 8 is met" framing, unqualified across the whole gate. ADR-0043
Decision 1's underlying empirical result (`decision.random` jointly
satisfies SC-1…6, robustly across 5 seeds) is **not wrong and is not
retracted** — it is re-scoped below to what it actually demonstrates.
ADR-0043 **Decision 2** (the `constraint.enforce` simultaneous-penalty
merge) is **untouched and stands** — no part of this ADR concerns it.
**Relates to:** manual §2.4 (hypotheses, exact DVs), §2.5 (falsification/kill
criterion), §28.2 (experiment sequence — E1–E4), §30.3–§30.4 (E1
pre-registration hypotheses and design), §30.9 (E1 filing checklist), §14.2
(R1/R2/R3 rigidity operationalisations), ADR-0040, ADR-0042, ADR-0043

## Context

ADR-0043 concluded "the Phase-2 gate's criterion 8 is met" because
`decision.random` (Arm C, the null) jointly satisfies SC-1…6. That framing
was challenged, correctly: **Arm C is 10 of 315 pre-registered E1 cells
(§30.4); Arms A and B — 305 cells, 96.8% — run `decision.satisficing`
exclusively, with no decision-plugin variation.** Showing the null arm's
mechanics work is real evidence the platform is not broken. It is not
evidence that criterion 8 ("the model is scientifically alive") holds for
the arms that actually carry the hypotheses. ADR-0043's Decision 1 conflated
these two claims. This ADR corrects that by asking the more precise
question directly: **for each hypothesis, does its testability actually
require SC-4/SC-5** (shaping selected, and selected successfully, at a
non-trivial rate under `decision.satisficing`) **or not?**

## Decision — per-hypothesis analysis

For each of H1a, H1b, H1c, H2, H3, H4 (§2.4 exactly; H5 is Phase 4, out of
scope), quoted verbatim, with its arm (§30.4) and whether SC-4/SC-5 is
required for its own testability:

| ID | Hypothesis (§2.4, verbatim) | DV | Arm | Needs SC-4/SC-5? |
|---|---|---|---|---|
| **H1a** | "Holding margin $h$ constant, increasing shortfall $\varsigma$ **increases** search width $w_{\text{eff}}$ and repertoire entropy $H_{\text{rep}}$. *(BTOF direction)*" | `search_width`, `repertoire_entropy` | A | **No** |
| **H1b** | "Holding shortfall $\varsigma$ constant, decreasing margin $h$ **decreases** them. *(Threat-rigidity direction)*" | `search_width`, `repertoire_entropy` | A | **No** |
| **H1c** | "Where $h$ and $\varsigma$ covary endogenously, the relationship is **non-monotonic**; a smooth inverted-U indicates simultaneous operation, a discontinuity indicates attention-switching." | `repertoire_entropy` | B | **No** |
| H2 | "At matched $h$, threat *novelty* produces greater narrowing than threat *magnitude*." | `repertoire_entropy` | B | **No** |
| **H3** | "Shaping availability reduces narrowing when shaping lag < time-to-boundary, and increases it when lag > time-to-boundary." | `repertoire_entropy`, `survival_time` | B | **Yes — and NOT met** |
| H4 | "Narrowing improves survival under low-novelty threats and impairs it under high-novelty threats." | `survival_time` | B | **No** |

### H1a, H1b (Arm A)

Arm A's own rationale (§30.2) is that $h$ and $\varsigma$ are **set directly
by intervention** — "The required manipulation... is unavailable in field
data... The model is the only instrument in which the orthogonal design is
realisable." The mechanism under test is exactly the `select()` seam
ADR-0040 extracted: Step 2 (`Focus::attend`) and Step 3
(`ψ(h) → w_eff`) respond to the forced `h`/`ς` inputs with **no dependency
on which action class subsequently satisfices**. VT-8's grid
(`r(h,ς)=+0.000000`, all four quadrants populated, both directional greps
empty) already verified this structurally, independent of whether shaping
ever appears among the realised actions.

`repertoire_entropy` (§14.2 R1) is `H_rep = -Σ_{a:p_a>0} p_a log2 p_a` over
the trailing window of **realised** actions. Nothing in its definition, in
§2.4's H1a/H1b text, or in SC-6's own threshold names a minimum shaping
frequency — entropy over a distribution supported entirely on the six
market actions is exactly as well-defined, and exactly as capable of being
non-degenerate, as one that includes shaping. **Stage 6 (ADR-0040) already
found SC-6 (entropy variance non-degenerate) "usually reachable
(0.01–0.13)" under `decision.satisficing`** — the DV Arm A needs is already
shown computable. **H1a and H1b do not need SC-4/SC-5. Gate-clear.**

### H1c (Arm B)

Same DV (`repertoire_entropy`), same argument: the inverted-U-vs-
discontinuity test is about how entropy over whichever actions the firm
actually distributes across (dominated by market actions, per ADR-0042's
structural finding that shaping is fallback-only) varies as $h$/$\varsigma$
co-evolve under an applied shock — not about shaping appearing in that
distribution. SC-6's already-established reachability under
`decision.satisficing` covers this DV directly. **H1c does not need
SC-4/SC-5. Gate-clear.**

### H2 (Arm B)

§30.4's factor table places both "Novelty" and "Shock magnitude" — the two
factors H2 compares — under Arm B, with the same DV as H1c
(`repertoire_entropy`). Identical reasoning: narrowing is measured over the
realised market-action distribution; shaping's presence or absence changes
nothing about whether that comparison is computable. **H2 does not need
SC-4/SC-5. Gate-clear.**

### H4 (Arm B)

DV is `survival_time` — how long a firm survives, as a function of
narrowing width and threat novelty. This requires the population to show a
genuine *spread* of survival outcomes (SC-1's concern), not shaping
selection. **Stage 6 (ADR-0040) already showed SC-1 reachable (0.60–0.90)
under `decision.satisficing`** in a mixed/threaded population with a
knife-edge seed — a real spread of survival outcomes exists without any
shaping. **H4 does not need SC-4/SC-5. Gate-clear.**

### H3 (Arm B) — needs SC-4/SC-5, and it is not met

H3 is the one hypothesis whose predicted comparison is *literally about*
shaping: whether its availability, contrasted against how its lag compares
to time-to-boundary, changes narrowing (and survival). This cannot be
evaluated without shaping actually occurring at a rate that varies
meaningfully with lag — which is exactly what SC-4 (>5% of actions are
shaping attempts) and SC-5 (success rate 0.1–0.6) are checking for.

**Stage 6b's Path 2 result is decisive here.** `cfg_arm_b_satisficing` — an
Arm-B-*shaped* config: `decision.satisficing`, a regulatory ramp + a
sustained resource-price shock, shaping lag `(2, 6)` (one of §30.4's three
levels) — is exactly the kind of run H3 needs data from. Its result:
**`SC-4 = 0.0000`.** Zero shaping selections, under a shock, in an Arm-B
regime, at a shaping lag H3 explicitly parameterises over. This is not an
artefact of that one config: ADR-0042 already established the mechanism
generally — a shaping action never satisfices any focus (its one-step
lookahead shows only cost), so it is only ever the scan fallback, which
requires every market action ahead of it in the priority order to be
inadmissible; a shock does not, by itself, force that (a firm can usually
still `hold` or eventually re-acquire input). The Stage 6b run under an
actual shock **confirms this holds in the shocked regime H3 is meant to be
tested in**, not just the no-shock sweep ADR-0040/0042 already covered.

**Consequence, stated precisely, per the task's own framing.** This is not
"H3 is false" — a false hypothesis has a computed comparison that
contradicts its prediction. Here, **there is no possible outcome under the
current build that could either support or contradict H3's predicted
comparison**, because its precondition — shaping actually occurring, at a
rate that can be compared across lag levels — is structurally absent under
`decision.satisficing` regardless of lag or time-to-boundary. An E3-style
ablation ("shaping available" vs. "shaping removed", §28.2, paired CRN
forks) would show **zero difference between arms**, because "available"
never actually invokes it under ordinary market conditions — that null
result would misrepresent an untestable hypothesis as a *falsified* one.
**H3 is untestable as currently built, not merely hard to observe.**

**A corroborating signal from §14.2.** R3 ("shaping abandonment",
`SA = 1 − (capital committed to actions 6,7,8) / (capital committed to all
actions)`) is a *third* rigidity operationalisation, required alongside R1
whenever "any rigidity claim" is reported. If shaping is never selected,
`SA ≡ 1` for every firm-tick — well-defined (its stated undefined case is a
zero *total*-capital denominator, not zero shaping capital), but constant,
carrying no information. This is the same underlying fact as H3's
untestability, seen through a different metric, and is worth naming
alongside it as a declared limitation (§30.10) if H3 is descoped.

## Decision — the gate, restated per-hypothesis

**Phase-2 gate criterion 8 (§27.3/§16.2) is met for H1a, H1b, H1c, H2, and
H4**, under the no-shock/Arm-A/Arm-B `decision.satisficing` regimes already
built and reported (ADR-0040, ADR-0042): each of these hypotheses' DVs
depends only on SC-1, SC-2, SC-3, or SC-6 — never SC-4/SC-5 — and every one
of those has already been shown reachable under `decision.satisficing` in
at least one config (SC-1 via the threaded/knife-edge config; SC-3 partially,
same config; SC-6 via the healthy config). This is a **narrower and more
defensible claim** than ADR-0043's unqualified "the gate is met" — it names
exactly which hypotheses are covered and why, rather than resting the whole
gate on the null arm's plumbing check.

**Criterion 8 is NOT met for H3.** Its predicted comparison has no
computable data under the current `decision.satisficing` build, in any arm,
at any tested shaping lag. This is reported here as an honest, named
finding — **not fixed in this Stage.** Per the manual's own falsification
framing (§2.5) and the task's instruction, two options exist, neither
attempted here:

1. **A targeted model revision** that gives shaping a path to satisficing
   under some condition H3 cares about (e.g., a multi-step lookahead, or a
   different affordability/priority treatment for shaping actions) — this
   would need its own ADR, its own justification, and (per the project's
   standing rule on significant, contribution-shaping changes) an
   adversarial literature check before being committed to. **Not attempted
   here.**
2. **An explicit, documented descope of H3** from the initial E1 filing —
   removing its predicted sign from §30.3, noting E3 ("Shaping availability
   ablation", §28.2 — described there as testing "the distinctive
   mechanism") as blocked pending (1), and adding the untestability (and
   R3's resulting degeneracy) to §30.10's declared limitations. **Flagged
   for a §2.4/§30 PATCH, not decided here.**

### The kill criterion (§2.5) — checked precisely, not conflated

§2.5, quoted exactly: **"H1a and H1b are jointly load-bearing. If both do
not hold across the pre-registered sweep at ≥200 seeds per cell, the
dissociation does not operate in this model."** And: **"Kill criterion
(binding). If at the end of Phase 3 the platform cannot produce *either*
support for H1a/H1b/H1c *or* an interpretable, publishable negative result,
the project MUST stop... and not proceed to Phase 4."**

The kill criterion names exactly **H1a, H1b, H1c** — no others. H2, H3, and
H4 do not appear in it at all. **H3's untestability has no bearing on the
kill criterion.** All three hypotheses the kill criterion actually depends
on (H1a, H1b, H1c) are the ones this ADR finds gate-clear — their DVs do
not require SC-4/SC-5, and the sanity conditions they *do* depend on are
already shown reachable under `decision.satisficing`. **H3's problem is
real, named, and consequential for E3 and for §30.3/§30.9 as currently
drafted — but it does not threaten the project's continuation past Phase 3
under the letter of §2.5.** Conflating "a secondary hypothesis has a
testability problem" with "the kill criterion is triggered" would overstate
this finding; the manual text does not support that reading.

### A further, narrower consequence for §30.9 — flagged, not resolved

§30.9's E1 filing checklist requires, as a flat bullet: "Sanity conditions
SC-1…SC-6 satisfied (§16.2)" — unqualified by hypothesis. §30.3 currently
declares predicted signs for **H1a, H1b, H1c, H2, H3, H4** as part of *this*
pre-registration document. As long as H3's predicted sign remains in §30.3,
§30.9's flat SC-1…6 bullet is not satisfiable by this ADR's per-hypothesis
reading (SC-4/SC-5 remain unmet, and are needed for H3 specifically) — this
is a distinct, later gate from the Phase-2 criterion-8 question this Stage
was asked to resolve, and is not addressed by this ADR. It resolves itself
automatically if option 2 above (descoping H3 from §30.3) is taken; it does
not resolve automatically under option 1 until that revision lands. Named
here so it is not rediscovered as a surprise when Phase 3 planning reaches
§30.9's checklist.

## Alternatives

- **Leaving ADR-0043's unqualified "gate is met" claim standing.** Rejected
  — it rests the whole gate on Arm C (10/315 cells, no plugin variation
  matching Arms A/B), which does not establish that the hypothesis-bearing
  arms are alive, and it does not surface H3's untestability at all.
- **Treating H3's untestability as a defect requiring an immediate fix.**
  Rejected for this Stage — per the project's standing rule, a
  contribution-shaping model change needs an adversarial literature check
  first; fixing it here, under time pressure from a gate question, is
  exactly the failure mode that rule exists to prevent. Flagged instead.
- **Reading H3's untestability as triggering the §2.5 kill criterion.**
  Rejected — the kill criterion's text names H1a/H1b/H1c only; extending it
  to H3 is not supported by the manual and would overstate a real but
  narrower problem.

## Consequences

- **Positive.** The gate question now has a precise, falsifiable per-
  hypothesis answer instead of a single number resting on the null arm.
  H1a/H1b/H1c — the kill-criterion-relevant hypotheses — are confirmed
  gate-clear with a stated mechanism (their DVs don't depend on shaping,
  and the SC's they do depend on are already shown reachable). H3's real
  problem is named, evidenced (Stage 6b's own shocked-config run), and
  given two concrete paths forward, neither silently deferred nor patched
  over.
- **Negative, accepted.** E3 ("the distinctive mechanism", §28.2) and the
  Arm-B shaping-lag factor within E1 remain blocked until one of the two
  options above is taken. §30.9's E1-filing checklist is not satisfiable
  as currently drafted (§30.3 still declaring H3's predicted sign) without
  one of those two paths.
- **Neutral.** No code change this Stage — this ADR is a documentation and
  analysis correction. ADR-0043 Decision 2 (the `constraint.enforce` fix)
  is unaffected and unchanged.

## Compliance

- This ADR itself is the record; no test changes accompany it (no code
  changed). `PROGRESS.md`'s Stage-6b framing is corrected to point here for
  the per-hypothesis reading; ADR-0043's `Status` line is updated (metadata
  only, per the house rule) to point to this ADR.
- Any future work on H3 (option 1 or 2 above) MUST cite this ADR as the
  finding it addresses, and MUST get its own ADR — this one does not
  authorise a fix, only names the problem.

## Note

§28.2 describes E3 ("Shaping availability ablation," testing H3) as **"the
distinctive mechanism"** — the one part of this model's design that is not
just another instantiation of BTOF/threat-rigidity search-width theory, but
specific to bringing RDT's constraint-shaping machinery into contact with
it. That is precisely the hypothesis this ADR finds untestable as built.
This is worth being honest about at exactly this level of stakes: it is not
a footnote, and it is not a project-ending finding either — it is a named,
evidenced, open problem, sitting outside the kill criterion, that Phase 3
planning needs to pick up deliberately rather than discover by surprise
when E3 produces a suspiciously clean null result.
