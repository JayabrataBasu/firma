#!/usr/bin/env python3
"""DIAGNOSTIC-ONLY - the 'blind spot' explanation of the compliance cliff (review R22).

    .venv/bin/python scripts/diag_blindspot.py --write-spec docs/review/evidence/R22/blind_spec.json
    .venv/bin/python scripts/diag_blindspot.py --spec docs/review/evidence/R22/blind_spec.json --out <dir>

Part 1 tests three predictions, fixed in the spec before any run, on a grid of
fixed shock magnitudes x action-window length ``l_w`` x ``h_crit``. A failed
prediction is printed as FALSIFIED with its numbers. Part 2 repeats the R20
Q2/Q4 measurements at ``l_w = 8``. Part 3 repeats R21's resource cells with
``resource.constant`` in place of ``resource.patchy``.

Imports (does not copy or edit) ``diag_cliff.py``, ``hetero_check.py``,
``population_generator.py``, ``regime_search.py`` and
``exploratory_e1_slice.py``. Refuses to run unless the spec file equals
``SPEC`` byte for byte. Seeds >= 910001 only.
"""

from __future__ import annotations

import argparse
import collections
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


dc = _load("diag_cliff")
hc, pg, rs, sl = dc.hc, dc.pg, dc.rs, dc.sl

from firma_lab import load  # noqa: E402

BANNER = "DIAGNOSTIC-ONLY - BLIND-SPOT DIAGNOSTIC (R22) - NOT REGISTERED - NOT FOR TUNING"

ONSET = 40
MAGNITUDES = [round(0.100 + 0.025 * i, 3) for i in range(11)]  # 0.100 .. 0.350
L_WS = [4, 8]
H_CRITS = [0.15, 0.30]
POP_SEEDS = list(range(910_001, 910_011))
WINDOW_RULES = ("decision.satisficing", "constraint.action_window", "constraint.enforce")
INTERIOR = [0.175, 0.200, 0.225]
EDGES = [0.150, 0.250]
OUTSIDE = [0.100, 0.125, 0.275, 0.300, 0.325, 0.350]
STRIKE_FIELD = "compliance_last_violation_tick"

SPEC: dict[str, Any] = {
    "task": "R22 BLIND - DIAGNOSTIC-ONLY; nothing registered",
    "hypothesis": "u moves in steps of 1/L_W = 0.25 (l_w = 4), larger than h_crit = 0.15, so for "
    "0.65 < theta_new < 0.75 a firm at u = 0.5 has margin above h_crit (no survival focus), takes "
    "one more regulated step to u = 0.75 and violates; below 0.65 survival focus triggers one step "
    "earlier and it steers away. If it holds, the cliff is an artefact of l_w = 4 with h_crit = 0.15.",
    "part_1": {
        "population": "R20 medium spread, aspiration read RELATIVE (R21 part B), 12 firms, init "
        "seeds 910001..910010",
        "shock": "shock.stochastic with onset_min = onset_max = 40 and magnitude_sd = 0 (so onset "
        "and magnitude are fixed): regulatory/theta_limit, instant, permanent, targets all; shock "
        "seed = init seed (no draw depends on it)",
        "fixed": "everything else as R20/R21: theta_limit 0.90, theta_cap 0.40, theta_q 40, "
        "output_price 3, input_price 2, observation.full, base rule set (resource.patchy), horizon "
        "120, mechanism seed 910001, environment seed 910002",
        "grid": {"magnitude": MAGNITUDES, "l_w": L_WS, "h_crit": H_CRITS,
                 "cells": 44, "shocked_runs": 440,
                 "forks": "one no-shock run per (l_w, h_crit, init seed): 40"},
        "l_w": "set to the same value in decision.satisficing, constraint.action_window and "
        "constraint.enforce",
        "h_crit": "decision.satisficing params key 'h_crit' (SatisficingParams::h_crit, "
        "firma-plugin-decision/src/lib.rs:812-813; default 0.15)",
        "per_cell": {
            "compliance_death_share": "share of the 10 runs in which at least one firm dies with "
            "cause 'compliance'",
            "survivors": "distribution of survivors at end (of 12)",
            "first_strike": "a 'compliance strike' is a constraint.enforce write of "
            "compliance_last_violation_tick (a first violation). T = the first tick >= 40 with any "
            "strike. Reported per cell: share of runs with such a strike; mean number of firms "
            "struck at T; mean number of deaths (any cause) at T",
            "first_compliance_death_tick": "also reported, because 'deaths at the tick of first "
            "compliance strike' can be read either way: mean number of compliance deaths in the "
            "first tick that has a compliance death",
            "nonzero_matched_delta_h_share": "R14 definition, against the cell's no-shock fork",
        },
    },
    "predictions_verbatim_from_the_R22_prompt": {
        "P1": "P1 (l_w = 4, h_crit = 0.15): compliance deaths appear only for m in [0.150, 0.250] "
        "(allow the grid resolution) and the share is >= 80% inside that interval, <= 10% outside.",
        "P2": "P2 (l_w = 4, h_crit = 0.30): no such band: compliance-death share <= 10% for every m "
        "(0.30 exceeds the 0.25 step).",
        "P3": "P3 (l_w = 8, h_crit = 0.15): no such band: compliance-death share <= 10% for every m "
        "(step 0.125 < 0.15).",
        "rule": "A prediction that fails is reported as FALSIFIED with the numbers; do not "
        "reinterpret.",
    },
    "how_the_predictions_are_evaluated": {
        "P1": "'allow the grid resolution' is read as: the two edge levels m = 0.150 and m = 0.250 "
        "(theta_new exactly 0.75 and 0.65, the boundaries of the hypothesised open interval) are "
        "reported but not counted. P1 HOLDS iff the compliance-death share is >= 0.80 at EACH of "
        "m = 0.175, 0.200, 0.225 and <= 0.10 at EACH of m = 0.100, 0.125, 0.275, 0.300, 0.325, "
        "0.350; otherwise FALSIFIED.",
        "P1_strict_for_information": "the same with the two edge levels counted as inside (share "
        ">= 0.80 required at all five of 0.150 .. 0.250) is also printed; it is not the verdict",
        "P2": "HOLDS iff the share is <= 0.10 at each of the 11 magnitudes in the (4, 0.30) cells",
        "P3": "HOLDS iff the share is <= 0.10 at each of the 11 magnitudes in the (8, 0.15) cells",
        "no_prediction": "(l_w = 8, h_crit = 0.30) is run and reported; nothing was predicted",
    },
    "falsification_statements": {
        "P1": "FALSIFIED if any interior level has share < 0.80 or any outside level has share > 0.10",
        "P2": "FALSIFIED if any magnitude has share > 0.10 at l_w = 4, h_crit = 0.30",
        "P3": "FALSIFIED if any magnitude has share > 0.10 at l_w = 8, h_crit = 0.15",
        "hypothesis": "the blind-spot explanation is not supported as stated if any of P1, P2, P3 "
        "is falsified",
    },
    "part_2": {
        "design": "exactly the R20 Q2/Q4 measurements (three spread levels, sets A and B, shock "
        "seed = init seed, shock.stochastic regulatory mean 0.20 sd 0.20/3 onset uniform 10..80, "
        "one shocked run and its fork per init seed), relative aspiration, with l_w = 8 in the "
        "three rules and h_crit = 0.15",
        "criteria": "R20's, unchanged; no new criteria",
        "comparison": "next to the l_w = 4 relative-reading numbers copied from "
        "docs/review/evidence/R21/summary.json (part B)",
    },
    "part_3": {
        "design": "R21 part D resource cells (+1, +2, +4 on input_price; shock.stochastic channel "
        "resource, magnitude_sd 0, onset uniform 10..80, instant, permanent), medium spread, "
        "relative aspiration, init seeds 910001..910050, shock seed = init seed, l_w = 8, h_crit = "
        "0.15, with the rule resource.patchy REPLACED by {id: resource.constant, params: {}} in "
        "both the shocked run and its fork",
        "resource_constant": "takes no parameters and emits nothing (ADR-0038 section 1; "
        "firma-plugin-resource/src/lib.rs:57, :90-92, :223-225)",
        "price_path": "input_price per tick for init seed 910001 at +4, shocked against no-shock, "
        "reconstructed from the logged AdjustGlobalInt deltas",
    },
    "controls": {
        "byte_equal": "part-1 cell (m = 0.200, l_w = 4, h_crit = 0.15, init seed 910001) run twice",
        "r20_row": "R20 medium init seed 910001 (absolute reading, stochastic shock) built by this "
        "script with l_w = 4 and h_crit = 0.15 set explicitly reproduces its "
        "docs/review/evidence/R20/q2_runs.csv row",
        "config_shown": "the three l_w values and h_crit of one generated config are printed",
    },
    "choices_made_by_the_builder_before_any_run": [
        "the P1 evaluation rule (edges exempt; per-level thresholds), with the strict variant "
        "printed for information",
        "fixed onset and magnitude via shock.stochastic with degenerate ranges, not shock.scheduled",
        "a 'compliance strike' is a first-violation record; both readings of 'deaths at the tick "
        "of first compliance strike' are reported",
        "part 3 uses the relative aspiration reading and replaces resource.patchy in the fork too",
        "part 3 keeps the stochastic onset (uniform 10..80) of R21 part D",
    ],
}


def spec_bytes() -> bytes:
    return (json.dumps(SPEC, indent=2, sort_keys=True) + "\n").encode("utf-8")


def spec_sha256() -> str:
    return hashlib.sha256(spec_bytes()).hexdigest()


# ---------------------------------------------------------------------------
# Configs
# ---------------------------------------------------------------------------


def _rule(cfg: Mapping[str, Any], rule_id: str) -> dict[str, Any]:
    hits = [r for r in cfg["rules"] if r["id"] == rule_id]
    if len(hits) != 1:
        raise ValueError(f"expected exactly one {rule_id} rule, found {len(hits)}")
    return hits[0]


def set_window_and_h_crit(cfg: dict[str, Any], l_w: int, h_crit: float) -> dict[str, Any]:
    """Set ``l_w`` in the three rules that carry it and ``h_crit`` in
    ``decision.satisficing``."""
    for rule_id in WINDOW_RULES:
        _rule(cfg, rule_id)["params"]["l_w"] = l_w
    _rule(cfg, "decision.satisficing")["params"]["h_crit"] = h_crit
    return cfg


def window_settings(cfg: Mapping[str, Any]) -> dict[str, Any]:
    out = {f"{rid}.l_w": _rule(cfg, rid)["params"].get("l_w") for rid in WINDOW_RULES}
    out["decision.satisficing.h_crit"] = _rule(cfg, "decision.satisficing")["params"].get("h_crit")
    return out


def use_constant_resource(cfg: dict[str, Any]) -> dict[str, Any]:
    """Replace ``resource.patchy`` by ``resource.constant`` (no parameters)."""
    for i, r in enumerate(cfg["rules"]):
        if r["id"] == "resource.patchy":
            cfg["rules"][i] = {"id": "resource.constant", "version": "^1", "params": {}}
            return cfg
    raise ValueError("no resource.patchy rule to replace")


def build_part1_config(base: Mapping[str, Any], init_seed: int, *, l_w: int, h_crit: float,
                       magnitude: float | None) -> dict[str, Any]:
    """Part 1. ``magnitude=None`` is the no-shock fork."""
    shocked = magnitude is not None
    cfg = dc.build_config(base, init_seed, "medium", reading="relative", shocked=shocked,
                          channel="regulatory", magnitude=magnitude if shocked else hc.SHOCK_MEAN)
    if shocked:
        p = _rule(cfg, "shock.stochastic")["params"]
        p["onset_min"] = p["onset_max"] = ONSET
        p["magnitude_mean"] = magnitude
        p["magnitude_sd"] = 0.0
    return set_window_and_h_crit(cfg, l_w, h_crit)


def build_part2_config(base: Mapping[str, Any], init_seed: int, level: str, *,
                       shocked: bool) -> dict[str, Any]:
    cfg = dc.build_config(base, init_seed, level, reading="relative", shocked=shocked)
    return set_window_and_h_crit(cfg, 8, 0.15)


def build_part3_config(base: Mapping[str, Any], init_seed: int, *, magnitude: int | None,
                       constant: bool = True) -> dict[str, Any]:
    shocked = magnitude is not None
    cfg = dc.build_config(base, init_seed, "medium", reading="relative", shocked=shocked,
                          channel="resource", magnitude=magnitude if shocked else 1)
    set_window_and_h_crit(cfg, 8, 0.15)
    return use_constant_resource(cfg) if constant else cfg


# ---------------------------------------------------------------------------
# Pure analysis
# ---------------------------------------------------------------------------


def strike_summary(strikes: Sequence[tuple[int, int]], deaths: Sequence[tuple[int, int, str]],
                   onset: int) -> dict[str, Any]:
    """From ``(tick, firm)`` first-violation records and ``(tick, firm,
    cause)`` death records: the first strike tick at or after ``onset`` and
    what happens in it; and the first compliance-death tick."""
    after = sorted(t for t, _ in strikes if t >= onset)
    out: dict[str, Any] = {"first_strike_tick": None, "firms_struck_at_first_strike": 0,
                           "deaths_at_first_strike_tick": 0, "first_compliance_death_tick": None,
                           "compliance_deaths_at_first_death_tick": 0}
    if after:
        t0 = after[0]
        out.update({"first_strike_tick": t0,
                    "firms_struck_at_first_strike": len({a for t, a in strikes if t == t0}),
                    "deaths_at_first_strike_tick": sum(1 for t, _, _ in deaths if t == t0)})
    cd = sorted(t for t, _, c in deaths if c == "compliance")
    if cd:
        out.update({"first_compliance_death_tick": cd[0],
                    "compliance_deaths_at_first_death_tick": sum(
                        1 for t, _, c in deaths if t == cd[0] and c == "compliance")})
    return out


def cell_summary(rows: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    n = len(rows)
    struck = [r for r in rows if r["first_strike_tick"] is not None]
    died = [r for r in rows if r["first_compliance_death_tick"] is not None]
    mean = lambda xs: statistics.fmean(xs) if xs else None  # noqa: E731
    return {
        "runs": n,
        "compliance_death_share": sum(r["compliance_deaths"] > 0 for r in rows) / n,
        "survivors_distribution": dict(sorted(collections.Counter(
            r["survivors"] for r in rows).items())),
        "share_with_a_strike_after_onset": len(struck) / n,
        "mean_firms_struck_at_first_strike": mean([r["firms_struck_at_first_strike"] for r in struck]),
        "mean_deaths_at_first_strike_tick": mean([r["deaths_at_first_strike_tick"] for r in struck]),
        "mean_compliance_deaths_at_first_death_tick": mean(
            [r["compliance_deaths_at_first_death_tick"] for r in died]),
        "first_compliance_death_ticks": dict(sorted(collections.Counter(
            r["first_compliance_death_tick"] for r in died).items())),
        "nonzero_matched_delta_h_share": sum(r["nonzero_matched_delta_h"] for r in rows) / n,
    }


def evaluate_predictions(share: Mapping[tuple[int, float, float], float]) -> dict[str, Any]:
    """``share[(l_w, h_crit, m)]`` -> the three verdicts, by the declared rules."""
    p1_in = {m: share[(4, 0.15, m)] for m in INTERIOR}
    p1_edge = {m: share[(4, 0.15, m)] for m in EDGES}
    p1_out = {m: share[(4, 0.15, m)] for m in OUTSIDE}
    p1 = all(v >= 0.80 for v in p1_in.values()) and all(v <= 0.10 for v in p1_out.values())
    p1_strict = p1 and all(v >= 0.80 for v in p1_edge.values())
    p2 = {m: share[(4, 0.30, m)] for m in MAGNITUDES}
    p3 = {m: share[(8, 0.15, m)] for m in MAGNITUDES}
    return {
        "P1": {"holds": p1, "interior": p1_in, "edges": p1_edge, "outside": p1_out,
               "strict_variant_holds": p1_strict},
        "P2": {"holds": all(v <= 0.10 for v in p2.values()), "shares": p2},
        "P3": {"holds": all(v <= 0.10 for v in p3.values()), "shares": p3},
    }


def price_series(events, initial: int, field: str, ticks: int) -> list[int]:
    """``field`` (a global int) at the end of each tick, from the logged
    ``AdjustGlobalInt`` deltas."""
    col = "kind.AdjustGlobalInt.field"
    per_tick: dict[int, int] = collections.defaultdict(int)
    if col in events.columns:
        g = events[(events["event"] == "delta_applied") & (events[col] == field)]
        for t, d in zip(g["tick"], g["kind.AdjustGlobalInt.delta"]):
            per_tick[int(t)] += int(d)
    out, value = [], initial
    for t in range(ticks):
        value += per_tick.get(t, 0)
        out.append(value)
    return out


# ---------------------------------------------------------------------------
# Runs
# ---------------------------------------------------------------------------


def _run(cfg: Mapping[str, Any], d: Path, firma: Path) -> None:
    shutil.rmtree(d, ignore_errors=True)
    err = sl.run_config(cfg, d, firma)
    if err:
        raise RuntimeError(f"{d.name}: {err}")


def measure(run_dir: Path) -> dict[str, Any]:
    events = load.read_events(run_dir / "run")
    by_agent, deaths = sl.decisions_and_deaths(events)
    field_col = "kind.SetAgentInt.field"
    s = events[(events["event"] == "delta_applied") & (events[field_col] == STRIKE_FIELD)
               & events["origin"].astype(str).str.startswith("constraint.enforce")]
    return {
        "entropies": sl.firm_tick_entropies(by_agent, deaths, sl.OUTCOME_L_W),
        "death_records": sorted((t, a, c) for a, (t, c) in deaths.items()),
        "strikes": sorted((int(t), int(a)) for t, a in zip(s["tick"], s["target.Agent"])),
        "events_sha256": hashlib.sha256((run_dir / "run" / "events.ndjson").read_bytes()).hexdigest(),
    }


def _part1_worker(args: tuple[int, float, int, str, str]) -> list[dict[str, Any]]:
    l_w, h_crit, seed, work, firma = args
    base = json.loads(rs.BASE_CONFIG.read_text())
    w = Path(work)
    tag = f"p1_{l_w}_{h_crit}_{seed}"
    d = w / f"{tag}_fork"
    _run(build_part1_config(base, seed, l_w=l_w, h_crit=h_crit, magnitude=None), d, Path(firma))
    fork = measure(d)
    shutil.rmtree(d, ignore_errors=True)
    rows = []
    for m in MAGNITUDES:
        d = w / f"{tag}_{m}"
        _run(build_part1_config(base, seed, l_w=l_w, h_crit=h_crit, magnitude=m), d, Path(firma))
        s = measure(d)
        matched = sl.matched_summary(s["entropies"], fork["entropies"], sl.rust_firm_ticks(d))
        shutil.rmtree(d, ignore_errors=True)
        row = {"l_w": l_w, "h_crit": h_crit, "magnitude": m, "init_seed": seed,
               "survivors": hc.N_FIRMS - len(s["death_records"]),
               "fork_survivors": hc.N_FIRMS - len(fork["death_records"]),
               "compliance_deaths": sum(1 for _, _, c in s["death_records"] if c == "compliance"),
               "delta_h_rep_matched": matched["delta_h_rep_matched"],
               "nonzero_matched_delta_h": matched["delta_h_rep_matched"] != 0.0,
               "fork_compliance_deaths": sum(1 for _, _, c in fork["death_records"]
                                             if c == "compliance"),
               "fork_strikes_total": len(fork["strikes"]),
               "death_records": s["death_records"], "events_sha256": s["events_sha256"]}
        row.update(strike_summary(s["strikes"], s["death_records"], ONSET))
        rows.append(row)
    return rows


def _pair_worker(args: tuple[str, dict[str, Any], dict[str, Any], dict[str, Any], str, str]
                 ) -> dict[str, Any]:
    """Parts 2 and 3: one fork + one shocked run through hc.evaluate_pair."""
    label, meta, fork_cfg, shocked_cfg, work, firma = args
    res = hc.evaluate_pair(label, hc.N_FIRMS, fork_cfg, shocked_cfg, Path(work), Path(firma))
    if res["failed"]:
        raise RuntimeError(res["failed"])
    res.update(meta)
    return res


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

PAIR_FIELDS = ["part", "level", "seed_set", "magnitude", "init_seed", "sc1_survival", "sc1_ok",
               "sc2_h_below_crit", "sc2_ok", "sc3_binds", "sc3_ok", "sc6_entropy_variance",
               "sc6_ok", "T1", "baseline_survivors", "shocked_survivors", "shocked_collapse",
               "shocked_compliance_deaths", "shocked_distinct_compliance_death_ticks",
               "shocked_max_deaths_in_one_tick", "shocked_mass_death_tick", "delta_h_rep_matched",
               "nonzero_matched_delta_h", "drawn_onset", "drawn_magnitude", "shocked_death_records",
               "baseline_death_records", "baseline_events_sha256", "shocked_events_sha256"]


def _f(x: Any) -> str:
    return "  n/a" if x is None else f"{x:5.2f}"


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=BANNER)
    ap.add_argument("--write-spec", help="STEP 0: write blind_spec.json to this path and exit")
    ap.add_argument("--spec", help="blind_spec.json written at step 0 (required to run)")
    ap.add_argument("--out", help="output directory (default: a temp dir)")
    ap.add_argument("--firma", help="path to the firma binary")
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--r20-evidence", default=str(REPO_ROOT / "docs/review/evidence/R20"))
    ap.add_argument("--r21-evidence", default=str(REPO_ROOT / "docs/review/evidence/R21"))
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
        print("REFUSED: --spec <blind_spec.json written at step 0> is required")
        return 1
    on_disk = Path(args.spec).read_bytes()
    if on_disk != spec_bytes():
        print("REFUSED: the spec file differs from the spec this script declares "
              f"(file sha256 {hashlib.sha256(on_disk).hexdigest()}, script {spec_sha256()})")
        return 1
    print(f"spec: {args.spec}  sha256 {spec_sha256()}")
    started = time.time()
    firma = sl.find_firma(args.firma)
    out = Path(args.out) if args.out else Path(tempfile.mkdtemp(prefix="firma-blind-"))
    work = out / "runs"
    work.mkdir(parents=True, exist_ok=True)
    base = json.loads(rs.BASE_CONFIG.read_text())
    jobs = max(1, args.jobs)
    W, F = str(work), str(firma)
    r20, r21 = Path(args.r20_evidence), Path(args.r21_evidence)
    problems: list[str] = []

    # -- controls ------------------------------------------------------------
    print("\n== controls ==")
    for l_w, h in ((4, 0.15), (8, 0.30)):
        cfg = build_part1_config(base, 910_001, l_w=l_w, h_crit=h, magnitude=0.2)
        print(f"  generated config (l_w {l_w}, h_crit {h}): {window_settings(cfg)}; shock params "
              f"onset {_rule(cfg, 'shock.stochastic')['params']['onset_min']}.."
              f"{_rule(cfg, 'shock.stochastic')['params']['onset_max']}, magnitude "
              f"{_rule(cfg, 'shock.stochastic')['params']['magnitude_mean']} sd "
              f"{_rule(cfg, 'shock.stochastic')['params']['magnitude_sd']}")
    shas = []
    cfg = build_part1_config(base, 910_001, l_w=4, h_crit=0.15, magnitude=0.2)
    for i in (1, 2):
        d = work / f"ctl_{i}"
        _run(cfg, d, firma)
        shas.append(hashlib.sha256((d / "run" / "events.ndjson").read_bytes()).hexdigest())
        shutil.rmtree(d, ignore_errors=True)
    print(f"  part-1 cell (m 0.200, l_w 4, h_crit 0.15, init seed 910001) run twice: "
          f"{'BYTE-EQUAL' if shas[0] == shas[1] else 'DIFFERENT'}  {shas[0]}")
    if shas[0] != shas[1]:
        problems.append("byte-equality control")

    row20 = next(r for r in csv.DictReader(open(r20 / "q2_runs.csv"))
                 if r["level"] == "medium" and r["init_seed"] == "910001")
    mk = lambda shocked: set_window_and_h_crit(  # noqa: E731
        dc.build_config(base, 910_001, "medium", reading="absolute", shocked=shocked), 4, 0.15)
    p = hc.evaluate_pair("ctl_r20", hc.N_FIRMS, mk(False), mk(True), work, firma)
    checks = {
        "shocked_survivors": p["shocked_survivors"] == int(row20["shocked_survivors"]),
        "baseline_survivors": p["baseline_survivors"] == int(row20["baseline_survivors"]),
        "shocked_death_records": p["shocked_death_records"] == [
            tuple(x) for x in json.loads(row20["shocked_death_records"])],
        "T1": str(p["T1"]) == row20["T1"],
        "sc2_h_below_crit": p["sc2_h_below_crit"] == float(row20["sc2_h_below_crit"]),
        "delta_h_rep_matched": p["delta_h_rep_matched"] == float(row20["delta_h_rep_matched"]),
        "shocked_events_sha256": p["shocked_events_sha256"] == row20["shocked_events_sha256"],
    }
    outcome_ok = all(v for k, v in checks.items() if k != "shocked_events_sha256")
    print(f"  R20 medium init seed 910001 with l_w = 4, h_crit = 0.15 set explicitly: "
          f"{'REPRODUCES' if outcome_ok else 'DIFFERS FROM'} its q2_runs.csv row ({checks}); "
          f"shocked survivors {p['shocked_survivors']} of 12, compliance deaths "
          f"{p['shocked_compliance_deaths']}, largest same-tick death count "
          f"{p['shocked_max_deaths_in_one_tick']}")
    if not outcome_ok:
        problems.append("R20 row control")

    # -- part 1 --------------------------------------------------------------
    print("\n== PART 1: fixed onset 40, fixed magnitude; 44 cells x 10 populations ==")
    items = [(l_w, h, seed, W, F) for l_w in L_WS for h in H_CRITS for seed in POP_SEEDS]
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        p1 = [row for rows in pool.map(_part1_worker, items) for row in rows]
    cells, share = {}, {}
    for l_w in L_WS:
        for h in H_CRITS:
            fork_rows = [r for r in p1 if r["l_w"] == l_w and r["h_crit"] == h
                         and r["magnitude"] == MAGNITUDES[0]]
            print(f" l_w = {l_w}, h_crit = {h}   (no-shock forks: survivors "
                  f"{dict(sorted(collections.Counter(r['fork_survivors'] for r in fork_rows).items()))}"
                  f", forks with a compliance death "
                  f"{sum(r['fork_compliance_deaths'] > 0 for r in fork_rows)} of 10)")
            print("     m    theta_new  compl.-death  strike>=40  firms struck  deaths at     compl. deaths at  "
                  "non-zero  survivors (of 12): runs")
            print("                     share         share       at 1st strike 1st strike    1st death tick    "
                  "dH share")
            for m in MAGNITUDES:
                rows = [r for r in p1 if r["l_w"] == l_w and r["h_crit"] == h and r["magnitude"] == m]
                c = cell_summary(rows)
                cells[(l_w, h, m)] = c
                share[(l_w, h, m)] = c["compliance_death_share"]
                print(f"   {m:.3f}  {0.9 - m:.3f}      {c['compliance_death_share']:.2f}          "
                      f"{c['share_with_a_strike_after_onset']:.2f}        "
                      f"{_f(c['mean_firms_struck_at_first_strike'])}         "
                      f"{_f(c['mean_deaths_at_first_strike_tick'])}         "
                      f"{_f(c['mean_compliance_deaths_at_first_death_tick'])}             "
                      f"{c['nonzero_matched_delta_h_share']:.2f}     {c['survivors_distribution']}")
    verdicts = evaluate_predictions(share)
    v1 = verdicts["P1"]
    print("\n  PREDICTIONS (rules fixed in the spec):")
    print(f"  P1 (l_w 4, h_crit 0.15): {'HOLDS' if v1['holds'] else 'FALSIFIED'} - interior "
          f"{v1['interior']} (each must be >= 0.80); outside {v1['outside']} (each must be <= 0.10); "
          f"edges, not counted: {v1['edges']}. Strict variant with the edges counted as inside "
          f"(information only): {'holds' if v1['strict_variant_holds'] else 'does not hold'}")
    for key, label in (("P2", "l_w 4, h_crit 0.30"), ("P3", "l_w 8, h_crit 0.15")):
        v = verdicts[key]
        print(f"  {key} ({label}): {'HOLDS' if v['holds'] else 'FALSIFIED'} - shares by magnitude "
              f"{v['shares']} (each must be <= 0.10)")
    print(f"  no prediction (l_w 8, h_crit 0.30): shares "
          f"{ {m: share[(8, 0.30, m)] for m in MAGNITUDES} }")

    # -- part 2 --------------------------------------------------------------
    print("\n== PART 2: R20 Q2/Q4 at l_w = 8, h_crit = 0.15 (relative aspiration) ==")
    items2 = [(f"p2_{lvl}_{seed}", {"part": 2, "level": lvl, "seed_set": name, "init_seed": seed},
               build_part2_config(base, seed, lvl, shocked=False),
               build_part2_config(base, seed, lvl, shocked=True), W, F)
              for lvl in pg.SPREAD_LEVELS for name, seeds in hc.SEED_SETS.items() for seed in seeds]
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        p2 = list(pool.map(_pair_worker, items2))
    old = json.loads((r21 / "summary.json").read_text())["B"]
    keys = ("collapse_rate", "mass_death_tick_share", "mean_survival_shocked",
            "mean_survival_baseline", "nonzero_matched_delta_h_share", "sc1_pass_share",
            "sc2_pass_share", "sc3_pass_share", "sc6_pass_share", "t1_pass_share")
    p2_summary: dict[str, Any] = {}
    p2_verdicts = {}
    for lvl in pg.SPREAD_LEVELS:
        p2_summary[lvl] = {}
        for name in ("A", "B", "pooled"):
            rows = [r for r in p2 if r["level"] == lvl and (name == "pooled" or r["seed_set"] == name)]
            p2_summary[lvl][name] = hc.summarise(rows)
        new, was = p2_summary[lvl]["pooled"], old[lvl]["pooled"]
        print(f" spread level {lvl}:")
        print(f"   {'metric (pooled, 100 init seeds)':<34} l_w = 8   l_w = 4 (R21 B)")
        for k in keys:
            print(f"   {k:<34} {new[k]:.3f}     {was[k]:.3f}")
        print(f"   survivors, shocked, l_w = 8: {new['shocked_survivors_distribution']}")
        print(f"   survivors, shocked, l_w = 4: "
              f"{ {int(k): x for k, x in was['shocked_survivors_distribution'].items()} }")
        print(f"   survivors, baseline, l_w = 8: {new['baseline_survivors_distribution']}")
        print(f"   runs with any compliance death, l_w = 8: {new['runs_with_a_compliance_death']} "
              f"of {new['runs']} (l_w = 4: {was['runs_with_a_compliance_death']}); distinct "
              f"compliance-death ticks per run: {new['distinct_compliance_death_ticks_distribution']}")
        sc1 = [r["sc1_survival"] for r in rows]
        sc2 = [r["sc2_h_below_crit"] for r in rows]
        sc6 = [r["sc6_entropy_variance"] for r in rows]
        binds = [sum(bool(r["sc3_binds"][i]) for r in rows) for i in range(4)]
        print(f"   baseline sanity values, l_w = 8 (100 baselines): SC-1 mean {statistics.fmean(sc1):.3f} "
              f"[{min(sc1):.3f}, {max(sc1):.3f}]; SC-2 mean {statistics.fmean(sc2):.3f} "
              f"[{min(sc2):.3f}, {max(sc2):.3f}] (too low {sum(x < 0.05 for x in sc2)}, too high "
              f"{sum(x > 0.25 for x in sc2)}); SC-3 binds [solvency, compliance, scope, obligation] "
              f"{binds}; SC-6 variance mean {statistics.fmean(sc6):.4f} [{min(sc6):.4f}, {max(sc6):.4f}]")
        for name in ("A", "B", "pooled"):
            s = p2_summary[lvl][name]
            verdict = hc.q4_verdict(s)
            if name == "pooled":
                p2_verdicts[lvl] = verdict
            print(f"   Q4 set {name:<6} SC-1 {s['sc1_pass_share']:.2f} SC-2 {s['sc2_pass_share']:.2f} "
                  f"SC-3 {s['sc3_pass_share']:.2f} SC-6 {s['sc6_pass_share']:.2f} T1 "
                  f"{s['t1_pass_share']:.2f} | collapse {s['collapse_rate']:.2f} | non-zero dH "
                  f"{s['nonzero_matched_delta_h_share']:.2f} | "
                  f"{'PASS' if verdict['pass'] else 'FAIL'}{' (verdict)' if name == 'pooled' else ''}")
    print("  R20 CRITERIA at l_w = 8: " + "; ".join(
        f"{lvl} {'PASS' if x['pass'] else 'FAIL'}" for lvl, x in p2_verdicts.items())
          + "   | at l_w = 4: low FAIL; medium FAIL; high FAIL")

    # -- part 3 --------------------------------------------------------------
    print("\n== PART 3: resource shock with resource.constant (medium, relative, l_w = 8, set A) ==")
    items3 = [(f"p3_{mag}_{seed}", {"part": 3, "level": "medium", "seed_set": "A",
                                    "magnitude": mag, "init_seed": seed},
               build_part3_config(base, seed, magnitude=None),
               build_part3_config(base, seed, magnitude=mag), W, F)
              for mag in (1, 2, 4) for seed in hc.SEED_SETS["A"]]
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        p3 = list(pool.map(_pair_worker, items3))
    p3_summary = []
    for mag in (1, 2, 4):
        rows = [r for r in p3 if r["magnitude"] == mag]
        s, g = hc.summarise(rows), dc.graded(rows)
        p3_summary.append({"magnitude": mag, **s, **g})
        causes = collections.Counter(c for r in rows for _, _, c in r["shocked_death_records"])
        print(f"  resource +{mag}: survivors {s['shocked_survivors_distribution']}; baseline "
              f"{s['baseline_survivors_distribution']}")
        print(f"    collapse rate {s['collapse_rate']:.3f}; >= 5 deaths in one tick "
              f"{s['mass_death_tick_share']:.3f}; non-zero matched DeltaH "
              f"{s['nonzero_matched_delta_h_share']:.3f}; mean survival {s['mean_survival_shocked']:.3f} "
              f"(baseline {s['mean_survival_baseline']:.3f}); deaths by cause {dict(sorted(causes.items()))}; "
              f"runs whose survivors differ from the baseline's "
              f"{sum(r['shocked_survivors'] != r['baseline_survivors'] for r in rows)} of {len(rows)}")
        print(f"    R21 'graded' rule (information; not a criterion of this task): extremes "
              f"{g['extreme_share']:.2f}, distinct survivor counts {g['distinct_survivor_counts']} -> "
              f"{'graded' if g['graded'] else 'not graded'}")
    paths = {}
    for label, mag, constant in (("constant, no shock", None, True), ("constant, +4", 4, True),
                                 ("patchy, +4 (R21 rule set, l_w 8)", 4, False)):
        d = work / "p3_path"
        cfg = build_part3_config(base, 910_001, magnitude=mag, constant=constant)
        _run(cfg, d, firma)
        ev = load.read_events(d / "run")
        paths[label] = price_series(ev, cfg["world"]["global_ints"]["input_price"], "input_price",
                                    cfg["world"]["ticks"])
        onset = sl.drawn_shock(ev)[0]
        if onset is not None:
            paths["onset"] = onset
        shutil.rmtree(d, ignore_errors=True)
    onset = paths["onset"]
    lo, hi = onset - 3, onset + 14
    print(f"  input_price path, init seed 910001, drawn onset {onset}; ticks {lo}..{hi - 1}:")
    for label in ("constant, no shock", "constant, +4", "patchy, +4 (R21 rule set, l_w 8)"):
        print(f"    {label:<34} {paths[label][lo:hi]}  ... last tick {paths[label][-1]}")
    stays = set(paths["constant, +4"][onset:]) == {paths["constant, no shock"][0] + 4}
    print(f"  with resource.constant the price stays shifted from onset to the last tick: {stays}")

    # -- files ---------------------------------------------------------------
    p1_fields = [k for k in p1[0] if k != "death_records"] + ["death_records"]
    with open(out / "part1_runs.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=p1_fields)
        w.writeheader()
        for r in p1:
            w.writerow({k: (json.dumps(r[k]) if isinstance(r[k], list) else r[k]) for k in p1_fields})
    for name, rows in (("part2_runs.csv", p2), ("part3_runs.csv", p3)):
        with open(out / name, "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=PAIR_FIELDS)
            w.writeheader()
            for r in rows:
                w.writerow({k: (json.dumps(r.get(k)) if isinstance(r.get(k), list) else r.get(k))
                            for k in PAIR_FIELDS})
    (out / "summary.json").write_text(json.dumps({
        "spec_sha256": spec_sha256(),
        "part1_cells": [{"l_w": k[0], "h_crit": k[1], "magnitude": k[2], **v}
                        for k, v in cells.items()],
        "predictions": {k: {kk: ({str(m): s for m, s in vv.items()} if isinstance(vv, dict) else vv)
                            for kk, vv in v.items()} for k, v in verdicts.items()},
        "part2": p2_summary, "part2_verdicts": p2_verdicts, "part3": p3_summary,
        "part3_price_paths": paths, "problems": problems}, indent=1))
    print(f"\nwritten under {out}: part1_runs.csv, part2_runs.csv, part3_runs.csv, summary.json")
    print(f"problems: {problems if problems else 'none'}")
    print(f"wall-clock time: {time.time() - started:.1f} s")
    print(BANNER)
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
