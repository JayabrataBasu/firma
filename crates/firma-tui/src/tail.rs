//! A safe log tailer (manual §23.2: "Reads the event log by tailing").
//!
//! Handles the two ordinary states of an unattended, in-progress run without
//! erroring: the file does not exist yet (the run hasn't started writing),
//! and the file's newest bytes are an incomplete trailing line (a write is
//! mid-flight). Neither is a fault — both resolve themselves on the next
//! poll.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use firma_core::Event;

/// One polled line, parsed if it was valid JSON.
#[derive(Debug)]
pub enum PolledLine {
    /// A complete, valid event record.
    Parsed(Box<Event>),
    /// A complete line that failed to parse (corrupt, not merely partial —
    /// a partial line is never surfaced; it is carried to the next poll).
    Unparsed(String),
}

/// Tails one `events.ndjson` file, yielding newly-completed lines since the
/// last poll.
pub struct Tailer {
    path: PathBuf,
    file: Option<File>,
    offset: u64,
    carry: String,
}

impl Tailer {
    /// Watch `path`. The file need not exist yet.
    #[must_use]
    pub fn new(path: PathBuf) -> Tailer {
        Tailer {
            path,
            file: None,
            offset: 0,
            carry: String::new(),
        }
    }

    /// Read whatever new, complete lines are available since the last call.
    /// `Ok(vec![])` if the file does not exist yet, has not grown, or its
    /// only new bytes are an incomplete trailing line.
    ///
    /// # Errors
    /// An IO error other than "not found" while opening or reading the file.
    pub fn poll(&mut self) -> std::io::Result<Vec<PolledLine>> {
        if self.file.is_none() {
            match File::open(&self.path) {
                Ok(f) => self.file = Some(f),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
                Err(e) => return Err(e),
            }
        }
        let file = self.file.as_mut().expect("just set");
        file.seek(SeekFrom::Start(self.offset))?;
        let mut raw = Vec::new();
        file.read_to_end(&mut raw)?;
        if raw.is_empty() {
            return Ok(Vec::new());
        }
        self.offset += raw.len() as u64;

        let mut buf = std::mem::take(&mut self.carry);
        buf.push_str(&String::from_utf8_lossy(&raw));

        let mut out = Vec::new();
        let mut rest: &str = &buf;
        while let Some(i) = rest.find('\n') {
            let line = &rest[..i];
            rest = &rest[i + 1..];
            if line.trim().is_empty() {
                continue;
            }
            out.push(match serde_json::from_str::<Event>(line) {
                Ok(ev) => PolledLine::Parsed(Box::new(ev)),
                Err(_) => PolledLine::Unparsed(line.to_string()),
            });
        }
        self.carry = rest.to_string();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn missing_file_polls_empty_not_err() {
        let mut t = Tailer::new(PathBuf::from("/nonexistent/does-not-exist.ndjson"));
        assert!(t.poll().unwrap().is_empty());
    }

    #[test]
    fn partial_final_line_is_carried_not_surfaced() {
        let dir = std::env::temp_dir().join(format!("tui-tail-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.ndjson");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, r#"{{"event":"tick_started","tick":0}}"#).unwrap();
        // no trailing newline yet — a write in progress.
        drop(f);

        let mut t = Tailer::new(path.clone());
        let lines = t.poll().unwrap();
        assert!(
            lines.is_empty(),
            "a partial final line must not be surfaced as a record"
        );

        // the writer finishes the line and starts a new one.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        write!(f, "\n{{\"event\":\"tick_started\",\"tick\":1}}\n").unwrap();
        drop(f);

        let lines = t.poll().unwrap();
        assert_eq!(lines.len(), 2, "both completed lines now surface");
        assert!(matches!(
            &lines[0],
            PolledLine::Parsed(e) if matches!(**e, Event::TickStarted { tick: 0 })
        ));
        assert!(matches!(
            &lines[1],
            PolledLine::Parsed(e) if matches!(**e, Event::TickStarted { tick: 1 })
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_complete_line_is_reported_not_fatal() {
        let dir = std::env::temp_dir().join(format!("tui-tail-test2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.ndjson");
        std::fs::write(
            &path,
            "{not json}\n{\"event\":\"tick_started\",\"tick\":0}\n",
        )
        .unwrap();

        let mut t = Tailer::new(path);
        let lines = t.poll().unwrap();
        assert_eq!(lines.len(), 2);
        assert!(matches!(&lines[0], PolledLine::Unparsed(_)));
        assert!(matches!(&lines[1], PolledLine::Parsed(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
