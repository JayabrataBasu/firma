"""firma_lab.sensitivity -- robustness across operationalisations (manual
Sec 14.5, Sec 30.7) and variance-based sensitivity over a factorial design
(Sec 23.1, Sec 29.2).

**Robustness (Sec 30.7: "results must hold under R1, R2, R3 and under N1
and N2. Disagreement is the result and will be reported as such").**
``robustness`` re-runs one ``firma_lab.stats`` analysis once per
operationalisation, changing only the arguments that operationalisation
changes, and reports every verdict side by side. It never picks a
"favourable" measure (Sec 14.5); ``holds_under_all`` is true only if every
variant is ``"supported"``.

The directions below come from the Sec 14.2 definitions, not from data:

- **R1** repertoire entropy (``ΔH_rep`` in E1, Sec 14.6): higher = broader
  repertoire -> ``outcome_direction = +1``;
- **R2** search narrowing ``mean(w_eff) / w_max``: higher = wider search ->
  ``+1``;
- **R3** shaping abandonment ``SA = 1 - shaping share of committed
  capital``: higher = more rigid -> ``-1``.

N1/N2 (Sec 13.3) are alternative *novelty* measures; for H2/H4 they replace
the novelty column.

**Variance-based sensitivity.** Sec 23.1 names Sobol' indices and Morris
screening. On a *full factorial* design with the factors taken as uniform
over their registered levels, first-order and total Sobol' indices of the
cell-mean response are exact finite-population variance ratios
(``factorial_sobol_indices``); no extra runs are needed. **Morris screening
is not built**: it needs its own one-at-a-time trajectory sample through
parameter space, i.e. runs outside the registered grid -- a separate,
exploratory experiment, not an analysis of E1's output.
"""

from __future__ import annotations

import itertools
from dataclasses import dataclass, field
from typing import Any, Callable, Mapping, Sequence

import pandas as pd

from firma_lab.stats import HypothesisResult, InsufficientDataError

#: Sec 14.2 direction of each rigidity operationalisation (see module doc).
RIGIDITY_DIRECTIONS: Mapping[str, int] = {"R1": 1, "R2": 1, "R3": -1}


def rigidity_variants(*, r1: str, r2: str, r3: str) -> dict[str, dict[str, Any]]:
    """Argument overrides for R1/R2/R3: the outcome column each is stored in,
    plus its Sec 14.2 direction."""
    cols = {"R1": r1, "R2": r2, "R3": r3}
    return {
        name: {"outcome": col, "outcome_direction": RIGIDITY_DIRECTIONS[name]}
        for name, col in cols.items()
    }


def novelty_variants(*, n1: str, n2: str) -> dict[str, dict[str, Any]]:
    """Argument overrides for N1/N2: the novelty column each is stored in."""
    return {"N1": {"novelty_col": n1}, "N2": {"novelty_col": n2}}


@dataclass(frozen=True)
class RobustnessReport:
    """Every variant's verdict for every hypothesis the analysis returned."""

    verdicts: Mapping[str, Mapping[str, str]]  # hypothesis -> variant -> verdict
    results: Mapping[str, Mapping[str, HypothesisResult]]
    #: variant -> reason, for a variant whose analysis could not be estimated
    #: at all (``InsufficientDataError``). Such a variant counts against
    #: ``holds_under_all`` and towards ``disagreement`` for every hypothesis.
    not_estimable: Mapping[str, str] = field(default_factory=dict)

    def holds_under_all(self, hypothesis: str) -> bool:
        return not self.not_estimable and all(
            v == "supported" for v in self.verdicts[hypothesis].values()
        )

    def disagreement(self, hypothesis: str) -> bool:
        kinds = set(self.verdicts[hypothesis].values())
        if self.not_estimable:
            kinds.add("not_estimable")
        return len(kinds) > 1

    def table(self) -> pd.DataFrame:
        return pd.DataFrame(self.verdicts).T.sort_index()


def robustness(
    analysis: Callable[..., Any],
    variants: Mapping[str, Mapping[str, Any]],
    **common: Any,
) -> RobustnessReport:
    """Run ``analysis(**common, **variants[name])`` for every variant.

    ``analysis`` is any ``firma_lab.stats.analyse_*`` function. It may return
    one ``HypothesisResult`` or a tuple whose ``HypothesisResult`` members
    are collected (``analyse_h1a_h1b`` returns H1a, H1b and a dict). A
    variant that overrides an argument also given in ``common`` is an error,
    so no variant silently inherits the wrong column.
    """
    if not variants:
        raise ValueError("at least one variant is required")
    verdicts: dict[str, dict[str, str]] = {}
    results: dict[str, dict[str, HypothesisResult]] = {}
    not_estimable: dict[str, str] = {}
    for name, override in variants.items():
        clash = set(override) & set(common)
        if clash:
            raise ValueError(f"variant {name!r} overrides common argument(s) {sorted(clash)}")
        try:
            out = analysis(**common, **override)
        except InsufficientDataError as e:
            not_estimable[name] = str(e)
            continue
        found = [out] if isinstance(out, HypothesisResult) else [
            x for x in out if isinstance(x, HypothesisResult)
        ]
        for r in found:
            verdicts.setdefault(r.hypothesis, {})[name] = r.verdict
            results.setdefault(r.hypothesis, {})[name] = r
    return RobustnessReport(verdicts, results, not_estimable)


def factorial_sobol_indices(
    df: pd.DataFrame, *, factors: Sequence[str], outcome: str
) -> pd.DataFrame:
    """First-order and total Sobol' indices of the **cell-mean** response
    over a complete full-factorial design.

    Each factor is treated as uniform over the levels present, and the
    response is the mean of ``outcome`` over replicates in each cell, so the
    indices describe how the design factors drive the expected outcome
    (seed-to-seed noise is averaged out first, not attributed to any
    factor). For factor ``i``: ``S_i = Var(E[Y | X_i]) / Var(Y)`` and
    ``S_Ti = 1 - Var(E[Y | X_~i]) / Var(Y)``, both exact on a complete grid.

    Raises ``InsufficientDataError`` if any factor combination is missing or
    the cell means do not vary.
    """
    if len(factors) < 1:
        raise ValueError("at least one factor is required")
    missing = [c for c in [*factors, outcome] if c not in df.columns]
    if missing:
        raise InsufficientDataError(f"missing column(s): {missing}")
    if df[outcome].isna().any():
        raise InsufficientDataError(f"missing values in {outcome!r}")
    cells = df.groupby(list(factors), sort=True)[outcome].mean()
    levels = [sorted(df[f].unique()) for f in factors]
    expected = 1
    for lv in levels:
        expected *= len(lv)
    if len(cells) != expected:
        present = set(cells.index if len(factors) > 1 else [(i,) for i in cells.index])
        absent = [c for c in itertools.product(*levels) if c not in present]
        raise InsufficientDataError(
            f"incomplete factorial: {len(cells)} of {expected} cells present; "
            f"first missing: {absent[:3]}"
        )
    y = cells.reset_index()
    total_var = float(y[outcome].var(ddof=0))
    if not total_var > 0:
        raise InsufficientDataError("cell means do not vary; indices undefined")
    rows = []
    for f in factors:
        first = float(y.groupby(f)[outcome].mean().var(ddof=0)) / total_var
        others = [g for g in factors if g != f]
        if others:
            rest = float(y.groupby(others)[outcome].mean().var(ddof=0))
        else:
            rest = 0.0
        rows.append({"factor": f, "first_order": first, "total": 1.0 - rest / total_var})
    return pd.DataFrame(rows).set_index("factor")
