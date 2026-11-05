"""Verbatim transcription of tests/tests/sanity.rs::cfg_input_starved (HEAD)."""
import json, sys
m = lambda n: {"id": f"action.market.standard.{n}", "version": "^1", "params": {}}
ag = lambda i, c: {"id": i, "stocks": {"capital": c, "input": 0}, "reals": {"capability": 0.50, "legitimacy": 1.0, "aspiration_capital_growth": 8.0}}
cfg = {"experiment": "sc-starved", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
  "seeds": {"mechanism": 11, "environment": 2, "shock": 3, "init": 4},
  "world": {"ticks": 200, "resources": ["capital", "input"], "conflict_resolver": {"id": "conflict.additive", "version": "^1"}, "snapshot_every": 200, "global_reals": {"theta_limit": 0.90, "theta_cap": 0.15}, "global_ints": {"theta_q": 100, "input_price": 30, "output_price": 3}},
  "agents": [ag(0, 28), ag(1, 27), ag(2, 26)],
  "environment": {"stocks": {"capital": 100000000, "input": 100000000}},
  "rules": [{"id": "decision.satisficing", "version": "^1", "params": {"l_w": 8, "w_max": 9, "shaping": {"lobby_cost": 25,
      "lobby_success": {"lag": {"min": 2, "max": 6}, "success": {"p0": 0.25, "b_lambda": 0.2, "b_kappa": 0.1, "p_max": 0.75, "kappa_min": 25}, "delta_theta": 0.10}}}},
    m("hold"), m("produce_ordinary"), m("produce_regulated"), m("acquire_input"), m("invest_capability"), m("deliver"),
    {"id": "action.shaping.rdt_standard.resolve_lagged", "version": "^1", "params": {}},
    {"id": "action.shaping.rdt_standard.lobby", "version": "^1", "params": {}},
    {"id": "constraint.action_window", "version": "^1", "params": {"l_w": 8}},
    {"id": "constraint.enforce", "version": "^1", "params": {"t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": 8}},
    {"id": "decision.aspiration_update", "version": "^1", "params": {"alpha": 0.10}}]}
json.dump(cfg, open(sys.argv[1], "w"), indent=1)
