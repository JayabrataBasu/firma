#!/bin/bash
# R07 H-1: is the working tree's numerical output unchanged relative to HEAD?
# usage: hash_equivalence.sh <HEAD-export> <working-tree-copy> <work-dir> <python> <repo-root>
# Uses evidence/R06/inertness.sh (left = HEAD export, right = copy of the
# working tree). DIAGNOSTIC-ONLY configs, all seeds >= 910001.
HEAD=$1; WT=$2; WORK=$3; PY=$4; REPO=$5
I=$REPO/docs/review/evidence/R06/inertness.sh
"$PY" "$REPO/docs/review/evidence/R06/mk_inertness_cfgs.py" "$WT" "$WORK/cfg"
"$PY" - "$WT" "$WORK/cfg" <<'PY'
import json, sys
wt, out = sys.argv[1], sys.argv[2]
# the three shipped smoke configs, seeds replaced (their own seeds have streams inside 1-200)
for name in ("phase1-smoke", "phase2-smoke", "phase2-stage5-smoke"):
    c = json.load(open(f"{wt}/configs/experiments/{name}.json"))
    c["seeds"] = {"mechanism": 910001, "environment": 910002, "shock": 910003, "init": 910004}
    json.dump(c, open(f"{out}/smoke_{name}.json", "w"), indent=1)
PY
eq=0; ne=0
t() { case $1 in 0) eq=$((eq+1));; *) ne=$((ne+1));; esac; }
for k in 0 1 2 3 4; do "$I" "$HEAD" "$WT" "$WORK/runs" "shocked_k$k" "$WORK/cfg/shocked_k$k.json"; t $?; done
for w in 3 6 9; do for b in 0.0 0.5 1.0 2.0 4.0; do "$I" "$HEAD" "$WT" "$WORK/runs" "probe_pre_w${w}_b${b}" "$WORK/cfg/probe_pre_w${w}_b${b}.json"; t $?; done; done
"$I" "$HEAD" "$WT" "$WORK/runs" "probe_optin_head_w6_b0.0" "$WORK/cfg/probe_head_w6_b0.0.json"; t $?
for n in phase2-smoke phase2-stage5-smoke; do "$I" "$HEAD" "$WT" "$WORK/runs" "smoke_$n" "$WORK/cfg/smoke_$n.json"; t $?; done
echo "SUMMARY HEAD vs working tree (expected: all EQUAL): equal=$eq not-equal=$ne"
