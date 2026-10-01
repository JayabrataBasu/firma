"""firma_lab -- FIRMA offline orchestration and analysis (manual Sec 23.1).

All eight modules of the manual's Sec 23.1 table exist: `load` and
`metrics` (Stage 7, Phase 2 MVP), `spec` and `runner` (ADR-0053/0054 round),
and `stats`, `sensitivity`, `plot`, `prereg` (the Phase-3 analysis
pipeline, built and tested on synthetic data only -- no registered-design
output exists yet).

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
