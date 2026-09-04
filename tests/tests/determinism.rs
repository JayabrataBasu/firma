//! DT-1 … DT-6 — the determinism gate (manual §25.2). All six MUST be green
//! for the Phase 1 gate (§26.3).

use firma_cli::{execute_run, prepare_run, replay_run, standard_registry, CliError, RunOptions};
use firma_conformance::{active_config, config, deltas_to_agents, run_kernel, scratch};
use firma_core::Intervention;
use firma_kernel::{Event, Kernel, KernelError};

/// DT-1 — Same manifest twice → byte-identical event log.
#[test]
fn dt1_same_manifest_byte_identical_log() {
    let cfg = active_config(300);
    let reg = standard_registry();

    let d1 = scratch("dt1-a");
    let d2 = scratch("dt1-b");
    let r1 = execute_run(&cfg, &reg, &d1, &RunOptions::default()).unwrap();
    let r2 = execute_run(&cfg, &reg, &d2, &RunOptions::default()).unwrap();

    assert_eq!(r1.run_id, r2.run_id, "DT-1: run ids differ");
    let log1 = std::fs::read(d1.join("events.ndjson")).unwrap();
    let log2 = std::fs::read(d2.join("events.ndjson")).unwrap();
    assert_eq!(log1, log2, "DT-1: event logs are not byte-identical");
    assert_eq!(r1.event_log_sha256, r2.event_log_sha256);

    // …and the same via the `replay` path.
    let replay = replay_run(&d1).unwrap();
    assert!(replay.identical, "DT-1: replay produced a different log");

    let _ = std::fs::remove_dir_all(&d1);
    let _ = std::fs::remove_dir_all(&d2);
}

/// DT-2 — Shuffling internal agent storage order → identical output.
#[test]
fn dt2_agent_order_irrelevant() {
    let reg = standard_registry();
    let body = |agents: &str| {
        format!(
            r#"{{
              "experiment": "dt2",
              "schema_version": "1.0.0",
              "engine": ">=0.1.0, <0.2.0",
              "seeds": {{ "mechanism": 4, "environment": 4, "shock": 4, "init": 4 }},
              "world": {{ "ticks": 120, "resources": ["capital"],
                "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }} }},
              "agents": {agents},
              "rules": [
                {{ "id": "testkit.transfer", "version": "^1",
                   "params": {{ "resource": "capital", "from": {{ "agent": 0 }}, "to": {{ "agent": 2 }}, "amount": 1 }} }},
                {{ "id": "testkit.keyed_nudge", "version": "^1",
                   "params": {{ "resource": "capital", "from": 2, "to": 1, "modulus": 3, "purpose": "n" }} }}
              ]
            }}"#
        )
    };
    let ascending = r#"[{"id":0,"stocks":{"capital":100000}},{"id":1,"stocks":{"capital":5}},{"id":2,"stocks":{"capital":0}}]"#;
    let shuffled = r#"[{"id":2,"stocks":{"capital":0}},{"id":0,"stocks":{"capital":100000}},{"id":1,"stocks":{"capital":5}}]"#;

    let da = scratch("dt2-a");
    let db = scratch("dt2-b");
    execute_run(&config(&body(ascending)), &reg, &da, &RunOptions::default()).unwrap();
    execute_run(&config(&body(shuffled)), &reg, &db, &RunOptions::default()).unwrap();

    let la = std::fs::read(da.join("events.ndjson")).unwrap();
    let lb = std::fs::read(db.join("events.ndjson")).unwrap();
    assert_eq!(la, lb, "DT-2: agent listing order changed the output");

    let _ = std::fs::remove_dir_all(&da);
    let _ = std::fs::remove_dir_all(&db);
}

/// DT-3 — Single- vs multi-threaded phase execution → identical output.
#[test]
fn dt3_sequential_vs_parallel_identical() {
    let cfg = active_config(250);
    let reg = standard_registry();

    let dseq = scratch("dt3-seq");
    let dpar = scratch("dt3-par");
    execute_run(
        &cfg,
        &reg,
        &dseq,
        &RunOptions {
            parallel: false,
            write_snapshots: false,
        },
    )
    .unwrap();
    execute_run(
        &cfg,
        &reg,
        &dpar,
        &RunOptions {
            parallel: true,
            write_snapshots: false,
        },
    )
    .unwrap();

    let lseq = std::fs::read(dseq.join("events.ndjson")).unwrap();
    let lpar = std::fs::read(dpar.join("events.ndjson")).unwrap();
    assert_eq!(
        lseq, lpar,
        "DT-3: parallel phase execution changed the output"
    );

    let _ = std::fs::remove_dir_all(&dseq);
    let _ = std::fs::remove_dir_all(&dpar);
}

/// DT-4 — Fork with null intervention → identical to uninterrupted continuation.
#[test]
fn dt4_null_fork_equals_continuation() {
    let reg = standard_registry();
    let kernel = Kernel::new();
    let cfg = active_config(80);

    let (mut world, schedule) = prepare_run(&cfg, &reg).unwrap();
    run_kernel(&kernel, &mut world, &schedule, 30);
    let snap = kernel.snapshot(&world);

    let mut cont = kernel.restore(&snap);
    let cont_events = run_kernel(&kernel, &mut cont, &schedule, 50);

    let mut forked = kernel.fork(&snap, &Intervention::Null).unwrap();
    let fork_events = run_kernel(&kernel, &mut forked, &schedule, 50);

    assert_eq!(
        cont_events, fork_events,
        "DT-4: null fork diverged from continuation"
    );
    assert_eq!(cont, forked, "DT-4: null fork produced different state");
}

/// DT-5 — Snapshot → restore → continue equals an uninterrupted run.
#[test]
fn dt5_snapshot_restore_continue() {
    let reg = standard_registry();
    let kernel = Kernel::new();
    let cfg = active_config(100);

    let (mut straight, sched) = prepare_run(&cfg, &reg).unwrap();
    let straight_events = run_kernel(&kernel, &mut straight, &sched, 90);

    let (mut broken, sched2) = prepare_run(&cfg, &reg).unwrap();
    let mut broken_events = run_kernel(&kernel, &mut broken, &sched2, 37);
    let snap = kernel.snapshot(&broken);
    let mut resumed = kernel.restore(&snap);
    broken_events.extend(run_kernel(&kernel, &mut resumed, &sched2, 53));

    assert_eq!(
        straight_events, broken_events,
        "DT-5: checkpoint resume diverged"
    );
    assert_eq!(
        straight, resumed,
        "DT-5: resumed state differs from uninterrupted"
    );

    // …and the same guarantee via the `verify` path over a written run.
    let dir = scratch("dt5-verify");
    execute_run(&cfg, &reg, &dir, &RunOptions::default()).unwrap();
    let v = firma_cli::verify_run(&dir).unwrap();
    assert_eq!(
        v.snapshots_matched, v.snapshots_total,
        "DT-5: a stored snapshot did not match the re-executed state"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// DT-6 — Adding a randomness-consuming rule does not alter another rule's draws
/// (stream isolation, §21.2 property 4). **The most important test in the
/// suite** (§25.2): its failure produces plausible wrong numbers, not a crash.
#[test]
fn dt6_stream_isolation() {
    let reg = standard_registry();
    let kernel = Kernel::new();

    // baseline: transfer(0->1) + keyed A (2<->3)
    let base_json = r#"{
      "experiment": "dt6-base",
      "schema_version": "1.0.0",
      "engine": ">=0.1.0, <0.2.0",
      "seeds": { "mechanism": 31337, "environment": 1, "shock": 1, "init": 1 },
      "world": { "ticks": 200, "resources": ["capital"],
        "conflict_resolver": { "id": "conflict.additive", "version": "^1" } },
      "agents": [
        {"id":0,"stocks":{"capital":1000000}},{"id":1,"stocks":{"capital":1000000}},
        {"id":2,"stocks":{"capital":1000000}},{"id":3,"stocks":{"capital":1000000}},
        {"id":4,"stocks":{"capital":1000000}},{"id":5,"stocks":{"capital":1000000}}
      ],
      "rules": [
        { "id": "testkit.transfer", "version": "^1",
          "params": { "resource": "capital", "from": {"agent":0}, "to": {"agent":1}, "amount": 3 } },
        { "id": "testkit.keyed_nudge", "version": "^1",
          "params": { "resource": "capital", "from": 2, "to": 3, "modulus": 7, "purpose": "A" } }
      ]
    }"#;

    // extended: same, plus keyed B (4<->5) — an extra randomness consumer.
    let ext_json = base_json.replace(
        "\"purpose\": \"A\" } }\n      ]",
        "\"purpose\": \"A\" } },\n        { \"id\": \"testkit.keyed_nudge\", \"version\": \"^1\", \
         \"params\": { \"resource\": \"capital\", \"from\": 4, \"to\": 5, \"modulus\": 7, \
         \"purpose\": \"B\" } }\n      ]",
    );

    let (mut wb, sb) = prepare_run(&config(base_json), &reg).unwrap();
    let (mut we, se) = prepare_run(&config(&ext_json), &reg).unwrap();
    let eb = run_kernel(&kernel, &mut wb, &sb, 200);
    let ee = run_kernel(&kernel, &mut we, &se, 200);

    // The extra keyed rule must perturb neither the non-keyed transfer …
    assert_eq!(
        deltas_to_agents(&eb, &[0, 1]),
        deltas_to_agents(&ee, &[0, 1]),
        "DT-6: adding a keyed rule changed the transfer rule's deltas"
    );
    // … nor keyed rule A's own draw-driven deltas.
    assert_eq!(
        deltas_to_agents(&eb, &[2, 3]),
        deltas_to_agents(&ee, &[2, 3]),
        "DT-6: adding keyed rule B shifted keyed rule A's draws"
    );
    // Sanity: rule B actually did something (so the test is not vacuous).
    assert!(
        !deltas_to_agents(&ee, &[4, 5]).is_empty(),
        "DT-6: the added rule emitted nothing — fixture is vacuous"
    );
}

/// The tick number a phase-level event belongs to, or `None` for records that
/// are not per-tick phase events (`RunStarted`, `InterventionApplied`).
fn phase_event_tick(e: &Event) -> Option<u64> {
    match e {
        Event::TickStarted { tick }
        | Event::AgentBorn { tick, .. }
        | Event::DeltaApplied { tick, .. }
        | Event::PhaseCompleted { tick, .. }
        | Event::AgentDied { tick, .. }
        | Event::TickCompleted { tick, .. } => Some(*tick),
        Event::RunStarted { .. } | Event::InterventionApplied { .. } => None,
    }
}

fn per_agent_config(agents: &str) -> String {
    // `testkit.keyed_per_agent`: every live agent draws its own keyed value via
    // `open_for` (agent id in its proper key slot, §21.2). One phase-global
    // draw would not exercise this path — `testkit.keyed_nudge` does not.
    format!(
        r#"{{
          "experiment": "dt2b",
          "schema_version": "1.0.0",
          "engine": ">=0.1.0, <0.2.0",
          "seeds": {{ "mechanism": 424242, "environment": 1, "shock": 1, "init": 1 }},
          "world": {{ "ticks": 150, "resources": ["capital"],
            "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }} }},
          "agents": {agents},
          "rules": [
            {{ "id": "testkit.keyed_per_agent", "version": "^1",
               "params": {{ "resource": "capital", "modulus": 5, "purpose": "per_agent" }} }}
          ]
        }}"#
    )
}

/// DT-2b — per-agent keyed draws are independent of agent processing/storage
/// order (manual §21.2 property 1, "order invariance"). This is the case DT-2
/// does not cover: DT-2's keyed rule takes one phase-global draw and
/// config-literal agent ids, so shuffling storage cannot perturb it. Here the
/// rule's draw depends on *which agent* it is evaluating, through the real
/// kernel `rng_key()` → `open_for()` path.
#[test]
fn dt2b_per_agent_keyed_draw_order_irrelevant() {
    let reg = standard_registry();
    let ascending = r#"[
        {"id":10,"stocks":{"capital":1000000}},
        {"id":20,"stocks":{"capital":1000000}},
        {"id":30,"stocks":{"capital":1000000}},
        {"id":40,"stocks":{"capital":1000000}}
    ]"#;
    let shuffled = r#"[
        {"id":30,"stocks":{"capital":1000000}},
        {"id":10,"stocks":{"capital":1000000}},
        {"id":40,"stocks":{"capital":1000000}},
        {"id":20,"stocks":{"capital":1000000}}
    ]"#;

    let da = scratch("dt2b-a");
    let db = scratch("dt2b-b");
    execute_run(
        &config(&per_agent_config(ascending)),
        &reg,
        &da,
        &RunOptions::default(),
    )
    .unwrap();
    execute_run(
        &config(&per_agent_config(shuffled)),
        &reg,
        &db,
        &RunOptions::default(),
    )
    .unwrap();

    let la = std::fs::read(da.join("events.ndjson")).unwrap();
    let lb = std::fs::read(db.join("events.ndjson")).unwrap();
    assert_eq!(
        la, lb,
        "DT-2b: per-agent keyed draws depended on agent order (§21.2 property 1)"
    );

    // Non-vacuous: the per-agent rule must actually have drawn and moved
    // something. If `open_for` ignored the agent id, every agent's draw would be
    // equal, every ring net would be zero, and the rule would emit nothing.
    let events: Vec<Event> = firma_io::read_events::<Event>(&da.join("events.ndjson"))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let per_agent_deltas: Vec<i64> = events
        .iter()
        .filter_map(|e| match e {
            Event::DeltaApplied {
                origin,
                kind: firma_core::DeltaKind::AdjustStock { amount, .. },
                ..
            } if origin.as_str() == "testkit.keyed_per_agent" => Some(*amount),
            _ => None,
        })
        .collect();
    assert!(
        !per_agent_deltas.is_empty(),
        "DT-2b vacuous: testkit.keyed_per_agent emitted no deltas — per-agent keying may be a no-op"
    );
    let distinct: std::collections::BTreeSet<i64> = per_agent_deltas.iter().copied().collect();
    assert!(
        distinct.len() > 1,
        "DT-2b: every per-agent net was identical ({distinct:?}) — agents are not getting \
         distinct draws through the kernel (mirrors firma-rng property_2)"
    );

    let _ = std::fs::remove_dir_all(&da);
    let _ = std::fs::remove_dir_all(&db);
}

/// Event-log atomicity under a mid-phase invariant abort (manual §19.4 step 6,
/// §19.5: "Failure aborts; there is no repair-and-continue path"). A deliberate
/// non-negativity violation during a tick's `act_market` phase must leave the
/// on-disk `events.ndjson` with **zero** events for that tick — the failing
/// tick's records live only in the kernel's in-memory `Vec<Event>`, which the
/// orchestrator appends to the log only after `step()` returns `Ok`. It must
/// also leave no `manifest.json` / `run_id.txt`, so an aborted run is not a
/// verifiable run directory.
#[test]
fn tick_is_atomic_wrt_event_log_on_invariant_abort() {
    let reg = standard_registry();
    // agent 0 holds 2 capital and is drained 1/tick unconditionally -> at tick 2
    // its delta (-1, applied first by the §19.4 sort order) takes it to -1 and
    // aborts the phase before the other three deltas — and the whole tick —
    // reach the event log. A second force_adjust (agents 2/3) would succeed if
    // reached; it must not be.
    let cfg = config(
        r#"{
          "experiment": "abort-atomicity",
          "schema_version": "1.0.0",
          "engine": ">=0.1.0, <0.2.0",
          "seeds": { "mechanism": 1, "environment": 1, "shock": 1, "init": 1 },
          "world": { "ticks": 10, "resources": ["capital"],
            "conflict_resolver": { "id": "conflict.additive", "version": "^1" } },
          "agents": [
            {"id":0,"stocks":{"capital":2}},
            {"id":1,"stocks":{"capital":0}},
            {"id":2,"stocks":{"capital":100}},
            {"id":3,"stocks":{"capital":0}}
          ],
          "rules": [
            { "id": "testkit.force_adjust", "version": "^1",
              "params": { "resource": "capital", "agent": 0, "other": 1, "amount": 1 } },
            { "id": "testkit.force_adjust", "version": "^1",
              "params": { "resource": "capital", "agent": 2, "other": 3, "amount": 1 } }
          ]
        }"#,
    );

    let dir = scratch("abort-atomicity");
    let err = match execute_run(&cfg, &reg, &dir, &RunOptions::default()) {
        Ok(r) => panic!("expected an invariant abort, got a completed run: {r:?}"),
        Err(e) => e,
    };
    assert!(
        matches!(err, CliError::Kernel(KernelError::InvariantViolation(_))),
        "expected a non-negativity InvariantViolation, got {err:?}"
    );

    // An aborted run is not a verifiable run directory.
    assert!(
        !dir.join("manifest.json").exists(),
        "manifest.json written for an aborted run"
    );
    assert!(
        !dir.join("run_id.txt").exists(),
        "run_id.txt written for an aborted run"
    );

    let events: Vec<Event> = firma_io::read_events::<Event>(&dir.join("events.ndjson"))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    // Ticks 0 and 1 completed and are fully on disk.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::TickCompleted { tick: 1, .. })),
        "the last good tick (1) is missing from the log"
    );
    // Tick 2 — the aborted tick — contributed nothing to the log.
    let tick2 = events
        .iter()
        .filter(|e| phase_event_tick(e) == Some(2))
        .count();
    assert_eq!(
        tick2,
        0,
        "partial-tick events for the aborted tick reached events.ndjson (§19.5): {:?}",
        events
            .iter()
            .filter(|e| phase_event_tick(e) == Some(2))
            .collect::<Vec<_>>()
    );
    // Nothing past tick 2 either.
    assert!(
        events
            .iter()
            .all(|e| phase_event_tick(e).is_none_or(|t| t <= 1)),
        "the log extends past the last good tick"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
