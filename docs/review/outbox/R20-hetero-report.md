# R20 — HETERO: seeded population heterogeneity — builder report

DIAGNOSTIC-ONLY - NOT REGISTERED - NOT FOR TUNING.
2026-10-02. No commit, no push. Smallest seed in any executed config: 910001.
Evidence: `docs/review/evidence/R20/` (`SHA256SUMS` also covers the two
scripts, the test file and the DRAFT ADR).

**DISCLOSURE.** θ_limit = 0.90 was chosen by the reviewer after seeing regime
grid 2 (R18: SC-2 was only reachable there). The distributions and spread
levels are reviewer-proposed and were fixed before any run.

## 0. Answers

| Question | Answer |
|---|---|
| Q1 Derivation deterministic and order-independent? | Yes, on every check. |
| Q2 Does heterogeneity remove the synchronised cliff? | **No.** Collapse rate 57% (low), 51% (medium), 29% (high), against 50% for the R18 reference. A tick with five or more deaths occurs in 50–57% of shocked runs at every level. |
| Q3 Property of the population or of one seed set? | The shocked-run metrics agree between sets A and B at every level. The baseline sanity shares differ between sets. |
| Q4 Reviewer-proposed criteria | **FAIL at all three levels** (T1 pass share 1%, 13%, 9%; collapse 57%, 51%, 29%). |
| Q5 Does the draw reach the Arm A outcome? | **No, not through decisions.** Every drawn firm that survives selects the same action sequence as the fixed firm. Sequences differ only where the firm dies. |
| Q6 Can `targets` express a subset? | A fixed subset yes, a predicate or random subset no; and the regulatory channel ignores `targets` altogether. |

## 1. Task as understood

Add a config-level generator that draws each firm's capital, capability,
aspiration and obligation from the init seed by a keyed, order-independent
derivation; then answer six named doubts with raw numbers, at three fixed
spread levels and two seed sets, against criteria fixed beforehand. New
files only; no engine change.

## 2. Step 0 — the spec

`docs/review/evidence/R20/hetero_spec.json`, written before any run.
**sha256 `b10d402e83de170c6a507f41dad2accc35b7eb775d610a680c5e06593d492bab`.**
`hetero_check.py` refuses to run unless the file equals the spec in the
generator byte for byte. It was not changed afterwards.

Distributions and levels are the prompt's. **Choices the prompt left open,
declared in the spec before any run:**
- **`a_spread` is an absolute half-width** (aspiration uniform on
  8.0 ± a_spread, so 7.5 to 8.5 at "high"). The relative reading,
  8.0 × (1 ± a_spread), was not run. This is the choice I am least sure of.
- Big-endian byte order for the 8 hash bytes.
- Capital rounded half up to an integer; capability not rounded.
- Obligation q = 50 + floor(31·u), integers 50 to 80.
- θ_Q = 40 in every drawn population, seeded firm or not.
- Collapse = at most 2 survivors of 12.
- The Q4 verdict is computed over sets A and B pooled; per-set numbers shown.
- Q5 runs have no shock; the drawn single firm keeps its obligation draw;
  the negative control is base-config firm 0.
- Q3: a zero against a non-zero counts as a difference.

Per-firm fields not drawn (input stock 40, legitimacy 1.0) are copied from
base-config firm 0, as in R16/R18.

## 3. Files changed

| Path | Lines |
|---|---|
| `scripts/population_generator.py` | new |
| `scripts/hetero_check.py` | new |
| `python/tests/test_population_generator.py` | new, 30 tests |
| `docs/adr/0058-seeded-population-heterogeneity.md` | new, Status DRAFT, 65 lines |
| `docs/review/evidence/R20/*`, this report | new |

Nothing else was edited. Existing scripts are imported, not copied. The ADR
index (`docs/adr/README.md`, manual §34.0) and `PROGRESS.md` were not
updated: not authorised here.

## 4. Tests added

`generator_tests.txt`: 30 passed. New files, so no failing-first run.
- Derivation: six known-answer vectors from GNU `sha256sum`; two values
  checked against `bc` arithmetic; equality with a second, differently
  composed implementation on 240 keys; separator prevents key collisions;
  inverse-CDF identity; the declared formulas; ranges at every level;
  population moments near the declared ones.
- Independence: shuffled and reversed generation order give identical
  firms; populations of 1, 13 and 40 firms agree with the 12-firm one on
  the shared ids.
- Spec: step-0 file equals the declared spec; step 0 never overwrites; a
  changed spec is refused.
- Configs: fixed items; fork differs only by the shock rule; Q5 config has
  one firm, four pins, β = 1, no shock; a low seed is refused.
- Metrics: the cliff pattern is detected; spread-out deaths are not; the
  collapse threshold; the Q3 rule; the Q4 verdict needs all three criteria.
- Two end-to-end runs (a drawn pair run twice, byte-equal; one pinned firm).

## 5. Full suite summary

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 269 passed, 0 failed, 1 ignored |
| `pytest python/tests` | 223 passed, 0 skipped (193 + 30 new) |

No Rust change.

## 6. Results

Raw: `hetero_stdout.txt` (stderr empty, exit 0), `q2_runs.csv` (300 rows),
`q5_runs.csv` (400 rows), `summary.json`, `control_runs.json`,
`results_description.txt`, `provenance.txt`. 1,082 runs, 0 failed, 69.3 s.

### Q1 — derivation

| Check | Result |
|---|---|
| Known-answer vectors from `sha256sum` (not Python), 6 keys | all equal (`known_answer_vectors.txt`) |
| Second implementation, 240 keys | bit-equal |
| Same seed, config built twice | byte-equal (sha256 `32731739…`) |
| Same seed, event log from two runs | byte-equal (`b7aed4f0…` twice) |
| Generation order permuted (shuffled, reversed) | every firm unchanged |
| Firm added (12 → 13, 40) or removed (12 → 1) | other firms unchanged |

### Controls

- **Byte-equal re-run:** as above.
- **The code can see the cliff.** R18 cell 79 at shock mean 0.20 through the
  Q2/Q4 code reproduces its recorded numbers exactly (SC values, per-run
  survivors, non-zero share, mean survival) and shows collapse rate 0.50 and
  a five-or-more-death tick in 0.50 of runs (nine deaths in one tick in all
  ten collapsed runs).
- **The 4-firm smoke config** also reproduces its R16 numbers exactly
  (SC-1 0.75, SC-2 0.591, non-zero share 0.40, mean survival 0.45). With 4
  firms the 20% collapse threshold (0.8 firms) and the five-death tick
  cannot trigger, so these two metrics read 0 for it by construction.

### Q2 — the cliff (pooled over 100 init seeds per level)

Reference copied from `evidence/R18`: cell 79 collapse rate 10% / 50% / 45%
at shock mean 0.10 / 0.20 / 0.30; nine same-tick compliance deaths in its
collapsed runs.

| | Low | Medium | High | R18 cell 79 (mean 0.20) |
|---|---|---|---|---|
| Collapse rate (≤ 2 survivors) | 0.57 | 0.51 | 0.29 | 0.50 |
| Runs with ≥ 5 deaths in one tick | 0.57 | 0.57 | 0.50 | 0.50 |
| Runs with any compliance death | 57 | 57 | 57 | 10 of 20 |
| Distinct compliance-death ticks, mean over those runs | 1.44 | 1.61 | 1.54 | 1.00 |
| Mean survival, shocked | 0.376 | 0.426 | 0.500 | 0.458 |
| Mean survival, baseline | 0.847 | 0.847 | 0.848 | 0.833 |
| Non-zero matched ΔH share | 1.00 | 1.00 | 1.00 | 1.00 |

Survivors at end, shocked runs (count of runs by survivors of 12):

| Level | Distribution |
|---|---|
| Low | 0: 50, 1: 7, 8: 4, 9: 6, 10: 14, 11: 10, 12: 9 |
| Medium | 0: 17, 1: 19, 2: 15, 3: 6, 8: 4, 9: 6, 10: 14, 11: 10, 12: 9 |
| High | 0: 3, 1: 12, 2: 14, 3: 8, 4: 14, 5: 3, 6: 1, 7: 1, 8: 5, 9: 6, 10: 14, 11: 10, 12: 9 |

Distinct compliance-death ticks per shocked run (count of runs):

| Level | 0 | 1 | 2 | 3 | 4 |
|---|---|---|---|---|---|
| Low | 43 | 36 | 17 | 4 | 0 |
| Medium | 43 | 28 | 24 | 4 | 1 |
| High | 43 | 30 | 23 | 4 | 0 |

Described from the per-run table (`results_description.txt`), no cause tested:
- **The same 57 init seeds have a compliance death at every level, and the
  same 43 do not.** The 43 give an identical survivors distribution at all
  three levels.
- In the 57, the largest same-tick death count is 6 to 12 at low, 5 to 12 at
  medium, 3 to 11 at high. More spread leaves more survivors (0–1 at low,
  0–3 at medium, 0–8 at high) but the deaths still fall in one to three
  ticks.
- Those 57 runs have drawn magnitudes between 0.151 and 0.245; the 43 span
  0.006 to 0.335.

### Q3 — set A against set B

Shocked-run metrics, all "same" under the declared rule at every level:

| Metric | Low A / B | Medium A / B | High A / B |
|---|---|---|---|
| Collapse rate | 0.58 / 0.56 | 0.50 / 0.52 | 0.30 / 0.28 |
| ≥ 5 deaths in one tick | 0.58 / 0.56 | 0.58 / 0.56 | 0.52 / 0.48 |
| Mean survival, shocked | 0.370 / 0.382 | 0.423 / 0.428 | 0.483 / 0.517 |
| Mean survival, baseline | 0.843 / 0.850 | 0.843 / 0.850 | 0.845 / 0.850 |
| Non-zero matched ΔH | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |

Metrics that differ by more than 0.10 absolute or 2× relative:

| Level | Metric | A | B |
|---|---|---|---|
| all three | SC-1 pass share | 0.48 | 0.62 |
| Low | SC-2 pass share | 0.50 | 0.66 |
| Low | SC-3 pass share | 0.02 | 0.00 |
| Low | T1 pass share | 0.02 | 0.00 |
| Medium | SC-2 pass share | 0.40 | 0.70 |
| Medium | SC-3 pass share | 0.58 | 0.46 |
| Medium | baseline ≥ 5-death tick share | 0.02 | 0.00 |
| High | T1 pass share | 0.02 | 0.16 |

Three of these (low SC-3, low T1, medium baseline share) are one run against
zero and are flagged only by the zero rule.

### Q4 — cross-seed stressed-but-alive rate

Share of init seeds whose baseline passes each condition:

| Level | Set | SC-1 | SC-2 | SC-3 | SC-6 | T1 | Collapse | Non-zero ΔH | Verdict |
|---|---|---|---|---|---|---|---|---|---|
| Low | A | 0.48 | 0.50 | 0.02 | 1.00 | 0.02 | 0.58 | 1.00 | fail |
| Low | B | 0.62 | 0.66 | 0.00 | 1.00 | 0.00 | 0.56 | 1.00 | fail |
| Low | pooled | 0.55 | 0.58 | 0.01 | 1.00 | 0.01 | 0.57 | 1.00 | **FAIL** |
| Medium | A | 0.48 | 0.40 | 0.58 | 1.00 | 0.10 | 0.50 | 1.00 | fail |
| Medium | B | 0.62 | 0.70 | 0.46 | 1.00 | 0.16 | 0.52 | 1.00 | fail |
| Medium | pooled | 0.55 | 0.55 | 0.52 | 1.00 | 0.13 | 0.51 | 1.00 | **FAIL** |
| High | A | 0.48 | 0.26 | 0.88 | 1.00 | 0.02 | 0.30 | 1.00 | fail |
| High | B | 0.62 | 0.32 | 0.78 | 1.00 | 0.16 | 0.28 | 1.00 | fail |
| High | pooled | 0.55 | 0.29 | 0.83 | 1.00 | 0.09 | 0.29 | 1.00 | **FAIL** |

Criteria (fixed): T1 share ≥ 0.50 **and** collapse ≤ 0.10 **and** non-zero
ΔH share ≥ 0.50. The third is met everywhere; the first two fail everywhere,
per set as well as pooled.

Init seeds whose baseline passes T1 (full per-seed table: `q2_runs.csv`):
- Low: 910041.
- Medium: 910011, 910035, 910037, 910039, 910050; 911001, 911006, 911009,
  911021, 911027, 911033, 911037, 911050.
- High: 910035; 911003, 911019, 911021, 911027, 911029, 911034, 911036,
  911042.

How the conditions fail, described only:
- **SC-1** is identical across levels: 43 of 100 baselines have survival
  above 0.90 and 2 below 0.60. In 299 of 300 baselines the firms that die
  are exactly the obligation-seeded firms, all of solvency. The obligation
  draw does not depend on the spread level.
- **SC-2** fails low at the low level (40 baselines under 0.05) and high at
  the high level (69 over 0.25).
- **SC-3**: the scope constraint binds in 1, 58 and 95 of 100 baselines at
  low, medium and high. Solvency and obligation each bind in 87; compliance
  in all 100.

### Q5 — Arm A outcome, one pinned firm, β = 1, no shock, 50 init seeds

The pin mechanism can be driven from a script without editing `firma_lab`:
four `set_agent_real` interventions at tick 0, written with the existing
`firma_lab.runner._add_pin_intervention`.

| Arm | h | Shortfall | Distinct sequences | H_rep mean | H_rep sd | Runs where the firm died |
|---|---|---|---|---|---|---|
| Drawn | 0.02 | 0.0 | 2 | 0.5765 | 0.2151 | 6 |
| Drawn | 0.02 | 1.0 | 2 | 0.5765 | 0.2151 | 6 |
| Drawn | 0.40 | 0.0 | 6 | 0.0000 | 0.0000 | 6 |
| Drawn | 0.40 | 1.0 | 2 | 0.0262 | 0.1297 | 48 |
| Fixed (negative control) | 0.02 | 0.0 | 1 | 0.6552 | 0 | 0 |
| Fixed | 0.02 | 1.0 | 1 | 0.6552 | 0 | 0 |
| Fixed | 0.40 | 0.0 | 1 | 0.0000 | 0 | 0 |
| Fixed | 0.40 | 1.0 | 1 | 0.0000 | 0 | 50 |

The negative control gives 1 sequence in every cell, as expected.

**Heterogeneity does not reach the pinned firm's decisions.** The drawn
firms span capital 22 to 233 (46 distinct values) and capability 0.29 to
0.75, yet:
- In every cell, all drawn firms that survive select one and the same
  sequence, and it is the fixed firm's sequence (same hash).
- The extra sequences are truncations by death. In three cells the six
  obligation-seeded firms die of solvency (tick 40 at h 0.02; ticks 3 to 9
  at h 0.40 / shortfall 0, which gives the 6 sequences).
- At h 0.40 / shortfall 1.0 the firm dies of compliance at tick 4 in 48
  drawn runs and all 50 fixed runs. The two drawn survivors are the two
  drawn firms with capability below θ_cap = 0.40.
- The H_rep spread in the drawn arm is therefore two values per cell
  (survivor's value, and 0 for a firm that died), not a continuum.

### Q6 — `targets` (read-only; `Q6_shock_targets.txt`)

- Type: `ShockTargets { All, Set { ids } }`, `firma-domain/src/shock.rs:133-144`.
  The manual's `Predicate` (§13.2, `MANUAL.md:879`) is deferred and does not
  exist in code.
- Accepted JSON: `{"kind": "all"}`; `{"kind": "set", "ids": [1, 3]}`.
  Predicate: not expressible. Random subset: not expressible in the plugin
  (`shock.stochastic` copies a fixed `targets`,
  `firma-plugin-shock/src/lib.rs:400-401, :459`); a config generator could
  write a per-seed `set`.
- **`targets` is read on the reputational channel only**
  (`firma-plugin-shock/src/lib.rs:166`). The regulatory channel writes one
  global `theta_limit` / `theta_cap` delta (`:151-160`) and the resource and
  competitive channels one global price delta (`:181-193`); none consults
  `targets`. So for the regulatory shock used in R12–R20, a subset is
  accepted by the schema and has no effect. This is from reading the code;
  I did not run it.

## 7. What I did NOT verify

- Causes. I did not test why the same 57 seeds collapse at every level, why
  the pinned firm's actions ignore capital and capability, or why
  obligation-seeded firms die of solvency.
- The relative reading of `a_spread`, any other spread level, shock mean,
  θ_limit, or more than one shocked run per init seed.
- Q6 by execution.
- Whether SHA-256-derived draws are adequate beyond the moment checks in the
  tests.
- Mechanism and environment seeds other than 910001 / 910002.

## 8. Outside scope, noticed

- **Init seed and shock seed are tied in Q2–Q4** (shock seed = init seed, as
  the prompt specifies). Since the same 57 seeds collapse at all three
  spread levels, collapse versus no collapse tracks the seed-specific shock
  draw, and this design cannot separate that from the population draw.
- **SC-1 is set by the obligation draw alone** in these populations: the
  number of seeded firms (0 to 5 per population) is the number of baseline
  deaths in 299 of 300 baselines.
- The DRAFT ADR is 65 lines, slightly over a printed page. Its last
  sentence says R20 does not support the decision as proposed.
- ADR index and `PROGRESS.md` not updated (not authorised).

## 9. Questions for the owner

1. Was `a_spread` meant as absolute (8.0 ± a) or relative (8.0 × (1 ± a))?
   I ran absolute. Evidence that would settle whether it matters: Q5 shows
   the pinned firm's actions did not respond to the aspiration draw at ±0.25;
   the relative reading at ±50% was not run.
2. Not blocking: with shock seed tied to init seed, should a follow-up cross
   them (same populations, different shock seeds) to separate the two?

## 10. Notes on what I changed my mind on

`NOTES_changed_my_mind.md`: the `a_spread` reading (leaned relative, took
the literal absolute); declaring open choices in the spec instead of
stopping; adding a closing sentence to the ADR so the DRAFT cannot be read
as supported; a descriptive script on the saved tables after the result.
Nothing crashed; the check ran once.

## 11. Three things I am least sure about

1. **The `a_spread` reading** (§9.1). It makes the aspiration draw narrow at
   every level.
2. **Whether Q5 is a fair test of "variance reaches Arm A".** The pins fix
   the margin and the shortfalls, which are the inputs the decision reads;
   capital and capability may have no remaining path to the choice by
   construction. I report that actions did not move; I did not establish
   that they could have.
3. **Collapse at ≤ 2 survivors.** 20% of 12 is 2.4. At the high level 8 runs
   end with 3 survivors and 14 with 4, so the 29% collapse rate is sensitive
   to where that line sits; the ≥ 5-deaths-in-one-tick share (50%) is not.
