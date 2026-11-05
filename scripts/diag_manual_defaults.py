#!/usr/bin/env python3
"""DIAGNOSTIC-ONLY - the regime question at the manual defaults (review R23, task MANUALDEF).

    .venv/bin/python scripts/diag_manual_defaults.py --write-spec docs/review/evidence/R23/md_spec.json
    .venv/bin/python scripts/diag_manual_defaults.py --spec docs/review/evidence/R23/md_spec.json \
        --gate-output <captured `cargo test sc16_gate sc4_wmax_beta_probe -- --nocapture` output> --out <dir>

Part 1: a per-firm, per-tick trace around the shock for one l_w = 4 cell and
the matching l_w = 8 cell (prediction P4).
Part 2: the Phase-2 gate configs of tests/tests/sanity.rs, transcribed, at
l_w = 4 and l_w = 8 (prediction P5).
Part 3: a 40-cell baseline regime search at l_w = 8, h_crit = 0.15, with a
shocked second stage for cells that qualify (prediction P6).

Predictions and their evaluation rules are fixed in ``SPEC`` (step 0); a
failed prediction is printed as FALSIFIED. Seeds >= 910001 only: the gate
configs' own seeds lie in the reserved range 1-200 and are NOT executed here
(see ``SPEC["part_2"]``). Imports, without editing, the earlier diagnostic
scripts. Refuses to run unless the spec file equals ``SPEC`` byte for byte.
"""

from __future__ import annotations

import argparse
import collections
import copy
import csv
import hashlib
import importlib.util
import itertools
import json
import math
import os
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


bs = _load("diag_blindspot")
dc, hc, pg, rs, sl = bs.dc, bs.hc, bs.pg, bs.rs, bs.sl

from firma_lab import load, metrics  # noqa: E402

BANNER = "DIAGNOSTIC-ONLY - MANUAL-DEFAULTS DIAGNOSTIC (R23) - NOT REGISTERED - NOT FOR TUNING"

PRODUCE_REGULATED = 2  # firma-domain/src/margin.rs:18
FOCUS_NAMES = {-1: "NONE", 0: "SURV", 1: "G1", 2: "G2", 3: "G3"}
TRACE_SEED = 910_001
TRACE_TICKS = (36, 52)
TRACE_MAGNITUDE = 0.20
H_CRIT = 0.15
GATE_SEEDS = {"mechanism": 910_001, "environment": 910_002, "shock": 910_003, "init": 910_004}
GATE_SEEDS_2 = {"mechanism": 911_001, "environment": 911_002, "shock": 911_003, "init": 911_004}
WIDE_W_MAX, WIDE_BETA = 6, 1.0
P3_SEEDS = list(range(910_001, 910_021))
P3_AXES = [
    ("theta_limit", [0.50, 0.60, 0.70, 0.80, 0.90]),
    ("obligation_probability", [0.0, 1.0 / 6.0]),
    ("thin_fraction", [0.0, 1.0 / 6.0]),
    ("output_price", [2, 3]),
]
P3_SHOCK_MEANS = [0.10, 0.20, 0.30]
THIN_FACTOR = 0.2
SC_KEYS = ("sc1_survival", "sc2_survival_attention", "sc2_reconstructed_h_below_crit",
           "sc3_binds", "sc4_shaping_fraction", "sc5_shaping_success", "sc6_entropy_variance")

SPEC: dict[str, Any] = {
    "task": "R23 MANUALDEF - DIAGNOSTIC-ONLY; nothing registered",
    "predictions_verbatim_from_the_R23_prompt": {
        "P4": "P4 (part 1) the l_w = 4 first violation is preceded by a tick with margin >= 0.15 "
        "and non-survival focus",
        "P5": "P5 (part 2) changing l_w between 4 and 8 changes SC-2 and/or SC-3 for cfg_threaded "
        "or cfg_healthy; falsified if all six SC values are unchanged for both configs.",
        "P6": "P6 (part 3) at least one theta_limit level makes SC-3 pass in >= 50% of populations.",
        "rule": "State FALSIFIED where they fail; do not reinterpret.",
    },
    "part_1": {
        "cells": "R22 part-1 config (medium spread, relative aspiration, onset 40, fixed magnitude "
        "0.20, h_crit 0.15, init seed 910001) at l_w = 4 and at l_w = 8",
        "window": "ticks 36..52 inclusive, every firm alive at tick 36",
        "columns": {
            "action": "logged selected_action (decision.* SetAgentInt)",
            "focus": "logged focus code (-1 NONE, 0 SURVIVAL, 1..3 GOAL j)",
            "u": "NOT a logged scalar. Computed here from the LOGGED action window "
            "(constraint.action_window's ReplaceAgentList 'action_window') as it stood before "
            "the tick's decision: (number of entries with action 2 = produce_regulated among the "
            "last l_w) / l_w, the formula of firma-domain margin::u_from_window",
            "theta_limit": "config value plus the logged AdjustGlobalReal deltas up to and "
            "including the tick (the shock is applied in the environment phase, before decide)",
            "margin": "theta_limit - u (the compliance margin at decision time)",
            "h_t": "the margin the plugin itself used, from its existing read-only diagnostic "
            "trace (FIRMA_TRACE_DECISION 'selection' records); not in the event log",
            "strike": "constraint.enforce write of compliance_last_violation_tick in that tick",
            "death": "agent_died record in that tick",
        },
        "trace_control": "the event log with the diagnostic trace switched on must be byte-equal "
        "to the log without it",
        "P4_rule": "For every firm whose first compliance strike at or after tick 40 is at tick T "
        "(l_w = 4 cell): take its decision IN tick T (made before that tick's enforcement; the "
        "action that completes the violating window). P4 HOLDS iff for every such firm the "
        "decision-time compliance margin is >= 0.15 AND the focus is not SURVIVAL; FALSIFIED if "
        "any struck firm fails either, or if no firm is struck. The same two values at tick T-1 "
        "are printed for information.",
        "l_w_8_statement": "reported, no prediction: for the l_w = 8 cell, whether any firm is "
        "struck in 40..52, and for each firm the first tick >= 40 with SURVIVAL focus",
    },
    "part_2": {
        "configs": "cfg_healthy, cfg_threaded and cfg_wide_search(w_max = 6, beta = 1.0) of "
        "tests/tests/sanity.rs, transcribed into Python",
        "reserved_seeds": "The Rust configs use seeds 55/2/3/4, 7/2/3/4 and 909/2/3/4. Seeds "
        "1-200 are reserved (BUILDER.md, STATE.md hard stop) and are NOT executed by this "
        "script. The transcriptions run with mechanism/environment/shock/init = "
        "910001/910002/910003/910004. The original numbers are taken from the output of the "
        "existing tests themselves (`cargo test -p firma-conformance --test sanity sc16_gate "
        "sc4_wmax_beta_probe -- --nocapture`, part of the standard suite), passed in with "
        "--gate-output.",
        "validation": [
            "a unit test rebuilds each config mechanically from the Rust source text of "
            "tests/tests/sanity.rs and requires equality with the transcription (seeds aside)",
            "the transcription at its original l_w must reproduce the test's printed SC-1..SC-6 "
            "lines exactly (same formatted strings): cfg_threaded SC-2 1.000, SC-4 0/3202; "
            "cfg_healthy SC-2 0.005, SC-4 0/1600; cfg_wide_search(6, 1.0): the GOAL / NONE / "
            "SURVIVAL decision and shaping counts printed by sc4_wmax_beta_probe",
            "seed-insensitivity: each transcription is also run with seeds 911001..911004 and "
            "the two event logs are compared; if they are byte-equal the reserved-seed run "
            "would give the same log",
        ],
        "runs": "each of the three configs at l_w = 4 and l_w = 8, set in decision.satisficing, "
        "constraint.action_window and constraint.enforce; no other parameter changes",
        "reported": "SC-1..SC-6 for every (config, l_w), via firma_lab.metrics.sanity_report",
        "P5_rule": "Compared values per config between l_w = 4 and 8: SC-1 survival, SC-2 "
        "(survival-attention share, the gate's SC-2; and the reconstructed share), SC-3 binds "
        "vector, SC-4 fraction, SC-5, SC-6 variance. P5 HOLDS iff SC-2 (either share) or the SC-3 "
        "binds vector differs for cfg_threaded or for cfg_healthy. FALSIFIED iff all the compared "
        "values are unchanged for both configs. If SC-2 and SC-3 are unchanged for both but "
        "another SC value changes, the result is printed as NEITHER (the prompt's wording covers "
        "neither case). cfg_wide_search is reported and does not enter P5.",
    },
    "part_3": {
        "population": "R20 generator, medium spread, relative aspiration (R21), 12 firms, init "
        "seeds 910001..910020",
        "fixed": "l_w = 8 in the three rules; h_crit = 0.15; observation.full; base-config rule "
        "set (resource.patchy); horizon 120; input_price 2; theta_cap 0.40; theta_q 40; mechanism "
        "seed 910001; environment seed 910002; shock seed = init seed",
        "axes_in_declared_order": [
            {"name": "theta_limit", "levels": [0.50, 0.60, 0.70, 0.80, 0.90]},
            {"name": "obligation_probability", "levels": [0.0, 1.0 / 6.0],
             "meaning": "0: no firm starts with an obligation (theta_q stays 40); 1/6: the R20 "
             "keyed obligation draw, unchanged"},
            {"name": "thin_fraction", "levels": [0.0, 1.0 / 6.0],
             "meaning": "1/6: firm ids 0 and 1 (2 of 12) are thin-capital: capital = "
             "round_half_up(0.2 x the firm's own unrounded lognormal draw), i.e. median 20 with "
             "the same keyed normal; 0: no thin firm"},
            {"name": "output_price", "levels": [2, 3]},
        ],
        "cell_order": "itertools.product in the order listed, last axis fastest; 40 cells",
        "stage_1": "per cell, one no-shock baseline per init seed (20); per cell the pass share "
        "of SC-1 (sc1_survival in [0.60, 0.90]), SC-2 (sc2_reconstructed_h_below_crit in [0.05, "
        "0.25]), SC-3 (sc3_pass), SC-6 (sc6_pass) and T1 (all four), as in R16-R22",
        "stage_2": "only cells with T1 pass share >= 0.50: regulatory/theta_limit shock, onset "
        "uniform 10..80, instant, permanent, magnitude mean 0.10 / 0.20 / 0.30 with sd mean/3, "
        "one shocked run per init seed with its no-shock fork; per (cell, magnitude) row: "
        "collapse rate (survivors <= 2 of 12), non-zero matched DeltaH share (R14), and the "
        "R20 Q4 verdict (collapse <= 0.10 AND non-zero >= 0.50 AND T1 share >= 0.50)",
        "P6_rule": "P6 HOLDS iff at some theta_limit level at least one of its 8 cells has an "
        "SC-3 pass share >= 0.50; otherwise FALSIFIED. For information the SC-3 pass share "
        "pooled over all 8 cells of each level (160 baselines) is also printed; cells with "
        "obligation probability 0 cannot bind the obligation constraint.",
    },
    "controls": {
        "byte_equal": "part-3 cell 0, init seed 910001, baseline run twice",
        "r22_reproduces": "the part-3 cell (theta_limit 0.90, obligation 1/6, thin 0, price 3) "
        "is the R22 part-2 medium baseline config; for init seeds 910001..910020 its SC values "
        "and baseline event-log sha256 must equal docs/review/evidence/R22/part2_runs.csv",
    },
    "choices_made_by_the_builder_before_any_run": [
        "reserved seeds are not executed; gate transcriptions run at seeds 910001..910004",
        "cfg_wide_search is run at (w_max 6, beta 1.0), the manual defaults",
        "P4 is evaluated at the decision in the strike tick T; T-1 is shown for information",
        "the P5 NEITHER outcome",
        "P6 is evaluated per cell (any cell at a level), pooled share shown for information",
        "the thin-capital firms are ids 0 and 1; theta_q stays 40 when no firm is seeded",
        "u is computed in Python from the logged action window; h_t comes from the plugin's "
        "diagnostic trace",
    ],
}


def spec_bytes() -> bytes:
    return (json.dumps(SPEC, indent=2, sort_keys=True) + "\n").encode("utf-8")


def spec_sha256() -> str:
    return hashlib.sha256(spec_bytes()).hexdigest()


# ---------------------------------------------------------------------------
# Part 2: transcriptions of tests/tests/sanity.rs
# ---------------------------------------------------------------------------


def _market_actions() -> list[dict[str, Any]]:
    names = ("hold", "produce_ordinary", "produce_regulated", "acquire_input",
             "invest_capability", "deliver")
    return [{"id": f"action.market.standard.{n}", "version": "^1", "params": {}} for n in names] + [
        {"id": "action.shaping.rdt_standard.resolve_lagged", "version": "^1", "params": {}}]


def _constraint_rules(l_w: int) -> list[dict[str, Any]]:
    return [
        {"id": "constraint.action_window", "version": "^1", "params": {"l_w": l_w}},
        {"id": "constraint.enforce", "version": "^1",
         "params": {"t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": l_w}},
        {"id": "decision.aspiration_update", "version": "^1", "params": {"alpha": 0.10}},
    ]


def _world(ticks: int, theta_limit: float, theta_cap: float, theta_q: int,
           output_price: int) -> dict[str, Any]:
    return {"ticks": ticks, "resources": ["capital", "input"],
            "conflict_resolver": {"id": "conflict.additive", "version": "^1"},
            "snapshot_every": ticks,
            "global_reals": {"theta_limit": theta_limit, "theta_cap": theta_cap},
            "global_ints": {"theta_q": theta_q, "input_price": 2, "output_price": output_price}}


_ENV = {"stocks": {"capital": 100000000, "input": 100000000}}


def cfg_healthy(seeds: Mapping[str, int] = GATE_SEEDS) -> dict[str, Any]:
    """tests/tests/sanity.rs::cfg_healthy (original seeds 55/2/3/4; l_w 8)."""
    firms = [{"id": i, "stocks": {"capital": 500, "input": 500000},
              "reals": {"capability": 0.50 + i * 0.1, "legitimacy": 1.0,
                        "aspiration_capital_growth": 6.0}} for i in range(4)]
    return {"experiment": "sc-healthy", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
            "seeds": dict(seeds), "world": _world(400, 0.90, 0.20, 100, 3), "agents": firms,
            "environment": copy.deepcopy(_ENV),
            "rules": [{"id": "decision.satisficing", "version": "^1", "params": {"l_w": 8}},
                      *_market_actions(), *_constraint_rules(8)]}


def cfg_threaded(seeds: Mapping[str, int] = GATE_SEEDS) -> dict[str, Any]:
    """tests/tests/sanity.rs::cfg_threaded (original seeds 7/2/3/4; l_w 4)."""
    def firm(i, capital, inp, cap, asp=None, obligation=None):
        f: dict[str, Any] = {"id": i, "stocks": {"capital": capital, "input": inp},
                             "reals": {"capability": cap, "legitimacy": 1.0}}
        if asp is not None:
            f["reals"]["aspiration_capital_growth"] = asp
        if obligation is not None:
            f["ints"] = {"obligation": obligation}
        return f
    firms = [firm(0, 400, 500000, 0.55, 8.0), firm(1, 400, 500000, 0.55, 8.0),
             firm(2, 400, 500000, 0.60, 5.0), firm(3, 400, 500000, 0.70, 5.0),
             firm(4, 400, 500000, 0.80, 5.0), firm(5, 400, 500000, 0.90, 5.0),
             firm(6, 2, 0, 0.10), firm(7, 400, 500000, 0.25, 3.0),
             firm(8, 400, 500000, 0.50, obligation=9), firm(9, 2, 0, 0.10)]
    return {"experiment": "sc-thread", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
            "seeds": dict(seeds), "world": _world(400, 0.40, 0.35, 6, 3), "agents": firms,
            "environment": copy.deepcopy(_ENV),
            "rules": [{"id": "decision.satisficing", "version": "^1", "params": {"l_w": 4}},
                      *_market_actions(), *_constraint_rules(4)]}


def cfg_wide_search(w_max: int = WIDE_W_MAX, beta: float = WIDE_BETA,
                    seeds: Mapping[str, int] = GATE_SEEDS) -> dict[str, Any]:
    """tests/tests/sanity.rs::cfg_wide_search (original seeds 909/2/3/4; l_w 8)."""
    succ_c = {"p0": 0.30, "b_lambda": 0.20, "b_kappa": 0.10, "p_max": 0.60, "kappa_min": 25}
    lag = {"min": 2, "max": 6}
    shaping = {"lobby_cost": 25, "contract_cost": 25, "diversify_cost": 25,
               "lobby_success": {"lag": dict(lag),
                                 "success": {"p0": 0.25, "b_lambda": 0.2, "b_kappa": 0.1,
                                             "p_max": 0.75, "kappa_min": 25},
                                 "delta_theta": 0.10},
               "contract_success": {"lag": dict(lag), "success": dict(succ_c),
                                    "delta_q": 4, "q0": 3}}
    firms = [{"id": i, "stocks": {"capital": 100000, "input": 500000},
              "reals": {"capability": c, "legitimacy": 1.0,
                        "aspiration_capital_growth": 100000.0}} for i, c in ((0, 0.40), (1, 0.50))]
    return {"experiment": "sc-wide", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
            "seeds": dict(seeds), "world": _world(200, 0.90, 0.15, 100, 1), "agents": firms,
            "environment": copy.deepcopy(_ENV),
            "rules": [
                {"id": "decision.satisficing", "version": "^1",
                 "params": {"l_w": 8, "w_max": w_max, "beta": beta, "shaping": shaping}},
                *_market_actions(),
                {"id": "action.shaping.rdt_standard.lobby", "version": "^1", "params": {}},
                {"id": "action.shaping.rdt_standard.contract", "version": "^1",
                 "params": {"delta_q": 4, "q0": 3, "success": dict(succ_c), "lag": dict(lag)}},
                {"id": "action.shaping.rdt_standard.diversify", "version": "^1",
                 "params": {"success": dict(succ_c), "lag": dict(lag)}},
                *_constraint_rules(8)]}


GATE_CONFIGS = {"cfg_healthy": (cfg_healthy, 8, "all-healthy (SC-2 floor)"),
                "cfg_threaded": (cfg_threaded, 4, "threaded needle (closest)"),
                "cfg_wide_search": (cfg_wide_search, 8, None)}


def with_l_w(cfg: Mapping[str, Any], l_w: int) -> dict[str, Any]:
    out = copy.deepcopy(dict(cfg))
    for rid in bs.WINDOW_RULES:
        bs._rule(out, rid)["params"]["l_w"] = l_w
    return out


def report_lines(sc: Mapping[str, Any]) -> list[str]:
    """SC-1..SC-6 formatted exactly as tests/tests/sanity.rs::print_report
    formats them (the six value lines), for string comparison with the
    test's captured output."""
    mark = lambda b: "PASS" if b else "fail"  # noqa: E731
    binds = "[" + ", ".join("true" if b else "false" for b in sc["sc3_binds"]) + "]"
    first = "[" + ", ".join("None" if t is None else f"Some({t})"
                            for t in sc["sc3_first_bind_tick"]) + "]"
    s5 = sc["sc5_shaping_success"]
    s5_txt = "None" if s5 is None else f"Some({_rust_f64(round(s5 * 1000.0) / 1000.0)})"
    return [
        f"  SC-1 survival           = {sc['sc1_survival']:.3f}   [0.60, 0.90]   {mark(sc['sc1_pass'])}",
        f"  SC-2 survival-attention = {sc['sc2_survival_attention']:.3f}   [0.05, 0.25]   "
        f"{mark(sc['sc2_pass'])}   (reconstructed h<h_crit = {sc['sc2_reconstructed_h_below_crit']:.3f})",
        f"  SC-3 binds [sol,cmp,scp,obl] = {binds}   first@ {first}   {mark(sc['sc3_pass'])}",
        f"  SC-4 shaping fraction    = {sc['sc4_shaping_fraction']:.4f}   (> 0.05)      "
        f"{mark(sc['sc4_pass'])}   ({round(sc['sc4_shaping_fraction'] * sc['decisions'])}/"
        f"{sc['decisions']} decisions)",
        f"  SC-5 shaping success    = {s5_txt}   [0.10, 0.60]   {mark(sc['sc5_pass'])}   "
        f"({sc['shaping_commits']} commits)",
        f"  SC-6 entropy variance   = {sc['sc6_entropy_variance']:.4f}   (≥ 0.01)      "
        f"{mark(sc['sc6_pass'])}   (mean H_rep = {sc['sc6_entropy_mean']:.3f} bits)",
    ]


def _rust_f64(x: float) -> str:
    """Rust's ``{:?}`` for an f64 that is a rounded 3-decimal value (the
    shortest round-trip representation, as Python's ``repr``)."""
    return repr(float(x))


def gate_block(output: str, title: str) -> list[str]:
    """The six SC lines printed under ``=== <title> ===`` in the captured
    test output."""
    lines = output.splitlines()
    for i, line in enumerate(lines):
        if line.startswith(f"=== {title} ==="):
            return lines[i + 1:i + 7]
    raise ValueError(f"no '=== {title} ===' block in the captured gate output")


def wide_line(output: str, w_max: int, beta: float) -> str:
    prefix = f"  w_max={w_max} β={beta:.1f} → GOAL shaping"
    for line in output.splitlines():
        if line.startswith(prefix):
            return line
    raise ValueError(f"no '{prefix}' line in the captured gate output")


def wide_counts(events) -> tuple[str, int]:
    """The GOAL / NONE / SURVIVAL decision and shaping counts, formatted as
    sc4_wmax_beta_probe prints them (up to and excluding its '(predicted'
    clause)."""
    acts, focus = _decisions(events)
    rows = [(focus[k], a) for k, a in acts.items()]
    part = lambda sel: (sum(1 for f, a in rows if sel(f) and a >= 6),  # noqa: E731
                        sum(1 for f, _ in rows if sel(f)))
    g, n, s = part(lambda f: f >= 1), part(lambda f: f == -1), part(lambda f: f == 0)
    return (f"GOAL shaping {g[0]}/{g[1]} | NONE shaping {n[0]}/{n[1]} | SURVIVAL shaping "
            f"{s[0]}/{s[1]}"), len(rows)


def p5_verdict(by: Mapping[tuple[str, int], Mapping[str, Any]]) -> dict[str, Any]:
    """``by[(config, l_w)]`` = sanity report. The declared P5 rule."""
    changed: dict[str, list[str]] = {}
    for name in ("cfg_threaded", "cfg_healthy"):
        a, b = by[(name, 4)], by[(name, 8)]
        changed[name] = [k for k in SC_KEYS if a[k] != b[k]]
    sc23 = {"sc2_survival_attention", "sc2_reconstructed_h_below_crit", "sc3_binds"}
    holds = any(sc23 & set(v) for v in changed.values())
    falsified = all(not v for v in changed.values())
    return {"changed": changed,
            "verdict": "HOLDS" if holds else ("FALSIFIED" if falsified else "NEITHER")}


# ---------------------------------------------------------------------------
# Part 1: the trace
# ---------------------------------------------------------------------------


def _decisions(events) -> tuple[dict[tuple[int, int], int], dict[tuple[int, int], int]]:
    """``{(tick, agent): action}`` and ``{(tick, agent): focus}`` from decision.* records."""
    fc, vc = "kind.SetAgentInt.field", "kind.SetAgentInt.value"
    d = events[(events["event"] == "delta_applied")
               & events["origin"].astype(str).str.startswith("decision.")
               & events[fc].isin(["selected_action", "focus"])]
    acts, focus = {}, {}
    for t, a, f, v in zip(d["tick"], d["target.Agent"], d[fc], d[vc]):
        (acts if f == "selected_action" else focus)[(int(t), int(a))] = int(v)
    return acts, focus


def windows(events) -> dict[tuple[int, int], list[int]]:
    """``{(tick, agent): [action, ...]}``: the action window as rewritten by
    constraint.action_window in that tick (oldest first)."""
    lc, rc = "kind.ReplaceAgentList.list", "kind.ReplaceAgentList.records_json"
    out: dict[tuple[int, int], list[int]] = {}
    if lc not in events.columns:
        return out
    w = events[(events["event"] == "delta_applied") & (events[lc] == "action_window")]
    for t, a, recs in zip(w["tick"], w["target.Agent"], w[rc]):
        out[(int(t), int(a))] = [int(json.loads(r)["action"]) for r in recs]
    return out


def u_from_window(window: Sequence[int], l_w: int) -> float:
    """firma-domain margin::u_from_window: regulated entries among the last
    ``l_w``, divided by ``l_w``."""
    return sum(1 for a in list(window)[-l_w:] if a == PRODUCE_REGULATED) / l_w


def theta_path(events, initial: float, ticks: int) -> list[float]:
    """theta_limit in force at each tick's decision (deltas of that tick included)."""
    col = "kind.AdjustGlobalReal.field"
    per: dict[int, float] = collections.defaultdict(float)
    if col in events.columns:
        g = events[(events["event"] == "delta_applied") & (events[col] == "theta_limit")]
        for t, d in zip(g["tick"], g["kind.AdjustGlobalReal.delta"]):
            per[int(t)] += float(d)
    out, v = [], initial
    for t in range(ticks):
        v += per.get(t, 0.0)
        out.append(v)
    return out


def trace_rows(events, trace: Mapping[tuple[int, int], float], cfg: Mapping[str, Any],
               l_w: int, lo: int, hi: int) -> list[dict[str, Any]]:
    """One row per (firm, tick) with a decision in ``lo..hi``."""
    acts, focus = _decisions(events)
    win = windows(events)
    theta = theta_path(events, cfg["world"]["global_reals"]["theta_limit"], cfg["world"]["ticks"])
    fc = "kind.SetAgentInt.field"
    s = events[(events["event"] == "delta_applied") & (events[fc] == bs.STRIKE_FIELD)
               & events["origin"].astype(str).str.startswith("constraint.enforce")]
    strikes = {(int(t), int(a)) for t, a in zip(s["tick"], s["target.Agent"])}
    _, deaths = sl.decisions_and_deaths(events)
    rows = []
    for (t, a), act in sorted(acts.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        if not lo <= t <= hi:
            continue
        before = win.get((t - 1, a), [])
        u = u_from_window(before, l_w)
        rows.append({"agent": a, "tick": t, "action": act, "focus": focus.get((t, a)),
                     "u": u, "theta_limit": theta[t], "margin": theta[t] - u,
                     "h_t": trace.get((t, a)), "window_before": list(before)[-l_w:],
                     "strike": (t, a) in strikes,
                     "death": deaths.get(a, (None, None))[0] == t,
                     "death_cause": deaths[a][1] if deaths.get(a, (None,))[0] == t else None})
    return rows


def p4_verdict(rows: Sequence[Mapping[str, Any]], onset: int, h_crit: float) -> dict[str, Any]:
    """The declared P4 rule on the l_w = 4 trace rows."""
    by = {(r["agent"], r["tick"]): r for r in rows}
    firms = []
    for a in sorted({r["agent"] for r in rows}):
        ticks = sorted(r["tick"] for r in rows if r["agent"] == a and r["strike"] and r["tick"] >= onset)
        if not ticks:
            continue
        t = ticks[0]
        at, prev = by[(a, t)], by.get((a, t - 1))
        ok = at["margin"] >= h_crit and at["focus"] != 0
        firms.append({"agent": a, "strike_tick": t, "margin_at_T": at["margin"],
                      "focus_at_T": at["focus"], "action_at_T": at["action"], "u_at_T": at["u"],
                      "h_t_at_T": at["h_t"], "ok": ok,
                      "margin_at_T_minus_1": prev["margin"] if prev else None,
                      "focus_at_T_minus_1": prev["focus"] if prev else None})
    return {"struck_firms": firms,
            "verdict": "HOLDS" if firms and all(f["ok"] for f in firms) else "FALSIFIED"}


def run_traced(cfg: Mapping[str, Any], d: Path, firma: Path, trace: bool
               ) -> tuple[Any, dict[tuple[int, int], float], str]:
    shutil.rmtree(d, ignore_errors=True)
    d.mkdir(parents=True)
    saved = {k: os.environ.get(k) for k in ("FIRMA_TRACE_DECISION", "FIRMA_TRACE_DECISION_PATH")}
    try:
        if trace:
            os.environ["FIRMA_TRACE_DECISION"] = "1"
            os.environ["FIRMA_TRACE_DECISION_PATH"] = str(d / "trace.ndjson")
        else:
            os.environ.pop("FIRMA_TRACE_DECISION", None)
            os.environ.pop("FIRMA_TRACE_DECISION_PATH", None)
        err = sl.run_config(cfg, d, firma)
    finally:
        for k, v in saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
    if err:
        raise RuntimeError(err)
    h_t: dict[tuple[int, int], float] = {}
    if trace:
        for line in open(d / "trace.ndjson"):
            rec = json.loads(line)
            if rec.get("kind") == "selection":
                h_t[(int(rec["tick"]), int(rec["agent"]))] = float(rec["h_t"])
    events = load.read_events(d / "run")
    sha = hashlib.sha256((d / "run" / "events.ndjson").read_bytes()).hexdigest()
    return events, h_t, sha


# ---------------------------------------------------------------------------
# Part 3: configs
# ---------------------------------------------------------------------------


def p3_cells() -> list[dict[str, Any]]:
    out = []
    for i, combo in enumerate(itertools.product(*(levels for _, levels in P3_AXES))):
        cell = {"cell": i}
        cell.update({name: v for (name, _), v in zip(P3_AXES, combo)})
        out.append(cell)
    assert len(out) == 40
    return out


def thin_capital(init_seed: int, firm_id: int, level: str = "medium") -> int:
    """``round_half_up(0.2 x the firm's unrounded lognormal capital draw)``."""
    s = pg.SPEC["spread_levels"][level]
    sigma = math.sqrt(math.log(1.0 + s["cv_cap"] ** 2))
    raw = pg.SPEC["distributions"]["capital"]["median"] * math.exp(
        sigma * pg.standard_normal(init_seed, firm_id, pg.TAG_CAPITAL))
    return int(math.floor(THIN_FACTOR * raw + 0.5))


def p3_population(init_seed: int, cell: Mapping[str, Any],
                  template: Mapping[str, Any]) -> list[dict[str, Any]]:
    firms = dc.population(init_seed, "medium", "relative", template)
    if cell["obligation_probability"] == 0.0:
        for f in firms:
            f.pop("ints", None)
    n_thin = int(cell["thin_fraction"] * hc.N_FIRMS + 0.5)
    for f in firms[:n_thin]:
        f["stocks"]["capital"] = thin_capital(init_seed, f["id"])
    return firms


def build_p3_config(base: Mapping[str, Any], init_seed: int, cell: Mapping[str, Any], *,
                    shock_mean: float | None) -> dict[str, Any]:
    """Part 3. ``shock_mean=None`` is the no-shock baseline / fork."""
    shocked = shock_mean is not None
    cfg = bs.build_part2_config(base, init_seed, "medium", shocked=shocked)
    cfg["world"]["global_reals"]["theta_limit"] = cell["theta_limit"]
    cfg["world"]["global_ints"]["output_price"] = cell["output_price"]
    cfg["agents"] = p3_population(init_seed, cell, base["agents"][0])
    if shocked:
        p = bs._rule(cfg, "shock.stochastic")["params"]
        p["magnitude_mean"] = shock_mean
        p["magnitude_sd"] = shock_mean / 3.0
    return cfg


def p6_verdict(share: Mapping[int, float], cells: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """``share[cell index]`` = SC-3 pass share. The declared P6 rule."""
    levels = {}
    for lvl in P3_AXES[0][1]:
        idx = [c["cell"] for c in cells if c["theta_limit"] == lvl]
        levels[lvl] = {"best_cell_share": max(share[i] for i in idx),
                       "pooled_share": sum(share[i] for i in idx) / len(idx)}
    holds = any(v["best_cell_share"] >= 0.50 for v in levels.values())
    return {"levels": levels, "verdict": "HOLDS" if holds else "FALSIFIED"}


# ---------------------------------------------------------------------------
# Workers
# ---------------------------------------------------------------------------


def _baseline_worker(args: tuple[dict[str, Any], int, str, str]) -> dict[str, Any]:
    cell, seed, work, firma = args
    base = json.loads(rs.BASE_CONFIG.read_text())
    d = Path(work) / f"s1_{cell['cell']}_{seed}"
    shutil.rmtree(d, ignore_errors=True)
    err = sl.run_config(build_p3_config(base, seed, cell, shock_mean=None), d, Path(firma))
    if err:
        raise RuntimeError(err)
    rep = metrics.sanity_report(d / "run")
    sha = hashlib.sha256((d / "run" / "events.ndjson").read_bytes()).hexdigest()
    shutil.rmtree(d, ignore_errors=True)
    t1 = rs.evaluate_targets(rep, [], hc.N_FIRMS)
    out = {"cell": cell["cell"], "init_seed": seed, "baseline_events_sha256": sha,
           "baseline_survivors": rep["survivors"],
           "info_sc2_survival_attention": rep["sc2_survival_attention"]}
    out.update({k: t1[k] for k in ("sc1_survival", "sc1_ok", "sc2_h_below_crit", "sc2_ok",
                                   "sc3_binds", "sc3_ok", "sc6_entropy_variance", "sc6_ok", "T1")})
    return out


def _stage2_worker(args: tuple[dict[str, Any], float, int, str, str]) -> dict[str, Any]:
    cell, mean, seed, work, firma = args
    base = json.loads(rs.BASE_CONFIG.read_text())
    res = hc.evaluate_pair(
        f"s2_{cell['cell']}_{mean}_{seed}", hc.N_FIRMS,
        build_p3_config(base, seed, cell, shock_mean=None),
        build_p3_config(base, seed, cell, shock_mean=mean), Path(work), Path(firma))
    if res["failed"]:
        raise RuntimeError(res["failed"])
    res.update({"cell": cell["cell"], "shock_mean": mean, "init_seed": seed})
    return res


def _gate_worker(args: tuple[str, int, dict[str, int], str, str]) -> dict[str, Any]:
    name, l_w, seeds, work, firma = args
    cfg = with_l_w(GATE_CONFIGS[name][0](seeds=seeds), l_w)
    sl.check_seeds(cfg["seeds"])
    d = Path(work) / f"gate_{name}_{l_w}_{seeds['mechanism']}"
    shutil.rmtree(d, ignore_errors=True)
    err = sl.run_config(cfg, d, Path(firma))
    if err:
        raise RuntimeError(err)
    rep = metrics.sanity_report(d / "run")
    counts, total = wide_counts(load.read_events(d / "run"))
    sha = hashlib.sha256((d / "run" / "events.ndjson").read_bytes()).hexdigest()
    shutil.rmtree(d, ignore_errors=True)
    return {"config": name, "l_w": l_w, "mechanism_seed": seeds["mechanism"], "report": rep,
            "focus_counts": counts, "decisions_in_log": total, "events_sha256": sha}


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def _fmt_trace(rows: Sequence[Mapping[str, Any]], l_w: int) -> list[str]:
    out = []
    for a in sorted({r["agent"] for r in rows}):
        out.append(f"   firm {a}:  tick  action focus   u     theta   margin   h_t     window before "
                   "the decision (last l_w)   events")
        for r in rows:
            if r["agent"] != a:
                continue
            ev = " ".join(x for x in ("STRIKE" if r["strike"] else "",
                                      f"DIED({r['death_cause']})" if r["death"] else "") if x)
            h = "  n/a " if r["h_t"] is None else f"{r['h_t']:6.3f}"
            out.append(f"            {r['tick']:>4}    {r['action']}    {FOCUS_NAMES.get(r['focus'], '?'):<4}  "
                       f"{r['u']:.3f}  {r['theta_limit']:.3f}  {r['margin']:6.3f}  {h}   "
                       f"{str(r['window_before']):<34} {ev}")
    return out


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=BANNER)
    ap.add_argument("--write-spec", help="STEP 0: write md_spec.json to this path and exit")
    ap.add_argument("--spec", help="md_spec.json written at step 0 (required to run)")
    ap.add_argument("--gate-output", help="captured output of the existing sanity tests")
    ap.add_argument("--out", help="output directory (default: a temp dir)")
    ap.add_argument("--firma", help="path to the firma binary")
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--r22-evidence", default=str(REPO_ROOT / "docs/review/evidence/R22"))
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
    if not args.spec or not args.gate_output:
        print("REFUSED: --spec <md_spec.json> and --gate-output <captured test output> are required")
        return 1
    on_disk = Path(args.spec).read_bytes()
    if on_disk != spec_bytes():
        print("REFUSED: the spec file differs from the spec this script declares "
              f"(file sha256 {hashlib.sha256(on_disk).hexdigest()}, script {spec_sha256()})")
        return 1
    print(f"spec: {args.spec}  sha256 {spec_sha256()}")
    started = time.time()
    firma = sl.find_firma(args.firma)
    out = Path(args.out) if args.out else Path(tempfile.mkdtemp(prefix="firma-md-"))
    work = out / "runs"
    work.mkdir(parents=True, exist_ok=True)
    base = json.loads(rs.BASE_CONFIG.read_text())
    jobs = max(1, args.jobs)
    W, F = str(work), str(firma)
    gate_output = Path(args.gate_output).read_text()
    problems: list[str] = []
    cells = p3_cells()

    # -- controls ------------------------------------------------------------
    print("\n== controls ==")
    c0 = build_p3_config(base, 910_001, cells[0], shock_mean=None)
    shas = []
    for i in (1, 2):
        d = work / f"ctl_{i}"
        bs._run(c0, d, firma)
        shas.append(hashlib.sha256((d / "run" / "events.ndjson").read_bytes()).hexdigest())
        shutil.rmtree(d, ignore_errors=True)
    print(f"  part-3 cell 0, init seed 910001, baseline run twice: "
          f"{'BYTE-EQUAL' if shas[0] == shas[1] else 'DIFFERENT'}  {shas[0]}")
    if shas[0] != shas[1]:
        problems.append("byte-equality control")

    # -- part 1 --------------------------------------------------------------
    print(f"\n== PART 1: trace, ticks {TRACE_TICKS[0]}..{TRACE_TICKS[1]}, init seed {TRACE_SEED}, "
          f"onset {bs.ONSET}, magnitude {TRACE_MAGNITUDE}, h_crit {H_CRIT} ==")
    print("  action ids: 0 hold, 1 produce_ordinary, 2 produce_regulated, 3 acquire_input, "
          "4 invest_capability, 5 deliver. u is computed from the logged action window; h_t is "
          "the plugin's own decision-time margin from its diagnostic trace.")
    traces: dict[int, list[dict[str, Any]]] = {}
    for l_w in (4, 8):
        cfg = bs.build_part1_config(base, TRACE_SEED, l_w=l_w, h_crit=H_CRIT,
                                    magnitude=TRACE_MAGNITUDE)
        events, h_t, sha_on = run_traced(cfg, work / f"trace_{l_w}", firma, trace=True)
        _, _, sha_off = run_traced(cfg, work / f"trace_{l_w}_off", firma, trace=False)
        rows = trace_rows(events, h_t, cfg, l_w, *TRACE_TICKS)
        traces[l_w] = rows
        print(f"\n  l_w = {l_w}: event log with the trace on vs off: "
              f"{'BYTE-EQUAL' if sha_on == sha_off else 'DIFFERENT'}; decisions without a trace "
              f"h_t: {sum(r['h_t'] is None for r in rows)} of {len(rows)}")
        if sha_on != sha_off:
            problems.append(f"trace changes the event log at l_w {l_w}")
        mism = [r for r in rows if r["h_t"] is not None and (r["h_t"] < H_CRIT) != (r["focus"] == 0)]
        print(f"  consistency: decisions where (h_t < h_crit) disagrees with SURVIVAL focus: {len(mism)}")
        for line in _fmt_trace(rows, l_w):
            print(line)
        shutil.rmtree(work / f"trace_{l_w}", ignore_errors=True)
        shutil.rmtree(work / f"trace_{l_w}_off", ignore_errors=True)
    v4 = p4_verdict(traces[4], bs.ONSET, H_CRIT)
    print(f"\n  P4 (l_w = 4): {v4['verdict']} - struck firms: {len(v4['struck_firms'])}")
    for f in v4["struck_firms"]:
        print(f"    firm {f['agent']}: first strike at tick {f['strike_tick']}; decision in that "
              f"tick: action {f['action_at_T']}, u {f['u_at_T']:.3f}, margin {f['margin_at_T']:.3f}, "
              f"h_t {f['h_t_at_T']}, focus {FOCUS_NAMES.get(f['focus_at_T'])} -> "
              f"{'ok' if f['ok'] else 'NOT ok'}; at T-1: margin {f['margin_at_T_minus_1']}, focus "
              f"{FOCUS_NAMES.get(f['focus_at_T_minus_1'])}")
    r8 = traces[8]
    struck8 = sorted({r["agent"] for r in r8 if r["strike"] and r["tick"] >= bs.ONSET})
    first_surv = {a: min((r["tick"] for r in r8 if r["agent"] == a and r["focus"] == 0
                          and r["tick"] >= bs.ONSET), default=None)
                  for a in sorted({r["agent"] for r in r8})}
    print(f"  l_w = 8 (no prediction): firms struck in {bs.ONSET}..{TRACE_TICKS[1]}: {struck8}; "
          f"first tick >= {bs.ONSET} with SURVIVAL focus per firm: {first_surv}; smallest "
          f"decision-time margin per firm from tick {bs.ONSET}: "
          f"{ {a: round(min(r['margin'] for r in r8 if r['agent'] == a and r['tick'] >= bs.ONSET), 3) for a in first_surv} }")

    # -- part 2 --------------------------------------------------------------
    print("\n== PART 2: the Phase-2 gate configs at l_w = 4 and l_w = 8 ==")
    print(f"  reserved seeds are not executed; transcriptions run at seeds {GATE_SEEDS}")
    items = [(name, l_w, GATE_SEEDS, W, F) for name in GATE_CONFIGS for l_w in (4, 8)]
    items += [(name, GATE_CONFIGS[name][1], GATE_SEEDS_2, W, F) for name in GATE_CONFIGS]
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        gate = list(pool.map(_gate_worker, items))
    by = {(g["config"], g["l_w"]): g for g in gate if g["mechanism_seed"] == GATE_SEEDS["mechanism"]}
    print("  validation of the transcription at the original l_w against the existing test's output:")
    for name, (_, l_w0, title) in GATE_CONFIGS.items():
        g = by[(name, l_w0)]
        other = next(x for x in gate if x["config"] == name
                     and x["mechanism_seed"] == GATE_SEEDS_2["mechanism"])
        same_log = other["events_sha256"] == g["events_sha256"]
        if title is not None:
            mine, theirs = report_lines(g["report"]), gate_block(gate_output, title)
            ok = mine == theirs
            print(f"   {name} (l_w {l_w0}): six SC lines {'IDENTICAL to' if ok else 'DIFFER FROM'} "
                  f"the test's printed block '{title}'")
            for a, b in zip(mine, theirs):
                print(f"      {'=' if a == b else 'X'} {a.strip()}" + ("" if a == b else f"\n        test: {b.strip()}"))
        else:
            theirs = wide_line(gate_output, WIDE_W_MAX, WIDE_BETA)
            ok = theirs.startswith(f"  w_max={WIDE_W_MAX} β={WIDE_BETA:.1f} → {g['focus_counts']} (")
            ok = ok and theirs.rstrip().endswith(f"| total {g['decisions_in_log']}")
            print(f"   {name}({WIDE_W_MAX}, {WIDE_BETA}) (l_w {l_w0}): focus/shaping counts "
                  f"{'IDENTICAL to' if ok else 'DIFFER FROM'} sc4_wmax_beta_probe's line")
            print(f"      mine: {g['focus_counts']} | total {g['decisions_in_log']}")
            print(f"      test: {theirs.strip()}")
        print(f"      seed-insensitivity: event log at seeds 910001.. vs 911001..: "
              f"{'BYTE-EQUAL' if same_log else 'DIFFERENT'}")
        if not ok:
            problems.append(f"transcription of {name} does not reproduce the test output")
        if not same_log:
            problems.append(f"{name} depends on its seeds; the reserved-seed numbers are not "
                            "guaranteed to be reproduced")
    print("  SC-1..SC-6 for every (config, l_w):")
    print("   config           l_w  SC-1   SC-2 attn  SC-2 recon  SC-3 [sol,cmp,scp,obl]  first bind "
          "ticks              SC-4 (shaping/decisions)  SC-5   SC-6 var  survivors")
    for name in GATE_CONFIGS:
        for l_w in (4, 8):
            r = by[(name, l_w)]["report"]
            b = "".join("Y" if x else "n" for x in r["sc3_binds"])
            orig = " (original)" if l_w == GATE_CONFIGS[name][1] else ""
            print(f"   {name:<16} {l_w:>2}   {r['sc1_survival']:.3f}  {r['sc2_survival_attention']:.3f}      "
                  f"{r['sc2_reconstructed_h_below_crit']:.3f}       {b}                     "
                  f"{str(r['sc3_first_bind_tick']):<30} {r['sc4_shaping_fraction']:.4f} "
                  f"({round(r['sc4_shaping_fraction'] * r['decisions'])}/{r['decisions']})"
                  f"          {r['sc5_shaping_success']}   {r['sc6_entropy_variance']:.4f}    "
                  f"{r['survivors']}/{r['initial_firms']}{orig}")
    v5 = p5_verdict({k: v["report"] for k, v in by.items()})
    print(f"  P5: {v5['verdict']} - values that differ between l_w = 4 and l_w = 8: {v5['changed']}")
    wide_changed = [k for k in SC_KEYS if by[("cfg_wide_search", 4)]["report"][k]
                    != by[("cfg_wide_search", 8)]["report"][k]]
    print(f"  cfg_wide_search (not part of P5): values that differ: {wide_changed}")

    # -- part 3 --------------------------------------------------------------
    print("\n== PART 3, STAGE 1: 40 cells x 20 init seeds, no-shock baselines, l_w = 8, h_crit = 0.15 ==")
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        s1 = list(pool.map(_baseline_worker, [(c, s, W, F) for c in cells for s in P3_SEEDS]))
    r22 = {int(r["init_seed"]): r for r in csv.DictReader(open(Path(args.r22_evidence) / "part2_runs.csv"))
           if r["level"] == "medium" and int(r["init_seed"]) in P3_SEEDS}
    ref = next(c for c in cells if c["theta_limit"] == 0.90 and c["obligation_probability"] > 0
               and c["thin_fraction"] == 0.0 and c["output_price"] == 3)
    same = [r for r in s1 if r["cell"] == ref["cell"]]
    ok22 = all(r["baseline_events_sha256"] == r22[r["init_seed"]]["baseline_events_sha256"]
               and r["sc1_survival"] == float(r22[r["init_seed"]]["sc1_survival"])
               and r["sc2_h_below_crit"] == float(r22[r["init_seed"]]["sc2_h_below_crit"])
               and r["sc3_binds"] == json.loads(r22[r["init_seed"]]["sc3_binds"])
               and r["sc6_entropy_variance"] == float(r22[r["init_seed"]]["sc6_entropy_variance"])
               for r in same)
    print(f"  control: cell {ref['cell']} (theta_limit 0.90, obligation 1/6, thin 0, price 3) "
          f"{'REPRODUCES' if ok22 else 'DIFFERS FROM'} the R22 l_w = 8 baselines for init seeds "
          f"910001..910020 (SC-1, SC-2, SC-3 binds, SC-6 and the event-log sha256)")
    if not ok22:
        problems.append("R22 baseline control")
    print("  cell thL   oblig thin  price | pass share: SC-1  SC-2  SC-3  SC-6  T1   | SC-1 mean  "
          "SC-2 mean [min, max]    binds of 20 [sol,cmp,scp,obl]  baseline survivors")
    cell_rows, sc3_share = [], {}
    for c in cells:
        rows = [r for r in s1 if r["cell"] == c["cell"]]
        sh = lambda k: sum(bool(r[k]) for r in rows) / len(rows)  # noqa: E731
        sc2 = [r["sc2_h_below_crit"] for r in rows]
        binds = [sum(bool(r["sc3_binds"][i]) for r in rows) for i in range(4)]
        row = {**c, "sc1_share": sh("sc1_ok"), "sc2_share": sh("sc2_ok"), "sc3_share": sh("sc3_ok"),
               "sc6_share": sh("sc6_ok"), "t1_share": sh("T1"),
               "sc1_mean": statistics.fmean(r["sc1_survival"] for r in rows),
               "sc2_mean": statistics.fmean(sc2), "sc2_min": min(sc2), "sc2_max": max(sc2),
               "binds": binds,
               "survivors": dict(sorted(collections.Counter(r["baseline_survivors"] for r in rows).items()))}
        cell_rows.append(row)
        sc3_share[c["cell"]] = row["sc3_share"]
        print(f"  {c['cell']:>3}  {c['theta_limit']:.2f}  {'1/6' if c['obligation_probability'] else '0  '}   "
              f"{'1/6' if c['thin_fraction'] else '0  '}   {c['output_price']}    |             "
              f"{row['sc1_share']:.2f}  {row['sc2_share']:.2f}  {row['sc3_share']:.2f}  "
              f"{row['sc6_share']:.2f}  {row['t1_share']:.2f} | {row['sc1_mean']:.3f}      "
              f"{row['sc2_mean']:.3f} [{row['sc2_min']:.3f}, {row['sc2_max']:.3f}]   "
              f"{str(binds):<18}             {row['survivors']}")
    v6 = p6_verdict(sc3_share, cells)
    print(f"  P6: {v6['verdict']} - SC-3 pass share by theta_limit level (best cell / pooled over "
          f"8 cells): " + "; ".join(f"{lvl:.2f}: {x['best_cell_share']:.2f} / {x['pooled_share']:.2f}"
                                    for lvl, x in v6["levels"].items()))
    for key, label in (("sc1_share", "SC-1"), ("sc2_share", "SC-2"), ("sc3_share", "SC-3"),
                       ("sc6_share", "SC-6"), ("t1_share", "T1")):
        print(f"    cells with {label} pass share >= 0.50: "
              f"{sum(r[key] >= 0.50 for r in cell_rows)} of 40; highest share {max(r[key] for r in cell_rows):.2f}")

    qualifying = [r for r in cell_rows if r["t1_share"] >= 0.50]
    print(f"\n== PART 3, STAGE 2: cells with T1 pass share >= 0.50: {len(qualifying)} ==")
    s2_rows, s2 = [], []
    if not qualifying:
        print("  no cell qualifies; Stage 2 has nothing to run")
    else:
        items2 = [({k: r[k] for k in ("cell", *[n for n, _ in P3_AXES])}, m, s, W, F)
                  for r in qualifying for m in P3_SHOCK_MEANS for s in P3_SEEDS]
        with ProcessPoolExecutor(max_workers=jobs) as pool:
            s2 = list(pool.map(_stage2_worker, items2))
        for r in qualifying:
            for m in P3_SHOCK_MEANS:
                rows = [x for x in s2 if x["cell"] == r["cell"] and x["shock_mean"] == m]
                s = hc.summarise(rows)
                passes = (s["collapse_rate"] <= 0.10 and s["nonzero_matched_delta_h_share"] >= 0.50
                          and r["t1_share"] >= 0.50)
                s2_rows.append({"cell": r["cell"], "shock_mean": m, "t1_share": r["t1_share"],
                                **s, "pass": passes})
                print(f"  cell {r['cell']:>2} (thL {r['theta_limit']:.2f}, oblig "
                      f"{'1/6' if r['obligation_probability'] else '0'}, thin "
                      f"{'1/6' if r['thin_fraction'] else '0'}, price {r['output_price']}) mean {m:.2f}: "
                      f"collapse {s['collapse_rate']:.2f}; non-zero dH "
                      f"{s['nonzero_matched_delta_h_share']:.2f}; T1 share {r['t1_share']:.2f}; mean "
                      f"survival {s['mean_survival_shocked']:.3f} (baseline "
                      f"{s['mean_survival_baseline']:.3f}); >= 5 deaths in a tick "
                      f"{s['mass_death_tick_share']:.2f}; runs with a compliance death "
                      f"{s['runs_with_a_compliance_death']}; survivors "
                      f"{s['shocked_survivors_distribution']} -> {'PASS' if passes else 'FAIL'}")
    print(f"  Stage-2 rows passing the R20 Q4 criteria: {sum(r['pass'] for r in s2_rows)} of "
          f"{len(s2_rows)}; cells with at least one passing row: "
          f"{sorted({r['cell'] for r in s2_rows if r['pass']})}")

    # -- files ---------------------------------------------------------------
    with open(out / "part1_trace.csv", "w", newline="") as f:
        fields = ["l_w", "agent", "tick", "action", "focus", "u", "theta_limit", "margin", "h_t",
                  "window_before", "strike", "death", "death_cause"]
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        for l_w, rows in traces.items():
            for r in rows:
                w.writerow({"l_w": l_w, **{k: (json.dumps(r[k]) if isinstance(r[k], list) else r[k])
                                           for k in fields if k != "l_w"}})
    with open(out / "part3_stage1_runs.csv", "w", newline="") as f:
        fields = list(s1[0])
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        for r in s1:
            w.writerow({k: (json.dumps(r[k]) if isinstance(r[k], list) else r[k]) for k in fields})
    with open(out / "part3_stage2_runs.csv", "w", newline="") as f:
        fields = ["cell", "shock_mean", "init_seed", "T1", "baseline_survivors", "shocked_survivors",
                  "shocked_collapse", "shocked_compliance_deaths", "shocked_max_deaths_in_one_tick",
                  "delta_h_rep_matched", "nonzero_matched_delta_h", "drawn_onset", "drawn_magnitude",
                  "shocked_death_records", "baseline_events_sha256", "shocked_events_sha256"]
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        for r in s2:
            w.writerow({k: (json.dumps(r[k]) if isinstance(r[k], list) else r[k]) for k in fields})
    (out / "summary.json").write_text(json.dumps({
        "spec_sha256": spec_sha256(), "P4": v4, "P5": v5, "P6": {
            "verdict": v6["verdict"], "levels": {str(k): v for k, v in v6["levels"].items()}},
        "part2": [{"config": g["config"], "l_w": g["l_w"], "mechanism_seed": g["mechanism_seed"],
                   "events_sha256": g["events_sha256"], "focus_counts": g["focus_counts"],
                   "report": g["report"]} for g in gate],
        "part3_cells": cell_rows, "part3_stage2": s2_rows, "problems": problems}, indent=1))
    print(f"\nwritten under {out}: part1_trace.csv, part3_stage1_runs.csv, part3_stage2_runs.csv, "
          "summary.json")
    print(f"PREDICTIONS: P4 {v4['verdict']}; P5 {v5['verdict']}; P6 {v6['verdict']}")
    print(f"problems: {problems if problems else 'none'}")
    print(f"wall-clock time: {time.time() - started:.1f} s")
    print(BANNER)
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
