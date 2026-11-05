# R18 — REGIME-2: regime search, grid 2 (two stages) — builder report

DIAGNOSTIC-ONLY - REGIME SEARCH - NOT REGISTERED - NOT FOR TUNING.
2026-10-02. No commit, no push. Smallest seed in any executed config: 910001.
Evidence: `docs/review/evidence/R18/` (`SHA256SUMS` also covers the two
scripts and the two test files).

**DISCLOSURE.** Grid 1 (72 cells, R16) passed 0 cells. The axes of grid 2 were
chosen by the reviewer AFTER seeing grid 1's results (R17). Grid 1 stays in
the record unchanged (`evidence/R16/`, all data files still verify).

## 0. Answer

**6 of 288 cells are stressed-but-alive under the declared targets**: cells
79, 87, 103, 111, 127 and 135.

| Count | Cells |
|---|---|
| Pass T1 (Stage 1) | 6 of 288 |
| Pass T2 given T1 (Stage 2) | 6 of 6 (all 18 magnitude rows pass) |
| Pass both | 6 |

Three facts about those six cells belong next to the count:

1. **They are one regime, not six.** All six have one low-capability firm,
   seeded obligation, θ_limit 0.90, and output price 2 or 3. They differ only
   in the thin-capital count (0, 2, 4) and the price (2, 3), and their
   Stage-2 results are identical run for run (same survivors and same
   matched ΔH in every one of the 20 runs, at every magnitude).
2. **Every shocked run ends with either 10 or 1 survivors of 12; none in
   between.** The mean survival lands in the T2 band by mixing the two:

   | Shock mean | Runs with 10 survivors | Runs with 1 survivor | Mean survival |
   |---|---|---|---|
   | 0.10 | 18 of 20 | 2 of 20 | 0.758 |
   | 0.20 | 10 of 20 | 10 of 20 | 0.458 |
   | 0.30 | 11 of 20 | 9 of 20 | 0.496 |

   In the 1-survivor runs of cell 79 that I listed, nine firms die of
   compliance in the same tick and the survivor is the low-capability firm.
3. **Baseline survival of 0.833 is the two obligation-seeded firms dying.**
   In each of the six baselines exactly those two firms die, of solvency, at
   tick 40. No other firm dies without a shock.

Whether a population that is either intact or nearly wiped out per run is
what "stressed-but-alive" was meant to capture is the owner's call; the
declared T2 only looks at the mean.

## 1. Task as understood

Second pre-declared grid, two stages. Stage 1: one no-shock baseline per
cell over 288 cells, T1 only. Stage 2: for every T1-passing cell, three
shock magnitudes × 20 shocked runs with forks, T2. Same targets as R16. Fix
the slice script's no-deaths crash with a failing test first. No hypothesis
statistic.

## 2. Step 0 — the grid

`docs/review/evidence/R18/grid2.json`, written before any run.
**sha256 `617239841e91e5c53cd9dc3a3ef46ad0b4b884ad281779c11b0082bae1dcf792`.**
The run refuses to start unless the file is byte-identical to the script's
grid. It was not changed afterwards.

Axes and levels are exactly the prompt's: low-capability count 0, 1, 2, 3;
thin-capital count 0, 2, 4; output price 2, 3, 5; obligation none / seeded
(θ_Q 40, two ordinary firms at q = 60); θ_limit 0.60, 0.70, 0.80, 0.90.
288 cells, product order, last axis fastest. Type ranges as given (L:
capability 0.25–0.35, capital 200–300; T: 0.50–0.70, 8–20; O: 0.50–0.80,
150–300).

**Choices the prompt left open, which I declared in `grid2.json` before any
run:**
- Types are assigned to ids in the order L, then T, then O.
- A type with exactly one firm gets the midpoint of its range (so the single
  low-capability firm has capability 0.30 and capital 250).
- The two seeded firms are the two lowest-id ordinary firms.
- θ_Q stays at the base config's 100 in the "none" level.
- Control (b) uses grid-1 cells 0, 36 and 71.
- A cell passes both if any one of its three magnitude rows passes T2.

Fixed items are R16's, stated there as assumptions: satisficing, smoke rule
set, `observation.full`, stochastic regulatory shock, horizon 120, seeds
910001 / 910002 / 910004, θ_cap 0.40, input price 2, n = 12.

## 3. Files changed

| Path | Change |
|---|---|
| `scripts/exploratory_e1_slice.py` | 2 lines: guard in `decisions_and_deaths` for a run with no deaths |
| `python/tests/test_exploratory_slice.py` | +1 test (26 in the file) |
| `scripts/regime_search.py` | grid-2 mode added; grid-1 code unchanged except `measure()` losing its workaround |
| `python/tests/test_regime_search.py` | +12 tests (33 in the file) |
| `docs/review/evidence/R18/*`, this report | new |

Nothing else was edited. The R14 and R16 `SHA256SUMS` now mismatch on these
four source files only, as expected; every data file in both still verifies.

## 4. Tests added, with before/after for the fix

**Slice fix (failing test first).**
`test_a_run_in_which_no_firm_dies_has_no_deaths_and_does_not_raise`:
- before the fix (`slice_fix_BEFORE.txt`): `KeyError: 'cause'`, 1 failed;
- after (`slice_fix_AFTER.txt`): 1 passed. The other 25 slice tests are
  unchanged and pass.

**Regime search (12 new).** Grid-2 cell count and order; the step-0 file
equals the script's grid; grid-2 targets equal grid-1's; refusal when the
file differs and no overwrite at step 0; within-type spacing and the
midpoint rule; type assignment by id and the two seeded firms; the all-O
population; cell config (price, θ, θ_Q, shock parameters); fork differs only
by the shock rule and a reserved seed is refused; grid-1 cell 0 reproduces
its R16 numbers; one grid-2 baseline end to end; the SC-5 `n/a` printer
(the crash in §8).

## 5. Full suite summary

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 269 passed, 0 failed, 1 ignored |
| `pytest python/tests` | 193 passed, 0 skipped (180 + 13 new) |

No Rust change.

## 6. Results

Raw: `regime2_stdout.txt` (stderr empty, exit 0), `stage1.csv` /
`stage1_full.json` (288 rows), `stage2.csv` / `stage2_full.json` (18 rows,
360 per-run records), `controls.json`, `passing_cells_description.txt`,
`provenance.txt`. Complete run: 1,032 runs, 0 failed, 80.4 s.

### Controls

- **(a)** Grid-2 cell 0 baseline run twice: byte-equal (`16862c75…` twice).
  All 360 Stage-2 forks were byte-identical to their cell's baseline.
- **(b)** The R12 smoke config and grid-1 cells 0, 36, 71, baseline only:
  SC-1, SC-2, SC-3 binds, SC-6 variance and T1 are exactly equal to the R16
  record in all four.
- **(c)** First cell in declared order where each condition fails / passes:

| Condition | First failing cell (raw) | First passing cell (raw) |
|---|---|---|
| SC-1 | cell 0: survival 1.000 | cell 4: survival 0.833 |
| SC-2 | cell 0: share 0.417 | cell 7: share 0.070 |
| SC-3 | cell 0: binds none | cell 77: binds all four |
| SC-6 | cell 1: variance 0.0000 | cell 0: variance 0.4426 |

### Stage 1 — which conditions fail

| Condition | Passes (of 288) | Fails among the 282 that fail T1 |
|---|---|---|
| SC-1 survival | 72 | 216 |
| SC-2 share below h_crit | 33 | 255 |
| SC-3 all four bind | 54 | 234 |
| SC-6 variance | 283 | 5 |

SC-2 fails most often, then SC-3, then SC-1. Eighteen cells fail exactly one
condition: twelve fail only SC-2 and six fail only SC-3.

Pass counts by axis level, described only:
- **SC-1** passes only with seeded obligation (72 of 144; 0 of 144 without),
  never at price 5, never at θ_limit 0.70. Baseline survival takes eight
  values: 0, 0.083, 0.167, 0.25, 0.333, 0.417, 0.833, 1.0.
- **SC-2** passes only at θ_limit 0.90 (33 of 72) and never with three
  low-capability firms. The in-band values are 0.070, 0.091, 0.163, 0.174.
- **SC-3** passes only with seeded obligation, at least one low-capability
  firm, and θ_limit 0.70 or 0.90. Each of solvency, compliance and
  obligation binds in 144 cells; scope in 216.
- The thin-capital axis changes no pass count for any condition.
- No shaping action is selected in any cell (SC-4 is 0 everywhere; SC-5 is
  undefined everywhere).

### Stage 1 — all 288 rows, declared order

L and T are the counts of low-capability and thin-capital firms. SC-3
column: Y = bound at least once, order solvency, compliance, scope,
obligation. SC-4 and SC-5 are information only.

| Cell | L | T | Price | Obligation | θ_limit | SC-1 | SC-2 | SC-3 binds (sol, cmp, scp, obl) | SC-6 var | T1 | SC-4 | SC-5 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 0 | 2 | none | 0.60 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4426 pass | FAIL | 0.000 | n/a |
| 1 | 0 | 0 | 2 | none | 0.70 | 0.000 FAIL | 0.333 FAIL | nYnn FAIL | 0.0000 FAIL | FAIL | 0.000 | n/a |
| 2 | 0 | 0 | 2 | none | 0.80 | 1.000 FAIL | 0.267 FAIL | nnnn FAIL | 0.2009 pass | FAIL | 0.000 | n/a |
| 3 | 0 | 0 | 2 | none | 0.90 | 1.000 FAIL | 0.008 FAIL | nYnn FAIL | 0.1831 pass | FAIL | 0.000 | n/a |
| 4 | 0 | 0 | 2 | seeded | 0.60 | 0.833 pass | 0.453 FAIL | YnnY FAIL | 0.4665 pass | FAIL | 0.000 | n/a |
| 5 | 0 | 0 | 2 | seeded | 0.70 | 0.000 FAIL | 0.818 FAIL | YYnY FAIL | 0.0000 FAIL | FAIL | 0.000 | n/a |
| 6 | 0 | 0 | 2 | seeded | 0.80 | 0.833 pass | 0.312 FAIL | YnnY FAIL | 0.2123 pass | FAIL | 0.000 | n/a |
| 7 | 0 | 0 | 2 | seeded | 0.90 | 0.833 pass | 0.070 pass | YYnY FAIL | 0.1934 pass | FAIL | 0.000 | n/a |
| 8 | 0 | 0 | 3 | none | 0.60 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4426 pass | FAIL | 0.000 | n/a |
| 9 | 0 | 0 | 3 | none | 0.70 | 0.000 FAIL | 0.333 FAIL | nYnn FAIL | 0.0000 FAIL | FAIL | 0.000 | n/a |
| 10 | 0 | 0 | 3 | none | 0.80 | 1.000 FAIL | 0.267 FAIL | nnnn FAIL | 0.2009 pass | FAIL | 0.000 | n/a |
| 11 | 0 | 0 | 3 | none | 0.90 | 1.000 FAIL | 0.008 FAIL | nYnn FAIL | 0.1831 pass | FAIL | 0.000 | n/a |
| 12 | 0 | 0 | 3 | seeded | 0.60 | 0.833 pass | 0.453 FAIL | YnnY FAIL | 0.4665 pass | FAIL | 0.000 | n/a |
| 13 | 0 | 0 | 3 | seeded | 0.70 | 0.000 FAIL | 0.818 FAIL | YYnY FAIL | 0.0000 FAIL | FAIL | 0.000 | n/a |
| 14 | 0 | 0 | 3 | seeded | 0.80 | 0.833 pass | 0.312 FAIL | YnnY FAIL | 0.2123 pass | FAIL | 0.000 | n/a |
| 15 | 0 | 0 | 3 | seeded | 0.90 | 0.833 pass | 0.070 pass | YYnY FAIL | 0.1934 pass | FAIL | 0.000 | n/a |
| 16 | 0 | 0 | 5 | none | 0.60 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4426 pass | FAIL | 0.000 | n/a |
| 17 | 0 | 0 | 5 | none | 0.70 | 0.000 FAIL | 0.333 FAIL | nYnn FAIL | 0.0000 FAIL | FAIL | 0.000 | n/a |
| 18 | 0 | 0 | 5 | none | 0.80 | 1.000 FAIL | 0.267 FAIL | nnnn FAIL | 0.2009 pass | FAIL | 0.000 | n/a |
| 19 | 0 | 0 | 5 | none | 0.90 | 1.000 FAIL | 0.008 FAIL | nYnn FAIL | 0.1831 pass | FAIL | 0.000 | n/a |
| 20 | 0 | 0 | 5 | seeded | 0.60 | 1.000 FAIL | 0.514 FAIL | YnnY FAIL | 0.4161 pass | FAIL | 0.000 | n/a |
| 21 | 0 | 0 | 5 | seeded | 0.70 | 0.167 FAIL | 0.926 FAIL | YYnY FAIL | 0.2397 pass | FAIL | 0.000 | n/a |
| 22 | 0 | 0 | 5 | seeded | 0.80 | 1.000 FAIL | 0.389 FAIL | YnnY FAIL | 0.2042 pass | FAIL | 0.000 | n/a |
| 23 | 0 | 0 | 5 | seeded | 0.90 | 1.000 FAIL | 0.174 pass | YYnY FAIL | 0.1898 pass | FAIL | 0.000 | n/a |
| 24 | 0 | 2 | 2 | none | 0.60 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4414 pass | FAIL | 0.000 | n/a |
| 25 | 0 | 2 | 2 | none | 0.70 | 0.000 FAIL | 0.324 FAIL | nYnn FAIL | 0.0622 pass | FAIL | 0.000 | n/a |
| 26 | 0 | 2 | 2 | none | 0.80 | 1.000 FAIL | 0.267 FAIL | nnnn FAIL | 0.2002 pass | FAIL | 0.000 | n/a |
| 27 | 0 | 2 | 2 | none | 0.90 | 1.000 FAIL | 0.008 FAIL | nYnn FAIL | 0.1826 pass | FAIL | 0.000 | n/a |
| 28 | 0 | 2 | 2 | seeded | 0.60 | 0.833 pass | 0.453 FAIL | YnnY FAIL | 0.4653 pass | FAIL | 0.000 | n/a |
| 29 | 0 | 2 | 2 | seeded | 0.70 | 0.000 FAIL | 0.811 FAIL | YYnY FAIL | 0.0219 pass | FAIL | 0.000 | n/a |
| 30 | 0 | 2 | 2 | seeded | 0.80 | 0.833 pass | 0.312 FAIL | YnnY FAIL | 0.2117 pass | FAIL | 0.000 | n/a |
| 31 | 0 | 2 | 2 | seeded | 0.90 | 0.833 pass | 0.070 pass | YYnY FAIL | 0.1929 pass | FAIL | 0.000 | n/a |
| 32 | 0 | 2 | 3 | none | 0.60 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4414 pass | FAIL | 0.000 | n/a |
| 33 | 0 | 2 | 3 | none | 0.70 | 0.000 FAIL | 0.324 FAIL | nYnn FAIL | 0.0622 pass | FAIL | 0.000 | n/a |
| 34 | 0 | 2 | 3 | none | 0.80 | 1.000 FAIL | 0.267 FAIL | nnnn FAIL | 0.2002 pass | FAIL | 0.000 | n/a |
| 35 | 0 | 2 | 3 | none | 0.90 | 1.000 FAIL | 0.008 FAIL | nYnn FAIL | 0.1826 pass | FAIL | 0.000 | n/a |
| 36 | 0 | 2 | 3 | seeded | 0.60 | 0.833 pass | 0.453 FAIL | YnnY FAIL | 0.4653 pass | FAIL | 0.000 | n/a |
| 37 | 0 | 2 | 3 | seeded | 0.70 | 0.000 FAIL | 0.811 FAIL | YYnY FAIL | 0.0219 pass | FAIL | 0.000 | n/a |
| 38 | 0 | 2 | 3 | seeded | 0.80 | 0.833 pass | 0.312 FAIL | YnnY FAIL | 0.2117 pass | FAIL | 0.000 | n/a |
| 39 | 0 | 2 | 3 | seeded | 0.90 | 0.833 pass | 0.070 pass | YYnY FAIL | 0.1929 pass | FAIL | 0.000 | n/a |
| 40 | 0 | 2 | 5 | none | 0.60 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4414 pass | FAIL | 0.000 | n/a |
| 41 | 0 | 2 | 5 | none | 0.70 | 0.000 FAIL | 0.324 FAIL | nYnn FAIL | 0.0622 pass | FAIL | 0.000 | n/a |
| 42 | 0 | 2 | 5 | none | 0.80 | 1.000 FAIL | 0.267 FAIL | nnnn FAIL | 0.2002 pass | FAIL | 0.000 | n/a |
| 43 | 0 | 2 | 5 | none | 0.90 | 1.000 FAIL | 0.008 FAIL | nYnn FAIL | 0.1826 pass | FAIL | 0.000 | n/a |
| 44 | 0 | 2 | 5 | seeded | 0.60 | 1.000 FAIL | 0.514 FAIL | YnnY FAIL | 0.4150 pass | FAIL | 0.000 | n/a |
| 45 | 0 | 2 | 5 | seeded | 0.70 | 0.167 FAIL | 0.923 FAIL | YYnY FAIL | 0.2374 pass | FAIL | 0.000 | n/a |
| 46 | 0 | 2 | 5 | seeded | 0.80 | 1.000 FAIL | 0.389 FAIL | YnnY FAIL | 0.2035 pass | FAIL | 0.000 | n/a |
| 47 | 0 | 2 | 5 | seeded | 0.90 | 1.000 FAIL | 0.174 pass | YYnY FAIL | 0.1892 pass | FAIL | 0.000 | n/a |
| 48 | 0 | 4 | 2 | none | 0.60 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4402 pass | FAIL | 0.000 | n/a |
| 49 | 0 | 4 | 2 | none | 0.70 | 0.000 FAIL | 0.316 FAIL | nYnn FAIL | 0.1110 pass | FAIL | 0.000 | n/a |
| 50 | 0 | 4 | 2 | none | 0.80 | 1.000 FAIL | 0.267 FAIL | nnnn FAIL | 0.1995 pass | FAIL | 0.000 | n/a |
| 51 | 0 | 4 | 2 | none | 0.90 | 1.000 FAIL | 0.008 FAIL | nYnn FAIL | 0.1820 pass | FAIL | 0.000 | n/a |
| 52 | 0 | 4 | 2 | seeded | 0.60 | 0.833 pass | 0.453 FAIL | YnnY FAIL | 0.4641 pass | FAIL | 0.000 | n/a |
| 53 | 0 | 4 | 2 | seeded | 0.70 | 0.000 FAIL | 0.804 FAIL | YYnY FAIL | 0.0423 pass | FAIL | 0.000 | n/a |
| 54 | 0 | 4 | 2 | seeded | 0.80 | 0.833 pass | 0.312 FAIL | YnnY FAIL | 0.2111 pass | FAIL | 0.000 | n/a |
| 55 | 0 | 4 | 2 | seeded | 0.90 | 0.833 pass | 0.070 pass | YYnY FAIL | 0.1924 pass | FAIL | 0.000 | n/a |
| 56 | 0 | 4 | 3 | none | 0.60 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4402 pass | FAIL | 0.000 | n/a |
| 57 | 0 | 4 | 3 | none | 0.70 | 0.000 FAIL | 0.316 FAIL | nYnn FAIL | 0.1110 pass | FAIL | 0.000 | n/a |
| 58 | 0 | 4 | 3 | none | 0.80 | 1.000 FAIL | 0.267 FAIL | nnnn FAIL | 0.1995 pass | FAIL | 0.000 | n/a |
| 59 | 0 | 4 | 3 | none | 0.90 | 1.000 FAIL | 0.008 FAIL | nYnn FAIL | 0.1820 pass | FAIL | 0.000 | n/a |
| 60 | 0 | 4 | 3 | seeded | 0.60 | 0.833 pass | 0.453 FAIL | YnnY FAIL | 0.4641 pass | FAIL | 0.000 | n/a |
| 61 | 0 | 4 | 3 | seeded | 0.70 | 0.000 FAIL | 0.804 FAIL | YYnY FAIL | 0.0423 pass | FAIL | 0.000 | n/a |
| 62 | 0 | 4 | 3 | seeded | 0.80 | 0.833 pass | 0.312 FAIL | YnnY FAIL | 0.2111 pass | FAIL | 0.000 | n/a |
| 63 | 0 | 4 | 3 | seeded | 0.90 | 0.833 pass | 0.070 pass | YYnY FAIL | 0.1924 pass | FAIL | 0.000 | n/a |
| 64 | 0 | 4 | 5 | none | 0.60 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4402 pass | FAIL | 0.000 | n/a |
| 65 | 0 | 4 | 5 | none | 0.70 | 0.000 FAIL | 0.316 FAIL | nYnn FAIL | 0.1110 pass | FAIL | 0.000 | n/a |
| 66 | 0 | 4 | 5 | none | 0.80 | 1.000 FAIL | 0.267 FAIL | nnnn FAIL | 0.1995 pass | FAIL | 0.000 | n/a |
| 67 | 0 | 4 | 5 | none | 0.90 | 1.000 FAIL | 0.008 FAIL | nYnn FAIL | 0.1820 pass | FAIL | 0.000 | n/a |
| 68 | 0 | 4 | 5 | seeded | 0.60 | 1.000 FAIL | 0.514 FAIL | YnnY FAIL | 0.4139 pass | FAIL | 0.000 | n/a |
| 69 | 0 | 4 | 5 | seeded | 0.70 | 0.167 FAIL | 0.919 FAIL | YYnY FAIL | 0.2349 pass | FAIL | 0.000 | n/a |
| 70 | 0 | 4 | 5 | seeded | 0.80 | 1.000 FAIL | 0.389 FAIL | YnnY FAIL | 0.2028 pass | FAIL | 0.000 | n/a |
| 71 | 0 | 4 | 5 | seeded | 0.90 | 1.000 FAIL | 0.174 pass | YYnY FAIL | 0.1886 pass | FAIL | 0.000 | n/a |
| 72 | 1 | 0 | 2 | none | 0.60 | 1.000 FAIL | 0.465 FAIL | nnYn FAIL | 0.4299 pass | FAIL | 0.000 | n/a |
| 73 | 1 | 0 | 2 | none | 0.70 | 0.083 FAIL | 0.856 FAIL | nYYn FAIL | 0.2476 pass | FAIL | 0.000 | n/a |
| 74 | 1 | 0 | 2 | none | 0.80 | 1.000 FAIL | 0.328 FAIL | nnYn FAIL | 0.2026 pass | FAIL | 0.000 | n/a |
| 75 | 1 | 0 | 2 | none | 0.90 | 1.000 FAIL | 0.091 pass | nYYn FAIL | 0.1865 pass | FAIL | 0.000 | n/a |
| 76 | 1 | 0 | 2 | seeded | 0.60 | 0.833 pass | 0.508 FAIL | YnYY FAIL | 0.4491 pass | FAIL | 0.000 | n/a |
| 77 | 1 | 0 | 2 | seeded | 0.70 | 0.083 FAIL | 0.921 FAIL | YYYY pass | 0.2264 pass | FAIL | 0.000 | n/a |
| 78 | 1 | 0 | 2 | seeded | 0.80 | 0.833 pass | 0.381 FAIL | YnYY FAIL | 0.2144 pass | FAIL | 0.000 | n/a |
| 79 | 1 | 0 | 2 | seeded | 0.90 | 0.833 pass | 0.163 pass | YYYY pass | 0.1976 pass | **PASS** | 0.000 | n/a |
| 80 | 1 | 0 | 3 | none | 0.60 | 1.000 FAIL | 0.465 FAIL | nnYn FAIL | 0.4299 pass | FAIL | 0.000 | n/a |
| 81 | 1 | 0 | 3 | none | 0.70 | 0.083 FAIL | 0.856 FAIL | nYYn FAIL | 0.2476 pass | FAIL | 0.000 | n/a |
| 82 | 1 | 0 | 3 | none | 0.80 | 1.000 FAIL | 0.328 FAIL | nnYn FAIL | 0.2026 pass | FAIL | 0.000 | n/a |
| 83 | 1 | 0 | 3 | none | 0.90 | 1.000 FAIL | 0.091 pass | nYYn FAIL | 0.1865 pass | FAIL | 0.000 | n/a |
| 84 | 1 | 0 | 3 | seeded | 0.60 | 0.833 pass | 0.508 FAIL | YnYY FAIL | 0.4491 pass | FAIL | 0.000 | n/a |
| 85 | 1 | 0 | 3 | seeded | 0.70 | 0.083 FAIL | 0.921 FAIL | YYYY pass | 0.2264 pass | FAIL | 0.000 | n/a |
| 86 | 1 | 0 | 3 | seeded | 0.80 | 0.833 pass | 0.381 FAIL | YnYY FAIL | 0.2144 pass | FAIL | 0.000 | n/a |
| 87 | 1 | 0 | 3 | seeded | 0.90 | 0.833 pass | 0.163 pass | YYYY pass | 0.1976 pass | **PASS** | 0.000 | n/a |
| 88 | 1 | 0 | 5 | none | 0.60 | 1.000 FAIL | 0.465 FAIL | nnYn FAIL | 0.4299 pass | FAIL | 0.000 | n/a |
| 89 | 1 | 0 | 5 | none | 0.70 | 0.083 FAIL | 0.856 FAIL | nYYn FAIL | 0.2476 pass | FAIL | 0.000 | n/a |
| 90 | 1 | 0 | 5 | none | 0.80 | 1.000 FAIL | 0.328 FAIL | nnYn FAIL | 0.2026 pass | FAIL | 0.000 | n/a |
| 91 | 1 | 0 | 5 | none | 0.90 | 1.000 FAIL | 0.091 pass | nYYn FAIL | 0.1865 pass | FAIL | 0.000 | n/a |
| 92 | 1 | 0 | 5 | seeded | 0.60 | 1.000 FAIL | 0.562 FAIL | YnYY FAIL | 0.4013 pass | FAIL | 0.000 | n/a |
| 93 | 1 | 0 | 5 | seeded | 0.70 | 0.250 FAIL | 0.953 FAIL | YYYY pass | 0.2339 pass | FAIL | 0.000 | n/a |
| 94 | 1 | 0 | 5 | seeded | 0.80 | 1.000 FAIL | 0.450 FAIL | YnYY FAIL | 0.2059 pass | FAIL | 0.000 | n/a |
| 95 | 1 | 0 | 5 | seeded | 0.90 | 1.000 FAIL | 0.256 FAIL | YYYY pass | 0.1931 pass | FAIL | 0.000 | n/a |
| 96 | 1 | 2 | 2 | none | 0.60 | 1.000 FAIL | 0.465 FAIL | nnYn FAIL | 0.4287 pass | FAIL | 0.000 | n/a |
| 97 | 1 | 2 | 2 | none | 0.70 | 0.083 FAIL | 0.851 FAIL | nYYn FAIL | 0.2453 pass | FAIL | 0.000 | n/a |
| 98 | 1 | 2 | 2 | none | 0.80 | 1.000 FAIL | 0.328 FAIL | nnYn FAIL | 0.2019 pass | FAIL | 0.000 | n/a |
| 99 | 1 | 2 | 2 | none | 0.90 | 1.000 FAIL | 0.091 pass | nYYn FAIL | 0.1859 pass | FAIL | 0.000 | n/a |
| 100 | 1 | 2 | 2 | seeded | 0.60 | 0.833 pass | 0.508 FAIL | YnYY FAIL | 0.4480 pass | FAIL | 0.000 | n/a |
| 101 | 1 | 2 | 2 | seeded | 0.70 | 0.083 FAIL | 0.917 FAIL | YYYY pass | 0.2284 pass | FAIL | 0.000 | n/a |
| 102 | 1 | 2 | 2 | seeded | 0.80 | 0.833 pass | 0.381 FAIL | YnYY FAIL | 0.2138 pass | FAIL | 0.000 | n/a |
| 103 | 1 | 2 | 2 | seeded | 0.90 | 0.833 pass | 0.163 pass | YYYY pass | 0.1971 pass | **PASS** | 0.000 | n/a |
| 104 | 1 | 2 | 3 | none | 0.60 | 1.000 FAIL | 0.465 FAIL | nnYn FAIL | 0.4287 pass | FAIL | 0.000 | n/a |
| 105 | 1 | 2 | 3 | none | 0.70 | 0.083 FAIL | 0.851 FAIL | nYYn FAIL | 0.2453 pass | FAIL | 0.000 | n/a |
| 106 | 1 | 2 | 3 | none | 0.80 | 1.000 FAIL | 0.328 FAIL | nnYn FAIL | 0.2019 pass | FAIL | 0.000 | n/a |
| 107 | 1 | 2 | 3 | none | 0.90 | 1.000 FAIL | 0.091 pass | nYYn FAIL | 0.1859 pass | FAIL | 0.000 | n/a |
| 108 | 1 | 2 | 3 | seeded | 0.60 | 0.833 pass | 0.508 FAIL | YnYY FAIL | 0.4480 pass | FAIL | 0.000 | n/a |
| 109 | 1 | 2 | 3 | seeded | 0.70 | 0.083 FAIL | 0.917 FAIL | YYYY pass | 0.2284 pass | FAIL | 0.000 | n/a |
| 110 | 1 | 2 | 3 | seeded | 0.80 | 0.833 pass | 0.381 FAIL | YnYY FAIL | 0.2138 pass | FAIL | 0.000 | n/a |
| 111 | 1 | 2 | 3 | seeded | 0.90 | 0.833 pass | 0.163 pass | YYYY pass | 0.1971 pass | **PASS** | 0.000 | n/a |
| 112 | 1 | 2 | 5 | none | 0.60 | 1.000 FAIL | 0.465 FAIL | nnYn FAIL | 0.4287 pass | FAIL | 0.000 | n/a |
| 113 | 1 | 2 | 5 | none | 0.70 | 0.083 FAIL | 0.851 FAIL | nYYn FAIL | 0.2453 pass | FAIL | 0.000 | n/a |
| 114 | 1 | 2 | 5 | none | 0.80 | 1.000 FAIL | 0.328 FAIL | nnYn FAIL | 0.2019 pass | FAIL | 0.000 | n/a |
| 115 | 1 | 2 | 5 | none | 0.90 | 1.000 FAIL | 0.091 pass | nYYn FAIL | 0.1859 pass | FAIL | 0.000 | n/a |
| 116 | 1 | 2 | 5 | seeded | 0.60 | 1.000 FAIL | 0.562 FAIL | YnYY FAIL | 0.4002 pass | FAIL | 0.000 | n/a |
| 117 | 1 | 2 | 5 | seeded | 0.70 | 0.250 FAIL | 0.951 FAIL | YYYY pass | 0.2320 pass | FAIL | 0.000 | n/a |
| 118 | 1 | 2 | 5 | seeded | 0.80 | 1.000 FAIL | 0.450 FAIL | YnYY FAIL | 0.2052 pass | FAIL | 0.000 | n/a |
| 119 | 1 | 2 | 5 | seeded | 0.90 | 1.000 FAIL | 0.256 FAIL | YYYY pass | 0.1925 pass | FAIL | 0.000 | n/a |
| 120 | 1 | 4 | 2 | none | 0.60 | 1.000 FAIL | 0.465 FAIL | nnYn FAIL | 0.4276 pass | FAIL | 0.000 | n/a |
| 121 | 1 | 4 | 2 | none | 0.70 | 0.083 FAIL | 0.845 FAIL | nYYn FAIL | 0.2427 pass | FAIL | 0.000 | n/a |
| 122 | 1 | 4 | 2 | none | 0.80 | 1.000 FAIL | 0.328 FAIL | nnYn FAIL | 0.2012 pass | FAIL | 0.000 | n/a |
| 123 | 1 | 4 | 2 | none | 0.90 | 1.000 FAIL | 0.091 pass | nYYn FAIL | 0.1853 pass | FAIL | 0.000 | n/a |
| 124 | 1 | 4 | 2 | seeded | 0.60 | 0.833 pass | 0.508 FAIL | YnYY FAIL | 0.4468 pass | FAIL | 0.000 | n/a |
| 125 | 1 | 4 | 2 | seeded | 0.70 | 0.083 FAIL | 0.913 FAIL | YYYY pass | 0.2302 pass | FAIL | 0.000 | n/a |
| 126 | 1 | 4 | 2 | seeded | 0.80 | 0.833 pass | 0.381 FAIL | YnYY FAIL | 0.2131 pass | FAIL | 0.000 | n/a |
| 127 | 1 | 4 | 2 | seeded | 0.90 | 0.833 pass | 0.163 pass | YYYY pass | 0.1965 pass | **PASS** | 0.000 | n/a |
| 128 | 1 | 4 | 3 | none | 0.60 | 1.000 FAIL | 0.465 FAIL | nnYn FAIL | 0.4276 pass | FAIL | 0.000 | n/a |
| 129 | 1 | 4 | 3 | none | 0.70 | 0.083 FAIL | 0.845 FAIL | nYYn FAIL | 0.2427 pass | FAIL | 0.000 | n/a |
| 130 | 1 | 4 | 3 | none | 0.80 | 1.000 FAIL | 0.328 FAIL | nnYn FAIL | 0.2012 pass | FAIL | 0.000 | n/a |
| 131 | 1 | 4 | 3 | none | 0.90 | 1.000 FAIL | 0.091 pass | nYYn FAIL | 0.1853 pass | FAIL | 0.000 | n/a |
| 132 | 1 | 4 | 3 | seeded | 0.60 | 0.833 pass | 0.508 FAIL | YnYY FAIL | 0.4468 pass | FAIL | 0.000 | n/a |
| 133 | 1 | 4 | 3 | seeded | 0.70 | 0.083 FAIL | 0.913 FAIL | YYYY pass | 0.2302 pass | FAIL | 0.000 | n/a |
| 134 | 1 | 4 | 3 | seeded | 0.80 | 0.833 pass | 0.381 FAIL | YnYY FAIL | 0.2131 pass | FAIL | 0.000 | n/a |
| 135 | 1 | 4 | 3 | seeded | 0.90 | 0.833 pass | 0.163 pass | YYYY pass | 0.1965 pass | **PASS** | 0.000 | n/a |
| 136 | 1 | 4 | 5 | none | 0.60 | 1.000 FAIL | 0.465 FAIL | nnYn FAIL | 0.4276 pass | FAIL | 0.000 | n/a |
| 137 | 1 | 4 | 5 | none | 0.70 | 0.083 FAIL | 0.845 FAIL | nYYn FAIL | 0.2427 pass | FAIL | 0.000 | n/a |
| 138 | 1 | 4 | 5 | none | 0.80 | 1.000 FAIL | 0.328 FAIL | nnYn FAIL | 0.2012 pass | FAIL | 0.000 | n/a |
| 139 | 1 | 4 | 5 | none | 0.90 | 1.000 FAIL | 0.091 pass | nYYn FAIL | 0.1853 pass | FAIL | 0.000 | n/a |
| 140 | 1 | 4 | 5 | seeded | 0.60 | 1.000 FAIL | 0.562 FAIL | YnYY FAIL | 0.3992 pass | FAIL | 0.000 | n/a |
| 141 | 1 | 4 | 5 | seeded | 0.70 | 0.250 FAIL | 0.949 FAIL | YYYY pass | 0.2301 pass | FAIL | 0.000 | n/a |
| 142 | 1 | 4 | 5 | seeded | 0.80 | 1.000 FAIL | 0.450 FAIL | YnYY FAIL | 0.2045 pass | FAIL | 0.000 | n/a |
| 143 | 1 | 4 | 5 | seeded | 0.90 | 1.000 FAIL | 0.256 FAIL | YYYY pass | 0.1919 pass | FAIL | 0.000 | n/a |
| 144 | 2 | 0 | 2 | none | 0.60 | 1.000 FAIL | 0.514 FAIL | nnYn FAIL | 0.4161 pass | FAIL | 0.000 | n/a |
| 145 | 2 | 0 | 2 | none | 0.70 | 0.167 FAIL | 0.926 FAIL | nYYn FAIL | 0.2397 pass | FAIL | 0.000 | n/a |
| 146 | 2 | 0 | 2 | none | 0.80 | 1.000 FAIL | 0.389 FAIL | nnYn FAIL | 0.2042 pass | FAIL | 0.000 | n/a |
| 147 | 2 | 0 | 2 | none | 0.90 | 1.000 FAIL | 0.174 pass | nYYn FAIL | 0.1898 pass | FAIL | 0.000 | n/a |
| 148 | 2 | 0 | 2 | seeded | 0.60 | 0.833 pass | 0.562 FAIL | YnYY FAIL | 0.4303 pass | FAIL | 0.000 | n/a |
| 149 | 2 | 0 | 2 | seeded | 0.70 | 0.167 FAIL | 0.953 FAIL | YYYY pass | 0.2468 pass | FAIL | 0.000 | n/a |
| 150 | 2 | 0 | 2 | seeded | 0.80 | 0.833 pass | 0.450 FAIL | YnYY FAIL | 0.2165 pass | FAIL | 0.000 | n/a |
| 151 | 2 | 0 | 2 | seeded | 0.90 | 0.833 pass | 0.256 FAIL | YYYY pass | 0.2017 pass | FAIL | 0.000 | n/a |
| 152 | 2 | 0 | 3 | none | 0.60 | 1.000 FAIL | 0.514 FAIL | nnYn FAIL | 0.4161 pass | FAIL | 0.000 | n/a |
| 153 | 2 | 0 | 3 | none | 0.70 | 0.167 FAIL | 0.926 FAIL | nYYn FAIL | 0.2397 pass | FAIL | 0.000 | n/a |
| 154 | 2 | 0 | 3 | none | 0.80 | 1.000 FAIL | 0.389 FAIL | nnYn FAIL | 0.2042 pass | FAIL | 0.000 | n/a |
| 155 | 2 | 0 | 3 | none | 0.90 | 1.000 FAIL | 0.174 pass | nYYn FAIL | 0.1898 pass | FAIL | 0.000 | n/a |
| 156 | 2 | 0 | 3 | seeded | 0.60 | 0.833 pass | 0.562 FAIL | YnYY FAIL | 0.4303 pass | FAIL | 0.000 | n/a |
| 157 | 2 | 0 | 3 | seeded | 0.70 | 0.167 FAIL | 0.953 FAIL | YYYY pass | 0.2468 pass | FAIL | 0.000 | n/a |
| 158 | 2 | 0 | 3 | seeded | 0.80 | 0.833 pass | 0.450 FAIL | YnYY FAIL | 0.2165 pass | FAIL | 0.000 | n/a |
| 159 | 2 | 0 | 3 | seeded | 0.90 | 0.833 pass | 0.256 FAIL | YYYY pass | 0.2017 pass | FAIL | 0.000 | n/a |
| 160 | 2 | 0 | 5 | none | 0.60 | 1.000 FAIL | 0.514 FAIL | nnYn FAIL | 0.4161 pass | FAIL | 0.000 | n/a |
| 161 | 2 | 0 | 5 | none | 0.70 | 0.167 FAIL | 0.926 FAIL | nYYn FAIL | 0.2397 pass | FAIL | 0.000 | n/a |
| 162 | 2 | 0 | 5 | none | 0.80 | 1.000 FAIL | 0.389 FAIL | nnYn FAIL | 0.2042 pass | FAIL | 0.000 | n/a |
| 163 | 2 | 0 | 5 | none | 0.90 | 1.000 FAIL | 0.174 pass | nYYn FAIL | 0.1898 pass | FAIL | 0.000 | n/a |
| 164 | 2 | 0 | 5 | seeded | 0.60 | 1.000 FAIL | 0.611 FAIL | YnYY FAIL | 0.3854 pass | FAIL | 0.000 | n/a |
| 165 | 2 | 0 | 5 | seeded | 0.70 | 0.333 FAIL | 0.968 FAIL | YYYY pass | 0.2302 pass | FAIL | 0.000 | n/a |
| 166 | 2 | 0 | 5 | seeded | 0.80 | 1.000 FAIL | 0.511 FAIL | YnYY FAIL | 0.2076 pass | FAIL | 0.000 | n/a |
| 167 | 2 | 0 | 5 | seeded | 0.90 | 1.000 FAIL | 0.339 FAIL | YYYY pass | 0.1963 pass | FAIL | 0.000 | n/a |
| 168 | 2 | 2 | 2 | none | 0.60 | 1.000 FAIL | 0.514 FAIL | nnYn FAIL | 0.4150 pass | FAIL | 0.000 | n/a |
| 169 | 2 | 2 | 2 | none | 0.70 | 0.167 FAIL | 0.923 FAIL | nYYn FAIL | 0.2374 pass | FAIL | 0.000 | n/a |
| 170 | 2 | 2 | 2 | none | 0.80 | 1.000 FAIL | 0.389 FAIL | nnYn FAIL | 0.2035 pass | FAIL | 0.000 | n/a |
| 171 | 2 | 2 | 2 | none | 0.90 | 1.000 FAIL | 0.174 pass | nYYn FAIL | 0.1892 pass | FAIL | 0.000 | n/a |
| 172 | 2 | 2 | 2 | seeded | 0.60 | 0.833 pass | 0.562 FAIL | YnYY FAIL | 0.4293 pass | FAIL | 0.000 | n/a |
| 173 | 2 | 2 | 2 | seeded | 0.70 | 0.167 FAIL | 0.951 FAIL | YYYY pass | 0.2466 pass | FAIL | 0.000 | n/a |
| 174 | 2 | 2 | 2 | seeded | 0.80 | 0.833 pass | 0.450 FAIL | YnYY FAIL | 0.2158 pass | FAIL | 0.000 | n/a |
| 175 | 2 | 2 | 2 | seeded | 0.90 | 0.833 pass | 0.256 FAIL | YYYY pass | 0.2011 pass | FAIL | 0.000 | n/a |
| 176 | 2 | 2 | 3 | none | 0.60 | 1.000 FAIL | 0.514 FAIL | nnYn FAIL | 0.4150 pass | FAIL | 0.000 | n/a |
| 177 | 2 | 2 | 3 | none | 0.70 | 0.167 FAIL | 0.923 FAIL | nYYn FAIL | 0.2374 pass | FAIL | 0.000 | n/a |
| 178 | 2 | 2 | 3 | none | 0.80 | 1.000 FAIL | 0.389 FAIL | nnYn FAIL | 0.2035 pass | FAIL | 0.000 | n/a |
| 179 | 2 | 2 | 3 | none | 0.90 | 1.000 FAIL | 0.174 pass | nYYn FAIL | 0.1892 pass | FAIL | 0.000 | n/a |
| 180 | 2 | 2 | 3 | seeded | 0.60 | 0.833 pass | 0.562 FAIL | YnYY FAIL | 0.4293 pass | FAIL | 0.000 | n/a |
| 181 | 2 | 2 | 3 | seeded | 0.70 | 0.167 FAIL | 0.951 FAIL | YYYY pass | 0.2466 pass | FAIL | 0.000 | n/a |
| 182 | 2 | 2 | 3 | seeded | 0.80 | 0.833 pass | 0.450 FAIL | YnYY FAIL | 0.2158 pass | FAIL | 0.000 | n/a |
| 183 | 2 | 2 | 3 | seeded | 0.90 | 0.833 pass | 0.256 FAIL | YYYY pass | 0.2011 pass | FAIL | 0.000 | n/a |
| 184 | 2 | 2 | 5 | none | 0.60 | 1.000 FAIL | 0.514 FAIL | nnYn FAIL | 0.4150 pass | FAIL | 0.000 | n/a |
| 185 | 2 | 2 | 5 | none | 0.70 | 0.167 FAIL | 0.923 FAIL | nYYn FAIL | 0.2374 pass | FAIL | 0.000 | n/a |
| 186 | 2 | 2 | 5 | none | 0.80 | 1.000 FAIL | 0.389 FAIL | nnYn FAIL | 0.2035 pass | FAIL | 0.000 | n/a |
| 187 | 2 | 2 | 5 | none | 0.90 | 1.000 FAIL | 0.174 pass | nYYn FAIL | 0.1892 pass | FAIL | 0.000 | n/a |
| 188 | 2 | 2 | 5 | seeded | 0.60 | 1.000 FAIL | 0.611 FAIL | YnYY FAIL | 0.3844 pass | FAIL | 0.000 | n/a |
| 189 | 2 | 2 | 5 | seeded | 0.70 | 0.333 FAIL | 0.966 FAIL | YYYY pass | 0.2286 pass | FAIL | 0.000 | n/a |
| 190 | 2 | 2 | 5 | seeded | 0.80 | 1.000 FAIL | 0.511 FAIL | YnYY FAIL | 0.2069 pass | FAIL | 0.000 | n/a |
| 191 | 2 | 2 | 5 | seeded | 0.90 | 1.000 FAIL | 0.339 FAIL | YYYY pass | 0.1957 pass | FAIL | 0.000 | n/a |
| 192 | 2 | 4 | 2 | none | 0.60 | 1.000 FAIL | 0.514 FAIL | nnYn FAIL | 0.4139 pass | FAIL | 0.000 | n/a |
| 193 | 2 | 4 | 2 | none | 0.70 | 0.167 FAIL | 0.919 FAIL | nYYn FAIL | 0.2349 pass | FAIL | 0.000 | n/a |
| 194 | 2 | 4 | 2 | none | 0.80 | 1.000 FAIL | 0.389 FAIL | nnYn FAIL | 0.2028 pass | FAIL | 0.000 | n/a |
| 195 | 2 | 4 | 2 | none | 0.90 | 1.000 FAIL | 0.174 pass | nYYn FAIL | 0.1886 pass | FAIL | 0.000 | n/a |
| 196 | 2 | 4 | 2 | seeded | 0.60 | 0.833 pass | 0.562 FAIL | YnYY FAIL | 0.4283 pass | FAIL | 0.000 | n/a |
| 197 | 2 | 4 | 2 | seeded | 0.70 | 0.167 FAIL | 0.948 FAIL | YYYY pass | 0.2463 pass | FAIL | 0.000 | n/a |
| 198 | 2 | 4 | 2 | seeded | 0.80 | 0.833 pass | 0.450 FAIL | YnYY FAIL | 0.2152 pass | FAIL | 0.000 | n/a |
| 199 | 2 | 4 | 2 | seeded | 0.90 | 0.833 pass | 0.256 FAIL | YYYY pass | 0.2006 pass | FAIL | 0.000 | n/a |
| 200 | 2 | 4 | 3 | none | 0.60 | 1.000 FAIL | 0.514 FAIL | nnYn FAIL | 0.4139 pass | FAIL | 0.000 | n/a |
| 201 | 2 | 4 | 3 | none | 0.70 | 0.167 FAIL | 0.919 FAIL | nYYn FAIL | 0.2349 pass | FAIL | 0.000 | n/a |
| 202 | 2 | 4 | 3 | none | 0.80 | 1.000 FAIL | 0.389 FAIL | nnYn FAIL | 0.2028 pass | FAIL | 0.000 | n/a |
| 203 | 2 | 4 | 3 | none | 0.90 | 1.000 FAIL | 0.174 pass | nYYn FAIL | 0.1886 pass | FAIL | 0.000 | n/a |
| 204 | 2 | 4 | 3 | seeded | 0.60 | 0.833 pass | 0.562 FAIL | YnYY FAIL | 0.4283 pass | FAIL | 0.000 | n/a |
| 205 | 2 | 4 | 3 | seeded | 0.70 | 0.167 FAIL | 0.948 FAIL | YYYY pass | 0.2463 pass | FAIL | 0.000 | n/a |
| 206 | 2 | 4 | 3 | seeded | 0.80 | 0.833 pass | 0.450 FAIL | YnYY FAIL | 0.2152 pass | FAIL | 0.000 | n/a |
| 207 | 2 | 4 | 3 | seeded | 0.90 | 0.833 pass | 0.256 FAIL | YYYY pass | 0.2006 pass | FAIL | 0.000 | n/a |
| 208 | 2 | 4 | 5 | none | 0.60 | 1.000 FAIL | 0.514 FAIL | nnYn FAIL | 0.4139 pass | FAIL | 0.000 | n/a |
| 209 | 2 | 4 | 5 | none | 0.70 | 0.167 FAIL | 0.919 FAIL | nYYn FAIL | 0.2349 pass | FAIL | 0.000 | n/a |
| 210 | 2 | 4 | 5 | none | 0.80 | 1.000 FAIL | 0.389 FAIL | nnYn FAIL | 0.2028 pass | FAIL | 0.000 | n/a |
| 211 | 2 | 4 | 5 | none | 0.90 | 1.000 FAIL | 0.174 pass | nYYn FAIL | 0.1886 pass | FAIL | 0.000 | n/a |
| 212 | 2 | 4 | 5 | seeded | 0.60 | 1.000 FAIL | 0.611 FAIL | YnYY FAIL | 0.3834 pass | FAIL | 0.000 | n/a |
| 213 | 2 | 4 | 5 | seeded | 0.70 | 0.333 FAIL | 0.964 FAIL | YYYY pass | 0.2270 pass | FAIL | 0.000 | n/a |
| 214 | 2 | 4 | 5 | seeded | 0.80 | 1.000 FAIL | 0.511 FAIL | YnYY FAIL | 0.2062 pass | FAIL | 0.000 | n/a |
| 215 | 2 | 4 | 5 | seeded | 0.90 | 1.000 FAIL | 0.339 FAIL | YYYY pass | 0.1951 pass | FAIL | 0.000 | n/a |
| 216 | 3 | 0 | 2 | none | 0.60 | 1.000 FAIL | 0.562 FAIL | nnYn FAIL | 0.4013 pass | FAIL | 0.000 | n/a |
| 217 | 3 | 0 | 2 | none | 0.70 | 0.250 FAIL | 0.953 FAIL | nYYn FAIL | 0.2339 pass | FAIL | 0.000 | n/a |
| 218 | 3 | 0 | 2 | none | 0.80 | 1.000 FAIL | 0.450 FAIL | nnYn FAIL | 0.2059 pass | FAIL | 0.000 | n/a |
| 219 | 3 | 0 | 2 | none | 0.90 | 1.000 FAIL | 0.256 FAIL | nYYn FAIL | 0.1931 pass | FAIL | 0.000 | n/a |
| 220 | 3 | 0 | 2 | seeded | 0.60 | 0.833 pass | 0.617 FAIL | YnYY FAIL | 0.4102 pass | FAIL | 0.000 | n/a |
| 221 | 3 | 0 | 2 | seeded | 0.70 | 0.250 FAIL | 0.970 FAIL | YYYY pass | 0.2477 pass | FAIL | 0.000 | n/a |
| 222 | 3 | 0 | 2 | seeded | 0.80 | 0.833 pass | 0.519 FAIL | YnYY FAIL | 0.2185 pass | FAIL | 0.000 | n/a |
| 223 | 3 | 0 | 2 | seeded | 0.90 | 0.833 pass | 0.349 FAIL | YYYY pass | 0.2057 pass | FAIL | 0.000 | n/a |
| 224 | 3 | 0 | 3 | none | 0.60 | 1.000 FAIL | 0.562 FAIL | nnYn FAIL | 0.4013 pass | FAIL | 0.000 | n/a |
| 225 | 3 | 0 | 3 | none | 0.70 | 0.250 FAIL | 0.953 FAIL | nYYn FAIL | 0.2339 pass | FAIL | 0.000 | n/a |
| 226 | 3 | 0 | 3 | none | 0.80 | 1.000 FAIL | 0.450 FAIL | nnYn FAIL | 0.2059 pass | FAIL | 0.000 | n/a |
| 227 | 3 | 0 | 3 | none | 0.90 | 1.000 FAIL | 0.256 FAIL | nYYn FAIL | 0.1931 pass | FAIL | 0.000 | n/a |
| 228 | 3 | 0 | 3 | seeded | 0.60 | 0.833 pass | 0.617 FAIL | YnYY FAIL | 0.4102 pass | FAIL | 0.000 | n/a |
| 229 | 3 | 0 | 3 | seeded | 0.70 | 0.250 FAIL | 0.970 FAIL | YYYY pass | 0.2477 pass | FAIL | 0.000 | n/a |
| 230 | 3 | 0 | 3 | seeded | 0.80 | 0.833 pass | 0.519 FAIL | YnYY FAIL | 0.2185 pass | FAIL | 0.000 | n/a |
| 231 | 3 | 0 | 3 | seeded | 0.90 | 0.833 pass | 0.349 FAIL | YYYY pass | 0.2057 pass | FAIL | 0.000 | n/a |
| 232 | 3 | 0 | 5 | none | 0.60 | 1.000 FAIL | 0.562 FAIL | nnYn FAIL | 0.4013 pass | FAIL | 0.000 | n/a |
| 233 | 3 | 0 | 5 | none | 0.70 | 0.250 FAIL | 0.953 FAIL | nYYn FAIL | 0.2339 pass | FAIL | 0.000 | n/a |
| 234 | 3 | 0 | 5 | none | 0.80 | 1.000 FAIL | 0.450 FAIL | nnYn FAIL | 0.2059 pass | FAIL | 0.000 | n/a |
| 235 | 3 | 0 | 5 | none | 0.90 | 1.000 FAIL | 0.256 FAIL | nYYn FAIL | 0.1931 pass | FAIL | 0.000 | n/a |
| 236 | 3 | 0 | 5 | seeded | 0.60 | 1.000 FAIL | 0.660 FAIL | YnYY FAIL | 0.3684 pass | FAIL | 0.000 | n/a |
| 237 | 3 | 0 | 5 | seeded | 0.70 | 0.417 FAIL | 0.977 FAIL | YYYY pass | 0.2276 pass | FAIL | 0.000 | n/a |
| 238 | 3 | 0 | 5 | seeded | 0.80 | 1.000 FAIL | 0.572 FAIL | YnYY FAIL | 0.2093 pass | FAIL | 0.000 | n/a |
| 239 | 3 | 0 | 5 | seeded | 0.90 | 1.000 FAIL | 0.422 FAIL | YYYY pass | 0.1995 pass | FAIL | 0.000 | n/a |
| 240 | 3 | 2 | 2 | none | 0.60 | 1.000 FAIL | 0.562 FAIL | nnYn FAIL | 0.4002 pass | FAIL | 0.000 | n/a |
| 241 | 3 | 2 | 2 | none | 0.70 | 0.250 FAIL | 0.951 FAIL | nYYn FAIL | 0.2320 pass | FAIL | 0.000 | n/a |
| 242 | 3 | 2 | 2 | none | 0.80 | 1.000 FAIL | 0.450 FAIL | nnYn FAIL | 0.2052 pass | FAIL | 0.000 | n/a |
| 243 | 3 | 2 | 2 | none | 0.90 | 1.000 FAIL | 0.256 FAIL | nYYn FAIL | 0.1925 pass | FAIL | 0.000 | n/a |
| 244 | 3 | 2 | 2 | seeded | 0.60 | 0.833 pass | 0.617 FAIL | YnYY FAIL | 0.4093 pass | FAIL | 0.000 | n/a |
| 245 | 3 | 2 | 2 | seeded | 0.70 | 0.250 FAIL | 0.968 FAIL | YYYY pass | 0.2470 pass | FAIL | 0.000 | n/a |
| 246 | 3 | 2 | 2 | seeded | 0.80 | 0.833 pass | 0.519 FAIL | YnYY FAIL | 0.2179 pass | FAIL | 0.000 | n/a |
| 247 | 3 | 2 | 2 | seeded | 0.90 | 0.833 pass | 0.349 FAIL | YYYY pass | 0.2052 pass | FAIL | 0.000 | n/a |
| 248 | 3 | 2 | 3 | none | 0.60 | 1.000 FAIL | 0.562 FAIL | nnYn FAIL | 0.4002 pass | FAIL | 0.000 | n/a |
| 249 | 3 | 2 | 3 | none | 0.70 | 0.250 FAIL | 0.951 FAIL | nYYn FAIL | 0.2320 pass | FAIL | 0.000 | n/a |
| 250 | 3 | 2 | 3 | none | 0.80 | 1.000 FAIL | 0.450 FAIL | nnYn FAIL | 0.2052 pass | FAIL | 0.000 | n/a |
| 251 | 3 | 2 | 3 | none | 0.90 | 1.000 FAIL | 0.256 FAIL | nYYn FAIL | 0.1925 pass | FAIL | 0.000 | n/a |
| 252 | 3 | 2 | 3 | seeded | 0.60 | 0.833 pass | 0.617 FAIL | YnYY FAIL | 0.4093 pass | FAIL | 0.000 | n/a |
| 253 | 3 | 2 | 3 | seeded | 0.70 | 0.250 FAIL | 0.968 FAIL | YYYY pass | 0.2470 pass | FAIL | 0.000 | n/a |
| 254 | 3 | 2 | 3 | seeded | 0.80 | 0.833 pass | 0.519 FAIL | YnYY FAIL | 0.2179 pass | FAIL | 0.000 | n/a |
| 255 | 3 | 2 | 3 | seeded | 0.90 | 0.833 pass | 0.349 FAIL | YYYY pass | 0.2052 pass | FAIL | 0.000 | n/a |
| 256 | 3 | 2 | 5 | none | 0.60 | 1.000 FAIL | 0.562 FAIL | nnYn FAIL | 0.4002 pass | FAIL | 0.000 | n/a |
| 257 | 3 | 2 | 5 | none | 0.70 | 0.250 FAIL | 0.951 FAIL | nYYn FAIL | 0.2320 pass | FAIL | 0.000 | n/a |
| 258 | 3 | 2 | 5 | none | 0.80 | 1.000 FAIL | 0.450 FAIL | nnYn FAIL | 0.2052 pass | FAIL | 0.000 | n/a |
| 259 | 3 | 2 | 5 | none | 0.90 | 1.000 FAIL | 0.256 FAIL | nYYn FAIL | 0.1925 pass | FAIL | 0.000 | n/a |
| 260 | 3 | 2 | 5 | seeded | 0.60 | 1.000 FAIL | 0.660 FAIL | YnYY FAIL | 0.3675 pass | FAIL | 0.000 | n/a |
| 261 | 3 | 2 | 5 | seeded | 0.70 | 0.417 FAIL | 0.976 FAIL | YYYY pass | 0.2263 pass | FAIL | 0.000 | n/a |
| 262 | 3 | 2 | 5 | seeded | 0.80 | 1.000 FAIL | 0.572 FAIL | YnYY FAIL | 0.2085 pass | FAIL | 0.000 | n/a |
| 263 | 3 | 2 | 5 | seeded | 0.90 | 1.000 FAIL | 0.422 FAIL | YYYY pass | 0.1989 pass | FAIL | 0.000 | n/a |
| 264 | 3 | 4 | 2 | none | 0.60 | 1.000 FAIL | 0.562 FAIL | nnYn FAIL | 0.3992 pass | FAIL | 0.000 | n/a |
| 265 | 3 | 4 | 2 | none | 0.70 | 0.250 FAIL | 0.949 FAIL | nYYn FAIL | 0.2301 pass | FAIL | 0.000 | n/a |
| 266 | 3 | 4 | 2 | none | 0.80 | 1.000 FAIL | 0.450 FAIL | nnYn FAIL | 0.2045 pass | FAIL | 0.000 | n/a |
| 267 | 3 | 4 | 2 | none | 0.90 | 1.000 FAIL | 0.256 FAIL | nYYn FAIL | 0.1919 pass | FAIL | 0.000 | n/a |
| 268 | 3 | 4 | 2 | seeded | 0.60 | 0.833 pass | 0.617 FAIL | YnYY FAIL | 0.4083 pass | FAIL | 0.000 | n/a |
| 269 | 3 | 4 | 2 | seeded | 0.70 | 0.250 FAIL | 0.965 FAIL | YYYY pass | 0.2463 pass | FAIL | 0.000 | n/a |
| 270 | 3 | 4 | 2 | seeded | 0.80 | 0.833 pass | 0.519 FAIL | YnYY FAIL | 0.2172 pass | FAIL | 0.000 | n/a |
| 271 | 3 | 4 | 2 | seeded | 0.90 | 0.833 pass | 0.349 FAIL | YYYY pass | 0.2046 pass | FAIL | 0.000 | n/a |
| 272 | 3 | 4 | 3 | none | 0.60 | 1.000 FAIL | 0.562 FAIL | nnYn FAIL | 0.3992 pass | FAIL | 0.000 | n/a |
| 273 | 3 | 4 | 3 | none | 0.70 | 0.250 FAIL | 0.949 FAIL | nYYn FAIL | 0.2301 pass | FAIL | 0.000 | n/a |
| 274 | 3 | 4 | 3 | none | 0.80 | 1.000 FAIL | 0.450 FAIL | nnYn FAIL | 0.2045 pass | FAIL | 0.000 | n/a |
| 275 | 3 | 4 | 3 | none | 0.90 | 1.000 FAIL | 0.256 FAIL | nYYn FAIL | 0.1919 pass | FAIL | 0.000 | n/a |
| 276 | 3 | 4 | 3 | seeded | 0.60 | 0.833 pass | 0.617 FAIL | YnYY FAIL | 0.4083 pass | FAIL | 0.000 | n/a |
| 277 | 3 | 4 | 3 | seeded | 0.70 | 0.250 FAIL | 0.965 FAIL | YYYY pass | 0.2463 pass | FAIL | 0.000 | n/a |
| 278 | 3 | 4 | 3 | seeded | 0.80 | 0.833 pass | 0.519 FAIL | YnYY FAIL | 0.2172 pass | FAIL | 0.000 | n/a |
| 279 | 3 | 4 | 3 | seeded | 0.90 | 0.833 pass | 0.349 FAIL | YYYY pass | 0.2046 pass | FAIL | 0.000 | n/a |
| 280 | 3 | 4 | 5 | none | 0.60 | 1.000 FAIL | 0.562 FAIL | nnYn FAIL | 0.3992 pass | FAIL | 0.000 | n/a |
| 281 | 3 | 4 | 5 | none | 0.70 | 0.250 FAIL | 0.949 FAIL | nYYn FAIL | 0.2301 pass | FAIL | 0.000 | n/a |
| 282 | 3 | 4 | 5 | none | 0.80 | 1.000 FAIL | 0.450 FAIL | nnYn FAIL | 0.2045 pass | FAIL | 0.000 | n/a |
| 283 | 3 | 4 | 5 | none | 0.90 | 1.000 FAIL | 0.256 FAIL | nYYn FAIL | 0.1919 pass | FAIL | 0.000 | n/a |
| 284 | 3 | 4 | 5 | seeded | 0.60 | 1.000 FAIL | 0.660 FAIL | YnYY FAIL | 0.3666 pass | FAIL | 0.000 | n/a |
| 285 | 3 | 4 | 5 | seeded | 0.70 | 0.417 FAIL | 0.974 FAIL | YYYY pass | 0.2250 pass | FAIL | 0.000 | n/a |
| 286 | 3 | 4 | 5 | seeded | 0.80 | 1.000 FAIL | 0.572 FAIL | YnYY FAIL | 0.2078 pass | FAIL | 0.000 | n/a |
| 287 | 3 | 4 | 5 | seeded | 0.90 | 1.000 FAIL | 0.422 FAIL | YYYY pass | 0.1982 pass | FAIL | 0.000 | n/a |

### Stage 2 — every row

| Cell | L | T | Price | Obligation | θ_limit | Shock mean | Non-zero ΔH share | Mean survival | Distinct sequences | T2 |
|---|---|---|---|---|---|---|---|---|---|---|
| 79 | 1 | 0 | 2 | seeded | 0.90 | 0.10 | 1.00 pass | 0.758 pass | 11 | pass |
| 79 | 1 | 0 | 2 | seeded | 0.90 | 0.20 | 1.00 pass | 0.458 pass | 13 | pass |
| 79 | 1 | 0 | 2 | seeded | 0.90 | 0.30 | 0.85 pass | 0.496 pass | 13 | pass |
| 87 | 1 | 0 | 3 | seeded | 0.90 | 0.10 | 1.00 pass | 0.758 pass | 11 | pass |
| 87 | 1 | 0 | 3 | seeded | 0.90 | 0.20 | 1.00 pass | 0.458 pass | 13 | pass |
| 87 | 1 | 0 | 3 | seeded | 0.90 | 0.30 | 0.85 pass | 0.496 pass | 13 | pass |
| 103 | 1 | 2 | 2 | seeded | 0.90 | 0.10 | 1.00 pass | 0.758 pass | 11 | pass |
| 103 | 1 | 2 | 2 | seeded | 0.90 | 0.20 | 1.00 pass | 0.458 pass | 13 | pass |
| 103 | 1 | 2 | 2 | seeded | 0.90 | 0.30 | 0.85 pass | 0.496 pass | 13 | pass |
| 111 | 1 | 2 | 3 | seeded | 0.90 | 0.10 | 1.00 pass | 0.758 pass | 11 | pass |
| 111 | 1 | 2 | 3 | seeded | 0.90 | 0.20 | 1.00 pass | 0.458 pass | 13 | pass |
| 111 | 1 | 2 | 3 | seeded | 0.90 | 0.30 | 0.85 pass | 0.496 pass | 13 | pass |
| 127 | 1 | 4 | 2 | seeded | 0.90 | 0.10 | 1.00 pass | 0.758 pass | 11 | pass |
| 127 | 1 | 4 | 2 | seeded | 0.90 | 0.20 | 1.00 pass | 0.458 pass | 13 | pass |
| 127 | 1 | 4 | 2 | seeded | 0.90 | 0.30 | 0.85 pass | 0.496 pass | 13 | pass |
| 135 | 1 | 4 | 3 | seeded | 0.90 | 0.10 | 1.00 pass | 0.758 pass | 11 | pass |
| 135 | 1 | 4 | 3 | seeded | 0.90 | 0.20 | 1.00 pass | 0.458 pass | 13 | pass |
| 135 | 1 | 4 | 3 | seeded | 0.90 | 0.30 | 0.85 pass | 0.496 pass | 13 | pass |

Per-run detail for the six cells (`passing_cells_description.txt`):
- 360 runs: 234 end with 10 survivors, 126 with 1.
- Matched ΔH is non-zero in all 234 ten-survivor runs and in 108 of the 126
  one-survivor runs.
- Survival is not monotone in the drawn magnitude. In cell 79 with shock
  seed 910015 (onset 18): magnitude 0.152 → 1 survivor; 0.305 → 10
  survivors; 0.457 → 1 survivor.

## 7. What I did NOT verify

- Why any of this happens. I did not test causes: why the seeded firms die
  of solvency at tick 40, why nine firms die of compliance in one tick, why
  survival is non-monotone in magnitude, why θ_limit 0.70 behaves unlike
  0.60 and 0.80, or why the thin-capital count has no effect.
- Whether the six cells stay passing under other mechanism, environment or
  init seeds. Only the shock seed varied.
- Grid 1 was not re-run in full; three of its cells and one end-to-end test
  were.
- Any cell outside the declared grid.

## 8. Outside scope, noticed

- **First run crashed in a print statement** before any Stage-1 row was
  printed or saved: SC-5 is `None` when no shaping action was committed, and
  the table printer formatted it as a number. I fixed the printer, added a
  test, and re-ran once with `grid2.json` untouched. The crashed attempt's
  output is kept (`attempt1_crashed_before_any_result_*.txt`). I saw no cell
  result between the attempts.
- **T2's survival leg is satisfied by a mixture.** See §0 item 2. A per-run
  survival criterion would give a different verdict on these cells; that is
  a target change and I did not make it.
- **SC-1 is met by construction in the passing cells.** The two firms seeded
  above θ_Q are the only baseline deaths (§0 item 3).
- After seeing the result I wrote `describe_passing_cells.py` (15 re-runs
  from the declared grid) to list deaths and survivor counts. It changes no
  target or verdict.

## 9. Question for the owner

Not blocking: should a cell count as stressed-but-alive when its shocked
runs are bimodal (10 of 12 or 1 of 12 survivors) and only the mean is in
band? The evidence is in `stage2_full.json` (per-run survivors) and the
table in §0.

## 10. Three things I am least sure about

1. **The midpoint rule for a single firm.** All six passing cells have
   exactly one low-capability firm, placed by my rule at capability 0.30 and
   capital 250. The prompt gave a range, not a rule for one firm. A different
   placement (0.25 or 0.35) was not run.
2. **Whether the six cells are a regime or an artefact of one seed set.**
   Their Stage-2 results are identical across thin-capital count and price,
   and everything except the shock stream is held at one seed.
3. **Reading SC-2 as the reconstructed-margin share**, as in R16. With
   `observation.full` the focus-based share is close to it in the passing
   cells (0.165 against 0.163), but I applied the band only to the former.
