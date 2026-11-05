# ADR 0052 — Manual naming patch: §28.2's experiment table relabeled to resolve the "E1" collision with §30, applied directly as an owner-directed exception

**Status:** Accepted (owner-directed; applied 2026-09-06).
**Phase:** documentation, Phase 2→3 boundary (surfaced while confirming
ADR-0051's cell-count computation)
**Supersedes:** Nothing.
**Relates to:** manual §0.6 (change control / versioning), §28.2 (the
table this ADR edits), §30 (the pre-registration document whose "E1"
usage is preserved), ADR-0051 Point 4 (where the collision was first
named, as a flagged-not-fixed observation)

## Context

ADR-0051 Point 4, while computing the narrowed E1 design's cell count,
surfaced and flagged (without fixing) a manual-internal naming
inconsistency: §28.2's experiment-sequence table names **"E1"** as testing
H1a/H1b/H1c specifically, with H2/H3/H4 as separate experiments E2/E3/E4.
§30's actual pre-registration document, titled "Pre-registration:
Experiment E1," bundles **all six hypotheses** (§30.3) and **all of Arm
B's shared factors** — including Novelty (nominally E2's own factor per
§28.2) and Shaping lag (nominally E3's) — into one filing also called
"E1." Every Accepted ADR that cites "the E1 pre-registration" (confirmed
by direct grep below) uses "E1" in §30's bundled sense, not §28.2's
one-hypothesis-per-experiment sense.

This ADR is **an explicit, owner-directed exception** to this project's
normal convention (log a manual-PATCH candidate in `PROGRESS.md`, apply it
later in a batch — the Stage-0 precedent). The owner directed this one
patch be applied directly, now, rather than batched — recorded here as the
exception it is, not as a new general practice.

## Decision

### (a) What was renamed, and why

**§28.2's table rows E1, E2, E3, E4 → "Result 1," "Result 2," "Result 3,"
"Result 4"** (rows for H1a/H1b/H1c, H2, H3, H4 respectively). E5 through
E11 are **untouched** — they are not part of the collision (see (c)
below).

**Direction: rename the §28.2 side, not §30's — decided by the owner,
reasoning restated here for the citable record.** §30's usage is what
every Accepted, append-only ADR (ADR-0044, ADR-0050, ADR-0051, and,
per the grep in (c), earlier ones too) already cites. Renaming §30 would
retroactively make those already-Accepted ADRs' citations read incorrectly
against current manual text — worse than the collision itself, since it
would silently invalidate historical citations rather than resolve an
ambiguity. §28.2's table is the smaller, more local piece of text, and
nothing outside that table depends on its specific "E1"–"E4" spelling
(confirmed in (c)).

**Label chosen: "Result *N*," not a new "E"-prefixed label.** The
instruction's own constraint — do not invent a label that could create a
*second* collision — was checked against two tempting alternatives and
both were rejected before "Result *N*" was chosen: "R*N*" collides with
§14.2's already-established `R1`/`R2`/`R3` (the three rigidity
operationalisations — `R3` specifically is "shaping abandonment," a
different concept entirely from "Result 3 / H3"); "N*N*" collides with
§30.7's `N1`/`N2` (the two novelty operationalisations). "Result *N*"
(a full word, not a letter-number code) collides with neither, and reads
naturally against the table's existing "Purpose" column language ("Primary
result," "the distinctive mechanism," etc.).

**One clarifying sentence was added directly under the table**, not
merely a silent rename, because the instruction's target is the
*ambiguity*, not just the label string: *"Results 1–4 are reported
together under one shared Phase-3 pre-registration filing (§30,
'Pre-registration: Experiment E1' in that document's own naming) — they
are not four independently registered experiments, and 'Result *N*' here
is deliberately not an 'E*N*' label so it cannot be misread as one."* A
future reader hitting either §28.2 or §30 now has the cross-reference
explaining why the two documents' numbering schemes differ, rather than
being left to rediscover the collision the way this ADR's own predecessor
(ADR-0051) had to.

### (b) Applied directly, not batched — the exception, recorded plainly

This patch was applied directly to `docs/MANUAL.md` in this instruction,
**not** logged as a `PROGRESS.md` PATCH candidate for later batching (the
project's normal convention, per the Stage-0 precedent and per ADR-0051's
own Points 2/5 manual-PATCH candidates, still pending). This is an
explicit, one-time, owner-directed exception — not a new standing
practice. Every other manual-PATCH candidate this project has accumulated
(§30.9's checklist-scoping reword, §30.3/§30.8's H3 annotations from
ADR-0051, §19.4's delta-uniqueness wording from OQ-10, §12.3's "one-step
lookahead" ambiguity from ADR-0047, and others) remains logged and
unapplied, governed by the normal batching convention. This ADR does not
change that convention for anything but this one item.

**Version discipline, per §0.6 (the manual's own change-control table,
read directly, not assumed):**

| Change | Bump |
|---|---|
| Altering a MUST, a primitive, or the formal model | MAJOR |
| Adding a phase, plugin category, metric, ADR, or contract | MINOR |
| Clarification, typo, expanded rationale | PATCH |

This change is a clarification — a table-label rename plus one explanatory
sentence, altering no MUST, primitive, formal-model element, phase,
plugin category, metric, ADR, or contract. **PATCH**, per §0.6's own
table. `docs/MANUAL.md`'s version field bumped `1.0.0` → `1.0.1`, its date
line annotated, and its closing line ("*End of FIRMA Project Manual
v1.0.1*") updated to match. `CLAUDE.md`'s own citation of the manual's
version (`docs/MANUAL.md` (v1.0.0)) was also updated to `v1.0.1` for
consistency, since it would otherwise go stale the moment this patch
landed. **The manual has no dedicated changelog section** (checked
directly — only the `Version:`/`Date:` header fields and the closing
line carry version information); this PATCH's provenance is recorded in
the version-field annotation itself and in this ADR, which is the closest
equivalent this project currently has to a changelog entry.

**A real consequence, found and deliberately NOT fixed as part of this
exception, flagged instead:** `firma_core::MANUAL_VERSION` (a Rust
constant, currently `"1.0.0"`) is written into every run's `RunIdentity`
— and `RunIdentity`'s canonical-JSON SHA-256 **is** the run's `run_id`
(manual §22.3; `crates/firma-io/src/manifest.rs`, confirmed by reading the
struct and `run_id()` directly). Updating that constant to `"1.0.1"` to
match this patch would therefore change **every** `run_id` this codebase
computes, including every committed golden trace and frozen smoke-test
hash (`golden_trace_unchanged`, `phase1-smoke`, `phase2-smoke`,
`phase2-stage5-smoke`) — a code-level ripple across the whole test suite,
categorically outside what "apply this one [manual] patch directly" was
scoped to authorise. **`firma_core::MANUAL_VERSION` is left at `"1.0.0"`,
now stale relative to the manual's own version field, exactly the kind of
silent drift this project's discipline exists to prevent — logged here,
not fixed, as its own follow-up item** (a separate, deliberate decision:
either bump the constant and accept/regenerate the resulting golden-trace
diffs, or reconsider whether every manual version digit really needs to be
load-bearing in `run_id` given PATCH-level manual changes are, by §0.6's
own table, defined as not altering trajectory-determining content at all).

### (c) Confirmed: no other manual section's meaning altered — grep evidence

Full-manual grep for `E1`–`E11` (word-boundary matched, run before any
edit) found every occurrence; classified below:

- **§30-side "E1" (untouched by this ADR):** the table of contents entry
  (`| 30 | Pre-registration: Experiment E1 |`) and §30's own header (`#
  30. Pre-registration: Experiment E1`) — both are the usage this ADR
  preserves, per the rename-direction decision in (a).
- **§28.2's table itself (the only rows edited):** `E1`–`E4`, now `Result
  1`–`Result 4`.
- **`E2`, `E3`, `E4` elsewhere in the manual: zero occurrences outside the
  table.** Confirmed directly — neither symbol appears anywhere else in
  `docs/MANUAL.md`.
- **`E5`–`E11`: every occurrence outside the table is self-consistent with
  the table's own labels and untouched by this rename** — `E5` (§28.2's
  own emphasis sentence, unedited), `E7` (§12.3's `decision.optimizing`
  discussion, §20.3's plugin-category text, and §28.2's own row — three
  consistent references, all still correct), `E11` (§28.3's unitary-vs-
  coalitional discussion and its own row — both consistent, unaffected).
  `E6`, `E8`, `E9`, `E10` appear only inside the table itself, nowhere
  else, so nothing outside it could be broken by leaving those four rows
  unchanged.
- **No other section cross-references §28.2 by number** — grepped
  directly for `§28.2`/`28.2`: the only hit is the section's own header.
  No other part of the manual assumes §28.2's table uses specifically
  "E1"–"E4" as opposed to any other label for those four rows.

**One thing surfaced by this grep, named rather than silently absorbed,
but explicitly out of this exception's authorised scope (the instruction
named exactly "the §28.2-vs-§30 'E1' collision," not a broader review):**
§30.9's filing checklist requires "AT-1…AT-5 run and reported" (§28.2's
`E6` content, verbatim) and §30.7's robustness requirement names "R1, R2,
R3 and... N1 and N2" (substantively `E5`'s "rigidity-measure convergent
validity" concept). Both are, like Results 1–4, effectively folded into
what §30 requires of the same shared "E1" filing — a softer version of
the same collision this ADR fixes for E1–E4, but for `E5`/`E6`
specifically. **Not touched here** — the instruction's authorisation was
scoped to the four colliding rows named explicitly, and extending it
unilaterally to `E5`/`E6` would be exactly the kind of scope-creep this
project's "ask, don't assume" discipline exists to prevent. Flagged for a
future, separately-authorised look, not fixed now.

## Alternatives

- **Rename §30's "Experiment E1" instead of §28.2's rows.** Rejected —
  reasoned through in (a): would retroactively misalign every Accepted
  ADR's existing citation of "the E1 pre-registration," which is worse
  than the ambiguity it would resolve.
- **Invent new "F1"–"F4" or similar single-letter-prefixed labels.**
  Rejected — checked directly against every existing single/double-letter
  scheme in the manual (`R1`–`R3`, `N1`–`N2`, `V1`–`V8`, `AT-1`–`AT-5`,
  `DT-1`–`DT-6`, `SC-1`–`SC-6`, `VT-1`–`VT-8`) before choosing "Result
  *N*" specifically to avoid colliding with any of them.
- **Also relabel `E5`/`E6`** to fully resolve the softer collision named in
  (c). Rejected for this ADR — outside the instruction's explicit
  authorisation (named exactly the four-row §28.2-vs-§30 "E1" collision),
  and this project's standing discipline treats scope-expansion during an
  explicitly-bounded exception as exactly the failure mode to avoid.
- **Also bump `firma_core::MANUAL_VERSION`** to keep the code constant and
  the manual's own version field in sync. Rejected for this ADR — a
  code-level change with a whole-test-suite hash ripple, categorically
  outside "apply this one manual patch directly"; named as a follow-up
  item in (b) instead.

## Consequences

- **Positive.** The §28.2-vs-§30 "E1" collision ADR-0051 surfaced is
  resolved at the source, not merely re-flagged a second time. A future
  reader of either §28.2 or §30 now finds the cross-reference explaining
  why the two documents' numbering differs.
- **Negative, accepted.** `firma_core::MANUAL_VERSION` is now stale
  relative to `docs/MANUAL.md`'s own version field (`"1.0.0"` vs.
  `1.0.1`) — a real, named inconsistency, deliberately not fixed here
  given its whole-test-suite hash ripple; a follow-up decision (bump and
  regenerate golden traces, or reconsider whether manual PATCH bumps
  should be trajectory-hashed at all) is still needed. The softer
  `E5`/`E6` collision named in (c) remains unresolved, out of this
  exception's scope.
- **Neutral.** One table edited, one explanatory sentence added, two
  version-field updates (`docs/MANUAL.md`, `CLAUDE.md`) — no code, test,
  or config file touched; no ADR text (ADR-0044/0050/0051) touched.

## Compliance

- This ADR is the record of the exception and its reasoning. No test
  changes accompany it (no code changed).
- Any future manual patch reverting or extending this rename (e.g., a
  later decision to also relabel `E5`/`E6`) MUST cite this ADR.
- The `firma_core::MANUAL_VERSION` staleness named in (b)/Consequences
  MUST be resolved by its own deliberate decision (its own ADR if the
  constant is bumped, given the golden-trace regeneration it would force)
  before being silently left to drift further across future manual PATCH
  bumps.

## Note

This ADR is unusual for this project in being the *documentation* twin of
what an ordinary code ADR does when it fixes a small, real defect found
mid-task: the fix is made, the reasoning is recorded, and the thing that
was *deliberately not* fixed alongside it (the `MANUAL_VERSION` constant,
the softer `E5`/`E6` collision) is named exactly as carefully as the thing
that was.
