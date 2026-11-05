"""R18, descriptive only: who dies, when and of what in the T1-passing cells,
and the distribution of survivors over their Stage-2 runs. Re-runs a few
configs (seeds >= 910001) with the search script's own builders.
usage: describe_passing_cells.py <repo> <work-dir>"""
import collections, importlib.util, json, sys
from pathlib import Path
repo, work = Path(sys.argv[1]), Path(sys.argv[2])
sys.path.insert(0, str(repo / "python"))
spec = importlib.util.spec_from_file_location("rs", repo / "scripts/regime_search.py")
rs = importlib.util.module_from_spec(spec); spec.loader.exec_module(rs)
from firma_lab import load
ev = repo / "docs/review/evidence/R18"
s1 = json.loads((ev / "stage1_full.json").read_text()); s2 = json.loads((ev / "stage2_full.json").read_text())
base = json.loads(rs.BASE_CONFIG.read_text()); firma = rs.sl.find_firma(None)

def deaths(cfg, label):
    d = work / label
    err = rs.sl.run_config(cfg, d, firma); assert not err, err
    _, dd = rs.sl.decisions_and_deaths(load.read_events(d / "run"))
    return sorted((t, a, c) for a, (t, c) in dd.items())

print("== Stage-2 survivors (of 12) per run, all 18 rows ==")
tab = collections.Counter(); cross = collections.Counter()
for r in s2:
    for x in r["shocked_runs"]:
        tab[(r["shock_magnitude_mean"], x["survivors"])] += 1
        cross[(x["survivors"], x["delta_h_rep_matched"] != 0.0)] += 1
for k in sorted(tab): print(f"  magnitude {k[0]:.2f}: {tab[k]:>3} runs end with {k[1]} survivors")
print("  runs by (survivors, matched DeltaH non-zero):", dict(sorted(cross.items())))
print("  per-run (survivors, DeltaH) lists identical across the six cells, per magnitude:",
      {m: len({json.dumps([(x['survivors'], x['delta_h_rep_matched']) for x in r['shocked_runs']])
               for r in s2 if r['shock_magnitude_mean'] == m}) == 1 for m in rs.GRID2_SHOCK_MEANS})

print("\n== death records (tick, firm id, cause) ==")
cells = {c["cell"]: c for c in rs.cells2()}
for r in s1:
    if not r["T1"]: continue
    c = cells[r["cell_params"]["cell"]]
    pop = rs.population2(c["n_type_L"], c["n_type_T"], True, base["agents"][0])
    print(f"cell {c['cell']}: types by id {''.join(f['_type'] for f in pop)}; seeded ids "
          f"{[f['id'] for f in pop if 'ints' in f]}")
    print("   baseline:", deaths(rs.build_cell2_config(base, c, shock_seed=910001, shocked=False), f"b{c['cell']}"))
c = cells[79]
for m in rs.GRID2_SHOCK_MEANS:
    for seed in (910001, 910002, 910015):
        d = deaths(rs.build_cell2_config(base, c, shock_seed=seed, shocked=True, shock_mean=m), f"s{m}_{seed}")
        row = next(x for r in s2 if r["cell_params"]["cell"] == 79 and r["shock_magnitude_mean"] == m
                   for x in r["shocked_runs"] if x["shock_seed"] == seed)
        print(f"cell 79 magnitude {m:.2f} shock seed {seed}: drawn onset {row['drawn_onset']}, drawn "
              f"magnitude {row['drawn_magnitude']:.4f}, survivors {row['survivors']}; deaths {d}")
