# R02 deviation from the original manual (v1.0.0)
Method: the pasted v1.0.0 equals the manual at commit 9c49c34 (same header, date 2 Sep 2026). I diffed that against HEAD (git diff 9c49c34 HEAD -- docs/MANUAL.md: +110/-20 lines) and read ADR titles 0010-0057 and Status lines. I did NOT line-compare against the pasted text, and did not re-run code. Code-vs-manual deviations are ADR/audit-reported, not re-verified.

## 1 Verdict: the manual text barely moved; the project moved around it
Manual text changes (all labelled, 20 lines removed): crate map updated for firma-domain and firma-analysis; E1-E4 renamed Result 1-4 (ADR-0052); H3 marked excluded from first filing; 30.9 SC wording relaxed; clarifications to 9.1, 12.3, 16.1, 19.4/19.5; ADR index. 30.4 (315 cells/63,000 runs) NOT updated.
## 2 Deviations by weight
HEAVY (change what E1 can claim)
D1 SC-1..6 gate. Orig 16.2: "if SC-1..6 cannot be jointly satisfied ... model MUST be revised before Phase 3." ADR-0043/0044 re-read the gate per hypothesis; 30.9 was reworded (ADR-0051). Audit-reported: no satisficing config meets SC-1,2,3,6 together. By the original wording Phase 2 is not passed.
D2 H3/E3 dropped from first filing (ADR-0050/0051). Orig 30.3-30.4 included shaping lag; 28.2 calls E3 "the distinctive mechanism". Cells 315 -> 195 (or 187/191 pending ADR-0055).
D3 Shaping (acting on constraints) is effectively inert under decision.satisficing (SC-4/SC-5 fail, ADR-0042/0044). Orig 31.2 calls the endogenous survival point the "largest surviving gap" and the project's distinguishing feature (1.2). E1 as narrowed does not test it. Contribution that remains: search breadth as DV + simultaneity (H1c), which 31.2 rates "moderate".
D4 Decision rule changed on branch (ADR-0047/48/49). Orig 12.3: satisfices() SURVIVAL = one-step lookahead, "the firm does not simulate". Now has expected-shaping-relief and a time_to_boundary race. Lives on h3-satisficing-lookahead; master is 9c49c34. Caused regression (ADR-0056/0057) and lookahead asymmetry (D-A).
MEDIUM
D5 Arm C 10 cells -> 6 or 2 (ADR-0055 draft). Arm B 180 -> 60 (ADR-0051).
D6 Arm A "set directly by intervention" implemented via four pin-read keys inside decision.satisficing (ADR-0054); audit F04 suspects it does not identify H1a/H1b.
D7 Event log NDJSON, not Arrow/Parquet (ADR-0012) - orig 22.1 named Arrow/Parquet. Benign, ADR'd.
LIGHT / structural, ADR'd: firma-domain, firma-analysis crates; delta-uniqueness exceptions (19.4); decide->act hand-off; RNG stream registration; viability solver pulled into Phase 1 (ADR-0011).
## 3 Not drifted but not done (orig required)
AT-1..5 (25.5, 30.9): none exist. DeltaH_rep primary outcome (14.6, 30.7): not computable. firma-inspect (Phase 3): none. Independent re-implementation (25.8): none. 22.2 log-sufficiency review and 33.6 licence/data plan: UNVERIFIED. MANUAL_VERSION stale (20.5/0.6: manual version MUST be recorded).
## 4 Judgement
Engineering stayed faithful: kernel purity, determinism, ADR discipline all adhered to. Scientific scope drifted: gate reinterpreted (D1), distinctive mechanism parked (D2/D3), decision rule altered on a side branch (D4). None is hidden - each has an ADR - but D1 and D3 were reinterpretations by builder + owner, not manual-version events (0.6: altering a MUST = MAJOR + ADR; D1 changes how a MUST in 16.2 is read).
## 5 Owner decision raised
D-K: either (a) formally amend 16.2/27.3 via a MAJOR manual bump + ADR so the per-hypothesis gate is the rule, or (b) treat Phase 2 as unpassed and do the model revision 26.4 prescribes. Options not ranked here.
## 6 Least sure
1 Whether any D4 change is on master (no: master 9c49c34 - verified by branch list).
2 ADR bodies not read; judgements rely on titles/Status lines.
3 Cell counts in D5 from ADR text only.
