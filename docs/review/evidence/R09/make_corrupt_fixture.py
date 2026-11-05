#!/usr/bin/env python3
"""R09 H-2, DIAGNOSTIC-ONLY. Builds a run directory whose event log contains
exactly one corrupt persisted record.
usage: make_corrupt_fixture.py <repo-root> <firma-binary> <out-dir>
Runs configs/experiments/phase2-stage5-smoke.json (seeds replaced by
910001..910004) into <out-dir>/clean, copies it to <out-dir>/corrupt and
replaces ONE entry of ONE action_window ReplaceAgentList record list with
{"tick":"CORRUPT-RECORD"}. Every event line stays valid JSON."""
import json, shutil, subprocess, sys
repo, firma, out = sys.argv[1:4]
cfg = json.load(open(f"{repo}/configs/experiments/phase2-stage5-smoke.json"))
cfg["experiment"] = "DIAGNOSTIC-ONLY-R09-corrupt-record-fixture"
cfg["seeds"] = {"mechanism": 910001, "environment": 910002, "shock": 910003, "init": 910004}
json.dump(cfg, open(f"{out}/DIAGNOSTIC_ONLY_config.json", "w"), indent=1)
shutil.rmtree(f"{out}/clean", ignore_errors=True); shutil.rmtree(f"{out}/corrupt", ignore_errors=True)
subprocess.run([firma, "run", "--model", "--config", f"{out}/DIAGNOSTIC_ONLY_config.json", "--out", f"{out}/clean"], check=True, capture_output=True)
shutil.copytree(f"{out}/clean", f"{out}/corrupt")
lines = open(f"{out}/clean/events.ndjson").read().splitlines()
done = 0
for i, l in enumerate(lines):
    if done: break
    if '"ReplaceAgentList"' in l and '"action_window"' in l:
        e = json.loads(l)
        recs = e["kind"]["ReplaceAgentList"]["records_json"]
        if e["tick"] >= 5 and len(recs) >= 3:
            recs[1] = '{"tick":"CORRUPT-RECORD"}'
            lines[i] = json.dumps(e, separators=(",", ":"), ensure_ascii=False)
            done = 1
            print(f"corrupted line {i+1}: tick {e['tick']}, agent {e['target']}, entry 1 of {len(recs)}")
assert done
open(f"{out}/corrupt/events.ndjson", "w").write("\n".join(lines) + "\n")
