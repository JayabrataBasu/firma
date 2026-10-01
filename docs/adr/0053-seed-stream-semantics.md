# ADR 0053 — Seed/stream semantics for the narrowed E1 design: `derive_seeds` matches all four `StreamSeeds` fields to the replicate index

**Status:** Accepted (owner-directed close-out after a stale-evidence finding,
OQ-15: Part 4's overlap table was re-run in full and Part 5's §30.6
reasoning extended to every overlap found, *before* acceptance, as DRAFT
edits — not as a post-acceptance correction). Per the ADR immutability
house rule, this document's body is now append-only; any further
correction is a new, superseding ADR, the `Status` line itself excepted.
**Phase:** Phase 2→3 boundary (tooling for ADR-0051's narrowed E1 design;
implemented in `firma_lab.spec`/`firma_lab.runner`, this instruction's
Part C)
**Supersedes:** Nothing.
**Relates to:** manual §21.2 (RNG architecture, key formula), §21.3
(stream separation, the matched-environment design), §30.5 (randomisation
and blinding), §30.6 (sampling, the seed-range-invalidation rule), ADR-0051
(the 195-cell narrowed E1 design this seed scheme serves), `firma_config::
StreamSeeds` (`crates/firma-config/src/lib.rs`)

## Context

§30.5/§30.6's "Seeds 1–200 per cell" language is ambiguous against the
actual `StreamSeeds` struct — four independent `u64` fields
(`mechanism`, `environment`, `shock`, `init`), not one scalar "seed."
Before any real job for the narrowed E1 (195 cells × 200 seeds = 39,000
runs, ADR-0051) gets built, this needed one unambiguous, documented,
*tested* answer. §21.2, §21.3, and §30.5 were read directly for this ADR
(not from any prior paraphrase) before designing anything.

## Decision

### Part 1 — the seed-derivation function

`firma_lab.spec.derive_seeds(replicate: int) -> StreamSeeds`
(`python/firma_lab/spec.py`): for replicate index `n`, **every one of the
four `StreamSeeds` fields is set to `n`** —
`StreamSeeds(mechanism=n, environment=n, shock=n, init=n)`.

**Reasoning, stated precisely (also recorded as the module's own header
comment, since this is exactly the kind of design decision this project
requires be enforced in code, not left in a docstring alone):**

1. §21.3's one illustrative sentence — *"A matched-environment design —
   same `environment` and `shock`, different `mechanism` configuration —
   removes a large variance component at zero cost"* — names environment
   and shock as the fields a matched design holds fixed while "mechanism
   *configuration*" (the swept factor's parameters — which `h`/`ς`/`β`
   cell, which decision plugin) differs. It is not phrased as an
   exhaustive list of which *stream seeds* may not also be matched; it
   contrasts held-fixed streams against a differing *configuration*, a
   different thing from a differing *seed*.
2. **Checked directly against the actual RNG key formula, not assumed:**
   §21.2's key formula — `H(run_seed ‖ stream_id ‖ plugin_id ‖ phase_id ‖
   tick ‖ agent_id ‖ purpose_tag)` — does not take any economic parameter
   value (`h`, `ς`, `β`, novelty, shock magnitude, …) as an input. Two
   cells sharing the same `mechanism` seed therefore produce bit-identical
   draw sequences at every `(tick, agent, plugin, phase, purpose)`
   regardless of which factor-level cell they belong to. Matching
   `mechanism`'s seed (not only environment/shock) only *strengthens* the
   "all condition comparisons use common random numbers" property §30.5
   requires, at the same zero cost §21.3 already claims for
   environment/shock.
3. **"Seeds 1–200 per cell" is read literally**: every cell, across every
   arm, draws from the *same* numbered range 1–200 — not a disjoint
   per-cell or per-arm sub-range. A single function of the replicate index
   alone, with no arm- or cell-specific branching, is the simplest scheme
   consistent with that reading, and is what `ExperimentSpec.jobs()`
   implements (one `derive_seeds(n)` call per replicate, reused across
   every cell at that replicate — see Part 3).
4. `init` is not named in §21.3's one sentence, but nothing in the manual
   treats `init` as a *swept factor* the way `h`/`ς`/`β`/novelty/shock-
   magnitude/decision-plugin are — it plays the same non-condition-varying
   role as environment/shock ("initial conditions," not a treatment).
   Point 2's reasoning (the key formula's independence from cell config)
   applies to it identically; matching it too keeps the design uniform
   rather than inventing an unstated fifth rule for exactly one field.

### Part 1b — re-investigated: is this a textual derivation or a design decision? (added this round, after a closer, code-grounded read)

A later review round asked directly whether "different mechanism
configuration" (§21.3) could instead mean the `mechanism` **seed itself**
should *differ* across compared cells (only `environment`/`shock` matched)
— the opposite of what Part 1 adopted. Re-read in full (not fragments):
§21.2, §21.3, all of §30.5, §20.4's worked config example, and — because
this project's own primary outcome is defined via exactly this mechanism
— §14.6 and §6.5.

**§21.2, in full:**

> Counter-based (Philox-4×32 or Threefry-4×64) with hierarchical key
> derivation.
> ```
> key  = H(run_seed ‖ stream_id ‖ plugin_id ‖ phase_id ‖ tick ‖ agent_id ‖ purpose_tag)
> draw = philox(key, counter)
> ```
> Five required properties; a stateful generator fails four:
> 1. Order invariance — results independent of agent processing order, so
>    parallelism cannot change outcomes.
> 2. Random access — any draw regenerable from its key without replaying
>    history.
> 3. Common random numbers — a fork reuses identical keys for identical
>    `(plugin, phase, tick, agent, purpose)` tuples, so only the
>    intervention differs.
> 4. Stream isolation — adding or removing a rule perturbs no other
>    rule's draws. [...]
> 5. Statistical quality — Philox and Threefry pass standard batteries.

**§21.3, in full** (this is the entire section — confirmed by reading
past it to §21.4's heading; there is no further sentence, example, or
footnote):

> Four independent, separately seeded streams: `mechanism` (decisions,
> action outcomes, shaping success), `environment` (resource dynamics,
> exogenous variation), `shock` (timing and magnitude), `init` (initial
> conditions).
>
> A matched-environment design — same `environment` and `shock`,
> different `mechanism` configuration — removes a large variance
> component at zero cost.

**§30.5, in full:**

> Counter-based RNG, four streams (§21.2–21.3). Seeds 1–200 per cell.
> Matched-environment design. **All condition comparisons use common
> random numbers.** Analysis of flagged events uses blinded mode (§23.2).

Neither section, read in full, contains a sentence that directly settles
whether `mechanism`'s own *seed* is matched or varied across compared
conditions — Part 1's conclusion was, honestly, an inference from "the
key formula doesn't depend on config" (Part 1 point 2), not a quoted
rule. This round went further and checked the mechanism §21.2 property 3
and §21.3 actually describe — "a fork" — against its concrete technical
definition, not just its name:

- **`Intervention` (`crates/firma-core/src/intervention.rs`) and §6.5's
  operator table have no "fork" variant at all.** A fork is not one of
  the `do(a)` operators (`set_state`, `set_param`, `remove_rule`/
  `add_rule`, `freeze_rule`, `apply_shock`, `remove_agent`/`add_agent`);
  §6.5 defines it as $X^{do(a)}_T = a(X_T)$, evolution thereafter under
  identical $\kappa$ — applying an intervention *to a state*, then
  continuing under the same parameters. A "fork" is the *pairing* of that
  counterfactual continuation against the actual (factual) one, both
  starting from the same state.
- **This project's own primary outcome is defined this way, concretely**
  (§14.6, in full): *"For E1 the fork is the no-shock branch. The
  reported quantity is the CRN-matched paired difference
  $\Delta H_{\text{rep}} = H_{\text{rep}}^{\text{factual}} -
  H_{\text{rep}}^{\text{no-shock}}$, not the raw level."* The fork here
  is a shocked run's *own* no-shock counterfactual — not a comparison
  between two different h/ς/β/novelty *cells*.
- **Checked directly in code, not assumed: a fork's "identical keys"
  property (§21.2 property 3) requires identical `StreamSeeds` on *all
  four* fields, not just environment/shock.** `crates/firma-rng/src/
  lib.rs`'s `key()` takes `run_seed: u64` as one undifferentiated
  parameter; `crates/firma-kernel/src/lib.rs:692` supplies it as
  `world.seeds.for_stream(stream)`, and `crates/firma-kernel/src/
  world.rs:31-34` shows `for_stream` maps `StreamId::Mechanism ->
  self.mechanism`, `::Environment -> self.environment`, `::Shock ->
  self.shock`, `::Init -> self.init` — i.e. **every stream's `run_seed`
  component comes from that stream's own `StreamSeeds` field.** For a
  `mechanism`-stream draw's key to be identical between a factual run and
  its fork (property 3's own requirement), `StreamSeeds.mechanism` must
  be identical between them — a fork, as this codebase actually
  implements the idea, **cannot** have a different `mechanism` seed from
  its parent and still produce "identical keys." Reading (b) (mechanism's
  seed should *differ* across compared conditions) is not merely
  unsupported here — it would make the fork mechanism §21.2 property 3
  and §14.6 both describe not work as stated.
- **`tests/tests/integration.rs::stage5_rng_streams_follow_the_declared_
  stream`, read directly (not restated from an earlier claim):** it holds
  `environment=5, shock=77, init=1` fixed and varies only `mechanism`
  (`"11"` vs. `"999"`), asserting the *shock* stream's own draw is
  unchanged. Its own in-code comment: *"Matched-environment (§21.3): the
  shock draw is on the `shock` stream, so changing only the `mechanism`
  seed does not move it."* **This tests stream isolation (§21.2 property
  4) — that changing `mechanism` does not perturb `shock`'s draws — not
  whether `mechanism`'s seed should be matched or varied across compared
  experimental *cells*.** No claim was found anywhere in this session
  (this ADR included) that this test constitutes evidence for either
  reading, and having now read it directly: it does not. It is evidence
  for neither (a) nor (b) — a genuinely different, unrelated question
  (§21.2 property 4), exactly the third option this round's own
  instruction offered.

**Conclusion, calibrated honestly.** No single sentence in the manual
explicitly states "match `mechanism`'s seed across compared cells too."
But reading (b) is not merely less-supported than (a) — it is
**structurally incompatible** with how this codebase's own "fork" (the
manual's one concrete, worked instance of "matched-environment design...
common random numbers," §14.6) is technically required to work: a fork's
defining property (identical keys) is impossible under reading (b) for
the `mechanism` stream specifically. Combined with §30.5's own general
"**all** condition comparisons use common random numbers" (not "the
shock/no-shock fork alone"), Part 1's adopted design — match all four
fields — has real structural support, not just plausibility. What remains
genuinely open, honestly: whether the manual's "all condition
comparisons" was intended to reach as far as Arm A/B/C's **cross-cell**
sweep (125/60/10 independently-run configurations, not literal engine
forks of one another) the way it demonstrably reaches the shocked/
no-shock fork pairing. No text was found that answers this extension
question directly either way. `derive_seeds`'s cross-cell matching is
therefore the **structurally consistent, variance-reducing extension**
of a principle the manual clearly requires for the fork case, applied by
inference (standard CRN practice: share the same underlying draws across
compared systems, vary only the treatment) to the cross-cell case the
manual does not separately spell out. **This extension is a design
decision, made and justified here, not a directly quoted manual rule —
recorded as such, not overclaimed as textually settled.** If the owner's
own reading of "all condition comparisons" differs, that is exactly the
kind of disagreement this ADR being DRAFT, not Accepted, exists to catch
before it is load-bearing.

**No change to `derive_seeds`'s implementation follows from this
re-investigation** — the evidence found this round supports the adopted
design more strongly than Part 1's original citation alone did, not less.

`derive_seeds` is deterministic (same `replicate` → equal output) and
injective in `replicate` (trivially — `replicate` is reproduced verbatim
into every field, so two distinct inputs cannot collide on all four
fields simultaneously). It is **not** bounded to `1..=200` internally —
only `replicate >= 1` is enforced — because that bound is the
*registered E1 design's* own choice (`ExperimentSpec.num_replicates`),
not a property of the general index-to-seeds mapping; a later, separate
H3/E3 filing (ADR-0051 Point 6) needs a different, non-overlapping
replicate range from the *same* function, not a different function.

### Part 2 — one scheme for all three arms, checked against §30.4, not assumed

The instruction asked whether Arm A/B/C need different matching
behaviour. Checked directly against what each arm actually compares
(§30.4, §30.7):

- **Arm A** (`h`/`ς` set by direct intervention, §30.4) — the mixed-model
  formula (§30.7: `ΔH_rep ~ ς + h + ς:h + β + (1|seed)`) treats `seed` as
  a random-effects grouping factor across the whole 125-cell grid: since
  `h`/`ς` are *forced*, not organically seed-dependent, the cleanest
  matched design shares one seed tuple across all 125 cells at a given
  replicate, exactly what the adopted scheme gives.
- **Arm B** (`h`/`ς` evolve endogenously under an applied shock) — this is
  §21.3's own illustrative case; the adopted scheme satisfies it and
  extends it (Part 1, point 2).
- **Arm C** (`decision.satisficing` vs. `decision.random`, β swept) — one
  scheme still applies cleanly: `decision.satisficing` draws no
  randomness from `mechanism` at all (`apply()` never touches
  `firma_rng`, per `firma-plugin-decision`'s own module docstring), so
  matching `mechanism`'s seed across the two decision-plugin cells is
  inert for the satisficing side and gives genuine CRN for
  `decision.random`'s own draws.

**One scheme for all three arms is therefore justified by checking each
arm's actual comparison, not assumed** — no arm's design requires a
different matching rule, so `derive_seeds` takes no arm parameter.

### Part 3 — enforced in code, not just documented

`ExperimentSpec.jobs()` (`python/firma_lab/spec.py`) computes
`derive_seeds(n)` **exactly once per replicate** (a dict comprehension
over `1..=num_replicates`) and reuses that one result across every cell
in every arm at that replicate — it does not call `derive_seeds` per
cell. `firma_lab.runner` never calls `derive_seeds` itself; every job's
seeds come from the `Job` object `ExperimentSpec.jobs()` already
produced. This is the "single, pure, documented function... reused, not
re-derived" requirement enforced structurally: there is exactly one call
site that ever invokes `derive_seeds`, and `firma_lab/spec.py`'s own
module docstring names it as such.

**Unit tests** (`python/tests/test_spec.py`, run and passing — 42/42):
determinism (`test_derive_seeds_deterministic`), injectivity across the
full 1–200 range with an explicit collision check
(`test_derive_seeds_injective_no_collisions_1_to_200`), the matched
property verified on real `Job` output rather than only on `derive_seeds`
in isolation (`test_jobs_matched_across_cells_at_the_same_replicate`),
genuine per-replicate variation
(`test_jobs_vary_genuinely_across_replicates`), and no job-identity
collision across the full real 39,000-job space
(`test_narrowed_e1_spec_no_full_job_id_collision_across_39000_jobs`).

### Part 4 — overlap re-check, real numbers (re-run in full before acceptance)

**No prior "earlier audit" of this kind was found anywhere in this
repository** before this ADR's first draft, so the check was run fresh.
**It was then re-run in full immediately before this ADR's acceptance**,
because the first draft's table had gone stale: a later round (ADR-0054's
implementation) added a new exact collision, and the first draft had only
enumerated Rust sources — it missed Python tests that *execute* the
`firma` binary. The table below supersedes the first draft's.

Method: every literal `"mechanism"/"environment"/"shock"/"init"` tuple in
`tests/`, `crates/`, `configs/`, and `python/` (regex over `.rs`, `.json`,
`.py`), plus every Python test that executes jobs through
`firma_lab.runner` (whose seeds come from `derive_seeds`, not literals),
checked for an **exact** `(n,n,n,n)` match with `n ∈ [1, 200]` — the only
tuple shape the adopted scheme ever assigns.

| Location | What it is | Registry | Tuple(s) | Exact `(n,n,n,n)`? |
|---|---|---|---|---|
| `tests/tests/determinism.rs:437` | `tick_is_atomic_wrt_event_log_on_invariant_abort` (DT, Phase 1) | `standard_registry()` | `(1,1,1,1)` | **Yes — n=1** |
| `tests/tests/determinism.rs:44` | `dt2_agent_order_irrelevant` (DT, Phase 1) | `standard_registry()` | `(4,4,4,4)` | **Yes — n=4** |
| `tests/tests/integration.rs:624` | `adr0054_pin_persists_and_real_violations_still_kill` (added by ADR-0054's implementation) | **`model_registry()`** | `(1,1,1,1)` | **Yes — n=1** |
| `python/tests/test_runner.py` (8 tests via `build_synthetic_spec` / `_failing_spec`) | `firma_lab.runner` pipeline tests, real `firma run --model` subprocesses | **`model_registry()`** | `derive_seeds(1..=3)` = `(1,1,1,1)`, `(2,2,2,2)`, `(3,3,3,3)` | **Yes — n=1, 2, 3** |
| `tests/tests/determinism.rs:254`, `:325` | DT tests | `standard_registry()` | `(31337,1,1,1)`, `(424242,1,1,1)` | No (mechanism ≠ others) |
| `tests/src/lib.rs:79` | `active_config` (DT-1/4/5, and ADR-0054's RNG-non-interference test) | `standard_registry()` | `(777,12,5,9)` | No |
| `tests/tests/sanity.rs` (8 configs) | SC-gate / H3 diagnostic configs | `model_registry()` | `(55\|7\|5\|909\|11\|47, 2, 3, 4)`, `(8080,41,97,13)`, and `cfg_random_binding_seed(m)` = `(m,2,3,4)` for `m ∈ {4242,1,77,900001,31337}` | No |
| `tests/tests/golden.rs:15`, `integration.rs:493`, `tui_live_demo.rs:25` | golden / smoke / TUI demo | mixed | `(20260903,7,11,13)`, `(7,20260906,3,1)`, `(20260906,2,3,4)` | No |
| `configs/experiments/*.json` | shipped smoke configs | mixed | `(1,2,3,4)`, `(20260904,7,11,13)`, `(20260905,41,97,13)` | No |
| `crates/firma-config/src/lib.rs:293`, `crates/firma-cli/src/orchestrator.rs:568`, `python/tests/test_spec.py:71` | unit-test fixtures | — | `(1,2,3,4)`, `(11,22,33,44)`, `(1,2,3,4)` | No |

**Also disclosed, because §30.6 says "any Phase 2 run," not "any committed
test":** during the ADR-0054 implementation round (this conversation's
transcript; outputs in a session scratchpad, never committed), ad hoc CLI
runs were executed at seeds `(1,1,1,1)`, including one config generated by
`build_job_config` from the real narrowed-E1 spec's **Arm A cell 0**
(`h=0.02, ς=0.0, β=0`, replicate 1) — but on an ad hoc one-firm,
20-tick smoke-test template, not a registered template (none exists).
Its `focus` writes were inspected (all `0`, SURVIVAL). This is the closest
any run in this project's history has come to a registered cell, and it
is named here rather than left in a transcript.

### Part 5 — §30.6 invalidation reasoning, checked against every overlap found

§30.6, in full: *"Simulation runs, not human participants. **No runs of
the registered design have been executed.** Phase 2 exploratory runs,
executed to establish sanity conditions, MUST be disclosed as exploratory.
**Any Phase 2 run touching the registered design grid invalidates this
registration and requires re-registration with a fresh seed range.**"*

**What "touching the registered design grid" is taken to mean, stated
as the criterion this ADR applies (the manual does not define it
further):** a registered run is fully determined by its *entire* resolved
config — the registered `model_config_template` (agent population,
environment, world, rule set), the cell's factor levels, and the
replicate's seeds. The failure §30.6 exists to prevent is data snooping:
having already seen an outcome that will be part of the confirmatory
dataset, or enough of it to shape later analysis choices. A run touches
the grid in that sense **iff** it reproduces (or is outcome-equivalent to)
a registered run — i.e. its resolved config equals a registered job's
resolved config. Sharing a seed value, or a factor value, with a
registered job is not sufficient: FIRMA's trajectory is a deterministic
function of the whole config, so a run with a different template produces
a different trajectory at the same seed and factor values, and its
outcome is not an observation from the registered dataset.

Applied to each overlap:

1. **`determinism.rs:437` (n=1), `determinism.rs:44` (n=4)** — Phase-1
   kernel tests under `standard_registry()`, structurally incapable of
   loading any E1 rule. Not touching, by the strongest possible argument
   (unchanged from this ADR's first draft).
2. **`integration.rs:624` (n=1)** — runs under `model_registry()` with
   `decision.satisficing` and the real ADR-0054 pin mechanism, at
   `h=0.40`, `ς_1=ς_2=ς_3=0.0`, default `β=1.0` — values that coincide
   with a real Arm-A cell. What it inspects, read directly from the test
   body: the tick set of `InterventionApplied` events, every `focus` and
   `selected_action` write for agent 0, and whether `AgentDied` occurs.
   Correcting a framing that circulated in review: **this is not the RNG
   non-interference test** — that is `determinism.rs::
   adr0054_set_agent_real_perturbs_no_rng_stream`, under
   `standard_registry()` at `(777,12,5,9)`, no collision. This test *does*
   observe raw ingredients of E1's DVs — the realized action sequence
   (`H_rep`'s input, §14.2 R1) and a death event (`survival_time`'s
   input) — so the weaker "it only checked mechanics" argument is **not**
   available for it. It is nonetheless not touching the grid under the
   criterion above: its template (one firm, capital/input 5,000, seeded
   `selected_action = 2`, `θ_limit = 0.40`, 20 ticks, a rule set without
   `decision.aspiration_update` or any Observation/shock plugin) is not a
   registered template — none has been fixed — so its trajectory is not a
   registered run's trajectory. What it demonstrates (a healthy pin with
   zero shortfall resolves `Focus::None`, and a firm pinned healthy still
   dies of a real violation) is a property of the mechanism's code,
   already knowable from reading `select()` and `constraint.enforce`, not
   an outcome of the registered experiment.
3. **`python/tests/test_runner.py` (n=1, 2, 3)** — real `model_registry()`
   runs with `decision.satisficing` (default `β=1.0`, organic `h`/`ς`),
   varying only `world.ticks ∈ {5,8}` and `l_w ∈ {4,8}` — neither a
   registered factor — on the synthetic one-firm template. Assertions
   concern pipeline mechanics (manifests, resumability, failure recording,
   `event_log_sha256` equality under concurrency), not any outcome value.
   Not touching.
4. **Ad hoc scratch runs (disclosed in Part 4)** — including the one
   generated from Arm A cell 0 with replicate-1 seeds. Same analysis as
   item 2: a non-registered template, so not a registered run; the only
   value inspected (`focus = 0`, SURVIVAL, at a pinned `h = 0.02 <
   h_crit`) is the mechanism's deterministic response, not a registered
   outcome.

**Conclusion, stated plainly: none of these overlaps constitutes touching
the registered design grid, and no re-registration is required — with one
explicit condition.** Every argument in items 2–4 rests on the registered
template differing from each of these test/scratch templates. That
condition cannot be verified today (the registered template does not
exist yet) and **must be verified mechanically when it is fixed**: before
filing, no executed exploratory run's resolved config (seeds included)
may equal any registered job's resolved config. This is operationalised
as a check in `firma_lab.prereg` (built alongside this ADR's acceptance),
not left to memory. A lower-cost way to remove the question entirely for
future test additions — moving test fixtures' seeds outside `[1, 200]`,
the convention most of this codebase's diagnostic configs already follow
— is recommended but not applied here (out of this ADR's scope).

**A known limitation, recorded rather than implied away:** Part 1 says a
later H3/E3 filing (ADR-0051 Point 6) "needs a different, non-overlapping
replicate range from the same function." `derive_seeds` supports that;
`ExperimentSpec.jobs()` does not yet — it always enumerates replicates
`1..=num_replicates`. A later filing needs a `first_replicate` (or
equivalent) field on `ExperimentSpec`; not built.

## Alternatives

- **Match only `environment`/`shock`, let `mechanism`/`init` vary
  independently per cell.** Rejected — §21.2's key formula shows matching
  `mechanism`/`init` too costs nothing and only strengthens the CRN
  property; inventing a *weaker* design when a stronger one is free and
  equally consistent with the manual's text is not justified.
- **A per-arm seed-derivation scheme** (different matching rules for A, B,
  C). Rejected — Part 2 checked each arm's actual comparison directly and
  found no arm needs different behaviour; a per-arm scheme would be
  unjustified complexity.
- **Bound `derive_seeds` to `1..=200` internally.** Rejected — conflates
  the general index-to-seeds function with this *particular* filing's
  seed-range choice; a later H3/E3 filing needs the same function with a
  different range (ADR-0051 Point 6), not a new function.

## Consequences

- **Positive.** A single, tested, documented answer to a previously
  ambiguous question, enforced in code (one call site) rather than left
  to convention. The registered seed range 1–200 is confirmed still
  clean under the adopted scheme, with real numbers, not restated from an
  untraceable prior claim.
- **Negative, accepted.** The in-range collisions found (Part 4: `n=1`,
  `n=4` in Phase-1 DT tests; `n=1` in `integration.rs:624`; `n=1..3` in
  `python/tests/test_runner.py`; plus disclosed ad hoc scratch runs) are
  cleared only *conditionally* — on the registered template differing from
  every one of those test/scratch templates (Part 5). That condition is
  checked mechanically by `firma_lab.prereg`, not assumed.
- **Neutral.** One function (`derive_seeds`), integrated into
  `ExperimentSpec.jobs()`; no code outside `firma_lab` is touched by this
  ADR.

## Compliance

- `python/tests/test_spec.py` — 10 tests directly on this ADR's subject,
  run and passing: `test_derive_seeds_{deterministic,
  matched_across_all_four_fields, injective_no_collisions_1_to_200,
  rejects_zero_and_negative, not_bounded_to_200}`,
  `test_stream_seeds_to_dict_shape`, `test_jobs_{matched_across_cells_at_the_same_replicate,
  vary_genuinely_across_replicates, count_matches_total_jobs}`, and
  `test_narrowed_e1_spec_no_full_job_id_collision_across_39000_jobs`.
- `firma_lab.prereg`'s template-identity check (Part 5's condition).
- Any future change to `derive_seeds`'s field-matching design, or to which
  fields the registered E1 design treats as swept vs. matched, MUST cite
  this ADR.
- A later H3/E3 filing (ADR-0051 Point 6) MUST choose a replicate range
  disjoint from `1..=200` when calling `derive_seeds` for its own design —
  this ADR does not assign that range.

## Note

The collisions found in Part 4 are almost certainly coincidental — small
integers are an obvious, cheap choice for a hand-written test's seed, and
`derive_seeds(1..=k)` is the natural thing for a pipeline test to use.
The more useful lesson is the one that forced this ADR's Part 4 to be
redone before acceptance: an evidence table enumerating "every occurrence
in the repository" is only true at the moment it was run. The first draft
went stale within one round, and had also silently scoped itself to Rust
sources. Recorded exactly as what it is: a coincidence checked and found
harmless *under a stated, mechanically-checkable condition* — not a
coincidence assumed harmless.
