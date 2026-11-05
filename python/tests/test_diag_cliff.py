"""Tests for scripts/diag_cliff.py (review R21; DIAGNOSTIC-ONLY)."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path

import pandas as pd
import pytest

REPO = Path(__file__).resolve().parents[2]
FIRMA_BIN = REPO / "target" / "debug" / "firma"
SPEC_FILE = REPO / "docs" / "review" / "evidence" / "R21" / "diag_spec.json"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


dc = _load("diag_cliff")
pg, hc = dc.pg, dc.hc
BASE = json.loads(dc.rs.BASE_CONFIG.read_text())
TEMPLATE = BASE["agents"][0]


# -- spec ---------------------------------------------------------------------


@pytest.mark.skipif(not SPEC_FILE.exists(), reason="step-0 diag_spec.json not present")
def test_the_step_0_spec_file_is_exactly_the_spec_the_script_declares():
    assert SPEC_FILE.read_bytes() == dc.spec_bytes()
    assert hashlib.sha256(SPEC_FILE.read_bytes()).hexdigest() == dc.spec_sha256()


def test_step_0_never_overwrites_and_a_changed_or_missing_spec_is_refused(tmp_path, capsys):
    p = tmp_path / "spec.json"
    assert dc.main(["--write-spec", str(p)]) == 0
    assert dc.main(["--write-spec", str(p)]) == 1
    p.write_bytes(dc.spec_bytes().replace(b"910101", b"910102", 1))
    assert dc.main(["--spec", str(p), "--out", str(tmp_path / "o")]) == 1
    assert dc.main(["--out", str(tmp_path / "o")]) == 1
    assert "REFUSED" in capsys.readouterr().out and not (tmp_path / "o").exists()


def test_declared_seed_sets_and_cells():
    assert dc.A_POP_SEEDS == list(range(910_001, 910_031))
    assert dc.A_SHOCK_SEEDS == list(range(910_101, 910_131))
    assert dc.D_SEEDS == list(range(910_001, 910_051)) and dc.C_LAST_TICK == 39
    assert dc.D_CELLS == [("regulatory", 0.20), ("resource", 1), ("resource", 2),
                          ("resource", 4), ("competitive", 1), ("competitive", 2)]


# -- relative aspiration ------------------------------------------------------


def test_relative_aspiration_is_eight_times_one_plus_a_uniform_in_plus_minus_a():
    for level, a in (("low", 0.10), ("medium", 0.25), ("high", 0.50)):
        vals = [dc.relative_aspiration(s, f, level) for s in range(910_001, 910_031) for f in range(12)]
        assert 8.0 * (1 - a) <= min(vals) and max(vals) < 8.0 * (1 + a)
        assert max(vals) - min(vals) > 8.0 * a  # really uses the wider range
    u = pg.uniform(910_001, 3, pg.TAG_ASPIRATION)
    assert dc.relative_aspiration(910_001, 3, "high") == 8.0 * (1.0 + (-0.5 + 2 * 0.5 * u))


def test_the_relative_reading_changes_only_the_aspiration_and_keeps_the_same_uniform():
    absolute = dc.population(910_007, "medium", "absolute", TEMPLATE)
    relative = dc.population(910_007, "medium", "relative", TEMPLATE)
    assert absolute == pg.population(910_007, 12, "medium", TEMPLATE)
    for a, r in zip(absolute, relative):
        assert a["stocks"] == r["stocks"] and a.get("ints") == r.get("ints")
        assert a["reals"]["capability"] == r["reals"]["capability"]
        # same uniform: (abs - 8) / 0.25 == (rel / 8 - 1) / 0.25
        assert (a["reals"]["aspiration_capital_growth"] - 8.0) == pytest.approx(
            (r["reals"]["aspiration_capital_growth"] / 8.0 - 1.0), abs=1e-12)
    with pytest.raises(ValueError):
        dc.population(910_007, "medium", "percent", TEMPLATE)


# -- configs ------------------------------------------------------------------


def test_default_config_is_the_r20_config():
    for shocked in (True, False):
        mine = dc.build_config(BASE, 910_001, "medium", reading="absolute", shocked=shocked)
        assert mine == hc.build_config(BASE, 910_001, "medium", shocked=shocked)


def test_crossing_changes_only_the_shock_seed():
    r20 = hc.build_config(BASE, 910_005, "medium", shocked=True)
    crossed = dc.build_config(BASE, 910_005, "medium", reading="absolute", shocked=True,
                              shock_seed=910_117)
    assert crossed["seeds"] == dict(r20["seeds"], shock=910_117)
    assert {k: v for k, v in crossed.items() if k != "seeds"} == {
        k: v for k, v in r20.items() if k != "seeds"}
    with pytest.raises(dc.sl.SliceSeedError):
        dc.build_config(BASE, 910_005, "medium", reading="absolute", shocked=True, shock_seed=150)


def test_channel_cells_set_the_declared_shock_and_nothing_else():
    reg = dc.build_config(BASE, 910_002, "medium", reading="relative", shocked=True)
    for channel, mag in (("resource", 4), ("competitive", 2)):
        cfg = dc.build_config(BASE, 910_002, "medium", reading="relative", shocked=True,
                              channel=channel, magnitude=mag)
        p = next(r for r in cfg["rules"] if r["id"] == "shock.stochastic")["params"]
        assert p["channel"] == {"kind": channel}
        assert p["magnitude_mean"] == float(mag) and p["magnitude_sd"] == 0.0
        assert (p["onset_min"], p["onset_max"]) == (10, 80)
        assert p["ramp"] == {"kind": "instant"} and p["persistence"] == {"kind": "permanent"}
        assert p["targets"] == {"kind": "all"}
        strip = lambda c: [r for r in c["rules"] if r["id"] != "shock.stochastic"]  # noqa: E731
        assert strip(cfg) == strip(reg)
        for k in ("seeds", "world", "agents", "environment"):
            assert cfg[k] == reg[k]
    p = next(r for r in reg["rules"] if r["id"] == "shock.stochastic")["params"]
    assert p["channel"] == {"kind": "regulatory", "target": "theta_limit"}
    assert p["magnitude_mean"] == 0.20 and p["magnitude_sd"] == pytest.approx(0.20 / 3)
    with pytest.raises(ValueError):
        dc.shock_params("reputational", 0.1)


# -- part A arithmetic --------------------------------------------------------


def test_variance_is_all_shock_when_columns_decide():
    m = [[1, 0, 1, 0]] * 3
    v = dc.variance_shares(m)
    assert v["share_shock"] == pytest.approx(1.0) and v["share_population"] == pytest.approx(0.0)
    assert v["share_interaction"] == pytest.approx(0.0)
    assert (v["range_shock"], v["range_population"]) == (1.0, 0.0)
    assert dc.attribution_reading(v["range_population"], v["range_shock"]) == "shock-determined"


def test_variance_is_all_population_when_rows_decide():
    m = [[1, 1, 1], [0, 0, 0], [1, 1, 1], [0, 0, 0]]
    v = dc.variance_shares(m)
    assert v["share_population"] == pytest.approx(1.0) and v["share_shock"] == pytest.approx(0.0)
    assert dc.attribution_reading(v["range_population"], v["range_shock"]) == "population-determined"


def test_variance_is_all_interaction_for_a_checkerboard():
    m = [[1, 0], [0, 1]]
    v = dc.variance_shares(m)
    assert v["share_interaction"] == pytest.approx(1.0)
    assert v["range_shock"] == 0.0 and v["range_population"] == 0.0
    assert dc.attribution_reading(0.0, 0.0) == "mixed"


def test_variance_shares_hand_computed_example():
    # rows 2/3, 1/3 ; columns 1, 1/2, 0 ; grand 1/2
    m = [[1, 1, 0], [1, 0, 0]]
    v = dc.variance_shares(m)
    assert v["grand_rate"] == pytest.approx(0.5) and v["ss_total"] == pytest.approx(1.5)
    assert v["ss_population"] == pytest.approx(3 * 2 * (1 / 6) ** 2)
    assert v["ss_shock"] == pytest.approx(2 * (0.25 + 0 + 0.25))
    assert v["share_population"] + v["share_shock"] + v["share_interaction"] == pytest.approx(1.0)
    assert dc.variance_shares([[0, 0], [0, 0]])["share_shock"] is None


def test_the_attribution_reading_uses_a_factor_of_three():
    # binary-exact values, so "exactly three times" is exact
    assert dc.attribution_reading(0.125, 0.375) == "shock-determined"
    assert dc.attribution_reading(0.125, 0.25) == "mixed"
    assert dc.attribution_reading(0.375, 0.125) == "population-determined"
    assert dc.attribution_reading(0.25, 0.25) == "mixed"
    assert dc.attribution_reading(0.0, 0.5) == "shock-determined"


# -- part C and D arithmetic --------------------------------------------------


def _decision(tick, agent, field, value, origin="decision.satisficing"):
    return {"event": "delta_applied", "tick": tick, "origin": origin, "target": {"Agent": agent},
            "kind": {"SetAgentInt": {"field": field, "value": value}}}


def test_firm_sequences_count_identical_firms_once_and_respect_the_window():
    rows = []
    for t in range(4):
        for agent in (0, 1):                      # identical actions; focus differs at tick 1
            rows.append(_decision(t, agent, "selected_action", t % 2))
            rows.append(_decision(t, agent, "focus", 1 if (agent == 1 and t == 1) else 0))
        rows.append(_decision(t, 2, "selected_action", 5))
        rows.append(_decision(t, 2, "focus", 0))
    rows.append(_decision(1, 0, "selected_action", 9, origin="constraint.enforce"))  # not a decision
    a_seq, fa_seq = dc.firm_sequences(pd.json_normalize(rows), last_tick=2)
    assert a_seq[0] == ((0, 0), (1, 1), (2, 0)) == a_seq[1] and len(set(a_seq.values())) == 2
    assert len(set(fa_seq.values())) == 3
    assert fa_seq[1][1] == (1, 1, 1) and all(len(v) == 3 for v in a_seq.values())


def _row(shocked, baseline):
    return {"shocked_survivors": shocked, "baseline_survivors": baseline}


def test_graded_needs_few_runs_at_the_extremes_and_four_distinct_counts():
    spread = [_row(s, 11) for s in (3, 4, 5, 6, 7, 8, 9, 10, 4, 6)]
    g = dc.graded(spread)
    assert g["graded"] and g["extreme_share"] == 0.0 and g["distinct_survivor_counts"] == 8
    cliff = [_row(1, 10)] * 5 + [_row(10, 10)] * 5
    g = dc.graded(cliff)
    assert not g["graded"] and g["extreme_share"] == 1.0 and (g["collapsed"], g["untouched"]) == (5, 5)
    # exactly 20% at the extremes is not "fewer than 20%"
    assert not dc.graded([_row(2, 11), _row(11, 11)] + [_row(s, 11) for s in (3, 4, 5, 6, 7, 8, 9, 10)])["graded"]
    # few extremes but only three distinct counts
    assert not dc.graded([_row(s, 11) for s in (5, 6, 7, 5, 6, 7)])["graded"]


# -- one real crossed pair ----------------------------------------------------


@pytest.mark.skipif(not FIRMA_BIN.exists(),
                    reason=f"{FIRMA_BIN} not built -- run `cargo build` first")
def test_one_resource_channel_pair_end_to_end(tmp_path):
    r = dc._pair_worker(("t", {"init_seed": 910_001, "level": "medium", "reading": "relative",
                               "channel": "resource", "magnitude": 2}, str(tmp_path), str(FIRMA_BIN)))
    assert r["failed"] is None and r["drawn_magnitude"] == 2.0 and 10 <= r["drawn_onset"] <= 80
    assert 0 <= r["shocked_survivors"] <= 12 and isinstance(r["T1"], bool)
