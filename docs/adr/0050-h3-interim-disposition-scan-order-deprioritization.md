# ADR 0050 — H3 interim disposition: `decision.satisficing`'s fixed-order scan structurally deprioritizes shaping; investigation remains open, not descoped

**Status:** Accepted (owner review complete). Per the project's ADR
immutability house rule (`docs/adr/README.md`), this document's body is now
append-only — any further correction is a new, superseding ADR, never an
in-place edit here, the `Status` line itself excepted. (For the record:
this document was drafted and reviewed under a literal "DRAFT — pending
owner review" status line, unlike ADR-0047–0049's "Accepted — flagged for
owner review before merge" phrasing for the same not-yet-settled state —
the discrepancy was noted at draft time rather than silently harmonized,
and is preserved here now that review is complete.)
**Phase:** 2 (Model), post-Stage-7, consolidating four diagnostic tracing
rounds run after ADR-0049 (same branch, `h3-satisficing-lookahead`, not
merged)
**Supersedes:** Nothing. This ADR does not correct or contradict any prior
decision — it consolidates new, more granular mechanistic evidence in
support of ADR-0044's finding ("H3 is untestable as built, not merely hard
to observe") and extends it with a specific account of *why*, at the level
of the scan mechanism, not just the observed `SC-4 = 0.0000` outcome.
**Relates to:** manual §2.4 (H3's exact text, quoted below), §2.5
(falsification/kill criterion), §4.1 (claim tiers — Tier 1/2/3), §12.3
(`satisfices()`, the fixed-order scan), §14.3 (`time_to_boundary`), §28.2
(E3 — "the distinctive mechanism"), §30.3/§30.9 (E1 predicted signs / filing
checklist), ADR-0040 (the `select()` seam), ADR-0042 (shaping is
width-independent fallback-only), ADR-0044 (per-hypothesis gate reading;
H3 untestable, not kill-criterion-relevant), ADR-0047/0048/0049 (the three
H3 model-revision rounds this document reports the tracing results of)

## Context

ADR-0044 established, from aggregate `SC-4`/`SC-5` statistics alone, that
H3 (manual §2.4: *"Shaping availability reduces narrowing when shaping lag
< time-to-boundary, and increases it when lag > time-to-boundary"*) is
**untestable as built** under `decision.satisficing` — shaping is never
selected at a rate that could support or contradict H3's predicted
comparison, in any arm, at any tested lag. ADR-0047–0049 then made three
successive owner-directed revisions to `satisfices()`'s `SURVIVAL` branch
and added a real decision-time race (`time_to_boundary`, §14.3) so that
shaping's lag-vs-danger-proximity comparison is a genuine, evaluated
mechanism rather than something the pre-revision code could never even
compute. Re-testing after all three rounds still showed the mechanism
essentially never gets selected in the realistic (`cfg_arm_b_satisficing`)
scenario.

This ADR does not make a fourth model revision. It consolidates **four
rounds of read-only diagnostic tracing**, run in this working session after
ADR-0049, that instrument the scan loop and the ADR-0047–0049 gate/payoff
computation directly (via a temporary, env-var-gated trace module added to
`firma-plugin-decision/src/lib.rs`, confirmed byte-identical
`event_log_sha256` with tracing on vs. off in every round) to establish
**why**, mechanistically, shaping is so rarely reached and, when reached,
so rarely selected — rather than leaving "SC-4 stays near zero" as an
unexplained aggregate fact.

## Decision

### 1. The finding

**`decision.satisficing`'s fixed-order, first-good-enough scan (§12.3
Steps 4–5, the `select()` loop) structurally deprioritizes shaping actions
relative to market actions — this is the dominant effect, and it operates
largely independent of how favorable the shaping payoff calculation
(ADR-0047's expected-relief formula) turns out to be, because market
actions occupy earlier positions in every focus's scan order and satisfy
the "good enough" bar first, before shaping is ever evaluated at all.**

Direct evidence, `cfg_arm_b_satisficing` (the Arm-B-shaped, shocked,
design-representative scenario — 10 firms, 400 ticks, 1484 total
decisions):

- **Lobby's checklist position is reached in only 75/1484 decisions
  (5.1%).** In the other **1409 (94.9%), the scan never gets there at
  all** — its search-width budget (`w_eff`) is exhausted by higher-priority
  admissible market actions before the scan reaches lobby's position
  (7th of 9 under `SURVIVAL` focus, 4th of 9 under `GOAL(1)`). This is the
  scan-order/narrowing effect described above, and it is the majority
  cause of zero shaping selections — **structurally prior to, and
  independent of, the ADR-0047–0049 payoff/gate mechanism.**
- Of the 75 reaches: 0 were inadmissible; 67 occurred under `SURVIVAL`
  focus (where the ADR-0048 timing gate applies) and 8 under `GOAL(1)`
  focus (the cost-only branch, unaffected by ADR-0047 by design).
- **Of the 67 `SURVIVAL`-focus reaches, the ADR-0048 gate
  (`lag_min < time_to_boundary`) closed in all 67 (100%)** — real numbers:
  `lag_min = 2`, `time_to_boundary = 1`, in every single one of the 67
  evaluations (zero variance), spanning 19 distinct ticks (7–39, all
  before either scheduled shock's onset at 40/80) and 5 of the 10 firms.
  **The actual expected-payoff calculation (ADR-0047's formula) never runs
  once in this scenario** — the gate is what stops it, not an unfavorable
  payoff.
- Contract (action 7) is not configured in this scenario at all (no
  `contract_cost`) — its zero-selection count has nothing to do with
  ADR-0047–0049; it is structurally absent from the repertoire.

**The gate/payoff mechanism itself does work when reached under different
conditions** — this matters for distinguishing "the ADR-0047–0049 revision
doesn't work" from "the revision works, but the realistic scenario rarely
lets the firm reach it in a state where it wins":

- `cfg_wide_search` at `w_max ∈ {6, 9}`, `β = 0.0` (no narrowing): 4/400
  decisions selected lobby. At those ticks (8, 17, both firms),
  `time_to_boundary` computed as unbounded (`None`), the gate opened, and
  the payoff comparison won on real numbers (`p_success = 0.45`,
  `e_h = 0.070 > h_t = 0.025` at tick 8; `e_h = 0.145 > h_t = 0.100` at
  tick 17). One tick earlier (tick 7, same firms), `time_to_boundary = 1`
  and the gate closed by the same `lag_min = 2` margin seen throughout
  `cfg_arm_b_satisficing`.
- The neighboring zero-shaping cell (`β = 0.5`, same `w_max`, same ticks)
  showed the scan-order effect directly: `w_eff` narrowed to 3 (from
  `ψ(h)`), and under that budget the scan never reached lobby's position
  (7th) at all — a market action satisficed first, within budget.
- A window-size generality check (widening `l_w` and `w_max` on
  `cfg_arm_b_satisficing`-derived variants) showed the same two-tier
  pattern: the `time_to_boundary = 1` lock still dominates (96–97% of
  evaluations at `l_w ∈ {6, 8}`), but a minority of evaluations — exactly
  the ones where the gate opens (3/75 at `l_w=6`; 12/363 at `l_w=8`;
  9/44 at `w_max=15`) — show a materially different `time_to_boundary`
  (`4`, `40`, or unbounded). These cluster in two distinct regimes: an
  early-run startup transient (ticks 3–7, before any shock) and the
  active shock-ramp window (ticks 47–52) — neither of which is the
  mid-run distress window (ticks 7–39) where the lock is observed.
  Shifting **lobby's own `LagRange`** (an earlier round's variant; the
  shock itself only had its *timing*, not a lag range, shifted) moved the
  margin by exactly the shift amount while `time_to_boundary` itself
  stayed fixed — since `time_to_boundary`'s actual inputs do not include
  the shaping action's lag range at all, ruling out a computational bug in
  `time_to_boundary` directly, both by reading its actual inputs and by
  this empirical result.

### 2. Model-vs-reality caveat

**This is a finding about what this specific formalization of firm
decision-making predicts — not a claim about how real firms behave.**
Two simplifications in the current model are named here explicitly, as
documented limitations rather than defects, because both plausibly bear on
why shaping is so hard to reach in this build:

- **No standing political/relational capital.** The model evaluates a
  shaping action as freshly costly every time it is considered — there is
  no representation of an existing government-affairs function, a
  standing lobbyist retainer, or accumulated relational capital that would
  make shaping cheaper, faster, or more likely to already be "in progress"
  for a firm that invests in it continuously, the way `κ_a` (the
  affordability cost) and the lag distribution are currently modeled as
  freshly incurred at the moment of commitment (§11.2/§11.3).
- **Single action choice per tick.** The model requires the firm to select
  exactly one action per tick (§12.3's scan returns one `a`), which forces
  shaping to compete one-for-one, in a fixed priority order, against
  market actions — rather than allowing a firm to pursue market operations
  and shaping in parallel, as real firms routinely do (running production
  while also running government relations). This structural exclusivity
  is a direct contributor to the scan-order deprioritization described in
  Decision 1.

**Connection to established theory — plausibility only, not validation.**
The direction of this finding — a threatened organization under-explores
novel options relative to familiar ones — is independently predicted by
both problemistic search (Cyert & March, 1963 — the project's own
foundational citation for `decision.satisficing`, §12.3) and
threat-rigidity theory (Staw, Sandelands & Dutton, 1981), via different
mechanisms than this model's literal fixed-order scan (problemistic
search predicts search stops at the first adequate option found in a
familiarity-ordered search; threat-rigidity theory predicts a threat
response that itself narrows the option set considered). **This citation
supports plausibility of direction, not validation of the model against
real data — no Tier 3 claim (§4.1: "Firms do *Y*.") is being made.** This
finding, and any claim built on it, is Tier 1 (a model claim: "under
`decision.satisficing`'s fixed-order scan, shaping is selected in
`N`/`M` decisions...") unless and until a translation contract (§4.4,
§35) is written to support a Tier 2 conditional prediction — none is
proposed here. **The Staw, Sandelands & Dutton (1981) citation's exact
details (journal, title, page range) have not been verified in this
session — this is flagged explicitly for the owner's review-partner
Claude to check against the literature before this ADR is finalized; a
local build agent verifying literature citations is out of scope for this
task and was not attempted.**

### 3. Disposition: open, not closed

**H3 remains an open hypothesis.** It is not descoped, not falsified, and
not found unviable. Per the owner's standing standard, a hypothesis is set
aside only if it is proven unviable (no legitimate configuration could
ever test it), proven false (tested and refuted), or found to threaten the
project's integrity if pursued further — **none of those three conditions
has been established by this document or by any of the four diagnostic
rounds it consolidates.** What has been established is a specific,
mechanistic account of *why* H3 is difficult to test under the current
model, which is itself a real, citable finding, reported here as one — not
as grounds for closing the investigation.

H3 is not part of the current E1 filing's critical path: per ADR-0044,
the manual's kill criterion (§2.5) names only H1a, H1b, and H1c, all three
already gate-clear; H3's untestability "does not threaten the project's
continuation past Phase 3 under the letter of §2.5" (ADR-0044). It remains
a named, tracked target for future work.

### 4. Revisit triggers

This disposition should be actively revisited — not left to be
rediscovered by surprise later — under at least these conditions:

- **When `decision.optimizing` (Phase 4, experiment E7, manual §28.2) is
  built.** A comparative, all-options-weighed decision procedure is
  structurally the natural test bed for whether shaping becomes reachable
  once the model is not restricted to first-good-enough, fixed-order
  search — Decision 1's finding is specifically about `decision.
  satisficing`'s scan structure, and does not claim anything about how a
  different decision plugin would behave.
- **If either of the two simplifications named in Decision 2 is ever
  relaxed** in a future model revision — standing political/relational
  capital represented in the shaping cost/success model, or the
  single-action-per-tick constraint relaxed to allow parallel action
  pursuit — H3 should be re-evaluated at that point, regardless of the
  reason the change was made for.
- **If a future SC-1…6 re-run, on any config, for any reason, shows
  materially different shaping-selection rates than currently documented**
  (this ADR's Decision 1 figures, or ADR-0042's/ADR-0044's `SC-4 = 0.0000`
  finding) — that would itself warrant revisiting whether this disposition
  still holds, independent of whether a deliberate model change caused it.

### 5. Pointers to the underlying evidence

The real trace numbers cited in Decision 1 come from **four diagnostic
tracing rounds run in this working session**, each governed by its own
instruction file from the owner (relayed by a review-partner Claude
session), consolidated here rather than restated in full:

1. **Round 1** — initial instrumentation of the scan loop and the
   ADR-0047–0049 gate/payoff computation (a temporary, env-var-gated
   `trace` module, four call sites, in
   `firma-plugin-decision/src/lib.rs`); full trace on `cfg_arm_b_satisficing`
   (1484 decisions) and on the two non-zero `cfg_wide_search` cells
   (`w_max ∈ {6, 9}`, `β = 0.0`), with a `β = 0.5` neighbor-cell contrast.
2. **Round 2** — margin-generality check: lobby's `LagRange` shifted ±1
   tick, and the shock schedule's onset shifted ±3 ticks, individually and
   combined, on `cfg_arm_b_satisficing`-derived variants, to test whether
   the exact "`time_to_boundary` short by 1 tick, every time" pattern was
   a computational artifact or scenario arithmetic. (It was arithmetic —
   `time_to_boundary` itself never moved in response to the lag-range
   shift, exactly as its actual input signature predicts.)
3. **Round 3** — generality check across window size (`l_w`, `w_max`
   widened and narrowed) and shock severity (magnitude halved/quartered
   and doubled), same methodology, surfacing the two-tier
   lock-vs-exception pattern described in Decision 1's last paragraph.
4. This ADR — consolidation only; no new tracing was run.

**This evidence has not yet been committed anywhere durable.** The trace
module in `firma-plugin-decision/src/lib.rs` is uncommitted working-tree
state (same branch, `h3-satisficing-lookahead`); the raw NDJSON trace
output and the fourteen-plus temporary variant config files it was run
against exist only in this session's scratchpad directory (not part of
the repository) and in the four instruction-file responses in this
conversation's transcript. **Flagged plainly: this is at real risk of
being lost** once the session ends, unless the owner chooses to preserve
it — e.g., committing the trace module (or a cleaned-up version of it)
and a representative sample of the raw trace output alongside this ADR,
or archiving the full transcript separately. This document's own citation
of specific numbers should not be treated as a substitute for the
underlying evidence remaining available to a future reader, consistent
with this project's standing rule that summaries are not themselves
evidence.

## Alternatives

- **Descoping H3 from the E1 filing now, since this document further
  documents the same untestability ADR-0044 already found.** Rejected —
  none of the owner's three descoping conditions (proven unviable, proven
  false, or project-integrity threat) is met by this document or the
  tracing it consolidates; doing so anyway would silently convert a
  documentation task into a scope decision it was explicitly instructed
  not to make.
- **Attempting a fourth model revision now** (e.g., representing standing
  political capital, or relaxing single-action-per-tick), since Decision 2
  already names two plausible mechanisms. Rejected for this document —
  out of scope for a documentation task, and per the project's standing
  rule, any further contribution-shaping model change needs its own ADR
  and an adversarial literature check first (the same rule ADR-0044 and
  ADR-0047 already invoke), neither of which has been done.
- **Leaving this consolidation undocumented, relying on the conversation
  transcript or the owner's memory of four separate instruction-file
  exchanges.** Rejected — this project's standing rule is that a finding
  of this weight gets a citable, durable record (an ADR), not an
  unreconstructable trail across sessions; this is precisely the purpose
  this document serves.

## Consequences

- **Positive.** The aggregate `SC-4 = 0.0000` finding (ADR-0042, ADR-0044)
  now has a mechanistic explanation with real, cited numbers, rather than
  standing as an unexplained statistic: the scan-order/narrowing effect is
  the dominant cause (94.9% of decisions never reach lobby's position at
  all); the ADR-0048 timing gate is a real, working mechanism (demonstrated
  opening and winning in `cfg_wide_search` and in the window-widened
  variants) that happens to close with unusual consistency in the specific
  mid-run distress window `cfg_arm_b_satisficing` puts its firms through.
  Two concrete, plausible-but-unverified simplifications (Decision 2) give
  future model-revision work a specific starting point rather than a blank
  page.
- **Negative, accepted.** H3 remains untestable under the current build —
  this document does not change that. E3 (§28.2, "the distinctive
  mechanism") and the Arm-B shaping-lag factor within E1 remain blocked,
  as ADR-0044 already found. The underlying trace evidence is currently
  unpreserved outside this session (Decision 5) — a real, near-term risk
  this document flags but does not resolve.
- **Neutral.** No code, config, or test file is changed by this ADR. The
  temporary trace instrumentation referenced in Decision 5 remains
  uncommitted, unchanged by this document.

## Compliance

- This ADR itself, plus `PROGRESS.md` OQ-12, are the record. No test
  changes accompany it (no code changed).
- Any future work that revises the model to address H3's untestability, or
  that descopes H3, MUST cite this ADR (and ADR-0044) as the finding it
  addresses — this document does not authorise either action, only names
  and evidences the problem, per the same posture ADR-0044 already took.
- Per the revisit triggers (Decision 4), a future `decision.optimizing`
  build, a relaxation of either named simplification, or a materially
  different SC-4/SC-5 re-run result should each prompt an explicit
  re-check of whether this disposition still holds, recorded in a new ADR
  rather than an edit to this one.

## Note

Round 1's `cfg_wide_search` result — 4 real, evidenced instances of the
ADR-0047–0049 mechanism actually working, gate opening and payoff winning
on real computed numbers — is worth holding onto precisely because it is
easy to lose sight of amid a headline "shaping is essentially never
selected" finding. The mechanism is not inert; it is rarely *reached* in a
state where it wins, under the specific scenario this project's realistic
Arm-B config puts firms through. That distinction — a rarely-triggered but
functioning mechanism vs. a broken one — is the kind of thing a future
reader re-deriving this from an aggregate `SC-4` number alone would not be
able to tell, which is exactly why this document exists.
