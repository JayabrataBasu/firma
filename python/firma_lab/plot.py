"""firma_lab.plot -- seed-band figures, fork comparisons, parameter-space
maps, distributions (manual Sec 23.1, Sec 29.2). Deliberately thin.

Conventions enforced here, not left to the caller:

- **Sec 23.1 (binding):** "every published time series MUST be a band across
  seeds, never a single line." ``seed_band`` and ``fork_band`` refuse data
  with fewer than two seeds at any x. The band's quantiles are a required
  argument -- the manual does not fix them, so every figure states its own.
- "Single trajectories MAY appear only in appendices, explicitly labelled
  illustrative": ``illustrative_trajectory`` always titles the figure
  ``ILLUSTRATIVE -- not evidence``.
- **Sec 29.2:** "Full distributions (ECDF or violin), never means alone":
  ``distribution`` draws ECDFs.

Dependency: **matplotlib** (added in this build) -- the standard scientific
plotting library; every function returns the ``Figure`` and never calls
``show()``, so it works headless (the ``Agg`` backend is selected here).
"""

from __future__ import annotations

from typing import Sequence

import matplotlib

matplotlib.use("Agg")

import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

ILLUSTRATIVE_PREFIX = "ILLUSTRATIVE -- not evidence (manual Sec 23.1)"


def _check_band(band: tuple[float, float]) -> None:
    lo, hi = band
    if not 0.0 <= lo < hi <= 1.0:
        raise ValueError(f"band must be quantiles 0 <= lo < hi <= 1, got {band}")


def _band_frame(
    df: pd.DataFrame, x: str, y: str, seed_col: str, band: tuple[float, float]
) -> pd.DataFrame:
    _check_band(band)
    n = df.groupby(x)[seed_col].nunique()
    if (n < 2).any():
        raise ValueError(
            f"a time series must be a band across seeds (Sec 23.1); "
            f"x value(s) {list(n[n < 2].index[:5])} have fewer than 2 seeds"
        )
    g = df.groupby(x)[y]
    return pd.DataFrame(
        {"median": g.median(), "lo": g.quantile(band[0]), "hi": g.quantile(band[1]), "n_seeds": n}
    ).sort_index()


def seed_band(
    df: pd.DataFrame,
    *,
    x: str,
    y: str,
    seed_col: str,
    band: tuple[float, float],
    group_col: str | None = None,
    title: str = "",
) -> plt.Figure:
    """Median line plus a ``band`` quantile band across seeds, per x value
    (optionally one band per ``group_col`` level)."""
    fig, ax = plt.subplots()
    groups = [(None, df)] if group_col is None else list(df.groupby(group_col, sort=True))
    for key, sub in groups:
        b = _band_frame(sub, x, y, seed_col, band)
        label = None if key is None else f"{group_col}={key}"
        (line,) = ax.plot(b.index, b["median"], label=label)
        ax.fill_between(b.index, b["lo"], b["hi"], alpha=0.25, color=line.get_color())
    ax.set_xlabel(x)
    ax.set_ylabel(f"{y} (median, {band[0]:.0%}-{band[1]:.0%} band across seeds)")
    if group_col is not None:
        ax.legend()
    ax.set_title(title)
    return fig


def fork_band(
    df: pd.DataFrame,
    *,
    x: str,
    factual: str,
    fork: str,
    seed_col: str,
    band: tuple[float, float],
    title: str = "",
) -> plt.Figure:
    """The CRN-matched paired difference ``factual - fork`` (Sec 14.6), as a
    band across seeds. Pairing is within a row: each row holds one seed's
    factual and forked value at one x."""
    d = df[[x, seed_col]].copy()
    d["diff"] = df[factual] - df[fork]
    fig = seed_band(d, x=x, y="diff", seed_col=seed_col, band=band, title=title)
    fig.axes[0].axhline(0.0, color="grey", linewidth=0.8)
    fig.axes[0].set_ylabel(f"{factual} - {fork} (paired, {band[0]:.0%}-{band[1]:.0%} band)")
    return fig


def illustrative_trajectory(df: pd.DataFrame, *, x: str, y: str, title: str) -> plt.Figure:
    """A single trajectory, for an appendix only; the title always carries
    ``ILLUSTRATIVE_PREFIX``."""
    fig, ax = plt.subplots()
    ax.plot(df[x], df[y])
    ax.set_xlabel(x)
    ax.set_ylabel(y)
    ax.set_title(f"{ILLUSTRATIVE_PREFIX}\n{title}")
    return fig


def parameter_space_map(
    df: pd.DataFrame, *, x: str, y: str, value: str, statistic: str, title: str = ""
) -> plt.Figure:
    """A heatmap of ``statistic`` (``"median"`` or ``"mean"``) of ``value``
    over the ``x`` x ``y`` grid of design cells. Cells with no data stay
    blank rather than being interpolated."""
    if statistic not in ("median", "mean"):
        raise ValueError("statistic must be 'median' or 'mean'")
    grid = df.pivot_table(index=y, columns=x, values=value, aggfunc=statistic).sort_index()
    fig, ax = plt.subplots()
    im = ax.imshow(np.ma.masked_invalid(grid.to_numpy()), origin="lower", aspect="auto")
    ax.set_xticks(range(len(grid.columns)), [str(c) for c in grid.columns])
    ax.set_yticks(range(len(grid.index)), [str(i) for i in grid.index])
    ax.set_xlabel(x)
    ax.set_ylabel(y)
    fig.colorbar(im, ax=ax, label=f"{statistic} {value}")
    ax.set_title(title)
    return fig


def distribution(
    df: pd.DataFrame, *, value: str, by: str | None = None, title: str = ""
) -> plt.Figure:
    """Empirical CDFs of ``value`` (one per level of ``by``), per Sec 29.2's
    "full distributions, never means alone"."""
    fig, ax = plt.subplots()
    groups: Sequence = [(None, df)] if by is None else list(df.groupby(by, sort=True))
    for key, sub in groups:
        v = np.sort(sub[value].to_numpy())
        ax.step(v, np.arange(1, len(v) + 1) / len(v), where="post",
                label=None if key is None else f"{by}={key}")
    ax.set_xlabel(value)
    ax.set_ylabel("ECDF")
    if by is not None:
        ax.legend()
    ax.set_title(title)
    return fig
