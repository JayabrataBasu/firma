# R04 resolution plan (reviewer; no code run; nothing here is a guarantee)
Honesty first: several blockers are scientific decisions (variance source, Arm A design, gate reading). Those cannot be solved by implementation, and the protocol (REVIEWER.md: one scoped task per prompt, design before code, owner approves each ADR, 3-round cap) forbids a one-shot. This plan sequences the work so each builder prompt is safe and complete. Recommendations are marked (REC) and chosen for consistency with the manual/ADRs, not for the result they produce.

## Problem -> fix map
| # | Problem | Fix | Needs owner? |
|---|---|---|---|
| P1 | No E1 base config (F05) | Draft base-config ADRs, then build template | yes (approve ADRs) |
| P2 | No between-seed variance (F01) | New variance-source mechanism ADR, then implement | yes (D-C) |
| P3 | Arm A may not identify H1a/H1b (F04, R03 F-A) | Register DV, varsigma mapping, manipulation check with thresholds fixed first | yes (D-D) |
| P4 | Gate: SC-1,2,3,6 never shown jointly on a satisficing config (F12) | Joint SC run on the real E1 base config, diagnostic seeds >= 910000, thresholds predeclared; then amend 16.2/27.3 (MAJOR + ADR) | yes (D-K) |
| P5 | Arm C count (D-B), spec cannot represent it | S1 (187) REC; implement ArmDesign blocks per ADR-0055 | yes (D-B) |
| P6 | H3 revision (ADR-0047-49) regression, asymmetry (D-A) | Prove inert when unconfigured; run E1 without opt-in (D-L); defer D-A to H3 filing | yes (D-L) |
| P7 | Missing deliverables: DeltaH_rep forks, per-run metrics, AT-1..5 | Separate builder tasks after P1-P3 | no, after specs |
| P8 | Hygiene (CI, silent drops, skipped tests, false comments) | Direct fixes | no |
| P9 | Versioning (D-F, D-G): MANUAL_VERSION, model golden trace, spec hash | One batch, once, before filing | yes (D-F) |
| P10 | Uncommitted work (D-I) | Owner commits/pushes both worktrees | yes |

## Recommended decisions (REC; owner decides)
D-B: S1 (Arm C = satisficing@beta=0 + random; 187 cells, 37,400 runs). Reason: only reading where every Arm C cell is a null (ADR-0055, 30.4 prose, 28.4). Trade-off: beta sweep inside Arm C is lost (Arms A/B already sweep it).
D-L: yes, if S1-task shows byte-identical logs. Reason: H3 is out of E1 (ADR-0051); boring version (A7).
D-C: decide after the S3 options paper. Constraint I can state now: Arm A has pins and no shock, so shocks cannot supply variance there; variance must come from the init stream (manual 21.3 lists it) via per-agent heterogeneity, and Arm B can add shock.stochastic timing. This is a new mechanism: ADR first. I am not certain which option the kernel supports; do not assume.
D-D: register H1a on repertoire entropy (H_rep); report w_eff but label it mechanically independent of varsigma (12.3 Step 3). Use ADR-0054 Option B (varsigma_1 swept, others 0.0, as VT-8). Manipulation check = predeclared thresholds (e.g. number of distinct behavioural groups across the 25 cells must be >= a number fixed BEFORE running); owner picks the number.
D-K: do not amend the manual until P4's joint-SC run on the real base config exists. If it passes, amend 16.2/27.3 to the per-hypothesis wording (MAJOR + ADR). If it fails, 26.4 says revise the model, not tune.
D-F: bump MANUAL_VERSION to the manual's version, add a model-level golden trace, settle spec-hash fields (first_replicate) in ONE change before any hash is filed.

## Sequence (each step: builder task -> reviewer round -> owner gate)
S0 Owner: commit/push both worktrees (D-I). Not an agent task.
S1 Builder (read-only): R03-inertness-check (prompt in R03 section 7).
S2 Builder: hygiene batch H-1 (below).
S3 Builder (DESIGN ONLY, DRAFT ADRs, no code): options papers for D-C and D-D, and a skeleton E1 base-config ADR set split into small DRAFTs (shock definition, population/heterogeneity, rest). Each lists options, assumptions, effect on H1a/b/c/H2/H4, falsifiers.
GATE: owner accepts/edits ADRs and decides D-B, D-C, D-D, D-K(procedure), D-L, D-F.
S4 Builder: implement ADR-0055 blocks + chosen Arm C; update the two cell-count tests (they must fail first).
S5 Builder: build model_config_template + variance mechanism per accepted ADRs; failing test first for each.
S6 Builder: joint-SC diagnostic on the base config (seeds >= 910000, DIAGNOSTIC-ONLY), thresholds predeclared; report as is.
S7 Builder tasks (one each): ΔH_rep via fork runs; per-run metrics through _native; AT-1; AT-2; AT-3; AT-4; AT-5.
S8 Owner: D-F batch; manual patches (30.4, 16.2 if S6 passes); ADR acceptance/status lines.
S9 Pre-filing: 30.9 checklist, content hashes, exploratory-run disclosure (incl. R03 F-D), independent re-implementation (25.8).

## Prompt for S2 (hygiene H-1) -- ready to hand to the builder
"Read docs/review/BUILDER.md, STATE.md, newest inbox file. Task H-1. Authorised files: .github/workflows/ci.yml; the code sites that silently drop unparsable records (find: from_json / unwrap_or_default / filter_map(ok) in firma-analysis and firma-io; list them first in the report before changing any); python/tests skip markers; tests/tests/sanity.rs COMMENTS ONLY (false fixture comments named in docs/audit/AUDIT-2026-10-01.md section 2); docs/review/outbox/*, evidence/R05/*. For each fix: write a failing test or reproducible failing command first, raw before/after output. CI: add a pytest job that builds the CLI so the 14 skipped tests run, make deny.toml discoverable, trigger on this branch; do not claim CI green unless you saw it run. Silent drops: make them loud (error or counted warning in the output) without changing numerical results; prove with a test fixture containing one bad record. Must not change: any numerical output, any ADR, the manual, thresholds, seeds. No commits, no seeds 1-200. Predeclared: any change that alters a frozen hash is reverted and reported, not kept. Stop and ask if a manual-unspecified choice arises. Report to docs/review/outbox/R05-h1-report.md. Stay silent between tool calls. List the 3 things you are least sure about."

## What Opus can safely take now
S1 and S2 (no owner decision involved). S3 is safe as design-only. Do NOT give Opus S4-S7 until the GATE.
## Least sure about
1 Whether the init stream is actually wired to any per-agent randomness in the current kernel (unchecked).
2 The audit-sourced hygiene list (builder-written, not re-verified).
3 Whether S1 (inertness) passes; if not, D-L fails and D-A returns to the critical path.
