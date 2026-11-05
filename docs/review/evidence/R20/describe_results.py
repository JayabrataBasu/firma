"""R20, descriptive only, from the saved per-run tables (no runs).
usage: describe_results.py <repo>"""
import collections, csv, importlib.util, json, sys
from pathlib import Path
repo = Path(sys.argv[1]); ev = repo / "docs/review/evidence/R20"
spec = importlib.util.spec_from_file_location("pg", repo / "scripts/population_generator.py")
pg = importlib.util.module_from_spec(spec); spec.loader.exec_module(pg)
q2 = list(csv.DictReader(open(ev / "q2_runs.csv"))); q5 = list(csv.DictReader(open(ev / "q5_runs.csv")))

print("== Q2: shocked runs split by whether any firm died of compliance ==")
for lvl in pg.SPREAD_LEVELS:
    rows = [r for r in q2 if r["level"] == lvl]
    hit = [r for r in rows if int(r["shocked_compliance_deaths"]) > 0]
    miss = [r for r in rows if int(r["shocked_compliance_deaths"]) == 0]
    c = lambda rs_, k: dict(sorted(collections.Counter(int(r[k]) for r in rs_).items()))
    print(f" {lvl}: {len(miss)} runs with no compliance death: survivors {c(miss, 'shocked_survivors')}")
    print(f"   {len(hit)} runs with a compliance death: survivors {c(hit, 'shocked_survivors')}; "
          f"largest same-tick death count {c(hit, 'shocked_max_deaths_in_one_tick')}; "
          f"compliance deaths per run {c(hit, 'shocked_compliance_deaths')}")
    print(f"   init seeds of the runs with a compliance death identical to the low level's: "
          f"{sorted(r['init_seed'] for r in hit) == sorted(r['init_seed'] for r in q2 if r['level'] == 'low' and int(r['shocked_compliance_deaths']) > 0)}")
    on = sorted(float(r["drawn_magnitude"]) for r in hit); off = sorted(float(r["drawn_magnitude"]) for r in miss)
    print(f"   drawn shock magnitude: with a compliance death min {on[0]:.3f} max {on[-1]:.3f}; without min {off[0]:.3f} max {off[-1]:.3f}")

print("\n== Q2/Q4: baseline deaths against the obligation draw ==")
for lvl in pg.SPREAD_LEVELS:
    exact = subset = 0; causes = collections.Counter(); n = 0
    for r in q2:
        if r["level"] != lvl: continue
        n += 1
        seeded = {f for f in range(12) if pg.draw_values(int(r["init_seed"]), f, lvl)["obligation"] is not None}
        dead = {a for _, a, _ in json.loads(r["baseline_death_records"])}
        causes.update(c for _, _, c in json.loads(r["baseline_death_records"]))
        exact += dead == seeded; subset += dead <= seeded
    print(f" {lvl}: baseline dead firms == obligation-seeded firms in {exact} of {n} baselines; "
          f"dead is a subset of seeded in {subset}; baseline death causes {dict(causes)}")
ns = collections.Counter(sum(pg.draw_values(s, f, 'low')['obligation'] is not None for f in range(12))
                         for s in list(range(910001, 910051)) + list(range(911001, 911051)))
print(f" obligation-seeded firms per population (100 init seeds; the draw does not depend on the level): {dict(sorted(ns.items()))}")

print("\n== Q5: what distinguishes the action sequences ==")
for arm in ("drawn", "fixed"):
    for h in ("0.02", "0.4"):
        for s in ("0.0", "1.0"):
            rows = [r for r in q5 if r["arm"] == arm and r["pinned_margin"] == h and r["pinned_shortfall"] == s]
            groups = collections.defaultdict(list)
            for r in rows: groups[r["action_sha"]].append(r)
            desc = []
            for sha, g in sorted(groups.items(), key=lambda kv: -len(kv[1])):
                dr = collections.Counter(r["death_records"] for r in g)
                obl = collections.Counter("seeded" if r["obligation"] else "none" for r in g)
                desc.append(f"{len(g)} runs [{sha[:8]}] decisions {sorted({int(r['decisions']) for r in g})} "
                            f"H_rep {sorted({round(float(r['h_rep_mean']), 4) for r in g})} obligation {dict(obl)} deaths {dict(dr)}")
            alive = {r["action_sha"] for r in rows if int(r["deaths"]) == 0}
            print(f" {arm} h {h} shortfall {s}: distinct sequences among runs where the firm survived: {len(alive)}")
            for d in desc: print("    " + d[:330])
drawn = [r for r in q5 if r["arm"] == "drawn" and r["pinned_margin"] == "0.02" and r["pinned_shortfall"] == "0.0"]
caps = sorted(int(r["capital"]) for r in drawn); ks = sorted(float(r["capability"]) for r in drawn)
print(f" drawn single firms (50 init seeds): capital min {caps[0]} median {caps[25]} max {caps[-1]}; "
      f"capability min {ks[0]:.3f} max {ks[-1]:.3f}; distinct capital values {len(set(caps))}; "
      f"with an obligation: {sum(bool(r['obligation']) for r in drawn)}")
