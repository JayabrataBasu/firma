"""Verbatim transcription of tests/tests/sanity.rs::cfg_h3_survival_lobby_opens (HEAD)."""
import json, sys
m = lambda n: {"id": f"action.market.standard.{n}", "version": "^1", "params": {}}
cfg = {"experiment": "h3-survival-lobby", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
  "seeds": {"mechanism": 47, "environment": 2, "shock": 3, "init": 4},
  "world": {"ticks": 1, "resources": ["capital", "input"], "conflict_resolver": {"id": "conflict.additive", "version": "^1"}, "snapshot_every": 1,
            "global_reals": {"theta_limit": 0.5, "theta_cap": 0.4}, "global_ints": {"theta_q": 100, "input_price": 2, "output_price": 3}},
  "agents": [{"id": 0, "stocks": {"capital": 500, "input": 20}, "reals": {"capability": 0.9, "legitimacy": 1.0, "regulated_intensity": 0.4}, "ints": {"obligation": 3}}],
  "environment": {"stocks": {"capital": 100000000, "input": 100000000}},
  "rules": [{"id": "decision.satisficing", "version": "^1", "params": {"l_w": 8, "beta": 0.5, "w_max": 9, "shaping": {"lobby_cost": 25,
      "lobby_success": {"lag": {"min": 2, "max": 6}, "success": {"p0": 0.25, "b_lambda": 0.2, "b_kappa": 0.1, "p_max": 0.75, "kappa_min": 25}, "delta_theta": 0.10}}}},
    m("hold"), m("produce_ordinary"), m("produce_regulated"), m("acquire_input"), m("invest_capability"), m("deliver"),
    {"id": "action.shaping.rdt_standard.resolve_lagged", "version": "^1", "params": {}},
    {"id": "action.shaping.rdt_standard.lobby", "version": "^1", "params": {}},
    {"id": "constraint.action_window", "version": "^1", "params": {"l_w": 8}},
    {"id": "constraint.enforce", "version": "^1", "params": {"t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": 8}},
    {"id": "decision.aspiration_update", "version": "^1", "params": {"alpha": 0.10}}]}
json.dump(cfg, open(sys.argv[1], "w"), indent=1)
