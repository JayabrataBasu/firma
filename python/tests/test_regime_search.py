"""Tests for scripts/regime_search.py (review R16; DIAGNOSTIC-ONLY)."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
FIRMA_BIN = REPO / "target" / "debug" / "firma"
GRID_FILE = REPO / "docs" / "review" / "evidence" / "R16" / "grid.json"


def _load():
    spec = importlib.util.spec_from_file_location("rs", REPO / "scripts" / "regime_search.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


rs = _load()
BASE = json.loads(rs.BASE_CONFIG.read_text())


# -- the declared grid --------------------------------------------------------


def test_the_grid_has_72_cells_in_product_order_with_the_last_axis_fastest():
    c = rs.cells()
    assert len(c) == 72 and [x["cell"] for x in c] == list(range(72))
    assert c[0] == {"cell": 0, "n": 8, "capital": [50, 100], "capability": [0.30, 0.60],
                    "shock_magnitude_mean": 0.10, "theta_limit": 0.55}
    assert c[1]["theta_limit"] == 0.90 and c[2]["shock_magnitude_mean"] == 0.20
    assert c[36]["n"] == 12 and c[71]["capital"] == [150, 300]


@pytest.mark.skipif(not GRID_FILE.exists(), reason="step-0 grid.json not present")
def test_the_step_0_grid_file_is_exactly_the_grid_the_script_declares():
    assert GRID_FILE.read_bytes() == rs.grid_bytes()
    assert hashlib.sha256(GRID_FILE.read_bytes()).hexdigest() == rs.grid_sha256()


def test_a_run_is_refused_when_the_grid_file_differs(tmp_path, capsys):
    g = tmp_path / "grid.json"
    g.write_bytes(rs.grid_bytes().replace(b"0.25", b"0.35", 1))
    assert rs.main(["--grid", str(g), "--out", str(tmp_path / "o")]) == 1
    assert "REFUSED" in capsys.readouterr().out and not (tmp_path / "o").exists()


def test_a_run_is_refused_without_a_grid_file_and_step_0_never_overwrites(tmp_path, capsys):
    assert rs.main(["--out", str(tmp_path / "o")]) == 1
    g = tmp_path / "grid.json"
    assert rs.main(["--write-grid", str(g)]) == 0
    assert rs.main(["--write-grid", str(g)]) == 1
    assert "REFUSED" in capsys.readouterr().out


# -- population and configs ---------------------------------------------------


def test_population_is_linear_in_capital_and_capability():
    firms = rs.population(8, [50, 100], [0.30, 0.60], BASE["agents"][0])
    assert [f["id"] for f in firms] == list(range(8))
    assert [f["stocks"]["capital"] for f in firms] == [50, 57, 64, 71, 79, 86, 93, 100]
    assert firms[0]["reals"]["capability"] == 0.30 and firms[-1]["reals"]["capability"] == 0.60
    assert firms[3]["reals"]["capability"] == pytest.approx(0.30 + 3 * 0.30 / 7, abs=1e-6)
    for f in firms:
        assert f["stocks"]["input"] == 40 and f["reals"]["legitimacy"] == 1.0
        assert f["reals"]["aspiration_capital_growth"] == 8.0
    assert BASE["agents"][0]["stocks"]["capital"] == 600  # template not mutated


def test_cell_config_fixed_items():
    cell = rs.cells()[5]
    cfg = rs.build_cell_config(BASE, cell, shock_seed=910_007, shocked=True)
    assert cfg["seeds"] == {"mechanism": 910_001, "environment": 910_002,
                            "shock": 910_007, "init": 910_004}
    assert cfg["world"]["ticks"] == 120
    assert cfg["world"]["global_reals"]["theta_limit"] == cell["theta_limit"]
    ids = [r["id"] for r in cfg["rules"]]
    assert "observation.full" in ids and "observation.delayed" not in ids
    assert "shock.scheduled" not in ids and ids.count("shock.stochastic") == 1
    p = next(r for r in cfg["rules"] if r["id"] == "shock.stochastic")["params"]
    assert p["magnitude_mean"] == cell["shock_magnitude_mean"]
    assert p["magnitude_sd"] == pytest.approx(cell["shock_magnitude_mean"] / 3)
    assert (p["onset_min"], p["onset_max"]) == (10, 80)
    assert p["channel"] == {"kind": "regulatory", "target": "theta_limit"}
    assert len(cfg["agents"]) == cell["n"]
    assert "DIAGNOSTIC-ONLY" in cfg["experiment"]


def test_the_no_shock_config_differs_only_by_the_shock_rule():
    cell = rs.cells()[0]
    s = rs.build_cell_config(BASE, cell, shock_seed=910_001, shocked=True)
    f = rs.build_cell_config(BASE, cell, shock_seed=910_001, shocked=False)
    assert [r for r in s["rules"] if r["id"] != "shock.stochastic"] == f["rules"]
    for k in ("seeds", "world", "agents", "environment"):
        assert s[k] == f[k]


def test_a_reserved_seed_is_refused():
    with pytest.raises(rs.sl.SliceSeedError):
        rs.build_cell_config(BASE, rs.cells()[0], shock_seed=200, shocked=True)


# -- targets ------------------------------------------------------------------


def _baseline(**over):
    b = {"sc1_survival": 0.75, "sc2_reconstructed_h_below_crit": 0.10, "sc3_pass": True,
         "sc3_binds": [True] * 4, "sc6_pass": True, "sc6_entropy_variance": 0.2,
         "sc2_survival_attention": 0.1, "sc2_pass": True, "sc1_pass": True,
         "sc4_shaping_fraction": 0.0, "sc5_shaping_success": 0.0}
    b.update(over)
    return b


def _shocked(deltas, survivors):
    return [{"delta_h_rep_matched": d, "survivors": s} for d, s in zip(deltas, survivors)]


def test_a_cell_meeting_every_target_is_stressed_but_alive():
    r = rs.evaluate_targets(_baseline(), _shocked([-0.1, 0.0, 0.2, 0.0], [6, 6, 5, 7]), 8)
    assert r["T1"] and r["T2"] and r["stressed_but_alive"]
    assert r["nonzero_matched_fraction"] == 0.5 and r["mean_survival_shocked"] == 0.75


@pytest.mark.parametrize("over", [
    {"sc1_survival": 0.59}, {"sc1_survival": 0.91},
    {"sc2_reconstructed_h_below_crit": 0.049}, {"sc2_reconstructed_h_below_crit": 0.26},
    {"sc3_pass": False, "sc3_binds": [True, True, True, False]},
    {"sc6_pass": False},
])
def test_each_t1_leg_can_fail_t1_on_its_own(over):
    r = rs.evaluate_targets(_baseline(**over), _shocked([-0.1, 0.1], [6, 6]), 8)
    assert not r["T1"] and r["T2"] and not r["stressed_but_alive"]


def test_t1_bounds_are_inclusive():
    for sc1, sc2 in ((0.60, 0.05), (0.90, 0.25)):
        r = rs.evaluate_targets(
            _baseline(sc1_survival=sc1, sc2_reconstructed_h_below_crit=sc2),
            _shocked([0.1], [6]), 8)
        assert r["T1"]


def test_t2_fails_when_under_half_the_runs_have_non_zero_matched_delta():
    r = rs.evaluate_targets(_baseline(), _shocked([0.0, 0.0, 0.0, 0.1], [6] * 4), 8)
    assert r["T1"] and not r["nonzero_ok"] and not r["T2"] and not r["stressed_but_alive"]


@pytest.mark.parametrize("survivors", [[3, 3], [8, 8]])
def test_t2_fails_when_mean_survival_is_outside_the_band(survivors):
    r = rs.evaluate_targets(_baseline(), _shocked([0.1, 0.1], survivors), 8)
    assert r["nonzero_ok"] and not r["survival_ok"] and not r["T2"]


def test_t2_fails_with_no_shocked_runs_and_the_information_column_is_not_a_target():
    r = rs.evaluate_targets(
        _baseline(sc3_pass=False, sc3_binds=[True, True, True, False]), [], 8)
    assert not r["T2"] and not r["T1"] and r["info_T1_without_obligation_leg"]


# -- one real cell ------------------------------------------------------------


@pytest.mark.skipif(not FIRMA_BIN.exists(),
                    reason=f"{FIRMA_BIN} not built -- run `cargo build` first")
def test_one_cell_end_to_end_with_two_shock_seeds(tmp_path):
    cell = rs.cells()[0]
    res = rs.evaluate_cell(
        "cell00", cell["n"],
        lambda seed, shocked: rs.build_cell_config(BASE, cell, shock_seed=seed, shocked=shocked),
        tmp_path, FIRMA_BIN, [910_001, 910_002])
    assert res["failed"] == [] and res["n_shocked_runs"] == 2
    # the fork does not depend on the shock seed: both forks equal the baseline byte for byte
    assert res["forks_identical_to_baseline"] == 2
    assert 0.0 <= res["sc1_survival"] <= 1.0 and len(res["sc3_binds"]) == 4
    assert isinstance(res["T1"], bool) and isinstance(res["T2"], bool)
    assert res["baseline_survivors"] == res["baseline_survivors_from_log"]
    for r in res["shocked_runs"]:
        assert r["survivors"] + r["deaths"] == cell["n"]
    assert not (tmp_path / "cell00").exists()  # run directories are cleaned up


# -- grid 2 (review R18) ------------------------------------------------------

GRID2_FILE = REPO / "docs" / "review" / "evidence" / "R18" / "grid2.json"


def test_grid_2_has_288_cells_in_product_order_with_the_last_axis_fastest():
    c = rs.cells2()
    assert len(c) == 288 and [x["cell"] for x in c] == list(range(288))
    assert c[0] == {"cell": 0, "n_type_L": 0, "n_type_T": 0, "output_price": 2,
                    "obligation_seeding": "none", "theta_limit": 0.60}
    assert [x["theta_limit"] for x in c[:4]] == [0.60, 0.70, 0.80, 0.90]
    assert c[4]["obligation_seeding"] == "seeded" and c[8]["output_price"] == 3
    assert c[24]["n_type_T"] == 2 and c[72]["n_type_L"] == 1
    assert c[287] == {"cell": 287, "n_type_L": 3, "n_type_T": 4, "output_price": 5,
                      "obligation_seeding": "seeded", "theta_limit": 0.90}


@pytest.mark.skipif(not GRID2_FILE.exists(), reason="step-0 grid2.json not present")
def test_the_step_0_grid_2_file_is_exactly_the_grid_the_script_declares():
    assert GRID2_FILE.read_bytes() == rs.grid2_bytes()


def test_grid_2_targets_are_the_grid_1_targets():
    for k in ("T1_baseline", "T2_shocked"):
        assert rs.GRID2["targets"][k]["bounds"] == rs.GRID["targets"][k]["bounds"]


def test_a_grid_2_run_is_refused_when_the_file_differs_and_step_0_never_overwrites(tmp_path, capsys):
    g = tmp_path / "grid2.json"
    assert rs.main(["--write-grid2", str(g)]) == 0
    assert rs.main(["--write-grid2", str(g)]) == 1
    g.write_bytes(rs.grid2_bytes().replace(b"0.25", b"0.35", 1))
    assert rs.main(["--grid2", str(g), "--out", str(tmp_path / "o")]) == 1
    assert "REFUSED" in capsys.readouterr().out and not (tmp_path / "o").exists()


def test_within_type_spacing_is_linear_and_a_single_firm_gets_the_midpoint():
    assert rs._spaced(0.25, 0.35, 1) == [pytest.approx(0.30)]
    assert rs._spaced(8, 20, 4) == [8, 12, 16, 20]
    assert rs._spaced(0.25, 0.35, 3) == [pytest.approx(v) for v in (0.25, 0.30, 0.35)]


def test_population_2_assigns_types_by_id_and_seeds_the_first_two_ordinary_firms():
    firms = rs.population2(3, 4, True, BASE["agents"][0])
    assert [f["id"] for f in firms] == list(range(12))
    assert [f["_type"] for f in firms] == ["L"] * 3 + ["T"] * 4 + ["O"] * 5
    assert [f["stocks"]["capital"] for f in firms] == [200, 250, 300, 8, 12, 16, 20,
                                                       150, 188, 225, 263, 300]
    assert [f["reals"]["capability"] for f in firms] == [
        0.25, 0.30, 0.35, 0.5, 0.566667, 0.633333, 0.7, 0.5, 0.575, 0.65, 0.725, 0.8]
    assert all(f["reals"]["capability"] < 0.4 for f in firms[:3])
    assert [f.get("ints") for f in firms] == [None] * 7 + [{"obligation": 60}] * 2 + [None] * 3
    for f in firms:
        assert f["stocks"]["input"] == 40 and f["reals"]["aspiration_capital_growth"] == 8.0


def test_population_2_with_no_special_types_is_twelve_ordinary_firms_and_no_obligation():
    firms = rs.population2(0, 0, False, BASE["agents"][0])
    assert len(firms) == 12 and {f["_type"] for f in firms} == {"O"}
    assert firms[0]["stocks"]["capital"] == 150 and firms[-1]["stocks"]["capital"] == 300
    assert all("ints" not in f for f in firms)
    one_l = rs.population2(1, 0, False, BASE["agents"][0])
    assert (one_l[0]["reals"]["capability"], one_l[0]["stocks"]["capital"]) == (0.3, 250)


def test_grid_2_cell_config_sets_price_theta_and_obligation_threshold():
    c = rs.cells2()
    seeded = rs.build_cell2_config(BASE, c[287], shock_seed=910_003, shocked=True, shock_mean=0.2)
    assert seeded["world"]["global_ints"] == {"theta_q": 40, "input_price": 2, "output_price": 5}
    assert seeded["world"]["global_reals"] == {"theta_limit": 0.90, "theta_cap": 0.4}
    assert seeded["world"]["ticks"] == 120 and seeded["seeds"]["shock"] == 910_003
    assert all("_type" not in a for a in seeded["agents"])
    p = next(r for r in seeded["rules"] if r["id"] == "shock.stochastic")["params"]
    assert p["magnitude_mean"] == 0.2 and p["magnitude_sd"] == pytest.approx(0.2 / 3)
    ids = [r["id"] for r in seeded["rules"]]
    assert "observation.full" in ids and "observation.delayed" not in ids
    none = rs.build_cell2_config(BASE, c[0], shock_seed=910_001, shocked=False)
    assert none["world"]["global_ints"] == {"theta_q": 100, "input_price": 2, "output_price": 2}
    assert "shock.stochastic" not in [r["id"] for r in none["rules"]]
    assert BASE["world"]["global_ints"]["output_price"] == 3  # base not mutated


def test_a_grid_2_fork_differs_from_its_shocked_run_only_by_the_shock_rule():
    cell = rs.cells2()[100]
    s = rs.build_cell2_config(BASE, cell, shock_seed=910_002, shocked=True, shock_mean=0.1)
    f = rs.build_cell2_config(BASE, cell, shock_seed=910_002, shocked=False)
    assert [r for r in s["rules"] if r["id"] != "shock.stochastic"] == f["rules"]
    for k in ("seeds", "world", "agents", "environment"):
        assert s[k] == f[k]
    with pytest.raises(ValueError):
        rs.build_cell2_config(BASE, cell, shock_seed=910_002, shocked=True)
    with pytest.raises(rs.sl.SliceSeedError):
        rs.build_cell2_config(BASE, cell, shock_seed=7, shocked=False)


R16_CELLS = REPO / "docs" / "review" / "evidence" / "R16" / "cells_full.json"


@pytest.mark.skipif(not (FIRMA_BIN.exists() and R16_CELLS.exists()),
                    reason="needs the firma binary and the R16 evidence")
def test_grid_1_cell_0_baseline_reproduces_its_r16_numbers(tmp_path):
    recorded = json.loads(R16_CELLS.read_text())[0]
    now = rs.evaluate_baseline(
        "c0", 8, rs.build_cell_config(BASE, rs.cells()[0], shock_seed=910_001, shocked=False),
        tmp_path, FIRMA_BIN)
    for k in rs.T1_KEYS:
        assert now[k] == recorded[k]
    assert not (tmp_path / "c0").exists()


@pytest.mark.skipif(not FIRMA_BIN.exists(),
                    reason=f"{FIRMA_BIN} not built -- run `cargo build` first")
def test_one_grid_2_baseline_end_to_end(tmp_path):
    cell = rs.cells2()[287]
    res = rs.evaluate_baseline(
        "g2", 12, rs.build_cell2_config(BASE, cell, shock_seed=910_001, shocked=False),
        tmp_path, FIRMA_BIN)
    assert res["failed"] == [] and len(res["sc3_binds"]) == 4
    assert 0.0 <= res["sc1_survival"] <= 1.0 and isinstance(res["T1"], bool)
    assert "T2" not in res and len(res["events_sha256"]) == 64
    res["cell_params"] = cell
    assert rs.stage1_line(res).startswith("287 L3 T4 price 5 seeded")


def test_stage_1_line_prints_an_undefined_sc5_as_not_available():
    # R18: the first grid-2 run crashed here, before printing any result,
    # because SC-5 is None when no shaping action was committed.
    res = {"cell_params": rs.cells2()[0], "sc1_survival": 1.0, "sc1_ok": False,
           "sc2_h_below_crit": 0.5, "sc2_ok": False, "sc3_binds": [False] * 4, "sc3_ok": False,
           "sc6_entropy_variance": 0.2, "sc6_ok": True, "T1": False,
           "info_sc4_shaping_fraction": 0.0, "info_sc5_shaping_success": None}
    assert rs.stage1_line(res).endswith("SC-4 0.000 SC-5 n/a")
