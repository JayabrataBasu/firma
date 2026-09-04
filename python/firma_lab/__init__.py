"""firma_lab -- FIRMA offline orchestration and analysis (manual Sec 23.1).

Stage 7 (Phase 2 MVP, manual Sec 27.2) builds two of the eight modules the
manual's Sec 23.1 table lists -- `load` and `metrics` -- plus explicit
Phase-3 stubs for the rest (`spec`, `runner`, `stats`, `sensitivity`, `plot`,
`prereg`): the E1-sweep tooling that has nothing to compute against yet
until Phase 3 actually runs the sweep.

`metrics` does not reimplement any FIRMA formula in Python. It calls
straight into `firma_lab._native`, the PyO3 extension built from
`firma-analysis` (ADR 0045/0046) -- the same Rust crate `firma-conformance`'s
tests and `firma-tui`'s live panels use. One formula, three consumers.
"""

from firma_lab import load, metrics, spec, runner, stats, sensitivity, plot, prereg

__all__ = [
    "load",
    "metrics",
    "spec",
    "runner",
    "stats",
    "sensitivity",
    "plot",
    "prereg",
]
