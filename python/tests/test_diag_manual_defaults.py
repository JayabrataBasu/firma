"""Tests for scripts/diag_manual_defaults.py (review R23; DIAGNOSTIC-ONLY)."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import re
from pathlib import Path

import pandas as pd
import pytest

REPO = Path(__file__).resolve().parents[2]
FIRMA_BIN = REPO / "target" / "debug" / "firma"
SPEC_FILE = REPO / "docs" / "review" / "evidence" / "R23" / "md_spec.json"
SANITY_RS = (REPO / "tests" / "tests" / "sanity.rs").read_text()


def _load(name):
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


md = _load("diag_manual_defaults")
bs, dc, hc = md.bs, md.dc, md.hc
BASE = json.loads(md.rs.BASE_CONFIG.read_text())


# -- spec ---------------------------------------------------------------------


@pytest.mark.skipif(not SPEC_FILE.exists(), reason="step-0 md_spec.json not present")
def test_the_step_0_spec_file_is_exactly_the_spec_the_script_declares():
    assert SPEC_FILE.read_bytes() == md.spec_bytes()
    assert hashlib.sha256(SPEC_FILE.read_bytes()).hexdigest() == md.spec_sha256()


def test_step_0_never_overwrites_and_a_changed_or_missing_spec_is_refused(tmp_path, capsys):
    p, g = tmp_path / "spec.json", tmp_path / "gate.txt"
    g.write_text("")
    assert md.main(["--write-spec", str(p)]) == 0
    assert md.main(["--write-spec", str(p)]) == 1
    p.write_bytes(md.spec_bytes().replace(b"0.50", b"0.40", 1))
    assert md.main(["--spec", str(p), "--gate-output", str(g), "--out", str(tmp_path / "o")]) == 1
    assert md.main(["--out", str(tmp_path / "o")]) == 1
    assert "REFUSED" in capsys.readouterr().out and not (tmp_path / "o").exists()


def test_the_predictions_are_in_the_spec_verbatim():
    p = md.SPEC["predictions_verbatim_from_the_R23_prompt"]
    assert p["P4"] == ("P4 (part 1) the l_w = 4 first violation is preceded by a tick with margin "
                       ">= 0.15 and non-survival focus")
    assert "falsified if all six SC values are unchanged for both configs" in p["P5"]
    assert "at least one theta_limit level makes SC-3 pass in >= 50% of populations" in p["P6"]


# -- part 2: the transcription, checked mechanically against the Rust source --


def _raw(after: str, starts_with: str) -> str:
    """The first Rust raw string ``r#"..."#`` after ``after`` whose content
    (stripped) starts with ``starts_with``."""
    body = SANITY_RS[SANITY_RS.index(after):]
    for m in re.finditer(r'r#"(.*?)"#', body, re.S):
        if m.group(1).strip().startswith(starts_with):
            return m.group(1)
    raise AssertionError(f"no raw string starting {starts_with!r} after {after!r}")


def _unescape(fmt: str, named: dict, positional: list[str]) -> str:
    """Apply Rust ``format!`` substitution to a format string."""
    pos = iter(positional)
    out, i = [], 0
    while i < len(fmt):
        if fmt.startswith("{{", i):
            out.append("{"); i += 2
        elif fmt.startswith("}}", i):
            out.append("}"); i += 2
        elif fmt[i] == "{":
            j = fmt.index("}", i)
            key = fmt[i + 1:j]
            out.append(next(pos) if key == "" else str(named[key]))
            i = j + 1
        else:
            out.append(fmt[i]); i += 1
    assert next(pos, None) is None, "unused positional argument"
    return "".join(out)


MARKET_ACTIONS_RS = _raw("const MARKET_ACTIONS", '{ "id": "action.market.standard.hold"')


def _constraint_rules_rs(l_w: int) -> str:
    return _unescape(_raw("fn constraint_rules", '{{ "id": "constraint.action_window"'),
                     {"l_w": l_w}, [])


def _from_rust(fn: str, positional: list[str], **named) -> dict:
    fmt = _raw(f"fn {fn}(", '{{ "experiment"')
    return json.loads(_unescape(fmt, {"MARKET_ACTIONS": MARKET_ACTIONS_RS, **named}, positional))


def test_cfg_threaded_equals_the_rust_source():
    rust = _from_rust("cfg_threaded", [_constraint_rules_rs(4)])
    assert rust["seeds"] == {"mechanism": 7, "environment": 2, "shock": 3, "init": 4}
    assert md.cfg_threaded(seeds=rust["seeds"]) == rust
    assert md.cfg_threaded()["seeds"] == md.GATE_SEEDS and min(md.GATE_SEEDS.values()) >= 910_001


def test_cfg_healthy_equals_the_rust_source():
    firm_fmt = _raw("fn cfg_healthy(", '{{ "id": {i}')
    firms = [_unescape(firm_fmt, {"i": i, "c": repr(0.50 + i * 0.1)}, []) for i in range(4)]
    rust = _from_rust("cfg_healthy", [",".join(firms), _constraint_rules_rs(8)])
    assert rust["seeds"] == {"mechanism": 55, "environment": 2, "shock": 3, "init": 4}
    assert md.cfg_healthy(seeds=rust["seeds"]) == rust
    assert [a["reals"]["capability"] for a in rust["agents"]] == [0.5, 0.6, 0.7, 0.8]


def test_cfg_wide_search_equals_the_rust_source():
    rust = _from_rust("cfg_wide_search", [_constraint_rules_rs(8)], w_max=6, beta=1.0)
    assert rust["seeds"] == {"mechanism": 909, "environment": 2, "shock": 3, "init": 4}
    assert md.cfg_wide_search(6, 1.0, seeds=rust["seeds"]) == rust


def test_with_l_w_changes_only_the_three_l_w_values():
    for name, (fn, l_w0, _) in md.GATE_CONFIGS.items():
        cfg = fn()
        assert md.with_l_w(cfg, l_w0) == cfg
        other = md.with_l_w(cfg, 12 - l_w0)
        assert bs.window_settings(other)["constraint.enforce.l_w"] == 12 - l_w0
        assert md.with_l_w(other, l_w0) == cfg and cfg == fn()  # not mutated


def test_report_lines_use_the_rust_print_format():
    sc = {"sc1_survival": 0.8, "sc1_pass": True, "sc2_survival_attention": 1.0, "sc2_pass": False,
          "sc2_reconstructed_h_below_crit": 0.99949, "sc3_binds": [True, False, True, True],
          "sc3_first_bind_tick": [0, None, 3, 0], "sc3_pass": False, "sc4_shaping_fraction": 0.0,
          "sc4_pass": False, "decisions": 3202, "sc5_shaping_success": None, "sc5_pass": False,
          "shaping_commits": 0, "sc6_entropy_variance": 0.00004, "sc6_pass": False,
          "sc6_entropy_mean": 0.0123}
    lines = md.report_lines(sc)
    assert lines[0] == "  SC-1 survival           = 0.800   [0.60, 0.90]   PASS"
    assert lines[1] == ("  SC-2 survival-attention = 1.000   [0.05, 0.25]   fail   "
                        "(reconstructed h<h_crit = 0.999)")
    assert lines[2] == ("  SC-3 binds [sol,cmp,scp,obl] = [true, false, true, true]   "
                        "first@ [Some(0), None, Some(3), Some(0)]   fail")
    assert lines[3] == "  SC-4 shaping fraction    = 0.0000   (> 0.05)      fail   (0/3202 decisions)"
    assert lines[4] == "  SC-5 shaping success    = None   [0.10, 0.60]   fail   (0 commits)"
    assert lines[5] == "  SC-6 entropy variance   = 0.0000   (≥ 0.01)      fail   (mean H_rep = 0.012 bits)"
    assert "Some(0.25)" in md.report_lines(dict(sc, sc5_shaping_success=0.2504))[4]


def test_gate_block_and_wide_line_are_read_from_captured_output():
    text = ("noise\n\n=== threaded needle (closest) ===  (10 firms → 8 survivors)\n"
            "l1\nl2\nl3\nl4\nl5\nl6\n  ALL SIX: fail\n"
            "  w_max=6 β=1.0 → GOAL shaping 0/10 | NONE shaping 0/5 | SURVIVAL shaping 1/2 (predicted) | total 17\n")
    assert md.gate_block(text, "threaded needle (closest)") == ["l1", "l2", "l3", "l4", "l5", "l6"]
    assert md.wide_line(text, 6, 1.0).endswith("| total 17")
    with pytest.raises(ValueError):
        md.gate_block(text, "all-healthy (SC-2 floor)")


def _rep(**over):
    r = {"sc1_survival": 0.8, "sc2_survival_attention": 1.0, "sc2_reconstructed_h_below_crit": 1.0,
         "sc3_binds": [True, False, True, True], "sc4_shaping_fraction": 0.0,
         "sc5_shaping_success": None, "sc6_entropy_variance": 0.0}
    r.update(over)
    return r


def test_p5_holds_falsified_and_neither():
    same = {(n, l): _rep() for n in ("cfg_threaded", "cfg_healthy") for l in (4, 8)}
    assert md.p5_verdict(same)["verdict"] == "FALSIFIED"
    sc3 = dict(same)
    sc3[("cfg_healthy", 4)] = _rep(sc3_binds=[True, True, True, True])
    v = md.p5_verdict(sc3)
    assert v["verdict"] == "HOLDS" and v["changed"] == {"cfg_threaded": [], "cfg_healthy": ["sc3_binds"]}
    sc2 = dict(same)
    sc2[("cfg_threaded", 8)] = _rep(sc2_survival_attention=0.4)
    assert md.p5_verdict(sc2)["verdict"] == "HOLDS"
    only_sc6 = dict(same)
    only_sc6[("cfg_threaded", 8)] = _rep(sc6_entropy_variance=0.2)
    assert md.p5_verdict(only_sc6)["verdict"] == "NEITHER"


# -- part 1 arithmetic --------------------------------------------------------


def test_u_is_the_regulated_share_of_the_last_l_w_entries():
    assert md.u_from_window([2, 2, 1, 2], 4) == 0.75
    assert md.u_from_window([2, 2, 2, 2, 1, 1, 2, 0], 4) == 0.25      # only the last four
    assert md.u_from_window([2, 2, 2, 2, 1, 1, 2, 0], 8) == 0.625
    assert md.u_from_window([2], 4) == 0.25 and md.u_from_window([], 4) == 0.0


def _ev(tick, agent, field, value, origin="decision.satisficing"):
    return {"event": "delta_applied", "tick": tick, "origin": origin, "target": {"Agent": agent},
            "kind": {"SetAgentInt": {"field": field, "value": value}}}


def _win(tick, agent, actions):
    return {"event": "delta_applied", "tick": tick, "origin": "constraint.action_window",
            "target": {"Agent": agent},
            "kind": {"ReplaceAgentList": {"list": "action_window", "records_json": [
                json.dumps({"tick": i, "action": a}) for i, a in enumerate(actions)]}}}


def _story_events():
    """One firm, l_w 4: u = 0.5 before tick 41, theta drops to 0.7 at tick 40; it takes a
    regulated step at 41 (margin 0.2, GOAL focus), reaching u = 0.75 > 0.7: struck at 41."""
    rows = [_win(39, 0, [1, 1, 2, 2]),
            {"event": "delta_applied", "tick": 40, "origin": "shock.stochastic", "target": "Global",
             "kind": {"AdjustGlobalReal": {"field": "theta_limit", "delta": -0.2}}},
            _ev(40, 0, "selected_action", 1), _ev(40, 0, "focus", 1), _win(40, 0, [1, 2, 2, 1]),
            _ev(41, 0, "selected_action", 2), _ev(41, 0, "focus", 1), _win(41, 0, [2, 2, 1, 2]),
            _ev(41, 0, "compliance_last_violation_tick", 41, origin="constraint.enforce"),
            _ev(42, 0, "selected_action", 1), _ev(42, 0, "focus", 0), _win(42, 0, [2, 1, 2, 1]),
            {"event": "agent_died", "tick": 43, "agent": 0, "cause": "compliance"}]
    return pd.json_normalize(rows)


CFG = {"world": {"global_reals": {"theta_limit": 0.9}, "ticks": 50}}


def test_trace_rows_use_the_window_before_the_decision_and_the_theta_in_force():
    rows = md.trace_rows(_story_events(), {(41, 0): 0.2}, CFG, 4, 40, 42)
    by = {r["tick"]: r for r in rows}
    assert by[40]["u"] == 0.5 and by[40]["theta_limit"] == pytest.approx(0.7)
    assert by[41]["u"] == 0.5 and by[41]["margin"] == pytest.approx(0.2) and by[41]["strike"]
    assert by[41]["h_t"] == 0.2 and by[40]["h_t"] is None and by[41]["focus"] == 1
    assert by[42]["u"] == 0.75 and by[42]["margin"] == pytest.approx(-0.05) and by[42]["focus"] == 0
    assert md.theta_path(_story_events(), 0.9, 42)[39:42] == pytest.approx([0.9, 0.7, 0.7])


def test_p4_holds_on_the_hypothesised_story_and_is_falsified_otherwise():
    rows = md.trace_rows(_story_events(), {}, CFG, 4, 40, 42)
    v = md.p4_verdict(rows, onset=40, h_crit=0.15)
    assert v["verdict"] == "HOLDS" and v["struck_firms"][0]["strike_tick"] == 41
    assert v["struck_firms"][0]["margin_at_T"] == pytest.approx(0.2)
    survival = [dict(r, focus=0) if r["tick"] == 41 else r for r in rows]
    assert md.p4_verdict(survival, 40, 0.15)["verdict"] == "FALSIFIED"
    thin = [dict(r, margin=0.1) if r["tick"] == 41 else r for r in rows]
    assert md.p4_verdict(thin, 40, 0.15)["verdict"] == "FALSIFIED"
    nobody = [dict(r, strike=False) for r in rows]
    assert md.p4_verdict(nobody, 40, 0.15)["verdict"] == "FALSIFIED"


# -- part 3 -------------------------------------------------------------------


def test_part_3_has_40_cells_in_product_order():
    c = md.p3_cells()
    assert len(c) == 40 and [x["cell"] for x in c] == list(range(40))
    assert c[0] == {"cell": 0, "theta_limit": 0.50, "obligation_probability": 0.0,
                    "thin_fraction": 0.0, "output_price": 2}
    assert c[1]["output_price"] == 3 and c[2]["thin_fraction"] == 1 / 6
    assert c[4]["obligation_probability"] == 1 / 6 and c[8]["theta_limit"] == 0.60
    assert c[39] == {"cell": 39, "theta_limit": 0.90, "obligation_probability": 1 / 6,
                     "thin_fraction": 1 / 6, "output_price": 3}


def test_part_3_population_axes():
    t = BASE["agents"][0]
    ref = dc.population(910_005, "medium", "relative", t)
    full = md.p3_population(910_005, {"obligation_probability": 1 / 6, "thin_fraction": 0.0}, t)
    assert full == ref
    none = md.p3_population(910_005, {"obligation_probability": 0.0, "thin_fraction": 0.0}, t)
    assert all("ints" not in f for f in none)
    assert [f["stocks"] for f in none] == [f["stocks"] for f in ref]
    thin = md.p3_population(910_005, {"obligation_probability": 1 / 6, "thin_fraction": 1 / 6}, t)
    assert [f["stocks"]["capital"] for f in thin[2:]] == [f["stocks"]["capital"] for f in ref[2:]]
    for i in (0, 1):
        assert thin[i]["stocks"]["capital"] == md.thin_capital(910_005, i)
        assert abs(thin[i]["stocks"]["capital"] - 0.2 * ref[i]["stocks"]["capital"]) <= 0.6
        assert thin[i]["reals"] == ref[i]["reals"]


def test_part_3_default_cell_is_the_r22_part_2_config_and_the_fork_differs_only_by_the_shock():
    cell = md.p3_cells()[37]
    assert (cell["theta_limit"], cell["thin_fraction"], cell["output_price"]) == (0.90, 0.0, 3)
    assert md.build_p3_config(BASE, 910_003, cell, shock_mean=None) == bs.build_part2_config(
        BASE, 910_003, "medium", shocked=False)
    assert md.build_p3_config(BASE, 910_003, cell, shock_mean=0.20) == bs.build_part2_config(
        BASE, 910_003, "medium", shocked=True)
    c0 = md.p3_cells()[0]
    s = md.build_p3_config(BASE, 910_003, c0, shock_mean=0.30)
    f = md.build_p3_config(BASE, 910_003, c0, shock_mean=None)
    p = next(r for r in s["rules"] if r["id"] == "shock.stochastic")["params"]
    assert p["magnitude_mean"] == 0.30 and p["magnitude_sd"] == pytest.approx(0.1)
    assert [r for r in s["rules"] if r["id"] != "shock.stochastic"] == f["rules"]
    assert s["world"]["global_reals"]["theta_limit"] == 0.50
    assert s["world"]["global_ints"] == {"theta_q": 40, "input_price": 2, "output_price": 2}
    assert bs.window_settings(s) == {"decision.satisficing.l_w": 8, "constraint.action_window.l_w": 8,
                                     "constraint.enforce.l_w": 8, "decision.satisficing.h_crit": 0.15}
    assert s["seeds"]["shock"] == s["seeds"]["init"] == 910_003


def test_p6_is_per_cell_at_a_level_and_reports_the_pooled_share():
    cells = md.p3_cells()
    share = {c["cell"]: 0.0 for c in cells}
    assert md.p6_verdict(share, cells)["verdict"] == "FALSIFIED"
    share[20] = 0.45
    assert md.p6_verdict(share, cells)["verdict"] == "FALSIFIED"
    share[20] = 0.50
    v = md.p6_verdict(share, cells)
    assert v["verdict"] == "HOLDS" and cells[20]["theta_limit"] == 0.70
    assert v["levels"][0.70] == {"best_cell_share": 0.5, "pooled_share": 0.0625}


# -- one real baseline --------------------------------------------------------


@pytest.mark.skipif(not FIRMA_BIN.exists(),
                    reason=f"{FIRMA_BIN} not built -- run `cargo build` first")
def test_one_part_3_baseline_end_to_end(tmp_path):
    r = md._baseline_worker((md.p3_cells()[39], 910_001, str(tmp_path), str(FIRMA_BIN)))
    assert r["cell"] == 39 and len(r["sc3_binds"]) == 4 and isinstance(r["T1"], bool)
    assert 0.0 <= r["sc1_survival"] <= 1.0 and not any(tmp_path.iterdir())
