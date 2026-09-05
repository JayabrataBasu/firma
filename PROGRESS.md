# FIRMA — build progress

Tracks what is built, what is deliberately deferred, and open questions for
review. The manual (`docs/MANUAL.md` v1.0.0) is the source of truth; this file
is a running log, not a spec.

---

## Phase status

| Phase | Manual | State |
|---|---|---|
| 0 — Foundations | §26.2 | Complete (pre-existing). This build adds repo + CI + lint infra, the remaining §26.2 item. |
| 1 — Kernel | §26.3 | **Gate met.** + ADR-0013 hardening + final Threads A/B pass. |
| **2 — Model** | §26.4 | **Stage 7 done. This was Phase 2's last Stage. Since then: an owner-directed H3 model revision (ADR-0047, round 2 ADR-0048, round 3 ADR-0049) is implemented on branch `h3-satisficing-lookahead`, NOT merged — see the dedicated sections below.** Gate criterion 8 MET *per-hypothesis* for H1a/H1b/H1c/H2/H4 (ADR-0044); H3 is untestable as built (a named, evidenced, non-kill-criterion finding, not fixed). Stage 7 (ADR-0045, ADR-0046) adds the MVP interface layer: `firma-tui` (ratatui/crossterm live monitor, structurally kernel-free) and `firma_lab` (Python, PyO3 `load`+`metrics`, six Phase-3 modules stubbed) — neither changes the gate status; both are debugging/monitoring aids ("neither produces evidence", §23.2). `Event` relocated `firma-kernel` → `firma-core`; new `firma-analysis` crate promotes Stage-6's `replay.rs` reconstruction for `firma-tui`/`firma-py`/tests to share. Stages 0–5 as below. Stage 6 (ADR-0040): **VT-8 passes** — §12.3 Steps 2–5 extracted to a pure `select(h, ς, …)` function, `r(h, ς) = 0` on the grid, all 4 quadrants populated, both directional greps empty. Stage 6 also found **SC-1…SC-6 cannot be jointly satisfied *under `decision.satisficing`* in a no-shock baseline** — SC-4/SC-5 fail structurally (never selects shaping — R3 by design, §12.3; ADR-0042 confirms width-*independence* across the full §16.1 `β × w_max` sweep), SC-2 is bimodal, SC-3 solvency only binds from a knife-edge seed. This `decision.satisficing`-specific finding **still stands unchanged**. Stage 6b (ADR-0043) asked whether §16.2/§27.3 criterion 8 is scoped to `decision.satisficing` at all, found `decision.random` jointly satisfies all six robustly across 5 seeds, and read the whole gate as met on that basis — **that unqualified framing was challenged and is superseded by ADR-0044** (Stage 6c): Arm C is 10/315 E1 cells; Arms A/B (96.8%) run `decision.satisficing` exclusively. ADR-0044 checked each hypothesis's actual DV dependency and found **H1a/H1b/H1c/H2/H4 need only SC-1/SC-2/SC-3/SC-6 — never SC-4/SC-5 — and are gate-clear**; **H3 (the shaping-lag-vs-narrowing hypothesis, "the distinctive mechanism" per §28.2) needs SC-4/SC-5 and is untestable**: Stage 6b's own Arm-B-shaped, shocked `decision.satisficing` run already showed `SC-4 = 0.0000` at the exact shaping lag H3 parameterises over. This is not a kill-criterion trigger (§2.5 names only H1a/H1b/H1c, all three gate-clear) but is a real, open problem for E3 and for §30.3/§30.9 as currently drafted, flagged for a §2.4/§30 PATCH — not fixed this Stage, per the project's rule that a contribution-shaping model change needs an adversarial literature check first. Also ADR-0041 (a Stage-6-discovered `resolve_lagged` multi-firm bug, fixed), ADR-0042 (the SC-4/SC-5 `β × w_max` re-check), and ADR-0043 Decision 2 (a `constraint.enforce` simultaneous compliance+obligation penalty merge, found and fixed while building Stage 6b's config — untouched by ADR-0044). |

### Phase 2 build stages (owner-gated; instructed one at a time)

| Stage | Content | State |
|---|---|---|
| 0 | Prerequisite ADRs (§16.3 ×5 + Locality/Resource scope + `firma-domain` crate) and minimal §8.1/§8.2 state/param types | Done |
| **1** | `firma-viability` domain surface (`margin`, `FirmaDynamics`, `firma_kernel`); the four `Constraint` plugins; §11 deterministic core; VT-1, VT-3 | Done — reviewed |
| **2** | The decide→act hand-off (ADR 0022, 8 `DeltaKind` variants incl. `SetAgentInt`); the `Λ` lagged-effect queue (ADR 0023); `action.market.standard` (execute-only); `action.shaping.rdt_standard` + `resolve_lagged`; the minimal §8.3 relation graph; VT-7; `testkit.select_action`; 3 review fixes (ADRs 0024/0025) | Done — reviewed |
| **3** | ADR immutability rule (Part A: revert in-place edits, ADR-0024/0025 supersede); `decision.satisficing` (§12.3), `decision.random` (ADR-0027), `decision.aspiration_update` (§12.1); `firma-domain::margin` (ADR-0026); `Attention` interim extended (`focus`/`w_eff` keys); VT-4, VT-5 | Done — reviewed |
| **4** | `constrain` + `enforce` phases (ADR 0028/0029); config seeds domain state (ADR 0030); reconciler fixes ADR 0031 + ADR 0033 (`Environment`-only `ResourcePool` uniqueness carve-out) + ADR 0032 (`RemoveAgent` applied last); `firma_cli::model_registry` + `--model`; `phase2-smoke` end-to-end run | **Done — awaiting sign-off (ADR-0033 follow-up included)** |
| **5** | `observation.{full,noisy,delayed}` (ADR 0035); `shock.{scheduled,stochastic}` (ADR 0036); `locality.{wellmixed,network}` (ADR 0037); `resource.{constant,patchy}` (ADR 0038); `Rule::rng_stream` + `KeyedRng::next_normal` (ADR 0034); `phase2-stage5-smoke` run. Follow-up: ADR 0039 (registration-metadata alternative to `rng_stream` weighed + rejected — no code change) + a test proving `observation.noisy` flips a decision | **Done — awaiting sign-off (0039 follow-up included)** |
| **6** | Phase-2 gate: **VT-8** (ADR 0040 — pure `select` seam, `r(h,ς)=0`, the two greps) + **SC-1…SC-6** (offline `firma_conformance::replay`). VT-8 passes; **SC-1…6 not jointly satisfiable** — reported as a §16.2 finding. `resolve_lagged` multi-firm bug (ADR 0041). | **Done — awaiting sign-off. GATE NOT MET (finding).** |
| 7 | Resolve the SC-1…6 spec questions (Phase-3 boundary); `firma-tui`; `firma_lab` (Python — offline metrics, plots); §22.2 log-sufficiency review; then file the §30.9 pre-registration | Not started — **blocked on the SC decision** |

> **Stage renumber (Stage 4).** What earlier tables called "Stage 3b" (the
> `enforce` / `constrain` phase work) became Stage 4. The owner's Stage 5
> instruction then combined the former "locality/resource" and
> "shocks/observation" rows into one Stage (observation + shocks + locality +
> resource), so the old Stage-6 gate work is now Stage 6. The numbers track
> instructions, not the original plan.

> **Stage re-sequencing (owner instruction, Stage 2).** The owner moved the
> action plugins ahead of the decision procedure: Stage 2 builds the rules that
> *execute* an already-selected action and designs the hand-off interface both
> sides need; Stage 3 builds `decision.satisficing`, which writes
> `selected_action` through that interface. The action rules are exercised in the
> meantime by `testkit.select_action` (a hard-coded stand-in) and, for VT-7,
> directly against a mock `View`. VT-7 moved from Stage 5 to Stage 2 with the
> plugins it tests.

> Stage 1 originally split `firma-viability` (Stage 2) from the constraints
> (Stage 1); the Stage-0 review merged them — they are one coupled unit
> (§18.2's `margin(&self, x, θ, cs: &[Constraint])`). ADR 0020's dependency-graph
> table and Compliance section were edited to read "Stage 1" accordingly
> (Part B of this stage's instruction). No technical reason for a split existed.

**Phase 2 gate (§26.4):** all VT tests incl. VT-8; SC-1…SC-6 satisfied; a run
inspectable end-to-end and explicable per §12.3.
**Status after Stage 6c (ADR-0044, superseding ADR-0043 Decision 1's
framing):** VT-1…VT-8 all pass; a run is inspectable and explicable (the
smoke reports). **SC-1…SC-6 under `decision.satisficing` in a no-shock
baseline are still not jointly satisfiable** — a §16.2 finding (ADR-0040,
unchanged): SC-4/SC-5 fail as §12.3's R3 by design (ADR-0042:
width-*independent* across the full §16.1 `β × w_max` sweep); SC-2 is
bimodal; SC-1 / SC-3-solvency need attrition the no-shock model does not
produce under `decision.satisficing`. Stage 6b (ADR-0043) then found
`decision.random` jointly satisfies all six robustly across 5 seeds, and
read the whole Phase-2 gate as met on that basis. **That framing was
challenged — correctly — and is superseded by ADR-0044**: Arm C is 10/315
E1 cells; Arms A/B (96.8%) run `decision.satisficing` exclusively, so
showing the null arm's plumbing works does not show criterion 8 holds for
the hypothesis-bearing arms.

**ADR-0044's per-hypothesis reading.** For each of H1a, H1b, H1c, H2, H3, H4
(§2.4), checked whether its own DV actually requires SC-4/SC-5:
**H1a/H1b (Arm A) and H1c/H2/H4 (Arm B) do not** — their DVs
(`search_width`, `repertoire_entropy`, `survival_time`) depend only on
SC-1/SC-2/SC-3/SC-6, all already shown reachable under `decision.
satisficing` in at least one config (ADR-0040). **These five are gate-clear
under the existing no-shock/Arm-A/Arm-B `decision.satisficing` builds —
no null-arm substitution needed.** **H3 does need SC-4/SC-5, and it is not
met**: its predicted comparison (narrowing vs. shaping lag relative to
time-to-boundary) has no computable data, because shaping is never
selected under `decision.satisficing` — confirmed directly by Stage 6b's
own Arm-B-shaped, shocked config (`SC-4 = 0.0000`) at exactly the shaping
lag H3 parameterises over, and structurally by ADR-0042 (shaping never
satisfices, so it is only ever the fallback, and a shock does not force
every market action ahead of it inadmissible). **This is not "H3 is false"
— it is "H3 cannot be tested"** under the current build, in any arm, at any
lag. **The §2.5 kill criterion names only H1a/H1b/H1c — all three
gate-clear — so H3's untestability does not trigger it**, though it is a
real, named, open problem for E3 ("the distinctive mechanism", §28.2) and
for §30.3/§30.9 (E1's filing checklist requires flat "SC-1…SC-6 satisfied,"
and §30.3 currently declares H3's predicted sign as part of this filing).
Two options are flagged, neither attempted this Stage: a targeted model
revision (needs its own ADR + an adversarial literature check, per the
project's standing rule on contribution-shaping changes), or an explicit,
documented descope of H3 from the initial E1 filing (a §2.4/§30 PATCH).
`decision.random`'s joint-SC-1…6 result (ADR-0043 Decision 1) stands as real
evidence the shaping/constraint machinery is correctly wired — it is not
retracted, only no longer used as grounds for an unqualified "the gate is
met."

### The SC-1…6 / per-hypothesis finding table, updated (ADR-0044)

| SC | target | under `decision.satisficing`, no-shock (ADR-0040/0042) | under `decision.random`, tuned (ADR-0043 Decision 1) | needed by which hypotheses (ADR-0044) |
|---|---|---|---|---|
| SC-1 (0.60–0.90) | survival | reachable only from knife-edge seeds | reachable, robust across 5 seeds (0.667–0.833) | H4 (already reachable under `decision.satisficing`) |
| SC-2 (0.05–0.25) | `h < h_crit` fraction | bimodal, no middle | reachable, robust (0.050–0.081) | none of H1a/b/c/H2/H4's DVs require this band |
| SC-3 (all four bind) | — | partial (solvency knife-edge) | reachable, robust (all four, all 5 seeds) | background realism only — no hypothesis's DV depends on it directly |
| SC-4 (> 5%) | shaping fraction | **0 at every config tried, incl. the full `β×w_max` grid and under an applied shock** | reachable, robust (0.297–0.311) | **H3 only — not met under `decision.satisficing` in any arm** |
| SC-5 (0.10–0.60) | shaping success | unreachable (consequence of SC-4) | reachable, robust (0.47–0.51) | **H3 only — not met** |
| SC-6 (≥ 0.01) | entropy variance | usually reachable | reachable, robust (0.142–0.173) | H1a/H1b/H1c/H2 (already reachable under `decision.satisficing`) |
| **gate status** | | **H1a/H1b/H1c/H2/H4: gate-clear. H3: not met.** | (real evidence the plumbing works; not the basis for the per-hypothesis reading) | |

**The kill-criterion-relevant hypotheses (H1a, H1b, H1c) are all
gate-clear.** H3 ("the distinctive mechanism") is untestable as built — a
named, evidenced, non-kill-criterion finding requiring a Phase-3 decision
(model revision vs. documented descope), not fixed this Stage.

### Stage 0 outputs

- **ADR 0014** — `u` = simple trailing-window mean over `L_W` (§16.3 item 1).
- **ADR 0015** — θ global for the MVP; per-firm deferred to Phase 4; H5/E8 defined against global θ (§16.3 item 2).
- **ADR 0016** — additive simultaneous lobbying; shaping deltas are `ConflictClass::Independent` (no new class, no resolver); contested → Phase 4 (§16.3 item 3).
- **ADR 0017** — incident relation edges vanish atomically at death, no damping; cascade realism is an open Phase 3 AT-3/AT-5 question (§16.3 item 4).
- **ADR 0018** — no capability decay in the MVP; `Capability.decay_rate` field fixed at 0.0; late-game `invest_capability` dominance is an SC-6 watch item (§16.3 item 5).
- **ADR 0019** — `firma-plugin-locality` + `firma-plugin-resource` are Phase 2 scope (manual gap: §26.4 list vs §27.2 MVP contents); implementation is Stage 4.
- **ADR 0020** — new crate `firma-domain` (`← firma-core` only) for the §8 types; **not** `firma-core`, because that would put firm types in `firma-kernel`'s reachable graph (§7.3 / A1). `firma-kernel` deps verified unchanged: `{firma-core, firma-rng}`.
- **`crates/firma-domain`** — `FirmState` (§8.1 d=4), `FirmAuxState` (λ, u, aspirations), `Aspirations`, `ConstraintParams` (§8.2, global), `ScaleFactors` (§9.2 defaults), `StateBounds` (§16.1 defaults). Types only, serde-derived, `deny_unknown_fields`. 7 unit tests.
- Manual PATCH items queued for the next revision: add ADR 0014–0027 to §34.0 index; add `firma-domain` + `firma-plugin-constraint` + `firma-plugin-locality` + `firma-plugin-resource` + `firma-plugin-decision` to §38 layout and §26.4 deliverables; §18.1 dependency-graph edits (ADR 0020/0021); §16.1 gains `P_q` (ADR 0021) and `b_λ`/`b_κ` (ADR 0023/0025); §18.2 gains a `Constraint` trait sketch (ADR 0021); §12.3 gains an `h < 0` / `ψ` note and a pointer to `decision.random`'s design (ADR 0027); §9.1 `g_j` now single-sourced in `firma-domain::margin` (ADR 0026); optional §15.3 / §16.4 clarifications (ADR 0014 / 0018).
- **No shipped numerical output changed** — golden trace, `phase1-smoke` run id (`14b9eb59…3f593a0`), and all workspace tests unchanged.

### Stage 1 outputs

- **ADR 0015 edit** (Part A) — added the `theta_q: f64` rejected alternative
  (uniformity vs. exact-integer arithmetic; `i64` wins — §15.1's `g_4 = −70`).
- **ADR 0020 edits** (Part B) — `firma-viability ← domain` and the four
  constraint plugins are **Stage 1**, not Stage 2; Compliance section updated.
  No technical reason for a split.
- **ADR 0021** (Parts C, D) — the `Constraint` interface:
  `MarginTerm` (`g`, `scale`) + `Constraint: MarginTerm` (id, version,
  `ViolationSemantic`, `assumption`); a separate trait, **not** a `Rule`.
  `firma-viability` depends on `MarginTerm` only (compile-time policy-free).
  `scope` reaches `Admissibility` through its own `g_3` (no separate predicate).
  §11 deterministic core → shared pure fns in `firma-domain::dynamics` (one
  source; Stage-2 action plugins reuse). §9.3 approximations recorded in the
  `FirmaDynamics` doc + ADR. Stage 1 does **not** touch `firma-registry`.
- **`firma-domain` +`dynamics` module** — `EnvParams`, `ActionParams` (§16.1
  defaults), `y_o`/`y_r` (§11.1), `MarketAction`, `market_core`; +`constraint`
  module — `MarginTerm`, `Constraint`, `ConstraintContext`, `ViolationSemantic`
  (+`bounds_viability_kernel`). +`semver` dep.
- **`firma-viability` domain surface** — `margin` (§9.2), `FirmaDynamics` (§9.3
  deterministic core over `d≤4` `(r^L, r^I, c_index, q)`), `firma_kernel`
  (`K(θ, u_context)` = constraint `K^(0)` + `kernel_from`), `in_kernel`,
  `volume`. Phase-1 solver refactored: `kernel_from(grid, dyn, initial)` +
  `KernelSet::{empty, full, from_predicate}` now `pub` (algorithm unchanged —
  VT-2 still green).
- **`crates/firma-plugins/firma-plugin-constraint`** — `Solvency`,
  `Compliance`, `Scope`, `Obligation`, one per §9.1 row. `P_q` (`obligation`
  penalty) is a **required** param — §16.1 gives no value (manual gap).
- **`firma-cli::standard_constraints(P_q)`** — the four in canonical order
  (mirrors `standard_registry()`).
- **VT-1** (`vt1_kernel_analytic_1d_and_2d`) — analytic kernel for a 1-D
  monotone-decay box (`|Viab| = 0`) and §15.2's 2-D Example B
  (`|Viab| = 19`, vol `0.76`, sizes `[25,21,19,19]`). Explicitly counted
  toward the Phase 2 gate (was an opportunistic assertion in the Phase-1
  `vt2_*` test).
- **VT-3** (`vt3_margin_vs_exact_kernel_membership`) — **reporting** test
  (§25.4 "Report correlation"). **Finding:** at Stage 1 the market-only exact
  kernel is either the whole grid (`u_context ≤ θ_limit`) or empty
  (`u_context > θ_limit`) — no backward-iteration pruning, because `hold`
  (§11.1 action 0) costs 0 and nothing in the §11.1 core decays, so any state
  satisfying the *lethal* constraints (solvency, compliance) survives forever
  by holding. So `in_kernel ⟺ lethal-constraints-satisfied`, and the
  φ-correlation with `h > 0` is degenerate at this stage. The divergence that
  **is** present is §9.2's caveat *from the other side*: states with `h ≤ 0`
  (scope violated, `g_3 > 0`) that are perfectly viable, because `scope` is
  never lethal (§9.1) — concrete example `FirmState{r^L:0, r^I:0, c:0.0, q:0}`,
  `h = −0.800`. The §15.2-direction divergence (large `h`, outside the kernel)
  requires shock dynamics (θ moving out from under a firm) — Stage 2 — and
  VT-3 becomes a meaningful correlation then.
- **`§9.3 approximations`** recorded: (a) deterministic-core transition
  "with stochastic terms at expectation"; (b) `invest_capability` lag
  collapsed to immediate; (c) `u` held at `u_context` (not a `d≤4` dimension).
- Lint: `scripts/lint-architecture.sh` `SIM_PATH_SRC` gained `firma-domain/src`
  and `firma-plugin-constraint/src`; `check_deps.py` gained a comment that
  `firma-domain` is deliberately excluded from `firma-kernel`. The
  `no-magic-numbers` heuristic now warns on §9.2/§15.1 reference literals in
  those crates' **test** modules (test-only, warn-not-fail — OQ-5 extended).
- Dependency trace: `firma-kernel <- {firma-core, firma-rng}` **unchanged**;
  `firma-viability <- {firma-domain}`; `firma-domain` reverse-deps =
  {firma-viability, firma-plugin-constraint, firma-cli, firma-conformance} —
  **never** firma-kernel / firma-registry / firma-io / firma-config.
- **No shipped numerical output changed** — golden trace + `phase1-smoke`
  `run_id 14b9eb59…3f593a0` / `event_log_sha256 310f636f…a5ae4945`
  byte-identical. Workspace tests 62 → 76 (+14 new).

### Stage 2 outputs

- **ADR 0022** (revised during Stage-2 review — see below). The decide→act
  hand-off. "This tick's selected action" is a per-agent integer in the
  kernel's **opaque keyed store**, written by the **generic**
  `DeltaKind::SetAgentInt { field, value }` carrying
  `field = keys::SELECTED_ACTION`, read by the `act_*` rules
  (`== Some(my_index)` ⇒ act; absent/stale ⇒ safe no-op). Not a field on
  `FirmState` / `FirmAuxState` (attention-layer concern, §9.3 boundary).
  `firma-core::DeltaKind` gains **eight** variants (one per mutation *shape* —
  set-int / add-real / add-int / add-global-real / add-global-int /
  append-agent / append-global / replace-list — reused across all domain
  fields; every one has a concrete Stage-2 emitter), stored opaquely by the
  kernel (§17 A1 intact — `grep selected_action crates/firma-kernel/src` is
  empty). `Delta::Ord` and the §19.5 uniqueness check gain a `slot` key
  (resource / field / list name) so a rule may emit e.g. `AdjustStock{capital}`
  **and** `AdjustStock{input}` for one agent; the append kinds are
  `allows_repeat()` and exempt from uniqueness.
- **ADR 0023** — the `Λ` queue. `firma-domain::Effect`
  (`CapabilityGain` / `Lobby` / `Contract` / `Diversify`) + `LaggedRecord`
  (`{maturity_tick, effect}`), carried as canonical JSON in `PushAgentRecord`.
  **Shaping success is decided at commitment** (phase 5) using legitimacy *now*
  and frozen into `Effect.applied` (ADR-0014-style pinning of an underspecified
  timing; avoids retroactive legitimacy effects); `resolve_lagged` just applies
  the record. Two RNG purpose tags — `"shaping_lag"` and `"shaping_success"` —
  first real per-agent keyed RNG in the running kernel. `invest_capability` uses
  the same queue (fixed `Δ_cap = 3`, not drawn).
- **`firma-core`** — 8 `DeltaKind` variants + `DeltaKindTag`s;
  `discriminant() 0..=8`, `slot()`, `allows_repeat()`; manual `impl Eq` on
  `Delta` / `DeltaKind` (the `f64` scalar payloads block the derive) — sound
  because `firma-kernel` **enforces** the finite-payload invariant (Fix 1
  below), not because it is assumed; 6 new `View` methods with `None` / `&[]`
  default impls; `firma-core::delta::tests` (discriminants, slot ordering,
  strict-weak, finite-`f64` equality, documented NaN behaviour).
- **`firma-kernel`** — `World` gains six `#[serde(default)]` opaque stores
  (`agent_reals` / `agent_ints` / `global_reals` / `global_ints` /
  `agent_lists` / `global_lists`) + seed/read/mutate methods; `apply_delta`
  gains 8 arms, **none naming a domain key** (routes by the carried string);
  `WorldView` delegates the 6 reads; the per-delta validation loop rejects a
  non-finite `Adjust*Real` (`KernelError::NonFiniteDelta`, Fix 1); uniqueness
  check widened to `(target, disc, slot)` with the `allows_repeat` exemption.
  5 new tests (round-trip, snapshot round-trip, repeat-push-ok/repeat-set-
  rejected, slot-uniqueness through the kernel, non-finite-rejected-atomically).
- **`firma-domain`** — `keys` (canonical `&str` constants), `effect`
  (`Effect`, `LaggedRecord`, `Effect::deltas_at_maturity`), `relation`
  (`RelationGraph` / `Edge` / `EdgeKind` — minimal: `has_supply_partner`,
  `supply_source_count`), `shaping` (`SuccessModel` + `p_success`, `LagRange`,
  `LobbyParams` [§16.1 defaults], `ContractParams` / `DiversifyParams`
  [every field required — no §16.1 values]). **`SuccessModel.b_lambda` /
  `b_kappa` have no serde default** (Fix 2): required on every path; a config
  omitting them is rejected, not silently given `0.2` / `0.1` — `lobby`'s
  defaults supply them explicitly as `[D]` constants `LOBBY_B_LAMBDA` /
  `LOBBY_B_KAPPA`. `+serde_json` dep (already a workspace dep; `Effect` owns
  its wire format). +14 unit tests (25 total).
- **`crates/firma-plugins/firma-plugin-action-market`** (`1.0.0`) — six
  `MarketRule`s (§11.1 indices 0–5). Each calls `firma_domain::dynamics::
  market_core` (no formula re-implementation) and emits the state diff as
  `AdjustStock` deltas **paired with the env pool** for conservation;
  `invest_capability` emits only the cost now + a lagged `CapabilityGain`;
  `deliver` emits an unpaired `AdjustAgentInt` on `obligation` (not conserved).
  `market_core` → `None` ⇒ silent no-op (defensive; documented). 8 tests.
- **`crates/firma-plugins/firma-plugin-action-shaping`** (`1.0.0`) — `lobby` /
  `contract` / `diversify` `ShapingRule`s (§11.2 indices 6–8) implementing all
  three §11.3 properties (cost at commitment, drawn lag ≥ 1 via
  `open_for(_, "shaping_lag")`, probabilistic success via
  `open_for(_, "shaping_success")` vs `p_max < 1`); plus `LaggedEffectResolver`
  (phase `ResolveLagged`, calls `Effect::deltas_at_maturity`, drains with
  `ReplaceAgentList`). `support::MockView` for cross-crate tests. 13 tests.
- **`firma-plugin-testkit::SelectAction`** (`testkit.select_action`) — Part F:
  a decision-procedure stand-in that emits
  `SetAgentInt { field: keys::SELECTED_ACTION, value }` from a fixed
  `(agent → action)` map every `decide` phase. No theory. This adds the
  testkit's **only** `firma-domain` dependency (`plugins ← domain`, §18.1).
- **VT-7** (`vt7_shaping_actions_are_costly_lagged_and_uncertain`) — iterates
  `firma_plugin_action_shaping::shaping_catalog()` generically; per plugin,
  asserts declared `p_max < 1` and `Δ_min ≥ 1`, then over 400 seeds: cost paid
  and conserving on every seed regardless of outcome, `maturity_tick >
  commit_tick ≤ Δ_max` always, both success and failure occur, realised rate
  `≤ p_max` and near `p_success`. Raw pass output (400 seeds each):
  `lobby` 143✓/257✗ rate 0.357 (p 0.350); `contract` 125✓/275✗ rate 0.312
  (p 0.300); `diversify` 98✓/302✗ rate 0.245 (p 0.250); lags spanned their
  full `[Δ_min, Δ_max]`.
- **Conservation model** — production creates `r^L` / consumes `r^I`, so every
  market and shaping stock change is paired with the opposite change on the
  shared env pool (the "market"). A run using these rules MUST seed the env
  pool large enough that it never goes negative; env-pool dynamics are a
  Stage-4 Resource-plugin concern. Recorded in both plugin crates' docs.
- Lint / workspace: `Cargo.toml` gains the two plugin crates + workspace deps;
  `scripts/lint-architecture.sh` `SIM_PATH_SRC` gains their `src`; the two new
  crates are `1.0.0` (ADR 0022), not `version.workspace`;
  `firma-plugin-testkit` gains a `firma-domain` dependency (Fix 3 — §18.1
  `plugins ← domain`).

- **Stage-2 review — three approved fixes applied** (round 3):
  1. **Non-finite real deltas rejected at the boundary.** New
     `KernelError::NonFiniteDelta { plugin, field }`; `run_phase`'s per-delta
     validation loop rejects a non-finite `AdjustAgentReal` / `AdjustGlobalReal`
     in the same pre-apply pass as the undeclared-kind / duplicate checks, so
     it goes through the identical abort path (test
     `non_finite_real_delta_is_rejected_atomically` — `NaN` **and** `+∞`, and
     asserts the phase's *valid* sibling delta did not apply). The manual
     `impl Eq` on `Delta` now rests on an enforced invariant.
  2. **`b_λ` / `b_κ` required.** Serde defaults removed from `SuccessModel`;
     `LOBBY_B_LAMBDA` / `LOBBY_B_KAPPA` constants keep the `[D]` values for
     `LobbyParams::default()` only. Tests
     `success_model_b_coefficients_have_no_serde_default` (domain) and
     `b_coefficients_are_required_when_a_success_block_is_given` (plugin). VT-7
     numbers unchanged.
  3. **`SetSelectedAction { action: u8 }` → generic `SetAgentInt { field,
     value }`.** `apply_delta` routes by the carried `field`;
     `grep selected_action crates/firma-kernel/src` → **empty**.
     `testkit.select_action` emits `SetAgentInt { field: keys::SELECTED_ACTION,
     .. }`.

  **ADR process correction (Stage 3, Part A).** Fixes 2 and 3 were first
  applied as **in-place edits to accepted ADR bodies** (ADR-0022 Decisions
  1–3, ADR-0023's parameter-default note). That violates the immutability rule.
  Both ADRs have been **reverted to their as-accepted text**; the corrections
  now live in new superseding ADRs — **ADR-0024** (`SetAgentInt` + the
  enforced-`NaN` invariant, supersedes ADR-0022 Decisions 1–3) and **ADR-0025**
  (`b_λ`/`b_κ` required, supersedes ADR-0023's note). No code changed — the
  Stage-2 tree was already correct (`SetAgentInt` everywhere). The strict rule
  ("accepted ADR bodies are append-only; only `Status` lines change;
  corrections are new ADRs") is now written into `docs/adr/README.md`.
- **`firma-core` / `firma-kernel` are no longer "untouched since Phase 1"**
  (ADR 0022 says so plainly). Their **dependency sets are unchanged**
  (`firma-core ← {}`, `firma-kernel ← {firma-core, firma-rng}` — verified by
  `cargo tree`); only the API grew.
- Verification: `cargo fmt --all --check` clean; `cargo clippy --workspace
  --all-targets -- -D warnings` clean; `cargo test --workspace` **128 pass, 0
  fail** (76 → 128); `bash scripts/lint-architecture.sh` passes (dep graph
  clean — `no-kernel-domain-deps` / `no-cross-plugin-deps` unaffected by the
  testkit → `firma-domain` edge); `cargo metadata | check_deps.py` exit 0;
  `cargo tree -p firma-kernel` shows only `firma-core`, `firma-rng`;
  `grep -rn selected_action crates/firma-kernel/src` → empty.
- **No shipped numerical output changed.** Golden trace (`GOLDEN_RUN_ID
  f304edd4…5a099`) green + `phase1-smoke` re-run gives
  `run_id 14b9eb598a0c…3f593a0` / `event_log_sha256 310f636fe4ed…a5ae4945`
  byte-identical to the recorded values (no existing config emits a new
  variant). **Workspace version stays `0.1.0`** (ADR 0022 Decision 4): bumping it would change every
  `RunIdentity.engine_version` → every `run_id`, including `phase1-smoke`'s,
  for a version string with no trajectory behind it. **Pre-1.0 breaking
  `firma-core` changes are accumulating for a single MAJOR bump at the first
  real release (1.0), with one golden-trace regeneration then.** So far:
  (Stage 2) 8 `DeltaKind` variants + `DeltaKindTag`s (incl. the generic
  `SetAgentInt`), `Delta` / `DeltaKind` lose derived `Eq` for a manual impl,
  `Delta::Ord` gains a key, 6 `View` methods (defaulted — additive for external
  impls), `KernelError` gains `NonFiniteDelta`. (Stage 3 adds no `firma-core` /
  `firma-kernel` change — the decision plugins are pure `Rule`s.)

### Stage 3 outputs

- **Part A — the ADR immutability rule, decided and enforced.** Strict, full
  immutability from acceptance: accepted ADR **bodies are append-only**, only
  `Status` lines change, corrections are always new numbered ADRs. Written into
  `docs/adr/README.md`. **Reverted** the two Stage-2 in-place edits: ADR-0022
  Decisions 1–3 (Fix 3's `SetAgentInt` text) → restored to the as-accepted
  `SetSelectedAction` version; ADR-0023's parameter-default note (Fix 2) →
  restored. The corrections now live in:
  - **ADR-0024** — generalise `SetSelectedAction` → `SetAgentInt`; put `Delta`'s
    manual `Eq` on the enforced non-`NaN` invariant. Supersedes ADR-0022
    Decisions 1–3. Paperwork-only (the code was already correct).
  - **ADR-0025** — `b_λ` / `b_κ` are required shaping config, not
    serde-defaulted. Supersedes ADR-0023's parameter-default note.
    Paperwork-only.
  ADR-0022 / 0023 `Status` lines updated to point at the superseders; README
  index shows the supersession. **No code changed for Part A.**
- **ADR-0026** — the four §9.1 `g_j` and a concrete `standard_margin` move to
  `firma-domain::margin`. `firma-plugin-constraint`'s four `MarginTerm::g` impls
  become one-line delegates (behaviourally inert — §15.1 / VT-1 / VT-2 / VT-3
  byte-identical). `firma-viability::margin` (generic fold) unchanged.
  `decision.satisficing` computes `h` from `standard_margin` with no
  cross-plugin dependency and no formula duplication.
- **ADR-0027** — `decision.random`'s selection rule (§20.3 category, no manual
  formula): **uniform over the admissible set**, ignoring focus / narrowing /
  priority / satisficing entirely; one `mechanism`-stream draw per firm,
  `purpose_tag = "decision_random"`. Reasoning: uniform-over-*all* with
  inadmissible→no-op manufactures an idle-rate ↔ `h` correlation; the admissible
  set is never empty (`hold`); admissibility shrinkage stays visible as the
  §28.3 alternative explanation rather than masked.
- **`crates/firma-plugins/firma-plugin-decision`** (`1.0.0`):
  - `decision.satisficing` — §12.3 Steps 1–5 transcribed exactly. `ψ(h)`
    handles `h < 0` via `max(h,0)` (⇒ `ψ = 0`, `w_eff = 1` for `β > 0`;
    `0^0 = 1` for `β = 0` — the null falls out of the formula, no special
    branch). Scan-order table verbatim. First-satisficing (not argmax);
    inadmissible actions don't consume budget; fallback = first admissible in
    priority order. `NONE` focus ⇒ repeat last tick's `keys::SELECTED_ACTION`
    (absent ⇒ `hold`), no scan. Writes `SELECTED_ACTION`, `FOCUS`, `W_EFF`.
    **Zero `firma_rng` calls** — grep-checked by
    `rng_is_used_only_by_decision_random`.
  - `decision.random` — ADR-0027. No `FOCUS` / `W_EFF`.
  - `decision.aspiration_update` — §12.1 `A ← A + α(v − A)` (`α ∈ (0,1)`,
    §16.1 default `0.10`), in the `record` phase (§10.1 phase 9). Absent
    aspiration ⇒ seed to `v_j`. Persists `v_1` (`REALIZED_CAPITAL_GROWTH`) and
    the `r^L_{t-1}` baseline (`PREV_TICK_CAPITAL`) that `decision.satisficing`
    reads next `decide`.
  - Admissibility (`market_core(…).is_some()` for market; cost + supply-partner
    for shaping) and one-step lookahead shared between `satisficing` and
    `random`; `firma_domain::dynamics::shaping_cost_step` (cost-only shaping
    lookahead — the θ/edge payoff is lagged and invisible at `t+1`, so a shaping
    action never satisfices a positive shortfall on a one-step horizon; SC-4
    watch item). 16 unit tests.
- **`firma-domain`** — `+margin` module (`g_solvency` … `standard_margin`),
  `+dynamics::shaping_cost_step`, `+7 keys` (`FOCUS`, `W_EFF`,
  `REALIZED_CAPITAL_GROWTH`, `PREV_TICK_CAPITAL`, `REGULATED_INTENSITY`, three
  `ASPIRATION_*`). 25 → 28 unit tests (margin::tests).
- **Part E — `Attention` component (§8.4).** **Deferred again, deliberately.**
  The keyed-store interim (ADR-0022 Decision 1 / ADR-0024) is extended to carry
  `focus` and `w_eff` (`keys::FOCUS` / `keys::W_EFF`), which is all offline R2
  (§14.2 search-width rigidity) and `firma-tui` display need from the event log
  — without building the agent component-bag, which is a larger architectural
  move (Phase 5 coalitions need it too). **Trigger to build the typed
  component:** Stage-6 `firma-tui` needing typed, in-memory focus *history*
  (not just the last value) for live display, or the component-bag refactor
  happening for Phase 5. Noted in `firma-domain::keys` and here so "eventual
  home" does not quietly become "never".
- **VT-4** (`vt4_beta_zero_no_shortfall_action_never_changes`) — §3.1 inertia
  principle. Focus = NONE (no aspiration seeded ⇒ every `ς_j = 0`) ⇒
  `decision.satisficing` repeats the previous action for 60 ticks, for **every**
  `β ∈ {0, 0.5, 1, 2, 4}` (β-independent — NONE bypasses Steps 3–5). Both a
  no-prior firm (→ constant `hold`) and a prior-action-2 firm (→ constant
  `produce_regulated`): **0 action changes** in every configuration.
- **VT-5** (`vt5_decision_random_establishes_the_null`) — reporting test
  (ADR-0027). 30-cell grid over `r^L` × `u`-gap × capability. Per cell:
  `d_TV(realised, uniform-over-admissible)` from 2000 draws. Raw pass output:
  `max d_TV = 0.0278` (choice is uniform-over-admissible at every `h`);
  `r(h, d_TV) = −0.069`, **bootstrap 95 % CI [−0.398, +0.259] contains 0** ⇒
  the null holds (no `h` ↔ narrowing-rigidity relationship); `r(h, |A_adm|) =
  +0.735` — the §28.3 admissibility channel, visible and strong as intended.
  "Indistinguishable from zero" ≡ that bootstrap CI (2000 resamples,
  deterministic RNG) contains 0.
- Lint / workspace: `Cargo.toml` + `scripts/lint-architecture.sh` `SIM_PATH_SRC`
  gain the decision crate; it is `1.0.0`.
- Verification: `cargo fmt --all --check` clean; `cargo clippy --workspace
  --all-targets -- -D warnings` clean; `cargo test --workspace` **148 pass, 0
  fail** (128 → 148); `bash scripts/lint-architecture.sh` passes;
  `cargo metadata | check_deps.py` exit 0; `cargo tree -p firma-kernel` still
  `{firma-core, firma-rng}`; `grep -rn selected_action crates/firma-kernel/src`
  empty. **No shipped numerical output changed** — golden trace green,
  `phase1-smoke` `run_id 14b9eb59…3f593a0` / `event_log_sha256 310f636f…a5ae4945`
  byte-identical. `firma-core` / `firma-kernel` **untouched** this Stage.

### Stage 4 outputs

`constrain` + `enforce`, and the first real end-to-end run.

- **Part A — phase-attribution tension resolved (ADR-0028).** Every θ-writer was
  traced: a θ value only ever changes via a phase-1 scheduled shock or a phase-6
  matured lagged effect (`LaggedEffectResolver`, per ADR-0023). There is no third
  θ-writer. §10.1's phase-7 line "θ updates" describes θ's *already-settled*
  state going into `constrain`, not a write performed there. `LaggedEffectResolver`
  **stays in phase 6** — no ADR supersedes ADR-0023. Phase 7's only job is
  maintaining the action window `W`.
- **Part B/C — the action window `W` and `u` (ADR-0028).** `W` is a per-agent
  keyed list under `keys::ACTION_WINDOW`, reusing `ReplaceAgentList` (one delta
  per agent per tick). `constraint.action_window` (phase `Constrain`) appends
  this tick's `selected_action` and trims to `L_W`. **`u` is computed on demand**
  from `W` (`firma_domain::margin::u_from_window`, ADR-0014), exactly like `h` —
  no cache, **no new `SetAgentReal` DeltaKind**. `firm_u` falls back to a seeded
  `keys::REGULATED_INTENSITY` while `W` is empty (tick 0). `h` is not persisted
  (§17 A5 holds).
- **Part D — the `enforce` phase (ADR-0029).** All four §9.1 semantics, one pass
  per live agent (ascending `AgentId`): `solvency` → `RemoveAgent` (fires at
  `g_1 ≥ 0` — `r^L == 0` is insolvency and `g_1 > 0` is unreachable under
  non-negativity); `compliance` → graduated (`P_c = 30` fine + `δ_λ = 0.15`
  legitimacy loss on a first strike; a second strike **within `T_c = 4` ticks,
  inclusive** is fatal); `scope` → explicit no-op arm (`AdmissibilityGate` —
  checked in `decide`, never here); `obligation` → sever the agent's `supply`
  edges + `P_q` fine, never lethal (`P_q` required, no §16.1 default). Two new
  DeltaKinds: `RemoveAgent { reason }` (disc 9) and `ReplaceGlobalList { list,
  records_json }` (disc 10). `run_phase` emits `Event::AgentDied` on a
  `RemoveAgent` apply (symmetric with `AgentBorn`). Edge severance is the rule's
  job (not the kernel — A1): one `ReplaceGlobalList` per phase. A dying firm's
  stocks are **transferred to the environment pool** via paired `AdjustStock`
  (conservation stays exact; the flow is in the log — §22.2). 14 unit tests in
  `firma-plugin-constraint`.
- **Two reconciler bugs found by the first multi-agent run, each fixed with a
  numbered ADR:**
  - **ADR-0031, narrowed by ADR-0033** — a single rule acting for `N` agents
    against the shared environment pool makes `N` legitimate `ResourcePool`
    claims on one cell (`DeltaTarget::Environment::sort_key()` is a sentinel
    with no agent in it); the per-rule uniqueness guard (widened to `(target,
    kind, slot)` by ADR-0022 D3) wrongly rejected the 2nd. Fix: **`ResourcePool`
    deltas targeting `Environment`** are exempt from that guard — the §19.4
    step-4 resolver aggregates them and the stable step-2 sort keeps
    ascending-`AgentId` order. ADR-0031 first wrote this as "all `ResourcePool`
    deltas", which also dropped the guard for agent-targeted pooled deltas (a
    silent same-agent double-debit path — confirmed by a test); **ADR-0033**
    narrows it to the `Environment` target only, restoring the guard for
    `Agent`-targeted `ResourcePool` deltas. `Independent` deltas keep the strict
    guard throughout. **Flags a manual §19.4/§19.5 wording discrepancy — OQ-10.**
  - **ADR-0032** — `Delta`'s `Ord` compares `conflict_class` before
    `discriminant`, so a dying agent's `Independent` `RemoveAgent` sorts *before*
    its own `ResourcePool` stock-transfer deltas — the transfers then hit a
    removed agent. ADR-0029 Decision 2's "disc 0 sorts before disc 9" rationale
    was wrong. Fix: `RemoveAgent` is applied in a final order-preserving
    sub-pass, after every value delta. Supersedes **only** ADR-0029 Decision 2's
    ordering rationale (the mechanism is unchanged); ADR-0029 `Status` line and
    README updated. 1 new kernel test.
- **ADR-0030 — config seeds domain state.** `WorldConfig.{global_reals,
  global_ints}` and `AgentConfig.{reals, ints}` (all `BTreeMap`, `#[serde(default,
  skip_serializing_if = "BTreeMap::is_empty")]` — an omitted map serialises to
  nothing, so a pre-ADR-0030 config canonicalises byte-identically and the
  golden / `phase1-smoke` `run_id` is provably unaffected). Orchestrator
  `build_world` seeds via the existing `World::set_*`. `schema_version` stays
  `1.0.0` (purely additive).
- **Part E — `firma_cli::model_registry()` + `firma run --model` + the run.**
  `model_registry()` assembles every `Rule` built so far (6 market actions,
  3 shaping + `resolve_lagged`, `decision.{satisficing,random,aspiration_update}`,
  `constraint.{action_window,enforce}`) + the additive resolver.
  `configs/experiments/phase2-smoke.json` — 4 firms, 10 ticks, no shocks, static
  prices, θ / capability / aspirations seeded. **The run (real event-log output
  in the Stage-4 report):** agent 3 dies of `solvency` at tick 0 (spent its last
  capital on input); agents 0 & 1 over-produce regulated output, take a
  `compliance` first strike at tick 1 (−30 capital, −0.15 legitimacy), then die
  of the graduated second strike at tick 2 — 533 capital + 17 input each
  transferred to the pool; agent 2 invests in capability every tick and its
  lagged `+0.05` gains mature in phase 6 from tick 3 on; it is the sole survivor.
  **Conservation reconstructed from the event log alone** (VT-6 style): capital
  `1 001 212` and input `1 000 045`, unchanged start to finish. Two conformance
  tests added (`phase2_smoke_runs_a_realistic_tick_and_conserves`,
  `config_seeds_domain_state_into_the_world`, `config_without_seed_maps_is_unchanged`).
- **Emergent observation (not a bug).** With `decision.satisficing`, a firm
  whose regulated-activity intensity approaches `θ_limit − h_crit` drops into
  SURVIVAL focus and stops choosing `produce_regulated` (it is dominated in the
  SURVIVAL scan order by `produce_ordinary`). So a *compliance violation* is only
  reachable when the per-tick `u` step `1/L_W` is large enough to jump the firm
  from "just under the survival band" to "over `θ_limit`" in one tick — the
  smoke config uses `L_W = 4` (step `0.25`) for exactly this. At `L_W = 8` the
  satisficing firm self-regulates below the limit and `enforce` never fires on
  `compliance`. Worth a note for E1 interpretation; not acted on.
- **ADR-0033 follow-up (post-review of Stage 4).** A test showed ADR-0031's
  exemption was over-broad: a single rule emitting two `Agent`-targeted
  `ResourcePool` `AdjustStock` deltas for one `(agent, resource)` in a phase
  applied both silently (agent debited 10 instead of 5; conservation and
  non-negativity both pass, so nothing downstream catches it). ADR-0033 narrows
  the exemption to `DeltaTarget::Environment` only; `Agent`-targeted
  `ResourcePool` deltas are back under the guard. New kernel test
  `same_agent_duplicate_resourcepool_delta_is_rejected`; the ADR-0031/0032
  multi-agent test and the `phase2-smoke` run are byte-identical before and
  after (their repeated pool deltas all target `Environment`). ADR-0031 `Status`
  line + README updated.
- Verification: `cargo fmt --all --check` clean; `cargo clippy --workspace
  --all-targets -- -D warnings` clean; `cargo test --workspace` **all green**
  (kernel 13→15, constraint 5→19, cli 3→4, conformance integration 3→6); `bash
  scripts/lint-architecture.sh` passes; `cargo metadata | check_deps.py` exit 0;
  `cargo tree -p firma-kernel` = `{firma-core, firma-rng}` (unchanged). **No
  shipped numerical output changed** — golden trace green (`run_id f304edd4…`);
  `phase1-smoke` `run_id 14b9eb59…3f593a0` / `event_log_sha256 310f636f…a5ae4945`
  byte-identical, even though this Stage adds the most machinery yet.

### Stage 5 outputs

observation, shocks, locality, resource dynamics — the `environment` /
`observe` phases, plus the two interaction/resource plugin categories ADR-0019
scoped into Phase 2. **This Stage completes the §26.4 model deliverable set.**

- **Where the manual was genuinely silent, and had to be designed (not
  transcribed):** the `Ramp` / `Persistence` per-tick formulas (ADR-0036, all
  `[D]` in `firma_domain::shock` rustdoc); the `Exponential(rate)` reading
  (ramp-*up*, not decay); the `Recurring(period)` reading (staircase, not
  spike); the incremental-shift-with-reversal model for level-shift channels;
  the `resource.patchy` mechanic (mean-reverting `π^I`, ADR-0038 — the §20.4
  example's `regen`-pool reading is deferred, needs kernel source/sink
  accounting); the `Observation` Rust interface (ADR-0035); the `Locality`
  Rust interface (ADR-0037). Everything from §13.2's channel *table* and
  §12.2's variant *names* is transcribed exactly.
- **ADR-0034** — `Rule::rng_stream() -> Option<StreamId>` (the 8th `Rule`
  method, against §20.2 "resist growth" — justified: two §21.3 stream needs
  (`shock`, `environment`) both live in phase 1, so the stream is not
  derivable from the phase). Defaulted `None` ⇒ phase default ⇒ every Phase-1
  rule byte-identical. Plus `KeyedRng::next_normal` (Box–Muller) in `firma-rng`.
- **Part A — `firma-plugin-observation` (ADR-0035).** `observation.{full,
  noisy,delayed}`, phase `Observe`, writing one `EnvSnapshot` per agent under
  `keys::OBSERVED_ENV` (`ReplaceAgentList`). `noisy` — `Normal(0, σ)` per
  field per firm, **`environment`** stream (a `β` sweep must hold the
  perceptual channel fixed), prices re-rounded; `delayed` — a global
  `ENV_HISTORY` ring, every firm sees `k` ticks back. `σ` / `k` **required**
  (no §16.1 value). `decision.{satisficing,random}` `theta()`/`env_params()`
  now read the per-agent snapshot **or fall back field-by-field to the true
  global store** — the one `decide` change this Stage, byte-identical for a run
  with no `Observation` plugin (verified: golden, `phase1-smoke`,
  `phase2-smoke` all unchanged). §12.2's own-state/own-`h` caveat is stated in
  every variant's docs and `assumption()`.
- **Part B — `firma-plugin-shock` (ADR-0036).** `shock.{scheduled,stochastic}`,
  phase `Environment`. All four §13.2 channels: `Resource` (`AdjustGlobalInt`
  `input_price`), `Regulatory{ThetaLimit|ThetaCap}` (`AdjustGlobalReal` — the
  **same** mechanism `Effect::Lobby` uses, §9.4), `Competitive`
  (`AdjustGlobalInt` `output_price`), `Reputational` (`AdjustAgentReal`
  `legitimacy` per targeted firm — the "supply weights reduced" companion is
  deferred, no consumer reads `Edge.weight`). Deltas are **incremental**: a
  shock tracks a target cumulative shift `S(t)` and emits `S(t) − S(t−1)`; a
  `Transient` shock reverses in one step when its window closes. `Σ_t` =
  `keys::ACTIVE_SHOCKS` (logged for offline N1/N2 — the memory ring `M` stays
  deferred, N1 is offline). `shock.stochastic` draws `onset`/`magnitude` once
  at tick 0 from the **`shock`** stream. **One shock plugin per config** (like
  one decision plugin — two rewriting `ACTIVE_SHOCKS` under Jacobi would
  clobber). **No `Event::ShockFired`** — a shock is fully in the log via its
  deltas + `ACTIVE_SHOCKS` (§22.2).
- **Part C — `firma-plugin-locality` (ADR-0037).** `Locality` trait
  (`neighbours(agent, live, tick)`), `WellMixed` / `Network`. **Not a `Rule`**
  (a query service — emits no delta, no phase); **not in `model_registry`**;
  reached via `firma_cli::locality_catalogue()`. Distinct from `G_t`:
  potential-interaction structure vs realised economic ties. **Nothing in the
  MVP consumes `neighbours()`** — the consumers (rivalry, H5 externalities)
  are Phase 4 (§26.6); built now per §27.2 / ADR-0019.
- **Part D — `firma-plugin-resource` (ADR-0038).** `resource.constant` (a
  `Rule` that emits nothing — makes the implicit static environment of every
  Stage-2–4 config explicit and manifest-recorded) and `resource.patchy`
  (mean-reverting random walk on `π^I` around a `baseline`, `environment`
  stream, all params required `[D]`). "Patchy" = temporal price variation
  stressing `solvency`, **not** a depleting/regenerating pool (that reading of
  the §20.4 example config is deferred — needs kernel source/sink accounting).
- **New DeltaKind needed this Stage: none.** Observation uses `ReplaceAgentList`
  + `ReplaceGlobalList`; shock uses `AdjustGlobal{Real,Int}` / `AdjustAgentReal`
  + `ReplaceGlobalList`; resource uses `AdjustGlobalInt`; locality emits
  nothing.
- **Part E — wiring + `configs/experiments/phase2-stage5-smoke.json`.**
  `model_registry()` gains `observation.*`, `shock.*`, `resource.*` (22 rules;
  `locality.*` separate). The smoke config: 4 firms, 16 ticks,
  `resource.patchy` on `π^I`, `shock.scheduled` (a `Regulatory` `Linear(3)`
  ramp `θ_limit −0.10` at onset 6, and a `Transient(3)` `Resource` spike `+2`
  at onset 10 that reverses), `observation.delayed(2)`. **Real run:** the
  regulatory shock ramps `θ_limit` down in three −0.0333 steps (ticks 7/8/9);
  the resource spike raises `π^I` by 2 at tick 10 and reverses at tick 13;
  `resource.patchy` walks `π^I` around baseline 2; every firm decides on a
  two-tick-stale environment (agent 0 at decide-tick 3 sees source-tick 1,
  etc.). One firm dies (solvency, from the input-cost spike), 3 survive.
  **Conservation exact** (reconstructed from the log — VT-6). **Deterministic**
  (identical `event_log_sha256` across two runs). A separate conformance test
  (`stage5_rng_streams_follow_the_declared_stream`) runs `observation.noisy` +
  `shock.stochastic` together and confirms determinism *and* that changing
  only the `mechanism` seed leaves the drawn shock untouched (matched-
  environment design, §21.3).
- **Follow-up (post-review of Stage 5).**
  - **ADR-0039** — the registration-time-metadata alternative to
    `Rule::rng_stream()` (an `Option<StreamId>` on `RegisteredRule` instead of
    a trait method) was weighed and **rejected**: the stream is a property of
    the draw's semantics (§21.3 assigns streams to kinds of work — there is no
    valid config where `shock.stochastic` draws from `mechanism`), the kernel
    sees `&dyn Rule` and never `RegisteredRule` (routing metadata there is
    *more* plumbing — `Schedule` shape, orchestrator threading, `AddRule`
    intervention, every kernel test fixture), and a defaulted trait method is
    checkable in an isolated unit test where registration metadata has no
    home. ADR-0034's body is unchanged (immutability house rule); its `Status`
    line points at 0039. **No code change.**
  - **`observation_noisy_changes_a_decision_versus_the_true_value_baseline`**
    (conformance) — a firm with `c = θ_cap = 0.40` exactly: under
    `observation.full` `h = 0 < h_crit` ⇒ SURVIVAL ⇒ `selected_action = 1`
    (`produce_ordinary`); under `observation.noisy(σ=0.15)` the seeded
    perturbation puts perceived `θ_cap` below `c` ⇒ `h > 0` ⇒ GOAL(1) ⇒
    `selected_action = 2` (`produce_regulated`); at `σ=0.05` a *third* action
    (`4`, `invest_capability`). `noisy` is demonstrably **not inert when
    present**, and the effect is graded. σ = 0.15 is where `produce_regulated`
    specifically appears because the flip is a *focus* change needing
    perceived `θ_cap` ≈ `h_crit·s_c = 0.075` below true — a hazy-threshold
    read, not a degenerate σ (the manual gives no σ; it is an experimental
    knob).
- Verification: `cargo fmt --all --check` clean; `cargo clippy --workspace
  --all-targets -- -D warnings` clean; `cargo test --workspace` **205 pass, 0
  fail** (new: `firma-rng` +1, `firma-plugin-observation` 6, `-shock` 7,
  `-locality` 3, `-resource` 7, conformance `integration` 6→10); `bash
  scripts/lint-architecture.sh` passes (`SIM_PATH_SRC` gains the 4 new plugin
  `src` dirs); `cargo metadata | check_deps.py` exit 0 (no cross-plugin deps);
  `cargo tree -p firma-kernel` = `{firma-core, firma-rng}` unchanged. **No
  shipped numerical output changed** — golden trace green (`run_id f304edd4…`);
  `phase1-smoke` `run_id 14b9eb59…3f593a0` / `event_log_sha256 310f636f…a5ae4945`,
  the Stage-4 `phase2-smoke` `run_id 0529c6bb…` / `event_log_sha256 9a476938…`,
  and the Stage-5 `phase2-stage5-smoke` `run_id 5824031c…` / `event_log_sha256
  fd3aa4f2…` all byte-identical (the `Rule` trait grew a method and the
  decision plugin grew a fallback read, both inert for existing configs).

### Stage 6 outputs — the Phase-2 gate

**Outcome: VT-8 passes; SC-1…SC-6 are NOT jointly satisfiable — reported as a
§16.2 finding, not tuned around. The Phase-2 gate is not met, and passing it
requires a Phase-3 specification decision, not more work at this level.**

- **Part A / VT-8 seam (ADR-0040).** §12.3 Steps 2–5 (Attend, Narrow, Scan
  order, Satisficing selection) extracted to `firma_plugin_decision::select(h,
  shortfalls, β, h_crit, w_max, prev, admissible, satisfices) -> Selection`.
  `h` and `shortfalls` are **independent typed parameters with no expression
  relating them** in the body — criterion (iii)'s structural half is a
  one-signature fact (§17 A3 posture applied to a validation test). The
  **pure-function alternative the instruction asked me to weigh was adopted**;
  the drift risk is closed by `Satisficing::apply` carrying **zero Step-2–5
  logic of its own** (it computes `h`/`ς`, builds the two closures, calls
  `select`) — verified byte-identical (golden + all three smoke configs).
- **Part B / criterion (iii) greps.** Two one-directional greps, both empty:
  (a) `firma-domain::margin` / `constraint` / `firma-viability` never read an
  aspiration or shortfall (only `#[cfg(test)]` fixtures + a zeroed
  `FirmAuxState.aspirations` field + one comment saying so); (b)
  `decision.aspiration_update` never reads `θ`, any `g_j`, `h`, `h_crit`,
  `focus`, `w_eff`, `λ`, or `u`. **The distinction stated in the ADR:**
  independent = *no functional path* between `h` and `ς`; **not** uncorrelated
  (they legitimately share `r^L`, `c`, `q` as common inputs, which is why they
  covary in Arm B).
- **Part C / VT-8 grid** (`vt8_orthogonal_manipulation_of_h_and_shortfall`).
  7×7 grid. **(i) `r(h, ς) = +0.000000`** — a balanced full factorial has
  exactly zero factor correlation. **(ii) quadrants 9/12/12/16** — all four of
  `(h < h_crit) × (max_j ς_j > 0)` populated. **(iii)** the Step-3 narrowing
  width `ψ(h)→w_eff` is a single value per `h`-level across all ς (asserted).
  Descriptive: `r(h, w_eff) = +0.36` (narrowing), `r(ς, w_eff) = +0.38`
  (ς→focus→whether a scan happens) — both non-zero, neither mechanical, so
  H1a/H1b are real claims. **VT-8's three criteria all hold.**
- **Part D / SC-1…SC-6** (`firma_conformance::replay` — offline
  reconstruction of `(r^L, r^I, c, q, λ, u, θ)` from `DeltaApplied`, `h`/`g_j`
  recomputed with `firma_domain::margin`; VT-6 fold pattern, §17 A5). Metric
  definitions: SC-1 survivor fraction; SC-2 fraction of firm-decide-ticks with
  logged `focus == SURVIVAL`; SC-3 each `g_j` crosses its bind threshold
  (`g_1 ≥ 0`, others `> 0`) or a death cause names it; SC-4 `selected_action ∈
  {6,7,8}` / all decisions; SC-5 matured `Effect.applied == true` rate; SC-6
  `H_rep = −Σ p_a log₂ p_a` over `W` per firm-tick, variance ≥ `0.01`.
  **~24 configs** (`T=400`; `θ_limit ∈ [0.40, 0.90]`, `θ_cap ∈ [0.20, 0.90]`,
  `θ_Q ∈ [6, 100]`, `L_W ∈ {4,6,8}`, `n ∈ {2,4,10,12,20}`, ± shocks):

  | SC | outcome | mechanism |
  |---|---|---|
  | SC-1 (0.60–0.90) | **reachable** (0.80 at θ_limit 0.40 / L_W 4 + knife-edge seeds) — but default no-shock outcome is **100 % survival** (satisficing SURVIVAL + inertia are highly self-preserving) |
  | SC-2 (5–25 %) | **NOT reachable — bimodal**: `0.005` (all-healthy) or `0.50–1.00` (any binding regime). `h = −max_j(g_j/s_j)` ⇒ `h < h_crit` for a wide band around *every* boundary; a firm engaging regulated production or below `θ_cap` sits permanently in that band. **SC-2 ⊥ SC-3.** |
  | SC-3 (all four bind) | **partial** — compliance/scope/obligation emerge; **solvency only from a `r^L = π^I, r^I = 0` knife-edge seed** (any producer earns positive cash) |
  | SC-4 (> 5 % shaping) | **NOT reachable under `decision.satisficing`** — **0 / ~50 000 decisions**, and (ADR-0042) **0 / 400 at every one of the 15 §16.1 `(w_max, β)` cells** including `w_max = 9, β = 0`. Shaping never satisfices any `GOAL` (one-step lookahead shows only cost) ⇒ it is only ever the scan *fallback*, and the fallback ignores `w_eff` — so the effect is **width-*independent***, not the manual's "with small `w_eff`" artefact. Reached only via a degenerate input-starved channel (`π^I > κ_ℓ`), and even then one-shot per firm (`3 / 600` = 0.5 %). **`decision.random` (Arm C): 30.5 %.** |
  | SC-5 (0.10–0.60) | **NOT reachable** — consequence of SC-4 (`decision.random`: 0.492, in range) |
  | SC-6 (non-degenerate variance) | **usually reachable** (0.01–0.13) |

  **Model defect vs. spec issue (§2.5):** none of the failures is an
  implementation bug. SC-4/SC-5's failure *is* §12.3's stated R3 behaviour;
  only SC-1 says "under no shock", so SC-4/SC-5 (and probably SC-1) may be
  intended as **Arm-B** conditions (§30.4 — the shock regime where H3 predicts
  shaping). SC-2's bimodality is a real model property (§12.3 Step 2 is a hard
  branch). **A Phase-3 decision on the SC / arm structure is required before
  the gate can be met.** `sc16_gate` locks the finding as a regression;
  `sc16_search` (ignored) reproduces the sweep.

- **Stage-6 follow-up (ADR-0042) — SC-4/SC-5 across the `w_max × β` sweep.**
  ADR-0040's SC-4/SC-5 row quoted the manual's conditional "with small
  `w_eff`" but had not swept `β` (`{0, 0.5, 1, 2, 4}`) or `w_max` (`{3, 6, 9}`)
  — §16.1's two search-width parameters, and the two that phrasing points at.
  `lobby` sits at position 4 in the `GOAL(1)` order `[2, 1, 3, 6, …]`, so a
  wide-`w_eff` firm with sustained `ς_1` *could* in principle reach it before
  the scan budget runs out. **Result: it does not, at any `(w_max, β)`.**
  A structural unit test (`validation::sc4_shaping_selected_only_as_the_goal_fallback_never_by_the_scan`)
  drives `select` across all 15 cells; a full-run probe
  (`sanity::sc4_wmax_beta_probe`) confirms **0 / 400 shaping for a firm
  engineered to force the scan deep** (`aspiration_capital_growth = 100 000`,
  `h` held well above `h_crit`) at every cell, including `w_max = 9, β = 0`.
  The reason is now stated precisely rather than via the manual's shorthand:
  (1) a shaping action **never satisfices** any `GOAL` — its one-step lookahead
  (`shaping_cost_step`) only subtracts cost, so `Δv_j < ς_j` always; (2) it is
  therefore **only ever the scan fallback**, which scans the full priority
  order **unbounded by `w_eff`**; (3) `ψ(h) = 1` for `h ≥ h_crit` for **every**
  `β` (the formula, not an approximation), so `β` is inert for a `GOAL` firm
  and there is no "small `w_eff`" to be had; (4) shaping is picked **iff every
  market action ahead of it in the `GOAL` order is inadmissible** — an
  input-starved firm that cannot afford its input, and even that degenerate
  channel (`π^I = 30 > κ_ℓ = 25`) yields only `3 / 600` (one lobby per firm,
  then permanent idle). **The "SC-1…SC-6 cannot be jointly satisfied"
  conclusion stands unchanged and is now sharpened: SC-4/SC-5 are
  width-*independent*, not small-`w_eff` artefacts.** The Phase-2 gate outcome
  does not change.

  **A substantively interesting connection (per the follow-up brief).** The
  *only* thing that would make a `decision.satisficing` firm shape is a large
  unmet goal shortfall — which is exactly what **H1a** predicts as the driver
  of *wider* search. But in this model H1a's channel (`ς → w_eff`, via focus)
  and the shaping channel are **disjoint**: a firm with a large `ς_1` gets
  `GOAL(1)` focus and a wide `w_eff`, then takes the *first admissible market
  action* in that order (`produce_regulated`) — never shaping, because shaping
  never satisfices and a market action is always ahead of it and admissible.
  Shaping is reached only when the firm has been cut off from operating
  entirely. Whether that disjunction between the BTOF search-widening
  mechanism and the RDT shaping repertoire is intended, or a gap to close
  before Phase 3, is the same spec question ADR-0040 raised.
- **ADR-0041 (discovered in Stage 6, outside VT-8/SC scope).**
  `resolve_lagged` emitted two `AdjustGlobalReal { theta_limit }` when two
  firms' matured `lobby` effects landed in one tick → `DuplicateDelta` abort
  (same class as ADR-0031/shock). Fixed: `merge_global_scalar_adjusts` sums
  same-`(Global, field)` adjusts into one delta — correct per ADR-0016
  ("simultaneous lobbying is additive"). Byte-identical for shipped configs
  (none matures two global effects in one tick). Regression test added.
- Verification: `cargo fmt --all --check` clean; `cargo clippy --workspace
  --all-targets -- -D warnings` clean; `cargo test --workspace` **210 pass, 0
  fail** (208 at the Stage-6 submission; +2 for the ADR-0042 follow-up —
  `validation::sc4_shaping_selected_only_as_the_goal_fallback_never_by_the_scan`
  and `sanity::sc4_wmax_beta_probe`); `bash scripts/lint-architecture.sh` passes;
  `cargo metadata | check_deps.py` exit 0; `cargo tree -p firma-kernel` =
  `{firma-core, firma-rng}`. **No shipped numerical output changed** — golden
  (`run_id f304edd4…`); `phase1-smoke` `14b9eb59…3f593a0` / `310f636f…`;
  `phase2-smoke` `0529c6bb…` / `9a476938…`; `phase2-stage5-smoke` `5824031c…` /
  `fd3aa4f2…` all byte-identical (the Steps-2–5 refactor and the
  `resolve_lagged` merge are both inert for existing configs).

### Stage 6b outputs — the SC-1…6 / experimental-arm scoping question

**Outcome: Path 1 resolves it. `decision.random` jointly satisfies SC-1…6,
robustly across 5 seeds, under a reading of §16.2/§27.3 that scopes SC-1…6
to no particular decision plugin. The Phase-2 gate's criterion 8 is met
under this reading — flagged for owner review before being treated as
settled, per this Stage's own instruction. Path 2 (an Arm-B-shaped
`decision.satisficing` config) was run for completeness, was not needed to
resolve the gate, and reconfirms ADR-0042's finding under an applied shock.
A `constraint.enforce` bug was found and fixed along the way. See ADR-0043.**

- **The manual text, checked first.** §16.2's SC-1…6 table names no decision
  plugin anywhere except SC-1 ("under no shock"). §27.3 criterion 8: "A
  parameter regime exists in which constraints genuinely bind... Criterion 8
  is not a software test. It is the check that the model is scientifically
  alive." §30.4's factor table lists `Decision plugin | satisficing, random |
  C` as a swept factor of the model, on equal footing with `β`, shock
  magnitude, and shaping lag. Nothing scopes SC-2…6 to `decision.satisficing`.
- **Path 1 test: `decision.random`, tuned so all four constraints
  genuinely bind.** `cfg_random_binding` — 12 firms, `T = 400`,
  `output_price = 7` / `input_price = 2` (near-zero expected capital drift
  at `c ≈ 0.5`, the same SC-1/SC-2 knife-edge ADR-0040 found, now engineered
  for the null plugin), `θ_cap = 0.45`, `θ_limit = 0.25`, `θ_Q = 10`, 3
  thin-capital low-capability firms (solvency), 2 firms seeded
  `q ∈ {90, 70} > θ_Q` (non-lethal obligation stress), `lobby` + `diversify`
  in the repertoire. One deliberate deviation from §16.1: **`t_c = 1`**
  (not `4`) — under uniform-random action choice, `u`'s draws are
  uncorrelated tick to tick, so the default `T_c` window turns almost every
  `compliance` first strike into a near-certain lethal second strike a few
  ticks later, conflating "survival" (SC-1) with "compliance binds at least
  once" (SC-3). `t_c = 1` decouples them without changing the violation
  semantics.

  | SC | target | seed 4242 | seed 1 | seed 77 | seed 900001 | seed 31337 |
  |---|---|---|---|---|---|---|
  | SC-1 | 0.60–0.90 | 0.667 | 0.667 | 0.833 | 0.833 | 0.667 |
  | SC-2 (reconstructed `h<h_crit`) | 0.05–0.25 | 0.081 | 0.056 | 0.074 | 0.050 | 0.062 |
  | SC-3 | all four | ✓ | ✓ | ✓ | ✓ | ✓ |
  | SC-4 | > 0.05 | 0.298 | 0.299 | 0.301 | 0.297 | 0.311 |
  | SC-5 | 0.10–0.60 | 0.47 | 0.49 | 0.47 | 0.48 | 0.47 |
  | SC-6 | ≥ 0.01 | 0.173 | 0.152 | 0.154 | 0.142 | 0.155 |
  | **all six** | | **PASS** | **PASS** | **PASS** | **PASS** | **PASS** |

  SC-2 is read via `sanity_from_run`'s reconstructed `h < h_crit` metric
  (`decision.random` writes no `focus`, so the `focus == SURVIVAL` proxy
  Stage 6 used for `decision.satisficing` doesn't apply) — the manual's own
  literal SC-2 text, already computed as a cross-check in `replay.rs`.
- **The reading adopted, and the caveat it does not remove.** Given the
  manual's silence on scope and §30.4's treatment of the decision plugin as
  a model parameter, this ADR reads SC-1…6/criterion 8 as satisfied by Path
  1. It explicitly does **not** claim this shows Arms A/B (§30.4 — where
  H1a/H1b/H1c and E1 actually live, both under `decision.satisficing`) will
  ever exercise SC-4/SC-5-like behaviour; ADR-0040/ADR-0042's
  `decision.satisficing`-specific finding is untouched. A §16.2 PATCH is
  recommended to state the intended scope explicitly next revision.
- **Path 2, for completeness (not required — Path 1 already resolves the
  gate).** `cfg_arm_b_satisficing` (`decision.satisficing`, a regulatory ramp
  + a sustained resource-price shock, shaping lag `(2,6)`): SC-1 (shocked)
  0.300, SC-2/3/6 (no-shock baseline `cfg_threaded`) fail as before, **SC-4
  (shocked) = 0.0000** — reconfirming ADR-0042 under an applied shock, not
  just the no-shock sweep. Not tuned further once Path 1 resolved the
  question, per the instruction not to force an outcome or over-invest once
  one path already answers it.
- **ADR-0043 Decision 2 — a `constraint.enforce` bug, found incidentally
  while building `cfg_random_binding`.** A moderate `θ_limit` (so `compliance`
  strikes sometimes) combined with seeded-high-obligation firms produced one
  agent violating both `compliance` (first strike) and `obligation` in the
  same tick — two independent, non-lethal §9.1 consequences that nothing
  says are mutually exclusive. `Enforce::apply` called
  `to_env(agent, capital, pen)` once per violated semantic instead of once
  per agent, so this combination emitted **two** same-agent
  `AdjustStock{capital}` deltas from one rule, origin `constraint.enforce` —
  exactly the same-agent double-debit shape ADR-0033's per-rule uniqueness
  guard exists to catch, and it correctly aborted the run with
  `DuplicateDelta`. Demonstrated first, before any fix, with a direct
  `Enforce::apply` unit test:
  ```
  capital deltas on agent 0: [-30, -7]
  thread '...' panicked: assertion `left == right` failed: enforce must
  emit exactly one capital-penalty delta per agent per tick ...
    left: 2
   right: 1
  ```
  **Fixed** by accumulating `capital_penalty` across both branches and
  emitting one `to_env` call per agent per tick, capped by the agent's
  starting-of-tick capital (identical cap to what each branch used alone, so
  a single-violation tick is unaffected). Re-run: `capital deltas on agent 0:
  [-37]`, `20 passed; 0 failed` (19 pre-existing + this one, none regressed).
  **Byte-identical for every shipped config** — this combination was never
  previously reachable in any smoke config or the golden trace; all four
  frozen hashes confirmed unchanged before and after.
- Verification: `cargo fmt --all --check` clean; `cargo clippy --workspace
  --all-targets -- -D warnings` clean; `cargo test --workspace` all green
  (`firma-plugin-constraint` 19→20; `firma-conformance` sanity 2→3 passed +
  1 ignored: + `sc16b_arm_scoping`); `bash scripts/lint-architecture.sh`
  passes; `cargo metadata | check_deps.py` exit 0; `cargo tree -p
  firma-kernel` = `{firma-core, firma-rng}`. **No shipped numerical output
  changed** — golden (`run_id f304edd4…`); `phase1-smoke` `14b9eb59…3f593a0`
  / `310f636f…`; `phase2-smoke` `0529c6bb…` / `9a476938…`;
  `phase2-stage5-smoke` `5824031c…` / `fd3aa4f2…` all byte-identical (the
  `constraint.enforce` merge fix is inert for every existing config; nothing
  else shipped changed).

### Stage 6c outputs — does each E1 hypothesis actually need SC-4/SC-5? (ADR-0044)

**Outcome: ADR-0043 Decision 1's unqualified "the gate is met" is superseded.
Per-hypothesis: H1a, H1b, H1c, H2, H4 are gate-clear under existing
`decision.satisficing` builds (no null-arm substitution needed). H3 is
untestable as built — a real, named, non-kill-criterion finding, not fixed
this Stage. No code changed.**

- **The challenge, precisely.** ADR-0043 rested "the gate is met" on
  `decision.random` — Arm C, 10 of 315 pre-registered E1 cells (§30.4).
  Arms A and B, 305 cells (96.8%), run `decision.satisficing` exclusively
  with no decision-plugin variation. Showing the null arm's mechanics work
  is not the same as showing criterion 8 ("the model is scientifically
  alive," §27.3) holds for the arms carrying the hypotheses.
- **Per-hypothesis analysis** (§2.4 exact text/DVs, §30.4 arms):
  - **H1a, H1b** (Arm A; DVs `search_width`, `repertoire_entropy`). Arm A's
    own method (§30.2) sets `h`/`ς` by direct intervention — the mechanism
    under test is the `select()` seam's Step 2/3 response, already verified
    structurally by VT-8 independent of which action class satisfices.
    `repertoire_entropy` (§14.2 R1) has no minimum-shaping-frequency
    requirement in its definition; SC-6 (its non-degeneracy check) is
    already shown reachable under `decision.satisficing` (ADR-0040).
    **Does not need SC-4/SC-5.**
  - **H1c** (Arm B; DV `repertoire_entropy`). Same DV, same argument — the
    inverted-U-vs-discontinuity test is over however the firm distributes
    across realised (mostly market) actions. **Does not need SC-4/SC-5.**
  - **H2** (Arm B — "Novelty" and "Shock magnitude" are both Arm-B factors,
    §30.4; DV `repertoire_entropy`). Same reasoning as H1c. **Does not need
    SC-4/SC-5.**
  - **H4** (Arm B; DV `survival_time`). Needs a genuine spread of survival
    outcomes (SC-1's concern), already shown reachable (0.60–0.90) under
    `decision.satisficing` in a threaded/knife-edge config (ADR-0040).
    **Does not need SC-4/SC-5.**
  - **H3** (Arm B; DVs `repertoire_entropy`, `survival_time`) — "Shaping
    availability reduces narrowing when shaping lag < time-to-boundary, and
    increases it when lag > time-to-boundary." This is the one hypothesis
    literally about shaping. **Needs SC-4/SC-5, and it is not met.**
    Stage 6b's own `cfg_arm_b_satisficing` — `decision.satisficing`, a
    regulatory + resource shock, shaping lag `(2,6)` (one of §30.4's three
    levels) — already showed `SC-4 = 0.0000`: zero shaping selections, in
    an Arm-B regime, at a tested lag, under a shock. ADR-0042's mechanism
    (shaping never satisfices → fallback-only → needs every market action
    ahead of it inadmissible, which a shock does not by itself force)
    explains why this is not an artefact of that one config. **Consequence,
    stated precisely: this is not "H3 is false" — there is no possible
    outcome under the current build that could support or contradict its
    predicted comparison, because the comparison's precondition (shaping
    occurring, at a rate comparable across lag levels) is structurally
    absent. H3 is untestable, not falsified.** A corroborating signal from
    §14.2: R3 ("shaping abandonment," a required rigidity metric alongside
    R1) is constant at `1.0` whenever shaping is never selected — the same
    fact through a different metric.
- **The kill criterion (§2.5), checked precisely.** Quoted exactly: "H1a and
  H1b are jointly load-bearing... Kill criterion (binding). If at the end of
  Phase 3 the platform cannot produce *either* support for H1a/H1b/H1c *or*
  an interpretable, publishable negative result, the project MUST stop..."
  **Only H1a/H1b/H1c are named — all three are gate-clear per this Stage.
  H3's untestability has no bearing on the kill criterion**; conflating a
  secondary hypothesis's testability problem with the project's
  continuation criterion is not supported by the manual text.
- **A further, narrower consequence, flagged not resolved.** §30.9's E1
  filing checklist requires a flat, unqualified "Sanity conditions SC-1…SC-6
  satisfied (§16.2)," and §30.3 currently declares H3's predicted sign as
  part of *this* pre-registration document. As long as that stays true,
  §30.9's checklist is not satisfiable by this Stage's per-hypothesis
  reading — a distinct, later gate from the Phase-2 criterion-8 question
  this Stage resolves, not addressed here. Two options, neither attempted:
  (1) a targeted model revision giving shaping a path to satisficing under
  some H3-relevant condition — needs its own ADR and an adversarial
  literature check first, per the project's standing rule on
  contribution-shaping changes; (2) an explicit, documented descope of H3
  from the initial E1 filing (§2.4/§30 PATCH), noting E3 ("the distinctive
  mechanism," §28.2) as blocked pending (1), and adding the untestability
  (and R3's resulting degeneracy) to §30.10's declared limitations.
- **No code changed this Stage** — this is a documentation/analysis
  correction. ADR-0043 Decision 1's framing is superseded by ADR-0044;
  ADR-0043 Decision 2 (the `constraint.enforce` fix) is untouched and
  stands. `docs/adr/README.md` and ADR-0043's `Status` line updated
  (metadata only, per the house rule).
- Verification (no code changed, so this reconfirms rather than re-tests):
  `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets
  -- -D warnings` clean; `cargo test --workspace` unchanged from Stage 6b
  (all green); `bash scripts/lint-architecture.sh` passes; `cargo metadata |
  check_deps.py` exit 0; `cargo tree -p firma-kernel` = `{firma-core,
  firma-rng}`. **No shipped numerical output changed** (nothing touched it)
  — golden (`run_id f304edd4…`); `phase1-smoke` `14b9eb59…3f593a0` /
  `310f636f…`; `phase2-smoke` `0529c6bb…` / `9a476938…`;
  `phase2-stage5-smoke` `5824031c…` / `fd3aa4f2…` all byte-identical.

### Stage 7 outputs — `firma-tui` and `firma_lab` (the MVP interface layer)

**Phase 2's last Stage.** Neither tool changes the Phase-2 gate status
(ADR-0044's per-hypothesis reading stands unchanged) — both are debugging/
monitoring aids, explicitly not evidence (§23.2: "Neither produces
evidence... A screenshot MUST NOT appear in a results section").

- **Part B (built first — Part A depends on it): `firma-analysis`
  (ADR-0045).** `Event` relocated `firma-kernel` → `firma-core` (it depended
  only on other `firma-core` types; `firma-kernel` re-exports it, so no
  existing call site changed) — required because `firma-tui` "MUST NOT link
  against the kernel" (§23.2) and needs `Event` to tail the log. New crate
  `firma-analysis ← firma-core, firma-domain, firma-config, firma-io`
  (never kernel, never a plugin) promotes Stage-6's `tests/src/replay.rs`
  into `Reconstruction` (an incremental, event-at-a-time fold reused by both
  a batch reader and a live tailer) + `Notable` (the single place "what does
  this raw event mean" is decided) + `sanity::sanity_from_run`
  (behaviourally identical to the original — verified against all four
  frozen hashes and the full test suite). `tests/src/replay.rs` is now a
  five-line re-export shim; no test call site changed.
- **Part A: `firma-tui`.** ratatui 0.30 + crossterm 0.29 (ADR-0046), all
  eight §23.2 panels (tick progress/ETA, agents alive/dead, margin
  distribution, action histogram, shock timeline, invariant status,
  throughput, log tail). Margin distribution reuses `Reconstruction`
  directly — the same `h`/`g_j` formulas `sc16_gate` uses, never a second
  implementation. The tailer (`tail::Tailer`) never errors on a missing
  file or a partial trailing line — both are the ordinary state of an
  unattended, in-progress run (3 unit tests). `firma-cli`'s orchestrator
  now flushes the event log every tick, not just at the end (bytes
  unchanged, only *when* they reach disk — confirmed byte-identical against
  all four hashes), so a tailer actually sees progress. **Dependency
  purity**: `cargo tree -p firma-tui -e normal` contains no `firma-kernel`
  node; `lint-architecture.sh` gained a 9th check (`no-tui-kernel-deps`)
  enforcing this in CI, not just by inspection. **Real-run demo**
  (`tests/tests/tui_live_demo.rs`): a 20,000-tick, 2-firm run executes on a
  background thread while the main thread builds a real `firma_tui::app::App`
  against the *same, still-growing* `events.ndjson` and renders via
  `ratatui::backend::TestBackend` (the "screenshot" for a non-interactive
  environment) — 12 snapshots captured over ~8.6s of genuine wall-clock
  execution, tick progression `0 → 1698 → 4106 → … → 19999 (finished)`,
  every panel populated from real reconstructed data (throughput
  `27,476.8 ticks/s`, real per-action tallies, real per-firm `h` bars). One
  known cosmetic limitation, not fixed this Stage: a firm with `h < 0`
  renders as a zero-height (invisible) bar rather than a flagged negative
  one, since `ratatui::BarChart` cannot draw negative bars.
- **Part C: `firma_lab` (Python, PyO3).** ADR-0046: PyO3 0.29.2
  (`extension-module`), maturin build backend, one venv
  (`.venv/`, gitignored — Arch's system Python is externally managed,
  PEP 668). Compiled extension `firma_lab._native` (nested inside the
  `firma_lab` package — a maturin `python-source` constraint, documented in
  ADR-0046's Note after the first naming attempt failed with a clear
  error). **`load`** (pure Python, pandas — generic NDJSON/manifest
  parsing, no FIRMA formula in it) vs. **`metrics`** (a thin PyO3 wrapper
  over `firma-analysis` — `sanity_report` and `final_margins`, zero
  Python-side formula re-derivation) — a deliberately asymmetric split,
  reasoned through in ADR-0046. `spec`, `runner`, `stats`, `sensitivity`,
  `plot`, `prereg` are docstring-only Phase-3 stubs (no callable that could
  be mistaken for working logic). `firma-py` carries no Rust unit tests
  (`extension-module`'s trade-off — a `cargo test`-linked binary can't
  resolve Python symbols at build time); correctness rests on the Python
  cross-check instead (Part D).
- **Part D: the cross-language agreement proof.** A real run
  (`configs/experiments/phase2-stage5-smoke.json`) computed two ways
  against the *same* run directory: `firma-analysis`'s own
  `examples/sanity_report_json` binary (Rust) vs. `firma_lab.metrics`
  (Python/PyO3). `python/tests/test_metrics_cross_check.py` (pytest, 3
  tests) asserts exact equality field-by-field — `sc1_survival = 0.75`,
  `sc2_survival_attention = 0.36065573770491804`,
  `sc6_entropy_mean = 0.09098579861712074`, every firm's `h`/`g_j`, etc. —
  all matched byte-for-byte on the first run, the same "two paths, one
  formula, byte-identical" standard ADR-0026 set for the
  constraint/decision-plugin agreement.
- Verification: `cargo fmt --all --check` clean; `cargo clippy --workspace
  --all-targets -- -D warnings` clean (0 issues, including the new
  `firma-analysis`/`firma-tui`/`firma-py` crates); `cargo test --workspace`
  all green (adds: `firma-analysis` unit tests, `firma-tui` 3 tail tests,
  `firma-conformance::tui_live_demo` 1 test); `bash
  scripts/lint-architecture.sh` passes (9/9, including the new
  `no-tui-kernel-deps` check); `cargo metadata | check_deps.py` exit 0;
  `cargo tree -p firma-kernel` = `{firma-core, firma-rng}` (unchanged);
  `cargo tree -p firma-tui` / `-p firma-py` both exclude `firma-kernel`.
  `python -m pytest python/tests/` — 3 passed. **No shipped numerical
  output changed** — golden (`run_id f304edd4…`); `phase1-smoke`
  `14b9eb59…3f593a0` / `310f636f…`; `phase2-smoke` `0529c6bb…` /
  `9a476938…`; `phase2-stage5-smoke` `5824031c…` / `fd3aa4f2…` all
  byte-identical (the `Event` relocation, the `replay.rs` → `firma-analysis`
  promotion, and the per-tick flush are all inert for existing configs).
- **Phase 2 closes out with this Stage** (the last one this Stage's own
  instruction named): the model, its sanity gate (ADR-0040/0042/0043/0044,
  H3 open per the flagged Phase-3 decision), and its MVP interface layer are
  all built and reported. Phase 3 (E1 execution) is next, gated on the
  owner's review of ADR-0044's per-hypothesis reading and the H3
  model-revision-vs-descope decision it names.

### H3 model revision, round 3 (ADR-0049) — same branch, still NOT merged

**Owner-confirmed genuine defect, not an acceptable approximation**: round
2's `time_to_boundary` only ever advanced cash/input/capability via
`market_core` — it never advanced the compliance action window, so `u`
stayed frozen for the whole 50-tick projection, and a firm whose only real
danger was a compliance violation was told "no danger foreseeable."

- **Part A audit, done first, before any code** (full table in the ADR):
  of `g_1`'s `r^L`, `g_2`'s `u`/`θ_limit`, `g_3`'s `c`/`θ_cap`, and `g_4`'s
  `q`/`θ_Q`, two were bugs as named (`u` frozen; `θ_limit` frozen — latent,
  only wrong when a `lobby` matures) and the audit found **a third,
  unnamed instance of the identical shape**: `q`/`θ_Q` via an already-
  pending `contract`'s maturity was also invisible. All three fixed under
  the same standard. `θ_cap` via a scheduled `Regulatory` shock and `λ`
  (read by no `g_j`) are **deliberately left frozen**, each with its own
  stated reasoning, not silence.
- **`advance_window`** (new, `firma_domain::window`) — extracted verbatim
  from `ActionWindow::apply`'s real append+trim logic; both the real
  `constrain` rule and `time_to_boundary`'s projection now call the same
  function. New explicit `initial_u: f64` parameter (not derived from
  `window.is_empty()`) so the tick-0 check matches the caller's own `h_t`
  exactly, including the `REGULATED_INTENSITY` seed fallback.
- **`apply_maturing`** (new, private, `firma_domain::dynamics`) — at every
  simulated tick, checks the firm's real `pending: &[LaggedRecord]` against
  `matures_at(tick)` and applies matured effects via
  `Effect::deltas_at_maturity` (reused, not re-derived) through a small
  `apply_projected_delta` dispatch. `time_to_boundary`'s signature grew
  from 7 to 11 arguments (`legitimacy`, `initial_u`, `window`, `l_w`,
  `pending`, `start_tick` added; `aux: &FirmAuxState` removed in favour of
  its two fields individually).
- **Two new tests, hand-verified**: `time_to_boundary_sees_compliance_
  danger_the_round_2_projection_missed` (ample cash/input/capability,
  `u` trending toward `θ_limit`; predicted `Some(2)`, got `Some(2)`) and
  `time_to_boundary_sees_an_already_pending_rescue` (below `θ_cap` now,
  `Some(0)` with nothing pending; `None` once a maturing-this-tick
  `CapabilityGain{delta:0.2}` is supplied — the maturity must apply before
  the tick-0 boundary check, and it does).
- **Full backward compatibility, verified not assumed**: all 7 pre-existing
  `time_to_boundary` tests (rounds 1/2) pass unchanged after the signature
  change (mechanical argument insertion only); all 6 pre-existing H3
  `firma-plugin-decision` tests pass with **zero test-code modification**
  after `DecideCtx` gained 4 new fields, because none of them seed
  `ACTION_WINDOW`/`LAGGED_EFFECTS`; `ActionWindow`'s 20-test suite
  unchanged in count and result after switching to `advance_window`.
- **Full regression, named test-by-test**: VT-1…VT-8 (10/10); `sc16_gate`
  (ADR-0044's per-hypothesis gate-clear finding for H1a/H1b/H1c/H2/H4
  re-confirmed), `sc4_wmax_beta_probe`, `sc16b_arm_scoping` (all pass);
  full workspace `cargo test` green (0 failures across every crate, incl.
  `firma-domain` 45, `firma-plugin-constraint` 20, `firma-plugin-decision`
  22); both VT-8 greps re-run fresh (clean); full workspace clippy `-D
  warnings` clean; `cargo fmt --all --check` clean (one formatting pass
  applied — two multi-line function signatures rewrapped, no logic
  change); `lint-architecture.sh` 9/9; `check_deps.py` clean; `cargo tree
  -p firma-kernel` unchanged (`firma-core`, `firma-rng`, `serde` only).
- **All four frozen hashes re-confirmed byte-identical**: golden
  (`run_id f304edd4…`); `phase1-smoke` (`14b9eb59…3f593a0` /
  `310f636f…a5ae4945`); `phase2-smoke` (`0529c6bb…f017` / `9a476938…eadb`,
  run via `firma run --model`); `phase2-stage5-smoke` (`5824031c…df59` /
  `fd3aa4f2…06e1`, likewise) — expected, since neither shipped smoke
  config configures `shaping`.
- **Deliberately not fixed, flagged for a future ADR**: whether a firm's
  forward projection may see a scheduled-but-not-yet-onset `Regulatory`
  shock moving `θ_cap` — reasoned in the ADR as a separate,
  Observation-architecture-adjacent design question, not a `time_to_
  boundary` completeness bug of the same shape as this round's two fixes.
- **Not done**: no merge, no E3, no further Phase. Comes back for review.
  ADR-0048 stays Accepted; its Status line now points to this ADR for
  `time_to_boundary`'s scope specifically, per the project's ADR-immutability
  house rule (correction = new ADR, never an edit).

### H3 model revision, round 2 (ADR-0048) — same branch, still NOT merged

**Owner-confirmed**: the timing comparison (H3's actual two-sided text) was
the wanted mechanism, not an optional extra — round 1 (ADR-0047) evaluated a
shaping payoff's *quality* but never whether it could arrive *in time*.

- **`time_to_boundary`** (§14.3) — new `firma_domain::dynamics` function,
  repeatedly applying `market_core` (no second copy) under the firm's
  `prev_action` (the same value/default `NONE`'s inertia fallback already
  uses) until `h ≤ 0` or a 50-tick cap (comfortably past §16.1's widest
  lag sweep, Δ_max=16; an eighth of `T=400`). 4 new unit tests: exact
  crossing, inadmissible-mid-projection, already-past, unbounded/cap.
- **The race, a real branch**: `Δ_min < time_to_boundary` ⇒ ADR-0047's
  expected-relief calc runs; `Δ_min ≥ time_to_boundary` ⇒ falls back to
  cost-only, exactly implementing H3's "increases narrowing" direction
  mechanically. `Δ_min` chosen over `Δ_max`/mean — H3 asks "can arrive,"
  an existential claim, reasoned through in the ADR.
- **Config-only toggle**: `ShapingScanParams::require_time_margin: bool`,
  **defaults to `true`** (time-aware is now the standard; justified in the
  ADR — costs nothing for configs that don't set `lobby_success`/
  `contract_success`, which still default to `None`). `false` reverts
  exactly to ADR-0047's payoff-only behaviour — genuinely config-only, no
  code change, demonstrated by a passing test.
- **Satisficing/optimising re-confirmed**: `select()` untouched; the race
  is a threshold gate in front of a threshold test for one action at a
  time, never a cross-option comparison.
- **The actual race demonstrated** (`firma-plugin-decision` tests, a
  solvency-eroding scenario with `time_to_boundary=10`): short lag (min=2)
  wins, lobby satisfices; long lag (min=12) loses, falls back to
  `produce_ordinary` (not lobby); toggle off + long lag ⇒ lobby satisfices
  again despite the timing. All three pass.
- **Round 1's own finding, sharpened not contradicted**: "lag doesn't
  matter" was true specifically because that scenario's `time_to_boundary`
  is unbounded (compliance-only bind, `u` held fixed) — verified the
  original test still passes unchanged under the new default, and the new
  finite-`time_to_boundary` scenario shows lag deciding the outcome when it
  actually can.
- **Full regression**: VT-1…VT-8 (10/10), `sc16_gate`/`sc4_wmax_beta_probe`/
  `sc16b_arm_scoping` (all pass, unaffected), both VT-8 greps re-run fresh
  (still empty), full workspace `cargo test` green, clippy/fmt clean,
  `lint-architecture.sh` 9/9, all four frozen hashes byte-identical
  (re-confirmed — neither shipped smoke config sets `shaping` at all).
- **Adversarial-literature-check still applies and is still not done** — no
  contribution claim about H3 anywhere here or in the ADR.
- **Not done**: no merge, no E3, no further Phase. Comes back for review.

### H3 model revision, round 1 (ADR-0047) — branch `h3-satisficing-lookahead`, NOT merged

**Owner-directed** (ADR-0044 named this as one of two options for H3;
descope was the other — the owner chose to attempt the revision, with the
risk stated up front). **Retractable:** tag `pre-h3-revision` marks the
exact commit this branch forked from; nothing has been committed on this
branch (see the working agreement — commits/pushes are the owner's alone,
including on a dedicated branch); `git checkout main` (or `master`) plus
discarding this branch's uncommitted working-tree changes fully reverts.

- **The mechanism** (full reasoning in the ADR): `satisfices()`'s
  `SURVIVAL` branch only, for `lobby`/`contract` specifically, now computes
  `E[h_{t+1}]` as a proper expectation over the action's declared
  `p_success` (reusing `firma_domain::shaping::SuccessModel::p_success` —
  the *same* function `action.shaping.rdt_standard` calls at commitment,
  same `legitimacy` input) and its declared payoff, instead of a cost-only
  lookahead. `GOAL(j)` and `diversify` are unchanged (their payoffs don't
  move `v_1`/`v_2`/`v_3` — reasoned through, not assumed, in the ADR). Two
  new config fields (`ShapingScanParams::lobby_success` /
  `contract_success`), both `Option<...>` defaulting to `None` — additive,
  opt-in, behaviour-preserving for every config that predates this ADR.
  **No discount by lag** — a deliberate choice (reasoning in the ADR: no
  manual anchor for a discount rate; matches the existing `invest_capability`
  §9.3 lag-collapse precedent, also undiscounted).
- **VT-8 criterion (iii) re-checked, not assumed**: both ADR-0040 directional
  greps re-run against current code — still empty/unchanged. The new
  `shaping_expected_survival_margin` method itself greps clean for
  `aspiration`/`shortfall`/`sc[` and is called only from the `SURVIVAL`
  match arm — confirmed, not asserted.
- **The channel opens, and responds to the action's actual economics — not
  just "reachable somehow now"**: three new structural unit tests in
  `firma-plugin-decision` (a compliance-bound `SURVIVAL` firm) —
  `lobby` satisfices when its expected relief clears `h_t` (verified
  `w_eff=8`, genuinely scanned, not the fallback); does **not** satisfice
  when its payoff is worthless (`δ_θ=0`, falls back to `produce_ordinary`);
  and — reported honestly, not glossed over — the decision does **not**
  vary with lag length (`(1,1)` vs `(8,16)` select identically), because
  the mechanism deliberately doesn't discount by lag. H3's lag-vs-
  time-to-boundary comparison is therefore an emergent, downstream property
  (does the firm survive to reap a matured lagged effect — the existing,
  unchanged `resolve_lagged` machinery), measured across config-level lag
  settings (§30.4's own swept-factor design), not built into one decision.
  A fourth test confirms the channel opens in a real, kernel-executed,
  single-tick run (`sanity.rs::h3_channel_opens_in_a_real_run`,
  `1/1` decisions were shaping).
- **Full regression, named test-by-test**: VT-1…VT-8 (10/10 pass, including
  `vt8_orthogonal_manipulation_of_h_and_shortfall` and
  `sc4_shaping_selected_only_as_the_goal_fallback_never_by_the_scan`); the
  `sc16_gate`/`sc4_wmax_beta_probe`/`sc16b_arm_scoping` sanity harness (all
  pass, ADR-0042's regression-locked `SC-4 == 0.0` assertions for
  `decision.satisficing` **still hold** — none of those configs opt into
  the new fields); full workspace `cargo test` green (0 failures);
  `cargo clippy --workspace --all-targets -- -D warnings` clean;
  `cargo fmt --all --check` clean; `lint-architecture.sh` 9/9;
  `check_deps.py` exit 0; `cargo tree -p firma-kernel` unchanged.
- **Hashes — confirmed empirically, not assumed**: golden and
  `phase1-smoke` byte-identical (expected — testkit-only, never touch
  `decision.satisficing`). **`phase2-smoke` and `phase2-stage5-smoke` are
  ALSO byte-identical** — checked directly: neither config sets a
  `shaping` key in `decision.satisficing.params` at all (shaping isn't
  even in either firm's repertoire), so the new opt-in fields cannot
  matter regardless. **No golden-trace regeneration and no version bump
  are needed** — §20.5's MAJOR-bump trigger ("anything altering numerical
  output for an existing config") did not fire for any existing config.
- **The adversarial-literature-check house rule applies to any
  *contribution claim*** about H3 now being testable or reachable — not
  yet done. Nothing in this section, the ADR, or the tests above should be
  read as "H3 is confirmed," "H3 is supported," or even "H3 is reliably
  testable in practice." What is established: the code compiles, is
  regression-clean, and a hand-constructed scenario demonstrates the
  mechanism behaves as designed. Whether this mechanism is a *good* model
  of bounded-rational shaping evaluation, and whether it survives contact
  with a real multi-config E1/E3 sweep, are separate, larger questions this
  Stage does not answer.
- **Not done, per the task's explicit stop instruction**: no merge to
  `main`/`master`, no E3 drafting, no Phase-3 work. This comes back for
  review first.

### Phase 1 gate checklist (§26.3, §27.3)

- [x] `firma-core` — types, ids, `Delta`, `View`, total ordering (§18.2)
- [x] `firma-rng` — Philox-4×32-10 + hierarchical keys + 4 streams (§21.2–21.3); Example E fixture (§15.5); Philox KATs
- [x] `firma-kernel` — state store, 9-phase scheduler, reconciler, invariant enforcer, snapshot/restore/fork (§19)
- [x] `firma-io` — event log, snapshots, manifest (§22) — NDJSON per ADR 0012
- [x] `firma-config` — schema, `deny_unknown_fields`, content hash, engine-version check
- [x] `firma-registry` — compile-time plugin registration + resolution (unknown / version / hash / params rejections)
- [x] `firma-viability` — generic backward-iteration solver for VT-2 (ADR 0011)
- [x] `firma-cli` — `run`, `replay`, `verify`
- [x] test plugins — `testkit.transfer` (moves one integer, §26.3), `testkit.keyed_nudge` (DT-6 stream isolation), `testkit.keyed_per_agent` (DT-2b, per-agent RNG key path / §21.2 property 1), `testkit.force_adjust` (abort-atomicity fixture, §19.5), `conflict.additive`
- [x] CI — `.github/workflows/ci.yml`: format, clippy `-D warnings`, `scripts/lint-architecture.sh` (§25.6), cargo-deny, test, determinism, golden trace, macOS determinism
- [x] **DT-1 … DT-6 green** (§25.2) — `tests/tests/determinism.rs`, plus kernel-level `crates/firma-kernel/src/tests.rs`
- [x] **VT-2 pass** (§25.4) — `tests/tests/validation.rs`, sizes `[25, 21, 19, 19]` = §15.2
- [x] **VT-6 pass** (§25.4) — conservation across 10,000 ticks, exact
- [x] 10,000-tick run with test plugins is byte-reproducible (seq == parallel) and conserves exactly

### Gate results (see build report for pasted output)

```
determinism.rs  8/8   DT-1 byte-identical log · DT-2 agent-order · DT-2b per-agent keyed order
                      DT-3 seq==parallel · DT-4 null-fork==continuation · DT-5 snapshot/restore
                      DT-6 stream isolation · tick-atomic-on-abort (§19.5)
validation.rs   2/2   VT-2 monotone→fixed point (+ VT-1 |Viab|=19, vol 0.76) · VT-6 conservation @ 10k ticks
golden.rs       1/1   event-log SHA-256 + run-id frozen
integration.rs  3/3   manifest assumptions non-empty · run-id stable · run+verify e2e
workspace       55/55  all crates  (48 Phase-1 gate + 3 ADR-0013 + 4 final hardening pass)
```

Post-gate hardening: **ADR-0013** (`PluginId::numeric()` uniqueness guard in
`firma-registry`); **final pass** (Threads A/B) — per-agent RNG key path made
reachable + tested (`testkit.keyed_per_agent`, DT-2b), event-log
abort-atomicity confirmed already-correct + tested (`testkit.force_adjust`).
No manual change, no numerical output change.

Not run locally: `cargo-deny` (binary not installed in this environment; `deny.toml` present, CI job wired).

### Observed, not fixed (for owner review)

- **run_id is not stable under agent-list reordering.** `RunConfig::normalise`
  sorts `world.resources` but not `agents`. Two configs that differ only in
  agent listing order produce **byte-identical event logs** (DT-2, DT-2b
  confirm this) but **different `run_id`s**, because the resolved config —
  which the run identity hashes (§22.3) — preserves the list order. This
  violates no MUST (§22.3 requires *same manifest → same log*, not the
  converse; §22.4 R1 regeneration still works per-manifest). Fixing it would
  be a one-line addition to `normalise` (`self.agents.sort_by_key(|a| a.id)`),
  but it changes `run_id` output for any config with unsorted agents, so it
  needs an explicit decision. No config in the repo currently has unsorted
  agents.

---

## Deliberately deferred to Phase 2+ (not oversights)

| Item | Manual | Why deferred |
|---|---|---|
| Domain plugin crates (`firma-plugin-locality/resource/constraint/decision/action-*/shock/observation`) | §38, §20.3 | §26.3: "No domain logic whatsoever." Directory markers left under `crates/firma-plugins/` (README only); crates land in Phase 2 (§26.4). |
| `firma-viability` domain surface — `margin(state, params, constraints)`, the four constraints, the FIRMA `Dynamics` | §9.2, §18.2, §26.4 | Domain modelling. Phase-1 crate is the generic solver only (ADR 0011). |
| `firma-tui` (ratatui monitor) | §23.2, §27.2 | MVP interface, Phase 2 deliverable (§26.4). Directory marker only. |
| `firma-py` (PyO3 bindings), `python/firma_lab/` | §23.1 | Orchestration/analysis, Phase 2+. `python/firma_lab/` has a placeholder README. |
| Arrow/Parquet event log | §22.1 | ADR 0012 — NDJSON for Phase 1; migration is a Phase-2 task. |
| `proptest` | §25.1, §25.3 | ADR 0010 — Phase 1 property checks use deterministic in-tree RNG; Phase-1 gate needs no property tests. |
| `clap` | — | ADR 0010 — `firma-cli` hand-rolls parsing for 3 subcommands. |
| VT-8; AT-1..5; SC-1..6 | §25.4, §25.5, §16.2 | Phase 2 gate (§26.4), not Phase 1. VT-1/3 done Stage 1, VT-7 Stage 2, VT-4/5 Stage 3. VT-8 needs the full Arm-A grid (Stage 5). |
| Golden traces for domain configs | §25.7 | Phase 1 commits a golden trace for the *test-plugin* reference run only. |
| Container digest, engine git SHA in manifest | §22.3 | No container here and the repo may not be a git checkout in every environment; manifest records `null` with a note and fills them when available. See OQ-4. |

---

## Open questions for review

Minor choices the manual does not fully pin down. Structural ones became ADRs
(0010–0012); these are logged here per the build instructions.

**OQ-1 — RNG key field encoding.** §15.5 (Example E) shows the key input as a
concatenation with mixed, illustrative field widths (`run_seed` as 32-bit, `tick`
and `agent_id` as 32-bit) and does not give an expected draw value — only four
*properties* the fixture must satisfy. Phase-1 choice: fixed-width **big-endian**
encoding of every field (`run_seed` u64, `stream_id` u8, `plugin_id` u64,
`phase_id` u8, `tick` u64, `agent_id` u64, then `u64` length prefix + UTF-8 bytes
of `purpose_tag`), hashed with SHA-256; first 8 bytes → Philox key, next 16 →
initial counter. This is injective in every field, which is all §15.5 properties
1–4 require. **If the manual later pins exact widths or a reference draw, this
changes and gets a MAJOR bump + golden-trace update.** Candidate for its own ADR
if you want it pinned harder now.

**OQ-2 — Config format.** Manual §20.4 shows YAML. Phase-1 config is **JSON**
(`serde_json`, `deny_unknown_fields`). Reason: `serde_yaml` is unmaintained;
content-hashing wants one canonical encoder; the Phase-1 config is tiny. YAML
support (via a maintained crate or a YAML→JSON front-end) is a Phase-2 decision
with its own ADR if adopted.

**OQ-3 — Event log format.** ADR 0012. Flagging here too because it is the item
most likely to matter at sweep scale.

**OQ-4 — Manifest provenance fields.** §22.3 lists "engine git SHA and build
hash" and "container digest". Phase-1 manifest records: workspace version,
`Cargo.lock` SHA-256, `rustc -vV` string, target triple, OS. Git SHA and
container digest are `null` when unavailable, with a `provenance_note` field
explaining. Wire up properly when the CI image is fixed.

**OQ-5 — `no-magic-numbers` automation.** §25.6 lists it as a lint; there is no
good language-agnostic automatic check. CI runs a grep *heuristic* that warns
(does not fail). Binding enforcement is the §24.6 review checklist. If you want it
to fail the build, we need a `dylint`/`clippy` custom lint — a Phase-2 tooling
task.

**OQ-6 — `tests/` package layout.** §38 shows `tests/{determinism,validation,integration}/`
as bare directories. Implemented as a workspace member package named
`firma-conformance` rooted at `tests/`, with `tests/determinism.rs`,
`tests/validation.rs`, `tests/integration.rs`, `tests/golden.rs` as test targets
and `src/lib.rs` for shared harness code. Same intent, Cargo-shaped.

**OQ-7 — `ConflictResolver` default location.** Resolvers are plugins (§19.4,
§20.3). Phase 1 ships one `additive` resolver, and it lives in the test-plugin
crate, not the kernel (A1). If a "no-op/identity" resolver is wanted as a kernel
built-in for the degenerate single-delta case, that is a small ADR.

**OQ-8 — Phase count vs. §26.3 deliverable list.** `firma-viability` is present
in Phase 1 though §26.3's list omits it (ADR 0011 explains). Noted so the
workspace/manual comparison is not surprising.

**OQ-9 — `--model` registry selection on the CLI (Stage 4).** `firma run` now
takes a `--model` flag choosing `firma_cli::model_registry()` (the full Phase-2
model) over the Phase-1 `standard_registry()`. `replay` / `verify` still
hard-wire `standard_registry()`, so they cannot yet re-check a `--model` run
end to end (the `phase2_smoke` conformance test does the equivalent check
against `model_registry()` directly). A cleaner design records the registry
identity in the manifest and reconstructs it on replay; deferred. Minor,
logged rather than ADR'd.

**OQ-10 — Manual §19.4 / §19.5 delta-uniqueness wording vs. ADR-0031 (as
narrowed by ADR-0033).** ADR-0031/0033 carve **`Environment`-targeted**
`ResourcePool` deltas out of the per-rule "one delta per (target, kind) per
phase" guard (a single rule acting for `N` agents legitimately makes `N` claims
on the shared pool cell, whose `sort_key()` is a sentinel; the §19.4 step-4
resolver aggregates them, and the stable step-2 sort keeps ascending-`AgentId`
order). Agent-targeted `ResourcePool` deltas and every `Independent` delta stay
under the guard. The manual states the guard unconditionally in §19.4 step 2
and the §19.5 invariant table. This needs a PATCH-level clarification. Per
CLAUDE.md the manual is authoritative; flagging the discrepancy rather than
silently following the narrower reading.

**OQ-11 — `time_to_boundary` does not see a scheduled-but-not-yet-onset
regulatory shock (ADR-0048/0049).** `time_to_boundary` (ADR-0048) does not
account for a scheduled but not-yet-onset regulatory shock moving
`θ_cap`/`θ_limit` during the projection window. A firm approaching a
boundary that a known future policy change would worsen is not currently
detected. Deliberately out of scope for ADR-0049 — would require the
projection to read `shock.scheduled`'s future schedule, a cross-plugin
architecture question not yet addressed. Tracked here so it is not lost;
not yet scheduled for a fix.

**OQ-12 — H3 interim disposition: shaping's scan-order deprioritization,
ADR-0050 (Accepted).** Four rounds of read-only
diagnostic tracing on `cfg_arm_b_satisficing` and `cfg_wide_search` (this
working session, after ADR-0049) found that `decision.satisficing`'s
fixed-order scan reaches lobby's checklist position in only 75/1484
decisions (5.1%) in the realistic Arm-B-shaped scenario — the other 94.9%
never get there because higher-priority market actions exhaust the scan's
search-width budget first. Of the 75 reaches, the ADR-0048 timing gate
closed in all 67 `SURVIVAL`-focus evaluations, with `time_to_boundary`
locked at exactly 1 tick (vs. `lag_min = 2`) in every one — confirmed via
lag-range and shock-timing shift variants to be scenario arithmetic
(`time_to_boundary`'s real inputs do not depend on the shaping lag range at
all), not a computational defect in `time_to_boundary`. The mechanism does
work when reached under other conditions: `cfg_wide_search`'s 4/400
non-zero cell shows the gate opening and the payoff comparison winning on
real numbers, and window-widened variants of `cfg_arm_b_satisficing` show
the same. ADR-0050 consolidates this, names two model-vs-reality
simplifications (no standing political/relational capital; single
action per tick forces shaping to compete one-for-one against market
actions) as plausible, unverified contributors, and states explicitly:
**H3 remains open — not descoped, not falsified, not found unviable**,
per the owner's standing three-condition standard for setting a hypothesis
aside. **The underlying raw trace output (NDJSON, ~1.2 MB per round) and
the temporary tracing instrumentation it depends on
(`firma-plugin-decision/src/lib.rs`, uncommitted) currently exist only in
this session's scratchpad directory and this conversation's transcript —
not committed anywhere durable. Flagged for the owner to decide on
preservation** (e.g. committing a cleaned-up trace module and a
representative sample of raw output alongside ADR-0050, or archiving the
transcript separately) before it is lost — **still unresolved**: the trace
module, the two representative NDJSON evidence files under
`docs/adr/evidence/adr-0050/`, and this ADR are prepared and staged in the
working tree, but per the standing no-self-commit rule, actually committing
them remains the owner's own action, not yet taken. See ADR-0050 for the
full account, real numbers, and citations.

**OQ-13 — E1 filing-scope decision, ADR-0051 (Accepted).** Executes
ADR-0044's named-but-unattempted path 2 ("an explicit, documented descope
of H3 from the initial E1 filing") narrowly, as a **filing-scope** decision
only: file E1 (§30) now for H1a/H1b/H1c/H2/H4, excluding Arm B's "Shaping
lag" factor (levels `(1,1)`, `(2,6)`, `(8,16)`) and H3/E3's own predicted
comparison from this filing's registered design. Computed directly from
the current §30.4 table: Arm B shrinks from 180 to 60 cells (`β`(5) ×
Novelty(3) × Shock magnitude(4), dropping the lag(3) factor only H3's own
§30.7 analysis formula reads); total 315 → 195 cells, 63,000 → 39,000 runs
at 200 seeds/cell; Arm C confirmed unaffected (its own factor table
carries no shaping-lag row, and ADR-0044's per-hypothesis table maps H3 to
Arm B only). **Explicitly not a descoping of H3 as a hypothesis** —
ADR-0050's disposition (open, not descoped, not falsified, not found
unviable) is unmodified; the ADR states this repeatedly, by design, so it
survives being read or quoted out of context. Also explicit: SC-4/SC-5
remain unsatisfied under `decision.satisficing` full stop — this ADR
changes what is being filed, not whether those sanity conditions hold. A
standing anti-tuning constraint is imposed: no future re-inclusion of
H3/E3 via threshold/cost/success-model tuning — only a literature-checked
model revision or one of ADR-0050's three named revisit triggers.

**Points 2 and 5 — resolved by explicit owner decision this round, not
left open:** (1) **§30.9's checklist bullet** ("Sanity conditions SC-1…
SC-6 satisfied") is read as **scoped to the design actually registered and
filed** — under this reading the narrowed E1 honestly clears it as
written, with no manual edit required first; SC-4/SC-5 remaining
genuinely unsatisfied under `decision.satisficing` is unaffected by this
reading (Point 2, ADR-0051). (2) **§30.3/§30.8** retain H3, annotated
"excluded from this filing's registered design; see ADR-0051," rather than
removed (Point 5, ADR-0051 — the recommendation adopted without
modification). Neither decision is applied to `docs/MANUAL.md` itself —
both are logged as manual-PATCH candidates below, per this project's
convention (code/ADRs proceed under the ADR's documented reading; the
manual is patched later in a batch — the Stage-0 precedent above).

A manual-internal inconsistency was surfaced while confirming the cell
counts (still not fixed, and not part of this round's Point 2/5
resolutions): §28.2's own experiment table names "E1" as testing only
H1a/H1b/H1c, with H2/H3/H4 as separate experiments E2/E3/E4, while §30's
actual pre-registration document bundles all six hypotheses and Arm B's
shared factors under one "E1" filing — still flagged as a candidate for
its own manual PATCH, still requiring an owner decision. This ADR runs
nothing, revises no model, and touches no code/config/test file. See
ADR-0051 for the full account.

**Manual PATCH items queued for the next revision (ADR-0051 Points 2 and
5, owner-decided, not yet applied to `docs/MANUAL.md` — Stage-0
precedent above):**

- **§30.9** — reword the "Sanity conditions SC-1…SC-6 satisfied (§16.2)"
  bullet to state explicitly that it applies to the hypotheses/factors
  actually registered in a given filing, e.g.: *"Sanity conditions
  SC-1…SC-6 satisfied for every hypothesis and Arm actually registered in
  this filing (§16.2). A hypothesis or design factor explicitly excluded
  from a given filing's registered design (see e.g. ADR-0051) is not
  required to clear a sanity condition it would otherwise need."* Purpose:
  so a later, separate H3/E3 filing does not need this same
  checklist-scoping question re-decided (ADR-0051 Point 6's re-filing
  path already assumes this reading).
- **§30.3** — after "H1a, H1b, H1c, H2, H3, H4 exactly as §2.4, with
  predicted signs," add: *"H3 is excluded from [this filing]'s registered
  design (Arm B's shaping-lag factor not included) — see ADR-0051. It
  remains an open hypothesis (ADR-0050), to be filed separately."*
- **§30.8** — annotate the falsification table's H3 row ("No interaction
  between shaping lag and time-to-boundary") with a footnote: *"excluded
  from [this filing]'s registered design; see ADR-0051."*

`[this filing]` above is a placeholder for whatever this project's actual
filed-document naming convention turns out to be once the narrowed E1 is
built (Phase 3 work, not this ADR) — left unresolved deliberately, since
naming it precisely is a Phase 3 build decision, not a Phase 2/3-boundary
scope decision.
