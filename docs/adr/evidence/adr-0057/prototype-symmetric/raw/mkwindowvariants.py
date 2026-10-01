"""Construct cfg_arm_b_satisficing window-variant configs (l_w=6, l_w=8,
w_max=15) by patching the l_w/w_max fields of decision.satisficing,
constraint.action_window, and constraint.enforce in a copy of the config
extract_cfg.py produces for cfg_arm_b_satisficing. Reproduces the prior
Option-A round's own HEAD and A numbers exactly (verified: decisions=1834/
3242/1562 and HEAD shaping_selected=3/12/9 match prototype-option-a/
adr0050_window_variants_head_vs_prototype.txt's "head" row; A's 0/12/5
match its "proto" row)."""
import json, sys
src = json.load(open(sys.argv[1]))  # cfg/arm_b.json from extract_cfg.py
def set_lw_wmax(cfg, lw=None, wmax=None):
    c = json.loads(json.dumps(cfg))
    for rule in c["rules"]:
        if rule["id"] == "decision.satisficing":
            if lw is not None: rule["params"]["l_w"] = lw
            if wmax is not None: rule["params"]["w_max"] = wmax
        if rule["id"] in ("constraint.action_window", "constraint.enforce"):
            if lw is not None: rule["params"]["l_w"] = lw
    return c
json.dump(set_lw_wmax(src, lw=6), open(sys.argv[2], "w"), indent=1)
json.dump(set_lw_wmax(src, lw=8), open(sys.argv[3], "w"), indent=1)
json.dump(set_lw_wmax(src, wmax=15), open(sys.argv[4], "w"), indent=1)
