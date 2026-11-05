"""R21 part A, descriptive, from a_runs.csv and summary.json (no runs).
usage: describe_a.py <repo>"""
import collections, csv, json, sys
from pathlib import Path
ev = Path(sys.argv[1]) / "docs/review/evidence/R21"
rows = list(csv.DictReader(open(ev / "a_runs.csv"))); S = json.loads((ev / "summary.json").read_text())["A"]
base = {int(k): v for k, v in S["baseline_survivors"].items()}
by_shock = collections.defaultdict(list)
for r in rows: by_shock[int(r["shock_seed"])].append(r)
hit = sorted(s for s, rs in by_shock.items() if any(int(r["compliance_deaths"]) > 0 for r in rs))
print(f"shock seeds under which at least one population has a compliance death: {len(hit)}: {hit}")
allhit = [s for s in hit if all(int(r["compliance_deaths"]) > 0 for r in by_shock[s])]
print(f"  ... under which ALL 30 populations have a compliance death: {len(allhit)}")
nohit = sorted(set(by_shock) - set(hit))
print(f"shock seeds under which NO population has a compliance death: {len(nohit)}")
print(f"  in those {len(nohit) * 30} runs, survivors == that population's baseline survivors in "
      f"{sum(int(r['survivors']) == base[int(r['population_seed'])] for s in nohit for r in by_shock[s])}")
c_hit = collections.Counter(int(r["survivors"]) for s in hit for r in by_shock[s])
print(f"survivors in the {len(hit) * 30} runs under the {len(hit)} shock seeds: {dict(sorted(c_hit.items()))}")
c_md = collections.Counter(int(r["max_deaths_in_one_tick"]) for s in hit for r in by_shock[s])
print(f"largest same-tick death count in those runs: {dict(sorted(c_md.items()))}")
per_pop = collections.defaultdict(list)
for s in hit:
    for r in by_shock[s]: per_pop[int(r["population_seed"])].append(int(r["survivors"]))
print("per population: survivors under each of those shock seeds (seed order), baseline survivors")
for p, v in sorted(per_pop.items()):
    print(f"  {p}: {v}  distinct {sorted(set(v))}  baseline {base[p]}")
mags = {s: float(by_shock[s][0]["drawn_magnitude"]) for s in by_shock}; ons = {s: int(by_shock[s][0]["drawn_onset"]) for s in by_shock}
print(f"drawn magnitude: the {len(hit)} shock seeds min {min(mags[s] for s in hit):.4f} max {max(mags[s] for s in hit):.4f}; "
      f"the other {len(nohit)}: {sorted(round(mags[s], 4) for s in nohit)}")
print(f"  other seeds with magnitude inside [{min(mags[s] for s in hit):.4f}, {max(mags[s] for s in hit):.4f}]: "
      f"{[s for s in nohit if min(mags[x] for x in hit) <= mags[s] <= max(mags[x] for x in hit)]}")
print(f"drawn onset: the {len(hit)} shock seeds {sorted(ons[s] for s in hit)}; the other {len(nohit)}: {sorted(ons[s] for s in nohit)}")
# the same decomposition on 'any compliance death' instead of 'survivors <= 2'
m = [[0] * 30 for _ in range(30)]
pops = sorted({int(r["population_seed"]) for r in rows}); shocks = sorted(by_shock)
for r in rows: m[pops.index(int(r["population_seed"]))][shocks.index(int(r["shock_seed"]))] = int(int(r["compliance_deaths"]) > 0)
print(f"NOT the predeclared metric - indicator 'any compliance death': distinct rows in the 30x30 matrix {len({tuple(x) for x in m})}; "
      f"row sums {sorted(set(map(sum, m)))}; column sums {sorted(set(sum(c) for c in zip(*m)))}")
