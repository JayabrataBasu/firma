#!/bin/bash
# DIAGNOSTIC-ONLY (review round R06). Inertness check between two source exports.
#
# usage: inertness.sh <left-export-dir> <right-export-dir> <work-dir> <label> <left-config.json> [<right-config.json>]
#
# Builds `firma` (release) in both export dirs (never in a git checkout: pass
# `git archive` exports), runs <left-config> on the left binary and
# <right-config> (default: the same file) on the right binary with
# `firma run --model`, and prints one line:
#   <label> EQUAL|DIFFERENT|ERROR left=<event_log_sha256|ERROR:msg> right=<...>
# Exit status: 0 EQUAL, 1 DIFFERENT, 2 a side failed to run. The caller decides
# which outcome is expected.
set -u
LEFT=$1; RIGHT=$2; WORK=$3; LABEL=$4; LCFG=$5; RCFG=${6:-$5}
for d in "$LEFT" "$RIGHT"; do
  if [ -e "$d/.git" ]; then echo "$LABEL REFUSED: $d is a git checkout, pass an export"; exit 3; fi
  (cd "$d" && cargo build --release -p firma-cli >/dev/null 2>&1) || { echo "$LABEL ERROR build failed in $d"; exit 2; }
done
run_one() { # <export> <config> <outdir>
  rm -rf "$3"; mkdir -p "$3"
  if out=$("$1/target/release/firma" run --model --config "$2" --out "$3/run" 2>"$3/stderr.txt"); then
    echo "$out" | awk '$1=="event_log_sha256"{print $2}'
  else
    echo "ERROR:$(head -c 160 "$3/stderr.txt" | tr '\n' ' ' | tr -s ' ')"
  fi
}
safe=$(echo "$LABEL" | tr -c 'A-Za-z0-9._-' '_')
l=$(run_one "$LEFT" "$LCFG" "$WORK/$safe/left")
r=$(run_one "$RIGHT" "$RCFG" "$WORK/$safe/right")
case "$l$r" in
  *ERROR:*) echo "$LABEL ERROR left=$l right=$r"; exit 2;;
esac
if [ "$l" = "$r" ]; then echo "$LABEL EQUAL left=$l right=$r"; exit 0
else echo "$LABEL DIFFERENT left=$l right=$r"; exit 1; fi
