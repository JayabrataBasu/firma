# ADR 0039 — The registration-metadata alternative to `Rule::rng_stream()`, considered and rejected

**Status:** Accepted (2026-09-05)
**Phase:** 2 (Model), Stage 5 (follow-up)
**Relates to:** **ADR-0034** (which added `Rule::rng_stream()`), manual §20.2
("resist growth" on the `Rule` trait), §21.2 (the RNG key tuple), §21.3
(stream separation), §18.2 (`firma-kernel::step` operates on a resolved
`Schedule`)

## Context

ADR-0034 added an 8th method to `Rule`, `rng_stream() -> Option<StreamId>`,
so a rule whose §21.3 stream is not its phase's default (`shock.stochastic` in
phase 1 wanting `shock`; `observation.noisy` in phase 2 wanting `environment`)
can declare it. Its Alternatives section weighed a finer phase→stream map, a
plugin-derived sub-seed, and smuggling the stream into `purpose_tag` — but it
**did not weigh** the narrower option of making the override
**registration-time metadata** rather than a trait method: an
`Option<StreamId>` on `firma-registry::RegisteredRule`, supplied once when the
registry entry is constructed, so no `Rule` implementation — including future
third-party ones — carries the method unless it needs it.

This ADR fills that gap. Per the ADR-immutability house rule
(`docs/adr/README.md`) ADR-0034's body is not edited; this is the new numbered
ADR that records the analysis. (The Stage-5 follow-up instruction asked for
the entry to be added to ADR-0034's Alternatives section directly; the house
rule — owner-set, retroactive, "not even to fix a mistake the ADR itself
contains" — takes precedence, so the analysis lives here and ADR-0034's
`Status` line points at it.)

## Decision

**The registration-metadata approach is rejected. `Rule::rng_stream()` stays
on the trait.** No code change.

### Why the stream belongs on the trait, not the registration

1. **The stream is a property of the draw's *semantics*, not the deployment.**
   §21.3 assigns each stream to a *kind of work* — `shock` is "shock timing
   and magnitude", `environment` is "resource dynamics, exogenous variation".
   `shock.stochastic` draws shock timing and magnitude *by what it is*. **There
   is no valid configuration in which `shock.stochastic` should draw from
   `mechanism`** — that would be wrong in every run. A per-registered-instance
   override is the right shape only when the choice legitimately varies by
   instance; here it never does. It is a fact about the rule's code, in the
   same category as `phase()`, `reads()`, and `writes()` — all already on the
   trait, all per-rule-type invariants describing *what the rule does*, none
   of them configurable per registration. §21.2's key tuple even places
   `stream_id` immediately next to `plugin_id`: "what is drawing" includes
   "on which stream".

2. **The kernel sees `&dyn Rule`, never `RegisteredRule`.** After
   `Registry::resolve_rule` the `RegisteredRule` (id, version, content_hash,
   ctor) is consumed; only `Box<dyn Rule>` reaches `Schedule { rules:
   Vec<Box<dyn Rule>> }`, which is what `run_phase` iterates. Carrying an
   `Option<StreamId>` from `RegisteredRule` to the per-rule key derivation
   would require: `Schedule` to hold a parallel `Vec<Option<StreamId>>` (or
   pair each rule); `firma-cli::build_schedule` to look up each resolved
   rule's registered stream and thread it; the `Intervention::AddRule` path
   (which rebuilds the `Schedule` mid-run) to thread it too; and every kernel
   test fixture that calls `Schedule::new(vec![Box::new(...)], ...)` directly
   to take the new shape. That is **strictly more plumbing** than a method the
   kernel can already call on the `&dyn Rule` it already holds.

3. **Isolated unit-testability.** `firma-plugin-*` tests build an `RngKey` by
   hand and call `rule.apply(&view, key)` with no registry present.
   `rule.rng_stream()` is a single checkable source of truth —
   `firma-plugin-observation`'s test asserts
   `Noisy::new(..).rng_stream() == Some(StreamId::Environment)` directly.
   Registration metadata has no in-crate home; an isolated test would have to
   hard-code the expected stream, duplicating the fact the code should own.

4. **Where would the metadata come from at registration?** Either each
   plugin's `registered()` returns a 4-tuple `(id, hash, ctor, Option<StreamId>)`
   — the declaration is *still in the plugin crate*, just moved from a typed
   trait method into an untyped tuple slot — or `firma-cli::model_registry()`
   hard-codes `"shock.stochastic" => Some(Shock)`, which puts §21.3 stream
   *policy* for specific plugins in the CLI crate, away from the code that
   draws. Both are worse than the trait method on every axis: co-location,
   type safety, and A1-style "the composition layer holds no domain policy".

5. **The third-party burden the concern is about is near-zero.** The method
   is defaulted to `None`. An author whose rule never needs a non-default
   stream never writes it — `impl Rule for MyRule { /* seven methods */ }`
   compiles unchanged. "Resist growth" is a real constraint and this *is*
   growth, but it is the minimal growth that closes the gap: one defaulted
   method, one enum byte across the WASM boundary, sitting beside the other
   per-rule-invariant descriptors.

### What would have made the alternative correct

If the stream a rule draws from genuinely varied by deployment — e.g. if the
same rule type were legitimately registered once against `shock` and once
against `environment` in different experiments — then it would be
per-registration state and `RegisteredRule` would be its home. It does not:
§21.3's stream-to-work mapping is fixed, and a rule's work is fixed.

## Consequences

- **Positive.** The RNG-stream declaration stays co-located with the drawing
  code, type-checked, and checkable in an isolated unit test; the kernel keeps
  operating purely on `&dyn Rule`; `firma-cli` holds no per-plugin stream
  policy.
- **Negative, accepted (unchanged from ADR-0034).** `Rule` is 8 methods, not
  7. Defaulted; the smallest growth that closes the gap.
- **Neutral.** No code change. No shipped numerical output changes.

## Compliance

- `firma_core::Rule::rng_stream()` remains as ADR-0034 defined it.
- ADR-0034's `Status` line notes that its Alternatives analysis is extended
  here.
- No test change required; `stage5_rng_streams_follow_the_declared_stream`
  still passes unchanged.

## Note

Every prior trait-vs-other-mechanism call in this project was argued in an
Alternatives section (ADR-0022's `DeltaKind` expansion, ADR-0021's `Constraint`
interface). ADR-0034's omission of the registration-metadata option was a
real hole in that record. The answer is the same one those ADRs reached from
the other direction: a property that is invariant for a rule *type* and read
by the kernel belongs on the trait the kernel holds; per-run, per-instance
choices belong in config or registration. The stream is the former.
