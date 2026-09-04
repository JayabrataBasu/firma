# ADR 0024 — Generalise `SetSelectedAction` to `SetAgentInt`; put `Delta`'s manual `Eq` on an enforced invariant — supersedes ADR-0022 Decisions 1–3

**Status:** Accepted (2026-09-04)
**Phase:** 2 (Model), Stage 2 review → recorded at Stage 3
**Supersedes:** ADR-0022 Decisions 1, 2, 3 (in part). ADR-0022 Decision 4 and
the rest of that ADR stand unchanged.
**Relates to:** manual §12.3, §17 A1, §19.4, §19.5, §21.4; ADR-0020
(no domain concepts in `firma-kernel`'s reachable graph), ADR-0022

## Context

This ADR is **paperwork for a change that has already shipped.** The
`SetAgentInt` variant, the `KernelError::NonFiniteDelta` check, and the
testkit `firma-domain` dependency are all in the tree and green as of the
Stage-2 review (2026-09-04). What follows records *why*, correctly this time,
because the fix was first applied as an in-place edit to ADR-0022's body —
which is not how ADR corrections are done (`docs/adr/README.md`: accepted ADR
bodies are append-only; corrections are new numbered ADRs). ADR-0022 has been
restored to its as-accepted text; this ADR carries the correction.

### What ADR-0022 decided, and the two problems the Stage-2 review found

1. **Decision 1** stated that `selected_action` lives in the kernel's *opaque*
   keyed store and that "the kernel holds the value, the domain meaning lives
   in `firma-domain::keys` and the plugins."

2. **Decision 2**'s variant table shipped a **dedicated**
   `SetSelectedAction { action: u8 }` — a `set`-an-int operation that, unlike
   every other new variant, carries **no `field` string**.

3. Implementing (2) forced `firma-kernel::apply_delta` to name the key
   itself:

   ```rust
   DeltaKind::SetSelectedAction { action } => {
       let a = agent_of(&d.target)?;
       world.set_agent_int(a, "selected_action", i64::from(*action))  // <-- literal
           .ok_or(KernelError::UnknownAgent(a.0))
   }
   ```

   The string `"selected_action"` — a §12.3 decision-procedure concept — was a
   literal in `firma-kernel/src/lib.rs`. This **contradicts Decision 1 in
   substance**: the kernel *did* name a domain key. The `no-kernel-domain-deps`
   lint (a crate-dependency check) does not catch a string literal, so it
   passed. Exactly the "name-matching lint satisfied while smuggling domain
   awareness in" failure mode ADR-0020 exists to prevent.

4. **Decision 2** also claimed the pre-serialised-JSON-string choice for
   *record* payloads was why "`Delta` keeps deriving `PartialEq, Eq`". This is
   wrong: variants 2 and 4 (`AdjustAgentReal` / `AdjustGlobalReal`) carry
   `delta: f64` **directly**, which blocks `#[derive(Eq)]` regardless of the
   record choice. The implementation added a bare `impl Eq for DeltaKind {}` /
   `impl Eq for Delta {}` with a comment asserting "the `f64` fields … hold
   finite model deltas — never `NaN`" — an *assumption*, not an enforced
   invariant. A `NaN` payload would make `PartialEq` non-reflexive and the
   manual `Eq` unsound (and `Delta: Ord`, needed by the reconciler's `sort()`,
   has `Eq` as a supertrait).

## Decision

### 1. `SetSelectedAction { action: u8 }` → `SetAgentInt { field: String, value: i64 }`

`firma-core::DeltaKind` variant 1 (discriminant 1) is the **generic**
`SetAgentInt { field: String, value: i64 }` — the `set`-last-write counterpart
to `AdjustAgentInt`'s `add`. `firma-core::DeltaKindTag::SetAgentInt` matches.
`slot()` returns the carried `field`; `discriminant()`, `allows_repeat()`
(`false`), and the `Delta::Ord` tie-break are otherwise as ADR-0022 Decision 3
specified.

`apply_delta`'s arm routes by the carried string, identical in shape to the
other seven keyed arms — **no key literal in `firma-kernel`**:

```rust
DeltaKind::SetAgentInt { field, value } => {
    let a = agent_of(&d.target)?;
    world.set_agent_int(a, field, *value).ok_or(KernelError::UnknownAgent(a.0))
}
```

`grep -rn "selected_action" crates/firma-kernel/src` is **empty** (doc-comment
examples and test fixtures were also neutralised — kernel fixtures use a
placeholder key).

The decide→act hand-off is `SetAgentInt { field: keys::SELECTED_ACTION, value:
action_index }`. `keys::SELECTED_ACTION` (`= "selected_action"`) in
`firma-domain` is the *only* place the name lives. `firma-plugin-testkit`
gains a single `firma-domain` dependency (`plugins ← domain`, §18.1) so its
`SelectAction` Part-F stand-in can reference the constant; that is the
testkit's only `firma-domain` use, isolated to that one rule.

Decision 1's "opaque keyed store", "reconciled state not rule-to-rule
passing", "clearing" (`act_*` rules act iff `agent_int(a, SELECTED_ACTION) ==
Some(my_index)`), "not on `FirmState`/`FirmAuxState`" and "eventual home =
§8.4 `Attention`" all stand — `SetAgentInt` is the mechanism that makes the
"opaque" part *actually* true.

### 2. The manual `Delta` `Eq` rests on an enforced non-`NaN` invariant

`DeltaKind` / `Delta` keep the **manual** `impl Eq` (the `f64` scalar payloads
block the derive; `PartialEq` stays derived). The soundness condition —
*every real-valued delta payload is finite* — is **enforced by the kernel**,
in `run_phase`'s per-delta validation loop, alongside the existing
undeclared-kind and duplicate-delta checks and therefore on the identical
pre-sort, pre-apply, abort-without-partial-effect path:

```rust
match &d.kind {
    DeltaKind::AdjustAgentReal { field, delta }
    | DeltaKind::AdjustGlobalReal { field, delta }
        if !delta.is_finite() =>
    {
        return Err(KernelError::NonFiniteDelta {
            plugin: rule.id().0.clone(),
            field: field.clone(),
        });
    }
    _ => {}
}
```

`KernelError` gains `NonFiniteDelta { plugin: String, field: String }`.
`Delta::Ord` never inspects the `f64` payload, so `sort()` is `NaN`-immune
irrespective of this check; the check is what makes the `Eq` *marker* honest
for any code that relies on reflexivity (e.g. `assert_eq!` over a `World` in
DT-4 / DT-5).

### 3. No further code change

Everything above is in the tree. This ADR changes nothing executable; it is
the correct record.

## Alternatives

- **Keep `SetSelectedAction`, move the key literal behind a `firma-core`
  constant.** The kernel would still *contain* the string and still "know"
  there is a selected-action slot. `SetAgentInt` removes the concept, not just
  the spelling.
- **A generic `MergeDomainJson`.** Already rejected in ADR-0022 (merge vs.
  additive `θ`); unchanged.
- **Newtype the `f64` payload with `total_cmp`-based `Ord`/`Eq`.** Heavier —
  touches every construction site — and does not remove the need for a finite
  invariant somewhere (a `total_cmp` `Eq` would make `NaN == NaN` true, hiding
  a bug rather than rejecting it). The boundary check is the smaller, louder
  choice and matches §21.4's "floats are suspect" posture and Stage 1's
  decision to make `P_q` an explicit required parameter rather than a silent
  default.

## Consequences

- **Positive.** The kernel is now opaque *in substance* for the hand-off, not
  just by lint. `SetAgentInt` is reusable for any future set-an-int need
  (Stage 3 uses it for `selected_action`, `focus`, `w_eff`, `prev_capital`).
  The `Eq` marker is backed by a test-visible guarantee.
- **Negative, accepted.** `firma-plugin-testkit` now depends on
  `firma-domain`. This is §18.1-permitted (`plugins ← domain`) and confined to
  one rule, but the testkit's "no domain logic" self-description now carries a
  one-line exception (stated in its crate docs).
- **Neutral.** No shipped numerical output changed — the golden trace and
  `phase1-smoke` `run_id` / `event_log_sha256` are byte-identical across the
  whole Stage-2 review (no existing config emits any Stage-2 variant).

## Compliance

- `firma-core::DeltaKind::SetAgentInt`; `DeltaKindTag::SetAgentInt`;
  `KernelError::NonFiniteDelta`.
- `grep -rn "selected_action" crates/firma-kernel/src` → empty (CI-checkable).
- `firma-plugin-testkit` `Cargo.toml` gains `firma-domain`; only
  `SelectAction` uses it.
- Tests: `firma-kernel::tests::non_finite_real_delta_is_rejected_atomically`
  (NaN and +∞, asserts the phase's valid sibling delta did not apply);
  `firma-core::delta::tests::{nan_payload_breaks_partialeq_reflexivity_but_ordering_is_immune,
  f64_payload_equality_is_reflexive_for_finite_values}`;
  `firma-plugin-testkit::tests::select_action_emits_set_deltas_for_live_agents_only`
  matches `SetAgentInt { field, value }` with `field == keys::SELECTED_ACTION`.

## Note

The lesson recorded in `docs/adr/README.md` after this: a dedicated typed
variant "for clarity" that the substrate then has to interpret is worse than a
generic one it can route blindly. When the kernel must not understand a thing,
give it nothing to understand — a `(field, value)` pair, not a named
operation.
