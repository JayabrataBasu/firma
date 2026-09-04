//! `firma-tui` — the live event-log monitor (manual §23.2, MVP interface
//! layer, §27.2).
//!
//! Built on **ratatui** + the **crossterm** backend, per the manual's own
//! choice, not a discretionary one (ADR 0046). Renders to any terminal,
//! including a headless server over SSH — the actual workload (an
//! unattended sweep, no display attached).
//!
//! **Reads the event log by tailing** ([`tail::Tailer`]) **and MUST NOT link
//! against the kernel or influence the run** (§23.2, capitalised in the
//! manual). This is enforced structurally, the same posture as every other
//! determinism rule in this project (§17 A3): [`app::App`] is built entirely
//! from `firma_analysis::Reconstruction` (ADR 0045), which depends on
//! `firma-core`/`firma-domain`/`firma-config`/`firma-io` and nothing kernel-
//! or plugin-shaped — checkable with `cargo tree -p firma-tui -e normal`
//! (Stage 7 Part A's verification, and `scripts/lint-architecture.sh`'s
//! `no-tui-kernel-deps` check).
//!
//! "**Neither [`firma-tui` nor `firma-inspect`] produces evidence**" (§23.2)
//! — this is a debugging/monitoring aid, not a source of published results.

#![forbid(unsafe_code)]

pub mod app;
pub mod tail;
pub mod ui;
