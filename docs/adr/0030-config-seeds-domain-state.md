# ADR 0030 — A run config can seed global and per-agent domain-state values

**Status:** Accepted (2026-09-04)
**Phase:** 2 (Model), Stage 4
**Relates to:** manual §8.2 (θ), §8.3 (prices), §12.1 (aspirations), §16.1
(defaults), §20.4 (config is data); ADR-0002 (config must be data — no
conditionals/loops), ADR-0015 (θ is global and MUST be set by config),
ADR-0022 (the kernel's opaque keyed store), ADR-0028

## Context

Stage 4 runs the model end to end for the first time. `decision.satisficing`
and `enforce` read `θ_limit` / `θ_cap` / `θ_Q` from the global store, the
market rules read `π^I` / `π^O`, and a firm needs an initial `capability`,
`legitimacy`, `obligation`, and three aspiration levels. `World` has had `pub`
seed methods (`set_global_real` / `set_global_int` / `set_agent_real` /
`set_agent_int_value`) since Stage 2 "for the orchestrator to call from
config" — but the config schema exposes only per-agent conserved `stocks` and
the environment pool's `stocks`. There is nowhere in a config file to put
`θ` or `capability`.

## Decision

`firma-config` gains four optional, `#[serde(default)]` maps. Existing configs
(no new keys) parse unchanged.

```jsonc
"world": {
  ...,
  "global_reals": { "theta_limit": 0.90, "theta_cap": 0.40 },   // → World::set_global_real
  "global_ints":  { "theta_q": 100, "input_price": 2, "output_price": 3 }  // → World::set_global_int
},
"agents": [
  { "id": 0, "stocks": { "capital": 400, "input": 8 },
    "reals": { "capability": 0.5, "legitimacy": 1.0,             // → World::set_agent_real
               "aspiration_capital_growth": 3.0,
               "aspiration_capability": 0.7,
               "aspiration_obligation_clearance": 0.0 },
    "ints":  { "obligation": 2 } }                               // → World::set_agent_int_value
]
```

- Keys are **opaque strings** — `firma-config` does not know `"theta_limit"`
  means anything, exactly as `firma-kernel` does not (ADR-0022). The
  `firma-domain::keys` constants are the shared vocabulary; a typo in a config
  is a mis-seed, caught by the run behaving wrongly, not a schema error. (A
  future `firma-config` could validate keys against a registry; out of scope.)
- `#[serde(deny_unknown_fields)]` still holds on `WorldConfig` / `AgentConfig`
  — the four new fields are *known*; unknown siblings are still rejected.
- The orchestrator's `build_world` calls the matching `World::set_*` for every
  entry, after constructing agents and before the first tick. Order within a
  map is `BTreeMap` order (deterministic); order between maps is fixed
  (globals then per-agent, ascending agent id).
- Every new field is `#[serde(default, skip_serializing_if =
  "BTreeMap::is_empty")]`. An **empty** map serialises to nothing, so a config
  that omits all four canonicalises **byte-identically** to a pre-ADR-0030
  config — the golden trace and `phase1-smoke` `run_id` (SHA-256 over the
  re-serialised canonical JSON) are provably unaffected. A config that *uses*
  the fields hashes over them, so its manifest records exactly what was seeded.
- **`schema_version` stays `1.0.0`.** The change is purely additive and
  optional; no existing config's meaning changes. (If the project later wants
  strict schema-version gating on additive changes, that is a separate
  decision.)

Shocks (Stage 5) will *modify* these config-seeded baselines in the
`environment` phase — the config is the `t = 0` state, not a per-tick source.

## Alternatives

- **A `seed` / `env.constant` `Rule`** that emits `AdjustGlobalReal` etc. at
  tick 0. Rejected: initial conditions are config's job (§20.4), not a
  plugin's; a seed-rule would need ripping out when the real shock/environment
  plugin lands, and it clutters the event log with tick-0 setup deltas.
- **New `Intervention` variants** (`SetGlobal`, `SetAgentReal`) fired at tick
  0 via the `interventions` array. Rejected: interventions are the
  *counterfactual* `do(·)` machinery (§6.5); using them for initial conditions
  conflates "what the run started as" with "what we changed about it".
- **One `serde_json::Value` map** instead of typed real/int maps. Rejected:
  `Value` re-introduces the `f64`/`NaN` and `Eq` problems ADR-0022/0024 spent
  effort avoiding, and blurs whether `100` is `theta_q` (int) or a real.

## Consequences

- **Positive.** A config fully describes a runnable world. The Stage-4 smoke
  config, and every Phase-2 experiment config, can now set θ / prices /
  aspirations declaratively.
- **Negative, accepted.** `firma-config` — a Phase-1 crate — grows four
  fields and `build_world` grows ~8 lines. Behaviourally inert for existing
  configs (verified: golden trace + `phase1-smoke` byte-identical).
- **Neutral.** No new dependency.

## Compliance

- `firma_config::WorldConfig::{global_reals, global_ints}`;
  `firma_config::AgentConfig::{reals, ints}` — all `BTreeMap<String, _>`,
  `#[serde(default)]`.
- `firma_cli::orchestrator::build_world` seeds them via the existing
  `World::set_*` methods.
- Tests: a config with the new fields round-trips and seeds a `World` whose
  `global_real` / `agent_int` reads return the config values; a config without
  them is unchanged (golden trace green).
