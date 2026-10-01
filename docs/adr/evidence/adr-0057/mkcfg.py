"""Verbatim Python transcription of tests/tests/sanity.rs::cfg_wide_search (HEAD)
plus MARKET_ACTIONS and constraint_rules(8). Variant 'pre' drops the two
*_success blocks added by commit 17ba7bb."""
import json, sys, copy
SUCC_L = {"lag": {"min": 2, "max": 6}, "success": {"p0": 0.25, "b_lambda": 0.2, "b_kappa": 0.1, "p_max": 0.75, "kappa_min": 25}, "delta_theta": 0.10}
SUCC_C = {"lag": {"min": 2, "max": 6}, "success": {"p0": 0.30, "b_lambda": 0.20, "b_kappa": 0.10, "p_max": 0.60, "kappa_min": 25}, "delta_q": 4, "q0": 3}
def cfg(w_max, beta, variant):
    shaping = {"lobby_cost": 25, "contract_cost": 25, "diversify_cost": 25}
    if variant in ("head", "lobby_only", "contract_only"):
        if variant != "contract_only": shaping["lobby_success"] = SUCC_L
        if variant != "lobby_only": shaping["contract_success"] = SUCC_C
    if variant == "head_no_gate":
        shaping.update(lobby_success=SUCC_L, contract_success=SUCC_C, require_time_margin=False)
    m = lambda n: {"id": f"action.market.standard.{n}", "version": "^1", "params": {}}
    return {
      "experiment": "sc-wide", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": {"mechanism": 909, "environment": 2, "shock": 3, "init": 4},
      "world": {"ticks": 200, "resources": ["capital", "input"],
        "conflict_resolver": {"id": "conflict.additive", "version": "^1"}, "snapshot_every": 200,
        "global_reals": {"theta_limit": 0.90, "theta_cap": 0.15},
        "global_ints": {"theta_q": 100, "input_price": 2, "output_price": 1}},
      "agents": [
        {"id": 0, "stocks": {"capital": 100000, "input": 500000},
         "reals": {"capability": 0.40, "legitimacy": 1.0, "aspiration_capital_growth": 100000.0}},
        {"id": 1, "stocks": {"capital": 100000, "input": 500000},
         "reals": {"capability": 0.50, "legitimacy": 1.0, "aspiration_capital_growth": 100000.0}}],
      "environment": {"stocks": {"capital": 100000000, "input": 100000000}},
      "rules": [
        {"id": "decision.satisficing", "version": "^1", "params": {"l_w": 8, "w_max": w_max, "beta": beta, "shaping": shaping}},
        m("hold"), m("produce_ordinary"), m("produce_regulated"), m("acquire_input"), m("invest_capability"), m("deliver"),
        {"id": "action.shaping.rdt_standard.resolve_lagged", "version": "^1", "params": {}},
        {"id": "action.shaping.rdt_standard.lobby", "version": "^1", "params": {}},
        {"id": "action.shaping.rdt_standard.contract", "version": "^1", "params": {"delta_q": 4, "q0": 3, "success": SUCC_C["success"], "lag": {"min": 2, "max": 6}}},
        {"id": "action.shaping.rdt_standard.diversify", "version": "^1", "params": {"success": {"p0": 0.30, "b_lambda": 0.20, "b_kappa": 0.10, "p_max": 0.60, "kappa_min": 25}, "lag": {"min": 2, "max": 6}}},
        {"id": "constraint.action_window", "version": "^1", "params": {"l_w": 8}},
        {"id": "constraint.enforce", "version": "^1", "params": {"t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": 8}},
        {"id": "decision.aspiration_update", "version": "^1", "params": {"alpha": 0.10}}]}
w, b, v, out = int(sys.argv[1]), float(sys.argv[2]), sys.argv[3], sys.argv[4]
json.dump(cfg(w, b, v), open(out, "w"), indent=1)
