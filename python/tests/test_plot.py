"""Tests for firma_lab.plot's enforced conventions (synthetic data)."""

from __future__ import annotations

import numpy as np
import pandas as pd
import pytest

from firma_lab import plot


def series(n_seeds: int) -> pd.DataFrame:
    rng = np.random.default_rng(5)
    return pd.DataFrame(
        [dict(tick=t, seed=1001 + s, y=t * 0.1 + rng.normal(), y0=t * 0.1)
         for s in range(n_seeds) for t in range(20)]
    )


def test_seed_band_draws_a_band_not_a_line():
    fig = plot.seed_band(series(5), x="tick", y="y", seed_col="seed", band=(0.1, 0.9))
    ax = fig.axes[0]
    assert len(ax.lines) == 1 and len(ax.collections) == 1  # median + filled band
    assert "band across seeds" in ax.get_ylabel()


def test_single_seed_time_series_is_refused():
    with pytest.raises(ValueError, match="band across seeds"):
        plot.seed_band(series(1), x="tick", y="y", seed_col="seed", band=(0.1, 0.9))


def test_band_quantiles_are_validated():
    with pytest.raises(ValueError, match="quantiles"):
        plot.seed_band(series(3), x="tick", y="y", seed_col="seed", band=(0.9, 0.1))


def test_fork_band_plots_the_paired_difference():
    fig = plot.fork_band(series(4), x="tick", factual="y", fork="y0", seed_col="seed", band=(0.25, 0.75))
    assert "paired" in fig.axes[0].get_ylabel()


def test_illustrative_trajectory_is_always_labelled():
    fig = plot.illustrative_trajectory(series(1), x="tick", y="y", title="one run")
    assert fig.axes[0].get_title().startswith(plot.ILLUSTRATIVE_PREFIX)


def test_parameter_space_map_requires_an_explicit_statistic():
    df = pd.DataFrame([dict(h=h, s=s, v=h + s) for h in (0.1, 0.2) for s in (0, 1)])
    plot.parameter_space_map(df, x="h", y="s", value="v", statistic="median")
    with pytest.raises(ValueError):
        plot.parameter_space_map(df, x="h", y="s", value="v", statistic="max")


def test_distribution_draws_ecdfs():
    fig = plot.distribution(series(3), value="y", by="seed")
    assert len(fig.axes[0].lines) == 3 and fig.axes[0].get_ylabel() == "ECDF"
