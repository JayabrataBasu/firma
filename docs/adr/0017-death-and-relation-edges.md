# ADR 0017 — At death, all incident relation edges vanish; cascade realism is a Phase 3 AT question

**Status:** Accepted (2026-09-03)
**Phase:** 2 (Model), Stage 0
**Resolves:** manual §16.3 item 4 ("Death and relations. A dying firm's
`supply` edges vanish, which can cascade. Whether cascades are finding or
artefact needs AT-style testing.")
**Relates to:** §8.3 (relation graph `G_t`), §9.1 (`solvency`, `compliance`
death; `obligation` edge severance), §10.1 phase 8 (`enforce`), §13.1
(dependence), §25.5 (AT-3, AT-5), §16.4 ("Entry … none are founded in the
MVP"), §7.2 primitive 6 (Relation)

## Context

The MVP relation graph `G_t` (§8.3) carries directed typed edges
`(i, j, ℓ, w, age)` with `ℓ ∈ {supply, alliance, rivalry}`. Firms die — from
`solvency` (`g_1 = −r^L`, "Agent removed at end of `enforce`") or from a second
`compliance` violation within `T_c` ticks (§9.1). §16.3 item 4 asks what
happens to a dead firm's edges, and whether the *chained* deaths that can
follow (a firm that loses its only supplier and then fails its own
constraints) are a real phenomenon the model reveals, or an artefact of how
edge removal is coded.

The manual is explicit that the second question is **not settled here**: "needs
AT-style testing" — i.e. AT-3 (change the conflict resolver → is the
conclusion robust, or is the dependence on it reported?) and AT-5 (perturb
initial conditions within a small ball → are conclusions stable?), both §25.5,
both Phase 3.

Note the `obligation` constraint (§9.1, `g_4 = q − θ_Q`) *already* severs
`supply` edges on violation without death — so edge removal on constraint
events is a pre-existing part of the model, and death is one more trigger.

## Decision

**Stage 0 settles only the mechanism, not the empirical interpretation.**

**Mechanism (binding for Stage 1+):**

1. When an agent dies, it is removed from the population in the `enforce` phase
   (§10.1 phase 8), and in the **same atomic step** every edge of `G_t`
   incident to it — inbound and outbound, all three edge types — is deleted.
2. There is **no cascade-damping**: no grace period, no "orphan supplier"
   substitution, no edge inheritance by a third party, no delayed removal.
   A firm that this leaves with zero `supply` edges simply has zero `supply`
   edges from the next tick, its dependence `D_i` is recomputed accordingly
   (§13.1: "If `Σ_j w^k_{ij} = 0` over the window, `D_i^k := 1` and the case is
   logged"), and whatever its constraints then imply, happens.
3. A chained death that follows is **not prevented and not specially flagged**
   by the kernel. It is emitted to the event log like any other death, with
   its own cause (`solvency` / `compliance`), and offline analysis
   reconstructs the chain if it wants to.
4. Consistent with §16.4, no firm is founded to replace a dead one (no entry
   in the MVP), so the population is monotone non-increasing within a run.

**Explicitly NOT settled by this ADR:** whether observed death cascades are a
genuine finding about dependence concentration or an encoding artefact of the
"edges vanish instantly, no damping" choice above. That is an **open empirical
question for Phase 3's AT suite** — specifically AT-3 and AT-5 (§25.5), plus
any cascade-specific robustness check the E-series experiments add. A future
ADR may introduce a damping mechanism *if and only if* the AT suite shows the
current mechanism drives a conclusion that does not survive perturbation; per
§25.5, "An AT failure is not necessarily a defect — it may be a genuine
sensitivity finding. But it MUST be reported, never suppressed."

## Rejected alternatives

- **A grace period** (a dead firm's `supply` edges persist for `g` ticks so
  partners can find a new source). Rejected for Stage 0: `g` is an unspecified
  parameter with direct weight on SQ3 ("How does concentration of resource
  dependence moderate threat response?"), and adding it now pre-judges the
  cascade question §16.3 says the AT suite must answer.

- **Edge inheritance** (a dead supplier's downstream edges rewire to its own
  upstream sources). Rejected: introduces network-rewiring dynamics with no
  §2 question behind them, and confounds any dependence measurement.

- **Prevent chained death** (a firm that would die *only* because a partner
  just died gets a one-tick reprieve). Rejected outright: this is
  repair-and-continue, which §19.5 forbids, and it would make the cascade
  question unanswerable by construction.

## Consequences

**Positive.**
- The mechanism is the simplest possible (A7) and matches the manual's plain
  reading ("edges vanish").
- Death is atomic — population and graph are always mutually consistent at a
  tick boundary, so a snapshot is never mid-cascade.
- The cascade question stays genuinely open for Phase 3 to answer with data,
  rather than being engineered away now.

**Negative, accepted.**
- Instant, total edge removal is the *most* cascade-prone choice. If Phase 3
  finds cascades dominate outcomes and do not survive AT-3/AT-5, this ADR's
  mechanism is the suspect and a superseding ADR adds damping. That is an
  anticipated, acceptable path — not a failure.
- `diversify` (§11.2) is the firm's in-model defence against supplier loss;
  whether it is fast enough (lag `Δ_d` vs. time-to-boundary) is exactly what
  H3 tests, so the harshness of instant edge removal is partly the point.

**Neutral.**
- **No shipped numerical output changes.** There is no relation graph in the
  kernel yet, no `enforce`-phase death logic beyond Phase 1's plain
  `remove_agent` (which has no edges to remove), and no dependence metric.
  Phase 1 golden trace, `phase1-smoke` run id, and all 55 workspace tests are
  unaffected. This ADR governs Stage 1+ kernel and Phase 3 analysis.

## Compliance

- Stage 1 extends the kernel's agent-removal path (currently
  `World::remove_agent`, `firma-kernel`) to also delete all incident edges,
  in the `enforce` phase, atomically. A test: a 2-firm `A → B` supply chain
  where `A` dies leaves `B` with zero `supply` edges and `D_B` recomputed to
  the logged degenerate case (§13.1).
- Death events (with cause) are logged; a chained death is a normal death
  event, not a special record.
- Phase 3's AT-3 and AT-5 runs report cascade sensitivity explicitly (§25.5);
  the E-series analysis plan notes death-cascade prevalence as a quantity to
  report, per §22.2 log-sufficiency (log every death and every edge removal).
- Manual §34.0 index gains a row for ADR 0017 at the next version bump.

## Note

The one thing to get right here is *what this ADR does not do*. It fixes a
mechanism so Stage 1 can build the `enforce` phase without guessing. It does
**not** claim that instant total edge removal is realistic, and it does not
claim the resulting cascades are meaningful. §16.3 was explicit that the
realism question is for "AT-style testing"; this ADR keeps that question open
and points at AT-3/AT-5 as where it gets answered.
