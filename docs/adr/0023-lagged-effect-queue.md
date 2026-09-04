# ADR 0023 — The `Λ` lagged-effect queue, `Effect`, shaping success timing, and the shaping RNG purpose tags

**Status:** Accepted (2026-09-03). The "Negative, accepted" bullet on
`b_λ` / `b_κ` parameter defaults is **superseded by ADR-0025** (2026-09-04):
those coefficients became required config with no serde default. The body below
is unchanged from acceptance (ADR bodies are append-only — `docs/adr/README.md`).
**Phase:** 2 (Model), Stage 2
**Relates to:** manual §8.1 (`Λ_{i,t}`), §9.4, §10.1 phases 5–6, §11.1 (action 4
`invest_capability`, lagged), §11.2 (shaping actions), §11.3 (mandatory
properties), §15.5 (Example E, `"shaping_lag"`), §21.2 (purpose-tag
discipline), §25.4 VT-7, ADR 0014 (precedent: pinning an underspecified
derived-value timing), ADR 0016, ADR 0021, ADR 0022

## Context

§8.1: `Λ_{i,t}` is a "Lagged-effect queue" of `(maturity_tick, Effect)` pairs.
§10.1 phase 6 (`resolve_lagged`): "Effects with `maturity_tick == t` fire."
The manual defines **neither** `Effect`'s shape **nor** when a probabilistic
shaping action's success is determined. Both are needed for
`action.shaping.rdt_standard` and `action.market.standard`'s
`invest_capability` (Stage 2). §11.3 rule 1 pins only *cost* timing ("paid at
commitment"), not when the coin is flipped.

## Decision

### 1. `Effect` — a `firma-domain` enum

```rust
pub enum Effect {
    /// `invest_capability` maturity (§11.1 action 4).
    CapabilityGain { delta: f64 },
    /// `lobby` maturity (§11.2 action 6). `applied` is fixed at commitment
    /// (Decision 2). On failure nothing is applied — the record is still
    /// enqueued so the attempt is visible in the log (§22.2).
    Lobby { applied: bool, theta_limit_delta: f64 },
    /// `contract` maturity (§11.2 action 7): raises θ_Q, adds q0 to the acting
    /// firm's obligation, fixes a `supply` edge to `source`.
    Contract { applied: bool, theta_q_delta: i64, q0: i64, source: u64 },
    /// `diversify` maturity (§11.2 action 8): a new `supply` edge from the
    /// acting firm to a fresh synthetic `source`.
    Diversify { applied: bool, source: u64 },
}
```

`Effect` is serialised to a canonical JSON string and carried in
`DeltaKind::PushAgentRecord { list: "lagged_effects", record_json }` (ADR 0022)
with a sibling `maturity_tick`. The stored Λ record is
`{"maturity_tick": u64, "effect": <Effect>}`.

**`diversify`'s "fresh source".** §16.4 forbids agent entry in the MVP, so the
new `supply` source is **not** a real agent — it is a synthetic supply-channel
id, deterministic: `source = 1_000_000_000 + agent_id * 100_000 + tick`. It
counts toward dependence concentration (§13.1) without needing an agent
record. Stage-2 simplification, recorded.

### 2. Shaping success is determined **at commitment** (phase 5), not at maturity

`p_success` (§11.2) is evaluated in `act_shaping` (phase 5) using the firm's
`λ_i` **at commitment**, the coin is flipped from the `mechanism` stream
there, and the outcome is frozen into the `Effect` (`applied: bool`).
`resolve_lagged` (phase 6) just applies whatever the matured record says.

Rationale — both readings are defensible (§11.3 does not pick), so this ADR
must, following ADR 0014's precedent:

- **"the firm's legitimacy at the time it acts"** is the natural reading of
  `λ_i` in `p_success` — the firm commits resources on the strength of its
  standing *now*, not its standing `Δ_a` ticks later.
- Determining at maturity would let a firm's legitimacy *after* it has
  committed (and paid) change the odds of an already-paid-for action — a firm
  that loses legitimacy to a `compliance` penalty between commitment and
  maturity would see its in-flight lobby's odds drop, which reads as
  retroactive and is a reason to reject the maturity option.
- `resolve_lagged` stays trivial (apply the record), and the `mechanism`
  stream is consumed once per attempt at a well-defined tick.

The lag is still real and ≥ 1 (§11.3 rule 2): the *effect* does not fire until
maturity even though its success was decided at commitment. The firm cannot
observe the outcome before maturity (no shaping feedback in the MVP, §16.4),
so commitment-time determination is invisible to the decision procedure.

### 3. RNG purpose tags — `"shaping_lag"` and `"shaping_success"`

A shaping rule draws twice in one `apply` call — the lag and the success
coin. §21.2: "a rule may draw for several distinct purposes within one apply
call and each must be an independent stream"; reusing a tag "silently
correlates draws that should be independent" (the DT-6 hazard, within one
rule this time).

- **Lag draw:** `purpose_tag = "shaping_lag"` (§11.2, §15.5 Example E),
  `mechanism` stream, `Uniform{Δ^min_a, …, Δ^max_a}`, keyed per acting agent
  via `firma_rng::open_for(&key, Some(agent), "shaping_lag")`.
- **Success draw:** `purpose_tag = "shaping_success"`, `mechanism` stream,
  `open_for(&key, Some(agent), "shaping_success")`, compared against
  `p_success`.

Both use `open_for` with the acting agent's id — the **first real per-agent
keyed RNG usage in the running kernel**. Phase 1's
`dt2b_per_agent_keyed_draw_order_irrelevant` already validated that `open_for`
is agent-processing-order-independent; that coverage **transfers** (same
`open_for` path, same key tuple). A shaping-specific determinism test is still
added (Part E, VT-7-adjacent): changing the `"shaping_lag"` draw does not
perturb the `"shaping_success"` draw and vice versa — DT-6's shape, two draws
inside one rule.

### 4. `invest_capability` uses the same `Λ` queue

§11.1 action 4 is a *market* action with a *fixed* lag (`Δ_cap = 3`, §16.1 —
**not** drawn, unlike shaping). It enqueues `Effect::CapabilityGain { delta:
δ_c }` at `maturity_tick = t + Δ_cap` into the **same** `lagged_effects` list.
One hand-off mechanism, not two. `resolve_lagged` handles every `Effect`
variant. Stage 1's `FirmaDynamics` collapses this lag to immediate for
*kernel-computation* purposes (ADR 0021 Decision 4, already documented); the
**running** implementation here is properly lagged, and the two are **not**
expected to match tick-for-tick — the kernel's stated approximation scope
covers the gap.

### 5. `resolve_lagged` — a `Rule` in the shaping plugin crate

Phase 6 currently drains nothing. A `LaggedEffectResolver` `Rule`
(`action.shaping.rdt_standard` crate, phase `ResolveLagged`): for each live
agent, read `view.agent_records(agent, "lagged_effects")`, partition into
`matured` (`maturity_tick == view.tick()`) and `pending`, emit the apply
deltas for each *matured & applied* effect, and one
`ReplaceAgentList { list: "lagged_effects", records_json: <pending> }` to
drain. It calls `firma_domain::Effect`'s pure `deltas_at_maturity` helper so
the apply logic is not duplicated. Living in the shaping crate (not the market
crate) avoids a cross-plugin dep for the `CapabilityGain` case — `Effect` is
in `firma-domain`, which both action crates depend on.

## Rejected alternatives

- **Determine success at maturity.** Rejected per Decision 2 (retroactive
  legitimacy effect; heavier `resolve_lagged`).
- **`Effect` in `firma-core`.** It names shaping outcomes (θ, `q`, edges) —
  domain vocabulary. `firma-core` stays substrate (same reasoning as ADR 0020,
  0021); `Effect` is `firma-domain`.
- **A separate lagged path for `invest_capability`.** Two mechanisms for one
  concept; rejected per Decision 4.
- **Reuse `"shaping_lag"` for the success draw** (or draw both from one
  `KeyedRng` stream sequentially). Rejected — §21.2's explicit "each must be an
  independent stream"; the DT-6 failure mode, within one rule.
- **Not enqueueing failed attempts.** Rejected — §22.2 log sufficiency: an
  analyst wants to see that a lobby was attempted, cost was paid, and it
  matured to nothing.

## Consequences

**Positive.**
- `resolve_lagged` is trivial (apply the record); the `mechanism` stream is
  consumed at one well-defined tick per attempt.
- One Λ queue for capability + all shaping; VT-7 can iterate shaping plugins
  generically.
- Two distinct purpose tags ⇒ lag and success are independent streams by
  construction (DT-6-style test, Part E).

**Negative, accepted.**
- Commitment-time success means a firm's post-commitment legitimacy changes
  cannot affect an in-flight action. This is the intended reading (Decision 2)
  but it is a modelling commitment to state in any publication about H3.
- `diversify`'s synthetic source is not a modelled agent; dependence (§13.1,
  later work) treats it as a source id only. Recorded.
- `contract` / `diversify` parameters (`κ_k`, `Δ_k`, `p^k_0`, `p^k_max`,
  `δ_Q`, `q_0`, `κ_d`, `Δ_d`, `p^d_0`, `p^d_max`) and the `b_λ`, `b_κ`
  coefficients have **no §16.1 values** — a manual gap. `lobby`'s are given
  (§16.1) and are the plugin defaults; `contract`/`diversify` params are
  required config; `b_λ = 0.2`, `b_κ = 0.1` default with a `[D]`-not-calibrated
  caveat. Flagged for a §16.1 PATCH.

**Neutral.**
- No shipped numerical output changed (Stage 2 does not wire these into any
  existing config).
- `firma-domain` gains a `serde_json` dependency so `Effect` / `LaggedRecord` /
  `Edge` own their canonical JSON wire format (the payload of `PushAgentRecord`
  / `PushGlobalRecord`). `serde_json` is already an accepted workspace
  dependency (ADR 0010) — this adds it to one more crate, not a new dependency,
  and `firma-domain` still does no other I/O.

## Compliance

- `firma-domain::{Effect, LaggedRecord}` + `Effect::deltas_at_maturity`.
- `firma-domain::RelationGraph` / `Edge` / `EdgeKind` (§8.3) — minimal:
  edge existence + `has_supply_edge` + `supply_source_count`; severance and
  dependence are later work.
- Shaping rules: `"shaping_lag"` and `"shaping_success"` tags, `open_for` per
  agent, cost delta at commitment, `Effect` enqueued at `t + lag`.
- `LaggedEffectResolver` rule, phase `ResolveLagged`.
- Tests: Part E (VT-7) — cost-on-failure, lag ≥ 1, `p ≤ p_max`, success and
  failure both occur across seeds; plus the two-tag independence test.
- Manual PATCH: §34.0 index; §16.1 gains `contract`/`diversify`/`b_λ`/`b_κ`;
  §8.1 / §10.1 phase 6 gain an `Effect` sketch or a pointer here.

## Note

ADR 0014 pinned an underspecified derived-value timing (`u`'s window) by
picking the reading most faithful to the manual's own worked example. This ADR
does the same for shaping success: "`λ_i` at the time the firm acts" is the
plain reading of §11.2's formula, and it keeps `resolve_lagged` a one-liner.
