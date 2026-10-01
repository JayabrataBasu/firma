"""Tests for firma_lab.sensitivity on synthetic data (seeds >= 1001)."""

from __future__ import annotations

import itertools

import numpy as np
import pandas as pd
import pytest

from firma_lab import sensitivity, stats

FAST = 40
CENTERS = {"varsigma": 0.5, "h": 0.154}


def three_measures(r3_responds: bool) -> pd.DataFrame:
    rng = np.random.default_rng(31)
    rows = []
    for seed in range(1001, 1016):
        u = rng.normal(0, 0.2)
        for h in (0.02, 0.05, 0.10, 0.20, 0.40):
            for s in (0.0, 0.5, 1.0):
                for b in (0, 1, 4):
                    breadth = 0.8 * s + 1.5 * h - 0.1 * b + u
                    rows.append(dict(
                        seed=seed, h=h, varsigma=s, beta=b,
                        r1=breadth + rng.normal(0, 0.1),
                        r2=0.5 * breadth + rng.normal(0, 0.1),
                        # R3 (abandonment) rises as the repertoire narrows;
                        # in the non-responding case it has seed-level
                        # variation only
                        r3=(-breadth if r3_responds else u) + rng.normal(0, 0.1),
                    ))
    return pd.DataFrame(rows)


COMMON = dict(
    shortfall_col="varsigma", margin_col="h", beta_col="beta", seed_col="seed",
    centers=CENTERS, bootstrap_seed=4, n_resamples=FAST,
)


def test_rigidity_directions_follow_section_14_2():
    v = sensitivity.rigidity_variants(r1="a", r2="b", r3="c")
    assert {k: x["outcome_direction"] for k, x in v.items()} == {"R1": 1, "R2": 1, "R3": -1}


def test_robust_when_all_three_measures_agree():
    rep = sensitivity.robustness(
        lambda **kw: stats.analyse_h1a_h1b(three_measures(True), **kw),
        sensitivity.rigidity_variants(r1="r1", r2="r2", r3="r3"),
        **COMMON,
    )
    assert rep.holds_under_all("H1a") and rep.holds_under_all("H1b")
    assert not rep.disagreement("H1a")
    assert set(rep.table().columns) == {"R1", "R2", "R3"}


def test_disagreement_is_reported_not_resolved():
    rep = sensitivity.robustness(
        lambda **kw: stats.analyse_h1a_h1b(three_measures(False), **kw),
        sensitivity.rigidity_variants(r1="r1", r2="r2", r3="r3"),
        **COMMON,
    )
    assert rep.verdicts["H1a"]["R1"] == "supported"
    assert rep.verdicts["H1a"]["R3"] == "not_supported"
    assert rep.disagreement("H1a") and not rep.holds_under_all("H1a")


def test_a_variant_that_cannot_be_estimated_is_reported_not_dropped():
    df = three_measures(True)
    df["r3"] = 0.0  # constant outcome: nothing to estimate
    rep = sensitivity.robustness(
        lambda **kw: stats.analyse_h1a_h1b(df, **kw),
        sensitivity.rigidity_variants(r1="r1", r2="r2", r3="r3"),
        **COMMON,
    )
    assert "R3" in rep.not_estimable
    assert rep.verdicts["H1a"] == {"R1": "supported", "R2": "supported"}
    assert not rep.holds_under_all("H1a") and rep.disagreement("H1a")


def test_variant_may_not_silently_override_a_common_argument():
    with pytest.raises(ValueError, match="overrides common"):
        sensitivity.robustness(lambda **kw: None, {"R1": {"outcome": "x"}}, outcome="y")


def test_novelty_variants_swap_the_novelty_column():
    assert sensitivity.novelty_variants(n1="n1", n2="n2") == {
        "N1": {"novelty_col": "n1"}, "N2": {"novelty_col": "n2"},
    }


def grid(fn) -> pd.DataFrame:
    rows = []
    for a, b, seed in itertools.product((0, 1, 2), (0, 1), range(1001, 1004)):
        rows.append(dict(a=a, b=b, seed=seed, y=fn(a, b)))
    return pd.DataFrame(rows)


def test_sobol_additive_single_factor():
    s = sensitivity.factorial_sobol_indices(grid(lambda a, b: 2.0 * a), factors=["a", "b"], outcome="y")
    assert s.loc["a", "first_order"] == pytest.approx(1.0)
    assert s.loc["b", "first_order"] == pytest.approx(0.0)
    assert s.loc["b", "total"] == pytest.approx(0.0)


def test_sobol_pure_interaction_has_zero_first_order_full_total():
    s = sensitivity.factorial_sobol_indices(
        grid(lambda a, b: (a - 1) * (b - 0.5)), factors=["a", "b"], outcome="y"
    )
    assert s["first_order"].to_list() == pytest.approx([0.0, 0.0])
    assert s["total"].to_list() == pytest.approx([1.0, 1.0])


def test_sobol_refuses_an_incomplete_grid():
    df = grid(lambda a, b: a + b)
    with pytest.raises(stats.InsufficientDataError, match="incomplete factorial"):
        sensitivity.factorial_sobol_indices(df[~((df.a == 2) & (df.b == 1))], factors=["a", "b"], outcome="y")


def test_sobol_refuses_a_constant_response():
    with pytest.raises(stats.InsufficientDataError, match="do not vary"):
        sensitivity.factorial_sobol_indices(grid(lambda a, b: 1.0), factors=["a", "b"], outcome="y")
