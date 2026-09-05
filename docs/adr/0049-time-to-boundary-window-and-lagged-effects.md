# ADR 0049 — H3 revision, round 3: `time_to_boundary` must advance the compliance window and apply already-pending lagged effects

**Status:** Accepted (2026-09-06) — **implemented on branch
`h3-satisficing-lookahead`, NOT merged**. Round 3 of an owner-directed
revision (ADR-0047 round 1, ADR-0048 round 2). **Supersedes ADR-0048's
`time_to_boundary` scope specifically** — its signature, its `u`-held-fixed
behaviour, and its "negative, accepted" consequence about compliance-only
firms being unbounded. **Everything else in ADR-0048 stands as written**:
the race-branch design (Part B), `Δ_min` vs `Δ_max` (Part B), the
config-only toggle and its default (Part C), and the satisficing/optimising
re-confirmation. ADR-0047 is untouched by this round.
**Phase:** 2 (Model), post-Stage-7 (H3 revision, round 3)
**Relates to:** manual §8.1 (the `Λ` queue), §9.1 (`g_2` compliance, `g_3`
scope, `g_4` obligation), §10.1 phase 6/7 (`resolve_lagged`, `constrain`),
§11.2 (shaping maturity effects), §14.3 (`time_to_boundary`), ADR-0014
(the action window `W` and `u`), ADR-0023 (`Λ`/`Effect::deltas_at_maturity`
as the single source of maturity mutation), ADR-0048 (round 2, superseded in
part by this ADR)

## Context

**This is a defect fix, found after owner review — not the discovery of a
newly-acceptable approximation.** Round 2 (ADR-0048) built a real
decision-time race between a shaping action's lag and `time_to_boundary`,
but that projection only ever advanced `(liquid_capital, input_stock,
capability, obligation)` via `market_core`. It never advanced the
compliance action window `W`, so the regulated-intensity `u` it used stayed
pinned at whatever value it started at for the entire 50-tick projection.
Concretely: **a firm whose only real danger is a compliance violation was
told "no danger foreseeable" by a function whose entire job is foreseeing
danger** — `time_to_boundary` returned `None` (unbounded) for exactly the
case §14.3 built it to catch. ADR-0048 itself named this consequence
("negative, accepted... `time_to_boundary` under a `compliance`-only
binding constraint is always unbounded") and reasoned it was inherited from
an existing, already-accepted `u`-held-fixed approximation used elsewhere in
this codebase. **The owner reviewed that reasoning and rejected it**: this
is not a faithful inheritance of an acceptable simplification — it is an
asymmetry serious enough to question the projection's realism, because
unlike the other one-step-lookahead sites this precedent was drawn from,
`time_to_boundary` is specifically a *multi-tick* forward simulation, and a
window that never advances across fifty simulated ticks is not a
simplification of the same shape as freezing `u` for one step. This ADR
documents the fix, not a newly-discovered exception.

The owner additionally asked for a systematic check — not just the one
named gap — covering every `g_j`'s real-world inputs, whether each is
correctly advanced in the projection, a bug, or deliberately and
justifiably frozen. That audit surfaced a second gap of the identical
shape, not separately named by the owner: **any already-enqueued, real,
not-yet-matured `Λ` effect (a pending `lobby`, `contract`, or
`invest_capability` maturity) was also invisible to the projection**,
because it only ever advanced state via `market_core`'s *hypothetical
continued-action* transition, never by checking the firm's *actual*
committed queue. This is fixed under the same standard as the named gap,
not left as a silent third case.

## Decision

### Part A — the audit (per `g_j`, every real-world input, correct / bug / justified-frozen)

| `g_j` | Formula | Real-world input | Status *before* this round | Status *after* this round |
|---|---|---|---|---|
| `g_1` solvency | `−r^L` | `r^L` (liquid capital) | **1 — correct.** Recomputed every projected tick by `market_core` under the held action (production revenue, input cost, investment cost). | Unchanged — correct, untouched. |
| `g_2` compliance | `u − θ_limit` | `u` (trailing-window mean of the action window `W`, ADR-0014) | **2 — bug.** `W` was never advanced; `u` stayed pinned at its tick-0 value for the whole projection. **This is the named defect.** | **Fixed.** `advance_window` (new, `firma_domain::window`) is called every projected tick with the held action's own index, then `u_from_window` recomputes `u` from the growing projected window — the exact update `ActionWindow::apply` (the real `constrain` rule) performs, because both now call the same function. |
| `g_2` compliance | `θ_limit` (constraint parameter) | **2 — bug, latent.** Only wrong if a `lobby` effect is already pending; otherwise frozen was already correct by accident, not by reasoning. | **Fixed.** `apply_maturing` walks the firm's real, passed-in `pending: &[LaggedRecord]` and applies any `Effect::Lobby` whose `maturity_tick` falls within the simulated horizon, via `Effect::deltas_at_maturity` — reused, not re-derived. When nothing is pending, `θ_limit` correctly stays frozen, now as an emergent property of the walk finding nothing, not a special case. |
| `g_3` scope | `θ_cap − c` | `c` (capability), via *continuing* `invest_capability` | **1 — correct, pre-existing, unrelated to this round.** `market_core`'s `InvestCapability` arm already collapses that action's lag to immediate (§9.3 approximation, documented in its own comment, predates every H3 round). | Unchanged. This round did not touch `market_core`; the pending-effect fix below is additive to it. |
| `g_3` scope | `θ_cap − c` | `c`, via an already-committed, *separate*, still-pending `invest_capability`'s `Effect::CapabilityGain` | **2 — bug.** Same shape as the compliance/`θ_limit` gap: a real, already-enqueued rescue was invisible to the projection. **This is the audit's second finding, beyond the one gap the owner named.** | **Fixed** by the same generic `apply_maturing` walk (`Effect::CapabilityGain` is one of the four variants it dispatches on). Verified directly by `time_to_boundary_sees_an_already_pending_rescue` (Part D). |
| `g_3` scope | `θ_cap − c` | `θ_cap` (constraint parameter) | **3 — deliberately frozen.** No mechanism in this codebase moves `θ_cap` except a `Regulatory` shock (`firma-plugin-shock`, `ACTIVE_SHOCKS`). | **Still deliberately frozen — not fixed this round, reasoning below.** |
| `g_4` obligation | `q − θ_Q` | `q` (obligation stock), via continuing `Deliver` | **1 — correct.** `market_core`'s `Deliver` arm decrements `obligation` immediately, every projected tick — no lag involved. | Unchanged. |
| `g_4` obligation | `q − θ_Q` | `q`, via an already-pending matured `contract`'s `q0` | **2 — bug.** Same shape again — the audit's third instance of the identical gap. | **Fixed** by the same `apply_maturing` walk. `Effect::Contract` emits both an `OBLIGATION` delta (`q0`) and a `THETA_Q` delta (`theta_q_delta`) at maturity; both are now applied. |
| `g_4` obligation | `q − θ_Q` | `θ_Q` (constraint parameter), via the same matured `contract` | **2 — bug** (same effect, same fix). | **Fixed** — same walk, same maturity event, both deltas applied together, exactly as `resolve_lagged` would apply them for real. |
| *(no `g_j` reads it)* | — | `λ` (legitimacy) | **3 — deliberately frozen.** | **Still frozen — correctly so, reasoning below.** |

**Reasoning for the two things this round leaves frozen, stated, not
assumed:**

- **`θ_cap` via a scheduled `Regulatory` shock.** `ACTIVE_SHOCKS` is
  technically readable from any plugin — a not-yet-onset scheduled shock is
  kept in that record precisely so it survives to its onset tick (the
  shock-plugin's own "keep it pending" logic). Incorporating it into
  `time_to_boundary` was considered and **deliberately deferred, not
  fixed**, because it is a different *kind* of gap from the two named
  above: those were the firm's own, agent-owned, already-committed
  decisions (a `Λ` record it enqueued itself) becoming visible to its own
  forward projection — a purely internal consistency fix. A scheduled
  shock is global, exogenous, and cross-plugin; letting a firm's decision
  procedure read ahead into a not-yet-onset shock raises a genuinely
  separate design question — whether a firm's own forward projection is
  *entitled* to know about a shock before its declared observability
  channel would reveal it (`firma-plugin-observation`'s whole reason for
  existing, per ADR-0035). That is an Observation-architecture question,
  not a `time_to_boundary` completeness bug, and folding it in here without
  that separate design pass would quietly grant every firm perfect
  foresight of the future through a side door the Observation plugin was
  built specifically to gate. Flagged here, not silently skipped.
- **`λ` (legitimacy).** No `g_j` formula reads `λ` at all — only
  `SuccessModel::p_success` (ADR-0047's shaping-payoff calculation) does,
  which is untouched by this round. The one channel that *could* move `λ`
  before a boundary is reached — `Compliance`'s `Graduated` violation
  penalty, `δ_λ`, applied in `enforce` (phase 8) — only ever fires *at* a
  compliance violation, which is by definition the exact tick
  `time_to_boundary` already stops at (`margin_of(...) ≤ 0`, checked first,
  before any state is advanced further). `λ` therefore never needs to move
  *before* the answer this function computes; freezing it is not an
  approximation of something that matters, it is a fact about what the
  function terminates on.

### Part B — the compliance-window fix: `advance_window`, one function, two callers

New `firma_domain::window::advance_window(window: &[WindowEntry], tick: u64,
action: u8, l_w: usize) -> Vec<WindowEntry>` — extracted verbatim from
`ActionWindow::apply`'s real logic (append this tick's entry, trim from the
front down to `l_w`), which now calls it instead of inlining the same three
lines. `time_to_boundary` calls the identical function every simulated
tick, with the projected `MarketAction`'s own §11 index, then derives `u`
via the existing `margin::u_from_window`. One function, one home, two
callers — not a hand-derived parallel formula, satisfying the task's
explicit instruction to reuse or say plainly why not.

**The tick-0 seed subtlety, caught before it shipped, not after.** The
caller's own `h_t` at decision time uses `firm_u`, which falls back to the
live `REGULATED_INTENSITY` scalar only while the *real* window is still
empty (a tick-0-only fact). `time_to_boundary` cannot derive that same
fallback from an empty *projected* window, because an empty projected
window doesn't mean the real firm has never acted — it would silently
diverge from the caller's actual `h_t` whenever a firm relies on that seed.
Fixed by taking an explicit `initial_u: f64` parameter, supplied by the
caller as exactly the same `u` it already computed for `h_t`, used only for
the tick-0 boundary check; from tick 1 onward `u` is always freshly derived
from the growing projected window, matching what the real `constrain` rule
would produce.

### Part C — the pending-effects fix: `apply_maturing`, reusing `Effect::deltas_at_maturity`

New private `apply_maturing(state, theta, pending: &[LaggedRecord], tick)`
inside `firma_domain::dynamics`. At each simulated tick (including tick 0,
before the first boundary check — the firm's `decide` phase runs before
this tick's own `resolve_lagged`, but `resolve_lagged` is guaranteed to run
later this same real tick, so any record maturing at the current tick is a
certainty, not a projection), it checks every entry in the firm's actual,
already-enqueued `pending` list against `LaggedRecord::matures_at(tick)`
and, for each match, applies **`Effect::deltas_at_maturity`** — the single
existing source of maturity mutation (ADR-0023), the same function
`LaggedEffectResolver` calls for real — translating the returned `Delta`s
into direct field mutations on the projected `FirmState`/`ConstraintParams`
via a small `apply_projected_delta` dispatch (`CAPABILITY`, `OBLIGATION`,
`THETA_LIMIT`, `THETA_Q`; a `PushGlobalRecord` edge-change delta is ignored
— no `g_j` reads relation-graph topology). No maturity formula is
re-derived; only the *application* of an already-computed `Delta` to a
plain struct (rather than through the kernel's `View`/reconciler) is new,
which is the minimum unavoidable difference between "the real kernel" and
"a pure forward projection over a copied `FirmState`."

`time_to_boundary`'s signature grew from ADR-0048's 7 arguments to 11:
`legitimacy`, `initial_u`, `window`, `l_w`, `pending`, `start_tick` are new;
`aux: &FirmAuxState` was removed (its two fields are now passed
individually, since `regulated_intensity` needed to become mutable state
across the loop rather than a fixed input, and `aspirations` was never
read by `standard_margin`'s constraint terms — confirmed, not assumed, by
grep). All 7 pre-existing `firma-domain` unit tests were updated
mechanically (new arguments inserted, no behavioural change), all still
pass with identical expected outcomes — direct evidence the refactor did
not silently change round 2's behaviour for the cases it already covered.

### Part D — new tests, targeting exactly the surfaced scenarios

- **`time_to_boundary_sees_compliance_danger_the_round_2_projection_missed`**:
  a firm with ample cash/input/capability, repeatedly choosing
  `ProduceRegulated`, with `θ_limit = 0.5` and `l_w = 4`. Hand-computed: `u`
  crosses `0.5` at projected tick 2 (window fills `[2,1] → u=0.25` at tick
  1, `[2,2,1] → ...`; exact trace in the test's own comment). Asserts
  `Some(2)`. Under round 2's code this scenario would have returned `None`
  (unbounded) — confirmed by re-reading round 2's `time_to_boundary` body
  before this round touched it.
- **`time_to_boundary_sees_an_already_pending_rescue`**: a firm already
  below `θ_cap` (violating `scope` *now*, `Some(0)` with no pending
  effect), then re-run with a `LaggedRecord` carrying
  `Effect::CapabilityGain { delta: 0.2 }` maturing at the current tick —
  asserts `None` (the boundary check at tick 0 must see the maturing gain
  applied *before* it evaluates `margin_of`, since `resolve_lagged`
  genuinely precedes the next `decide` for a record maturing this tick).
- Both pass on the first run, matching the hand-computed values exactly.
- **Rounds 1 and 2's existing tests, re-run unchanged**: all 6 pre-existing
  H3 tests in `firma-plugin-decision` pass with zero modification, because
  none of them seed `ACTION_WINDOW` or `LAGGED_EFFECTS` records — `firm_window`
  and `firm_pending_effects` correctly return empty for them, and
  `firm_u`'s existing seeded-`REGULATED_INTENSITY` fallback is exactly what
  the new `initial_u` parameter needs. This is direct evidence of backward
  compatibility, not an assumption.

## Alternatives

- **Leave `u` frozen, document it as an accepted limitation (as ADR-0048
  did).** Rejected — the owner reviewed exactly this framing and rejected
  it as a genuine defect, not an acceptable approximation, for the reason
  stated in Context: a multi-tick forward simulation freezing an input for
  its entire horizon is not the same shape of approximation as the
  single-step lookahead sites this precedent was drawn from.
- **Hand-derive a parallel "projected `u`" formula instead of reusing
  `ActionWindow::apply`'s logic.** Rejected per the task's explicit
  instruction to reuse or say plainly why not — no such reason existed;
  `advance_window`'s extraction was clean and required no design
  compromise on either caller's side.
- **Re-derive lagged-effect maturity inline in `time_to_boundary` instead of
  calling `Effect::deltas_at_maturity`.** Rejected for the same reason —
  `deltas_at_maturity` already exists as the single source of this
  mutation (ADR-0023); duplicating its match arms would create exactly the
  two-copies-of-one-formula risk the manual's "single source of the
  deterministic core" principle (ADR-0021 Decision 4) exists to prevent.
- **Fold the scheduled-shock case into this round's fix, since it is
  technically readable the same way.** Rejected — reasoned through in Part
  A: it is a different kind of gap (cross-plugin, exogenous, entangled
  with the Observation plugin's role) and deserves its own design pass, not
  a piggyback fix under this round's narrower "the firm's own committed
  state" scope.
- **Silently stop at the two gaps the owner named, without the wider
  audit.** Rejected per the task's explicit instruction ("if it surfaces
  something beyond the two gaps already named... fix that too under the
  same standard, don't stop at two") — the audit found a third instance
  (obligation/`θ_Q`/`q` via a pending `contract`) that was not separately
  named, and it is fixed here under the identical standard as the two that
  were.

## Consequences

- **Positive.** `time_to_boundary` now advances every `g_j` input that
  deterministically changes under the projection's own assumptions
  (continued current action, plus the firm's real already-committed
  future) — compliance is no longer structurally invisible to the race
  check, and a firm with a genuinely pending rescue is no longer told it is
  in danger when it provably is not. ADR-0048's race mechanism, `Δ_min` vs
  `Δ_max` reasoning, config toggle, and satisficing/optimising boundary are
  all unaffected and re-confirmed, not re-litigated.
- **Negative, accepted.** `time_to_boundary` still cannot see a scheduled-
  but-not-yet-onset shock moving `θ_cap`, and still cannot see `λ` move
  (though, as reasoned in Part A, the second is not actually a limitation
  given what the function computes). The first is a real, named boundary
  of this round's fix, not a claim of completeness beyond what was audited.
- **Neutral.** `time_to_boundary`'s signature is now four parameters longer
  (11 total); the two new small functions (`advance_window`,
  `apply_maturing`/`apply_projected_delta`) are the only additions to
  `firma_domain`'s public/private surface. `DecideCtx` gains `window`,
  `l_w`, `pending`, `tick` alongside round 2's `prev_action`.

## Compliance

- Two new `firma-domain` unit tests targeting exactly the two named
  scenarios (Part D), both hand-verified, both passing on first run.
- All 7 pre-existing `time_to_boundary` unit tests (rounds 1/2) updated
  mechanically for the new signature, all still pass with identical
  expected outcomes.
- `firma-plugin-constraint`'s `ActionWindow` refactored to call
  `advance_window` instead of inlining the same logic — its full 20-test
  suite re-run, same count, same results, confirming the extraction changed
  nothing observable.
- All 6 pre-existing H3 `firma-plugin-decision` tests (rounds 1/2) pass
  unchanged, with zero test-code modification, after `DecideCtx` gained 4
  new fields — direct evidence of backward compatibility.
- Both VT-8 criterion-(iii) directional greps re-run fresh — still empty.
- Full VT-1…VT-8 (10/10), the `sc16_gate`/`sc4_wmax_beta_probe`/
  `sc16b_arm_scoping` sanity harness (all pass — ADR-0044's per-hypothesis
  gate-clear finding for H1a/H1b/H1c/H2/H4 re-confirmed by `sc16_gate`
  specifically), full workspace `cargo test` (green, 0 failures), full
  workspace `cargo clippy --all-targets -- -D warnings` (clean),
  `cargo fmt --all --check` (clean after applying), `lint-architecture.sh`
  (9/9), `check_deps.py` (clean), `cargo tree -p firma-kernel` (unchanged:
  `firma-core`, `firma-rng`, `serde` only) — all re-run this round.
- All four frozen hashes re-confirmed byte-identical this round: golden
  (`run_id f304edd4…`), `phase1-smoke` (`14b9eb59…3f593a0` /
  `310f636f…a5ae4945`), `phase2-smoke` (`0529c6bb…f017` /
  `9a476938…eadb`), `phase2-stage5-smoke` (`5824031c…df59` /
  `fd3aa4f2…06e1`) — expected, since neither shipped smoke config
  configures `shaping`, so none of this round's new code paths are
  exercised by them.

## Note

The scheduled-shock gap named in Part A (`θ_cap` via a not-yet-onset
`Regulatory` shock) is deliberately left open, not fixed, and is a good
candidate for its own future ADR once the Observation-architecture question
it raises — whether a firm's forward projection may see further ahead than
its own declared observation channel — has had its own design pass. Folding
it in here would have been a scope-widening shortcut past that question,
not a fix of the same kind as the two gaps this round addresses.
