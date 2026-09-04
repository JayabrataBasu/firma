//! Panel layout and rendering (manual §23.2's eight panels). Pure function
//! of [`App`] state to a ratatui [`Frame`] — no IO, no polling, so it can be
//! driven identically from the live event loop or a `TestBackend` snapshot
//! (see `tests/tests/tui_live_demo.rs`).

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Borders, Gauge, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Draw every panel for the current [`App`] state.
pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header: tick progress/ETA + throughput + invariant status
            Constraint::Min(8),    // agents / margin distribution / action histogram
            Constraint::Length(9), // shock timeline / log tail
        ])
        .split(frame.area());

    draw_header(frame, root[0], app);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(37),
            Constraint::Percentage(38),
        ])
        .split(root[1]);
    draw_agents(frame, mid[0], app);
    draw_margin_distribution(frame, mid[1], app);
    draw_action_histogram(frame, mid[2], app);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(root[2]);
    draw_shock_timeline(frame, bottom[0], app);
    draw_log_tail(frame, bottom[1], app);
}

/// Panel 1/2: tick progress + ETA. Panel 7: throughput. Panel 6: invariant
/// status. Combined into one header strip — all three are short facts.
fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(25),
            Constraint::Percentage(30),
        ])
        .split(area);

    let tick = app.reconstruction().tick();
    let horizon = app.horizon();
    let ratio = match horizon {
        Some(h) if h > 0 => ((tick + 1).min(h) as f64 / h as f64).clamp(0.0, 1.0),
        _ => 0.0,
    };
    let eta_label = match app.eta() {
        Some(d) if d.as_secs() > 0 || d.subsec_millis() > 0 => {
            format!("ETA {:.1}s", d.as_secs_f64())
        }
        Some(_) => "ETA <1s".to_string(),
        None => "ETA —".to_string(),
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("tick progress"),
        )
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(ratio)
        .label(format!(
            "{}/{}  {eta_label}",
            tick + horizon.map(|_| 1).unwrap_or(0),
            horizon
                .map(|h| h.to_string())
                .unwrap_or_else(|| "?".to_string())
        ));
    frame.render_widget(gauge, cols[0]);

    let tp = app
        .throughput()
        .map(|r| format!("{r:.1} ticks/s"))
        .unwrap_or_else(|| "—".to_string());
    let throughput = Paragraph::new(Line::from(vec![Span::raw(tp)]))
        .block(Block::default().borders(Borders::ALL).title("throughput"));
    frame.render_widget(throughput, cols[1]);

    let status = app.invariant_status();
    let style = if status.starts_with("OK") {
        Style::default().fg(Color::Green)
    } else if status.starts_with("STALLED") {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let inv = Paragraph::new(Line::from(vec![Span::styled(status, style)])).block(
        Block::default()
            .borders(Borders::ALL)
            .title("invariant status"),
    );
    frame.render_widget(inv, cols[2]);
}

/// Panel 3: agents alive/dead.
fn draw_agents(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let r = app.reconstruction();
    let live = r.live_count();
    let total = r.total_count();
    let mut items: Vec<ListItem<'_>> = vec![ListItem::new(format!(
        "alive: {live} / {total}  (dead: {})",
        total.saturating_sub(live)
    ))];
    for d in app.deaths().iter().rev().take(8) {
        items.push(ListItem::new(format!(
            "t{}: agent {} died ({})",
            d.tick, d.agent, d.cause
        )));
    }
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("agents alive/dead"),
    );
    frame.render_widget(list, area);
}

/// Panel 4: margin distribution — one bar per alive firm's current `h`,
/// reconstructed via `firma-analysis` (Part B) exactly as the offline SC
/// metrics are, so this panel and `sc16_gate` never disagree about what `h`
/// is for the same log.
fn draw_margin_distribution(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let h_crit = app.reconstruction().h_crit();
    let firms = app.reconstruction().firms();
    let bars: Vec<Bar<'_>> = firms
        .iter()
        .filter(|f| f.alive)
        .map(|f| {
            // scale h into a displayable non-negative integer (bps of margin,
            // floored at 0 for display — negative h is shown at height 0 with
            // the firm's id still visible, since a `BarChart` bar cannot be
            // negative).
            let bps = (f.h.max(0.0) * 1000.0).round() as u64;
            let color = if f.h < h_crit {
                Color::Red
            } else {
                Color::Green
            };
            Bar::default()
                .label(Line::from(format!("f{}", f.id)))
                .value(bps)
                .text_value(format!("{:.3}", f.h))
                .style(Style::default().fg(color))
        })
        .collect();
    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("margin distribution (h, h_crit={h_crit:.2})")),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(6)
        .bar_gap(1);
    frame.render_widget(chart, area);
}

/// Panel 5: action histogram — cumulative selected-action tally across the
/// whole population, canonical §11 action indices 0–8.
fn draw_action_histogram(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let totals = app.action_totals();
    let bars: Vec<Bar<'_>> = (0u8..=8)
        .map(|a| {
            let n = totals.get(&a).copied().unwrap_or(0);
            let color = if (6..=8).contains(&a) {
                Color::Magenta
            } else {
                Color::Blue
            };
            Bar::default()
                .label(Line::from(action_name(a)))
                .value(n)
                .style(Style::default().fg(color))
        })
        .collect();
    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("action histogram (0-5 market, 6-8 shaping)"),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(4)
        .bar_gap(1);
    frame.render_widget(chart, area);
}

fn action_name(a: u8) -> &'static str {
    match a {
        0 => "hold",
        1 => "prd_o",
        2 => "prd_r",
        3 => "acq",
        4 => "inv_c",
        5 => "dlv",
        6 => "lobby",
        7 => "ctrct",
        8 => "divrs",
        _ => "?",
    }
}

/// Panel 6: shock timeline.
fn draw_shock_timeline(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let items: Vec<ListItem<'_>> = if app.shock_timeline().is_empty() {
        vec![ListItem::new("(no shocks observed)")]
    } else {
        app.shock_timeline()
            .iter()
            .map(|s| ListItem::new(format!("t{}: {} moved {}", s.tick, s.origin, s.field)))
            .collect()
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("shock timeline"),
    );
    frame.render_widget(list, area);
}

/// Panel 8: log tail — the last few lines, human-summarised (raw JSON would
/// overflow a narrow panel).
fn draw_log_tail(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut items: Vec<ListItem<'_>> = app
        .raw_tail()
        .iter()
        .map(|l| ListItem::new(l.clone()))
        .collect();
    if app.parse_errors() > 0 {
        items.push(ListItem::new(format!(
            "({} unparsed line(s) seen)",
            app.parse_errors()
        )));
    }
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("log tail"));
    frame.render_widget(list, area);
}
