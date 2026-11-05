#!/bin/bash
# R09 H-2: do the example binary and firma_lab._native surface a corrupt record?
# usage: surface_check.sh <repo-root> <fixture-dir>
R=$1; F=$2
for kind in clean corrupt; do
  echo "== sanity_report_json on the $kind log"
  "$R/target/debug/examples/sanity_report_json" "$F/DIAGNOSTIC_ONLY_config.json" "$F/$kind" > "$F/example_$kind.out" 2> "$F/example_$kind.err"; rc=$?
  echo "   exit $rc; stdout $(wc -c < "$F/example_$kind.out") bytes; stderr: $(head -c 400 "$F/example_$kind.err" | tr '\n' ' ')"
  echo "== firma_lab.metrics on the $kind log"
  PYTHONPATH="$R/python" "$R/.venv/bin/python" - "$F" "$kind" <<'PY'
import sys, json
from firma_lab import _native
f, kind = sys.argv[1], sys.argv[2]
cfg = open(f"{f}/DIAGNOSTIC_ONLY_config.json").read()
for name in ("sanity_report", "final_margins"):
    try:
        r = getattr(_native, name)(cfg, f"{f}/{kind}")
        detail = f"sc6_entropy_mean={r['sc6_entropy_mean']!r}" if name == "sanity_report" else f"{len(r)} firms"
        print(f"   {name}: returned normally ({detail})")
    except BaseException as e:
        print(f"   {name}: raised {type(e).__name__}: {str(e)[:330]}")
PY
done
