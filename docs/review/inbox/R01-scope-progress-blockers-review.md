# R01 scope, progress and blockers (owner-requested orientation round)
Date 2026-10-01. Sources read this round: docs/MANUAL.md 1.2-2.5, 26.1-26.8, 27, 30.9; docs/audit/AUDIT-2026-10-01.md (sections 7, 9, 11, findings index); PROGRESS.md (grep only); ADR Status lines. NOTHING WAS RUN. Audit and PROGRESS statements are builder-written and are labelled "audit-reported", not re-verified.

## 1 Verdict
BLOCKED ON OWNER. The project is not stuck on code; it is stuck on about five decisions and one missing artifact (the E1 base config). H3 side work is not on the critical path.

## 2 What the project is (plain terms)
FIRMA is a computer simulation (Rust engine, Python analysis) of fake firms. Each firm tries to stay alive under hard limits (cash, law, scope, obligations). Question to answer (manual 2.1): when a firm is (a) missing its performance target and (b) close to dying, do these push how widely it searches for options in opposite directions, at the same time, and does being able to lobby/contract (change the limits) alter that?
Published goal: one pre-registered experiment, E1, answering H1a, H1b, H1c (plus H2, H4), either way. A negative answer is acceptable (manual 26.5, 2.5). If no interpretable result: stop (kill criterion).
Hypotheses: H1a more shortfall -> wider search. H1b closer to death -> narrower search. H1c combined shape non-monotonic. H2 novelty of threat narrows more than size of threat. H3 shaping helps if fast, hurts if slow (PARKED, ADR-0050/0051). H4 narrowing helps survival under familiar threats, hurts under novel ones. H5 rivals (Phase 4).
Glossary: varsigma = shortfall; h = margin to death; w_eff = search width; H_rep = how spread out the firm's choices are; ADR = a written design decision; VT = validation test; DT = determinism test; SC = sanity condition (does the model behave alive); AT = robustness test; Arm A = manipulate h and varsigma directly; Arm B = shocks and shaping; Arm C = null controls; seeds 1-200 = reserved for the real run, never to be touched before filing.

## 3 Phases and how far along (manual 26)
| Phase | What | State | Basis |
|---|---|---|---|
| 0 Foundations | manual, ADRs, literature | DONE | manual 26.2 |
| 1 Kernel | deterministic engine | DONE (reported; not re-run by me) | PROGRESS, audit 237 tests pass |
| 2 Model | firms, constraints, VT tests, SC-1..6 | BUILT, GATE CONTESTED | audit 7: no satisficing config meets SC-1,2,3,6 together; ADR-0044 reads the gate per hypothesis; audit F12 says its SC-1 "spread" was two tick-0 deaths |
| 3 First result (E1) | stats, full sweep, AT tests, manuscript | NOT STARTED in substance | no base config, no AT tests, no DeltaH_rep, spec not materialisable (F05) |
| 4-6 | platform, coalitions, empirical | NOT STARTED, correctly | manual 26.6-26.8 |
My judgement (not a measurement): 2 of the 4 phases needed for the first publishable result are done; Phase 2 is mostly built and Phase 3 has analysis scaffolding only. Nothing has been filed and no real run has been executed.
Correction to my R00/STATE wording: "Phases 0-2 complete" overstated Phase 2.

## 4 Why it feels out of control
H3 (shaping lag) was found untestable at the Phase 2 gate (ADR-0044). That started a model-revision chain, ADR-0047 -> 0049, which introduced a regression, then ADR-0050 -> 0057 (11 ADRs) to document and untangle it. H3 is not in E1 (ADR-0051). So roughly a fifth of all decision records concern a hypothesis that does not gate the first result.

## 5 What blocks Phase 3 (ranked by how directly it stops E1)
1 No E1 base model config (F05). Nothing exists to run. Needs small DRAFT ADRs: shocks, population/heterogeneity, rest. OWNER approval each.
2 Between-seed randomness (F01, D-C). Satisficing runs are identical across seeds unless something stochastic is configured. 200 identical seeds give a zero-width CI. Stats now refuse such data (Batch 1), so the pipeline halts rather than lies. OWNER decision on the variance source.
3 Arm A may not measure what it claims (F04, D-D, T2). 25 (h, varsigma) cells collapse to 3 behavioural groups; units of varsigma vs engine integers undecided. If h and varsigma cannot be independently manipulated, E1 is void (manual 2.5, VT-8). OWNER decision.
4 Degenerate regimes (F12). No satisficing config where constraints bind without mass early death or never binding. Manual 26.4 says: a model revision, not parameter fudging. Probably resolved together with 1.
5 Missing deliverables: DeltaH_rep (fork runs), AT-1..5 (no tests exist), per-run metrics exposure. DIRECT builder work, but only after 1-3 fix what to build.
6 Version/hash hygiene before filing (D-F, D-G, D-A): MANUAL_VERSION stale, no model golden trace, lookahead asymmetry (D-A) changes hashes. Must be settled before the filing, not before exploration.
7 Process: the 7,400-line uncommitted risk in the audit was reduced by commit 2820610; now 276 diff lines plus a prototype worktree diff remain uncommitted (D-I).

## 6 Owner decisions: which matter now
Critical path to a first E1 run: D-C (variance source), D-D (Arm A mapping and gate), D-A (lookahead), D-F (versions), D-B (Arm C count).
Can wait: D-E, D-G, D-H, D-J, ADR-0056/0057 acceptance paperwork (except where D-A forces it).
I recommend deciding D-C and D-D first because they decide whether E1 is meaningful at all, and everything else is bookkeeping around them. Options per decision are not rewritten here; see STATE.md and R00; I will produce options with falsifiers per round when asked, one decision at a time.

## 7 Next builder prompt
none, blocked on owner (D-C, D-D). R00's R01-verify-claims prompt (re-running the suite and the probe at seeds >= 910000) remains safe to run if the owner wants an independent re-check; it makes no decisions.

## 8 Least sure about
1 The audit's regime table (F12) is builder-written and I did not re-run it; Phase 2's "contested" status rests on it.
2 My phase-progress judgement is qualitative; the manual gives durations, not completion measures.
3 Whether Phase 1 is truly clean: I accepted "DT green" from reports.
