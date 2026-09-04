# ADR 0014 — `u` (regulated-activity intensity) is a simple trailing-window mean

**Status:** Accepted (2026-09-03)
**Phase:** 2 (Model), Stage 0
**Resolves:** manual §16.3 item 1 ("`u` trailing-window definition. Simple mean
over `L_W` assumed; exponentially weighted is an alternative.")
**Relates to:** §8.1 (auxiliary state, the note on `u`), §9.1 (`compliance`,
`g_2 = u − θ_limit`), §9.2 (scale `s_u`), §10.1 phase 7 (`constrain`),
§15.3 (Example C, ticks 2–3), §24.6 A7

## Context

§16.3 lists this as an "ambiguity to resolve in Phase 1", but §26.3's Phase 1
gate forbade domain logic, so it was not resolved. Stage 1's `compliance`
constraint plugin (§9.1) computes `g_2 = u − θ_limit`, and `u` is the only
input to that constraint that is not a raw state field — it is a derived
trailing statistic of the firm's own past actions (§8.1). Its definition
therefore has to be pinned before `compliance` is written.

The manual states (§8.1, note on `u`): "a *derived* trailing statistic of past
actions … `u` is fully determined by `W_{i,t}` … An implementation MAY
recompute `u` from `W`; results MUST be identical." `W_{i,t}` is the action
window — "Last `L_W` actions" (§8.1). §16.3 names two candidates: a simple
mean over `L_W`, or an exponentially weighted moving average (EWMA).

The choice affects how fast compliance pressure decays after a regulatory
shock, and therefore recovery timing — §15.3 (Example C) is the manual's own
worked reference: after the tick-2 shock the firm stops doing regulated
production and `u` "decays out of the window" (0.55 → 0.50 → …), lifting `h`
back above `h_crit` over the following ticks.

## Decision

**`u` is a simple arithmetic mean over the fixed-length `L_W` action window:**

```
u_{i,t} = (1 / L_W) · Σ_{k=1..L_W} ι(W_{i,t}[k])

ι(a) = 1  if a = produce_regulated
ι(a) = 0  otherwise
```

- The denominator is **always exactly `L_W`**. Window slots not yet filled
  early in a run count as `ι = 0` (equivalently: the window is pre-filled with
  `hold`), so a fresh firm has `u = 0`.
- `u` takes values in `{0, 1/L_W, 2/L_W, …, 1}` and changes by a multiple of
  `1/L_W` per tick.
- `u` is **recomputed in the `constrain` phase** (§10.1 phase 7) from the
  action window as it stands after that tick's `act_market` phase, so `u_t`
  reflects the action selected in phase 3 / executed in phase 4 of tick `t`.
- Per §8.1, `u` MAY be cached on the firm's auxiliary state for efficiency,
  but any cache MUST equal the value recomputed from `W` bit-for-bit.

`L_W` default 8, swept `{4, 8, 16}` (§16.1) — unchanged.

### On §15.3's numeric values

§15.3's `u = 0.55` and `u = 0.50` are **illustrative trace inputs**, not
hand-computed fixtures. Unlike §15.1 (margin — exact table), §15.2 (kernel —
iteration counts), and §15.4 (entropy — "Fixture tolerance 10⁻⁶"), §15.3
gives no numeric-fixture framing or tolerance for `u`; it posits "suppose the
firm's `u` is 0.55 here" and traces the decision from there. `0.55` is not on
the `k/8` grid this definition produces for `L_W = 8`. The definition above is
consistent with §15.3 **qualitatively** — `u` "decays out of the window" as
`produce_regulated` actions age past the `L_W` boundary, exactly the wording
§15.3 uses. If a future revision wants §15.3 to be a tight numeric fixture,
the per-action contribution `ι` needs an explicit value in the manual; that is
a PATCH-level clarification (§0.6), not a change to this decision.

## Rejected alternatives

- **Exponentially weighted moving average (EWMA), `u_t = (1−ρ)·u_{t−1} + ρ·ι(a_t)`.**
  More faithful to how a real regulator might weight recent activity, and
  §16.3 names it. Rejected for the MVP: (a) it introduces a new smoothing
  parameter `ρ` with no manual default and no empirical basis — precisely the
  researcher-degrees-of-freedom hazard §32.1 risk 2 warns about; (b) "decays
  out of the window" (§15.3) is windowed-mean language, not exponential-decay
  language; (c) an EWMA is "determined by `W`" only in a weaker sense (it needs
  the whole history, or a carried running value that can drift from a
  recompute) — the windowed mean is exactly recomputable from the `L_W`-slot
  window, which is what §8.1's "results MUST be identical" wants; (d) it is the
  less boring choice (A7). EWMA stays available as a Phase-3+ sensitivity
  variant: if AT-1 (halve tick length) shows recovery timing is a knife-edge,
  re-open this with a new ADR.

- **Windowed mean with a variable denominator** (divide by the number of
  filled slots early in a run). Rejected: makes `u` jumpy in the first `L_W`
  ticks and complicates the "fresh firm at `u = 0`" baseline for no modelling
  gain.

- **Count of `produce_regulated` over a trailing *tick* window** (rather than
  an *action* window). Equivalent here because the MVP firm takes exactly one
  action per tick (§12.3), but §8.1 defines `W` as an action window, so the
  ADR follows §8.1's framing to stay forward-compatible with a firm that could
  take multiple actions per tick in a later phase.

## Consequences

**Positive.**
- `compliance` (Stage 1) has an unambiguous, integer-friendly `u`.
- `u` is exactly recomputable from `W` — §8.1's identity requirement holds by
  construction, and a cache-vs-recompute determinism test is trivial to write.
- Recovery dynamics after a regulatory shock are a clean function of `L_W`
  (the sweep factor), which is the behaviour §15.3 describes.

**Negative, accepted.**
- Recovery is *stepwise* (multiples of `1/L_W` per tick), not smooth. A real
  regulator's assessment is presumably smoother. This is a known abstraction,
  stated here; the smooth alternative is EWMA, deferred above.
- §15.3's specific `u` values are not reproducible from this formula for
  `L_W = 8`; §15.3 is therefore a *decision-trace* fixture (the scan order,
  `ψ`, `w_eff`, the aspiration update) and not a `u`-value fixture. Stage 1's
  `compliance` tests will fixture `u` against hand-computed windows of known
  action sequences instead.

**Neutral.**
- **No shipped numerical output changes.** No `compliance` constraint, no `u`
  computation, and no `constrain` phase logic exist yet. The Phase 1 golden
  trace, `phase1-smoke` run id, and all 55 workspace tests are unaffected.
  This ADR governs code Stage 1 will write.

## Compliance

- Stage 1's `firma-plugin-constraint` (`compliance`) implements the formula
  above; a unit test fixtures `u` for several hand-built `L_W`-action windows
  including the empty-window (`u = 0`) case.
- A determinism test asserts a cached `u` equals the value recomputed from `W`
  (§8.1 identity requirement).
- The `constrain`-phase recompute point (§10.1 phase 7) is asserted by an
  integration test once the phase scheduler carries domain rules.
- Manual §34.0 index gains a row for ADR 0014 at the next version bump; until
  then this file is the record. The `ι` per-action value is a candidate
  §15.3 PATCH clarification, noted for the next manual revision.

## Note

The windowed mean was the manual's own stated assumption ("Simple mean over
`L_W` assumed"). This ADR does little more than make that assumption binding
and specify the denominator and the empty-window baseline — the two details
that "simple mean" leaves open. The interesting downstream question — whether
stepwise vs. smooth recovery changes any E1 conclusion — is a Phase-3 AT-1
matter, and this ADR deliberately does not pre-empt it.
