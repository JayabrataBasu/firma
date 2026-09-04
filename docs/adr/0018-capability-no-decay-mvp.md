# ADR 0018 — Capability `c` has no decay in the Phase 2 MVP; late-game dominance is an SC-6 watch item

**Status:** Accepted (2026-09-03)
**Phase:** 2 (Model), Stage 0
**Resolves:** manual §16.3 item 5 ("Capability decay. Not modelled; `c` is
monotone non-decreasing, which is unrealistic and may make `invest_capability`
dominant late in runs.")
**Relates to:** §8.1 (`c ∈ [0,1]`), §8.4 (`Capability` component — "Levels by
domain, decay rate"), §11.1 (action 4 `invest_capability`, `c += δ_c` lagged),
§9.1 (`scope`, `g_3 = θ_cap − c`), §16.2 SC-6, §16.4, §32.1 risk 2, §24.6 A7

## Context

Capability `c` (§8.1, `f64 ∈ [0,1]`) is raised by the market action
`invest_capability` (§11.1 action 4: lagged `c += δ_c` after `Δ_cap` ticks,
`κ_c` cost). Nothing in §11 lowers `c`. §16.3 item 5 flags two things: `c`
being monotone non-decreasing is "unrealistic", and it "may make
`invest_capability` dominant late in runs" — because a firm that has topped up
`c` keeps the benefit forever at no upkeep, so once other goals are satisficed
there is a standing incentive to keep investing.

Countervailing manual signals:

- §16.4 ("What the model deliberately cannot represent") lists internal
  structure, anticipation, learning, geography, perception error, and entry —
  and **does not list capability decay**. So decay is not a planned feature
  being deferred; it is simply absent, and §16.4's job is to say "omission is
  not oversight".
- §8.4's `Capability` component schema *does* say "Levels by domain, **decay
  rate**" — a field exists in the component design, currently unused.
- `c` is hard-capped at 1 (§8.1 domain `[0,1]`), so `invest_capability`
  self-limits: once `c = 1` there is no further gain, and the `scope`
  constraint `g_3 = θ_cap − c` is maximally slack.

## Decision

**The Phase 2 MVP keeps `c` monotone non-decreasing — no decay.**

1. `invest_capability` is the only thing that changes `c`; it only ever raises
   it (lagged, §11.1).
2. `c` is clamped to `[0, 1]`; `invest_capability` past `c = 1` is a no-op gain
   (the cost is still paid at commitment — that is the action's own
   affordability check, not a decay mechanism).
3. The `Capability` component (§8.4, Part C's `firma-domain`) carries a
   `decay_rate` field **fixed at 0.0** for the MVP, so the schema matches §8.4
   and a future decay ADR is a value change plus one line of `constrain`-phase
   logic, not a new field.
4. **No decay parameter is introduced, defaulted, or swept.** `δ_c`, `κ_c`,
   `Δ_cap` (§16.1: 20, 0.05, 3, "fixed") are unchanged.

**The "`invest_capability` dominant late-game" risk is handled by observation,
not pre-emptive design.** It is recorded as a **symptom to watch during Stage 5
sanity-condition checks**, specifically:

- **SC-6** (§16.2): "Repertoire-entropy variance across firm-ticks —
  non-degenerate." If, late in runs, repertoire entropy collapses because
  surviving firms do nothing but `invest_capability`, SC-6 fails, and *that*
  is the trigger.
- Secondary tell-tales: `invest_capability`'s share of `A^used` rising
  monotonically with tick index; `scope` never binding after the first
  quarter of a run.

If SC-6 (or the tell-tales) show the dominance actually materialises in a
regime that otherwise satisfies SC-1…SC-5, the response is a **Phase-2-revision
ADR** that either adds a small decay rate or caps consecutive
`invest_capability` actions — decided against the observed behaviour, with the
decay rate chosen to restore SC-6, not guessed now.

## Rejected alternatives

- **Add a small capability decay now** (`c ← max(0, c − ρ_c)` each tick, or on
  ticks with no `invest_capability`). Rejected: `ρ_c` has no manual default
  and no empirical basis; picking one now is exactly the "excessive degrees of
  freedom / the model can produce any result" hazard of §32.1 risk 2. Decay
  should be added — if at all — *in response to* a demonstrated SC-6 failure,
  with `ρ_c` tuned to the symptom.

- **Cap `c` well below 1** (e.g. `c ∈ [0, 0.8]`) to blunt the incentive.
  Rejected: contradicts §8.1's explicit `[0,1]` domain and would distort the
  `scope` constraint `g_3 = θ_cap − c` and the `s_c = 0.5` scale factor
  (§9.2, "half the capability range").

- **Make `invest_capability` more expensive as `c` rises.** Rejected: a
  convex cost curve is a new modelling assumption with its own parameters, and
  it addresses the symptom by discouraging a legal action rather than by
  fixing the missing dynamic (decay).

## Consequences

**Positive.**
- Simplest choice (A7); matches §16.4's framing (decay is absent, not
  deferred-with-a-plan).
- Zero new parameters — no expansion of the researcher-degrees-of-freedom
  surface (§32.1 risk 2).
- The `[0,1]` cap gives `invest_capability` a natural ceiling, so the
  dominance risk is bounded even in the worst case: a firm cannot invest
  forever *usefully*, only wastefully, and wasteful investment shows up as an
  affordability failure once `r^L` is spent.

**Negative, accepted, and a §33 / publication limitations line.**
- `c` monotone non-decreasing is unrealistic — real capability erodes without
  maintenance. Every Phase 2–4 result is a result about a firm whose
  capability, once built, is free to keep. This joins the MVP's stated
  simplifications (§30.10, §33) and must be named in any publication's
  limitations, not buried.
- If the dominance does materialise, Phase 2 costs a revision cycle (an ADR +
  a small `constrain`-phase change + re-running SC checks). That is the
  accepted cost of not guessing `ρ_c` up front.

**Neutral.**
- **No shipped numerical output changes.** No `c`, no `invest_capability`, no
  `Capability` component, no `constrain` phase exists yet. Phase 1 golden
  trace, `phase1-smoke` run id, and all 55 workspace tests are unaffected.

## Compliance

- Part C's `firma-domain::Capability` carries `level: f64` and
  `decay_rate: f64` (documented "fixed 0.0 for the MVP; see ADR 0018").
- Stage 2's `action.market.standard` implements `invest_capability` as
  raise-only with a `[0,1]` clamp; its `assumption()` notes "capability does
  not decay (ADR 0018)".
- Stage 5's SC harness computes SC-6 and the `invest_capability`-share and
  `scope`-binding tell-tales, and the SC report explicitly checks for the
  late-game dominance symptom.
- Any future decay mechanism requires a superseding ADR citing the SC-6
  evidence that motivated it.
- Manual §34.0 index gains a row for ADR 0018 at the next version bump; §16.4
  is a candidate for a one-line clarification ("capability decay: absent, see
  ADR 0018") at the next revision.

## Note

The discipline this ADR encodes: a known-unrealistic simplification with a
*named, measurable* failure symptom (SC-6) does not get pre-emptively
engineered around. You ship the simple version, you watch the specific gauge
the manual already put on the dashboard for exactly this, and you fix it with
data if the gauge moves. Guessing `ρ_c` now would trade a stated limitation
for an unstated free parameter — a bad trade under §32.1.
