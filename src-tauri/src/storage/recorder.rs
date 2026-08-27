use rusqlite::{Connection, OptionalExtension, params};

use super::{ObservatoryStorage, from_sql_integer, to_sql_integer};
use crate::error::ObservatoryError;
use crate::model::{
    AutomaticObserverStatus, ImportOutcome, RecorderCandidateStatus, RecorderDiscoverySource,
    RecorderHealth, RecorderLedgerEntry,
};

const ACTIVE_STATUSES: &str =
    "'discovered', 'stabilising', 'ready', 'reading', 'retryable_failure'";

#[derive(Clone, Debug)]
pub(crate) struct StoredRecorderCandidate {
    pub candidate_id: i64,
    pub status: RecorderCandidateStatus,
    pub attempt_count: u32,
}

pub(crate) fn recover_interrupted_candidates(
    connection: &Connection,
) -> Result<(), ObservatoryError> {
    connection.execute(
        "UPDATE recorder_candidates \
         SET status = 'discovered', error_code = 'interrupted' \
         WHERE status IN ('stabilising', 'ready', 'reading')",
        [],
    )?;
    Ok(())
}

impl ObservatoryStorage {
    pub(crate) fn recorder_directory_is_initialised(
        &self,
        directory_identity: &str,
    ) -> Result<bool, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM recorder_directories \
                 WHERE source_directory_identity = ?1)",
                [directory_identity],
                |row| row.get::<_, bool>(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn mark_recorder_directory_initialised(
        &self,
        directory_identity: &str,
        initialised_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "INSERT OR IGNORE INTO recorder_directories(\
                 source_directory_identity, initialised_at_ms\
             ) VALUES(?1, ?2)",
            params![directory_identity, initialised_at_ms],
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn discover_recorder_candidate(
        &self,
        directory_identity: &str,
        file_name: &str,
        file_size: u64,
        modified_ms: i64,
        discovered_at_ms: i64,
        discovery_source: RecorderDiscoverySource,
    ) -> Result<StoredRecorderCandidate, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            &format!(
                "UPDATE recorder_candidates \
                 SET status = 'superseded', completed_at_ms = ?1, last_seen_at_ms = ?1 \
                 WHERE source_directory_identity = ?2 AND source_file_name = ?3 \
                   AND (source_file_size <> ?4 OR source_modified_ms <> ?5) \
                   AND status IN ({ACTIVE_STATUSES}, 'terminal_failure')"
            ),
            params![
                discovered_at_ms,
                directory_identity,
                file_name,
                to_sql_integer(file_size)?,
                modified_ms,
            ],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO recorder_candidates(\
                 source_directory_identity, source_file_name, source_file_size,\
                 source_modified_ms, status, discovery_source, discovered_at_ms,\
                 last_seen_at_ms\
             ) VALUES(?1, ?2, ?3, ?4, 'discovered', ?5, ?6, ?6)",
            params![
                directory_identity,
                file_name,
                to_sql_integer(file_size)?,
                modified_ms,
                discovery_source.as_str(),
                discovered_at_ms,
            ],
        )?;
        transaction.execute(
            "UPDATE recorder_candidates SET last_seen_at_ms = ?1 \
             WHERE source_directory_identity = ?2 AND source_file_name = ?3 \
               AND source_file_size = ?4 AND source_modified_ms = ?5",
            params![
                discovered_at_ms,
                directory_identity,
                file_name,
                to_sql_integer(file_size)?,
                modified_ms,
            ],
        )?;
        let candidate = transaction.query_row(
            "SELECT candidate_id, status, discovered_at_ms, attempt_count \
             FROM recorder_candidates \
             WHERE source_directory_identity = ?1 AND source_file_name = ?2 \
               AND source_file_size = ?3 AND source_modified_ms = ?4",
            params![
                directory_identity,
                file_name,
                to_sql_integer(file_size)?,
                modified_ms,
            ],
            stored_candidate_from_row,
        )?;
        transaction.commit()?;
        Ok(candidate)
    }

    pub(crate) fn mark_recorder_candidate_stabilising(
        &self,
        candidate_id: i64,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "UPDATE recorder_candidates SET status = 'stabilising', error_code = NULL \
             WHERE candidate_id = ?1 AND status IN ('discovered', 'retryable_failure')",
            [candidate_id],
        )?;
        Ok(())
    }

    pub(crate) fn mark_recorder_candidate_ready(
        &self,
        candidate_id: i64,
        ready_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "UPDATE recorder_candidates \
             SET status = 'ready', first_stable_at_ms = COALESCE(first_stable_at_ms, ?1) \
             WHERE candidate_id = ?2 AND status IN ('stabilising', 'retryable_failure')",
            params![ready_at_ms, candidate_id],
        )?;
        Ok(())
    }

    pub(crate) fn mark_recorder_candidate_reading(
        &self,
        candidate_id: i64,
        attempt_at_ms: i64,
    ) -> Result<u32, ObservatoryError> {
        let connection = self.connect()?;
        connection.execute(
            "UPDATE recorder_candidates \
             SET status = 'reading', last_attempt_at_ms = ?1, attempt_count = attempt_count + 1 \
             WHERE candidate_id = ?2 AND status IN ('ready', 'retryable_failure')",
            params![attempt_at_ms, candidate_id],
        )?;
        connection
            .query_row(
                "SELECT attempt_count FROM recorder_candidates WHERE candidate_id = ?1",
                [candidate_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn complete_recorder_candidate(
        &self,
        candidate_id: i64,
        outcome: ImportOutcome,
        payload_hash: &str,
        completed_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let status = match outcome {
            ImportOutcome::Imported => RecorderCandidateStatus::Imported,
            ImportOutcome::Duplicate => RecorderCandidateStatus::Duplicate,
        };
        self.connect()?.execute(
            "UPDATE recorder_candidates \
             SET status = ?1, completed_at_ms = ?2, import_outcome = ?1,\
                 payload_hash = ?3, error_code = NULL \
             WHERE candidate_id = ?4",
            params![status.as_str(), completed_at_ms, payload_hash, candidate_id],
        )?;
        Ok(())
    }

    pub(crate) fn fail_recorder_candidate(
        &self,
        candidate_id: i64,
        retryable: bool,
        error_code: &str,
        failed_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let status = if retryable {
            RecorderCandidateStatus::RetryableFailure
        } else {
            RecorderCandidateStatus::TerminalFailure
        };
        self.connect()?.execute(
            "UPDATE recorder_candidates \
             SET status = ?1, error_code = ?2,\
                 completed_at_ms = CASE WHEN ?1 = 'terminal_failure' THEN ?3 ELSE NULL END \
             WHERE candidate_id = ?4",
            params![status.as_str(), error_code, failed_at_ms, candidate_id],
        )?;
        Ok(())
    }

    pub(crate) fn supersede_recorder_candidate(
        &self,
        candidate_id: i64,
        superseded_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            &format!(
                "UPDATE recorder_candidates \
                 SET status = 'superseded', completed_at_ms = ?1 \
                 WHERE candidate_id = ?2 AND status IN ({ACTIVE_STATUSES})"
            ),
            params![superseded_at_ms, candidate_id],
        )?;
        Ok(())
    }

    pub(crate) fn supersede_unseen_recorder_candidates(
        &self,
        directory_identity: &str,
        scan_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            &format!(
                "UPDATE recorder_candidates \
                 SET status = 'superseded', completed_at_ms = ?1 \
                 WHERE source_directory_identity = ?2 AND last_seen_at_ms < ?1 \
                   AND status IN ({ACTIVE_STATUSES})"
            ),
            params![scan_at_ms, directory_identity],
        )?;
        Ok(())
    }

    pub(crate) fn note_recorder_scan(&self, scanned_at_ms: i64) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "UPDATE recorder_runtime_state SET last_scan_ms = ?1 WHERE singleton_id = 1",
            [scanned_at_ms],
        )?;
        Ok(())
    }

    pub(crate) fn note_recorder_filesystem_event(
        &self,
        event_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "UPDATE recorder_runtime_state SET last_filesystem_event_ms = ?1 \
             WHERE singleton_id = 1",
            [event_at_ms],
        )?;
        Ok(())
    }

    pub fn load_recorder_health(
        &self,
        observer: AutomaticObserverStatus,
    ) -> Result<RecorderHealth, ObservatoryError> {
        let connection = self.connect()?;
        let runtime = connection.query_row(
            "SELECT last_scan_ms, last_filesystem_event_ms \
             FROM recorder_runtime_state WHERE singleton_id = 1",
            [],
            |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?)),
        )?;
        let queue_depth = connection.query_row(
            &format!(
                "SELECT COUNT(*) FROM recorder_candidates WHERE status IN ({ACTIVE_STATUSES})"
            ),
            [],
            |row| row.get(0),
        )?;
        let attention_count = connection.query_row(
            "SELECT COUNT(*) FROM recorder_candidates WHERE status = 'terminal_failure'",
            [],
            |row| row.get(0),
        )?;
        let completed_count = connection.query_row(
            "SELECT COUNT(*) FROM recorder_candidates WHERE status IN ('imported', 'duplicate')",
            [],
            |row| row.get(0),
        )?;
        let latest_completed = connection
            .query_row(
                "SELECT source_file_name, completed_at_ms, completed_at_ms - discovered_at_ms \
                 FROM recorder_candidates WHERE status IN ('imported', 'duplicate') \
                 ORDER BY completed_at_ms DESC, candidate_id DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?;
        let latest_entries = load_latest_entries(&connection)?;
        Ok(RecorderHealth {
            observer,
            last_scan_ms: runtime.0,
            last_filesystem_event_ms: runtime.1,
            last_completed_at_ms: latest_completed.as_ref().map(|value| value.1),
            last_completed_file_name: latest_completed.as_ref().map(|value| value.0.clone()),
            last_processing_latency_ms: latest_completed.map(|value| value.2),
            queue_depth,
            attention_count,
            completed_count,
            latest_entries,
        })
    }

    #[cfg(test)]
    pub(crate) fn recorder_candidate_count(&self) -> Result<u32, ObservatoryError> {
        self.connect()?
            .query_row("SELECT COUNT(*) FROM recorder_candidates", [], |row| {
                row.get(0)
            })
            .map_err(Into::into)
    }
}

fn stored_candidate_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<StoredRecorderCandidate, rusqlite::Error> {
    let status = row.get::<_, String>(1)?;
    Ok(StoredRecorderCandidate {
        candidate_id: row.get(0)?,
        status: RecorderCandidateStatus::from_storage(&status).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(1, "status".to_owned(), rusqlite::types::Type::Text)
        })?,
        attempt_count: row.get(3)?,
    })
}

fn load_latest_entries(
    connection: &Connection,
) -> Result<Vec<RecorderLedgerEntry>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT candidate_id, source_file_name, source_file_size, source_modified_ms,\
                status, discovery_source, discovered_at_ms, first_stable_at_ms,\
                last_attempt_at_ms, completed_at_ms, attempt_count, error_code,\
                import_outcome, payload_hash \
         FROM recorder_candidates \
         ORDER BY COALESCE(completed_at_ms, last_attempt_at_ms, discovered_at_ms) DESC,\
                  candidate_id DESC LIMIT 32",
    )?;
    statement
        .query_map([], |row| {
            let status_raw = row.get::<_, String>(4)?;
            let discovery_raw = row.get::<_, String>(5)?;
            let discovered_at_ms = row.get::<_, i64>(6)?;
            let completed_at_ms = row.get::<_, Option<i64>>(9)?;
            let outcome_raw = row.get::<_, Option<String>>(12)?;
            Ok(RecorderLedgerEntry {
                candidate_id: row.get(0)?,
                file_name: row.get(1)?,
                file_size: from_sql_integer(row.get(2)?)?,
                source_modified_ms: row.get(3)?,
                status: RecorderCandidateStatus::from_storage(&status_raw).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        4,
                        "status".to_owned(),
                        rusqlite::types::Type::Text,
                    )
                })?,
                discovery_source: RecorderDiscoverySource::from_storage(&discovery_raw)
                    .ok_or_else(|| {
                        rusqlite::Error::InvalidColumnType(
                            5,
                            "discovery_source".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                discovered_at_ms,
                first_stable_at_ms: row.get(7)?,
                last_attempt_at_ms: row.get(8)?,
                completed_at_ms,
                attempt_count: row.get(10)?,
                error_code: row.get(11)?,
                import_outcome: outcome_raw
                    .as_deref()
                    .map(|value| match value {
                        "imported" => Ok(ImportOutcome::Imported),
                        "duplicate" => Ok(ImportOutcome::Duplicate),
                        _ => Err(rusqlite::Error::InvalidColumnType(
                            12,
                            "import_outcome".to_owned(),
                            rusqlite::types::Type::Text,
                        )),
                    })
                    .transpose()?,
                payload_hash: row.get(13)?,
                processing_latency_ms: completed_at_ms.map(|value| value - discovered_at_ms),
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}
