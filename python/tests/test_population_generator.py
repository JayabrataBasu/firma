"""Tests for scripts/population_generator.py and scripts/hetero_check.py
(review R20; DIAGNOSTIC-ONLY)."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import math
import random
import statistics
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
FIRMA_BIN = REPO / "target" / "debug" / "firma"
SPEC_FILE = REPO / "docs" / "review" / "evidence" / "R20" / "hetero_spec.json"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


pg = _load("population_generator")
hc = _load("hetero_check")
BASE = json.loads(hc.rs.BASE_CONFIG.read_text())
TEMPLATE = BASE["agents"][0]

# First 8 bytes of SHA-256 of the key, computed with GNU coreutils
# (`printf '<key>' | sha256sum`), not with Python:
# docs/review/evidence/R20/known_answer_vectors.txt
COREUTILS_VECTORS = {
    (910001, 0, "capital"): "c509fa6fc458083b",
    (910001, 0, "capability"): "fa918c34ab083bc7",
    (910001, 11, "capital"): "49da2142b3410851",
    (911050, 7, "aspiration_capital_growth"): "a80aa919648b7565",
    (910001, 3, "obligation_flag"): "bbd32b8e7167e5fb",
    (910001, 3, "obligation_q"): "93edf476472956e2",
}


def second_implementation(seed, firm, tag):
    """The same uniform, composed differently: incremental hashing of the
    three parts, hex digest, base-16 parse, exact rational division."""
    h = hashlib.new("sha256")
    for part in (str(seed), "|", str(firm), "|", tag):
        h.update(part.encode("ascii"))
    return int(h.hexdigest()[:16], 16) / 18446744073709551616


# -- Q1: the derivation -------------------------------------------------------


@pytest.mark.parametrize("key,hex8", sorted(COREUTILS_VECTORS.items()))
def test_uniform_matches_the_coreutils_known_answer_vectors(key, hex8):
    assert pg.uniform(*key) == int(hex8, 16) / 2**64


def test_uniform_matches_bc_arithmetic_to_15_digits():
    # `bc` at scale 20 on the coreutils hex (same evidence file)
    assert pg.uniform(910001, 0, "capital") == pytest.approx(0.76968350628553150780, abs=1e-15)
    assert pg.uniform(911050, 7, "aspiration_capital_growth") == pytest.approx(
        0.65641266698780305891, abs=1e-15)


def test_uniform_equals_an_independently_composed_second_implementation():
    for seed in (910001, 910050, 911001, 911050):
        for firm in range(12):
            for tag in pg.SPEC["derivation"]["purpose_tags"]:
                u = pg.uniform(seed, firm, tag)
                assert u == second_implementation(seed, firm, tag)
                assert 0.0 <= u < 1.0


def test_the_five_purposes_and_neighbouring_keys_give_different_draws():
    us = {pg.uniform(910001, 0, t) for t in pg.SPEC["derivation"]["purpose_tags"]}
    assert len(us) == 5
    assert pg.uniform(910001, 1, "capital") != pg.uniform(910001, 0, "capital")
    assert pg.uniform(910002, 0, "capital") != pg.uniform(910001, 0, "capital")
    # "1|12|x" and "11|2|x" are different keys: the separator prevents collisions
    assert pg.uniform(1, 12, "capital") != pg.uniform(11, 2, "capital")


def test_standard_normal_is_the_inverse_cdf_of_the_uniform():
    z = pg.standard_normal(910001, 0, "capital")
    assert statistics.NormalDist().cdf(z) == pytest.approx(pg.uniform(910001, 0, "capital"), abs=1e-12)


def test_draw_values_follow_the_declared_formulas():
    v = pg.draw_values(910001, 0, "medium")
    sigma = math.sqrt(math.log(1 + 0.5**2))
    z_cap = statistics.NormalDist().inv_cdf(second_implementation(910001, 0, "capital"))
    assert v["capital"] == int(math.floor(100 * math.exp(sigma * z_cap) + 0.5))
    z_k = statistics.NormalDist().inv_cdf(second_implementation(910001, 0, "capability"))
    assert v["capability"] == min(1.0, max(0.05, 0.55 + 0.10 * z_k))
    u_a = second_implementation(910001, 0, "aspiration_capital_growth")
    assert v["aspiration_capital_growth"] == 8.0 - 0.25 + 0.5 * u_a
    flag = second_implementation(910001, 0, "obligation_flag") < 1 / 6
    assert (v["obligation"] is not None) == flag


def test_values_stay_inside_their_declared_ranges_at_every_level():
    for level in pg.SPREAD_LEVELS:
        s = pg.SPEC["spread_levels"][level]
        for seed in list(range(910001, 910051)) + list(range(911001, 911051)):
            for firm in range(12):
                v = pg.draw_values(seed, firm, level)
                assert v["capital"] >= 1 and isinstance(v["capital"], int)
                assert 0.05 <= v["capability"] <= 1.0
                assert 8.0 - s["a_spread"] <= v["aspiration_capital_growth"] < 8.0 + s["a_spread"]
                assert v["obligation"] is None or 50 <= v["obligation"] <= 80


def test_population_moments_are_near_the_declared_ones():
    # 100 seeds x 12 firms = 1200 draws per level; loose bounds, a sanity check only.
    seeds = list(range(910001, 910051)) + list(range(911001, 911051))
    for level in pg.SPREAD_LEVELS:
        s = pg.SPEC["spread_levels"][level]
        vals = [pg.draw_values(seed, f, level) for seed in seeds for f in range(12)]
        caps = [v["capital"] for v in vals]
        assert statistics.median(caps) == pytest.approx(100, rel=0.08)
        assert statistics.pstdev(caps) / statistics.fmean(caps) == pytest.approx(s["cv_cap"], rel=0.20)
        assert statistics.fmean(v["aspiration_capital_growth"] for v in vals) == pytest.approx(8.0, abs=0.05)
        share = sum(v["obligation"] is not None for v in vals) / len(vals)
        assert share == pytest.approx(1 / 6, abs=0.04)
    low = [pg.draw_values(seed, f, "low")["capability"] for seed in seeds for f in range(12)]
    assert statistics.fmean(low) == pytest.approx(0.55, abs=0.01)
    assert statistics.pstdev(low) == pytest.approx(0.05, rel=0.10)


def test_generation_order_does_not_change_any_firms_values():
    straight = pg.population(910001, 12, "high", TEMPLATE)
    order = list(range(12))
    random.Random(0).shuffle(order)
    assert order != list(range(12))
    assert pg.population(910001, 12, "high", TEMPLATE, generation_order=order) == straight
    assert pg.population(910001, 12, "high", TEMPLATE, generation_order=order[::-1]) == straight
    with pytest.raises(ValueError):
        pg.population(910001, 12, "high", TEMPLATE, generation_order=[0, 0, 1])


def test_adding_a_firm_does_not_change_the_others():
    twelve = pg.population(910001, 12, "medium", TEMPLATE)
    assert pg.population(910001, 13, "medium", TEMPLATE)[:12] == twelve
    assert pg.population(910001, 1, "medium", TEMPLATE) == twelve[:1]
    assert pg.population(910001, 40, "medium", TEMPLATE)[:12] == twelve


def test_a_different_seed_or_level_gives_a_different_population():
    a = pg.population(910001, 12, "medium", TEMPLATE)
    assert pg.population(910002, 12, "medium", TEMPLATE) != a
    assert pg.population(910001, 12, "high", TEMPLATE) != a


def test_firms_keep_the_template_fields_and_only_seeded_firms_have_an_obligation():
    firms = pg.population(910003, 12, "medium", TEMPLATE)
    assert [f["id"] for f in firms] == list(range(12))
    for f in firms:
        assert f["stocks"]["input"] == TEMPLATE["stocks"]["input"]
        assert f["reals"]["legitimacy"] == TEMPLATE["reals"]["legitimacy"]
        assert set(f.get("ints", {})) <= {"obligation"}
        assert ("ints" in f) == (pg.draw_values(910003, f["id"], "medium")["obligation"] is not None)
    assert TEMPLATE["stocks"]["capital"] == 600  # template not mutated


# -- the declared spec --------------------------------------------------------


@pytest.mark.skipif(not SPEC_FILE.exists(), reason="step-0 hetero_spec.json not present")
def test_the_step_0_spec_file_is_exactly_the_spec_the_script_declares():
    assert SPEC_FILE.read_bytes() == pg.spec_bytes()
    assert hashlib.sha256(SPEC_FILE.read_bytes()).hexdigest() == pg.spec_sha256()


def test_step_0_never_overwrites_and_the_check_refuses_a_changed_spec(tmp_path, capsys):
    p = tmp_path / "spec.json"
    assert pg.main(["--write-spec", str(p)]) == 0
    assert pg.main(["--write-spec", str(p)]) == 1
    p.write_bytes(pg.spec_bytes().replace(b"0.25", b"0.30", 1))
    assert hc.main(["--spec", str(p), "--out", str(tmp_path / "o")]) == 1
    assert "REFUSED" in capsys.readouterr().out and not (tmp_path / "o").exists()


def test_spread_levels_are_the_three_declared_ones_in_order():
    assert pg.SPREAD_LEVELS == ("low", "medium", "high")
    assert pg.SPEC["spread_levels"]["high"] == {"cv_cap": 1.0, "sd_cap": 0.2, "a_spread": 0.5}


# -- configs ------------------------------------------------------------------


def test_q2_config_fixed_items_and_fork_differs_only_by_the_shock_rule():
    s = hc.build_config(BASE, 911_007, "medium", shocked=True)
    f = hc.build_config(BASE, 911_007, "medium", shocked=False)
    assert s["seeds"] == {"mechanism": 910_001, "environment": 910_002,
                          "shock": 911_007, "init": 911_007}
    assert s["world"]["global_ints"] == {"theta_q": 40, "input_price": 2, "output_price": 3}
    assert s["world"]["global_reals"] == {"theta_limit": 0.90, "theta_cap": 0.4}
    assert s["world"]["ticks"] == 120 and len(s["agents"]) == 12
    assert s["agents"] == pg.population(911_007, 12, "medium", TEMPLATE)
    ids = [r["id"] for r in s["rules"]]
    assert "observation.full" in ids and "observation.delayed" not in ids
    p = next(r for r in s["rules"] if r["id"] == "shock.stochastic")["params"]
    assert p["magnitude_mean"] == 0.20 and p["magnitude_sd"] == pytest.approx(0.20 / 3)
    assert p["targets"] == {"kind": "all"} and (p["onset_min"], p["onset_max"]) == (10, 80)
    assert [r for r in s["rules"] if r["id"] != "shock.stochastic"] == f["rules"]
    for k in ("seeds", "world", "agents", "environment"):
        assert s[k] == f[k]
    assert json.dumps(s, sort_keys=True) == json.dumps(
        hc.build_config(BASE, 911_007, "medium", shocked=True), sort_keys=True)


def test_a_reserved_or_low_seed_is_refused():
    with pytest.raises(hc.sl.SliceSeedError):
        hc.build_config(BASE, 200, "low", shocked=False)


def test_q5_config_has_one_firm_all_four_pins_beta_one_and_no_shock():
    cfg = hc.build_q5_config(BASE, 910_009, drawn=True, pinned_margin=0.02, pinned_shortfall=1.0)
    assert cfg["agents"] == pg.population(910_009, 1, "medium", TEMPLATE)
    assert cfg["agents"][0] == pg.population(910_009, 12, "medium", TEMPLATE)[0]
    pins = {(i["field"], i["value"]) for i in cfg["interventions"]}
    assert pins == {("pinned_margin", 0.02), ("pinned_shortfall_capital_growth", 1.0),
                    ("pinned_shortfall_capability", 0.0),
                    ("pinned_shortfall_obligation_clearance", 0.0)}
    assert all(i == {"at": 0, "op": "set_agent_real", "agent": 0, "field": i["field"],
                     "value": i["value"]} for i in cfg["interventions"])
    ids = [r["id"] for r in cfg["rules"]]
    assert "shock.stochastic" not in ids and "shock.scheduled" not in ids
    assert next(r for r in cfg["rules"] if r["id"] == "decision.satisficing")["params"] == {
        "l_w": 4, "beta": 1.0}
    assert cfg["seeds"]["init"] == 910_009 and cfg["seeds"]["shock"] == 910_001
    fixed = hc.build_q5_config(BASE, 910_009, drawn=False, pinned_margin=0.4, pinned_shortfall=0.0)
    assert fixed["agents"] == [TEMPLATE]


# -- Q2 / Q3 / Q4 arithmetic --------------------------------------------------


def test_death_metrics_see_a_synchronised_cliff():
    # the R18 cell 79 pattern: two solvency deaths at 40, nine compliance deaths in one tick
    cliff = [(40, 1, "solvency"), (40, 2, "solvency")] + [(48, i, "compliance") for i in range(3, 12)]
    m = hc.death_metrics(cliff, 12)
    assert m == {"survivors": 1, "collapse": True, "distinct_compliance_death_ticks": 1,
                 "compliance_deaths": 9, "max_deaths_in_one_tick": 9, "mass_death_tick": True}


def test_death_metrics_on_spread_out_deaths_and_on_no_deaths():
    spread = [(10, 0, "compliance"), (20, 1, "compliance"), (20, 2, "solvency"), (30, 3, "compliance")]
    m = hc.death_metrics(spread, 12)
    assert (m["survivors"], m["collapse"], m["distinct_compliance_death_ticks"],
            m["max_deaths_in_one_tick"], m["mass_death_tick"]) == (8, False, 3, 2, False)
    none = hc.death_metrics([], 12)
    assert (none["survivors"], none["collapse"], none["mass_death_tick"],
            none["distinct_compliance_death_ticks"]) == (12, False, False, 0)


def test_collapse_is_at_most_two_survivors_of_twelve():
    dead = lambda k: [(i, i, "compliance") for i in range(k)]  # noqa: E731
    assert hc.death_metrics(dead(10), 12)["collapse"] is True       # 2 survivors
    assert hc.death_metrics(dead(9), 12)["collapse"] is False       # 3 survivors
    # five deaths in five different ticks is not a mass-death tick
    assert hc.death_metrics(dead(5), 12)["mass_death_tick"] is False


def test_the_q3_difference_rule():
    assert hc.differs(0.10, 0.21) and not hc.differs(0.50, 0.60)      # absolute > 0.10
    assert hc.differs(0.02, 0.05) and not hc.differs(0.04, 0.08)      # relative > 2x
    assert hc.differs(0.0, 0.02) and not hc.differs(0.0, 0.0)


def test_the_q4_verdict_needs_all_three_criteria():
    ok = {"t1_pass_share": 0.5, "collapse_rate": 0.10, "nonzero_matched_delta_h_share": 0.5}
    assert hc.q4_verdict(ok)["pass"]
    for key, bad in (("t1_pass_share", 0.49), ("collapse_rate", 0.11),
                     ("nonzero_matched_delta_h_share", 0.49)):
        assert not hc.q4_verdict(dict(ok, **{key: bad}))["pass"]


# -- one real pair ------------------------------------------------------------


@pytest.mark.skipif(not FIRMA_BIN.exists(),
                    reason=f"{FIRMA_BIN} not built -- run `cargo build` first")
def test_one_drawn_population_end_to_end_and_byte_equal_on_rerun(tmp_path):
    b = hc.build_config(BASE, 910_001, "low", shocked=False)
    s = hc.build_config(BASE, 910_001, "low", shocked=True)
    first = hc.evaluate_pair("p1", 12, b, s, tmp_path, FIRMA_BIN)
    again = hc.evaluate_pair("p2", 12, b, s, tmp_path, FIRMA_BIN)
    assert first["failed"] is None
    assert first["shocked_events_sha256"] == again["shocked_events_sha256"]
    assert first["baseline_events_sha256"] == again["baseline_events_sha256"]
    assert first["shocked_survivors"] == 12 - len(first["shocked_death_records"])
    assert 0 <= first["shocked_survivors"] <= 12 and isinstance(first["T1"], bool)
    assert not (tmp_path / "p1_b").exists()


@pytest.mark.skipif(not FIRMA_BIN.exists(),
                    reason=f"{FIRMA_BIN} not built -- run `cargo build` first")
def test_one_pinned_single_firm_run(tmp_path):
    r = hc._q5_worker((True, 0.40, 0.0, 910_001, str(tmp_path), str(FIRMA_BIN)))
    assert r["failed"] is None and r["decisions"] > 0 and len(r["action_sha"]) == 64
