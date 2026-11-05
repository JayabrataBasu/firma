# ADR-0057 — Symmetric prototype: predeclared choices and criteria

**Written before any measurement**, per the owner's Part 3/Task 2 instruction.
Code: `option_a_symmetric_prototype.diff` (this folder), applied in the worktree
`/home/jayabratabasu/firma-adr0057-proto` on top of clean `c5eb8cd`
(`option_a_prototype.diff` is **not** overwritten — it stays as the
market-only Option A prototype). `sha256sum` of the diff:
`a712ed9e4298e35c78173c907ec058498b519d72c6eb21e73f6d415bb2d8d267`.

## 1. What lobby's next-tick window actually contains if lobby is chosen, and how the prototype represents it

From the Part 2 read: `constrain` (`ActionWindow::apply`,
`firma-plugin-constraint/src/phase_rules.rs:137–170`) reads only
`keys::SELECTED_ACTION` and calls `advance_window(w, tick, action, l_w)`
unconditionally for every live agent, **regardless of action class and
regardless of whether a shaping action's later success/failure draw has
happened yet** (that draw occurs in `act_shaping`, a different phase, and its
*effect* on `θ`/capability/obligation is deferred to `resolve_lagged` at
`maturity_tick`, but the *window entry* is not deferred — it is written the
same tick the action was selected, by `constrain`, independent of maturity).
So if lobby (index 6) is chosen at tick `t`, the real engine's window at
tick `t+1` is `advance_window(W_t, t, 6, l_w)` — exactly the same shape of
entry a market action would produce, differing only in which index is
recorded. Since `u_from_window`'s indicator is `action == PRODUCE_REGULATED`
(`= 2`) and `6 ≠ 2`, recording a `lobby` tick has the *same* effect on `u` as
recording any other non-regulated action: it cannot raise `u`, and it drops
the oldest window entry once full.

The prototype represents this with one new method,
`DecideCtx::aux_after(a, aux)` (`lib.rs`, new, ~20 lines), which calls
`advance_window(&self.window, self.tick, a, self.l_w)` then
`margin::u_from_window` on the result, for **whatever `a` is being
evaluated** — a market action, `lobby`, `contract`, or `diversify` alike. It
replaces only `FirmAuxState::regulated_intensity`; `legitimacy` and
`aspirations` are carried over unchanged (`..*aux`), matching the existing
codebase's own stance that `λ` is held fixed within a one-step/multi-tick
projection until an `enforce`-phase violation
(`firma-domain/src/dynamics.rs:278–280`, already the design `time_to_boundary`
uses).

`aux_after` is called from two sites:
- the unified cost-only fallthrough in `satisfices`'s `Focus::Survival` arm
  (covers all 6 market actions, `lobby`/`contract` when unconfigured or gate-
  closed, and `diversify`, which always falls through);
- `shaping_expected_survival_margin`'s `h_success`/`h_failure` computation,
  for `lobby`/`contract` when configured and gate-open — **both branches use
  the same `next_aux`**, because the window entry does not depend on which
  of success/failure is realised (see §1 above — only `a` was selected).

## 2. Modeling choices the manual does not specify

**None found that require a silent default or a stop.** I audited every
place the symmetric extension could have introduced a free choice beyond
what Option A already made, and each one resolves without a decision, either
because the real engine's own code mechanically determines the answer, or
because the task's own instruction fixes it, or because it is already
precedented by existing, unchanged code:

| Candidate choice | Resolution | Why it is not a free choice |
|---|---|---|
| Does `lobby`/`contract`'s `h_success` and `h_failure` use the same advanced window, or two different ones (e.g. one reflecting a maturity-fired world)? | Same window for both. | `ActionWindow::apply` reads only `SELECTED_ACTION`, never `LAGGED_EFFECTS` or any success/failure flag (`phase_rules.rs:143–157`) — mechanically determined, not assumed. |
| Does `diversify` (8) need special-casing in `u_from_window`? | No special case; it falls out of the existing indicator `action == PRODUCE_REGULATED` the same as any non-2 index. | `margin.rs:29–37` — the function does not distinguish *which* non-regulated action was taken. |
| Should the ADR-0048 gate's `time_to_boundary` call (which projects *continuing `prev_action`*) also use the newly-advanced-by-`a` window? | No — left untouched. | Fixed by the task's own instruction ("keep ADR-0047's lag semantics as-is... change nothing else"), and conceptually a different question (what happens if the firm keeps doing what it's already doing, vs. what happens if it takes `a` now) — not a default I picked. |
| Should `θ`'s success benefit (`δ_θ`, `δ_q`) also be deferred to reflect the real drawn lag (`min 2, max 6` ticks in every probed config), to match `u` now being modelled at one-step granularity? | No — left untouched, exactly as the task instructed. | This is the asymmetry logged as a new open question below (§3), not resolved here. |
| Does `h_t` (the current-tick margin being compared against) need to change? | No. | `h_t` is Step 1's "Evaluate" quantity (§12.3) — a fact about *now*, not a counterfactual under a candidate `a`. Neither Option A nor this prototype touches its computation (`Satisficing::apply`, unchanged). |
| Does `market_core` need any change to stay consistent with the advanced window? | No. | `market_core` operates purely on `FirmState`; `u` tracking is explicitly a caller concern, not part of this core (§9.3, ADR-0021 D4) — already the design the codebase states for itself. |

If any of these had actually required picking an unstated default, I would
have stopped here rather than proceeding — none did.

## 3. New open question: lobby/contract's `θ` benefit is still credited at one-step granularity while its real lag is 2–6 ticks

Logged in `PROGRESS.md` as **OQ-22** (not fixed, per the task instruction to
keep ADR-0047's lag semantics as-is). Summary: every probed config draws
`lobby`'s and `contract`'s maturity lag uniformly from `{2, …, 6}`
(`"lag": {"min": 2, "max": 6}`, `docs/adr/evidence/adr-0057/mkcfg.py:5–6` and
`tests/tests/sanity.rs`'s `lobby_success`/`contract_success` blocks) — the
real effect on `θ_limit`/`θ_Q` cannot land before tick `t+2` at the earliest,
and ADR-0048's gate already treats this race explicitly. The symmetric
prototype now models a market action's `u` effect at exactly the correct
one-step-ahead granularity (`t+1`, matching `constrain`'s own timing), but
`lobby`/`contract`'s `θ` effect inside `shaping_expected_survival_margin`
(`h_success = margin_with_theta(state_success, next_aux, theta_success,
scales)`, unchanged by this prototype) still assumes `δ_θ`/`δ_q` is realised
*in the same one-step comparison* — i.e. at `t+1`, the same tick `u`'s effect
now correctly lands, even though the earliest `θ` could actually move is
`t+2` and the gate's own `lag_min` can be as late as `t+6`. So even under the
symmetric prototype, `lobby` keeps a structural one-step-timing advantage
over the market actions it is now being compared against on equal `u`-terms:
its benefit is still pulled forward to a tick it cannot actually arrive at
except in the `lag_min = 2` best case the ADR-0048 gate already screens for
separately. This is not evaluated or fixed here.

## 4. Decision criteria, fixed now, before any measurement

**Why exact equality, not a percentage band.** The two binaries differ in
exactly one respect: `aux_after` is now also called from the `lobby`/
`contract` cost-only fallthrough, the `diversify` fallthrough, and
`lobby`/`contract`'s `h_success`/`h_failure` when gate-open and configured.
For every market action (`a < 6`), `aux_after(a, aux)` computes **bit-for-bit
the same thing** Option A's dedicated `a < 6` branch already computed (same
two functions, same arguments) — so A and A-symmetric are behaviourally
identical for every decision that never reaches `a ≥ 6`, and for every
decision that reaches `a ≥ 6` only after a market action already satisficed
(the scan stops at the first satisficing hit). The run is deterministic and
each tick's `SELECTED_ACTION` feeds next tick's window, so this is a
cascading system, not a noisy one: a changed decision at tick `t` changes the
window from `t+1` onward, which can change later decisions too. There is no
principled "small" percentage difference in this setting — either a given
measurement is produced by a chain of decisions identical to A's, in which
case it is byte-identical, or at least one decision in the chain differs, in
which case the divergence is a real, traceable effect of this specific code
change, not sampling noise. A tolerance band would either be too loose to
mean anything or would mask a genuine divergence, so none is used.

**Primary metrics (the ones the task named — lobby-reached count and probe
cells), both existing scenarios only:**

1. **Probe, 15 cells, current config** (`cfg_wide_search` sweep,
   `docs/adr/evidence/adr-0057/sweep15_head_vs_pre.txt` is the HEAD/pre-
   `17ba7bb` baseline; Option A's value is `0/400` in all 15 cells, from
   `prototype-option-a/README.md` row 1). **Stable** = A-symmetric gives
   `0/400` in all 15 cells, matching A exactly. **Swings** = any cell's count
   differs from A's `0`, reported cell-by-cell with the exact count, no
   characterisation.
2. **Probe, 15 cells, pre-`17ba7bb` config.** Both HEAD and A already give
   `0/400` in all 15 cells. **Stable** = A-symmetric also gives `0/400` in
   all 15. **Swings** = any nonzero cell.
3. **`cfg_arm_b_satisficing` lobby-reached count** (the scan reaches action
   6's `SURVIVAL`-branch evaluation at all — `shaping_eval_entered` trace
   events for `action: 6`). A's measured value: **42** (down from HEAD's 75;
   `prototype-option-a/README.md` row 3). **Stable** = A-symmetric's reached
   count is **exactly 42**. Per the structural argument above, this should
   hold with certainty: reaching action 6 in the scan order
   `[1,3,5,2,0,4,6,7,8]` depends only on actions 1,3,5,2,0,4's admissibility
   and satisficing outcomes, all of which are market actions (`a<6`) and
   therefore identical between A and A-symmetric. **Swings** = any deviation
   from 42 at all, which would itself be informative (it would mean some
   market-action decision differs between A and A-symmetric despite the
   `a<6` code path being unchanged — a correctness flag worth surfacing on
   its own, not just a modelling question).

**Supporting metrics (existing scenarios, full previous-round measurement
set, reported regardless, not themselves the stability verdict):**

4. **`cfg_arm_b_satisficing` selections** (lobby/contract/diversify actually
   picked under `SURVIVAL`). A: **0**. Stable = A-symmetric is also **0**.
5. **`cfg_arm_b_satisficing` event log** — A's log was byte-identical to
   HEAD's `8663d47a…`. Stable = A-symmetric's log is also byte-identical to
   that same hash. Any difference is reported with the first diverging tick.
6. **Part (b)** (`cfg_input_starved`, run directly): A gave `3/600`
   (unchanged from HEAD, since the triggering decisions are tick-0 fallback
   picks with no gate/payoff record — `prototype-option-a/README.md` row 2).
   Stable = A-symmetric also gives `3/600`.
7. **Window variants** (`l_w=6`, `l_w=8`, `w_max=15`) picks. A: `0`, `12`,
   `5` respectively (`prototype-option-a/README.md` row 3). Stable =
   A-symmetric matches each exactly.
8. **`sc16_gate` / `sc16b_arm_scoping`** printed SC lines. A: every line
   identical to HEAD. Stable = A-symmetric also identical to HEAD.
9. **Four frozen golden hashes.** A: all four unchanged from HEAD. Stable =
   A-symmetric's are unchanged too. Any difference here is a correctness
   flag, not a "swing" — it would mean something outside this change's
   intended scope moved.
10. **Failing-test set.** A's 5 failures: `h3_channel_opens_in_a_real_run`
    plus `h3_lobby_satisfices_survival_when_expected_relief_clears_h_t`,
    `h3_the_survival_test_is_not_lag_sensitive_by_design`,
    `h3_race_short_lag_wins_lobby_satisfices`,
    `h3_toggle_off_reverts_to_payoff_only_despite_long_lag`. Reported as a
    set comparison (A-symmetric's failing set vs. A's), not scored
    stable/swing — a changed failing-test set here is expected to be
    reported precisely, since the extra code paths this prototype touches
    (gate-closed/unconfigured fallthrough, `diversify`) are exactly where
    more of the existing fixtures' "frozen `u`" assumptions could live, per
    `prototype-option-a/README.md`'s own note that the five A-failures share
    one construction (empty-window, seeded-`u` fixtures).

**Headline verdict, defined now:** "H3's footprint is stable across A and
A-symmetric" means metrics 1–3 above are all exact matches to A's measured
values. Any single mismatch in 1–3 is reported as "swings," with the exact
numbers and (where traceable from the trace log) the first tick at which the
selected action differs between A and A-symmetric. Metrics 4–10 are reported
in full either way, not folded into the verdict.

## 5. Test scenarios used

**Existing scenarios only, exactly as named above — no new scenario is
constructed.** This reuses, unmodified: the probe's `cfg_wide_search` 15-cell
sweep (current and pre-`17ba7bb` configs), `cfg_input_starved` (part b),
`cfg_arm_b_satisficing`, the three window-variant configs (`l_w=6`, `l_w=8`,
`w_max=15`), `sc16_gate`, `sc16b_arm_scoping`, and the full
`cargo test --workspace` / `clippy` / `fmt` / architecture-lint /
`check_deps.py` suite.

**Proposal, not acted on here:** the five H3 demonstration tests that fail
under Option A (and are expected to still fail, or fail differently, under
the symmetric prototype) share the single empty-window/seeded-`u` fixture
construction `prototype-option-a/README.md` identifies. If Option A or the
symmetric variant is ever adopted, those five fixtures would need rebuilding
around a window that is actually populated (so the lookahead has real
history to advance), not a `regulated_intensity` seed the engine itself
already discards after tick 0. That rebuild is out of scope for this task
and is not done here.
