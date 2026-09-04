//! Golden trace (manual §25.7): the full event-log hash of a fixed reference
//! run is committed here. Any change fails CI and MUST be acknowledged in the
//! PR with a MAJOR bump and a stated reason (§24.6). This makes an accidental
//! science-altering change impossible to merge unnoticed.

use firma_cli::{execute_run, standard_registry, RunOptions};
use firma_conformance::{config, scratch};

/// The reference configuration. Frozen — do not edit without a golden-trace
/// update.
const GOLDEN_CONFIG: &str = r#"{
  "experiment": "golden-phase1",
  "schema_version": "1.0.0",
  "engine": ">=0.1.0, <0.2.0",
  "seeds": { "mechanism": 20260903, "environment": 7, "shock": 11, "init": 13 },
  "world": {
    "ticks": 200,
    "resources": ["capital", "input"],
    "conflict_resolver": { "id": "conflict.additive", "version": "^1" },
    "snapshot_every": 50
  },
  "agents": [
    { "id": 0, "stocks": { "capital": 500000, "input": 500000 } },
    { "id": 1, "stocks": { "capital": 500000, "input": 500000 } },
    { "id": 2, "stocks": { "capital": 500000, "input": 500000 } }
  ],
  "environment": { "stocks": { "capital": 0, "input": 0 } },
  "rules": [
    { "id": "testkit.transfer", "version": "^1",
      "params": { "resource": "capital", "from": { "agent": 0 }, "to": { "agent": 1 }, "amount": 2 } },
    { "id": "testkit.transfer", "version": "^1",
      "params": { "resource": "input", "from": { "agent": 1 }, "to": { "agent": 2 }, "amount": 1,
                  "conflict_class": "resource_pool" } },
    { "id": "testkit.keyed_nudge", "version": "^1",
      "params": { "resource": "capital", "from": 1, "to": 0, "modulus": 6, "purpose": "golden" } }
  ]
}"#;

/// SHA-256 of the reference run's `events.ndjson`. Update deliberately only.
const GOLDEN_EVENT_LOG_SHA256: &str =
    "19f0d85d98c1d7f2090b7f12f3e17032bcf00b6ed8a49fb677e382f1e1602a87";

/// SHA-256 of the reference run's manifest identity (the run id). Update
/// deliberately only.
const GOLDEN_RUN_ID: &str = "f304edd4759ce26eef9212ba677424302cef869923d63228b0e2be423da5a099";

#[test]
fn golden_trace_unchanged() {
    let reg = standard_registry();
    let dir = scratch("golden");
    let report = execute_run(&config(GOLDEN_CONFIG), &reg, &dir, &RunOptions::default()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    // Set FIRMA_UPDATE_GOLDEN=1 to print the current values when a change is
    // intentional (manual §25.7 / §24.6).
    if std::env::var("FIRMA_UPDATE_GOLDEN").is_ok() {
        eprintln!("GOLDEN_EVENT_LOG_SHA256 = \"{}\"", report.event_log_sha256);
        eprintln!("GOLDEN_RUN_ID          = \"{}\"", report.run_id);
    }

    assert_eq!(
        report.event_log_sha256, GOLDEN_EVENT_LOG_SHA256,
        "golden trace changed. If intentional: MAJOR bump + stated reason (§25.7, §24.6), then \
         update GOLDEN_EVENT_LOG_SHA256 (run with FIRMA_UPDATE_GOLDEN=1 to print it)."
    );
    assert_eq!(
        report.run_id, GOLDEN_RUN_ID,
        "golden run id changed — see the message above."
    );
}
