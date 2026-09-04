# ADR 0045 — `firma-analysis`: promoting the log-reconstruction logic, and relocating `Event` to `firma-core`

**Status:** Accepted (2026-09-06)
**Phase:** 2 (Model), Stage 7 (Part B)
**Relates to:** manual §14 (rigidity/viability metrics), §17 A5 (measurement
is offline), §18.1 (dependency graph), §22.2 (log sufficiency), §23.1
(`firma_lab`), §23.2 (`firma-tui` — "MUST NOT link against the kernel"),
ADR 0020 (why `firma-domain` is its own crate, not `firma-core`), ADR 0021 /
ADR 0026 / ADR 0040 (single-source-formula precedent this ADR continues)

## Context

Stage 6 built substantial offline event-log reconstruction in
`tests/src/replay.rs` — folding `DeltaApplied` records back into per-firm
`(r^L, r^I, c, q, λ, u, W)`, recomputing `h`/`g_j` via `firma_domain::margin`,
and deriving the SC-1…6 metrics. It lived in the tests-only conformance
crate. Stage 7 needs the same computations from two more places:
`firma-tui`'s "margin distribution" panel (a **live** view) and
`firma_lab`'s `metrics` module (a **Python** binding, Stage 7 Part C). Per
this project's standing rule (ADR-0021/0026/0040: one formula, one place),
none of these should re-derive `standard_margin`/`g_j`/`u`/entropy
independently.

**A real dependency-graph obstacle surfaced while planning the promotion.**
The manual's §18.1 table gives `firma-tui ← core, io` and states, in bold,
"**MUST NOT** link against the kernel or influence it" (§23.2). But
`replay.rs`'s reconstruction reads `firma_kernel::Event` — and `Event` is
defined *in* `firma-kernel`. Any crate that wants to fold the event log
(promoted or not) needs the `Event` type; if that type stays kernel-owned,
`firma-tui` cannot depend on the promoted crate without violating §23.2's
explicit MUST NOT.

Inspecting `Event`'s actual definition
(`crates/firma-kernel/src/event.rs`, now moved) shows it depends on nothing
kernel-internal: only `firma_core::{AgentId, DeltaKind, DeltaTarget,
PluginId}` — the same wire types `Delta` itself already lives with. `Event`
being physically housed in `firma-kernel` was incidental (it's what the
kernel *emits*), not structural (it doesn't need anything the kernel *does*).
This is the same shape of problem ADR-0020 solved for firm-state types
("the crate that defines 'firm' ... keep that out of the kernel's reachable
graph") — here it runs the other direction: a type the kernel emits, needed
by crates that must stay outside the kernel's graph.

## Decision

**Two moves, one purpose: let an offline-analysis crate exist without ever
reaching `firma-kernel`.**

### Decision 1 — `Event` moves from `firma-kernel` to `firma-core`

`crates/firma-kernel/src/event.rs` → `crates/firma-core/src/event.rs`,
internal `firma_core::{...}` imports rewritten to `crate::{...}`.
`firma-kernel` keeps `pub use firma_core::Event;` — **every existing
`firma_kernel::Event` call site is unaffected**; this is a pure relocation,
not an API change. `firma-core`'s doc comment updated to name `Event` among
its "types only, no behaviour" (§18.2) contents.

### Decision 2 — `firma-analysis`, promoting `replay.rs`

New crate, `firma-analysis ← firma-core, firma-domain, firma-config,
firma-io` (never `firma-kernel`, never a plugin — checked below). Two
modules:

- **`reconstruct`** — `Reconstruction`, an *incremental* fold: `apply(&mut
  self, ev: &Event) -> Vec<Notable>`, one call per logged event, usable by
  both a batch reader and a live tailer. `Notable` is the single place "what
  does this raw `DeltaApplied` mean" is decided (a decision, a `focus` log,
  a constraint bind, a shaping commit, a shock) — `sanity::sanity_from_run`
  and `firma-tui` both fold `Notable`s instead of re-reading `DeltaKind`
  discriminants themselves. `FirmSnapshot` computes `h`/`g_j`/`u` on demand
  from the raw ledger, via `firma_domain::margin`, in exactly one place.
- **`sanity`** — `SanityReport` + `sanity_from_run`, the promoted Stage-6
  batch entry point, rebuilt on top of `Reconstruction`/`Notable` instead of
  its own hand-rolled `DeltaApplied` match. **Behaviourally identical** to
  the original (verified below) — same field names, same thresholds, same
  `SC6_MIN_VARIANCE`.

`tests/src/replay.rs` becomes a one-line re-export shim
(`pub use firma_analysis::sanity::*;`) — the only test-crate change needed;
every existing `firma_conformance::replay::{sanity_from_run, SanityReport}`
call site is unaffected.

### The dependency graph, traced

```
firma-analysis  ← firma-core, firma-domain, firma-config, firma-io
firma-tui       ← firma-core, firma-io, firma-analysis   (extends §18.1's core+io)
firma-py        ← firma-core, firma-config, firma-io, firma-analysis
                  (§18.1 additionally lists firma-kernel, firma-registry — for
                  the future `runner`/`spec` modules, explicitly stubbed this
                  Stage, §27.2/§23.1 scope; adding an unused dependency now
                  would be exactly the "design for a hypothetical future
                  requirement" this project's style rule disallows. Added when
                  `runner` is actually implemented, Phase 3.)
```

Checked against the two binding hard rules (§18.1): **firma-analysis never
reaches `firma-kernel`** (not in its dependency list, and `cargo tree -p
firma-tui` — Part A's verification — confirms no `firma-kernel` node
anywhere in the resolved tree); **firma-analysis never reaches a plugin
crate** (not in its dependency list either). Both hold. `firma-tui`'s and
`firma-py`'s dependency lists extend beyond what §18.1's table currently
shows (which predates this Stage's `firma-analysis` crate) — flagged below
for a §18.1 PATCH, not silently assumed.

## Alternatives

- **Give `firma-analysis` its own duplicate `Event`-shaped enum** (mirroring
  the JSON schema without being the same Rust type). Rejected: this is
  exactly the "write it a third time" pattern the project's anti-duplication
  rule exists to prevent, just at the type level instead of the formula
  level — any future `Event` variant would need updating in two places with
  no compiler enforcement of agreement.
- **Let `firma-analysis` (and `firma-tui`) depend on `firma-kernel` just for
  the `Event` type.** Rejected: directly violates §23.2's explicit,
  capitalised "**MUST NOT** link against the kernel" for `firma-tui` — not a
  SHOULD to weigh, a MUST NOT to honour.
- **A separate `firma-events` crate just for the `Event` type**, rather than
  folding it into `firma-core`. Considered and rejected as unnecessary
  ceremony: `Event` already depends only on existing `firma-core` types, has
  no state or behaviour of its own, and `firma-core` is already the
  "wire types, no behaviour" crate (§18.2) `Delta`/`DeltaKind`/`DeltaTarget`
  live in — `Event` belongs with them, not in a crate of one.

## Consequences

- **Positive.** One reconstruction, three consumers (tests, `firma-tui`,
  `firma-py`) — no metric formula exists in more than one place. `Event`'s
  new home makes the kernel-independence of analysis tooling a structural
  fact (checkable via `cargo tree`), not a promise, matching this project's
  general posture (§17 A3: "determinism is structural, not disciplinary" —
  the same discipline applied to the kernel-independence guarantee).
- **Negative, accepted.** `firma-tui`'s and `firma-py`'s dependency lists now
  exceed what §18.1's table states; §18.1 needs a PATCH adding
  `firma-analysis` and updating both rows. `Reconstruction`'s `Notable`
  enum is a new abstraction with its own small maintenance surface — kept
  deliberately thin (facts, not display formatting) so `firma-tui` and
  `sanity_from_run` each do their own thing with it.
- **Neutral.** `tests/src/replay.rs` shrinks to a five-line shim; no test
  file needed to change beyond that (`sanity.rs`'s existing `use
  firma_conformance::replay::{sanity_from_run, SanityReport};` resolves
  unchanged). No shipped numerical output changes — this is a pure code
  reorganisation, confirmed against all four frozen hashes.

## Compliance

- `cargo tree -p firma-kernel -e normal` — unchanged: `{firma-core,
  firma-rng}`.
- `cargo tree -p firma-tui -e normal` — confirmed to contain no
  `firma-kernel` node (Part A's verification block).
- `cargo metadata | scripts/check_deps.py` — unaffected (only checks
  `firma-kernel`'s and plugins' deps; `firma-analysis`/`firma-tui` are
  outside its scope, same as `firma-cli`/`firma-io` today). `lint-
  architecture.sh` gains a new check for the `firma-tui`-kernel-independence
  rule (Part A).
- `cargo test --workspace` — all green post-move (same counts as the
  Stage-6c report); the four frozen hashes (golden, phase1-smoke,
  phase2-smoke, phase2-stage5-smoke) unchanged.
- `docs/adr/README.md` updated with this ADR's row.

## Note

§18.1's table is a forward-looking declaration of the *full* eventual graph,
not a record of what exists yet — `firma-tui`/`firma-py` were directory
markers before this Stage. Extending their listed dependencies to include a
crate the manual's table couldn't have anticipated (`firma-analysis` did not
exist when §18.1 was written) is the kind of gap this project logs as a
PATCH item rather than either silently ignoring or silently deviating from.
Recorded here so it is not rediscovered as a surprise at the next manual
revision.
