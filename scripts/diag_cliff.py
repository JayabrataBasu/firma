#!/usr/bin/env python3
"""DIAGNOSTIC-ONLY - what drives the synchronised collapse (review R21, task DIAG).

    .venv/bin/python scripts/diag_cliff.py --write-spec docs/review/evidence/R21/diag_spec.json
    .venv/bin/python scripts/diag_cliff.py --spec docs/review/evidence/R21/diag_spec.json --out <dir>

Four parts, all always run and printed in the declared order; no criterion
or spread level from R20 is changed and no hypothesis statistic is computed:

A  attribution: 30 population (init) seeds x 30 shock seeds, fully crossed,
   medium spread as in R20; collapse matrix, marginals, two-way variance shares.
B  the R20 Q2/Q4 measurements with the aspiration spread read as RELATIVE.
C  behavioural diversity among the 12 firms in ticks 0..39 of the baseline.
D  shock channel: regulatory, resource, competitive on one population set.

Imports (does not copy or edit) ``population_generator.py``,
``hetero_check.py``, ``regime_search.py`` and ``exploratory_e1_slice.py``.
The run refuses to start unless the spec file equals ``SPEC`` byte for byte.
"""

from __future__ import annotations

import argparse
import collections
import copy
import csv
import hashlib
import importlib.util
import json
import shutil
import statistics
import sys
import tempfile
import time
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path
from typing import Any, Mapping, Sequence

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "python"))


def _load(name: str):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


hc = _load("hetero_check")
pg, rs, sl = hc.pg, hc.rs, hc.sl

from firma_lab import load  # noqa: E402

BANNER = "DIAGNOSTIC-ONLY - COLLAPSE DIAGNOSTIC (R21) - NOT REGISTERED - NOT FOR TUNING"

A_POP_SEEDS = list(range(910_001, 910_031))
A_SHOCK_SEEDS = list(range(910_101, 910_131))
C_SEEDS = list(range(910_001, 910_031))
C_LAST_TICK = 39
D_SEEDS = hc.SEED_SETS["A"]
D_CELLS = [("regulatory", 0.20), ("resource", 1), ("resource", 2), ("resource", 4),
           ("competitive", 1), ("competitive", 2)]
READINGS = ("absolute", "relative")

SPEC: dict[str, Any] = {
    "task": "R21 DIAG - DIAGNOSTIC-ONLY; nothing registered; no R20 criterion or spread level "
    "changed",
    "inherits": "everything not stated here is the R20 spec (docs/review/evidence/R20/"
    "hetero_spec.json, sha256 b10d402e83de170c6a507f41dad2accc35b7eb775d610a680c5e06593d492bab): "
    "generator, distributions, spread levels, fixed items, T1, collapse = survivors <= 2 of 12, "
    "Q3 difference rule, Q4 criteria (pooled over sets A and B)",
    "aspiration_readings": {
        "absolute": "R20: aspiration = 8.0 - a + 2 a u (pg.population unchanged)",
        "relative": "R21: aspiration = 8.0 * (1 + (-a + 2 a u)), with the SAME keyed uniform u "
        "(purpose tag aspiration_capital_growth) and a = 0.10 / 0.25 / 0.50 for low / medium / "
        "high; every other drawn value is unchanged",
    },
    "A_attribution": {
        "population": "medium spread, ABSOLUTE aspiration reading (the prompt says 'R20 spec')",
        "population_init_seeds": "910001..910030",
        "shock_seeds": "910101..910130",
        "design": "fully crossed: 900 shocked runs; one no-shock baseline per population seed "
        "(shock rule removed; its shock seed is 910101)",
        "shock": "regulatory/theta_limit, magnitude mean 0.20, sd 0.20/3, onset uniform 10..80, "
        "instant, permanent (as R20)",
        "collapse": "survivors <= 2 of 12",
        "variance_method": "two-way decomposition without replication of the 30 x 30 matrix of "
        "0/1 collapse indicators x[p][s]: SS_total = sum (x - grand)^2; SS_population = 30 * sum_p "
        "(mean_p - grand)^2; SS_shock = 30 * sum_s (mean_s - grand)^2; SS_interaction = SS_total - "
        "SS_population - SS_shock (interaction and residual are not separable with one run per "
        "cell). Shares are SS / SS_total. Undefined if SS_total = 0",
        "reading_rule": "range = max - min of the 30 marginal collapse rates. 'shock-determined' "
        "if range over shock seeds >= 3 x range over population seeds; 'population-determined' if "
        "range over population seeds >= 3 x range over shock seeds; otherwise 'mixed' (also "
        "'mixed' if both ranges are 0)",
        "also_reported": "drawn onset and magnitude for all 30 shock seeds with their marginal "
        "collapse rate (the prompt asks for 'the 20 collapse-prone shock seeds'; all 30 are "
        "listed, in seed order, so no selection is made)",
        "fork_check": "for population seed 910001 the no-shock run is executed at shock seeds "
        "910101, 910115 and 910130 and must be byte-equal",
    },
    "B_relative_aspiration": {
        "design": "exactly R20 Q2/Q4 (three spread levels, sets A and B, shock seed = init seed, "
        "one shocked run and its no-shock fork per init seed) with the relative reading",
        "criteria": "R20's, unchanged",
        "comparison": "printed next to the absolute-reading numbers copied from "
        "docs/review/evidence/R20/summary.json",
    },
    "C_behavioural_diversity": {
        "runs": "no-shock baseline, init seeds 910001..910030, each spread level, BOTH aspiration "
        "readings (the prompt does not say which; both are run, neither is chosen)",
        "window": "ticks 0..39 inclusive",
        "action_sequence": "per firm, the tuple of (tick, selected_action) decision records with "
        "tick <= 39 (a firm that dies earlier has a shorter tuple)",
        "focus_action_sequence": "per firm, the tuple of (tick, focus, selected_action)",
        "reported": "number of distinct sequences among the 12 firms; mean, min, max over seeds",
    },
    "D_shock_channel": {
        "population": "medium spread, RELATIVE aspiration reading, init seeds 910001..910050 "
        "(set A), shock seed = init seed",
        "cells": [
            {"channel": "regulatory", "target": "theta_limit", "magnitude_mean": 0.20,
             "magnitude_sd": "0.20 / 3"},
            {"channel": "resource", "magnitude": 1, "effect": "input_price 2 -> 3"},
            {"channel": "resource", "magnitude": 2, "effect": "input_price 2 -> 4"},
            {"channel": "resource", "magnitude": 4, "effect": "input_price 2 -> 6"},
            {"channel": "competitive", "magnitude": 1, "effect": "output_price 3 -> 2"},
            {"channel": "competitive", "magnitude": 2, "effect": "output_price 3 -> 1"},
        ],
        "resource_and_competitive_shock": "shock.stochastic with magnitude_mean = the level and "
        "magnitude_sd = 0 (the plugin then uses the mean exactly), onset uniform 10..80, instant, "
        "permanent, targets all",
        "validity": "read from the plugin before running: prices are unclamped i64 (ADR-0036); all "
        "five price levels leave both prices positive; none is dropped",
        "graded_rule": "a cell is 'graded' iff the share of runs with survivors <= 2 OR survivors "
        "== that seed's baseline survivors (12 minus baseline deaths) is below 0.20 (the two "
        "extremes counted together) AND at least 4 distinct survivor counts occur",
    },
    "controls": {
        "byte_equal": "A cell (population 910001, shock seed 910101) run twice",
        "r18_cell_79": "shock mean 0.20, shock seeds 910001..910020: SC values, per-run "
        "survivors, non-zero share and mean survival equal the R18 record",
        "r20_medium_910001": "the absolute-reading medium pair for init seed 910001 equals its "
        "row in docs/review/evidence/R20/q2_runs.csv, including both event-log sha256",
    },
    "choices_made_by_the_builder_before_any_run": [
        "A uses the absolute aspiration reading",
        "C runs both aspiration readings",
        "the SS decomposition and that interaction includes residual",
        "the reading rule when a range is 0",
        "all 30 shock seeds are listed instead of selecting 20",
        "resource/competitive magnitudes are fixed (sd = 0)",
        "the two 'graded' extremes are counted together against the 20% bound",
        "a dead firm's sequence in C is its shorter tuple",
    ],
}


def spec_bytes() -> bytes:
    return (json.dumps(SPEC, indent=2, sort_keys=True) + "\n").encode("utf-8")


def spec_sha256() -> str:
    return hashlib.sha256(spec_bytes()).hexdigest()


# ---------------------------------------------------------------------------
# Populations and configs
# ---------------------------------------------------------------------------


def relative_aspiration(init_seed: int, firm_id: int, level: str) -> float:
    """``8.0 * (1 + u')``, ``u'`` uniform on ``[-a, +a]``, from the same keyed
    uniform the generator uses for the aspiration purpose."""
    a = pg.SPEC["spread_levels"][level]["a_spread"]
    centre = pg.SPEC["distributions"]["aspiration_capital_growth"]["centre"]
    u = pg.uniform(init_seed, firm_id, pg.TAG_ASPIRATION)
    return centre * (1.0 + (-a + 2.0 * a * u))


def population(init_seed: int, level: str, reading: str,
               template: Mapping[str, Any]) -> list[dict[str, Any]]:
    """The R20 population; under the relative reading only the aspiration
    value is replaced."""
    if reading not in READINGS:
        raise ValueError(f"unknown aspiration reading {reading!r}")
    firms = pg.population(init_seed, hc.N_FIRMS, level, template)
    if reading == "relative":
        for f in firms:
            f["reals"]["aspiration_capital_growth"] = relative_aspiration(init_seed, f["id"], level)
    return firms


def shock_params(channel: str, magnitude: float) -> dict[str, Any]:
    """The shock.stochastic params for one D cell (regulatory: as R20)."""
    p = copy.deepcopy(sl.STOCHASTIC_SHOCK["params"])
    if channel == "regulatory":
        p["channel"] = {"kind": "regulatory", "target": "theta_limit"}
        p["magnitude_mean"] = magnitude
        p["magnitude_sd"] = magnitude / 3.0
    elif channel in ("resource", "competitive"):
        p["channel"] = {"kind": channel}
        p["magnitude_mean"] = float(magnitude)
        p["magnitude_sd"] = 0.0
    else:
        raise ValueError(f"unknown channel {channel!r}")
    return p


def build_config(base: Mapping[str, Any], init_seed: int, level: str, *, reading: str,
                 shocked: bool, shock_seed: int | None = None, channel: str = "regulatory",
                 magnitude: float = hc.SHOCK_MEAN) -> dict[str, Any]:
    """``hc.build_config`` (R20) with, optionally, a different shock seed, the
    relative aspiration reading and a different shock channel. With the
    defaults and ``reading='absolute'`` only the experiment label differs."""
    cfg = hc.build_config(base, init_seed, level, shocked=shocked)
    if shock_seed is not None:
        cfg["seeds"]["shock"] = shock_seed
    sl.check_seeds(cfg["seeds"])
    cfg["agents"] = population(init_seed, level, reading, base["agents"][0])
    if shocked:
        rule = next(r for r in cfg["rules"] if r["id"] == "shock.stochastic")
        rule["params"] = shock_params(channel, magnitude)
    return cfg


# ---------------------------------------------------------------------------
# Pure analysis
# ---------------------------------------------------------------------------


def variance_shares(matrix: Sequence[Sequence[int]]) -> dict[str, Any]:
    """The declared two-way decomposition (rows = population seeds, columns =
    shock seeds, one 0/1 observation per cell)."""
    n_p, n_s = len(matrix), len(matrix[0])
    grand = sum(map(sum, matrix)) / (n_p * n_s)
    row = [sum(r) / n_s for r in matrix]
    col = [sum(matrix[p][s] for p in range(n_p)) / n_p for s in range(n_s)]
    ss_total = sum((x - grand) ** 2 for r in matrix for x in r)
    ss_pop = n_s * sum((m - grand) ** 2 for m in row)
    ss_shock = n_p * sum((m - grand) ** 2 for m in col)
    ss_int = ss_total - ss_pop - ss_shock
    out = {"grand_rate": grand, "population_marginals": row, "shock_marginals": col,
           "ss_total": ss_total, "ss_population": ss_pop, "ss_shock": ss_shock,
           "ss_interaction": ss_int,
           "range_population": max(row) - min(row), "range_shock": max(col) - min(col)}
    if ss_total > 0:
        out.update({"share_population": ss_pop / ss_total, "share_shock": ss_shock / ss_total,
                    "share_interaction": ss_int / ss_total})
    else:
        out.update({"share_population": None, "share_shock": None, "share_interaction": None})
    return out


def attribution_reading(range_population: float, range_shock: float) -> str:
    """The predeclared reading of part A."""
    if range_population == 0 and range_shock == 0:
        return "mixed"
    if range_shock >= 3 * range_population:
        return "shock-determined"
    if range_population >= 3 * range_shock:
        return "population-determined"
    return "mixed"


def graded(rows: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """The predeclared 'graded' reading for one D cell."""
    n = len(rows)
    extreme = sum(1 for r in rows if r["shocked_survivors"] <= 2
                  or r["shocked_survivors"] == r["baseline_survivors"])
    distinct = len({r["shocked_survivors"] for r in rows})
    return {"extreme_share": extreme / n, "distinct_survivor_counts": distinct,
            "collapsed": sum(r["shocked_survivors"] <= 2 for r in rows),
            "untouched": sum(r["shocked_survivors"] == r["baseline_survivors"]
                             and r["shocked_survivors"] > 2 for r in rows),
            "graded": extreme / n < 0.20 and distinct >= 4}


def firm_sequences(events, last_tick: int) -> tuple[dict[int, tuple], dict[int, tuple]]:
    """Per firm, up to ``last_tick`` inclusive: the ``(tick, action)`` tuple
    and the ``(tick, focus, action)`` tuple, from ``decision.*`` records."""
    field_col, value_col = "kind.SetAgentInt.field", "kind.SetAgentInt.value"
    d = events[(events["event"] == "delta_applied")
               & events["origin"].astype(str).str.startswith("decision.")
               & events[field_col].isin(["selected_action", "focus"])
               & (events["tick"] <= last_tick)]
    acts: dict[int, dict[int, int]] = collections.defaultdict(dict)
    focus: dict[int, dict[int, int]] = collections.defaultdict(dict)
    for tick, agent, field, value in zip(d["tick"], d["target.Agent"], d[field_col], d[value_col]):
        (acts if field == "selected_action" else focus)[int(agent)][int(tick)] = int(value)
    a_seq = {a: tuple(sorted(v.items())) for a, v in acts.items()}
    fa_seq = {a: tuple((t, focus[a].get(t), act) for t, act in sorted(v.items()))
              for a, v in acts.items()}
    return a_seq, fa_seq


# ---------------------------------------------------------------------------
# Workers
# ---------------------------------------------------------------------------


def _run(cfg: Mapping[str, Any], d: Path, firma: Path) -> None:
    shutil.rmtree(d, ignore_errors=True)
    err = sl.run_config(cfg, d, firma)
    if err:
        raise RuntimeError(f"{d.name}: {err}")


def _a_worker(args: tuple[int, str, str]) -> dict[str, Any]:
    pop_seed, work, firma = args
    base = json.loads(rs.BASE_CONFIG.read_text())
    w = Path(work)
    d = w / f"a_{pop_seed}_base"
    _run(build_config(base, pop_seed, "medium", reading="absolute", shocked=False,
                      shock_seed=A_SHOCK_SEEDS[0]), d, Path(firma))
    b = hc.measure(d)
    shutil.rmtree(d, ignore_errors=True)
    rows = []
    for s in A_SHOCK_SEEDS:
        d = w / f"a_{pop_seed}_{s}"
        _run(build_config(base, pop_seed, "medium", reading="absolute", shocked=True,
                          shock_seed=s), d, Path(firma))
        m = hc.measure(d)
        shutil.rmtree(d, ignore_errors=True)
        dm = hc.death_metrics(m["death_records"], hc.N_FIRMS)
        rows.append({"population_seed": pop_seed, "shock_seed": s, "survivors": dm["survivors"],
                     "collapse": dm["collapse"], "compliance_deaths": dm["compliance_deaths"],
                     "max_deaths_in_one_tick": dm["max_deaths_in_one_tick"],
                     "drawn_onset": m["drawn_onset"], "drawn_magnitude": m["drawn_magnitude"],
                     "events_sha256": m["events_sha256"]})
    return {"population_seed": pop_seed,
            "baseline_survivors": hc.N_FIRMS - len(b["death_records"]),
            "baseline_events_sha256": b["events_sha256"], "runs": rows}


def _pair_worker(args: tuple[str, dict[str, Any], str, str]) -> dict[str, Any]:
    """B and D: one baseline + one shocked run through hc.evaluate_pair."""
    label, kw, work, firma = args
    base = json.loads(rs.BASE_CONFIG.read_text())
    common = {k: kw[k] for k in ("init_seed", "level", "reading")}
    res = hc.evaluate_pair(
        label, hc.N_FIRMS,
        build_config(base, shocked=False, **common),
        build_config(base, shocked=True, channel=kw["channel"], magnitude=kw["magnitude"], **common),
        Path(work), Path(firma))
    if res["failed"]:
        raise RuntimeError(res["failed"])
    res.update(kw)
    return res


def _cell79_worker(args: tuple[int, str, str]) -> dict[str, Any]:
    """Control: R18 cell 79 through hetero_check's pair evaluation. A wrapper
    is needed because a function of a module loaded by path cannot be sent to
    a worker process by reference."""
    shock_seed, work, firma = args
    return hc._control_worker(("cell79", shock_seed, work, firma))


def _c_worker(args: tuple[str, str, int, str, str]) -> dict[str, Any]:
    level, reading, seed, work, firma = args
    base = json.loads(rs.BASE_CONFIG.read_text())
    d = Path(work) / f"c_{level}_{reading}_{seed}"
    _run(build_config(base, seed, level, reading=reading, shocked=False), d, Path(firma))
    a_seq, fa_seq = firm_sequences(load.read_events(d / "run"), C_LAST_TICK)
    shutil.rmtree(d, ignore_errors=True)
    return {"level": level, "reading": reading, "init_seed": seed, "firms_with_a_decision": len(a_seq),
            "distinct_action_sequences": len(set(a_seq.values())),
            "distinct_focus_action_sequences": len(set(fa_seq.values()))}


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def _dist(rows: Sequence[Mapping[str, Any]], key: str) -> dict[int, int]:
    return dict(sorted(collections.Counter(r[key] for r in rows).items()))


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=BANNER)
    ap.add_argument("--write-spec", help="STEP 0: write diag_spec.json to this path and exit")
    ap.add_argument("--spec", help="diag_spec.json written at step 0 (required to run)")
    ap.add_argument("--out", help="output directory (default: a temp dir)")
    ap.add_argument("--firma", help="path to the firma binary")
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--r18-evidence", default=str(REPO_ROOT / "docs/review/evidence/R18"))
    ap.add_argument("--r20-evidence", default=str(REPO_ROOT / "docs/review/evidence/R20"))
    args = ap.parse_args(argv)
    print(BANNER)
    if args.write_spec:
        path = Path(args.write_spec)
        if path.exists():
            print(f"REFUSED: {path} already exists; the spec is declared once")
            return 1
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(spec_bytes())
        print(f"step 0: wrote {path}; sha256 {spec_sha256()}")
        return 0
    if not args.spec:
        print("REFUSED: --spec <diag_spec.json written at step 0> is required")
        return 1
    on_disk = Path(args.spec).read_bytes()
    if on_disk != spec_bytes():
        print("REFUSED: the spec file differs from the spec this script declares "
              f"(file sha256 {hashlib.sha256(on_disk).hexdigest()}, script {spec_sha256()})")
        return 1
    print(f"spec: {args.spec}  sha256 {spec_sha256()}")
    started = time.time()
    firma = sl.find_firma(args.firma)
    out = Path(args.out) if args.out else Path(tempfile.mkdtemp(prefix="firma-diag-"))
    work = out / "runs"
    work.mkdir(parents=True, exist_ok=True)
    base = json.loads(rs.BASE_CONFIG.read_text())
    jobs = max(1, args.jobs)
    r18, r20 = Path(args.r18_evidence), Path(args.r20_evidence)
    problems: list[str] = []
    W, F = str(work), str(firma)

    # -- controls ------------------------------------------------------------
    print("\n== controls ==")
    shas = []
    cfg = build_config(base, A_POP_SEEDS[0], "medium", reading="absolute", shocked=True,
                       shock_seed=A_SHOCK_SEEDS[0])
    for i in (1, 2):
        d = work / f"ctl_a_{i}"
        _run(cfg, d, firma)
        shas.append(hashlib.sha256((d / "run" / "events.ndjson").read_bytes()).hexdigest())
        shutil.rmtree(d, ignore_errors=True)
    print(f"  A cell (population 910001, shock seed 910101) run twice: "
          f"{'BYTE-EQUAL' if shas[0] == shas[1] else 'DIFFERENT'}  {shas[0]}")
    if shas[0] != shas[1]:
        problems.append("byte-equality control")

    fork_shas = []
    for s in (A_SHOCK_SEEDS[0], A_SHOCK_SEEDS[14], A_SHOCK_SEEDS[-1]):
        d = work / f"ctl_fork_{s}"
        _run(build_config(base, A_POP_SEEDS[0], "medium", reading="absolute", shocked=False,
                          shock_seed=s), d, firma)
        fork_shas.append(hashlib.sha256((d / "run" / "events.ndjson").read_bytes()).hexdigest())
        shutil.rmtree(d, ignore_errors=True)
    fork_ok = len(set(fork_shas)) == 1
    print(f"  no-shock run of population 910001 at shock seeds 910101 / 910115 / 910130: "
          f"{'BYTE-EQUAL' if fork_ok else 'DIFFERENT'} (one baseline per population is "
          f"{'valid' if fork_ok else 'NOT valid'})")
    if not fork_ok:
        problems.append("fork depends on the shock seed")

    with ProcessPoolExecutor(max_workers=jobs) as pool:
        ctl = list(pool.map(_cell79_worker, [(s, W, F) for s in hc.CONTROL_SHOCK_SEEDS]))
    rec1 = next(r for r in json.loads((r18 / "stage1_full.json").read_text())
                if r["cell_params"]["cell"] == 79)
    rec2 = next(r for r in json.loads((r18 / "stage2_full.json").read_text())
                if r["cell_params"]["cell"] == 79 and r["shock_magnitude_mean"] == hc.SHOCK_MEAN)
    s79 = hc.summarise(ctl)
    now = {"sc1_survival": ctl[0]["sc1_survival"], "sc2_h_below_crit": ctl[0]["sc2_h_below_crit"],
           "sc3_binds": ctl[0]["sc3_binds"], "sc6_entropy_variance": ctl[0]["sc6_entropy_variance"],
           "survivors_per_run": [r["shocked_survivors"] for r in ctl],
           "nonzero_matched_fraction": s79["nonzero_matched_delta_h_share"]}
    rec = {"sc1_survival": rec1["sc1_survival"], "sc2_h_below_crit": rec1["sc2_h_below_crit"],
           "sc3_binds": rec1["sc3_binds"], "sc6_entropy_variance": rec1["sc6_entropy_variance"],
           "survivors_per_run": [x["survivors"] for x in rec2["shocked_runs"]],
           "nonzero_matched_fraction": rec2["nonzero_matched_fraction"]}
    ok79 = now == rec and abs(s79["mean_survival_shocked"] - rec2["mean_survival_shocked"]) < 1e-12
    print(f"  R18 cell 79 (shock mean 0.20, 20 shock seeds): "
          f"{'REPRODUCES' if ok79 else 'DIFFERS FROM'} the R18 record; collapse rate "
          f"{s79['collapse_rate']:.2f}, >= 5 deaths in one tick {s79['mass_death_tick_share']:.2f}, "
          f"survivors {s79['shocked_survivors_distribution']}")
    if not ok79:
        problems.append("R18 cell 79 control")
        print(f"    now {now}\n    recorded {rec}")

    row20 = next(r for r in csv.DictReader(open(r20 / "q2_runs.csv"))
                 if r["level"] == "medium" and r["init_seed"] == "910001")
    p = _pair_worker(("ctl_r20", {"init_seed": 910_001, "level": "medium", "reading": "absolute",
                                  "channel": "regulatory", "magnitude": hc.SHOCK_MEAN}, W, F))
    checks = {
        "shocked_events_sha256": p["shocked_events_sha256"] == row20["shocked_events_sha256"],
        "baseline_events_sha256": p["baseline_events_sha256"] == row20["baseline_events_sha256"],
        "shocked_survivors": p["shocked_survivors"] == int(row20["shocked_survivors"]),
        "baseline_survivors": p["baseline_survivors"] == int(row20["baseline_survivors"]),
        "T1": str(p["T1"]) == row20["T1"],
        "sc2_h_below_crit": p["sc2_h_below_crit"] == float(row20["sc2_h_below_crit"]),
        "delta_h_rep_matched": p["delta_h_rep_matched"] == float(row20["delta_h_rep_matched"]),
    }
    print(f"  R20 medium init seed 910001: {'REPRODUCES' if all(checks.values()) else 'DIFFERS FROM'} "
          f"its q2_runs.csv row ({checks}); shocked survivors {p['shocked_survivors']}, event log "
          f"{p['shocked_events_sha256'][:16]}")
    if not all(checks.values()):
        problems.append("R20 medium 910001 control")

    # -- A -------------------------------------------------------------------
    print("\n== A: attribution - 30 population seeds x 30 shock seeds, medium spread ==")
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        a = list(pool.map(_a_worker, [(s, W, F) for s in A_POP_SEEDS]))
    matrix = [[int(r["collapse"]) for r in pop["runs"]] for pop in a]
    v = variance_shares(matrix)
    print("  collapse matrix (rows: population seed; columns: shock seeds 910101..910130; "
          "1 = survivors <= 2):")
    print("           " + "".join(str((s % 100) // 10) for s in A_SHOCK_SEEDS))
    print("           " + "".join(str(s % 10) for s in A_SHOCK_SEEDS))
    for pop, row, m in zip(a, matrix, v["population_marginals"]):
        print(f"    {pop['population_seed']} " + "".join(map(str, row))
              + f"  rate {m:.3f}  baseline survivors {pop['baseline_survivors']}")
    print("  marginal collapse rate by shock seed, with the drawn onset and magnitude "
          "(all 30, seed order):")
    draws = {}
    for j, s in enumerate(A_SHOCK_SEEDS):
        onsets = {pop["runs"][j]["drawn_onset"] for pop in a}
        mags = {pop["runs"][j]["drawn_magnitude"] for pop in a}
        draws[s] = (sorted(onsets), sorted(mags))
        same = len(onsets) == 1 and len(mags) == 1
        print(f"    {s}: rate {v['shock_marginals'][j]:.3f}  onset {sorted(onsets)[0]:>2}  "
              f"magnitude {sorted(mags)[0]:.4f}"
              + ("" if same else "  (DRAW DIFFERS ACROSS POPULATIONS)"))
    print(f"  overall collapse rate: {v['grand_rate']:.4f} ({sum(map(sum, matrix))} of 900)")
    print(f"  range of marginal rates: across shock seeds {v['range_shock']:.3f} "
          f"(min {min(v['shock_marginals']):.3f}, max {max(v['shock_marginals']):.3f}); across "
          f"population seeds {v['range_population']:.3f} (min {min(v['population_marginals']):.3f}, "
          f"max {max(v['population_marginals']):.3f})")
    if v["share_shock"] is None:
        print("  variance shares: undefined (no variation in collapse)")
    else:
        print(f"  share of collapse variance (two-way SS, one run per cell): shock seed "
              f"{v['share_shock']:.3f}; population seed {v['share_population']:.3f}; interaction "
              f"(incl. residual) {v['share_interaction']:.3f}")
    reading = attribution_reading(v["range_population"], v["range_shock"])
    print(f"  PREDECLARED READING: {reading}")
    a_all = [r for pop in a for r in pop["runs"]]
    print(f"  survivors over the 900 shocked runs: {_dist(a_all, 'survivors')}")
    print(f"  shock seeds with marginal rate exactly 0: "
          f"{sum(m == 0 for m in v['shock_marginals'])}; exactly 1: "
          f"{sum(m == 1 for m in v['shock_marginals'])}; strictly between: "
          f"{sum(0 < m < 1 for m in v['shock_marginals'])}")

    # -- B -------------------------------------------------------------------
    print("\n== B: R20 Q2/Q4 with the aspiration spread read as RELATIVE ==")
    items = [(f"b_{lvl}_{seed}", {"init_seed": seed, "level": lvl, "reading": "relative",
                                  "channel": "regulatory", "magnitude": hc.SHOCK_MEAN,
                                  "seed_set": name}, W, F)
             for lvl in pg.SPREAD_LEVELS for name, seeds in hc.SEED_SETS.items() for seed in seeds]
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        b = list(pool.map(_pair_worker, items))
    r20_summary = json.loads((r20 / "summary.json").read_text())["q2_q3_q4"]
    b_summary: dict[str, dict[str, Any]] = {}
    b_verdicts = {}
    keys = ("collapse_rate", "mass_death_tick_share", "mean_survival_shocked",
            "mean_survival_baseline", "nonzero_matched_delta_h_share", "sc1_pass_share",
            "sc2_pass_share", "sc3_pass_share", "sc6_pass_share", "t1_pass_share")
    for lvl in pg.SPREAD_LEVELS:
        b_summary[lvl] = {}
        a_spread = pg.SPEC["spread_levels"][lvl]["a_spread"]
        print(f" spread level {lvl} (aspiration 8.0 x (1 +/- {a_spread}): "
              f"{8 * (1 - a_spread):.1f} to {8 * (1 + a_spread):.1f}; R20 absolute: "
              f"{8 - a_spread:.2f} to {8 + a_spread:.2f})")
        for name in ("A", "B", "pooled"):
            rows = [r for r in b if r["level"] == lvl and (name == "pooled" or r["seed_set"] == name)]
            b_summary[lvl][name] = hc.summarise(rows)
        pooled, old = b_summary[lvl]["pooled"], r20_summary[lvl]["pooled"]
        print(f"   {'metric (pooled, 100 init seeds)':<34} relative   R20 absolute")
        for k in keys:
            print(f"   {k:<34} {pooled[k]:.3f}      {old[k]:.3f}")
        print(f"   survivors, shocked (relative): {pooled['shocked_survivors_distribution']}")
        print(f"   survivors, shocked (R20 abs.): "
              f"{ {int(k): v_ for k, v_ in old['shocked_survivors_distribution'].items()} }")
        print(f"   distinct compliance-death ticks per shocked run (relative): "
              f"{pooled['distinct_compliance_death_ticks_distribution']}")
        for name in ("A", "B", "pooled"):
            s = b_summary[lvl][name]
            verdict = hc.q4_verdict(s)
            if name == "pooled":
                b_verdicts[lvl] = verdict
            print(f"   Q4 set {name:<6} SC-1 {s['sc1_pass_share']:.2f} SC-2 {s['sc2_pass_share']:.2f} "
                  f"SC-3 {s['sc3_pass_share']:.2f} SC-6 {s['sc6_pass_share']:.2f} T1 "
                  f"{s['t1_pass_share']:.2f} | collapse {s['collapse_rate']:.2f} | non-zero dH "
                  f"{s['nonzero_matched_delta_h_share']:.2f} | {'PASS' if verdict['pass'] else 'FAIL'}"
                  f"{' (verdict)' if name == 'pooled' else ''}")
        diff = [m for m in hc.Q3_METRICS
                if hc.differs(b_summary[lvl]["A"][m], b_summary[lvl]["B"][m])]
        print(f"   set A vs set B, metrics that differ under the R20 rule: {diff if diff else 'none'}")
        hit = sorted(r["init_seed"] for r in rows if r["shocked_compliance_deaths"] > 0)
        print(f"   init seeds with a compliance death: {len(hit)} of {len(rows)}")
    print("  VERDICTS (relative reading): " + "; ".join(
        f"{lvl} {'PASS' if x['pass'] else 'FAIL'}" for lvl, x in b_verdicts.items())
          + "   | R20 absolute reading: low FAIL; medium FAIL; high FAIL")

    # -- C -------------------------------------------------------------------
    print(f"\n== C: distinct behaviour among the 12 firms, baseline, ticks 0..{C_LAST_TICK} ==")
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        c = list(pool.map(_c_worker, [(lvl, rd, s, W, F) for lvl in pg.SPREAD_LEVELS
                                      for rd in READINGS for s in C_SEEDS]))
    c_summary = []
    for lvl in pg.SPREAD_LEVELS:
        for rd in READINGS:
            rows = [r for r in c if r["level"] == lvl and r["reading"] == rd]
            row = {"level": lvl, "reading": rd, "seeds": len(rows)}
            for key in ("distinct_action_sequences", "distinct_focus_action_sequences"):
                vals = [r[key] for r in rows]
                row[key] = {"mean": statistics.fmean(vals), "min": min(vals), "max": max(vals),
                            "distribution": dict(sorted(collections.Counter(vals).items()))}
            c_summary.append(row)
            da, df = row["distinct_action_sequences"], row["distinct_focus_action_sequences"]
            print(f"  {lvl:<6} {rd:<8}: action sequences mean {da['mean']:.2f} range "
                  f"[{da['min']}, {da['max']}] {da['distribution']}; (focus, action) sequences "
                  f"mean {df['mean']:.2f} range [{df['min']}, {df['max']}] {df['distribution']}")

    # -- D -------------------------------------------------------------------
    print("\n== D: shock channel (medium spread, relative aspiration, set A, 50 init seeds) ==")
    items = [(f"d_{ch}_{mag}_{seed}", {"init_seed": seed, "level": "medium", "reading": "relative",
                                       "channel": ch, "magnitude": mag}, W, F)
             for ch, mag in D_CELLS for seed in D_SEEDS]
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        dd = list(pool.map(_pair_worker, items))
    d_summary = []
    base_shas = collections.defaultdict(set)
    for r in dd:
        base_shas[r["init_seed"]].add(r["baseline_events_sha256"])
    same_base = all(len(x) == 1 for x in base_shas.values())
    print(f"  baseline identical across the six cells for every init seed: {same_base}")
    if not same_base:
        problems.append("D baselines differ across channels")
    for ch, mag in D_CELLS:
        rows = [r for r in dd if r["channel"] == ch and r["magnitude"] == mag]
        s, g = hc.summarise(rows), graded(rows)
        d_summary.append({"channel": ch, "magnitude": mag, **s, **g})
        print(f"  {ch} magnitude {mag}:")
        print(f"    survivors (of 12): {s['shocked_survivors_distribution']}; baseline: "
              f"{s['baseline_survivors_distribution']}")
        print(f"    collapse rate {s['collapse_rate']:.3f}; >= 5 deaths in one tick "
              f"{s['mass_death_tick_share']:.3f}; non-zero matched DeltaH "
              f"{s['nonzero_matched_delta_h_share']:.3f}; mean survival {s['mean_survival_shocked']:.3f} "
              f"(baseline {s['mean_survival_baseline']:.3f}); T1 pass share {s['t1_pass_share']:.3f}")
        causes = collections.Counter(c_ for r in rows for _, _, c_ in r["shocked_death_records"])
        print(f"    deaths by cause over the 50 shocked runs: {dict(sorted(causes.items()))}")
        print(f"    graded rule: runs at an extreme {g['extreme_share']:.3f} (collapsed "
              f"{g['collapsed']}, equal to baseline survivors {g['untouched']}); distinct survivor "
              f"counts {g['distinct_survivor_counts']} -> {'GRADED' if g['graded'] else 'not graded'}")

    # -- files ---------------------------------------------------------------
    with open(out / "a_matrix.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["population_seed", *A_SHOCK_SEEDS])
        for pop, row in zip(a, matrix):
            w.writerow([pop["population_seed"], *row])
    with open(out / "a_runs.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(a_all[0]))
        w.writeheader()
        w.writerows(a_all)
    pair_fields = ["level", "reading", "channel", "magnitude", "seed_set", "init_seed",
                   "sc1_survival", "sc1_ok", "sc2_h_below_crit", "sc2_ok", "sc3_binds", "sc3_ok",
                   "sc6_entropy_variance", "sc6_ok", "T1", "baseline_survivors",
                   "shocked_survivors", "shocked_collapse", "shocked_compliance_deaths",
                   "shocked_distinct_compliance_death_ticks", "shocked_max_deaths_in_one_tick",
                   "shocked_mass_death_tick", "delta_h_rep_matched", "nonzero_matched_delta_h",
                   "drawn_onset", "drawn_magnitude", "shocked_death_records",
                   "baseline_death_records", "baseline_events_sha256", "shocked_events_sha256"]
    for name, rows in (("b_runs.csv", b), ("d_runs.csv", dd)):
        with open(out / name, "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=pair_fields)
            w.writeheader()
            for r in rows:
                w.writerow({k: (json.dumps(r.get(k)) if isinstance(r.get(k), list) else r.get(k))
                            for k in pair_fields})
    with open(out / "c_runs.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(c[0]))
        w.writeheader()
        w.writerows(c)
    (out / "summary.json").write_text(json.dumps({
        "spec_sha256": spec_sha256(),
        "A": {**v, "reading": reading,
              "shock_draws": {str(k): x for k, x in draws.items()},
              "baseline_survivors": {str(p_["population_seed"]): p_["baseline_survivors"] for p_ in a}},
        "B": b_summary, "B_verdicts": b_verdicts, "C": c_summary, "D": d_summary,
        "problems": problems}, indent=1))
    print(f"\nwritten under {out}: a_matrix.csv, a_runs.csv, b_runs.csv, c_runs.csv, d_runs.csv, "
          "summary.json")
    print(f"problems: {problems if problems else 'none'}")
    print(f"wall-clock time: {time.time() - started:.1f} s")
    print(BANNER)
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
