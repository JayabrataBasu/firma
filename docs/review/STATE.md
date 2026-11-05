# STATE (reviewer-owned)
Round: R24 (R23 reviewed; slice-3 prompt written) | Source: owner handoff + reports; only 'Verified facts' is verified; 'Claims reported by builders' remain UNVERIFIED
Branch: h3-satisficing-lookahead (master 9c49c34). HEAD 2820610 (VERIFIED, pushed; origin/h3-satisficing-lookahead = 2820610).

## Hard stops (unchanged unless owner edits)
No commits/pushes by agents. No reserved seeds 1-200. No E1 registered-design
run. No tuning. ADR bodies immutable. Owner approves each ADR.

## Project plan (from manual 26; confirm against docs/MANUAL.md)
Phases 0-2 complete. Phase 3 (E1 first result) current; gate: H1a/H1b/H1c
answered either way, robust across 3 rigidity + 2 novelty measures + AT-1..5;
only an uninterpretable sweep fails. Phase 4 "Platform" (WASM plugin host,
regulator.strategic, multi-firm rivalry, decision.optimizing...) not started.
E1 narrowed (ADR-0051) to H1a/H1b/H1c/H2/H4 = 195 cells / 39,000 runs
(Arm A 125, Arm B 60, Arm C 10 or 6 or 2 per ADR-0055). H3 is open (ADR-0050),
not in E1's filing, not on the Phase 3 critical path. No real run executed.

## Open tasks
T1 Baseline R00 verification of this file (reviewer).
T2 Arm A manipulation: F04 diagnostic found 25 (h, varsigma) cells collapse to
   3 behavioural groups. Next: builder digest on primary-outcome definitions,
   unpinned sc[j-1]/varsigma units, three meanings of pinned h in the
   SURVIVAL lookahead (judge only on unit consistency), manipulation-check
   design with thresholds fixed first. No implementation yet.
T3 Hygiene batches: CI/deny.toml and branch trigger and pytest job; silent
   from_json drops; silently skipped Python tests; false comments in
   sanity.rs; the sc4 select-level test stubs satisfices=false; VT-4 static
   state; stale figures (manual 30.4 says 315/63,000); ADR-0040 Status line;
   CLAUDE.md status; firma-analysis unit tests; partial seed-invariance
   reporting in stats.
T4 A-sym positive control: shadow run showed 0 disagreements over 9,032
   a>=6 records and identical event logs A vs A-sym, but chosen ticks had
   u=0 under both, so the control was insensitive. Needed: e_h value
   differences (not verdicts) and a unit-test fixture with a full window
   whose oldest entry is produce_regulated.
T5 Draft E1 base model_config_template ADRs (outline exists:
   draft-outline-e1-base-config-ADR.md); split into small DRAFTs; shock
   definition, population/heterogeneity, rest; sensitivity sweeps fixed
   before outcomes.

## Pending OWNER decisions
D-A Lookahead (ADR-0057): adopt Option A (market actions only), A-symmetric
    (all action classes, same advance_window/u_from_window), or keep HEAD
    plus Option B test hygiene with the asymmetry documented as known not
    intended. Must precede E1 filing (affects model/code version hashes).
D-B ADR-0055 Arm C count: S1 = 2 cells (187 total / 37,400 runs; matches the
    two null conditions named in manual 28.4) vs S5 = 6 cells (191 / 38,200).
    Reviewer leans S1: ADR-0055 and 30.4 prose support it textually (R03 restored the lean).
D-C Source of between-seed variance in E1 (F01) and heterogeneity.
D-D Arm A: map of normalised varsigma into engine units; meaning of pinned h
    in the SURVIVAL lookahead; whether the manipulation check becomes a
    pre-registered gate.
D-E NONE-focus inertia (F03): check admissibility? window records selected or
    executed actions? Decide jointly with D-A and legitimacy default (F07).
D-F Versioning: MANUAL_VERSION "1.0.0" vs manual 1.1.0; decision plugin
    version/content hash unchanged across ADR-0047-0049 code; adding a
    model-level golden trace. Both change hashes; decide together.
D-G OQ-24: ExperimentSpec.first_replicate added, changes every spec hash
    (harmless until a hash is filed).
D-H F12/F10/F16 and ADR-0056/0057 acceptance, ADR-0040 annotation,
    manual 30.4 patch (OQ-23).
D-I Backup/commit of uncommitted work (owner).
D-L (R03) Run E1 with ADR-0047-49 opt-in mechanism unset and defer D-A to the H3 filing; needs inertness check (UNVERIFIED).
D-M (R08) panic vs Result in rule paths (REC panic). D-N deny.toml policy: key rename, versionless path deps, Apache-2.0 WITH LLVM-exception.
D-O (R10) failed-job policy for filing: exclude+report (REC with pre-registered halt fraction) vs stop-and-investigate.
D-K (R02) Phase 2 gate: amend manual 16.2/27.3 (MAJOR + ADR) to per-hypothesis, or treat Phase 2 as unpassed and revise model per 26.4.

## Claims reported by builders (UNVERIFIED until R00/next rounds)
- HEAD c5eb8cd probe sc4_wmax_beta_probe fails 4/400 at (w_max,beta)=(6,0.0)
  and (9,0.0); 0/400 elsewhere; pre-17ba7bb config gives 0/400 everywhere.
- Option A prototype (20 lines, SURVIVAL branch, a<6 only): probe 0/400;
  cfg_arm_b_satisficing event log identical to HEAD, lobby reached 75 -> 42;
  five H3 tests fail (h3_channel_opens_in_a_real_run and four ADR-0047/0048
  unit tests, all an empty-window seeded-u fixture); four frozen hashes
  unchanged; clippy/fmt/lints clean.
- A-symmetric prototype: identical to A on all 35 existing-scenario runs.
- Lobby payoff holds u fixed in both h_success and h_failure; advance_window
  treats lobby like any non-regulated action (PRODUCE_REGULATED == 2).
- Audit (docs/audit/AUDIT-2026-10-01.md) findings F01-F20: F01 satisficing
  runs are seed-invariant unless shaping commits or a stochastic plugin is
  configured; F02 stats return "supported" with zero-width CI on invariant
  data (fixed in Batch 1); F03 NONE inertia repeats a non-executed action
  that enters the window; F05 narrowed E1 spec cannot be materialised (5
  Arm C KeyError, no template, no DeltaH_rep, no AT-1..5); F06 no seed guard
  (fixed in Batch 1 for run_experiment only); F07 legitimacy default 1.0 in
  decide vs 0.0 in act_shaping; F08 MANUAL_VERSION stale; F09 no frozen trace
  covers model plugins; F10 trace module reads env and writes files; F12
  degenerate existing regimes; F13 CI never green; F14 large uncommitted work.
- Batch 1 done: run_experiment seed guard; four fixtures moved out of
  1-200; stats refuse zero-variance data; multi-agent Arm A rejected; 15 new
  tests; cargo 237 pass/1 ignored; pytest 134 pass.
- No OSF/pre-registration filing evidence found in repo; manual 30 is DRAFT,
  all six 30.9 boxes unticked. Owner does not know if anything was filed.
- Manual 26.5 text (Phase 3 gate) and 26.6 (Phase 4) as paraphrased above.

## Verified facts (R11 add: owner local run: cargo test 262 passed/0 failed/1 ignored; pytest 134 passed/0 skipped; lints 9/9; clippy clean; fmt and check_deps silent (exit codes not shown). R10 add: H-2 numerics unchanged on fresh seeds (15/15 at 200 ticks, 3/3 at 400); corrupt-log guard reproduced (example exit 1); all persisted-record writers serialise typed values via plugins, no intervention injects raw lists, config does not seed edges, so rule-path corruption reachable only via damaged snapshot restore; failed jobs: runner marks failed, stats.invariant_exclusions excludes and reports (stats.py:828); Part A evidence was NOT delivered by the builder (reviewer closed it). R08 add: S1 reproduced (23/50 distinct action hashes; within-survival groups still differ); H-1 numerics unchanged (HEAD vs working tree 15/15+15/15; sc16_gate identical); 9 drops loud; 6 sites remain (phase_rules:347, shock x3, shaping:248, reconstruct:286,:462); deny fails at deny.toml:25; parse_errors() has no reader. R06 add: shock.stochastic varies actions (6 distinct of 8 seeds) and deaths; inertness also 3/3 on shocked 4-firm run; pytest working tree 120 passed/14 skipped (CLI absent); sanity.rs:132-135 comment and :549 string false (sc16_gate output); evidence R05/{shocked,hygiene}.txt. R05 add: init stream has no consumer; decision.satisficing draws no randomness (decision/lib.rs:1024); resource.patchy noise changes logs but NOT action sequences in 3 regimes; inertness 15/15 byte-equal 9c49c34 vs HEAD on unconfigured probe config, control OK (evidence/R05/inertness.txt); hygiene sites and CI gaps confirmed. R03 add: spec.py build_narrowed_e1_spec gives A125/B60/C10=195 cells, 39,000 runs at 200 reps; manual 30.4 arithmetic 315/63,000; S1=187/37,400, S5=191/38,200 are arithmetic only, not implemented. ADR-0054 has no owner decision on the varsigma mapping.)
## Verified facts (R00; evidence docs/review/evidence/R00/r00-git-and-doc-checks.txt)
- HEAD 2820610 on h3-satisficing-lookahead; origin has the same sha; master 9c49c34 local = origin.
- Uncommitted: 11 tracked files modified (+276/-26) and untracked python/tests/test_run_safety.py + docs/review/. NOT backed up anywhere except the working tree.
- Second worktree ../firma-adr0057-proto (branch adr0057-survival-lookahead-u-fix, at c5eb8cd) has an UNCOMMITTED +46/-3 diff in firma-plugin-decision/src/lib.rs (the Option A prototype). Not backed up.
- CLAUDE.md is gitignored (.gitignore:9). The Review protocol block added in Phase A exists only locally, and CLAUDE.md is therefore not shared via the repo.
- ADR 0055, 0056, 0057 Status = DRAFT; 0050-0054 Accepted. ADR-0042 Status already points to 0056. ADR-0040 Status has NOT been annotated for 0056/0057 (T3/D-H still open).
- MANUAL_VERSION = "1.0.0" in firma-core/src/lib.rs:42; manual header is v1.1.0 (F08/D-F confirmed).
- Manual 30.4 still says 315 cells / 63,000 runs (OQ-23 confirmed). ADR-0051 table: 125+60+10=195, 39,000 runs.
- Manual 30.9 boxes unticked; no OSF filing evidence in repo (owner still unsure).
- Manual 26.5 gate text and 26.6 "Phase 4 - Platform" match STATE paraphrase.
- Manual 28.4 lists five nulls (random decision, no narrowing, no-shock fork, frozen constraints, shuffled shocks); NOT "two". See D-B note.
- CI: .github/workflows/ci.yml has zero mentions of pytest (T3 pytest-job gap confirmed); deny.toml exists at root.
- CLAUDE.md "Current status" still says Phase 1 not started (stale; T3).
- draft-outline-e1-base-config-ADR.md exists but only at docs/adr/evidence/adr-0057/prototype-symmetric/ (odd location for an E1 base-config outline).

## Retracted / corrected claims
- R24: R23 result: P4 trace holds (blind spot hit twice at l_w=4; at l_w=8 firms enter survival focus first); P5 holds by its rule but the Phase-2 gate verdict is UNCHANGED by l_w (cfg_threaded identical at 4 and 8; cfg_healthy <0.05) so 'gate failure came from l_w=4' is NOT supported; P6 falsified: at l_w=8 compliance binds in 0/800 baselines, T1 0/800. SC scope: manual 16.2 scopes only SC-1 to no shock; all searches evaluated every SC on baselines (my T1 definition). Seeds: builder skipped sanity.rs configs with small seeds (conservative).
- R23: blind-spot hypothesis CONFIRMED by R22 (P1-P3 hold; edges match strict inequalities). Consequence: all regime searches R16-R21 and sanity.rs cfg_threaded used l_w=4 vs manual default 8 (cfg_healthy and cfg_wide_search already use 8); the Phase-2 gate failure (F12/ADR-0044 premise) may be an l_w=4 artefact: untested, R23 part 2. At l_w=8: no cliff, but compliance never binds at theta_limit 0.90 (SC-3 0/300).
- R22: R21 reproduced checks: compliance deaths occur iff drawn magnitude in [0.1507,0.2451] (none of the 43 others inside). Edges bracket theta_new = 0.75 and 0.65 = 0.5 + h_crit, i.e. u steps of 1/L_W (0.25) exceed h_crit (0.15); smoke config uses l_w=4, manual default is 8. Hypothesis, not yet tested; R22 prompt has predictions P1-P3.
- R21: R20 heterogeneity failed all criteria; same 57/100 seeds have compliance deaths at all three spreads (verified); Q5: drawn firms select the fixed firm's sequence. Untested levers: relative aspiration spread (my prompt was ambiguous), shock channel (only regulatory used; channel is unspecified by manual 30.4). ADR-0058 DRAFT does not support itself.
- R19: grid-2 'passing' cells (6/288, one regime) are a synchronised cliff (10 or 1 survivors; nine firms die in the same tick), not stressed-but-alive; T2 (mean only) was too weak. Reproduced independently. Next: choose how heterogeneity is introduced (config-level draws from init seed, or targeted shocks).
- R17: regime search grid-1 found 0/72 stressed-but-alive; grid omitted price ratio, obligation, population mixtures; SC-2 clusters track the fraction of low-capability firms and theta_limit, capital irrelevant. Not proof of non-existence. slice script bug: KeyError 'cause' when no deaths.
- R15: slice-2 reproduced. Matched outcome removes the death artefact (null exactly 0). Stage5-smoke shock kills rather than stresses; satisficing 'not_supported' = uninformative (6 distinct outcome values, quad CI [-28.6,+2.97]). Blocker = a base config with a stressed-but-alive regime.
- R13 (finding): DeltaH_rep as aggregated over live firm-ticks is dominated by death count (satisficing 1 death mean -0.061 vs 3 deaths -0.283); needs matched firm-ticks. Slice: pipeline works, no H1c verdict (regressor unspecified).
- R12 (new finding, verified in code): decision/lib.rs:175-191 attend() is a pure switch at h_crit; manual 12.3 Step 2 says the same; manual 31.2's 'both acting additively on separate variables' is inconsistent with it. H1c cannot discriminate simultaneous vs switching under this decision rule. Owner told; options in R12 chat memo.
- R11: 'sc4_wmax_beta_probe fails 4/400 at HEAD' is OUTDATED. HEAD 2820610 already contains the ADR-0057 Option B restructure (a1/a2) and the test passes (owner local run). D-A remains open; Option B half is committed with the asymmetry documented as known-not-intended while ADR-0057 is still DRAFT.
- R01: 'Phases 0-2 complete' overstated Phase 2. Gate met only under ADR-0044's per-hypothesis reading; audit-reported (not re-run) that no satisficing config meets SC-1,2,3,6 together (F12). Phase 3 is not started in substance (no base config, no AT tests, no DeltaH_rep).
- R05: R04 claim 'variance must come from the init stream' was wrong: no plugin uses the init stream; existing environment noise exists but did not reach actions in tested regimes.
- R03: the R00 retraction below is itself WITHDRAWN; ADR-0055 says S1 has stronger textual support, the 'two' refers to the nulls named in 30.4 prose. S1 lean restored (owner decision still open).
- R00 (withdrawn in R03): D-B reviewer lean to S1 was premised on 'two null conditions named in 28.4'. 28.4 names five nulls; Arm C as drafted pairs only two of them (random decision, beta=0). The S1 lean is NOT supported by that sentence alone; ADR-0055's own reading of the 30.4 Arm C prose is what matters. Left open for owner, lean withdrawn.
- ADR-0042/0048/0049 "0/400, all pass" claims false (corrected by DRAFT
  ADR-0056).
- Aspiration decay is NOT a manual deviation: manual 12.1 and TC-004 specify
  the exponential update and call falling aspirations Intended. The
  probe's 100,000 capital goal is the artifact.
- ADR-0050 revisit trigger 2 names political capital and single-action-per-
  tick, not frozen u; Option A does not fire it. Trigger 3 (materially
  different shaping-selection rates) arguably fired when ADR-0056 found the
  regression.
- An earlier plan reconstruction wrongly described Phase 4 as individual
  thresholds and capability decay; manual 26.6 says "Platform".

## Open questions
OQ-14 Arm A single-firm-per-job unenforced (builder fix: reject multi-agent
template, done in Batch 1). OQ-17 resolution of the probe finding. OQ-18
stale citations. OQ-19 two seed literals in test_spec.py. OQ-20 stats
pipeline design choices (H1a/H1b centering can flip a verdict; no fork
runs or DeltaH_rep exist). OQ-21 unbounded theta_limit. OQ-22 lobby/contract
theta benefit credited at one-step while real lag is 2-6 ticks. OQ-23
manual 30.4 still 315/63,000. OQ-24 first_replicate changes spec hashes.

## Reviewer notes
Template-dependent caveats: F01/F12 reach and F04's h levels depend on the
E1 base config, which does not exist yet.
