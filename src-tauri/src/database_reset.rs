//! Guarded, restart-time removal of Observatory-owned database files.
//!
//! The command writes one bounded marker while databases are open. The next
//! process removes an exact allowlist before storage is initialised. Source game
//! and save directories are never accepted as inputs and are never traversed.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ObservatoryError;

pub const DATABASE_RESET_CONFIRMATION: &str = "ERASE OBSERVATORY DATA";
const RESET_MARKER_NAME: &str = ".observatory-database-reset.json";
const RESET_MARKER_TEMP_NAME: &str = ".observatory-database-reset.pending";
const MAX_MARKER_BYTES: u64 = 1_024;
const DATABASE_FILES: [&str; 8] = [
    "republic-observatory.sqlite3",
    "republic-observatory.sqlite3-wal",
    "republic-observatory.sqlite3-shm",
    "republic-observatory.sqlite3-journal",
    "republic-observatory.duckdb",
    "republic-observatory.duckdb.wal",
    "republic-observatory.duckdb.tmp",
    "republic-observatory.duckdb.lock",
];

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DatabaseResetMarker {
    schema_version: u32,
    confirmation: String,
}

pub fn schedule_database_reset(
    data_directory: &Path,
    confirmation: &str,
) -> Result<(), ObservatoryError> {
    if confirmation != DATABASE_RESET_CONFIRMATION {
        return Err(ObservatoryError::DatabaseResetConfirmationInvalid);
    }
    let metadata =
        fs::symlink_metadata(data_directory).map_err(|_| ObservatoryError::DatabaseResetFailed)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ObservatoryError::DatabaseResetFailed);
    }
    let marker = DatabaseResetMarker {
        schema_version: 1,
        confirmation: confirmation.to_owned(),
    };
    let bytes = serde_json::to_vec(&marker).map_err(|_| ObservatoryError::DatabaseResetFailed)?;
    let temporary = data_directory.join(RESET_MARKER_TEMP_NAME);
    let destination = data_directory.join(RESET_MARKER_NAME);
    if let Ok(metadata) = fs::symlink_metadata(&temporary)
        && (!metadata.is_file() || metadata.file_type().is_symlink())
    {
        return Err(ObservatoryError::DatabaseResetFailed);
    }
    fs::write(&temporary, bytes).map_err(|_| ObservatoryError::DatabaseResetFailed)?;
    fs::rename(&temporary, &destination).map_err(|_| ObservatoryError::DatabaseResetFailed)
}

pub fn apply_pending_database_reset(data_directory: &Path) -> Result<bool, ObservatoryError> {
    let marker_path = data_directory.join(RESET_MARKER_NAME);
    let metadata = match fs::symlink_metadata(&marker_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(ObservatoryError::DatabaseResetFailed),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_MARKER_BYTES
    {
        return Err(ObservatoryError::DatabaseResetFailed);
    }
    let bytes = fs::read(&marker_path).map_err(|_| ObservatoryError::DatabaseResetFailed)?;
    let marker = serde_json::from_slice::<DatabaseResetMarker>(&bytes)
        .map_err(|_| ObservatoryError::DatabaseResetFailed)?;
    if marker.schema_version != 1 || marker.confirmation != DATABASE_RESET_CONFIRMATION {
        return Err(ObservatoryError::DatabaseResetFailed);
    }

    for name in DATABASE_FILES {
        remove_database_file(&data_directory.join(name))?;
    }
    fs::remove_file(marker_path).map_err(|_| ObservatoryError::DatabaseResetFailed)?;
    Ok(true)
}

fn remove_database_file(path: &Path) -> Result<(), ObservatoryError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(ObservatoryError::DatabaseResetFailed),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ObservatoryError::DatabaseResetFailed);
    }
    fs::remove_file(path).map_err(|_| ObservatoryError::DatabaseResetFailed)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        DATABASE_RESET_CONFIRMATION, apply_pending_database_reset, schedule_database_reset,
    };

    #[test]
    fn removes_only_the_exact_observatory_database_allowlist() {
        let root = tempdir().expect("data root");
        fs::write(root.path().join("republic-observatory.sqlite3"), b"sqlite").expect("sqlite");
        fs::write(root.path().join("republic-observatory.sqlite3-wal"), b"wal").expect("wal");
        fs::write(root.path().join("republic-observatory.duckdb"), b"duckdb").expect("duckdb");
        fs::write(root.path().join("keep-me.txt"), b"unrelated").expect("unrelated");
        fs::create_dir(root.path().join("save_cloud")).expect("save folder");
        fs::write(root.path().join("save_cloud/campaign.zip"), b"save").expect("save file");

        schedule_database_reset(root.path(), DATABASE_RESET_CONFIRMATION).expect("schedule");
        assert!(apply_pending_database_reset(root.path()).expect("apply"));

        assert!(!root.path().join("republic-observatory.sqlite3").exists());
        assert!(
            !root
                .path()
                .join("republic-observatory.sqlite3-wal")
                .exists()
        );
        assert!(!root.path().join("republic-observatory.duckdb").exists());
        assert_eq!(
            fs::read(root.path().join("keep-me.txt")).expect("unrelated"),
            b"unrelated"
        );
        assert_eq!(
            fs::read(root.path().join("save_cloud/campaign.zip")).expect("save file"),
            b"save"
        );
        assert!(!apply_pending_database_reset(root.path()).expect("idempotent"));
    }

    #[test]
    fn rejects_missing_confirmation_and_tampered_markers() {
        let root = tempdir().expect("data root");
        assert!(schedule_database_reset(root.path(), "DELETE").is_err());
        fs::write(
            root.path().join(".observatory-database-reset.json"),
            br#"{"schema_version":1,"confirmation":"wrong"}"#,
        )
        .expect("tampered marker");
        assert!(apply_pending_database_reset(root.path()).is_err());
    }
}
