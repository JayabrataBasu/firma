# ADR 0029 — The `enforce` phase (§10.1 phase 8): violation detection and processing

**Status:** Accepted (2026-09-04). Decision 2's *ordering rationale* ("`AdjustStock`
disc 0 sorts before `RemoveAgent` disc 9") is superseded by
[ADR-0032](0032-removeagent-applied-last-in-a-phase.md) — the mechanism it
describes (transfers first, then removal, conservation intact) is unchanged;
only the reason it happens is corrected. All other decisions stand.
**Phase:** 2 (Model), Stage 4
**Relates to:** manual §9.1 (the four violation semantics), §10.1 phase 8
(`enforce` — "Violations processed; deaths recorded"), §16.1 (`T_c=4`,
`P_c=30`, `δ_λ=0.15`; `P_q` has **no** §16.1 value), §19.2 (deterministic
agent iteration), §19.5 (invariants), §22.2 (log sufficiency); ADR-0017
(atomic death + incident-edge removal), ADR-0021 (`scope` is
admissibility-only; `obligation` severs `supply` edges), ADR-0026
(`firma_domain::margin`), ADR-0028

## Context

§9.1 fixes *what* each violation means; it does not fix the mechanics of
detecting and applying them in one phase across many agents with possibly
simultaneous violations. The `enforce` phase is a `Rule` (A1 — the kernel
holds no §9.1 policy), so violation *consequences* must be expressed as
`Delta`s. Two consequences have no existing `Delta` shape: **removing an
agent** and **rewriting the global relation-edge list**.

## Decision

### 1. Two new `DeltaKind` variants

| # | disc | variant | target | semantics |
|---|---|---|---|---|
| 9 | 9 | `RemoveAgent { reason: String }` | Agent | remove the agent record and its per-agent keyed state; **no** edge logic, **no** conservation rebase |
| 10 | 10 | `ReplaceGlobalList { list: String, records_json: Vec<String> }` | Global | replace a named global list wholesale (the `Global` sibling of `ReplaceAgentList`) |

- `slot()`: `RemoveAgent` → `"agent"` (constant); `ReplaceGlobalList` → its
  `list`. `allows_repeat()` → `false` for both. `DeltaKindTag` gains
  `RemoveAgent`, `ReplaceGlobalList`. Neither carries an `f64`, so ADR-0024's
  non-finite check is unaffected.
- `apply_delta`: `RemoveAgent` → `World::remove_agent_and_keyed_state(a)`
  (drops the `agents` entry **and** the agent's `agent_reals` / `agent_ints` /
  `agent_lists` subtrees, rebuilds the live cache); `ReplaceGlobalList` →
  `World::replace_global_list(list, records)`.
- `run_phase`, on applying a `RemoveAgent`, also pushes `Event::AgentDied {
  tick, agent, cause: reason }` — a structural lifecycle event, symmetric with
  the `AgentBorn` that `step()` already emits. This is the only kernel change
  that *reads* a `DeltaKind`'s payload, and it reads only `reason` as an
  opaque string for the log.

**Edge removal is the `enforce` rule's job, not the kernel's** (ADR-0017's
Compliance said "the kernel's agent-removal path", but the kernel parsing
`Edge` JSON to find incident edges would be domain logic in the kernel — A1).
The `enforce` rule, which already parses `firma_domain::Edge`, computes the
surviving edge set and emits **one** `ReplaceGlobalList { list:
keys::RELATION_EDGES }` for the whole phase (see §4). `RemoveAgent` then only
removes the agent and its own keyed state. ADR-0017's *mechanism* (atomic
removal + all incident edges gone in the same reconciliation step, no
damping) is preserved — the `ReplaceGlobalList` and the `RemoveAgent`s are all
in the phase-8 reconciliation, applied together.

### 2. Conservation on death — stocks are transferred, not vanished

A dead firm may hold `r^I > 0` (and, for a `compliance` death, `r^L > 0`).
`World::remove_agent_and_keyed_state` does **not** rebase conservation. Instead
the `enforce` rule, for each dying agent, emits **paired `AdjustStock` deltas
transferring every remaining stock to the environment pool** (the market
reabsorbs a failed firm's assets), then the `RemoveAgent`. Because
`AdjustStock` (disc 0) sorts before `RemoveAgent` (disc 9), the transfers
apply first; `RemoveAgent` then removes an agent with zero stocks and
conservation is untouched — and the event log records exactly where the
resources went (§22.2). A `RemoveAgent` on an agent with non-zero stocks would
trip the phase-8 conservation invariant — a loud failure, as intended.

### 3. The four semantics — dispatch on `ViolationSemantic`

The `enforce` rule builds the four `Constraint` objects (in-crate) and, per
live agent, evaluates `firma_domain::margin::all_g(&ctx)` (canonical order
`[solvency, compliance, scope, obligation]`) paired with each constraint's
`violation()`:

```
for (g_j, semantic) in all_g(ctx).zip(constraints.map(|c| c.violation())):
    let violated = match semantic {
        Death        => g_j >= 0.0,   //  see below — the ≥ is deliberate
        _            => g_j > 0.0,
    };
    if !violated { continue; }
    match semantic {
        Death                       => mark the agent lethal (cause "solvency")
        Graduated { penalty, window_ticks, legitimacy_loss }
                                    => graduated handling (§3a)
        AdmissibilityGate           => { /* explicit, deliberate no-op —
                                           scope is checked in `decide`
                                           (ADR-0021), never here */ }
        Relational { penalty }      => sever supply edges + penalty (§3b)
    }
```

The `AdmissibilityGate` arm is an **explicit empty branch with a comment**,
not a fallthrough — `scope` violations are real (a firm below `θ_cap` cannot
`produce_regulated`) but their entire consequence is admissibility, applied
in phase 3, and `enforce` must be seen to do nothing with them.

**Why `Death` fires at `g_1 ≥ 0`, not `g_1 > 0`.** `g_1 = −r^L` and the
non-negativity invariant (§19.5) makes `r^L ≥ 0` always, so `g_1 > 0` is
**unreachable** — a strict check would make the `solvency` `Death` semantic
dead code. §9.1's own gloss and the `solvency` plugin's `assumption()` ("a
firm that runs out of spendable capital is removed") make `r^L == 0` (i.e.
`g_1 == 0`) the insolvency condition. The `Death` semantic's boundary state is
*itself* fatal ("removed"); the other three treat `g_j == 0` as the last safe
state (consequences begin on crossing). So `Death ⇒ g_j ≥ 0`, everything else
`⇒ g_j > 0`. This is the only asymmetry and it is keyed on the declared
semantic, not on the constraint id. `compliance` (declared `Graduated`, even
though its second strike is lethal) uses `g_2 > 0`, matching "intensity
**above** the permitted limit".

`decision.satisficing` / `standard_margin` already treat `r^L == 0` as
`h ≤ 0` (`g_1/s_1 = 0`, so `−max_j ≤ 0`) ⇒ SURVIVAL focus — consistent: a firm
at the solvency boundary is in survival mode this tick and removed at
`enforce`.

#### 3a. `compliance` — graduated, with a persisted last-violation tick

State: `keys::COMPLIANCE_LAST_VIOLATION_TICK` (per-agent int), written by
`enforce` via `SetAgentInt`.

- **No prior violation, or prior violation more than `T_c` ticks ago** ⇒
  *first strike*: penalty `min(P_c, r^L)` (paired `AdjustStock`
  agent↔environment — a firm cannot be fined more than it has; if this leaves
  `r^L == 0` it dies of `solvency` **next** tick, Jacobi); `λ ←
  max(0, λ − δ_λ)` via `AdjustAgentReal { LEGITIMACY, −min(δ_λ, λ) }` (skipped
  if `λ == 0`); `SetAgentInt { COMPLIANCE_LAST_VIOLATION_TICK, t }`.
- **Prior violation within `T_c` ticks** ⇒ *second strike* ⇒ the agent is
  marked lethal, cause `"compliance"`. No penalty, no `λ` change (it's dying).

**The `T_c` boundary is inclusive.** A violation at tick `t_last` and another
at tick `t_last + T_c` — a gap of exactly `T_c` ticks — **counts as "within
`T_c` ticks"**: `t − t_last <= T_c`. Chosen because "within `N` ticks" in
plain English includes the endpoint, and because the stricter reading matches
the model's "cannot do whatever it wants" posture. (`T_c = 4`, so a firm that
violates at tick 10 and again at any of ticks 11–14 dies; a re-violation at
tick 15 is a fresh first strike.)

#### 3b. `obligation` — relational, never lethal

- Every `supply` edge with the agent as `source` **or** `target` is marked for
  removal (accumulated into the phase's single `ReplaceGlobalList`, §4).
- Penalty `min(P_q, r^L)` (paired `AdjustStock` agent↔environment).
- **The agent survives.** No `RemoveAgent`; it simply has fewer edges and less
  capital.
- `P_q` has **no §16.1 default** (confirmed — ADR-0021); the `enforce` rule's
  params require it, and the Stage-4 wiring config supplies a value.

### 4. Ordering, simultaneity, idempotency, determinism

- The `enforce` rule iterates `view.live_agents()` — **ascending `AgentId`,
  `BTreeMap`-backed** (§19.2). No `HashMap`. `firma-plugin-constraint/src` is
  in `SIM_PATH_SRC`, so `no-hashmap-iteration` covers this loop.
- **One pass.** Per agent, all four constraints are checked; a `lethal` flag
  and a `cause` string accumulate. If `lethal`, the agent gets **exactly one**
  `RemoveAgent { reason: cause }` (cause is `"solvency"`, `"compliance"`, or
  `"solvency+compliance"` if both) plus its stock-transfer deltas, and its
  first-strike / obligation handling is **skipped** — a dead firm pays no
  fine and severs no edges beyond what `RemoveAgent` + the death edge-cleanup
  do. So a penalty is never double-applied and an agent is never
  double-removed.
- **Edges: one `ReplaceGlobalList` for the phase.** All edge removals — every
  incident edge of every dying agent (all three types, ADR-0017) plus every
  `supply` edge of every `obligation`-violating survivor — are computed into a
  single surviving-edge list and emitted as one `ReplaceGlobalList { list:
  RELATION_EDGES }`. Two agents both triggering edge removal in one phase
  therefore do **not** collide on `(Global, disc 10, "relation_edges")`. The
  delta is emitted only if the list actually changed.
- Multiple deaths → multiple `RemoveAgent` (distinct `Agent` targets, no
  collision) applied in `AgentId` order after the shared `ReplaceGlobalList`
  and all `AdjustStock` transfers.

Cite: §9.1, §16.1 (`T_c=4`, `P_c=30`, `δ_λ=0.15`; **`P_q` absent**), ADR-0017,
ADR-0021.

## Alternatives

- **Kernel-side death + edge removal.** Puts §9.1 policy and `Edge`-JSON
  parsing in the kernel (A1). Rejected.
- **A `SeverEdge { edge_id }` delta per edge.** Edges have no stable id
  (they're append-only JSON in a list); and simultaneous severances from
  different agents would need per-edge dedup. One `ReplaceGlobalList` computed
  by the one rule is simpler and collision-free.
- **`RemoveAgent` rebases conservation** (like the `remove_agent`
  intervention). Rejected — it makes a dead firm's `r^I` silently vanish from
  the accounting, defeating the §22.2 log-sufficiency VT-6 reconstruction. The
  paired stock transfers make the flow explicit.
- **`T_c` exclusive (`t − t_last < T_c`).** A defensible reading of "within";
  rejected for the endpoint-inclusive plain-English reading and the stricter
  posture. Recorded so a future revision can flip it with a one-line change
  and a superseding ADR if the manual clarifies.

## Consequences

- **Positive.** The simulation can now kill firms and sever relationships from
  its own dynamics. `ReplaceGlobalList` is a reusable `set`-a-global-list
  primitive (the shock plugin will want it for `active_shocks`).
- **Negative, accepted.** Two more `DeltaKind` variants (11 total). Each is a
  distinct structural operation with a concrete Stage-4 emitter; the
  `set`/`add` × `agent`/`global` × `scalar`/`list` grid is now nearly
  complete and should not grow much further.
- **Neutral.** No shipped numerical output changes — no config runs `enforce`
  yet; golden trace + `phase1-smoke` hash byte-identical.

## Compliance

- `firma-core::DeltaKind::{RemoveAgent, ReplaceGlobalList}`;
  `DeltaKindTag::{RemoveAgent, ReplaceGlobalList}`; `discriminant() 0..=10`.
- `firma-kernel::World::{remove_agent_and_keyed_state, replace_global_list}`;
  `run_phase` emits `AgentDied` on a `RemoveAgent` apply.
- `firma-plugin-constraint::Enforce` — phase `Enforce`; the four-semantic
  dispatch with an explicit empty `AdmissibilityGate` arm; single
  `ReplaceGlobalList` per phase; one `RemoveAgent` per dying agent with
  stock-transfer deltas first.
- `P_q` is a required parameter (no default); the Stage-4 registry config
  supplies it.
- Tests (per §Part D of the instruction): one per violation path (`solvency`
  death, `compliance` first-strike, `compliance` second-strike within `T_c`,
  `compliance` re-strike after `T_c` treated as first, `obligation` severance
  + penalty + survival, `scope` no-op), the `T_c` boundary-tick case
  (inclusive), and the simultaneous `solvency` + `compliance`-second idempotency
  case (one death, one `RemoveAgent`, no penalty).
