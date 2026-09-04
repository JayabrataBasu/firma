# ADR 0019 — `firma-plugin-locality` and `firma-plugin-resource` are in Phase 2 scope

**Status:** Accepted (2026-09-03)
**Phase:** 2 (Model), Stage 0 (scope decision only — implementation is Stage 4)
**Relates to:** manual §26.4 (Phase 2 deliverables), §27.1 (MVP definition),
§27.2 (MVP contents), §20.3 (plugin categories), §7.6 (Space / `Locality`
interface), §7.2 primitive 3 (Resource), §34.7 (non-spatial by default),
ADR 0011 (same reasoning shape), ADR 0007

## Context

§26.4's Phase 2 **deliverables paragraph** enumerates: `firma-viability`; the
four constraint plugins; `decision.satisficing` / `decision.random`;
`action.market.standard`; `action.shaping.rdt_standard`; `shock.scheduled` /
`shock.stochastic`; `observation.{full,noisy,delayed}`; `firma-tui`;
`firma_lab`. It **does not mention** `Locality` or `Resource` plugins.

But three other binding parts of the manual require them for the MVP:

- **§27.2 MVP-contents table** lists, as MVP dimensions:
  "Locality | `wellmixed`, `network`" and "Resources | 3 types, integer,
  conserved" (resource *dynamics* being the `Resource` category's job).
- **§20.3 plugin-category table** lists `Locality` ("How agents interact",
  MVP impls `wellmixed`, `network`) and `Resource` ("Resource dynamics", MVP
  impls `patchy`, `constant`).
- **§7.6** specifies the `Locality` interface `neighbours(i, t) → Set<AgentId>`
  with "MVP: `wellmixed`, `network`".

And **§27.1** defines the MVP as "the state of the system at the **Phase 2
gate**" — so anything §27.2 lists as an MVP dimension must exist by the Phase 2
gate, regardless of whether §26.4's prose enumeration happens to name it.

Phase 1 anticipated this: `PROGRESS.md`'s deferred-items table left README-only
directory markers at `crates/firma-plugins/firma-plugin-locality/` and
`crates/firma-plugins/firma-plugin-resource/` "crates land in Phase 2 (§26.4)".

This is the **same situation as ADR 0011**, where §26.3's Phase-1 gate required
VT-2 but §26.3's deliverables list omitted `firma-viability`. The resolution
shape is the same: the gate / MVP-contents requirement wins over the
deliverables-list omission, and the honest fix is a manual PATCH to the list.

## Decision

**Build `firma-plugin-locality` (`wellmixed`, `network`) and
`firma-plugin-resource` (`patchy`, `constant`) in Phase 2**, as MVP
deliverables, despite their absence from §26.4's deliverables paragraph.

Justification, in order:

1. §27.2 lists `Locality: wellmixed, network` as an MVP dimension, and §27.1
   binds the MVP to the Phase-2-gate state.
2. §20.3 lists both categories with MVP implementations.
3. §7.6 (for `Locality`) and §7.2 primitive 3 + §8.3 (for `Resource`) require
   the mechanism the MVP model runs on — `wellmixed` vs. `network` is
   §27.2's stated two-level manipulation, and resource regeneration (`patchy`
   / `constant`) is what makes the 3 conserved resource types *dynamic* rather
   than static.
4. This is a **manual gap**, not an implementation decision: §26.4's
   deliverables enumeration is inconsistent with §27.2's MVP-contents
   requirement. The honest fix is adding `firma-plugin-locality` and
   `firma-plugin-resource` to §26.4's list at the next manual revision — a
   PATCH-level clarification (§0.6), no numerical output moves.

**Scope of this ADR:** the *decision that they belong in Phase 2*. It does
**not** implement `wellmixed`/`network`/`patchy`/`constant` — that is Stage 4
of the Phase 2 build, instructed separately. This ADR exists now so that
Stages 1–3 do not build anything that assumes locality or resource-dynamics
plugins are out of scope (e.g. hard-wiring `wellmixed` neighbour semantics
into a constraint or decision plugin instead of reading the `Locality`
interface).

Both remain consistent with ADR 0007 (non-spatial by default): `Locality` is
an interface with `wellmixed` (the null) and `network` (topology without
geometry); `metric_space` stays Phase 4.

## Rejected alternatives

- **Treat §26.4's deliverables list as authoritative and defer `Locality` /
  `Resource` to Phase 4.** Rejected: it contradicts §27.2 (MVP contents) and
  §27.1 (MVP = Phase-2-gate state), and it would make several Phase 2 model
  behaviours (any interaction structure at all; resource regeneration)
  impossible, so the MVP could not run the E1 sweep §27.1 requires it to.

- **Fold locality into the kernel** (a built-in `wellmixed` with `network` as
  config). Rejected: §7.6 and A1 (§17) — "How agents interact" is a
  replaceable theory, it belongs in a plugin behind `neighbours(i, t)`, and
  the kernel must not know about it (ADR 0007 is explicit).

- **Skip `patchy`, ship only `constant` resource dynamics.** Rejected: §20.3
  lists both as MVP; `patchy` (regenerating pools) is needed for the resource
  scarcity that makes `solvency` and dependence (§13.1) bind. `constant` alone
  is a degenerate environment.

## Consequences

**Positive.**
- Removes an ambiguity before Stages 1–3 can trip on it.
- The MVP as actually specified (§27.2) becomes buildable and E1-capable.
- Parity with ADR 0011: the workspace's crate set is driven by the gate /
  MVP-contents requirements, with the deliverables-list gaps recorded for a
  manual PATCH.

**Negative, accepted.**
- Two more plugin crates in Phase 2 than §26.4's paragraph implies. A reader
  comparing the workspace to §26.4's prose will find `firma-plugin-locality`
  and `firma-plugin-resource` "extra"; this ADR is the explanation, alongside
  §27.2.
- Slightly more Phase 2 surface (Stage 4). Bounded: four small plugins
  (`wellmixed`, `network`, `patchy`, `constant`), all with existing interface
  specs (§7.6, §8.3).

**Neutral.**
- **No shipped numerical output changes.** This ADR is scope-only; it writes
  no code. Phase 1 golden trace, `phase1-smoke` run id, and all 55 workspace
  tests are unaffected.

## Compliance

- Stage 4 builds `crates/firma-plugins/firma-plugin-locality` (`wellmixed`,
  `network`) and `crates/firma-plugins/firma-plugin-resource` (`patchy`,
  `constant`), replacing the current README-only directory markers.
- Each depends only on `firma-core` (+ `firma-rng` for `patchy` regeneration
  draws, + Part C's `firma-domain` if it reads domain state) — the
  `plugins ← core, rng, viability, domain` rule (see ADR 0020); never on
  `firma-kernel` or each other (`no-cross-plugin-deps`, §25.6).
- `firma_lab` and the E1 `ExperimentSpec` reference `locality.wellmixed` /
  `locality.network` as the §27.2 two-level factor.
- Manual §26.4's deliverables list and §34.0 index gain the relevant rows at
  the next version bump; this ADR is the interim record and names the PATCH.

## Note

§26.4's deliverables paragraph and §27.2's MVP-contents table are two
enumerations of the same thing that drifted apart. §27.1 tells you which one
wins ("the state of the system at the Phase 2 gate"). This ADR just writes
that down so nobody has to re-litigate it mid-build — exactly what ADR 0011
did for `firma-viability`.
