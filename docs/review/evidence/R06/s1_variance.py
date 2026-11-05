#!/usr/bin/env python3
"""DIAGNOSTIC-ONLY (review round R06, task S1 'variance reach'). Not an E1
template; nothing here is a registered run.

usage: s1_variance.py <HEAD-export-dir> <work-dir>

Base config: <export>/configs/experiments/phase2-stage5-smoke.json with
ticks=120 and shock.scheduled replaced by one shock.stochastic rule.
Seeds: mechanism 910001, environment 910002, init 910004; ONLY the shock
seed varies, 910001..910050.

Per run: sha256 of the selected_action event lines, survivors at the
horizon, and mean repertoire entropy = SanityReport.sc6_entropy_mean
(crates/firma-analysis/src/sanity.rs: mean over live firm-ticks with a
non-empty window of H_rep = -sum p log2 p over the trailing action window),
printed by the crate's own example `sanity_report_json`.
"""
import copy, hashlib, json, os, shutil, statistics, subprocess, sys

EXPORT, WORK = sys.argv[1], sys.argv[2]
BIN = f"{EXPORT}/target/release/firma"
REPORT = f"{EXPORT}/target/release/examples/sanity_report_json"
SHOCK_SEEDS = range(910001, 910051)
BASE = json.load(open(f"{EXPORT}/configs/experiments/phase2-stage5-smoke.json"))

def stochastic(onset_min, onset_max, sd):
    return {"id": "shock.stochastic", "version": "^1", "params": {
        "id": "rand-reg", "channel": {"kind": "regulatory", "target": "theta_limit"},
        "onset_min": onset_min, "onset_max": onset_max,
        "magnitude_mean": 0.3, "magnitude_sd": sd,
        "ramp": {"kind": "instant"}, "persistence": {"kind": "permanent"},
        "observability": {"kind": "full"}, "novelty": 0.5, "targets": {"kind": "all"}}}

def config(arm, shock_seed):
    c = copy.deepcopy(BASE)
    c["experiment"] = f"DIAGNOSTIC-ONLY-R06-S1-{arm}-not-an-E1-template"
    c["world"]["ticks"] = 120
    c["world"]["snapshot_every"] = 120
    c["seeds"] = {"mechanism": 910001, "environment": 910002, "shock": shock_seed, "init": 910004}
    if arm == "satisficing":
        sh = stochastic(10, 80, 0.1)
    elif arm.startswith("negative-onset"):
        t = int(arm.rsplit("onset", 1)[1]); sh = stochastic(t, t, 0.0)
    elif arm == "positive-random":
        sh = stochastic(10, 80, 0.1)
    else:
        raise SystemExit(arm)
    rules = []
    for r in c["rules"]:
        if r["id"] == "shock.scheduled":
            rules.append(sh)
        elif r["id"] == "decision.satisficing" and arm == "positive-random":
            rules.append({"id": "decision.random", "version": "^1", "params": {}})
        else:
            rules.append(r)
    assert sum(r["id"] == "shock.stochastic" for r in rules) == 1
    c["rules"] = rules
    assert all(v >= 910001 for v in c["seeds"].values())
    return c

def run(arm, shock_seed):
    d = f"{WORK}/{arm}/shock{shock_seed}"
    shutil.rmtree(d, ignore_errors=True); os.makedirs(d)
    cfg = f"{d}/DIAGNOSTIC_ONLY_config.json"
    json.dump(config(arm, shock_seed), open(cfg, "w"), indent=1)
    p = subprocess.run([BIN, "run", "--model", "--config", cfg, "--out", f"{d}/run"],
                       capture_output=True, text=True)
    if p.returncode != 0:
        raise SystemExit(f"{arm} {shock_seed}: firma failed: {p.stderr.strip()}")
    out = dict(l.split(None, 1) for l in p.stdout.strip().splitlines())
    h = hashlib.sha256(); n = 0; shock = None; deaths = []
    for line in open(f"{d}/run/events.ndjson"):
        if '"field":"selected_action"' in line:
            h.update(line.encode()); n += 1
        elif '"agent_died"' in line:
            e = json.loads(line); deaths.append((e["tick"], e["agent"], e["cause"]))
        elif shock is None and '"active_shocks"' in line:
            recs = json.loads(line)["kind"]["ReplaceGlobalList"]["records_json"]
            s = json.loads(recs[0])["shock"]; shock = (s["onset"], round(s["magnitude"], 4))
    rep = json.loads(subprocess.run([REPORT, cfg, f"{d}/run"], capture_output=True, text=True, check=True).stdout)["sanity"]
    assert rep["survivors"] == int(out["final_live"]), (rep["survivors"], out["final_live"])
    return dict(shock_seed=shock_seed, action_sha=h.hexdigest(), n_action_lines=n,
                survivors=rep["survivors"], h_rep_mean=rep["sc6_entropy_mean"],
                log_sha=out["event_log_sha256"], drawn_onset=shock[0], drawn_magnitude=shock[1],
                deaths=deaths)

ARMS = ["satisficing", "negative-onset10", "negative-onset45", "negative-onset80", "positive-random"]
summary = {}
for arm in ARMS:
    rows = [run(arm, s) for s in SHOCK_SEEDS]
    print(f"\n== arm {arm} ==")
    print("shock_seed drawn_onset drawn_mag action_sha12  n_act survivors H_rep_mean         log_sha12    deaths(tick,agent,cause)")
    for r in rows:
        print(f"{r['shock_seed']} {r['drawn_onset']:>10} {r['drawn_magnitude']:>9} {r['action_sha'][:12]} {r['n_action_lines']:>5} "
              f"{r['survivors']:>9} {r['h_rep_mean']!r:<18} {r['log_sha'][:12]} {r['deaths']}")
    hs = [r["h_rep_mean"] for r in rows]
    summary[arm] = dict(
        distinct_action_hashes=len({r["action_sha"] for r in rows}),
        distinct_full_logs=len({r["log_sha"] for r in rows}),
        survivors_counts={k: sum(r["survivors"] == k for r in rows) for k in sorted({r["survivors"] for r in rows})},
        h_rep_distinct=len(set(hs)), h_rep_min=min(hs), h_rep_max=max(hs),
        h_rep_mean_over_runs=statistics.fmean(hs), h_rep_sd_sample=statistics.stdev(hs))
print("\n== summary (50 shock seeds per arm) ==")
for arm, s in summary.items():
    print(arm, json.dumps(s))
sat = summary["satisficing"]
negs = {a: summary[a]["distinct_action_hashes"] for a in ARMS if a.startswith("negative")}
verdict = sat["distinct_action_hashes"] >= 5 and sat["h_rep_sd_sample"] > 0 and all(v == 1 for v in negs.values())
print("\n== predeclared reading ==")
print(f"satisficing distinct action hashes >= 5 of 50: {sat['distinct_action_hashes']} -> {sat['distinct_action_hashes'] >= 5}")
print(f"satisficing sd(H_rep) > 0: {sat['h_rep_sd_sample']!r} -> {sat['h_rep_sd_sample'] > 0}")
print(f"negative control exactly 1 distinct hash: {negs} -> {all(v == 1 for v in negs.values())}")
print(f"positive control (decision.random) distinct action hashes: {summary['positive-random']['distinct_action_hashes']}")
print(f"'variance reaches the outcome' under the predeclared reading: {verdict}")
