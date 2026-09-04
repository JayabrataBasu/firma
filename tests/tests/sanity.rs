//! SC-1…SC-6 sanity conditions (manual §16.2) — the second half of the
//! Phase-2 gate (§26.4, §30.9). Computed offline from an event log
//! (`firma_conformance::replay`), on `T = 400` multi-firm runs.
//!
//! **The gate outcome for this Stage is: SC-1…SC-6 CANNOT be jointly
//! satisfied.** Three of the six fail structurally, not for want of tuning
//! (~24 configs across `θ_limit ∈ [0.40, 0.90]`, `θ_cap ∈ [0.20, 0.90]`,
//! `θ_Q ∈ [6, 100]`, `L_W ∈ {4, 6, 8}`, `n ∈ {2, 4, 10, 12, 20}`, with/without
//! shocks; and — the Stage-6 follow-up, ADR-0042 — the full §16.1
//! `β ∈ {0, 0.5, 1, 2, 4} × w_max ∈ {3, 6, 9}` sweep for SC-4/SC-5). Per §16.2
//! this is a legitimate Phase-2 failure condition and a substantive finding —
//! see ADR-0040 / ADR-0042 and the Stage-6 report. `sc16_gate` locks the
//! finding as a regression test.

use firma_cli::{execute_run, model_registry, RunOptions};
use firma_conformance::replay::{sanity_from_run, SanityReport};
use firma_conformance::{config, scratch};

fn run_report(name: &str, json: &str) -> SanityReport {
    let cfg = config(json);
    let dir = scratch(name);
    let rep = execute_run(&cfg, &model_registry(), &dir, &RunOptions::default()).unwrap();
    assert!(rep.conservation_ok, "{name}: conservation failed");
    let sc = sanity_from_run(&cfg, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    sc
}

fn print_report(name: &str, sc: &SanityReport) {
    eprintln!(
        "\n=== {name} ===  ({} firms → {} survivors)",
        sc.initial_firms, sc.survivors
    );
    eprintln!(
        "  SC-1 survival           = {:.3}   [0.60, 0.90]   {}",
        sc.sc1_survival,
        mark(sc.sc1_pass())
    );
    eprintln!(
        "  SC-2 survival-attention = {:.3}   [0.05, 0.25]   {}   (reconstructed h<h_crit = {:.3})",
        sc.sc2_survival_attention,
        mark(sc.sc2_pass()),
        sc.sc2_reconstructed_h_below_crit
    );
    eprintln!(
        "  SC-3 binds [sol,cmp,scp,obl] = {:?}   first@ {:?}   {}",
        sc.sc3_binds,
        sc.sc3_first_bind_tick,
        mark(sc.sc3_pass())
    );
    eprintln!(
        "  SC-4 shaping fraction    = {:.4}   (> 0.05)      {}   ({}/{} decisions)",
        sc.sc4_shaping_fraction,
        mark(sc.sc4_pass()),
        (sc.sc4_shaping_fraction * sc.decisions as f64).round() as i64,
        sc.decisions
    );
    eprintln!(
        "  SC-5 shaping success    = {:?}   [0.10, 0.60]   {}   ({} commits)",
        sc.sc5_shaping_success
            .map(|r| (r * 1000.0).round() / 1000.0),
        mark(sc.sc5_pass()),
        sc.shaping_commits
    );
    eprintln!(
        "  SC-6 entropy variance   = {:.4}   (≥ 0.01)      {}   (mean H_rep = {:.3} bits)",
        sc.sc6_entropy_variance,
        mark(sc.sc6_pass()),
        sc.sc6_entropy_mean
    );
    eprintln!("  ALL SIX: {}", mark(sc.all_pass()));
}

fn mark(b: bool) -> &'static str {
    if b {
        "PASS"
    } else {
        "fail"
    }
}

// --------------------------------------------------------------------------
// representative configs
// --------------------------------------------------------------------------

const MARKET_ACTIONS: &str = r#"
        { "id": "action.market.standard.hold", "version": "^1", "params": {} },
        { "id": "action.market.standard.produce_ordinary", "version": "^1", "params": {} },
        { "id": "action.market.standard.produce_regulated", "version": "^1", "params": {} },
        { "id": "action.market.standard.acquire_input", "version": "^1", "params": {} },
        { "id": "action.market.standard.invest_capability", "version": "^1", "params": {} },
        { "id": "action.market.standard.deliver", "version": "^1", "params": {} },
        { "id": "action.shaping.rdt_standard.resolve_lagged", "version": "^1", "params": {} }"#;

fn constraint_rules(l_w: u32) -> String {
    format!(
        r#"
        {{ "id": "constraint.action_window", "version": "^1", "params": {{ "l_w": {l_w} }} }},
        {{ "id": "constraint.enforce", "version": "^1",
           "params": {{ "t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": {l_w} }} }},
        {{ "id": "decision.aspiration_update", "version": "^1", "params": {{ "alpha": 0.10 }} }}"#
    )
}

/// The "all-healthy" config — every firm comfortably clear of all four
/// constraints. Demonstrates SC-2's floor (≈ 0) and that SC-3 then fails.
fn cfg_healthy() -> String {
    let firms: Vec<String> = (0..4)
        .map(|i| format!(
            r#"{{ "id": {i}, "stocks": {{ "capital": 500, "input": 500000 }},
               "reals": {{ "capability": {c}, "legitimacy": 1.0, "aspiration_capital_growth": 6.0 }} }}"#,
            c = 0.50 + i as f64 * 0.1
        ))
        .collect();
    format!(
        r#"{{ "experiment": "sc-healthy", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": {{ "mechanism": 55, "environment": 2, "shock": 3, "init": 4 }},
      "world": {{ "ticks": 400, "resources": ["capital", "input"],
        "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }}, "snapshot_every": 400,
        "global_reals": {{ "theta_limit": 0.90, "theta_cap": 0.20 }},
        "global_ints": {{ "theta_q": 100, "input_price": 2, "output_price": 3 }} }},
      "agents": [{}],
      "environment": {{ "stocks": {{ "capital": 100000000, "input": 100000000 }} }},
      "rules": [
        {{ "id": "decision.satisficing", "version": "^1", "params": {{ "l_w": 8 }} }},{MARKET_ACTIONS},{}
      ] }}"#,
        firms.join(","),
        constraint_rules(8)
    )
}

/// The "threaded needle" — mostly healthy, a few stressed, `θ_limit = 0.40 /
/// L_W = 4` so a subset dies of `compliance`. This is the closest any config
/// gets: SC-1, SC-3 (solvency via a knife-edge seed + compliance + scope +
/// obligation), and SC-6 pass; SC-2 and SC-4/SC-5 fail.
fn cfg_threaded() -> String {
    format!(
        r#"{{ "experiment": "sc-thread", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": {{ "mechanism": 7, "environment": 2, "shock": 3, "init": 4 }},
      "world": {{ "ticks": 400, "resources": ["capital", "input"],
        "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }}, "snapshot_every": 400,
        "global_reals": {{ "theta_limit": 0.40, "theta_cap": 0.35 }},
        "global_ints": {{ "theta_q": 6, "input_price": 2, "output_price": 3 }} }},
      "agents": [
        {{ "id": 0, "stocks": {{ "capital": 400, "input": 500000 }}, "reals": {{ "capability": 0.55, "legitimacy": 1.0, "aspiration_capital_growth": 8.0 }} }},
        {{ "id": 1, "stocks": {{ "capital": 400, "input": 500000 }}, "reals": {{ "capability": 0.55, "legitimacy": 1.0, "aspiration_capital_growth": 8.0 }} }},
        {{ "id": 2, "stocks": {{ "capital": 400, "input": 500000 }}, "reals": {{ "capability": 0.60, "legitimacy": 1.0, "aspiration_capital_growth": 5.0 }} }},
        {{ "id": 3, "stocks": {{ "capital": 400, "input": 500000 }}, "reals": {{ "capability": 0.70, "legitimacy": 1.0, "aspiration_capital_growth": 5.0 }} }},
        {{ "id": 4, "stocks": {{ "capital": 400, "input": 500000 }}, "reals": {{ "capability": 0.80, "legitimacy": 1.0, "aspiration_capital_growth": 5.0 }} }},
        {{ "id": 5, "stocks": {{ "capital": 400, "input": 500000 }}, "reals": {{ "capability": 0.90, "legitimacy": 1.0, "aspiration_capital_growth": 5.0 }} }},
        {{ "id": 6, "stocks": {{ "capital": 2, "input": 0 }}, "reals": {{ "capability": 0.10, "legitimacy": 1.0 }} }},
        {{ "id": 7, "stocks": {{ "capital": 400, "input": 500000 }}, "reals": {{ "capability": 0.25, "legitimacy": 1.0, "aspiration_capital_growth": 3.0 }} }},
        {{ "id": 8, "stocks": {{ "capital": 400, "input": 500000 }}, "reals": {{ "capability": 0.50, "legitimacy": 1.0 }}, "ints": {{ "obligation": 9 }} }},
        {{ "id": 9, "stocks": {{ "capital": 2, "input": 0 }}, "reals": {{ "capability": 0.10, "legitimacy": 1.0 }} }}
      ],
      "environment": {{ "stocks": {{ "capital": 100000000, "input": 100000000 }} }},
      "rules": [
        {{ "id": "decision.satisficing", "version": "^1", "params": {{ "l_w": 4 }} }},{MARKET_ACTIONS},{}
      ] }}"#,
        constraint_rules(4)
    )
}

/// `decision.random` (Arm C) with shaping in the repertoire — shows SC-4's
/// failure is specific to `decision.satisficing`'s scan order + one-step
/// lookahead, not the action set.
fn cfg_random_shaping() -> String {
    format!(
        r#"{{ "experiment": "sc-rand", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": {{ "mechanism": 5, "environment": 2, "shock": 3, "init": 4 }},
      "world": {{ "ticks": 100, "resources": ["capital", "input"],
        "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }}, "snapshot_every": 100,
        "global_reals": {{ "theta_limit": 0.90, "theta_cap": 0.20 }},
        "global_ints": {{ "theta_q": 100, "input_price": 2, "output_price": 3 }} }},
      "agents": [
        {{ "id": 0, "stocks": {{ "capital": 500000, "input": 500000 }}, "reals": {{ "capability": 0.5, "legitimacy": 1.0 }} }},
        {{ "id": 1, "stocks": {{ "capital": 500000, "input": 500000 }}, "reals": {{ "capability": 0.5, "legitimacy": 1.0 }} }}
      ],
      "environment": {{ "stocks": {{ "capital": 100000000, "input": 100000000 }} }},
      "rules": [
        {{ "id": "decision.random", "version": "^1", "params": {{ "shaping": {{ "lobby_cost": 25, "diversify_cost": 25 }} }} }},{MARKET_ACTIONS},
        {{ "id": "action.shaping.rdt_standard.lobby", "version": "^1", "params": {{}} }},
        {{ "id": "action.shaping.rdt_standard.diversify", "version": "^1",
          "params": {{ "success": {{ "p0": 0.30, "b_lambda": 0.20, "b_kappa": 0.10, "p_max": 0.60, "kappa_min": 25 }},
            "lag": {{ "min": 2, "max": 6 }} }} }},{}
      ] }}"#,
        constraint_rules(8)
    )
}

// --------------------------------------------------------------------------
// SC-4/SC-5 follow-up — the w_max / β channel probe
// --------------------------------------------------------------------------

/// A firm engineered for the follow-up hypothesis: **`GOAL(1)` focus that
/// persists** (aspiration far beyond what any market action delivers, so no
/// market action satisfices and `focus` never drops to `NONE` or `SURVIVAL`),
/// **comfortable `h`** (`θ_limit = 0.90`, `θ_cap = 0.15` ⇒ well above
/// `h_crit`, so `ψ(h) = 1` and `w_eff = w_max` for every `β`), a **wide
/// `w_max`**, and the **full shaping repertoire**. If a wide search width ever
/// lets the scan reach `lobby` (position 4 in the `GOAL(1)` order) and select
/// it, it happens here.
fn cfg_wide_search(w_max: u32, beta: f64) -> String {
    format!(
        r#"{{ "experiment": "sc-wide", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": {{ "mechanism": 909, "environment": 2, "shock": 3, "init": 4 }},
      "world": {{ "ticks": 200, "resources": ["capital", "input"],
        "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }}, "snapshot_every": 200,
        "global_reals": {{ "theta_limit": 0.90, "theta_cap": 0.15 }},
        "global_ints": {{ "theta_q": 100, "input_price": 2, "output_price": 1 }} }},
      "agents": [
        {{ "id": 0, "stocks": {{ "capital": 100000, "input": 500000 }},
           "reals": {{ "capability": 0.40, "legitimacy": 1.0, "aspiration_capital_growth": 100000.0 }} }},
        {{ "id": 1, "stocks": {{ "capital": 100000, "input": 500000 }},
           "reals": {{ "capability": 0.50, "legitimacy": 1.0, "aspiration_capital_growth": 100000.0 }} }}
      ],
      "environment": {{ "stocks": {{ "capital": 100000000, "input": 100000000 }} }},
      "rules": [
        {{ "id": "decision.satisficing", "version": "^1",
           "params": {{ "l_w": 8, "w_max": {w_max}, "beta": {beta},
             "shaping": {{ "lobby_cost": 25, "contract_cost": 25, "diversify_cost": 25 }} }} }},{MARKET_ACTIONS},
        {{ "id": "action.shaping.rdt_standard.lobby", "version": "^1", "params": {{}} }},
        {{ "id": "action.shaping.rdt_standard.contract", "version": "^1",
           "params": {{ "delta_q": 4, "q0": 3,
             "success": {{ "p0": 0.30, "b_lambda": 0.20, "b_kappa": 0.10, "p_max": 0.60, "kappa_min": 25 }},
             "lag": {{ "min": 2, "max": 6 }} }} }},
        {{ "id": "action.shaping.rdt_standard.diversify", "version": "^1",
           "params": {{ "success": {{ "p0": 0.30, "b_lambda": 0.20, "b_kappa": 0.10, "p_max": 0.60, "kappa_min": 25 }},
             "lag": {{ "min": 2, "max": 6 }} }} }},{}
      ] }}"#,
        constraint_rules(8)
    )
}

/// The **only** channel by which `decision.satisficing` reaches a shaping
/// action: an **input-starved firm that cannot afford input** (`r^I = 0`,
/// `r^L < π^I`), so `produce_ordinary` / `produce_regulated` / `acquire_input`
/// — everything ahead of `lobby` in the `GOAL(1)` order — is inadmissible and
/// `lobby` becomes the fallback. Requires `π^I > κ_ℓ` (here `π^I = 30`,
/// `κ_ℓ = 25`) — a degenerate price. `w_max` = 9 so it is not a width effect.
fn cfg_input_starved() -> String {
    format!(
        r#"{{ "experiment": "sc-starved", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": {{ "mechanism": 11, "environment": 2, "shock": 3, "init": 4 }},
      "world": {{ "ticks": 200, "resources": ["capital", "input"],
        "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }}, "snapshot_every": 200,
        "global_reals": {{ "theta_limit": 0.90, "theta_cap": 0.15 }},
        "global_ints": {{ "theta_q": 100, "input_price": 30, "output_price": 3 }} }},
      "agents": [
        {{ "id": 0, "stocks": {{ "capital": 28, "input": 0 }},
           "reals": {{ "capability": 0.50, "legitimacy": 1.0, "aspiration_capital_growth": 8.0 }} }},
        {{ "id": 1, "stocks": {{ "capital": 27, "input": 0 }},
           "reals": {{ "capability": 0.50, "legitimacy": 1.0, "aspiration_capital_growth": 8.0 }} }},
        {{ "id": 2, "stocks": {{ "capital": 26, "input": 0 }},
           "reals": {{ "capability": 0.50, "legitimacy": 1.0, "aspiration_capital_growth": 8.0 }} }}
      ],
      "environment": {{ "stocks": {{ "capital": 100000000, "input": 100000000 }} }},
      "rules": [
        {{ "id": "decision.satisficing", "version": "^1",
           "params": {{ "l_w": 8, "w_max": 9, "shaping": {{ "lobby_cost": 25 }} }} }},{MARKET_ACTIONS},
        {{ "id": "action.shaping.rdt_standard.lobby", "version": "^1", "params": {{}} }},{}
      ] }}"#,
        constraint_rules(8)
    )
}

/// Follow-up: does a wide `w_max` / any `β` open the shaping channel for
/// `decision.satisficing`? **No** — `cfg_wide_search` produces 0 shaping over
/// the full §16.1 `β × w_max` sweep; only `cfg_input_starved` (a degenerate
/// `π^I > κ_ℓ`, can't-operate firm) reaches `lobby`, and only as a one-shot
/// (the firm cannot afford to lobby twice). This complements the structural
/// unit test `validation::sc4_shaping_selected_only_as_the_goal_fallback_...`.
#[test]
fn sc4_wmax_beta_probe() {
    eprintln!("\n--- (a) healthy GOAL(1) firm, full §16.1 β × w_max sweep ---");
    let mut any_shaping = false;
    for &wm in &[3u32, 6, 9] {
        for &b in &[0.0_f64, 0.5, 1.0, 2.0, 4.0] {
            let r = run_report("sc-wide", &cfg_wide_search(wm, b));
            let n_shaping = (r.sc4_shaping_fraction * r.decisions as f64).round() as i64;
            eprintln!(
                "  w_max={wm} β={b:.1} → shaping {n_shaping}/{} decisions ({:.4})",
                r.decisions, r.sc4_shaping_fraction
            );
            if r.sc4_shaping_fraction > 0.0 {
                any_shaping = true;
            }
        }
    }
    assert!(
        !any_shaping,
        "a healthy GOAL(1) firm selected a shaping action at some (w_max, β) — the SC-4 finding needs revising"
    );

    eprintln!("\n--- (b) the degenerate channel: input-starved firm, π^I = 30 > κ_ℓ = 25 ---");
    let s = run_report("sc-starved", &cfg_input_starved());
    let n_shaping = (s.sc4_shaping_fraction * s.decisions as f64).round() as i64;
    eprintln!(
        "  input-starved → shaping {n_shaping}/{} decisions ({:.4})   SC-4 pass = {}",
        s.decisions,
        s.sc4_shaping_fraction,
        s.sc4_pass()
    );
    // The channel exists but produces at most a one-shot lobby per firm — a
    // firm that cannot operate cannot afford to lobby repeatedly.
    assert!(
        s.sc4_shaping_fraction > 0.0,
        "the input-starved channel should reach lobby at least once"
    );
    assert!(
        !s.sc4_pass(),
        "even the degenerate channel should not reach 5% — a can't-operate firm lobbies at most once"
    );
}

// --------------------------------------------------------------------------
// the gate — locks the finding
// --------------------------------------------------------------------------

/// **SC-1…SC-6 gate.** Outcome: **not jointly satisfiable** (§16.2 failure
/// condition — a substantive finding). This test documents *which* conditions
/// hold in which regime and locks that as a regression.
#[test]
fn sc16_gate() {
    let healthy = run_report("sc-healthy", &cfg_healthy());
    print_report("all-healthy (SC-2 floor)", &healthy);

    let threaded = run_report("sc-thread", &cfg_threaded());
    print_report("threaded needle (closest)", &threaded);

    let rand = run_report("sc-rand", &cfg_random_shaping());
    print_report("decision.random + shaping (Arm C)", &rand);

    eprintln!("\n=== Stage-6 SC gate outcome: SC-1…SC-6 NOT jointly satisfiable ===");
    eprintln!(
        "  SC-1  achievable   (threaded: {:.2})",
        threaded.sc1_survival
    );
    eprintln!("  SC-2  NOT achievable — bimodal: {:.3} (healthy) vs {:.3} (threaded); never in [0.05, 0.25]",
        healthy.sc2_survival_attention, threaded.sc2_survival_attention);
    eprintln!("  SC-3  partial — compliance/scope/obligation emerge; solvency only from a knife-edge seed");
    eprintln!(
        "  SC-4  NOT achievable under decision.satisficing ({:.4}) — incl. full §16.1 β×w_max sweep\n\
         \x20       (ADR-0042; see sc4_wmax_beta_probe); {:.3} under decision.random",
        threaded.sc4_shaping_fraction, rand.sc4_shaping_fraction
    );
    eprintln!("  SC-5  NOT achievable — no shaping ⇒ no success rate");
    eprintln!(
        "  SC-6  achievable   (healthy: {:.3})",
        healthy.sc6_entropy_variance
    );

    // --- lock the finding ---
    // SC-2 is bimodal with no middle: near-0 when constraints don't bite,
    // high when they do.
    assert!(
        healthy.sc2_survival_attention < 0.05,
        "SC-2 floor moved above 0.05 ({:.3}) — re-examine the finding",
        healthy.sc2_survival_attention
    );
    assert!(
        threaded.sc2_survival_attention > 0.25,
        "SC-2 in the binding regime dropped into range ({:.3}) — the finding may be stale",
        threaded.sc2_survival_attention
    );
    // SC-4: decision.satisficing structurally never selects shaping...
    assert_eq!(
        threaded.sc4_shaping_fraction, 0.0,
        "decision.satisficing selected a shaping action — the SC-4 structural finding changed"
    );
    // ...while decision.random selects it readily.
    assert!(
        rand.sc4_shaping_fraction > 0.05,
        "decision.random shaping fraction {:.3} ≤ 5% — Arm C should exercise shaping",
        rand.sc4_shaping_fraction
    );
    // SC-1 and SC-6 ARE reachable.
    assert!(
        threaded.sc1_pass(),
        "SC-1 no longer reachable in the threaded regime"
    );
    assert!(
        healthy.sc6_pass(),
        "SC-6 no longer reachable in the healthy regime"
    );
    // The joint condition fails.
    assert!(
        !healthy.all_pass() && !threaded.all_pass() && !rand.all_pass(),
        "a config now passes all six — Phase-2 gate status has changed, re-review"
    );
}

/// The exploratory sweep behind the finding. Run with `--ignored --nocapture`.
///
/// (0) a `w_max × β` sweep on `cfg_wide_search` — SC-4 stays `0` at every cell
///     (the follow-up dimension; ADR-0042);
/// (1) a `θ_cap` sweep on the healthy config — SC-2 jumps from ~0 to ~1 as the
///     scope boundary crosses the firms' capabilities, with **no middle**;
/// (2) a `θ_limit × L_W` sweep on the threaded config — SC-1/SC-3 track the
///     compliance-death regime while SC-2/SC-4 never move.
#[test]
#[ignore = "exploratory: cargo test -p firma-conformance --test sanity sc16_search -- --ignored --nocapture"]
fn sc16_search() {
    eprintln!("\n--- (0) w_max × β sweep on the wide-search config (ADR-0042) ---");
    for &wm in &[3u32, 6, 9] {
        for &b in &[0.0_f64, 1.0, 4.0] {
            let r = run_report("sweep0", &cfg_wide_search(wm, b));
            eprintln!("  w_max={wm} β={b:.1} → SC-4={:.4}", r.sc4_shaping_fraction);
        }
    }

    eprintln!(
        "\n--- (1) θ_cap sweep on the healthy config (firms at c ∈ {{0.5, 0.6, 0.7, 0.8}}) ---"
    );
    for &tc in &[0.10_f64, 0.30, 0.45, 0.55, 0.65, 0.75] {
        let cfg = cfg_healthy().replace("\"theta_cap\": 0.20", &format!("\"theta_cap\": {tc}"));
        let r = run_report("sweep1", &cfg);
        eprintln!(
            "  θ_cap={tc:.2} → SC-2={:.3}   SC-3={:?}   (bimodal: no value lands in [0.05, 0.25])",
            r.sc2_survival_attention, r.sc3_binds
        );
    }

    eprintln!(
        "\n--- (2) θ_limit × L_W sweep, threaded config: once constraints bind, SC-2 is pinned at ~1.0 ---"
    );
    for &l_w in &[4u32, 6, 8] {
        for &tl in &[0.35_f64, 0.45, 0.60, 0.90] {
            let cfg = cfg_threaded()
                .replace("\"theta_limit\": 0.40", &format!("\"theta_limit\": {tl}"))
                .replace("\"l_w\": 4", &format!("\"l_w\": {l_w}"));
            let r = run_report("sweep2", &cfg);
            eprintln!(
                "  L_W={l_w} θ_limit={tl:.2} → SC-1={:.2} SC-2={:.2} SC-3={:?} SC-4={:.3} SC-6={:.3}",
                r.sc1_survival, r.sc2_survival_attention, r.sc3_binds,
                r.sc4_shaping_fraction, r.sc6_entropy_variance
            );
        }
    }
}

// --------------------------------------------------------------------------
// Stage 6b — the SC-1…6 / experimental-arm scoping question
// (§16.2, §27.3 criterion 8, §30.4). See ADR-0043.
// --------------------------------------------------------------------------

/// **Path 1 config — `decision.random` (§30.4 Arm C plugin) in a regime where
/// all four constraints bite.** The manual scopes SC-1…6 to no decision plugin
/// (only SC-1 carries a qualifier, "under no shock"); §30.4 lists the decision
/// plugin as a swept factor. This config tests whether *some* already-built
/// plugin can satisfy all six jointly, reading SC-1…6 as a check on the
/// model's plumbing rather than on `decision.satisficing`'s emergent behaviour.
///
/// Economy tuned so a low-capability firm bleeds out (insolvency) while a
/// high-capability firm compounds: `π^O = 4`, `π^I = 2`, so
/// `E[Δr^L] ≈ 0` for `c ≈ 0.5` and negative below. `θ_cap = 0.45` puts the
/// `c ∈ {0.25, 0.35}` firms permanently in `scope` violation; two firms are
/// seeded `q = 20 > θ_q = 10` for the `obligation` bind; `L_W = 4` lets a run
/// of `produce_regulated` draws push `u` past `θ_limit = 0.45` for the
/// `compliance` bind. `lobby` + `diversify` in the repertoire ⇒ a
/// uniform-random firm attempts shaping ~2/8 of ticks.
fn cfg_random_binding() -> String {
    cfg_random_binding_seed(4242)
}

fn cfg_random_binding_seed(mechanism: u64) -> String {
    format!(
        r#"{{ "experiment": "sc-arm-c", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": {{ "mechanism": {mechanism}, "environment": 2, "shock": 3, "init": 4 }},
      "world": {{ "ticks": 400, "resources": ["capital", "input"],
        "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }}, "snapshot_every": 400,
        "global_reals": {{ "theta_limit": 0.25, "theta_cap": 0.45 }},
        "global_ints": {{ "theta_q": 10, "input_price": 2, "output_price": 7 }} }},
      "agents": [
        {{ "id": 0,  "stocks": {{ "capital": 110, "input": 20 }}, "reals": {{ "capability": 0.22, "legitimacy": 1.0 }} }},
        {{ "id": 1,  "stocks": {{ "capital": 120, "input": 20 }}, "reals": {{ "capability": 0.30, "legitimacy": 1.0 }} }},
        {{ "id": 2,  "stocks": {{ "capital": 150, "input": 20 }}, "reals": {{ "capability": 0.40, "legitimacy": 1.0 }} }},
        {{ "id": 3,  "stocks": {{ "capital": 600, "input": 60 }}, "reals": {{ "capability": 0.50, "legitimacy": 1.0 }} }},
        {{ "id": 4,  "stocks": {{ "capital": 600, "input": 60 }}, "reals": {{ "capability": 0.55, "legitimacy": 1.0 }} }},
        {{ "id": 5,  "stocks": {{ "capital": 600, "input": 60 }}, "reals": {{ "capability": 0.60, "legitimacy": 1.0 }} }},
        {{ "id": 6,  "stocks": {{ "capital": 650, "input": 60 }}, "reals": {{ "capability": 0.65, "legitimacy": 1.0 }} }},
        {{ "id": 7,  "stocks": {{ "capital": 650, "input": 60 }}, "reals": {{ "capability": 0.70, "legitimacy": 1.0 }} }},
        {{ "id": 8,  "stocks": {{ "capital": 700, "input": 60 }}, "reals": {{ "capability": 0.78, "legitimacy": 1.0 }} }},
        {{ "id": 9,  "stocks": {{ "capital": 700, "input": 60 }}, "reals": {{ "capability": 0.85, "legitimacy": 1.0 }} }},
        {{ "id": 10, "stocks": {{ "capital": 600, "input": 40 }}, "reals": {{ "capability": 0.62, "legitimacy": 1.0 }}, "ints": {{ "obligation": 90 }} }},
        {{ "id": 11, "stocks": {{ "capital": 650, "input": 60 }}, "reals": {{ "capability": 0.66, "legitimacy": 1.0 }}, "ints": {{ "obligation": 70 }} }}
      ],
      "environment": {{ "stocks": {{ "capital": 100000000, "input": 100000000 }} }},
      "rules": [
        {{ "id": "decision.random", "version": "^1",
           "params": {{ "shaping": {{ "lobby_cost": 25, "diversify_cost": 25 }} }} }},{MARKET_ACTIONS},
        {{ "id": "action.shaping.rdt_standard.lobby", "version": "^1", "params": {{}} }},
        {{ "id": "action.shaping.rdt_standard.diversify", "version": "^1",
           "params": {{ "success": {{ "p0": 0.30, "b_lambda": 0.20, "b_kappa": 0.10, "p_max": 0.60, "kappa_min": 25 }},
             "lag": {{ "min": 2, "max": 6 }} }} }},{}
      ] }}"#,
        constraint_rules(4)
    )
    // `t_c = 1` (not the §16.1 default 4): under `decision.random` u swings
    // are uncorrelated draws, not a persistent regime, so the default T_c
    // window turns almost every first compliance strike into a near-certain
    // lethal second strike a few ticks later — that conflates SC-1 (survival)
    // with SC-3 (compliance must bind *at least once*, not repeatedly kill).
    // `t_c = 1` keeps the same violation semantics (`g_2 = u - θ_limit`,
    // Graduated) while letting a first strike register without demanding a
    // near-immediate repeat to avoid death.
    .replace("\"t_c\": 4", "\"t_c\": 1")
}

// (`cfg_arm_b_satisficing` below still uses the §16.1 default `t_c = 4`.)

/// **Path 2 config — Arm-B-shaped: `decision.satisficing` under an applied
/// shock** (§30.4 Arm B: "both evolve from the model's own dynamics under an
/// applied shock"). A regulatory ramp tightens `θ_limit`, then a sustained
/// resource shock drives `π^I` above what stressed firms can afford. Tests
/// whether a shock makes `decision.satisficing` reach a shaping action —
/// per ADR-0042 it can only do so via the input-starved GOAL fallback.
/// Shaping lag `(2, 6)` is one of §30.4's three levels.
fn cfg_arm_b_satisficing() -> String {
    format!(
        r#"{{ "experiment": "sc-arm-b", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": {{ "mechanism": 8080, "environment": 41, "shock": 97, "init": 13 }},
      "world": {{ "ticks": 400, "resources": ["capital", "input"],
        "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }}, "snapshot_every": 400,
        "global_reals": {{ "theta_limit": 0.55, "theta_cap": 0.45 }},
        "global_ints": {{ "theta_q": 10, "input_price": 3, "output_price": 4 }} }},
      "agents": [
        {{ "id": 0, "stocks": {{ "capital": 140, "input": 6 }}, "reals": {{ "capability": 0.35, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }} }},
        {{ "id": 1, "stocks": {{ "capital": 140, "input": 6 }}, "reals": {{ "capability": 0.40, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }} }},
        {{ "id": 2, "stocks": {{ "capital": 150, "input": 6 }}, "reals": {{ "capability": 0.45, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }} }},
        {{ "id": 3, "stocks": {{ "capital": 150, "input": 6 }}, "reals": {{ "capability": 0.50, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }} }},
        {{ "id": 4, "stocks": {{ "capital": 160, "input": 6 }}, "reals": {{ "capability": 0.55, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }} }},
        {{ "id": 5, "stocks": {{ "capital": 160, "input": 6 }}, "reals": {{ "capability": 0.60, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }} }},
        {{ "id": 6, "stocks": {{ "capital": 170, "input": 6 }}, "reals": {{ "capability": 0.65, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }} }},
        {{ "id": 7, "stocks": {{ "capital": 170, "input": 6 }}, "reals": {{ "capability": 0.70, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }} }},
        {{ "id": 8, "stocks": {{ "capital": 150, "input": 6 }}, "reals": {{ "capability": 0.60, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }}, "ints": {{ "obligation": 20 }} }},
        {{ "id": 9, "stocks": {{ "capital": 150, "input": 6 }}, "reals": {{ "capability": 0.55, "legitimacy": 1.0, "aspiration_capital_growth": 40.0 }}, "ints": {{ "obligation": 20 }} }}
      ],
      "environment": {{ "stocks": {{ "capital": 100000000, "input": 100000000 }} }},
      "rules": [
        {{ "id": "shock.scheduled", "version": "^1",
           "params": {{ "shocks": [
             {{ "id": "reg-tighten", "channel": {{ "kind": "regulatory", "target": "theta_limit" }},
               "magnitude": 0.20, "onset": 40,
               "ramp": {{ "kind": "linear", "duration": 20 }},
               "persistence": {{ "kind": "permanent" }},
               "observability": {{ "kind": "full" }}, "novelty": 0.7, "targets": {{ "kind": "all" }} }},
             {{ "id": "input-squeeze", "channel": {{ "kind": "resource" }},
               "magnitude": 40.0, "onset": 80,
               "ramp": {{ "kind": "linear", "duration": 30 }},
               "persistence": {{ "kind": "permanent" }},
               "observability": {{ "kind": "full" }}, "novelty": 0.5, "targets": {{ "kind": "all" }} }}
           ] }} }},
        {{ "id": "decision.satisficing", "version": "^1",
           "params": {{ "l_w": 4, "w_max": 9,
             "shaping": {{ "lobby_cost": 25, "diversify_cost": 25 }} }} }},{MARKET_ACTIONS},
        {{ "id": "action.shaping.rdt_standard.lobby", "version": "^1", "params": {{}} }},
        {{ "id": "action.shaping.rdt_standard.diversify", "version": "^1",
           "params": {{ "success": {{ "p0": 0.30, "b_lambda": 0.20, "b_kappa": 0.10, "p_max": 0.60, "kappa_min": 25 }},
             "lag": {{ "min": 2, "max": 6 }} }} }},{}
      ] }}"#,
        constraint_rules(4)
    )
}

fn sc_line(tag: &str, val: String, pass: bool) -> String {
    format!("  {tag:<44} {val:<26} {}", mark(pass))
}

/// **Stage 6b — the SC-1…6 / arm-scoping resolution.** Reports Path 1
/// (`decision.random`, all six against the manual's literal metric — SC-2 by
/// reconstructed `h < h_crit` since `decision.random` writes no `focus`) and
/// Path 2 (Arm-B-shaped `decision.satisficing`, SC-1/4/5 under shock and
/// SC-2/3/6 in the no-shock baseline). Prints one table for each; the ADR
/// (0043) draws the conclusion.
#[test]
fn sc16b_arm_scoping() {
    // ---- Path 1: decision.random, all six jointly ----
    let r = run_report("sc-arm-c", &cfg_random_binding());
    // SC-2 for a no-focus plugin: the manual's literal metric is
    // "firm-ticks with h < h_crit", which replay reconstructs directly.
    let sc2_val = r.sc2_reconstructed_h_below_crit;
    let sc2_pass = (0.05..=0.25).contains(&sc2_val);

    eprintln!("\n=== PATH 1 — decision.random (Arm C plugin), SC-1…6 jointly ===");
    eprintln!(
        "  ({} firms → {} survivors; {} decisions, {} shaping commits)",
        r.initial_firms, r.survivors, r.decisions, r.shaping_commits
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-1 survival [0.60, 0.90]",
            format!("{:.3}", r.sc1_survival),
            r.sc1_pass()
        )
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-2 h<h_crit [0.05, 0.25] (reconstructed)",
            format!("{sc2_val:.3}"),
            sc2_pass
        )
    );
    eprintln!(
        "{}   first@ {:?}",
        sc_line(
            "SC-3 all four bind",
            format!("{:?}", r.sc3_binds),
            r.sc3_pass()
        ),
        r.sc3_first_bind_tick
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-4 shaping > 5%",
            format!("{:.4}", r.sc4_shaping_fraction),
            r.sc4_pass()
        )
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-5 shaping success [0.10, 0.60]",
            format!(
                "{:?}",
                r.sc5_shaping_success.map(|x| (x * 1000.0).round() / 1000.0)
            ),
            r.sc5_pass()
        )
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-6 entropy variance >= 0.01",
            format!("{:.4}", r.sc6_entropy_variance),
            r.sc6_pass()
        )
    );
    let path1_joint =
        r.sc1_pass() && sc2_pass && r.sc3_pass() && r.sc4_pass() && r.sc5_pass() && r.sc6_pass();
    eprintln!("  PATH 1 JOINT: {}", mark(path1_joint));

    // ---- Path 2: Arm-B-shaped decision.satisficing ----
    let b = run_report("sc-arm-b", &cfg_arm_b_satisficing());
    // no-shock baseline for SC-2/3/6 = the Stage-6 "threaded needle" config.
    let base = run_report("sc-thread", &cfg_threaded());

    eprintln!("\n=== PATH 2 — Arm-B-shaped decision.satisficing (SC-1/4/5 shocked; SC-2/3/6 no-shock) ===");
    eprintln!(
        "  shocked: {} firms → {} survivors, {} decisions, {} shaping commits",
        b.initial_firms, b.survivors, b.decisions, b.shaping_commits
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-1 survival [0.60, 0.90] (shocked)",
            format!("{:.3}", b.sc1_survival),
            b.sc1_pass()
        )
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-2 attention [0.05, 0.25] (no-shock)",
            format!("{:.3}", base.sc2_survival_attention),
            base.sc2_pass()
        )
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-3 all four bind (no-shock)",
            format!("{:?}", base.sc3_binds),
            base.sc3_pass()
        )
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-4 shaping > 5% (shocked)",
            format!("{:.4}", b.sc4_shaping_fraction),
            b.sc4_pass()
        )
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-5 shaping success [0.10, 0.60] (shocked)",
            format!(
                "{:?}",
                b.sc5_shaping_success.map(|x| (x * 1000.0).round() / 1000.0)
            ),
            b.sc5_pass()
        )
    );
    eprintln!(
        "{}",
        sc_line(
            "SC-6 entropy variance >= 0.01 (no-shock)",
            format!("{:.4}", base.sc6_entropy_variance),
            base.sc6_pass()
        )
    );
    let path2_joint = b.sc1_pass()
        && base.sc2_pass()
        && base.sc3_pass()
        && b.sc4_pass()
        && b.sc5_pass()
        && base.sc6_pass();
    eprintln!("  PATH 2 JOINT: {}", mark(path2_joint));

    // ---- Path 1 robustness: the same config over five mechanism seeds ----
    eprintln!("\n=== PATH 1 robustness — cfg_random_binding over 5 seeds ===");
    let mut all_seeds_joint = true;
    for seed in [4242u64, 1, 77, 900001, 31337] {
        let s = run_report("sc-arm-c", &cfg_random_binding_seed(seed));
        let s2 = s.sc2_reconstructed_h_below_crit;
        let joint = s.sc1_pass()
            && (0.05..=0.25).contains(&s2)
            && s.sc3_pass()
            && s.sc4_pass()
            && s.sc5_pass()
            && s.sc6_pass();
        all_seeds_joint &= joint;
        eprintln!(
            "  seed {seed:>7}: SC-1={:.3} SC-2={:.3} SC-3={:?} SC-4={:.3} SC-5={:?} SC-6={:.3}  → {}",
            s.sc1_survival,
            s2,
            s.sc3_binds,
            s.sc4_shaping_fraction,
            s.sc5_shaping_success.map(|x| (x * 100.0).round() / 100.0),
            s.sc6_entropy_variance,
            mark(joint)
        );
    }

    eprintln!("\n=== outcome ===");
    eprintln!(
        "  Path 1 (decision.random, all six one config):     {}",
        mark(path1_joint)
    );
    eprintln!(
        "  Path 1 holds across all 5 seeds:                   {}",
        mark(all_seeds_joint)
    );
    eprintln!(
        "  Path 2 (Arm-B satisficing, SC-1/4/5 under shock):  {}",
        mark(path2_joint)
    );

    // Lock the finding.
    assert!(path1_joint, "Path 1: decision.random no longer jointly satisfies SC-1…6 — re-review the arm-scoping ADR (0043)");
    assert!(
        all_seeds_joint,
        "Path 1: the joint SC-1…6 pass is now seed-fragile — re-review"
    );
    assert!(
        !path2_joint,
        "Path 2 now also passes — worth noting, but ADR-0043's primary reading is Path 1"
    );
    // decision.satisficing under shock still never selects shaping (ADR-0042).
    assert_eq!(
        b.sc4_shaping_fraction, 0.0,
        "decision.satisficing selected a shaping action under shock — ADR-0042's structural finding changed"
    );
}
