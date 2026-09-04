//! The snapshot store — one pretty-JSON file per snapshot tick (ADR 0012).

use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::IoError;

/// A directory of full-state snapshots, keyed by tick (manual §22.1). Snapshots
/// are fork points and replay-from-checkpoint anchors (§22.1, DT-5).
pub struct SnapshotStore {
    dir: PathBuf,
}

impl SnapshotStore {
    /// Open (creating if needed) a snapshot store rooted at `dir`.
    ///
    /// # Errors
    /// [`IoError::Fs`] if the directory cannot be created.
    pub fn open(dir: &Path) -> Result<SnapshotStore, IoError> {
        fs::create_dir_all(dir)?;
        Ok(SnapshotStore {
            dir: dir.to_path_buf(),
        })
    }

    fn path(&self, tick: u64) -> PathBuf {
        self.dir.join(format!("snapshot-{tick:012}.json"))
    }

    /// Store the snapshot for `tick`.
    ///
    /// # Errors
    /// [`IoError::Codec`] if it will not serialise; [`IoError::Fs`] on write
    /// failure.
    pub fn put<T: Serialize>(&self, tick: u64, snapshot: &T) -> Result<(), IoError> {
        let json =
            serde_json::to_string_pretty(snapshot).map_err(|e| IoError::Codec(e.to_string()))?;
        fs::write(self.path(tick), json)?;
        Ok(())
    }

    /// Load the snapshot for `tick`.
    ///
    /// # Errors
    /// [`IoError::Fs`] if the file is missing; [`IoError::Codec`] if it does not
    /// parse.
    pub fn get<T: DeserializeOwned>(&self, tick: u64) -> Result<T, IoError> {
        let text = fs::read_to_string(self.path(tick))?;
        serde_json::from_str(&text).map_err(|e| IoError::Codec(e.to_string()))
    }

    /// All stored snapshot ticks, ascending.
    ///
    /// # Errors
    /// [`IoError::Fs`] if the directory cannot be read.
    pub fn ticks(&self) -> Result<Vec<u64>, IoError> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let name = entry?.file_name();
            let name = name.to_string_lossy();
            if let Some(rest) = name.strip_prefix("snapshot-") {
                if let Some(digits) = rest.strip_suffix(".json") {
                    if let Ok(t) = digits.parse::<u64>() {
                        out.push(t);
                    }
                }
            }
        }
        out.sort_unstable();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Snap {
        tick: u64,
        stocks: std::collections::BTreeMap<String, i64>,
    }

    #[test]
    fn put_get_ticks() {
        let dir = std::env::temp_dir().join(format!("firma-snap-{}", std::process::id()));
        let store = SnapshotStore::open(&dir).unwrap();
        let mut stocks = std::collections::BTreeMap::new();
        stocks.insert("capital".to_string(), 42);
        let s = Snap { tick: 25, stocks };
        store.put(25, &s).unwrap();
        store.put(50, &s).unwrap();
        assert_eq!(store.ticks().unwrap(), vec![25, 50]);
        let back: Snap = store.get(25).unwrap();
        assert_eq!(back, s);
        fs::remove_dir_all(&dir).ok();
    }
}
