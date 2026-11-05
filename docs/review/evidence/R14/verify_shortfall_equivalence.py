#!/usr/bin/env python3
"""R14, DIAGNOSTIC-ONLY. Is the analysis crate's reconstructed shortfall equal
to the value decision.satisficing itself used?

usage: verify_shortfall_equivalence.py <repo-root> <work-dir>

For every shock seed 910001..910050 (the R12/R14 slice configs, satisficing
arm, shocked run and no-shock fork) the run is executed with
FIRMA_TRACE_DECISION=1, so the plugin writes the `shortfalls` array and `h_t`
it passed to select() for each decision. Those are compared, per (tick,
agent), with firma_lab._native.firm_ticks (Rust, firma-analysis) on the same
event log. Shortfalls are compared exactly and the largest difference is reported. h is compared for information only:
the analysis h is the true margin at the end of the tick, the plugin's h_t is
the perceived margin at decision time (observed theta, ADR 0035)."""
import importlib.util, json, os, subprocess, sys
from pathlib import Path
repo, work = Path(sys.argv[1]), Path(sys.argv[2])
sys.path.insert(0, str(repo / "python"))
spec = importlib.util.spec_from_file_location("sl", repo / "scripts" / "exploratory_e1_slice.py")
sl = importlib.util.module_from_spec(spec); spec.loader.exec_module(sl)
from firma_lab import _native, load
firma = sl.find_firma(None)
base = json.loads(sl.BASE_CONFIG.read_text())
n_cmp = n_runs = 0; max_abs = 0.0; max_abs_max = 0.0; max_rel = 0.0; sign_flips = 0; argmax_flips = 0; n_unequal = 0; h_abs = []; only_trace = 0; only_rust = 0
for seed in sl.SHOCK_SEEDS:
    for shocked in (True, False):
        d = work / f"shock{seed}" / ("shocked" if shocked else "noshock")
        cfg = sl.build_config(base, decision="satisficing", shock_seed=seed, shocked=shocked)
        sl.check_seeds(cfg["seeds"])
        if d.exists():
            import shutil; shutil.rmtree(d)
        d.mkdir(parents=True)
        (d / "c.json").write_text(json.dumps(cfg))
        env = dict(os.environ, FIRMA_TRACE_DECISION="1", FIRMA_TRACE_DECISION_PATH=str(d / "trace.ndjson"))
        subprocess.run([str(firma), "run", "--model", "--config", str(d / "c.json"), "--out", str(d / "run")],
                       check=True, capture_output=True, env=env)
        trace = {}
        for line in open(d / "trace.ndjson"):
            r = json.loads(line)
            if r["kind"] == "selection":
                trace[(r["tick"], r["agent"])] = (r["shortfalls"], r["h_t"])
        rust = {(f["tick"], f["agent"]): f for f in _native.firm_ticks(load.read_config_json(d / "run"), str(d / "run"))}
        only_trace += len(set(trace) - set(rust)); only_rust += len(set(rust) - set(trace))
        for k in set(trace) & set(rust):
            n_cmp += 1
            t_sc, t_h = trace[k]
            diff = max(abs(a - b) for a, b in zip(t_sc, rust[k]["shortfalls"]))
            max_abs = max(max_abs, diff); n_unequal += diff != 0.0
            dm = abs(rust[k]["shortfall_max"] - max(t_sc))
            max_abs_max = max(max_abs_max, dm)
            rel = diff / max(1e-300, max(abs(x) for x in t_sc))
            max_rel = max(max_rel, rel)
            sign_flips += (max(t_sc) > 0.0) != (rust[k]["shortfall_max"] > 0.0)
            argmax_flips += t_sc.index(max(t_sc)) != rust[k]["shortfalls"].index(max(rust[k]["shortfalls"]))
            h_abs.append(abs(t_h - rust[k]["h"]))
        n_runs += 1
import statistics
print(f"runs compared: {n_runs} (50 shock seeds x shocked + no-shock fork, satisficing arm)")
print(f"firm-ticks compared (present in both): {n_cmp}")
print(f"decisions in the plugin trace with no live firm-tick record (the tick a firm dies in): {only_trace}")
print(f"live firm-tick records with no plugin trace record: {only_rust}")
print(f"shortfalls [s1, s2, s3]: firm-ticks where any component differs: {n_unequal}; max |difference|: {max_abs!r}")
print(f"largest relative difference in any component: {max_rel!r}")
print(f"shortfall_max vs max(plugin shortfalls): max |difference| {max_abs_max!r}")
print(f"firm-ticks where the sign test attend() applies (max shortfall > 0) differs: {sign_flips}")
print(f"firm-ticks where the arg-max goal differs: {argmax_flips}")
print(f"h, for information: plugin h_t (perceived, decision time) vs analysis h (true, end of tick): "
      f"identical on {sum(x == 0 for x in h_abs)} of {len(h_abs)} firm-ticks; "
      f"mean |difference| {statistics.fmean(h_abs):.6f}; max {max(h_abs):.6f}")
