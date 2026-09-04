# ADR 0022 — The decide→act hand-off, and the `DeltaKind` variants Phase 2 adds

**Status:** Accepted (2026-09-03). Decisions 1–3 **partially superseded by
ADR-0024** (2026-09-04): `SetSelectedAction { action: u8 }` is generalised to
`SetAgentInt { field, value }`, and the manual `Delta` `Eq` is put on an
enforced non-`NaN` invariant. Decisions 4 and the rest of the ADR stand. The
body below is the text as accepted — **not** edited to reflect ADR-0024 (ADR
bodies are append-only; corrections are new ADRs — see `docs/adr/README.md`).
**Phase:** 2 (Model), Stage 2
**Relates to:** manual §6.4, §10.1 phases 3–6, §12.3 (decision procedure),
§17 A2, §18.2, §19.4 (reconciliation, `Delta` order), §20.5 (versioning),
§21.4, §25.7 (golden traces); the `firma-core::delta` doc "New variants are
breaking and require an ADR (§18.2, §34.0)"; ADR 0004, ADR 0016, ADR 0021

## Context

**This Stage touches `firma-core` and `firma-kernel`.** Stages 0 and 1 both
confirmed those crates were untouched; Stage 2 cannot, and does not try to
force it. The decision procedure (§12.3, Stage 3) runs in `decide` (phase 3)
and selects **one action index 0–8 per firm** holding `Strategic`. `act_market`
(phase 4) and `act_shaping` (phase 5) then execute it. The manual specifies
neither how "this firm's selected action" is represented nor how a `Rule`
writes it — and there is only one `DeltaKind` variant (`AdjustStock`), which
cannot express any of the domain-state mutations §11 requires (`c += δ_c`,
`q += q_0`, `θ_limit += δ_θ`, enqueue a lagged effect, add a relation edge).

## Decision

### 1. Where "this tick's selected action" lives

**A per-agent integer in the kernel's opaque keyed store**, written with
`DeltaKind::SetSelectedAction { action: u8 }` and read with
`View::agent_int(agent, keys::SELECTED_ACTION)` (`keys::SELECTED_ACTION =
"selected_action"`). Per-firm, per-tick.

- It is **reconciled state**, not rule-to-rule passing: `decide` (phase 3)
  emits a `Delta` that the reconciler applies, so by the time `act_market`
  (phase 4) starts its `View` already reflects it (§17 A2 — only the
  reconciler mutates; a later phase's `View` sees only what has been
  reconciled).
- **Clearing:** `decide` overwrites it every tick for every firm it manages.
  A firm with no decide-rule (ablation, or missing `Strategic`) has a stale or
  absent value; the `act_*` rules act **iff `agent_int(a, SELECTED_ACTION) ==
  Some(my_index)`**, so an absent/stale value is a safe no-op. Explicit
  per-tick clearing is a Stage-3 `decision.satisficing` concern and is not
  built now.
- **Not on `FirmState` / `FirmAuxState`.** `FirmState` is the `d ≤ 4`
  constraint-carrying vector (§9.3) — `selected_action` is not a `g_j` input.
  `FirmAuxState` per its own doc carries "the scalar auxiliary fields the
  constraint and goal layers need" — `selected_action` is an *attention*-layer
  field, read only by the `act_*` rules, so putting it there would widen a
  constraint/goal type with an unrelated concern and force an edit at every
  `FirmAuxState` construction site for a field nothing in those layers reads.
  The opaque kernel store is the same mechanism already used for `capability`
  (`agent_real "capability"`) and `theta_limit` (`global_real`): the kernel
  holds the value, the domain meaning lives in `firma-domain::keys` and the
  plugins.
- **Eventual home:** §8.4's `Attention` component ("Current focus, search
  state"). The Stage-2 interim is the keyed store; it migrates to a typed
  `Attention` component when `decision.satisficing` and the full component bag
  land (Stage 3). The wire representation (a per-agent int keyed
  `"selected_action"`) is expected to survive that migration.

### 2. How a rule writes domain state — new `DeltaKind` variants

`firma-core::DeltaKind` gains eight variants. The kernel stores each as an
**opaque keyed value** — it never interprets the string keys ("capability",
"theta_limit", …), exactly as it never interprets a `ResourceKind` string
(§7.2). The *meaning* lives in the plugins and in `firma-domain::keys`. This
keeps §17 A1 intact in substance: no firm/constraint/action/theory logic
enters the kernel, only a generic typed+opaque store.

| # | disc | variant | target | semantics |
|---|---|---|---|---|
| 0 | 0 | `AdjustStock { resource, amount: i64 }` | Agent / Env | **add** (Phase 1, unchanged) |
| 1 | 1 | `SetSelectedAction { action: u8 }` | Agent | **set** (last write; one decide-rule per firm/tick) |
| 2 | 2 | `AdjustAgentReal { field: String, delta: f64 }` | Agent | **add** — e.g. `capability` |
| 3 | 3 | `AdjustAgentInt { field: String, delta: i64 }` | Agent | **add** — e.g. `obligation` |
| 4 | 4 | `AdjustGlobalReal { field: String, delta: f64 }` | Global | **add** — additive combination (ADR 0016), e.g. `theta_limit` |
| 5 | 5 | `AdjustGlobalInt { field: String, delta: i64 }` | Global | **add** — e.g. `theta_q` |
| 6 | 6 | `PushAgentRecord { list: String, record_json: String }` | Agent | **append** — e.g. `lagged_effects` (Λ) |
| 7 | 7 | `PushGlobalRecord { list: String, record_json: String }` | Global | **append** — e.g. `relation_edges` |
| 8 | 8 | `ReplaceAgentList { list: String, records_json: Vec<String> }` | Agent | **set list** — `resolve_lagged` drains Λ with this |

Records are **pre-serialised canonical JSON strings**, not `serde_json::Value`,
so `Delta` keeps deriving `PartialEq, Eq` (a `Value` contains `f64` and is not
`Eq`) and `firma-core` needs no `serde_json` in `Delta`. The plugins own the
schema (`firma_domain::Effect`, `firma_domain::Edge`).

### 3. `discriminant()`, the `slot`, and the total order

- `discriminant()` returns `0..=8` (table above), unique per variant.
- **New:** `DeltaKind::slot(&self) -> &str` — the resource / field / list name
  (`"action"` for `SetSelectedAction`). Two `AdjustStock` deltas from the same
  rule to the same agent for *different resources* (`acquire_input` adjusts
  both `r^L` and `r^I`) share `(target, discriminant)` but differ in `slot` —
  Phase 1's uniqueness check keyed only on `(target, discriminant)` and would
  have rejected this. The check is widened to `(target, discriminant, slot)`.
- **`allows_repeat()`** — `true` for `PushAgentRecord` / `PushGlobalRecord`
  (a rule legitimately enqueues two effects in one phase); those are exempt
  from the uniqueness check. Their relative order is preserved by the *stable*
  `proposed.sort()` (Rust `slice::sort` is stable), so append order =
  deterministic emission order.
- **`Delta`'s `Ord`** gains `slot` as the final tie-breaker:
  `(target_id, conflict_class, origin_string, discriminant, slot)`. Still a
  total order (lexicographic over totally-ordered fields). The Phase-1 order
  is unchanged for any pair of `AdjustStock` deltas that already differed on
  an earlier key; the only pairs affected are ones Phase 1 could not produce
  (same target, class, origin, discriminant — impossible under the old
  uniqueness rule).
- Phase 1 shipped no dedicated `Delta`-ordering unit test — the order is
  exercised indirectly by the DT-1…DT-6 suite, which passes **unmodified**
  after this change (verified). This ADR adds the dedicated test
  (`firma-core::delta::tests`): discriminants are exactly `0..=8` and unique;
  `allows_repeat()` is true only for the two append kinds;
  `AdjustStock{resource:"capital"}` and `AdjustStock{resource:"input"}` to the
  same agent from the same rule sort deterministically by `slot`
  (`"capital" < "input"`); an `AdjustStock` and a `SetSelectedAction` to the
  same agent sort by discriminant `0 < 1`; a pair that already differs on
  `origin` sorts by `origin` exactly as in Phase 1 (the `slot` key is never
  consulted); and the relation is a strict weak ordering (reflexive,
  antisymmetric, transitive) over a mixed set.

### 4. §20.5 / MAJOR-bump check

**Does adding these variants change the output of any existing config?**
**No.** No Phase 1, Stage 0, or Stage 1 test config or the `phase1-smoke`
config emits any new variant. The Phase 1 golden trace
(`GOLDEN_EVENT_LOG_SHA256`) is a run of `testkit.*` plugins, which are
untouched. Verified: golden trace green, `phase1-smoke` `event_log_sha256`
byte-identical.

**Does the *crate version* need a bump anyway?** §20.5's table is written for
*plugin* version vs. *config* output ("The engine MUST refuse a config whose
plugin MAJOR mismatches") — its MAJOR row is "anything altering numerical
output for an existing config", which this is not, so §20.5's
"MAJOR bump MUST come with a golden-trace update" (§25.7) does **not** trigger.
By pure SemVer, adding variants to a public non-`#[non_exhaustive]` enum is a
breaking API change; pre-1.0 that is a **MINOR** bump (`0.1 → 0.2`).

**Decision: do not bump the workspace version for Stage 2.** Rationale:
1. The project is pre-1.0, unreleased, single-repo — SemVer's break protection
   is for downstream consumers, of which `firma-core` has none.
2. The workspace `version` feeds `engine_version` in every `RunIdentity`
   (§22.3), so bumping it changes every `run_id` — including `phase1-smoke`'s
   (`GOLDEN_RUN_ID`) — *purely from a version string*, with a byte-identical
   event log. That is a golden-trace failure with no trajectory behind it,
   which is exactly the noise §25.7 is designed to avoid.
3. Phase 2's accumulated breaking `firma-core` changes get **one** MAJOR bump
   at the first real release (1.0), with a single golden-trace regeneration.
   `PROGRESS.md` tracks the accumulation.

`DeltaKindTag` (used by `Rule::writes()`) is `#[non_exhaustive]`-in-spirit —
adding tags is additive for rules that don't emit the new kinds. The new
**action plugin crates** are `1.0.0` (new). `firma-plugin-testkit` gains a
test-only helper (Part F) — additive, no version change needed for a
test-scoped addition.

## Rejected alternatives

- **A dedicated `Attention` component now.** §8.4's real home, but building the
  component bag is Stage 3 work bundled with `decision.satisficing`. The
  keyed-store interim costs one `keys` constant and one migration note.

- **`selected_action: Option<u8>` on `FirmAuxState`.** Rejected per Decision 1:
  `FirmAuxState` is a constraint/goal-layer type, `selected_action` is an
  attention-layer field read only by `act_market` / `act_shaping`. Adding it
  there forces a struct-literal edit at every construction site (five, across
  `firma-viability` and the test suites) for a field none of those layers
  reads, and blurs the §9.3 boundary the two-type split exists to keep sharp.

- **Rules pass the selected action to each other directly** (a shared
  scratch map threaded through the phase loop). Violates §17 A2 — a phase-4
  rule would read something a phase-3 rule wrote *without reconciliation*.
  The `View` must only show reconciled state.

- **One generic `DeltaKind::MergeDomainJson { patch }` (JSON merge).** Merge is
  *set* semantics; `θ_limit += δ_θ` from two firms' matured lobbies must **add**
  (ADR 0016). A merge would clobber. The typed `Adjust*Real/Int` variants get
  additive combination for free via the existing `Independent` reconciler path
  (§19.4 step 5), which is the whole point of ADR 0016.

- **`serde_json::Value` payloads in `DeltaKind`.** `Value` is not `Eq` (it
  holds `f64`), which would strip `Delta`'s derived `Eq` and ripple into
  `Event`'s and the DT test comparisons. Pre-serialised canonical JSON strings
  avoid it entirely and are deterministic.

- **Bump the workspace version to 0.2.0.** Rejected per Decision 4 point 2 —
  breaks `phase1-smoke`'s `run_id` for a non-reason.

## Consequences

**Positive.**
- The hand-off is reconciled state, so `decision.satisficing` (Stage 3) writes
  it the same way any rule writes anything, and the `act_*` rules read it
  through the one channel a rule has.
- The kernel's domain-state store is a generic keyed map — no `θ`/`c`/`q`
  meaning in the kernel, §17 A1 intact.
- Additive θ combination (ADR 0016) is free: `AdjustGlobalReal` rides the
  `Independent` path.

**Negative, accepted.**
- `firma-core` and `firma-kernel` are no longer "untouched since Phase 1".
  Their **dependency sets are still `{}` / `{firma-core, firma-rng}`** (the new
  variants carry only `String`/`i64`/`f64`/`u8`/`Vec<String>`), which the
  verification trace confirms. But the API changed. Stated plainly.
- Eight new `DeltaKind` variants is a wide surface. Each is one mutation shape
  (set / add-real / add-int / append / set-list), reused across all domain
  fields — not one variant per field — so it does not grow further as Stage 3+
  add fields.
- String-keyed domain state trades compile-time field checking for runtime
  key agreement. Mitigated: `firma-domain::keys` holds the canonical `&str`
  constants; every producer and consumer imports them.

**Neutral.**
- **No shipped numerical output changed.** Golden trace + `phase1-smoke` hash
  byte-identical; workspace version unchanged (Decision 4).

## Compliance

- `firma-core::DeltaKind` — eight variants, `discriminant()` `0..=8`,
  `slot()`, `allows_repeat()`, `Delta::Ord` gains `slot`.
- `firma-kernel::World` — six opaque stores (`agent_reals`, `agent_ints`,
  `global_reals`, `global_ints`, `agent_lists`, `global_lists`), all
  `#[serde(default)]`, in the snapshot; `WorldView` exposes six read methods;
  the reconciler `apply_delta` gains eight arms; the uniqueness check widens to
  `(target, disc, slot)` and skips `allows_repeat()` kinds.
- `firma-core::View` — six new methods (`agent_real`, `agent_int`,
  `global_real`, `global_int`, `agent_records`, `global_records`), with
  default impls (`None` / `&[]`) so a test `View` need only override what it
  uses.
- `firma-domain::keys` (canonical `&str` field/list constants, incl.
  `SELECTED_ACTION`); no new field on `FirmState` / `FirmAuxState`.
- Tests: Phase-1 `Delta`-order tests pass unmodified; new order/uniqueness
  tests (Part A.3); reconciler round-trip for each new kind; snapshot
  round-trip.
- **Version:** workspace stays `0.1.0`; `PROGRESS.md` records the accumulating
  pre-1.0 breaking `firma-core` changes for a single MAJOR bump at 1.0.

## Note

The wide `DeltaKind` is the price of "only the reconciler mutates" (§17 A2)
meeting "the model has more state than one integer per agent" (§8). The
alternative — a side channel for domain-state mutation — would be a second,
unaudited path to state, which is exactly what A2 exists to prevent. Eight
generic mutation shapes, opaque to the kernel, is the smallest thing that
keeps A2 whole while letting §11 execute.
