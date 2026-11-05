# DRAFT OUTLINE ONLY — not a numbered ADR, not for acceptance

Written in response to a request to draft (not implement) an ADR outline
for E1's base `model_config_template`. This is a list of what such an ADR
would need to cover, not a proposal of values. **No numbered slot claimed**
(no `docs/adr/00NN-*.md` file created) — per the working agreement, writing
and numbering the real ADR is the owner's call once they're ready to decide
these questions, not something to pre-empt here. If adopted, it would follow
manual §34.1–§34.9's shape (Context / Decision / Alternatives /
Consequences / Compliance / Note).

## A. What `model_config_template` must specify

Read from `firma run --model`'s config shape and
`firma_lab.runner.build_job_config`'s patching contract (it deep-copies the
template and patches only the factor fields `build_narrowed_e1_spec`
declares — everything else must already be correct in the template):

- `world.ticks` (run horizon)
- `world.resources`, `world.conflict_resolver`, `world.snapshot_every`
- `world.global_reals` (`theta_limit`, `theta_cap`) and `world.global_ints`
  (`theta_q`, `input_price`, `output_price`) — the baseline values a factor
  patch or scheduled shock then moves
- `agents[]`: how many agents/firms per job, and each one's `stocks`
  (`capital`, `input`), `reals` (`capability`, `legitimacy`,
  `aspiration_capital_growth`, any other seeded aspiration/real field
  `decision.satisficing`/`decision.aspiration_update` read), `ints`
  (`obligation`) where nonzero
- `environment.stocks` (the market-side capital/input reservoirs)
- `rules[]`, in full, for every rule class E1 needs regardless of arm:
  - `decision.satisficing`'s own non-swept params (`h_crit`, `l_w`,
    `action`, and — ADR-0051 Point 7's own still-open question — whether
    `shaping` is configured at all for Arm B/C firms, and at what fixed
    cost/success/lag values if so)
  - the six `action.market.standard.*` rules
  - `action.shaping.rdt_standard.{lobby,contract,diversify,resolve_lagged}`
    and their non-swept params
  - `shock.scheduled`'s shock definition(s) for Arm B: channel, onset tick,
    ramp kind/duration, persistence kind, observability, and `targets` —
    none of which are §30.4-listed factors (only "Novelty" and "Shock
    magnitude" are swept)
  - `constraint.action_window` (`l_w`), `constraint.enforce` (`t_c`, `p_c`,
    `delta_lambda`, `p_q`)
  - `decision.aspiration_update` (`alpha`)
  - an `observation.*` plugin choice (`full`/`noisy(σ)`/`delayed(k)`)
- the `seeds` field (a placeholder — `build_job_config` overwrites it from
  `job.seeds`; confirm this interacts correctly with ADR-0053's seed-stream
  derivation, not just re-derive it informally here)
- Arm-specific variants: at minimum, which rule set Arm C's
  `decision.random` population carries (does it keep `action.shaping.*`
  and a configured `shaping` repertoire, or omit shaping entirely as
  structurally inapplicable to a plugin with no attention/narrowing step?)

## B. Choices the manual leaves open (list only — none defaulted here)

- Whether `shaping` stays configured, at a fixed non-swept value, in Arm-B
  firms' repertoires, or is omitted entirely — **ADR-0051 Point 7's own
  named open question**, explicitly deferred, not re-opened or resolved
  here.
- Population size per job cell (how many firms share one run) — no §30.4
  factor or manual text sets this.
- Agent heterogeneity within one job (identical seeded agents vs. a spread
  of initial capital/capability) — not specified.
- The Arm-B shock's own structural design (channel, onset, ramp, persistence,
  observability) beneath the two swept factors (novelty, magnitude) — not
  in §30.4's table at all.
- Baseline `theta_limit`/`theta_cap`/`theta_q` values before any shock or
  Arm-A intervention — not given a §30.4-listed value.
- `h_crit` and `decision.aspiration_update`'s `alpha` for E1 specifically —
  §16.1 gives project-wide defaults (0.15, 0.10) but §30.4 does not restate
  them as E1's own fixed (unswept) values; carrying the default over is a
  template decision, not something the manual pins down for this
  experiment by name.
- `world.ticks` for E1 — §16.1/§27.2 give a project-wide default (T = 400),
  but §30 never restates it for E1 specifically, and §30.6 cites
  survival-analysis power for H4 as the reason for 200 seeds/cell without
  saying whether 400 ticks is also chosen for that reason or inherited by
  default.
- Observation plugin choice for E1 — §35.1's declared limitation ("exact
  self-observation of margin") suggests `observation.full` throughout, but
  this is stated as an MVP-wide scope limitation, not as an explicit E1
  config field; the template must set it, not infer it from the limitation
  note.
- Environment-side capital/input pool sizes — not specified by §30.4.
- How `|A^{used}|` vs `|A^{adm}|` (§28.3's required alternative-explanation
  check) gets computed/reported for this specific template — a measurement
  question the base config's design can make easy or hard depending on the
  above choices, not yet worked through.

## C. Dependency on the A/A-sym (ADR-0057) decision

- Whichever of {current frozen-`u` behaviour, Option A, Option
  A-symmetric, Option B alone, or "document as an intended model property"}
  the owner eventually accepts for ADR-0057 changes the numerical behaviour
  of every `SURVIVAL`-focus decision under compliance pressure — which
  bears directly on Arm B's endogenous regime (H1c, the arm that evolves
  `h`/`ς` from the model's own dynamics under a shock) and on any Arm-A/C
  agent whose organic or pinned state lands it in a compliance-bound
  `SURVIVAL` focus.
- Per §30.6's own existing rule — **"Any Phase 2 run touching the
  registered design grid invalidates this registration and requires
  re-registration with a fresh seed range"** — building and locking the E1
  base template, then later adopting an ADR-0057 fix that changes
  `SURVIVAL`'s behaviour, would invalidate whatever was already filed/run
  under seeds 1–200. This is a sequencing dependency, not a technical one:
  **ADR-0057 most likely needs to be resolved (A, A-symmetric, B-alone, or
  "no change") before the E1 base template is finalized and before the
  reserved seed range is consumed**, to avoid burning that range on a
  design the kernel's own decision behaviour later invalidates.
- This outline does not resolve that ordering — it is listed here as a
  dependency the real ADR would need to state plainly in its Context,
  not something to decide inside this draft.

## D. Status

Draft outline only, written for review. No value is proposed for any item
in B; no file under `docs/adr/` was created; no ADR number is claimed.
