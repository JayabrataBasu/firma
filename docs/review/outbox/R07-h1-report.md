# R07 — H-1 hygiene fixes — builder report (final)

2026-10-02. No commit, no push. The AUTHORISATION line was present in the
prompt, so the existing suites were run as they are. This replaces the
earlier, partial version of this report: every item it listed as unverified
has now been checked, and §6 says exactly what cannot be observed from here.

All evidence is in `docs/review/evidence/R07/` (23 files, checksums in
`SHA256SUMS`).

## 1. Task as understood

(1) Replace silent drops of unparsable persisted records at nine named sites
with a loud failure naming agent, key and offending string — a panic in rule
paths, a counted error in `firma-analysis`. (2) CI: run on this branch, add a
Python job that fails on "not built" skips, make cargo-deny discoverable and
say why it failed. (3) Correct the false comment and summary string in
`sanity.rs` for `cfg_threaded`. No numerical output, hash, assertion,
threshold, seed or config may change.

## 2. Files changed (`git diff --numstat`)

| Path | + / − |
|---|---|
| `.github/workflows/ci.yml` | 53 / 3 |
| `crates/firma-plugins/firma-plugin-decision/src/lib.rs` | 42 / 7 |
| `crates/firma-plugins/firma-plugin-decision/src/tests.rs` | 91 / 0 |
| `crates/firma-plugins/firma-plugin-constraint/src/phase_rules.rs` | 24 / 10 |
| `crates/firma-plugins/firma-plugin-constraint/src/phase_rules/tests.rs` | 55 / 0 |
| `crates/firma-plugins/firma-plugin-observation/src/lib.rs` | 13 / 1 |
| `crates/firma-plugins/firma-plugin-observation/src/tests.rs` | 31 / 0 |
| `crates/firma-analysis/src/reconstruct.rs` | 95 / 4 |
| `tests/tests/sanity.rs` | 22 / 5 |
| `docs/review/evidence/R07/*`, this report | new |

Edits beyond the literal site lines, needed to make the sites loud:
- decision `lib.rs`: helpers `corrupt_record(agent, key, record, err) -> !`
  and `relation_graph(view)` (used by sites :1027 and :1205).
- constraint `phase_rules.rs`: helper `parse_window`, used by both sites.
- `reconstruct.rs`: a `parse_errors: Vec<String>` field, its initialiser, a
  `parse_errors()` accessor, and the crate's first unit-test module.

`sanity.rs`: the diff (`sanity_rs.diff`) contains only doc-comment lines and
one `eprintln!` — no assertion, config, seed or threshold line.

## 3. Tests added, with raw before/after

Before: `silent_drops_before.txt`. After: `silent_drops_after.txt`. Each
fixture contains exactly one corrupt record (`{"tick":"CORRUPT-RECORD"}`).

| Test | Before | After |
|---|---|---|
| decision `corrupt_action_window_record_panics_naming_agent_key_and_record` | FAILED — `expected a panic on the corrupt record, but the rule returned normally` | ok |
| decision `corrupt_lagged_effect_record_panics_naming_agent_key_and_record` | FAILED, same | ok |
| decision `corrupt_observed_env_record_panics_naming_agent_key_and_record` | FAILED, same | ok |
| decision `corrupt_relation_edge_panics_in_satisficing_naming_key_and_record` | FAILED, same | ok |
| decision `corrupt_relation_edge_panics_in_decision_random_naming_key_and_record` | FAILED, same | ok |
| constraint `action_window_panics_on_a_corrupt_record_naming_agent_key_and_record` | FAILED, same | ok |
| constraint `enforce_panics_on_a_corrupt_window_record_naming_agent_key_and_record` | FAILED, same | ok |
| observation `delayed_panics_on_a_corrupt_history_record_naming_key_and_record` | FAILED, same | ok |
| analysis `one_corrupt_window_record_is_counted_and_named_not_silently_dropped` | FAILED — `left: 0, right: 1` | ok |
| analysis `a_well_formed_window_reports_no_parse_error` | ok | ok |

Sample message after the fix:
`agent 0: corrupt persisted record under `action_window` (invalid WindowEntry: invalid type: string "CORRUPT-RECORD", expected u64 at line 1 column 24); offending record: "{\"tick\":\"CORRUPT-RECORD\"}". Refusing to drop it and continue (manual §19.5).`

The analysis "before" was taken with the `parse_errors` field and accessor
present but nothing recording into it. No existing test relied on silent
dropping: the full suite passes unchanged.

Fix (3) has no test (the prompt forbids changing `sanity.rs` assertions). Its
before/after is the `sc16_gate` printout; the only differing line
(`sc16_gate_diff.txt`):

```
<   SC-3  partial — compliance/scope/obligation emerge; solvency only from a knife-edge seed
>   SC-3  NOT achieved — threaded binds [sol,cmp,scp,obl] = [true, false, true, true], first@ [Some(0), None, Some(0), Some(0)]
```

The new line is printed from the report values, not hard-coded.

Fix (2), skip check, with the job's own shell logic:
- binaries absent: `120 passed, 14 skipped`, exit 1
  (`ci_python_check_cli_absent.txt`);
- binaries present: `134 passed`, exit 0
  (`ci_python_check_cli_present.txt`).

## 4. Full suite summary (working tree)

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 247 passed, 0 failed, 1 ignored; `golden_trace_unchanged` ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `scripts/lint-architecture.sh` | passed (9/9) |
| `check_deps.py` | exit 0 |
| `pytest python/tests` (binaries built) | 134 passed, 0 skipped |

## 5. Claims, each with evidence

**Numerical output is unchanged.**
- `hash_equivalence_output.txt`: event logs of HEAD (`2820610` export) and
  the working tree are byte-equal on 23 of 23 diagnostic configs, seeds
  ≥ 910001: 5 shocked stage5-smoke runs, the 15-cell `pre` probe family, the
  opt-in probe cell (6, 0.0), and the `phase2-smoke` and
  `phase2-stage5-smoke` shipped configs with their seeds replaced. The
  `phase2-smoke` hash is `9a476938…`, the value recorded in ADR-0049.
- `sc16_gate_diff.txt`: every number in the printout is unchanged.
- `cargo_test_workspace.txt`: golden trace unchanged.

**What a rule panic does end to end** (`rule_panic_end_to_end.txt`; a
partial Arm-A pin at tick 4 was used because a corrupt record cannot be
injected through a config, and it fails through the same mechanism):
- sequential and `--parallel`: exit code 101, the message on stderr, nothing
  on stdout;
- the run directory holds `events.ndjson` and `snapshots/` only — no
  `manifest.json`, no `run_id.txt`;
- the event log has complete ticks 0–3 and one tick-4 record (the
  `intervention_applied` line), no `tick_started` for tick 4;
- `firma_lab.runner` records a non-zero exit as a failed job with the stderr
  text (`runner.py`, `proc.returncode != 0` branch).

**CI workflow.**
- The file parses as YAML; jobs: `deny, determinism-macos, format, lint,
  python, test`; push branches `[main, master, h3-satisficing-lookahead]`.
- `ci_rust_jobs_replay.txt`: the `format`, `lint` and `test` job commands,
  run locally with the workflow's `RUSTFLAGS`/`RUSTDOCFLAGS`, all exit 0
  (`cargo test --workspace --all-features`: 247 passed, 1 ignored).
- `ci_python_job_replay_py312.txt`: the `python` job's steps replayed in a
  scratch copy with a fresh Python 3.12 venv and no pre-built extension.
  First replay: `132 passed, 2 skipped`, step exit 1 — the job built only the
  CLI, and two tests also need `target/debug/examples/sanity_report_json`.
  The check caught it. The workflow now builds both binaries; second replay:
  `134 passed`, step exit 0.
- `taiki-e/install-action` has a `cargo-deny` manifest (checked through the
  GitHub API).

**Why the deny job fails** (`cargo_deny_as_is.txt`, cargo-deny 0.20.2, the
version in the 2026-09-04 CI log, run on a scratch copy):
`error[unexpected-keys]` at `deny.toml:25`, `wildcard-dependencies = "deny"`;
the accepted key is `wildcards`. The config does not load, so no check runs.
The second cause in the old log (`override toolchain '1.98.0-…-musl' is not
installed`) came from the docker action; the workflow now installs cargo-deny
on the runner beside the pinned toolchain.

**What a corrected key would reveal** (scratch copies only; the repo's
`deny.toml` is untouched):
- with `wildcards = "deny"` (`cargo_deny_scratch_key_fixed.txt`):
  `advisories ok, bans FAILED, licenses FAILED, sources ok`;
- bans: 21 `error[wildcard]` — workspace path dependencies carry no version;
- licences: 1 `error[rejected]` — `target-lexicon 0.13.5`
  (`Apache-2.0 WITH LLVM-exception`), a build dependency of `pyo3` used by
  `firma-py`;
- adding `allow-wildcard-paths = true`
  (`cargo_deny_scratch_wildcard_paths.txt`) leaves 17 wildcard errors.

So the deny job will fail after this change, for the reason at
`deny.toml:25`, and would still fail after that key is renamed.

## 6. What cannot be verified from here

- **A GitHub-hosted run.** Nothing was pushed (I may not push), so no CI run
  exists for these changes. Every job's commands except
  `determinism-macos` were replayed locally (`format`, `lint`, `test`,
  `python` pass; `deny` fails as described in §5); that is not the same as
  seeing the workflow run.
- **`determinism-macos`.** No macOS machine here. Its command is unchanged.
- **The `actions/setup-python` and `taiki-e/install-action` steps
  themselves.** Their effects were reproduced by other means (a uv-provided
  Python 3.12; a downloaded cargo-deny 0.20.2).

Nothing else in this task is unverified.

## 7. Outside scope, noticed (not touched)

Remaining silent handling of unparsable persisted records
(`remaining_silent_drops.txt`, plus reading):

| Site | Behaviour |
|---|---|
| `firma-plugin-constraint/src/phase_rules.rs:347` (`Enforce`) | `filter_map(... from_str::<Edge>(s).ok())` on `relation_edges`; if edges change, the list is rewritten without the unparsable record. In an authorised file but not an authorised line; the audit's grep missed it |
| `firma-plugin-action-shaping/src/lib.rs:248` | corrupt relation store → the rule emits nothing |
| `firma-plugin-action-shaping/src/lib.rs:466-469` | an unparsable lagged record is kept in the queue forever, silently |
| `firma-plugin-shock/src/lib.rs:93, 243, 472` | an unparsable active-shock record is dropped |
| `firma-analysis/src/reconstruct.rs:462` | an unparsable lagged record is skipped |
| `firma-analysis/src/reconstruct.rs:286` | an unparsable `scales` parameter falls back to defaults (a config value, not a persisted record) |

Other points:
- `Reconstruction::parse_errors()` has no reader. `sanity_from_run`
  (`firma-analysis/src/sanity.rs:120`), `firma-py` (`lib.rs:97`) and the
  `sanity_report_json` example build a `Reconstruction` and never check it;
  `firma-tui` has a separate counter of its own. Wiring them needs files
  outside this prompt's authorisation.
- Making the deny job pass needs three decisions in files I may not edit:
  the `[bans]` key name, the policy for versionless workspace path
  dependencies, and whether `Apache-2.0 WITH LLVM-exception` is allowed
  (ADR-0010's licence list).
- The `sc16_gate` SC-4 summary line was left: SC-4 is below 5 % in every
  probe cell, so it is not false.
- `PROGRESS.md` was not updated; it is not an authorised file here.

## 8. Least sure about

Nothing in the work itself. Two items are decisions rather than
uncertainties, and one is a limit of this environment:

1. **Panic vs. another loud failure in rule paths** is the owner's choice.
   What a panic does is now measured (§5); the alternative needs a
   `Rule::apply` signature change.
2. **Whether to wire `parse_errors()` into its callers and fix the six
   remaining silent sites** is a scope decision for the next prompt.
3. **The first hosted CI run** can only be seen after the owner pushes.
   Expected from the replays: `format`, `lint`, `test`, `python` pass;
   `deny` fails on `deny.toml:25`; `determinism-macos` not predicted.
