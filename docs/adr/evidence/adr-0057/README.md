# Evidence for ADR-0056 and ADR-0057 (`sc4_wmax_beta_probe` 4/400)

Produced 2026-10-01 on branch `h3-satisficing-lookahead` at HEAD `c5eb8cd`
(plus that session's uncommitted working tree, which does not touch the
decision plugin or `firma-domain`), with `firma run --model` and the
read-only `FIRMA_TRACE_DECISION` instrumentation. No reserved seeds: the
probe configs use `(909,2,3,4)` and `(11,2,3,4)`.

| File | What it is |
|---|---|
| `mkcfg.py` | Verbatim Python transcription of `tests/tests/sanity.rs::cfg_wide_search` (+ `MARKET_ACTIONS`, `constraint_rules(8)`). Variant `head` = config at HEAD; `pre` = the same without the `lobby_success`/`contract_success` blocks `17ba7bb` added (i.e. the `9c49c34` config). |
| `mkstarved.py` | Verbatim transcription of `cfg_input_starved` (part (b)). |
| `run.sh` | `run.sh <variant> <w_max> <beta>`: write the config, run with tracing, print the SC-4 count via `firma_lab.metrics.sanity_report` (the same Rust `sanity_from_run` the test uses). |
| `sweep15_head_vs_pre.txt` | All 15 `(w_max, β)` cells for both variants on HEAD code. `head` reproduces the test exactly (4/400 at (6,0.0) and (9,0.0), 0/400 elsewhere) — this is the validation of the transcription. `pre`: 0/400 in all 15. |
| `trace_head-w6-b0p0_ticks6-9_15-18.ndjson`, `trace_head-w9-b0p0_…`, `trace_pre-w6-b0p0_…` | Every trace record (both agents) for ticks 6–9 and 15–18: focus, `h_t`, `w_eff`, every admissibility/satisfices check, the gate (`lag_min`, `time_to_boundary`) and the payoff (`p_success`, `h_success`, `h_failure`, `E[h]`). |
| `theta_limit_events_head-w6-b0p0.ndjson` | The `theta_limit` delta events of the `(6, 0.0)` run (0.90 → 1.00 → 1.10 → 1.20). |

Not saved as files (re-derivable with the scripts above):
- the `9c49c34` comparison: that commit was built in a scratch worktree
  (removed); with `mkcfg.py` variant `pre`, agent 0 is in `SURVIVAL` on 44 of
  200 ticks, 0 shaping selections, at `w_max ∈ {6, 9}`, `β = 0`;
- part (b): `mkstarved.py` → `3 / 600`, three tick-0 `GOAL(1)` fallback
  `lobby` picks.
