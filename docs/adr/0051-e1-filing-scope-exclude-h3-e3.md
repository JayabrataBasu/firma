# ADR 0051 — E1 filing-scope decision: file the H1a/H1b/H1c/H2/H4 sweep now, excluding the Arm-B shaping-lag factor and H3/E3

**Status:** Accepted (owner review complete, including explicit resolution
of Points 2 and 5 — see those sections; Point 4's manual-internal naming
inconsistency remains open, unresolved by this round). Per the project's
ADR immutability house rule, this document's body is now append-only —
any further correction is a new, superseding ADR, never an in-place edit
here, the `Status` line itself excepted.
**Phase:** 2→3 boundary (this decision governs what Phase 3's §26.5 gate
deliverable — "the full pre-registered E1 sweep" — actually consists of;
it makes no Phase-3 run itself)
**Supersedes:** Nothing. Does not modify, soften, or reopen ADR-0044's or
ADR-0050's text or disposition — see Point 1 below, stated repeatedly by
design.
**Relates to:** manual §2.4 (H3's exact text), §2.5 (kill criterion),
§4.1 (claim tiers), §26.5 (Phase 3 gate), §28.2 (the E1–E4 experiment
table; E1 = "H1a, H1b, H1c" per that table specifically — see the
manual-internal note in Point 4), §30.1–§30.11 (the E1 pre-registration
document this ADR scopes), ADR-0040, ADR-0042, ADR-0044 (the per-hypothesis
gate reading this decision executes "option 2" of), ADR-0050 (the H3
mechanistic finding and disposition this decision rests on and does not
alter)

## Context — read this before Point 1

§30.9's filing checklist requires, as a flat, unconditional bullet:
"Sanity conditions SC-1…SC-6 satisfied (§16.2)." SC-4/SC-5 are not
satisfied under `decision.satisficing` (ADR-0040, ADR-0042, sharpened
mechanistically by ADR-0050) — this remains true and is **not changed by
this ADR**. H3 (manual §2.4) stays open per ADR-0050's disposition: not
descoped, not falsified, not found unviable. ADR-0044 already named two
paths forward and took neither: (1) a targeted model revision (attempted,
partially, in ADR-0047–0049; ADR-0050 found it does not resolve the
untestability in the realistic scenario), or (2) "an explicit, documented
descope of H3 from the initial E1 filing... removing its predicted sign
from §30.3... Flagged for a §2.4/§30 PATCH, not decided [by ADR-0044]."

This ADR takes path (2) — but ADR-0044 itself used the word "descope"
loosely, for H3's *presence in this particular filing document*, not for
H3 as a hypothesis. That looseness is exactly the trap this ADR must not
fall into. **This ADR makes a narrow, filing-scope decision: register and
file E1 now, for H1a/H1b/H1c/H2/H4, in a version whose Arm-B design
excludes the shaping-lag factor and therefore does not carry H3's
predicted comparison. It is not a decision about H3's status as a
hypothesis, which ADR-0050 already settled and this ADR does not
revisit.**

## Decision

### Point 1 — This is not, and must never be read as, a descoping of H3 as a hypothesis

**Stated here, and restated at every point below where the exclusion
itself is described, so it cannot be quoted out of context in isolation.**
ADR-0050's disposition — H3 is open, not descoped, not falsified, not
found unviable, retained as a named, tracked target for future work — is
**unchanged, unmodified, and not superseded by any part of this document.**
What this ADR decides is narrower and different in kind: which
hypotheses' predicted comparisons are included in **one specific filing
document** (the E1 pre-registration, §30). Excluding H3's factor from that
one document is a statement about *what is being filed*, not a statement
about *H3's truth, testability in principle, or the project's continued
interest in it*. A reader who encounters only the phrase "H3 excluded from
the E1 filing" without this qualification has been given a misleading
summary of this ADR, not an accurate one.

### Point 2 — §30.9's checklist bullet is read as scoped to the filed design (owner-decided); this does not retroactively satisfy the bullet in general, and does not touch the already-resolved Phase 2 criterion 8

Two distinct gates exist in this project's history, and this ADR touches
only the second:

1. **Phase 2's own criterion 8** (§27.3/§16.2, "the model is
   scientifically alive") — **already resolved, per-hypothesis, by
   ADR-0044.** H1a/H1b/H1c/H2/H4 are gate-clear (their DVs need only
   SC-1/SC-2/SC-3/SC-6, already shown reachable); H3 needs SC-4/SC-5 and
   is not gate-clear. This ADR does not reopen, re-decide, or restate that
   finding as if it were new — it cites it as settled.
2. **Phase 3's §30.9 filing checklist** — a later, separate gate,
   specific to *what gets pre-registered and filed as E1*. This is the
   gate this ADR addresses.

**Stated precisely, not implied:** SC-4/SC-5 remain unsatisfied under
`decision.satisficing`, full stop — that fact does not change because this
ADR excludes H3/E3 from the filing. What changes is narrower: for the
hypotheses actually being filed (H1a/H1b/H1c/H2/H4), ADR-0044 already
established that their own DVs never required SC-4/SC-5 in the first
place, and the SCs they *do* need are already shown reachable. **This ADR
does not make §30.9's flat "SC-1…SC-6 satisfied" bullet true in any
general sense — it makes the filing this bullet applies to no longer
include the one hypothesis whose testability genuinely depends on the
unsatisfied SCs.**

**Owner-decided (this round): §30.9's checklist bullet is read as scoped
to the design actually being registered and filed, not as a literal,
unconditional requirement spanning hypotheses not included in that
filing.** Under this reading, the narrowed E1 (H1a/H1b/H1c/H2/H4, 195
cells) honestly clears this checklist bullet as written — no manual change
is required to file it. This is a decided *reading* of the existing
bullet's scope, not a retroactive claim that SC-4/SC-5 hold; the two
paragraphs above remain true and unmodified by this decision. §30.9's
bullet text itself is not amended by this ADR (per this project's
convention: code and ADRs proceed under the ADR's documented reading, the
manual itself is patched later in a batch) — the exact wording it should
eventually carry, so a later H3/E3 filing does not need this same question
re-decided (Point 6), is logged as a manual-PATCH candidate in
`PROGRESS.md`, not applied to `docs/MANUAL.md` here.

### Point 3 — This is not a backdoor to future tuning

**Standing constraint, imposed by this ADR on all future work, until
superseded by a new ADR:** nothing about this filing-scope decision
changes the anti-tuning discipline already established (ADR-0042's
width-independence finding; ADR-0044's per-hypothesis reasoning). No
future attempt to re-include H3/E3 in a filing may be made by adjusting
thresholds, costs, or success-model parameters until SC-4/SC-5 happen to
pass. **The only legitimate paths back are:**

- a genuine model revision, subject to its own ADR and an adversarial
  literature check (the same rule ADR-0044 and ADR-0047 already invoke),
  or
- one of the three revisit triggers ADR-0050 already named: the Phase-4
  `decision.optimizing` build (manual §26.6, §28.2 E7); relaxation of
  either of ADR-0050's two named simplifications (standing
  political/relational capital, or the single-action-per-tick
  constraint); or a future SC-1…6 re-run, on any config, showing
  materially different shaping-selection rates than currently documented.

Tuning `w_max`, `β`, `κ_a`, a success-model coefficient, or the gate's
comparison specifically to make SC-4/SC-5 pass is explicitly named here as
**out of bounds** for the purpose of re-including H3/E3 in a filing — this
would be exactly the "probing for a configuration that makes shaping fire"
failure mode this investigation's diagnostic instructions repeatedly
warned against, retroactively applied as a standing rule.

### Point 4 — What falls out of scope, concretely

**The affected §30.4 factor row: "Shaping lag," levels `(1,1)`, `(2,6)`,
`(8,16)`, Arm B only.** Confirmed directly from the current manual text
(re-read for this ADR, not assumed from an earlier session's quote),
§30.4's design table and §30.7's per-hypothesis analysis formulas:

| Hypothesis | Arm | Uses the Shaping-lag factor in its own analysis (§30.7)? |
|---|---|---|
| H1a, H1b | A | No — Arm A does not carry this factor at all |
| H1c | B | No — `ΔH_rep` fit on linear/quadratic `h`/`ς` terms |
| H2 | B | No — contrasts novelty vs. magnitude coefficients |
| **H3** | B | **Yes — the only formula referencing it: "three-way interaction `β × Δ_ℓ × time-to-boundary`"** |
| H4 | B | No — Cox proportional hazards, stated without a lag term |

Shaping lag (`Δ_ℓ`) is Arm B's factor, but **only H3's own analysis
formula reads it.** Removing it does not remove any factor H1c/H2/H4
actually need.

**New cell count, computed directly, not left as "fewer cells":**

| Arm | Factors (levels) | Cells before | Cells after |
|---|---|---|---|
| A | `h`(5) × `ς`(5) × `β`(5) | 125 | 125 (unaffected) |
| B | `β`(5) × Novelty(3) × Shock magnitude(4) × ~~Shaping lag(3)~~ | 180 | **60** |
| C | `β`(5) × Decision plugin(2) | 10 | 10 (unaffected) |
| **Total** | | **315** | **195** |

At 200 seeds/cell (§30.6, unchanged): **63,000 → 39,000 runs.** 120 cells
removed, all from Arm B, all attributable to dropping the one 3-level
factor.

**Arm C — confirmed unaffected, reasoned through, not assumed.** §30.4's
design table lists "Shaping lag" with `Arm = B` only; Arm C's own row
("Decision plugin: `satisficing`, `random`" — Arm C) carries no lag
factor. Arm C's stated purpose ("Null. `β=0` and `decision.random`. Both
must show no relationship") is a negative control for the *narrowing*
hypotheses (H1a/H1b/H1c), not a test bed for H3 — ADR-0044's per-hypothesis
table maps H3 to Arm B exclusively, and `decision.random`'s own repertoire
configuration (`cfg_random_binding`, ADR-0043) is not swept over shaping
lag as a registered E1 factor at all (it is a fixed config choice in a
sanity-condition test, not one of §30.4's Arm C cells). **Arm C is
untouched by this ADR in every respect.**

**A manual-internal inconsistency surfaced while confirming these numbers,
named here rather than silently smoothed over (this project's standing
practice — see CLAUDE.md, "when this file and the manual disagree... say
so"):** §28.2's own experiment table names **E1** as testing "H1a, H1b,
H1c" specifically ("Primary result"), with H2, H3, H4 as the *separate*
experiments E2, E3, E4. But §30's actual pre-registration document, titled
"Pre-registration: Experiment E1," bundles all six hypotheses (§30.3) and
all of Arm B's factors — including Novelty (E2's own factor) and Shaping
lag (E3's own factor) — into one shared Arm-B design. §30's document is
therefore, textually, a combined pre-registration for E1+E2+E3+E4's shared
Arm-B grid under one filing labeled "E1," not literally scoped to what
§28.2's table calls "E1." This ADR's exclusion is scoped precisely to
**§30's document as it exists** (drop the Shaping-lag factor, keep
Novelty/Shock-magnitude, which H2/H4 need) — it does not attempt to
resolve the §28.2-vs-§30 naming inconsistency, which predates this ADR and
is flagged here as its own candidate for a manual PATCH, separate from
this filing-scope decision.

### Point 5 — §30.3 and §30.8: owner-decided — retain, annotated, not removed

**Owner-decided (this round): adopted as recommended, without
modification.** H3 stays in §30.3's hypothesis list and its row stays in
§30.8's falsification table; neither is deleted. Both are annotated —
"excluded from this filing's registered design; see ADR-0051." Reasoning
(unchanged from the recommendation this decision adopts): deleting the row
would make the manual's own text silently imply H3 was never a hypothesis
of this project at all, which is false and inconsistent with ADR-0050's
explicit "open, not descoped" disposition; retaining it with an annotation
keeps the historical and scientific record honest while still making
unambiguous that it is not part of *this* filing's registered comparisons.
**The decision is made; the manual edit implementing it is not made
here.** Consistent with this project's convention (code and ADRs proceed
under the ADR's documented reading; the manual itself is patched later in
a batch — the Stage-0 precedent in `PROGRESS.md`), the exact annotation
text for both §30.3 and §30.8 is logged as a manual-PATCH candidate in
`PROGRESS.md`, not applied to `docs/MANUAL.md` as part of this ADR.

### Point 6 — The re-filing path

H3/E3 get their own, later, separate pre-registration when **any one** of
ADR-0050's three revisit triggers is met (restated from Point 3 for
completeness, not introduced here): the `decision.optimizing` build
(Phase 4, §28.2 E7); relaxation of either named simplification (standing
political/relational capital, or single-action-per-tick); or a materially
different SC-4/SC-5 result on any future config re-run. At that point, a
new pre-registration document (§30-shaped: its own hypothesis statement,
design, falsification table, filing checklist) covers H3/E3's Arm-B
shaping-lag sweep specifically.

**Confirmed: a later H3/E3 filing does not require re-registering or
re-running any part of the narrowed E1 this ADR scopes.** It is additive.
The narrowed E1 (H1a/H1b/H1c/H2/H4, 195 cells, 39,000 runs, seed range
1–200 per §30.6) is a complete, independently interpretable pre-
registration once filed and executed — nothing about a later H3/E3
registration revises its design, its already-consumed seed range, or its
results. Per §30.6's existing rule ("Any Phase 2 run touching the
registered design grid invalidates this registration and requires
re-registration with a fresh seed range"), the same principle — a fresh,
non-overlapping seed range for a new registration — should govern an H3/E3
filing's own seed choice, so the two filings' CRN forks never collide;
this ADR does not itself assign that future seed range.

### Point 7 — This ADR runs nothing, revises no model, changes no code

**Stated explicitly, mirroring ADR-0050's own discipline:** this document
makes a scope decision only. It does not execute any run, does not modify
`decision.satisficing`, `time_to_boundary`, any threshold, cost, or
success-model parameter, and does not touch any code, test, or config
file. The actual E1 run configurations reflecting this narrowed 195-cell
design (including whatever choice is made about whether `shaping` remains
configured, at a fixed non-swept value, in Arm-B firms' repertoires, or is
omitted entirely — a genuine open implementation question this ADR
deliberately does not resolve) are Phase 3 build work, not part of this
ADR.

### Point 8 — What this rests on

This decision is a natural, narrow next step from two already-accepted
findings, not a new independent judgment about H3's status:

- **ADR-0044** established, per-hypothesis, that H1a/H1b/H1c/H2/H4 are
  gate-clear (Phase 2 criterion 8) without SC-4/SC-5, and that H3 is
  untestable as built — and named "an explicit, documented descope of H3
  from the initial E1 filing" as one of exactly two unattempted paths
  forward.
- **ADR-0050** deepened *why* H3 is untestable (the scan-order
  deprioritization mechanism, with real trace numbers) and set its
  disposition: open, not descoped, not falsified, not found unviable,
  with three named revisit triggers.

This ADR executes ADR-0044's named path (2), narrowly, as a filing-scope
decision — using ADR-0050's disposition and revisit triggers as the
standing conditions under which that scope decision is reopened, not as
something this ADR modifies.

## Alternatives

- **File the full, unmodified 315-cell E1, accepting that §30.9's
  checklist is not honestly clearable.** Rejected — this would either
  block Phase 3 indefinitely on a hypothesis the kill criterion does not
  depend on (ADR-0044), or invite filing a non-compliant pre-registration,
  neither acceptable.
- **Formally descope H3 as a hypothesis** (remove it from §2.4, mark it
  falsified or abandoned). Rejected outright — none of the owner's three
  conditions for setting a hypothesis aside (proven unviable, proven
  false, project-integrity threat) has been met; this would directly
  contradict ADR-0050's disposition, which this ADR is not authorized to
  and does not modify.
- **Silently drop the Shaping-lag factor from §30.4 without a dedicated
  ADR**, treating it as an implementation detail. Rejected — per this
  project's standing rule, any deviation from what an existing
  pre-registered design specifies requires its own ADR, not a quiet
  config change; this is exactly the kind of decision that needs a
  citable, findable record given how easily "H3 excluded from filing" can
  be misread as "H3 abandoned" (Point 1).
- **Wait for a fourth model-revision attempt before filing anything.**
  Rejected for this ADR's purpose — a further model revision requires its
  own ADR and adversarial literature check (Point 3), and Phase 3's own
  gate (§26.5) does not depend on H3 at all; delaying the entire E1 filing
  on a non-kill-criterion hypothesis would be a scope decision at least as
  consequential as the one this ADR makes explicitly, made instead by
  default and unexamined.

## Consequences

- **Positive.** The narrowed E1 (H1a/H1b/H1c/H2/H4, 195 cells, 39,000
  runs) can honestly proceed toward filing on grounds ADR-0044 already
  established — no new tuning, no new model change, no reopening of H3's
  status. Phase 3's actual gate (§26.5: "H1a/H1b/H1c answered... Either
  answer passes") never depended on H3 in the first place, so this scope
  decision does not put Phase 3's own completion at risk. H3 stays
  correctly recorded as open, with an explicit, concrete re-filing path
  (Point 6) rather than being silently dropped from the project's
  hypothesis set. **Points 2 and 5 are now owner-decided** (scoped
  §30.9 reading adopted; §30.3/§30.8 retain-with-annotation adopted) — the
  narrowed E1 honestly clears §30.9's filing checklist bullet as written,
  under that decided reading, without requiring a manual edit first.
- **Negative, accepted.** E3 and the Arm-B shaping-lag factor within E1
  remain filed separately, later, contingent on one of three named
  triggers — a real, accepted delay to H3's own testing, not resolved by
  this ADR. The §28.2-vs-§30 naming inconsistency (Point 4) is surfaced
  but remains open, not fixed here — unlike Points 2 and 5, no owner
  decision on it was made in this round. The actual `docs/MANUAL.md` edits
  implementing the Point 2/Point 5 decisions are also not made by this
  ADR — logged as PATCH candidates in `PROGRESS.md`, applied later in a
  batch per this project's convention.
- **Neutral.** No code, config, model, or test file is touched. No run is
  executed. `PROGRESS.md`'s new tracked entry (below) cross-references
  this ADR.

## Compliance

- This ADR itself, plus the corresponding `PROGRESS.md` entry, are the
  record. No test or code changes accompany it.
- Any future attempt to re-include H3/E3 in a filing MUST cite this ADR
  and MUST proceed via one of the two paths named in Point 3 (a new,
  literature-checked model-revision ADR, or a demonstrated revisit
  trigger from ADR-0050) — adjusting parameters to force SC-4/SC-5 without
  either is explicitly out of bounds per this ADR.
- Points 2 and 5 are owner-decided as of this round; their corresponding
  manual-PATCH candidate text (logged in `PROGRESS.md`, not applied to
  `docs/MANUAL.md`) MUST be carried into the next manual-revision batch
  faithfully to what those Points now state, not re-derived from scratch.
  Resolving the §28.2-vs-§30 naming inconsistency (Point 4) still requires
  the owner's decision — this ADR does not authorize it.

## Note

The easiest way to get this kind of decision wrong is not in the decision
itself but in how it gets *summarized* later — "H3 was dropped from E1"
quietly drifting, a few retellings later, into "H3 was dropped." This ADR
was written under the assumption that it will eventually be read out of
context, by someone who has not read ADR-0044 or ADR-0050 first, and tried
to make every section survive that reading intact rather than relying on
this Context section alone to carry the qualification.
