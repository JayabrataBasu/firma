# ADR 0058 — Seeded population heterogeneity in the config generator

**Status:** DRAFT — pending owner review. Nothing is registered; no run config in
`configs/` uses this.
**Phase:** 3 (E1 base-config work). **Supersedes:** nothing.
**Relates to:** ADR-0003 (keyed RNG), ADR-0030 (config seeds domain state),
ADR-0054 (Arm A pins), manual §9.2, §16.2, §26.4. Evidence: review round R20
report (`docs/review/outbox/R20-hetero-report.md`), not this file.

## Context

Hand-specified firm types (regime grids R16, R18) behave as one agent: the
only regime that met the sanity targets collapsed in a single tick. The owner
decided to draw each firm's starting values per seed, in the config generator.

## Decision

A run's `agents` list is generated outside the engine from the run's **init
seed**. For firm `i` and purpose tag `p`:

    u = first 8 bytes (big-endian) of SHA-256("{init_seed}|{i}|{p}") / 2**64

`u` is uniform on [0, 1); normal and lognormal values come from the inverse
normal CDF of `u`. There is no stateful generator: a value depends only on
`(init_seed, firm id, purpose)`, never on how many firms or purposes exist or
on generation order. Drawn per firm: starting capital (lognormal, median
`s_L` = 100), capability (clipped normal), `aspiration_capital_growth`
(uniform), and an initial obligation with probability 1/6. The exact
parameters and three spread levels are in
`docs/review/evidence/R20/hetero_spec.json`. Implementation:
`scripts/population_generator.py`. No kernel, engine, plugin or `firma_lab`
change; the engine sees an ordinary config.

## Assumptions

- The init seed is the right stream to key on: population draws are
  initial conditions, and must not move when mechanism, environment or shock
  seeds change.
- SHA-256 output bits are uniform enough for 12 to a few hundred draws.
- Firms are independent draws from one distribution (no firm types, no
  correlation between capital and capability).
- The distributions and spreads are reviewer-proposed starting points, not
  calibrated values (manual §33.3).

## Alternatives rejected

- **Hand-specified types** (R18): synchronised collapse.
- **Drawing inside the engine** from the kernel's keyed RNG: a kernel or
  plugin change for something a config can carry.
- **A stateful generator seeded once** (`random.Random(init_seed)`): a firm's
  values would depend on generation order and on the number of firms.
- **Shocking a subset of firms via `targets`**: the regulatory channel
  ignores `targets` (R20 Q6).

## What would falsify it

- Any firm's values change when another firm is added or the generation
  order changes; or the same seed gives a different config or event log.
- Heterogeneity does not remove the same-tick collapse, or does not give a
  stressed-but-alive population across seed sets (R20 Q2–Q4 criteria).
- The drawn values do not reach a pinned firm's actions (R20 Q5), so the draw
  cannot supply Arm A's between-seed variance.

R20's first measurement is in the report. The second and third conditions
were met there: as proposed, this decision is **not** supported by R20.
