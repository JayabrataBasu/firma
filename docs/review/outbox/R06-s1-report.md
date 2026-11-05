# R06 — S1 "variance reach" and inertness — builder report

DIAGNOSTIC-ONLY. 2026-10-02. No commit, no push, no source edit, no run on a
seed below 910001.

## 1. Task as understood

1. On an export of HEAD, run `configs/experiments/phase2-stage5-smoke.json`
   with ticks = 120 and `shock.scheduled` replaced by one `shock.stochastic`
   rule, varying only the shock seed over 910001..910050. Per run: sha256 of
   the `selected_action` event lines, survivors at tick 120, mean `H_rep`.
   Same 50 seeds for a negative control (no randomness in the shock) and a
   positive control (`decision.random`). Apply the predeclared reading.
2. Write `inertness.sh` (two export dirs → build, run, compare event-log
   sha256). Run it for `9c49c34` vs HEAD on the shocked config (5 seed sets)
   and on the 15-cell `mkcfg.py` `pre` probe family, with a positive control.

## 2. Files changed

Only new files, all under the two authorised paths:

| Path | Lines |
|---|---|
| `docs/review/evidence/R06/s1_variance.py` | +114 |
| `docs/review/evidence/R06/s1_variance_output.txt` | +279 |
| `docs/review/evidence/R06/inertness.sh` | +34 |
| `docs/review/evidence/R06/inertness_run.sh` | +17 |
| `docs/review/evidence/R06/mk_inertness_cfgs.py` | +39 |
| `docs/review/evidence/R06/inertness_output.txt` | +31 |
| `docs/review/evidence/R06/provenance.txt` | +11 |
| `docs/review/evidence/R06/SHA256SUMS` | +7 |
| `docs/review/outbox/R06-s1-report.md` | this file |

Nothing else in the main checkout was edited or built into. Builds and runs
happened in `git archive` exports of `2820610` and `9c49c34` under the
session scratch directory.

## 3. Tests added

None. This task is a diagnostic; no fix was made, so there is no
failing-then-passing test.

## 4. Full suite summary

**Not run.** No source changed, and the committed suites at HEAD still
contain the `(n,n,n,n)`, n ≤ 4 fixtures (the Batch 1 move is uncommitted, so
it is not in the export). Running them would execute reserved tuples, which
this prompt does not authorise.

## 5. Claims, each with evidence

All numbers are from `evidence/R06/s1_variance_output.txt` and
`evidence/R06/inertness_output.txt`; checksums in `evidence/R06/SHA256SUMS`.

### Part 1 — variance reach (50 shock seeds per arm)

| Arm | Distinct action hashes | Distinct full logs | Survivors at tick 120 | mean `H_rep`: distinct values, min–max, sample sd |
|---|---|---|---|---|
| satisficing, stochastic shock (onset 10–80, magnitude 0.3 ± 0.1) | **23** | 50 | 1 in 27 runs, 3 in 23 runs | 23; 0.480–0.825; **0.1161** |
| negative control, onset fixed at 10, sd 0 | **1** | 1 | 3 in 50 | 1; 0.8026; 0 |
| negative control, onset fixed at 45, sd 0 | **1** | 1 | 3 in 50 | 1; 0.7999; 0 |
| negative control, onset fixed at 80, sd 0 | **1** | 1 | 3 in 50 | 1; 0.8265; 0 |
| positive control, `decision.random` | **32** | 50 | 0 in 48 runs, 1 in 2 | 32; 1.254–1.418; 0.0421 |

**Predeclared reading:** satisficing has ≥ 5 distinct action hashes (23),
sd(`H_rep`) > 0 (0.1161), and the negative control has exactly 1 distinct
hash (at all three onsets tried). **The condition "variance reaches the
outcome" is met.** No claim about statistical power is made.

Details that qualify the number 23:

- 14 of the 50 runs share one action hash (`161a582b…`): drawn onsets 13–39
  with magnitudes 0.159–0.296. Within that region neither the onset nor the
  magnitude changed any action.
- Firm 2 dies of `solvency` at tick 12 in all 50 satisficing runs and in
  all 150 negative-control runs, whatever the shock. (It does not die at
  tick 12 in any `decision.random` run.)
- The 27 runs with one survivor all lose firms 0 and 1 to `compliance`. In
  22 of them both die on the tick after the drawn onset; in the other 5
  they die 7, 12, 20, 22 and 32 ticks after it. The 23 runs with three
  survivors have no compliance death.
- `H_rep` here is `SanityReport.sc6_entropy_mean`
  (`crates/firma-analysis/src/sanity.rs:161-162,243-244`): the mean, over
  live firm-ticks with a non-empty window, of `−Σ p log₂ p` over the
  trailing action window (`L_W = 4` in this config). It was obtained from
  the crate's own example binary `sanity_report_json`, built in the HEAD
  export. A per-run value is therefore exposed; nothing was invented. It is
  a run-level mean over firms and ticks, so it shifts when firms die
  earlier — it is not a per-firm outcome and it is not `ΔH_rep`.

### Part 2 — inertness, `9c49c34` vs HEAD (`2820610`)

| Check | Result |
|---|---|
| A. Shocked stage5-smoke config as shipped (`shock.scheduled`, `resource.patchy`, `observation.delayed`), ticks = 120, 5 seed sets `(910001+10k … 910004+10k)` | **5 / 5 EQUAL** |
| B. Probe family, `mkcfg.py` variant `pre`, 15 cells | **15 / 15 EQUAL** |
| Summary A + B | equal = 20, different = 0, error = 0 |
| Control 1: opt-in variant (`head`, w_max 6, β 0) on old vs new | **ERROR, not a comparison**: the `9c49c34` binary rejects the config (`unknown field contract_success`) |
| Control 2: HEAD binary, `pre` vs opt-in `head` at (6, 0.0) | **DIFFERENT** (`0effed07…` vs `b63441b6…`) — the check can see the opt-in mechanism |
| Control 3: HEAD vs HEAD, same config | EQUAL — the comparison is not always-different |

The 15 probe hashes equal the ones in `evidence/R05/inertness.txt` although
the seeds differ (see §7).

## 6. What I did NOT verify

- Statistical power, effect size, or anything about `ΔH_rep` (no fork run
  exists).
- Whether the variance in `H_rep` comes from changed decisions of surviving
  firms or only from firms dying at different ticks. The measure mixes both.
- Any other shock parameterisation, channel, horizon, population or
  template. One config, one magnitude mean, 120 ticks, 4 firms.
- The positive control varies only the shock seed; `decision.random` draws
  from the mechanism stream, whose seed was held at 910001. Its 32 distinct
  hashes therefore come from the shock changing state and admissibility,
  not from different random choices.
- Inertness for pinned (Arm A) runs, multi-firm runs with shaping opted in,
  400-tick horizons, or `decision.random`.
- The test suites (§4). cargo-deny, CI.

## 7. Outside scope, noticed

- **Probe seeds.** `mkcfg.py` hard-codes seeds `(909, 2, 3, 4)`; three of
  those streams are inside 1–200. I replaced only the seeds with
  `(910001, 910002, 910003, 910004)` rather than run them. The resulting 15
  event-log hashes are identical to R05's, which used the original seeds:
  the `pre` probe family does not depend on its seeds at all.
- The negative control's fixed onset was not specified in the prompt. I did
  not pick one: I ran the two endpoints of the given range and the midpoint
  (10, 45, 80). All three give exactly one hash.
- "The same config with shock.scheduled" was read as: the shipped config,
  ticks = 120, its two scheduled shocks untouched. If the shipped 16-tick
  horizon was meant, part A needs a rerun.
- The uncommitted Batch 1 changes are not in either export, as instructed
  ("export of HEAD"). They touch no Rust non-test code, so the binaries
  would be the same either way.
- `inertness.sh` refuses to run on a directory containing `.git`.

## 8. Three things I am least sure about

1. Whether `sc6_entropy_mean` is the "mean repertoire entropy" the reviewer
   had in mind. It is the only per-run entropy value the analysis crate
   exposes, but survival differences move it (survivors 1 vs 3 split almost
   evenly), so sd(`H_rep`) > 0 may largely be a survival effect.
2. Whether Control 1 should count as a positive control at all. The old
   binary cannot parse the opt-in config, so only Control 2 shows that a
   real behavioural difference is detected.
3. How far the result carries. 14 of 50 seeds collapsed to one action
   sequence, and firm 2's death is deterministic; a different base config
   could give far more or far less variance.
