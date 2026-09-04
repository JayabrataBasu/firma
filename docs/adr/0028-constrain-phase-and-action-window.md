# ADR 0028 — `constrain` (phase 7) does not write `θ`; it maintains the action window `W`, and `u` (like `h`) is computed on demand

**Status:** Accepted (2026-09-04)
**Phase:** 2 (Model), Stage 4
**Relates to:** manual §8.1 (action window `W`), §9.1 (`g_2 = u − θ_limit`),
§10.1 phase 7 (`constrain` — "θ updates; u and h recomputed"), §13.2 (shock
channels — all phase-1), §17 A5 (measurement is offline); ADR-0014 (`u` is the
simple `L_W` window mean), ADR-0015 (θ is global), ADR-0016 (θ deltas are
additive), ADR-0023 (matured lagged effects fire in phase 6),
ADR-0026 (`firma_domain::margin`)

## Context — Part A of the Stage-4 instruction

§10.1's phase-7 (`constrain`) row reads "**θ updates**; `u` and `h`
recomputed." But nothing in Stages 0–3 gives phase 7 a θ-write, and there is
an apparent tension: Stage 2's `LaggedEffectResolver` (ADR-0023) applies
matured `lobby` / `contract` effects — including `θ_limit += δ_θ` and
`θ_Q += δ_Q` — in **phase 6** (`resolve_lagged`), not phase 7.

### Trace: every mechanism that writes θ

| mechanism | when | this Stage? |
|---|---|---|
| Scheduled regulatory shock (`θ_limit −= m`, `θ_cap += m`, §13.2) | **phase 1** (`environment`, "scheduled shocks fire") | no — Stage 5 |
| Matured `lobby` effect (`θ_limit += δ_θ`) | **phase 6** (`resolve_lagged`, §10.1: "Effects with `maturity_tick == t` fire"; ADR-0023) | built (Stage 2) |
| Matured `contract` effect (`θ_Q += δ_Q`) | **phase 6** | built (Stage 2) |
| Per-firm θ regimes | — | **Phase 4** (ADR-0015); not in the MVP |
| θ decay / mean-reversion toward a baseline | — | **not in the manual** — θ is a walk driven only by shocks + shaping, no reversion |

There is **no fourth θ-writer**, and no θ-writer that is neither a phase-1
shock nor a phase-6 matured effect. By the time phase 7 begins, both of θ's
sources for the tick have fully settled (phase 1 precedes phase 6 precedes
phase 7).

## Decision

### 1. Phase 7 writes nothing to θ. `LaggedEffectResolver` stays in phase 6.

§10.1's "θ updates" in the phase-7 row **describes θ's state as of phase 7**
— already complete — not a write phase 7 performs. `LaggedEffectResolver` is
**correct** in phase 6: §10.1 phase 6 is *definitionally* "Effects with
`maturity_tick == t` fire", the Λ queue drain belongs there by the manual's
own words, and phase 6 has no other mechanism. Moving it to phase 7 would
leave phase 6 empty and would not match the manual any better. **No ADR
supersedes ADR-0023; `LaggedEffectResolver` does not move.**

A matured `lobby` shifts `θ_limit` in phase 6; phase 8 (`enforce`) then
checks `g_2 = u − θ_limit` against that shifted value the same tick — the
correct ordering (shock → … → matured shaping → constrain → enforce reads
final θ).

### 2. `u` (like `h`) is computed on demand, not persisted. Phase 7's concrete job is `W` maintenance.

ADR-0014 defined `u` as `(1/L_W) · Σ ι(action)` over the last `L_W` actions
and permitted (not required) caching it on `aux`, "MUST equal the value
recomputed from `W` bit-for-bit." The **extreme faithful case of that rule is
no cache**: `u` is a pure function of `W`, so every consumer recomputes it.

- `firma-domain::margin::u_from_window(window: &[WindowEntry], l_w: usize) ->
  f64` — the single source of ADR-0014's formula.
- `firma-plugin-constraint`'s `enforce` rule (Stage 4, `g_2` check) and
  `firma-plugin-decision`'s `Satisficing` (`h` in §12.3 Step 1) both build
  their `FirmAuxState { regulated_intensity: u_from_window(W, l_w), … }` — they
  no longer read a persisted `keys::REGULATED_INTENSITY` after tick 0.
- This mirrors `h`, which §17 A5 already forbids the kernel from storing:
  every consumer computes `h` fresh via `firma_domain::margin::standard_margin`.
  `u` gets the same treatment — it is a decision/enforcement *input* computed
  by a plugin, never a value the kernel stores and serves. `keys::REGULATED_
  INTENSITY` remains a valid key for **seeding** an initial `u` (config,
  ADR-0030) and for a future Stage that wants a cache, but Stage 4 writes it
  nowhere.

So `constrain` (phase 7)'s only job is: **append this tick's selected action
to `W`, trim `W` to `L_W` entries.**

### 3. `W` is a per-agent keyed list — no new storage path

`W` lives under `keys::ACTION_WINDOW` in the existing `agent_lists` store
(ADR-0022). Each `constrain` tick, per live agent:

- read `W = view.agent_records(agent, keys::ACTION_WINDOW)`;
- the action is `view.agent_int(agent, keys::SELECTED_ACTION)` — set in phase 3
  (`decide`), reconciled, valid in phase 7 (ADR-0022's reconciled-state rule).
  A firm with no decide-rule (no `selected_action`) is treated as having held;
- push `WindowEntry { tick, action }` and, if the result exceeds `L_W`, drop
  the oldest — emitted as **one `ReplaceAgentList { list: ACTION_WINDOW,
  records_json: <trimmed window> }`** per agent. `ReplaceAgentList` is
  `Agent`-target, `set`-list semantics, `allows_repeat() == false` → one per
  agent per phase per list; the `constrain` rule is the only phase-7 writer of
  `ACTION_WINDOW` and emits exactly one, so the §19.5 uniqueness check is
  satisfied. (`LaggedEffectResolver`'s `ReplaceAgentList` targets a different
  list — `lagged_effects` — and a different phase — 6.)

**`constrain` needs no new `DeltaKind`.** `ReplaceAgentList` and
`PushAgentRecord` (ADR-0022) are reused as-is.

`constrain` is a `Rule` in `firma-plugin-constraint` (phase `Constrain`),
alongside the four `Constraint` trait impls and the Stage-4 `enforce` rule —
`firma-plugin-constraint` becomes "the four §9.1 constraints, end to end:
their `g_j`, their violation semantics, and the phase-7/8 rules that maintain
their inputs and apply their consequences."

## Alternatives

- **Give phase 7 a θ-writing `Rule`.** Nothing for it to write — traced above.
- **Move `LaggedEffectResolver` to phase 7** to match "θ updates" literally.
  Rejected: contradicts phase 6's definition, empties phase 6, and the
  `CapabilityGain` effect it also resolves has nothing to do with θ.
- **Persist `u` via a `SetAgentReal` `DeltaKind`** (the `set` sibling of
  `AdjustAgentReal`, parallel to `SetAgentInt`). Considered — `constrain` needs
  to overwrite `u` with a freshly computed absolute value, and `AdjustAgentReal`
  only adds. Rejected because computing `u` on demand from `W` (like `h`) is
  simpler, removes a `DeltaKind`, and is what ADR-0014's "cache MUST equal
  recompute" already licenses. If a later Stage needs the cache for
  performance, `SetAgentReal` gets its own ADR then.
- **A dedicated `W` component / storage.** The keyed `agent_lists` store is
  exactly the right shape; ADR-0022 built it for this.

## Consequences

- **Positive.** Phase 7 is one small rule. `u` and `h` are handled the same
  way (on-demand, plugin-computed, A5-clean). No `DeltaKind` growth.
- **Negative, accepted.** Every `h` / `g_2` computation now re-scans `W` (≤ 16
  entries) and re-parses its JSON. Negligible; and it is the price of not
  having a cache that can drift.
- **Neutral.** No shipped numerical output changes — no config runs the
  `constrain` rule yet; golden trace and `phase1-smoke` unaffected.

## Compliance

- `firma-domain::margin::u_from_window`; `firma-domain::window::WindowEntry`.
- `firma-plugin-constraint::Constrain` — phase `Constrain`, emits one
  `ReplaceAgentList { list: keys::ACTION_WINDOW }` per live agent.
- `Satisficing` and `enforce` build `regulated_intensity` from
  `u_from_window`, not from a persisted read.
- Tests: `u_from_window` matches ADR-0014's formula (zero-padded early window,
  `k/L_W` grid); `Constrain` appends + trims to `L_W`; a two-tick trace where
  a `produce_regulated` choice moves `u` and then ages out of the window.
