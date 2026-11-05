# R05 the three uncertainties from R04, checked (reviewer; read-only + scratch builds)
Evidence: docs/review/evidence/R05/inertness.txt (sha256 603c2ecc...7714). Scratch builds of 9c49c34 and HEAD in the session scratchpad (volatile). Seeds used: mechanism 909 (existing probe config), environment 910001-910005. No reserved seeds.

## 1 Init stream: VERIFIED not wired to any randomness
World stores seeds.init (kernel/world.rs:34) but stream_for_phase maps only Environment->Environment, everything else->Mechanism (kernel/lib.rs:675-680); no plugin returns StreamId::Init from rng_stream (only resource, observation -> Environment; shock -> Shock). Initial state comes from config values (ADR-0030). My R04 statement "variance must come from the init stream" was WRONG about the mechanism: there is no init-stream consumer today, so using it is new code.
decision.satisficing draws NO randomness: Satisficing::apply names its key `_key` unused (decision/lib.rs:1024). Randomness exists in decision.random (:1229), shaping success/lag, shock.stochastic, resource.patchy and observation.noisy.
Run: adding resource.patchy (environment stream) to the probe config, sigma 0/1/3, 4 environment seeds: full event logs differ across seeds for sigma>0 (so a variance source exists) BUT the selected-action sequence hash is IDENTICAL across all seeds (12/12). Two more low-stock configs (capital 60/30, input 3/2, sigma 3, 5 seeds each): action hashes identical (10/10). So price noise does not reach actions in any regime tried (3 regimes). Conclusion, established: with existing plugins, Arm A (pinned, no shocks) has NO source of seed variance that reaches the primary outcome. Not established: whether shock.stochastic in Arm B reaches actions (not run; plausible via h, UNVERIFIED).
=> D-C requires either (a) accepting Arm A is deterministic and saying so in the filing (then 200 seeds add nothing and stats must treat them as one run), or (b) a new variance mechanism by ADR (init-stream heterogeneity is the manual-named candidate, 21.3). Needs owner.

## 2 Hygiene list: PARTLY VERIFIED
Verified present at audit-cited sites: silent from_json(...).ok() drops at decision/lib.rs:362,385,411, reconstruct.rs:436, observation:290, phase_rules.rs:57,152; unwrap_or_default at decision:1027,1205. CI: ci.yml triggers push only on main/master (not this branch), contains no pytest job. Python skips: test_metrics_cross_check.py:38,57 and test_runner.py:31 (skipif). sanity.rs:595-598 asserts the gate FAILS (as audit says). Line numbers match.
Not verified (not run): "14 tests skip silently", cfg_threaded SC-2=1.000 and 3200/3202 produce_ordinary, vt4/vt8 premise claims, deny job having failed.

## 3 Inertness (D-L): VERIFIED for the tested family, with a control
Unconfigured probe config (the 9c49c34 'pre' variant), all 15 (w_max,beta) cells, run on a 9c49c34 build and a HEAD build: events.ndjson sha256 EQUAL in 15/15. Positive control: on the HEAD build, the opted-in 'head' variant gives a DIFFERENT log than 'pre' at (6,0.0); the 9c49c34 build rejects the opted-in config (ERR), as expected. So the comparison can detect a difference.
Coverage limits (state them in any filing): 2 firms, 200 ticks, no shocks, no multi-firm, no pins, contract/diversify unconfigured, one seed tuple. This config enters survival focus on 44/200 ticks (per ADR-0056), i.e. the regime where the changed code would matter if opted in. Also: tests/tests/golden.rs is byte-unchanged between 9c49c34 and HEAD and covers the Phase-1 registry only (F09), so "four frozen hashes unchanged" cannot show inertness for model plugins; my event-log comparison can.
Code reading agrees: constraint phase_rules.rs change is a pure refactor into advance_window; kernel change adds an Intervention variant; decision-plugin deletions are replaced by equivalent logic (UNVERIFIED line by line).
=> D-L is supported for this family; before relying on it for E1, repeat on the real E1 base config once it exists.

## 4 New facts for STATE
- No init-stream consumer exists. Satisficing is draw-free. Environment noise did not reach actions in 3 regimes.
- Inertness 15/15 equal with control.
## 5 Least sure about
1 shock.stochastic's effect on Arm B actions (not run).
2 Inertness beyond the one config family.
3 Hygiene counts from the audit that I did not run.
