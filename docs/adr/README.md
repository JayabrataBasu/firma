# Architecture Decision Records — 0010 onward

ADRs 0001–0009 are consolidated into the manual (§34.1–§34.9). New ADRs live
here as separate files and are folded into the §34.0 index at the next manual
version bump (manual §38).

## The immutability rule (house rule, decided Phase 2 Stage 3)

**An accepted ADR's body is append-only. It is never edited in place — not to
fix a typo, not to correct a mistake it contains, not even within the same
session and not even before the owner has reviewed it.** The single exception
is the `Status` line, which may be updated to point at a superseding ADR
(status transitions are metadata, not content).

A correction — of any size — is **always a new, numbered ADR** that names the
earlier ADR and Decision it supersedes (fully or in part), states plainly what
the earlier one got wrong and how it was found, and (if the fix was already
coded, e.g. in a review round) says so. The point of immutability is that
"ADR-0022 Decision 1" stays a stable citable reference forever: in later ADRs,
in code comments, in manual PATCH notes, and eventually in run manifests
recording which decisions governed a run. A "not really immutable until
reviewed" carve-out has no clean boundary and has already produced drift in
this project (ADR-0021's dependency-trace note vs. ADR-0022's in-place Decision
1–3 edit in the Stage-2 review, since reverted — see ADR-0024).

An ADR is REQUIRED for (manual §34.0): any deviation from a SHOULD; a new
primitive (§7.2); a new `Delta` variant; a new dependency category; any MSRV or
toolchain change; enabling an optional theory plugin (§20.6); any MAJOR manual
bump.

An ADR is REQUIRED for (manual §34.0): any deviation from a SHOULD; a new
primitive (§7.2); a new `Delta` variant; a new dependency category; any MSRV or
toolchain change; enabling an optional theory plugin (§20.6); any MAJOR manual
bump.

| ID | Title | Status |
|---|---|---|
| [0010](0010-tooling-and-dependencies.md) | Phase 1 tooling, lint infrastructure, and dependency set | Accepted |
| [0011](0011-viability-solver-in-phase-1.md) | Include the `firma-viability` backward-iteration solver in Phase 1 | Accepted |
| [0012](0012-phase-1-event-log-format.md) | Phase 1 event-log and snapshot format: deterministic NDJSON | Accepted |
| [0013](0013-plugin-id-numeric-uniqueness.md) | Enforce `PluginId::numeric()` uniqueness at rule registration | Accepted |
| [0014](0014-u-trailing-window-definition.md) | `u` (regulated-activity intensity) is a simple trailing-window mean | Accepted |
| [0015](0015-theta-globality.md) | Constraint parameters θ are global for the Phase 2 MVP | Accepted |
| [0016](0016-simultaneous-lobbying-additive.md) | Simultaneous lobbying is additive; shaping deltas are `Independent` class | Accepted |
| [0017](0017-death-and-relation-edges.md) | At death, all incident relation edges vanish; cascade realism is a Phase 3 AT question | Accepted |
| [0018](0018-capability-no-decay-mvp.md) | Capability `c` has no decay in the Phase 2 MVP; late-game dominance is an SC-6 watch item | Accepted |
| [0019](0019-locality-resource-in-phase-2.md) | `firma-plugin-locality` and `firma-plugin-resource` are in Phase 2 scope | Accepted |
| [0020](0020-firma-domain-crate.md) | Shared §8 state/parameter types live in a new `firma-domain` crate, not `firma-core` | Accepted |
| [0021](0021-constraint-interface-and-deterministic-core.md) | The `Constraint` plugin interface, and where §11's deterministic core lives | Accepted |
| [0022](0022-decide-act-handoff.md) | The decide→act hand-off, and the eight `DeltaKind` variants Phase 2 adds | Accepted; Decisions 1–3 partially superseded by [0024](0024-setagentint-supersedes-0022.md) |
| [0023](0023-lagged-effect-queue.md) | The `Λ` lagged-effect queue, `Effect`, shaping-success timing, and the shaping RNG purpose tags | Accepted; parameter-default note superseded by [0025](0025-shaping-b-coefficients-required.md) |
| [0024](0024-setagentint-supersedes-0022.md) | Generalise `SetSelectedAction` to `SetAgentInt`; enforce `Delta`'s non-`NaN` invariant — supersedes ADR-0022 Decisions 1–3 | Accepted |
| [0025](0025-shaping-b-coefficients-required.md) | `b_λ` / `b_κ` are required shaping config, not serde-defaulted — supersedes ADR-0023's parameter-default note | Accepted |
| [0026](0026-standard-margin-in-firma-domain.md) | The four §9.1 `g_j` formulas and the standard-four margin move to `firma-domain::margin` | Accepted |
| [0027](0027-decision-random-design.md) | `decision.random`'s selection mechanism (§20.3 category, no manual formula) | Accepted |
| [0028](0028-constrain-phase-and-action-window.md) | The `constrain` phase writes nothing to θ; it maintains the action window `W`, from which `u` is derived on demand | Accepted |
| [0029](0029-enforce-phase-violation-processing.md) | The `enforce` phase: violation detection/processing, `RemoveAgent` + `ReplaceGlobalList` deltas, death-stock transfer, `T_c` inclusive | Accepted; Decision 2's ordering rationale superseded by [0032](0032-removeagent-applied-last-in-a-phase.md) |
| [0030](0030-config-seeds-domain-state.md) | A run config can seed global and per-agent domain-state values (θ, prices, capability, aspirations) | Accepted |
| [0031](0031-resourcepool-deltas-exempt-from-uniqueness-guard.md) | `ResourcePool` deltas are exempt from the per-rule delta-uniqueness guard (the resolver aggregates them) | Accepted; exemption narrowed to the `Environment` target by [0033](0033-resourcepool-uniqueness-exemption-narrowed-to-environment.md) |
| [0032](0032-removeagent-applied-last-in-a-phase.md) | `RemoveAgent` is applied in a final sub-pass of a phase, after every value delta — supersedes ADR-0029 Decision 2's ordering claim | Accepted |
| [0033](0033-resourcepool-uniqueness-exemption-narrowed-to-environment.md) | The `ResourcePool` uniqueness-guard exemption is narrowed to `Environment`-targeted deltas only — agent-targeted pooled deltas are back under the guard | Accepted |
| [0034](0034-rule-rng-stream-and-keyedrng-normal.md) | `Rule::rng_stream()` (a rule's §21.3 stream is not always its phase's default) + `KeyedRng::next_normal` | Accepted; Alternatives extended by [0039](0039-rng-stream-registration-metadata-alternative.md) |
| [0035](0035-observation-interface.md) | The `Observation` interface: a phase-2 per-agent env/θ snapshot `decide` reads, byte-identical fallback to the true store | Accepted |
| [0036](0036-shock-channels-ramp-persistence.md) | Shock: incremental channel deltas, `[D]` ramp/persistence formulas, one shock plugin per config, novelty stays offline | Accepted |
| [0037](0037-locality-interface.md) | The `Locality` interface: a query service (not a `Rule`), distinct from `G_t`, with no MVP consumer | Accepted |
| [0038](0038-resource-dynamics.md) | `resource.constant` (explicit null) and `resource.patchy` (mean-reverting `π^I`; the regen-pool reading is deferred) | Accepted |
| [0039](0039-rng-stream-registration-metadata-alternative.md) | The registration-metadata alternative to `Rule::rng_stream()`, considered and rejected — extends ADR-0034's Alternatives | Accepted |
| [0040](0040-vt8-orthogonal-manipulation.md) | VT-8: §12.3 Steps 2–5 extracted to a pure `select` function (the `h`/`ς` seam); the two directional greps; and the SC-1…6 "not jointly satisfiable" finding | Accepted; SC-4/5 row sharpened by [0042](0042-sc4-shaping-unreachable-across-wmax-beta.md) |
| [0041](0041-resolve-lagged-global-scalar-merge.md) | `resolve_lagged` sums multiple firms' simultaneous global-scalar shaping effects into one delta (ADR-0016 additivity; discovered in Stage 6) | Accepted |
| [0042](0042-sc4-shaping-unreachable-across-wmax-beta.md) | SC-4/SC-5: shaping stays unreachable under `decision.satisficing` across the full §16.1 `β × w_max` sweep — width-independent, because shaping never satisfices (fallback-only) | Accepted |
| [0043](0043-sc16-arm-scoping-and-enforce-simultaneous-penalty-merge.md) | SC-1…6 is unscoped to a decision plugin (§16.2/§27.3): `decision.random` jointly satisfies all six, robust across 5 seeds; plus a `constraint.enforce` simultaneous compliance+obligation penalty merge fix | Accepted; Decision 1's gate-is-met framing superseded by [0044](0044-sc4-sc5-gate-readiness-is-per-hypothesis-h3-untestable.md); Decision 2 stands |
| [0044](0044-sc4-sc5-gate-readiness-is-per-hypothesis-h3-untestable.md) | SC-4/SC-5 gate-readiness is per-hypothesis: H1a/H1b/H1c/H2/H4 are gate-clear (DVs never need shaping); H3 is untestable as built (shaping never selected in any arm, any lag) — not a kill-criterion trigger, flagged for a §2.4/§30 PATCH | Accepted — flagged for owner review (research-design decision) |
| [0045](0045-firma-analysis-and-event-relocation.md) | `Event` moves from `firma-kernel` to `firma-core` (structurally kernel-free); new `firma-analysis` crate promotes `replay.rs`'s log reconstruction for `firma-tui`/`firma-py`/tests to share, one formula each | Accepted |
| [0046](0046-firma-tui-and-firma-lab-toolchain.md) | `firma-tui` (ratatui 0.30/crossterm 0.29) and `firma_lab` (PyO3 0.29/maturin) toolchain choices: version pins, `firma_lab._native` module layout, `load` (pure Python) vs `metrics` (PyO3 wrapper) split, docstring-only Phase-3 stubs | Accepted |

Template (from the shape of manual §34.1–§34.9): **Context** (the forces and the
problem) · **Decision** (what, precisely) · **Alternatives** (each with why
rejected) · **Consequences** (Positive / Negative-accepted / Neutral) ·
**Compliance** (how it is enforced — lint, test, gate) · **Note** (the thing a
reader in five years will want said out loud).
