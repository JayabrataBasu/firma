#!/bin/bash
# R14 predeclared check: HEAD vs working tree event logs byte-equal on the R12/R14 slice configs.
# usage: hash_equivalence.sh <HEAD-export> <working-tree-copy> <work-dir> <python> <repo-root>
# 2 arms x 50 shock seeds x (shocked + no-shock fork) = 200 configs, built by the slice
# script's own build_config; compared with docs/review/evidence/R06/inertness.sh.
HEAD=$1; WT=$2; WORK=$3; PY=$4; REPO=$5
I=$REPO/docs/review/evidence/R06/inertness.sh
mkdir -p "$WORK/cfg"
"$PY" - "$REPO" "$WORK/cfg" <<'PY'
import importlib.util, json, sys
repo, out = sys.argv[1], sys.argv[2]
sys.path.insert(0, f"{repo}/python")
spec = importlib.util.spec_from_file_location("sl", f"{repo}/scripts/exploratory_e1_slice.py")
sl = importlib.util.module_from_spec(spec); spec.loader.exec_module(sl)
base = json.loads(sl.BASE_CONFIG.read_text())
for d in ("satisficing", "random"):
    for s in sl.SHOCK_SEEDS:
        for shocked in (True, False):
            c = sl.build_config(base, decision=d, shock_seed=s, shocked=shocked)
            json.dump(c, open(f"{out}/{d}_{s}_{'shocked' if shocked else 'noshock'}.json", "w"))
PY
eq=0; ne=0
for f in "$WORK"/cfg/*.json; do
  "$I" "$HEAD" "$WT" "$WORK/runs" "$(basename "$f" .json)" "$f" > "$WORK/line.txt"; rc=$?
  cat "$WORK/line.txt"
  if [ $rc -eq 0 ]; then eq=$((eq+1)); else ne=$((ne+1)); fi
done
echo "SUMMARY HEAD vs working tree (expected: all EQUAL): equal=$eq not-equal=$ne"
echo "--- sensitivity control (expected DIFFERENT): one binary, shocked vs no-shock fork of seed 910001"
"$I" "$WT" "$WT" "$WORK/runs" "control_shocked_vs_fork" "$WORK/cfg/satisficing_910001_shocked.json" "$WORK/cfg/satisficing_910001_noshock.json"; echo "  control exit=$? (1 = DIFFERENT = control ok)"
