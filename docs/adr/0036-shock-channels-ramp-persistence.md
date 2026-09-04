# ADR 0036 — Shock: channel deltas, ramp/persistence formulas, one plugin per config

**Status:** Accepted (2026-09-05)
**Phase:** 2 (Model), Stage 5
**Relates to:** manual §13.2 (the `Shock` object + channel table), §13.3
(novelty N1/N2), §8.3 (`e_t` includes `Σ_t`), §9.4 (θ is the shared
threat/response surface), §10.1 (phase 1 `environment`), §21.3 (`shock`
stream), §22.2 (log sufficiency), §16.4 (no anticipation); ADR-0016 (θ moves
are additive), ADR-0023 (`Effect::Lobby` emits `AdjustGlobalReal` on
`theta_limit`), ADR-0029 (`ReplaceGlobalList`), ADR-0034 (`rng_stream`)

## Context

§13.2 gives the `Shock` struct and a channel table, but **not** the per-tick
magnitude functions for `ramp` (`Instant | Linear(d) | Exponential(rate)`) or
`persistence` (`Transient(d) | Permanent | Recurring(period)`), and not how a
level-shift channel (`θ_limit -= m`) composes over many ticks. The surrounding
§13.2 text adds only that the regulatory channel matters most and shares θ's
surface with `lobby`. So the formulas are **designed, flagged `[D]`** — the
same posture as `b_λ` / `P_q` / the `patchy` mechanic.

## Decision

### 1. Channel deltas — the same mechanisms the rest of the model uses

| channel | world delta | emitted as |
|---|---|---|
| `Resource` | `π^I += m` | `AdjustGlobalInt { input_price }` (Global) |
| `Regulatory { ThetaLimit }` | `θ_limit -= m` | `AdjustGlobalReal { theta_limit }` (Global) — **identical to `Effect::Lobby`** (§9.4) |
| `Regulatory { ThetaCap }` | `θ_cap += m` | `AdjustGlobalReal { theta_cap }` (Global) |
| `Competitive` | `π^O -= m` | `AdjustGlobalInt { output_price }` (Global) |
| `Reputational` | `λ -= m` per targeted firm | `AdjustAgentReal { legitimacy }` per targeted live agent |

`Regulatory` names which θ field it moves (`RegulatoryTarget`), since the
table lists both. **No new DeltaKind.**

**The Reputational "`supply` weights reduced" companion effect is deferred.**
Nothing in the running model reads `Edge.weight` (only the not-yet-built
§13.1 dependence metric would). Reducing it would be inert. Trigger to build
it: §13.1 dependence landing with a live consumer.

### 2. Deltas are incremental, tracked against a cumulative target

A shock has a **target cumulative shift** `S(t) ≥ 0` in magnitude units
(`Shock::effective_shift`); its channel applies it with a sign
(`Shock::channel_sign`). Each tick the rule emits only the *step*
`S(t) − S(t−1)`, storing the shift applied so far in the shock's
`ACTIVE_SHOCKS` record (`applied_real` for θ/λ, `applied_int` for prices —
the `i64` channels round the *cumulative* target each tick so rounding never
drifts). A level-shift channel therefore moves the world once (or ramped) and
then goes silent; it does not re-subtract `m` every tick.

### 3. Ramp / persistence formulas (`[D]`)

**Ramp factor** (`0` before onset; `dt = t − onset`):
- `Instant` → `1`.
- `Linear(d)` → `clamp(dt / d, 0, 1)` — `0` at onset, `1` at `onset + d`.
- `Exponential(rate)` → `1 − exp(−rate·dt)` — **read as ramp-*up*** (the name
  is ambiguous; this is the stated reading), asymptotic, `rate > 0` the
  approach speed. A "shift then fade" reading is served by `Transient`.

**Persistence:**
- `Transient(d)` → `S(t) = m · ramp_factor(dt)` while `dt < d`; at `dt = d`
  the window is closed, `S = 0`, and the rule emits **one reversing step**
  `−applied`; the shock leaves `Σ_t` the following tick.
- `Permanent` → `S(t) = m · ramp_factor(dt)` for all `dt ≥ 0`; stays in
  `Σ_t` for the run.
- `Recurring(period)` → **staircase** (stated reading): `S(t) = m · (1 +
  ⌊dt / period⌋)` — a fresh `m` step every `period` ticks, permanently, ramp
  ignored (each step instant). Models repeated tightening, the §13-relevant
  case. The one-tick-spike alternative is noted and rejected.

Reversal on `Transient` is exact for `Regulatory` (θ is moved only by shocks
and `lobby`, both additive and tracked) and for the unclamped `i64` price
channels. For `Reputational` the reversal is by the same nominal amount; if
`λ` was clamped to `[0, 1]` or moved by `enforce` in between, the restoration
is approximate — an accepted `[D]` imprecision.

### 4. Observability composition

`Shock.observability` (`Full | Delayed | Noisy | Hidden`) is the **shock's
own** channel, distinct from the general `Observation` plugin (ADR-0035).
They compose by **layering**: a shock's move lands in the *true* global store
(step 1), then whatever `Observation` plugin is configured perturbs the
firm's *view* of that store. So a `Hidden` regulatory shock still changes the
true `θ_limit`; whether a firm perceives the change is governed by the
`Observation` plugin, not by `observability`. This field is **logged, not
consumed** this Stage — the MVP decision procedure reads only the
`Observation`-plugin view. Building shock-specific observability into `decide`
would double the perceptual machinery for little gain and is deferred.

### 5. `Σ_t`, novelty, and events

- `Σ_t` is the `ACTIVE_SHOCKS` global list — each entry the full `Shock` plus
  the `applied_*` bookkeeping — rewritten (`ReplaceGlobalList`) only when it
  changes. Written for offline **novelty N1/N2** (§13.3): N1 needs the shock's
  parameter vector `z` (the whole `Shock` is logged) and the firm's memory
  `M`; **the memory ring buffer stays deferred** — N1 is offline (§14.1, same
  as `h`) and `M` is reconstructable from the event log's shock and outcome
  records. Nothing online consumes novelty.
- **No `Event::ShockFired`.** A shock is fully visible in the log through its
  `DeltaApplied` events (origin = the shock plugin) plus the `ACTIVE_SHOCKS`
  rewrites — sufficient for §22.2 reconstruction. Adding a dedicated event is
  a `firma-core` / `firma-io` change with no reconstruction need.

### 6. One shock plugin per config

`shock.scheduled` and `shock.stochastic` each **own** the single
`ACTIVE_SHOCKS` list. Two of them rewriting it in one phase under Jacobi
semantics would clobber each other (neither sees the other's step;
last-writer-wins). So **a config uses at most one shock plugin** — the same
rule as "at most one decision plugin". A future `shock.composite` combines a
schedule and a stochastic source in one rule if a run needs both.

`shock.stochastic` draws its `onset` (`Uniform{min..=max}`) and `magnitude`
(`|Normal(mean, sd)|`) **once, at tick 0** (`key.tick == 0`), from the
**`shock`** stream (§21.3, ADR-0034); the concrete shock then lives in
`ACTIVE_SHOCKS` and every later tick reuses it.

## Alternatives

- **Apply the full `θ_limit -= m` every tick while active.** Crashes θ to
  `−∞`. The channel is a level shift, not a rate.
- **A dedicated `Event::ShockFired`.** Kernel/IO surface growth for a
  reconstruction need the delta log + `ACTIVE_SHOCKS` already meet.
- **Per-owner `ACTIVE_SHOCKS` sublists so both plugins can coexist.** Adds a
  key per plugin and Jacobi-merge complexity for a combination ("both shock
  plugins at once") that has no use case the single-plugin rule doesn't cover.
- **Build the memory ring `M` now for online novelty.** Nothing online reads
  novelty (§14.1); the ring is offline-reconstructable. Deferred with a
  trigger.

## Consequences

- **Positive.** All four §13.2 channels drive the true model; the regulatory
  channel uses the exact `lobby` mechanism, so the Phase-4 two-sided version
  is coherent (§13.2). Ramp/persistence give a real onset-shape manipulation.
  `ReplaceGlobalList` reused (ADR-0029) — no new primitive.
- **Negative, accepted.** The ramp/persistence formulas and the
  incremental-with-reversal model are invented; each is stated in
  `firma_domain::shock` rustdoc as `[D]`. `Reputational` `Transient` reversal
  is approximate under clamping.
- **Neutral.** No new DeltaKind. No shipped numerical output changes (no
  golden/`phase1-smoke`/`phase2-smoke` config runs a shock plugin).

## Compliance

- `firma_domain::shock` — `Shock`, `ShockChannel`, `Ramp`, `Persistence`,
  `ShockObservability`, `ShockTargets`, `Shock::{effective_shift, in_sigma,
  channel_sign, validate}`; tests for each ramp/persistence case.
- `firma_domain::keys::ACTIVE_SHOCKS`.
- `firma-plugin-shock` — `Scheduled` / `Stochastic`, phase `Environment`;
  `Stochastic::rng_stream() == Some(Shock)`, draws once at tick 0; incremental
  channel steps; `Transient` reversal; one `ReplaceGlobalList` per phase only
  on change. Module docs state "one shock plugin per config".
- Tests: `firma-plugin-shock` (instant/linear/transient/recurring, resource
  rounding, reputational targeting, stochastic draw determinism + window) and
  conformance `phase2_stage5_smoke_*` / `stage5_rng_streams_*`.

## Note

The channel table is the one part of §13 that reads like a spec, and it still
leaves the two hardest questions — "how does a one-line `-= m` behave over 400
ticks" and "what does `Linear(4)` actually compute" — to the implementer. The
incremental-shift model is the answer to the first; it is the only reading
under which `Permanent` and `Transient` and a ramp all compose without the
world state running away.
