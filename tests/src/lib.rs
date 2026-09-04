//! Shared harness for the FIRMA conformance suite (manual §25).
//!
//! Maps to the `tests/{determinism,validation,integration}/` layout of §38 as a
//! workspace member package with one test target per concern (PROGRESS.md
//! OQ-6). Helpers here build configs and drive full runs through
//! `firma_cli`'s orchestrator so the tests exercise the whole stack.

#![forbid(unsafe_code)]

pub mod replay;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use firma_config::RunConfig;
use firma_kernel::{Event, Kernel, Schedule, World};

/// A unique scratch directory for a test artefact.
#[must_use]
pub fn scratch(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("firma-conf-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Build a [`RunConfig`] from a JSON body, panicking on parse failure.
#[must_use]
pub fn config(json: &str) -> RunConfig {
    RunConfig::from_json(json).expect("test config parses")
}

/// Run `ticks` steps of the kernel over `world`/`schedule`, collecting every
/// event. Panics on any kernel error (an invariant violation aborts the run —
/// §19.5 — and the test should fail loudly).
pub fn run_kernel(
    kernel: &Kernel,
    world: &mut World,
    schedule: &Schedule,
    ticks: u64,
) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        kernel
            .step(world, schedule, &mut events)
            .expect("kernel step must not error in a determinism fixture");
    }
    events
}

/// `DeltaApplied` events whose target is one of `agents` — used to isolate one
/// rule's effect for DT-6.
#[must_use]
pub fn deltas_to_agents(events: &[Event], agents: &[u64]) -> Vec<Event> {
    events
        .iter()
        .filter(|e| match e {
            Event::DeltaApplied {
                target: firma_core::DeltaTarget::Agent(a),
                ..
            } => agents.contains(&a.0),
            _ => false,
        })
        .cloned()
        .collect()
}

/// A config with `n` well-funded agents and a small set of always-active
/// testkit rules, parameterised by tick count. Every constant here is a test
/// fixture value, not a model parameter.
#[must_use]
pub fn active_config(ticks: u64) -> RunConfig {
    let json = format!(
        r#"{{
          "experiment": "conformance-active",
          "schema_version": "1.0.0",
          "engine": ">=0.1.0, <0.2.0",
          "seeds": {{ "mechanism": 777, "environment": 12, "shock": 5, "init": 9 }},
          "world": {{
            "ticks": {ticks},
            "resources": ["capital", "input"],
            "conflict_resolver": {{ "id": "conflict.additive", "version": "^1" }},
            "snapshot_every": 50
          }},
          "agents": [
            {{ "id": 0, "stocks": {{ "capital": 1000000, "input": 1000000 }} }},
            {{ "id": 1, "stocks": {{ "capital": 1000000, "input": 1000000 }} }},
            {{ "id": 2, "stocks": {{ "capital": 1000000, "input": 1000000 }} }},
            {{ "id": 3, "stocks": {{ "capital": 1000000, "input": 1000000 }} }}
          ],
          "environment": {{ "stocks": {{ "capital": 0, "input": 0 }} }},
          "rules": [
            {{ "id": "testkit.transfer", "version": "^1",
               "params": {{ "resource": "capital", "from": {{ "agent": 0 }}, "to": {{ "agent": 1 }}, "amount": 2 }} }},
            {{ "id": "testkit.transfer", "version": "^1",
               "params": {{ "resource": "input", "from": {{ "agent": 2 }}, "to": {{ "agent": 3 }}, "amount": 1,
                            "conflict_class": "resource_pool" }} }},
            {{ "id": "testkit.keyed_nudge", "version": "^1",
               "params": {{ "resource": "capital", "from": 1, "to": 2, "modulus": 5, "purpose": "nudge" }} }}
          ]
        }}"#
    );
    config(&json)
}
