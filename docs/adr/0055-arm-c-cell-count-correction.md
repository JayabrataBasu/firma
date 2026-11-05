# ADR 0055 — Arm C's cell count: `decision.random` is not crossed with β — narrow correction to ADR-0051 Point 4's Arm C arithmetic

**Status:** DRAFT — pending owner review. Not implemented: `firma_lab.spec`
still builds the 10-cell Arm C this ADR says is wrong (see Compliance).
**Phase:** Phase 2→3 boundary (E1 registered-design arithmetic)
**Supersedes:** ADR-0051 **Point 4, Arm C row and the totals that depend on
it only** (`C | β(5) × Decision plugin(2) | 10 | 10 (unaffected)` and the
derived `195` cells / `39,000` runs). Same narrow shape as ADR-0032's
supersession of ADR-0029 Decision 2's ordering claim: nothing else in
ADR-0051 is touched — its filing-scope decision (exclude Arm B's shaping-lag
factor and H3/E3), H3's disposition (ADR-0050: open, not descoped), the
§30.9 scoped reading, and the §30.3/§30.8 annotation decision all stand.
**Relates to:** manual §28.4 (null models), §30.4 (design table and Arm C
prose), §12.3 (`ψ(h)`, the only place β acts), ADR-0027 (`decision.random`,
Accepted), ADR-0044 (cites "Arm C is 10 of 315 cells"), ADR-0051 (the
narrowed E1 design), OQ-16 (where this was first found)

## Context

ADR-0051 Point 4 computed the narrowed E1 design by taking §30.4's factor
table as a literal cross-product per arm. For Arm C that gives
`β(5) × Decision plugin(2) = 10`, matching the manual's own stated
"Arm C: 10." `firma_lab.spec.build_narrowed_e1_spec` implements exactly
that. Job-config generation then fails for the 5 `decision.random` cells
(a `KeyError` from Arm C's `beta` `Factor`, which hardcodes
`rule_id="decision.satisficing"`). Investigating why (OQ-16) found the
failure is a symptom: the cross-product itself contradicts an Accepted ADR
and the manual's own prose.

### The text this rests on, quoted exactly

**§30.4, Arm C prose:** *"**Arm C — Null.** β = 0 and `decision.random`.
Both must show no relationship."*

**§30.4, factor table rows marked for Arm C:** *"Narrowing β | 0, 0.5, 1,
2, 4 | A, B, C"* and *"Decision plugin | `satisficing`, `random` | C"*;
stated total: *"Arm A: 125 cells. Arm B: 180. Arm C: 10. **Total 315 cells
× 200 seeds = 63,000 runs.**"*

**§28.4, null models (two of five rows):** *"Random decision |
`decision.random` | Results require the decision mechanism"* and *"No
narrowing | β = 0 | Isolates the narrowing mechanism."*

**ADR-0027 Decision 2 (Accepted):** *"`decision.random` runs **none** of
§12.3 Steps 2–5. It does not compute `h` for attention, does not compute
`ς_j`, does not compute `ψ(h)` / `w_eff`, does not consult the scan-order
table, does not apply the satisficing test. It computes the admissible set
and draws. **Arm C pairs `decision.random` with `β = 0`.** `β = 0` already
removes *narrowing* (`ψ ≡ 1`, `w_eff = w_max`) while leaving the rest of
§12.3 intact — that is the "No narrowing" null (§28.4), isolating the
narrowing mechanism. `decision.random` is the *stronger* null (§28.4
"Results require the decision mechanism")."*

**Code, checked directly this session:** `RandomParams`
(`crates/firma-plugins/firma-plugin-decision/src/lib.rs`) has exactly two
fields, `action` and `shaping`, under `#[serde(deny_unknown_fields)]`; the
`DecisionRandom` block contains no `psi`, `w_eff`, `focus`, or `beta`
symbol (its one mention of `satisfices` is a comment saying it never calls
it). β acts only through `ψ(h) = (max(h,0)/h_crit)^β` in `decision.
satisficing`'s Step 3.

## Decision

### 1. The `decision.random` side of Arm C is one cell, not five — settled

β has no code path into `decision.random`. Five `decision.random` cells
labelled β ∈ {0, 0.5, 1, 2, 4} would be five configurations with
*identical* behaviour — the same admissible-set draw at every key (ADR-0053's
matched seeds make them bit-identical, not merely statistically alike) —
not five experimental conditions. ADR-0027 already states the pairing
("Arm C pairs `decision.random` with `β = 0`"), singular. A `beta` field on
`RandomParams` would be a config field with no mechanism to attach to; it is
rejected as incoherent, not merely unnecessary.

**Arm C's `decision.random` cells: 5 → 1.** This part does not depend on
any further reading of the manual; it follows from the code and an Accepted
ADR.

### 2. The `decision.satisficing` side — an explicit owner decision point, not settled here

The evidence does **not** settle how many `decision.satisficing` cells Arm C
has, and this ADR does not pretend otherwise. Two readings survive a close
read:

- **Reading S5 — keep the table's β sweep for satisficing (5 cells).** The
  table marks β for A, B, *and* C with five levels; the only minimal change
  forced by Decision 1 is on the random side. Arm C = 5 + 1 = **6 cells**.
  Weakness: satisficing at β ∈ {0.5, 1, 2, 4} is not a null condition —
  it *is* the narrowing mechanism Arms A/B sweep — so it is hard to square
  with "**Arm C — Null** … Both must show no relationship."
- **Reading S1 — Arm C is exactly §28.4's two nulls (1 cell).** The prose
  "β = 0 and `decision.random`. **Both** must show no relationship" names
  two things; §28.4 lists exactly those two as nulls ("No narrowing: β = 0",
  "Random decision: `decision.random`"); ADR-0027 calls β = 0 "the 'No
  narrowing' null" and `decision.random` "the stronger null." Arm C =
  satisficing@β=0 + random = **2 cells**. Weakness: it reads the table's
  "A, B, C" on the β row as "β is set for C" rather than "C sweeps all five
  levels," and it departs furthest from the manual's stated "10."

**Recommendation, with the reasoning shown:** Reading S1 has the stronger
*textual* support — it is the only reading under which every Arm C cell is
a null, which is what the prose, §28.4, and ADR-0027 all say Arm C is. But
it contradicts the factor table and the stated total more deeply than S5,
and the choice changes the registered design's size. This is a
research-design call for the owner, not a derivation. **This ADR's
settled decision is Decision 1 only; the satisficing side is left for the
owner to choose between S5 and S1.**

### 3. Corrected totals, computed directly (not estimated)

| Design | Arm A | Arm B | Arm C | Total cells | × 200 seeds |
|---|---|---|---|---|---|
| Manual §30.4 as stated | 125 | 180 | 10 | 315 | 63,000 |
| ADR-0051 narrowed E1, as implemented today | 125 | 60 | 10 | 195 | 39,000 |
| ADR-0051 narrowed E1 + Decision 1, Reading S5 | 125 | 60 | **6** | **191** | **38,200** |
| ADR-0051 narrowed E1 + Decision 1, Reading S1 | 125 | 60 | **2** | **187** | **37,400** |
| Manual §30.4 (unnarrowed) + Decision 1, S5 / S1 | 125 | 180 | 6 / 2 | 311 / 307 | 62,200 / 61,400 |

`firma_lab.spec`'s `ExperimentSpec.content_hash()` for the narrowed E1
spec **will change** under either reading once implemented (both the cell
set and, per Decision 4, the canonical structure of Arm C change). No hash
is stated here: there is no implemented design to hash yet, and a
predicted hash would be fabricated.

### 4. `firma_lab.spec`'s data model cannot represent this today — the structural change needed

`ArmDesign.cells()` is `itertools.product` over every `Factor` in the arm;
an arm's cell set is always a full cartesian product. "5 satisficing cells
× β, plus 1 random cell with no β" (S5), or even "satisficing@β=0 plus
random" (S1 — which *is* a product, 1×2, but only if β is reduced to a
single level *and* its `rule_param` application is conditional on the
plugin), is not a single cross-product of independent factors. A
representation is needed where a factor's presence depends on another
factor's level. Design options, not implemented:

- **(recommended) Blocks.** `ArmDesign` holds one or more *blocks*; each
  block is a cross-product of its own factors plus fixed per-block
  constants (e.g. block 1: `decision_plugin = satisficing` × β-levels;
  block 2: `decision_plugin = random`, no β). `cells()` concatenates blocks
  in declaration order. Deterministic, hashable, and a strict
  generalisation (a one-block arm is today's `ArmDesign`).
- **Conditional factors** (`Factor.applies_when={"decision_plugin":
  ...}`). Compact but makes `cells()` a filtered product whose size is no
  longer a simple product of level counts, and makes validation harder.
- **Explicit cell list.** Simplest, least structured; loses the "factor ×
  levels" legibility §28.1 asks a spec to have.

Whichever is chosen, `Factor`'s `rule_param` application for β must then
never be attempted on a cell whose rule set no longer contains
`decision.satisficing` — the current `KeyError` is the correct loud failure
for the current, wrong design, and must not be "fixed" by catching it.

## Alternatives

- **Add `beta` to `RandomParams`.** Rejected — no mechanism to attach to
  (Decision 1).
- **Keep 10 cells, treat the 4 extra `decision.random` cells as harmless
  replicates.** Rejected — they would be pre-registered as distinct
  conditions while being behaviourally identical, inflating the design by
  800 runs (at 200 seeds) of duplicated data and misrepresenting the arm's
  structure in the filed spec.
- **Silently pick S5 (the figure a previous investigation round reported).**
  Rejected — a closer read this round found S1 at least as well supported
  textually; reporting only the smaller change would have understated the
  question.

## Consequences

- **Positive.** Arm C's `decision.random` side is fixed on grounds that
  don't depend on interpretation. The satisficing-side question is surfaced
  with both options costed, rather than decided by default.
- **Negative, accepted.** The narrowed E1 design's size is now open between
  191 and 187 cells until the owner chooses. ADR-0044's Context cites "Arm C
  is 10 of 315 … cells; Arms A and B — 305 cells, 96.8%" — those figures
  become stale under either reading (A+B = 305 of 311 = 98.1%, or of 307 =
  99.3%), but ADR-0044's *decision* (Arm C is a small minority of E1, so the
  null arm's gate-clearance cannot stand in for Arms A/B) holds *more*
  strongly, not less. ADR-0044 is Accepted and is not edited; logged in
  `PROGRESS.md`.
- **Neutral.** No code changed by this ADR. The manual's §30.4 Arm C row and
  total need a PATCH once the owner chooses S5 or S1 — deliberately **not**
  applied in the same session's manual-PATCH batch, since this ADR is not
  Accepted.

## Compliance

- **Not implemented.** Implementation requires (a) the owner's S5/S1
  choice, (b) the `ArmDesign` block structure (Decision 4) or an accepted
  alternative, (c) updating `build_narrowed_e1_spec`'s Arm C and the
  `test_narrowed_e1_spec_cell_counts_match_adr_0051` /
  `..._total_jobs_at_200_replicates` tests, which currently assert 10 / 195
  / 39,000 and would correctly fail after the fix.
- Downstream Phase-3 tooling (`firma_lab.stats`, `.sensitivity`,
  `.prereg`) is written to operate over whatever `ExperimentSpec` it is
  given and hardcodes no cell count, so it does not block on this ADR.

## Note

The manual's own internal tension — a table whose literal cross-product
gives 10, and prose plus §28.4 that describe two nulls — survived ADR-0027,
ADR-0044, and ADR-0051 unnoticed because each of those cited the stated
total rather than re-deriving it against `decision.random`'s design. It was
found only because implementing job-config generation forced every cell to
become a runnable config. That is an argument for building executable
tooling against a design before trusting its arithmetic, not just reading
it.
