"""Tests for scripts/diag_blindspot.py (review R22; DIAGNOSTIC-ONLY)."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path

import pandas as pd
import pytest

REPO = Path(__file__).resolve().parents[2]
FIRMA_BIN = REPO / "target" / "debug" / "firma"
SPEC_FILE = REPO / "docs" / "review" / "evidence" / "R22" / "blind_spec.json"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


bs = _load("diag_blindspot")
dc, hc = bs.dc, bs.hc
BASE = json.loads(bs.rs.BASE_CONFIG.read_text())


# -- spec ---------------------------------------------------------------------


@pytest.mark.skipif(not SPEC_FILE.exists(), reason="step-0 blind_spec.json not present")
def test_the_step_0_spec_file_is_exactly_the_spec_the_script_declares():
    assert SPEC_FILE.read_bytes() == bs.spec_bytes()
    assert hashlib.sha256(SPEC_FILE.read_bytes()).hexdigest() == bs.spec_sha256()


def test_step_0_never_overwrites_and_a_changed_or_missing_spec_is_refused(tmp_path, capsys):
    p = tmp_path / "spec.json"
    assert bs.main(["--write-spec", str(p)]) == 0
    assert bs.main(["--write-spec", str(p)]) == 1
    p.write_bytes(bs.spec_bytes().replace(b"0.80", b"0.70", 1))
    assert bs.main(["--spec", str(p), "--out", str(tmp_path / "o")]) == 1
    assert bs.main(["--out", str(tmp_path / "o")]) == 1
    assert "REFUSED" in capsys.readouterr().out and not (tmp_path / "o").exists()


def test_the_grid_is_the_declared_one_and_the_predictions_are_in_the_spec_verbatim():
    assert bs.MAGNITUDES == [0.1, 0.125, 0.15, 0.175, 0.2, 0.225, 0.25, 0.275, 0.3, 0.325, 0.35]
    assert bs.L_WS == [4, 8] and bs.H_CRITS == [0.15, 0.30] and bs.ONSET == 40
    assert bs.POP_SEEDS == list(range(910_001, 910_011))
    assert sorted(bs.INTERIOR + bs.EDGES + bs.OUTSIDE) == bs.MAGNITUDES
    p = bs.SPEC["predictions_verbatim_from_the_R22_prompt"]
    assert p["P1"].startswith("P1 (l_w = 4, h_crit = 0.15): compliance deaths appear only for m in "
                              "[0.150, 0.250] (allow the grid resolution)")
    assert ">= 80% inside that interval, <= 10% outside" in p["P1"]
    assert "share <= 10% for every m (0.30 exceeds the 0.25 step)" in p["P2"]
    assert "share <= 10% for every m (step 0.125 < 0.15)" in p["P3"]


# -- configs ------------------------------------------------------------------


@pytest.mark.parametrize("l_w,h_crit", [(4, 0.15), (4, 0.30), (8, 0.15), (8, 0.30)])
def test_l_w_is_set_in_all_three_rules_and_h_crit_in_the_decision_rule(l_w, h_crit):
    cfg = bs.build_part1_config(BASE, 910_003, l_w=l_w, h_crit=h_crit, magnitude=0.2)
    assert bs.window_settings(cfg) == {
        "decision.satisficing.l_w": l_w, "constraint.action_window.l_w": l_w,
        "constraint.enforce.l_w": l_w, "decision.satisficing.h_crit": h_crit}
    # no other rule carries an l_w
    assert [r["id"] for r in cfg["rules"] if "l_w" in r["params"]] == list(bs.WINDOW_RULES)


def test_part_1_shock_has_fixed_onset_and_magnitude_and_the_fork_differs_only_by_the_shock():
    s = bs.build_part1_config(BASE, 910_003, l_w=4, h_crit=0.15, magnitude=0.225)
    f = bs.build_part1_config(BASE, 910_003, l_w=4, h_crit=0.15, magnitude=None)
    p = next(r for r in s["rules"] if r["id"] == "shock.stochastic")["params"]
    assert (p["onset_min"], p["onset_max"], p["magnitude_mean"], p["magnitude_sd"]) == (40, 40, 0.225, 0.0)
    assert p["channel"] == {"kind": "regulatory", "target": "theta_limit"}
    assert p["ramp"] == {"kind": "instant"} and p["persistence"] == {"kind": "permanent"}
    assert [r for r in s["rules"] if r["id"] != "shock.stochastic"] == f["rules"]
    for k in ("seeds", "world", "agents", "environment"):
        assert s[k] == f[k]
    assert s["agents"] == dc.population(910_003, "medium", "relative", BASE["agents"][0])
    assert "observation.full" in [r["id"] for r in s["rules"]] and s["world"]["ticks"] == 120


def test_with_l_w_4_and_h_crit_015_only_the_explicit_h_crit_key_differs_from_r21():
    mine = bs.set_window_and_h_crit(
        dc.build_config(BASE, 910_001, "medium", reading="absolute", shocked=True), 4, 0.15)
    r20 = hc.build_config(BASE, 910_001, "medium", shocked=True)
    next(r for r in r20["rules"] if r["id"] == "decision.satisficing")["params"]["h_crit"] = 0.15
    assert mine == r20


def test_part_2_config_is_the_r21_relative_config_with_l_w_8():
    cfg = bs.build_part2_config(BASE, 911_004, "high", shocked=True)
    ref = dc.build_config(BASE, 911_004, "high", reading="relative", shocked=True)
    assert bs.window_settings(cfg)["constraint.enforce.l_w"] == 8
    assert cfg["agents"] == ref["agents"] and cfg["seeds"] == ref["seeds"]
    p = next(r for r in cfg["rules"] if r["id"] == "shock.stochastic")["params"]
    assert p["magnitude_mean"] == 0.20 and p["magnitude_sd"] == pytest.approx(0.2 / 3)
    assert (p["onset_min"], p["onset_max"]) == (10, 80)


def test_part_3_replaces_patchy_by_constant_in_both_branches():
    for mag in (None, 4):
        cfg = bs.build_part3_config(BASE, 910_002, magnitude=mag)
        ids = [r["id"] for r in cfg["rules"]]
        assert "resource.patchy" not in ids and ids[0] == "resource.constant"
        assert cfg["rules"][0] == {"id": "resource.constant", "version": "^1", "params": {}}
        assert bs.window_settings(cfg)["decision.satisficing.l_w"] == 8
    p = next(r for r in cfg["rules"] if r["id"] == "shock.stochastic")["params"]
    assert p["channel"] == {"kind": "resource"} and p["magnitude_mean"] == 4.0
    assert p["magnitude_sd"] == 0.0 and (p["onset_min"], p["onset_max"]) == (10, 80)
    assert "resource.patchy" in [r["id"] for r in
                                 bs.build_part3_config(BASE, 910_002, magnitude=4, constant=False)["rules"]]
    with pytest.raises(ValueError):
        bs.use_constant_resource(cfg)


# -- arithmetic ---------------------------------------------------------------


def test_strike_summary_takes_the_first_strike_at_or_after_onset():
    strikes = [(12, 0), (44, 1), (44, 2), (44, 3), (47, 4)]          # tick 12 is before the shock
    deaths = [(40, 9, "solvency"), (44, 8, "solvency"), (48, 1, "compliance"),
              (48, 2, "compliance"), (51, 4, "compliance")]
    s = bs.strike_summary(strikes, deaths, onset=40)
    assert s == {"first_strike_tick": 44, "firms_struck_at_first_strike": 3,
                 "deaths_at_first_strike_tick": 1, "first_compliance_death_tick": 48,
                 "compliance_deaths_at_first_death_tick": 2}
    none = bs.strike_summary([(12, 0)], [(40, 9, "solvency")], onset=40)
    assert none["first_strike_tick"] is None and none["first_compliance_death_tick"] is None


def _run_row(survivors, compliance, nonzero, first_strike=None, struck=0, at=0, fd=None, cd=0):
    return {"survivors": survivors, "compliance_deaths": compliance,
            "nonzero_matched_delta_h": nonzero, "first_strike_tick": first_strike,
            "firms_struck_at_first_strike": struck, "deaths_at_first_strike_tick": at,
            "first_compliance_death_tick": fd, "compliance_deaths_at_first_death_tick": cd}


def test_cell_summary_shares_and_means():
    rows = [_run_row(1, 9, True, 44, 9, 0, 48, 9), _run_row(2, 8, True, 44, 7, 2, 48, 5),
            _run_row(10, 0, False), _run_row(10, 0, True, 60, 1, 0)]
    c = bs.cell_summary(rows)
    assert c["compliance_death_share"] == 0.5 and c["survivors_distribution"] == {1: 1, 2: 1, 10: 2}
    assert c["share_with_a_strike_after_onset"] == 0.75
    assert c["mean_firms_struck_at_first_strike"] == pytest.approx(17 / 3)
    assert c["mean_deaths_at_first_strike_tick"] == pytest.approx(2 / 3)
    assert c["mean_compliance_deaths_at_first_death_tick"] == 7.0
    assert c["nonzero_matched_delta_h_share"] == 0.75
    assert bs.cell_summary([_run_row(12, 0, False)])["mean_firms_struck_at_first_strike"] is None


def _shares(p1_in=1.0, p1_edge=0.0, p1_out=0.0, p2=0.0, p3=0.0):
    s = {}
    for m in bs.MAGNITUDES:
        s[(4, 0.15, m)] = p1_in if m in bs.INTERIOR else (p1_edge if m in bs.EDGES else p1_out)
        s[(4, 0.30, m)] = p2
        s[(8, 0.15, m)] = p3
        s[(8, 0.30, m)] = 0.0
    return s


def test_all_three_predictions_hold_on_the_hypothesised_pattern_whatever_the_edges_do():
    for edge in (0.0, 0.5, 1.0):
        v = bs.evaluate_predictions(_shares(p1_edge=edge))
        assert v["P1"]["holds"] and v["P2"]["holds"] and v["P3"]["holds"]
        assert v["P1"]["strict_variant_holds"] is (edge == 1.0)


def test_p1_is_falsified_by_a_weak_interior_level_or_any_outside_level():
    assert not bs.evaluate_predictions(_shares(p1_in=0.7))["P1"]["holds"]
    assert bs.evaluate_predictions(_shares(p1_in=0.8, p1_out=0.1))["P1"]["holds"]
    assert not bs.evaluate_predictions(_shares(p1_out=0.2))["P1"]["holds"]
    one_bad = _shares()
    one_bad[(4, 0.15, 0.35)] = 0.2
    assert not bs.evaluate_predictions(one_bad)["P1"]["holds"]


def test_p2_and_p3_are_falsified_by_a_single_magnitude_above_ten_percent():
    for key, cell in (("P2", (4, 0.30)), ("P3", (8, 0.15))):
        s = _shares()
        assert bs.evaluate_predictions(s)[key]["holds"]
        s[(*cell, 0.2)] = 0.2
        v = bs.evaluate_predictions(s)
        assert not v[key]["holds"] and v["P1"]["holds"]
        s[(*cell, 0.2)] = 0.1
        assert bs.evaluate_predictions(s)[key]["holds"]


def test_price_series_accumulates_logged_deltas_per_tick():
    rows = [{"event": "delta_applied", "tick": t, "kind": {"AdjustGlobalInt": {"field": f, "delta": d}}}
            for t, f, d in ((1, "input_price", 4), (1, "output_price", -1), (3, "input_price", -1),
                            (3, "input_price", -1))]
    e = pd.json_normalize(rows)
    assert bs.price_series(e, 2, "input_price", 5) == [2, 6, 6, 4, 4]
    assert bs.price_series(e, 3, "output_price", 3) == [3, 2, 2]
    assert bs.price_series(pd.DataFrame({"event": ["x"], "tick": [0]}), 2, "input_price", 2) == [2, 2]


# -- one real cell ------------------------------------------------------------


@pytest.mark.skipif(not FIRMA_BIN.exists(),
                    reason=f"{FIRMA_BIN} not built -- run `cargo build` first")
def test_one_part_1_population_end_to_end(tmp_path, monkeypatch):
    monkeypatch.setattr(bs, "MAGNITUDES", [0.2])
    rows = bs._part1_worker((8, 0.30, 910_001, str(tmp_path), str(FIRMA_BIN)))
    assert len(rows) == 1 and rows[0]["magnitude"] == 0.2 and 0 <= rows[0]["survivors"] <= 12
    assert rows[0]["first_strike_tick"] is None or rows[0]["first_strike_tick"] >= 40
    assert not any(tmp_path.iterdir())
