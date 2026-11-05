#!/usr/bin/env python3
"""R14 diagnostic: why is the Rust-reconstructed shortfall not bit-equal to
the plugin's? Replays ONE run's aspiration_capital_growth arithmetic with
Python's exact float parser (seed from the config, then `A += delta` for each
logged AdjustAgentReal, v_1 = last logged realized_capital_growth) and
compares s_1 = A_1 - v_1 with the plugin trace at each decision. This is a
cause check only; no result in the slice uses this Python arithmetic.
usage: diagnose_float_difference.py <run-pair-dir>"""
import json, sys
d = sys.argv[1]
cfg = json.load(open(f"{d}/c.json"))
A = {a["id"]: a.get("reals", {}).get("aspiration_capital_growth") for a in cfg["agents"]}
v = {a["id"]: 0 for a in cfg["agents"]}
trace = {}
for line in open(f"{d}/trace.ndjson"):
    r = json.loads(line)
    if r["kind"] == "selection": trace[(r["tick"], r["agent"])] = r["shortfalls"][0]
n = exact = 0; worst = 0.0
for line in open(f"{d}/run/events.ndjson"):
    e = json.loads(line)
    if e["event"] != "delta_applied" or not isinstance(e["target"], dict): continue
    a = e["target"]["Agent"]; k = e["kind"]
    if "SetAgentInt" in k and k["SetAgentInt"]["field"] == "selected_action" and e["origin"].startswith("decision."):
        s1 = 0.0 if A[a] is None else A[a] - v[a]
        t = trace[(e["tick"], a)]; n += 1; exact += (s1 == t); worst = max(worst, abs(s1 - t))
    elif "AdjustAgentReal" in k and k["AdjustAgentReal"]["field"] == "aspiration_capital_growth":
        A[a] = (A[a] or 0.0) + k["AdjustAgentReal"]["delta"]
    elif "SetAgentInt" in k and k["SetAgentInt"]["field"] == "realized_capital_growth":
        v[a] = k["SetAgentInt"]["value"]
print(f"{d}: decisions {n}; s_1 replayed with an exact float parser is bit-equal to the plugin's on {exact}; max |difference| {worst!r}")
