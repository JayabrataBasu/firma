//! The run loop: config → schedule → kernel → event log, snapshots, manifest
//! (manual §22, §23.1 `runner`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use firma_config::{RunConfig, TimedIntervention};
use firma_core::{AgentId, Intervention, ResourceKind, Tick};
use firma_io::{
    BuildRecord, EventLog, ExecutionRecord, Manifest, PluginManifestEntry, RunIdentity,
    SnapshotStore,
};
use firma_kernel::{Event, Kernel, RunSeeds, Schedule, Snapshot, World};
use firma_registry::Registry;

use crate::error::CliError;
use crate::{engine_version, RNG_SCHEME};

/// Options for [`execute_run`].
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Execute each phase's rules on scoped threads (still deterministic — DT-3).
    pub parallel: bool,
    /// Write periodic snapshots (cadence from `config.world.snapshot_every`).
    pub write_snapshots: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        RunOptions {
            parallel: false,
            write_snapshots: true,
        }
    }
}

/// Outcome of [`execute_run`].
#[derive(Debug, Clone)]
pub struct RunReport {
    /// SHA-256 of the manifest identity (manual §22.3).
    pub run_id: String,
    /// Directory the run was written to.
    pub run_dir: PathBuf,
    /// Ticks executed.
    pub ticks: u64,
    /// Events written to the log.
    pub events_written: u64,
    /// SHA-256 of `events.ndjson` (the golden-trace value, §25.7).
    pub event_log_sha256: String,
    /// Live agents at the end.
    pub final_live_agents: u32,
    /// Per-resource conservation held for the whole run (it always must — the
    /// kernel would have aborted otherwise; recorded for the report).
    pub conservation_ok: bool,
}

/// Outcome of [`replay_run`].
#[derive(Debug, Clone)]
pub struct ReplayReport {
    /// The run id from the manifest.
    pub run_id: String,
    /// Whether the re-executed event log is byte-identical (DT-1).
    pub identical: bool,
}

/// Outcome of [`verify_run`].
#[derive(Debug, Clone)]
pub struct VerifyReport {
    /// The recomputed run id matches the stored `run_id.txt`.
    pub run_id_ok: bool,
    /// The re-executed event log is byte-identical to the stored one (DT-1).
    pub log_identical: bool,
    /// Number of stored snapshots that matched the re-executed state (DT-5).
    pub snapshots_matched: usize,
    /// Total stored snapshots checked.
    pub snapshots_total: usize,
    /// Conservation reconstructed from the event log alone holds (VT-6 / log
    /// sufficiency, §22.2).
    pub conservation_from_log_ok: bool,
}

impl VerifyReport {
    /// Whether every check passed.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.run_id_ok
            && self.log_identical
            && self.snapshots_matched == self.snapshots_total
            && self.conservation_from_log_ok
    }
}

// --------------------------------------------------------------------------

fn build_world(cfg: &RunConfig) -> World {
    let mut agents = BTreeMap::new();
    for a in &cfg.agents {
        agents.insert(
            AgentId(a.id),
            firma_kernel::AgentState {
                birth_tick: Tick(a.birth_tick),
                stocks: a.stocks.clone(),
            },
        );
    }
    let seeds = RunSeeds {
        mechanism: cfg.seeds.mechanism,
        environment: cfg.seeds.environment,
        shock: cfg.seeds.shock,
        init: cfg.seeds.init,
    };
    let mut world = World::new(
        cfg.world.resources.clone(),
        seeds,
        agents,
        cfg.environment.stocks.clone(),
    );
    // ADR 0030: seed global + per-agent domain-state values before tick 0.
    // BTreeMap order is deterministic; globals first, then per-agent ascending.
    for (k, v) in &cfg.world.global_reals {
        world.set_global_real(k.clone(), *v);
    }
    for (k, v) in &cfg.world.global_ints {
        world.set_global_int(k.clone(), *v);
    }
    for a in &cfg.agents {
        for (k, v) in &a.reals {
            world.set_agent_real(AgentId(a.id), k.clone(), *v);
        }
        for (k, v) in &a.ints {
            world.set_agent_int_value(AgentId(a.id), k.clone(), *v);
        }
    }
    world
}

fn resolve_rule_entry(
    registry: &Registry,
    spec: &firma_core::PluginRef,
) -> Result<(Box<dyn firma_core::Rule>, PluginManifestEntry), CliError> {
    let rule = registry.resolve_rule(spec)?;
    let hash = registry
        .rules()
        .find(|r| r.id == spec.id)
        .map(|r| r.content_hash.clone())
        .unwrap_or_default();
    let entry = PluginManifestEntry {
        id: rule.id().0.clone(),
        version: rule.version().to_string(),
        content_hash: hash,
        assumption: rule.assumption().to_string(),
    };
    Ok((rule, entry))
}

fn build_schedule(
    cfg: &RunConfig,
    registry: &Registry,
) -> Result<(Schedule, Vec<PluginManifestEntry>, PluginManifestEntry), CliError> {
    let mut rules: Vec<Box<dyn firma_core::Rule>> = Vec::new();
    let mut entries = Vec::new();
    for spec in &cfg.rules {
        let (rule, entry) = resolve_rule_entry(registry, spec)?;
        rules.push(rule);
        entries.push(entry);
    }

    let resolver = registry.resolve_resolver(&cfg.world.conflict_resolver)?;
    let rhash = registry
        .resolvers()
        .find(|r| r.id == cfg.world.conflict_resolver.id)
        .map(|r| r.content_hash.clone())
        .unwrap_or_default();
    let resolver_entry = PluginManifestEntry {
        id: resolver.id().0.clone(),
        version: resolver.version().to_string(),
        content_hash: rhash,
        assumption: resolver.assumption().to_string(),
    };

    Ok((Schedule::new(rules, resolver), entries, resolver_entry))
}

/// Resolve a config into a ready-to-run [`World`] and [`Schedule`] without
/// touching the filesystem. Used by the conformance suite to drive the kernel
/// directly for DT-4/DT-5/DT-6.
///
/// # Errors
/// [`CliError`] on config validation or plugin resolution failure.
pub fn prepare_run(cfg: &RunConfig, registry: &Registry) -> Result<(World, Schedule), CliError> {
    let mut cfg = cfg.clone();
    cfg.normalise();
    cfg.validate(&engine_version())?;
    let (schedule, _entries, _resolver) = build_schedule(&cfg, registry)?;
    Ok((build_world(&cfg), schedule))
}

fn build_identity(
    cfg: &RunConfig,
    rule_entries: Vec<PluginManifestEntry>,
    resolver_entry: PluginManifestEntry,
) -> Result<RunIdentity, CliError> {
    Ok(RunIdentity {
        manual_version: firma_core::MANUAL_VERSION.to_string(),
        engine_version: engine_version().to_string(),
        rng_scheme: RNG_SCHEME.to_string(),
        config_hash: cfg.content_hash()?,
        resolved_config: cfg.clone(),
        seeds: cfg.seeds,
        rules: rule_entries,
        conflict_resolver: resolver_entry,
        interventions: cfg.interventions.clone(),
        experiment_spec_hash: None,
    })
}

fn execution_record(duration_secs: Option<f64>) -> ExecutionRecord {
    ExecutionRecord {
        build: BuildRecord {
            rustc: "rustc 1.98.0 (pinned via rust-toolchain.toml)".to_string(),
            target: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
            lockfile_sha256: lockfile_sha256(),
        },
        host_platform: format!(
            "{} {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::env::consts::FAMILY
        ),
        engine_git_sha: None,
        container_digest: None,
        duration_secs,
        provenance_note: "Phase 1: rustc string is the pinned toolchain, not a captured \
            `rustc -vV`; git sha and container digest unavailable in this environment \
            (PROGRESS.md OQ-4). None of these affect the trajectory or the run id."
            .to_string(),
    }
}

/// Best-effort SHA-256 of the workspace `Cargo.lock`, embedded at build time.
fn lockfile_sha256() -> Option<String> {
    // `CARGO_MANIFEST_DIR` for this crate is `<ws>/crates/firma-cli`.
    const LOCK: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock"));
    Some(firma_io::sha256_hex(LOCK.as_bytes()))
}

fn apply_interventions_at(
    kernel: &Kernel,
    world: &mut World,
    schedule: &mut Option<Schedule>,
    registry: &Registry,
    interventions: &[TimedIntervention],
    tick: u64,
    log: &mut EventLog<std::fs::File>,
) -> Result<(), CliError> {
    for iv in interventions.iter().filter(|i| i.at == tick) {
        match &iv.intervention {
            Intervention::AddRule(spec) => {
                let (rule, _entry) = resolve_rule_entry(registry, spec)?;
                let (mut rules, resolver) = schedule.take().expect("schedule present").into_parts();
                rules.push(rule);
                *schedule = Some(Schedule::new(rules, resolver));
            }
            Intervention::RemoveRule(id) => {
                let (rules, resolver) = schedule.take().expect("schedule present").into_parts();
                let kept: Vec<_> = rules.into_iter().filter(|r| &r.id() != id).collect();
                *schedule = Some(Schedule::new(kept, resolver));
            }
            other => {
                kernel.apply_intervention(world, other)?;
            }
        }
        log.append(&Event::InterventionApplied {
            tick,
            op: describe_intervention(&iv.intervention),
        })?;
    }
    Ok(())
}

fn describe_intervention(iv: &Intervention) -> String {
    match iv {
        Intervention::Null => "null".to_string(),
        Intervention::SetStock {
            agent,
            resource,
            value,
        } => {
            format!("set_stock(agent {}, {resource}, {value})", agent.0)
        }
        Intervention::AddRule(spec) => format!("add_rule({})", spec.id),
        Intervention::RemoveRule(id) => format!("remove_rule({id})"),
        Intervention::FreezeRule(id) => format!("freeze_rule({id})"),
        Intervention::RemoveAgent(a) => format!("remove_agent({})", a.0),
        Intervention::SetAgentReal {
            agent,
            field,
            value,
        } => {
            format!("set_agent_real(agent {}, {field}, {value})", agent.0)
        }
    }
}

// --------------------------------------------------------------------------

/// Execute a run and write `events.ndjson`, `snapshots/`, `manifest.json`, and
/// `run_id.txt` into `run_dir` (manual §22).
///
/// # Errors
/// [`CliError`] on config, plugin, kernel, or IO failure.
pub fn execute_run(
    cfg: &RunConfig,
    registry: &Registry,
    run_dir: &Path,
    opts: &RunOptions,
) -> Result<RunReport, CliError> {
    let mut cfg = cfg.clone();
    cfg.normalise();
    cfg.validate(&engine_version())?;

    std::fs::create_dir_all(run_dir).map_err(|e| CliError::Io(e.into()))?;

    let (schedule, rule_entries, resolver_entry) = build_schedule(&cfg, registry)?;
    let mut schedule = Some(schedule);

    let identity = build_identity(&cfg, rule_entries, resolver_entry)?;
    let run_id = identity.run_id().map_err(CliError::Io)?;

    let kernel = if opts.parallel {
        Kernel::parallel()
    } else {
        Kernel::new()
    };
    let mut world = build_world(&cfg);

    let snapshots = if opts.write_snapshots {
        Some(SnapshotStore::open(&run_dir.join("snapshots")).map_err(CliError::Io)?)
    } else {
        None
    };

    let events_path = run_dir.join("events.ndjson");
    let mut log = EventLog::create(&events_path).map_err(CliError::Io)?;
    log.append(&Event::RunStarted {
        manual_version: firma_core::MANUAL_VERSION.to_string(),
        engine_version: engine_version().to_string(),
        horizon: cfg.world.ticks,
    })?;

    let start = std::time::Instant::now();
    let k = cfg.world.snapshot_every.max(1);
    let mut buf: Vec<Event> = Vec::new();

    for tick in 0..cfg.world.ticks {
        apply_interventions_at(
            &kernel,
            &mut world,
            &mut schedule,
            registry,
            &cfg.interventions,
            tick,
            &mut log,
        )?;

        if let Some(store) = &snapshots {
            if tick % k == 0 {
                store
                    .put(tick, &kernel.snapshot(&world))
                    .map_err(CliError::Io)?;
            }
        }

        buf.clear();
        kernel.step(&mut world, schedule.as_ref().expect("schedule"), &mut buf)?;
        for ev in &buf {
            log.append(ev)?;
        }
        // Flush every tick (not just at the end) so a live tailer — `firma-tui`
        // (§23.2), or `tail -f` — sees progress during an unattended sweep,
        // not just after it finishes. Bytes written are identical either way
        // (DT-1 unaffected); this only changes *when* they reach disk.
        log.flush()?;
    }

    // Final snapshot at the horizon.
    if let Some(store) = &snapshots {
        store
            .put(cfg.world.ticks, &kernel.snapshot(&world))
            .map_err(CliError::Io)?;
    }
    log.flush()?;
    let events_written = log.len();
    drop(log);

    let duration = start.elapsed().as_secs_f64();
    let manifest = Manifest {
        identity,
        execution: execution_record(Some(duration)),
    };
    manifest.write(run_dir).map_err(CliError::Io)?;
    std::fs::write(run_dir.join("run_id.txt"), &run_id).map_err(|e| CliError::Io(e.into()))?;

    let event_bytes = std::fs::read(&events_path).map_err(|e| CliError::Io(e.into()))?;
    let conservation_ok = cfg
        .world
        .resources
        .iter()
        .all(|r| world.live_total(r) == world.initial_total(r));

    Ok(RunReport {
        run_id,
        run_dir: run_dir.to_path_buf(),
        ticks: cfg.world.ticks,
        events_written,
        event_log_sha256: firma_io::sha256_hex(&event_bytes),
        final_live_agents: u32::try_from(world.live_agents().len()).unwrap_or(u32::MAX),
        conservation_ok,
    })
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch_dir(tag: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("firma-{tag}-{}-{n}", std::process::id()))
}

/// Re-execute the run described by `run_dir/manifest.json` and check the event
/// log is byte-identical to `run_dir/events.ndjson` (the DT-1 check, §25.2).
///
/// # Errors
/// [`CliError`] on IO failure or if the re-execution itself errors.
pub fn replay_run(run_dir: &Path) -> Result<ReplayReport, CliError> {
    let manifest = Manifest::read(run_dir).map_err(CliError::Io)?;
    let registry = crate::standard_registry();
    let scratch = scratch_dir("replay");
    let report = execute_run(
        &manifest.identity.resolved_config,
        &registry,
        &scratch,
        &RunOptions {
            parallel: false,
            write_snapshots: false,
        },
    )?;

    let original =
        std::fs::read(run_dir.join("events.ndjson")).map_err(|e| CliError::Io(e.into()))?;
    let replayed =
        std::fs::read(scratch.join("events.ndjson")).map_err(|e| CliError::Io(e.into()))?;
    let _ = std::fs::remove_dir_all(&scratch);

    Ok(ReplayReport {
        run_id: report.run_id,
        identical: original == replayed,
    })
}

/// Full verification of a run directory (manual §18.2 firma-io tests):
/// recompute the run id, replay for byte-identity (DT-1), match every stored
/// snapshot against the re-executed state (DT-5), and reconstruct conservation
/// from the event log alone (§22.2 log sufficiency, VT-6).
///
/// # Errors
/// [`CliError`] on IO failure or a re-execution error. A *failed check* is
/// reported in the [`VerifyReport`], not as an error.
pub fn verify_run(run_dir: &Path) -> Result<VerifyReport, CliError> {
    let manifest = Manifest::read(run_dir).map_err(CliError::Io)?;
    let recomputed = manifest.identity.run_id().map_err(CliError::Io)?;
    let stored_id =
        std::fs::read_to_string(run_dir.join("run_id.txt")).map_err(|e| CliError::Io(e.into()))?;
    let run_id_ok = recomputed.trim() == stored_id.trim();

    let registry = crate::standard_registry();
    let scratch = scratch_dir("verify");
    execute_run(
        &manifest.identity.resolved_config,
        &registry,
        &scratch,
        &RunOptions {
            parallel: false,
            write_snapshots: true,
        },
    )?;

    let original =
        std::fs::read(run_dir.join("events.ndjson")).map_err(|e| CliError::Io(e.into()))?;
    let replayed =
        std::fs::read(scratch.join("events.ndjson")).map_err(|e| CliError::Io(e.into()))?;
    let log_identical = original == replayed;

    // Snapshot cross-check.
    let orig_store = SnapshotStore::open(&run_dir.join("snapshots")).map_err(CliError::Io)?;
    let new_store = SnapshotStore::open(&scratch.join("snapshots")).map_err(CliError::Io)?;
    let ticks = orig_store.ticks().map_err(CliError::Io)?;
    let mut matched = 0usize;
    for t in &ticks {
        let a: Snapshot = orig_store.get(*t).map_err(CliError::Io)?;
        let b: Snapshot = new_store.get(*t).map_err(CliError::Io)?;
        let (mut a, mut b) = (a, b);
        a.world.rebuild_live();
        b.world.rebuild_live();
        if a == b {
            matched += 1;
        }
    }

    // Conservation from the event log alone.
    let conservation_from_log_ok =
        conservation_from_log(&manifest.identity.resolved_config, run_dir)?;

    let _ = std::fs::remove_dir_all(&scratch);

    Ok(VerifyReport {
        run_id_ok,
        log_identical,
        snapshots_matched: matched,
        snapshots_total: ticks.len(),
        conservation_from_log_ok,
    })
}

/// Fold the `DeltaApplied` events over the config's opening balances and check
/// every resource's total is unchanged (§22.2: the log MUST be sufficient to
/// reconstruct any needed quantity — here, conservation).
fn conservation_from_log(cfg: &RunConfig, run_dir: &Path) -> Result<bool, CliError> {
    let mut totals: BTreeMap<ResourceKind, i64> = BTreeMap::new();
    for r in &cfg.world.resources {
        totals.insert(r.clone(), 0);
    }
    for a in &cfg.agents {
        for (r, q) in &a.stocks {
            *totals.entry(r.clone()).or_insert(0) += q;
        }
    }
    for (r, q) in &cfg.environment.stocks {
        *totals.entry(r.clone()).or_insert(0) += q;
    }
    let initial = totals.clone();

    let events =
        firma_io::read_events::<Event>(&run_dir.join("events.ndjson")).map_err(CliError::Io)?;
    for ev in events {
        let ev = ev.map_err(CliError::Io)?;
        if let Event::DeltaApplied {
            kind: firma_core::DeltaKind::AdjustStock { resource, amount },
            ..
        } = ev
        {
            *totals.entry(resource).or_insert(0) += amount;
        }
    }
    Ok(totals == initial)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smoke_config(ticks: u64) -> RunConfig {
        let json = format!(
            r#"{{
              "experiment": "orch-smoke",
              "schema_version": "1.0.0",
              "engine": ">=0.1.0, <0.2.0",
              "seeds": {{"mechanism": 11, "environment": 22, "shock": 33, "init": 44}},
              "world": {{
                "ticks": {ticks},
                "resources": ["capital"],
                "conflict_resolver": {{"id": "conflict.additive", "version": "^1"}},
                "snapshot_every": 25
              }},
              "agents": [
                {{"id": 0, "stocks": {{"capital": 100000}}}},
                {{"id": 1, "stocks": {{"capital": 100000}}}}
              ],
              "rules": [
                {{"id": "testkit.transfer", "version": "^1",
                 "params": {{"resource": "capital", "from": {{"agent": 0}}, "to": {{"agent": 1}}, "amount": 1}}}},
                {{"id": "testkit.keyed_nudge", "version": "^1",
                 "params": {{"resource": "capital", "from": 1, "to": 0, "modulus": 3, "purpose": "nudge"}}}}
              ]
            }}"#
        );
        RunConfig::from_json(&json).unwrap()
    }

    #[test]
    fn run_then_replay_is_byte_identical() {
        let cfg = smoke_config(200);
        let reg = crate::standard_registry();
        let dir = scratch_dir("orch-test");
        let report = execute_run(&cfg, &reg, &dir, &RunOptions::default()).unwrap();
        assert!(report.conservation_ok);
        assert_eq!(report.ticks, 200);

        let replay = replay_run(&dir).unwrap();
        assert!(replay.identical, "DT-1: replayed log differs");

        let verify = verify_run(&dir).unwrap();
        assert!(verify.ok(), "verify failed: {verify:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
