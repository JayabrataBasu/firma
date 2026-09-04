# ADR 0021 — The `Constraint` plugin interface, and where §11's deterministic core lives

**Status:** Accepted (2026-09-03)
**Phase:** 2 (Model), Stage 1
**Relates to:** manual §9.1 (four constraints, violation semantics), §9.2
(`h`, scale factors), §9.3 (deterministic-core approximation for the kernel),
§9.4, §10.1 phases 7–8 (`constrain`, `enforce`), §11.1 (market-action
deterministic core), §11.2 (shaping deterministic core), §11.4 (`Admissibility`
service), §18.2 (`firma-viability` signatures), §20.2 (`Rule` trait), §24.6 A7,
ADR 0011, ADR 0014, ADR 0016, ADR 0020

## Context

The manual specifies the `Rule` trait's seven methods exactly (§20.2). It gives
**no equivalent for `Constraint`**. §18.2 shows `firma-viability` consuming
`cs: &[Constraint]` as an opaque slice; §9.1 gives four *genuinely different*
violation semantics (death; graduated penalty then death; admissibility-gate
only; relational edge-severance with penalty); §11.4 says an `Admissibility`
service consults "hard constraints" during `decide`. Before any of the four
constraint plugins (Part F) or `firma-viability`'s `margin`/`kernel` (Part E)
can be written, the Rust-level `Constraint` interface must be pinned.

Separately (Part D): §9.3 requires the kernel-computing transition to be "the
deterministic core of §11 with stochastic terms at expectation". The §11
*action plugins* are Stage 2, but the deterministic-core formulas
(`y_O(c)`, `y_R(c)`, the six market transitions, the shaping transitions minus
lag/probability) are needed **now** for `firma-viability`'s FIRMA `Dynamics`,
and **again** in Stage 2 for the action `Rule`s. They must not be written
twice.

## Decision

### 1. `Constraint` is its own trait, not a `Rule`

A `Rule` proposes `Vec<Delta>` from `(view, rng_key)` every tick (§20.2). A
constraint does not: it evaluates `g_j(x, θ)` and, on violation, triggers a
consequence that is **semantically fixed by §9.1** (`[D]`, a theoretical
assumption) and **acted on by kernel services** (`enforce`, `Admissibility`),
not by a plugin emitting deltas. Three of the four constraints emit nothing
most ticks. Forcing them through `Rule` would mean a no-op `apply` or a
"ViolationDetected" delta that smuggles constraint *policy* (what death means)
out of the kernel and into a plugin.

```rust
// firma-domain::constraint

/// The narrow interface `firma-viability` depends on (§9.2's `h` uses only
/// `g_j` and `s_j`). `firma-viability` never sees violation semantics — that
/// keeps it free of death/graduated/relational knowledge, matching its
/// `← core, domain` purity (ADR 0020).
pub trait MarginTerm: Send + Sync {
    /// `g_j(x, θ)` (§9.1). Real-valued; `g_j ≤ 0` ⇒ constraint satisfied.
    fn g(&self, ctx: &ConstraintContext<'_>) -> f64;
    /// `s_j` — the §9.2 scale factor for this constraint.
    fn scale(&self) -> f64;
}

/// A full constraint plugin (manual §20.3 category `Constraint`).
pub trait Constraint: MarginTerm {
    /// Stable id, `"constraint.<name>"` (§24.2).
    fn id(&self) -> PluginId;
    /// This build's version (§20.5).
    fn version(&self) -> semver::Version;
    /// What happens on violation (§9.1). An `enforce`-phase / `Admissibility`
    /// dispatch tag — never consulted by `firma-viability`.
    fn violation(&self) -> ViolationSemantic;
    /// Plain-language statement of `g_j` (§20.2 non-empty; §25.6
    /// `assumption-nonempty`).
    fn assumption(&self) -> &str;
}

pub struct ConstraintContext<'a> {
    pub state: &'a FirmState,        // §8.1 constraint-carrying (r^L, r^I, c, q)
    pub aux:   &'a FirmAuxState,     // §8.1 auxiliary (compliance needs `u`)
    pub theta: &'a ConstraintParams, // §8.2, global (ADR 0015)
}

pub enum ViolationSemantic {
    /// §9.1 `solvency`: agent removed at end of `enforce` (§10.1 phase 8).
    Death,
    /// §9.1 `compliance`: first violation → penalty + `λ −= legitimacy_loss`;
    /// a second within `window_ticks` → death.
    Graduated { penalty: i64, window_ticks: u64, legitimacy_loss: f64 },
    /// §9.1 `scope`: never lethal. The constraint only gates action
    /// admissibility (§11.4); `enforce` does nothing with it.
    AdmissibilityGate,
    /// §9.1 `obligation`: all `supply` edges severed, `penalty` applied. Never
    /// lethal.
    Relational { penalty: i64 },
}
```

### 2. `scope` reaches `Admissibility` through its own `g_3`, not a separate predicate

Trace (Part C question 2): the `Admissibility` service (§11.4) runs in `decide`
(§12.3 step 5, `admissible(a, x, θ, e)`), *before* `enforce`. §11.1's
precondition for `produce_regulated` is `c ≥ θ_cap` — which is exactly
`g_3 = θ_cap − c ≤ 0`, i.e. `scope` satisfied on the post-action state. So the
Admissibility service, when it checks "the immediate effect does not violate a
hard constraint" (§11.4), calls **`scope.g(post_action_ctx)` directly**;
`scope` publishes no `is_admissible_for(action)` predicate — its `g_3` *is* the
gate.

**Which constraints the Admissibility service consults (Stage 2 contract, tagged
now):** only `scope` (`AdmissibilityGate`). `solvency` "gating" is really the
actions' own affordability preconditions (`r^L ≥ π^I`, `r^L ≥ κ_c`, …, §11.1);
`compliance` is deliberately *not* an admissibility gate (`produce_regulated`
is allowed even when it worsens `u` — that is §11.1's "core operating
tension"); `obligation` is `Relational`, not a bar to acting. So the
Admissibility service's constraint check is exactly: evaluate `scope.g` for
scope-gated actions.

### 3. `firma-viability` depends on `MarginTerm` only

`firma-viability::margin` takes `&[&dyn MarginTerm]` and computes
`h = −max_j (g_j / s_j)` (§9.2). It never names `Constraint`, `ViolationSemantic`,
`enforce`, or `Admissibility`. Violation handling is an `enforce`-phase concern
(Stage 2). This is a compile-time guarantee that the kernel solver crate stays
domain-*state*-aware but domain-*policy*-free.

### 4. The §11 deterministic core lives in `firma-domain::dynamics`

Shared pure functions — `y_o(c, p)`, `y_r(c, p)`, `market_core(action, state,
θ, env, p) -> Option<FirmState>` (the six §11.1 transitions and their
preconditions; `None` when a precondition fails) — in a new
`firma-domain::dynamics` module. **Both** `firma-viability`'s Stage-1 FIRMA
`Dynamics` **and** Stage 2's `action.market.standard` `Rule` call these; the
action plugin layers cost accounting, the drawn lag, and success probability
(§11.2–11.3) on top of the identical core.

Trace against ADR 0020's graph: `firma-domain ← firma-core` only; it depends on
neither `firma-viability` nor any plugin, so `firma-viability ← domain` and
`plugins ← …, domain` calling into `firma-domain::dynamics` introduces **no
cycle**. `firma-domain` gains no new reachability — it stays a `← core` leaf.

§9.3 approximation, recorded verbatim in the `FirmaDynamics` doc comment and
here: *"The transition `f` used for kernel computation is the deterministic
core of §11 with stochastic terms at expectation — an approximation that MUST
be recorded as such."* Two concrete approximations in the Stage-1 `Dynamics`:
(a) `invest_capability`'s lag (`Δ_cap` ticks) is collapsed to an immediate
`c += δ_c` (the lagged effect still arrives, so it does not change *whether* a
state can survive indefinitely); (b) `u` — an auxiliary variable that in the
full model co-evolves with the action window (ADR 0014) — is held at a fixed
`u_context` throughout the backward iteration, since `u` is not a
kernel-carrying dimension (§8.1: "d = 4, not 5"). The exact kernel is therefore
`K(θ, u_context)`, a slice; VT-3 reports its correlation with `h` at stated
`(θ, u_context)`.

### 5. Stage 1 does not touch `firma-registry`

Config-driven constraint resolution (a `constraints: [solvency, …]` list per
§20.4 → `Vec<Box<dyn Constraint>>`) belongs with the running model, which does
not exist until the `decide`/`act`/`enforce` phases do (Stage 2). Stage 1's
constraint plugins are consumed by `firma-viability::margin` and by tests, via
a `firma-cli::standard_constraints()` helper that mirrors
`standard_registry()`. Formal `firma-registry` integration (a
`RegisteredConstraint` table, `firma-registry ← domain`) lands in Stage 2. This
keeps `firma-registry` — and its Phase-1 tests, including the ADR 0013 guard —
untouched this Stage.

## Rejected alternatives

- **Make `Constraint` a `Rule`.** Rejected: (a) 3/4 constraints emit no deltas
  most ticks; (b) the violation *consequence* is a fixed §9.1 assumption acted
  on by the kernel, not a plugin choice — a "ViolationDetected" delta would
  move that policy into a plugin; (c) `Rule::apply` takes an `RngKey`;
  constraints are deterministic functions of `(x, θ)` and have no use for one.

- **One god-trait with `g`, `scale`, `violation`, `enforce_effect`,
  `is_admissible_for`, …** Rejected: it forces `firma-viability` to link (even
  if not call) death/graduated/relational logic, breaking the ADR 0020 purity
  argument; and `enforce_effect` would put state mutation on the constraint
  when §17 A2 says only the reconciler mutates. The `MarginTerm` / `Constraint`
  split keeps the solver's dependency surface minimal.

- **`Constraint` in `firma-core`.** Rejected for the same reason as ADR 0020:
  `Constraint::g` takes `FirmState` — a firm-shaped type — so putting the trait
  in `firma-core` puts firm types in `firma-kernel`'s reachable graph.
  `firma-domain` is the home.

- **A standalone copy of the §11 formulas in `firma-viability` for kernel use
  only.** Rejected explicitly by the Part D instruction: Stage 2's
  `action.market.standard` would then re-implement `y_O(c) = ⌊y_0(1+ηc)⌋`
  independently, the two could drift, and nothing would catch it. Shared
  functions in `firma-domain::dynamics` — a `← core` leaf both consumers can
  reach — is the answer.

- **Promote `u` to a 5th kernel dimension** so `compliance` is exact in the
  kernel. Rejected: §8.1 is explicit that `d = 4, not 5` and §9.3 that the MVP
  kernel "MUST live" at `d ≤ 4`. `u` at a fixed `u_context` is the §9.3-class
  approximation; a co-evolving or worst-case `u` is a Phase-3+ refinement.

- **`K(θ)` = states satisfying *all four* `g_j`, not only the Death/Graduated
  ones.** `§9.3`'s `K^(0) = K(θ)` and `§5.1`'s "Constraint set *K* — subset of
  state space in which the agent may exist ... Violation means death" read, in
  isolation, as "every `g_j ≤ 0` defines `K`". Rejected — the specific,
  authoritative §9.1 wins over the general §5.1:

  1. **§5.1's "subset of state space in which the agent may exist" is about
     literal removal from the state space, and only two of the four
     constraints do that.** `solvency` (`Death`) removes the agent;
     `compliance` (`Graduated`) removes it on the "second within `T_c`". A
     `scope` violation leaves the agent fully in the state space — it just
     makes `produce_regulated` inadmissible (§11.4). An `obligation` violation
     leaves the agent in the state space — it just severs `supply` edges and
     applies `P_q`. Neither removes the agent from "the space in which the
     agent may exist", so neither belongs in the set whose backward iteration
     defines *survival*. §9.1's `[D]` note is explicit that this distinction is
     the point: "making all lethal would collapse the model to a single
     survival constraint and destroy the distinction between kinds of
     pressure." Folding `scope`/`obligation` into `K^(0)` is exactly that
     collapse.

  2. **What changes under this rejected reading — concretely, not as a
     footnote.** If `scope` (`g_3 = θ_cap − c`) entered `K^(0)`, then *every*
     grid point with `c < θ_cap` would be excluded from the kernel **before
     any backward iteration runs**. The Stage-1 VT-3 finding —
     `kernel size = 3549/3549`, `volume = 1.000` at `u_context = 0.00`,
     backward-iteration sizes `[3549, 3549]` — would not hold as reported: the
     initial kernel would already be a proper subset of the grid (only the
     `c ≥ θ_cap` slice), and the reported observation (that the backward
     iteration itself does *no* pruning, because `hold` is a free indefinite-
     survival action) would be about a pre-pruned set, not the grid. The
     reading is therefore load-bearing for what VT-3 actually measured, not a
     labelling nicety.

  3. **This ADR's chosen reading is lethal-only, and that is what the code
     implements.** `ViolationSemantic::bounds_viability_kernel()` returns
     `true` for `Death` and `Graduated` only. `firma-viability::firma_kernel`'s
     caller (`tests`/`firma-cli`, holding the full `Constraint` objects) passes
     exactly the `solvency` + `compliance` terms as `K^(0)`'s predicate;
     `margin` (Part E) still uses **all four** `g_j` per §9.2's `h` formula.
     VT-3's reported finding — kernel = whole grid or empty depending only on
     `u_context`; the divergence present is `h ≤ 0` (scope violated) but
     in-kernel — stands under this reading and is what Part E/F actually
     compute.

## Consequences

**Positive.**
- The four constraint plugins have one clear interface; `firma-viability` has a
  minimal one (`MarginTerm`).
- No formula duplication: `firma-domain::dynamics` is the single source of the
  §11 deterministic core.
- `firma-registry`, `firma-kernel`, and every Phase-1 test are untouched.
- The `ViolationSemantic` enum makes §9.1's four different semantics a
  compile-time exhaustive `match` for the Stage-2 `enforce` phase.

**Negative, accepted.**
- Two traits (`MarginTerm`, `Constraint`) where one might have done. The split
  is load-bearing for the ADR 0020 purity claim, so it is kept.
- The kernel's `u_context` approximation (Decision 4b) means the FIRMA exact
  kernel is a *slice* `K(θ, u_context)`, not a single object. VT-3 reports per
  slice; the divergence it documents (§9.2: "large `h`, outside the kernel") is
  present regardless of `u_context`.
- `P_q` (the `obligation` relational penalty, §9.1) has **no value in §16.1** —
  a manual gap. Stage 1's `obligation` plugin takes it as a required parameter
  (no default, §32.1 risk 2); flagged for a §16.1 PATCH.

**Neutral.**
- **No shipped numerical output changes.** No constraint is wired into a
  running simulation this Stage — the `decide`/`act`/`enforce`/`constrain`
  phases do not carry domain rules yet. The Phase 1 golden trace, `phase1-smoke`
  run id, and all workspace tests are unaffected by anything except the
  addition of new tests.

## Compliance

- `firma-domain::{MarginTerm, Constraint, ConstraintContext, ViolationSemantic}`
  and `firma-domain::dynamics::{y_o, y_r, market_core, MarketAction, ...}`.
- `firma-viability::margin` signature takes `&[&dyn MarginTerm]`; a compile
  check that `firma-viability` does not import `Constraint` / `ViolationSemantic`.
- **`firma-viability`'s dependency set:** Phase 1 shipped it with an *empty*
  `[dependencies]` block (a pure-`std` generic solver over the abstract
  `Dynamics` trait + `Grid`). ADR 0011's Compliance line — "`firma-viability`
  `Cargo.toml` depends only on `firma-core`" — overstated that (the reality was
  "depends on nothing", which is a subset of `{firma-core}`), and its "checked
  by `scripts/check_deps.py` extended coverage" was never implemented
  (`check_deps.py` only guards `firma-kernel` and cross-plugin edges). Stage 1
  adds `firma-domain = { workspace = true }` as the crate's **first-ever**
  dependency; `[dependencies]` is now exactly `{firma-domain}`. `firma-viability`
  references **nothing** from `firma-core` directly —
  `grep -rn 'firma_core::' crates/firma-viability/src` returns no matches — so
  no direct `firma-core` edge is needed; `firma-core` appears in the resolved
  tree only transitively, for `firma-domain`'s own use. This is correct and
  intentional: the §18.1 `←` is an *allowed* upper bound, and the actual set
  being a strict subset (`{}` in Phase 1, `{firma-domain}` in Stage 1) is fine.
- `crates/firma-plugins/firma-plugin-constraint` — `solvency`, `compliance`,
  `scope`, `obligation`, each `impl Constraint`, each with a non-empty
  `assumption()` (a Stage-1 test asserts this, mirroring the Phase-1
  `assumption-nonempty` integration test).
- §15.1 (Example A) is a direct `margin` fixture: `h = 0.180`, binding
  `compliance` (Part E test).
- `scripts/lint-architecture.sh` `SIM_PATH_SRC` gains `crates/firma-domain/src`
  and `crates/firma-plugins/firma-plugin-constraint/src` in Stage 1 (they now
  carry result-affecting arithmetic).
- Manual PATCH items: §34.0 index row for ADR 0021; §16.1 gains `P_q`; §18.2
  gains a `Constraint` trait sketch or a pointer to this ADR.

## Note

The manual gave `Rule` seven methods and `Constraint` a slice type. This ADR
fills the gap the way §20.2 would have if it had reached `Constraint`: a small
trait, a fixed enum for the §9.1 semantics, and a deliberate split so the
kernel solver never has to know what "death" means. The Part D decision —
formulas in `firma-domain::dynamics`, not copied — is the one that prevents a
silent-drift bug six months from now when Stage 2 writes the action plugins.
```
