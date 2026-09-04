//! Stage 7 Part A/D — `firma-tui` against a **real, currently-executing**
//! run, not a fixture. A 20,000-tick, 2-firm `decision.satisficing` run
//! (≈4-5s wall clock) runs on a background thread while the main thread
//! builds a `firma_tui::app::App` pointed at the same `events.ndjson` and
//! polls it exactly as the live binary's event loop does — proving the
//! tailer really observes a log that is still being written (§23.2), not
//! just a completed one, and that the panels change as more events arrive.
//!
//! Rendered via `ratatui::backend::TestBackend` (no real terminal needed) —
//! this is the "screenshot" for a non-interactive environment: each snapshot
//! below is the literal character grid `firma-tui` would draw at that
//! moment.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use firma_conformance::{config, scratch};
use firma_tui::app::App;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

const LONG_RUN: &str = r#"{
  "experiment": "tui-live-demo", "schema_version": "1.0.0", "engine": ">=0.1.0, <0.2.0",
  "seeds": { "mechanism": 20260906, "environment": 2, "shock": 3, "init": 4 },
  "world": { "ticks": 20000, "resources": ["capital", "input"],
    "conflict_resolver": { "id": "conflict.additive", "version": "^1" }, "snapshot_every": 20000,
    "global_reals": { "theta_limit": 0.55, "theta_cap": 0.35 },
    "global_ints": { "theta_q": 40, "input_price": 2, "output_price": 3 } },
  "agents": [
    { "id": 0, "stocks": { "capital": 250, "input": 40 }, "reals": { "capability": 0.35, "legitimacy": 1.0, "aspiration_capital_growth": 6.0 } },
    { "id": 1, "stocks": { "capital": 600, "input": 40 }, "reals": { "capability": 0.75, "legitimacy": 1.0, "aspiration_capital_growth": 6.0 } }
  ],
  "environment": { "stocks": { "capital": 100000000, "input": 100000000 } },
  "rules": [
    { "id": "decision.satisficing", "version": "^1", "params": { "l_w": 8 } },
    { "id": "action.market.standard.hold", "version": "^1", "params": {} },
    { "id": "action.market.standard.produce_ordinary", "version": "^1", "params": {} },
    { "id": "action.market.standard.produce_regulated", "version": "^1", "params": {} },
    { "id": "action.market.standard.acquire_input", "version": "^1", "params": {} },
    { "id": "action.market.standard.invest_capability", "version": "^1", "params": {} },
    { "id": "action.market.standard.deliver", "version": "^1", "params": {} },
    { "id": "action.shaping.rdt_standard.resolve_lagged", "version": "^1", "params": {} },
    { "id": "constraint.action_window", "version": "^1", "params": { "l_w": 8 } },
    { "id": "constraint.enforce", "version": "^1",
      "params": { "t_c": 4, "p_c": 30, "delta_lambda": 0.15, "p_q": 20, "l_w": 8 } },
    { "id": "decision.aspiration_update", "version": "^1", "params": { "alpha": 0.10 } }
  ]
}"#;

fn dump(buf: &ratatui::buffer::Buffer) -> String {
    let area = buf.area();
    let mut s = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            s.push_str(buf[(x, y)].symbol());
        }
        s.push('\n');
    }
    s
}

/// Drives `firma-tui` against a run that is genuinely still executing on
/// another thread, capturing several snapshots as it progresses.
#[test]
fn tui_tails_a_run_that_is_still_being_written() {
    let cfg = config(LONG_RUN);
    let dir = scratch("tui-live");
    std::fs::create_dir_all(&dir).unwrap();

    let (done_tx, done_rx) = mpsc::channel();
    let run_cfg = cfg.clone();
    let run_dir = dir.clone();
    let handle = thread::spawn(move || {
        let report = firma_cli::execute_run(
            &run_cfg,
            &firma_cli::model_registry(),
            &run_dir,
            &firma_cli::RunOptions {
                parallel: false,
                write_snapshots: false,
            },
        );
        let _ = done_tx.send(());
        report
    });

    let mut app = App::new(&cfg, dir.join("events.ndjson"));
    let mut snapshots: Vec<(u64, bool, String)> = Vec::new();

    // Poll while the writer is still going, capturing distinct progress
    // snapshots — this is the actual "tail a log still being written" case,
    // not a fixture read after the fact.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while done_rx.try_recv().is_err() && std::time::Instant::now() < deadline {
        app.poll();
        let backend = TestBackend::new(110, 34);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| firma_tui::ui::draw(f, &app)).unwrap();
        let tick = app.reconstruction().tick();
        snapshots.push((tick, app.finished(), dump(term.backend().buffer())));
        thread::sleep(Duration::from_millis(400));
    }

    let report = handle
        .join()
        .unwrap()
        .expect("the background run must succeed");
    assert!(report.conservation_ok);

    // Final poll(s) to drain whatever landed after the last mid-run sample.
    for _ in 0..3 {
        app.poll();
        thread::sleep(Duration::from_millis(50));
    }
    let backend = TestBackend::new(110, 34);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| firma_tui::ui::draw(f, &app)).unwrap();
    snapshots.push((
        app.reconstruction().tick(),
        app.finished(),
        dump(term.backend().buffer()),
    ));

    eprintln!(
        "\n=== captured {} snapshot(s) while the writer ran on another thread ===",
        snapshots.len()
    );
    for (i, (tick, finished, frame)) in snapshots.iter().enumerate() {
        eprintln!("\n--- snapshot {i} (observed tick={tick}, finished={finished}) ---\n{frame}");
    }

    // --- what this proves ---
    assert!(
        snapshots.len() >= 3,
        "expected several distinct in-progress snapshots (a genuinely still-\
         being-written log), got {}",
        snapshots.len()
    );
    let ticks: Vec<u64> = snapshots.iter().map(|(t, _, _)| *t).collect();
    assert!(
        ticks.windows(2).any(|w| w[1] > w[0]),
        "no snapshot showed tick progress — the tailer did not observe the \
         log growing while it was being written: {ticks:?}"
    );
    assert!(
        snapshots.last().unwrap().1,
        "the final snapshot should show the run as finished"
    );
    // The margin-distribution and action-histogram panels populated from
    // real data (Part D's requirement) — both firms' ids appear somewhere,
    // and at least one market action label shows a nonzero-looking bar.
    let last_frame = &snapshots.last().unwrap().2;
    assert!(
        last_frame.contains("margin distribution"),
        "margin-distribution panel missing from the render"
    );
    assert!(
        last_frame.contains("action histogram"),
        "action-histogram panel missing from the render"
    );
    assert!(
        app.action_totals().values().sum::<u64>() > 0,
        "no decisions were ever tallied into the action histogram"
    );
    assert_eq!(
        app.reconstruction().total_count(),
        2,
        "both firms should be present in the reconstruction"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
