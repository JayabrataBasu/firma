# ADR 0015 — Constraint parameters θ are global for the Phase 2 MVP

**Status:** Accepted (2026-09-03)
**Phase:** 2 (Model), Stage 0
**Resolves:** manual §16.3 item 2 ("θ globality. Global assumed. Per-firm
regimes would change H5 substantially.")
**Relates to:** §8.2 (constraint parameters), §9.4 (endogenous constraint
set), §2.3 SQ4, §2.4 H5, §13.2 (regulatory shock channel), §26.6 Phase 4,
§34.7

## Context

§8.2 already states: "**Global by default** (one regime for all firms).
Per-firm variant is Phase 4. This is the object making the viability kernel
endogenous (§9.4)." §16.3 re-flags it as an ambiguity to *settle* with an ADR,
and adds the research-design stake: "Per-firm regimes would change H5
substantially."

`θ = (θ_limit, θ_cap, θ_Q)` is the object every constraint-shaping action
moves (§9.4: `θ_{t+1} = γ(θ_t, {a^c_{i,t}}_i, ζ_t)`) and the object the
regulatory shock channel perturbs (§13.2: `Regulatory` → `θ_limit −= m` or
`θ_cap += m`). It is the shared surface on which threat and response act
(§13.2: "Threat and response act on a shared surface, which makes the
two-sided version (Phase 4) coherent").

**Research-design consequence.** H5 (§2.4) is: "Under rivalry, successful
constraint-shaping by one firm contracts rivals' viability kernels", with DV
`kernel_volume_delta`; SQ4 (§2.3) asks whether one firm's shaping "impose[s]
externalities on rivals' viability kernels". With **global θ**, one firm's
successful `lobby` moves `θ_limit` for every firm, so the externality channel
is `θ` itself — mechanical and total. With **per-firm θ**, a firm shapes only
its own boundary and there is *no* automatic cross-firm externality; H5 would
then need a different mechanism (shared resource pools, shared regulator
attention) and would be a different hypothesis. So this is not only an
implementation choice — the shape of H5/E8 depends on it.

## Decision

**All three components of θ are global — one shared vector per world — for the
Phase 2 MVP, Phase 3 (E1), and Phase 4's E8/H5.** Per-firm θ is deferred to
Phase 4 as a distinct, ADR-gated capability, exactly as §8.2 says.

- `θ_limit : f64`, `θ_cap : f64` — compared against `u` and `c` (both `f64`).
- `θ_Q : i64` — compared against `q` (`i64`) in `g_4 = q − θ_Q`; kept integer
  so that constraint is exact integer arithmetic (§21.4 spirit, matching q's
  type). §15.1's `g_4 = 30 − 100 = −70` holds under either type; `i64` is
  chosen for exactness and type-consistency with `q`.
- θ lives once on world state (not on any firm's component bag). It is read by
  every firm's `compliance`/`scope`/`obligation` constraint and written only
  by the `constrain` phase (§10.1 phase 7), which folds in matured shaping
  effects (`resolve_lagged`, phase 6) and regulatory shocks (`environment`,
  phase 1).

**H5/E8 (Phase 4) uses global θ as the externality channel.** If Phase 4 also
introduces per-firm θ, that work must sequence *after* E8 or E8 must pin
"global θ" explicitly in its `ExperimentSpec`, so the two do not silently
confound.

## Rejected alternatives

- **Per-firm θ from the Phase 2 MVP.** Rejected: (a) §8.2 explicitly defers it
  to Phase 4; (b) it changes H5 from "shaping externality via shared boundary"
  to a different, weaker hypothesis with no externality channel — a
  research-question change, not an implementation one, and §34.9's standing
  rule (§39C) says contribution-affecting changes get scrutiny, not a silent
  flip; (c) it multiplies the endogenous-kernel bookkeeping (`θ` per firm →
  `Viab(K(θ_i))` per firm) with no Phase-2 question needing it.

- **Global θ_limit / θ_cap but per-firm θ_Q** (since `contract` and the
  `obligation` constraint are the most "bilateral" of the three). Tempting —
  a supply contract really is between two parties. Rejected for the MVP to
  keep θ a single uniform object; the bilateral reading of `contract` is a
  Phase-4 refinement bundled with per-firm θ generally.

- **`θ_Q : f64`, matching `θ_limit` and `θ_cap`** (all three θ components one
  type, for uniformity). This is the uniformity argument: a `θ` vector of three
  `f64`s is simpler to store, serialise, and reason about than a `(f64, f64,
  i64)` mix, and `contract`'s effect (`θ_Q += δ_Q`) could then use a
  fractional `δ_Q` if a later phase wanted one. Rejected in favour of `i64` on
  the exact-arithmetic argument: `g_4 = q − θ_Q` compares `θ_Q` against `q`,
  which is `i64` (§8.1), and the manual's own §15.1 worked example evaluates
  `g_4` to an *exact integer* (`30 − 100 = −70`, and `g_4/s_q = −70/50 =
  −1.400` exactly). Making `θ_Q` an `f64` reintroduces the possibility of a
  rounding error into a constraint that is integer-clean by nature — the same
  hazard ADR 0004 removed from the resource ledger, in the same spirit as
  §21.4's "conserved quantities integer". `contract` in the MVP adds an
  integer `δ_Q` (§11.2, §16.1 leaves `δ_Q` a fixed integer), so no expressive
  power is lost. The `(f64, f64, i64)` mix is a small, contained awkwardness in
  the `ConstraintParams` struct (Stage 0's `firma-domain`); if a future phase
  genuinely needs fractional obligation caps, that is a superseding ADR that
  also revisits whether `q` itself should become fixed-point.

## Consequences

**Positive.**
- The endogenous viability kernel `Viab(K(θ_t))` is a single object per tick
  (§9.4), computable once and shared — which is what makes the `d ≤ 4` exact
  kernel tractable (§9.3) and `kernel_volume` a well-defined single metric.
- H5's externality channel is unambiguous and mechanical, so E8 measures a
  clean effect.
- Regulatory shocks (§13.2) hit one object, matching "Threat and response act
  on a shared surface" (§13.2).

**Negative, accepted, and to be stated in any Phase 2–4 publication.**
- A single global regulatory regime cannot represent regional or
  firm-specific regulation, differential enforcement, or a regulator that
  treats firms unequally. TC-003 (§35.3) already lists "A single global
  regulatory parameter" under *Introduced*; this ADR is the implementation
  commitment behind that contract line.
- Every firm's `compliance`/`scope` margin moves in lockstep when θ moves.
  Cross-firm heterogeneity in constraint pressure comes only from
  heterogeneity in `u` and `c`, not from θ.

**Neutral.**
- **No shipped numerical output changes.** No θ, no constraint plugin, no
  `constrain` phase, no shaping action exists yet. Phase 1 golden trace,
  `phase1-smoke` run id, and all 55 workspace tests are unaffected.

## Compliance

- Part C's `firma-domain::ConstraintParams` is a single struct (not indexed by
  `AgentId`); Stage 1 stores one instance on world state.
- Per-firm θ, if built in Phase 4, requires a superseding ADR and, per §8.2,
  is a Phase 4 deliverable — not a Phase 2/3 one.
- E8's `ExperimentSpec` (Phase 4) records "θ: global" explicitly.
- Manual §26.4 deliverables and §34.0 index gain the relevant rows at the next
  version bump; §8.2 already carries the "global by default / per-firm Phase 4"
  statement, so no manual text conflicts with this ADR.

## Note

This ADR mostly ratifies §8.2. Its real content is the second half: recording
that H5 and E8 are *defined against* global θ, so that when Phase 4 builds
both per-firm θ and the rivalry experiment, nobody has to re-derive that the
two interact. A reviewer asking "why is the regulator one knob for everyone?"
gets pointed here and to TC-003.
