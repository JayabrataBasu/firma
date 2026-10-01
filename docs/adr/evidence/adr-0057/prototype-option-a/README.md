# ADR-0057 Option A — prototype measurements (2026-10-01)

**Prototype, not adopted.** Code lives only in the git worktree
`/home/jayabratabasu/firma-adr0057-proto`, branch
`adr0057-survival-lookahead-u-fix` (branch point = tag `pre-adr0057-fix` =
`c5eb8cd`), uncommitted. `option_a_prototype.diff` is the entire change: in
`Satisficing::satisfices`' `Focus::Survival` arm, a market action's
`h_{t+1}` uses `u_from_window(advance_window(W, tick, a, L_W))` — the same two
functions the real `constrain` rule and `time_to_boundary` call.

All numbers below are from fresh runs on 2026-10-01; HEAD = `c5eb8cd` binary,
prototype = worktree binary.

| # | Measurement | HEAD | Prototype | File |
|---|---|---|---|---|
| 1 | probe, current config, 15 cells | 4/400 at (6,0.0), (9,0.0); 0 elsewhere | **0/400 in all 15** | `sweep15_proto.txt` (HEAD: `../sweep15_head_vs_pre.txt`) |
| 1 | probe, pre-`17ba7bb` config, 15 cells | 0/400 all | 0/400 all | same |
| 1 | tick 8 / 17 decisions (agent 0, (6,0.0)) | lobby via `E[h]` | `produce_ordinary` satisfices as first scanned action (`u` 7/8→6/8, `h` 0.025→0.15); 44/200 SURVIVAL ticks | trace in session scratchpad; summarised in PROGRESS.md |
| 2 | part (b), inside the HEAD-version test | unreachable (panic in (a)) | **runs: 3/600, SC-4 pass = false, test passes (a)+(b)** | `sanity_harness_prototype.txt` |
| 3 | `cfg_arm_b_satisficing` (event log `8663d47a…`, reproduced) | 1484 decisions; lobby reached 75; 67 SURVIVAL / 8 GOAL(1); 67 gates all closed, ttb=1; 0 selected | **event log byte-identical**; lobby reached **42**; **34** SURVIVAL / 8 GOAL(1); 34 gates all closed, ttb=1; 0 selected. The 33 lost reaches: `acquire_input` now satisfices (was already the fallback choice) | `adr0050_arm_b_metrics_head_vs_prototype.txt` |
| 3 | `cfg_wide_search(6,0.0)` (log `68c155f8…`, reproduced) | 4/400, gates open at ticks 8/17 | 0/400; lobby never reached | `sweep15_proto.txt` |
| 3 | window variants (reconstructed; HEAD reproduces ADR-0050's 3/75, 12/363, 9/44 exactly) | `l_w=6`: 3/75 open, 3 selected; `l_w=8`: 12/363, 12; `w_max=15`: 9/44, 9 | `l_w=6`: 0/27, 0; `l_w=8`: 12/51, 12 (log identical); `w_max=15`: 5/14, 5 | `adr0050_window_variants_head_vs_prototype.txt` |
| 4 | `sc16_gate`, `sc16b_arm_scoping` | — | **every SC line identical to HEAD** | `sanity_harness_*.txt` |
| 4 | `h3_channel_opens_in_a_real_run` | passes (1/1 lobby) | **fails** (0/1: `produce_ordinary` satisfices first) | same |
| 5 | frozen hashes | — | golden test passes; `phase1-smoke` `14b9eb59…`/`310f636f…`, `phase2-smoke` `0529c6bb…`/`9a476938…`, `phase2-stage5-smoke` `5824031c…`/`fd3aa4f2…` — **all four unchanged** | — |
| 6 | `cargo test --workspace` | 1 failure (probe) | **5 failures**: `h3_channel_opens_in_a_real_run` + decision-plugin unit tests `h3_lobby_satisfices_survival_when_expected_relief_clears_h_t`, `h3_the_survival_test_is_not_lag_sensitive_by_design`, `h3_race_short_lag_wins_lobby_satisfices`, `h3_toggle_off_reverts_to_payoff_only_despite_long_lag` (all expect lobby, get 1); probe now passes | `workspace_tests_prototype.txt` |
| 6 | clippy / fmt / lint-architecture / check_deps | — | clean / clean / 9/9 / exit 0 | — |

The five new failures share one construction: an **empty action window with
a seeded `regulated_intensity`** (`cfg_compliance_bound_survival`,
`cfg_h3_survival_lobby_opens`), whose own comments say the scenario relies on
the lookahead holding `u` fixed. With the window empty, the engine's own `u`
after one non-regulated action is 0 (the seed is a tick-0-only value, ADR-0014/
0028), so the prototype sees a large real improvement.

`extract_cfg.py` reproduces `tests/tests/sanity.rs` config strings exactly
(validated by the two ADR-0050 `event_log_sha256` values); `adr0050_metrics.py`
recomputes ADR-0050's trace statistics; `mkh3open.py` transcribes
`cfg_h3_survival_lobby_opens`; `runb.sh` runs a cell against a chosen binary.
