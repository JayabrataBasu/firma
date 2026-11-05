# R14 — SLICE-2: matched outcome, Rust-derived shortfall, H1c fit — builder report

EXPLORATORY - DIAGNOSTIC-ONLY - NOT REGISTERED - NOT FOR TUNING.
2026-10-02. No commit, no push. Smallest seed in any executed config: 910001.
Evidence: `docs/review/evidence/R14/` (24 entries in `SHA256SUMS`, which also
covers the script, the test file and the new Rust module).

## 0. Outcome in one paragraph

The command now prints two verdict lines. For the satisficing arm the
existing H1c analysis returned **not_supported in this exploratory run**
(quadratic term −5.71, 95% CI [−28.59, +2.97]; the linear model predicts
better out of sample). For the `decision.random` null the analysis **refused
the data: the matched outcome is exactly 0 in all 50 runs**, so that verdict
is "uninterpretable". Both known-answer controls gave the expected answer.
The matched outcome removes most of what R12/R13 saw: in the 27 runs with
three deaths, the matched ΔH is exactly 0.

## 1. Task as understood

Extend the R12 slice with the owner's three definitions: (1) shortfall per
firm-tick = `max_j ς_j` as `Focus::attend` uses it, from the Rust analysis
path; (2) outcome = ΔH per firm over firm-ticks alive in both branches, then
averaged over firms, with the old unmatched value kept beside it; (3) H1c
regressor = mean `h` over matched live firm-ticks, covariate = mean
max-shortfall over the same. Evidence first on where the shortfall formula
lives; stop if reuse needs code moved into `firma-domain`.

## 2. Evidence first — where the shortfall formula lives

Recorded before any edit: `evidence_first_shortfall_location.txt`.

- The formula is a **private method of the decision plugin**:
  `Satisficing::shortfalls`, `firma-plugin-decision/src/lib.rs:914-933`
  (`ς_j = A_j − v_j`; `v_1` = stored `realized_capital_growth`, absent ⇒ 0;
  `v_2 = c`; `v_3 = −q`; an absent aspiration ⇒ `ς_j = 0`). The reduction to
  one number is `Focus::attend`, `:175-191`.
- `firma-domain` contains no shortfall code (only the pin keys).
- `firma-analysis` depends on core, domain, config, io only.

**Reuse would require moving the method into `firma-domain`. I did not do
that** (the prompt says to stop on that path). I took the other path the
prompt allows: derive it from persisted state inside `firma-analysis` and
prove equivalence with the plugin's own values.

**How it is derived.** `Reconstruction` now tracks, per firm, the three
aspirations (seeded from the config's `reals`, then each logged
`AdjustAgentReal` on an `aspiration_*` key; absent until first written) and
`realized_capital_growth` (each logged `SetAgentInt`; absent ⇒ 0). It applies
the same four lines of arithmetic. The value is read at the moment the fold
reaches a firm's `decision.*` record for a tick, i.e. the state that
decision saw.

**Equivalence** (`shortfall_equivalence.txt`, `float_difference_cause.txt`),
against the `shortfalls` array the plugin writes to its diagnostic trace:

| Check | Result |
|---|---|
| Runs compared | 100 (50 shock seeds × shocked + fork, satisficing) |
| Firm-ticks compared | 33,972 |
| Firm-ticks where any component differs at all | 12,438 |
| Largest absolute difference | 3.55e-15 (relative 1.2e-14) |
| Firm-ticks where the sign test `max ς > 0` differs | 0 |
| Firm-ticks where the arg-max goal differs | 0 |
| Same arithmetic replayed with an exact float parser, 3 runs (799 decisions) | bit-equal on every one |

So the formula and the persisted-state derivation are exactly equivalent;
the Rust reconstruction is **not bit-equal**, only equal to about 1e-14. The
cause is float parsing of the event log on the Rust side: `serde_json` is
used without its `float_roundtrip` feature (`Cargo.toml:63`), so each logged
delta can be read back one unit in the last place off, and aspirations
accumulate hundreds of deltas. I did not change `Cargo.toml` (not
authorised). The same imprecision applies to every other real the
reconstruction accumulates from the log.

**Consequence to decide later:** there are now two copies of the shortfall
arithmetic in Rust (plugin and analysis crate). The options are (a) keep both
with this equivalence check, or (b) move the function into `firma-domain`
and call it from both (ADR-0026 precedent) — which needs its own
authorisation.

## 3. Files changed

`git diff --numstat` against HEAD; † = cumulative with H-1/H-2.

| Path | + / − |
|---|---|
| `crates/firma-analysis/src/firm_ticks.rs` | +239 (new) |
| `crates/firma-analysis/src/reconstruct.rs` † | 383 / 16 |
| `crates/firma-analysis/src/sanity.rs` † | 169 / 2 (this round: one function made `pub(crate)`) |
| `crates/firma-analysis/src/lib.rs` | 2 / 1 |
| `crates/firma-py/src/lib.rs` † | 112 / 13 (this round: `firm_ticks` exposed) |
| `scripts/exploratory_e1_slice.py` | 765 lines (untracked; was 605) |
| `python/tests/test_exploratory_slice.py` | 309 lines (untracked; was 204) |
| `docs/review/evidence/R14/*`, this report | new |

Not touched: plugins, `firma-domain`, `firma_lab` modules, configs, ADRs, the
manual, `Cargo.toml`. The git-ignored Python extension was rebuilt in the
repo's `.venv` (`maturin develop`) to expose `firm_ticks`.

## 4. Tests added

New features, so there is no failing-first run. All pass
(`firma_analysis_unit_tests.txt`, `slice_tests.txt`).

Rust, `firma-analysis` (7 new, 18 in the crate):
- `shortfalls_match_the_manual_12_1_table_on_the_15_3_numbers` —
  `A = (5, 0.6, −20)`, `v_1 = 2`, `c = 0.60`, `q = 3` → `ς = (3, 0, −17)`;
  then §15.3's update `A_1 = 4.7`.
- `an_absent_aspiration_gives_zero_shortfall_until_it_is_first_written`
- `max_shortfall_is_the_value_attend_reads`
- `one_record_per_live_firm_tick_and_the_death_tick_is_excluded`
- `shortfall_is_the_value_at_the_decision_not_after_the_record_phase`
- `h_is_the_reconstructed_margin_at_the_end_of_the_tick`
- `a_corrupt_record_is_refused`

Python, `test_exploratory_slice.py` (8 new, 25 in the file):
- `test_a_firm_dying_earlier_in_the_shocked_branch_drops_out_of_matching` —
  hand-built: firm 1 dies in tick 2 of the shocked branch; its fork ticks 2–3
  never enter; firms are weighted equally (0.5, not the pooled 3.5/6).
- `test_matching_also_drops_ticks_missing_from_the_fork_and_handles_no_overlap`
- `test_identical_branches_give_exactly_zero_matched_delta`
- `test_a_matched_firm_tick_without_a_rust_record_is_an_error`
- `test_firm_tick_entropies_are_keyed_by_agent_and_tick_and_skip_the_death_tick`
- `test_an_arm_with_no_outcome_variation_is_uninterpretable_not_a_verdict`
- `test_known_answer_frames_have_the_real_frames_columns`
- `test_rust_shortfall_equals_the_plugins_own_decision_time_value` — one
  traced run; tolerance 1e-9.

## 5. Full suite summary (working tree)

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 269 passed, 0 failed, 1 ignored (262 + 7 new) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `scripts/lint-architecture.sh` | passed |
| `check_deps.py` | exit 0 |
| `pytest python/tests` | 159 passed, 0 skipped (151 + 8 new) |

## 6. Results of the run

Raw stdout `slice2_stdout.txt` (stderr empty; `slice2_exit.txt`: exit 2).
Per-run table `per_run_table.csv` (100 rows). Command, git rev, hashes:
`provenance.txt`. 200 runs, 0 failed, 396 s (10,000 resamples per fit).

### (a) Matched vs unmatched ΔH_rep by death count

| Arm | Deaths | Runs | Unmatched mean [min, max] | Matched mean [min, max] |
|---|---|---|---|---|
| satisficing | 1 | 23 | −0.061 [−0.200, −0.012] | −0.047 [−0.155, −0.009] |
| satisficing | 3 | 27 | −0.283 [−0.408, −0.159] | **0.000 [0.000, 0.000]** |
| random | 3 | 2 | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] |
| random | 4 | 48 | −0.108 [−0.186, −0.049] | **0.000 [0.000, 0.000]** |

The unmatched column reproduces R13's table. Overall satisficing matched
mean −0.0218, range [−0.1554, 0], sample sd 0.0458.

- In the 27 three-death runs the matched ΔH is exactly 0. Checked directly on
  shock seed 910001: firms 0 and 1 differ from the fork only in the tick they
  die in (not a live firm-tick), and firm 3 never differs.
- The matched outcome takes **6 distinct values** over 50 satisficing runs:
  0 (27 runs), −0.0191 (14), −0.1554 (5), −0.0121 (2), −0.0113 (1),
  −0.0094 (1).
- Under `decision.random` the matched ΔH is exactly 0 in all 50 runs: on
  every firm-tick alive in both branches the firm selects the same action.

### (b) Matched firm-ticks per run

| Arm | Min | Median | Max | Fork live firm-ticks | Firms with a matched tick |
|---|---|---|---|---|---|
| satisficing | 218 | 291 | 372 | 372 | 4 in every run |
| random | 82 | 139 | 227 | 227 | 4 in every run |

Matched firm-ticks equal the shocked run's live firm-ticks in every run (a
firm alive in the shocked branch is always alive in the fork). Firm 2
contributes 12 ticks (0–11) in every satisficing run.

### (c) Fit — `ΔH_rep_matched ~ h + h² + mean max-shortfall + (1 | seed)`

| | Linear term (h) [95% CI] | Quadratic term (h²) [95% CI] | CV MSE linear / quadratic | Verdict |
|---|---|---|---|---|
| satisficing, 50 runs | +1.830 [+0.128, +4.037] | −5.713 [−28.586, +2.973] | 0.001665 / 0.001824 | **not_supported** |
| random, 50 runs | — | — | — | **uninterpretable**: "column(s) with no variation: ['delta_h_rep_matched']" |
| known-answer, planted inverted-U | +0.275 [+0.231, +0.314] | −1.058 [−1.145, −0.973] | — | supported (expected) |
| known-answer, planted monotone | +0.275 [+0.231, +0.314] | −0.058 [−0.145, +0.027] | — | not_supported (expected) |

10,000 resamples each, 0 failed to fit. Regressor range, satisficing:
mean `h` −0.056 to 0.085 (50 distinct values); covariate 2.00 to 3.27
(23 distinct values).

Verdict lines as printed:
- `VERDICT (satisficing arm): H1c not_supported in this exploratory run`
- `VERDICT (null control, decision.random): UNINTERPRETABLE (column(s) with no variation: ['delta_h_rep_matched'])`
- `VERDICT (known-answer controls): both as expected (supported / not_supported)`

No claim is made about H1c beyond this exploratory run.

### (d) Caveats printed by the script

Switching rule at `h_crit` (`decision/lib.rs:175`); firm 2 dies of solvency
at tick 12 in 50 of 50 satisficing shocked runs; one variance source (the
shock stream; one fork for all seeds); `h` is the true margin at tick end,
not the perceived one; `H_rep` counts selected, not executed, actions; one
exploratory run on a non-registered config.

### Numerics unchanged

`hash_equivalence_output.txt`: HEAD (`2820610` export) vs the working tree,
event logs byte-equal on **200 of 200** slice configs (2 arms × 50 seeds ×
shocked + fork). Sensitivity control: shocked vs fork on one binary is
DIFFERENT. The engine binary did not even rebuild: `firma-cli` does not
depend on `firma-analysis`.

## 7. What I did NOT verify

- Anything about H1c outside this config, population, horizon and seed set.
- `|A_adm|` and normalised entropy on real runs (not in the event log).
- The slice on another machine or Python version.
- Whether the regressor's variation (−0.056 to 0.085) is mostly a function
  of shock onset; I did not analyse it.

## 8. Outside scope, noticed

- **Float parsing** (§2): reconstruction from the log is accurate to about
  1e-14, not bit-exact, because of `serde_json`'s default float parser.
- **The null control cannot fail.** With the mechanism seed held fixed,
  `decision.random` makes the same draws in both branches, and a
  `theta_limit` shock does not change the admissible set, so matched ΔH is
  identically 0. A null that varied the mechanism seed would behave
  differently; that is a design change, not made.
- **The outcome is nearly degenerate** (6 values, 27 zeros). The fit ran
  without a failed resample, but it is a fit to very little variation.
- Two copies of the shortfall arithmetic now exist (§2).
- The script still launches the CLI itself (accepted in R13).

## 9. Notes on what I changed my mind on

`NOTES_changed_my_mind.md`: dropped the bit-equality assertion after it
failed, then found the cause; exit statuses redefined (this run exits 2);
R12's candidate `h` columns removed; known-answer frames now use the real
columns and covariate through the identical call; resamples default to the
manual's 10,000.

## 10. Three things I am least sure about

1. **Which `h` is "mean h".** I used the analysis crate's existing
   reconstructed margin: true `θ`, sampled at the end of each tick. The
   plugin's own decision-time `h_t` (observed `θ`, delayed by 2 ticks here)
   differs from it on 5,934 of 33,972 firm-ticks (mean |difference| 0.038,
   max 0.68). Shortfall is decision-time, so the two regressors are sampled
   at different instants. The owner's definition did not say which.
2. **Pooling.** The outcome is firm-averaged, as specified. The regressor
   and covariate are the mean over all matched firm-ticks (pooled), which is
   how I read "mean h over matched live firm-ticks". A firm-averaged
   regressor would be a different number.
3. **Excluding the death tick.** A firm's last tick is not a live firm-tick
   (the analysis crate's convention). In three-death runs that is the only
   tick where firms 0 and 1 act differently from the fork, so this choice is
   why their matched ΔH is exactly 0 rather than slightly non-zero.
