# R10 review of H-2 (R09 report), 2026-10-02
Builder report docs/review/outbox/R09-h2-report.md; builder evidence 21 files, sha256sum -c all OK. Reviewer re-runs: docs/review/evidence/R08/h2_recheck.txt (sha256 df08f499...), corrupt-log fixture run on my own build, unit tests of 6 crates.

## 1 Verdict: ACCEPT WITH FIXES
Code is correct and in scope. One instruction was not followed: Part A (evidence first) is absent from the report; I closed it myself by reading code (below). Not blocked on the owner.

## 2 Claims
| claim | verified? | how |
| 31/31 byte-equal HEAD vs working tree | REPRODUCED on fresh seeds | 15/15 (5 seed sets x scheduled/stochastic/decision.random, 200 ticks, seeds 910101-910105) + 3/3 stochastic at 400 ticks, all equal |
| corrupt log: example exits 1, Python raises | REPRODUCED (example) | regenerated fixture with their script on my build: clean exit 0 / 3333 bytes; corrupt exit 1 / 0 bytes with message naming tick, agent, key, record. Python side builder evidence only |
| 11 new tests fail before, pass after | PARTLY | tests in the six changed crates pass on my build (11,16,23,32,7,9 pass, 0 fail); the before-state is builder evidence |
| cargo test 262/1 ignored; pytest 134/0 skipped; clippy, fmt, lints, check_deps clean | UNVERIFIED by me | builder raw outputs checksum-verified; full suites include (n,n,n,n) tuples so I did not run them. Owner should run (command list in R10 chat) |
| sc16_gate unchanged | accepted | builder diff empty; my earlier printout matched pre-H-2 |
| three silent sites remain | VERIFIED | orchestrator.rs:147,174; action-shaping:~481; firma-tui not reading parse_errors |
| rule-path panics end to end through CLI | UNVERIFIED (builder says so too) | cannot inject a corrupt record through config |

## 3 Findings
F1 (process) Part A evidence not delivered: no reachability.md, no failed-job accounting. I did it:
 Reachability: every writer of action_window, lagged_effects, observed_env, env_history, active_shocks, relation_edges is a plugin pushing/replacing a string produced by serde from a typed value (observation:76,324; shock:280; action-market:218; shaping:334,494; constraint:176,505; effect.rs:127,137). No Intervention variant injects a raw list (none found in firma-core/intervention.rs). Config does not seed edges. So rule-path corruption is reachable only via a damaged snapshot (kernel restore, lib.rs:593) or a future code bug. The new panics are defensive guards, not expected in a clean run (consistent with 31/31 equal).
 Failed-job accounting: runner.py:339-342 marks any non-zero exit (including a panic, 101) status "failed" with reason; ExperimentResult.failed lists them; stats.invariant_exclusions (stats.py:828-838) excludes every non-"completed" job and reports counts and reasons. A panic and a conservation failure are treated identically. OK for 30.7 (counts reported), but see D-O.
F2 (hygiene) Reconstruction::new now panics on malformed `scales`; try_new returns the error. Acceptable; firma-tui (calls new) was out of scope. UNVERIFIED whether the engine already rejects a malformed scales at config load.
F3 (hygiene) builder ran `maturin develop` in the main checkout (git-ignored extension, python/firma_lab/_native*.so). No source impact; disclosed.
F4 (hygiene) remaining: orchestrator empty plugin hash fallback; resolve_lagged keeps an unparsable record forever; firma-tui ignores parse_errors. DIRECT-FIX (H-3, low priority).

## 4 Positive control / premise
Equality check could detect change (opt-in variant differs; scheduled vs stochastic differ). Premise: fixtures corrupt an event LOG (analysis input), not a live run's persisted state; the analysis-side guard is proven, the rule-side guard is proven by unit tests only.

## 5 Scope
Files changed all within authorisation (phase_rules, shock lib+tests, action-shaping lib+tests, reconstruct.rs, firma-analysis sanity.rs, example, firma-py, evidence/outbox). No ADR, manual, ci.yml, deny.toml, assertion, seed or threshold changed. New public API: Reconstruction::try_new, reconstruct_checked, try_sanity_from_run.

## 6 Owner decisions
D-M panic vs Result: no longer urgent; evidence shows panics are unreachable in clean runs. Keep panic (REC).
D-O (new) failed-job policy for the filing: (a) as now, exclude failed jobs and report counts/reasons; (b) treat any failed job as a stop-and-investigate event, because a panic or invariant failure correlated with outcome would bias an "exclude" analysis. 30.7 literally allows exclusion of invariant failures with counts reported, so (a) is manual-consistent; (b) is stricter. Falsifier for (a): failures cluster in particular cells. REC: (a) plus a pre-registered rule that more than a stated fraction of failures in any cell halts analysis (owner sets the fraction).
D-N deny.toml: parked (CI deferred).

## 7 Next builder prompt
S3 design-only: see R10 chat / below.

## S3 prompt (design only; DRAFT ADRs; evidence demanded)
"Read docs/review/BUILDER.md, STATE.md, newest inbox files (R08-R10). Task S3, DESIGN ONLY. Authorised: new files docs/adr/0058-*.md, 0059-*.md, 0060-*.md (all Status: DRAFT, owner approves each), docs/review/evidence/R10b/*, docs/review/outbox/R10b-s3-report.md. NO source, test, config, manual, or existing-ADR edits. Diagnostics: DIAGNOSTIC-ONLY, seeds >= 910001, scratch builds only.
Write three DRAFT ADRs. For every open parameter value write 'OWNER TO SET' with the candidate range; never choose a default. Every factual claim cites file:line or an evidence file.
ADR-A 'Between-seed variance in E1' (D-C). Established already: decision.satisficing draws no randomness (decision/lib.rs:1024); the init stream has no consumer (kernel/world.rs:34; stream_for_phase maps only Environment); environment noise (resource.patchy) changed logs but not action sequences in 3 regimes; shock.stochastic changes actions and survival (23 distinct action hashes of 50 seeds; evidence R06). Arm A has pins and no shock. Deliver: (1) options: accept deterministic Arm A and state what the 200 seeds then mean; per-agent initial-condition heterogeneity drawn from the init stream (say exactly which rule/phase would draw, how it stays kernel-pure A1, purpose_tag per 21.2, effect on DT-6 and stream isolation); any other source you can evidence; (2) for each: assumptions, effect on H1a/H1b/H1c/H2/H4, what would falsify it, and which is most consistent with manual 21.3/30.5 (not which flatters a hypothesis); (3) evidence: for the heterogeneity option, a throwaway diagnostic prototype OUTSIDE the repo (scratch copy) showing whether init-drawn differences reach the action sequence in an Arm-A-like pinned run; report distinct action hashes over 50 seeds with a negative control (no heterogeneity -> 1 hash).
ADR-B 'Arm A registration' (D-D). Deliver: (1) which DV H1a/H1b are registered on, with the code path from varsigma to each candidate DV (w_eff depends on h only: manual 12.3 Step 3); (2) the mapping of the single swept varsigma onto the three varsigma_j: options A (uniform) and B (varsigma_1 only, others 0.0 as VT-8) per ADR-0054, plus the unit mismatch (pinned varsigma in [0,1] vs integer capital change, audit F04a) with every distinct meaning of pinned h in the SURVIVAL lookahead (decision/lib.rs, list the lines); (3) a manipulation check: define the quantity measured and offer candidate thresholds as options for the owner, fixed BEFORE running; (4) evidence: run the 25 (h, varsigma) cells at one beta on the ad hoc one-firm synthetic template, DIAGNOSTIC-ONLY, and report per cell action-sequence hash and H_rep, then group cells by identical behaviour (audit F04 predicts 3 groups); state the template's limits.
ADR-C 'E1 base config skeleton' (F05). Deliver the list of every field the model_config_template must supply (population size and heterogeneity, plugin set, shock definition per arm, horizon, resource/observation plugins, initial stocks), mapped to the manual 30.4 factors that firma_lab.runner.build_job_config patches; mark each field specified-by-manual / specified-by-ADR / UNSPECIFIED. No values for UNSPECIFIED fields. Split into the smallest independently reviewable sub-decisions.
Stop and ask if you need a choice the manual does not specify; put the question in the report with the evidence that would settle it. Report negatives plainly. No commits. Digest in chat: table of named results; raw output in evidence with sha256. Stay silent between tool calls. List the 3 things you are least sure about."
