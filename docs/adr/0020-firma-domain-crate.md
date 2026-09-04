# ADR 0020 — Shared §8 state/parameter types live in a new `firma-domain` crate, not `firma-core`

**Status:** Accepted (2026-09-03)
**Phase:** 2 (Model), Stage 0
**Relates to:** manual §8.1 (firm-agent state), §8.2 (constraint parameters),
§8.4 (components), §7.3–§7.5 (no `Firm` type in the kernel), §7.4 (ECS),
§17 A1, §18.1 (dependency rules), §25.6 (`no-kernel-domain-deps`), §38
(directory layout), ADR 0001, ADR 0011

> **Beyond the instructed six ADRs.** Stage 0's brief named five §16.3 ADRs
> plus one Locality/Resource ADR. This seventh is added because *where the
> §8 types live* is a **structural decision** — a new workspace crate and a
> new edge in the §18.1 dependency graph — and CLAUDE.md requires a numbered
> ADR for structural choices the manual does not fully specify. If the owner
> prefers this recorded as a `PROGRESS.md` note instead, downgrade it; the
> decision itself stands either way.

## Context

Stage 1+ (constraint plugins, then decision, then actions, then
`firma-viability`'s `margin`) all read and write the firm-agent state of §8.1
(`r^L`, `r^I`, `c`, `q`, plus auxiliary `λ`, `u`, aspirations, …) and the
constraint parameters θ of §8.2. Those types need one canonical home that
every domain crate depends on. Phase 1's `firma-core` is the model for "shared
types, no behaviour" (§18.2) — the question is whether the §8 types extend
`firma-core` or get a new crate.

**The A1 trace (the instruction's "don't assume either way").**

- §17 A1: "The kernel contains no domain logic. **No firm, constraint,
  action, or theory in the kernel crate.**" Its CI enforcement is narrower:
  `firma-kernel` MUST NOT depend on any *plugin* crate; `scripts/check_deps.py`
  implements this as "`firma-kernel`'s workspace deps ⊆ {`firma-core`,
  `firma-rng`}".
- §18.1: `firma-kernel ← core, rng`. So `firma-kernel` depends on
  `firma-core` transitively-publicly — everything `pub` in `firma-core` is in
  `firma-kernel`'s dependency surface.
- Therefore: **if `FirmState`, `Capability`, `Aspirations`, `ConstraintParams`
  went into `firma-core`, `firma-kernel`'s dependency graph would contain
  firm-shaped domain types.** The `no-kernel-domain-deps` lint (which matches
  on crate *names*, not contents) would not catch it — but it would violate
  the A1 *principle* ("no firm … in the kernel crate"), which §7.3 makes
  concrete: "There is no `Firm` type in the kernel."
- §7.4/§7.5: the kernel sees domain state only as **opaque registered
  components** (ECS component bags). It never names `r^L` or `capability`. The
  §8 types are read by *plugins* and by `firma-viability`, never by
  `firma-kernel`.

So `firma-core` is the wrong home. A new crate that `firma-kernel` does **not**
depend on is the right one.

## Decision

**Create `crates/firma-domain` — a new workspace crate holding the §8 state
and parameter types as plain data (serde-derived, `pub` fields, no behaviour
beyond trivial constants). It depends only on `firma-core`. `firma-kernel`
does not, and must never, depend on it.**

### §18.1 dependency-graph extension

```
firma-core      ← nothing
firma-domain    ← core                       (NEW)
firma-rng       ← core
firma-config    ← core
firma-viability ← core, domain               (domain added in Stage 1)
firma-kernel    ← core, rng            [MUST NOT depend on any plugin, and MUST NOT depend on firma-domain]
firma-registry  ← core, config
firma-io        ← core, config
plugins         ← core, rng, viability, domain   [domain added; still MUST NOT depend on kernel or each other]
firma-cli       ← everything
firma-tui       ← core, io
firma-py        ← core, kernel, registry, io
```

`firma-domain ← firma-core` (not zero-dep): it reuses `firma_core::ResourceKind`
for the ledger's resource-keyed stocks and `firma_core::AgentId` /
`firma_core::Tick` for the identifiers Stage 1's constraint-instance and
relation types will need — one vocabulary, no re-definition.

**Stage 0 wires `firma-domain` into the workspace `members` and
`[workspace.dependencies]` only. No existing crate gains it as a dependency in
Stage 0** — `firma-viability` picks it up in **Stage 1** (its `margin` /
`kernel` domain surface and the four `Constraint` plugins are one coupled unit:
§18.2's `margin(&self, x, theta, cs: &[Constraint])` cannot be built without
the `Constraint` interface, and vice versa), and the remaining plugin crates
pick it up in Stages 1–3.

### Lint

`scripts/check_deps.py`'s existing rule — `firma-kernel` deps ⊆ {`firma-core`,
`firma-rng`} — **already forbids** `firma-domain` in the kernel. A comment is
added there in Stage 1 noting `firma-domain` is deliberately excluded; no logic
change is needed.

## Rejected alternatives

- **Extend `firma-core` with the §8 types.** Rejected on the A1 trace above:
  it puts firm-shaped types in `firma-kernel`'s dependency surface, violating
  the §7.3 "no `Firm` type in the kernel" principle even though the
  name-matching lint would stay green. `firma-core` must remain pure
  substrate.

- **Zero-dependency `firma-domain`** (just `serde`, no `firma-core`).
  Rejected: the ledger is resource-keyed (`ResourceKind`), constraint
  instances and relations are agent-keyed (`AgentId`), and the `constrain`
  phase is tick-stamped (`Tick`). Re-defining those newtypes in `firma-domain`
  would fork the vocabulary §5 says must be used "exactly". One `← core` edge
  is cheaper than two parallel `AgentId` types.

- **Put the types in `firma-viability`** (since `margin` is their first
  non-plugin consumer). Rejected: the constraint plugins need them before
  `firma-viability`'s domain layer exists, and `firma-viability` is itself a
  `← core` crate that plugins depend on — folding domain state into it
  conflates "the kernel solver" with "the state vocabulary".

- **A `firma-plugins/firma-plugin-domain` under the plugins tree.** Rejected:
  it is not a plugin (no `Rule`, no `assumption()`), and `no-cross-plugin-deps`
  would then forbid other plugin crates from depending on it.

## Consequences

**Positive.**
- The A1 principle holds structurally: `firma-kernel` cannot name a firm type
  because it cannot reach the crate that defines one.
- One canonical §8 vocabulary for every Stage 1+ crate.
- `firma-domain` is a small `← core` leaf; adding it is low-risk and its blast
  radius in Stage 0 is zero (nothing depends on it yet).

**Negative, accepted.**
- One more crate than §38's directory layout lists. §38 will gain
  `crates/firma-domain/` at the next manual revision (PATCH — a layout
  clarification, no numerical output). Same shape as ADR 0011's
  `firma-viability`-is-early note.
- §18.1's dependency table needs the three edits above at the next manual
  revision. Recorded here in the interim.

**Neutral.**
- **No shipped numerical output changes.** `firma-domain` contains only type
  definitions; nothing executes it. `firma-kernel`, `firma-core`, and every
  other Phase 1 crate keep their exact dependency sets (verified by
  `cargo tree` in the Stage 0 report). Phase 1 golden trace, `phase1-smoke`
  run id, and all 55 workspace tests are unaffected.

## Compliance

- `crates/firma-domain/Cargo.toml` declares `firma-core` + `serde` in Stage 0;
  Stage 1 adds `semver` (the `Constraint` trait's `version()` returns
  `semver::Version`, mirroring `Rule`).
- `firma-kernel/Cargo.toml` is **not** touched by any Stage 0 or Stage 1 work;
  `scripts/check_deps.py` (kernel deps ⊆ {core, rng}) continues to pass and now
  also serves as the guard against `firma-domain` reaching the kernel.
- **Stage 1** adds `firma-domain` to `firma-viability/Cargo.toml` and to
  `crates/firma-plugins/firma-plugin-constraint`; later stages add it to each
  further plugin crate. A `cargo tree -p firma-kernel` check (or the existing
  `check_deps.py`) confirms `firma-domain` never appears in the kernel's tree.
- Manual §18.1 and §38 gain the edits above at the next version bump.

## Note

The subtlety the instruction flagged is real: the `no-kernel-domain-deps` lint
matches crate names, so it would *not* have caught domain types smuggled into
`firma-core`. The defence is architectural, not lint-based — keep the crate
that defines "firm" out of `firma-kernel`'s reachable graph entirely. That is
the whole reason `firma-domain` is a sibling of `firma-core` rather than part
of it.
