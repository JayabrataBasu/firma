#!/bin/bash
# usage: runb.sh <label> <firma-binary> <variant> <w_max> <beta>
set -e
R=/home/jayabratabasu/firma; D=$PWD/runs/$1-$3-w$4-b$5; rm -rf $D; mkdir -p $D
$R/.venv/bin/python mkcfg.py $4 $5 $3 $D/config.json
FIRMA_TRACE_DECISION=1 FIRMA_TRACE_DECISION_PATH=$D/trace.ndjson $2 run --model --config $D/config.json --out $D/run >/dev/null
cd $R; PYTHONPATH=python .venv/bin/python -c "
from firma_lab import metrics; r=metrics.sanity_report('$D/run'); print('$1 $3 w_max=$4 beta=$5 shaping', round(r['sc4_shaping_fraction']*r['decisions']), '/', r['decisions'])"
