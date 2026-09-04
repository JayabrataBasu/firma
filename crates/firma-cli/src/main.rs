//! `firma` — the command-line entry point (manual §26.3: `run`, `replay`,
//! `verify`). Argument parsing is hand-rolled (ADR 0010).

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use firma_cli::{
    execute_run, model_registry, replay_run, standard_registry, verify_run, CliError, RunOptions,
};
use firma_config::RunConfig;

const USAGE: &str = "\
firma — FIRMA engine CLI (Phase 1 kernel)

USAGE:
    firma run    --config <FILE> --out <DIR> [--parallel] [--no-snapshots] [--model]
    firma replay --run <DIR>
    firma verify --run <DIR>

SUBCOMMANDS:
    run     Execute a run: writes events.ndjson, snapshots/, manifest.json, run_id.txt
    replay  Re-execute from a run's manifest and confirm the event log is byte-identical (DT-1)
    verify  replay + recompute the run id + match every stored snapshot (DT-1, DT-5) + log-sufficiency

FLAGS:
    --model  Resolve rules against the full Phase-2 model registry (decision.*,
             action.*, constraint.action_window/enforce) instead of the Phase-1
             testkit registry (PROGRESS.md OQ-7).
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("firma: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), CliError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(sub) = args.first() else {
        print!("{USAGE}");
        return Ok(());
    };

    match sub.as_str() {
        "-h" | "--help" | "help" => {
            print!("{USAGE}");
            Ok(())
        }
        "run" => cmd_run(&args[1..]),
        "replay" => cmd_replay(&args[1..]),
        "verify" => cmd_verify(&args[1..]),
        other => Err(CliError::Usage(format!(
            "unknown subcommand {other:?}\n\n{USAGE}"
        ))),
    }
}

/// Minimal `--flag value` / `--bool-flag` parser.
struct Args {
    values: std::collections::BTreeMap<String, String>,
    flags: std::collections::BTreeSet<String>,
}

impl Args {
    fn parse(raw: &[String], bool_flags: &[&str]) -> Result<Args, CliError> {
        let mut values = std::collections::BTreeMap::new();
        let mut flags = std::collections::BTreeSet::new();
        let mut i = 0;
        while i < raw.len() {
            let a = &raw[i];
            let Some(name) = a.strip_prefix("--") else {
                return Err(CliError::Usage(format!("unexpected argument {a:?}")));
            };
            if bool_flags.contains(&name) {
                flags.insert(name.to_string());
                i += 1;
            } else {
                let v = raw
                    .get(i + 1)
                    .ok_or_else(|| CliError::Usage(format!("--{name} needs a value")))?;
                values.insert(name.to_string(), v.clone());
                i += 2;
            }
        }
        Ok(Args { values, flags })
    }

    fn required(&self, name: &str) -> Result<&str, CliError> {
        self.values
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| CliError::Usage(format!("--{name} is required")))
    }

    fn has(&self, name: &str) -> bool {
        self.flags.contains(name)
    }
}

fn cmd_run(raw: &[String]) -> Result<(), CliError> {
    let args = Args::parse(raw, &["parallel", "no-snapshots", "model"])?;
    let config_path = PathBuf::from(args.required("config")?);
    let out = PathBuf::from(args.required("out")?);

    let cfg = RunConfig::load(&config_path)?;
    let registry = if args.has("model") {
        model_registry()
    } else {
        standard_registry()
    };
    let opts = RunOptions {
        parallel: args.has("parallel"),
        write_snapshots: !args.has("no-snapshots"),
    };
    let report = execute_run(&cfg, &registry, &out, &opts)?;

    println!("run_id          {}", report.run_id);
    println!("run_dir         {}", report.run_dir.display());
    println!("ticks           {}", report.ticks);
    println!("events          {}", report.events_written);
    println!("event_log_sha256 {}", report.event_log_sha256);
    println!("final_live      {}", report.final_live_agents);
    println!("conservation_ok {}", report.conservation_ok);
    Ok(())
}

fn cmd_replay(raw: &[String]) -> Result<(), CliError> {
    let args = Args::parse(raw, &[])?;
    let dir = PathBuf::from(args.required("run")?);
    let report = replay_run(&dir)?;
    println!("run_id     {}", report.run_id);
    println!("identical  {}", report.identical);
    if report.identical {
        Ok(())
    } else {
        Err(CliError::Mismatch(
            "replayed event log is not byte-identical (DT-1 failure)".into(),
        ))
    }
}

fn cmd_verify(raw: &[String]) -> Result<(), CliError> {
    let args = Args::parse(raw, &[])?;
    let dir = PathBuf::from(args.required("run")?);
    let r = verify_run(&dir)?;
    println!("run_id_ok               {}", r.run_id_ok);
    println!("log_identical           {}", r.log_identical);
    println!(
        "snapshots_matched       {}/{}",
        r.snapshots_matched, r.snapshots_total
    );
    println!("conservation_from_log   {}", r.conservation_from_log_ok);
    if r.ok() {
        println!("VERIFIED");
        Ok(())
    } else {
        Err(CliError::Mismatch("verification failed".into()))
    }
}
