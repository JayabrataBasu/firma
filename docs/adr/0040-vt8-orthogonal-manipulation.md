# ADR 0040 — VT-8: the `h` / `ς` intervention seam, and the SC-1…6 finding

**Status:** Accepted (2026-09-06). The SC-4/SC-5 row is **confirmed and
sharpened** by [ADR-0042](0042-sc4-shaping-unreachable-across-wmax-beta.md),
which sweeps the full §16.1 `β × w_max` grid (untouched by this ADR's ~24
configs): shaping stays unreachable at every `(w_max, β)`, because it is
width-*independent* — a shaping action never satisfices, so it is only ever
the scan fallback, which ignores `w_eff`. The conclusion is unchanged.
**Phase:** 2 (Model), Stage 6
**Relates to:** manual §25.4 VT-8 (independence of `h` and `ς`), §2.5
("without VT-8, H1a and H1b are guaranteed by construction and E1 is void"),
§30.4 Arm A ("`h` and `ς` set directly by intervention … independently"),
§12.1 / §35.4 TC-004 (aspiration adaptation and its "independence from
viability"), §16.2 (SC-1…6), §17 A3 (structural not disciplinary), §17 A5
(measurement is offline); ADR-0026 (`firma_domain::margin`), ADR-0027
(`decision.random`)

## Context

VT-8 has three criteria: (i) empirical `r(h, ς) ≈ 0` across the Arm A grid
under intervention, (ii) all four quadrants reachable and populated, (iii) an
analytic trace showing **no path in the decision procedure computing one from
the other**. The manual does not specify the intervention *mechanism* for
Arm A ("`h` and `ς` set directly by intervention at each measurement tick").

## Decision

### 1. §12.3 Steps 2–5 become one pure function; `apply` keeps no decision logic

`firma_plugin_decision::select`:

```rust
#[allow(clippy::too_many_arguments)] // every input named on purpose (crit. iii)
pub fn select(
    h: f64,
    shortfalls: [f64; 3],
    beta: f64,
    h_crit: f64,
    w_max: u32,
    prev_action: u8,
    admissible: impl Fn(u8) -> bool,
    satisfices: impl Fn(u8, Focus) -> bool,
) -> Selection // { focus, action, w_eff }
```

Its body: `Focus::attend(h, shortfalls, h_crit)` (Step 2 — branches on `h`,
then on `shortfalls`, computes neither from the other); early-return on `NONE`;
`w_eff(psi(h, h_crit, beta), w_max)` (Step 3 — reads **only** `h`); the
focus-ordered scan calling the two closures (Steps 4–5). **`h` and
`shortfalls` are independent typed parameters with no expression relating
them.** Criterion (iii)'s structural half is now a fact you check by reading
one ~25-line function, not by code archaeology (§17 A3 applied to a validation
test).

**Drift prevention.** `Satisficing::apply`'s per-agent block was rewritten to
carry **no Step-2–5 logic of its own**: it computes the real `h_t`
(`dc.margin_at` → `standard_margin`), the real `sc` (`A_j − v_j`), the two
closures over `DecideCtx`, and calls `select`. The Steps 2–5 code exists in
**exactly one place**, so the normal path and the VT-8 harness cannot diverge.
Verified byte-identical: golden, `phase1-smoke`, `phase2-smoke`,
`phase2-stage5-smoke` `run_id` / `event_log_sha256` all unchanged.

The VT-8 harness (`tests/tests/validation.rs::vt8_…`) calls `select`
**directly** with grid-constructed `(h, ς)` pairs and stub
`admissible` / `satisfices` closures — the grid never touches firm state.

### 2. Criterion (iii) — the two directional greps

`ς_j = A_j − v_j` (§12.1); `A_j` evolves **only** via
`decision.aspiration_update`'s `A ← A + α(v − A)`.

**(a) The margin / `h` path never reads an aspiration or a shortfall.** `grep`
of `firma-domain/src/margin.rs`, `constraint.rs`, and `firma-viability/src`
for `aspiration` / `Aspirations` / `shortfall` / `A_j` / `realized_capital`
returns only: `#[cfg(test)]` fixtures, and `firma-viability/src/domain.rs`'s
*import line* plus its comment "`legitimacy` and `aspirations` do not enter any
`g_j` (§9.1)" with the field zeroed. `g_solvency` = `−r^L`; `g_compliance` =
`u − θ_limit`; `g_scope` = `θ_cap − c`; `g_obligation` = `q − θ_Q`;
`standard_margin` = `−max_j(g_j/s_j)`. **None read `ctx.aux.aspirations`.**
The `FirmAuxState.aspirations` field exists (component bag) and is zeroed by
every caller.

**(b) `decision.aspiration_update` never reads `θ`, a `g_j`, `h`, `h_crit`,
`focus`, `w_eff`, `λ`, or `u`.** `grep` of its `impl` for `THETA` / `theta` /
`margin` / `standard_margin` / `all_g` / `g_solvency` / … / `h_crit` / `FOCUS`
/ `W_EFF` / `LEGITIMACY` / `REGULATED_INTENSITY` / `ConstraintContext` /
`viability` returns **empty**. It reads only `r^L`, `c`, `q`,
`prev_tick_capital`, and the three `aspiration_*` keys; it writes only
`aspiration_*`, `realized_capital_growth`, `prev_tick_capital`.

**Two one-directional greps, both empty. That is the evidence for criterion
(iii).**

### 3. "Independent" ≠ "uncorrelated" — the nuance

"No path computing one from the other" does **not** mean `h` and `ς` share no
underlying state. They do: `r^L` feeds `g_1` (→ `h`) *and* `v_1` (→ `ς_1`);
`c` feeds `g_3` *and* `v_2`; `q` feeds `g_4` *and* `v_3`. That common
dependence is expected and fine — it is why, in Arm B (endogenous), `h` and
`ς` covary. What VT-8 requires false is `ς` being computed **as a function
of** `h`, or vice versa, anywhere in the code. It is not: `aspiration_update`
computes each `v_j` from raw state (not from a `g_j` or `h`), and `select`
takes `h` and `ς` as separate parameters. **Independent (no functional path),
not uncorrelated (which they need not be).**

### 4. VT-8 result

`vt8_orthogonal_manipulation_of_h_and_shortfall` — a 7 × 7 grid
(`h ∈ {−0.05, 0.02, 0.05, 0.10, 0.15, 0.20, 0.40}`,
`ς_1 ∈ {−1.0, −0.25, 0.0, 0.25, 0.50, 1.0, 2.0}`, `ς_2 = ς_3 = 0`), fed to
`select` (`β = 1`, `h_crit = 0.15`, `w_max = 6`, market-only admissibility,
nothing-satisfices stub):

- **(i)** `r(h, ς) = +0.000000` — a full factorial of two independent factors
  has *exactly* zero sample correlation.
- **(ii)** the `(h < h_crit) × (max_j ς_j > 0)` partition: **9 / 12 / 12 / 16**
  cells — all four quadrants non-empty.
- **(iii)** the Step-3 narrowing width `ψ(h) → w_eff` is a **single value per
  `h`-level across all ς** (asserted); and both greps above.
- Descriptive: `r(h, Selection.w_eff) = +0.356` (h → narrowing) and
  `r(ς, Selection.w_eff) = +0.379` (ς → focus → whether a scan happens) — both
  non-zero, neither mechanical, which is what makes H1a/H1b *real* claims and
  not vacuous.

**VT-8's three criteria all hold.**

## SC-1…SC-6 finding — NOT jointly satisfiable

The other half of the Phase-2 gate (§16.2). Metrics are reconstructed offline
from the event log (`firma_conformance::replay`, §17 A5, VT-6 pattern):
per-firm `(r^L, r^I, c, q, λ, u, θ)` folded from `DeltaApplied`, `h` / `g_j`
recomputed with `firma_domain::margin`, `focus` / `selected_action` /
`action_window` read from the log.

**~24 configs** were run (`T = 400`; `θ_limit ∈ [0.40, 0.90]`,
`θ_cap ∈ [0.20, 0.90]`, `θ_Q ∈ [6, 100]`, `L_W ∈ {4, 6, 8}`,
`n ∈ {2, 4, 10, 12, 20}`, with and without shocks). **No setting satisfies
all six.**

| SC | outcome | why |
|---|---|---|
| SC-1 survival 0.60–0.90 | **reachable** (e.g. 0.80, θ_limit 0.40 / L_W 4 + knife-edge seeds) | but only via a compliance-death or knife-edge-solvency regime; the default no-shock outcome is **100 % survival** — the satisficing firm's own SURVIVAL attention + inertia are highly self-preserving (§3.1) |
| SC-2 firm-ticks `h < h_crit` 5–25 % | **not reachable** — **bimodal**: `0.005` (all-healthy) or `0.50–1.00` (any regime where a constraint binds); never in between | `h = −max_j(g_j/s_j)`, so `h < h_crit` for a *wide band* around every boundary. A firm engaging the regulated-production tradeoff sits permanently within `h_crit` of `compliance`; a firm below `θ_cap` is permanently in the `scope` band (`s_c = 0.5` ⇒ `g_3/s_c = 2(θ_cap−c)`). **SC-2 and SC-3 are mutually exclusive** given this margin structure. |
| SC-3 all four bind | **partial** — `compliance`, `scope`, `obligation` emerge; **`solvency` only from a degenerate knife-edge seed** (`r^L = π^I`, `r^I = 0`) | any firm that can `produce_ordinary` earns positive cash (`π^O·y_O − 0 > 0`), so emergent bankruptcy does not occur under no shock |
| SC-4 shaping > 5 % of actions | **not reachable under `decision.satisficing`** — **0 / ~50 000 decisions** | §12.3's scan-order table places shaping (6,7,8) after market actions that are almost always admissible, and shaping **never satisfices** (its one-step lookahead, `shaping_cost_step`, shows only cost — §12.3 "the firm does not simulate"). It is only ever the scan *fallback*, and the fallback reaches an admissible earlier market action first. §12.3 itself says "with small `w_eff` they are never reached — R3 emerges". **`decision.random` (Arm C) selects shaping 30.5 %** of the time — so the failure is specific to the satisficing scan order, not the action set. |
| SC-5 shaping success 0.10–0.60 | **not reachable** — no shaping ⇒ no rate (`decision.random`: 0.492, in range) | consequence of SC-4 |
| SC-6 entropy-variance non-degenerate | **usually reachable** (0.01–0.13); degenerates to 0 when firms are very repetitive | — |

**Per §16.2 this is a legitimate Phase-2 failure condition — reported as a
finding, not tuned around.** The Stage-6 report states which subset each
regime achieves. `tests/tests/sanity.rs::sc16_gate` locks the finding as a
regression.

### Model defect vs. spec issue (§2.5 requires distinguishing them)

- **SC-4 / SC-5** is a **spec/model tension, not a bug.** The satisficing
  procedure's near-total shaping abandonment *is* §12.3's stated behaviour
  (R3 "emerges"). But SC-4 requires shaping to be exercised. Only SC-1 says
  "under no shock"; SC-4 does not — so SC-4 may be intended to be checked in
  **Arm B** (§30.4, the shock-driven regime, where H3 predicts firms with
  `time_to_boundary > shaping_lag` shape more), or against a config that
  includes `decision.random` firms. **A Phase-3 decision is needed** on
  whether SC-4/5 are baseline conditions or Arm-B conditions.
- **SC-2** is a **model property worth recording, not a bug.** §12.3 Step 2 is
  a hard branch; the firm is either clear of all boundaries (attention
  elsewhere) or within `h_crit` of one (SURVIVAL). The 5–25 % band assumes a
  population that *hovers*; this model's firms do not.
- **SC-3 solvency / SC-1** point the same way: **the no-shock model has no
  emergent mortality.** That is arguably correct (inertia principle) and
  means SC-1's 60–90 % band, like SC-4, likely belongs with a baseline shock.

None of these is an implementation defect. All are **specification questions
about SC-1…6 and the E1 arm structure that must be resolved before Phase 3**,
which is exactly the check §16.2 / §27.3-criterion-8 exists to force.

## Alternatives

- **A `do(set_h)` / `do(set_ς)` intervention operating on the running model
  at a measurement tick.** `h` is not stored (§17 A5, ADR-0028) — there is no
  cell to `set`. `ς` is `A_j − v_j` and `A_j` *is* stored, so `do(set_A)`
  could set `ς`; but `h` would still need the pure-function route, so this is
  strictly more machinery for the same result, and it entangles the
  intervention with real firm state (the drift risk the pure function
  avoids). Rejected.
- **Keep Steps 2–5 inline in `apply` and prove (iii) by reading `apply`.**
  `apply` also computes `h`, `ς`, admissibility, and the lookahead, so the
  independence claim is buried in 80 lines that touch real state. The pure
  function makes it a one-signature fact. Rejected.
- **A `SelectParams { beta, h_crit, w_max }` struct to cut the argument
  count.** Bundling *parameters* is fine, but bundling `h` and `shortfalls`
  (the two the ADR is about) would hide the seam; and a params struct plus
  `h` / `shortfalls` / `prev` / 2 closures is still 6 args. `#[allow]` with a
  comment is the honest choice for a function whose point is naming every
  input.

## Consequences

- **Positive.** VT-8 criterion (iii) is structural. The pure `select` is
  independently testable (VT-4/VT-5 could migrate onto it) and is the natural
  seam for a future `decision.optimizing` (E7). No numerical output changed.
- **Negative, accepted.** `Rule`-adjacent surface grows by `select` /
  `Selection` / `Focus::attend` (all `pub` in `firma-plugin-decision`, not on
  a trait). The SC finding blocks the Phase-2 gate pending a spec decision.
- **Neutral.** The offline `firma_conformance::replay` module is new
  (~430 lines, tests-only crate).

## Compliance

- `firma_plugin_decision::{select, Selection, Focus::attend}`;
  `Satisficing::apply` carries no Step-2–5 logic.
- `tests/tests/validation.rs::vt8_orthogonal_manipulation_of_h_and_shortfall`
  — (i) `r(h,ς) = 0`, (ii) 4 quadrants populated, (iii) ς-independent width.
- `tests/src/replay.rs` — offline SC-1…6 reconstruction.
- `tests/tests/sanity.rs::sc16_gate` — locks "not jointly satisfiable" +
  which subset each regime reaches; `sc16_search` (ignored) reproduces the
  sweep.
- Golden / `phase1-smoke` / `phase2-smoke` / `phase2-stage5-smoke`
  byte-identical.

## Note

The pure-function seam is the same move §17 A3 makes for determinism —
"structural, not disciplinary" — applied to a validation guarantee. And the SC
finding is the point of §16.2: it asks whether the model is *scientifically
alive*, and the honest answer at the end of Stage 6 is "the mechanisms all
work, but the no-shock baseline does not produce the regime SC-1…6 describes —
which is itself a result, and a Phase-3 design question, not a tuning task."
