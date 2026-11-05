//! Stage 7 Part D cross-check tool: prints the SC-1…SC-6 sanity report and
//! every firm's final reconstructed state as one JSON object, so a Python
//! test can compare `firma_lab.metrics`'s numbers against this crate's own
//! output for the same run — the same "two paths, one formula,
//! byte-identical" standard ADR-0026 set for the constraint/decision-plugin
//! agreement.
//!
//! Not a shipped CLI surface — `firma-cli` is unaffected; this is a
//! verification tool only.
//!
//! Usage: `sanity_report_json <config.json path> <run_dir>`

use std::path::PathBuf;

use firma_analysis::sanity::{reconstruct_checked, try_sanity_from_run};
use firma_config::RunConfig;
use serde_json::json;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(config_path), Some(run_dir)) = (args.first(), args.get(1)) else {
        eprintln!("usage: sanity_report_json <config.json> <run_dir>");
        std::process::exit(2);
    };

    let cfg_json = std::fs::read_to_string(config_path).expect("read config");
    let cfg = RunConfig::from_json(&cfg_json).expect("parse config");
    let run_dir = PathBuf::from(run_dir);

    // A malformed `scales` parameter or an unparsable persisted record in the
    // log is an error with a non-zero exit, never a report computed without
    // the offending records.
    let fail = |e: String| -> ! {
        eprintln!("sanity_report_json: {e}");
        std::process::exit(1);
    };
    let report = try_sanity_from_run(&cfg, &run_dir).unwrap_or_else(|e| fail(e));

    let events: Vec<firma_core::Event> =
        firma_io::read_events::<firma_core::Event>(&run_dir.join("events.ndjson"))
            .expect("read events")
            .map(|e| e.expect("parse event"))
            .collect();
    let recon = reconstruct_checked(&cfg, &events).unwrap_or_else(|e| fail(e));

    let out = json!({
        "sanity": report,
        "firms": recon.firms(),
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
