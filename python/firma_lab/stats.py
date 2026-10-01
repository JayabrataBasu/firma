"""firma_lab.stats -- the pre-registered E1 analyses (manual Sec 30.7), plus
the bootstrap, FDR and equivalence machinery they need (Sec 23.1, Sec 29).

**What this module takes as input.** One row per *run* (Sec 29.1: "the unit
of analysis is the run" -- ticks are never observations here), in a pandas
DataFrame whose columns the caller names explicitly: the outcome, the design
factors, and the seed/replicate that groups CRN-matched runs. It does not
compute any FIRMA metric itself (Sec 17 A5; ADR 0046): ``delta_h_rep``,
``survival_time`` and friends arrive as columns, produced upstream by
``firma_lab.metrics`` from the event logs. ``analysis_table`` assembles such
a frame from ``firma_lab.runner.JobRecord``s and a caller-supplied
per-run outcome function, generically over any ``ExperimentSpec`` -- no cell
count, arm name, or factor list is hard-coded anywhere in this module.

**Dependencies (added in this module's build):**

- ``statsmodels`` -- ``MixedLM`` for Sec 30.7's ``(1 | seed)`` mixed model,
  ``PHReg`` for H4's Cox model, ``multipletests`` for Benjamini-Hochberg.
  The one mainstream Python library that has all three with a formula
  interface; ``lifelines`` would add Cox only, ``pymer4`` would add an R
  runtime.
- ``numpy`` (already via pandas) -- the bootstrap's seeded
  ``Generator(PCG64)``; the only RNG in this module, always seeded
  explicitly by the caller (``bootstrap_seed`` has no default).

**Choices the manual leaves open, made here and stated (each is also listed
in ``PROGRESS.md``'s OQ for this build so the owner can fix them in the
analysis plan before filing):**

1. *Bootstrap unit.* Runs sharing a seed are CRN-matched (ADR-0053), so they
   are not independent. Resampling is a **cluster bootstrap over seeds**
   (seeds drawn with replacement; a seed drawn twice becomes two distinct
   groups), matching the model's ``(1 | seed)`` grouping.
2. *CI type.* Percentile interval over the resampled estimates.
3. *Centering for H1a/H1b.* With an ``ς:h`` term in the model, the ``ς``
   coefficient is the ``ς`` slope at ``h = 0`` (and vice versa) -- outside
   Arm A's registered grid (``h`` >= 0.02). Whether to center is therefore
   not cosmetic; the caller must pass ``centers`` explicitly (``{}`` means
   "uncentered, as literally written"). ``grid_centers`` computes the mean
   of each factor's *registered* levels from the spec.
4. *"Falsified" for H1a/H1b.* Sec 30.8 says "coefficient zero or negative".
   Read here as: not supported (CI does not lie entirely above zero). The
   separate ``sign`` detail says whether the CI is entirely below zero.
5. *H1c predictor.* Sec 30.7 says "fit linear and quadratic
   specifications" without naming the regressor; the caller names it.
   Cross-validation folds are grouped by seed and assigned deterministically
   (seeds sorted, fold = rank mod k); CV uses fixed-effects OLS predictions,
   since held-out seeds have no random-effect estimate.
6. *H2 contrast.* Coefficients are standardised (outcome and both threat
   predictors z-scored) so novelty and magnitude are comparable; the
   equivalence test is TOST at the manual's +/-0.1 standardised margin,
   done as "the 90% bootstrap CI lies inside the margin". "At matched h" is
   implemented as adjusting for the caller-named matching covariate(s).
7. *H4.* Cox PH (``PHReg``) stratified by seed (the Cox analogue of
   ``(1 | seed)`` for CRN-matched runs), Efron ties. Verdict on the
   narrowing x novelty interaction, which is what Sec 30.8 falsifies;
   the narrowing log-hazard ratio at the lowest and highest novelty level
   is reported as detail (the "improves ... impairs" crossover in Sec 2.4).
8. *p-values.* Model-based Wald p-values (two-sided), used only for the
   multiplicity step; Sec 30.7 makes them secondary to the bootstrap CI.
9. *Multiplicity family.* Every result whose outcome is not the primary
   outcome forms one Benjamini-Hochberg family.
"""

from __future__ import annotations

import math
import warnings
from dataclasses import dataclass, field
from typing import Any, Callable, Iterable, Mapping, Sequence

import numpy as np
import pandas as pd

from firma_lab.spec import ExperimentSpec

#: Sec 30.7: "bootstrap 95% CIs (10,000 resamples)".
MANUAL_BOOTSTRAP_RESAMPLES = 10_000
#: Sec 30.7: "bootstrap 95% CIs".
MANUAL_CI_LEVEL = 0.95
#: Sec 30.7: "Benjamini-Hochberg FDR at q = 0.05".
MANUAL_FDR_Q = 0.05
#: Sec 30.7: "Equivalence testing (TOST, +/-0.1 standardised)".
MANUAL_EQUIVALENCE_MARGIN = 0.1


class InsufficientDataError(ValueError):
    """The data cannot support the requested analysis (missing columns,
    missing values, too few seeds, a predictor with no variation, or a
    rank-deficient design). Raised instead of returning a number that would
    look like an answer."""


# ---------------------------------------------------------------------------
# Result types
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class CoefficientEstimate:
    """One model term: point estimate, bootstrap CI, Wald p-value."""

    term: str
    estimate: float
    ci_low: float
    ci_high: float
    ci_level: float
    p_value: float | None
    n_resamples: int
    n_failed_resamples: int

    @property
    def ci_excludes_zero(self) -> bool:
        return self.ci_low > 0.0 or self.ci_high < 0.0

    @property
    def sign(self) -> str:
        """``"positive"`` / ``"negative"`` if the CI lies entirely on one
        side of zero, else ``"includes_zero"``."""
        if self.ci_low > 0.0:
            return "positive"
        if self.ci_high < 0.0:
            return "negative"
        return "includes_zero"


@dataclass(frozen=True)
class HypothesisResult:
    """The outcome of one pre-registered test.

    ``verdict`` is one of ``"supported"``, ``"not_supported"``,
    ``"falsified"`` (used only where Sec 30.8 names a positive falsifying
    finding, e.g. H2's equivalence) or ``"inconclusive"``. ``key_term`` names
    the estimate the verdict rests on; its Wald p-value is what the
    multiplicity step uses.
    """

    hypothesis: str
    outcome: str
    verdict: str
    key_term: str
    estimates: Mapping[str, CoefficientEstimate]
    details: Mapping[str, Any] = field(default_factory=dict)

    @property
    def p_value(self) -> float | None:
        return self.estimates[self.key_term].p_value


# ---------------------------------------------------------------------------
# Input validation
# ---------------------------------------------------------------------------


def _require_columns(df: pd.DataFrame, cols: Iterable[str]) -> None:
    missing = [c for c in cols if c not in df.columns]
    if missing:
        raise InsufficientDataError(f"missing column(s): {missing}")


def _require_complete(df: pd.DataFrame, cols: Sequence[str]) -> None:
    """No silent row dropping: Sec 30.7 allows exclusions only for engine-
    invariant failures, reported with counts and reasons -- so a missing
    value here is an upstream problem to fix, not something to skip."""
    na = df[list(cols)].isna().sum()
    bad = {c: int(n) for c, n in na.items() if n > 0}
    if bad:
        raise InsufficientDataError(
            f"missing values in {bad}; exclude runs explicitly upstream "
            "(firma_lab.stats.invariant_exclusions), never by outcome value"
        )


def _require_variation(df: pd.DataFrame, cols: Sequence[str]) -> None:
    flat = [c for c in cols if df[c].nunique(dropna=True) < 2]
    if flat:
        raise InsufficientDataError(f"column(s) with no variation: {flat}")


def _require_clusters(df: pd.DataFrame, seed_col: str, minimum: int = 2) -> None:
    n = df[seed_col].nunique()
    if n < minimum:
        raise InsufficientDataError(
            f"{n} distinct {seed_col!r} value(s); a cluster bootstrap over "
            f"seeds needs at least {minimum}"
        )


def _require_full_rank(df: pd.DataFrame, formula_rhs: str) -> None:
    import patsy

    x = patsy.dmatrix(formula_rhs, df, return_type="dataframe")
    rank = np.linalg.matrix_rank(x.to_numpy())
    if rank < x.shape[1]:
        raise InsufficientDataError(
            f"design matrix for '{formula_rhs}' is rank-deficient "
            f"(rank {rank} < {x.shape[1]} columns)"
        )


# ---------------------------------------------------------------------------
# Cluster bootstrap
# ---------------------------------------------------------------------------


def cluster_bootstrap(
    df: pd.DataFrame,
    *,
    seed_col: str,
    statistic: Callable[[pd.DataFrame], Mapping[str, float]],
    n_resamples: int,
    bootstrap_seed: int,
) -> tuple[pd.DataFrame, int]:
    """Resample seeds with replacement ``n_resamples`` times and evaluate
    ``statistic`` on each resampled frame.

    In each resample, the k-th drawn seed's rows are relabelled with group
    id ``k`` in ``seed_col``, so a seed drawn twice contributes two
    independent groups to a ``(1 | seed)`` model rather than one merged
    group. A resample where ``statistic`` raises is counted as failed and
    left out; the count is returned and carried into every
    ``CoefficientEstimate`` (never hidden).

    Returns ``(estimates, n_failed)``: one row per successful resample, one
    column per key ``statistic`` returns.
    """
    if n_resamples < 1:
        raise ValueError(f"n_resamples must be >= 1, got {n_resamples}")
    rng = np.random.Generator(np.random.PCG64(bootstrap_seed))
    seeds = np.array(sorted(df[seed_col].unique()))
    row_index = {s: np.flatnonzero(df[seed_col].to_numpy() == s) for s in seeds}
    rows: list[Mapping[str, float]] = []
    n_failed = 0
    for _ in range(n_resamples):
        drawn = rng.choice(seeds, size=len(seeds), replace=True)
        idx = np.concatenate([row_index[s] for s in drawn])
        labels = np.concatenate(
            [np.full(len(row_index[s]), k) for k, s in enumerate(drawn)]
        )
        sample = df.iloc[idx].copy()
        sample[seed_col] = labels
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                rows.append(dict(statistic(sample)))
        except Exception:  # noqa: BLE001 -- counted and reported, not hidden
            n_failed += 1
    if not rows:
        raise InsufficientDataError(
            f"all {n_resamples} bootstrap resamples failed to fit"
        )
    return pd.DataFrame(rows), n_failed


def _percentile_ci(values: np.ndarray, level: float) -> tuple[float, float]:
    alpha = 1.0 - level
    lo, hi = np.quantile(values, [alpha / 2.0, 1.0 - alpha / 2.0])
    return float(lo), float(hi)


def _estimates(
    point: Mapping[str, float],
    pvalues: Mapping[str, float | None],
    boot: pd.DataFrame,
    n_failed: int,
    ci_level: float,
) -> dict[str, CoefficientEstimate]:
    out = {}
    for term, est in point.items():
        lo, hi = _percentile_ci(boot[term].to_numpy(), ci_level)
        out[term] = CoefficientEstimate(
            term=term,
            estimate=float(est),
            ci_low=lo,
            ci_high=hi,
            ci_level=ci_level,
            p_value=pvalues.get(term),
            n_resamples=len(boot) + n_failed,
            n_failed_resamples=n_failed,
        )
    return out


# ---------------------------------------------------------------------------
# Model fitting helpers
# ---------------------------------------------------------------------------


def _fit_mixed(df: pd.DataFrame, formula: str, seed_col: str):
    import statsmodels.formula.api as smf

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        model = smf.mixedlm(formula, df, groups=df[seed_col])
        try:
            return model.fit(reml=True, method=["lbfgs", "bfgs", "cg"])
        except (np.linalg.LinAlgError, ValueError) as e:
            raise InsufficientDataError(
                f"mixed model '{formula} + (1 | {seed_col})' could not be fitted: {e} "
                "(e.g. no seed-level variation in the outcome)"
            ) from e


def grid_centers(spec: ExperimentSpec, arm: str, factors: Sequence[str]) -> dict[str, float]:
    """The mean of each named factor's **registered** levels in ``arm`` --
    fixed by the spec before any data exists, so centering on it is not a
    data-dependent choice. Raises if a factor's levels are not numeric."""
    by_name = {f.name: f for f in spec.arm(arm).factors}
    out = {}
    for name in factors:
        if name not in by_name:
            raise KeyError(f"arm {arm!r} has no factor {name!r}")
        levels = by_name[name].levels
        if not all(isinstance(v, (int, float)) and not isinstance(v, bool) for v in levels):
            raise TypeError(f"factor {name!r} has non-numeric levels {levels!r}")
        out[name] = float(sum(levels)) / len(levels)
    return out


def _centered(df: pd.DataFrame, centers: Mapping[str, float]) -> pd.DataFrame:
    out = df.copy()
    for col, c in centers.items():
        _require_columns(out, [col])
        out[col] = out[col] - c
    return out


def _direction_check(outcome_direction: int) -> None:
    if outcome_direction not in (1, -1):
        raise ValueError(
            "outcome_direction must be +1 (higher = broader repertoire/search, "
            "e.g. R1, R2) or -1 (higher = more rigid, e.g. R3)"
        )


# ---------------------------------------------------------------------------
# H1a / H1b (Arm A)
# ---------------------------------------------------------------------------


def analyse_h1a_h1b(
    df: pd.DataFrame,
    *,
    outcome: str,
    outcome_direction: int,
    shortfall_col: str,
    margin_col: str,
    beta_col: str,
    seed_col: str,
    centers: Mapping[str, float],
    bootstrap_seed: int,
    n_resamples: int = MANUAL_BOOTSTRAP_RESAMPLES,
    ci_level: float = MANUAL_CI_LEVEL,
) -> tuple[HypothesisResult, HypothesisResult, dict[str, Any]]:
    """Sec 30.7: ``outcome ~ ς + h + ς:h + β + (1 | seed)``.

    H1a is supported if the ``ς`` coefficient, times ``outcome_direction``,
    is positive with its bootstrap CI excluding zero; H1b likewise for
    ``h``. Returns ``(h1a, h1b, dissociation)`` where ``dissociation``
    records whether both hold (Sec 30.7: "Both must hold for the
    dissociation claim").
    """
    _direction_check(outcome_direction)
    cols = [outcome, shortfall_col, margin_col, beta_col, seed_col]
    _require_columns(df, cols)
    _require_complete(df, cols)
    _require_variation(df, [outcome, shortfall_col, margin_col, beta_col])
    _require_clusters(df, seed_col)
    data = _centered(df[cols], centers)
    rhs = f"{shortfall_col} + {margin_col} + {shortfall_col}:{margin_col} + {beta_col}"
    _require_full_rank(data, rhs)
    formula = f"{outcome} ~ {rhs}"
    terms = [shortfall_col, margin_col, f"{shortfall_col}:{margin_col}", beta_col]

    fit = _fit_mixed(data, formula, seed_col)
    point = {t: float(fit.fe_params[t]) for t in terms}
    pvals = {t: float(fit.pvalues[t]) for t in terms}

    def stat(sample: pd.DataFrame) -> Mapping[str, float]:
        f = _fit_mixed(sample, formula, seed_col)
        return {t: float(f.fe_params[t]) for t in terms}

    boot, failed = cluster_bootstrap(
        data, seed_col=seed_col, statistic=stat,
        n_resamples=n_resamples, bootstrap_seed=bootstrap_seed,
    )
    est = _estimates(point, pvals, boot, failed, ci_level)

    def verdict(term: str) -> tuple[str, str]:
        e = est[term]
        lo, hi = sorted((e.ci_low * outcome_direction, e.ci_high * outcome_direction))
        directed = "positive" if lo > 0 else "negative" if hi < 0 else "includes_zero"
        return ("supported" if directed == "positive" else "not_supported"), directed

    v_a, s_a = verdict(shortfall_col)
    v_b, s_b = verdict(margin_col)
    common = {
        "formula": formula,
        "centers": dict(centers),
        "outcome_direction": outcome_direction,
        "n_runs": len(data),
        "n_seeds": int(data[seed_col].nunique()),
    }
    h1a = HypothesisResult(
        "H1a", outcome, v_a, shortfall_col, est,
        {**common, "directed_sign": s_a, "falsified_per_30_8": v_a != "supported"},
    )
    h1b = HypothesisResult(
        "H1b", outcome, v_b, margin_col, est,
        {**common, "directed_sign": s_b, "falsified_per_30_8": v_b != "supported"},
    )
    dissociation = {
        "holds": v_a == "supported" and v_b == "supported",
        "h1a": v_a,
        "h1b": v_b,
    }
    return h1a, h1b, dissociation


# ---------------------------------------------------------------------------
# H1c (Arm B)
# ---------------------------------------------------------------------------


def _seed_folds(seeds: Iterable[Any], k: int) -> dict[Any, int]:
    ordered = sorted(set(seeds))
    return {s: i % k for i, s in enumerate(ordered)}


def _cv_mse(data: pd.DataFrame, formula: str, seed_col: str, k: int) -> float:
    import statsmodels.formula.api as smf

    folds = data[seed_col].map(_seed_folds(data[seed_col], k))
    errors = []
    for fold in range(k):
        train, test = data[folds != fold], data[folds == fold]
        fit = smf.ols(formula, train).fit()
        pred = fit.predict(test)
        y = test[formula.split("~")[0].strip()]
        errors.append(((y - pred) ** 2).to_numpy())
    return float(np.concatenate(errors).mean())


def analyse_h1c(
    df: pd.DataFrame,
    *,
    outcome: str,
    outcome_direction: int,
    predictor_col: str,
    covariates: Sequence[str],
    seed_col: str,
    cv_folds: int,
    bootstrap_seed: int,
    n_resamples: int = MANUAL_BOOTSTRAP_RESAMPLES,
    ci_level: float = MANUAL_CI_LEVEL,
) -> HypothesisResult:
    """Sec 30.7: fit ``outcome ~ x + covariates`` and ``outcome ~ x + x^2 +
    covariates`` (each with ``(1 | seed)``). Supported iff the quadratic
    term (times ``outcome_direction``) is negative with CI excluding zero
    **and** the quadratic model has the lower seed-grouped cross-validated
    mean squared error. The quadratic coefficient does not depend on
    centering ``x``, so no centering argument is taken."""
    _direction_check(outcome_direction)
    cols = [outcome, predictor_col, *covariates, seed_col]
    _require_columns(df, cols)
    _require_complete(df, cols)
    _require_variation(df, [outcome, predictor_col])
    _require_clusters(df, seed_col, minimum=max(2, cv_folds))
    if cv_folds < 2:
        raise ValueError("cv_folds must be >= 2")
    data = df[cols].copy()
    if data[predictor_col].nunique() < 3:
        raise InsufficientDataError(
            f"{predictor_col!r} has fewer than 3 distinct values; a quadratic "
            "term is not identified"
        )
    cov = "".join(f" + {c}" for c in covariates)
    sq = f"I({predictor_col} ** 2)"
    lin_f = f"{outcome} ~ {predictor_col}{cov}"
    quad_f = f"{outcome} ~ {predictor_col} + {sq}{cov}"
    _require_full_rank(data, quad_f.split("~")[1])

    fit = _fit_mixed(data, quad_f, seed_col)
    terms = [predictor_col, sq]
    point = {t: float(fit.fe_params[t]) for t in terms}
    pvals = {t: float(fit.pvalues[t]) for t in terms}

    def stat(sample: pd.DataFrame) -> Mapping[str, float]:
        f = _fit_mixed(sample, quad_f, seed_col)
        return {t: float(f.fe_params[t]) for t in terms}

    boot, failed = cluster_bootstrap(
        data, seed_col=seed_col, statistic=stat,
        n_resamples=n_resamples, bootstrap_seed=bootstrap_seed,
    )
    est = _estimates(point, pvals, boot, failed, ci_level)
    mse_lin = _cv_mse(data, lin_f, seed_col, cv_folds)
    mse_quad = _cv_mse(data, quad_f, seed_col, cv_folds)
    q = est[sq]
    lo, hi = sorted((q.ci_low * outcome_direction, q.ci_high * outcome_direction))
    negative = hi < 0
    preferred = mse_quad < mse_lin
    return HypothesisResult(
        "H1c", outcome, "supported" if (negative and preferred) else "not_supported", sq, est,
        {
            "linear_formula": lin_f,
            "quadratic_formula": quad_f,
            "cv_folds": cv_folds,
            "cv_mse_linear": mse_lin,
            "cv_mse_quadratic": mse_quad,
            "quadratic_preferred": preferred,
            "quadratic_directed_negative": negative,
            "outcome_direction": outcome_direction,
            "n_runs": len(data),
            "n_seeds": int(data[seed_col].nunique()),
        },
    )


# ---------------------------------------------------------------------------
# H2 (Arm B)
# ---------------------------------------------------------------------------


def _zscore(s: pd.Series) -> pd.Series:
    sd = s.std(ddof=1)
    if not sd > 0:
        raise InsufficientDataError(f"{s.name!r} has zero variance")
    return (s - s.mean()) / sd


def analyse_h2(
    df: pd.DataFrame,
    *,
    outcome: str,
    outcome_direction: int,
    novelty_col: str,
    magnitude_col: str,
    matching_cols: Sequence[str],
    seed_col: str,
    bootstrap_seed: int,
    n_resamples: int = MANUAL_BOOTSTRAP_RESAMPLES,
    ci_level: float = MANUAL_CI_LEVEL,
    equivalence_margin: float = MANUAL_EQUIVALENCE_MARGIN,
) -> HypothesisResult:
    """Sec 30.7: contrast novelty and magnitude coefficients at matched h.

    Model on z-scored outcome/novelty/magnitude:
    ``z(y) ~ z(novelty) + z(magnitude) + matching + (1 | seed)``. Narrowing
    effect of a threat dimension = ``-outcome_direction x coefficient``.
    Contrast ``c = narrowing(novelty) - narrowing(magnitude)``.

    - ``supported``: ``c``'s CI lies entirely above zero (novelty narrows more);
    - ``falsified``: TOST equivalence -- the ``1 - 2*alpha`` CI of ``c`` lies
      inside ``+/- equivalence_margin`` (Sec 30.8: "statistically equivalent");
    - ``inconclusive`` otherwise.

    The TOST is run at ``alpha = (1 - ci_level) / 2`` per side, i.e. a 90%
    interval for the default 95% level.
    """
    _direction_check(outcome_direction)
    cols = [outcome, novelty_col, magnitude_col, *matching_cols, seed_col]
    _require_columns(df, cols)
    _require_complete(df, cols)
    _require_variation(df, [outcome, novelty_col, magnitude_col])
    _require_clusters(df, seed_col)
    data = df[cols].copy()
    for c in (outcome, novelty_col, magnitude_col):
        data[c] = _zscore(data[c])
    rhs = f"{novelty_col} + {magnitude_col}" + "".join(f" + {c}" for c in matching_cols)
    _require_full_rank(data, rhs)
    formula = f"{outcome} ~ {rhs}"
    contrast = "contrast_novelty_minus_magnitude"

    def point_of(fit) -> dict[str, float]:
        bn, bm = float(fit.fe_params[novelty_col]), float(fit.fe_params[magnitude_col])
        return {
            novelty_col: bn,
            magnitude_col: bm,
            contrast: (-outcome_direction * bn) - (-outcome_direction * bm),
        }

    fit = _fit_mixed(data, formula, seed_col)
    point = point_of(fit)
    # Wald test of b_nov - b_mag = 0 for the contrast's p-value.
    names = list(fit.fe_params.index)
    r = np.zeros((1, len(names)))
    r[0, names.index(novelty_col)], r[0, names.index(magnitude_col)] = 1.0, -1.0
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        p_contrast = float(fit.t_test(r).pvalue)
    pvals = {
        novelty_col: float(fit.pvalues[novelty_col]),
        magnitude_col: float(fit.pvalues[magnitude_col]),
        contrast: p_contrast,
    }

    boot, failed = cluster_bootstrap(
        data, seed_col=seed_col, statistic=lambda s: point_of(_fit_mixed(s, formula, seed_col)),
        n_resamples=n_resamples, bootstrap_seed=bootstrap_seed,
    )
    est = _estimates(point, pvals, boot, failed, ci_level)
    tost_level = 1.0 - 2.0 * ((1.0 - ci_level) / 2.0)
    t_lo, t_hi = _percentile_ci(boot[contrast].to_numpy(), tost_level)
    equivalent = -equivalence_margin < t_lo and t_hi < equivalence_margin
    c = est[contrast]
    if c.ci_low > 0:
        verdict = "supported"
    elif equivalent:
        verdict = "falsified"
    else:
        verdict = "inconclusive"
    return HypothesisResult(
        "H2", outcome, verdict, contrast, est,
        {
            "formula": formula + "  (outcome, novelty, magnitude z-scored)",
            "matching_cols": list(matching_cols),
            "tost_interval": (t_lo, t_hi),
            "tost_level": tost_level,
            "equivalence_margin": equivalence_margin,
            "equivalent": equivalent,
            "outcome_direction": outcome_direction,
            "n_runs": len(data),
            "n_seeds": int(data[seed_col].nunique()),
        },
    )


# ---------------------------------------------------------------------------
# H3 -- excluded from this filing
# ---------------------------------------------------------------------------


def analyse_h3(*_args: Any, **_kwargs: Any) -> HypothesisResult:
    """Not implemented, deliberately. ADR-0051 excludes H3 (and Arm B's
    shaping-lag factor) from the first E1 filing's registered design; H3
    stays open (ADR-0050) and gets its own filing. Its Sec 30.7 analysis
    (the three-way ``β x Δ_ℓ x time-to-boundary`` interaction) is built with
    that filing, not here."""
    raise NotImplementedError(
        "H3 is excluded from this filing's registered design (ADR-0051); "
        "its analysis is built with its own, separate filing (ADR-0050)."
    )


# ---------------------------------------------------------------------------
# H4 (Arm B) -- Cox proportional hazards
# ---------------------------------------------------------------------------


def _fit_cox(data: pd.DataFrame, formula: str, event_col: str, seed_col: str):
    import statsmodels.formula.api as smf

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        model = smf.phreg(
            formula, data, status=data[event_col].to_numpy(),
            strata=data[seed_col].to_numpy(), ties="efron",
        )
        return model.fit()


def analyse_h4(
    df: pd.DataFrame,
    *,
    duration_col: str,
    event_col: str,
    narrowing_col: str,
    novelty_col: str,
    covariates: Sequence[str],
    seed_col: str,
    bootstrap_seed: int,
    n_resamples: int = MANUAL_BOOTSTRAP_RESAMPLES,
    ci_level: float = MANUAL_CI_LEVEL,
) -> HypothesisResult:
    """Sec 30.7: Cox proportional hazards on right-censored survival time
    (Sec 29.3), ``duration ~ narrowing + novelty + narrowing:novelty +
    covariates``, stratified by seed. ``event_col`` is 1 for a lethal
    violation, 0 for censored at the horizon.

    H4 predicts narrowing *helps* at low novelty and *hurts* at high novelty:
    in hazard terms, a positive ``narrowing:novelty`` coefficient. Supported
    iff that coefficient's CI lies entirely above zero; Sec 30.8 falsifies
    H4 on "no interaction", reported here as ``not_supported``.
    """
    cols = [duration_col, event_col, narrowing_col, novelty_col, *covariates, seed_col]
    _require_columns(df, cols)
    _require_complete(df, cols)
    _require_variation(df, [duration_col, narrowing_col, novelty_col])
    _require_clusters(df, seed_col)
    data = df[cols].copy()
    if not set(data[event_col].unique()) <= {0, 1}:
        raise InsufficientDataError(f"{event_col!r} must be 0/1")
    if int(data[event_col].sum()) == 0:
        raise InsufficientDataError("no events (every run censored); a Cox model is not identified")
    inter = f"{narrowing_col}:{novelty_col}"
    rhs = f"{narrowing_col} + {novelty_col} + {inter}" + "".join(f" + {c}" for c in covariates)
    formula = f"{duration_col} ~ {rhs}"
    lo_nov, hi_nov = float(data[novelty_col].min()), float(data[novelty_col].max())
    terms = [narrowing_col, novelty_col, inter]

    def point_of(fit) -> dict[str, float]:
        p = dict(zip(fit.model.exog_names, map(float, fit.params)))
        out = {t: p[t] for t in terms}
        out["narrowing_loghr_at_min_novelty"] = p[narrowing_col] + p[inter] * lo_nov
        out["narrowing_loghr_at_max_novelty"] = p[narrowing_col] + p[inter] * hi_nov
        return out

    fit = _fit_cox(data, formula, event_col, seed_col)
    point = point_of(fit)
    pv = dict(zip(fit.model.exog_names, map(float, fit.pvalues)))
    pvals = {t: pv[t] for t in terms}
    boot, failed = cluster_bootstrap(
        data, seed_col=seed_col,
        statistic=lambda s: point_of(_fit_cox(s, formula, event_col, seed_col)),
        n_resamples=n_resamples, bootstrap_seed=bootstrap_seed,
    )
    est = _estimates(point, pvals, boot, failed, ci_level)
    i = est[inter]
    return HypothesisResult(
        "H4", duration_col, "supported" if i.ci_low > 0 else "not_supported", inter, est,
        {
            "formula": formula,
            "strata": seed_col,
            "ties": "efron",
            "novelty_range": (lo_nov, hi_nov),
            "crossover": (
                est["narrowing_loghr_at_min_novelty"].sign == "negative"
                and est["narrowing_loghr_at_max_novelty"].sign == "positive"
            ),
            "n_runs": len(data),
            "n_events": int(data[event_col].sum()),
            "n_seeds": int(data[seed_col].nunique()),
        },
    )


# ---------------------------------------------------------------------------
# Multiplicity
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class MultiplicityRow:
    hypothesis: str
    outcome: str
    key_term: str
    role: str  # "primary" | "secondary"
    p_value: float | None
    p_adjusted: float | None
    significant: bool | None


def apply_multiplicity(
    results: Sequence[HypothesisResult],
    *,
    primary_outcome: str,
    q: float = MANUAL_FDR_Q,
) -> list[MultiplicityRow]:
    """Sec 30.7: "primary outcome uncorrected; all secondary outcomes
    Benjamini-Hochberg FDR at q = 0.05".

    Results on ``primary_outcome`` keep their raw p-value
    (``significant = p < q``). Every other result forms **one** BH family.
    A secondary result with no p-value is an error, not a skip.
    """
    from statsmodels.stats.multitest import multipletests

    rows: list[MultiplicityRow | None] = [None] * len(results)
    secondary = [i for i, r in enumerate(results) if r.outcome != primary_outcome]
    for i, r in enumerate(results):
        if r.outcome == primary_outcome:
            p = r.p_value
            rows[i] = MultiplicityRow(
                r.hypothesis, r.outcome, r.key_term, "primary", p, None,
                None if p is None else p < q,
            )
    if secondary:
        ps = [results[i].p_value for i in secondary]
        if any(p is None or math.isnan(p) for p in ps):
            raise InsufficientDataError("a secondary result has no p-value for the FDR step")
        reject, adj, _, _ = multipletests(ps, alpha=q, method="fdr_bh")
        for i, rej, pa in zip(secondary, reject, adj):
            r = results[i]
            rows[i] = MultiplicityRow(
                r.hypothesis, r.outcome, r.key_term, "secondary",
                r.p_value, float(pa), bool(rej),
            )
    return [r for r in rows if r is not None]


# ---------------------------------------------------------------------------
# From runner records to an analysis table (generic over any spec)
# ---------------------------------------------------------------------------


def invariant_exclusions(records: Iterable[Any]) -> tuple[list[Any], dict[str, Any]]:
    """Sec 30.7: "runs failing an engine invariant are excluded with counts
    and reasons reported. No exclusion on the basis of outcome values."

    Splits ``firma_lab.runner.JobRecord``s by ``status``: ``"completed"``
    kept, everything else excluded, with each excluded job's id and
    ``failure_reason``. Nothing here looks at an outcome.
    """
    kept, excluded = [], []
    for r in records:
        (kept if r.status == "completed" else excluded).append(r)
    report = {
        "n_kept": len(kept),
        "n_excluded": len(excluded),
        "excluded": [{"job_id": r.job_id, "reason": r.failure_reason} for r in excluded],
    }
    return kept, report


def _level_label(level: Any) -> Any:
    if isinstance(level, Mapping) and "id" in level:
        return level["id"]
    return level


def analysis_table(
    spec: ExperimentSpec,
    records: Iterable[Any],
    outcome_fn: Callable[[Any], Mapping[str, float]],
) -> pd.DataFrame:
    """One row per completed run: ``arm``, ``cell_index``, ``replicate``
    (the seed: ADR-0053 sets all four streams to it), every factor level of
    the run's arm as its own column (a ``rule_swap`` level becomes its rule
    ``id``), then whatever ``outcome_fn(record)`` returns.

    Records must come from ``spec`` (checked by content hash) and must all
    be ``"completed"`` -- pass them through ``invariant_exclusions`` first.
    """
    h = spec.content_hash()
    rows = []
    for r in records:
        if r.spec_content_hash != h:
            raise ValueError(f"job {r.job_id} belongs to spec {r.spec_content_hash[:12]}, not {h[:12]}")
        if r.status != "completed":
            raise ValueError(f"job {r.job_id} is {r.status!r}; exclude it via invariant_exclusions first")
        row: dict[str, Any] = {"arm": r.arm, "cell_index": r.cell_index, "replicate": r.replicate}
        for name, level in r.factor_levels.items():
            row[name] = _level_label(level)
        row.update(outcome_fn(r))
        rows.append(row)
    return pd.DataFrame(rows)


def missing_jobs(spec: ExperimentSpec, table: pd.DataFrame) -> list[tuple[str, int, int]]:
    """Every ``(arm, cell_index, replicate)`` the spec defines that has no row
    in ``table`` -- computed from ``spec.jobs()``, so it works for any design."""
    have = set(zip(table["arm"], table["cell_index"], table["replicate"])) if len(table) else set()
    return [
        (j.arm, j.cell_index, j.replicate)
        for j in spec.jobs()
        if (j.arm, j.cell_index, j.replicate) not in have
    ]
