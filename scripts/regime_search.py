#!/usr/bin/env python3
"""DIAGNOSTIC-ONLY - REGIME SEARCH - NOT REGISTERED - NOT FOR TUNING.

Review round R16, task REGIME. One question: does ANY configuration in a
pre-declared grid give a stressed-but-alive population (manual Sec 26.4)?
No H1a/H1b/H1c statistic is computed here.

Step 0 (before any run):

    .venv/bin/python scripts/regime_search.py --write-grid docs/review/evidence/R16/grid.json

Then:

    .venv/bin/python scripts/regime_search.py --grid docs/review/evidence/R16/grid.json --out <dir>

Grid 2 (review R18, two stages; its axes were chosen after grid 1's results
were seen, which the output states):

    .venv/bin/python scripts/regime_search.py --write-grid2 docs/review/evidence/R18/grid2.json
    .venv/bin/python scripts/regime_search.py --grid2 docs/review/evidence/R18/grid2.json --out <dir>

The second command refuses to run unless the grid file is byte-for-byte the
grid this script declares, so the grid and the targets cannot drift after
step 0. Results are printed and written in the declared cell order; nothing
is sorted by result.

The run launcher, the log reader, the per-firm-tick entropy and the matched
firm-tick DeltaH are imported from ``scripts/exploratory_e1_slice.py`` (R12/R14), not copied.
"""

from __future__ import annotations

import argparse
import copy
import csv
import hashlib
import importlib.util
import itertools
import json
import shutil
import sys
import tempfile
import time
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "python"))


def _load_slice():
    path = Path(__file__).with_name("exploratory_e1_slice.py")
    spec = importlib.util.spec_from_file_location("exploratory_e1_slice", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


sl = _load_slice()

from firma_lab import load, metrics  # noqa: E402

BANNER = "DIAGNOSTIC-ONLY - REGIME SEARCH - NOT REGISTERED - NOT FOR TUNING"

BASE_CONFIG = sl.BASE_CONFIG  # configs/experiments/phase2-stage5-smoke.json
MECHANISM_SEED, ENVIRONMENT_SEED, INIT_SEED = 910_001, 910_002, 910_004
SHOCK_SEEDS = list(range(910_001, 910_021))  # 20 shocked runs per cell
TICKS = 120

# ---------------------------------------------------------------------------
# The declared grid (STEP 0). Everything below is written verbatim into
# grid.json; after step 0 it may not change in this task.
# ---------------------------------------------------------------------------

GRID: dict[str, Any] = {
    "task": "R16 REGIME - DIAGNOSTIC-ONLY regime search; not registered; not for tuning",
    "question": "does any cell give a stressed-but-alive population?",
    "axes_in_declared_order": [
        {"name": "n", "levels": [8, 12], "meaning": "number of firms"},
        {
            "name": "capital",
            "levels": [[50, 100], [100, 200], [150, 300]],
            "meaning": "(c_min, c_max): starting liquid capital, linear across firms",
            "spacing_rule": "c_min in {0.5, 1.0, 1.5} x s_L (s_L = 100, manual 9.2); "
            "c_max = 2 x c_min, so the axis spans 0.5x to 3x s_L",
        },
        {
            "name": "capability",
            "levels": [[0.30, 0.60], [0.45, 0.90]],
            "meaning": "(cap_min, cap_max): capability c, linear across firms",
            "spacing_rule": "cap_min in {0.30, 0.45}; cap_max = 2 x cap_min "
            "(same max = 2 x min rule as capital)",
        },
        {
            "name": "shock_magnitude_mean",
            "levels": [0.10, 0.20, 0.30],
            "meaning": "shock.stochastic magnitude_mean; magnitude_sd = mean / 3",
        },
        {"name": "theta_limit", "levels": [0.55, 0.90], "meaning": "baseline theta_limit"},
    ],
    "cell_order": "itertools.product over the axes in the order listed; the last axis "
    "varies fastest; cell index = position in that product (0-based)",
    "n_cells": 72,
    "population_template_rule": {
        "ids": "0 .. n-1",
        "capital": "firm i gets round_half_up(c_min + i * (c_max - c_min) / (n - 1))",
        "capability": "firm i gets cap_min + i * (cap_max - cap_min) / (n - 1), rounded to 6 decimals",
        "every_other_per_firm_field": "copied from firm 0 of the base config "
        "(configs/experiments/phase2-stage5-smoke.json): input stock 40, legitimacy 1.0, "
        "aspiration_capital_growth 8.0; no obligation, no other aspiration",
    },
    "fixed_items_ASSUMPTIONS": {
        "base_config": "configs/experiments/phase2-stage5-smoke.json",
        "decision": "decision.satisficing with the base config's params (l_w = 4; all else plugin defaults)",
        "observation": "observation.full replaces the base config's observation.delayed "
        "(so decision-time h = true h)",
        "shock": "shock.stochastic replaces shock.scheduled: id rand-reg, channel "
        "regulatory/theta_limit, onset uniform 10..80, ramp instant, persistence permanent, "
        "observability full, novelty 0.5, targets all; magnitude per the grid",
        "horizon_ticks": 120,
        "seeds": {"mechanism": 910001, "environment": 910002, "init": 910004,
                  "shock": "910001..910020 (20 shocked runs per cell)"},
        "kept_from_base_config": {
            "theta_cap": 0.40, "theta_q": 100, "input_price": 2, "output_price": 3,
            "resource.patchy": {"baseline": 2, "reversion": 0.25, "sigma": 0.8, "floor": 1},
            "constraint.action_window": {"l_w": 4},
            "constraint.enforce": {"t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": 4},
            "decision.aspiration_update": {"alpha": 0.10},
            "market actions and resolve_lagged": "as in the base config, default params",
            "environment stocks": "as in the base config",
        },
    },
    "per_cell_procedure": {
        "baseline": "one NO-SHOCK run (shock rule removed, shock seed 910001); "
        "firma_lab.metrics.sanity_report gives SC-1..SC-6",
        "shocked": "20 shocked runs, shock seeds 910001..910020, each paired with its "
        "no-shock fork (same config and seeds, shock rule removed); matched firm-tick DeltaH "
        "per the R14 definition (scripts/exploratory_e1_slice.py::matched_summary)",
        "fork_note": "every fork is executed; a fork whose events.ndjson is byte-identical "
        "(sha256) to the cell's baseline reuses the baseline's measurement instead of being "
        "re-parsed; a fork that differs is measured on its own",
    },
    "targets": {
        "status": "PROPOSED by the reviewer (R16); fixed at step 0",
        "T1_baseline": {
            "SC-1": "sc1_survival in [0.60, 0.90]",
            "SC-2": "sc2_reconstructed_h_below_crit (share of live firm-ticks with h < h_crit) "
            "in [0.05, 0.25]",
            "SC-3": "sc3_pass from firma_analysis (each of the four constraints binds at least once)",
            "SC-6": "sc6_pass from firma_analysis (entropy variance >= SC6_MIN_VARIANCE)",
            "bounds": {"sc1": [0.60, 0.90], "sc2": [0.05, 0.25]},
            "rule": "T1 passes iff all four hold",
        },
        "T2_shocked": {
            "nonzero_fraction": "at least 0.50 of the 20 shocked runs have matched DeltaH != 0 (exactly)",
            "mean_survival": "mean over the 20 shocked runs of survivors / n lies in [0.40, 0.90]",
            "bounds": {"min_nonzero_fraction": 0.50, "mean_survival": [0.40, 0.90]},
            "rule": "T2 passes iff both hold",
        },
        "stressed_but_alive": "T1 and T2",
    },
    "reported_for_information_only_NOT_targets": [
        "SC-4, SC-5 (known unmet under satisficing)",
        "sc2_survival_attention and sc2_pass (the focus-based SC-2 that firma_analysis applies its own bounds to)",
        "sc3_binds per constraint [solvency, compliance, scope, obligation]",
        "whether T1 would hold with the obligation leg of SC-3 left out",
    ],
    "controls": {
        "a_determinism": "cell 0: baseline and the shocked run at shock seed 910001 are each run "
        "twice; the event logs must be byte-equal",
        "b_known_failure": "the R12 slice config (4-firm smoke population, observation.delayed, "
        "magnitude 0.3 / 0.1) evaluated with the same code over shock seeds 910001..910020 "
        "must fail the targets",
    },
    "declared_before_any_run_note": "Written before any run, from reading the code only: "
    "with the population rule above no firm starts with an obligation, and the only source of "
    "obligation is an applied 'contract' shaping effect (firma-domain effect.rs). SC-3 needs "
    "g_4 = q - theta_Q >= 0 with theta_Q = 100 at least once. Whether that can happen in this "
    "rule set is NOT established here. If it cannot, SC-3 and therefore T1 fail in every cell; "
    "the grid and targets are left exactly as the task specifies, and an information-only "
    "column reports T1 with the obligation leg left out.",
}


def grid_bytes() -> bytes:
    return (json.dumps(GRID, indent=2, sort_keys=True) + "\n").encode("utf-8")


def grid_sha256() -> str:
    return hashlib.sha256(grid_bytes()).hexdigest()


def cells() -> list[dict[str, Any]]:
    """Every cell, in the declared order (last axis fastest)."""
    axes = GRID["axes_in_declared_order"]
    out = []
    for i, combo in enumerate(itertools.product(*(a["levels"] for a in axes))):
        cell = {"cell": i}
        for a, v in zip(axes, combo):
            cell[a["name"]] = v
        out.append(cell)
    if len(out) != GRID["n_cells"] or len(out) > 72:
        raise AssertionError(f"declared grid has {len(out)} cells")
    return out


# ---------------------------------------------------------------------------
# Population and configs
# ---------------------------------------------------------------------------


def _round_half_up(x: float) -> int:
    return int(x + 0.5)


def population(n: int, capital: Sequence[float], capability: Sequence[float],
               template_firm: Mapping[str, Any]) -> list[dict[str, Any]]:
    """The declared population rule: ``n`` firms, capital and capability
    linear from min to max, every other per-firm field from ``template_firm``
    (firm 0 of the base config)."""
    if n < 2:
        raise ValueError("the linear spacing rule needs n >= 2")
    c_min, c_max = capital
    k_min, k_max = capability
    firms = []
    for i in range(n):
        firm = copy.deepcopy(dict(template_firm))
        firm["id"] = i
        firm["stocks"] = dict(firm["stocks"])
        firm["stocks"]["capital"] = _round_half_up(c_min + i * (c_max - c_min) / (n - 1))
        firm["reals"] = dict(firm["reals"])
        firm["reals"]["capability"] = round(k_min + i * (k_max - k_min) / (n - 1), 6)
        firms.append(firm)
    return firms


def build_cell_config(base: Mapping[str, Any], cell: Mapping[str, Any], *,
                      shock_seed: int, shocked: bool) -> dict[str, Any]:
    """The run config for one cell. ``shocked=False`` is the no-shock run
    (baseline and fork): the same config and seeds with the shock rule
    removed, nothing else."""
    cfg = copy.deepcopy(dict(base))
    seeds = {"mechanism": MECHANISM_SEED, "environment": ENVIRONMENT_SEED,
             "shock": shock_seed, "init": INIT_SEED}
    sl.check_seeds(seeds)
    cfg["seeds"] = seeds
    cfg["experiment"] = (
        f"DIAGNOSTIC-ONLY-R16-regime-cell{cell['cell']:02d}-{'shocked' if shocked else 'noshock'}"
    )
    cfg["world"]["ticks"] = TICKS
    cfg["world"]["snapshot_every"] = TICKS
    cfg["world"]["global_reals"]["theta_limit"] = cell["theta_limit"]
    cfg["agents"] = population(cell["n"], cell["capital"], cell["capability"], base["agents"][0])
    mean = cell["shock_magnitude_mean"]
    shock = copy.deepcopy(sl.STOCHASTIC_SHOCK)
    shock["params"]["magnitude_mean"] = mean
    shock["params"]["magnitude_sd"] = mean / 3.0
    rules, seen = [], {"shock.scheduled": 0, "observation.delayed": 0}
    for rule in cfg["rules"]:
        if rule["id"] == "shock.scheduled":
            seen["shock.scheduled"] += 1
            if shocked:
                rules.append(shock)
        elif rule["id"] == "observation.delayed":
            seen["observation.delayed"] += 1
            rules.append({"id": "observation.full", "version": "^1", "params": {}})
        else:
            rules.append(rule)
    if seen != {"shock.scheduled": 1, "observation.delayed": 1}:
        raise ValueError(f"base config rule set is not the one this task assumes: {seen}")
    cfg["rules"] = rules
    return cfg


# ---------------------------------------------------------------------------
# Targets (pure)
# ---------------------------------------------------------------------------


def evaluate_targets(baseline: Mapping[str, Any], shocked: Sequence[Mapping[str, Any]],
                     n_firms: int) -> dict[str, Any]:
    """T1 from the no-shock baseline's sanity report, T2 from the shocked
    runs (each with ``delta_h_rep_matched`` and ``survivors``). Bounds come
    from the declared grid; SC-3 and SC-6 pass/fail come from firma_analysis."""
    t = GRID["targets"]
    lo1, hi1 = t["T1_baseline"]["bounds"]["sc1"]
    lo2, hi2 = t["T1_baseline"]["bounds"]["sc2"]
    sc1 = baseline["sc1_survival"]
    sc2 = baseline["sc2_reconstructed_h_below_crit"]
    sc1_ok = lo1 <= sc1 <= hi1
    sc2_ok = lo2 <= sc2 <= hi2
    sc3_ok = bool(baseline["sc3_pass"])
    sc6_ok = bool(baseline["sc6_pass"])
    binds = list(baseline["sc3_binds"])
    t1 = sc1_ok and sc2_ok and sc3_ok and sc6_ok

    n = len(shocked)
    nonzero = sum(1 for r in shocked if r["delta_h_rep_matched"] != 0.0)
    frac = nonzero / n if n else float("nan")
    surv = sum(r["survivors"] / n_firms for r in shocked) / n if n else float("nan")
    b = t["T2_shocked"]["bounds"]
    frac_ok = n > 0 and frac >= b["min_nonzero_fraction"]
    surv_ok = n > 0 and b["mean_survival"][0] <= surv <= b["mean_survival"][1]
    t2 = frac_ok and surv_ok
    return {
        "sc1_survival": sc1, "sc1_ok": sc1_ok,
        "sc2_h_below_crit": sc2, "sc2_ok": sc2_ok,
        "sc3_binds": binds, "sc3_ok": sc3_ok,
        "sc6_entropy_variance": baseline["sc6_entropy_variance"], "sc6_ok": sc6_ok,
        "T1": t1,
        "nonzero_matched_fraction": frac, "nonzero_ok": frac_ok,
        "mean_survival_shocked": surv, "survival_ok": surv_ok,
        "T2": t2,
        "stressed_but_alive": t1 and t2,
        # information only, not targets
        "info_sc2_survival_attention": baseline["sc2_survival_attention"],
        "info_sc2_pass_focus_based": bool(baseline["sc2_pass"]),
        "info_sc1_pass_rust": bool(baseline["sc1_pass"]),
        "info_sc4_shaping_fraction": baseline["sc4_shaping_fraction"],
        "info_sc5_shaping_success": baseline["sc5_shaping_success"],
        "info_T1_without_obligation_leg": sc1_ok and sc2_ok and sc6_ok and all(binds[:3]),
    }


# ---------------------------------------------------------------------------
# One cell
# ---------------------------------------------------------------------------


def _events_sha(run_dir: Path) -> str:
    return hashlib.sha256((run_dir / "run" / "events.ndjson").read_bytes()).hexdigest()


def measure(run_dir: Path) -> dict[str, Any]:
    """What this task needs from one run, composed from the slice script's
    functions (``sl.measure_run`` also computes R12-only columns that are not
    needed here)."""
    events = load.read_events(run_dir / "run")
    by_agent, deaths = sl.decisions_and_deaths(events)
    entropies = sl.firm_tick_entropies(by_agent, deaths, sl.OUTCOME_L_W)
    firm_ticks = sl.rust_firm_ticks(run_dir)
    if set(firm_ticks) != set(entropies):
        raise ValueError(f"{run_dir}: live firm-ticks differ between the log reader "
                         f"({len(entropies)}) and firma_analysis::firm_ticks ({len(firm_ticks)})")
    onset, magnitude = sl.drawn_shock(events)
    return {
        "entropies": entropies,
        "firm_ticks": firm_ticks,
        "deaths": len(deaths),
        "survivors": len(by_agent) - len(deaths),
        "action_sha": sl.action_sha(by_agent),
        "drawn_onset": onset,
        "drawn_magnitude": magnitude,
    }


def evaluate_cell(label: str, n_firms: int,
                  builder: Callable[[int, bool], dict[str, Any]], work: Path, firma: Path,
                  shock_seeds: Sequence[int]) -> dict[str, Any]:
    """Baseline + shocked runs + forks for one configuration family.
    ``builder(shock_seed, shocked)`` returns the run config. Run directories
    are deleted as soon as they are measured."""
    cell_dir = work / label
    shutil.rmtree(cell_dir, ignore_errors=True)
    failures: list[str] = []

    base_dir = cell_dir / "baseline"
    err = sl.run_config(builder(shock_seeds[0], False), base_dir, firma)
    if err:
        return {"label": label, "failed": [f"baseline: {err}"]}
    baseline_report = metrics.sanity_report(base_dir / "run")
    baseline_meas = measure(base_dir)
    baseline_sha = _events_sha(base_dir)

    shocked_rows, forks_identical = [], 0
    for seed in shock_seeds:
        s_dir, f_dir = cell_dir / f"s{seed}", cell_dir / f"f{seed}"
        err = sl.run_config(builder(seed, True), s_dir, firma)
        if err:
            failures.append(f"shocked {seed}: {err}")
            continue
        err = sl.run_config(builder(seed, False), f_dir, firma)
        if err:
            failures.append(f"fork {seed}: {err}")
            shutil.rmtree(s_dir, ignore_errors=True)
            continue
        if _events_sha(f_dir) == baseline_sha:
            fork = baseline_meas
            forks_identical += 1
        else:
            fork = measure(f_dir)
        s = measure(s_dir)
        m = sl.matched_summary(s["entropies"], fork["entropies"], s["firm_ticks"])
        shocked_rows.append({
            "shock_seed": seed,
            "delta_h_rep_matched": m["delta_h_rep_matched"],
            "n_matched_firm_ticks": m["n_matched_firm_ticks"],
            "survivors": n_firms - s["deaths"],
            "deaths": s["deaths"],
            "action_sha": s["action_sha"],
            "drawn_onset": s["drawn_onset"],
            "drawn_magnitude": s["drawn_magnitude"],
        })
        shutil.rmtree(s_dir, ignore_errors=True)
        shutil.rmtree(f_dir, ignore_errors=True)
    shutil.rmtree(cell_dir, ignore_errors=True)

    out = {
        "label": label,
        "failed": failures,
        "n_firms": n_firms,
        "n_shocked_runs": len(shocked_rows),
        "forks_identical_to_baseline": forks_identical,
        "baseline_survivors": baseline_report["survivors"],
        "baseline_survivors_from_log": baseline_meas["survivors"],
        "baseline_decisions": baseline_report["decisions"],
        "distinct_action_sequences": len({r["action_sha"] for r in shocked_rows}),
        "shocked_runs": shocked_rows,
    }
    out.update(evaluate_targets(baseline_report, shocked_rows, n_firms))
    return out


def _cell_worker(args: tuple[dict[str, Any], str, str]) -> dict[str, Any]:
    cell, work, firma = args
    base = json.loads(BASE_CONFIG.read_text())
    res = evaluate_cell(
        f"cell{cell['cell']:02d}", cell["n"],
        lambda seed, shocked: build_cell_config(base, cell, shock_seed=seed, shocked=shocked),
        Path(work), Path(firma), SHOCK_SEEDS,
    )
    res["cell_params"] = cell
    return res


# ---------------------------------------------------------------------------
# Controls
# ---------------------------------------------------------------------------


def control_determinism(work: Path, firma: Path) -> list[str]:
    """(a) cell 0: baseline and shocked(910001) each run twice, logs byte-equal."""
    base = json.loads(BASE_CONFIG.read_text())
    cell = cells()[0]
    lines = []
    for shocked in (False, True):
        shas = []
        for rep in (1, 2):
            d = work / "control_a" / f"{'shocked' if shocked else 'baseline'}_{rep}"
            err = sl.run_config(build_cell_config(base, cell, shock_seed=SHOCK_SEEDS[0],
                                                  shocked=shocked), d, firma)
            if err:
                raise RuntimeError(f"control (a) run failed: {err}")
            shas.append(_events_sha(d))
        lines.append(
            f"  cell 0 {'shocked (shock seed 910001)' if shocked else 'baseline'}: "
            f"{'BYTE-EQUAL' if shas[0] == shas[1] else 'DIFFERENT'}  {shas[0]}  {shas[1]}"
        )
    shutil.rmtree(work / "control_a", ignore_errors=True)
    return lines


def control_known_failure(work: Path, firma: Path) -> dict[str, Any]:
    """(b) the R12 slice config, evaluated with the same code."""
    base = json.loads(BASE_CONFIG.read_text())
    return evaluate_cell(
        "control_b_r12_smoke", len(base["agents"]),
        lambda seed, shocked: sl.build_config(base, decision="satisficing",
                                              shock_seed=seed, shocked=shocked),
        work, firma, SHOCK_SEEDS,
    )


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

CSV_FIELDS = [
    "cell", "n", "capital", "capability", "shock_magnitude_mean", "theta_limit",
    "sc1_survival", "sc1_ok", "sc2_h_below_crit", "sc2_ok", "sc3_binds", "sc3_ok",
    "sc6_entropy_variance", "sc6_ok", "T1",
    "nonzero_matched_fraction", "nonzero_ok", "mean_survival_shocked", "survival_ok", "T2",
    "stressed_but_alive", "distinct_action_sequences", "n_shocked_runs",
    "forks_identical_to_baseline", "baseline_survivors", "n_failed_runs",
    "info_sc2_survival_attention", "info_sc2_pass_focus_based", "info_sc4_shaping_fraction",
    "info_sc5_shaping_success", "info_T1_without_obligation_leg",
]


def _yn(b: bool) -> str:
    return "PASS" if b else "fail"


def row_for_csv(res: Mapping[str, Any]) -> dict[str, Any]:
    p = res["cell_params"]
    row = {k: res.get(k) for k in CSV_FIELDS}
    row.update({"cell": p["cell"], "n": p["n"], "capital": json.dumps(p["capital"]),
                "capability": json.dumps(p["capability"]),
                "shock_magnitude_mean": p["shock_magnitude_mean"], "theta_limit": p["theta_limit"],
                "sc3_binds": json.dumps(res["sc3_binds"]), "n_failed_runs": len(res["failed"])})
    return row


def table_line(res: Mapping[str, Any]) -> str:
    p = res["cell_params"]
    b = "".join("Y" if x else "n" for x in res["sc3_binds"])
    return (
        f"{p['cell']:>3} {p['n']:>2} {str(p['capital']):<10} {str(p['capability']):<12} "
        f"{p['shock_magnitude_mean']:<4} {p['theta_limit']:<4} | "
        f"{res['sc1_survival']:.3f} {_yn(res['sc1_ok'])} | {res['sc2_h_below_crit']:.3f} "
        f"{_yn(res['sc2_ok'])} | {b} {_yn(res['sc3_ok'])} | {res['sc6_entropy_variance']:.4f} "
        f"{_yn(res['sc6_ok'])} | T1 {_yn(res['T1'])} || {res['nonzero_matched_fraction']:.2f} "
        f"{_yn(res['nonzero_ok'])} | {res['mean_survival_shocked']:.3f} {_yn(res['survival_ok'])} "
        f"| T2 {_yn(res['T2'])} || seqs {res['distinct_action_sequences']:>2} | "
        f"{'BOTH' if res['stressed_but_alive'] else '-'}"
    )


# ---------------------------------------------------------------------------
# GRID 2 (review R18, task REGIME-2): two stages. Declared at step 0 in
# grid2.json. Grid 1 above is unchanged and stays runnable.
# ---------------------------------------------------------------------------

GRID2_N = 12
GRID2_SHOCK_MEANS = [0.10, 0.20, 0.30]
GRID2_STAGE2_MAX_CELLS = 40
GRID2_CONTROL_B_GRID1_CELLS = [0, 36, 71]

GRID2: dict[str, Any] = {
    "task": "R18 REGIME-2 - DIAGNOSTIC-ONLY regime search, grid 2, two stages; not registered; "
    "not for tuning",
    "question": "does any cell give a stressed-but-alive population?",
    "disclosure": "Grid 1 (72 cells, grid.json sha256 1371026b859b2da07e87ad5186fed47c9063ec96d6cf214"
    "d866670577701963a) passed 0 cells. The axes of grid 2 were chosen by the reviewer AFTER "
    "seeing grid 1's results (R17). Grid 1 stays in the record unchanged.",
    "axes_in_declared_order": [
        {"name": "n_type_L", "levels": [0, 1, 2, 3],
         "meaning": "number of low-capability firms = round_half_up(fraction x 12) for fractions "
         "0, 1/12, 2/12, 3/12"},
        {"name": "n_type_T", "levels": [0, 2, 4],
         "meaning": "number of thin-capital firms = round_half_up(fraction x 12) for fractions "
         "0, 2/12, 4/12"},
        {"name": "output_price", "levels": [2, 3, 5],
         "meaning": "global int output_price; input_price fixed at 2 (ratio 1.0, 1.5, 2.5)"},
        {"name": "obligation_seeding", "levels": ["none", "seeded"],
         "meaning": "none: theta_q = 100 (base config) and no firm has an obligation; seeded: "
         "theta_q = 40 and the first two type-O firms start with obligation q = 60"},
        {"name": "theta_limit", "levels": [0.60, 0.70, 0.80, 0.90],
         "meaning": "baseline theta_limit"},
    ],
    "cell_order": "itertools.product over the axes in the order listed; the last axis varies "
    "fastest; cell index = position in that product (0-based)",
    "n_cells": 288,
    "population_rule": {
        "n": 12,
        "assignment_by_firm_id": "ids 0 .. n_L-1 are type L; the next n_T ids are type T; the "
        "remaining 12 - n_L - n_T ids are type O (never fewer than 5)",
        "type_L": {"capability": [0.25, 0.35], "capital": [200, 300]},
        "type_T": {"capability": [0.50, 0.70], "capital": [8, 20]},
        "type_O": {"capability": [0.50, 0.80], "capital": [150, 300]},
        "spacing_within_a_type": "with k firms of a type, the j-th (j = 0 .. k-1, in id order) "
        "gets min + j * (max - min) / (k - 1); with k = 1 it gets the midpoint (min + max) / 2. "
        "Capital is rounded half up to an integer; capability is rounded to 6 decimals",
        "every_other_per_firm_field": "copied from firm 0 of the base config, as in grid 1 "
        "(input stock 40, legitimacy 1.0, aspiration_capital_growth 8.0)",
        "obligation_seeded_firms": "the two lowest-id type-O firms get ints.obligation = 60",
    },
    "fixed_items_ASSUMPTIONS": {
        "as_grid_1": "base config configs/experiments/phase2-stage5-smoke.json and its rule set; "
        "decision.satisficing (l_w = 4); observation.full in place of observation.delayed; "
        "shock.stochastic (regulatory/theta_limit, onset uniform 10..80, instant, permanent, "
        "sd = mean / 3); horizon 120; theta_cap 0.40; input_price 2; all other parameters as "
        "listed under kept_from_base_config in grid.json",
        "seeds": {"mechanism": 910001, "environment": 910002, "init": 910004,
                  "shock": "910001 for Stage 1 baselines; 910001..910020 in Stage 2"},
    },
    "stage_1": {
        "runs": "one NO-SHOCK run per cell (shock rule removed, shock seed 910001)",
        "evaluates": "T1 only",
        "reported": "all 288 rows in declared order; SC-4 and SC-5 for information only",
    },
    "stage_2": {
        "cells": "every cell that passes T1; if more than 40 pass, the first 40 in declared order",
        "shock_magnitude_means": [0.10, 0.20, 0.30],
        "runs": "per cell and magnitude: 20 shocked runs (shock seeds 910001..910020), each with "
        "its no-shock fork; matched firm-tick DeltaH per the R14 definition",
        "evaluates": "T2, separately for each (cell, magnitude) row",
        "passing_both": "a cell passes both iff it passes T1 and at least one of its three "
        "magnitude rows passes T2; all three rows are reported",
    },
    "targets": {
        "status": "fixed, identical to grid.json (R16)",
        "T1_baseline": {
            "SC-1": "sc1_survival in [0.60, 0.90]",
            "SC-2": "sc2_reconstructed_h_below_crit in [0.05, 0.25]",
            "SC-3": "sc3_pass from firma_analysis (each of the four constraints binds at least once)",
            "SC-6": "sc6_pass from firma_analysis (entropy variance >= SC6_MIN_VARIANCE)",
            "bounds": {"sc1": [0.60, 0.90], "sc2": [0.05, 0.25]},
        },
        "T2_shocked": {
            "nonzero_fraction": "at least 0.50 of the 20 shocked runs have matched DeltaH != 0",
            "mean_survival": "mean over the 20 shocked runs of survivors / 12 in [0.40, 0.90]",
            "bounds": {"min_nonzero_fraction": 0.50, "mean_survival": [0.40, 0.90]},
        },
    },
    "controls": {
        "a_determinism": "grid-2 cell 0 baseline run twice; event logs byte-equal",
        "b_unchanged_behaviour": "the R12 smoke config and grid-1 cells 0, 36 and 71 (first, "
        "first of the n = 12 half, last), baseline only, must reproduce the SC-1, SC-2, SC-3 "
        "and SC-6 values recorded in evidence/R16 exactly",
        "c_each_sc_fail_and_pass": "for each of SC-1, SC-2, SC-3, SC-6: the first Stage-1 cell "
        "in declared order where it fails and the first where it passes, if any, with raw numbers",
    },
    "choices_made_by_the_builder_before_any_run": [
        "the order in which types are assigned to ids (L, then T, then O)",
        "the midpoint rule for a type with exactly one firm",
        "which two type-O firms are seeded (the two lowest ids)",
        "theta_q = 100 in the 'none' level (the base config's value)",
        "the three grid-1 cells used in control (b)",
        "a cell 'passes both' if any one of its three magnitude rows passes T2",
    ],
}

# The targets are the same numbers as grid 1's; evaluate_targets reads grid 1's.
assert GRID2["targets"]["T1_baseline"]["bounds"] == GRID["targets"]["T1_baseline"]["bounds"]
assert GRID2["targets"]["T2_shocked"]["bounds"] == GRID["targets"]["T2_shocked"]["bounds"]


def grid2_bytes() -> bytes:
    return (json.dumps(GRID2, indent=2, sort_keys=True) + "\n").encode("utf-8")


def grid2_sha256() -> str:
    return hashlib.sha256(grid2_bytes()).hexdigest()


def cells2() -> list[dict[str, Any]]:
    """Every grid-2 cell, in the declared order (last axis fastest)."""
    axes = GRID2["axes_in_declared_order"]
    out = []
    for i, combo in enumerate(itertools.product(*(a["levels"] for a in axes))):
        cell = {"cell": i}
        for a, v in zip(axes, combo):
            cell[a["name"]] = v
        out.append(cell)
    if len(out) != GRID2["n_cells"]:
        raise AssertionError(f"declared grid 2 has {len(out)} cells")
    return out


def _spaced(lo: float, hi: float, k: int) -> list[float]:
    """The declared within-type spacing: linear for k >= 2, midpoint for k = 1."""
    if k == 1:
        return [(lo + hi) / 2]
    return [lo + j * (hi - lo) / (k - 1) for j in range(k)]


def population2(n_l: int, n_t: int, seeded: bool,
                template_firm: Mapping[str, Any]) -> list[dict[str, Any]]:
    """The declared three-type population (12 firms). Each firm also carries
    a ``_type`` key, which ``build_cell2_config`` strips before the config is
    written."""
    rule = GRID2["population_rule"]
    n_o = GRID2_N - n_l - n_t
    if n_o < 2 or min(n_l, n_t) < 0:
        raise ValueError(f"invalid type counts L={n_l} T={n_t} O={n_o}")
    firms: list[dict[str, Any]] = []
    for name, k in (("L", n_l), ("T", n_t), ("O", n_o)):
        spec = rule[f"type_{name}"]
        caps = _spaced(*spec["capability"], k) if k else []
        capital = _spaced(*spec["capital"], k) if k else []
        for j in range(k):
            firm = copy.deepcopy(dict(template_firm))
            firm["id"] = len(firms)
            firm["stocks"] = dict(firm["stocks"])
            firm["stocks"]["capital"] = _round_half_up(capital[j])
            firm["reals"] = dict(firm["reals"])
            firm["reals"]["capability"] = round(caps[j], 6)
            firm["_type"] = name
            if seeded and name == "O" and j < 2:
                firm["ints"] = {"obligation": 60}
            firms.append(firm)
    return firms


def build_cell2_config(base: Mapping[str, Any], cell: Mapping[str, Any], *, shock_seed: int,
                       shocked: bool, shock_mean: float | None = None) -> dict[str, Any]:
    """The run config for one grid-2 cell. ``shocked=False`` is the no-shock
    run (Stage-1 baseline and Stage-2 fork)."""
    if shocked and shock_mean is None:
        raise ValueError("a shocked grid-2 run needs a shock magnitude mean")
    cfg = copy.deepcopy(dict(base))
    seeds = {"mechanism": MECHANISM_SEED, "environment": ENVIRONMENT_SEED,
             "shock": shock_seed, "init": INIT_SEED}
    sl.check_seeds(seeds)
    cfg["seeds"] = seeds
    cfg["experiment"] = (
        f"DIAGNOSTIC-ONLY-R18-regime2-cell{cell['cell']:03d}-{'shocked' if shocked else 'noshock'}"
    )
    cfg["world"]["ticks"] = TICKS
    cfg["world"]["snapshot_every"] = TICKS
    cfg["world"]["global_reals"]["theta_limit"] = cell["theta_limit"]
    cfg["world"]["global_ints"]["output_price"] = cell["output_price"]
    seeded = cell["obligation_seeding"] == "seeded"
    if seeded:
        cfg["world"]["global_ints"]["theta_q"] = 40
    firms = population2(cell["n_type_L"], cell["n_type_T"], seeded, base["agents"][0])
    for f in firms:
        del f["_type"]
    cfg["agents"] = firms
    shock = copy.deepcopy(sl.STOCHASTIC_SHOCK)
    if shocked:
        shock["params"]["magnitude_mean"] = shock_mean
        shock["params"]["magnitude_sd"] = shock_mean / 3.0
    rules, seen = [], {"shock.scheduled": 0, "observation.delayed": 0}
    for rule in cfg["rules"]:
        if rule["id"] == "shock.scheduled":
            seen["shock.scheduled"] += 1
            if shocked:
                rules.append(shock)
        elif rule["id"] == "observation.delayed":
            seen["observation.delayed"] += 1
            rules.append({"id": "observation.full", "version": "^1", "params": {}})
        else:
            rules.append(rule)
    if seen != {"shock.scheduled": 1, "observation.delayed": 1}:
        raise ValueError(f"base config rule set is not the one this task assumes: {seen}")
    cfg["rules"] = rules
    return cfg


def evaluate_baseline(label: str, n_firms: int, cfg: Mapping[str, Any], work: Path,
                      firma: Path) -> dict[str, Any]:
    """One no-shock run and its T1 verdict (no shocked runs, so T2 is not
    evaluated and is reported as absent)."""
    run_dir = work / label
    shutil.rmtree(run_dir, ignore_errors=True)
    err = sl.run_config(cfg, run_dir, firma)
    if err:
        return {"label": label, "failed": [f"baseline: {err}"]}
    report = metrics.sanity_report(run_dir / "run")
    sha = _events_sha(run_dir)
    shutil.rmtree(run_dir, ignore_errors=True)
    t = evaluate_targets(report, [], n_firms)
    out = {"label": label, "failed": [], "n_firms": n_firms, "events_sha256": sha,
           "baseline_survivors": report["survivors"],
           "sc3_first_bind_tick": list(report["sc3_first_bind_tick"])}
    out.update({k: v for k, v in t.items()
                if k.startswith(("sc", "info_")) or k == "T1"})
    return out


def _stage1_worker(args: tuple[dict[str, Any], str, str]) -> dict[str, Any]:
    cell, work, firma = args
    base = json.loads(BASE_CONFIG.read_text())
    res = evaluate_baseline(
        f"g2cell{cell['cell']:03d}", GRID2_N,
        build_cell2_config(base, cell, shock_seed=SHOCK_SEEDS[0], shocked=False),
        Path(work), Path(firma))
    res["cell_params"] = cell
    return res


def _stage2_worker(args: tuple[dict[str, Any], float, str, str]) -> dict[str, Any]:
    cell, mean, work, firma = args
    base = json.loads(BASE_CONFIG.read_text())
    res = evaluate_cell(
        f"g2cell{cell['cell']:03d}_m{int(round(mean * 100)):02d}", GRID2_N,
        lambda seed, shocked: build_cell2_config(base, cell, shock_seed=seed, shocked=shocked,
                                                 shock_mean=mean),
        Path(work), Path(firma), SHOCK_SEEDS)
    res["cell_params"] = cell
    res["shock_magnitude_mean"] = mean
    return res


T1_KEYS = ("sc1_survival", "sc2_h_below_crit", "sc3_binds", "sc6_entropy_variance", "T1")


def control2_unchanged_behaviour(work: Path, firma: Path, r16_dir: Path) -> list[dict[str, Any]]:
    """(b) the R12 smoke config and three grid-1 cells, baseline only, against
    the values recorded in evidence/R16."""
    base = json.loads(BASE_CONFIG.read_text())
    r16_cells = {c["cell_params"]["cell"]: c
                 for c in json.loads((r16_dir / "cells_full.json").read_text())}
    r16_smoke = json.loads((r16_dir / "control_b.json").read_text())
    jobs = [("R12 smoke config", len(base["agents"]),
             sl.build_config(base, decision="satisficing", shock_seed=SHOCK_SEEDS[0],
                             shocked=False), r16_smoke)]
    g1 = cells()
    for i in GRID2_CONTROL_B_GRID1_CELLS:
        jobs.append((f"grid-1 cell {i}", g1[i]["n"],
                     build_cell_config(base, g1[i], shock_seed=SHOCK_SEEDS[0], shocked=False),
                     r16_cells[i]))
    out = []
    for name, n, cfg, recorded in jobs:
        now = evaluate_baseline("control_b", n, cfg, work, firma)
        out.append({"name": name,
                    "now": {k: now[k] for k in T1_KEYS},
                    "r16": {k: recorded[k] for k in T1_KEYS},
                    "equal": all(now[k] == recorded[k] for k in T1_KEYS)})
    return out


STAGE1_FIELDS = [
    "cell", "n_type_L", "n_type_T", "output_price", "obligation_seeding", "theta_limit",
    "sc1_survival", "sc1_ok", "sc2_h_below_crit", "sc2_ok", "sc3_binds", "sc3_ok",
    "sc6_entropy_variance", "sc6_ok", "T1", "baseline_survivors", "info_sc4_shaping_fraction",
    "info_sc5_shaping_success", "info_sc2_survival_attention", "events_sha256",
]
STAGE2_FIELDS = [
    "cell", "n_type_L", "n_type_T", "output_price", "obligation_seeding", "theta_limit",
    "shock_magnitude_mean", "nonzero_matched_fraction", "nonzero_ok", "mean_survival_shocked",
    "survival_ok", "T2", "distinct_action_sequences", "n_shocked_runs",
    "forks_identical_to_baseline", "n_failed_runs",
]
_AXES2 = ("n_type_L", "n_type_T", "output_price", "obligation_seeding", "theta_limit")


def _flat2(res: Mapping[str, Any], fields: Sequence[str]) -> dict[str, Any]:
    row = {k: res.get(k) for k in fields}
    row["cell"] = res["cell_params"]["cell"]
    for a in _AXES2:
        row[a] = res["cell_params"][a]
    if "sc3_binds" in fields:
        row["sc3_binds"] = json.dumps(res["sc3_binds"])
    if "n_failed_runs" in fields:
        row["n_failed_runs"] = len(res["failed"])
    return row


def _num(x: float | None) -> str:
    """SC-5 is undefined (None) when no shaping action was committed."""
    return "n/a" if x is None else f"{x:.3f}"


def stage1_line(res: Mapping[str, Any]) -> str:
    p = res["cell_params"]
    b = "".join("Y" if x else "n" for x in res["sc3_binds"])
    return (
        f"{p['cell']:>3} L{p['n_type_L']} T{p['n_type_T']} price {p['output_price']} "
        f"{p['obligation_seeding']:<6} thL {p['theta_limit']:.2f} | {res['sc1_survival']:.3f} "
        f"{_yn(res['sc1_ok'])} | {res['sc2_h_below_crit']:.3f} {_yn(res['sc2_ok'])} | {b} "
        f"{_yn(res['sc3_ok'])} | {res['sc6_entropy_variance']:.4f} {_yn(res['sc6_ok'])} | "
        f"T1 {_yn(res['T1'])} | SC-4 {_num(res['info_sc4_shaping_fraction'])} "
        f"SC-5 {_num(res['info_sc5_shaping_success'])}"
    )


def stage2_line(res: Mapping[str, Any]) -> str:
    p = res["cell_params"]
    return (
        f"{p['cell']:>3} L{p['n_type_L']} T{p['n_type_T']} price {p['output_price']} "
        f"{p['obligation_seeding']:<6} thL {p['theta_limit']:.2f} mag {res['shock_magnitude_mean']:.2f} "
        f"| nonzero dH {res['nonzero_matched_fraction']:.2f} {_yn(res['nonzero_ok'])} | mean surv "
        f"{res['mean_survival_shocked']:.3f} {_yn(res['survival_ok'])} | T2 {_yn(res['T2'])} | "
        f"seqs {res['distinct_action_sequences']:>2}"
    )


def main_grid2(args: argparse.Namespace) -> int:
    on_disk = Path(args.grid2).read_bytes()
    if on_disk != grid2_bytes():
        print("REFUSED: the grid-2 file differs from the grid this script declares "
              f"(file sha256 {hashlib.sha256(on_disk).hexdigest()}, script {grid2_sha256()})")
        return 1
    print(f"grid 2: {args.grid2}  sha256 {grid2_sha256()}  ({len(cells2())} cells, declared order)")
    print(f"DISCLOSURE: {GRID2['disclosure']}")
    started = time.time()
    firma = sl.find_firma(args.firma)
    out = Path(args.out) if args.out else Path(tempfile.mkdtemp(prefix="firma-regime-search2-"))
    work = out / "runs"
    work.mkdir(parents=True, exist_ok=True)
    print(f"firma: {firma}; output: {out}")
    base = json.loads(BASE_CONFIG.read_text())
    jobs = max(1, args.jobs)

    print("\n== control (a): determinism (grid-2 cell 0 baseline, run twice) ==")
    c0 = build_cell2_config(base, cells2()[0], shock_seed=SHOCK_SEEDS[0], shocked=False)
    shas = [evaluate_baseline(f"control_a_{i}", GRID2_N, c0, work, firma)["events_sha256"]
            for i in (1, 2)]
    print(f"  {'BYTE-EQUAL' if shas[0] == shas[1] else 'DIFFERENT'}  {shas[0]}  {shas[1]}")

    print("\n== control (b): R12 smoke config and grid-1 cells reproduce their R16 T1 numbers ==")
    cb = control2_unchanged_behaviour(work, firma, Path(args.r16_evidence))
    for c in cb:
        print(f"  {c['name']}: {'EQUAL' if c['equal'] else 'DIFFERENT'}  now {c['now']}  R16 {c['r16']}")

    print("\n== STAGE 1: one no-shock baseline per cell, T1 only, declared order ==")
    print("cell types price obligation theta_limit | SC-1 | SC-2 h<h_crit | SC-3 [sol,cmp,scp,obl] "
          "| SC-6 var | T1 | information only")
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        s1 = list(pool.map(_stage1_worker, [(c, str(work), str(firma)) for c in cells2()]))
    ok1 = [r for r in s1 if "T1" in r]
    for r in s1:
        print(stage1_line(r) if "T1" in r else f"{r['label']}: FAILED {r['failed']}")
    passing = [r for r in ok1 if r["T1"]]
    print("\n== Stage 1 summary ==")
    print(f"cells evaluated: {len(ok1)} of {len(s1)}; cells passing T1: {len(passing)}")
    rest = [r for r in ok1 if not r["T1"]]
    for key, label in (("sc1_ok", "SC-1"), ("sc2_ok", "SC-2"), ("sc3_ok", "SC-3"),
                       ("sc6_ok", "SC-6")):
        print(f"  {label}: fails in {sum(not r[key] for r in rest)} of the {len(rest)} cells "
              f"that fail T1 (passes in {sum(bool(r[key]) for r in ok1)} of all {len(ok1)})")
    names = ("solvency", "compliance", "scope", "obligation")
    for i, nm in enumerate(names):
        print(f"  SC-3 {nm} binds in {sum(bool(r['sc3_binds'][i]) for r in ok1)} of {len(ok1)} cells")

    print("\n== control (c): first Stage-1 cell where each SC fails / passes (raw numbers) ==")
    for key, label in (("sc1_ok", "SC-1"), ("sc2_ok", "SC-2"), ("sc3_ok", "SC-3"),
                       ("sc6_ok", "SC-6")):
        for want, word in ((False, "fails "), (True, "passes")):
            hit = next((r for r in ok1 if bool(r[key]) is want), None)
            print(f"  {label} {word}: " + (stage1_line(hit) if hit else "no such cell"))

    print("\n== STAGE 2: shocked runs for every T1-passing cell ==")
    chosen = passing[:GRID2_STAGE2_MAX_CELLS]
    if len(passing) > GRID2_STAGE2_MAX_CELLS:
        print(f"  {len(passing)} cells pass T1; Stage 2 runs on the first "
              f"{GRID2_STAGE2_MAX_CELLS} in declared order")
    s2: list[dict[str, Any]] = []
    if not chosen:
        print("  no cell passes T1; Stage 2 has nothing to run")
    else:
        work2 = [(r["cell_params"], m, str(work), str(firma))
                 for r in chosen for m in GRID2_SHOCK_MEANS]
        with ProcessPoolExecutor(max_workers=jobs) as pool:
            s2 = list(pool.map(_stage2_worker, work2))
        for r in s2:
            print(stage2_line(r) if "T2" in r else f"{r['label']}: FAILED {r['failed']}")
    ok2 = [r for r in s2 if "T2" in r]
    cells_t2 = sorted({r["cell_params"]["cell"] for r in ok2 if r["T2"]})
    n_failed = sum(len(r["failed"]) for r in s1) + sum(len(r["failed"]) for r in s2)
    print("\n== summary ==")
    print(f"cells passing T1: {len(passing)} of {len(ok1)}")
    print(f"cells passing T2 given T1 (any magnitude): {len(cells_t2)} of {len(chosen)} run in Stage 2")
    print(f"Stage-2 rows passing T2: {sum(bool(r['T2']) for r in ok2)} of {len(ok2)}")
    print(f"ANSWER: {len(cells_t2)} of {len(ok1)} cells are stressed-but-alive under the declared "
          f"targets{' (cells ' + str(cells_t2) + ')' if cells_t2 else ''}")
    print(f"failed runs: {n_failed}")

    for name, rows, fields in (("stage1.csv", ok1, STAGE1_FIELDS), ("stage2.csv", ok2, STAGE2_FIELDS)):
        with open(out / name, "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=fields)
            w.writeheader()
            for r in rows:
                w.writerow(_flat2(r, fields))
    (out / "stage1_full.json").write_text(json.dumps(s1, indent=1))
    (out / "stage2_full.json").write_text(json.dumps(s2, indent=1))
    (out / "controls.json").write_text(json.dumps(
        {"a_events_sha256": shas, "b": cb}, indent=1))
    print(f"\nwritten under {out}: stage1.csv, stage1_full.json, stage2.csv, stage2_full.json, "
          "controls.json")
    print(f"wall-clock time: {time.time() - started:.1f} s")
    print(BANNER)
    controls_ok = shas[0] == shas[1] and all(c["equal"] for c in cb)
    return 0 if n_failed == 0 and len(ok1) == len(s1) and controls_ok else 1


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=BANNER)
    ap.add_argument("--write-grid", help="STEP 0: write the declared grid to this path and exit")
    ap.add_argument("--grid", help="path of the grid.json written at step 0 (required to run)")
    ap.add_argument("--out", help="output directory (default: a temp dir)")
    ap.add_argument("--firma", help="path to the firma binary")
    ap.add_argument("--jobs", type=int, default=6, help="cells evaluated in parallel")
    ap.add_argument("--write-grid2", help="GRID 2 STEP 0: write grid2.json to this path and exit")
    ap.add_argument("--grid2", help="run grid 2 (two stages) against this step-0 grid2.json")
    ap.add_argument("--r16-evidence", default=str(REPO_ROOT / "docs/review/evidence/R16"),
                    help="grid 2 control (b): directory holding R16's cells_full.json")
    args = ap.parse_args(argv)

    print(BANNER)
    if args.write_grid2:
        path = Path(args.write_grid2)
        if path.exists():
            print(f"REFUSED: {path} already exists; the grid is declared once")
            return 1
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(grid2_bytes())
        print(f"step 0: wrote {path} ({len(cells2())} cells); sha256 {grid2_sha256()}")
        return 0
    if args.grid2:
        return main_grid2(args)
    if args.write_grid:
        path = Path(args.write_grid)
        if path.exists():
            print(f"REFUSED: {path} already exists; the grid is declared once")
            return 1
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(grid_bytes())
        print(f"step 0: wrote {path} ({len(cells())} cells); sha256 {grid_sha256()}")
        return 0

    if not args.grid:
        print("REFUSED: --grid <grid.json written at step 0> is required")
        return 1
    on_disk = Path(args.grid).read_bytes()
    if on_disk != grid_bytes():
        print("REFUSED: the grid file differs from the grid this script declares "
              f"(file sha256 {hashlib.sha256(on_disk).hexdigest()}, script {grid_sha256()})")
        return 1
    print(f"grid: {args.grid}  sha256 {grid_sha256()}  ({len(cells())} cells, declared order)")

    started = time.time()
    firma = sl.find_firma(args.firma)
    out = Path(args.out) if args.out else Path(tempfile.mkdtemp(prefix="firma-regime-search-"))
    work = out / "runs"
    work.mkdir(parents=True, exist_ok=True)
    print(f"firma: {firma}; output: {out}; shock seeds {SHOCK_SEEDS[0]}..{SHOCK_SEEDS[-1]}")

    print("\n== control (a): determinism ==")
    for line in control_determinism(work, firma):
        print(line)

    print("\n== control (b): the R12 slice config under the same targets (expected: fails) ==")
    cb = control_known_failure(work, firma)
    print(
        f"  SC-1 {cb['sc1_survival']:.3f} {_yn(cb['sc1_ok'])}; SC-2 {cb['sc2_h_below_crit']:.3f} "
        f"{_yn(cb['sc2_ok'])}; SC-3 binds {cb['sc3_binds']} {_yn(cb['sc3_ok'])}; SC-6 "
        f"{cb['sc6_entropy_variance']:.4f} {_yn(cb['sc6_ok'])}; T1 {_yn(cb['T1'])}"
    )
    print(
        f"  non-zero matched DeltaH in {cb['nonzero_matched_fraction']:.2f} of "
        f"{cb['n_shocked_runs']} runs {_yn(cb['nonzero_ok'])}; mean survival "
        f"{cb['mean_survival_shocked']:.3f} {_yn(cb['survival_ok'])}; T2 {_yn(cb['T2'])}; "
        f"stressed-but-alive: {cb['stressed_but_alive']}"
    )

    jobs = [(c, str(work), str(firma)) for c in cells()]
    with ProcessPoolExecutor(max_workers=max(1, args.jobs)) as pool:
        results = list(pool.map(_cell_worker, jobs))  # map preserves the declared order

    print("\n== every cell, in the declared order ==")
    print("cell n capital    capability   mag  thL  | SC-1        | SC-2 h<h_crit | SC-3 [sol,cmp,scp,obl] "
          "| SC-6 var     | T1      || nonzero dH | mean surv   | T2      || distinct | both")
    for res in results:
        if "T1" not in res:
            print(f"{res['label']}: FAILED {res['failed']}")
        else:
            print(table_line(res))

    ok = [r for r in results if "T1" in r]
    n_fail_runs = sum(len(r["failed"]) for r in results)
    both = sum(r["stressed_but_alive"] for r in ok)
    t1_only = sum(r["T1"] and not r["T2"] for r in ok)
    t2_only = sum(r["T2"] and not r["T1"] for r in ok)
    neither = sum((not r["T1"]) and (not r["T2"]) for r in ok)
    print("\n== summary ==")
    print(f"cells evaluated: {len(ok)} of {len(results)}; failed runs: {n_fail_runs}")
    print(f"pass T1 only: {t1_only}; pass T2 only: {t2_only}; pass both: {both}; neither: {neither}")
    for key, label in (("sc1_ok", "SC-1"), ("sc2_ok", "SC-2"), ("sc3_ok", "SC-3"),
                       ("sc6_ok", "SC-6"), ("nonzero_ok", "T2 non-zero matched DeltaH >= 50%"),
                       ("survival_ok", "T2 mean survival in [0.40, 0.90]")):
        print(f"  {label}: fails in {sum(not r[key] for r in ok)} of {len(ok)} cells")
    print(f"  information only - T1 with the obligation leg of SC-3 left out would hold in "
          f"{sum(r['info_T1_without_obligation_leg'] for r in ok)} cells; "
          f"of those, T2 also holds in "
          f"{sum(r['info_T1_without_obligation_leg'] and r['T2'] for r in ok)}")
    print(f"ANSWER: {both} of {len(ok)} cells are stressed-but-alive under the declared targets")

    with open(out / "cells.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=CSV_FIELDS)
        w.writeheader()
        for res in ok:
            w.writerow(row_for_csv(res))
    (out / "cells_full.json").write_text(json.dumps(results, indent=1))
    (out / "control_b.json").write_text(json.dumps(cb, indent=1))
    print(f"\nwritten: {out / 'cells.csv'}, {out / 'cells_full.json'}, {out / 'control_b.json'}")
    print(f"wall-clock time: {time.time() - started:.1f} s")
    print(BANNER)
    return 0 if n_fail_runs == 0 and len(ok) == len(results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
