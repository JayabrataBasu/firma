"""R21: were the treatments in parts B and D actually applied? Descriptive.
(1) from the saved tables: do event logs differ between aspiration readings?
(2) four re-runs of declared configs (init seed 910001, medium): what the
engine logged for aspiration, prices and actions.
usage: verify_treatments_applied.py <repo> <work-dir>"""
import collections, csv, hashlib, importlib.util, json, sys
from pathlib import Path
repo, work = Path(sys.argv[1]), Path(sys.argv[2])
sys.path.insert(0, str(repo / "python"))
spec = importlib.util.spec_from_file_location("dc", repo / "scripts/diag_cliff.py")
dc = importlib.util.module_from_spec(spec); spec.loader.exec_module(dc)
from firma_lab import load
ev = repo / "docs/review/evidence"
b = list(csv.DictReader(open(ev / "R21/b_runs.csv"))); q2 = list(csv.DictReader(open(ev / "R20/q2_runs.csv")))
old = {(r["level"], r["init_seed"]): r for r in q2}
print("== (1) relative (R21 B) against absolute (R20) on the same 300 (level, init seed) pairs ==")
for lvl in dc.pg.SPREAD_LEVELS:
    rows = [r for r in b if r["level"] == lvl]
    cmp = lambda k: sum(r[k] == old[(lvl, r["init_seed"])][k] for r in rows)
    print(f" {lvl}: of {len(rows)} pairs, identical baseline event log {cmp('baseline_events_sha256')}, identical shocked "
          f"event log {cmp('shocked_events_sha256')}, identical shocked survivors {cmp('shocked_survivors')}, identical "
          f"death records {cmp('shocked_death_records')}, identical SC-2 value {cmp('sc2_h_below_crit')}, identical "
          f"matched DeltaH {cmp('delta_h_rep_matched')}, identical T1 {cmp('T1')}")

base = json.loads(dc.rs.BASE_CONFIG.read_text()); firma = dc.sl.find_firma(None)
def run(label, **kw):
    cfg = dc.build_config(base, 910001, "medium", **kw); d = work / label
    err = dc.sl.run_config(cfg, d, firma); assert not err, err
    e = load.read_events(d / "run"); by_agent, deaths = dc.sl.decisions_and_deaths(e)
    return cfg, e, by_agent, deaths
print("\n== (2a) aspiration: configured values and what the decision rule did with them ==")
res = {rd: run(f"asp_{rd}", reading=rd, shocked=False) for rd in dc.READINGS}
for rd, (cfg, e, by_agent, deaths) in res.items():
    asp = [round(a["reals"]["aspiration_capital_growth"], 3) for a in cfg["agents"]]
    fld = "kind.AdjustAgentReal.field"
    upd = e[(e["event"] == "delta_applied") & (e[fld] == "aspiration_capital_growth")] if fld in e.columns else []
    foc = e[(e["event"] == "delta_applied") & (e["kind.SetAgentInt.field"] == "focus")]
    print(f" {rd}: configured aspirations {asp}")
    print(f"   aspiration_capital_growth updates logged: {len(upd)}; focus values written (value: count): "
          f"{dict(sorted(collections.Counter(int(v) for v in foc['kind.SetAgentInt.value']).items()))}; action sha {dc.sl.action_sha(by_agent)[:16]}")
print(f" selected-action sequences identical between readings: {res['absolute'][2] == res['relative'][2]}; "
      f"deaths identical: {res['absolute'][3] == res['relative'][3]}")

print("\n== (2b) price shocks: what the engine logged ==")
for ch, mag in (("resource", 4), ("competitive", 2)):
    cfg, e, by_agent, deaths = run(f"ch_{ch}", reading="relative", shocked=True, channel=ch, magnitude=mag)
    fld = "kind.AdjustGlobalInt.field"
    g = e[(e["event"] == "delta_applied") & e[fld].notna()] if fld in e.columns else e.iloc[0:0]
    recs = [(int(t), o, f, int(dl)) for t, o, f, dl in zip(g["tick"], g["origin"], g[fld], g["kind.AdjustGlobalInt.delta"])]
    onset, m = dc.sl.drawn_shock(e)
    same_before = {a: [x for x in s if x[0] < onset] == [x for x in res["relative"][2][a] if x[0] < onset] for a, s in by_agent.items()}
    same_all = {a: s == res["relative"][2][a] for a, s in by_agent.items()}
    acts_after = collections.Counter(x[1] for s in by_agent.values() for x in s if x[0] >= onset)
    base_after = collections.Counter(x[1] for s in res["relative"][2].values() for x in s if x[0] >= onset)
    print(f" {ch} magnitude {mag}: drawn onset {onset}, magnitude {m}; global int deltas logged: {recs}")
    print(f"   firms whose action sequence equals the no-shock run's: before onset {sum(same_before.values())} of {len(same_before)}; whole run {sum(same_all.values())} of {len(same_all)}")
    print(f"   selected actions from onset on (action id: count): shocked {dict(sorted(acts_after.items()))}; no-shock {dict(sorted(base_after.items()))}")
    print(f"   deaths identical to the no-shock run: {deaths == res['relative'][3]}")
