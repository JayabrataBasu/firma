#!/bin/bash
# usage: run.sh variant w_max beta
set -e
R=/home/jayabratabasu/firma; D=$PWD/runs/$1-w$2-b$3; rm -rf $D; mkdir -p $D
$R/.venv/bin/python mkcfg.py $2 $3 $1 $D/config.json
FIRMA_TRACE_DECISION=1 FIRMA_TRACE_DECISION_PATH=$D/trace.ndjson $R/target/debug/firma run --model --config $D/config.json --out $D/run >/dev/null
cd $R; PYTHONPATH=python .venv/bin/python -c "
from firma_lab import metrics; r=metrics.sanity_report('$D/run'); print('$1 w_max=$2 beta=$3 shaping', round(r['sc4_shaping_fraction']*r['decisions']), '/', r['decisions'])"
