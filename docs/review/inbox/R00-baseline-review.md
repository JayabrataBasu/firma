# R00 baseline review
Date 2026-10-01. Evidence: docs/review/evidence/R00/r00-git-and-doc-checks.txt

## 1 Verdict
ACCEPT WITH FIXES (baseline only; no builder work reviewed). Flag: BLOCKED ON OWNER for D-A..D-I before any E1-affecting work; uncommitted work is at risk (see F2).

## 2 Claims table
| claim | verified? | how | note |
|---|---|---|---|
| HEAD pushed as 2820610 | VERIFIED | git ls-remote | |
| master = 9c49c34 | VERIFIED | git branch -vv | |
| E1 195 cells / 39,000 runs (125+60+10) | VERIFIED | ADR-0051 table | |
| Manual 30.4 says 315/63,000 | VERIFIED | MANUAL.md:2001 | OQ-23 |
| MANUAL_VERSION stale | VERIFIED | firma-core lib.rs:42 = 1.0.0, manual 1.1.0 | F08 |
| ADR 0055/56/57 DRAFT | VERIFIED | Status lines | |
| ADR-0040 Status not annotated | VERIFIED | Status line | T3 |
| No filing evidence; 30.9 unticked | VERIFIED (absence) | grep, manual | |
| 26.5 gate / 26.6 Platform | VERIFIED | MANUAL.md:1775-1790 | |
| CI lacks pytest job | VERIFIED | grep -c = 0 | |
| CLAUDE.md status stale | VERIFIED | line 24-26 | |
| 28.4 names "two null conditions" | CONTRADICTED | 28.4 lists five nulls | D-B lean withdrawn |
| Probe 4/400 at HEAD; Option A 0/400; A-sym identical; five H3 tests fail; 4 frozen hashes; Batch 1 counts (237/134); audit F01-F20 contents | UNVERIFIED | not run in R00 (cheap checks only) | verify in R01 |
| T4 control was insensitive | UNVERIFIED | inferred from builder text | |

## 3 Findings
| ID | sev | finding | evidence | class |
|---|---|---|---|---|
| F1 | hygiene | CLAUDE.md is gitignored; the review-protocol block (and project instructions) are not in the repo, so a builder in a fresh clone never sees them | .gitignore:9 | OWNER-DECISION (whether to track it) |
| F2 | correctness (process) | 11 modified files + test_run_safety.py + prototype worktree diff (+46/-3) are uncommitted and unbacked-up | git status, worktree diff | OWNER (D-I) |
| F3 | hygiene | draft-outline-e1-base-config-ADR.md lives under docs/adr/evidence/adr-0057/prototype-symmetric/, unrelated to ADR-0057 | find | DIRECT-FIX (move; small) |
| F4 | hygiene | STATE D-B premise wrong (five nulls in 28.4) | MANUAL.md:1895-1903 | corrected in STATE |
| F5 | E1-blocker | MANUAL_VERSION / spec-hash versioning unresolved | lib.rs:42 | OWNER (D-F) |

## 4 Positive-control and premise checks
None run in R00. Required in R01: T4 (e_h value differences with full window ending in produce_regulated), and the sc4 probe re-run at seeds >= 910000 only.

## 5 Scope and side effects
Reviewer wrote only docs/review/* and (per Phase A instruction) appended to the gitignored CLAUDE.md. Nothing committed. No tests or simulations run.

## 6 Owner decisions needed
D-A..D-I as in STATE.md. New: D-J whether to track CLAUDE.md; D-B needs a fresh look (see F4). Most urgent: D-I (back up/commit; owner acts).
No option ranking is offered in R00; D-A needs the R01 sensitivity control first.

## 7 Next builder prompt (proposed; one task, no OWNER-DECISION)
"Read docs/review/BUILDER.md, STATE.md. Task R01-verify-claims: in a scratch worktree of HEAD 2820610, re-run (a) cargo test --workspace, clippy -D warnings, fmt --check, check_deps.py, pytest; (b) the sc4_wmax_beta_probe at seeds >= 910000 only, DIAGNOSTIC-ONLY, and report failures per (w_max,beta). Predeclared: report counts exactly, no interpretation. Authorised files: docs/review/outbox/*, docs/review/evidence/R01/* only. Do not edit source, do not run seeds 1-200, no commits. Stop and ask on any manual-unspecified choice. Raw output to evidence with sha256; digest table in chat. Stay silent between tool calls. List the 3 things you are least sure about."

## 8 Least sure about
1. Whether the builder-reported probe/prototype numbers hold; entirely unchecked.
2. Whether the phase-status in STATE ("Phases 0-2 complete") matches PROGRESS.md; PROGRESS shows Phase 2 with unmerged H3 revision and H3 untestable, so "complete" may overstate.
3. Whether ADR-0055's reading of the Arm C prose (S1 vs S5) is sound; I only read the manual line, not the ADR's full argument.
