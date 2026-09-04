//! `firma-io` — the durable, reconstructible record of a run (manual §22).
//!
//! Three streams (§22.1):
//!
//! | stream | this crate | cadence |
//! |---|---|---|
//! | event log | [`EventLog`] | every tick |
//! | snapshots | [`SnapshotStore`] | every *k* ticks |
//! | manifest | [`Manifest`] | once per run |
//!
//! Phase 1 encodes the event log as **newline-delimited JSON** and snapshots /
//! manifest as pretty JSON (ADR 0012). The crate is generic over the concrete
//! `Event` and `Snapshot` types — those live in `firma-kernel` — so `firma-io`
//! stays free of any simulation semantics (§18.1: `firma-io ← core, config`).
//!
//! Determinism (DT-1, §25.2): [`EventLog`] writes exactly one line per event —
//! `canonical_json(ev)` then `"\n"` — in call order, through one buffered
//! writer. Given the same event sequence it produces byte-identical output.

#![forbid(unsafe_code)]

mod eventlog;
mod manifest;
mod snapshot;

pub use eventlog::{read_events, EventLog};
pub use manifest::{BuildRecord, ExecutionRecord, Manifest, PluginManifestEntry, RunIdentity};
pub use snapshot::SnapshotStore;

use std::fmt;

/// Errors from the persistence layer.
#[derive(Debug)]
pub enum IoError {
    /// A filesystem operation failed.
    Fs(String),
    /// A record could not be encoded or decoded.
    Codec(String),
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoError::Fs(m) => write!(f, "io fs: {m}"),
            IoError::Codec(m) => write!(f, "io codec: {m}"),
        }
    }
}

impl std::error::Error for IoError {}

impl From<std::io::Error> for IoError {
    fn from(e: std::io::Error) -> Self {
        IoError::Fs(e.to_string())
    }
}

/// SHA-256 of `bytes`, lower-case hex.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    let d = h.finalize();
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
