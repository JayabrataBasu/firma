//! `firma-tui` — the live monitor binary (manual §23.2). Hand-rolled
//! argument parsing (ADR 0010's precedent — `firma-cli`'s `main.rs`).

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use crossterm::event::{self, Event as CtEvent, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use firma_config::RunConfig;
use firma_tui::app::App;
use firma_tui::ui::draw;

const USAGE: &str = "\
firma-tui \u{2014} live event-log monitor (manual \u{00a7}23.2)

USAGE:
    firma-tui --config <FILE> --run <DIR> [--poll-ms <N>]

FLAGS:
    --config <FILE>   The RunConfig JSON the watched run was launched with
                       (the same file passed to `firma run --config`).
    --run <DIR>       The run directory being written (reads <DIR>/events.ndjson).
    --poll-ms <N>     Tail poll interval in milliseconds (default 200).

Reads the event log by tailing. MUST NOT link firma-kernel or influence the
run (\u{00a7}23.2) \u{2014} this binary only reads. Press 'q' to quit.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config_path: Option<PathBuf> = None;
    let mut run_dir: Option<PathBuf> = None;
    let mut poll_ms: u64 = 200;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                config_path = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--run" => {
                run_dir = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--poll-ms" => {
                poll_ms = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(200);
                i += 2;
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument: {other}\n{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }
    let (Some(config_path), Some(run_dir)) = (config_path, run_dir) else {
        eprint!("{USAGE}");
        return ExitCode::FAILURE;
    };

    let cfg_json = match std::fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {e}", config_path.display());
            return ExitCode::FAILURE;
        }
    };
    let cfg = match RunConfig::from_json(&cfg_json) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("invalid config: {e}");
            return ExitCode::FAILURE;
        }
    };

    let events_path = run_dir.join("events.ndjson");
    let mut app = App::new(&cfg, events_path);

    if let Err(e) = run_tui(&mut app, Duration::from_millis(poll_ms)) {
        eprintln!("terminal error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run_tui(app: &mut App, poll_interval: Duration) -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = (|| -> std::io::Result<()> {
        loop {
            app.poll();
            terminal.draw(|f| draw(f, app))?;
            if event::poll(poll_interval)? {
                if let CtEvent::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('q') {
                        app.quit();
                    }
                }
            }
            if app.should_quit() {
                break;
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    result
}
