# ADR 0056 — Correcting the record: `sc4_wmax_beta_probe` claims in ADR-0042, ADR-0048 and ADR-0049

**Status:** DRAFT — pending owner review. Paperwork only: no code, test or
config is changed by this ADR.
**Phase:** 2→3 boundary (Phase-2 evidence record)
**Supersedes, narrowly — one evidential claim each, nothing else:**
- **ADR-0042**, "The evidence", bullet **(a)** of `sanity::sc4_wmax_beta_probe`
  — the `0 / 400` figure *and* its stated premise about the probed firm.
- **ADR-0048**, Compliance, the bullet claiming the
  `sc16_gate`/`sc4_wmax_beta_probe`/`sc16b_arm_scoping` harness "all re-run,
  all green".
- **ADR-0049**, Compliance, the bullet claiming the same harness "all pass"
  and the full workspace `cargo test` "green, 0 failures".

ADR-0042's Decision text (shaping never satisfices a `GOAL` focus; the
`GOAL` fallback reaches shaping only when every market action ahead of it
is inadmissible), ADR-0048's race-check design and ADR-0049's
`time_to_boundary` fixes are **not** superseded.
**Relates to:** OQ-17 (where this was found), ADR-0040 (its Status line
summarises ADR-0042's claim — see Consequences), ADR-0044, ADR-0050,
ADR-0051 (unaffected, see Decision 3), ADR-0057 (the root-cause
investigation and fix proposals, DRAFT, same session).

## Context

The ADR-0040–0054 audit (OQ-17) found `sanity::sc4_wmax_beta_probe` failing
at HEAD (`c5eb8cd`). Bisected in scratch worktrees: `9c49c34` passes,
`17ba7bb` (the first commit containing ADR-0047, 0048, 0049 and their code)
fails with the same numbers, as does `89a4648`. ADR-0048 and ADR-0049 were
committed *in* `17ba7bb`, so their "all pass" claims were false against
the code they shipped with — not merely stale since.

This ADR records exactly what each ADR claimed and what is true. Re-checked
for this ADR, at HEAD, by running the probe's configs through
`firma run --model` (scripts and traces in
`docs/adr/evidence/adr-0057/`), not by re-reading earlier reports.

## Decision

### 1. What was claimed versus what is true

| ADR | Claim (quoted) | What is true |
|---|---|---|
| 0042, evidence (a) | "a healthy `GOAL(1)` firm (… so `h` stays well above `h_crit`) … **`0 / 400` shaping decisions at *every* one of the 15 `(w_max, β)` cells**" | At HEAD: `4 / 400` at `(w_max, β) = (6, 0.0)` and `(9, 0.0)`; `0 / 400` in the other 13 cells. The test fails at `tests/tests/sanity.rs:300`. |
| 0042, evidence (a), the premise | "`h` stays well above `h_crit`" (a `GOAL(1)` firm throughout) | **False even when ADR-0042 was written.** At `9c49c34`, with the original code and original config, agent 0 is in `SURVIVAL` focus on **44 of 200** ticks (a 9-tick cycle: 7–8, 16–17, 25–26, …), read from the run's own `focus` deltas. The probe passed then only because, before ADR-0047, no shaping action could satisfice under `SURVIVAL`. |
| 0048, Compliance | "the `sc16_gate`/`sc4_wmax_beta_probe`/`sc16b_arm_scoping` harness … all re-run, all green" | `sc4_wmax_beta_probe` fails at `17ba7bb` (`4 / 400` at `(6, 0.0)` and `(9, 0.0)`). |
| 0049, Compliance | "… sanity harness (all pass …), full workspace `cargo test` (green, 0 failures)" | Same failure at `17ba7bb`; `cargo test --workspace` therefore has 1 failure. |

### 2. A disclosure gap the three ADRs share

`17ba7bb` did not only add ADR-0047–0049's code. It also **edited the
probe's own configs** so that they opt into ADR-0047's mechanism:
`cfg_wide_search` gained `lobby_success` and `contract_success` blocks, and
`cfg_input_starved` and `cfg_arm_b_satisficing` gained `lobby_success`
(`git diff 9c49c34 17ba7bb -- tests/tests/sanity.rs`). ADR-0047's mechanism
is opt-in (`None` ⇒ cost-only, "behaviour-preserving by default"), so this
config edit is what made the probe exercise it. None of ADR-0047, 0048 or
0049 mentions editing these configs. Checked at HEAD: the probe's
**pre-`17ba7bb` config gives `0 / 400` in all 15 cells** on today's code —
ADR-0047's backward-compatibility claim holds; the probe's numbers changed
because its config changed *and* the mechanism exists.

### 3. What still holds despite the correction

- **SC-4 remains unmet under `decision.satisficing`.** The probe's worst cell
  is `4 / 400 = 1 %`, under §16.2's 5 %; `cfg_arm_b_satisficing` is `0 %`
  (ADR-0050). ADR-0044's per-hypothesis decision and ADR-0051's filing-scope
  decision rest on "SC-4/SC-5 unsatisfied under `decision.satisficing`", which
  is still true. **Their decisions are not affected**; only the evidence cited
  for one intermediate claim was wrong. Their text is not touched.
- **ADR-0042's `GOAL`-branch reasoning holds at HEAD.** ADR-0047 changed only
  the `SURVIVAL` branch of `satisfices()` (`firma-plugin-decision/src/lib.rs`,
  `Focus::Survival` arm; the `Focus::Goal(j)` arm is still cost-only). Direct
  evidence: at tick 16 of the `(6, 0.0)` run, agent 0 is in `GOAL(1)`, the
  scan reaches `lobby`, and `satisfices` returns `false`. The select-level test
  `validation::sc4_shaping_selected_only_as_the_goal_fallback_never_by_the_scan`
  passes.
- **ADR-0042's evidence (b) is reproduced exactly: `3 / 600`.** It cannot be
  reached *inside* the test — part (a)'s `assert!` panics and unwinds out of
  `sc4_wmax_beta_probe` before part (b) runs. Running `cfg_input_starved` (seeds
  `(11,2,3,4)`) directly through `firma run --model` gives `3 / 600`: one
  tick-0 `lobby` per firm, each a `GOAL(1)` fallback pick with no ADR-0047
  gate/payoff record. Both of part (b)'s assertions (`fraction > 0`,
  `!sc4_pass`) would hold.

### 4. Why the numbers changed — not decided here

The mechanism (which focus, which gate value, which payoff comparison, and
why only `β = 0` with `w_max ∈ {6, 9}`) and the proposed fixes are in
**ADR-0057 (DRAFT, pending owner review)**. This ADR deliberately does not
resolve them: correcting what was claimed is separate from deciding what
to do about it.

## Alternatives

- **Edit ADR-0042/0048/0049 in place.** Rejected — ADR immutability.
- **One ADR for the correction and the root cause.** Rejected — the
  correction is certain and paperwork-only; the root cause comes with fix
  options that need an owner decision. Kept separate so this one can be
  accepted without pre-committing to a fix.
- **Correct only the numbers, not the premise.** Rejected — the premise
  ("`h` stays well above `h_crit`") being false at the time is the more
  important correction: it means the probe never tested what ADR-0042
  said it tested.

## Consequences

- **Positive.** The three citable claims now have an accurate, superseding
  record; the undisclosed config edit is on record.
- **Negative, accepted.** `sc4_wmax_beta_probe` keeps failing until ADR-0057
  (or another decision) is acted on. Its assertion is not touched.
- **Neutral.** ADR-0040's Status line paraphrases ADR-0042's claim ("shaping
  stays unreachable at every `(w_max, β)`") and is now inaccurate in the same
  way. Its Status line has already been updated once (to point at ADR-0042)
  and the owner's instruction named only ADR-0042/0048/0049, so it is **not**
  annotated here — flagged for the owner.

## Compliance

- Status lines of ADR-0042, ADR-0048 and ADR-0049 point here (status
  metadata only; their bodies are unchanged). `docs/adr/README.md` index
  rows updated to match.
- Evidence: `docs/adr/evidence/adr-0057/` (`sweep15_head_vs_pre.txt` — all
  15 cells, both configs; traces of ticks 6–9 and 15–18; the config
  transcriptions and runner script). The transcription is validated by
  reproducing the test's own output exactly (all 15 cells).
- Nothing run used reserved seeds: the probe configs use `(909,2,3,4)` and
  `(11,2,3,4)`.

## Note

The probe's premise was wrong from the day it was written, and the test
still passed, because the one thing that would have revealed it — shaping
being able to win a `SURVIVAL` scan — did not exist yet. A sanity test that
checks only an outcome count, not that the scenario is in the state it
claims to be in, can pass for the wrong reason indefinitely.
