# ADR 0027 — `decision.random`'s selection mechanism

**Status:** Accepted (2026-09-04)
**Phase:** 2 (Model), Stage 3
**Relates to:** manual §20.3 (lists `decision.random` as an MVP `Decision`
plugin, no formula), §25.4 VT-5 ("`decision.random` null ⇒ no margin/rigidity
relationship — establishes the null"), §28.3 (the alternative-explanation
discipline: narrowing could be admissibility shrinking, discriminated by
`|A^used|` vs `|A^adm|`), §28.4 ("Random decision … Results require the
decision mechanism"), §30.4 Arm C ("β = 0 **and** `decision.random`. Both must
show no relationship"), §21.3 (stream separation), §12.3 (the procedure
`decision.random` is the null *against*); ADR-0021 (same "category exists,
formula doesn't — write an ADR" situation as the `Constraint` interface)

## Context

§20.3 names `decision.random` as an MVP `Decision`-category plugin and §25.4 /
§28.4 / §30.4 fix its *purpose* — a structural null — but **no section of the
manual specifies its selection rule.** Same situation as Stage 1's `Constraint`
interface (ADR-0021): design it, don't guess. Three sub-questions, each decided
below with the reasoning §25.4 / §28 demand.

## Decision

### 1. Uniform over the **admissible set**, not over all nine actions

`decision.random` selects **uniformly at random from the set of admissible
actions** at the firm's current state — `{ a : admissible(a, x, θ, e) }`,
enumerated in ascending action index for determinism, one draw with
`next_below(|set|)`.

- `admissible` is the **same** predicate `decision.satisficing` uses
  (ADR-0021 Decision 2 + this Stage): in-repertoire ∧ affordable ∧ (for
  scope-gated `produce_regulated`) `c ≥ θ_cap` ∧ (for `contract`) has a
  `supply` partner. For the six market actions this is exactly
  `firma_domain::dynamics::market_core(a, …).is_some()`.
- The admissible set is **never empty**: `hold` (index 0) has precondition
  "always" (§11.1). So no rejection loop, no "draw again if inadmissible".

**Why not uniform over all nine (with inadmissible actions executing as
no-ops)?** Because that couples behaviour to `h` through the back door: as
constraints tighten the admissible set shrinks, so a uniform-over-all firm
spends an increasing fraction of ticks doing nothing — an idle-rate that rises
as `h → 0`. That is a rigidity-shaped signal correlated with margin,
manufactured by the *null's own construction*, which is precisely what
"establishes the null" must not do. Uniform-over-admissible means the firm
always *acts*; any residual `h`↔repertoire relationship is then attributable to
the admissible set shrinking (§28.3's named alternative explanation), which the
experiment *wants* visible and discriminates by reporting `|A^used|` against
`|A^adm|` — not masked by the decision plugin.

**Why not uniform-over-all with rejection-until-admissible?** Distribution-
identical to uniform-over-admissible, but consumes an unbounded number of RNG
draws per decision (breaks the "one well-defined draw per decision" property
that keeps stream accounting legible, §21.2) for no benefit.

### 2. It ignores `focus` / Attend / Narrowing / Scan-order / Satisficing **entirely**

`decision.random` runs **none** of §12.3 Steps 2–5. It does not compute `h` for
attention, does not compute `ς_j`, does not compute `ψ(h)` / `w_eff`, does not
consult the scan-order table, does not apply the satisficing test. It computes
the admissible set and draws.

Arm C pairs `decision.random` with `β = 0`. `β = 0` already removes *narrowing*
(`ψ ≡ 1`, `w_eff = w_max`) while leaving the rest of §12.3 intact — that is the
"No narrowing" null (§28.4), isolating the narrowing mechanism. `decision.random`
is the *stronger* null (§28.4 "Results require the decision mechanism"): it
removes attention, priority ordering, and satisficing too. For it to be that
null it must not re-introduce any of them. In particular it must not compute
`focus`, because `focus` (via the scan-order table) is the R3 mechanism and a
random selector that still consulted `focus` would not be a clean null against
it.

The firm's `Attention` state (`keys::FOCUS`, `keys::W_EFF`) is left **unset** by
`decision.random` (or, if a prior `decision.satisficing` tick set it, stale and
ignored — like `selected_action`, ADR-0022 Decision 1's clearing rule). Offline
analysis keys off the *decision plugin id* recorded in the manifest, not a
per-tick focus value, to know an arm is the null.

### 3. RNG — `mechanism` stream, `purpose_tag = "decision_random"`, per acting agent

`firma_rng::open_for(&key, Some(agent.0), "decision_random")`, one
`next_below(|admissible|)` draw per firm per `decide` tick. The `decide` phase
maps to the `mechanism` stream (§21.3: `mechanism` = "decisions, action
outcomes, shaping success"; the kernel's `stream_for_phase` sends every
non-`environment` phase to `mechanism`). This is the **first `Decision`-category
RNG use**; it mirrors the shaping rules' `open_for` per-agent pattern (ADR-0023
Decision 3) so a matched-`mechanism` ablation (`satisficing` vs `random`, same
`environment`/`shock` seeds — §21.3) is well-defined.

## Alternatives

- **Uniform over all nine, inadmissible → no-op.** Rejected per Decision 1 —
  manufactures an idle-rate/`h` correlation.
- **Uniform over the full nine-action repertoire but re-draw if inadmissible.**
  Rejected per Decision 1 — unbounded draws per decision.
- **Random *among the satisficing* actions** (run Steps 1–5, then pick
  randomly among those that pass). Rejected — that is a null against
  *first-vs-random tie-breaking*, not against the decision mechanism; it keeps
  attention, narrowing and the priority table, so it fails Arm C's purpose.
- **Weight the draw by `ψ(h)` or repertoire history.** Any weighting that
  depends on `h` or past actions reintroduces exactly the structure the null
  must lack.

## Consequences

- **Positive.** A genuine structural null: with `decision.random`, repertoire
  concentration cannot arise from decision narrowing (there is none) or from
  attention/priority (ignored). VT-5's expected result — no `h`/rigidity
  relationship — follows by construction, and VT-5 *checks* that the
  implementation delivers it.
- **Negative, accepted.** A residual weak `h`↔repertoire relationship *can*
  still appear via admissible-set shrinkage. This is intended (§28.3) and is
  what `|A^used|` vs `|A^adm|` reporting exists to separate; VT-5's report
  states the correlation and its CI rather than asserting exactly zero.
- **Neutral.** `decision.random` shares the admissibility helper with
  `decision.satisficing`; no duplication.

## Compliance

- `firma-plugin-decision::DecisionRandom` — `phase() == Decide`; `apply()`
  builds the admissible set (ascending index), draws once via
  `open_for(_, Some(agent), "decision_random")`, emits
  `SetAgentInt { field: keys::SELECTED_ACTION, value }`; emits **no** `FOCUS` /
  `W_EFF`.
- Grep the plugin: the only `firma_rng` call is the single
  `open_for(_, _, "decision_random")`; no `focus` / `psi` / `w_eff` /
  `satisfic` symbols in `DecisionRandom`.
- Tests: VT-5 (§25.4, reporting); a unit test that the draw ranges exactly
  over the admissible set and that a fully-constrained firm (`hold` only) is
  deterministic.
