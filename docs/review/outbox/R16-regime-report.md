# R16 — REGIME: is there a stressed-but-alive cell? — builder report

DIAGNOSTIC-ONLY - REGIME SEARCH - NOT REGISTERED - NOT FOR TUNING.
2026-10-02. No commit, no push. Smallest seed in any executed config: 910001.
Evidence: `docs/review/evidence/R16/` (`SHA256SUMS` also covers the script and
the test file).

## 0. Answer

**0 of 72 cells are stressed-but-alive under the declared targets.**

| | Cells |
|---|---|
| Pass T1 only | 0 |
| Pass T2 only | 54 |
| Pass both | 0 |
| Pass neither | 18 |

Which targets fail, and how often:

| Target | Cells failing (of 72) | How |
|---|---|---|
| SC-1 survival in [0.60, 0.90] | 72 | survival is 1.000 in every baseline: no firm dies without a shock |
| SC-2 share of firm-ticks with h < h_crit in [0.05, 0.25] | 72 | too high in 54 (0.415 to 0.687), too low in 18 (0.010 to 0.011) |
| SC-3 all four constraints bind | 72 | obligation never binds; solvency never binds |
| SC-6 entropy variance | 0 | |
| T2: non-zero matched ΔH in ≥ 50% of runs | 12 | 0.10 in all twelve |
| T2: mean survival in [0.40, 0.90] | 12 | too high in 6 (0.938 to 0.942), too low in 6 (0.150) |

Per the prompt, no cell passes, so I stopped; the grid was not extended.

## 1. Task as understood

Declare a grid of at most 72 configurations and the targets before any run;
for each cell run a no-shock baseline (SC-1, SC-2, SC-3, SC-6) and 20 shocked
runs with their no-shock forks (matched firm-tick ΔH, R14 definition); report
every cell in declared order; say how many pass T1, T2, both, neither. No
hypothesis statistic. New files only.

## 2. Step 0 — the grid

`docs/review/evidence/R16/grid.json`, written before any run.
**sha256 `1371026b859b2da07e87ad5186fed47c9063ec96d6cf214d866670577701963a`.**
The run command refuses to start unless the file is byte-identical to the
grid in the script, and a second `--write-grid` to the same path is refused.

Levels I declared (the prompt left these to me):

| Axis | Levels | Spacing rule |
|---|---|---|
| n | 8, 12 | given |
| Capital (min–max) | 50–100, 100–200, 150–300 | min = 0.5, 1.0, 1.5 × s_L; max = 2 × min; spans 0.5× to 3× s_L |
| Capability (min–max) | 0.30–0.60, 0.45–0.90 | max = 2 × min, same rule as capital |
| Shock magnitude mean | 0.10, 0.20, 0.30 | given; sd = mean / 3 |
| θ_limit baseline | 0.55, 0.90 | given |

2 × 3 × 2 × 3 × 2 = 72 cells, so no level was dropped. Cell order is the
product of the axes in that order, last axis fastest.

**Assumptions (fixed items).** `decision.satisficing` with the smoke config's
parameters (`l_w = 4`); `observation.full` in place of `observation.delayed`;
`shock.stochastic` as in R12 (regulatory / θ_limit, onset uniform 10..80,
instant, permanent); horizon 120; mechanism / environment / init seeds
910001 / 910002 / 910004; shock seeds 910001..910020.

**Assumptions I added** (the prompt did not fix them; all are in `grid.json`):
- Every per-firm field other than capital and capability is copied from firm
  0 of the smoke config: input stock 40, legitimacy 1.0,
  `aspiration_capital_growth` 8.0, no obligation. The smoke config's four
  firms differ from each other in these fields; the grid's firms do not.
- Kept from the smoke config: θ_cap 0.40, θ_Q 100, input price 2, output
  price 3, `resource.patchy` (baseline 2, reversion 0.25, sigma 0.8, floor
  1), `constraint.enforce` (t_c 4, p_c 30, delta_lambda 0.15, p_q 20, l_w 4),
  `aspiration_update` alpha 0.10, the six market actions and
  `resolve_lagged`. Everything else is the plugins' defaults.
- SC-2 is `sc2_reconstructed_h_below_crit` (the share of live firm-ticks with
  h < h_crit), with the [0.05, 0.25] bounds applied in the script. SC-3 and
  SC-6 pass/fail are read from `firma_analysis`. The analysis crate's own
  `sc2_pass` uses the focus-based share instead; it is in the CSV for
  information.

## 3. Files changed

| Path | Lines |
|---|---|
| `scripts/regime_search.py` | new |
| `python/tests/test_regime_search.py` | new, 21 tests |
| `docs/review/evidence/R16/*`, this report | new |

Nothing else was edited. `scripts/exploratory_e1_slice.py` is unchanged (the
R14 checksums still verify).

## 4. Tests added

`regime_tests.txt`: 21 passed. New files, so no failing-first run.
- Grid: 72 cells in product order; the step-0 file equals the script's grid;
  a run is refused when the file differs or is absent; step 0 never
  overwrites.
- Population and configs: linear capital and capability; fixed items; the
  no-shock config differs only by the shock rule; a reserved seed is refused.
- Targets: a cell meeting everything passes; each T1 leg alone fails T1
  (6 cases); bounds inclusive; each T2 leg alone fails T2; no shocked runs
  fails T2.
- One real cell end to end with two shock seeds.

## 5. Full suite summary

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 269 passed, 0 failed, 1 ignored |
| `pytest python/tests` | 180 passed, 0 skipped (159 + 21 new) |

No Rust change, so no numerics comparison.

## 6. Results

Raw: `regime_stdout.txt` (stderr empty, exit 0), `cells.csv`,
`cells_full.json` (per-run rows for all 1,440 shocked runs),
`control_b.json`, `provenance.txt`. 2,952 runs, 0 failed, 216 s.

### Controls

- **(a) Determinism.** Cell 0, baseline and shocked (shock seed 910001), each
  run twice: event logs byte-equal (`15c2140b…` twice, `4dbe359f…` twice).
  In addition, all 1,440 forks were byte-identical to their cell's baseline.
- **(b) Known failure.** The R12 slice config through the same code: SC-1
  0.750 pass; SC-2 0.591 fail; SC-3 fail (compliance and obligation never
  bind); SC-6 pass; T1 fail. Non-zero matched ΔH in 0.40 of 20 runs: fail;
  mean survival 0.450 pass; T2 fail. Not stressed-but-alive, as expected.

### Every cell, in the declared order

SC-3 column: Y = that constraint bound at least once, in the order solvency,
compliance, scope, obligation.

| Cell | n | Capital | Capability | Shock mean | θ_limit | SC-1 | SC-2 | SC-3 binds (sol, cmp, scp, obl) | SC-6 var | T1 | Non-zero ΔH share | Mean survival | Distinct sequences | T2 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 8 | 50–100 | 0.30–0.60 | 0.10 | 0.55 | 1.000 FAIL | 0.673 FAIL | nnYn FAIL | 0.3794 pass | FAIL | 0.10 FAIL | 0.456 pass | 12 | FAIL |
| 1 | 8 | 50–100 | 0.30–0.60 | 0.10 | 0.90 | 1.000 FAIL | 0.426 FAIL | nYYn FAIL | 0.2000 pass | FAIL | 1.00 pass | 0.938 FAIL | 11 | FAIL |
| 2 | 8 | 50–100 | 0.30–0.60 | 0.20 | 0.55 | 1.000 FAIL | 0.673 FAIL | nnYn FAIL | 0.3794 pass | FAIL | 0.75 pass | 0.787 pass | 13 | pass |
| 3 | 8 | 50–100 | 0.30–0.60 | 0.20 | 0.90 | 1.000 FAIL | 0.426 FAIL | nYYn FAIL | 0.2000 pass | FAIL | 1.00 pass | 0.688 pass | 13 | pass |
| 4 | 8 | 50–100 | 0.30–0.60 | 0.30 | 0.55 | 1.000 FAIL | 0.673 FAIL | nnYn FAIL | 0.3794 pass | FAIL | 0.80 pass | 0.706 pass | 13 | pass |
| 5 | 8 | 50–100 | 0.30–0.60 | 0.30 | 0.90 | 1.000 FAIL | 0.426 FAIL | nYYn FAIL | 0.2000 pass | FAIL | 0.85 pass | 0.719 pass | 13 | pass |
| 6 | 8 | 50–100 | 0.45–0.90 | 0.10 | 0.55 | 1.000 FAIL | 0.415 FAIL | nnnn FAIL | 0.4441 pass | FAIL | 0.10 FAIL | 0.150 FAIL | 12 | FAIL |
| 7 | 8 | 50–100 | 0.45–0.90 | 0.10 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1838 pass | FAIL | 1.00 pass | 0.900 pass | 12 | pass |
| 8 | 8 | 50–100 | 0.45–0.90 | 0.20 | 0.55 | 1.000 FAIL | 0.415 FAIL | nnnn FAIL | 0.4441 pass | FAIL | 0.75 pass | 0.694 pass | 13 | pass |
| 9 | 8 | 50–100 | 0.45–0.90 | 0.20 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1838 pass | FAIL | 1.00 pass | 0.500 pass | 13 | pass |
| 10 | 8 | 50–100 | 0.45–0.90 | 0.30 | 0.55 | 1.000 FAIL | 0.415 FAIL | nnnn FAIL | 0.4441 pass | FAIL | 0.75 pass | 0.550 pass | 13 | pass |
| 11 | 8 | 50–100 | 0.45–0.90 | 0.30 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1838 pass | FAIL | 0.90 pass | 0.550 pass | 13 | pass |
| 12 | 8 | 100–200 | 0.30–0.60 | 0.10 | 0.55 | 1.000 FAIL | 0.676 FAIL | nnYn FAIL | 0.3787 pass | FAIL | 0.10 FAIL | 0.463 pass | 12 | FAIL |
| 13 | 8 | 100–200 | 0.30–0.60 | 0.10 | 0.90 | 1.000 FAIL | 0.427 FAIL | nYYn FAIL | 0.1992 pass | FAIL | 1.00 pass | 0.938 FAIL | 11 | FAIL |
| 14 | 8 | 100–200 | 0.30–0.60 | 0.20 | 0.55 | 1.000 FAIL | 0.676 FAIL | nnYn FAIL | 0.3787 pass | FAIL | 0.75 pass | 0.787 pass | 13 | pass |
| 15 | 8 | 100–200 | 0.30–0.60 | 0.20 | 0.90 | 1.000 FAIL | 0.427 FAIL | nYYn FAIL | 0.1992 pass | FAIL | 1.00 pass | 0.688 pass | 13 | pass |
| 16 | 8 | 100–200 | 0.30–0.60 | 0.30 | 0.55 | 1.000 FAIL | 0.676 FAIL | nnYn FAIL | 0.3787 pass | FAIL | 0.80 pass | 0.694 pass | 13 | pass |
| 17 | 8 | 100–200 | 0.30–0.60 | 0.30 | 0.90 | 1.000 FAIL | 0.427 FAIL | nYYn FAIL | 0.1992 pass | FAIL | 0.90 pass | 0.719 pass | 13 | pass |
| 18 | 8 | 100–200 | 0.45–0.90 | 0.10 | 0.55 | 1.000 FAIL | 0.418 FAIL | nnnn FAIL | 0.4438 pass | FAIL | 0.10 FAIL | 0.150 FAIL | 11 | FAIL |
| 19 | 8 | 100–200 | 0.45–0.90 | 0.10 | 0.90 | 1.000 FAIL | 0.011 FAIL | nYnn FAIL | 0.1834 pass | FAIL | 1.00 pass | 0.900 pass | 11 | pass |
| 20 | 8 | 100–200 | 0.45–0.90 | 0.20 | 0.55 | 1.000 FAIL | 0.418 FAIL | nnnn FAIL | 0.4438 pass | FAIL | 0.75 pass | 0.688 pass | 13 | pass |
| 21 | 8 | 100–200 | 0.45–0.90 | 0.20 | 0.90 | 1.000 FAIL | 0.011 FAIL | nYnn FAIL | 0.1834 pass | FAIL | 1.00 pass | 0.500 pass | 13 | pass |
| 22 | 8 | 100–200 | 0.45–0.90 | 0.30 | 0.55 | 1.000 FAIL | 0.418 FAIL | nnnn FAIL | 0.4438 pass | FAIL | 0.80 pass | 0.531 pass | 13 | pass |
| 23 | 8 | 100–200 | 0.45–0.90 | 0.30 | 0.90 | 1.000 FAIL | 0.011 FAIL | nYnn FAIL | 0.1834 pass | FAIL | 0.90 pass | 0.550 pass | 13 | pass |
| 24 | 8 | 150–300 | 0.30–0.60 | 0.10 | 0.55 | 1.000 FAIL | 0.676 FAIL | nnYn FAIL | 0.3787 pass | FAIL | 0.10 FAIL | 0.463 pass | 12 | FAIL |
| 25 | 8 | 150–300 | 0.30–0.60 | 0.10 | 0.90 | 1.000 FAIL | 0.427 FAIL | nYYn FAIL | 0.1992 pass | FAIL | 1.00 pass | 0.938 FAIL | 11 | FAIL |
| 26 | 8 | 150–300 | 0.30–0.60 | 0.20 | 0.55 | 1.000 FAIL | 0.676 FAIL | nnYn FAIL | 0.3787 pass | FAIL | 0.75 pass | 0.787 pass | 13 | pass |
| 27 | 8 | 150–300 | 0.30–0.60 | 0.20 | 0.90 | 1.000 FAIL | 0.427 FAIL | nYYn FAIL | 0.1992 pass | FAIL | 1.00 pass | 0.688 pass | 13 | pass |
| 28 | 8 | 150–300 | 0.30–0.60 | 0.30 | 0.55 | 1.000 FAIL | 0.676 FAIL | nnYn FAIL | 0.3787 pass | FAIL | 0.80 pass | 0.694 pass | 13 | pass |
| 29 | 8 | 150–300 | 0.30–0.60 | 0.30 | 0.90 | 1.000 FAIL | 0.427 FAIL | nYYn FAIL | 0.1992 pass | FAIL | 0.90 pass | 0.719 pass | 13 | pass |
| 30 | 8 | 150–300 | 0.45–0.90 | 0.10 | 0.55 | 1.000 FAIL | 0.418 FAIL | nnnn FAIL | 0.4438 pass | FAIL | 0.10 FAIL | 0.150 FAIL | 11 | FAIL |
| 31 | 8 | 150–300 | 0.45–0.90 | 0.10 | 0.90 | 1.000 FAIL | 0.011 FAIL | nYnn FAIL | 0.1834 pass | FAIL | 1.00 pass | 0.900 pass | 11 | pass |
| 32 | 8 | 150–300 | 0.45–0.90 | 0.20 | 0.55 | 1.000 FAIL | 0.418 FAIL | nnnn FAIL | 0.4438 pass | FAIL | 0.75 pass | 0.688 pass | 13 | pass |
| 33 | 8 | 150–300 | 0.45–0.90 | 0.20 | 0.90 | 1.000 FAIL | 0.011 FAIL | nYnn FAIL | 0.1834 pass | FAIL | 1.00 pass | 0.500 pass | 13 | pass |
| 34 | 8 | 150–300 | 0.45–0.90 | 0.30 | 0.55 | 1.000 FAIL | 0.418 FAIL | nnnn FAIL | 0.4438 pass | FAIL | 0.80 pass | 0.531 pass | 13 | pass |
| 35 | 8 | 150–300 | 0.45–0.90 | 0.30 | 0.90 | 1.000 FAIL | 0.011 FAIL | nYnn FAIL | 0.1834 pass | FAIL | 0.90 pass | 0.550 pass | 13 | pass |
| 36 | 12 | 50–100 | 0.30–0.60 | 0.10 | 0.55 | 1.000 FAIL | 0.685 FAIL | nnYn FAIL | 0.3699 pass | FAIL | 0.10 FAIL | 0.496 pass | 12 | FAIL |
| 37 | 12 | 50–100 | 0.30–0.60 | 0.10 | 0.90 | 1.000 FAIL | 0.452 FAIL | nYYn FAIL | 0.2009 pass | FAIL | 1.00 pass | 0.942 FAIL | 11 | FAIL |
| 38 | 12 | 50–100 | 0.30–0.60 | 0.20 | 0.55 | 1.000 FAIL | 0.685 FAIL | nnYn FAIL | 0.3699 pass | FAIL | 0.75 pass | 0.808 pass | 13 | pass |
| 39 | 12 | 50–100 | 0.30–0.60 | 0.20 | 0.90 | 1.000 FAIL | 0.452 FAIL | nYYn FAIL | 0.2009 pass | FAIL | 1.00 pass | 0.708 pass | 13 | pass |
| 40 | 12 | 50–100 | 0.30–0.60 | 0.30 | 0.55 | 1.000 FAIL | 0.685 FAIL | nnYn FAIL | 0.3699 pass | FAIL | 0.80 pass | 0.729 pass | 13 | pass |
| 41 | 12 | 50–100 | 0.30–0.60 | 0.30 | 0.90 | 1.000 FAIL | 0.452 FAIL | nYYn FAIL | 0.2009 pass | FAIL | 0.85 pass | 0.738 pass | 13 | pass |
| 42 | 12 | 50–100 | 0.45–0.90 | 0.10 | 0.55 | 1.000 FAIL | 0.415 FAIL | nnnn FAIL | 0.4436 pass | FAIL | 0.10 FAIL | 0.150 FAIL | 12 | FAIL |
| 43 | 12 | 50–100 | 0.45–0.90 | 0.10 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1836 pass | FAIL | 1.00 pass | 0.900 pass | 12 | pass |
| 44 | 12 | 50–100 | 0.45–0.90 | 0.20 | 0.55 | 1.000 FAIL | 0.415 FAIL | nnnn FAIL | 0.4436 pass | FAIL | 0.75 pass | 0.696 pass | 13 | pass |
| 45 | 12 | 50–100 | 0.45–0.90 | 0.20 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1836 pass | FAIL | 1.00 pass | 0.500 pass | 13 | pass |
| 46 | 12 | 50–100 | 0.45–0.90 | 0.30 | 0.55 | 1.000 FAIL | 0.415 FAIL | nnnn FAIL | 0.4436 pass | FAIL | 0.75 pass | 0.550 pass | 13 | pass |
| 47 | 12 | 50–100 | 0.45–0.90 | 0.30 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1836 pass | FAIL | 0.90 pass | 0.550 pass | 13 | pass |
| 48 | 12 | 100–200 | 0.30–0.60 | 0.10 | 0.55 | 1.000 FAIL | 0.687 FAIL | nnYn FAIL | 0.3695 pass | FAIL | 0.10 FAIL | 0.500 pass | 12 | FAIL |
| 49 | 12 | 100–200 | 0.30–0.60 | 0.10 | 0.90 | 1.000 FAIL | 0.453 FAIL | nYYn FAIL | 0.2004 pass | FAIL | 1.00 pass | 0.942 FAIL | 11 | FAIL |
| 50 | 12 | 100–200 | 0.30–0.60 | 0.20 | 0.55 | 1.000 FAIL | 0.687 FAIL | nnYn FAIL | 0.3695 pass | FAIL | 0.75 pass | 0.808 pass | 13 | pass |
| 51 | 12 | 100–200 | 0.30–0.60 | 0.20 | 0.90 | 1.000 FAIL | 0.453 FAIL | nYYn FAIL | 0.2004 pass | FAIL | 1.00 pass | 0.708 pass | 13 | pass |
| 52 | 12 | 100–200 | 0.30–0.60 | 0.30 | 0.55 | 1.000 FAIL | 0.687 FAIL | nnYn FAIL | 0.3695 pass | FAIL | 0.80 pass | 0.721 pass | 13 | pass |
| 53 | 12 | 100–200 | 0.30–0.60 | 0.30 | 0.90 | 1.000 FAIL | 0.453 FAIL | nYYn FAIL | 0.2004 pass | FAIL | 0.90 pass | 0.738 pass | 13 | pass |
| 54 | 12 | 100–200 | 0.45–0.90 | 0.10 | 0.55 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4434 pass | FAIL | 0.10 FAIL | 0.150 FAIL | 11 | FAIL |
| 55 | 12 | 100–200 | 0.45–0.90 | 0.10 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1833 pass | FAIL | 1.00 pass | 0.900 pass | 11 | pass |
| 56 | 12 | 100–200 | 0.45–0.90 | 0.20 | 0.55 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4434 pass | FAIL | 0.75 pass | 0.692 pass | 13 | pass |
| 57 | 12 | 100–200 | 0.45–0.90 | 0.20 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1833 pass | FAIL | 1.00 pass | 0.500 pass | 13 | pass |
| 58 | 12 | 100–200 | 0.45–0.90 | 0.30 | 0.55 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4434 pass | FAIL | 0.80 pass | 0.537 pass | 13 | pass |
| 59 | 12 | 100–200 | 0.45–0.90 | 0.30 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1833 pass | FAIL | 0.90 pass | 0.550 pass | 13 | pass |
| 60 | 12 | 150–300 | 0.30–0.60 | 0.10 | 0.55 | 1.000 FAIL | 0.687 FAIL | nnYn FAIL | 0.3695 pass | FAIL | 0.10 FAIL | 0.500 pass | 12 | FAIL |
| 61 | 12 | 150–300 | 0.30–0.60 | 0.10 | 0.90 | 1.000 FAIL | 0.453 FAIL | nYYn FAIL | 0.2004 pass | FAIL | 1.00 pass | 0.942 FAIL | 11 | FAIL |
| 62 | 12 | 150–300 | 0.30–0.60 | 0.20 | 0.55 | 1.000 FAIL | 0.687 FAIL | nnYn FAIL | 0.3695 pass | FAIL | 0.75 pass | 0.808 pass | 13 | pass |
| 63 | 12 | 150–300 | 0.30–0.60 | 0.20 | 0.90 | 1.000 FAIL | 0.453 FAIL | nYYn FAIL | 0.2004 pass | FAIL | 1.00 pass | 0.708 pass | 13 | pass |
| 64 | 12 | 150–300 | 0.30–0.60 | 0.30 | 0.55 | 1.000 FAIL | 0.687 FAIL | nnYn FAIL | 0.3695 pass | FAIL | 0.80 pass | 0.721 pass | 13 | pass |
| 65 | 12 | 150–300 | 0.30–0.60 | 0.30 | 0.90 | 1.000 FAIL | 0.453 FAIL | nYYn FAIL | 0.2004 pass | FAIL | 0.90 pass | 0.738 pass | 13 | pass |
| 66 | 12 | 150–300 | 0.45–0.90 | 0.10 | 0.55 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4434 pass | FAIL | 0.10 FAIL | 0.150 FAIL | 11 | FAIL |
| 67 | 12 | 150–300 | 0.45–0.90 | 0.10 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1833 pass | FAIL | 1.00 pass | 0.900 pass | 11 | pass |
| 68 | 12 | 150–300 | 0.45–0.90 | 0.20 | 0.55 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4434 pass | FAIL | 0.75 pass | 0.692 pass | 13 | pass |
| 69 | 12 | 150–300 | 0.45–0.90 | 0.20 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1833 pass | FAIL | 1.00 pass | 0.500 pass | 13 | pass |
| 70 | 12 | 150–300 | 0.45–0.90 | 0.30 | 0.55 | 1.000 FAIL | 0.417 FAIL | nnnn FAIL | 0.4434 pass | FAIL | 0.80 pass | 0.537 pass | 13 | pass |
| 71 | 12 | 150–300 | 0.45–0.90 | 0.30 | 0.90 | 1.000 FAIL | 0.010 FAIL | nYnn FAIL | 0.1833 pass | FAIL | 0.90 pass | 0.550 pass | 13 | pass |

### What the table shows, described only

- **Nobody dies without a shock.** Baseline survival is 1.000 in all 72
  cells, above the SC-1 band.
- **SC-2 has no cell inside its band.** It takes 13 distinct values in two
  groups: 0.010–0.011 (high capability with θ_limit 0.90) and 0.415–0.687
  (everything else).
- **SC-3 patterns:** scope only (18 cells), compliance and scope (18),
  compliance only (18), nothing binds (18). Solvency and obligation bind in
  no cell. No shaping action is selected in any baseline (SC-4 share 0.0).
- **Capital and n barely matter.** The three capital levels give the same
  pass/fail pattern and nearly the same numbers; so do n = 8 and n = 12.
- **The 12 cells failing T2's non-zero leg** are all shock mean 0.10 with
  θ_limit 0.55 (non-zero share 0.10).
- With the obligation leg of SC-3 left out (information only), T1 still
  holds in 0 cells.

## 7. What I did NOT verify

- Any configuration outside the declared grid.
- Why no firm dies in any baseline, or why solvency never binds. I report it;
  I did not investigate.
- Whether an obligation can arise at all in this rule set. From the code, the
  only source is an applied `contract` shaping effect, and no shaping action
  was selected in any baseline.
- The search on another machine or Python version.

## 8. Outside scope, noticed

- **The reused slice function crashes on a run with no deaths.**
  `sl.measure_run` → `decisions_and_deaths` raises `KeyError: 'cause'` when
  the event frame has no `agent_died` record. R12/R14 never hit it because
  firm 2 always died. I may not edit that file here, so `regime_search.py`
  composes the same imported functions (`decisions_and_deaths`,
  `firm_tick_entropies`, `rust_firm_ticks`, `matched_summary`,
  `action_sha`, `drawn_shock`) and adds the two empty columns first. The
  fix belongs in the slice script.
- **Order of events after step 0.** After `grid.json` was written, the
  end-to-end test ran one cell-0 baseline and two shock seeds; that exposed
  the crash above; I changed the script (grid bytes unchanged, enforced by
  the hash check) and then ran the full search once. The grid and targets
  were not touched after any result was seen.
- **Before step 0 I corrected one sentence of my own draft grid note.** It
  claimed obligation "can never bind"; reading the code showed I had not
  established that, so the note written to `grid.json` says it is not
  established. No run had happened.
- Mean shocked survival is exactly 0.900 in 6 cells (7, 19, 31, 43, 55, 67),
  which passes that leg only because the upper bound is inclusive.

## 9. Question for the owner

None blocks this report. One choice should be confirmed: the prompt says
"SC-2 share of firm-ticks with h < h_crit", which I took as the
reconstructed-h share, not the focus-based share that `sc2_pass` uses. Both
are in `cells.csv` (`sc2_h_below_crit`, `info_sc2_survival_attention`); the
focus-based share ranges 0.010 to 0.684 and lies inside [0.05, 0.25] in 0 of
72 cells, so the SC-2 verdict is the same under either reading.

## 10. Three things I am least sure about

1. **The population template.** Copying firm 0 for everything except capital
   and capability makes the firms much more alike than the smoke config's
   four, and gives every firm the same aspiration and no obligation. That is
   my reading of "population template rule"; it may be why no firm dies and
   why two constraints never bind.
2. **The capability levels.** The prompt gave no numbers. I used
   max = 2 × min with min 0.30 and 0.45; θ_cap is 0.40, so the two levels
   straddle it differently, and SC-3's scope column tracks this axis exactly.
3. **Capital levels have almost no effect.** Either capital in the range
   50–300 really is irrelevant to these firms at this horizon, or the
   0.5×–3× s_L range I was given is far from where solvency matters (the
   smoke config's firms start at 260–600). I did not test which.
