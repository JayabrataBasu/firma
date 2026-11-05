# R23 — MANUALDEF: the regime question at the manual defaults — builder report

DIAGNOSTIC-ONLY - NOT REGISTERED - NOT FOR TUNING.
2026-10-02. No commit, no push. Every seed executed by the new script is
>= 910001. Evidence: `docs/review/evidence/R23/` (`SHA256SUMS` also covers
the script and tests).

## 0. Answers

| Prediction | Result |
|---|---|
| P4 (part 1) | **HOLDS.** All 10 struck firms made the violating decision with compliance margin 0.20 and GOAL focus. |
| P5 (part 2) | **HOLDS** by its rule: `cfg_healthy` changes SC-2, SC-3 and SC-6 between `l_w` 4 and 8. `cfg_threaded` changes nothing. |
| P6 (part 3) | **FALSIFIED.** SC-3 pass share is 0.00 in all 40 cells; the compliance constraint binds in 0 of 800 baselines. |

What follows from them:
- **The blind-spot story is confirmed tick by tick** (part 1).
- **The Phase-2 gate verdict did not come from `l_w = 4`.** The bimodal SC-2
  (about 0 healthy, 1.000 threaded) is the same at both window lengths:
  `cfg_threaded` gives identical values for all six conditions at `l_w` 8,
  and `cfg_healthy` stays below 0.05 at `l_w` 4.
- **At the manual defaults no cell passes T1**, so Stage 2 had nothing to
  run. Zero cells are stressed-but-alive.

One constraint on how part 2 was done: the gate configs' own seeds are in the
reserved range 1–200, so I did not execute them. See §2.

## 1. Task as understood

(1) Trace one `l_w = 4` cell and its `l_w = 8` twin tick by tick to confirm or
contradict the blind-spot story. (2) Transcribe the Phase-2 gate configs,
validate the transcription against the gate's printed numbers, and run each
at both window lengths. (3) A 40-cell baseline regime search at `l_w = 8`,
`h_crit = 0.15`, with a shocked second stage for qualifying cells. Three
predictions fixed beforehand.

## 2. Step 0 — the spec, and the reserved seeds

`docs/review/evidence/R23/md_spec.json`, written before any run.
**sha256 `b9ec62b6a30243ead96eeb5bb3adb3627daa15a6b28d2fdb92f8dbe3bedaf03b`.**
It holds every level, the definitions, the three predictions verbatim and
their evaluation rules. The run refuses to start unless the file equals the
script's spec. Unchanged afterwards.

**Reserved seeds.** `cfg_healthy`, `cfg_threaded` and `cfg_wide_search` use
seeds 55/2/3/4, 7/2/3/4 and 909/2/3/4. BUILDER.md and STATE.md forbid running
seeds 1–200, and the prompt also says seeds >= 910001. I did not run them.
Declared before any run, the validation is instead:
1. The original numbers come from the existing tests' own output
   (`existing_test_output.txt`: `cargo test … sc16_gate sc4_wmax_beta_probe
   -- --nocapture`, which the standard suite runs every round).
2. The transcriptions run at seeds 910001..910004.
3. A unit test rebuilds each config mechanically from the Rust source text
   and requires equality with the transcription (seeds aside).
4. Each transcription is also run at seeds 911001..911004; the event logs
   are byte-equal, so these configs do not depend on their seeds.

All four came out exact (§6, part 2).

**Other choices the prompt left open, declared before any run:**
- P4 is evaluated at the decision in the strike tick T (made before that
  tick's enforcement); tick T−1 is printed for information.
- P5: HOLDS if SC-2 or the SC-3 binds vector differs for either config;
  FALSIFIED if all compared values are unchanged for both; NEITHER otherwise.
- P6: HOLDS if any cell at some θ_limit level has SC-3 pass share >= 0.50;
  the share pooled over the level's 8 cells is printed too.
- `cfg_wide_search` is run at (w_max 6, β 1.0), the manual defaults.
- Thin-capital firms are ids 0 and 1, with capital 0.2 × the firm's own
  unrounded draw; θ_Q stays 40 when no firm is seeded with an obligation.
- `u` is not a logged scalar. It is computed here from the logged action
  window with the formula of `margin::u_from_window`. The decision-time `h`
  comes from the plugin's existing diagnostic trace, not the event log.

## 3. Files changed

| Path | Lines |
|---|---|
| `scripts/diag_manual_defaults.py` | new |
| `python/tests/test_diag_manual_defaults.py` | new, 18 tests |
| `docs/review/evidence/R23/*`, this report | new |

Nothing else was edited; existing scripts are imported.

## 4. Tests added

`md_tests.txt`: 18 passed. New files, so no failing-first run.
- Spec: step-0 file equals the declared spec; no overwrite; changed or
  missing spec refused; predictions verbatim.
- Transcription: each of the three configs equals the config rebuilt from
  the Rust source of `tests/tests/sanity.rs` by applying `format!`
  substitution to its raw strings; `with_l_w` changes only the three `l_w`
  values; the SC lines use the Rust print format; reading the captured test
  output.
- Verdict rules: P5 (holds / falsified / neither), P4 (holds on the
  hypothesised story; falsified on SURVIVAL focus, thin margin, or nobody
  struck), P6 (per cell, with the pooled share).
- Trace arithmetic: `u` from the last `l_w` entries; the window before the
  decision and the θ in force.
- Part 3: 40 cells in order; the two population axes; the default cell equals
  the R22 part-2 config; fork differs only by the shock.
- One real baseline.

## 5. Full suite summary

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 269 passed, 0 failed, 1 ignored |
| `pytest python/tests` | 275 passed, 0 skipped (257 + 18 new) |

No Rust change.

## 6. Results

Raw: `md_stdout.txt` (stderr empty, exit 0), `part1_trace.csv` (346 rows),
`part3_stage1_runs.csv` (800 rows), `part3_stage2_runs.csv` (empty),
`summary.json`, `existing_test_output.txt`, `provenance.txt`. 815 runs, 0
failed, 69 s.

### Controls

- Part-3 cell 0, init seed 910001, baseline run twice: byte-equal.
- Part-3 cell 37 (θ_limit 0.90, obligation 1/6, no thin firms, price 3)
  reproduces the R22 `l_w = 8` baselines for init seeds 910001..910020: SC-1,
  SC-2, SC-3 binds, SC-6 and the event-log hash.
- Switching the diagnostic trace on leaves the event log byte-equal at both
  window lengths. On all 346 traced decisions, `h_t < h_crit` agrees with
  SURVIVAL focus.

### Part 1 — the trace (init seed 910001, onset 40, magnitude 0.20)

Full trace for all 12 firms at both window lengths: `md_stdout.txt` and
`part1_trace.csv`. Firm 0 at `l_w = 4`, typical of nine firms (action 1 =
produce_ordinary, 2 = produce_regulated, 3 = acquire_input):

| Tick | Action | Focus | u | θ_limit | Margin θ − u | h_t | Event |
|---|---|---|---|---|---|---|---|
| 39 | 1 | NONE | 0.000 | 0.900 | 0.900 | 0.706 | |
| 40 | 1 | NONE | 0.000 | 0.700 | 0.700 | 0.700 | shock |
| 41 | 3 | GOAL 1 | 0.000 | 0.700 | 0.700 | 0.700 | |
| 42 | 2 | GOAL 1 | 0.000 | 0.700 | 0.700 | 0.700 | |
| 43 | 2 | NONE | 0.250 | 0.700 | 0.450 | 0.450 | |
| 44 | 3 | GOAL 1 | 0.500 | 0.700 | 0.200 | 0.200 | |
| 45 | 2 | GOAL 1 | 0.500 | 0.700 | 0.200 | 0.200 | **strike** |
| 46 | 3 | SURVIVAL | 0.750 | 0.700 | −0.050 | −0.050 | |
| 47 | 2 | GOAL 1 | 0.500 | 0.700 | 0.200 | 0.200 | |
| 48 | 2 | NONE | 0.500 | 0.700 | 0.200 | 0.200 | **died (compliance)** |

- **P4 HOLDS.** Ten firms are struck (nine at tick 45, one at tick 47). For
  every one, the decision in the strike tick was taken at u = 0.500, margin
  0.200 (>= 0.15), focus GOAL 1, and selected produce_regulated. The same
  margin and focus hold at T−1.
- After the strike the firm is in SURVIVAL focus for one tick, drops back to
  u = 0.500, is again at margin 0.200 with non-survival focus, takes another
  regulated step and is struck a second time within T_c = 4: death at tick
  48 (one firm at 50). The blind spot is hit twice.
- The two firms never struck have u = 0 throughout; one of them dies of
  solvency at tick 40, as in its baseline.

At `l_w = 8`, same population and shock (no prediction):
- No firm is struck in ticks 40 to 52 and none dies of compliance.
- Every firm that uses the regulated action enters SURVIVAL focus before
  crossing: decision-time u peaks at 0.625 (margin 0.075 < 0.15) for seven
  firms, which then select a non-regulated action.
- Three firms are at u = 0.750 when the shock lands (margin −0.050 at tick
  40). They are in SURVIVAL focus in that same tick, select acquire_input,
  and end the tick at u = 0.625, so enforcement finds no violation.

The trace does not contradict the story at either window length.

### Part 2 — the gate configs at l_w = 4 and l_w = 8

Validation of the transcription at each config's original `l_w`:

| Config | Against | Result | Seeds 910001.. vs 911001.. |
|---|---|---|---|
| `cfg_healthy` (l_w 8) | the six SC lines printed by `sc16_gate` | identical strings, incl. SC-2 0.005 and SC-4 0/1600 | event log byte-equal |
| `cfg_threaded` (l_w 4) | the six SC lines printed by `sc16_gate` | identical strings, incl. SC-2 1.000 and SC-4 0/3202 | event log byte-equal |
| `cfg_wide_search(6, 1.0)` (l_w 8) | the line printed by `sc4_wmax_beta_probe` | identical: GOAL 0/202, NONE 0/110, SURVIVAL 0/88, total 400 | event log byte-equal |

SC-1 to SC-6 for every combination ("orig." marks the config's own `l_w`):

| Config | l_w | SC-1 | SC-2 attention | SC-2 reconstructed | SC-3 binds (sol, cmp, scp, obl) | SC-4 | SC-5 | SC-6 variance |
|---|---|---|---|---|---|---|---|---|
| `cfg_healthy` | 4 | 1.000 | 0.003 | 0.003 | n, **Y**, n, n | 0/1600 | none | 0.0057 |
| `cfg_healthy` | 8 orig. | 1.000 | 0.005 | 0.005 | n, n, n, n | 0/1600 | none | 0.0116 |
| `cfg_threaded` | 4 orig. | 0.800 | 1.000 | 1.000 | Y, n, Y, Y | 0/3202 | none | 0.0000 |
| `cfg_threaded` | 8 | 0.800 | 1.000 | 1.000 | Y, n, Y, Y | 0/3202 | none | 0.0000 |
| `cfg_wide_search` | 4 | 1.000 | 0.098 | 0.098 | n, **Y**, n, n | 0/400 | none | 0.1570 |
| `cfg_wide_search` | 8 orig. | 1.000 | 0.220 | 0.220 | n, n, n, n | 0/400 | none | 0.0311 |

- **P5 HOLDS.** `cfg_healthy`: SC-2 (0.003 against 0.005), SC-3 (compliance
  binds at `l_w` 4, first at tick 3, and not at 8) and SC-6 (0.0057 against
  0.0116, which moves it across the 0.01 threshold) differ.
- **What does not change:** `cfg_threaded` gives identical values for all
  six conditions at both window lengths. `cfg_healthy`'s SC-2 stays under
  0.05 at both. SC-1, SC-4 and SC-5 are unchanged in all three configs. The
  gate's SC-2 statement (bimodal, never in [0.05, 0.25] for these two
  configs) holds at `l_w` 4 and at `l_w` 8.
- **What changes:** in the two configs that run at θ_limit 0.90, compliance
  binds at `l_w` 4 and never at `l_w` 8.
- `cfg_wide_search`, not part of P5: SC-2 is 0.098 at `l_w` 4 and 0.220 at
  `l_w` 8, both inside [0.05, 0.25]. This is a two-firm probe config, not a
  gate config.

### Part 3, Stage 1 — 40 cells × 20 populations, baselines, l_w = 8

Pass shares over 20 init seeds. SC-2 is the reconstructed share, as in
R16–R22.

| Cell | θ_limit | Obligation | Thin | Price | SC-1 | SC-2 | SC-3 | SC-6 | T1 | SC-2 mean [min, max] | Binds of 20 (sol, cmp, scp, obl) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0.50 | 0 | 0 | 2 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.550 [0.467, 0.650] | 0, 0, 12, 0 |
| 1 | 0.50 | 0 | 0 | 3 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.543 [0.467, 0.650] | 0, 0, 12, 0 |
| 2 | 0.50 | 0 | 1/6 | 2 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.551 [0.469, 0.650] | 0, 0, 12, 0 |
| 3 | 0.50 | 0 | 1/6 | 3 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.543 [0.469, 0.650] | 0, 0, 12, 0 |
| 4 | 0.50 | 1/6 | 0 | 2 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.588 [0.485, 0.662] | 18, 0, 12, 18 |
| 5 | 0.50 | 1/6 | 0 | 3 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.584 [0.471, 0.669] | 18, 0, 12, 18 |
| 6 | 0.50 | 1/6 | 1/6 | 2 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.589 [0.485, 0.662] | 18, 0, 12, 18 |
| 7 | 0.50 | 1/6 | 1/6 | 3 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.583 [0.471, 0.669] | 18, 0, 12, 18 |
| 8 | 0.60 | 0 | 0 | 2 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.399 [0.301, 0.547] | 0, 0, 12, 0 |
| 9 | 0.60 | 0 | 0 | 3 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.372 [0.274, 0.519] | 0, 0, 12, 0 |
| 10 | 0.60 | 0 | 1/6 | 2 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.400 [0.300, 0.547] | 0, 0, 12, 0 |
| 11 | 0.60 | 0 | 1/6 | 3 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.371 [0.272, 0.519] | 0, 0, 12, 0 |
| 12 | 0.60 | 1/6 | 0 | 2 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.449 [0.301, 0.552] | 18, 0, 12, 18 |
| 13 | 0.60 | 1/6 | 0 | 3 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.427 [0.274, 0.543] | 18, 0, 12, 18 |
| 14 | 0.60 | 1/6 | 1/6 | 2 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.450 [0.301, 0.552] | 18, 0, 12, 18 |
| 15 | 0.60 | 1/6 | 1/6 | 3 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.427 [0.274, 0.543] | 18, 0, 12, 18 |
| 16 | 0.70 | 0 | 0 | 2 | 0.00 | 0.30 | 0.00 | 1.00 | 0.00 | 0.301 [0.180, 0.487] | 0, 0, 12, 0 |
| 17 | 0.70 | 0 | 0 | 3 | 0.00 | 0.50 | 0.00 | 1.00 | 0.00 | 0.263 [0.149, 0.435] | 0, 0, 12, 0 |
| 18 | 0.70 | 0 | 1/6 | 2 | 0.00 | 0.30 | 0.00 | 1.00 | 0.00 | 0.304 [0.183, 0.487] | 0, 0, 12, 0 |
| 19 | 0.70 | 0 | 1/6 | 3 | 0.00 | 0.50 | 0.00 | 1.00 | 0.00 | 0.263 [0.149, 0.435] | 0, 0, 12, 0 |
| 20 | 0.70 | 1/6 | 0 | 2 | 0.55 | 0.20 | 0.00 | 1.00 | 0.00 | 0.360 [0.191, 0.487] | 18, 0, 12, 18 |
| 21 | 0.70 | 1/6 | 0 | 3 | 0.55 | 0.30 | 0.00 | 1.00 | 0.00 | 0.327 [0.149, 0.465] | 18, 0, 12, 18 |
| 22 | 0.70 | 1/6 | 1/6 | 2 | 0.55 | 0.25 | 0.00 | 1.00 | 0.00 | 0.363 [0.201, 0.487] | 18, 0, 12, 18 |
| 23 | 0.70 | 1/6 | 1/6 | 3 | 0.55 | 0.30 | 0.00 | 1.00 | 0.00 | 0.327 [0.149, 0.465] | 18, 0, 12, 18 |
| 24 | 0.80 | 0 | 0 | 2 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.371 [0.259, 0.526] | 0, 0, 12, 0 |
| 25 | 0.80 | 0 | 0 | 3 | 0.00 | 0.30 | 0.00 | 1.00 | 0.00 | 0.330 [0.224, 0.485] | 0, 0, 12, 0 |
| 26 | 0.80 | 0 | 1/6 | 2 | 0.00 | 0.00 | 0.00 | 1.00 | 0.00 | 0.373 [0.261, 0.533] | 0, 0, 12, 0 |
| 27 | 0.80 | 0 | 1/6 | 3 | 0.00 | 0.30 | 0.00 | 1.00 | 0.00 | 0.330 [0.224, 0.485] | 0, 0, 12, 0 |
| 28 | 0.80 | 1/6 | 0 | 2 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.425 [0.285, 0.536] | 18, 0, 12, 18 |
| 29 | 0.80 | 1/6 | 0 | 3 | 0.55 | 0.05 | 0.00 | 1.00 | 0.00 | 0.389 [0.224, 0.512] | 18, 0, 12, 18 |
| 30 | 0.80 | 1/6 | 1/6 | 2 | 0.55 | 0.00 | 0.00 | 1.00 | 0.00 | 0.427 [0.285, 0.536] | 18, 0, 12, 18 |
| 31 | 0.80 | 1/6 | 1/6 | 3 | 0.55 | 0.10 | 0.00 | 1.00 | 0.00 | 0.389 [0.224, 0.512] | 18, 0, 12, 18 |
| 32 | 0.90 | 0 | 0 | 2 | 0.00 | 0.70 | 0.00 | 1.00 | 0.00 | 0.187 [0.049, 0.381] | 0, 0, 12, 0 |
| 33 | 0.90 | 0 | 0 | 3 | 0.00 | 0.45 | 0.00 | 1.00 | 0.00 | 0.163 [0.028, 0.366] | 0, 0, 12, 0 |
| 34 | 0.90 | 0 | 1/6 | 2 | 0.00 | 0.75 | 0.00 | 1.00 | 0.00 | 0.187 [0.051, 0.380] | 0, 0, 12, 0 |
| 35 | 0.90 | 0 | 1/6 | 3 | 0.00 | 0.45 | 0.00 | 1.00 | 0.00 | 0.165 [0.033, 0.370] | 0, 0, 12, 0 |
| 36 | 0.90 | 1/6 | 0 | 2 | 0.55 | 0.40 | 0.00 | 1.00 | 0.00 | 0.257 [0.071, 0.405] | 18, 0, 12, 18 |
| 37 | 0.90 | 1/6 | 0 | 3 | 0.55 | 0.45 | 0.00 | 1.00 | 0.00 | 0.235 [0.037, 0.393] | 18, 0, 12, 18 |
| 38 | 0.90 | 1/6 | 1/6 | 2 | 0.55 | 0.40 | 0.00 | 1.00 | 0.00 | 0.258 [0.070, 0.405] | 18, 0, 12, 18 |
| 39 | 0.90 | 1/6 | 1/6 | 3 | 0.55 | 0.45 | 0.00 | 1.00 | 0.00 | 0.238 [0.041, 0.393] | 18, 0, 12, 18 |

- **P6 FALSIFIED.** SC-3 pass share by θ_limit level, best cell / pooled:
  0.00 / 0.00 at each of 0.50, 0.60, 0.70, 0.80, 0.90.
- **Compliance binds in 0 of 800 baselines**, at every θ_limit from 0.50 to
  0.90. Scope binds in 12 of 20 populations in every cell; solvency and
  obligation in 18 of 20 where obligations are seeded and in none otherwise.
- Cells with a pass share >= 0.50: SC-1 20 of 40 (highest 0.55); SC-2 4 of
  40 (highest 0.75); SC-3 0; SC-6 40; **T1 0** (0 of 800 baselines pass T1).
- SC-1 depends only on the obligation axis: without seeding all 12 firms
  survive every baseline; with it the share is 0.55.
- SC-2 is in band in 0 of 160 baselines at θ_limit 0.50 and 0.60, 53 at
  0.70, 15 at 0.80, 81 at 0.90.
- The thin-capital axis changes no pass share by more than 0.05, and no
  thin-capital firm dies in any baseline without obligation seeding.

### Part 3, Stage 2

No cell has a T1 pass share >= 0.50, so Stage 2 ran nothing. Zero cells meet
the R20 Q4 criteria.

## 7. What I did NOT verify

- Why compliance never binds at `l_w = 8`. It is consistent with the same
  arithmetic as the blind spot (a step of 0.125 is smaller than h_crit, so
  SURVIVAL focus starts at least one step before u reaches θ, as the part-1
  trace shows for one population), but I did not test that as a cause across
  the grid.
- The gate configs at their original seeds. The byte-equal logs across two
  seed sets say the result would be the same; I did not run it.
- Part 1 on more than one population seed, magnitude or onset.
- `h_crit` other than 0.15, `l_w` other than 4 and 8, or any shocked run in
  part 3.
- Mechanism and environment seeds other than 910001 / 910002 in part 3.

## 8. Outside scope, noticed

- **SC-3 looks structurally out of reach for `decision.satisficing` at the
  manual defaults** in these populations: the firm avoids the compliance
  boundary whenever it can see it coming, and it could only be struck at
  `l_w = 4` because it could not. That is the Phase-2 gate finding again,
  from a different direction.
- **Thin capital does not kill.** Firms starting with about 20 in capital
  all survive 120 ticks.
- An earlier round's ADR-0057 evidence (`docs/adr/evidence/adr-0057/mkcfg.py`)
  ran `cfg_wide_search` at its original seeds 909/2/3/4. Environment, shock
  and init seeds there are inside 1–200.
- `sc16_gate`'s printed "mean H_rep = -0.000 bits" for `cfg_threaded` is a
  negative zero.

## 9. Questions for the owner

1. Is the reserved range meant to cover the small seeds in the existing
   Rust test configs when they are transcribed and run outside `cargo test`?
   I assumed yes and worked around it. The evidence that it made no
   difference: the three byte-equal seed comparisons in part 2.
2. Not blocking: with compliance unable to bind at `l_w = 8` and the cliff
   at `l_w = 4`, is SC-3 for compliance still a target for a satisficing
   baseline, or only under shock? The 800 baselines and the part-1 trace are
   the evidence either way.

## 10. Notes on what I changed my mind on

`NOTES_changed_my_mind.md`: not running the reserved seeds and validating the
transcription three other ways; which tick P4 is about; computing `u` from
the logged window and taking `h_t` from the trace; a three-outcome rule for
P5; P6 per cell; the thin-capital rule. Nothing crashed; one run.

## 11. Three things I am least sure about

1. **Whether P5 "holding" should be read as "the gate verdict came from
   l_w = 4".** By the rule I declared it holds, because `cfg_healthy`'s SC-3
   and SC-6 move. But the gate's stated finding, the bimodal SC-2, is the
   same at both window lengths. I report both and do not reconcile them.
2. **The part-1 trace is one population.** P4 is about that cell. The R22
   table (10 populations, same strike and death ticks) suggests it
   generalises; the tick-by-tick margins were only read for seed 910001.
3. **My handling of the reserved seeds.** It follows the hard stop, and the
   checks were exact, but it is not literally "reproduce at the original
   seeds", which is what the prompt asked for.
