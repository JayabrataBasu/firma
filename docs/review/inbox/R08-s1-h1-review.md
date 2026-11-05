# R08 review of S1 (variance reach) and H-1 (hygiene), 2026-10-02
Reports: docs/review/outbox/R06-s1-report.md, R07-h1-report.md. Reviewer re-runs: docs/review/evidence/R08/reviewer_recheck.txt (sha256 9a207a53...). Builder evidence checksums (R06 7 files, R07 23 files) re-verified with sha256sum -c: all OK. Scratch builds of HEAD and of the working tree (rsync) in the session scratchpad; seeds >= 910001.

## 1 Verdict
S1: ACCEPT. H-1: ACCEPT WITH FIXES (follow-up H-2; two owner decisions parked). Nothing blocks on the owner for the next builder step; D-C/D-D/D-B/D-K/D-F remain open.

## 2 Claims table
| claim | verified? | how | note |
|---|---|---|---|
| S1: 23 distinct action hashes of 50, deaths 27 vs 23 | VERIFIED | my own 50-seed re-run, own script, HEAD build: 23 distinct, {3 deaths:27, 1 death:23} | identical numbers |
| S1: negative control gives 1 hash per fixed onset | VERIFIED | 3 onsets x 10 seeds -> 3 hashes total (one per onset) | consistent |
| S1: variance is largely a survival effect? (builder's worry) | REFUTED by me | within the 27 three-death runs 18 distinct action hashes; within the 23 one-death runs 5 | decisions differ beyond survival |
| S1: H_rep range 0.480-0.825 sd 0.1161; random 1.254-1.418 | UNVERIFIED by me | needs sanity_report_json; scripts present, checksums OK | metric is run-level mean over live firm-ticks, not DeltaH_rep |
| S1: inertness 9c49c34 vs HEAD 20/20 equal | PARTLY | I ran 15/15 + 3/3 earlier (R05/R06) | consistent |
| H-1: numerical output unchanged | VERIFIED | HEAD vs working tree: 15/15 (scheduled, stochastic, decision.random, 5 seed sets) + 15/15 probe cells byte-equal; sc16_gate numbers identical to my pre-H-1 run; 63 unit tests in the four changed crates pass | builder: 23/23 |
| H-1: nine drops now loud | VERIFIED by reading the diff (decision lib:362,385,411,1027,1205; phase_rules:57,152; observation:290) and unit tests pass; before/after failing runs are builder-evidence | one helper each, messages name agent/key/record |
| H-1: files changed within authorisation | VERIFIED | git diff --stat vs authorised list: ci.yml, decision, constraint, observation, reconstruct.rs, their test modules, sanity.rs only | test_runner.py/spec.py diffs are Batch 1 (SYNTHETIC_FIRST_REPLICATE), not H-1 |
| H-1: sanity.rs diff only comments/strings | VERIFIED by reading output | sc16_gate lines now say SC-3 NOT achieved, compliance false | numbers unchanged |
| deny failure = deny.toml:25 `wildcard-dependencies` | PARTLY | line 25 confirmed in file; cargo-deny not installed here; builder's raw 0.20.2 output read | UNVERIFIED by me independently |
| CI comment cites run 33902141014 failing 2026-09-04 | VERIFIED | gh run view: conclusion failure, 2026-09-04T17:44:05Z | |
| hosted CI will show format/lint/test/python pass, deny fail | UNVERIFIED | needs a push | prediction only |
| rule panic -> exit 101, no manifest, runner records failed job | UNVERIFIED by me | builder evidence rule_panic_end_to_end.txt | |
| six drop sites remain outside authorisation | VERIFIED | grep: phase_rules.rs:347, shock lib:93/243/472, action-shaping:248, reconstruct.rs:286 and :462 | see F2 |

## 3 Findings
| ID | sev | finding | class |
| F1 | correctness (E1-adjacent) | S1 shows seed variance reaches actions and outcome in a shocked 4-firm config via the shock stream only; Arm A still has none. In that config firm 2 dies at tick 12 in every run (degenerate population, cf. F12) and 14/50 seeds share one action sequence. H_rep here is not DeltaH_rep. Not an E1 base config. | evidence for D-C; no fix |
| F2 | correctness | Enforce (phase_rules.rs:347) drops unparsable relation edges and may rewrite the edge list without them (a silent delete of persisted state); shock plugin and shaping plugin drops; reconstruct.rs:286 silently defaults a malformed `scales` param (a silent default, rule 4); reconstruct.rs:462 skips unparsable lagged records. | DIRECT-FIX (H-2) |
| F3 | hygiene | parse_errors() has no reader: sanity_from_run, firma-py, sanity_report_json never check it, so a corrupt log still yields a number. | DIRECT-FIX (H-2) |
| F4 | hygiene | orchestrator.rs:147,174 record an empty plugin content hash when a plugin is not found (silent provenance gap). Different class from F2. | DIRECT-FIX, separate small task |
| F5 | E1-blocker (CI only) | deny job will still fail: unknown key, 21 versionless workspace path deps, one licence (Apache-2.0 WITH LLVM-exception via pyo3/target-lexicon). | key rename = DIRECT-FIX; wildcard-path policy and licence allowance = OWNER-DECISION |
| F6 | hygiene | ci.yml hard-codes the branch name; adds third-party actions taiki-e/install-action and actions/setup-python. | note; owner may prefer a pattern |
| F7 | process | uncommitted work grew again (H-1 +~300 lines); D-I backup still open. | OWNER |

## 4 Positive-control and premise checks
Variance: negative control (fixed onset, sd 0) = 1 hash; positive control (decision.random) many; my within-survival-group split tests the confound. Inertness H-1: sensitivity shown earlier by the opt-in config changing the log; here three plugin families compared. Premise: stage5-smoke population loses firm 2 at tick 12 whatever the shock, so "variance reaches the outcome" is shown for an unhealthy population only.

## 5 Scope and side effects
Authorised file set respected. No ADR, manual, threshold, seed, or assertion changed. New public API: reconstruct.rs parse_errors() (no reader yet). New behaviour: rule panics on corrupt records (previously silent). No hashes changed. Builder deleted its own scratch copies after a tmpfs quota hit; outputs in evidence intact.

## 6 Owner decisions needed
D-M panic vs Result in rule paths. Panic follows 19.5 and ADR-0054 precedent and needs no API change; Result needs a Rule::apply signature change (ADR, WASM-boundary cost per 20.2). Most consistent with the manual: panic. Falsifier: a legitimate run where a record is malformed and execution should continue (none found in 30+ configs run).
D-N deny policy: (a) rename key to `wildcards`; (b) versionless workspace path deps: `allow-wildcard-paths = true` or add versions; (c) allow Apache-2.0 WITH LLVM-exception. (a) is mechanical; (b),(c) are licence/supply-chain policy.
D-C update: Arm B has a verified, reaching variance source (shock stream); Arm A has none. Options unchanged: accept deterministic Arm A, or new variance mechanism by ADR.

## 7 Next builder prompt: H-2 (see chat). 
## 8 Least sure about
1 H_rep numbers and the panic end-to-end claim rest on builder evidence.
2 Whether F2's Enforce edge drop is reachable in a real run (only through a corrupt store).
3 Hosted CI behaviour (no push yet).
