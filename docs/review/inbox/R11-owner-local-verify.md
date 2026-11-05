# R11 owner's local verification (pasted output, 2026-10-02) and a correction to STATE
Source: owner's terminal output; I did not run it. Machine: owner's, Python 3.14.7, pytest 9.1.1.
## Results (read from the paste)
- cargo test --workspace: summed per-binary results = 262 passed, 0 failed, 1 ignored (sc16_search, exploratory). Matches the builder's R09 report exactly.
- DT: determinism.rs 9/9 (DT-1..6 incl. adr0054 RNG non-interference); golden_trace_unchanged ok; validation.rs 10/10 (VT-1..8 names all present); sanity.rs 4 passed (h3_channel_opens_in_a_real_run, sc16_gate, sc4_wmax_beta_probe, sc16b_arm_scoping) + 1 ignored; integration 11/11; new H-1/H-2 panic tests all present and ok.
- clippy -D warnings: two "Finished" lines, no warnings. fmt --check: no output (pass by silence; exit code not shown). check_deps.py: no output shown (exit code not shown). lint-architecture: 9/9 ok; the no-magic-numbers list is the known heuristic warning, mostly test literals (non-test literals worth a named-constant review: firma-plugin-constraint/src/lib.rs:83,320; firma-rng/src/lib.rs:243).
- builds: firma-cli and the sanity_report_json example build. pytest: 134 passed, 0 skipped, 59 s.
## What this does and does not show
Shows: the working tree builds and every committed test passes on the owner's machine, reproducing the builder's counts. Does not show: that the science is right (sc16_gate and sc16b_arm_scoping pass BY asserting the Phase-2 gate fails; VT-3/4/8 premise issues stand), nor that CI would pass.
## Correction to STATE (my error)
STATE said sc4_wmax_beta_probe "fails 4/400 at HEAD". At HEAD 2820610 the test was already restructured per ADR-0057 Option B (tests/tests/sanity.rs, a1/a2 split; changed between c5eb8cd and HEAD) and passes. The restructure documents the lookahead asymmetry as a known current property, not an accepted design; ADR-0057 is still DRAFT and says Option B should FOLLOW the decision on Option A. So D-A stays open, but its "Option B" half is already committed. I had treated the c5eb8cd builder claim as current without re-checking HEAD.
