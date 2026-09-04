# ADR 0035 — The `Observation` interface: a phase-2 per-agent env/θ snapshot

**Status:** Accepted (2026-09-05)
**Phase:** 2 (Model), Stage 5
**Relates to:** manual §12.2 (observation — own state exact, env/θ through a
plugin: `full` / `noisy(σ)` / `delayed(k)`), §10.1 (phase 2 `observe` runs
before phase 3 `decide`), §16.4 (no perception error about own viability),
§21.3 (stream separation); ADR-0022 (the opaque keyed store), ADR-0029
(`ReplaceGlobalList`), ADR-0034 (`rng_stream`)

## Context

§12.2 gives the three `Observation` variants' names and one parameter each,
not a Rust interface. Two questions had to be settled:

1. **Does the decision procedure read observation *through* an `Observation`
   plugin, or does `Observation` write an "observed environment" the decision
   procedure then reads from the store?** Traced against §10.1: `observe` is
   phase 2, `decide` is phase 3, and §18.1 forbids one plugin calling another.
   A plugin-to-plugin call is out; the only architecture-consistent design is
   **`Observation` writes, `decide` reads**.
2. **The firm observes its own `x` and its own `h` exactly, always** (§12.2,
   §16.4). The `Observation` plugin must be unable to touch those.

## Decision

### 1. Each `Observation` rule runs in `observe` and writes a per-agent snapshot

`observation.{full,noisy,delayed}` are `Rule`s with `phase() == Observe`. Each
writes, per live agent, **one** `firma_domain::EnvSnapshot` — `(tick, π^I,
π^O, θ_limit, θ_cap, θ_Q)` — under `keys::OBSERVED_ENV` via `ReplaceAgentList`
(overwrite, not append). The snapshot carries only the fields the decision
procedure consults. It does **not** carry `x`, `h`, or `Σ_t`: own-state and
own-margin are read directly and exactly by `decide` (they never route through
this key), and reasoning about *which* shocks are active would be a form of
anticipation (§16.4).

### 2. `decide` reads the snapshot, with a byte-identical fallback

`decision.{satisficing,random}`'s `theta()` / `env_params()` helpers now take
the agent and read `keys::OBSERVED_ENV` first, **falling back field-by-field
to the true global store** when the key is absent:

```rust
theta_limit: observed.map(|o| o.theta_limit)
    .or_else(|| view.global_real(keys::THETA_LIMIT)).unwrap_or(0.0)
```

A run with **no `Observation` plugin** leaves `OBSERVED_ENV` unset ⇒ the
fallback path ⇒ **byte-identical** to a pre-Stage-5 run (verified: golden,
`phase1-smoke`, and the Stage-4 `phase2-smoke` `run_id` / `event_log_sha256`
all unchanged). This is the one place Stage 5 touches `decide`; it is a
behaviour-preserving read redirection, not a change to the decision logic.

### 3. The three variants

| variant | params | RNG | behaviour |
|---|---|---|---|
| `full` | none | none | snapshot = true env. Explicit and registered (like `resource.constant`) so the observation channel is in the log even when lossless. |
| `noisy` | `sigma` **required** | `Normal(0, σ)` per firm, **`environment`** stream, tag `obs_noise` | each observed field += independent draw; prices rounded back to `i64`. |
| `delayed` | `k` **required** | none | maintains a global `keys::ENV_HISTORY` ring of the last `k+1` true snapshots (`ReplaceGlobalList`); every firm sees `history[len-1-k]`, or the oldest available while the ring is still filling. |

`σ` and `k` have **no §16.1 default** and are required config — the same
discipline as `P_q`, `b_λ`, `b_κ`.

**Why `noisy` draws from the `environment` stream (ADR-0034):** the perceptual
channel is exogenous variation the firm *faces*. §21.3's matched-environment
design varies `mechanism` while holding `environment` and `shock` fixed; a
sweep over `β` must see the *same* observation noise, so it belongs on
`environment`, not `mechanism`.

### 4. `Σ_t` and the own-state caveat

`Σ_t` (active shocks) is logged separately under `keys::ACTIVE_SHOCKS` by the
shock plugin (ADR-0036), not in the snapshot. Each variant's `assumption()`
and rustdoc state explicitly that the lossy behaviour applies to
**environment and θ only** — `x` and `h` are always exact (§12.2, §35.1's
recorded non-transfer).

## Alternatives

- **`Observation` produces a view object `decide` reads through.** Requires
  `decision.*` to hold a reference to an `Observation` plugin — a plugin-to-
  plugin dependency (§18.1). Rejected.
- **Overwrite the true global `θ` keys with the observed values.** Corrupts
  shared state, and it is global where observation is per-firm. Rejected.
- **A new `SetAgentReal` DeltaKind for the observed θ scalars.** `ReplaceAgentList`
  with a one-record JSON list already gives overwrite semantics for a bundle
  of fields; a new variant for this is unwarranted (ADR-0028 reached the same
  conclusion about `u`).
- **Skip `full` (absence of an Observation plugin = full).** True functionally,
  but leaves "the firm sees everything" as an unstated default rather than a
  manifest-recorded choice.

## Consequences

- **Positive.** `noisy` and `delayed` genuinely change what firms decide on,
  and the observed environment is in the event log for offline "what could the
  firm have known" analysis. The interface is uniform across the three.
- **Negative, accepted.** One code change to `decision.*` this Stage (the
  read redirection), and three new keys (`OBSERVED_ENV`, `ENV_HISTORY`, and —
  ADR-0036 — `ACTIVE_SHOCKS`).
- **Neutral.** No new DeltaKind. No shipped numerical output changes for
  configs without an `Observation` plugin.

## Compliance

- `firma_domain::EnvSnapshot`; `firma_domain::keys::{OBSERVED_ENV, ENV_HISTORY}`.
- `firma-plugin-observation` — `Full` / `Noisy` / `Delayed`, phase `Observe`;
  `Noisy::rng_stream() == Some(Environment)`; `sigma` / `k` required.
- `firma_plugin_decision::{theta, env_params}` take `agent` and prefer
  `OBSERVED_ENV`; `observed_env` returns `None` ⇒ true-store fallback.
- Tests: `firma-plugin-observation` (lossless `full`, per-field per-agent
  noise + CRN determinism, `delayed` lag + ring, early-tick oldest-available,
  required params); conformance `phase2_stage5_smoke_*` (a `delayed(2)` lag is
  observed end to end) and `stage5_rng_streams_*`.

## Note

The phase order is the whole argument. `observe` sitting before `decide` in
§10.1, plus "plugins never call each other" from §18.1, leaves exactly one
shape: observation is a write in phase 2, decision is a read in phase 3, and
the store is the channel between them.
