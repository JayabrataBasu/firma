#!/usr/bin/env python3
"""EXPLORATORY - DIAGNOSTIC-ONLY - NOT REGISTERED - NOT FOR TUNING.

One command that runs a small shocked experiment end to end (review round
R12, task SLICE):

    .venv/bin/python scripts/exploratory_e1_slice.py

Design (fixed by the reviewer prompt, docs/review/inbox/R12-*.md; nothing
here may be changed to get a nicer result):

* Base config ``configs/experiments/phase2-stage5-smoke.json`` with
  ``ticks = 120`` and ``shock.scheduled`` replaced by one ``shock.stochastic``
  rule. Seeds: mechanism 910001, environment 910002, init 910004; the shock
  seed runs over 910001..910050 (50 runs).
* No-shock fork (manual Sec 14.6): the identical config and seeds with the
  shock rule removed.
* Outcome: ``DeltaH_rep = H_rep(shocked) - H_rep(no-shock)`` per run, where
  ``H_rep`` is manual Sec 14.2 R1 (trailing window ``L_W = 8`` over selected
  actions, log2, per firm-tick), averaged over live firm-ticks.
* Controls: NULL (``decision.random``) and KNOWN-ANSWER (synthetic frames
  through the existing ``firma_lab.stats.analyse_h1c``).

**SLICE-2 (review R14) definitions, decided by the owner:**

* Shortfall per firm-tick is ``max_j shortfall_j`` as ``Focus::attend`` uses
  it, taken from the Rust analysis path (``firma_lab._native.firm_ticks``,
  i.e. ``firma_analysis::firm_ticks``). It is not re-derived in Python.
* Matched outcome: for each firm, the mean of ``H_rep(shocked) -
  H_rep(no-shock)`` over the firm-ticks in which that firm is alive in BOTH
  branches; then the mean over firms. One ``DeltaH_rep_matched`` per run.
  The R12 unmatched ``DeltaH_rep`` is kept next to it.
* H1c regressor: mean ``h`` over the matched live firm-ticks (shocked
  branch); covariate: mean max-shortfall over the same firm-ticks.

``h`` here is the analysis crate's reconstructed margin (true theta) at the
end of each tick. It is not the margin the firm perceived at decision time.

**Two things this script does itself because ``firma_lab`` cannot:**

1. It launches ``firma run --model`` directly. ``firma_lab.runner`` can only
   run ``ExperimentSpec`` jobs whose four stream seeds are equal
   (``derive_seeds(n) = (n, n, n, n)``) and has no no-shock fork.
2. It computes ``H_rep`` in Python from the event log, as the prompt
   directs (``firma_lab.metrics`` exposes only the run-level mean over the
   model's own persisted window).
"""

from __future__ import annotations

import argparse
import collections
import copy
import csv
import hashlib
import json
import math
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "python"))

import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

from firma_lab import _native, load, metrics, stats  # noqa: E402
from firma_lab.spec import StreamSeeds, seeds_in_reserved_range  # noqa: E402

BANNER = "EXPLORATORY - DIAGNOSTIC-ONLY - NOT REGISTERED - NOT FOR TUNING"

#: Diagnostics use seeds >= 910000 (docs/review/BUILDER.md).
MIN_DIAGNOSTIC_SEED = 910_000
MECHANISM_SEED, ENVIRONMENT_SEED, INIT_SEED = 910_001, 910_002, 910_004
SHOCK_SEEDS = range(910_001, 910_051)
TICKS = 120
#: Manual Sec 15.4 / the reviewer prompt: the outcome window.
OUTCOME_L_W = 8
BASE_CONFIG = REPO_ROOT / "configs" / "experiments" / "phase2-stage5-smoke.json"

STOCHASTIC_SHOCK = {
    "id": "shock.stochastic",
    "version": "^1",
    "params": {
        "id": "rand-reg",
        "channel": {"kind": "regulatory", "target": "theta_limit"},
        "onset_min": 10,
        "onset_max": 80,
        "magnitude_mean": 0.3,
        "magnitude_sd": 0.1,
        "ramp": {"kind": "instant"},
        "persistence": {"kind": "permanent"},
        "observability": {"kind": "full"},
        "novelty": 0.5,
        "targets": {"kind": "all"},
    },
}

CAVEATS = (
    "the model's focus rule switches at h_crit (decision/lib.rs:175): below it "
    "shortfall is never read, above it h is never read, so a discontinuity at "
    "h_crit is expected by construction",
    "4-firm population in which firm 2 dies at tick 12 in every satisficing run "
    "(checked below against this run's own death records)",
    "one variance source: the shock stream. The no-shock fork does not depend on "
    "the shock seed at all, so every run is paired with the same fork",
    "h is the reconstructed TRUE margin at the end of each tick, not the margin "
    "the firm perceived when deciding (observation is delayed in this config)",
    "H_rep counts SELECTED actions; an action selected under NONE-focus inertia "
    "is recorded even when it is not executed (audit F03)",
    "this is one exploratory run on a non-registered config; the verdict lines "
    "say what the existing H1c analysis returned here and nothing more",
)

#: Exit statuses.
EXIT_OK = 0  # both verdict lines were computed (whatever they say)
EXIT_FAILED = 1  # a run failed or a known-answer control gave the wrong answer
EXIT_UNINTERPRETABLE = 2  # the H1c analysis refused the data for an arm
#: Manual Sec 30.7: bootstrap resamples for the H1c fits.
DEFAULT_RESAMPLES = stats.MANUAL_BOOTSTRAP_RESAMPLES
#: Seed-grouped cross-validation folds for H1c (not fixed by the manual; OQ-20).
CV_FOLDS = 5


class SliceSeedError(RuntimeError):
    """A seed below ``MIN_DIAGNOSTIC_SEED`` was requested."""


# ---------------------------------------------------------------------------
# Seed guard
# ---------------------------------------------------------------------------


def check_seeds(seeds: Mapping[str, int]) -> None:
    """Refuse any stream seed below ``MIN_DIAGNOSTIC_SEED``.

    Reuses the Batch 1 guard (``firma_lab.spec.seeds_in_reserved_range``,
    which covers the registered range 1-200) and adds this task's stricter
    floor. Raises before anything is written or executed."""
    reserved = seeds_in_reserved_range(StreamSeeds(**seeds))
    if reserved:
        raise SliceSeedError(
            f"stream seed(s) {reserved} are in the reserved range 1-200: {dict(seeds)}"
        )
    low = sorted(k for k, v in seeds.items() if v < MIN_DIAGNOSTIC_SEED)
    if low:
        raise SliceSeedError(
            f"stream seed(s) {low} are below {MIN_DIAGNOSTIC_SEED}: {dict(seeds)}; "
            "this exploratory slice only runs on diagnostic seeds"
        )


# ---------------------------------------------------------------------------
# Configs
# ---------------------------------------------------------------------------


def build_config(
    base: Mapping[str, Any], *, decision: str, shock_seed: int, shocked: bool
) -> dict[str, Any]:
    """The run config for one arm (``decision`` = ``"satisficing"`` or
    ``"random"``) and one shock seed. ``shocked=False`` is the no-shock fork:
    the same config and seeds with the shock rule removed, nothing else."""
    if decision not in ("satisficing", "random"):
        raise ValueError(f"unknown decision arm {decision!r}")
    cfg = copy.deepcopy(dict(base))
    seeds = {
        "mechanism": MECHANISM_SEED,
        "environment": ENVIRONMENT_SEED,
        "shock": shock_seed,
        "init": INIT_SEED,
    }
    check_seeds(seeds)
    cfg["seeds"] = seeds
    cfg["experiment"] = (
        f"EXPLORATORY-DIAGNOSTIC-ONLY-slice-{decision}-"
        f"{'shocked' if shocked else 'noshock-fork'}"
    )
    cfg["world"]["ticks"] = TICKS
    cfg["world"]["snapshot_every"] = TICKS
    rules: list[dict[str, Any]] = []
    n_scheduled = 0
    for rule in cfg["rules"]:
        if rule["id"] == "shock.scheduled":
            n_scheduled += 1
            if shocked:
                rules.append(copy.deepcopy(STOCHASTIC_SHOCK))
        elif rule["id"] == "decision.satisficing" and decision == "random":
            rules.append({"id": "decision.random", "version": "^1", "params": {}})
        else:
            rules.append(rule)
    if n_scheduled != 1:
        raise ValueError(f"expected exactly one shock.scheduled rule, found {n_scheduled}")
    cfg["rules"] = rules
    return cfg


def find_firma(explicit: str | None) -> Path:
    candidates = (
        [Path(explicit)]
        if explicit
        else [REPO_ROOT / "target" / "debug" / "firma", REPO_ROOT / "target" / "release" / "firma"]
    )
    for c in candidates:
        if c.exists():
            return c
    raise FileNotFoundError(
        f"firma binary not found (looked in {[str(c) for c in candidates]}); "
        "run `cargo build -p firma-cli` first"
    )


def run_config(cfg: Mapping[str, Any], run_dir: Path, firma: Path) -> str | None:
    """Run one config with ``firma run --model``. Returns ``None`` on success,
    else the failure text. (Local executor: see the module docstring for why
    ``firma_lab.runner`` cannot be used.)"""
    check_seeds(cfg["seeds"])
    shutil.rmtree(run_dir, ignore_errors=True)
    run_dir.mkdir(parents=True)
    config_path = run_dir / "DIAGNOSTIC_ONLY_config.json"
    config_path.write_text(json.dumps(cfg, indent=1))
    proc = subprocess.run(
        [str(firma), "run", "--model", "--config", str(config_path), "--out", str(run_dir / "run")],
        capture_output=True,
        text=True,
        timeout=600,
    )
    if proc.returncode != 0:
        return f"exit {proc.returncode}: {proc.stderr.strip()[-400:]}"
    return None


# ---------------------------------------------------------------------------
# Repertoire entropy (manual Sec 14.2 R1)
# ---------------------------------------------------------------------------


def repertoire_entropy(actions: Sequence[int]) -> float:
    """``H_rep = -sum_{a: p_a > 0} p_a log2 p_a`` [bits] over a window of
    action indices (manual Sec 14.2 R1). Empty window -> 0."""
    n = len(actions)
    if n == 0:
        return 0.0
    counts = collections.Counter(actions)
    return -sum((k / n) * math.log2(k / n) for k in counts.values())


def normalised_entropy(actions: Sequence[int], n_admissible: int) -> float:
    """``H_rep / log2 |A_adm|`` (manual Sec 14.2 R1)."""
    if n_admissible < 2:
        raise ValueError("normalised entropy needs |A_adm| >= 2")
    return repertoire_entropy(actions) / math.log2(n_admissible)


def decisions_and_deaths(
    events: pd.DataFrame,
) -> tuple[dict[int, list[tuple[int, int]]], dict[int, tuple[int, str]]]:
    """From ``firma_lab.load.read_events``: per agent, the ``(tick, selected
    action)`` list in tick order; and per dead agent, ``(tick, cause)``."""
    field_col, value_col = "kind.SetAgentInt.field", "kind.SetAgentInt.value"
    by_agent: dict[int, list[tuple[int, int]]] = collections.defaultdict(list)
    if field_col in events.columns:
        d = events[
            (events["event"] == "delta_applied")
            & (events[field_col] == "selected_action")
            & events["origin"].astype(str).str.startswith("decision.")
        ]
        for tick, agent, action in zip(d["tick"], d["target.Agent"], d[value_col]):
            by_agent[int(agent)].append((int(tick), int(action)))
    for seq in by_agent.values():
        seq.sort()
    deaths: dict[int, tuple[int, str]] = {}
    died = events[events["event"] == "agent_died"]
    if len(died):  # a run in which no firm dies has no "agent"/"cause" columns
        for tick, agent, cause in zip(died["tick"], died["agent"], died["cause"]):
            deaths[int(agent)] = (int(tick), str(cause))
    return dict(by_agent), deaths


def firm_tick_entropies(
    by_agent: Mapping[int, Sequence[tuple[int, int]]],
    deaths: Mapping[int, tuple[int, str]],
    l_w: int,
) -> dict[tuple[int, int], float]:
    """``H_rep`` for every live firm-tick, keyed ``(agent, tick)``.

    For each firm and each tick it decided in, the window is its last
    ``l_w`` selected actions up to and including that tick. A firm-tick is
    *live* if the firm is still alive when the tick completes -- the tick in
    which a firm dies is excluded -- which is the definition
    ``firma_analysis`` uses (``sanity`` and ``firm_ticks``)."""
    if l_w < 1:
        raise ValueError("l_w must be >= 1")
    out: dict[tuple[int, int], float] = {}
    for agent, seq in by_agent.items():
        death_tick = deaths[agent][0] if agent in deaths else None
        actions = [a for _, a in seq]
        for i, (tick, _) in enumerate(seq):
            if death_tick is not None and tick >= death_tick:
                continue
            out[(agent, tick)] = repertoire_entropy(actions[max(0, i + 1 - l_w) : i + 1])
    return out


def mean_repertoire_entropy(
    by_agent: Mapping[int, Sequence[tuple[int, int]]],
    deaths: Mapping[int, tuple[int, str]],
    l_w: int,
) -> tuple[float, int]:
    """Mean of ``H_rep`` over live firm-ticks, and the number of firm-ticks
    (the R12 unmatched per-run summary). See ``firm_tick_entropies``."""
    values = list(firm_tick_entropies(by_agent, deaths, l_w).values())
    if not values:
        return float("nan"), 0
    return float(sum(values) / len(values)), len(values)


def matched_summary(
    h_shocked: Mapping[tuple[int, int], float],
    h_fork: Mapping[tuple[int, int], float],
    ft_shocked: Mapping[tuple[int, int], Mapping[str, Any]],
) -> dict[str, Any]:
    """The owner-decided matched quantities for one shocked run and its fork.

    ``h_shocked`` / ``h_fork`` map ``(agent, tick)`` to ``H_rep`` for the live
    firm-ticks of each branch; ``ft_shocked`` maps the shocked branch's live
    firm-ticks to the Rust ``firm_ticks`` record (``h``, ``shortfall_max``).

    * matched firm-ticks: ``(agent, tick)`` live in BOTH branches;
    * ``delta_h_rep_matched``: per firm, the mean over its matched ticks of
      ``H_rep(shocked) - H_rep(fork)``; then the mean over the firms that have
      at least one matched tick;
    * ``h_mean_matched`` / ``shortfall_max_mean_matched``: the mean, over all
      matched firm-ticks, of the shocked branch's ``h`` and ``max_j
      shortfall_j``.
    """
    matched = sorted(set(h_shocked) & set(h_fork))
    per_firm: dict[int, list[float]] = collections.defaultdict(list)
    for agent, tick in matched:
        per_firm[agent].append(h_shocked[(agent, tick)] - h_fork[(agent, tick)])
    if not matched:
        nan = float("nan")
        return {"n_matched_firm_ticks": 0, "n_matched_firms": 0, "delta_h_rep_matched": nan,
                "h_mean_matched": nan, "shortfall_max_mean_matched": nan,
                "matched_ticks_per_firm": "{}"}
    firm_means = {a: sum(v) / len(v) for a, v in per_firm.items()}
    missing = [k for k in matched if k not in ft_shocked]
    if missing:
        raise ValueError(f"no Rust firm-tick record for matched firm-ticks, e.g. {missing[:3]}")
    return {
        "n_matched_firm_ticks": len(matched),
        "n_matched_firms": len(firm_means),
        "delta_h_rep_matched": sum(firm_means.values()) / len(firm_means),
        "h_mean_matched": sum(ft_shocked[k]["h"] for k in matched) / len(matched),
        "shortfall_max_mean_matched": sum(ft_shocked[k]["shortfall_max"] for k in matched)
        / len(matched),
        "matched_ticks_per_firm": json.dumps({a: len(v) for a, v in sorted(per_firm.items())}),
    }


def drawn_shock(events: pd.DataFrame) -> tuple[int | None, float | None]:
    """The stochastic shock's drawn (onset, magnitude), read from the first
    ``active_shocks`` record; ``(None, None)`` for a no-shock fork."""
    col = "kind.ReplaceGlobalList.records_json"
    if col not in events.columns:
        return None, None
    rows = events[events["kind.ReplaceGlobalList.list"] == "active_shocks"][col]
    for recs in rows:
        if isinstance(recs, list) and recs:
            s = json.loads(recs[0])["shock"]
            return int(s["onset"]), float(s["magnitude"])
    return None, None


def action_sha(by_agent: Mapping[int, Sequence[tuple[int, int]]]) -> str:
    h = hashlib.sha256()
    for agent in sorted(by_agent):
        h.update(json.dumps([agent, list(by_agent[agent])]).encode())
    return h.hexdigest()


# ---------------------------------------------------------------------------
# One arm: 50 shocked runs + 50 no-shock forks
# ---------------------------------------------------------------------------


def rust_firm_ticks(run_dir: Path) -> dict[tuple[int, int], dict[str, Any]]:
    """``firma_analysis::firm_ticks`` for one run, keyed ``(agent, tick)``.
    ``h`` and the shortfalls come from Rust; nothing is re-derived here."""
    rows = _native.firm_ticks(load.read_config_json(run_dir / "run"), str(run_dir / "run"))
    return {(int(r["agent"]), int(r["tick"])): r for r in rows}


def measure_run(run_dir: Path, model_l_w: int) -> dict[str, Any]:
    events = load.read_events(run_dir / "run")
    by_agent, deaths = decisions_and_deaths(events)
    entropies = firm_tick_entropies(by_agent, deaths, OUTCOME_L_W)
    h8, n_ft = mean_repertoire_entropy(by_agent, deaths, OUTCOME_L_W)
    h_model, _ = mean_repertoire_entropy(by_agent, deaths, model_l_w)
    onset, magnitude = drawn_shock(events)
    report = metrics.sanity_report(run_dir / "run")
    ft = rust_firm_ticks(run_dir)
    if set(ft) != set(entropies):
        raise ValueError(
            f"{run_dir}: live firm-ticks differ between the Python log reader "
            f"({len(entropies)}) and firma_analysis::firm_ticks ({len(ft)})"
        )
    return {
        "h_rep_l8": h8,
        "h_rep_model_window": h_model,
        "sc6_entropy_mean": report["sc6_entropy_mean"],
        "live_firm_ticks": n_ft,
        "deaths": len(deaths),
        "death_records": sorted((t, a, c) for a, (t, c) in deaths.items()),
        "survivors": report["survivors"],
        "drawn_onset": onset,
        "drawn_magnitude": magnitude,
        "action_sha": action_sha(by_agent),
        "entropies": entropies,
        "firm_ticks": ft,
    }


def run_arm(
    base: Mapping[str, Any], decision: str, work: Path, firma: Path, shock_seeds: Iterable[int]
) -> tuple[list[dict[str, Any]], list[tuple[int, str]]]:
    model_l_w = next(
        r["params"]["l_w"] for r in base["rules"] if r["id"] == "constraint.action_window"
    )
    rows, failed = [], []
    for seed in shock_seeds:
        pair = {}
        for shocked in (True, False):
            run_dir = work / decision / f"shock{seed}" / ("shocked" if shocked else "noshock")
            err = run_config(
                build_config(base, decision=decision, shock_seed=seed, shocked=shocked),
                run_dir,
                firma,
            )
            if err:
                failed.append((seed, f"{'shocked' if shocked else 'noshock'}: {err}"))
                pair = {}
                break
            pair[shocked] = measure_run(run_dir, model_l_w)
        if not pair:
            continue
        s, f = pair[True], pair[False]
        rows.append(
            {
                "arm": decision,
                "seed": seed,
                "shock_seed": seed,
                "drawn_onset": s["drawn_onset"],
                "drawn_magnitude": s["drawn_magnitude"],
                "deaths_shocked": s["deaths"],
                "survivors_shocked": s["survivors"],
                "deaths_noshock": f["deaths"],
                "h_rep_shocked_l8": s["h_rep_l8"],
                "h_rep_noshock_l8": f["h_rep_l8"],
                "delta_h_rep": s["h_rep_l8"] - f["h_rep_l8"],
                "h_rep_shocked_model_window": s["h_rep_model_window"],
                "sc6_entropy_mean_shocked": s["sc6_entropy_mean"],
                "crosscheck_diff": s["h_rep_model_window"] - s["sc6_entropy_mean"],
                "live_firm_ticks_shocked": s["live_firm_ticks"],
                "live_firm_ticks_noshock": f["live_firm_ticks"],
                **matched_summary(s["entropies"], f["entropies"], s["firm_ticks"]),
                "action_sha_shocked": s["action_sha"],
                "action_sha_noshock": f["action_sha"],
                "death_records_shocked": json.dumps(s["death_records"]),
            }
        )
    return rows, failed


# ---------------------------------------------------------------------------
# KNOWN-ANSWER control: the existing H1c analysis on synthetic frames
# ---------------------------------------------------------------------------


OUTCOME_COL = "delta_h_rep_matched"
PREDICTOR_COL = "h_mean_matched"
COVARIATE_COL = "shortfall_max_mean_matched"


def synthetic_frame(quad: float, *, n: int = 50, gen_seed: int = 5) -> pd.DataFrame:
    """One row per seed -- the shape of the real slice frame, with the real
    frame's column names -- where the outcome is ``0.3 x + quad x^2 + 0.2 z +
    noise`` for predictor ``x`` and an independent covariate ``z``.
    ``quad = -1`` plants an inverted U; ``quad = 0`` plants a monotone
    relation."""
    rng = np.random.default_rng(gen_seed)
    x = rng.uniform(-1.0, 1.0, n)
    z = rng.uniform(0.0, 1.0, n)
    return pd.DataFrame(
        {
            "seed": np.arange(MIN_DIAGNOSTIC_SEED + 1, MIN_DIAGNOSTIC_SEED + 1 + n),
            PREDICTOR_COL: x,
            COVARIATE_COL: z,
            OUTCOME_COL: 0.3 * x + quad * x**2 + 0.2 * z + rng.normal(0.0, 0.1, n),
        }
    )


def h1c_on_frame(df: pd.DataFrame, *, n_resamples: int):
    """The single call into the existing analysis that this slice uses, for
    real and synthetic frames alike (``firma_lab.stats.analyse_h1c``; unit of
    analysis = the run, grouped by seed, manual Sec 29.1). Owner-decided
    (R14): regressor = mean h over matched live firm-ticks, covariate = mean
    max-shortfall over the same firm-ticks; outcome = matched DeltaH_rep."""
    return stats.analyse_h1c(
        df,
        outcome=OUTCOME_COL,
        outcome_direction=1,
        predictor_col=PREDICTOR_COL,
        covariates=[COVARIATE_COL],
        seed_col="seed",
        cv_folds=CV_FOLDS,
        bootstrap_seed=910_001,
        n_resamples=n_resamples,
    )


def fit_summary(r) -> dict[str, Any]:
    q = r.estimates[r.key_term]
    lin = r.estimates[PREDICTOR_COL]
    return {
        "verdict": r.verdict,
        "linear": (lin.estimate, lin.ci_low, lin.ci_high),
        "quadratic": (q.estimate, q.ci_low, q.ci_high),
        "failed_resamples": q.n_failed_resamples,
        "resamples": q.n_resamples,
        "cv_mse_linear": r.details["cv_mse_linear"],
        "cv_mse_quadratic": r.details["cv_mse_quadratic"],
        "quadratic_preferred": r.details["quadratic_preferred"],
        "quadratic_directed_negative": r.details["quadratic_directed_negative"],
    }


def known_answer_controls(n_resamples: int) -> dict[str, Any]:
    out = {}
    for label, quad, expected in (
        ("planted inverted-U", -1.0, "supported"),
        ("planted monotone", 0.0, "not_supported"),
    ):
        f = fit_summary(h1c_on_frame(synthetic_frame(quad), n_resamples=n_resamples))
        out[label] = {"expected": expected, "ok": f["verdict"] == expected, **f}
    return out


def fit_arm(rows: Sequence[Mapping[str, Any]], n_resamples: int) -> dict[str, Any]:
    """H1c on one arm's per-run rows. A refusal by the analysis
    (``InsufficientDataError``) is returned as ``uninterpretable`` with its
    reason, never replaced by a number."""
    df = pd.DataFrame(
        [{k: r[k] for k in ("seed", OUTCOME_COL, PREDICTOR_COL, COVARIATE_COL)} for r in rows]
    )
    try:
        return fit_summary(h1c_on_frame(df, n_resamples=n_resamples))
    except stats.InsufficientDataError as e:
        return {"verdict": "uninterpretable", "reason": str(e)}


# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------


def describe(rows: Sequence[Mapping[str, Any]], failed: Sequence[tuple[int, str]]) -> list[str]:
    lines = []
    n = len(rows)
    lines.append(f"  runs completed: {n} shocked + {n} no-shock forks; failed: {len(failed)}")
    for seed, why in failed:
        lines.append(f"    FAILED shock seed {seed}: {why}")
    if not rows:
        return lines
    deaths = collections.Counter(r["deaths_shocked"] for r in rows)
    lines.append(
        "  deaths per shocked run: "
        + ", ".join(f"{k} deaths in {v} runs" for k, v in sorted(deaths.items()))
    )
    forks = {r["action_sha_noshock"] for r in rows}
    lines.append(
        f"  no-shock fork: {len(forks)} distinct action sequence(s) across {n} shock seeds; "
        f"H_rep(no-shock) = {sorted({round(r['h_rep_noshock_l8'], 12) for r in rows})}"
    )
    lines.append(
        f"  shocked runs: {len({r['action_sha_shocked'] for r in rows})} distinct action sequences"
    )
    d = np.array([r["delta_h_rep"] for r in rows])
    sd = float(d.std(ddof=1)) if n > 1 else float("nan")
    lines.append(
        f"  DeltaH_rep (L_W = {OUTCOME_L_W}): mean {d.mean():+.6f}, range "
        f"[{d.min():+.6f}, {d.max():+.6f}], sample sd {sd:.6f}, "
        f"{len(set(np.round(d, 12)))} distinct values"
    )
    x = np.array([abs(r["crosscheck_diff"]) for r in rows])
    lines.append(
        "  cross-check vs firma_lab.metrics sanity_report sc6_entropy_mean "
        f"(my mean at the model's own window): max |difference| over {n} runs = {x.max():.3e}"
    )
    m = np.array([r["delta_h_rep_matched"] for r in rows])
    lines.append(
        f"  DeltaH_rep MATCHED (per firm over firm-ticks alive in both branches, then mean "
        f"over firms): mean {np.nanmean(m):+.6f}, range [{np.nanmin(m):+.6f}, {np.nanmax(m):+.6f}], "
        f"sample sd {float(np.nanstd(m, ddof=1)):.6f}, {len(set(np.round(m, 12)))} distinct values"
    )
    nm = np.array([r["n_matched_firm_ticks"] for r in rows])
    ns = np.array([r["live_firm_ticks_shocked"] for r in rows])
    lines.append(
        f"  matched firm-ticks per run: min {nm.min()}, median {int(np.median(nm))}, max {nm.max()} "
        f"(live firm-ticks in the shocked run: {ns.min()}..{ns.max()}; in the fork: "
        f"{sorted({r['live_firm_ticks_noshock'] for r in rows})}); firms with a matched tick: "
        f"{sorted({r['n_matched_firms'] for r in rows})}"
    )
    lines.append("  DeltaH_rep by number of deaths in the shocked run:")
    lines.append("    deaths  runs  unmatched mean [min, max]            matched mean [min, max]")
    for k in sorted(deaths):
        sub_rows = [r for r in rows if r["deaths_shocked"] == k]
        u = np.array([r["delta_h_rep"] for r in sub_rows])
        mm = np.array([r["delta_h_rep_matched"] for r in sub_rows])
        lines.append(
            f"    {k:<7} {len(sub_rows):<5} {u.mean():+.3f} [{u.min():+.3f}, {u.max():+.3f}]"
            f"            {mm.mean():+.3f} [{mm.min():+.3f}, {mm.max():+.3f}]"
        )
    for key, label in (
        (PREDICTOR_COL, "regressor: mean h over matched firm-ticks"),
        (COVARIATE_COL, "covariate: mean max-shortfall over matched firm-ticks"),
    ):
        v = np.array([r[key] for r in rows], dtype=float)
        lines.append(
            f"  {label}: range [{np.nanmin(v):.4f}, {np.nanmax(v):.4f}], "
            f"{len(set(np.round(v, 12)))} distinct values"
        )
    return lines


def fit_lines(f: Mapping[str, Any]) -> list[str]:
    if f["verdict"] == "uninterpretable":
        return [f"  the analysis refused the data: {f['reason']}"]
    lin, quad = f["linear"], f["quadratic"]
    return [
        f"  fitted linear term (h):      {lin[0]:+.4f}  95% CI [{lin[1]:+.4f}, {lin[2]:+.4f}]",
        f"  fitted quadratic term (h^2): {quad[0]:+.4f}  95% CI [{quad[1]:+.4f}, {quad[2]:+.4f}]",
        f"  quadratic negative with CI excluding zero: {f['quadratic_directed_negative']}; "
        f"seed-grouped {CV_FOLDS}-fold CV MSE linear {f['cv_mse_linear']:.6f} vs quadratic "
        f"{f['cv_mse_quadratic']:.6f} -> quadratic preferred: {f['quadratic_preferred']}",
        f"  bootstrap resamples: {f['resamples']} ({f['failed_resamples']} failed to fit)",
    ]


def firm2_caveat_holds(rows: Sequence[Mapping[str, Any]]) -> tuple[int, int]:
    hit = sum(1 for r in rows if [12, 2, "solvency"] in json.loads(r["death_records_shocked"]))
    return hit, len(rows)


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=BANNER)
    ap.add_argument("--out", help="directory for run outputs and the per-run CSV (default: a temp dir)")
    ap.add_argument("--firma", help="path to the firma binary (default: target/debug, then target/release)")
    ap.add_argument(
        "--resamples", type=int, default=DEFAULT_RESAMPLES,
        help="bootstrap resamples for every H1c fit (default: the manual's 10,000)",
    )
    args = ap.parse_args(argv)
    started = time.time()

    print(BANNER)
    print(f"command: {' '.join([Path(sys.executable).name, *sys.argv])}")
    firma = find_firma(args.firma)
    out = Path(args.out) if args.out else Path(tempfile.mkdtemp(prefix="firma-exploratory-slice-"))
    out.mkdir(parents=True, exist_ok=True)
    base = json.loads(BASE_CONFIG.read_text())
    print(f"base config: {BASE_CONFIG.relative_to(REPO_ROOT)}; ticks = {TICKS}; firma: {firma}")
    print(
        f"seeds: mechanism {MECHANISM_SEED}, environment {ENVIRONMENT_SEED}, init {INIT_SEED}, "
        f"shock {SHOCK_SEEDS.start}..{SHOCK_SEEDS.stop - 1} ({len(SHOCK_SEEDS)} runs per arm)"
    )
    print(f"output directory: {out}")

    all_rows: list[dict[str, Any]] = []
    results = {}
    for decision, title in (
        ("satisficing", "MAIN ARM - decision.satisficing"),
        ("random", "NULL CONTROL - decision.random"),
    ):
        rows, failed = run_arm(base, decision, out / "runs", firma, SHOCK_SEEDS)
        results[decision] = (rows, failed)
        all_rows.extend(rows)
        print(f"\n== {title} ==")
        for line in describe(rows, failed):
            print(line)

    csv_path = out / "per_run_table.csv"
    if all_rows:
        with open(csv_path, "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=list(all_rows[0]))
            w.writeheader()
            w.writerows(all_rows)
        print(f"\nper-run table: {csv_path} ({len(all_rows)} rows)")

    print("\n== KNOWN-ANSWER CONTROL - the same H1c call on synthetic frames ==")
    print(f"  (one row per seed, 50 seeds, same columns and covariate; {args.resamples} resamples)")
    ka = known_answer_controls(args.resamples)
    for label, r in ka.items():
        lin, quad = r["linear"], r["quadratic"]
        print(
            f"  {label}: verdict {r['verdict']} (expected {r['expected']}) -> "
            f"{'OK' if r['ok'] else 'CONTROL FAILED'}; linear {lin[0]:+.3f} "
            f"[{lin[1]:+.3f}, {lin[2]:+.3f}], quadratic {quad[0]:+.3f} [{quad[1]:+.3f}, {quad[2]:+.3f}]"
        )
    ka_ok = all(r["ok"] for r in ka.values())

    fits = {}
    for decision, title in (("satisficing", "MAIN ARM"), ("random", "NULL CONTROL")):
        rows = results[decision][0]
        print(f"\n== H1c FIT - {title} ({decision}) ==")
        print(
            f"  {OUTCOME_COL} ~ {PREDICTOR_COL} + {PREDICTOR_COL}^2 + {COVARIATE_COL} + (1 | seed), "
            f"{len(rows)} runs"
        )
        fits[decision] = fit_arm(rows, args.resamples) if rows else {
            "verdict": "uninterpretable", "reason": "no completed runs"}
        for line in fit_lines(fits[decision]):
            print(line)

    def verdict_text(f):
        return (
            f"UNINTERPRETABLE ({f['reason']})" if f["verdict"] == "uninterpretable"
            else f"H1c {f['verdict']} in this exploratory run"
        )

    print(f"\nVERDICT (satisficing arm): {verdict_text(fits['satisficing'])}")
    print(f"VERDICT (null control, decision.random): {verdict_text(fits['random'])}")
    print(
        "VERDICT (known-answer controls): "
        + ("both as expected (supported / not_supported)" if ka_ok else "CONTROL FAILED")
    )

    print("\ncaveats:")
    sat_rows = results["satisficing"][0]
    hit, total = firm2_caveat_holds(sat_rows)
    for c in CAVEATS:
        print(f"  - {c}")
    print(f"    (firm 2 died of solvency at tick 12 in {hit} of {total} satisficing shocked runs)")
    print(f"\nwall-clock time: {time.time() - started:.1f} s")
    print(BANNER)

    n_failed = sum(len(f) for _, f in results.values())
    if not ka_ok or n_failed:
        status = EXIT_FAILED
    elif any(f["verdict"] == "uninterpretable" for f in fits.values()):
        status = EXIT_UNINTERPRETABLE
    else:
        status = EXIT_OK
    print(f"exit status: {status} (0 = both verdict lines computed; 1 = a run or control failed; "
          "2 = the analysis refused an arm's data)")
    return status


if __name__ == "__main__":
    raise SystemExit(main())
