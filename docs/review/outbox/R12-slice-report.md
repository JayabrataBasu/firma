# R12 — SLICE: exploratory end-to-end slice — builder report

EXPLORATORY - DIAGNOSTIC-ONLY - NOT REGISTERED - NOT FOR TUNING.
2026-10-02. No commit, no push. Smallest seed in any executed config: 910001.
Evidence: `docs/review/evidence/R12/` (16 files; `SHA256SUMS` also covers the
script and the test file).

## 0. Outcome in one paragraph

The command exists and runs end to end in 36 s, but **it prints no H1c
verdict**. The prompt says to stop and ask if a per-run `h` and shortfall
cannot be derived from the event log with existing functions. They cannot:
`firma_lab` has no shortfall function at all, and exposes `h` only as a
final-state value or as a fraction. So the slice produces the 200 runs, the
`ΔH_rep` outcome, the cross-check and both known-answer controls, and stops
before the test on the real runs. The question for the owner is in §9.

## 1. Task as understood

One command that runs a 50-seed shocked experiment and its no-shock forks,
computes `ΔH_rep` per run, tests H1c with the existing
`firma_lab.stats.analyse_h1c`, runs a null control (`decision.random`) and a
known-answer control, and prints one verdict with a fixed banner and
caveats. New files only; no edits to `firma_lab`, Rust, configs, ADRs or the
manual; stop rather than substitute for a missing function.

## 2. Files changed

| Path | Lines |
|---|---|
| `scripts/exploratory_e1_slice.py` | +605 (new) |
| `python/tests/test_exploratory_slice.py` | +204 (new) |
| `docs/review/evidence/R12/*`, this report | new |

Nothing else was edited.

## 3. Tests added

`python/tests/test_exploratory_slice.py`, 17 tests, all pass
(`slice_tests.txt`). These are new files, so there is no failing-first run;
no fix was made.

| Test | What it pins |
|---|---|
| `test_repertoire_entropy_matches_manual_example_15_4` | 5× `produce_ordinary`, 2× `deliver`, 1× `hold` → `H = 1.298795`, normalised `0.409724` at `|A_adm| = 9`, tolerance 1e-6 |
| `test_repertoire_entropy_edge_cases` | empty, single-action, uniform windows; order-independence |
| `test_mean_entropy_uses_a_trailing_window_per_firm_tick` | hand-computed trailing-window means at `L_W` 2 and 8 |
| `test_the_tick_a_firm_dies_in_is_not_a_live_firm_tick` | the live firm-tick rule |
| `test_decisions_and_deaths_are_read_from_an_event_frame` | only `decision.*` `selected_action` records count |
| `test_guard_refuses_any_seed_below_910000` (4 cases: 1, 200, 909, 909999) | guard in `check_seeds` and `build_config` |
| `test_guard_refuses_before_anything_is_written` | no directory is created |
| `test_the_designed_seeds_pass_the_guard` | 910001..910050, 50 seeds |
| `test_shocked_config_replaces_the_scheduled_shock_and_nothing_else` | shock parameters as specified; base config not mutated |
| `test_no_shock_fork_differs_from_its_shocked_run_only_by_the_shock_rule` | CRN pairing |
| `test_null_arm_swaps_only_the_decision_plugin` | null control construction |
| `test_known_answer_controls_give_supported_and_not_supported` | planted inverted-U / monotone |
| `test_known_answer_also_holds_on_the_existing_test_stats_fixture` | same, on `test_stats.arm_b_curve` |
| `test_one_seed_end_to_end_and_cross_check_against_firma_lab_metrics` | one real pair (shock seed 910001); cross-check difference < 1e-9 |

## 4. Full suite summary (working tree)

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 262 passed, 0 failed, 1 ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `scripts/lint-architecture.sh` | passed |
| `check_deps.py` | exit 0 |
| `pytest python/tests` | 151 passed, 0 skipped (134 + 17 new) |

## 5. Claims, each with evidence

Raw stdout: `slice_stdout.txt` (stderr empty; `slice_exit.txt`: exit 3).
Per-run table: `per_run_table.csv` (100 rows). Git rev, command, binary hash:
`provenance.txt`.

**Runs.** 200 runs (2 arms × 50 seeds × shocked + fork), 0 failed.

| | satisficing (main) | `decision.random` (null) |
|---|---|---|
| Deaths per shocked run | 1 death in 23 runs, 3 deaths in 27 | 3 deaths in 2 runs, 4 deaths in 48 |
| Deaths in the no-shock fork | 1 in all 50 | 3 in all 50 |
| Distinct action sequences, shocked | 23 | 32 |
| Distinct action sequences, no-shock fork | 1 | 1 |
| `H_rep` of the no-shock fork (`L_W = 8`) | 0.907236 | 1.826504 |
| `ΔH_rep`: mean | −0.180936 | −0.103947 |
| `ΔH_rep`: range | [−0.407790, −0.012097] | [−0.185591, 0.000000] |
| `ΔH_rep`: sample sd, distinct values | 0.139210, 23 | 0.054687, 32 |

- The no-shock fork is the same run for every shock seed (one action
  sequence per arm), so `ΔH_rep` is `H_rep(shocked)` minus a constant.
- `ΔH_rep` is negative in all 50 satisficing runs. No test was applied to it.
- Firm 2 dies of `solvency` at tick 12 in 50 of 50 satisficing shocked runs
  (and is the one death in every satisficing fork).

**`H_rep` is computed correctly.**
- Manual §15.4 reproduced to 1e-6 (test above).
- Cross-check against `firma_lab.metrics.sanity_report`'s
  `sc6_entropy_mean`, on all 100 shocked runs, not five: max |difference|
  2.2e-16 (satisficing) and 6.7e-16 (random). The comparison is made at the
  model's own persisted window (`l_w = 4` in this config), because that is
  what `sc6_entropy_mean` uses; the outcome itself uses `L_W = 8` as the
  prompt directs. Column `crosscheck_diff` in the CSV.

**Known-answer control** (existing `analyse_h1c`, synthetic frames with one
row per seed, 50 seeds, 400 resamples):

| Frame | Verdict | Linear term [95% CI] | Quadratic term [95% CI] |
|---|---|---|---|
| Planted inverted-U | supported (expected) | +0.306 [+0.261, +0.350] | −1.000 [−1.072, −0.930] |
| Planted monotone | not_supported (expected) | +0.306 [+0.261, +0.350] | +0.000 [−0.072, +0.070] |

So the existing analysis accepts the shape of frame this slice would give it
(one run per seed) and returns the right answer on planted data.

**Verdict lines printed by the command:**
- main arm: `NOT COMPUTED - STOPPED BEFORE THE H1c TEST …`
- null control: `NOT COMPUTED - same reason`
- known-answer controls: `both as expected (supported / not_supported)`

**Guard.** Any stream seed below 910000 raises `SliceSeedError` before
anything is written. It calls the Batch 1 helper
(`firma_lab.spec.seeds_in_reserved_range`) and adds the 910000 floor.

## 6. What I did NOT verify or do

- **No H1c test on the real runs**, for either arm (§9).
- The caveat "h and shortfall covary through shock timing only" is printed
  as the task states it; I could not check it without deriving shortfall.
- Whether `ΔH_rep`'s spread reflects changed decisions or changed survival.
  It is a mean over live firm-ticks, and runs split 23/27 between one and
  three deaths, so composition moves it.
- `|A_adm|` per firm-tick and the normalised entropy on real runs (the event
  log does not record the admissible set).
- The script on any machine or Python other than this one (3.14.7).

## 7. Outside scope, noticed

- **The script launches `firma run --model` itself.** `firma_lab.runner` can
  only run `ExperimentSpec` jobs whose four stream seeds are equal
  (`derive_seeds(n) = (n,n,n,n)`, `spec.py:141`) and has no no-shock fork, so
  it cannot express this design. I did not stop on this; the script and its
  banner say so. If that should have been a stop, the runs are still valid
  as diagnostics and the reviewer can discard the executor.
- **Window mismatch.** The prompt fixes `L_W = 8` for the outcome; this
  config's model uses `l_w = 4` for its own window and for `u`. The two are
  both reported in the CSV (`h_rep_shocked_l8`,
  `h_rep_shocked_model_window`).
- `H_rep` counts selected actions, including actions selected under
  NONE-focus inertia that were not executed (audit F03).
- Exit status is 3, not 0, when no verdict is computed.

## 8. Notes on what I changed my mind on

`NOTES_changed_my_mind.md`: the exit status (0 → 3); whether to stop at the
runner gap (no — declared instead); keeping two candidate `h` columns as
description only; excluding the death tick from live firm-ticks to match
`firma-analysis`; marking the covariance caveat "not checked".

## 9. STOP — question for the owner

**What is missing, exactly** (`missing_functions.txt`):
1. **Shortfall.** No function in `firma_lab` or `firma-analysis` computes
   `ς_j = A_j − v_j` from an event log. `Reconstruction` sets the
   aspirations field to zero. The log does carry the ingredients
   (`aspiration_*` adjustments, `realized_capital_growth`; initial
   aspirations are in the config), so a derivation is possible but would be
   new code.
2. **`h` over a run.** Per-tick `h` is computed inside `Reconstruction` but
   reaches Python only as (a) the final-state `h` per firm
   (`metrics.final_margins`) and (b) the fraction of live firm-ticks with
   `h < h_crit` (`sanity_report`).
3. **The predictor itself.** §30.7 says "fit linear and quadratic
   specifications" without naming the regressor (OQ-20 item 3), and no
   document says how a per-firm-per-tick `h` and `ς` become one number per
   run.

**The decisions needed:**
- Which variable is the H1c regressor for this slice: `h`, shortfall, or
  something built from both?
- Which per-run summary: mean over live firm-ticks, value at a fixed tick
  (e.g. just before shock onset), final state, or the fraction below
  `h_crit`?
- For shortfall, which goal (`ς_1` only, as Arm A does, or the attended
  goal's) and whether it may be derived in Python or must come from
  `firma-analysis` through `_native` (the project's rule for metrics).

**Evidence that would settle it, already in the CSV** (described, not
tested): the two `h` summaries that existing functions return both vary
across the 50 satisficing runs — final-state `h` of live firms ranges
−0.300 to 0.103 (24 distinct values); the fraction of live firm-ticks below
`h_crit` ranges 0.581 to 0.887 (24 distinct values). I ran no test on
either, so that choosing between them cannot be influenced by a verdict.

## 10. Three things I am least sure about

1. Whether launching the CLI from the script, instead of stopping at the
   runner gap, is what the reviewer wants.
2. Whether "live firm-tick" should exclude the death tick. I matched
   `firma-analysis`; the manual does not say.
3. Whether printing the two candidate `h` summaries at all is acceptable,
   given the rule against choosing anything from results. They are
   descriptive ranges only.
