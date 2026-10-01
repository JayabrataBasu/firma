# ADR-0057 — Symmetric prototype vs. Option A vs. HEAD: measurement results

**Measured 2026-10-01.** Three release binaries, each built from a clean
`c5eb8cd` tree with exactly one diff applied (`raw/hashes.txt` has their
sha256):

| Binary | Diff applied | Built from |
|---|---|---|
| `firma-head` | none (clean `c5eb8cd`) | main checkout |
| `firma-a` | `prototype-option-a/option_a_prototype.diff` | worktree |
| `firma-asym` | `prototype-symmetric/option_a_symmetric_prototype.diff` | worktree |

Reserved seeds 1–200 were not used anywhere in this round: the probe uses
`(909,2,3,4)`, `cfg_arm_b_satisficing` uses `(8080,41,97,13)` — both outside
the reserved range (§30.5/§30.9 reserve 1–200 for E1's own registered design).

## 0. ADR-0050's two `event_log_sha256` values, reproduced on all three

| Config | HEAD | A | A-sym |
|---|---|---|---|
| `cfg_arm_b_satisficing` | `8663d47a…` | `8663d47a…` (identical) | `8663d47a…` (identical) |
| `cfg_wide_search(6,0.0)` current config | `68c155f8…` | `0effed07…` | `0effed07…` |

Both reference values reproduced exactly on HEAD before any comparison was
made. Full hashes: `raw/hashes.txt`.

## 1. PRIMARY — probe, 15 cells × 2 configs (30 runs), selection counts

Full per-cell table: `raw/sweep30_head_a_asym.tsv` (binary, config, w_max,
beta, shaping-action-selected-count, total-decisions, event_log_sha256).

| | HEAD | A | A-sym |
|---|---|---|---|
| Total shaping selections, 15×400 (current config) | **8** (4 at (6,0.0), 4 at (9,0.0), ticks 8 & 17, both agents, exactly reproducing ADR-0056/0057's F1/F5) | **0** | **0** |
| Total shaping selections, 15×400 (pre-`17ba7bb` config) | 0 | 0 | 0 |
| A vs A-sym: cells with differing selection count | — | — | **none — 0 of 30** |
| A vs A-sym: cells with differing `event_log_sha256` | — | — | **none — 0 of 30** |

**Verdict against PREDECLARED.md §4 criterion 1–2: STABLE.** Exact equality
on every one of the 30 cells, not just the aggregate.

## 2. PRIMARY — `cfg_arm_b_satisficing` lobby-reached count

| | HEAD | A | A-sym |
|---|---|---|---|
| decisions | 1484 | 1484 | 1484 |
| lobby_reached | 75 | **42** | **42** |
| SURVIVAL/GOAL(1) split of reaches | 67/8 | 34/8 | 34/8 |
| gate evaluations / closed | 67/67 | 34/34 | 34/34 |
| shaping selected | 0 | 0 | 0 |
| event_log_sha256 | `8663d47a…` | `8663d47a…` | `8663d47a…` |

**Verdict: STABLE, exact equality (42 = 42), as predicted** — reaching index
6 depends only on actions 1,3,5,2,0,4's admissibility/satisficing, all `a<6`
and therefore identical between A and A-sym by construction. Event log
byte-identical to HEAD too (A and A-sym never select shaping here, same as
HEAD).

## 3. First tick where A and A-sym diverge

**None exists in the full existing-scenario set measured** (30 probe cells +
`cfg_arm_b_satisficing` + `cfg_wide_search(6,0.0)` alone + 3 window variants
= 35 runs). A and A-sym produced byte-identical `event_log_sha256` in all 35.
The symmetric extension (lobby/contract's `h_success`/`h_failure`, the
unconfigured/gate-closed shaping fallthrough, and `diversify`'s fallthrough)
is exercised in some of these runs (e.g. the 4 shaping evaluations at
`(6,0.0)`/`(9,0.0)` do reach the gate, and some runs do reach the cost-only
shaping fallthrough) but never changes the outcome actually selected,
because in every case where it is exercised, a market action (`a<6`,
identical under A and A-sym) already satisfices first.

**Supporting comparison, HEAD vs. A-sym (where they do diverge, for
context):**
- `cfg_wide_search(6,0.0)`: first divergent tick = **tick 8**, agents 0 and
  1, `action 6 → 1` (`produce_ordinary`), exactly reproducing ADR-0057 F3's
  hand-computation. Second divergence at tick 17, same pattern.
- `wmax15` variant: first divergent tick = **tick 4**, agents 4–7,
  `action 6 → 1`. From tick 9 the two trajectories' *focus* also differs
  (`Goal(1)` vs `Survival`) — a real cascading effect, not a single-tick
  perturbation: HEAD's trace has 50 `(tick,agent)` decisions beyond
  A-sym's last one (agents survive longer, or die at a different tick,
  under A-sym).

## 4. Supporting metrics (existing scenarios, full previous-round set)

| Metric | HEAD | A | A-sym |
|---|---|---|---|
| `cfg_arm_b_satisficing` selections | 0 | 0 | 0 |
| part (b), `cfg_input_starved` direct | 3/600 (not re-run this round — unaffected by any `a<6`/`a≥6` fallthrough change per prior round; not touched by this prototype) | 3/600 | *(not re-measured — see note)* |
| `l_w=6` variant: decisions / shaping selected | 1834 / **3** | 1834 / **0** | 1834 / **0**, hash = A |
| `l_w=8` variant: decisions / shaping selected | 3242 / **12** | 3242 / **12** | 3242 / **12**, hash = A **= HEAD** |
| `w_max=15` variant: decisions / shaping selected | 1562 / **9** | 1512 / **5** | 1512 / **5**, hash = A |
| `sc16_gate` / `sc16b_arm_scoping` | — | identical to HEAD (prior round) | *(not re-run this round — scope was the 5 metrics above + test suite; see note)* |
| 4 frozen golden hashes | — | unchanged (prior round) | `golden_trace_unchanged` passes under A-sym (§5) — not independently re-diffed byte-for-byte against the other three's stored values this round |

**Note on scope:** part (b), `sc16_gate`/`sc16b_arm_scoping`, and the frozen
hashes were re-confirmed for **A** in the prior round
(`prototype-option-a/README.md`) and are reported here from that round
rather than re-run a third time, since nothing in the symmetric diff touches
any code path those exercise differently from A (none of them reach a
gate-open-and-configured shaping evaluation or the diversify/unconfigured
fallthrough in a way the prior round didn't already characterise as
identical to HEAD). This is a scope choice, not a result — flagged plainly
rather than presented as measured.

## 5. Full test suite under A-sym

`raw/workspace_tests_asym.txt` (full `cargo test --workspace --no-fail-fast`
output).

**Failing-test set: identical to A's** (`prototype-option-a/README.md`'s own
5): `h3_channel_opens_in_a_real_run` plus four `firma-plugin-decision` unit
tests. Raw assert text, each:

```
thread 'h3_channel_opens_in_a_real_run' panicked at tests/tests/sanity.rs:839:5:
assertion `left == right` failed: the one decision this tick should be lobby (action 6) — the ADR-0047 channel did not open in a real run the way the unit tests predict
  left: 0.0
 right: 1.0

thread 'tests::h3_lobby_satisfices_survival_when_expected_relief_clears_h_t' panicked at crates/firma-plugins/firma-plugin-decision/src/tests.rs:493:5:
assertion `left == right` failed: lobby should satisfice SURVIVAL once its expected relief is modelled (ADR 0047)
  left: Some(1)
 right: Some(6)

thread 'tests::h3_the_survival_test_is_not_lag_sensitive_by_design' panicked at crates/firma-plugins/firma-plugin-decision/src/tests.rs:572:5:
assertion `left == right` failed
  left: Some(1)
 right: Some(6)

thread 'tests::h3_race_short_lag_wins_lobby_satisfices' panicked at crates/firma-plugins/firma-plugin-decision/src/tests.rs:627:5:
assertion `left == right` failed: short lag (min=2) comfortably beats time_to_boundary=10 ⇒ lobby should satisfice
  left: Some(1)
 right: Some(6)

thread 'tests::h3_toggle_off_reverts_to_payoff_only_despite_long_lag' panicked at crates/firma-plugins/firma-plugin-decision/src/tests.rs:692:5:
assertion `left == right` failed: require_time_margin=false must ignore the race and restore ADR-0047's payoff-only satisficing test — a config-only switch, no code change
  left: Some(1)
 right: Some(6)
```

All 4 unit-test failures show the identical `left: Some(1), right: Some(6)`
pattern as under A — these fixtures reach the gate-open, configured-success
branch (the one place the symmetric diff's `h_success`/`h_failure` change is
actually live), and even there, A-sym's result is unchanged from A's: a
market action still satisfices first, for the same reason (empty-window,
seeded-`regulated_intensity` fixture construction,
`prototype-option-a/README.md`'s own diagnosis).

**Tool results:**
| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 2 targets failed (`firma-conformance --test sanity`, `firma-plugin-decision --lib`), same 5 individual test failures as A, `sc4_wmax_beta_probe` itself now **passes** (0/400 matches its unmodified assertion) |
| `cargo clippy --workspace --all-targets` | clean |
| `cargo fmt --check` | clean |
| `scripts/lint-architecture.sh` | 9/9 passed |
| `cargo metadata \| scripts/check_deps.py` | exit 0, no violations |

## 6. Amendment 5 — E1 diagnostic: **not run, blocked**

**No registered E1 config exists to run this diagnostic against.** Checked
directly: `python/firma_lab/spec.py::build_narrowed_e1_spec` builds the
ADR-0051 narrowed 195-cell *factor grid*, but it takes
`model_config_template` as a **required parameter with no default**, and its
own docstring states why: *"the manual's §30.4 table specifies factor
levels, not a base agent population/environment configuration... inventing
one silently here would be exactly the kind of silent default this
project's discipline forbids."* ADR-0051 Point 7 confirms this is deliberate
and unresolved: *"the actual E1 run configurations... are Phase 3 build
work, not part of this ADR."* Manual §30.6 confirms independently: **"No
runs of the registered design have been executed."**

So there is no concrete, registered E1 run configuration in this repository
— not a missing file I failed to find, but a parameter the project's own
code and its governing ADR both explicitly leave open for a later Phase-3
decision. Building one now, even for a read-only diagnostic, means choosing
an agent population/environment the manual does not specify, which is
exactly the choice CLAUDE.md and the working agreement say to stop and ask
about rather than pick silently. **Not done. Flagged for the owner**: if a
substitute population is wanted for this diagnostic (e.g. one of the
existing `cfg_*` scenarios already in `tests/tests/sanity.rs`, run through
`build_narrowed_e1_spec`'s factor grid instead of a from-scratch E1
population), that is a decision for the owner to make, not a default for me
to pick.
