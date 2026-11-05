import json, sys, collections
def metrics(trace):
    recs=[json.loads(l) for l in open(trace)]
    sel=[r for r in recs if r["kind"]=="selection"]
    reach=set(); inadm=0
    for r in recs:
        if r.get("action")==6 and r["kind"]=="admissible_check":
            reach.add((r["tick"],r["agent"])); inadm+= (not r["result"])
    sat6=collections.Counter(str(r["focus"]) for r in recs if r["kind"]=="satisfices_check" and r.get("action")==6)
    gates=[r for r in recs if r["kind"]=="shaping_gate"]
    gclosed=sum(1 for g in gates if not g["gate_open"]); ttb=collections.Counter(str(g["time_to_boundary"]) for g in gates)
    payoff=[r for r in recs if r["kind"]=="shaping_payoff_comparison"]
    focus=collections.Counter(r["focus"] for r in sel)
    chosen=collections.Counter(r["action"] for r in sel)
    return dict(decisions=len(sel), lobby_reached=len(reach), lobby_inadmissible=inadm,
        lobby_satisfices_evals_by_focus=dict(sat6), gate_evals=len(gates), gate_closed=gclosed,
        ttb_values=dict(ttb), payoff_comparisons=len(payoff), payoff_wins=sum(p["result"] for p in payoff),
        shaping_selected=sum(v for a,v in chosen.items() if a>=6), focus_counts=dict(focus), action_counts=dict(sorted(chosen.items())))
for t in sys.argv[1:]:
    print("==", t); [print(f"  {k}: {v}") for k,v in metrics(t).items()]
