# Raw diagnostic trace evidence for ADR-0050

**Status: same DRAFT / pending-owner-review posture as ADR-0050 itself.**
These two files are a representative sample, not the full record — see
ADR-0050 Decision 5 and PROGRESS.md OQ-12 for what else exists only in
this working session's scratchpad directory and this project's
conversation transcript (not preserved here), and why.

## What each file is

- **`trace_cfg_arm_b_satisficing_baseline.ndjson`** (12,892 lines) — the
  full `FIRMA_TRACE_DECISION=1` trace of one run of `cfg_arm_b_satisficing`
  (the Arm-B-shaped, shocked, 10-firm, 400-tick scenario defined in
  `tests/tests/sanity.rs`'s `cfg_arm_b_satisficing()`), against `firma`
  (the CLI binary) at `--model`. This is the source of ADR-0050 Decision
  1's headline numbers: 1484 total decisions, 75 reaching lobby's
  checklist position, 67 `SURVIVAL`-focus gate evaluations, all 67 closed
  with `lag_min=2`/`time_to_boundary=1`.
- **`trace_cfg_wide_search_wmax6_beta0.0.ndjson`** (3,565 lines) — the same
  tracing against `cfg_wide_search(w_max=6, beta=0.0)` (also
  `tests/tests/sanity.rs`), one of the two non-zero-shaping cells in the
  §16.1 `w_max × β` sweep. Source of ADR-0050 Decision 1's `4/400`
  selections and the tick-8/tick-17 gate-open/payoff-win numbers. The
  `w_max=9, β=0.0` cell (the other non-zero cell) produced numerically
  identical gate/payoff results (same firms, same ticks, same `e_h`/`h_t`)
  and was not separately preserved here — see ADR-0050 Decision 5.

## Trace record schema

Newline-delimited JSON. Every record carries `kind`, `tick`, `agent`; most
carry `action` (0–8, per manual §11.1/§11.2's action indices — 6=lobby,
7=contract, 8=diversify). Record kinds, emitted from
`firma-plugin-decision/src/lib.rs`'s temporary `mod trace` (see below):

| `kind` | Emitted from | Fields |
|---|---|---|
| `admissible_check` | `Satisficing::apply`'s wrapped `admissible` closure | `action`, `result` |
| `satisfices_check` | `Satisficing::apply`'s wrapped `satisfices` closure | `action`, `focus`, `h_t`, `result` |
| `selection` | `Satisficing::apply`, once per decision | `focus`, `w_eff`, `action`, `prev_action`, `h_t`, `shortfalls`, `scan_order` |
| `shaping_eval_entered` | `DecideCtx::shaping_expected_survival_margin`, on entry | `action` |
| `shaping_no_success_model_configured` | same, when `lobby_success`/`contract_success` is `None` | `action` |
| `shaping_gate` | same, at the ADR-0048 race check | `action`, `lag_min`, `time_to_boundary`, `gate_open` |
| `shaping_payoff_computed` | same, when the gate opens | `action`, `p_success`, `h_success`, `h_failure`, `e_h` |
| `shaping_payoff_comparison` | `Satisficing::satisfices`, the `SURVIVAL` shaping branch | `action`, `e_h`, `h_t`, `result` |

A decision's full picture is reconstructed by grouping records with the
same `(tick, agent)`.

## How these were generated (for independent verification)

The tracing module is the working-tree diff to
`crates/firma-plugins/firma-plugin-decision/src/lib.rs` referenced by
ADR-0050 Decision 5 (env-var-gated, `#[forbid(unsafe_code)]`-clean,
confirmed byte-identical `event_log_sha256` with tracing on vs. off — see
that diff's own doc comment for the full non-behavior-altering argument).
With that diff applied and the workspace built:

```sh
cargo build -p firma-cli --bin firma

# baseline (exact JSON body: tests/tests/sanity.rs's cfg_arm_b_satisficing())
FIRMA_TRACE_DECISION=1 FIRMA_TRACE_DECISION_PATH=/tmp/trace_arm_b.ndjson \
  ./target/debug/firma run --model --config <cfg_arm_b_satisficing.json> --out /tmp/out_arm_b
# event_log_sha256 8663d47acdf89547798be1f4e137665df6578238aab5dc8732a5417df921d125

# wide-search cell (exact JSON body: tests/tests/sanity.rs's cfg_wide_search(6, 0.0))
FIRMA_TRACE_DECISION=1 FIRMA_TRACE_DECISION_PATH=/tmp/trace_wide_6_0.ndjson \
  ./target/debug/firma run --model --config <cfg_wide_search_6_0.0.json> --out /tmp/out_wide
# event_log_sha256 68c155f8e6b640a9ed92974c631f65fac89ccf21aec9b8fbd5aeadf458a95578
```

The two config JSON bodies used are byte-for-byte the string literals
`cfg_arm_b_satisficing()` and `cfg_wide_search(6, 0.0)` produce in
`tests/tests/sanity.rs` — not reproduced here as separate files (per
ADR-0050 Decision 5, the ~14 temporary variant configs used across all
three diagnostic rounds were deliberately not committed; these two base
configs are fully recoverable from the test file already in the repo).
`event_log_sha256` is printed by `firma run`'s own output; both values
above were independently reproduced with tracing off (`FIRMA_TRACE_DECISION`
unset) and matched exactly, confirming the trace is non-behavior-altering
for these two runs specifically (ADR-0050's broader claim, checked across
every round, is not re-derived here).

## Using these files to check ADR-0050's cited numbers

Example — the "75 reached lobby's checklist position, 67 under `SURVIVAL`
focus, all 67 gate-closed at `lag_min=2`/`time_to_boundary=1`" claim:

```sh
python3 - <<'PY'
import json, collections
decisions = {}
for line in open("trace_cfg_arm_b_satisficing_baseline.ndjson"):
    r = json.loads(line)
    decisions.setdefault((r["tick"], r["agent"]), []).append(r)
reached = sum(1 for evs in decisions.values()
              if any(e["kind"] == "admissible_check" and e["action"] == 6 and e["result"]
                     for e in evs))
gates = [e for evs in decisions.values() for e in evs if e["kind"] == "shaping_gate"]
print("total decisions:", len(decisions))
print("reached lobby's position:", reached)
print("gate evaluations (all SURVIVAL-focus):", len(gates))
print("gate_open distribution:", collections.Counter(g["gate_open"] for g in gates))
print("(lag_min, ttb) distribution:",
      collections.Counter((g["lag_min"], g["time_to_boundary"]) for g in gates))
PY
```

Expected output: `total decisions: 1484`, `reached lobby's position: 75`,
`gate evaluations: 67`, `gate_open distribution: Counter({False: 67})`,
`(lag_min, ttb) distribution: Counter({(2, 1): 67})`.
