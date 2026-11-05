# R22 — BLIND: the 'blind spot' explanation of the compliance cliff — builder report

DIAGNOSTIC-ONLY - NOT REGISTERED - NOT FOR TUNING.
2026-10-02. No commit, no push. Smallest seed in any executed config: 910001.
Evidence: `docs/review/evidence/R22/` (`SHA256SUMS` also covers the script
and tests).

## 0. Answers

| Item | Result |
|---|---|
| P1 (l_w = 4, h_crit = 0.15) | **HOLDS.** Compliance-death share 1.00 at m = 0.175, 0.200, 0.225; 0.00 at all six outside levels. |
| P2 (l_w = 4, h_crit = 0.30) | **HOLDS.** Share 0.00 at all 11 magnitudes. |
| P3 (l_w = 8, h_crit = 0.15) | **HOLDS.** Share 0.00 at all 11 magnitudes. |
| Part 2 (R20 measurements at l_w = 8) | Collapse rate 0.00 at every spread level (was 0.57 / 0.51 / 0.29). The R20 criteria still **FAIL** at all three levels: T1 pass share is 0.00, because compliance never binds in any baseline. |
| Part 3 (resource shock, `resource.constant`) | The price now stays shifted. It still kills no additional firm in 150 runs and changes behaviour in at most 1 of 50. |

None of the three predictions is falsified. Two things qualify that:
- The edge levels behave asymmetrically: no compliance death at m = 0.150,
  compliance deaths in all 10 runs at m = 0.250. I had declared the edges
  not counted; with them counted as inside, P1 would not hold.
- Part 1 fixes the onset at 40. In part 2, with onsets drawn from 10 to 80,
  4 to 6 of 100 runs at l_w = 8 still have a compliance death.

## 1. Task as understood

Test three predictions of the blind-spot hypothesis on a grid of fixed shock
magnitudes × `l_w` × `h_crit`; repeat the R20 Q2/Q4 measurements at the
manual's `l_w = 8`; repeat R21's resource cells with a price process that
does not revert. Predictions fixed beforehand; no reinterpretation.

## 2. Step 0 — the spec

`docs/review/evidence/R22/blind_spec.json`, written before any run.
**sha256 `60a4c8ae0767a7cedc06305bcac275bc0ade85908904740b437bb84dec1ef25b`.**
It contains all levels, the three predictions verbatim, the evaluation rules
and the falsification statements. The run refuses to start unless the file
equals the script's spec. Unchanged afterwards.

**Choices the prompt left open, declared before any run:**
- **P1 scoring.** "Allow the grid resolution" is read as: m = 0.150 and
  0.250 (θ_new exactly 0.75 and 0.65) are reported but not counted. P1 holds
  iff the share is ≥ 0.80 at each of 0.175, 0.200, 0.225 and ≤ 0.10 at each
  of the six outside levels. The strict variant, with edges counted as
  inside, is printed for information.
- Fixed onset and magnitude use `shock.stochastic` with onset 40..40 and
  sd 0, not `shock.scheduled`.
- A "compliance strike" is a first-violation record
  (`compliance_last_violation_tick` written by `constraint.enforce`); both
  readings of "deaths at the tick of first compliance strike" are reported.
- Part 3 uses the relative aspiration reading, keeps R21's drawn onset
  (10..80), and replaces `resource.patchy` in the fork too.

Parameter keys, read from the code:
- `h_crit`: `SatisficingParams::h_crit`, config key `h_crit` under
  `decision.satisficing` (`firma-plugin-decision/src/lib.rs:812-813`,
  default 0.15).
- `l_w`: `decision.satisficing` (`lib.rs:820-823`), `constraint.action_window`
  (`phase_rules.rs:96-100`), `constraint.enforce` (`phase_rules.rs:215-217`).
  The generated configs carry the same value in all three (printed in
  `blind_stdout.txt`, and a test checks no other rule has an `l_w`).
- `resource.constant` takes no parameters and emits nothing (ADR-0038 §1;
  `firma-plugin-resource/src/lib.rs:57, :90-92, :223-225`).

## 3. Files changed

| Path | Lines |
|---|---|
| `scripts/diag_blindspot.py` | new |
| `python/tests/test_diag_blindspot.py` | new, 18 tests |
| `docs/review/evidence/R22/*`, this report | new |

Nothing else was edited; existing scripts are imported.

## 4. Tests added

`blind_tests.txt`: 18 passed. New files, so no failing-first run.
- Spec: step-0 file equals the declared spec; no overwrite; changed or
  missing spec refused; grid and verbatim predictions present.
- Configs: `l_w` in all three rules and `h_crit` in the decision rule (four
  combinations); fixed onset and magnitude; fork differs only by the shock;
  with l_w 4 and h_crit 0.15 only the explicit `h_crit` key differs from the
  R20 config; part 2 and part 3 configs.
- Arithmetic: first strike at or after onset; cell shares and means; the
  three verdict rules at their boundaries (P1 unaffected by the edges; one
  bad level falsifies); price reconstruction.
- One real part-1 population end to end.

## 5. Full suite summary

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 269 passed, 0 failed, 1 ignored |
| `pytest python/tests` | 257 passed, 0 skipped (239 + 18 new) |

No Rust change.

## 6. Results

Raw: `blind_stdout.txt` (stderr empty, exit 0), `part1_runs.csv` (440 rows),
`part2_runs.csv` (300), `part3_runs.csv` (150), `summary.json`,
`part2_compliance_runs.txt`, `provenance.txt`. 1,387 runs, 0 failed, 162 s.

### Controls

- Part-1 cell (m 0.200, l_w 4, h_crit 0.15, init seed 910001) run twice:
  byte-equal.
- R20 medium init seed 910001, built by this script with l_w = 4 and
  h_crit = 0.15 set explicitly, reproduces its R20 row: survivors, death
  records, T1, SC-2, matched ΔH, and the shocked event-log hash.

### Part 1 — share of 10 runs with any compliance death

Onset 40, magnitude fixed, θ_new = 0.90 − m.

| m | θ_new | l_w 4, h_crit 0.15 | l_w 4, h_crit 0.30 | l_w 8, h_crit 0.15 | l_w 8, h_crit 0.30 |
|---|---|---|---|---|---|
| 0.100 | 0.800 | 0.00 | 0.00 | 0.00 | 0.00 |
| 0.125 | 0.775 | 0.00 | 0.00 | 0.00 | 0.00 |
| 0.150 | 0.750 | 0.00 | 0.00 | 0.00 | 0.00 |
| 0.175 | 0.725 | **1.00** | 0.00 | 0.00 | 0.00 |
| 0.200 | 0.700 | **1.00** | 0.00 | 0.00 | 0.00 |
| 0.225 | 0.675 | **1.00** | 0.00 | 0.00 | 0.00 |
| 0.250 | 0.650 | **1.00** | 0.00 | 0.00 | 0.00 |
| 0.275 | 0.625 | 0.00 | 0.00 | 0.00 | 0.00 |
| 0.300 | 0.600 | 0.00 | 0.00 | 0.00 | 0.00 |
| 0.325 | 0.575 | 0.00 | 0.00 | 0.00 | 0.00 |
| 0.350 | 0.550 | 0.00 | 0.00 | 0.00 | 0.00 |

Verdicts by the declared rules:
- **P1 HOLDS.** Interior levels 1.00, 1.00, 1.00; outside levels all 0.00.
  Edges, not counted: 0.00 at m = 0.150, 1.00 at m = 0.250. The strict
  variant (edges inside) does not hold.
- **P2 HOLDS.** 0.00 at every magnitude.
- **P3 HOLDS.** 0.00 at every magnitude.
- No prediction was made for (l_w 8, h_crit 0.30): 0.00 at every magnitude.

Other per-cell quantities:

| Cells | Survivors (count of runs) | Runs with a strike at or after tick 40 | Firms struck at first strike, mean | Deaths in the first-strike tick, mean | Compliance deaths in the first compliance-death tick, mean | Non-zero matched ΔH |
|---|---|---|---|---|---|---|
| l_w 4, h_crit 0.15, m 0.175–0.250 | 0: 3, 1: 3, 2: 3, 4: 1 | 1.00 | 7.2 | 0.0 | 7.2 | 1.00 |
| l_w 8, h_crit 0.15, m 0.300–0.350 | same as fork | 0.90 | 2.67 | 1.78 | none | 1.00 |
| every other cell | same as fork | 0.00 | — | — | none | 1.00, except below |

- Fork survivors are 8: 4, 9: 2, 11: 2, 12: 2 in all four (l_w, h_crit)
  settings; no fork has a compliance death. In every cell without compliance
  deaths, each run ends with its fork's survivors.
- In the four cascade cells every population is struck at tick 45 and its
  first compliance deaths fall in tick 48, with 5 to 11 firms dying together.
  In 32 of the 40 cascade runs further compliance deaths follow in later
  ticks.
  Survivors, strike tick and death tick are identical across m = 0.175 to
  0.250 for every population.
- At l_w 8, h_crit 0.15 and m ≥ 0.300, 9 of 10 runs have first violations at
  tick 40 (1 to 4 firms) but no compliance death. The 1.78 deaths in that
  tick are the baseline solvency deaths, which also happen at tick 40.
- Non-zero matched ΔH is 1.00 in every cell except: 0.00 at m = 0.100 for
  both h_crit 0.30 settings, and 0.90 at m = 0.100 and 0.125 for l_w 8,
  h_crit 0.15.

### Part 2 — R20 Q2/Q4 at l_w = 8, h_crit = 0.15 (pooled, 100 init seeds)

"l_w 4" is copied from R21 part B (same populations, same shock draws).

| Metric | Low: l_w 8 / l_w 4 | Medium: l_w 8 / l_w 4 | High: l_w 8 / l_w 4 |
|---|---|---|---|
| Collapse rate | 0.00 / 0.57 | 0.00 / 0.51 | 0.00 / 0.29 |
| ≥ 5 deaths in one tick | 0.05 / 0.57 | 0.01 / 0.57 | 0.00 / 0.50 |
| Runs with any compliance death | 6 / 57 | 4 / 57 | 5 / 57 |
| Mean survival, shocked | 0.828 / 0.376 | 0.840 / 0.426 | 0.843 / 0.500 |
| Mean survival, baseline | 0.847 / 0.847 | 0.847 / 0.847 | 0.848 / 0.848 |
| Non-zero matched ΔH | 0.93 / 1.00 | 0.93 / 1.00 | 0.93 / 1.00 |
| SC-1 pass share | 0.55 / 0.55 | 0.55 / 0.55 | 0.55 / 0.55 |
| SC-2 pass share | 0.90 / 0.58 | 0.61 / 0.55 | 0.30 / 0.29 |
| SC-3 pass share | 0.00 / 0.01 | 0.00 / 0.52 | 0.00 / 0.83 |
| SC-6 pass share | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| T1 pass share | 0.00 / 0.01 | 0.00 / 0.13 | 0.00 / 0.09 |
| R20 criteria | FAIL / FAIL | FAIL / FAIL | FAIL / FAIL |

Survivors of 12, shocked runs at l_w = 8 (count of runs):
- Low: 5: 1, 6: 2, 7: 3, 8: 9, 9: 18, 10: 27, 11: 28, 12: 12.
- Medium: 7: 2, 8: 8, 9: 22, 10: 28, 11: 28, 12: 12.
- High: 7: 2, 8: 8, 9: 19, 10: 30, 11: 29, 12: 12.

Baseline sanity values at l_w = 8 (100 baselines per level):

| | Low | Medium | High |
|---|---|---|---|
| SC-1 survival, mean [min, max] | 0.847 [0.583, 1.000] | 0.847 [0.583, 1.000] | 0.848 [0.583, 1.000] |
| SC-2 share below h_crit, mean [min, max] | 0.116 [0.034, 0.295] | 0.207 [0.031, 0.446] | 0.344 [0.054, 0.729] |
| SC-2 too low / too high | 8 / 2 | 4 / 35 | 0 / 70 |
| SC-3 binds: solvency, compliance, scope, obligation (of 100) | 87, **0**, 1, 87 | 87, **0**, 58, 87 | 87, **0**, 95, 87 |
| SC-6 variance, mean [min, max] | 0.153 [0.103, 0.210] | 0.181 [0.142, 0.224] | 0.191 [0.151, 0.225] |

- The R20 criteria fail at l_w = 8 for a different reason than at l_w = 4:
  collapse (≤ 0.10) and non-zero ΔH (≥ 0.50) are now met; T1 is not, because
  **the compliance constraint binds in 0 of 300 baselines** (it bound in all
  of them at l_w = 4), so SC-3 fails everywhere. Per-set verdicts are the
  same (A and B both FAIL at each level).
- The shock now leaves 94 to 96 of 100 runs with exactly their baseline
  survivors.
- The residual compliance deaths (`part2_compliance_runs.txt`): 6, 4 and 5
  runs, at most 6, 3 and 1 compliance deaths in a run. Their drawn onsets
  are 10 to 34 and magnitudes 0.211 to 0.285; 23 to 26 runs without any
  compliance death have magnitudes in the same range, so this is not a
  magnitude band.

### Part 3 — resource shock with `resource.constant` (medium, l_w = 8, 50 seeds)

| Cell | Collapse | ≥ 5 deaths in one tick | Non-zero matched ΔH | Mean survival (baseline 0.843) | Runs whose survivors differ from baseline |
|---|---|---|---|---|---|
| +1 (input price 2 → 3) | 0.00 | 0.02 | 0.02 | 0.843 | 0 of 50 |
| +2 (2 → 4) | 0.00 | 0.02 | 0.02 | 0.843 | 0 of 50 |
| +4 (2 → 6) | 0.00 | 0.02 | 0.00 | 0.843 | 0 of 50 |

Survivors in all three cells: 7: 1, 8: 7, 9: 8, 10: 9, 11: 19, 12: 6, which
is the baseline distribution. All deaths are the 94 baseline solvency
deaths. The 0.02 is one baseline that already has five same-tick deaths.

**The price stays shifted.** Input price, init seed 910001, onset 52, ticks
49 to 65:
- `resource.constant`, no shock: 2 throughout.
- `resource.constant`, +4: 2, 2, 2, then 6 through to the last tick.
- `resource.patchy`, +4 (R21's rule set): 4, 3, 4, 7, 6, 5, 5, 4, 3, 3, 3,
  1, 1, 1, 1, 2, 3; last tick 1.

So with a shock that persists, tripling the input price still changes
selected actions in at most one run of 50 and kills no one.

## 7. What I did NOT verify

- The mechanism itself. The predictions are about where compliance deaths
  occur, and they hold; I did not trace a firm's `u`, margin and focus tick
  by tick to confirm the "one more regulated step" story.
- Part 1 at any onset other than 40, or any θ_limit other than 0.90.
- Why m = 0.250 (θ_new 0.65) cascades and m = 0.150 (θ_new 0.75) does not.
- Why compliance never binds in a baseline at l_w = 8, or what causes the
  residual compliance deaths in part 2.
- Why a persistent input-price shock has no effect.
- Mechanism and environment seeds other than 910001 / 910002.

## 8. Outside scope, noticed

- **Removing the cliff does not produce a stressed regime.** At l_w = 8 the
  regulatory shock leaves about 95% of runs with their baseline survivors,
  and SC-3 cannot pass because compliance does not bind without a shock.
- **SC-1 is unchanged by l_w** (pass share 0.55 at both): baseline deaths are
  the obligation-seeded firms, as in R20.
- The smoke config's `l_w = 4` differs from the manual's default 8 (§16.1);
  every regime result from R12 to R21 was obtained at 4.
- The hypothesis text in the prompt gives the band as open at both ends; the
  data put θ_new = 0.65 inside and 0.75 outside.

## 9. Questions for the owner

None blocking.

## 10. Notes on what I changed my mind on

`NOTES_changed_my_mind.md`: how to score P1's edges (declared before any
run; the strict variant does not hold); reporting both readings of "deaths
at the first strike tick"; `shock.stochastic` with degenerate ranges;
replacing the resource rule in the fork too. Nothing crashed; one run.

## 11. Three things I am least sure about

1. **The P1 scoring rule.** I exempted the two edge levels before running.
   That was my reading of "allow the grid resolution"; under a strict
   reading P1 fails at m = 0.150. The numbers for both are above.
2. **P3 at other onsets.** At onset 40 there is no compliance death at
   l_w = 8 in 110 runs, but part 2's drawn onsets give 4 to 6 per 100. The
   prediction as written holds; whether "no band at l_w = 8" holds generally
   was not tested on a grid of onsets.
3. **Whether part 3 says anything about the resource channel in general.**
   These populations start with capital around 100 and input stock 40; I did
   not check how much input they buy after the shock, so "no effect" may
   reflect firms that rarely purchase, not an insensitive model.
