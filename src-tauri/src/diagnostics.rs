use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::model::{DiagnosticEntry, DiagnosticLogView};
use crate::storage::now_ms;

const LOG_FILE_NAME: &str = "republic-observatory-diagnostics.jsonl";
const MAX_ENTRIES: usize = 300;
const MAX_FIELD_LENGTH: usize = 500;
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

static DIAGNOSTICS: OnceLock<DiagnosticLog> = OnceLock::new();

struct DiagnosticLog {
    path: Option<PathBuf>,
    entries: Mutex<VecDeque<DiagnosticEntry>>,
}

impl DiagnosticLog {
    fn open(directory: Option<&Path>) -> Self {
        let path = directory.map(|directory| directory.join(LOG_FILE_NAME));
        let entries = path.as_deref().and_then(load_entries).unwrap_or_default();
        Self {
            path,
            entries: Mutex::new(entries),
        }
    }

    fn record(&self, level: &str, code: &str, operation: &str, message: &str) {
        let entry = DiagnosticEntry {
            occurred_at_ms: now_ms(),
            level: bounded_field(level),
            code: bounded_field(code),
            operation: bounded_field(operation),
            message: bounded_field(message),
        };
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        entries.push_back(entry.clone());
        while entries.len() > MAX_ENTRIES {
            entries.pop_front();
        }
        if let Some(path) = &self.path
            && (append_entry(path, &entry).is_err() || entries.len() == MAX_ENTRIES)
        {
            let _ = rewrite_entries(path, &entries);
        }
    }

    fn view(&self) -> DiagnosticLogView {
        let entries = self
            .entries
            .lock()
            .map(|entries| entries.iter().cloned().collect())
            .unwrap_or_default();
        DiagnosticLogView {
            language: "English",
            storage: if self.path.is_some() {
                "local_file"
            } else {
                "memory_only"
            },
            entries,
        }
    }

    fn clear(&self) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        entries.clear();
        if let Some(path) = &self.path {
            let _ = rewrite_entries(path, &entries);
        }
    }
}

pub fn initialize(directory: Option<&Path>) {
    let _ = DIAGNOSTICS.set(DiagnosticLog::open(directory));
}

pub fn record(level: &str, code: &str, operation: &str, message: &str) {
    DIAGNOSTICS
        .get_or_init(|| DiagnosticLog::open(None))
        .record(level, code, operation, message);
}

pub fn view() -> DiagnosticLogView {
    DIAGNOSTICS.get_or_init(|| DiagnosticLog::open(None)).view()
}

pub fn clear() -> DiagnosticLogView {
    let diagnostics = DIAGNOSTICS.get_or_init(|| DiagnosticLog::open(None));
    diagnostics.clear();
    diagnostics.view()
}

fn bounded_field(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || *character == ' ')
        .take(MAX_FIELD_LENGTH)
        .collect()
}

fn load_entries(path: &Path) -> Option<VecDeque<DiagnosticEntry>> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_LOG_BYTES {
        return None;
    }
    let contents = fs::read_to_string(path).ok()?;
    let mut entries = contents
        .lines()
        .filter_map(|line| serde_json::from_str::<DiagnosticEntry>(line).ok())
        .collect::<VecDeque<_>>();
    while entries.len() > MAX_ENTRIES {
        entries.pop_front();
    }
    Some(entries)
}

fn append_entry(path: &Path, entry: &DiagnosticEntry) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, entry).map_err(std::io::Error::other)?;
    file.write_all(b"\n")
}

fn rewrite_entries(path: &Path, entries: &VecDeque<DiagnosticEntry>) -> Result<(), std::io::Error> {
    let mut file = fs::File::create(path)?;
    for entry in entries {
        serde_json::to_writer(&mut file, entry).map_err(std::io::Error::other)?;
        file.write_all(b"\n")?;
    }
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_rotates_and_removes_control_characters() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let log = DiagnosticLog::open(Some(directory.path()));
        for index in 0..=MAX_ENTRIES {
            log.record(
                "info",
                "catalogue.phase",
                "refresh_catalogue",
                &format!("safe\nphase {index}"),
            );
        }
        let restored = DiagnosticLog::open(Some(directory.path())).view();
        assert_eq!(restored.entries.len(), MAX_ENTRIES);
        assert_eq!(restored.entries[0].message, "safephase 1");
        assert_eq!(restored.storage, "local_file");
    }
}
