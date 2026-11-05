#!/bin/bash
# DIAGNOSTIC-ONLY (R06). usage: inertness_run.sh <old-export 9c49c34> <new-export HEAD> <work-dir> <python>
OLD=$1; NEW=$2; WORK=$3; PY=$4; HERE=$(cd "$(dirname "$0")" && pwd)
"$PY" "$HERE/mk_inertness_cfgs.py" "$NEW" "$WORK/cfg"
eq=0; diff=0; err=0
tally() { case $1 in 0) eq=$((eq+1));; 1) diff=$((diff+1));; *) err=$((err+1));; esac; }
echo "--- A. shocked 4-firm stage5-smoke config (shock.scheduled + resource.patchy + observation.delayed), 120 ticks, 5 seed sets ---"
for k in 0 1 2 3 4; do "$HERE/inertness.sh" "$OLD" "$NEW" "$WORK/runs" "shocked_k$k" "$WORK/cfg/shocked_k$k.json"; tally $?; done
echo "--- B. probe family, mkcfg.py variant 'pre', 15 cells, seeds (910001..910004) ---"
for w in 3 6 9; do for b in 0.0 0.5 1.0 2.0 4.0; do
  "$HERE/inertness.sh" "$OLD" "$NEW" "$WORK/runs" "probe_pre_w${w}_b${b}" "$WORK/cfg/probe_pre_w${w}_b${b}.json"; tally $?
done; done
echo "SUMMARY A+B (expected: all EQUAL): equal=$eq different=$diff error=$err"
echo "--- C. positive controls (expected: NOT equal) ---"
"$HERE/inertness.sh" "$OLD" "$NEW" "$WORK/runs" "control1_optin_head_w6_b0.0_old_vs_new" "$WORK/cfg/probe_head_w6_b0.0.json"; echo "  control1 exit=$? (0 would mean the check cannot see the opt-in mechanism)"
"$HERE/inertness.sh" "$NEW" "$NEW" "$WORK/runs" "control2_new_binary_pre_vs_optin_head_w6_b0.0" "$WORK/cfg/probe_pre_w6_b0.0.json" "$WORK/cfg/probe_head_w6_b0.0.json"; echo "  control2 exit=$? (1 = DIFFERENT = control ok)"
"$HERE/inertness.sh" "$NEW" "$NEW" "$WORK/runs" "control3_new_vs_new_same_config" "$WORK/cfg/shocked_k0.json"; echo "  control3 exit=$? (0 = EQUAL = the comparison is not always-different)"
