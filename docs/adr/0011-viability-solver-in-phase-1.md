# ADR 0011 — Include the `firma-viability` backward-iteration solver in Phase 1

**Status:** Accepted (2026-09-03)
**Phase:** 1 (Kernel)
**Relates to:** manual §9.3, §18.2 (`firma-viability` contract), §25.4 (VT-1, VT-2),
§26.3 (Phase 1 gate), §26.4 (Phase 2 deliverables), §15.2 (Example B)

## Context

The Phase-1 gate (§26.3) reads: "DT-1…DT-6 green. **VT-2 and VT-6 pass.**"
VT-2 (§25.4) is "Backward iteration monotone decreasing, reaches fixed point",
whose ground truth is the fixed-point theorem `[E]` and whose worked fixture is
Example B (§15.2, whose header explicitly reads "Fixture for VT-1 and VT-2").

But the **Phase-1 deliverables list** in §26.3 does *not* mention
`firma-viability`; that crate first appears in the **Phase-2** deliverables
(§26.4). The manual therefore requires a test in Phase 1 that its own deliverable
list does not provision a crate for. Per CLAUDE.md ("If this file and the manual
disagree… say so") and the standing instruction to write an ADR for any
structural deviation, this is recorded rather than resolved silently.

## Decision

**Create `crates/firma-viability` in Phase 1**, containing *only* the generic
viability-kernel machinery needed to satisfy VT-2:

- a discrete `Grid` (bounded integer lattice) and a `KernelSet` bitset;
- a `Dynamics` trait — an abstract admissible-transition relation
  `admissible_successors(&self, point) -> impl Iterator<Item = GridPoint>`;
- `kernel(grid, dynamics) -> KernelReport`, the backward iteration
  `K^(n+1) = { x ∈ K^(n) : ∃ successor of x in K^(n) }` (§9.3), returning the
  fixed point **and the full sequence `|K^(0)| ≥ |K^(1)| ≥ …`** so a test can
  assert monotone decrease and termination;
- `volume(&KernelSet)` = `|K| / |grid|`.

**No firm domain logic enters the crate.** It has no `Constraint`, no `State`,
no `Params`, no knowledge of solvency/compliance/scope/obligation. The Example B
system (the P and B transitions of §15.2) is defined **inside the VT-2/VT-1
test**, not in the crate — the crate sees only an opaque `Dynamics` impl. This
keeps `firma-viability`'s dependency edge at `firma-core` only (§18.1) and
leaves the domain-specific `margin(state, params, constraints)` and the
FIRMA `Dynamics` (the deterministic core of §11) for Phase 2, exactly as
§26.4 intends.

The crate is **not** wired into `firma-kernel` (A1 forbids it) and **not** used
by any Phase-1 plugin. It is exercised only by the conformance tests.

## Alternatives

- **Satisfy VT-2 with an ad-hoc solver inside the test crate, no
  `firma-viability`.** Keeps the Phase-1 deliverable list literally intact, but
  the solver is real, reusable infrastructure that Phase 2 needs regardless;
  hiding it in a test file only guarantees it gets rewritten. Rejected.
- **Defer VT-2 to Phase 2 and treat the gate as "DT-1…DT-6 + VT-6".** This is the
  cleaner reading of the *deliverables* list, but it directly contradicts the
  gate sentence and the user's explicit instruction that "VT-2 and VT-6 … is the
  literal Phase 1 gate." Rejected.
- **Build the full Phase-2 `firma-viability` now** (margin proxy, FIRMA dynamics,
  the four constraints). Rejected hard: that is domain modelling, forbidden in
  Phase 1 (§26.3 "No domain logic whatsoever"), and would drag in the whole §11
  action core.

## Consequences

**Positive.**
- VT-2 is tested against real, shipped code, and the fixed-point iteration is
  available for Phase 2 to build `in_kernel` / `kernel_volume` on top of.
- Example B also gives VT-1 (|Viab(K)| = 19, volume 0.76) essentially for free;
  the Phase-1 test asserts it, so Phase 2 inherits a passing VT-1 for the 2-D
  linear case.
- The `Dynamics` trait boundary is fixed early and is domain-free, which is the
  right shape for the Phase-4 sampling-based approximator (§18.2 extension point).

**Negative, accepted.**
- The Phase-1 crate count is one higher than §26.3's list. A reader comparing the
  workspace to the manual will find `firma-viability` present "early"; this ADR
  is the explanation.
- `firma-viability` will grow substantially in Phase 2. Its Phase-1 surface is
  deliberately minimal and its `//!` doc says so.

## Compliance

- `firma-viability` `Cargo.toml` depends only on `firma-core` (checked by
  `scripts/check_deps.py` extended coverage and by `no-cross-plugin-deps` spirit).
- `firma-kernel` does **not** depend on `firma-viability` (`no-kernel-domain-deps`).
- VT-1 and VT-2 live in `tests/tests/validation.rs`; the Example B transition
  system is local to that file.
- The manual's §34.0 index gains rows 0010–0012 "at the next manual version bump"
  (§38); until then this file is the record.

## Note

If the manual is revised, the honest fix is to add `firma-viability` to the
§26.3 deliverables list with a one-line note that its Phase-1 scope is "generic
backward-iteration solver only, for VT-2". That is a PATCH-level clarification
(§0.6), not a MAJOR change — no numerical output moves.
