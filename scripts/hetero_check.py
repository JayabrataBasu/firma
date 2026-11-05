#!/usr/bin/env python3
"""DIAGNOSTIC-ONLY - checks on seeded population heterogeneity (review R20).

    .venv/bin/python scripts/population_generator.py --write-spec docs/review/evidence/R20/hetero_spec.json
    .venv/bin/python scripts/hetero_check.py --spec docs/review/evidence/R20/hetero_spec.json --out <dir>

Refuses to run unless the spec file is byte-for-byte the spec declared in
``scripts/population_generator.py``. Answers Q1 (run-level part), Q2, Q3, Q4
and Q5 of the R20 prompt and the two controls; every level and both seed sets
are always run and printed in the declared order. No hypothesis statistic is
computed. Seeds >= 910001 only.

Imports (does not copy or edit) ``scripts/exploratory_e1_slice.py`` (run
launcher, log reader, entropy, matched firm-tick DeltaH) and
``scripts/regime_search.py`` (T1 evaluation, the R18 cell builder).
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
from typing import Any, Callable, Mapping, Sequence

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "python"))


def _load(name: str):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


pg = _load("population_generator")
rs = _load("regime_search")
sl = rs.sl

from firma_lab import load, metrics  # noqa: E402
from firma_lab.runner import _add_pin_intervention  # noqa: E402  (ADR-0054 pin; imported, not edited)

BANNER = pg.BANNER
FIXED = pg.SPEC["fixed"]
N_FIRMS = pg.SPEC["n_firms"]
SEED_SETS = {"A": list(range(910_001, 910_051)), "B": list(range(911_001, 911_051))}
SHOCK_MEAN = 0.20
CONTROL_SHOCK_SEEDS = list(range(910_001, 910_021))
Q5_CELLS = [(h, s) for h in pg.SPEC["Q5"]["cells"]["pinned_margin"]
            for s in pg.SPEC["Q5"]["cells"]["pinned_shortfall_capital_growth"]]
Q5_SEEDS = SEED_SETS["A"]


# ---------------------------------------------------------------------------
# Configs
# ---------------------------------------------------------------------------


def _fixed_world(base: Mapping[str, Any], init_seed: int, shock_seed: int,
                 label: str) -> dict[str, Any]:
    cfg = copy.deepcopy(dict(base))
    seeds = {"mechanism": FIXED["mechanism_seed"], "environment": FIXED["environment_seed"],
             "shock": shock_seed, "init": init_seed}
    sl.check_seeds(seeds)
    cfg["seeds"] = seeds
    cfg["experiment"] = f"DIAGNOSTIC-ONLY-R20-hetero-{label}"
    cfg["world"]["ticks"] = FIXED["horizon_ticks"]
    cfg["world"]["snapshot_every"] = FIXED["horizon_ticks"]
    cfg["world"]["global_reals"]["theta_limit"] = FIXED["theta_limit"]
    cfg["world"]["global_reals"]["theta_cap"] = FIXED["theta_cap"]
    cfg["world"]["global_ints"]["output_price"] = FIXED["output_price"]
    cfg["world"]["global_ints"]["input_price"] = FIXED["input_price"]
    cfg["world"]["global_ints"]["theta_q"] = FIXED["theta_q"]
    return cfg


def _rules(base_rules: Sequence[Mapping[str, Any]], *, shocked: bool,
           beta: float | None = None) -> list[dict[str, Any]]:
    """The base rule set with observation.full, and shock.scheduled either
    replaced by the R12 stochastic shock (mean 0.20, sd mean/3) or removed."""
    shock = copy.deepcopy(sl.STOCHASTIC_SHOCK)
    shock["params"]["magnitude_mean"] = SHOCK_MEAN
    shock["params"]["magnitude_sd"] = SHOCK_MEAN / 3.0
    out, seen = [], collections.Counter()
    for rule in base_rules:
        seen[rule["id"]] += 1
        if rule["id"] == "shock.scheduled":
            if shocked:
                out.append(shock)
        elif rule["id"] == "observation.delayed":
            out.append({"id": "observation.full", "version": "^1", "params": {}})
        elif rule["id"] == "decision.satisficing" and beta is not None:
            r = copy.deepcopy(dict(rule))
            r["params"] = dict(r["params"], beta=beta)
            out.append(r)
        else:
            out.append(copy.deepcopy(dict(rule)))
    for rid in ("shock.scheduled", "observation.delayed", "decision.satisficing"):
        if seen[rid] != 1:
            raise ValueError(f"base config rule set is not the one this task assumes: {rid}")
    return out


def build_config(base: Mapping[str, Any], init_seed: int, level: str, *,
                 shocked: bool) -> dict[str, Any]:
    """Q2/Q4: a drawn 12-firm population. Shock seed = init seed. With
    ``shocked=False`` this is both the baseline and the shocked run's fork."""
    cfg = _fixed_world(base, init_seed, init_seed,
                       f"{level}-init{init_seed}-{'shocked' if shocked else 'noshock'}")
    cfg["agents"] = pg.population(init_seed, N_FIRMS, level, base["agents"][0])
    cfg["rules"] = _rules(base["rules"], shocked=shocked)
    return cfg


def build_q5_config(base: Mapping[str, Any], init_seed: int, *, drawn: bool, pinned_margin: float,
                    pinned_shortfall: float) -> dict[str, Any]:
    """Q5: one firm, Arm A pins (ADR-0054), beta = 1, no shock. ``drawn``:
    firm 0 from the generator at the medium level; otherwise base-config
    firm 0 unchanged (the negative control)."""
    cfg = _fixed_world(base, init_seed, 910_001,
                       f"q5-{'drawn' if drawn else 'fixed'}-init{init_seed}")
    template = base["agents"][0]
    cfg["agents"] = (pg.population(init_seed, 1, "medium", template) if drawn
                     else [copy.deepcopy(dict(template))])
    cfg["rules"] = _rules(base["rules"], shocked=False, beta=pg.SPEC["Q5"]["cells"]["beta"])
    _add_pin_intervention(cfg, "pinned_margin", pinned_margin)
    _add_pin_intervention(cfg, "pinned_shortfall_capital_growth", pinned_shortfall)
    _add_pin_intervention(cfg, "pinned_shortfall_capability", 0.0)
    _add_pin_intervention(cfg, "pinned_shortfall_obligation_clearance", 0.0)
    return cfg


# ---------------------------------------------------------------------------
# Per-run measurement (pure parts first)
# ---------------------------------------------------------------------------


def death_metrics(deaths: Sequence[tuple[int, int, str]], n_firms: int) -> dict[str, Any]:
    """The Q2 per-run quantities from ``(tick, firm, cause)`` death records."""
    per_tick = collections.Counter(t for t, _, _ in deaths)
    survivors = n_firms - len(deaths)
    return {
        "survivors": survivors,
        "collapse": survivors <= 0.20 * n_firms,
        "distinct_compliance_death_ticks": len({t for t, _, c in deaths if c == "compliance"}),
        "compliance_deaths": sum(1 for _, _, c in deaths if c == "compliance"),
        "max_deaths_in_one_tick": max(per_tick.values()) if per_tick else 0,
        "mass_death_tick": bool(per_tick) and max(per_tick.values()) >= 5,
    }


def measure(run_dir: Path) -> dict[str, Any]:
    events = load.read_events(run_dir / "run")
    by_agent, deaths = sl.decisions_and_deaths(events)
    entropies = sl.firm_tick_entropies(by_agent, deaths, sl.OUTCOME_L_W)
    onset, magnitude = sl.drawn_shock(events)
    return {
        "by_agent": by_agent,
        "death_records": sorted((t, a, c) for a, (t, c) in deaths.items()),
        "entropies": entropies,
        "h_rep_mean": sl.mean_repertoire_entropy(by_agent, deaths, sl.OUTCOME_L_W)[0],
        "action_sha": sl.action_sha(by_agent),
        "drawn_onset": onset,
        "drawn_magnitude": magnitude,
        "events_sha256": hashlib.sha256((run_dir / "run" / "events.ndjson").read_bytes()).hexdigest(),
    }


def evaluate_pair(label: str, n_firms: int, baseline_cfg: Mapping[str, Any],
                  shocked_cfg: Mapping[str, Any], work: Path, firma: Path) -> dict[str, Any]:
    """One no-shock baseline (= the fork) and one shocked run: T1 on the
    baseline, the Q2 death metrics on both, matched DeltaH on the pair."""
    b_dir, s_dir = work / f"{label}_b", work / f"{label}_s"
    for d, cfg in ((b_dir, baseline_cfg), (s_dir, shocked_cfg)):
        shutil.rmtree(d, ignore_errors=True)
        err = sl.run_config(cfg, d, firma)
        if err:
            return {"label": label, "failed": err}
    report = metrics.sanity_report(b_dir / "run")
    t1 = rs.evaluate_targets(report, [], n_firms)
    b, s = measure(b_dir), measure(s_dir)
    matched = sl.matched_summary(s["entropies"], b["entropies"], sl.rust_firm_ticks(s_dir))
    shutil.rmtree(b_dir, ignore_errors=True)
    shutil.rmtree(s_dir, ignore_errors=True)
    out = {"label": label, "failed": None, "n_firms": n_firms}
    out.update({k: t1[k] for k in ("sc1_survival", "sc1_ok", "sc2_h_below_crit", "sc2_ok",
                                   "sc3_binds", "sc3_ok", "sc6_entropy_variance", "sc6_ok", "T1")})
    out.update({f"baseline_{k}": v for k, v in death_metrics(b["death_records"], n_firms).items()})
    out.update({f"shocked_{k}": v for k, v in death_metrics(s["death_records"], n_firms).items()})
    out.update({
        "baseline_death_records": b["death_records"],
        "shocked_death_records": s["death_records"],
        "delta_h_rep_matched": matched["delta_h_rep_matched"],
        "nonzero_matched_delta_h": matched["delta_h_rep_matched"] != 0.0,
        "n_matched_firm_ticks": matched["n_matched_firm_ticks"],
        "drawn_onset": s["drawn_onset"], "drawn_magnitude": s["drawn_magnitude"],
        "baseline_events_sha256": b["events_sha256"], "shocked_events_sha256": s["events_sha256"],
    })
    return out


def summarise(rows: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """Aggregate Q2/Q4 quantities over a set of (baseline, shocked) pairs."""
    n = len(rows)
    frac = lambda key: sum(bool(r[key]) for r in rows) / n  # noqa: E731
    with_compliance = [r["shocked_distinct_compliance_death_ticks"] for r in rows
                       if r["shocked_compliance_deaths"] > 0]
    return {
        "runs": n,
        "shocked_survivors_distribution": dict(sorted(collections.Counter(
            r["shocked_survivors"] for r in rows).items())),
        "baseline_survivors_distribution": dict(sorted(collections.Counter(
            r["baseline_survivors"] for r in rows).items())),
        "collapse_rate": frac("shocked_collapse"),
        "baseline_collapse_rate": frac("baseline_collapse"),
        "mass_death_tick_share": frac("shocked_mass_death_tick"),
        "baseline_mass_death_tick_share": frac("baseline_mass_death_tick"),
        "distinct_compliance_death_ticks_distribution": dict(sorted(collections.Counter(
            r["shocked_distinct_compliance_death_ticks"] for r in rows).items())),
        "runs_with_a_compliance_death": len(with_compliance),
        "mean_distinct_compliance_death_ticks_given_any": (
            statistics.fmean(with_compliance) if with_compliance else None),
        "max_deaths_in_one_tick_distribution": dict(sorted(collections.Counter(
            r["shocked_max_deaths_in_one_tick"] for r in rows).items())),
        "mean_survival_shocked": statistics.fmean(r["shocked_survivors"] / r["n_firms"] for r in rows),
        "mean_survival_baseline": statistics.fmean(r["baseline_survivors"] / r["n_firms"] for r in rows),
        "nonzero_matched_delta_h_share": frac("nonzero_matched_delta_h"),
        "sc1_pass_share": frac("sc1_ok"), "sc2_pass_share": frac("sc2_ok"),
        "sc3_pass_share": frac("sc3_ok"), "sc6_pass_share": frac("sc6_ok"),
        "t1_pass_share": frac("T1"),
        "distinct_shocked_event_logs": len({r["shocked_events_sha256"] for r in rows}),
        "distinct_baseline_event_logs": len({r["baseline_events_sha256"] for r in rows}),
    }


#: Scalar metrics compared between seed sets in Q3.
Q3_METRICS = (
    "collapse_rate", "mass_death_tick_share", "mean_survival_shocked", "mean_survival_baseline",
    "nonzero_matched_delta_h_share", "sc1_pass_share", "sc2_pass_share", "sc3_pass_share",
    "sc6_pass_share", "t1_pass_share", "baseline_collapse_rate", "baseline_mass_death_tick_share",
)


def differs(a: float, b: float) -> bool:
    """The declared Q3 rule: more than 0.10 apart in absolute terms, or more
    than 2x apart in relative terms (a zero against a non-zero differs)."""
    if abs(a - b) > 0.10:
        return True
    lo, hi = sorted((abs(a), abs(b)))
    if lo == 0.0:
        return hi > 0.0
    return hi / lo > 2.0


def q4_verdict(s: Mapping[str, Any]) -> dict[str, Any]:
    """The reviewer-proposed criteria, fixed at step 0."""
    c1 = s["t1_pass_share"] >= 0.50
    c2 = s["collapse_rate"] <= 0.10
    c3 = s["nonzero_matched_delta_h_share"] >= 0.50
    return {"t1_at_least_half": c1, "collapse_at_most_10pct": c2, "nonzero_at_least_half": c3,
            "pass": c1 and c2 and c3}


# ---------------------------------------------------------------------------
# Workers
# ---------------------------------------------------------------------------


def _q2_worker(args: tuple[str, str, int, str, str]) -> dict[str, Any]:
    level, set_name, init_seed, work, firma = args
    base = json.loads(rs.BASE_CONFIG.read_text())
    res = evaluate_pair(
        f"{level}_{init_seed}", N_FIRMS,
        build_config(base, init_seed, level, shocked=False),
        build_config(base, init_seed, level, shocked=True), Path(work), Path(firma))
    res.update({"level": level, "seed_set": set_name, "init_seed": init_seed})
    return res


def _q5_worker(args: tuple[bool, float, float, int, str, str]) -> dict[str, Any]:
    drawn, h, s, init_seed, work, firma = args
    base = json.loads(rs.BASE_CONFIG.read_text())
    label = f"q5_{'d' if drawn else 'f'}_{h}_{s}_{init_seed}"
    d = Path(work) / label
    shutil.rmtree(d, ignore_errors=True)
    cfg = build_q5_config(base, init_seed, drawn=drawn, pinned_margin=h, pinned_shortfall=s)
    err = sl.run_config(cfg, d, Path(firma))
    out = {"arm": "drawn" if drawn else "fixed", "pinned_margin": h, "pinned_shortfall": s,
           "init_seed": init_seed, "failed": err}
    if not err:
        m = measure(d)
        firm = cfg["agents"][0]
        out.update({
            "action_sha": m["action_sha"], "h_rep_mean": m["h_rep_mean"],
            "deaths": len(m["death_records"]), "death_records": m["death_records"],
            "decisions": sum(len(v) for v in m["by_agent"].values()),
            "capital": firm["stocks"]["capital"], "capability": firm["reals"]["capability"],
            "aspiration": firm["reals"]["aspiration_capital_growth"],
            "obligation": firm.get("ints", {}).get("obligation"),
        })
    shutil.rmtree(d, ignore_errors=True)
    return out


def _control_worker(args: tuple[str, int, str, str]) -> dict[str, Any]:
    which, shock_seed, work, firma = args
    base = json.loads(rs.BASE_CONFIG.read_text())
    if which == "cell79":
        cell = rs.cells2()[79]
        mk: Callable[[bool], dict[str, Any]] = lambda shocked: rs.build_cell2_config(  # noqa: E731
            base, cell, shock_seed=shock_seed, shocked=shocked, shock_mean=SHOCK_MEAN)
        n = rs.GRID2_N
    else:
        mk = lambda shocked: sl.build_config(  # noqa: E731
            base, decision="satisficing", shock_seed=shock_seed, shocked=shocked)
        n = len(base["agents"])
    res = evaluate_pair(f"ctl_{which}_{shock_seed}", n, mk(False), mk(True), Path(work), Path(firma))
    res.update({"control": which, "shock_seed": shock_seed})
    return res


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def _fmt(x: Any) -> str:
    return "n/a" if x is None else (f"{x:.3f}" if isinstance(x, float) else str(x))


def print_q2_block(title: str, s: Mapping[str, Any]) -> None:
    print(f"  {title}: {s['runs']} init seeds")
    print(f"    survivors at end, shocked (of 12): {s['shocked_survivors_distribution']}")
    print(f"    survivors at end, baseline:        {s['baseline_survivors_distribution']}")
    print(f"    collapse rate (survivors <= 2): shocked {s['collapse_rate']:.3f}; "
          f"baseline {s['baseline_collapse_rate']:.3f}")
    print(f"    distinct compliance-death ticks per shocked run: "
          f"{s['distinct_compliance_death_ticks_distribution']} "
          f"(mean over the {s['runs_with_a_compliance_death']} runs with any compliance death: "
          f"{_fmt(s['mean_distinct_compliance_death_ticks_given_any'])})")
    print(f"    share of runs with >= 5 deaths in one tick: shocked {s['mass_death_tick_share']:.3f}; "
          f"baseline {s['baseline_mass_death_tick_share']:.3f}; largest same-tick death count per "
          f"shocked run: {s['max_deaths_in_one_tick_distribution']}")
    print(f"    non-zero matched DeltaH: {s['nonzero_matched_delta_h_share']:.3f}; distinct event "
          f"logs: shocked {s['distinct_shocked_event_logs']}, baseline "
          f"{s['distinct_baseline_event_logs']}")


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=BANNER)
    ap.add_argument("--spec", required=True, help="hetero_spec.json written at step 0")
    ap.add_argument("--out", help="output directory (default: a temp dir)")
    ap.add_argument("--firma", help="path to the firma binary")
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--r16-evidence", default=str(REPO_ROOT / "docs/review/evidence/R16"))
    ap.add_argument("--r18-evidence", default=str(REPO_ROOT / "docs/review/evidence/R18"))
    args = ap.parse_args(argv)

    print(BANNER)
    on_disk = Path(args.spec).read_bytes()
    if on_disk != pg.spec_bytes():
        print("REFUSED: the spec file differs from the spec scripts/population_generator.py "
              f"declares (file sha256 {hashlib.sha256(on_disk).hexdigest()}, script "
              f"{pg.spec_sha256()})")
        return 1
    print(f"spec: {args.spec}  sha256 {pg.spec_sha256()}")
    print(f"DISCLOSURE: {pg.SPEC['disclosure']}")
    started = time.time()
    firma = sl.find_firma(args.firma)
    out = Path(args.out) if args.out else Path(tempfile.mkdtemp(prefix="firma-hetero-"))
    work = out / "runs"
    work.mkdir(parents=True, exist_ok=True)
    base = json.loads(rs.BASE_CONFIG.read_text())
    jobs = max(1, args.jobs)
    r16, r18 = Path(args.r16_evidence), Path(args.r18_evidence)
    problems: list[str] = []

    # -- Q1 (run-level part) and the byte-equality control ----------------
    print("\n== Q1 / control: same seed -> byte-equal config and byte-equal event log ==")
    texts = [json.dumps(build_config(base, 910_001, "low", shocked=True), sort_keys=True)
             for _ in (1, 2)]
    print(f"  config built twice (low, init seed 910001, shocked): "
          f"{'BYTE-EQUAL' if texts[0] == texts[1] else 'DIFFERENT'}  sha256 "
          f"{hashlib.sha256(texts[0].encode()).hexdigest()}")
    shas = []
    for i in (1, 2):
        d = work / f"q1_{i}"
        err = sl.run_config(json.loads(texts[i - 1]), d, firma)
        if err:
            raise RuntimeError(err)
        shas.append(hashlib.sha256((d / "run" / "events.ndjson").read_bytes()).hexdigest())
        shutil.rmtree(d, ignore_errors=True)
    print(f"  event log, run twice: {'BYTE-EQUAL' if shas[0] == shas[1] else 'DIFFERENT'}  "
          f"{shas[0]}  {shas[1]}")
    if texts[0] != texts[1] or shas[0] != shas[1]:
        problems.append("Q1 byte-equality")

    # -- Controls: can this code see the cliff? ---------------------------
    print("\n== control: R12 smoke config and R18 cell 79 through the Q2/Q4 code ==")
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        ctl = list(pool.map(_control_worker, [(w, s, str(work), str(firma))
                                              for w in ("smoke", "cell79")
                                              for s in CONTROL_SHOCK_SEEDS]))
    if any(r["failed"] for r in ctl):
        raise RuntimeError([r["failed"] for r in ctl if r["failed"]])
    ctl_summary = {}
    smoke_rec = json.loads((r16 / "control_b.json").read_text())
    cell79_s1 = next(r for r in json.loads((r18 / "stage1_full.json").read_text())
                     if r["cell_params"]["cell"] == 79)
    cell79_s2 = {r["shock_magnitude_mean"]: r for r in json.loads(
        (r18 / "stage2_full.json").read_text()) if r["cell_params"]["cell"] == 79}
    for which, rec_t1, rec_s2 in (("smoke", smoke_rec, smoke_rec),
                                  ("cell79", cell79_s1, cell79_s2[SHOCK_MEAN])):
        rows = [r for r in ctl if r["control"] == which]
        s = summarise(rows)
        ctl_summary[which] = s
        now = {
            "sc1_survival": rows[0]["sc1_survival"], "sc2_h_below_crit": rows[0]["sc2_h_below_crit"],
            "sc3_binds": rows[0]["sc3_binds"], "sc6_entropy_variance": rows[0]["sc6_entropy_variance"],
            "survivors_per_run": [r["shocked_survivors"] for r in rows],
            "nonzero_matched_fraction": s["nonzero_matched_delta_h_share"],
            "mean_survival_shocked": s["mean_survival_shocked"],
        }
        recorded = {
            "sc1_survival": rec_t1["sc1_survival"], "sc2_h_below_crit": rec_t1["sc2_h_below_crit"],
            "sc3_binds": rec_t1["sc3_binds"], "sc6_entropy_variance": rec_t1["sc6_entropy_variance"],
            "survivors_per_run": [x["survivors"] for x in rec_s2["shocked_runs"]],
            "nonzero_matched_fraction": rec_s2["nonzero_matched_fraction"],
            "mean_survival_shocked": rec_s2["mean_survival_shocked"],
        }
        equal = all(now[k] == recorded[k] for k in now if k != "mean_survival_shocked") and abs(
            now["mean_survival_shocked"] - recorded["mean_survival_shocked"]) < 1e-12
        if not equal:
            problems.append(f"control {which} does not reproduce the recorded numbers")
        print(f"  {which}: {'REPRODUCES' if equal else 'DIFFERS FROM'} the recorded numbers")
        print(f"    now:      {now}")
        print(f"    recorded: {recorded}")
        print_q2_block(f"{which} through the Q2 metrics (20 shock seeds, init seed 910004)", s)
    ctl_summary["cell79_recorded_collapse_rate_by_magnitude"] = {
        str(m): sum(x["survivors"] <= 2 for x in r["shocked_runs"]) / len(r["shocked_runs"])
        for m, r in sorted(cell79_s2.items())}

    # -- Q2 / Q3 / Q4 ------------------------------------------------------
    work_items = [(lvl, name, seed, str(work), str(firma))
                  for lvl in pg.SPREAD_LEVELS for name, seeds in SEED_SETS.items() for seed in seeds]
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        q2 = list(pool.map(_q2_worker, work_items))
    failed = [r for r in q2 if r["failed"]]
    if failed:
        print(f"FAILED RUNS: {[(r['label'], r['failed']) for r in failed]}")
        problems.append("failed Q2 runs")
    q2 = [r for r in q2 if not r["failed"]]

    summaries: dict[str, dict[str, Any]] = {}
    print("\n== Q2: does heterogeneity remove the synchronised cliff? ==")
    print(f"  reference, copied from evidence/R18 (cell 79, homogeneous types): collapse rate by "
          f"shock mean {ctl_summary['cell79_recorded_collapse_rate_by_magnitude']}; in its collapsed "
          "runs nine firms die of compliance in one tick (passing_cells_description.txt)")
    for lvl in pg.SPREAD_LEVELS:
        print(f" spread level {lvl}: {pg.SPEC['spread_levels'][lvl]}")
        summaries[lvl] = {}
        for name in (*SEED_SETS, "pooled"):
            rows = [r for r in q2 if r["level"] == lvl and (name == "pooled" or r["seed_set"] == name)]
            summaries[lvl][name] = summarise(rows)
            print_q2_block(f"set {name}", summaries[lvl][name])

    print("\n== Q3: set A against set B (declared rule: |A-B| > 0.10, or > 2x) ==")
    for lvl in pg.SPREAD_LEVELS:
        print(f" spread level {lvl}:")
        for m in Q3_METRICS:
            a, b = summaries[lvl]["A"][m], summaries[lvl]["B"][m]
            print(f"    {m:<32} A {a:.3f}   B {b:.3f}   {'DIFFERS' if differs(a, b) else 'same'}")

    print("\n== Q4: cross-seed stressed-but-alive rate (criteria fixed at step 0) ==")
    verdicts = {}
    for lvl in pg.SPREAD_LEVELS:
        print(f" spread level {lvl}:")
        for name in ("A", "B", "pooled"):
            s = summaries[lvl][name]
            v = q4_verdict(s)
            if name == "pooled":
                verdicts[lvl] = v
            print(f"    set {name:<6} SC-1 {s['sc1_pass_share']:.2f}  SC-2 {s['sc2_pass_share']:.2f}  "
                  f"SC-3 {s['sc3_pass_share']:.2f}  SC-6 {s['sc6_pass_share']:.2f}  T1 "
                  f"{s['t1_pass_share']:.2f} | collapse {s['collapse_rate']:.2f} | non-zero dH "
                  f"{s['nonzero_matched_delta_h_share']:.2f} | "
                  f"{'PASS' if v['pass'] else 'FAIL'}"
                  f"{' (verdict)' if name == 'pooled' else ''}")
        print(f"    per init seed T1 (set A then set B, seed order; 1 = pass): "
              + "".join(str(int(r["T1"])) for r in q2 if r["level"] == lvl))
    print("  VERDICTS: " + "; ".join(f"{lvl} {'PASS' if v['pass'] else 'FAIL'}"
                                     for lvl, v in verdicts.items()))

    # -- Q5 ----------------------------------------------------------------
    print("\n== Q5: does the draw reach the Arm A outcome? (one pinned firm, beta = 1, no shock) ==")
    items = [(drawn, h, s, seed, str(work), str(firma))
             for drawn in (True, False) for h, s in Q5_CELLS for seed in Q5_SEEDS]
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        q5 = list(pool.map(_q5_worker, items))
    q5_failed = [r for r in q5 if r["failed"]]
    if q5_failed:
        print(f"  FAILED RUNS: {len(q5_failed)}; first: {q5_failed[0]['failed'][:300]}")
        problems.append("failed Q5 runs")
    q5_summary = []
    for arm in ("drawn", "fixed"):
        for h, s in Q5_CELLS:
            rows = [r for r in q5 if r["arm"] == arm and r["pinned_margin"] == h
                    and r["pinned_shortfall"] == s and not r["failed"]]
            if not rows:
                continue
            hr = [r["h_rep_mean"] for r in rows]
            row = {"arm": arm, "pinned_margin": h, "pinned_shortfall": s, "runs": len(rows),
                   "distinct_action_sequences": len({r["action_sha"] for r in rows}),
                   "h_rep_mean": statistics.fmean(hr),
                   "h_rep_sd": statistics.stdev(hr) if len(hr) > 1 else 0.0,
                   "h_rep_distinct_values": len({round(x, 12) for x in hr}),
                   "runs_in_which_the_firm_died": sum(r["deaths"] > 0 for r in rows)}
            q5_summary.append(row)
            print(f"  {arm:<5} h {h:.2f} shortfall {s:.1f}: {row['runs']} init seeds, "
                  f"{row['distinct_action_sequences']} distinct action sequences, H_rep mean "
                  f"{row['h_rep_mean']:.6f} sd {row['h_rep_sd']:.6f} "
                  f"({row['h_rep_distinct_values']} distinct values); firm died in "
                  f"{row['runs_in_which_the_firm_died']} runs")
    neg_ok = all(r["distinct_action_sequences"] == 1 for r in q5_summary if r["arm"] == "fixed")
    print(f"  negative control (fixed firm): {'1 sequence in every cell, as expected' if neg_ok else 'NOT 1 SEQUENCE - UNEXPECTED'}")
    if not neg_ok:
        problems.append("Q5 negative control")

    # -- Files ---------------------------------------------------------------
    q2_fields = ["level", "seed_set", "init_seed", "sc1_survival", "sc1_ok", "sc2_h_below_crit",
                 "sc2_ok", "sc3_binds", "sc3_ok", "sc6_entropy_variance", "sc6_ok", "T1",
                 "baseline_survivors", "baseline_collapse", "baseline_max_deaths_in_one_tick",
                 "shocked_survivors", "shocked_collapse", "shocked_compliance_deaths",
                 "shocked_distinct_compliance_death_ticks", "shocked_max_deaths_in_one_tick",
                 "shocked_mass_death_tick", "delta_h_rep_matched", "nonzero_matched_delta_h",
                 "n_matched_firm_ticks", "drawn_onset", "drawn_magnitude",
                 "shocked_death_records", "baseline_death_records",
                 "baseline_events_sha256", "shocked_events_sha256"]
    with open(out / "q2_runs.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=q2_fields)
        w.writeheader()
        for r in q2:
            w.writerow({k: (json.dumps(r[k]) if isinstance(r[k], list) else r[k]) for k in q2_fields})
    q5_fields = ["arm", "pinned_margin", "pinned_shortfall", "init_seed", "action_sha",
                 "h_rep_mean", "deaths", "decisions", "capital", "capability", "aspiration",
                 "obligation", "death_records", "failed"]
    with open(out / "q5_runs.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=q5_fields)
        w.writeheader()
        for r in q5:
            w.writerow({k: (json.dumps(r.get(k)) if isinstance(r.get(k), list) else r.get(k))
                        for k in q5_fields})
    (out / "summary.json").write_text(json.dumps(
        {"spec_sha256": pg.spec_sha256(), "q2_q3_q4": summaries, "q4_verdicts": verdicts,
         "q5": q5_summary, "controls": ctl_summary, "q1_event_log_sha256": shas,
         "problems": problems}, indent=1))
    (out / "control_runs.json").write_text(json.dumps(ctl, indent=1))
    print(f"\nwritten under {out}: q2_runs.csv, q5_runs.csv, summary.json, control_runs.json")
    print(f"problems: {problems if problems else 'none'}")
    print(f"wall-clock time: {time.time() - started:.1f} s")
    print(BANNER)
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
