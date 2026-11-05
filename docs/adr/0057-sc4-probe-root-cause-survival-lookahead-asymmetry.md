# ADR 0057 — Root cause of the `sc4_wmax_beta_probe` 4/400: in a compliance-bound `SURVIVAL` scan, the one-step lookahead sees `lobby`'s effect on `θ` but not any market action's effect on `u` — investigation and fix options

**Status:** DRAFT — pending owner review. **Investigation and proposal only;
nothing is implemented.** No decision is taken here: the options are
costed and one is recommended, clearly marked as a recommendation.
**Phase:** 2→3 boundary (H3 model revision, ADR-0047–0049 follow-up)
**Supersedes:** nothing.
**Relates to:** ADR-0056 (the record correction this explains), ADR-0042,
ADR-0047 (expected-relief lookahead, `SURVIVAL` only), ADR-0048 (race gate),
ADR-0049 (`time_to_boundary` advances `u`; its Context on one-step `u`
freezing), ADR-0021 Decision 4 / §9.3 (`u` is not part of the
`FirmState → FirmState` core), ADR-0014/0028 (`u` from the action window),
ADR-0015/0016 (global, additive `θ`), ADR-0050 (H3 interim disposition),
manual §12.3.

## Context

ADR-0056 records that `sc4_wmax_beta_probe` part (a) shows `4 / 400`
shaping decisions at `(w_max, β) = (6, 0.0)` and `(9, 0.0)` since `17ba7bb`.
The owner asked for the actual mechanism — not a re-confirmation that
ADR-0047's commit introduced it — and for fixes that address the cause.
The constraints were explicit: no change to the test's assertion, no
descoping of H3, no reverting ADR-0047–0049.

**Method.** The probe's configs were transcribed verbatim
(`docs/adr/evidence/adr-0057/mkcfg.py`, `mkstarved.py`). They were run
through `firma run --model` at HEAD with the existing read-only
`FIRMA_TRACE_DECISION` instrumentation, which is intact
(`firma-plugin-decision/src/lib.rs`, `trace` module). The transcription is
validated by reproducing the test's own output in all 15 cells. The
pre-ADR-0047 commit `9c49c34` was built in a scratch worktree (removed
afterwards). The probe's seeds are `(909,2,3,4)`, outside the reserved
range.

## Findings (all from traces and event logs, numbers as observed)

### F1. All four selections are `SURVIVAL`-focus picks — the `GOAL(1)` hypothesis is refuted

The owner's primary hypothesis was that ADR-0047's gate might be reached
from `GOAL(1)` focus. It is not. In the code, the expected-relief path
(`DecideCtx::shaping_expected_survival_margin`) is called only from the
`Focus::Survival` arm of `Satisficing::satisfices`; the `Focus::Goal(j)` arm
is still cost-only. In the traces, all 4 selections at `(6, 0.0)` and all 4
at `(9, 0.0)` have `focus = Survival`:

| tick | agent | `h_t` | `w_eff` | `prev_action` | gate: `lag_min` vs `time_to_boundary` | `p_success` | `h_success` | `h_failure` | `E[h]` | `E[h] > h_t` |
|---|---|---|---|---|---|---|---|---|---|---|
| 8 | 0, 1 | 0.025 | 6 (9) | 1 | 2 vs `None` → open | 0.45 | 0.125 | 0.025 | 0.070 | yes |
| 17 | 0, 1 | 0.100 | 6 (9) | 2 | 2 vs `None` → open | 0.45 | 0.200 | 0.100 | 0.145 | yes |

The `GOAL(1)` branch was also observed directly. At tick 16 agent 0 is in
`GOAL(1)` (`h = 0.225`); the scan reaches `lobby` (4th in the `GOAL(1)`
order), and `satisfices` returns `false`.

### F2. The probe's firm is not a perpetual `GOAL(1)` firm — and never was

The firm's own `GOAL(1)` behaviour drives it into `SURVIVAL`. With
`aspiration_capital_growth = 100 000`, no market action satisfices
`GOAL(1)`, so `select` falls back to the first admissible action in the
`GOAL(1)` order, `produce_regulated` (2), every tick. That raises `u` (the
share of regulated production over the last `L_W = 8` ticks). With
`θ_limit = 0.90`, `h` falls 0.5 → 0.4 → 0.275 → 0.15 → **0.025 at tick 7**
(`u = 7/8`), and focus becomes `SURVIVAL`.

This is not new:

| Code / config | `SURVIVAL` ticks, agent 0, of 200 |
|---|---|
| `9c49c34` (pre-ADR-0047), original config | **44** (7–8, 16–17, 25–26, …) |
| HEAD, pre-`17ba7bb` config | 44 (same ticks) |
| HEAD, current config, `(6, 0.0)` | 3 (7, 8, 17) |

### F3. In a compliance-bound `SURVIVAL` scan no market action can ever satisfice, but `lobby` always does once the gate is open — this asymmetry is the root cause

`satisfices(a, SURVIVAL)` requires the lookahead `h_{t+1}` to exceed `h_t`.

- **For a market action**, the lookahead is `market_core` (`FirmState →
  FirmState`), evaluated with `u` held at its current value
  (`DecideCtx::margin_at`; "`u` tracking is a caller concern, not part of
  this core", `firma-domain/src/dynamics.rs` module doc, §9.3 / ADR-0021).
  When compliance (`g_2 = u − θ_limit`) is the binding constraint, nothing a
  market action does to `FirmState` moves the binding `g_j`. So
  `h_{t+1} = h_t` and the strict test fails, for **every** market action.
  Observed: at tick 8, actions 1, 2, 0 and 4 all return `false`.
- **For `lobby`** (ADR-0047), the lookahead is
  `p · h(θ_limit + δ_θ) + (1 − p) · h(θ_limit)`. With `s_u = 1` and
  compliance binding, this is `h_t + p · δ_θ = h_t + 0.45 × 0.10 =
  h_t + 0.045`. It is greater than `h_t` **whenever it is evaluated**.
  Observed: `0.025 → 0.070` and `0.100 → 0.145`.

So, inside one decision, the firm's model of the world says:
- holding, or switching from regulated to ordinary production, can never
  relieve a compliance squeeze;
- lobbying always does.

The first half is an artefact of freezing `u`. A u-aware one-step
lookahead, hand-computed from `firma_domain::window::advance_window` and
`margin::u_from_window` (the functions `constrain` and `time_to_boundary`
already use), gives:

- **Tick 8:** the window holds ticks 0–7 = `[2,2,2,2,2,2,2,1]`, so
  `u = 0.875`. Choosing `produce_ordinary` drops tick 0's regulated entry,
  so `u = 6/8 = 0.75` and `h_{t+1} = 0.15 > 0.025`. It satisfices, and it
  is first in the `SURVIVAL` order.
- **Tick 7:** the window holds only 7 entries, nothing drops, `u` stays
  `0.875`, and nothing satisfices. The fallback is `produce_ordinary`,
  exactly as today. (Tick 7's gate is closed anyway: `prev_action = 2`,
  `time_to_boundary = 1 ≤ lag_min = 2`.)
- **Tick 17**, on the current trajectory: `θ_limit = 1.1`. `produce_ordinary`
  gives `u = 7/8` and `h_{t+1} = 0.225 > 0.100`, so it satisfices.

These four values are hand-computed for these decisions only. What the
full 200-tick run would do under a u-aware lookahead is **not verified** —
that needs an implementation, which this ADR does not do.

### F4. The gate and the satisficing test disagree about `u` within the same decision

ADR-0049 made `time_to_boundary` advance the window. At tick 8,
`prev_action = 1` (`produce_ordinary`). The projection therefore sees `u`
falling under continued ordinary production, never reaches `h ≤ 0`, and
returns `None`. That is correct: continuing what the firm is already doing
relieves the threat, so ADR-0048's gate opens ("unbounded ⇒ arrives
comfortably").

The satisficing test then evaluates `produce_ordinary` with `u` frozen and
concludes it does nothing. One half of the decision models the window and
the other does not. ADR-0049's Context judged one-step `u` freezing a
simplification "not of the same shape" as freezing it across a
multi-tick projection. This finding is new evidence on that judgement: in
this scenario, the one-step freeze is the deciding factor.

### F5. Why exactly `β = 0` with `w_max ∈ {6, 9}`

In the `SURVIVAL` order `[1, 3, 5, 2, 0, 4, 6, 7, 8]`, actions 3
(`acquire_input`), 5 (`deliver`) and 7 (`contract`, no supply partner) are
inadmissible here (observed in the traces). Inadmissible actions do not
consume budget, so `lobby` is the **5th admissible action** and is reached
only if `w_eff ≥ 5`. At `h = 0.025`, `w_eff = max(1, ⌈w_max · (h/0.15)^β⌉)`:

| β | w_max = 3 | 6 | 9 |
|---|---|---|---|
| 0 | 3 | **6** | **9** |
| 0.5 | 2 | 3 | 4 |
| 1 | 1 | 1 | 2 |
| 2, 4 | 1 | 1 | 1 |

Only `(6, 0)` and `(9, 0)` reach 5. In every other cell `lobby` is never
scanned, `θ_limit` stays 0.90, and each `SURVIVAL` episode is the same
`h = 0.025` episode. So the cell pattern follows from the formula and the
admissibility pattern; it was not fitted to the result.

### F6. Why exactly 4 — a secondary finding about `θ_limit`

`θ` is global and lobbying is additive (ADR-0015/0016). In the `(6, 0.0)`
run, three of the four lobbies succeed: `θ_limit` goes 0.90 → 1.00 (tick
12) → 1.10 (tick 14) → 1.20 (tick 19)
(`theta_limit_events_head-w6-b0p0.ndjson`). Once `θ_limit > 1.0`, the
compliance limit sits above the highest share `u` can reach. Compliance can
then never bind for any firm, `SURVIVAL` stops, and there is no further
lobbying.

This is not the cause of the selections, but it is a model property worth
a separate look: nothing bounds `θ_limit`, so repeated success makes a
constraint vacuous for the whole population. Logged as an open question,
not proposed for change here.

### Mechanism, in one paragraph

The probe's firm cycles itself into a compliance-bound `SURVIVAL` state.
It always did, since ADR-0042 was written. Since `17ba7bb` its config opts
into ADR-0047's expected-relief lookahead for `lobby`. In that state the
one-step lookahead credits `lobby` with its full expected `θ` effect but
credits no market action with its effect on `u`. So `lobby` is the only
action that can satisfice. It is selected whenever the scan reaches it
(`w_eff ≥ 5`, i.e. only `β = 0`, `w_max ≥ 6`) and the ADR-0048 gate is
open. The gate is open precisely when the firm's current action (ordinary
production) is already relieving `u`, because `time_to_boundary` (ADR-0049)
does track `u`.

## Fix options

Each option says what it changes, why it addresses the cause rather than
the symptom, what it needs, and what it risks. None of them changes the
test's assertion, descopes H3, or reverts ADR-0047–0049.

### Option A — make the `SURVIVAL` one-step lookahead u-aware for market actions (mechanism fix)

**Change.** In `Satisficing::satisfices`' `Focus::Survival` arm, compute a
market action's `h_{t+1}` with `u` recomputed from
`advance_window(window, tick, a, l_w)` → `u_from_window`, instead of the
frozen `aux.regulated_intensity`. `DecideCtx` already carries `window`,
`l_w` and `tick` (added by ADR-0049). `market_core` and the viability-kernel
computation are untouched: §9.3's "`u` tracking is a caller concern" is
honoured by the caller actually doing it. `GOAL(j)` is untouched, since no
`v_j` reads `u`.

**Why it is the cause.** It removes the asymmetry in F3. Shaping's `θ`
effect and the market actions' `u` effect are both visible to the same
comparison. It also removes the internal inconsistency in F4: the gate and
the satisficing test would use the same model of `u`. On the four observed
decisions (hand-computed), `produce_ordinary` satisfices first at ticks 8
and 17. That happens not because shaping is suppressed, but because the
firm can now see that the market remedy relieves the threat — the remedy
is ahead of `lobby` in §12.3's own `SURVIVAL` order. Where no market action
relieves `u` (tick 7: window not yet full), nothing changes.

**Requires.**
1. A new ADR revisiting ADR-0049's Context judgement on one-step freezing,
   plus an interpretation of §12.3 "expected `h_{t+1}` under `a`
   (deterministic core)" — `u` is aux state, not core — probably with a
   §12.3 PATCH clarification.
2. About 10 lines in `satisfices` (or a `DecideCtx::market_margin_at(a)`
   helper), plus unit tests mirroring the hand computation above.
3. Re-running everything ADR-0049 re-ran.

**Risk / trade-off.**
- Changes behaviour in every config where a `SURVIVAL` firm faces
  compliance pressure with a full window. That is likely a numerical-output
  change (MAJOR bump, golden-trace review if any shipped config is
  affected) — **not yet measured**.
- It will very likely make `lobby` *rarer* in compliance-bound `SURVIVAL`.
  That includes `cfg_arm_b_satisficing`, whose ADR-0050 figures (75/1484
  reaches, 0 selections) were measured with frozen `u`. This bears on H3's
  reachability. It is a consequence of correcting the firm's model, not its
  purpose, but it reopens ADR-0050's evidence base for future rounds (not
  its text).
- May change `sc16_gate`/`sc16b_arm_scoping`'s printed SC numbers wherever
  compliance binds in `SURVIVAL` (not measured).
- Does not, by itself, settle what `sc4_wmax_beta_probe` should assert
  (Option B).

### Option B — correct the probe's design, not its threshold (test fix)

**Change.** Split part (a) into the two claims it was actually mixing:
- **(a1)** "no shaping selection under `GOAL` focus at any `(w_max, β)`" —
  ADR-0042's real structural claim. Count only decisions whose logged
  `focus` is `GOAL(j)`, asserting `0`, at full strength.
- **(a2)** an explicit `SURVIVAL` probe that states the prediction from the
  formula *before* the run (`lobby` reachable iff `w_eff ≥` its admissible
  rank and the gate is open) and asserts the cells and decisions match it,
  each with `E[h] > h_t` traceable.

Also add a precondition check that the scenario is in the state it claims
to be in (the probe never checked its own premise — ADR-0056's Note). And
restructure so part (b) is not masked by part (a)'s panic.

**Why this is not lowering the bar.** The current assertion counts two
populations under one premise, and that premise was false from the start:
44/200 `SURVIVAL` ticks at `9c49c34`. Splitting keeps each claim
zero-tolerance for unexplained shaping. Nothing is tolerated that the
formula does not predict, and the predicted cells come from F5's table, not
from the observed result.

The distinction from tuning-to-pass: tuning changes a number until the
observation fits; this changes *what is being measured* to match the claim
the test exists to check, and adds a check that would have caught the
false premise in 2026-09.

**Requires.** `sanity.rs` changes only (count by the `focus` deltas already
in the event log, via `firma_analysis` reconstruction). No model change.

**Risk / trade-off.** Done **alone**, (a2) would ratify F3's asymmetry as
intended behaviour, encoding "lobby always wins a compliance-bound
`SURVIVAL` scan when reached" into a regression test. If Option A is
adopted, (a2)'s expected values must be re-derived after A (at tick 8
`produce_ordinary` would satisfice first). So B should follow A's decision,
not precede it.

### Considered and rejected

- **Drop the `*_success` blocks from the probe's config** (restoring the
  pre-`17ba7bb` config, which gives `0 / 400` at HEAD). This tests a
  configuration with the mechanism switched off — routing around it, the
  forbidden class. Rejected.
- **Close the ADR-0048 gate when `time_to_boundary` is `None`.** This would
  silence these four decisions. But F4 shows `None` is *correct* here
  (continued ordinary production does relieve the threat). The defect is
  the satisficing test not seeing that, not the gate. It also reverses
  ADR-0048's explicit "unbounded ⇒ arrives comfortably" decision. Treats the
  symptom; rejected.
- **Scope ADR-0047 by focus.** It is already `SURVIVAL`-only (F1); nothing
  to scope.
- **Cap `θ_limit` at 1.0** (F6). This would not remove any of the four
  selections (the first two happen at `θ_limit = 0.90`) and changes the
  shaping model. It belongs to its own question.

## Recommendation (a recommendation, not a decision)

**Option A first, then Option B against A's results.**

A is the only option that changes the cause: the firm's lookahead being
blind to the one market lever that relieves the constraint it is being
squeezed by, while seeing shaping's lever in full. It also makes ADR-0048's
gate and the satisficing test agree.

Before accepting A, I recommend an authorized prototype on a branch, with
no commit to `master`, measuring four things:
1. the probe's 15 cells;
2. ADR-0050's `cfg_arm_b_satisficing` figures;
3. `sc16_gate`/`sc16b_arm_scoping`;
4. the four frozen hashes.

Then B, with (a2)'s predictions re-derived under A. If the owner judges
one-step `u` freezing to be an intended bounded-rationality assumption
(i.e. ADR-0049's Context stands), then A is rejected. B alone becomes the
correct fix, and (a2) should then document F3's asymmetry as an explicit,
named model property rather than leave it implicit.

## Consequences

- **Positive.** The 4/400 is explained mechanistically, with every number
  traced. The owner's `GOAL(1)` hypothesis is checked and refuted rather
  than assumed. The real question — whether the `SURVIVAL` lookahead should
  see `u` — is surfaced as a model decision.
- **Negative, accepted.** No fix is applied; `sc4_wmax_beta_probe` keeps
  failing until one is chosen. Option A's broader impact is unmeasured.
- **Neutral.** ADR-0044, 0050, 0051 and 0055 are untouched. SC-4 remains
  unmet under `decision.satisficing` (ADR-0056 Decision 3).

## Compliance

- Evidence: `docs/adr/evidence/adr-0057/` (README lists every file and how
  it was produced).
- Nothing implemented; no test, threshold, cost, gate or success-model
  parameter changed.
- No reserved seeds used (`(909,2,3,4)`, `(11,2,3,4)`).

## Note

ADR-0047 gave the firm a better model of one lever — shaping's expected
effect on `θ`. ADR-0049 then gave the danger projection a better model of
`u`. Each step was locally correct, but the satisficing test was left
between them, seeing shaping fully and market relief not at all. A model
revision that improves fidelity for one class of action should check
whether it has created a fidelity gap against the others in the same
comparison.
