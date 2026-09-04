# ADR 0026 — The four §9.1 `g_j` formulas and the standard-four margin move to `firma-domain::margin`

**Status:** Accepted (2026-09-04)
**Phase:** 2 (Model), Stage 3
**Relates to:** manual §9.1 (the four `g_j`), §9.2 (`h = −max_j g_j/s_j`), §12.3
Step 1 ("compute `h`"), §17 A5 (measurement is offline — but `h` is a decision
*input*, not offline measurement); ADR-0020 (`firma-domain` holds shared domain
vocabulary + deterministic cores), ADR-0021 (§11's `market_core` lives in
`firma-domain::dynamics` so it is not written twice)

## Context

`decision.satisficing` (§12.3) needs `h` in Step 1. Today the four `g_j`
formulas live *only* in `firma-plugin-constraint` (`Solvency::g` returns
`-(r^L as f64)`, etc.), and `h` is computed by
`firma-viability::margin(&[&dyn MarginTerm])` — a *generic* fold over an
arbitrary constraint set, which the kernel solver needs but which requires
constructing the four `MarginTerm` trait objects.

`decision.satisficing` **cannot depend on `firma-plugin-constraint`** — plugins
must not depend on each other (§18.1). Its options were:

1. Re-implement the four one-line `g_j` inside `decision.satisficing`. This is
   exactly the "don't silently duplicate the formulas in two places"
   prohibition ADR-0021 applied to §11's `market_core` — the same discipline
   applies to §9.1.
2. Depend on `firma-viability` and hand-build the four `MarginTerm`s. But the
   concrete `MarginTerm` impls are in `firma-plugin-constraint` (cross-plugin
   dep), and `firma-viability::margin` is generic-by-design (ADR-0021
   Decision 3: "never names `Constraint`") — putting the concrete `g_j` there
   would encode §9.1 policy into the kernel solver crate.
3. Move the four `g_j` + a concrete `standard_margin` into `firma-domain`,
   beside `firma-domain::dynamics` (which already holds §11's core for exactly
   this reason). Both `firma-plugin-constraint` and `decision.satisficing`
   call it; `firma-viability::margin` stays the generic interface.

## Decision

**Option 3.** New module `firma-domain::margin`:

```rust
pub fn g_solvency(ctx: &ConstraintContext<'_>)   -> f64 { -(ctx.state.liquid_capital as f64) }
pub fn g_compliance(ctx: &ConstraintContext<'_>) -> f64 { ctx.aux.regulated_intensity - ctx.theta.theta_limit }
pub fn g_scope(ctx: &ConstraintContext<'_>)      -> f64 { ctx.theta.theta_cap - ctx.state.capability }
pub fn g_obligation(ctx: &ConstraintContext<'_>) -> f64 { ctx.state.obligation as f64 - ctx.theta.theta_q as f64 }

/// h = −max_j (g_j / s_j)  (§9.2), for the standard four constraints.
pub fn standard_margin(ctx: &ConstraintContext<'_>, scales: &ScaleFactors) -> f64;
```

- **`firma-plugin-constraint`**: the four `MarginTerm::g` impls become one-line
  delegates (`Solvency::g` → `firma_domain::margin::g_solvency(ctx)`, …). Each
  plugin's own `scale()` and `ViolationSemantic` are unchanged. §15.1's
  worked example (`h = 0.180`, binding `compliance`) is arithmetically
  identical — the formulas moved, they did not change.
- **`firma-viability::margin(&[&dyn MarginTerm])`** is **unchanged**. The
  kernel solver still folds an arbitrary term slice; VT-1/VT-2/VT-3 and the
  §15.1 fixture (which go through `firma-cli::standard_constraints()` → the
  plugin `MarginTerm`s → `firma-viability::margin`) are untouched. The two
  paths now agree by construction because the plugin `g` bodies *are*
  `firma-domain::margin::g_*`.
- **`decision.satisficing`** calls `firma_domain::margin::standard_margin(&ctx,
  &scales)` directly — no cross-plugin dependency, no formula duplication.

`ScaleFactors` (`s_L=100`, `s_u=1.0`, `s_c=0.5`, `s_q=50` — §9.2 defaults)
already exists in `firma-domain::params`.

## Alternatives

- **Duplicate the four one-liners in `decision.satisficing`.** They are
  trivial, but "trivial" is how duplication starts; ADR-0021 set the
  precedent that §-numbered model formulas have exactly one home.
- **Add `standard_margin` to `firma-viability`.** `firma-viability` is the
  kernel-solver crate and ADR-0021 Decision 3 deliberately keeps it
  policy-free (`MarginTerm` only). §9.1's "solvency *is* `−r^L`" is model
  policy; it belongs with `market_core` in `firma-domain`.
- **Expose `h` on `View`.** §17 A5 forbids a kernel metric API. `h` here is a
  decision *input* computed by the plugin, which is fine, but it must be the
  *plugin* computing it, not the kernel serving it.

## Consequences

- **Positive.** One home for §9.1. `firma-plugin-constraint` and
  `decision.satisficing` provably agree. `firma-viability` stays generic and
  policy-free.
- **Negative, accepted.** `firma-plugin-constraint` — accepted in Stage 1 —
  gets an 8-line change (four `g` bodies → delegates). Behaviourally inert:
  §15.1 / VT-1 / VT-2 / VT-3 outputs are byte-identical (verified).
- **Neutral.** `firma-domain` gains a module; no new dependency (it already
  has `ConstraintContext`, `ScaleFactors`).

## Compliance

- `firma-domain::margin::{g_solvency, g_compliance, g_scope, g_obligation,
  standard_margin}`.
- `firma-plugin-constraint`'s four `MarginTerm::g` impls are delegates
  (grep: each `fn g` body is a single `firma_domain::margin::` call).
- Tests: `firma-domain::margin::tests` (each `g_j` matches §15.1's row;
  `standard_margin` = `0.180` on the §15.1 fixture); the Stage-1
  `validation::margin_matches_section_15_1` still passes unmodified.
