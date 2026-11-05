"""Tests for scripts/exploratory_e1_slice.py (review round R12).

EXPLORATORY - DIAGNOSTIC-ONLY. The slice is not a registered experiment;
these tests pin its arithmetic and its guards, not any scientific result.
"""

from __future__ import annotations

import importlib.util
import json
import math
from pathlib import Path

import pandas as pd
import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
FIRMA_BIN = REPO_ROOT / "target" / "debug" / "firma"


def _load_slice():
    path = REPO_ROOT / "scripts" / "exploratory_e1_slice.py"
    spec = importlib.util.spec_from_file_location("exploratory_e1_slice", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


sl = _load_slice()

# §11 canonical action indices used below.
HOLD, PRODUCE_ORDINARY, DELIVER = 0, 1, 5


# -- manual Sec 15.4, Example D ------------------------------------------------


def test_repertoire_entropy_matches_manual_example_15_4():
    window = [PRODUCE_ORDINARY] * 5 + [DELIVER] * 2 + [HOLD]
    assert len(window) == 8
    assert sl.repertoire_entropy(window) == pytest.approx(1.298795, abs=1e-6)
    assert sl.normalised_entropy(window, 9) == pytest.approx(0.409724, abs=1e-6)
    assert len(set(window)) == 3  # |A_used| = 3, reported with |A_adm| = 9


def test_repertoire_entropy_edge_cases():
    assert sl.repertoire_entropy([]) == 0.0
    assert sl.repertoire_entropy([2, 2, 2, 2]) == 0.0
    assert sl.repertoire_entropy([0, 1]) == pytest.approx(1.0)
    assert sl.repertoire_entropy(list(range(8))) == pytest.approx(3.0)
    # order within the window does not matter
    assert sl.repertoire_entropy([1, 5, 1, 0, 1, 5, 1, 1]) == pytest.approx(1.298795, abs=1e-6)


# -- mean over live firm-ticks ------------------------------------------------


def test_mean_entropy_uses_a_trailing_window_per_firm_tick():
    # One firm, actions 1,1,2,2 at ticks 0..3, window 2:
    # windows [1], [1,1], [1,2], [2,2] -> entropies 0, 0, 1, 0 -> mean 0.25.
    by_agent = {0: [(0, 1), (1, 1), (2, 2), (3, 2)]}
    mean, n = sl.mean_repertoire_entropy(by_agent, {}, l_w=2)
    assert (mean, n) == (pytest.approx(0.25), 4)
    # window 8 keeps everything: [1], [1,1], [1,1,2], [1,1,2,2]
    mean8, _ = sl.mean_repertoire_entropy(by_agent, {}, l_w=8)
    h3 = -(2 / 3) * math.log2(2 / 3) - (1 / 3) * math.log2(1 / 3)
    assert mean8 == pytest.approx((0 + 0 + h3 + 1.0) / 4)


def test_the_tick_a_firm_dies_in_is_not_a_live_firm_tick():
    by_agent = {0: [(0, 1), (1, 2), (2, 1)], 1: [(0, 1), (1, 1)]}
    # firm 1 dies at tick 1: only its tick-0 window counts.
    mean, n = sl.mean_repertoire_entropy(by_agent, {1: (1, "solvency")}, l_w=8)
    h3 = -(2 / 3) * math.log2(2 / 3) - (1 / 3) * math.log2(1 / 3)
    assert n == 4
    assert mean == pytest.approx((0 + 1.0 + h3 + 0) / 4)


def test_decisions_and_deaths_are_read_from_an_event_frame():
    rows = [
        {"event": "delta_applied", "tick": 0, "origin": "decision.satisficing",
         "target": {"Agent": 0}, "kind": {"SetAgentInt": {"field": "selected_action", "value": 2}}},
        {"event": "delta_applied", "tick": 0, "origin": "decision.satisficing",
         "target": {"Agent": 0}, "kind": {"SetAgentInt": {"field": "focus", "value": 1}}},
        # not a decision: the constrain rule also writes ints
        {"event": "delta_applied", "tick": 0, "origin": "constraint.enforce",
         "target": {"Agent": 0}, "kind": {"SetAgentInt": {"field": "selected_action", "value": 9}}},
        {"event": "delta_applied", "tick": 1, "origin": "decision.satisficing",
         "target": {"Agent": 0}, "kind": {"SetAgentInt": {"field": "selected_action", "value": 1}}},
        {"event": "agent_died", "tick": 1, "agent": 0, "cause": "compliance"},
    ]
    by_agent, deaths = sl.decisions_and_deaths(pd.json_normalize(rows))
    assert by_agent == {0: [(0, 2), (1, 1)]}
    assert deaths == {0: (1, "compliance")}


def test_a_run_in_which_no_firm_dies_has_no_deaths_and_does_not_raise():
    # No agent_died record at all, so the event frame has no "agent"/"cause"
    # columns. R18: this raised KeyError('cause').
    rows = [
        {"event": "delta_applied", "tick": t, "origin": "decision.satisficing",
         "target": {"Agent": 0}, "kind": {"SetAgentInt": {"field": "selected_action", "value": t}}}
        for t in range(3)
    ]
    frame = pd.json_normalize(rows)
    assert "cause" not in frame.columns
    by_agent, deaths = sl.decisions_and_deaths(frame)
    assert by_agent == {0: [(0, 0), (1, 1), (2, 2)]}
    assert deaths == {}
    assert set(sl.firm_tick_entropies(by_agent, deaths, 8)) == {(0, 0), (0, 1), (0, 2)}


# -- matched firm-ticks (SLICE-2, review R14) ---------------------------------


def _rust_like(keys, h, smax):
    return {k: {"h": h, "shortfall_max": smax} for k in keys}


def test_firm_tick_entropies_are_keyed_by_agent_and_tick_and_skip_the_death_tick():
    by_agent = {0: [(0, 1), (1, 2), (2, 1)], 1: [(0, 1), (1, 1)]}
    ent = sl.firm_tick_entropies(by_agent, {1: (1, "solvency")}, l_w=8)
    assert set(ent) == {(0, 0), (0, 1), (0, 2), (1, 0)}
    assert ent[(0, 1)] == pytest.approx(1.0)
    mean, n = sl.mean_repertoire_entropy(by_agent, {1: (1, "solvency")}, l_w=8)
    assert (mean, n) == (pytest.approx(sum(ent.values()) / 4), 4)


def test_a_firm_dying_earlier_in_the_shocked_branch_drops_out_of_matching():
    # Fork: firms 0 and 1 both live through ticks 0..3.
    # Shocked: firm 1 dies in tick 2, so it is live only in ticks 0..1.
    fork = {(a, t): 0.0 for a in (0, 1) for t in range(4)}
    shocked = {(0, 0): 0.0, (0, 1): 1.0, (0, 2): 1.0, (0, 3): 1.0, (1, 0): 0.0, (1, 1): 0.5}
    fork[(1, 2)], fork[(1, 3)] = 9.0, 9.0  # must never enter: firm 1 is dead in the shocked run
    ft = {k: {"h": 0.1 * (k[1] + 1), "shortfall_max": float(k[0])} for k in shocked}
    m = sl.matched_summary(shocked, fork, ft)
    assert m["n_matched_firm_ticks"] == 6 and m["n_matched_firms"] == 2
    assert json.loads(m["matched_ticks_per_firm"]) == {"0": 4, "1": 2}
    # firm 0: mean(0, 1, 1, 1) = 0.75; firm 1: mean(0, 0.5) = 0.25; mean over firms = 0.5
    assert m["delta_h_rep_matched"] == pytest.approx(0.5)
    # NOT the pooled firm-tick mean (3.5 / 6): each firm weighs the same
    assert m["delta_h_rep_matched"] != pytest.approx(3.5 / 6)
    # regressor / covariate: pooled over the six matched firm-ticks, shocked branch
    assert m["h_mean_matched"] == pytest.approx((0.1 + 0.2 + 0.3 + 0.4 + 0.1 + 0.2) / 6)
    assert m["shortfall_max_mean_matched"] == pytest.approx(2 / 6)


def test_matching_also_drops_ticks_missing_from_the_fork_and_handles_no_overlap():
    shocked = {(0, 0): 1.0, (0, 1): 1.0}
    fork = {(0, 0): 0.25}
    m = sl.matched_summary(shocked, fork, _rust_like(shocked, 0.3, 2.0))
    assert m["n_matched_firm_ticks"] == 1 and m["delta_h_rep_matched"] == pytest.approx(0.75)
    empty = sl.matched_summary({(0, 0): 1.0}, {(1, 0): 1.0}, {})
    assert empty["n_matched_firm_ticks"] == 0 and math.isnan(empty["delta_h_rep_matched"])


def test_identical_branches_give_exactly_zero_matched_delta():
    h = {(a, t): 0.1 * a + 0.01 * t for a in range(3) for t in range(5)}
    m = sl.matched_summary(h, dict(h), _rust_like(h, 0.2, 1.0))
    assert m["delta_h_rep_matched"] == 0.0 and m["n_matched_firm_ticks"] == 15


def test_a_matched_firm_tick_without_a_rust_record_is_an_error():
    with pytest.raises(ValueError, match="no Rust firm-tick record"):
        sl.matched_summary({(0, 0): 1.0}, {(0, 0): 0.0}, {})


def test_an_arm_with_no_outcome_variation_is_uninterpretable_not_a_verdict():
    rows = [{"seed": 910_001 + i, sl.OUTCOME_COL: 0.0, sl.PREDICTOR_COL: 0.1 * i,
             sl.COVARIATE_COL: 1.0 + i} for i in range(12)]
    f = sl.fit_arm(rows, n_resamples=20)
    assert f["verdict"] == "uninterpretable" and "no variation" in f["reason"]


# -- seed guard ---------------------------------------------------------------


@pytest.mark.parametrize("bad", [1, 200, 909, 909_999])
def test_guard_refuses_any_seed_below_910000(bad):
    seeds = {"mechanism": 910_001, "environment": 910_002, "shock": bad, "init": 910_004}
    with pytest.raises(sl.SliceSeedError):
        sl.check_seeds(seeds)
    base = json.loads(sl.BASE_CONFIG.read_text())
    with pytest.raises(sl.SliceSeedError):
        sl.build_config(base, decision="satisficing", shock_seed=bad, shocked=True)


def test_guard_refuses_before_anything_is_written(tmp_path):
    cfg = sl.build_config(json.loads(sl.BASE_CONFIG.read_text()),
                          decision="satisficing", shock_seed=910_001, shocked=True)
    cfg["seeds"]["environment"] = 7
    with pytest.raises(sl.SliceSeedError):
        sl.run_config(cfg, tmp_path / "run", tmp_path / "no-such-binary")
    assert not (tmp_path / "run").exists()


def test_the_designed_seeds_pass_the_guard():
    for s in sl.SHOCK_SEEDS:
        sl.check_seeds({"mechanism": sl.MECHANISM_SEED, "environment": sl.ENVIRONMENT_SEED,
                        "shock": s, "init": sl.INIT_SEED})
    assert (sl.SHOCK_SEEDS.start, sl.SHOCK_SEEDS.stop, len(sl.SHOCK_SEEDS)) == (910_001, 910_051, 50)


# -- config construction ------------------------------------------------------


def _rule_ids(cfg):
    return [r["id"] for r in cfg["rules"]]


def test_shocked_config_replaces_the_scheduled_shock_and_nothing_else():
    base = json.loads(sl.BASE_CONFIG.read_text())
    cfg = sl.build_config(base, decision="satisficing", shock_seed=910_017, shocked=True)
    assert cfg["world"]["ticks"] == 120
    assert cfg["seeds"] == {"mechanism": 910_001, "environment": 910_002,
                            "shock": 910_017, "init": 910_004}
    ids = _rule_ids(cfg)
    assert "shock.scheduled" not in ids and ids.count("shock.stochastic") == 1
    p = next(r for r in cfg["rules"] if r["id"] == "shock.stochastic")["params"]
    assert (p["onset_min"], p["onset_max"], p["magnitude_mean"], p["magnitude_sd"]) == (10, 80, 0.3, 0.1)
    assert p["channel"] == {"kind": "regulatory", "target": "theta_limit"}
    # every other rule is the base config's, in order
    assert [i for i in ids if i != "shock.stochastic"] == \
        [r["id"] for r in base["rules"] if r["id"] != "shock.scheduled"]
    assert base["world"]["ticks"] == 16, "the base config must not be mutated"


def test_no_shock_fork_differs_from_its_shocked_run_only_by_the_shock_rule():
    base = json.loads(sl.BASE_CONFIG.read_text())
    shocked = sl.build_config(base, decision="satisficing", shock_seed=910_017, shocked=True)
    fork = sl.build_config(base, decision="satisficing", shock_seed=910_017, shocked=False)
    assert not any(i.startswith("shock.") for i in _rule_ids(fork))
    assert fork["seeds"] == shocked["seeds"]
    strip = lambda c: {**c, "experiment": None,
                       "rules": [r for r in c["rules"] if not r["id"].startswith("shock.")]}
    assert strip(fork) == strip(shocked)


def test_null_arm_swaps_only_the_decision_plugin():
    base = json.loads(sl.BASE_CONFIG.read_text())
    sat = sl.build_config(base, decision="satisficing", shock_seed=910_001, shocked=True)
    rnd = sl.build_config(base, decision="random", shock_seed=910_001, shocked=True)
    assert "decision.random" in _rule_ids(rnd) and "decision.satisficing" not in _rule_ids(rnd)
    swap = lambda ids: ["DECISION" if i.startswith("decision.s") or i == "decision.random" else i
                        for i in ids]
    assert swap(_rule_ids(rnd)) == swap(_rule_ids(sat))


# -- KNOWN-ANSWER control ------------------------------------------------------


def test_known_answer_frames_have_the_real_frames_columns():
    df = sl.synthetic_frame(-1.0)
    assert list(df.columns) == ["seed", sl.PREDICTOR_COL, sl.COVARIATE_COL, sl.OUTCOME_COL]
    assert len(df) == df["seed"].nunique() == 50 and df["seed"].min() >= sl.MIN_DIAGNOSTIC_SEED


def test_known_answer_controls_give_supported_and_not_supported():
    out = sl.known_answer_controls(n_resamples=120)
    assert out["planted inverted-U"]["verdict"] == "supported"
    assert out["planted monotone"]["verdict"] == "not_supported"
    assert all(r["ok"] for r in out.values())


def test_known_answer_also_holds_on_the_existing_test_stats_fixture():
    path = Path(__file__).with_name("test_stats.py")
    spec = importlib.util.spec_from_file_location("_test_stats_for_slice", path)
    ts = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ts)
    assert ts.h1c(ts.arm_b_curve(-1.0)).verdict == "supported"
    assert ts.h1c(ts.arm_b_curve(0.0)).verdict == "not_supported"


# -- one real run end to end (needs the CLI) ----------------------------------


@pytest.mark.skipif(not FIRMA_BIN.exists(),
                    reason=f"{FIRMA_BIN} not built -- run `cargo build` first")
def test_one_seed_end_to_end_and_cross_check_against_firma_lab_metrics(tmp_path):
    base = json.loads(sl.BASE_CONFIG.read_text())
    rows, failed = sl.run_arm(base, "satisficing", tmp_path, FIRMA_BIN, [910_001])
    assert failed == [] and len(rows) == 1
    r = rows[0]
    # my mean, at the model's own window, equals firma-analysis' sc6_entropy_mean
    assert abs(r["crosscheck_diff"]) < 1e-9
    assert r["delta_h_rep"] == pytest.approx(r["h_rep_shocked_l8"] - r["h_rep_noshock_l8"])
    assert r["drawn_onset"] is not None and 10 <= r["drawn_onset"] <= 80
    assert r["action_sha_shocked"] != r["action_sha_noshock"]
    # SLICE-2 columns
    assert r["n_matched_firm_ticks"] == r["live_firm_ticks_shocked"] <= r["live_firm_ticks_noshock"]
    assert r["n_matched_firms"] == 4
    assert math.isfinite(r["delta_h_rep_matched"]) and math.isfinite(r["h_mean_matched"])
    assert math.isfinite(r["shortfall_max_mean_matched"])


@pytest.mark.skipif(not FIRMA_BIN.exists(),
                    reason=f"{FIRMA_BIN} not built -- run `cargo build` first")
def test_rust_shortfall_equals_the_plugins_own_decision_time_value(tmp_path):
    """One run with the plugin's diagnostic trace on: the shortfalls
    firma_analysis reconstructs from the log equal the ones
    decision.satisficing passed to select(), up to float-parsing noise."""
    import os
    import subprocess

    from firma_lab import _native, load

    cfg = sl.build_config(json.loads(sl.BASE_CONFIG.read_text()),
                          decision="satisficing", shock_seed=910_001, shocked=True)
    (tmp_path / "c.json").write_text(json.dumps(cfg))
    env = dict(os.environ, FIRMA_TRACE_DECISION="1",
               FIRMA_TRACE_DECISION_PATH=str(tmp_path / "trace.ndjson"))
    subprocess.run([str(FIRMA_BIN), "run", "--model", "--config", str(tmp_path / "c.json"),
                    "--out", str(tmp_path / "run")], check=True, capture_output=True, env=env)
    trace = {}
    for line in open(tmp_path / "trace.ndjson"):
        rec = json.loads(line)
        if rec["kind"] == "selection":
            trace[(rec["tick"], rec["agent"])] = rec["shortfalls"]
    rust = _native.firm_ticks(load.read_config_json(tmp_path / "run"), str(tmp_path / "run"))
    assert len(rust) > 200
    for f in rust:
        plugin = trace[(f["tick"], f["agent"])]
        assert f["shortfalls"] == pytest.approx(plugin, abs=1e-9)
        assert f["shortfall_max"] == pytest.approx(max(plugin), abs=1e-9)
        assert (f["shortfall_max"] > 0) == (max(plugin) > 0)
