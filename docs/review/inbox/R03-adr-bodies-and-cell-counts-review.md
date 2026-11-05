# R03 ADR bodies read + cell counts re-run (owner-authorised, this task only)
Read in full: ADR-0044, 0051, 0055, 0056, 0057. Read decision sections: 0043, 0047 (mechanism), 0050 (finding), 0053 (Parts 1, 4), 0054 (Q1, Q5, open question). NOT read: 0045, 0046, 0048, 0049, 0052 bodies; 0040-0042 bodies. Run: `build_narrowed_e1_spec` (spec.py loaded directly; pandas absent so package __init__ not imported). No simulation was run, no seed used.

## 1 Verdict: ACCEPT WITH FIXES (R02 corrected in 3 places, see 3)

## 2 Cell counts (VERIFIED by running)
| design | A | B | C | cells | runs @200 | how |
|---|---|---|---|---|---|---|
| manual 30.4 table arithmetic | 125 | 180 | 10 | 315 | 63,000 | python: 5*5*5, 5*3*4*3, 5*2 |
| built spec today | 125 | 60 | 10 | 195 | 39,000 | build_narrowed_e1_spec().arms cells(), num_replicates=200 |
| ADR-0055 S5 | 125 | 60 | 6 | 191 | 38,200 | arithmetic only; NOT implemented in code |
| ADR-0055 S1 | 125 | 60 | 2 | 187 | 37,400 | arithmetic only; NOT implemented |
Arm C as built = beta{0,.5,1,2,4} x {satisficing, random}: the 5 random cells are behaviourally identical (ADR-0055 Dec.1, from code: RandomParams has no beta). Printed cells confirm the cross-product. Evidence: this transcript's command output; no file stored (reviewer evidence rule: re-run `python3` snippet in R03 file below).

## 3 Corrections to my earlier rounds
C1 (R00 retraction withdrawn). I said D-B's S1 lean was unsupported because 28.4 lists five nulls. Wrong framing: ADR-0055 itself cites 30.4 prose ("beta = 0 and decision.random. Both must show no relationship") and 28.4's two matching rows, ADR-0027's "beta=0 is the No-narrowing null", and says S1 has the stronger textual support, S5 is hard to square with "Null". My R00 sentence misread what the "two" referred to. D-B stays an OWNER-DECISION; textual evidence favours S1; S1 costs 400 null-runs, S5 adds 4 beta>0 satisficing cells that are not nulls.
C2 (R02 D1). Not a builder reinterpretation: ADR-0051 Points 2 and 5 are explicitly OWNER-DECIDED (SC bullet read as scoped to the filed design). But ADR-0044 shows SC-1/2/3/6 each reachable in "at least one config", not jointly in one satisficing config, and manual 16.2 says jointly. Manual text was not amended (patch deferred, logged in PROGRESS). Status unchanged: gate reading is owner-sanctioned, manual still says otherwise.
C3 (R02 D4). ADR-0047 is opt-in (None => cost-only, behaviour-preserving). ADR-0056 reports pre-17ba7bb probe config gives 0/400 at HEAD. If so the H3 revision is inert unless a config opts in; E1 need not opt in (shaping lag is out of E1). UNVERIFIED by me: that ADR-0048/0049 code paths are also inert when unconfigured. Cheap check: run the frozen-hash tests. This could make D-A avoidable for E1 (not for H3). New OWNER-DECISION option D-L below.

## 4 Findings from the bodies
F-A (correctness, E1-adjacent, mostly manual-level). Arm A cannot test H1a on search_width: 12.3 Step 3 has w_eff = f(h) only; shortfall only picks focus. ADR-0054 Q5 confirms pins feed select() unchanged. Consequences by the manual's own rules (inference, not run): at h in {0.02,0.05,0.10} (< h_crit 0.15) focus is SURVIVAL regardless of varsigma; at h in {0.20,0.40} w_eff = w_max regardless of h and varsigma only switches GOAL(1) vs NONE (NONE repeats previous action). So H1a can only show through repertoire entropy via focus/scan-order, and 3 of 5 h levels give varsigma no role. This matches audit F04 and is a property of 12.3, not an implementation bug. VT-8 shows h and varsigma are independent inputs; it does not show varsigma has an effect. Class: OWNER-DECISION (pre-registration must state which DV H1a is claimed on; do not fix by changing 12.3 to make it pass).
F-B. ADR-0054 open question (map of the one swept varsigma onto three varsigma_j) has no owner decision recorded. Recommendation inside ADR: Option B (varsigma_1 swept, others 0.0, matching VT-8). Audit F04(a): pinned varsigma in [0,1] vs integer capital change so four non-zero levels give the same test. Still D-D.
F-C. ADR-0050 body: in the realistic Arm-B scenario lobby's slot is reached in 75/1484 decisions; in 67 SURVIVAL reaches the ADR-0048 gate closed 100% (lag_min 2 vs time_to_boundary 1); the expected-payoff formula never runs once. So ADR-0047-0049 did not open shaping in the design-representative scenario; the 4/400 probe selections arise in a case ADR-0057 traces to a lookahead asymmetry. Net: the revision has cost (regression, asymmetry) and no demonstrated benefit for E1. (All builder-measured; UNVERIFIED by me.)
F-D (process). ADR-0053 Part 4 discloses ad hoc CLI runs at (1,1,1,1) including a config from Arm A cell 0 (h=0.02, varsigma=0, beta=0, replicate 1) on a non-registered 1-firm 20-tick template. 30.6 says any Phase 2 run touching the registered grid invalidates registration. Disclosed, template unregistered, nothing filed: low risk but must be repeated in the filing's exploratory-run disclosure. Also (audit, Batch 1): four fixtures moved out of 1-200.
F-E. ADR-0057 F6: theta_limit unbounded (0.90 -> 1.20 in 19 ticks); once >1.0 compliance can never bind for any firm. Affects SC-3 and regimes in E1 (OQ-21).

## 5 Premise checks / positive controls
Cell counts: control = the manual table arithmetic reproduced 315 and 63,000 independently of spec.py, so the counting method can detect a different figure. Not done: any check that ADR-0047-49 are inert when unconfigured.

## 6 Owner decisions needed (new/updated)
D-B: S1 (187) vs S5 (191). Falsifier: none empirical; textual. Textually most consistent: S1. Under S5 the arm no longer is purely null; under S1 beta-sweep nulls are reduced to beta=0 only, so "no relationship across beta" cannot be tested inside Arm C (Arm A/B already sweep beta).
D-L (new): run E1 with the ADR-0047/48/49 mechanism switched off (config opt-in absent) and defer the lookahead question (D-A) to the H3 filing. Assumes inertness when unconfigured (UNVERIFIED). Effect: H1a/b/c/H2/H4 unaffected by definition; H3 unaffected (already out of E1). Falsified if the frozen-hash or an A/B run with/without the blocks shows event-log differences on an unconfigured template. Most internally consistent with ADR-0051 (H3 out of scope) and A7 (boring version).
D-D extended: decide the DV H1a is registered on (F-A) together with the varsigma mapping.

## 7 Next builder prompt (one scoped, no owner decision)
"Read docs/review/BUILDER.md, STATE.md. Task R03-inertness-check. In a scratch worktree of HEAD: (1) run the four frozen-hash tests and report pass/fail with raw output; (2) write a failing-first test only if a difference is found; (3) with an unconfigured model config (no *_success blocks, time_to_boundary toggle at default), run cfg_healthy and cfg_threaded (seeds (909,2,3,4) style, >=910000 mechanism seeds only) on HEAD and on 9c49c34 and compare event-log sha256; report equal/unequal per scenario. Authorised: docs/review/outbox/*, docs/review/evidence/R03/*. No source edits, no seeds 1-200, no commits. Predeclared: 'inert' = byte-identical logs for every scenario; any difference is reported, not explained away. Stop and ask on any manual-unspecified choice. Stay silent between tool calls. List the 3 things you are least sure about."

## 8 Least sure about
1 I read ADR-0048/0049 only via titles and the 0050/0057 summaries; inertness-when-unconfigured is untested.
2 F-A is my inference from 12.3 and ADR-0054 text; I did not run Arm A.
3 ADR 0040-0042 and 0045/0046/0052 bodies unread; conclusions about them rest on 0044/0056 citations.
