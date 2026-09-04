"""firma_lab.metrics -- offline metric computation (manual Sec 14, Sec
22.2, Sec 17 A5).

Every number this module returns comes from ``firma_lab._native`` --
the PyO3 extension built from ``firma-analysis`` (ADR 0045/0046) -- not from
a Python re-derivation. ``firma-conformance``'s Rust tests, ``firma-tui``'s
live panels, and this module all call the *same* Rust function for a given
computation; none of them carries its own copy of ``standard_margin``,
the four ``g_j``, ``u``, or repertoire entropy.
"""

from __future__ import annotations

from pathlib import Path
from typing import Union

import pandas as pd

from firma_lab import _native, load

PathLike = Union[str, Path]


def sanity_report(run_dir: PathLike) -> dict:
    """The SC-1...SC-6 sanity report (manual Sec 16.2) for a completed run.

    Exactly what ``firma-conformance``'s
    ``tests/tests/sanity.rs::sc16_gate`` would compute for the same log --
    same Rust function (``firma_analysis::sanity::sanity_from_run``), same
    thresholds. See Stage 7 Part D for a worked cross-check.
    """
    run_dir = Path(run_dir)
    config_json = load.read_config_json(run_dir)
    return _native.sanity_report(config_json, str(run_dir))


def final_margins(run_dir: PathLike) -> pd.DataFrame:
    """Every firm's final reconstructed state as a DataFrame, one row per
    firm: ``h``, the four ``g_j`` (Sec 9.1, columns ``g_solvency`` /
    ``g_compliance`` / ``g_scope`` / ``g_obligation``), ``u``
    (``regulated_intensity``), and ``repertoire_entropy`` over its trailing
    action window.
    """
    run_dir = Path(run_dir)
    config_json = load.read_config_json(run_dir)
    rows = _native.final_margins(config_json, str(run_dir))
    df = pd.DataFrame(rows)
    if not df.empty:
        g = pd.DataFrame(
            df["g"].tolist(),
            columns=["g_solvency", "g_compliance", "g_scope", "g_obligation"],
        )
        df = pd.concat([df.drop(columns=["g"]), g], axis=1)
    return df
