# R21 — DIAG: what drives the synchronised collapse — builder report

DIAGNOSTIC-ONLY - NOT REGISTERED - NOT FOR TUNING.
2026-10-02. No commit, no push. Smallest seed in any executed config: 910001.
No R20 criterion or spread level was changed. Evidence:
`docs/review/evidence/R21/` (`SHA256SUMS` also covers the script and tests).

## 0. Answers

| Part | Answer |
|---|---|
| A Attribution | Predeclared reading: **mixed** (range of marginal collapse rates 0.80 across shock seeds, 0.43 across population seeds; ratio 1.85, below 3). Variance shares: shock seed 0.69, population seed 0.13, interaction 0.17. |
| B Relative aspiration | **No change at all.** Every R20 number is reproduced exactly; all three levels still FAIL. |
| C Behavioural diversity | 2 to 3 distinct action sequences among 12 firms on average (range 1 to 5), at every level and under both aspiration readings. |
| D Shock channel | **No cell is graded.** Resource and competitive shocks kill no additional firm in any of 250 runs; the resource shock changes no action at all. |

One structure in part A is sharper than the predeclared metric shows, and I
report it separately in §6: whether any firm dies of compliance is decided
by the shock seed alone, and it coincides exactly with the drawn magnitude
lying between about 0.15 and 0.25.

## 1. Task as understood

Four diagnostics with levels fixed beforehand: (A) cross 30 population seeds
with 30 shock seeds and attribute collapse; (B) repeat R20 Q2/Q4 with the
aspiration spread read as relative; (C) count distinct behaviour among firms
before any shock; (D) compare three shock channels. Controls; new files only.

## 2. Step 0 — the spec

`docs/review/evidence/R21/diag_spec.json`, written before any run.
**sha256 `220a5c210756beceec3d8d42dcc11072e67d3368e691e89b2698ff93285c4e03`.**
The run refuses to start unless the file equals the script's spec. Unchanged
afterwards.

**Choices the prompt left open, declared in the spec before any run:**
- A uses the R20 (absolute) aspiration reading ("medium spread (R20 spec)").
- C runs both aspiration readings, since the prompt names neither.
- Variance method: two-way sums of squares on the 30 × 30 matrix of 0/1
  collapse indicators, one run per cell; shares are SS / SS_total;
  interaction includes residual (not separable without replication).
- The reading is "mixed" if both ranges are 0.
- "The 20 collapse-prone shock seeds": I could not tell which 20; all 30 are
  listed in seed order, no selection.
- Resource and competitive magnitudes are fixed (`magnitude_sd = 0`).
- The two "graded" extremes are counted together against the 20% bound.
- In C, a firm that dies before tick 39 contributes its shorter sequence.

## 3. Files changed

| Path | Lines |
|---|---|
| `scripts/diag_cliff.py` | new |
| `python/tests/test_diag_cliff.py` | new, 16 tests |
| `docs/review/evidence/R21/*`, this report | new |

Nothing else was edited; existing scripts are imported.

## 4. Tests added

`diag_tests.txt`: 16 passed. New files, so no failing-first run.
- Spec: step-0 file equals the declared spec; no overwrite; changed or
  missing spec refused; declared seeds and cells.
- Relative aspiration: range 8 × (1 ± a); uses the same keyed uniform; only
  the aspiration differs from the R20 population.
- Configs: the default config equals R20's; crossing changes only the shock
  seed; a channel cell changes only the shock rule; low seed refused.
- Part A arithmetic: all-shock, all-population, checkerboard (all
  interaction) and a hand-computed example; the factor-of-three rule.
- Parts C and D arithmetic: identical firms counted once, window respected,
  non-decision records ignored; the graded rule at its boundaries.
- One real resource-channel pair.

## 5. Full suite summary

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 269 passed, 0 failed, 1 ignored |
| `pytest python/tests` | 239 passed, 0 skipped (223 + 16 new) |

No Rust change.

## 6. Results

Raw: `diag_stdout.txt` (stderr empty, exit 0), `a_matrix.csv`, `a_runs.csv`
(900 rows), `b_runs.csv` (300), `c_runs.csv` (180), `d_runs.csv` (300),
`summary.json`, `provenance.txt`. 2,357 runs, 0 failed, 228 s.

### Controls

- A cell (population 910001, shock seed 910101) run twice: byte-equal.
- The no-shock run of a population is byte-equal at three different shock
  seeds, so one baseline per population is valid.
- R18 cell 79 through the new code reproduces its R18 record exactly
  (collapse 0.50, survivors 10 × 1 and 10 × 10).
- R20 medium init seed 910001 reproduces its `q2_runs.csv` row, including
  both event-log hashes.

### A — attribution (medium spread, 30 × 30, 900 shocked runs)

Overall collapse rate 0.347 (312 of 900).

| | Shock seeds | Population seeds |
|---|---|---|
| Marginal collapse rates | 13 seeds at 0.800, 17 seeds at 0.000 | 24 seeds at 0.433, 6 seeds at 0.000 |
| Range | 0.800 | 0.433 |
| Share of collapse variance | 0.694 | 0.133 |

Interaction (including residual): 0.173. **Predeclared reading: mixed**
(0.800 / 0.433 = 1.85).

The 30 × 30 matrix (`a_matrix.csv`) has only two kinds of row: 24
populations share the identical row `010011011110111100000001010000` and 6
populations (910005, 910012, 910018, 910023, 910027, 910029) are all zero.

Described from the per-run table (`a_description.txt`), not the predeclared
metric:
- **Under 13 shock seeds every one of the 30 populations has compliance
  deaths; under the other 17 none does**, and in those 510 runs survivors
  equal the population's baseline survivors in all 510.
- **Given one of the 13, the number of survivors depends on the population
  only.** Each population ends with the same count under all 13 shock seeds
  (0, 1, 2, 3 or 4). The six "never collapse" populations are the ones left
  with 3 or 4 survivors, just above the ≤ 2 line.
- In those 390 runs the largest same-tick death count is 5 to 12.
- So the 0.13 population share and the 0.17 interaction share both come from
  where the ≤ 2 threshold falls among populations that all lose most firms.

Onset and magnitude for all 30 shock seeds are in `diag_stdout.txt`. The
drawn onset and magnitude are the same for every population.

| | 13 shock seeds with compliance deaths | 17 without |
|---|---|---|
| Drawn magnitude | 0.151 to 0.232 | 0.105 to 0.144 (7 seeds) and 0.255 to 0.367 (10 seeds) |
| Drawn onset | 13 to 75 | 21 to 79 |

- **No shock seed without compliance deaths has a magnitude inside the
  0.151–0.232 band.**
- The same holds in R20's saved table (`magnitude_band_r20.txt`, medium
  level, 100 seeds): all 57 runs with a compliance death have magnitude
  0.151 to 0.245; none of the 43 without lies in that interval (nearest:
  0.149 below, 0.252 above).
- The reviewer's note that the ranges "overlap completely" is true of the
  ranges; no run without compliance deaths falls inside the band in either
  data set (130 shock draws in total). Onsets do overlap.
- With θ_limit 0.90, the band corresponds to a post-shock θ_limit of roughly
  0.65 to 0.75. I did not test this as a cause. R18's record shows baseline
  θ_limit 0.70 behaving unlike 0.60 and 0.80 (SC-1 never passed there).

### B — aspiration read as relative

Pooled over 100 init seeds per level; "abs." is copied from R20.

| Metric | Low rel. / abs. | Medium rel. / abs. | High rel. / abs. |
|---|---|---|---|
| Aspiration range | 7.2–8.8 / 7.9–8.1 | 6.0–10.0 / 7.75–8.25 | 4.0–12.0 / 7.5–8.5 |
| Collapse rate | 0.57 / 0.57 | 0.51 / 0.51 | 0.29 / 0.29 |
| ≥ 5 deaths in one tick | 0.57 / 0.57 | 0.57 / 0.57 | 0.50 / 0.50 |
| Mean survival, shocked | 0.376 / 0.376 | 0.426 / 0.426 | 0.500 / 0.500 |
| Non-zero matched ΔH | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| SC-1 / SC-2 / SC-3 pass share | 0.55 / 0.58 / 0.01 (same) | 0.55 / 0.55 / 0.52 (same) | 0.55 / 0.29 / 0.83 (same) |
| T1 pass share | 0.01 / 0.01 | 0.13 / 0.13 | 0.09 / 0.09 |
| Verdict (R20 criteria) | FAIL / FAIL | FAIL / FAIL | FAIL / FAIL |

Per-set numbers and survivor distributions are also identical to R20's
(`diag_stdout.txt`). The same 57 init seeds have a compliance death.

I checked that this is a finding and not a mistake (`treatments_applied.txt`):
- The configured aspirations differ (e.g. 6.09 to 9.06 against 7.76 to 8.13
  for init seed 910001), and all 600 event logs differ from R20's.
- Yet in all 300 pairs the survivors, death records, SC-2 value, matched ΔH
  and T1 are identical to R20's, and in the pair I re-ran the selected-action
  sequences and focus values are identical between readings.

So in this configuration the starting value of `aspiration_capital_growth`,
across 4 to 12, changes no decision and no outcome that was measured.

### C — distinct behaviour among the 12 firms, baseline, ticks 0..39

| Level | Distinct action sequences: mean [min, max] | Distribution over 30 seeds |
|---|---|---|
| Low | 2.53 [1, 4] | 1: 1, 2: 14, 3: 13, 4: 2 |
| Medium | 2.93 [2, 5] | 2: 9, 3: 15, 4: 5, 5: 1 |
| High | 2.67 [2, 5] | 2: 15, 3: 12, 4: 1, 5: 2 |

The (focus, action) counts are identical to the action counts in every row,
and both aspiration readings give identical numbers. Twelve firms with
distinct drawn capital and capability produce two or three behaviours.

### D — shock channel (medium spread, relative reading, 50 init seeds)

Semantics read before running (`D_channel_semantics.txt`): resource adds `m`
to `input_price`, competitive subtracts `m` from `output_price`, both as one
global integer delta (`firma-plugin-shock/src/lib.rs:6-12, :181-193`;
`firma-domain/src/shock.rs:295-307`); `sd = 0` fixes the magnitude
(`lib.rs:384-387`); prices are unclamped (ADR-0036). All five price levels
are valid; none was dropped. The baseline is byte-identical across the six
cells for every seed; T1 pass share is 0.10 in every cell.

| Cell | Collapse | ≥ 5 deaths in a tick | Non-zero ΔH | Mean survival | Runs at an extreme | Distinct counts | Graded? |
|---|---|---|---|---|---|---|---|
| Regulatory 0.20 | 0.50 | 0.58 | 1.00 | 0.423 | 0.92 | 9 | no |
| Resource +1 | 0.00 | 0.02 | 0.00 | 0.843 | 1.00 | 6 | no |
| Resource +2 | 0.00 | 0.02 | 0.00 | 0.843 | 1.00 | 6 | no |
| Resource +4 | 0.00 | 0.02 | 0.00 | 0.843 | 1.00 | 6 | no |
| Competitive −1 | 0.00 | 0.02 | 0.48 | 0.843 | 1.00 | 6 | no |
| Competitive −2 | 0.00 | 0.02 | 0.58 | 0.843 | 1.00 | 6 | no |

Survivors (count of runs by survivors of 12):
- Regulatory: 0: 7, 1: 10, 2: 8, 3: 4, 8: 3, 9: 3, 10: 4, 11: 7, 12: 4.
- All five price cells: 7: 1, 8: 7, 9: 8, 10: 9, 11: 19, 12: 6, which is
  exactly the baseline distribution. Every run ends with its baseline
  survivors. The same firms die, of the same cause (94 solvency deaths per
  cell), as in the baseline. Death records are identical in 240 of the 250
  runs; in 10 competitive runs one firm dies two ticks earlier (e.g. tick 40
  instead of 42).
- Baseline mean survival 0.843. The 0.02 in the price cells is one baseline
  with five solvency deaths in one tick.

Checked that the price shocks were applied (`price_path.txt`, init seed
910001, onset 52):
- **Resource +4:** the engine logs `input_price +4` at onset, but
  `resource.patchy`, part of the base rule set, also writes `input_price`
  every tick and reverts it: 7, 6, 5, 5, 4, 3 against 3, 3, 3, 4, 3, 2
  without the shock, equal again within eight ticks. No firm's action
  changes.
- **Competitive −2:** `output_price` goes 3 → 1 and stays. One firm of 12
  changes actions; deaths are unchanged in that run.

## 7. What I did NOT verify

- Causes: why magnitudes between about 0.15 and 0.25 produce compliance
  deaths and larger ones do not; why aspiration has no effect; why a
  permanent halving or worse of the output price kills no one.
- The magnitude band outside the sampled draws, or with θ_limit other than
  0.90. It is an observed coincidence in 130 draws, not a tested rule.
- The price-path check on more than one seed.
- Part A at the low or high level, or with the relative reading.
- Mechanism and environment seeds other than 910001 / 910002.

## 8. Outside scope, noticed

- **The "permanent" resource shock is not permanent in this rule set**
  because `resource.patchy` owns the same `input_price` and mean-reverts it.
  Any E1 design using the resource channel alongside `resource.patchy`
  inherits this.
- **The ≤ 2 collapse threshold shapes part A's verdict.** Populations left
  with 3 or 4 survivors count as "never collapse" (I flagged the threshold's
  sensitivity in R20).
- **First run crashed in a control** before any A–D result (a worker
  function could not be sent to a worker process). Fixed with a wrapper and
  re-run once; spec unchanged; crashed output kept.
- **One test was wrong, not the code**: `3 × 0.1 > 0.3` in floating point.
  Rewritten with binary-exact values after step 0 and before any run.

## 9. Questions for the owner

None blocking. For the next design step, the evidence here points at one
thing to decide: whether to test the magnitude band directly (fixed
magnitudes on a grid through 0.10 to 0.35, crossed with a few populations).
The evidence that would settle it is in `a_runs.csv` and
`magnitude_band_r20.txt`.

## 10. Notes on what I changed my mind on

`NOTES_changed_my_mind.md`: the crash and re-run; the wrong test; not
believing B and D until I verified the treatments reached the engine;
reporting "mixed" as predeclared and the sharper structure separately; all
30 shock seeds instead of an unidentified 20; both readings in C.

## 11. Three things I am least sure about

1. **How to read part A.** The predeclared rule gives "mixed". The per-run
   table says the shock seed alone decides whether a cascade happens and the
   population alone decides how many survive it. Both are true; they answer
   slightly different questions, and the first depends on the ≤ 2 line.
2. **The magnitude band.** It has no exception in 130 draws, but I did not
   manipulate magnitude; onset and magnitude are drawn together from one
   seed, and a third factor tied to the seed cannot be excluded from this
   design.
3. **Whether D's resource cells test what was intended.** The shock was
   applied as specified, but mean reversion removes it within about eight
   ticks, so "permanent, +4 on input price" was never the regime the firms
   faced. The competitive cells do not have this problem.
