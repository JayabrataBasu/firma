"""Tests for firma_lab.stats on synthetic, generated data only -- never on
output of the registered design. Synthetic "seeds" are group labels >= 1001,
outside the reserved 1-200 range (manual Sec 30.5/30.6)."""

from __future__ import annotations

import numpy as np
import pandas as pd
import pytest

from firma_lab import stats
from firma_lab.runner import JobRecord, build_job_config, job_id
from firma_lab.spec import build_synthetic_spec

SEEDS = range(1001, 1021)
H_LEVELS = (0.02, 0.05, 0.10, 0.20, 0.40)
S_LEVELS = (0.0, 0.25, 0.50, 0.75, 1.0)
B_LEVELS = (0, 0.5, 1, 2, 4)
CENTERS = {"varsigma": 0.5, "h": sum(H_LEVELS) / 5}
FAST = 60  # resamples: enough to exercise the machinery, not the manual's 10,000


def arm_a_data(b_s: float, b_h: float, *, gen_seed: int = 7, noise: float = 0.2) -> pd.DataFrame:
    rng = np.random.default_rng(gen_seed)
    rows = []
    for seed in SEEDS:
        u = rng.normal(0, 0.3)
        for h in H_LEVELS:
            for s in S_LEVELS:
                for b in B_LEVELS:
                    y = b_s * (s - 0.5) + b_h * (h - 0.154) - 0.1 * b + u + rng.normal(0, noise)
                    rows.append(dict(seed=seed, h=h, varsigma=s, beta=b, y=y))
    return pd.DataFrame(rows)


def h1ab(df, **kw):
    args = dict(
        outcome="y", outcome_direction=1, shortfall_col="varsigma", margin_col="h",
        beta_col="beta", seed_col="seed", centers=CENTERS, bootstrap_seed=11, n_resamples=FAST,
    )
    args.update(kw)
    return stats.analyse_h1a_h1b(df, **args)


# -- H1a / H1b ---------------------------------------------------------------


def test_h1ab_both_supported_when_both_effects_positive():
    a, b, d = h1ab(arm_a_data(0.8, 1.5))
    assert (a.verdict, b.verdict) == ("supported", "supported")
    assert a.estimates["varsigma"].ci_excludes_zero and a.estimates["varsigma"].ci_low > 0
    assert d["holds"] is True
    assert a.estimates["varsigma"].n_resamples == FAST


def test_h1a_not_supported_when_ci_includes_zero():
    # gen_seed=8, not the default 7: with a true zero shortfall effect, the
    # gen_seed=7 draw happens to be a genuine type-I error (Wald p = 0.028,
    # CI [0.004, 0.048]) -- expected ~5% of the time, not a defect. Draws
    # 8, 9, 10 give p = 0.95, 0.60, 0.84. Checked when this test was written.
    a, b, d = h1ab(arm_a_data(0.0, 1.5, gen_seed=8))
    assert a.estimates["varsigma"].sign == "includes_zero"
    assert a.verdict == "not_supported" and a.details["falsified_per_30_8"] is True
    assert b.verdict == "supported"
    assert d["holds"] is False


def test_h1a_negative_effect_is_not_supported_and_reported_negative():
    a, _, _ = h1ab(arm_a_data(-0.8, 1.5))
    assert a.verdict == "not_supported" and a.details["directed_sign"] == "negative"


def test_outcome_direction_flips_the_prediction_for_a_rigidity_measure():
    df = arm_a_data(0.8, 1.5)
    df["y"] = -df["y"]  # a rigidity measure: higher = more rigid
    a, b, _ = h1ab(df, outcome_direction=-1)
    assert (a.verdict, b.verdict) == ("supported", "supported")
    a_wrong, _, _ = h1ab(df, outcome_direction=1)
    assert a_wrong.verdict == "not_supported"


def test_centering_changes_main_effects_when_an_interaction_exists():
    rng = np.random.default_rng(3)
    df = arm_a_data(0.0, 0.0)
    df["y"] = 2.0 * df["varsigma"] * df["h"] + rng.normal(0, 0.05, len(df))
    uncentered, _, _ = h1ab(df, centers={})
    centered, _, _ = h1ab(df, centers=CENTERS)
    # Uncentered: the varsigma slope at h = 0 (outside the grid) is ~0.
    # Centered: the slope at the grid-mean h is ~2 * 0.154.
    assert abs(uncentered.estimates["varsigma"].estimate) < 0.05
    assert centered.estimates["varsigma"].estimate == pytest.approx(0.308, abs=0.05)


def test_bootstrap_is_deterministic_in_its_seed():
    df = arm_a_data(0.8, 1.5)
    a1, _, _ = h1ab(df, bootstrap_seed=5)
    a2, _, _ = h1ab(df, bootstrap_seed=5)
    a3, _, _ = h1ab(df, bootstrap_seed=6)
    e1, e2, e3 = (x.estimates["varsigma"] for x in (a1, a2, a3))
    assert (e1.ci_low, e1.ci_high) == (e2.ci_low, e2.ci_high)
    assert (e1.ci_low, e1.ci_high) != (e3.ci_low, e3.ci_high)


def test_grid_centers_come_from_registered_levels():
    spec = build_synthetic_spec(num_replicates=2)
    assert stats.grid_centers(spec, "T", ["ticks", "l_w"]) == {"ticks": 6.5, "l_w": 6.0}
    with pytest.raises(KeyError):
        stats.grid_centers(spec, "T", ["nope"])


# -- insufficient data -----------------------------------------------------


def test_missing_column_is_insufficient_data():
    with pytest.raises(stats.InsufficientDataError, match="missing column"):
        h1ab(arm_a_data(0.8, 1.5).drop(columns=["beta"]))


def test_missing_values_are_refused_not_dropped():
    df = arm_a_data(0.8, 1.5)
    df.loc[3, "y"] = np.nan
    with pytest.raises(stats.InsufficientDataError, match="missing values"):
        h1ab(df)


def test_single_seed_is_insufficient_for_a_cluster_bootstrap():
    df = arm_a_data(0.8, 1.5)
    with pytest.raises(stats.InsufficientDataError, match="distinct 'seed'"):
        h1ab(df[df.seed == 1001])


def test_predictor_without_variation_is_insufficient():
    df = arm_a_data(0.8, 1.5)
    with pytest.raises(stats.InsufficientDataError, match="no variation"):
        h1ab(df[df.h == 0.02])


def test_constant_outcome_is_insufficient_not_a_null_result():
    df = arm_a_data(0.8, 1.5)
    df["y"] = 0.0
    with pytest.raises(stats.InsufficientDataError, match="no variation"):
        h1ab(df)


def test_rank_deficient_design_is_insufficient():
    df = arm_a_data(0.8, 1.5)
    df["beta"] = df["varsigma"] * 2.0  # beta collinear with varsigma
    with pytest.raises(stats.InsufficientDataError, match="rank-deficient"):
        h1ab(df)


def test_cluster_bootstrap_relabels_duplicated_seeds_as_distinct_groups():
    df = pd.DataFrame({"seed": [1001, 1001, 1002, 1002, 1003, 1003], "y": range(6)})
    boot, failed = stats.cluster_bootstrap(
        df, seed_col="seed", n_resamples=200, bootstrap_seed=1,
        statistic=lambda s: {"groups": s["seed"].nunique(), "rows": len(s)},
    )
    assert failed == 0
    assert (boot["groups"] == 3).all() and (boot["rows"] == 6).all()


def test_cluster_bootstrap_counts_failures_and_raises_if_all_fail():
    df = pd.DataFrame({"seed": [1001, 1002], "y": [0.0, 1.0]})

    def sometimes(s):
        if s["y"].sum() == 0:
            raise RuntimeError("degenerate resample")
        return {"m": s["y"].mean()}

    boot, failed = stats.cluster_bootstrap(
        df, seed_col="seed", statistic=sometimes, n_resamples=100, bootstrap_seed=2
    )
    assert failed > 0 and len(boot) + failed == 100
    with pytest.raises(stats.InsufficientDataError, match="all 10"):
        stats.cluster_bootstrap(
            df, seed_col="seed", n_resamples=10, bootstrap_seed=2,
            statistic=lambda s: (_ for _ in ()).throw(RuntimeError()),
        )


# -- H1c ---------------------------------------------------------------------


def arm_b_curve(quad: float, *, gen_seed: int = 9) -> pd.DataFrame:
    rng = np.random.default_rng(gen_seed)
    rows = []
    for seed in SEEDS:
        u = rng.normal(0, 0.2)
        for x in np.linspace(-1, 1, 15):
            for b in B_LEVELS:
                y = 0.3 * x + quad * x**2 + 0.05 * b + u + rng.normal(0, 0.1)
                rows.append(dict(seed=seed, x=x, beta=b, y=y))
    return pd.DataFrame(rows)


def h1c(df, **kw):
    args = dict(
        outcome="y", outcome_direction=1, predictor_col="x", covariates=["beta"],
        seed_col="seed", cv_folds=5, bootstrap_seed=3, n_resamples=FAST,
    )
    args.update(kw)
    return stats.analyse_h1c(df, **args)


def test_h1c_supported_for_an_inverted_u():
    r = h1c(arm_b_curve(-1.0))
    assert r.verdict == "supported"
    assert r.details["quadratic_preferred"] and r.details["cv_mse_quadratic"] < r.details["cv_mse_linear"]


def test_h1c_not_supported_for_a_monotone_relationship():
    r = h1c(arm_b_curve(0.0))
    assert r.verdict == "not_supported"


def test_h1c_u_shape_is_not_an_inverted_u():
    assert h1c(arm_b_curve(+1.0)).verdict == "not_supported"


def test_h1c_needs_three_distinct_predictor_values():
    df = arm_b_curve(-1.0)
    df = df[df.x.isin(sorted(df.x.unique())[:2])]
    with pytest.raises(stats.InsufficientDataError, match="fewer than 3"):
        h1c(df)


# -- H2 ----------------------------------------------------------------------


def arm_b_threat(
    b_nov: float, b_mag: float, *, seeds=SEEDS, reps: int = 1, gen_seed: int = 4, noise: float = 0.1
) -> pd.DataFrame:
    rng = np.random.default_rng(gen_seed)
    rows = []
    for seed in seeds:
        u = rng.normal(0, 0.2)
        for nov in (0.0, 0.4, 0.8):
            for mag in (1.0, 2.0, 3.0, 4.0):
                for b in B_LEVELS:
                    for _ in range(reps):
                        h = rng.uniform(0, 0.4)
                        # entropy falls with each threat dimension (narrowing)
                        y = -b_nov * nov - b_mag * (mag / 4) + 0.5 * h + u + rng.normal(0, noise)
                        rows.append(dict(seed=seed, novelty=nov, magnitude=mag, h=h, beta=b, y=y))
    return pd.DataFrame(rows)


def h2(df, **kw):
    args = dict(
        outcome="y", outcome_direction=1, novelty_col="novelty", magnitude_col="magnitude",
        matching_cols=["h"], seed_col="seed", bootstrap_seed=8, n_resamples=FAST,
    )
    args.update(kw)
    return stats.analyse_h2(df, **args)


def test_h2_supported_when_novelty_narrows_more():
    assert h2(arm_b_threat(1.5, 0.2)).verdict == "supported"


def test_h2_falsified_when_effects_are_equivalent():
    # Equal standardised effects: novelty levels 0..0.8 and magnitude/4 in
    # 0.25..1 have equal SD when b_nov * sd(nov) == b_mag * sd(mag/4).
    sd_nov, sd_mag = np.std([0, 0.4, 0.8]), np.std([0.25, 0.5, 0.75, 1.0])
    r = h2(arm_b_threat(1.0, sd_nov / sd_mag, seeds=range(1001, 1061)))
    assert r.details["equivalent"] is True
    assert r.verdict == "falsified"


def test_h2_inconclusive_when_neither_supported_nor_equivalent():
    # Few seeds and heavy noise: the contrast CI is wide -- it straddles
    # zero *and* reaches past the +/-0.1 equivalence margin.
    r = h2(arm_b_threat(0.3, 0.2, seeds=range(1001, 1004), noise=1.0))
    lo, hi = r.estimates["contrast_novelty_minus_magnitude"].ci_low, r.estimates["contrast_novelty_minus_magnitude"].ci_high
    assert lo < 0 < hi and not r.details["equivalent"]
    assert r.verdict == "inconclusive"


# -- H3 ----------------------------------------------------------------------


def test_h3_is_excluded_by_adr_0051():
    with pytest.raises(NotImplementedError, match="ADR-0051"):
        stats.analyse_h3()


# -- H4 ----------------------------------------------------------------------


def survival_data(b_int: float, *, gen_seed: int = 12, horizon: int = 400) -> pd.DataFrame:
    rng = np.random.default_rng(gen_seed)
    rows = []
    for seed in range(1001, 1041):
        u = rng.normal(0, 0.2)
        for b in B_LEVELS:
            for nov in (0.0, 0.4, 0.8):
                rate = 0.01 * np.exp(-0.3 * b + 0.2 * nov + b_int * b * nov + u)
                t = int(np.ceil(rng.exponential(1.0 / rate)))
                rows.append(dict(seed=seed, beta=b, novelty=nov,
                                 survival_time=min(t, horizon), died=int(t <= horizon)))
    return pd.DataFrame(rows)


def h4(df, **kw):
    args = dict(
        duration_col="survival_time", event_col="died", narrowing_col="beta",
        novelty_col="novelty", covariates=[], seed_col="seed", bootstrap_seed=21, n_resamples=FAST,
    )
    args.update(kw)
    return stats.analyse_h4(df, **args)


def test_h4_supported_with_a_crossover_interaction():
    r = h4(survival_data(0.8))
    assert r.verdict == "supported"
    assert r.details["crossover"] is True


def test_h4_not_supported_without_interaction():
    assert h4(survival_data(0.0)).verdict == "not_supported"


def test_h4_all_censored_is_insufficient():
    df = survival_data(0.8)
    df["died"] = 0
    with pytest.raises(stats.InsufficientDataError, match="no events"):
        h4(df)


# -- multiplicity ------------------------------------------------------------


def fake(hyp: str, outcome: str, p: float) -> stats.HypothesisResult:
    e = stats.CoefficientEstimate("t", 1.0, 0.1, 2.0, 0.95, p, 10, 0)
    return stats.HypothesisResult(hyp, outcome, "supported", "t", {"t": e})


def test_primary_uncorrected_secondary_bh_and_fdr_changes_a_call():
    results = [
        fake("H1a", "delta_h_rep", 0.04),
        fake("H1a-R2", "search_narrowing", 0.04),
        fake("H1a-R3", "shaping_abandonment", 0.50),
        fake("H4", "survival_time", 0.60),
    ]
    rows = {r.hypothesis: r for r in stats.apply_multiplicity(results, primary_outcome="delta_h_rep")}
    assert rows["H1a"].role == "primary" and rows["H1a"].p_adjusted is None and rows["H1a"].significant
    # Same raw p = 0.04, but BH over a family of 3 gives 0.04 * 3 / 1 = 0.12.
    assert rows["H1a-R2"].p_adjusted == pytest.approx(0.12)
    assert rows["H1a-R2"].significant is False
    assert rows["H4"].role == "secondary"


def test_secondary_without_p_value_is_an_error():
    e = stats.CoefficientEstimate("t", 1.0, 0.1, 2.0, 0.95, None, 10, 0)
    r = stats.HypothesisResult("X", "other", "supported", "t", {"t": e})
    with pytest.raises(stats.InsufficientDataError, match="no p-value"):
        stats.apply_multiplicity([r], primary_outcome="delta_h_rep")


# -- analysis table (generic over any spec) ---------------------------------


def records_for(spec, *, fail=()):
    out = []
    for job in spec.jobs():
        jid = job_id(spec, job)
        out.append(JobRecord(
            job_id=jid, spec_id=spec.id, spec_content_hash=spec.content_hash(),
            arm=job.arm, cell_index=job.cell_index, factor_levels=dict(job.factor_levels),
            replicate=job.replicate, seeds=job.seeds.to_dict(),
            resolved_config=build_job_config(spec, job),
            status="failed" if jid in fail else "completed",
            failure_reason="conservation_ok=false" if jid in fail else None,
        ))
    return out


def test_analysis_table_exclusions_and_completeness_are_generic():
    spec = build_synthetic_spec(num_replicates=3)
    recs = records_for(spec)
    failed_id = recs[5].job_id
    recs = records_for(spec, fail={failed_id})
    kept, report = stats.invariant_exclusions(recs)
    assert report["n_excluded"] == 1 and report["excluded"][0]["job_id"] == failed_id
    assert report["n_kept"] == spec.total_jobs() - 1
    table = stats.analysis_table(spec, kept, lambda r: {"y": float(r.replicate)})
    assert len(table) == spec.total_jobs() - 1
    assert {"arm", "cell_index", "replicate", "ticks", "l_w", "y"} <= set(table.columns)
    assert stats.missing_jobs(spec, table) == [(recs[5].arm, recs[5].cell_index, recs[5].replicate)]


def test_analysis_table_refuses_unexcluded_failures_and_foreign_specs():
    spec = build_synthetic_spec(num_replicates=2)
    recs = records_for(spec)
    recs[0].status = "failed"
    with pytest.raises(ValueError, match="invariant_exclusions"):
        stats.analysis_table(spec, recs, lambda r: {})
    other = build_synthetic_spec(num_replicates=3)
    with pytest.raises(ValueError, match="belongs to spec"):
        stats.analysis_table(other, records_for(spec), lambda r: {})
