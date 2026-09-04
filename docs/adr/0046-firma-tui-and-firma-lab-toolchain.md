# ADR 0046 — `firma-tui` (ratatui/crossterm) and `firma_lab` (PyO3/maturin) toolchain choices

**Status:** Accepted (2026-09-06)
**Phase:** 2 (Model), Stage 7 (Parts A, C)
**Relates to:** manual §23.1 (`firma_lab` module table), §23.2 (interfaces),
§27.2 (MVP contents — "Interface: ratatui monitor"), ADR 0010 (Phase-1
tooling/dependency precedent this ADR follows), ADR 0045 (the dependency-
graph decisions this ADR's toolchain choices sit inside)

## Context

Stage 7 is the first Stage needing a second language toolchain (`firma_lab`
is Python, via PyO3) and the first GUI-adjacent dependency (`firma-tui`,
ratatui). Neither is discretionary in the big picture — §23.2 names ratatui
+ crossterm explicitly, and §23.1 names PyO3 explicitly — but *how* to wire
them into this workspace (versions, module layout, build tooling, what runs
in Rust vs. Python) is new ground this project's ADR precedent (ADR 0010)
hasn't covered yet. Recorded here per that precedent: "new dependency
requires PR justification" (§24.1).

## Decision

### `firma-tui`: ratatui 0.30, crossterm 0.29

Pinned to the current stable minor releases at the time of writing
(`ratatui = "0.30"`, `crossterm = "0.29"` — Cargo's default caret bounds,
matching this workspace's existing style for 1.x deps). No feature flags
beyond ratatui's default backend selection; `crossterm` is ratatui's
manual-mandated backend (§23.2), not a discretionary alternative to
`termion`/`termwiz`.

### `firma-py` / `firma_lab`: PyO3 0.29.2, maturin, one venv

- **PyO3 0.29.2** (`extension-module` feature) — current stable, and the
  first version with solid CPython 3.14 support (this environment's
  interpreter). `extension-module` is required to build a portable wheel
  (it defers resolving Python symbols to the interpreter that `dlopen`s the
  `.so`, rather than linking a specific libpython at build time) — the
  trade-off is that `cargo test -p firma-py` cannot link a standalone test
  binary against this feature. **Consequence, accepted:** `firma-py` carries
  no `#[cfg(test)]` Rust unit tests of its own; `cargo build`/`cargo clippy`
  (both compile-only, no test-binary link) still work and are part of the
  verification block. Correctness is verified from the Python side instead
  (`python/tests/test_metrics_cross_check.py`, Stage 7 Part D) — the
  standard, and arguably more honest, way to test a PyO3 extension: through
  the interpreter that actually loads it.
- **maturin** (`>=1.5,<2.0`) as the build backend (`pyproject.toml`
  `[build-system]`) — the standard PyO3 packaging tool; no alternative was
  seriously considered (`setuptools-rust` is older, less actively
  maintained for PyO3's newer ABI work).
- **Package layout.** The Rust crate lives at `crates/firma-py` (a normal
  workspace member, consistent with every other crate — not a separate
  top-level Rust project). `pyproject.toml` lives at the **repository
  root** (alongside the top-level `Cargo.toml`) with
  `[tool.maturin] manifest-path = "crates/firma-py/Cargo.toml"` pointing
  in; `python-source = "python"` points at the existing `python/firma_lab`
  package directory (a directory marker since Phase 1). The compiled
  extension is named `firma_lab._native` (`[lib] name = "_native"` in
  `crates/firma-py/Cargo.toml`, `#[pymodule] fn _native(...)`) — nested
  *inside* the `firma_lab` package, not a sibling top-level module, so
  `import firma_lab` is the only import surface a consumer needs; `_native`
  is an implementation detail `firma_lab.metrics` reaches into.
- **One venv, one place.** `.venv/` at the repository root (gitignored),
  built with `python3 -m venv .venv` (Arch's system Python is
  externally-managed, PEP 668 — a venv is not optional here, not a style
  choice) and `maturin develop --release` to install `firma_lab` editable
  into it. `pandas>=2.0` is `firma_lab`'s one runtime dependency
  (`pyproject.toml [project] dependencies`); `maturin`/`pytest` are
  dev-only, installed into the same venv, not declared as package
  dependencies.

### `load` is pure Python; `metrics` is a PyO3 wrapper — not symmetric, deliberately

§23.1's table lists `load` and `metrics` side by side, but they are
different *kinds* of module. `load` (read event logs and manifests into
DataFrames) is generic NDJSON/JSON parsing with **no FIRMA formula in it**
— Python's stdlib `json` plus `pandas.json_normalize` do this natively and
well; routing it through Rust would add an FFI hop for no formula-reuse
benefit. `metrics` (offline metric computation, §14) is exactly the
opposite: every number it returns (`h`, the four `g_j`, `u`, repertoire
entropy, SC-1…6) is a formula this project has already verified once
(§15.1, ADR 0021/0026/0040) and re-derives nowhere — so `metrics` is a thin
wrapper (`python/firma_lab/metrics.py`) calling straight into
`firma_lab._native`, which calls straight into `firma-analysis`
(ADR 0045). **`load` uses pandas because it is the natural tool for the
job; `metrics` uses Rust because duplicating a verified formula is the one
thing this project does not do.**

### The six Phase-3 stubs

`spec`, `runner`, `stats`, `sensitivity`, `plot`, `prereg` — per §23.1's
table, each is a module with **only a docstring**, no functions, stating
in one paragraph what it will do and why it is empty now (each needs
either `firma_lab.runner`'s not-yet-built sweep output, or an
`ExperimentSpec` that does not exist until Phase 3 specifies E1 in code).
Deliberately *not* a stub function raising `NotImplementedError` — per the
instruction this Stage was built under, "don't build placeholder logic that
looks more complete than it is." A module with nothing callable cannot be
mistaken for a working, if incomplete, implementation.

## Alternatives

- **`setuptools-rust` instead of maturin.** Rejected: maturin is the
  PyO3-project-recommended, more actively maintained tool for exactly this
  "Rust extension inside a Python package" shape; no reason to deviate.
- **A sibling top-level `firma_lab_native` module instead of nesting as
  `firma_lab._native`.** Considered first, hit a real maturin constraint
  (`python-source` expects the compiled module's parent path to already
  exist as a package under the python-source tree — see the Note below) and
  was also simply less clean as a consumer-facing import surface. Rejected.
- **Symmetric `load`/`metrics` — both through Rust, or both pure Python.**
  Rejected: `load` has no formula to reuse (routing it through Rust would
  be process, not principle), and letting `metrics` be pure Python would
  reintroduce exactly the "write it a third time" problem ADR 0045/0046's
  whole promotion exercise exists to prevent.
- **`NotImplementedError`-raising stub functions for the six Phase-3
  modules.** Rejected per this Stage's explicit instruction — a callable
  that raises on use still implies a shape (parameter names, a call
  convention) that hasn't actually been decided yet; a docstring-only module
  commits to nothing prematurely.

## Consequences

- **Positive.** No FIRMA formula exists in three languages — Rust owns
  every one, `firma-tui` and `firma_lab.metrics` both call through
  (ADR 0045's crate + this ADR's wiring). The venv/maturin setup is fully
  reproducible from `pyproject.toml` + `Cargo.toml` — no manual steps
  recorded only in a person's memory.
- **Negative, accepted.** `firma-py` has no Rust-side unit tests
  (`extension-module`'s trade-off, above) — correctness rests on the Python
  cross-check (Part D) instead, which is now the crate's only test
  coverage; if that suite were ever skipped, a regression here would not be
  caught by `cargo test --workspace`. Two toolchains means two places
  `cargo fmt`/`clippy` and a Python formatter/linter would eventually need
  to agree on style — out of scope for Stage 7, flagged for whenever
  `firma_lab` grows past the two modules built here.
- **Neutral.** `.venv/` and Python build artefacts are gitignored; a fresh
  checkout needs `python3 -m venv .venv && source .venv/bin/activate && pip
  install maturin pandas pytest && maturin develop --release` once before
  `pytest python/tests/` works — documented here rather than only in this
  Stage's report.

## Compliance

- `cargo build -p firma-tui`, `cargo clippy -p firma-tui --all-targets -- -D
  warnings`, `cargo test -p firma-tui` — all green (Stage 7 Part A/E).
- `cargo build -p firma-py`, `cargo clippy -p firma-py -- -D warnings` —
  green (no `--all-targets`; see the PyO3/`extension-module` note above).
- `python/tests/test_metrics_cross_check.py` — the Rust/Python agreement
  proof (Stage 7 Part D); raw `pytest` output in the Stage 7 report.
- `cargo tree -p firma-tui -e normal` — no `firma-kernel` node
  (ADR 0045's structural guarantee, reconfirmed here since this ADR is what
  actually builds the crate ADR 0045 designed for).

## Note

The `python-source` + nested-module-name maturin layout
(`firma_lab._native`) failed on the first attempt with a top-level
`firma_lab_native` name: maturin's error was explicit — *"the python module
at `.../python/firma_lab_native` does not exist"* — because `python-source`
expects the compiled extension's dotted path to resolve inside an
already-existing package tree, not to invent a new top-level package from
nothing. Recorded so a future maintainer adding a second native
sub-extension does not re-discover this by trial and error.
