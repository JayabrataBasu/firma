# R09 — H-2: finish the silent-drop fixes — builder report

2026-10-02. No commit, no push. No run on a seed below 910001 outside the
existing suites. Evidence: `docs/review/evidence/R09/` (21 files, checksums in
`SHA256SUMS`).

## 1. Task as understood

Finish what H-1 started.
1. Simulation-path rules: `Enforce`'s relation-edge parse, the shock plugin's
   three `ActiveShock` sites, and the shaping rule's corrupt-store branch
   become a panic naming key and offending record (manual §19.5; ADR-0054
   precedent).
2. `reconstruct.rs`: a malformed `scales` parameter is an error, not silent
   defaults (absent still means defaults); unparsable lagged records are
   counted in `parse_errors`.
3. `parse_errors()` gets readers: `sanity_from_run`, the example binary and
   `firma-py` fail loudly when it is non-empty.

Nothing numerical may change; HEAD vs working tree must be byte-equal on the
predeclared config set.

## 2. Files changed

`git diff --numstat` against HEAD. Rows marked † also carry H-1's changes, so
the figure is cumulative.

| Path | + / − |
|---|---|
| `crates/firma-plugins/firma-plugin-constraint/src/phase_rules.rs` † | 38 / 11 |
| `crates/firma-plugins/firma-plugin-constraint/src/phase_rules/tests.rs` † | 71 / 0 |
| `crates/firma-plugins/firma-plugin-shock/src/lib.rs` | 28 / 11 |
| `crates/firma-plugins/firma-plugin-shock/src/tests.rs` | 63 / 0 |
| `crates/firma-plugins/firma-plugin-action-shaping/src/lib.rs` | 16 / 4 |
| `crates/firma-plugins/firma-plugin-action-shaping/src/tests.rs` | 27 / 0 |
| `crates/firma-analysis/src/reconstruct.rs` † | 219 / 16 |
| `crates/firma-analysis/src/sanity.rs` | 169 / 2 |
| `crates/firma-analysis/examples/sanity_report_json.rs` | 15 / 10 |
| `crates/firma-py/src/lib.rs` | 81 / 13 |
| `docs/review/evidence/R09/*`, this report | new |

Not touched: `deny.toml`, `ci.yml`, `tests/tests/sanity.rs`, the manual, any
ADR, `orchestrator.rs`, `firma-tui`. (`ci.yml` and `sanity.rs` still show
their H-1 diffs, unchanged.)

API added, all in authorised files:
- `Reconstruction::try_new(cfg) -> Result<_, String>`. `Reconstruction::new`
  keeps its signature and now panics on a malformed `scales` (it cannot
  return an error without changing callers outside the authorisation — see
  §7).
- `sanity::try_sanity_from_run(cfg, run_dir) -> Result<SanityReport, String>`;
  `sanity_from_run` keeps its signature and panics on the same conditions.
- `sanity::reconstruct_checked(cfg, events) -> Result<Reconstruction, String>`.

Also rebuilt, as a side effect of testing `firma-py`:
`python/firma_lab/_native.cpython-314-x86_64-linux-gnu.so` (git-ignored) via
`maturin develop` in the repo's `.venv`.

## 3. Tests added, with raw before/after

Before: `h2_tests_before.txt` (taken with inert stubs of the new functions so
the tests compiled). After: `h2_tests_after.txt`. Each fixture has exactly
one corrupt record or one malformed value.

| Test | Before | After |
|---|---|---|
| constraint `enforce_panics_on_a_corrupt_relation_edge_naming_key_and_record` | FAILED | ok |
| shock `scheduled_panics_on_a_corrupt_active_shock_record_naming_key_and_record` | FAILED | ok |
| shock `stochastic_panics_on_a_corrupt_active_shock_record_naming_key_and_record` | FAILED | ok |
| shaping `shaping_rule_panics_on_a_corrupt_relation_edge_naming_key_and_record` | FAILED | ok |
| analysis `malformed_scales_param_is_an_error_not_silent_defaults` | FAILED | ok |
| analysis `infallible_constructor_panics_on_a_malformed_scales_param` | FAILED | ok |
| analysis `one_corrupt_lagged_record_is_counted_and_named_not_silently_skipped` | FAILED | ok |
| analysis `reconstruct_checked_refuses_a_log_with_one_corrupt_record` | FAILED | ok |
| analysis `try_sanity_from_run_refuses_a_log_with_one_corrupt_record` | FAILED | ok |
| analysis `sanity_from_run_panics_on_a_log_with_one_corrupt_record` | FAILED | ok |
| firma-py `final_firms_refuses_a_log_with_one_corrupt_record` | FAILED | ok |
| controls: `absent_scales_param_still_means_defaults`, `well_formed_scales_param_is_accepted`, `a_clean_log_reconstructs_and_reports`, `final_firms_returns_firms_for_a_clean_log`, and H-1's two window tests | ok | ok |

The four rule-path failures before the fix all read
`expected a panic on the corrupt record, but the rule returned normally`.

After the "before" capture, clippy rejected `.err().expect(...)` in four of
the new tests; they were rewritten as `let Err(err) = … else { panic!(…) }`
with the same assertions. The "after" file was captured after that rewrite.

**End-to-end proof on a real run** (`make_corrupt_fixture.py`,
`surface_check.sh`, `fixture.txt`): the stage5-smoke config with seeds
910001–910004 was run, and one entry of one `action_window` record list
(line 353, tick 5, agent 0) replaced by `{"tick":"CORRUPT-RECORD"}`.

| Consumer on the corrupt log | Before (`surface_before.txt`) | After (`surface_after.txt`) |
|---|---|---|
| `sanity_report_json` | exit 0, 3333 bytes of report | exit 1, nothing on stdout, message on stderr |
| `_native.sanity_report` | returned normally (`sc6_entropy_mean=0.0909…`) | `ValueError` |
| `_native.final_margins` | returned normally (4 firms) | `ValueError` |

On the clean log all three behave as before. Message after the fix:
`event log contains 1 unparsable persisted record(s); … no result is returned: tick 5, agent 0: corrupt record under `action_window` (invalid WindowEntry: …); offending record: "{\"tick\":\"CORRUPT-RECORD\"}"`.

## 4. Full suite summary (working tree)

| Suite | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 262 passed, 0 failed, 1 ignored; `golden_trace_unchanged` ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean (after the test rewrite above; the first run failed on `clippy::err_expect` in new test code) |
| `cargo fmt --check` | clean |
| `scripts/lint-architecture.sh` | passed |
| `check_deps.py` | exit 0 |
| `pytest python/tests` (binaries and extension rebuilt) | 134 passed, 0 skipped |

## 5. Claims, each with evidence

- **Predeclared check met: numerical output unchanged.**
  `hash_equivalence_output.txt` — HEAD (`2820610` export) vs an rsync copy of
  the working tree, event logs byte-equal on **31 of 31** configs, all seeds
  ≥ 910001: 5 seed sets each of the scheduled-shock, stochastic-shock and
  `decision.random` stage5 configs (120 ticks), the 15 `pre` probe cells,
  and the opt-in probe cell. Sensitivity control: scheduled vs stochastic on
  one binary is DIFFERENT.
- **`sc16_gate` numbers unchanged.** `sc16_gate_diff_vs_R07.txt` is empty
  (diff against the R07 printout).
- **Analysis output unchanged on a well-formed log.**
  `analysis_output_unchanged.txt`: the HEAD example binary and the
  working-tree example binary print byte-identical JSON for the clean
  fixture.
- **Every authorised site is loud.** Tests in §3; `h2_tests_after.txt`.
- **`parse_errors()` now has readers** in `sanity_from_run`,
  `try_sanity_from_run`, `reconstruct_checked`, the example binary and both
  `firma-py` functions. `surface_after.txt`.
- **What is left** (`remaining_silent_drops.txt`, a grep over non-test
  code): three sites, listed in §7.

## 6. What I did NOT verify

- A hosted CI run (nothing pushed).
- `firma-tui` with a malformed `scales` parameter: it calls
  `Reconstruction::new`, which now panics in that case. Read, not run.
- The rule-path panics end to end through the CLI. A corrupt persisted
  record cannot be injected through a config; R07's
  `rule_panic_end_to_end.txt` showed what any panic inside a rule does
  (exit 101, no manifest), and these use the same mechanism.
- The Python extension on any interpreter other than the repo's 3.14 venv
  this round.

## 7. Outside scope, noticed (not touched)

- **Separate finding, as requested: `orchestrator.rs:147` and `:174`.** After
  `registry.resolve_rule(spec)?` (and `resolve_resolver`) succeeds, the
  content hash is looked up by a second search and falls back to an empty
  string (`unwrap_or_default()`) if that search finds nothing. A manifest
  could then record a plugin with an empty content hash. Different class
  from the record drops; not changed.
- **`action-shaping/src/lib.rs:481`** (`resolve_lagged`): an unparsable
  lagged record is kept in the queue verbatim, forever, with no signal. Only
  `:248` was authorised in that file.
- **`firma-tui`** keeps using `Reconstruction::new` and does not read
  `Reconstruction::parse_errors()`; its own counter covers unparsable event
  lines only.
- `sanity_from_run` / `try_sanity_from_run` still panic if the log cannot be
  read or an event line does not parse (pre-existing `expect`s, documented).

## 8. Three things I am least sure about

1. **`Reconstruction::new` panicking on a malformed `scales`.** The prompt
   says "return an error"; `try_new` does. I kept `new` infallible in
   signature because `firma-tui` (not authorised) calls it, and made it panic
   rather than default. If a `Result`-returning `new` is wanted, `firma-tui`
   has to change with it.
2. **Rebuilding the git-ignored Python extension in the main checkout.** It
   was needed to test the `firma-py` change through Python, but it is a build
   product inside the repo tree.
3. **Whether "fail" or "report" is preferred for the analysis consumers.**
   I chose fail (no result at all on a corrupt log). A caller that wants a
   partial reconstruction plus a warning can still use `Reconstruction::new`
   and read `parse_errors()` itself.
