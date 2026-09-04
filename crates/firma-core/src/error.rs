//! Error enums shared across the workspace (manual §18.2). Hand-written
//! `Display` / `Error` impls — no `thiserror` in Phase 1 (ADR 0010).

use std::fmt;

/// Errors from core helpers (serialisation, mostly).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// A value could not be serialised to canonical JSON.
    Serialize(String),
    /// A value could not be deserialised.
    Deserialize(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Serialize(m) => write!(f, "serialize: {m}"),
            CoreError::Deserialize(m) => write!(f, "deserialize: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

/// Errors from `firma-config` schema loading and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The file could not be read.
    Io(String),
    /// The document did not parse or had an unknown key (manual §20.4:
    /// "Unknown keys are an error, not a warning").
    Parse(String),
    /// A field was present but invalid (out of range, empty, inconsistent).
    Invalid(String),
    /// The config's `engine` requirement does not admit this engine version
    /// (§20.4).
    EngineMismatch {
        /// The `engine` requirement string from the config.
        required: String,
        /// This engine's version.
        actual: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(m) => write!(f, "config io: {m}"),
            ConfigError::Parse(m) => write!(f, "config parse: {m}"),
            ConfigError::Invalid(m) => write!(f, "config invalid: {m}"),
            ConfigError::EngineMismatch { required, actual } => {
                write!(
                    f,
                    "engine {actual} does not satisfy config requirement {required:?}"
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Errors from `firma-registry` plugin resolution (manual §18.2, §20.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// No plugin with that id is registered.
    UnknownPlugin(String),
    /// The requested version range excludes every registered build (§20.5: the
    /// engine MUST refuse a MAJOR mismatch).
    VersionMismatch {
        /// The plugin id.
        id: String,
        /// The requested range.
        required: String,
        /// The registered version.
        available: String,
    },
    /// The declared content hash did not match the registered plugin (§18.2:
    /// "hash mismatch rejected").
    HashMismatch {
        /// The plugin id.
        id: String,
    },
    /// The plugin's parameters failed its own schema check.
    BadParams {
        /// The plugin id.
        id: String,
        /// What was wrong.
        detail: String,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::UnknownPlugin(id) => write!(f, "unknown plugin {id:?}"),
            RegistryError::VersionMismatch {
                id,
                required,
                available,
            } => write!(
                f,
                "plugin {id:?}: no build satisfies {required:?} (registered {available})"
            ),
            RegistryError::HashMismatch { id } => {
                write!(
                    f,
                    "plugin {id:?}: declared content hash does not match registered build"
                )
            }
            RegistryError::BadParams { id, detail } => {
                write!(f, "plugin {id:?}: bad params: {detail}")
            }
        }
    }
}

impl std::error::Error for RegistryError {}
