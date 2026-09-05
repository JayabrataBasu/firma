# ADR 0054 — Arm A direct manipulation: `Intervention::SetAgentReal` (generic, kernel-pure) plus four `decision.satisficing` pin-read keys

**Status:** Accepted (owner review complete; two scoping additions —
partial-pin behaviour and the aspiration-drift note, both below — made
prior to acceptance, not as a later correction). Per the project's ADR
immutability house rule, this document's body is now append-only — any
further correction is a new, superseding ADR, never an in-place edit
here, the `Status` line itself excepted. Implementation authorized by a
follow-up instruction and completed on this branch (see Compliance for
what was and was not implemented).
**Phase:** design, ahead of Phase 3 build work (Arm A, ADR-0051's narrowed
E1 design; found blocking Arm A's job-config generation in the prior
investigation round, `PROGRESS.md` Part C / this branch's history)
**Supersedes:** Nothing.
**Relates to:** manual §6.5 (intervention algebra, `X^{do(a)}_T = a(X_T)`),
§6.6 (interventional construct definition), §7.2 primitive 9
(`Intervention`), §10.1 (the 9-phase schedule), §14.6 (the fork/no-shock-
branch definition), §18.2 (`firma-kernel`'s module contract, including
`fn fork(&self, snap: &Snapshot, iv: &Intervention) -> World`), §20.2
("resist growth" — cited via ADR-0034's `rng_stream()` justification),
§21.2 (RNG key formula, CRN property 3), §21.3 (stream separation),
§25.2 DT-4/DT-5, §30.2/§30.4 (Arm A's stated design), §30.7 (H1a/H1b's
analysis formula), ADR-0021 (the `Constraint` interface — "category
exists, formula doesn't, design it" precedent this ADR follows), ADR-0022/
ADR-0024 (opaque keyed-field precedent: `SetAgentInt`/`AdjustAgentReal`),
ADR-0027 (`decision.random` — same "manual names it, doesn't specify it"
situation, same design-before-code posture), ADR-0040 (the `select(h,
shortfalls, ...)` seam this ADR does not touch), ADR-0053 (the CRN/fork
property this design must preserve, Part 1/1b)

## Context

The narrowed E1 design's Arm A (ADR-0051: 125 cells, `h`(5) × `ς`(5) ×
`β`(5)) requires, per §30.4, "Margin `h` and Shortfall `ς` set directly by
intervention at each measurement tick, independently, across the full
grid including empirically rare quadrants." A prior investigation round
found `firma_lab.runner.build_job_config` cannot expand any Arm-A cell
into a real job, because `firma_core::Intervention` has no variant that
can force `h`/`ς` — only `SetStock` (a raw resource stock), `AddRule`/
`RemoveRule`/`FreezeRule`, and `RemoveAgent` exist.

**This ADR designs the missing mechanism. It authorizes no
implementation** — a follow-up instruction, after owner review of this
document, does that.

## What was read directly before designing (per this instruction's own requirement)

- §6.5's operator table in full (reproduced below) and §6.6.
- §30.2 in full ("the required manipulation... is unavailable in field
  data... the model is the only instrument in which the orthogonal design
  is realisable") and §30.4 in full (Arm A/B/C's design text and the
  factor table).
- §14.6, and a full-manual grep for "intervention," "fork," "do(," and
  "counterfactual" (41 hits — every one read; the load-bearing ones are
  cited by section below, not just this Context).
- `crates/firma-core/src/intervention.rs` **read in full, not assumed
  empty**: the `Intervention` enum (`Null`, `SetStock`, `AddRule`,
  `RemoveRule`, `FreezeRule`, `RemoveAgent`) is exactly what the prior
  round reported — no stub, partial type, or comment for a margin/
  shortfall-forcing variant exists anywhere in that file.
- `firma-domain::margin::standard_margin` (how `h` is actually computed:
  a function of `FirmState` `(r^L, r^I, c, q)`, `FirmAuxState` (`λ`, `u`,
  aspirations), `ConstraintParams` `θ`, and `ScaleFactors`) and
  `Satisficing::shortfalls` (`firma-plugin-decision/src/lib.rs`: `ς_j =
  A_j − v_j`, `v_1` = realized capital growth, `v_2` = capability, `v_3` =
  `−obligation`, `A_j` from the four `ASPIRATION_*` keyed fields).
- `crates/firma-kernel/src/lib.rs`'s actual `fork`/`apply_intervention`
  implementation, `crates/firma-kernel/src/world.rs`'s `World` mutation
  methods, and `crates/firma-cli/src/orchestrator.rs`'s
  `apply_interventions_at` (the per-tick intervention dispatcher) — **all
  three already exist, are tested (DT-4/DT-5), and are load-bearing to
  this design**, detailed in Decision below.

**A significant finding, changing the shape of this design task from what
the prior round's framing implied:** this is **not** "design fork/CRN/
intervention architecture from scratch." That architecture already
exists, is specified in the manual (§18.2's `fn fork(&self, snap:
&Snapshot, iv: &Intervention) -> World`), is implemented
(`firma-kernel::Kernel::fork`/`apply_intervention`), and is tested (DT-4
"fork-with-null-intervention equals continuation," passing). **The actual
gap is narrow: one new `Intervention` variant**, slotting into an
existing, working dispatch mechanism — not a new subsystem.

## Decision

### Q1 — What does "set `h` and `ς` directly" mean at the state level?

**Resolved: override the *derived* values themselves (reading (a) in this
instruction's own framing), not the underlying state `h`/`ς` are computed
from. Reasoning, cited, not asserted:**

1. **§30.2's own rationale for Arm A** — *"The required manipulation —
   independently varying shortfall and viability proximity — is
   unavailable in field data, where they covary... The model is the only
   instrument in which the orthogonal design is realisable"* — is a
   rationale for **true independence**, not merely convenient
   independence. Reverse-solving for underlying state `(r^L, r^I, c, q)`
   that *produces* a target `h` under `standard_margin` risks
   reintroducing exactly the covariance the design exists to eliminate:
   changing `r^L` to hit a target `h` can also move `v_1` (realized
   capital growth), which moves `ς_1` — a hidden coupling between the two
   "independently" swept factors that a reverse-solve cannot generally
   avoid (the margin formula and the shortfall formula do not share a
   clean inverse).
2. **VT-8 criterion (iii)** (§25.4) already requires "no path in the
   decision procedure computing one from the other," verified today by an
   analytic trace over `select()`'s ~15-line body. A direct override of
   the two values *fed into* `select()` keeps this seam exactly as clean
   as VT-8 already found it — the override happens **before** `select()`
   is ever called, so `select()` itself needs no change (Q5) and the
   independence VT-8 verifies is preserved by construction, not merely by
   argument.
3. **§30.4's "including empirically rare quadrants"** is trivially
   satisfied by direct override (any `(h, ς)` combination is directly
   constructible — no search over state space required) and is **not**
   trivially satisfiable by reverse-solving (a target combination like
   "very healthy margin, very high shortfall" may correspond to no
   reachable real state at all, or a state so extreme it breaks other
   invariants).

**Alternative rejected, named explicitly:** reverse-solving for
underlying state. Rejected for the coupling risk in point 1, the
rare-quadrant reachability problem in point 3, and because it would make
"the next tick's dynamics" (this question's own second half) dependent on
which state variables were chosen to absorb the reverse-solve — a
strictly worse-specified, more fragile design than direct override for no
compensating benefit.

**Consequence for "the next tick's dynamics," stated plainly, not
glossed over:** under direct override, the *real* underlying state
(`r^L`, `r^I`, `c`, `q`, aspirations) is untouched and keeps evolving
under ordinary rule dynamics — production, market actions, `constraint.
enforce`'s real violation detection all continue to read and act on the
**real** state, never the pinned value. The firm's *decision* is driven
by the pinned `h`/`ς` (testing H1a/H1b's actual mechanism); its *survival*
is governed by the real state (a firm cannot be pinned out of an actual
compliance violation or insolvency). Real state and pinned `h`/`ς` can
therefore diverge arbitrarily over a run — **this is not a defect, it is
VT-8's independence property holding at runtime, not just at
construction.**

### Q2 — Phase placement

**Resolved: no new phase.** `crates/firma-cli/src/orchestrator.rs`'s
`apply_interventions_at` is already called **once per tick**, inside
`execute_run`'s `for tick in 0..cfg.world.ticks` loop, **before** that
tick's phase schedule runs, filtering `cfg.interventions` for entries
whose `at == tick` and dispatching non-rule-set variants straight to
`kernel.apply_intervention(world, other)`. A new `Intervention` variant
needs **zero** changes to this dispatcher's control flow — it is added as
one more match arm inside `Kernel::apply_intervention`
(`crates/firma-kernel/src/lib.rs`), reached through the exact same
per-tick loop every existing variant already goes through.

**"At each measurement tick" (§30.4) is satisfied by *read-side*
persistence, not *write-side* repetition — a design point worth stating
explicitly since it determines how many `TimedIntervention` entries one
Arm-A job's config needs.** The new variant writes into a keyed per-agent
store slot (Q4/Q5 below) that nothing else in the model touches; once
written, `decision.satisficing`'s Step 1 reads it **every tick** for the
rest of that run, the same way every other config-seeded value (e.g.
`θ_limit`) is set once and read every tick without being "reapplied."
**One `TimedIntervention` entry per pinned value, at `at: 0`, is
therefore sufficient for one Arm-A cell's dedicated run — not one entry
per tick.** (The alternative — literally repeating the intervention at
every tick — was considered and rejected: it would work, since
`apply_interventions_at` supports arbitrarily many scheduled entries, but
it is needless config bloat for a value nothing else in the model ever
overwrites, and it obscures the actual mechanism, which is persistence,
not repetition.)

**This also resolves, implicitly, how Arm A's 125 cells map to jobs**:
one dedicated `firma run` per cell (matching how `firma_lab.runner`
already treats every Arm B/C job — an independent subprocess invocation,
not a single run sweeping through many cells), with the pin set once at
`at: 0` and held for that job's entire horizon. Arm A's own DVs
(`repertoire_entropy`, `search_width`) need a **trailing window of
realized actions** to compute — a single decision (what VT-8's own
harness tests directly, per its own doc comment: *"the VT-8 harness calls
it directly with grid-constructed `(h, ς)` pairs that never touch firm
state"*) is not sufficient for Arm A's actual experimental measurement,
which is why Arm A needs a **full multi-tick run per cell**, not a
VT-8-style direct `select()` call.

### Q3 — How is the fork/CRN property preserved?

**Resolved: trivially, by construction, because the mechanism consumes no
randomness.** Checked directly, not assumed: every existing
`Intervention` match arm in `Kernel::apply_intervention`
(`crates/firma-kernel/src/lib.rs:619-657`) is a **pure, deterministic
`World` mutation** — `world.set_agent_stock(...)`, `world.remove_agent(...)`,
`world.frozen_rules.insert(...)` — none of them call into `firma-rng` at
all. A new `SetAgentReal`-shaped variant (Q4) follows the identical
pattern: one call to an existing, already-implemented, RNG-free `World`
method (`world.set_agent_real`, `crates/firma-kernel/src/world.rs:167`).
Applying it therefore perturbs **no** `(plugin, phase, tick, agent,
purpose)` key anywhere — the RNG key formula (§21.2) never sees the
intervention at all — so two cells sharing the same `StreamSeeds`
(ADR-0053's adopted scheme) and differing only in which `SetAgentReal`
intervention(s) their configs declare produce **bit-identical draws**
wherever their trajectories haven't yet diverged causally, exactly DT-4's
own property ("fork-with-null-intervention equals continuation")
generalized to a non-null intervention that itself draws nothing.

### Q4 — New `DeltaKind`, or reuse?

**Resolved: neither a new `DeltaKind` nor a new `World` method — reuse
both existing generic machinery, add one `Intervention` enum variant.**

Two things already exist and are directly reusable:

1. **`World::set_agent_real(&mut self, agent: AgentId, field: impl
   Into<String>, value: f64)`** (`crates/firma-kernel/src/world.rs:167`)
   — a fully generic, already-implemented, opaque-keyed-field setter for
   per-agent `f64` scalars. It is the **same underlying store**
   `DeltaKind::AdjustAgentReal`'s reconciler-side application already
   writes into via a different (delta-summed) path — this is one store,
   two writers (ordinary rule deltas via the reconciler; interventions
   directly, exactly as `SetStock`/`set_agent_stock` already establishes
   for stocks).
2. **The `SetAgentInt`/`AdjustAgentInt`/`AdjustAgentReal` `DeltaKind`
   precedent** (ADR-0022/ADR-0024): "opaque field key, kernel routes by
   the carried `field` like every other keyed variant and never names the
   key" — the exact pattern to extend, not reinvent.

Proposed new variant, in `crates/firma-core/src/intervention.rs`'s
`Intervention` enum, parallel to the existing `SetStock`:

```rust
/// `set_param(path, value)` restricted to a named per-agent real scalar
/// (manual §6.5's `set_param` operator) — force an agent's keyed real
/// field to an absolute value. Domain-agnostic: the kernel does not know
/// what `field` means (§17 A1); `firma-domain`/plugin crates define which
/// keys are meaningful to read (see ADR-00NN's `PINNED_MARGIN`/
/// `PINNED_SHORTFALL_*`).
SetAgentReal {
    /// Target agent.
    agent: AgentId,
    /// Opaque field key.
    field: String,
    /// New absolute value.
    value: f64,
},
```

`Kernel::apply_intervention`'s new match arm: `Intervention::SetAgentReal
{ agent, field, value } => { world.set_agent_real(firma_core::AgentId(agent.0), field, *value); Ok(()) }`
— no `rebase_conservation()` call (unlike `SetStock`): `f64` "reals" are
not part of the `i64` conservation ledger (§21.4) at all, confirmed by
reading `World::set_agent_real`'s own implementation, which touches no
conservation-tracking field.

**Why this satisfies §20.2's "resist growth" discipline (cited via
ADR-0034's precedent):** the *kernel-level* surface added is one enum
variant reusing an *already-implemented* method — smaller than `SetStock`
itself required (which needed `rebase_conservation()` bookkeeping this
variant does not). No new `DeltaKind`, no new `World` method, no new
phase, no new trait. The only genuinely new surface is domain-level (Q5):
two new `firma-domain::keys` constants and a small read added to
`Satisficing::apply()`'s Step 1 — exactly where the manual's own "category
exists, mechanism doesn't, design it" precedent (ADR-0021, ADR-0027) says
new domain-specific behavior belongs, never in the kernel.

### Q5 — Interaction with `decision.satisficing`'s existing reads; `select()` unchanged

**Resolved: `select()` requires zero changes. `Satisficing::apply()`
gains a small, localized read at Step 1, nowhere else.**

Two new `firma-domain::keys` constants, following the *exact* existing
naming pattern of `ASPIRATION_CAPITAL_GROWTH`/`ASPIRATION_CAPABILITY`/
`ASPIRATION_OBLIGATION_CLEARANCE` (three per-goal keys, not one
aggregate) for the shortfall side, plus one for margin:

```rust
pub const PINNED_MARGIN: &str = "pinned_margin";
pub const PINNED_SHORTFALL_CAPITAL_GROWTH: &str = "pinned_shortfall_capital_growth";
pub const PINNED_SHORTFALL_CAPABILITY: &str = "pinned_shortfall_capability";
pub const PINNED_SHORTFALL_OBLIGATION_CLEARANCE: &str = "pinned_shortfall_obligation_clearance";
```

`Satisficing::apply()`'s Step 1 (currently `let h_t = dc.margin_at(&dc.state,
&aux, &p.scales); let sc = Self::shortfalls(view, agent, &dc.state);`)
becomes: read each pinned key via `view.agent_real(agent, keys::PINNED_*)`;
if present, use it in place of the computed value for that one component
(`h_t`, or `sc[j]` individually); if absent (the default — every existing
config, `Option::None`, exactly ADR-0047's "opt-in, behaviour-preserving"
precedent), compute exactly as today. **`select(h_t, sc, p.beta, p.h_crit,
p.w_max, ...)` is called with whichever values resulted — `select()`
itself never learns whether a component was pinned or computed.** This is
the seam ADR-0040 built specifically so that this kind of substitution
needs no change to it: `h` and `shortfalls` enter as independent typed
parameters, and nothing between "how they were produced" and "what
`select()` does with them" is coupled.

**Scope boundary, stated explicitly (a real design decision, not an
oversight):** the pinned keys are read **only** by `Satisficing::apply()`'s
Step 1. `constraint.enforce`, `constraint.action_window`, and every other
rule continue to read/compute margin and shortfall-adjacent quantities
from **real** state, unaffected by a pin. A pinned firm cannot be pinned
out of a real compliance violation or insolvency — only its **own
decision procedure's inputs** are overridden, matching Q1's "the firm's
decision is driven by the pin; its survival is governed by reality"
resolution.

**Aspiration drift — stated explicitly, not left to be re-derived
later.** `decision.aspiration_update` (§12.1's adaptive update,
`A_{j,t+1} = A_j + α(v_{j,t} − A_j)`) continues updating the agent's
**real** aspiration values from the agent's **real** realised `v_j` every
tick, completely independent of any active pin — it reads/writes only the
`ASPIRATION_*` keys and `v_j`'s own inputs, never the `PINNED_*` keys, and
this ADR does not change it. Consequence, made concrete rather than left
implicit: the firm's *real*, unpinned `ς_j` (`A_j − v_j`, what `Self::
shortfalls` would compute if no pin were active) keeps evolving normally
in the background for the entire run, and **silently, continuously
diverges** from the pinned `ς_j` value actually driving the firm's
decisions — a pinned firm's decision procedure never sees this real,
drifting value at all once its pin is active. This is not a defect; it is
Q1's "real state and pinned values can diverge arbitrarily" made concrete
for this specific, easy-to-be-surprised-by case: a future reader
debugging an Arm-A result who inspects the event log and finds the firm's
*real* aspiration/shortfall trajectory looking nothing like what drove its
selected actions should find this paragraph, not have to re-derive it
from first principles.

**Partial-pin behaviour — decided explicitly (Part A of the instruction
that reviewed this ADR before acceptance).** A config or intervention
setting only `PINNED_MARGIN` without any `PINNED_SHORTFALL_*` key (or
vice versa, or setting one or two of the three shortfall keys but not all
three) is **not** a legitimate, meaningful configuration — it is treated
as almost-certainly a config error and rejected loudly. Checked directly
against Arm A's own design before deciding this: every one of the 125
Arm-A cells (`h`(5) × `ς`(5) × `β`(5)) specifies **both** an `h` level and
a `ς` level for every cell — Arm A itself never has a legitimate use for
pinning only one. No other part of this project's design was found to
need a partial pin either. **Contract: all four `PINNED_*` keys present
together, or none — never some.** `Satisficing::apply()` enforces this at
runtime (`Self::read_pins`, checked every tick, every agent) with a loud
panic naming exactly which keys are missing when a partial state is
detected, rather than silently computing a mix of pinned and real values.
**Stated honestly, not glossed over: this is runtime rejection, not true
config-load-time rejection** (unlike, e.g., ADR-0025's `b_λ`/`b_κ`
required-field precedent, enforced by `serde` at parse time) — the pinned
values arrive via a scheduled `Intervention`, not via static
`SatisficingParams`, so whether a given agent's pins are complete is not
knowable until the tick the intervention(s) actually apply. A loud panic
at that point is this codebase's available equivalent, given `Rule::
apply`'s `Vec<Delta>` return carries no `Result` to reject through.

### Q6 — Full grid coverage, including "empirically rare quadrants"

Satisfied without restriction, per Q1: `SetAgentReal` sets an absolute
`f64` value with no domain-level bound checking at the kernel level (the
kernel is domain-agnostic — it does not know "h" has a meaningful range).
Whether an experimenter-chosen `(h, ς)` combination is nonsensical (e.g.
`h` far outside `[-something, 1]`) is not the kernel's concern to police;
`firma_lab.spec.build_narrowed_e1_spec`'s own factor levels (§30.4's
table: `h ∈ {0.02, 0.05, 0.10, 0.20, 0.40}`, `ς ∈ {0.0, 0.25, 0.50, 0.75,
1.0}`, both already within `standard_margin`'s/shortfall's normal ranges)
are the only gate, and every one of the 25 `(h, ς)` combinations —
including the "empirically rare" ones like high-`h`/high-`ς` — is
directly constructible, since the two pins are independent per-key
writes with no cross-validation between them at all.

### Q7 — Scope boundary: Arm-A-specific, or general?

**The kernel-level primitive (`Intervention::SetAgentReal`) is general,
not Arm-A-specific**, and belongs exactly where the existing
`Intervention` enum already lives (`firma-core`) — it is domain-agnostic
in the identical sense `SetStock` already is (an opaque field key, no
"h"/"margin" vocabulary anywhere in `firma-core`). Any future work needing
to force an arbitrary per-agent real scalar (not only `h`/`ς`) can reuse
it without a further kernel change.

**What *is* Arm-A/H1a/H1b-specific** is entirely domain-level: the two
new `keys::PINNED_*` constants (in `firma-domain::keys`) and the small
read added to `Satisficing::apply()`. This split — generic kernel
primitive, domain-specific consumer — is the same architecture ADR-0022/
ADR-0024/ADR-0026 already established for every other opaque keyed field
in this codebase, applied here rather than invented here.

**No new crate is needed.** The kernel-level piece is a one-variant
addition to `firma-core::intervention`, already the home of every other
`Intervention` variant; the domain-level piece is two constants in
`firma-domain::keys` (already the home of every other keyed field) and a
localized change inside `firma-plugin-decision` (already the home of
`Satisficing::apply()`). Nothing here crosses a crate boundary that
doesn't already exist for this exact kind of change.

## A genuinely open question, flagged for the owner, not resolved unilaterally

**§30.4's "Shortfall `ς` (normalised)" is one swept scalar factor (5
levels); `select()`'s actual signature takes three independent shortfalls,
`shortfalls: [f64; 3]`.** How the one swept value maps onto the three
`ς_j` is not specified by the manual anywhere this ADR could find — grepped
§30.3/§30.4/§30.7/§14 for a mapping rule; none exists. §12.1, the manual's
own **formal** definition of shortfall, is unambiguous on one point: `ς`
is defined **only** per-goal (`ς_j = A_j − v_j`, a 3-row table, `j ∈
{1,2,3}`) — no aggregate, unsubscripted "ς" is defined anywhere as its own
derived quantity. §2.4's hypothesis text and §30.4's factor-table row both
use the bare symbol "`ς`" without a subscript, which is therefore
**informal/shorthand relative to §12.1's own formal definition**, not
evidence of a third, aggregate reading — this ADR does not treat the bare
symbol as implying an aggregation rule the manual never states.

**Two live options, compared explicitly, not just named:**

- **Option A — uniform.** Set all three `PINNED_SHORTFALL_*` keys to the
  same swept value (`ς_1 = ς_2 = ς_3`). Models "a firm deficient across
  every goal dimension simultaneously."
- **Option B — targeted.** Set only `PINNED_SHORTFALL_CAPITAL_GROWTH`
  (`ς_1`) to the swept value; hold `ς_2`/`ς_3` at an explicit, controlled
  **fixed baseline of `0.0`** (not "whatever the real state's aspiration-
  shortfall computation would give" — see the correction below).

**What each actually tests, checked against `Focus::attend`'s and
`satisfices()`'s real logic, not asserted:** `Focus::attend` is sensitive
to `ς` **only** through `argmax_j ς_j` (which goal becomes `GOAL(j)`, and
whether any exceeds `0`); once resolved to `GOAL(j)`, `satisfices()`'s
`Goal(j)` branch reads **only** `sc[j-1]` — the *other* two components are
never read again for that decision. Under **both** options, `ς_1` is the
value driving `argmax_j ς_j`'s outcome: under Option A because all three
are equal (any tie resolves to the lowest `j`, i.e. `j=1`, by the existing
tie-break rule); under Option B because `ς_1` is the swept value and
`ς_2`/`ς_3` are fixed at `0` (so `ς_1 ≥ 0` always wins outright, and
`ς_1 < 0` still ties at `0` with the other two and resolves to `j=1` by
the same tie-break). **For `decision.satisficing`'s actual mechanism, the
two options are functionally equivalent** — every quantity `select()`
and `satisfices()` compute from `ς` under Option A is identical to what
they compute under Option B, for every swept value in §30.4's `{0.0,
0.25, 0.50, 0.75, 1.0}` range, because `sc[1]`/`sc[2]` (`ς_2`/`ς_3`) are
never read once `j` resolves to `1` under either construction. They do
**not** test meaningfully different claims about H1a/H1b for this
mechanism specifically — one does not subsume the other in some richer
sense; they coincide.

**What VT-8's own construction is actually built around — read directly,
not restated from memory** (`tests/tests/validation.rs::
vt8_orthogonal_manipulation_of_h_and_shortfall`, lines ~936–968): the grid
is built with `select(h, [s1, 0.0, 0.0], beta, h_crit, w_max, 0,
admissible, satisfices)` — **`shortfalls[0]` (`ς_1`, capital growth)
alone is swept; `shortfalls[1]`/`shortfalls[2]` (`ς_2`, `ς_3`) are fixed
at exactly `0.0`**, with the test's own comment stating the reasoning:
*"ς_1 is positive and non-positive. ς_2 = ς_3 = 0 so max_j ς_j = max(ς_1,
0)."* This is **Option B**, exactly, already validated, already the
project's own existing precedent for constructing an `h`-vs-`ς` grid
against this exact `select()` seam — not a hypothetical alternative.

**Correction to this instruction's own framing of Option B, stated
plainly so the comparison is accurate**: the instruction describes Option
B as leaving `ς_2`/`ς_3` "at whatever their organically-arising values
would be for that cell's state" — VT-8's actual, already-validated
construction does **not** do this; it fixes them at an explicit,
controlled `0.0`, the same "no shortfall" resting value `Satisficing::
shortfalls`'s own code already uses for an absent aspiration (`view.
agent_real(...).unwrap_or`-style default → `0.0` when an aspiration key is
unset). An *organic*/uncontrolled reading of Option B would reintroduce
exactly the "shortfall covaries with something uncontrolled" problem Q1's
three points argue against for the reverse-solve alternative; a
*fixed-baseline* reading (VT-8's actual one) does not — the two are not
the same proposal, and only the fixed-baseline version is recommended
below.

**Recommendation, reasoning shown, not asserted:** adopt **Option B,
fixed-baseline, matching VT-8's construction exactly** — sweep only
`PINNED_SHORTFALL_CAPITAL_GROWTH`, fix the other two `PINNED_SHORTFALL_*`
keys at `0.0`. Not because Option A is wrong (the two are shown above to
be functionally equivalent for this mechanism), but because: (1) it
reuses this project's own already-validated construction instead of
introducing a second, formally-untested one for the same purpose — the
same "reuse, don't duplicate" discipline ADR-0021/ADR-0026/ADR-0040 already
apply elsewhere; (2) it makes no aggregation assumption at all, matching
§12.1's own formal per-goal-only definition of `ς` more directly than
Option A's implicit "these three are interchangeable" assumption; (3) it
is the narrower, more conservative claim, and this project's own review
discipline (§17 A7, "the boring version") favors the narrower
construction when two are shown equivalent for the mechanism under test.
**This remains a recommendation, not settled** — if the owner's own
reading of "Shortfall `ς` (normalised)" intends the aggregate reading
Option A embodies, despite its being functionally indistinguishable from
Option B here, that preference is legitimate to state and would change
this ADR's Q1-adjacent design accordingly, not just a parameter.

## Test plan (what a validation test suite would need to prove — not written here)

**Checked directly before writing item 1 below (not assumed): does DT-4's
*existing* coverage already exercise anything like this?** Both copies of
DT-4 (`crates/firma-kernel/src/tests.rs::dt4_null_fork_equals_continuation`
and `tests/tests/determinism.rs::dt4_null_fork_equals_continuation`, read
in full) fork **exclusively with `Intervention::Null`** — snapshot, fork
with `Null`, run both the fork and an unforked continuation, assert
identical events and identical final state. **Neither test exercises any
non-null variant at all**, `SetAgentReal` included (it does not exist yet)
— and, checked further, **no existing test proves RNG-draw preservation
for any of the *existing* non-null variants either**: the only other
`fork`-based test found (`crates/firma-kernel/src/tests.rs::
freeze_rule_intervention_silences_a_rule`) forks with
`Intervention::FreezeRule` and asserts only the *functional* effect (the
frozen rule stops moving stock) — it never compares against an unforked
continuation's RNG draws or event stream at all.

**Why a generic fork-mechanism test cannot be expected to generalize to a
specific new variant without its own case, stated precisely:** DT-4
proves the *snapshot/restore/fork pipe itself* introduces no spurious
divergence when *nothing* is intervened — i.e., that `RngKey` derivation
depends only on `(run_seed, stream_id, plugin_id, phase_id, tick,
agent_id, purpose_tag)` (§21.2) and not on incidental `World` bookkeeping
a snapshot/restore cycle might otherwise disturb. That is a property of
the *existing* pipe, verified once, for the *no-op* case. It cannot, by
construction, say anything about whether a *not-yet-written* match arm
(`SetAgentReal`'s, or any future variant) correctly limits itself to
`world.set_agent_real(...)` and never — even indirectly, e.g. through a
field some rule's own RNG-relevant logic happens to read — perturbs a key
component. That is a property of the new code, not of the pipe it plugs
into, and needs its own case. Item 1 below is written as that case,
explicitly, not as an assumed extension of DT-4.

1. **A new, specific case — `SetAgentReal`'s own RNG-non-interference
   property, modeled on DT-4's structure but scoped to this variant, not
   an assumed extension of DT-4 itself**: snapshot a run in progress, fork
   it twice — once with `Intervention::SetAgentReal { agent, field:
   PINNED_MARGIN, value: <some h> }`, once with `Intervention::Null` — and
   continue both for the same number of ticks. Assert that **every
   RNG-consuming rule's draws are identical between the two branches at
   every `(tick, agent, plugin, phase, purpose)` key that exists in both**
   — i.e., that applying `SetAgentReal` itself consumes or perturbs no
   RNG stream, the same way DT-4 proves this for `Null`. This assertion
   must hold **regardless of whether the pin causes `decision.
   satisficing` to select a different action** on either branch — the
   claim under test is narrower than "the two branches match" (they
   should *not* match once the pin's causal effect on action selection
   diverges them): it is "the RNG draw *sequence itself*, keyed as
   `RngKey` values, is unperturbed by the act of applying the
   intervention," checkable directly by comparing the sequence of
   `(RngKey, draw)` pairs each branch's event log records, not by
   comparing final state. A second sub-case with `SetAgentReal` targeting
   one of the `PINNED_SHORTFALL_*` keys instead of `PINNED_MARGIN` should
   be included, since it is a structurally identical but textually
   distinct match-arm path (Q4/Q5) worth its own assertion, not assumed
   covered by the margin case.
2. **Independence, VT-8-shaped**: with both `PINNED_MARGIN` and all three
   `PINNED_SHORTFALL_*` set, confirm `h_t`/`sc` fed to `select()` exactly
   equal the pinned values (not a blend, not influenced by real state at
   all) across a representative sample of real-state configurations —
   proving the override is total, not partial.
3. **Persistence across ticks**: one `TimedIntervention` at `at: 0`
   produces a pin that reads identically at every subsequent tick of a
   multi-tick run, with no repeated intervention entries, confirming the
   read-side-persistence design (Q2) actually holds in a real run, not
   only in isolated unit tests.
4. **Real-state independence (Q1's "not a defect")**: confirm
   `constraint.enforce`'s violation detection and `firma_domain::margin::
   standard_margin`'s own return value (computed fresh from real state,
   ignoring the pin) are **unaffected** by an active pin — i.e., a pinned
   firm can still die of a real compliance violation regardless of its
   pinned `h`.
5. **Backward compatibility**: every existing config (no `PINNED_*` keys
   present) produces byte-identical output to today's `decision.
   satisficing` — `Option::None` fallback confirmed on the same
   configs/hashes ADR-0047/0048/0049's own regression rounds already
   exercised.
6. **Rare-quadrant construction**: at least one deliberately "empirically
   rare" `(h, ς)` combination (e.g. high `h`, high `ς`) is constructed and
   confirmed reachable, distinctly from confirming the ordinary-range
   cells.
7. **The shortfall-mapping decision, if adopted (Option B, VT-8-matched,
   per the "genuinely open question" section)**: confirm the three
   `PINNED_SHORTFALL_*` keys, set as `(swept value, 0.0, 0.0)`, produce
   `Focus::attend`/`satisfices()` outcomes identical to VT-8's own
   `[s1, 0.0, 0.0]` grid construction for the same values — a direct
   cross-check against the already-validated precedent this ADR now
   recommends reusing, not a new independent design point. (If the owner
   instead directs Option A — uniform — this item becomes: confirm
   Option A's outcomes equal Option B's for every swept value, per the
   functional-equivalence argument in that section, as a regression
   guard on that claim rather than an independent property.)

## Alternatives

- **Reverse-solve underlying state for a target `h`/`ς`.** Rejected — Q1,
  three reasons (coupling risk, next-tick fragility, rare-quadrant
  unreachability).
- **A new phase dedicated to intervention application.** Rejected — Q2:
  the existing per-tick `apply_interventions_at` dispatcher already runs
  every tick, before that tick's schedule; a new phase would duplicate
  working, tested machinery for no gain.
- **Repeat the intervention at every tick** (write-side repetition instead
  of read-side persistence). Rejected — Q2: works, but is needless config
  bloat and obscures that persistence, not repetition, is the actual
  mechanism.
- **A new `DeltaKind` (e.g. `SetAgentReal` as a `Delta`, applied through
  the ordinary Rule→Delta→reconciler pipeline) instead of an
  `Intervention` variant.** Rejected — interventions are architecturally
  a *different*, already-established mutation path from ordinary rule
  deltas (§6.5's `do(·)`, applied once at a scheduled tick, not proposed
  by a `Rule` and reconciled); every existing `Intervention` variant
  already bypasses `Delta` entirely via direct, RNG-free `World`
  mutation, and this proposal follows that precedent rather than
  introducing a second mechanism for the same conceptual operation.
- **A general "pin any real, anywhere" mechanism with no scoping to
  `decision.satisficing`'s Step 1 specifically** (e.g., a global override
  table every rule consults). Rejected — Q5's scope boundary is
  deliberate: only the decision procedure's own inputs should be
  overridable, not the model's actual constraint-enforcement reality; a
  broader mechanism would risk exactly the kind of silent, hard-to-audit
  behavior change this project's review discipline exists to prevent.

## Consequences

- **Positive.** Arm A's job-config generation gap (the prior round's
  finding: 0/125 cells, 0/25,000 jobs) has a concrete, narrowly-scoped,
  precedent-following design ready for owner review — not a redesign of
  the intervention/fork architecture, which the investigation found
  already exists, is tested, and needs no change. `select()` (ADR-0040)
  is untouched. The kernel-level addition is smaller than the existing
  `SetStock` variant.
- **Negative, accepted.** The shortfall-mapping question (one swept
  scalar → three `ς_j`) is a genuine design decision this ADR proposes
  but does not consider settled — implementation should not proceed on
  it without explicit owner confirmation, flagged, not silently absorbed.
  Two new `firma-domain::keys` constants and one new `Intervention`
  variant are new, permanent model surface, subject to this project's
  "any deviation... requires an ADR" discipline going forward (§0.6/§34.0).
- **Neutral.** No code is written by this ADR. `firma_lab.spec`/`runner.py`
  are untouched (Arm A's `Factor(apply_kind="direct_intervention")`
  entries remain data-only pending the follow-up implementation
  instruction this ADR sets up).

## Compliance

**Implemented, on this branch, by the follow-up instruction this ADR
authorized (owner review complete, both Part A additions made first):**

- `Intervention::SetAgentReal { agent, field, value }`
  (`crates/firma-core/src/intervention.rs`) and its match arm in
  `Kernel::apply_intervention` (`crates/firma-kernel/src/lib.rs`, no
  `rebase_conservation()` call, per Q3/Q4) — exactly as specified above.
  `crates/firma-cli/src/orchestrator.rs::describe_intervention` updated
  for exhaustiveness (the only other place the compiler required a new
  match arm).
- The four `firma_domain::keys::PINNED_*` constants
  (`crates/firma-domain/src/keys.rs`).
- `Satisficing::apply()`'s Step 1 substitution and `Satisficing::
  read_pins` (the partial-pin "all four or none" contract, Part A item 1,
  enforced via a loud `panic!` — `Rule::apply` has no `Result` to reject
  through, so this is this codebase's available equivalent of "validate
  and reject," not true config-load-time rejection; stated explicitly in
  Part A item 1's own text above, not glossed over). `select()`
  (ADR-0040) untouched, confirmed by the diff.
- `firma_lab.spec`'s Arm-A factors (`h`, `varsigma`) now carry real
  `json_path`/`companions` data; `firma_lab.runner.build_job_config`
  implements `apply_kind="direct_intervention"` for real (emitting one
  `TimedIntervention` per agent per pinned field), replacing the prior
  `NotImplementedError`. **Option B (VT-8-matched) adopted**, per this
  ADR's recommendation, re-confirmed rather than silently defaulted: Arm
  A's `varsigma` factor targets `PINNED_SHORTFALL_CAPITAL_GROWTH` with two
  fixed-`0.0` companions for the other two shortfall keys.

**Test plan items, all implemented and passing (real output, not
predicted):**

1. RNG-non-interference —
   `tests/tests/determinism.rs::adr0054_set_agent_real_perturbs_no_rng_stream`
   (both the margin- and shortfall-key sub-cases).
2. Independence — `firma-plugin-decision`'s
   `adr0054_pins_override_real_state_totally`.
3. Persistence across ticks — `tests/tests/integration.rs::
   adr0054_pin_persists_and_real_violations_still_kill` (combined with
   item 4).
4. Real-state independence — same test as item 3.
5. Backward compatibility — this crate's existing 22-test suite (now
   27, the five new ADR-0054 tests added) stays green with zero
   modification to any of the 22; all four frozen hashes (golden,
   `phase1-smoke`, `phase2-smoke`, `phase2-stage5-smoke`) re-confirmed
   byte-identical by direct re-run.
6. Rare-quadrant construction — `firma-plugin-decision`'s
   `adr0054_rare_quadrant_healthy_margin_high_shortfall_is_constructible`.
7. Shortfall-mapping cross-check against VT-8 — `firma-plugin-decision`'s
   `adr0054_shortfall_mapping_matches_vt8_construction`, plus two
   dedicated partial-pin panic tests
   (`adr0054_partial_pin_panics`/`adr0054_single_pin_panics`) beyond the
   original seven-item plan, added because implementing item 5 (backward
   compatibility) required confirming the partial-pin contract itself is
   enforced, not just assumed.

**Full regression, re-run this round:** `cargo test --workspace` green
(the one pre-existing failure, `sc4_wmax_beta_probe`, is the
already-documented H3/ADR-0050 finding, unaffected by this change — its
own numbers, `4/400` at `w_max ∈ {6,9}, β=0.0`, are byte-identical to
every prior round's); `cargo clippy --workspace --all-targets -D
warnings` clean; `cargo fmt --all --check` clean (two files needed
`cargo fmt --all` applied, both mechanical); `lint-architecture.sh` 9/9;
`check_deps.py` clean. Job-config generation re-run against the full real
195-cell spec (not a sample): **Arm A now 125/125 cells, 25,000/25,000
jobs** (up from 0/25,000) — Arm B (12,000/12,000) and Arm C
(1,000/2,000 — its own separate, still-deferred thread) unchanged.

- Confirmed: this ADR does not modify ADR-0044, ADR-0050, ADR-0051,
  ADR-0052, or ADR-0053's own text — each is cited, none is edited.
- **Not done, per the authorizing instruction's explicit hard stop:** no
  real job from the 195-cell/39,000-job design was executed against real
  reserved seeds — job-config *generation* only. The Arm C
  `RandomParams`/β thread remains its own, separate, deferred question.

## Note

The most consequential finding of this investigation is not the design
itself but what *wasn't* needed: the prior round's framing ("design the
Intervention mechanism") suggested inventing fork/snapshot/CRN machinery
from scratch. Reading §18.2's own module contract and the actual
`firma-kernel` code directly showed that machinery already exists, is
already tested (DT-4), and needs exactly one new, small, precedent-
following enum variant to reach Arm A. Treating "the manual specifies an
interface, has anyone actually checked if it's built" as its own explicit
question — separate from "does a mechanism satisfying this description
exist anywhere" — is worth naming as a lesson for the next gap like this
one, not just this specific finding.
