//! The action window `W` (manual §8.1) and its entry type (ADR 0014, ADR 0028).
//!
//! `W` is a per-agent list of the last `L_W` actions the firm took, kept in the
//! kernel's opaque `agent_lists` store under [`keys::ACTION_WINDOW`](crate::keys::ACTION_WINDOW)
//! and maintained by the `constrain` rule. Its only consumer is
//! [`margin::u_from_window`](crate::margin::u_from_window), which derives the
//! regulated-activity intensity `u` (§9.1 `g_2`).

use serde::{Deserialize, Serialize};

/// One entry of the action window: the action a firm took and the tick it took
/// it (the tick is for log-readability; `u` uses only `action`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowEntry {
    /// The tick the action was taken.
    pub tick: u64,
    /// The §11 canonical action index 0–8. A firm with no decision this tick is
    /// recorded as `0` (`hold`).
    pub action: u8,
}

impl WindowEntry {
    /// Construct.
    #[must_use]
    pub fn new(tick: u64, action: u8) -> WindowEntry {
        WindowEntry { tick, action }
    }

    /// Serialise to the canonical JSON stored in the window.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("WindowEntry serialises")
    }

    /// Parse one stored window entry.
    ///
    /// # Errors
    /// If `s` is not a valid [`WindowEntry`] JSON object.
    pub fn from_json(s: &str) -> Result<WindowEntry, String> {
        serde_json::from_str(s).map_err(|e| format!("invalid WindowEntry: {e}"))
    }
}

/// `W`'s own update rule (§10.1 phase 7, `constrain`): append this tick's
/// action, then trim from the front down to `l_w` entries. **The single
/// source** of this rule — `firma-plugin-constraint::phase_rules::
/// ActionWindow::apply` (the real `constrain` rule) calls this; so does
/// `firma_domain::dynamics::time_to_boundary`'s forward projection (ADR
/// 0049) — one function, two callers, not two copies of the same three
/// lines.
#[must_use]
pub fn advance_window(
    window: &[WindowEntry],
    tick: u64,
    action: u8,
    l_w: usize,
) -> Vec<WindowEntry> {
    let mut w = window.to_vec();
    w.push(WindowEntry::new(tick, action));
    if w.len() > l_w {
        let drop = w.len() - l_w;
        w.drain(0..drop);
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let e = WindowEntry::new(42, 2);
        assert_eq!(WindowEntry::from_json(&e.to_json()).unwrap(), e);
    }

    #[test]
    fn advance_window_appends_and_trims() {
        let w = vec![WindowEntry::new(0, 1), WindowEntry::new(1, 2)];
        let w = advance_window(&w, 2, 3, 3);
        assert_eq!(
            w.iter().map(|e| e.action).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        let w = advance_window(&w, 3, 4, 3);
        assert_eq!(
            w.iter().map(|e| (e.tick, e.action)).collect::<Vec<_>>(),
            vec![(1, 2), (2, 3), (3, 4)],
            "oldest entry (tick 0) drops once the window is full"
        );
    }
}
