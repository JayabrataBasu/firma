//! Cross-crate integration checks that are not part of the DT/VT gate but back
//! up the §25.6 lints and the §27.3 acceptance criteria.

use firma_cli::{execute_run, model_registry, standard_registry, RunOptions};
use firma_conformance::{active_config, config, scratch};
use firma_io::Manifest;

/// §25.6 `assumption-nonempty` / §27.3 criterion 6: every plugin's
/// `assumption()` is non-empty **and appears in the manifest**.
#[test]
fn every_manifest_plugin_has_a_nonempty_assumption() {
    let cfg = active_config(5);
    let reg = standard_registry();
    let dir = scratch("assump");
    execute_run(&cfg, &reg, &dir, &RunOptions::default()).unwrap();

    let manifest = Manifest::read(&dir).unwrap();
    assert!(
        !manifest.identity.rules.is_empty(),
        "no rules in the manifest"
    );
    for entry in &manifest.identity.rules {
        assert!(
            !entry.assumption.trim().is_empty(),
            "plugin {:?} has an empty assumption in the manifest",
            entry.id
        );
    }
    assert!(
        !manifest
            .identity
            .conflict_resolver
            .assumption
            .trim()
            .is_empty(),
        "the conflict resolver has an empty assumption"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// §27.3 criterion 4: two runs from the same manifest are byte-identical; the
/// manifest's SHA-256 is the run id and is stable.
#[test]
fn manifest_run_id_is_stable() {
    let cfg = active_config(50);
    let reg = standard_registry();
    let a = scratch("rid-a");
    let b = scratch("rid-b");
    let ra = execute_run(&cfg, &reg, &a, &RunOptions::default()).unwrap();
    let rb = execute_run(&cfg, &reg, &b, &RunOptions::default()).unwrap();
    assert_eq!(ra.run_id, rb.run_id);
    assert_eq!(
        ra.run_id,
        Manifest::read(&a).unwrap().identity.run_id().unwrap()
    );
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// The shipped Stage-4 smoke config (ADR 0028–0032): the whole Phase-2 model
/// registry on a real multi-firm world.
const PHASE2_SMOKE: &str = include_str!("../../configs/experiments/phase2-smoke.json");

/// Stage-4 Part E: the first real end-to-end run of the full model — `decide →
/// act_market → resolve_lagged → constrain → enforce → record` over ten ticks,
/// several firms, no shocks. Asserts a realistic tick actually happened (not
/// just "it didn't crash"): a `produce_regulated` choice moved `u` far enough
/// to trip `compliance`, a graduated death occurred, a lagged capability gain
/// matured, and conservation held — reconstructed from the event log alone
/// (VT-6 / §22.2 log sufficiency).
#[test]
fn phase2_smoke_runs_a_realistic_tick_and_conserves() {
    use firma_kernel::Event;

    let cfg = config(PHASE2_SMOKE);
    let reg = model_registry();
    let dir = scratch("phase2-smoke");
    let report = execute_run(&cfg, &reg, &dir, &RunOptions::default()).unwrap();

    assert_eq!(report.ticks, 10);
    assert!(report.conservation_ok, "live conservation failed");

    // --- reconstruct every signal from the event log alone ---
    let events: Vec<Event> = firma_io::read_events::<Event>(&dir.join("events.ndjson"))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    // opening balances from the config
    let mut totals: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for a in &cfg.agents {
        for (r, q) in &a.stocks {
            *totals.entry(r.0.clone()).or_default() += q;
        }
    }
    for (r, q) in &cfg.environment.stocks {
        *totals.entry(r.0.clone()).or_default() += q;
    }
    let initial = totals.clone();

    let mut deaths: Vec<(u64, String)> = Vec::new();
    let mut regulated_intensity_seen = false;
    let mut lagged_capability_gain = false;
    let mut first_strike_penalty = false;

    for ev in &events {
        match ev {
            Event::DeltaApplied {
                kind: firma_core::DeltaKind::AdjustStock { resource, amount },
                ..
            } => {
                *totals.entry(resource.0.clone()).or_default() += amount;
            }
            Event::DeltaApplied {
                phase,
                kind: firma_core::DeltaKind::ReplaceAgentList { list, records_json },
                ..
            } if list == "action_window" => {
                if records_json.iter().any(|r| r.contains("\"action\":2")) {
                    regulated_intensity_seen = true;
                }
                let _ = phase;
            }
            Event::DeltaApplied {
                phase,
                kind: firma_core::DeltaKind::AdjustAgentReal { field, .. },
                ..
            } if field == "capability" && *phase == 6 => {
                lagged_capability_gain = true;
            }
            Event::DeltaApplied {
                phase,
                kind: firma_core::DeltaKind::SetAgentInt { field, .. },
                ..
            } if field == "compliance_last_violation_tick" && *phase == 8 => {
                first_strike_penalty = true;
            }
            Event::AgentDied { tick, cause, .. } => deaths.push((*tick, cause.clone())),
            _ => {}
        }
    }

    assert_eq!(
        totals, initial,
        "VT-6: conservation reconstructed from the log does not hold\n  initial {initial:?}\n  final   {totals:?}"
    );
    assert!(
        regulated_intensity_seen,
        "no firm ever chose produce_regulated — u never moved"
    );
    assert!(
        lagged_capability_gain,
        "no lagged capability gain matured in resolve_lagged (phase 6)"
    );
    assert!(
        first_strike_penalty,
        "no compliance first-strike was recorded in enforce (phase 8)"
    );
    assert!(
        deaths.iter().any(|(_, c)| c == "solvency"),
        "expected a solvency death, got {deaths:?}"
    );
    assert!(
        deaths.iter().filter(|(_, c)| c == "compliance").count() >= 2,
        "expected the two over-producers to die of compliance, got {deaths:?}"
    );
    assert_eq!(
        report.final_live_agents, 1,
        "only the capability-investing firm should survive"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// ADR 0030: a config may seed global and per-agent domain-state scalars, and
/// the orchestrator applies them to the `World` before tick 0.
#[test]
fn config_seeds_domain_state_into_the_world() {
    let cfg = config(PHASE2_SMOKE);
    // round-trips through canonical JSON unchanged
    let reparsed = firma_config::RunConfig::from_json(&cfg.to_json().unwrap()).unwrap();
    assert_eq!(cfg, reparsed);

    let (world, _schedule) = firma_cli::prepare_run(&cfg, &model_registry()).unwrap();
    assert_eq!(world.global_real("theta_limit"), Some(0.45));
    assert_eq!(world.global_int("input_price"), Some(2));
    assert_eq!(
        world.agent_real(firma_core::AgentId(0), "capability"),
        Some(0.60)
    );
    assert_eq!(
        world.agent_real(firma_core::AgentId(2), "aspiration_capability"),
        Some(0.80)
    );
}

/// ADR 0030 Consequences: a config that omits the four seed maps canonicalises
/// byte-identically to a pre-ADR-0030 config — the `phase1-smoke` reference is
/// unaffected by the schema growth.
#[test]
fn config_without_seed_maps_is_unchanged() {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../configs/experiments/phase1-smoke.json"
    ))
    .unwrap();
    let cfg = firma_config::RunConfig::from_json(&json).unwrap();
    let canonical = cfg.to_json().unwrap();
    assert!(
        !canonical.contains("global_reals")
            && !canonical.contains("global_ints")
            && !canonical.contains("\"reals\"")
            && !canonical.contains("\"ints\""),
        "an empty seed map leaked into the canonical form"
    );
}

/// The shipped Stage-5 smoke config: the full model registry plus
/// `resource.patchy`, `shock.scheduled` (a regulatory ramp and a transient
/// resource spike), and `observation.delayed`.
const STAGE5_SMOKE: &str = include_str!("../../configs/experiments/phase2-stage5-smoke.json");

/// Stage 5 Part E: environment/observe-phase machinery runs end to end. A
/// regulatory shock ramps `θ_limit` down over three ticks; a transient resource
/// shock spikes `π^I` and reverses; `resource.patchy` walks `π^I`; every firm's
/// decision reads a two-tick-stale environment. Conservation holds
/// (reconstructed from the log) and the run is deterministic.
#[test]
fn phase2_stage5_smoke_exercises_shocks_observation_and_resource_dynamics() {
    use firma_kernel::Event;

    let cfg = config(STAGE5_SMOKE);
    let reg = model_registry();
    let a = scratch("s5-a");
    let b = scratch("s5-b");
    let ra = execute_run(&cfg, &reg, &a, &RunOptions::default()).unwrap();
    let rb = execute_run(&cfg, &reg, &b, &RunOptions::default()).unwrap();
    assert_eq!(
        ra.event_log_sha256, rb.event_log_sha256,
        "run is not deterministic"
    );
    assert!(ra.conservation_ok);

    let events: Vec<Event> = firma_io::read_events::<Event>(&a.join("events.ndjson"))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut totals: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for ag in &cfg.agents {
        for (r, q) in &ag.stocks {
            *totals.entry(r.0.clone()).or_default() += q;
        }
    }
    for (r, q) in &cfg.environment.stocks {
        *totals.entry(r.0.clone()).or_default() += q;
    }
    let initial = totals.clone();

    let mut regulatory_shock_steps = 0usize;
    let mut resource_shock_seen = false;
    let mut patchy_moved_price = false;
    let mut observation_snapshots = 0usize;
    let mut delayed_lag_observed = false;

    for ev in &events {
        if let Event::DeltaApplied {
            origin,
            phase,
            kind,
            ..
        } = ev
        {
            match kind {
                firma_core::DeltaKind::AdjustStock { resource, amount } => {
                    *totals.entry(resource.0.clone()).or_default() += amount;
                }
                firma_core::DeltaKind::AdjustGlobalReal { field, .. }
                    if origin.0 == "shock.scheduled" && field == "theta_limit" =>
                {
                    regulatory_shock_steps += 1;
                }
                firma_core::DeltaKind::AdjustGlobalInt { field, .. }
                    if origin.0 == "shock.scheduled" && field == "input_price" =>
                {
                    resource_shock_seen = true;
                }
                firma_core::DeltaKind::AdjustGlobalInt { field, .. }
                    if origin.0 == "resource.patchy" && field == "input_price" =>
                {
                    patchy_moved_price = true;
                }
                firma_core::DeltaKind::ReplaceAgentList { list, records_json }
                    if *phase == 2 && list == "observed_env" =>
                {
                    observation_snapshots += 1;
                    // the observed source-tick lags the decide tick once the ring fills
                    if let Ok(snap) = firma_domain::EnvSnapshot::from_json(&records_json[0]) {
                        // ev has no tick field here; checked structurally below
                        let _ = snap;
                    }
                }
                _ => {}
            }
        }
    }

    // delayed(k=2): find a per-agent snapshot whose source tick is 2 behind its
    // own DeltaApplied tick.
    for ev in &events {
        if let Event::DeltaApplied {
            tick,
            phase: 2,
            kind: firma_core::DeltaKind::ReplaceAgentList { list, records_json },
            ..
        } = ev
        {
            if list == "observed_env" && *tick >= 3 {
                if let Ok(snap) = firma_domain::EnvSnapshot::from_json(&records_json[0]) {
                    if snap.tick == tick - 2 {
                        delayed_lag_observed = true;
                    }
                }
            }
        }
    }

    assert_eq!(totals, initial, "VT-6: log conservation failed");
    assert_eq!(
        regulatory_shock_steps, 3,
        "regulatory shock should ramp θ_limit in 3 linear steps"
    );
    assert!(
        resource_shock_seen,
        "the transient resource shock never fired"
    );
    assert!(patchy_moved_price, "resource.patchy never moved π^I");
    assert!(
        observation_snapshots > 0,
        "no observation snapshots written"
    );
    assert!(
        delayed_lag_observed,
        "observation.delayed did not lag by k=2"
    );

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// ADR 0034 / §21.3: a rule's RNG stream follows its declared `rng_stream()`,
/// not its phase. `observation.noisy` (phase `observe`, default `mechanism`)
/// and `shock.stochastic` (phase `environment`, default `environment`) both
/// override — and the run stays deterministic (the real plumbing check), while
/// changing only the `mechanism` seed leaves the drawn shock onset/magnitude
/// untouched (matched-environment design).
#[test]
fn stage5_rng_streams_follow_the_declared_stream() {
    use firma_kernel::Event;

    let base = r#"{
      "experiment": "s5-rng", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": { "mechanism": %M%, "environment": 5, "shock": 77, "init": 1 },
      "world": { "ticks": 12, "resources": ["capital", "input"],
        "conflict_resolver": { "id": "conflict.additive", "version": "^1" }, "snapshot_every": 6,
        "global_reals": { "theta_limit": 0.5, "theta_cap": 0.4 },
        "global_ints": { "theta_q": 100, "input_price": 2, "output_price": 3 } },
      "agents": [
        { "id": 0, "stocks": { "capital": 500, "input": 20 },
          "reals": { "capability": 0.5, "legitimacy": 1.0, "aspiration_capital_growth": 6.0 } },
        { "id": 1, "stocks": { "capital": 500, "input": 20 },
          "reals": { "capability": 0.3, "legitimacy": 1.0, "aspiration_capability": 0.7 } }
      ],
      "environment": { "stocks": { "capital": 1000000, "input": 1000000 } },
      "rules": [
        { "id": "observation.noisy", "version": "^1", "params": { "sigma": 0.03 } },
        { "id": "shock.stochastic", "version": "^1", "params": {
          "id": "z", "channel": { "kind": "regulatory", "target": "theta_limit" },
          "onset_min": 3, "onset_max": 7, "magnitude_mean": 0.2, "magnitude_sd": 0.05,
          "ramp": { "kind": "instant" }, "persistence": { "kind": "permanent" },
          "observability": { "kind": "full" }, "novelty": 0.6, "targets": { "kind": "all" } } },
        { "id": "decision.satisficing", "version": "^1", "params": { "l_w": 4 } },
        { "id": "action.market.standard.hold", "version": "^1", "params": {} },
        { "id": "action.market.standard.produce_ordinary", "version": "^1", "params": {} },
        { "id": "action.market.standard.produce_regulated", "version": "^1", "params": {} },
        { "id": "action.market.standard.acquire_input", "version": "^1", "params": {} },
        { "id": "action.market.standard.invest_capability", "version": "^1", "params": {} },
        { "id": "action.market.standard.deliver", "version": "^1", "params": {} },
        { "id": "action.shaping.rdt_standard.resolve_lagged", "version": "^1", "params": {} },
        { "id": "constraint.action_window", "version": "^1", "params": { "l_w": 4 } },
        { "id": "constraint.enforce", "version": "^1",
          "params": { "t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": 4 } },
        { "id": "decision.aspiration_update", "version": "^1", "params": { "alpha": 0.1 } }
      ]
    }"#;

    let onset_and_mag = |mech: &str| -> (u64, String) {
        let cfg = config(&base.replace("%M%", mech));
        let dir = scratch("s5rng");
        execute_run(&cfg, &model_registry(), &dir, &RunOptions::default()).unwrap();
        let events: Vec<Event> = firma_io::read_events::<Event>(&dir.join("events.ndjson"))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        // the drawn shock is logged in the first ACTIVE_SHOCKS rewrite
        for ev in &events {
            if let Event::DeltaApplied {
                kind: firma_core::DeltaKind::ReplaceGlobalList { list, records_json },
                ..
            } = ev
            {
                if list == "active_shocks" && !records_json.is_empty() {
                    let v: serde_json::Value = serde_json::from_str(&records_json[0]).unwrap();
                    return (
                        v["shock"]["onset"].as_u64().unwrap(),
                        v["shock"]["magnitude"].to_string(),
                    );
                }
            }
        }
        panic!("no shock drawn");
    };

    // Determinism: same everything ⇒ identical draw.
    assert_eq!(onset_and_mag("11"), onset_and_mag("11"));
    // Matched-environment (§21.3): the shock draw is on the `shock` stream, so
    // changing only the `mechanism` seed does not move it.
    assert_eq!(onset_and_mag("11"), onset_and_mag("999"));
    let (onset, _) = onset_and_mag("11");
    assert!(
        (3..=7).contains(&onset),
        "drawn onset {onset} outside the window"
    );
}

/// ADR 0037: the locality plugins construct and answer `neighbours()`.
/// Nothing in the MVP consumes them.
#[test]
fn locality_plugins_construct_and_answer_neighbours() {
    use firma_core::AgentId;

    let live = [AgentId(0), AgentId(1), AgentId(4)];
    for (id, _hash, ctor) in firma_cli::locality_catalogue() {
        let params = if id == "locality.network" {
            // a path 0—1—4, so agent 1's neighbours are 0 and 4 — the same
            // answer `wellmixed` gives for this fully-connected-by-the-path set
            serde_json::json!({ "edges": [[0, 1], [1, 4]] })
        } else {
            serde_json::json!({})
        };
        let loc = ctor(&params).unwrap();
        assert_eq!(loc.id(), id);
        assert!(!loc.assumption().trim().is_empty());
        let n = loc.neighbours(AgentId(1), &live, 0);
        assert!(!n.contains(&AgentId(1)), "self is not a neighbour");
        assert_eq!(n, vec![AgentId(0), AgentId(4)]);
    }

    // `network` really is a restriction: agent 0 sees only 1, not 4.
    let net = firma_cli::locality_catalogue()
        .into_iter()
        .find(|(id, ..)| *id == "locality.network")
        .map(|(_, _, ctor)| ctor(&serde_json::json!({ "edges": [[0, 1], [1, 4]] })).unwrap())
        .unwrap();
    assert_eq!(net.neighbours(AgentId(0), &live, 0), vec![AgentId(1)]);
}

/// ADR 0035, Stage-5 follow-up Item 2: `observation.noisy` is **not inert
/// when present** — with a firm poised exactly on the `θ_cap` boundary and a
/// `σ` that perturbs the perceived boundary off `c`, the decision procedure's
/// `selected_action` differs from the `observation.full` (true-value)
/// baseline, given identical true state and identical seeds for every stream.
///
/// True `c = θ_cap = 0.40`, so `g_3 = θ_cap − c = 0` and `h = 0 < h_crit`:
/// under `full` the firm is in **SURVIVAL** focus, `w_eff = 1`, and takes the
/// first admissible action in the survival scan order — `produce_ordinary`
/// (1). Under `noisy(σ = 0.15)` the fixed seed here perturbs the perceived
/// `θ_cap` **below** `c`, so `g_3 < 0`, `h > 0`, the firm attends to its
/// capital-growth goal instead (GOAL(1)), and — believing itself comfortably
/// in scope — takes the higher-yield `produce_regulated` (2). A perceptual
/// optimism that flips the firm to the riskier action: exactly the §12.2
/// consequence the noisy channel exists to model.
#[test]
fn observation_noisy_changes_a_decision_versus_the_true_value_baseline() {
    use firma_kernel::Event;

    // Identical but for the observation rule. One firm on the θ_cap boundary.
    let cfg_for = |obs_rule: &str| -> String {
        format!(
            r#"{{
              "experiment": "obs-flip", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
              "seeds": {{ "mechanism": 7, "environment": 20260906, "shock": 3, "init": 1 }},
              "world": {{ "ticks": 2, "resources": ["capital", "input"],
                "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }}, "snapshot_every": 2,
                "global_reals": {{ "theta_limit": 0.90, "theta_cap": 0.40 }},
                "global_ints": {{ "theta_q": 100, "input_price": 2, "output_price": 3 }} }},
              "agents": [
                {{ "id": 0, "stocks": {{ "capital": 500, "input": 20 }},
                   "reals": {{ "capability": 0.40, "legitimacy": 1.0, "aspiration_capital_growth": 5.0 }} }}
              ],
              "environment": {{ "stocks": {{ "capital": 1000000, "input": 1000000 }} }},
              "rules": [
                {obs_rule}
                {{ "id": "decision.satisficing", "version": "^1", "params": {{ "l_w": 4 }} }},
                {{ "id": "action.market.standard.hold", "version": "^1", "params": {{}} }},
                {{ "id": "action.market.standard.produce_ordinary", "version": "^1", "params": {{}} }},
                {{ "id": "action.market.standard.produce_regulated", "version": "^1", "params": {{}} }},
                {{ "id": "action.market.standard.acquire_input", "version": "^1", "params": {{}} }},
                {{ "id": "action.market.standard.invest_capability", "version": "^1", "params": {{}} }},
                {{ "id": "action.market.standard.deliver", "version": "^1", "params": {{}} }},
                {{ "id": "constraint.action_window", "version": "^1", "params": {{ "l_w": 4 }} }},
                {{ "id": "constraint.enforce", "version": "^1",
                   "params": {{ "t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": 4 }} }},
                {{ "id": "decision.aspiration_update", "version": "^1", "params": {{ "alpha": 0.1 }} }}
              ]
            }}"#
        )
    };

    let first_selected_action = |obs_rule: &str| -> i64 {
        let cfg = config(&cfg_for(obs_rule));
        let dir = scratch("obs-flip");
        execute_run(&cfg, &model_registry(), &dir, &RunOptions::default()).unwrap();
        let events: Vec<Event> = firma_io::read_events::<Event>(&dir.join("events.ndjson"))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        events
            .iter()
            .find_map(|ev| match ev {
                Event::DeltaApplied {
                    tick: 0,
                    origin,
                    kind: firma_core::DeltaKind::SetAgentInt { field, value },
                    ..
                } if origin.0 == "decision.satisficing" && field == "selected_action" => {
                    Some(*value)
                }
                _ => None,
            })
            .expect("a selected_action delta at tick 0")
    };

    let baseline =
        first_selected_action(r#"{ "id": "observation.full", "version": "^1", "params": {} },"#);
    let noisy = first_selected_action(
        r#"{ "id": "observation.noisy", "version": "^1", "params": { "sigma": 0.15 } },"#,
    );
    // Determinism of the noisy run itself.
    let noisy_again = first_selected_action(
        r#"{ "id": "observation.noisy", "version": "^1", "params": { "sigma": 0.15 } },"#,
    );

    // A third σ, to show the effect is graded and non-trivial (not a knife-edge
    // artefact): σ = 0.05 lands on yet another action.
    let noisy_005 = first_selected_action(
        r#"{ "id": "observation.noisy", "version": "^1", "params": { "sigma": 0.05 } },"#,
    );

    eprintln!("observation.full         -> selected_action = {baseline}   (SURVIVAL focus → produce_ordinary)");
    eprintln!("observation.noisy σ=0.05 -> selected_action = {noisy_005}   (misperceived e/θ → a different choice)");
    eprintln!("observation.noisy σ=0.15 -> selected_action = {noisy}   (perceived θ_cap < c → GOAL(1) → produce_regulated)");
    assert_eq!(noisy, noisy_again, "the noisy run is not deterministic");
    assert_eq!(
        baseline, 1,
        "true-value baseline: SURVIVAL fallback → produce_ordinary (1)"
    );
    assert_ne!(
        noisy, baseline,
        "observation.noisy did not change the decision (σ = 0.15, firm on the θ_cap boundary)"
    );
    assert_eq!(
        noisy, 2,
        "σ = 0.15: perceived θ_cap < c ⇒ GOAL(1) ⇒ produce_regulated (2)"
    );
    assert_ne!(noisy_005, baseline, "σ = 0.05 also changes the decision");

    // Why σ = 0.15 is where `produce_regulated` specifically appears: the flip
    // to GOAL(1) is a *focus* change, which needs perceived `θ_cap` about
    // `h_crit · s_c = 0.075` below its true 0.40 so that `h` crosses `h_crit`.
    // σ = 0.15 on a [0,1] parameter (default 0.40) is a firm with a genuinely
    // hazy read of the regulatory threshold — the manual gives no σ (it is an
    // experimental knob), and this is not a degenerate value.
}

/// The full CLI surface: `run` then `verify` on a fresh directory.
#[test]
fn run_and_verify_end_to_end() {
    let cfg = active_config(400);
    let reg = standard_registry();
    let dir = scratch("e2e");
    let report = execute_run(&cfg, &reg, &dir, &RunOptions::default()).unwrap();
    assert!(report.conservation_ok);
    assert!(dir.join("events.ndjson").exists());
    assert!(dir.join("manifest.json").exists());
    assert!(dir.join("run_id.txt").exists());

    let v = firma_cli::verify_run(&dir).unwrap();
    assert!(v.ok(), "verify failed: {v:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// ADR-0054 test-plan items 3 (persistence across ticks) and 4 (real-state
/// independence), combined in one scenario: a single `at: 0`
/// `SetAgentReal` intervention pins a *healthy* margin (`h = 0.40`, no
/// shortfall on any goal) and a seeded `SELECTED_ACTION = 2`
/// (`produce_regulated`). Under the pin, `Focus::attend` must resolve to
/// `None` (healthy margin, zero shortfall) at **every** tick without the
/// intervention ever being reapplied (item 3) — which means the firm
/// repeats `produce_regulated` every tick, driving its *real* regulated-
/// activity window past `θ_limit` regardless of the pin's claim of health.
/// `constraint.enforce`'s real violation detection must still kill the
/// firm (item 4) — a pinned firm is not pinned out of a real compliance
/// violation.
#[test]
fn adr0054_pin_persists_and_real_violations_still_kill() {
    use firma_core::{DeltaKind, DeltaTarget, Event};

    let cfg = config(
        r#"{
      "experiment": "adr0054-item3-4", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
      "seeds": { "mechanism": 1, "environment": 1, "shock": 1, "init": 1 },
      "world": { "ticks": 20, "resources": ["capital", "input"],
        "conflict_resolver": { "id": "conflict.additive", "version": "^1" },
        "global_reals": { "theta_limit": 0.40, "theta_cap": 0.20 },
        "global_ints": { "theta_q": 100, "input_price": 2, "output_price": 3 } },
      "agents": [ { "id": 0, "stocks": { "capital": 5000, "input": 5000 },
        "reals": { "capability": 0.9, "legitimacy": 1.0 },
        "ints": { "selected_action": 2 } } ],
      "environment": { "stocks": { "capital": 100000000, "input": 100000000 } },
      "interventions": [
        { "at": 0, "op": "set_agent_real", "agent": 0, "field": "pinned_margin", "value": 0.40 },
        { "at": 0, "op": "set_agent_real", "agent": 0, "field": "pinned_shortfall_capital_growth", "value": 0.0 },
        { "at": 0, "op": "set_agent_real", "agent": 0, "field": "pinned_shortfall_capability", "value": 0.0 },
        { "at": 0, "op": "set_agent_real", "agent": 0, "field": "pinned_shortfall_obligation_clearance", "value": 0.0 }
      ],
      "rules": [
        { "id": "decision.satisficing", "version": "^1", "params": { "l_w": 4 } },
        { "id": "action.market.standard.hold", "version": "^1", "params": {} },
        { "id": "action.market.standard.produce_ordinary", "version": "^1", "params": {} },
        { "id": "action.market.standard.produce_regulated", "version": "^1", "params": {} },
        { "id": "action.market.standard.acquire_input", "version": "^1", "params": {} },
        { "id": "action.market.standard.invest_capability", "version": "^1", "params": {} },
        { "id": "action.market.standard.deliver", "version": "^1", "params": {} },
        { "id": "constraint.action_window", "version": "^1", "params": { "l_w": 4 } },
        { "id": "constraint.enforce", "version": "^1",
          "params": { "t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": 4 } }
      ] }"#,
    );
    let reg = model_registry();
    let dir = scratch("adr0054-persist");
    let report = execute_run(&cfg, &reg, &dir, &RunOptions::default()).unwrap();
    assert!(report.conservation_ok);

    let events: Vec<Event> = firma_io::read_events::<Event>(&dir.join("events.ndjson"))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    // Item 1 (of this test's own scope) -- exactly one `at: 0` batch of
    // interventions, never reapplied.
    let intervention_ticks: std::collections::BTreeSet<u64> = events
        .iter()
        .filter_map(|e| match e {
            Event::InterventionApplied { tick, .. } => Some(*tick),
            _ => None,
        })
        .collect();
    assert_eq!(
        intervention_ticks,
        std::collections::BTreeSet::from([0]),
        "the pin must be applied once, at tick 0, never reapplied"
    );

    // Item 3 -- persistence: every recorded `focus`/`selected_action` write
    // reflects the pin (None / repeat-2), for the whole run, read fresh
    // from Step 1 every tick with no repeated intervention.
    let mut saw_focus = 0;
    let mut saw_action = 0;
    for e in &events {
        if let Event::DeltaApplied {
            target: DeltaTarget::Agent(a),
            kind: DeltaKind::SetAgentInt { field, value },
            ..
        } = e
        {
            if a.0 != 0 {
                continue;
            }
            if field == "focus" {
                assert_eq!(
                    *value, -1,
                    "Focus::None (-1) expected at every tick under this pin"
                );
                saw_focus += 1;
            }
            if field == "selected_action" {
                assert_eq!(
                    *value, 2,
                    "NONE focus must repeat the seeded selected_action (2, produce_regulated)"
                );
                saw_action += 1;
            }
        }
    }
    assert!(
        saw_focus >= 2,
        "expected multiple ticks' worth of focus writes before death"
    );
    assert!(
        saw_action >= 2,
        "expected multiple ticks' worth of selected_action writes"
    );

    // Item 4 -- real-state independence: despite the pin's claim of a
    // healthy h = 0.40, the firm's *real* regulated-activity window still
    // breaches θ_limit = 0.40 under sustained produce_regulated, and
    // `constraint.enforce` still kills it -- a pinned firm is not pinned
    // out of a real compliance violation.
    let died = events
        .iter()
        .any(|e| matches!(e, Event::AgentDied { agent, .. } if agent.0 == 0));
    assert!(
        died,
        "the firm must still die of a real compliance violation despite the healthy pin"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
