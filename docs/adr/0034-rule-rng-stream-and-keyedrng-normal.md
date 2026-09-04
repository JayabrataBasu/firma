# ADR 0034 — `Rule::rng_stream()`, and `KeyedRng::next_normal`

**Status:** Accepted (2026-09-05). Alternatives analysis **extended** by
[ADR-0039](0039-rng-stream-registration-metadata-alternative.md), which weighs
the registration-time-metadata option this ADR omitted and rejects it — the
decision (a defaulted trait method) is unchanged.
**Phase:** 2 (Model), Stage 5
**Relates to:** manual §21.2 (the RNG key tuple — `stream_id` is a key field),
§21.3 (the four streams and the matched-environment design), §20.2 ("every
`Rule` method added now must be marshalled across a WASM boundary later —
**resist growth**"), §10.1 (phase order); ADR-0003 (the RNG scheme)

## Context

The kernel builds each rule's `RngKey` and must fill in `stream_id`. Phase 1
did this with `stream_for_phase`: the `environment` phase → the `environment`
stream, every other phase → `mechanism` (with a note that "`shock` and `init`
are for Phase 2 plugins"). Stage 5 breaks that 1:1 mapping:

- **`shock.stochastic` runs in phase 1** (`environment`, §10.1) but its draws
  are *shock timing and magnitude* — §21.3 names the **`shock`** stream for
  exactly that, and the matched-environment design ("same `environment` and
  `shock`, different `mechanism`") depends on shock draws being on that stream,
  not `environment`.
- **`resource.patchy` also runs in phase 1** and legitimately wants the
  `environment` stream ("resource dynamics").
- **`observation.noisy` runs in phase 2** (`observe`, default `mechanism`) but
  its perceptual error is exogenous environment variation that a `β` sweep
  must hold fixed ⇒ **`environment`**.

Phase 1's single stream in phase 1 cannot serve both `shock.stochastic` and
`resource.patchy`, and the kernel cannot tell them apart without domain
knowledge (matching `"shock.*"` prefixes in the kernel would violate A1).

## Decision

### 1. `Rule` gains one defaulted method

```rust
fn rng_stream(&self) -> Option<StreamId> { None }
```

`None` ⇒ the kernel's phase-derived default (`stream_for_phase`). A rule whose
§21.3 stream differs from its phase's default overrides it:

| rule | phase | default | declares |
|---|---|---|---|
| `shock.stochastic` | `environment` | `Environment` | `Some(Shock)` |
| `resource.patchy` | `environment` | `Environment` | `Some(Environment)` (explicit, = default) |
| `observation.noisy` | `observe` | `Mechanism` | `Some(Environment)` |

`shock.scheduled`, `resource.constant`, `observation.{full,delayed}` never
draw and leave it `None`.

The kernel's `run_phase` now computes the stream **per rule**
(`rng_stream_for(rule, phase)`) rather than once per phase.

**This is the 8th `Rule` method**, against §20.2's "resist growth". Justified:
the stream is a *key field* (§21.2) that determines which of four seeds a draw
uses, it is not derivable from phase for two Stage-5 rules, and a shock plugin
silently drawing from the wrong stream would break §21.3's variance-reduction
design invisibly. It marshals as one enum byte. Every Phase-1 rule is
byte-identical without touching it (`stream_for_phase` is unchanged and is the
`None` fallback).

### 2. `KeyedRng::next_normal(mean, std_dev)`

`Normal(mean, σ)` by the Box–Muller transform, consuming two `next_f64_unit()`
draws (the sine companion is discarded so the mapping is a pure function of
the two consumed uniforms — a cached companion would make the sequence
position-dependent and break CRN forks). `ln` / `cos` are transcendental:
bit-identity across libm versions is a goal not a guarantee (§21.4), exactly
as for the existing `y_O` / `y_R` floors. Added to `firma-rng` (not
`firma-domain`, which has no `firma-rng` dependency); both `observation.noisy`
and `shock.stochastic` need it. Adding a method to a concrete struct is not an
interface-breaking change in the §20.2 sense and does not touch the kernel.

## Alternatives

- **Refine `stream_for_phase` to a finer phase→stream map.** Cannot work: two
  distinct §21.3 stream needs (`shock`, `environment`) both live in phase 1.
- **A per-shock sub-seed derived by the plugin.** Breaks the §21.3 guarantee
  that the `shock` seed *is* the shock stream — a matched-environment sweep
  could no longer hold shocks fixed.
- **Let the plugin smuggle the stream into `purpose_tag`.** The `purpose_tag`
  does not select the seed (`run_seed = seeds.for_stream(key.stream)`); only
  `key.stream` does, and that is the kernel's to set.
- **Implement Box–Muller inline in each plugin.** Duplicated transcendental
  code in two crates, and a third (Stage 6 `firma_lab` bootstrap) will want it
  too. One method on the generator is the boring choice.

## Consequences

- **Positive.** `shock.stochastic` draws on the `shock` stream and
  `observation.noisy` on `environment`, so §21.3's matched-environment design
  works as written. The stream a rule uses is now explicit at the rule, not
  implicit in a phase table.
- **Negative, accepted.** `Rule` is 8 methods, not 7. The method is defaulted
  and trivial to marshal; the growth is real but the smallest that closes the
  gap.
- **Neutral.** No shipped numerical output changes — every existing rule
  returns `None`, `stream_for_phase` is unchanged, `next_normal` is called by
  nothing pre-Stage-5. Golden trace and `phase1-smoke` byte-identical.

## Compliance

- `firma_core::Rule::rng_stream` — defaulted `-> Option<StreamId>`.
- `firma_kernel::rng_stream_for(rule, phase)` — `rule.rng_stream()` or
  `stream_for_phase(phase)`; called per rule in `run_phase`.
- `firma_rng::KeyedRng::next_normal` — Box–Muller; test
  `next_normal_is_deterministic_and_roughly_standard` (CRN identity, moments,
  `σ = 0`).
- Conformance test `stage5_rng_streams_follow_the_declared_stream`: a config
  with `observation.noisy` + `shock.stochastic` is deterministic across runs,
  and changing only the `mechanism` seed leaves the drawn shock onset/magnitude
  unchanged.

## Note

Phase 1's `stream_for_phase` comment already said "`shock` and `init` are for
Phase 2 plugins" — the phase→stream map was always a placeholder for the case
where a phase hosts more than one kind of randomness. Stage 5 is that case.
