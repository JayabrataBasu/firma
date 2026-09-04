# ADR 0010 — Phase 1 tooling, lint infrastructure, and dependency set

**Status:** Accepted (2026-09-03)
**Phase:** 1 (Kernel)
**Supersedes:** none
**Relates to:** manual §24.1 (toolchain), §25.6 (CI lints), §26.2 (Phase 0 remaining
work: "set up repository, CI skeleton, and lint infrastructure"), ADR 0001, ADR 0008

## Context

Manual §26.2 lists "set up repository, CI skeleton, and lint infrastructure" as
remaining Phase-0 work, and §26.3 requires "full CI (format, lint, test,
determinism, golden trace)" as a Phase-1 deliverable. The manual fixes the
*intent* of the toolchain (§24.1) and enumerates nine CI lints (§25.6) but does
not pin exact tool versions, the mechanism for the custom lints, or which
external crates the kernel may depend on. §24.1 also states "new dependency
requires PR justification". This ADR records those choices so a reviewer sees
one deliberate decision rather than a scatter of defaults.

## Decision

### Toolchain

- **Rust 1.98.0, pinned exactly** in `rust-toolchain.toml` (channel, not a range),
  with `rustfmt` and `clippy` components. Edition 2021. This is also the declared
  **MSRV**, recorded in `[workspace.package].rust-version`; raising it requires a
  superseding ADR (§24.1).
- **`rustfmt.toml`** committed, stable options only, `max_width = 100`,
  Unix newlines. CI runs `cargo fmt --all --check`.
- **`clippy` at `-D warnings`** applied via `RUSTFLAGS`/CI, *not* baked into crates
  as `#![deny(warnings)]`. Baking it in means a future compiler's new lint breaks
  every local build; enforcing it in CI achieves the §24.1 requirement without
  that fragility. Workspace `[lints]` do bake in the lints that are load-bearing
  for the architecture: `unsafe_code = "forbid"`, `missing_docs = "deny"`,
  `clippy::all = "deny"`.
- **`cargo-deny`** with a committed `deny.toml`: permissive licences only,
  advisories deny, and a `bans.deny` list for dynamic-loading crates
  (`libloading`, `dlopen`, `dlopen2`, `abi_stable`) — a second guard on ADR 0008.

### The nine §25.6 lints

Implemented as a single `scripts/lint-architecture.sh` (plus `scripts/check_deps.py`
for the `cargo metadata` graph checks) invoked by CI, **and** — where a runtime
check is stronger — as ordinary `#[test]`s:

| §25.6 lint | Mechanism |
|---|---|
| `no-kernel-domain-deps` | `cargo metadata` graph check: `firma-kernel`'s workspace deps ⊆ {`firma-core`, `firma-rng`} |
| `no-cross-plugin-deps` | `cargo metadata` graph check: no `firma-plugin-*` depends on another |
| `no-hashmap-iteration` | grep bans `HashMap`/`HashSet`/`hashbrown`/`IndexMap` in the sim-path crates' `src/` |
| `no-ambient-rng` | grep bans `thread_rng`, `OsRng`, `getrandom`, `SystemTime`, `Instant::now`, `std::time::` in the sim path |
| `no-unsafe` | grep asserts `#![forbid(unsafe_code)]` in every `lib.rs`; workspace lint forbids it anyway |
| `no-magic-numbers` | **heuristic grep tripwire only**; the binding check is the §24.6 review checklist (see Consequences) |
| `docs-required` | workspace `missing_docs = "deny"` + `RUSTDOCFLAGS=-D warnings` |
| `assumption-nonempty` | runtime `#[test]` in `tests/` iterating every registered plugin; grep backup for an empty literal |
| `no-dynamic-loading` | grep + `cargo-deny` `bans.deny` |

"Sim path" is defined once, in the script: `firma-core`, `firma-rng`,
`firma-viability`, `firma-kernel`, and the plugin crates. `firma-io`,
`firma-config`, and `firma-cli` are orchestration and may use hash maps as long
as iteration order never feeds a simulation result (§19.6).

### External dependency set (Phase 1)

The network was available at scaffold time, so a small vetted set is used rather
than re-implementing everything. Each is a mature, widely-audited crate with no
current advisory and a permissive licence:

| Crate | Used for | Why not hand-rolled |
|---|---|---|
| `serde` + `serde_json` | (de)serialisation of config, manifest, event log, snapshots | `serde_json` is deterministic for structs (field-declaration order) and `BTreeMap` (sorted); a hand parser is *more* clever, not less (A7) |
| `sha2` | SHA-256 for content hashing and the manifest run id (§22.3) | re-implementing a hash for content-addressing is exactly the cleverness A7 warns against |
| `semver` | engine/plugin version-requirement matching (§20.4, §20.5) | semver range logic is fiddly and easy to get subtly wrong |

**Philox is implemented in-tree** (`firma-rng`), not taken from a crate: it is the
core scientific primitive of §21.2 and must be visible, commented, and tested
against its own fixtures, not hidden behind a dependency.

**Deliberately deferred** (recorded so their absence is not mistaken for an
oversight; each is a Phase-2 line in `PROGRESS.md`):

- **Arrow/Parquet** for the event log (manual §22.1) — see ADR 0012.
- **`proptest`** (manual §25.1, §25.3) — Phase 1 property tests use deterministic
  loop-based randomised inputs driven by the in-tree Philox RNG. `proptest`
  brings a large transitive tree and its shrinking is most valuable once domain
  rules exist. Adopt in Phase 2.
- **`clap`** — `firma-cli` hand-rolls argument parsing for three subcommands;
  fewer transitive deps and a smaller audit surface for a tiny CLI.
- **`thiserror`** — error enums hand-write their `Display`/`Error` impls.
- **`serde_yaml`** — unmaintained; `serde_yml` is a young fork. Phase 1 config is
  **JSON** (see `PROGRESS.md` open question OQ-2); YAML support is a Phase-2
  decision that will get its own ADR if adopted.

## Alternatives

- **Zero external dependencies (pure `std`).** Attractive for a reproducibility
  project and briefly the plan when the network looked unavailable. Rejected once
  `cargo` fetch was confirmed working: hand-rolling SHA-256, a JSON parser, and
  semver matching is a large surface of bespoke code that a reviewer must now
  trust, for no reproducibility gain over three pinned, lockfile-hashed crates.
- **The full manual dependency set now** (`arrow`, `parquet`, `proptest`, `clap`,
  a YAML parser). Rejected for Phase 1: each adds audit surface and compile time
  for capability the Phase-1 gate does not exercise. They are scheduled, not
  dropped.
- **Custom `cargo` subcommand / `dylint` for the §25.6 lints.** More precise than
  grep, but adds a compiled tool and its own dependency tree to maintain from
  day one. The grep+metadata script is boring, fast, and inspectable; it can be
  upgraded to `dylint` later without changing what is enforced.

## Consequences

**Positive.**
- One `rust-toolchain.toml` means local, CI, and a reviewer's build all use the
  same compiler — a precondition for the §21.4 bit-identity goal.
- The §25.6 lints run in seconds and need no build.
- The dependency tree is small enough to audit by hand and is fully pinned by
  `Cargo.lock` (committed), whose hash goes into every manifest (§22.3).

**Negative, accepted.**
- **`no-magic-numbers` is only a heuristic in CI.** A grep cannot reliably tell a
  magic number from an array index or a bit width. The script emits warnings, not
  failures, for suspicious literals; the real enforcement is the §24.6 review
  checklist item. This is a known weakness in the automation and is called out in
  `PROGRESS.md` (OQ-5).
- The grep-based lints match on token text, so a creative alias
  (`use std::collections::HashMap as Table`) would slip past. Mitigated by
  `clippy::all` and review; upgradeable to a semantic lint later.
- Deferring `proptest` means Phase 1 property coverage is shallower than the
  manual's eventual bar. The Phase-1 *gate* (§26.3) does not require property
  tests, so this is a scheduling choice, not a gap in the gate.

## Compliance

- CI jobs `format`, `lint`, `deny`, `test` (manual §24.7, §25.1).
- `scripts/lint-architecture.sh` exit code gates the `lint` job.
- `Cargo.lock` committed; `deny.toml` committed.
- MSRV in `[workspace.package].rust-version`; change requires a superseding ADR.

## Note

The manual's dependency-justification rule (§24.1) exists so that "a new
dependency" is a visible, reasoned event rather than an `cargo add` reflex. That
is the entire purpose of the table above. If a Phase-2 change wants `arrow`,
`proptest`, `clap`, or a YAML crate, it adds a row here in a **new** ADR — this
one is immutable — with the same three columns filled in.
