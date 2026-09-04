//! The run manifest (manual §22.3).
//!
//! Split into two parts so the run identity is reproducible:
//!
//! * [`RunIdentity`] — everything that determines the trajectory. Its canonical
//!   SHA-256 **is** the run id (§22.3). Two runs with equal identity MUST
//!   produce byte-identical event logs (DT-1, §25.2).
//! * [`ExecutionRecord`] — provenance that does *not* affect the trajectory
//!   (toolchain, host, duration, git sha). Recorded, never hashed into the id.

use std::path::Path;

use serde::{Deserialize, Serialize};

use firma_config::{RunConfig, StreamSeeds, TimedIntervention};

use crate::{sha256_hex, IoError};

/// A plugin's manifest row (manual §22.3: "every plugin's
/// `(id, version, content_hash, assumption_text)`").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifestEntry {
    /// `"<category>.<name>"`.
    pub id: String,
    /// Resolved build version.
    pub version: String,
    /// Declared content hash (Phase 1; see `firma-registry` docs).
    pub content_hash: String,
    /// The plugin's `assumption()` sentence, verbatim (manual §20.2). MUST be
    /// non-empty (§25.6 `assumption-nonempty`).
    pub assumption: String,
}

/// The trajectory-determining part of the manifest. Its canonical JSON hash is
/// the run id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunIdentity {
    /// Manual version in force (manual §0.6, §22.3).
    pub manual_version: String,
    /// Engine semantic version.
    pub engine_version: String,
    /// The RNG scheme identifier (manual §21.2; encoding pinned per OQ-1).
    pub rng_scheme: String,
    /// Config content hash (manual §20.4).
    pub config_hash: String,
    /// Fully resolved config after defaults (manual §22.3).
    pub resolved_config: RunConfig,
    /// Seeds and their stream assignment (echoed for legibility; also inside
    /// `resolved_config`).
    pub seeds: StreamSeeds,
    /// Every rule plugin, ascending by id.
    pub rules: Vec<PluginManifestEntry>,
    /// The conflict resolver plugin.
    pub conflict_resolver: PluginManifestEntry,
    /// Complete intervention log with tick stamps (manual §22.3).
    pub interventions: Vec<TimedIntervention>,
    /// `ExperimentSpec` hash, when the run belongs to one (manual §28.1). `None`
    /// in Phase 1 — there is no `ExperimentSpec` type yet.
    pub experiment_spec_hash: Option<String>,
}

impl RunIdentity {
    /// The run id: SHA-256 (hex) over this value's canonical JSON (manual §22.3).
    ///
    /// # Errors
    /// [`IoError::Codec`] if serialisation fails.
    pub fn run_id(&self) -> Result<String, IoError> {
        let json =
            firma_core::to_canonical_json(self).map_err(|e| IoError::Codec(e.to_string()))?;
        Ok(sha256_hex(json.as_bytes()))
    }
}

/// The compiler and build environment (manual §24.1 pinned toolchain).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRecord {
    /// `rustc --version --verbose` first line, or the compile-time version.
    pub rustc: String,
    /// Target triple.
    pub target: String,
    /// SHA-256 of `Cargo.lock`, when locatable (manual §22.3 "lockfile hash").
    pub lockfile_sha256: Option<String>,
}

/// Provenance that does not affect the trajectory (manual §22.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Build environment.
    pub build: BuildRecord,
    /// Host OS string.
    pub host_platform: String,
    /// Engine git SHA, when the build was made from a git checkout (manual
    /// §22.3). `None` otherwise — see `PROGRESS.md` OQ-4.
    pub engine_git_sha: Option<String>,
    /// Container image digest, when run in one (manual §22.3). `None` in Phase 1.
    pub container_digest: Option<String>,
    /// Wall-clock run duration in seconds. Provenance only — never hashed into
    /// the run id.
    pub duration_secs: Option<f64>,
    /// Free-text note about which §22.3 fields are absent and why.
    pub provenance_note: String,
}

/// The complete manifest: identity + execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// The hashed, trajectory-determining part.
    pub identity: RunIdentity,
    /// The unhashed provenance part.
    pub execution: ExecutionRecord,
}

impl Manifest {
    /// The run id (delegates to [`RunIdentity::run_id`]).
    ///
    /// # Errors
    /// [`IoError::Codec`] if serialisation fails.
    pub fn run_id(&self) -> Result<String, IoError> {
        self.identity.run_id()
    }

    /// Write `manifest.json` (pretty) into `dir`.
    ///
    /// # Errors
    /// [`IoError::Codec`] on a serialisation failure; [`IoError::Fs`] on a write
    /// failure.
    pub fn write(&self, dir: &Path) -> Result<(), IoError> {
        std::fs::create_dir_all(dir)?;
        let json = serde_json::to_string_pretty(self).map_err(|e| IoError::Codec(e.to_string()))?;
        std::fs::write(dir.join("manifest.json"), json)?;
        Ok(())
    }

    /// Read a manifest back.
    ///
    /// # Errors
    /// [`IoError::Fs`] if the file is missing; [`IoError::Codec`] if it does not
    /// parse.
    pub fn read(dir: &Path) -> Result<Manifest, IoError> {
        let text = std::fs::read_to_string(dir.join("manifest.json"))?;
        serde_json::from_str(&text).map_err(|e| IoError::Codec(e.to_string()))
    }
}
