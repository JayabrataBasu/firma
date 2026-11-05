#!/usr/bin/env python3
"""DIAGNOSTIC-ONLY (R06). Writes the configs inertness_run.sh feeds to
inertness.sh. usage: mk_inertness_cfgs.py <HEAD-export-dir> <out-dir>

- shocked_k{0..4}.json: configs/experiments/phase2-stage5-smoke.json exactly
  as shipped (shock.scheduled, resource.patchy, observation.delayed) except
  ticks=120, snapshot_every=120, and seeds (910001+10k, 910002+10k,
  910003+10k, 910004+10k).
- probe_<variant>_w<w>_b<b>.json: docs/adr/evidence/adr-0057/mkcfg.py's
  cfg(w_max, beta, variant) with ONLY the seeds replaced: the script's own
  (909, 2, 3, 4) has three streams inside 1-200, so they become
  (910001, 910002, 910003, 910004).
"""
import json, os, subprocess, sys
EXPORT, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
base = json.load(open(f"{EXPORT}/configs/experiments/phase2-stage5-smoke.json"))
for k in range(5):
    c = json.loads(json.dumps(base))
    c["experiment"] = f"DIAGNOSTIC-ONLY-R06-inertness-shocked-k{k}"
    c["world"]["ticks"] = 120; c["world"]["snapshot_every"] = 120
    c["seeds"] = {n: 910001 + i + 10 * k for i, n in enumerate(("mechanism", "environment", "shock", "init"))}
    json.dump(c, open(f"{OUT}/shocked_k{k}.json", "w"), indent=1)
MK = f"{EXPORT}/docs/adr/evidence/adr-0057/mkcfg.py"
def probe(variant, w, b):
    p = f"{OUT}/probe_{variant}_w{w}_b{b}.json"
    subprocess.run([sys.executable, MK, str(w), str(b), variant, p], check=True)
    c = json.load(open(p))
    assert c["seeds"] == {"mechanism": 909, "environment": 2, "shock": 3, "init": 4}
    c["seeds"] = {"mechanism": 910001, "environment": 910002, "shock": 910003, "init": 910004}
    c["experiment"] = f"DIAGNOSTIC-ONLY-R06-inertness-probe-{variant}"
    json.dump(c, open(p, "w"), indent=1)
for w in (3, 6, 9):
    for b in (0.0, 0.5, 1.0, 2.0, 4.0):
        probe("pre", w, b)
probe("head", 6, 0.0)
bad = [f for f in os.listdir(OUT) if any(v < 910001 for v in json.load(open(f"{OUT}/{f}"))["seeds"].values())]
assert not bad, bad
print(len(os.listdir(OUT)), "configs written; all seeds >= 910001")
