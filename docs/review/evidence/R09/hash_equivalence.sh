#!/bin/bash
# R09 H-2 predeclared check: HEAD vs working tree, event logs byte-equal.
# usage: hash_equivalence.sh <HEAD-export> <working-tree-copy> <work-dir> <python> <repo-root>
# Reuses docs/review/evidence/R06/inertness.sh (left = HEAD export, right =
# rsync copy of the working tree). DIAGNOSTIC-ONLY configs; every seed >= 910001.
HEAD=$1; WT=$2; WORK=$3; PY=$4; REPO=$5
I=$REPO/docs/review/evidence/R06/inertness.sh
"$PY" "$REPO/docs/review/evidence/R06/mk_inertness_cfgs.py" "$WT" "$WORK/cfg"
"$PY" - "$WT" "$WORK/cfg" <<'PY'
import copy, json, sys
wt, out = sys.argv[1], sys.argv[2]
base = json.load(open(f"{wt}/configs/experiments/phase2-stage5-smoke.json"))
stoch = {"id": "shock.stochastic", "version": "^1", "params": {
    "id": "rand-reg", "channel": {"kind": "regulatory", "target": "theta_limit"},
    "onset_min": 10, "onset_max": 80, "magnitude_mean": 0.3, "magnitude_sd": 0.1,
    "ramp": {"kind": "instant"}, "persistence": {"kind": "permanent"},
    "observability": {"kind": "full"}, "novelty": 0.5, "targets": {"kind": "all"}}}
for k in range(5):
    for fam in ("stochastic", "random"):
        c = copy.deepcopy(base)
        c["experiment"] = f"DIAGNOSTIC-ONLY-R09-{fam}-k{k}"
        c["world"]["ticks"] = 120; c["world"]["snapshot_every"] = 120
        c["seeds"] = {n: 910001 + i + 10 * k for i, n in enumerate(("mechanism", "environment", "shock", "init"))}
        rules = []
        for r in c["rules"]:
            if r["id"] == "shock.scheduled": rules.append(stoch)
            elif r["id"] == "decision.satisficing" and fam == "random":
                rules.append({"id": "decision.random", "version": "^1", "params": {}})
            else: rules.append(r)
        c["rules"] = rules
        json.dump(c, open(f"{out}/{fam}_k{k}.json", "w"), indent=1)
PY
eq=0; ne=0
t() { case $1 in 0) eq=$((eq+1));; *) ne=$((ne+1));; esac; }
for fam in shocked stochastic random; do for k in 0 1 2 3 4; do
  "$I" "$HEAD" "$WT" "$WORK/runs" "${fam}_k$k" "$WORK/cfg/${fam}_k$k.json"; t $?
done; done
for w in 3 6 9; do for b in 0.0 0.5 1.0 2.0 4.0; do "$I" "$HEAD" "$WT" "$WORK/runs" "probe_pre_w${w}_b${b}" "$WORK/cfg/probe_pre_w${w}_b${b}.json"; t $?; done; done
"$I" "$HEAD" "$WT" "$WORK/runs" "probe_optin_head_w6_b0.0" "$WORK/cfg/probe_head_w6_b0.0.json"; t $?
echo "SUMMARY HEAD vs working tree (expected: all EQUAL): equal=$eq not-equal=$ne"
echo "--- sensitivity control (expected DIFFERENT): working-tree binary, shocked_k0 vs stochastic_k0"
"$I" "$WT" "$WT" "$WORK/runs" "control_shocked_vs_stochastic" "$WORK/cfg/shocked_k0.json" "$WORK/cfg/stochastic_k0.json"; echo "  control exit=$? (1 = DIFFERENT = control ok)"
