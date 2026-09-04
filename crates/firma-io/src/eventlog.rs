//! The event log — newline-delimited JSON, one record per line (ADR 0012).

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::IoError;

/// An append-only event sink. Generic over the event type so this crate carries
/// no simulation semantics.
///
/// Every [`append`](EventLog::append) writes `canonical_json(event)` followed by
/// a single `'\n'`. Two logs built from the same event sequence are
/// byte-identical (DT-1, §25.2).
pub struct EventLog<W: Write> {
    writer: BufWriter<W>,
    count: u64,
}

impl EventLog<Vec<u8>> {
    /// An in-memory log, for tests and for `firma-cli replay`'s comparison path.
    #[must_use]
    pub fn in_memory() -> EventLog<Vec<u8>> {
        EventLog {
            writer: BufWriter::new(Vec::new()),
            count: 0,
        }
    }

    /// Consume the log and return its raw bytes. Flushes first.
    ///
    /// # Errors
    /// [`IoError::Fs`] if the final flush fails.
    pub fn into_bytes(mut self) -> Result<Vec<u8>, IoError> {
        self.writer.flush()?;
        self.writer
            .into_inner()
            .map_err(|e| IoError::Fs(e.to_string()))
    }
}

impl EventLog<File> {
    /// Create (truncating) an event log file at `path`.
    ///
    /// # Errors
    /// [`IoError::Fs`] if the file cannot be created.
    pub fn create(path: &Path) -> Result<EventLog<File>, IoError> {
        let f = File::create(path)?;
        Ok(EventLog {
            writer: BufWriter::new(f),
            count: 0,
        })
    }
}

impl<W: Write> EventLog<W> {
    /// Append one event.
    ///
    /// # Errors
    /// [`IoError::Codec`] if the event will not serialise; [`IoError::Fs`] on a
    /// write failure.
    pub fn append<T: Serialize>(&mut self, event: &T) -> Result<(), IoError> {
        let line = serde_json::to_string(event).map_err(|e| IoError::Codec(e.to_string()))?;
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.count += 1;
        Ok(())
    }

    /// Number of events appended so far.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.count
    }

    /// Whether no events have been appended.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Flush buffered bytes to the underlying writer.
    ///
    /// # Errors
    /// [`IoError::Fs`] on failure.
    pub fn flush(&mut self) -> Result<(), IoError> {
        self.writer.flush()?;
        Ok(())
    }
}

/// Read an NDJSON event log back into typed events, in order.
///
/// # Errors
/// [`IoError::Fs`] if the file cannot be opened; each item is
/// [`IoError::Codec`] if that line does not parse.
pub fn read_events<T: DeserializeOwned>(
    path: &Path,
) -> Result<impl Iterator<Item = Result<T, IoError>>, IoError> {
    let f = File::open(path)?;
    let reader = BufReader::new(f);
    Ok(reader.lines().filter_map(|line| match line {
        Err(e) => Some(Err(IoError::Fs(e.to_string()))),
        Ok(l) if l.trim().is_empty() => None,
        Ok(l) => Some(serde_json::from_str::<T>(&l).map_err(|e| IoError::Codec(e.to_string()))),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Ev {
        tick: u64,
        what: String,
    }

    #[test]
    fn same_events_same_bytes() {
        let build = || {
            let mut log = EventLog::in_memory();
            for t in 0..5 {
                log.append(&Ev {
                    tick: t,
                    what: "x".into(),
                })
                .unwrap();
            }
            log.into_bytes().unwrap()
        };
        assert_eq!(build(), build());
    }

    #[test]
    fn roundtrip() {
        let mut log = EventLog::in_memory();
        log.append(&Ev {
            tick: 1,
            what: "a".into(),
        })
        .unwrap();
        log.append(&Ev {
            tick: 2,
            what: "b".into(),
        })
        .unwrap();
        let bytes = log.into_bytes().unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let back: Vec<Ev> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(
            back,
            vec![
                Ev {
                    tick: 1,
                    what: "a".into()
                },
                Ev {
                    tick: 2,
                    what: "b".into()
                }
            ]
        );
    }
}
