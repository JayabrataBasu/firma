//! `firma-cli` error type — wraps the layer errors with a little context.

use std::fmt;

use firma_core::{ConfigError, RegistryError};
use firma_io::IoError;
use firma_kernel::KernelError;

/// Anything that can go wrong orchestrating a run.
#[derive(Debug)]
pub enum CliError {
    /// Bad command-line usage.
    Usage(String),
    /// Config load or validation failed.
    Config(ConfigError),
    /// A plugin could not be resolved.
    Registry(RegistryError),
    /// The kernel aborted (an invariant violation or contract breach — §19.5).
    Kernel(KernelError),
    /// Persistence failed.
    Io(IoError),
    /// A replay/verify comparison failed.
    Mismatch(String),
    /// Anything else, with a message.
    Other(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Usage(m) => write!(f, "usage: {m}"),
            CliError::Config(e) => write!(f, "{e}"),
            CliError::Registry(e) => write!(f, "{e}"),
            CliError::Kernel(e) => write!(f, "{e}"),
            CliError::Io(e) => write!(f, "{e}"),
            CliError::Mismatch(m) => write!(f, "mismatch: {m}"),
            CliError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<ConfigError> for CliError {
    fn from(e: ConfigError) -> Self {
        CliError::Config(e)
    }
}
impl From<RegistryError> for CliError {
    fn from(e: RegistryError) -> Self {
        CliError::Registry(e)
    }
}
impl From<KernelError> for CliError {
    fn from(e: KernelError) -> Self {
        CliError::Kernel(e)
    }
}
impl From<IoError> for CliError {
    fn from(e: IoError) -> Self {
        CliError::Io(e)
    }
}
