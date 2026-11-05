"""R21: the price path around a resource / competitive shock (init seed 910001,
medium, relative reading). Reconstructs input_price and output_price per tick
from the logged AdjustGlobalInt deltas, shocked against no-shock.
usage: verify_price_path.py <repo> <work-dir>"""
import importlib.util, json, sys
from pathlib import Path
repo, work = Path(sys.argv[1]), Path(sys.argv[2])
sys.path.insert(0, str(repo / "python"))
spec = importlib.util.spec_from_file_location("dc", repo / "scripts/diag_cliff.py")
dc = importlib.util.module_from_spec(spec); spec.loader.exec_module(dc)
from firma_lab import load
base = json.loads(dc.rs.BASE_CONFIG.read_text()); firma = dc.sl.find_firma(None)
def path(label, **kw):
    cfg = dc.build_config(base, 910001, "medium", reading="relative", **kw); d = work / label
    err = dc.sl.run_config(cfg, d, firma); assert not err, err
    e = load.read_events(d / "run"); fld = "kind.AdjustGlobalInt.field"
    g = e[(e["event"] == "delta_applied") & e[fld].notna()]
    recs = [(int(t), o, f, int(dl)) for t, o, f, dl in zip(g["tick"], g["origin"], g[fld], g["kind.AdjustGlobalInt.delta"])]
    price = {"input_price": cfg["world"]["global_ints"]["input_price"], "output_price": cfg["world"]["global_ints"]["output_price"]}
    series = {k: [] for k in price}
    for t in range(cfg["world"]["ticks"]):
        for tt, _, f, dl in recs:
            if tt == t and f in price: price[f] += dl
        for k in price: series[k].append(price[k])
    return recs, series, dc.sl.drawn_shock(e)[0], dc.sl.decisions_and_deaths(e)[0]
_, none, _, acts0 = path("none", shocked=False)
for ch, mag in (("resource", 4), ("competitive", 2)):
    recs, s, onset, acts = path(ch, shocked=True, channel=ch, magnitude=mag)
    shock = [r for r in recs if r[1].startswith("shock.")]
    key = "input_price" if ch == "resource" else "output_price"
    print(f"== {ch} magnitude {mag}: onset {onset} ==")
    print(f"  deltas with a shock.* origin: {shock}")
    print(f"  origins writing global ints: {sorted({r[1] for r in recs})}")
    lo, hi = onset - 3, onset + 12
    print(f"  {key}, ticks {lo}..{hi-1}: shocked  {s[key][lo:hi]}")
    print(f"  {key}, ticks {lo}..{hi-1}: no-shock {none[key][lo:hi]}")
    print(f"  {key} at the last tick: shocked {s[key][-1]}, no-shock {none[key][-1]}; shocked minus no-shock from onset on: "
          f"{sorted(set(a - b for a, b in zip(s[key][onset:], none[key][onset:])))}")
    print(f"  min/max of {key} over the run: shocked {min(s[key])}/{max(s[key])}; no-shock {min(none[key])}/{max(none[key])}")
    diff = {a: [(x, y) for x, y in zip(acts[a], acts0[a]) if x != y][:3] for a in acts if acts[a] != acts0[a]}
    print(f"  firms whose selected actions differ from the no-shock run: {len(diff)} of {len(acts)}; first differences (tick, action) shocked vs no-shock: {diff}")
