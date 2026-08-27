use rusqlite::{Connection, OptionalExtension, params};

use super::archive::{
    persist_history_signature, persist_resolution, resolve_branch, selected_branch_id,
};
use super::history::{load_history, persist_compacted_history, persist_latest_metric_evidence};
use super::snapshots::persist_snapshots;
use super::{ObservatoryStorage, from_sql_integer, now_ms, to_sql_integer};
use crate::error::ObservatoryError;
use crate::model::{
    CoverageReport, CoverageStatus, FORMAT_PROFILE, MetricEvidence, PARSER_VERSION, REPUBLIC_SCOPE,
    ReceiverDataset, ReceiverHistoryPoint, SaveInspection,
};

impl ObservatoryStorage {
    pub fn save_inspection(&self, inspection: &SaveInspection) -> Result<bool, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM observation_sources WHERE payload_hash = ?1)",
            [&inspection.payload_hash],
            |row| row.get::<_, bool>(0),
        )?;

        if !exists {
            let resolution =
                resolve_branch(&transaction, &inspection.payload_hash, &inspection.records)?;
            let warnings_json = serde_json::to_string(&inspection.coverage.warnings)
                .map_err(|_| ObservatoryError::StorageUnavailable)?;
            transaction.execute(
                "INSERT INTO observation_sources(\
                     payload_hash, source_file_name, source_file_size, source_modified_ms,\
                     imported_at_ms, parser_version, format_profile, branch_id, geographic_scope,\
                     coverage_status, history_records, chartable_records, dropped_records, warnings_json\
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    inspection.payload_hash,
                    inspection.source_file_name,
                    to_sql_integer(inspection.source_file_size)?,
                    inspection.source_modified_ms,
                    now_ms(),
                    PARSER_VERSION,
                    FORMAT_PROFILE,
                    resolution.branch_id,
                    REPUBLIC_SCOPE,
                    inspection.coverage.status.as_str(),
                    inspection.coverage.history_records,
                    inspection.coverage.chartable_records,
                    inspection.coverage.dropped_records,
                    warnings_json,
                ],
            )?;

            persist_compacted_history(
                &transaction,
                &inspection.payload_hash,
                &inspection.records,
                resolution.shared_record_count as usize,
            )?;
            persist_latest_metric_evidence(
                &transaction,
                &inspection.payload_hash,
                &inspection.records,
            )?;
            persist_snapshots(
                &transaction,
                &inspection.payload_hash,
                &inspection.snapshots,
                &inspection.records,
            )?;
            persist_history_signature(&transaction, &inspection.payload_hash, &inspection.records)?;
            persist_resolution(&transaction, &inspection.payload_hash, &resolution)?;
            super::warehouse_jobs::enqueue_projection_job(
                &transaction,
                &format!("observation:{}", inspection.payload_hash),
                "observation",
                &inspection.payload_hash,
                now_ms(),
            )?;
        } else {
            let branch_id = transaction.query_row(
                "SELECT branch_id FROM observation_sources WHERE payload_hash = ?1",
                [&inspection.payload_hash],
                |row| row.get::<_, String>(0),
            )?;
            transaction.execute(
                "UPDATE archive_state SET selected_branch_id = ?1 WHERE singleton_id = 1",
                [branch_id],
            )?;
            let snapshots_exist = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM snapshot_scopes WHERE payload_hash = ?1)",
                [&inspection.payload_hash],
                |row| row.get::<_, bool>(0),
            )?;
            if !snapshots_exist && !inspection.snapshots.is_empty() {
                persist_snapshots(
                    &transaction,
                    &inspection.payload_hash,
                    &inspection.snapshots,
                    &inspection.records,
                )?;
            }
        }

        transaction.execute(
            "INSERT OR IGNORE INTO archive_observations(\
                 payload_hash, source_file_name, source_file_size, source_modified_ms,\
                 observed_at_ms, source_directory_identity\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                inspection.payload_hash,
                inspection.source_file_name,
                to_sql_integer(inspection.source_file_size)?,
                inspection.source_modified_ms,
                now_ms(),
                inspection.source_directory_identity,
            ],
        )?;
        transaction.execute(
            "UPDATE archive_observations SET source_directory_identity = ?1 \
             WHERE payload_hash = ?2 AND source_file_name = ?3 AND source_file_size = ?4 \
               AND source_modified_ms = ?5 AND source_directory_identity IS NULL",
            params![
                inspection.source_directory_identity,
                inspection.payload_hash,
                inspection.source_file_name,
                to_sql_integer(inspection.source_file_size)?,
                inspection.source_modified_ms,
            ],
        )?;

        transaction.commit()?;
        Ok(!exists)
    }

    pub fn load_latest_dataset(&self) -> Result<Option<ReceiverDataset>, ObservatoryError> {
        let connection = self.connect()?;
        let selected = selected_branch_id(&connection)?;
        let hash = connection
            .query_row(
                "SELECT payload_hash FROM observation_sources \
                 WHERE branch_id = ?1 ORDER BY imported_at_ms DESC, payload_hash DESC LIMIT 1",
                [&selected],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        hash.map(|hash| self.load_dataset_with_connection(&connection, &hash))
            .transpose()
    }

    pub fn load_dataset(&self, hash: &str) -> Result<ReceiverDataset, ObservatoryError> {
        let connection = self.connect()?;
        self.load_dataset_with_connection(&connection, hash)
    }

    pub(crate) fn file_observation_payload_hash(
        &self,
        source_directory_identity: &str,
        source_file_name: &str,
        source_file_size: u64,
        source_modified_ms: i64,
    ) -> Result<Option<String>, ObservatoryError> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT payload_hash FROM archive_observations \
                 WHERE source_directory_identity = ?1 AND source_file_name = ?2 \
                   AND source_file_size = ?3 AND source_modified_ms = ?4 \
                 ORDER BY observed_at_ms DESC LIMIT 1",
                params![
                    source_directory_identity,
                    source_file_name,
                    to_sql_integer(source_file_size)?,
                    source_modified_ms,
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    fn load_dataset_with_connection(
        &self,
        connection: &Connection,
        hash: &str,
    ) -> Result<ReceiverDataset, ObservatoryError> {
        let source = connection.query_row(
            r#"SELECT source_file_name, source_file_size, source_modified_ms, imported_at_ms,
                      parser_version, format_profile, branch_id, geographic_scope, coverage_status,
                      history_records, chartable_records, dropped_records, warnings_json
               FROM observation_sources WHERE payload_hash = ?1"#,
            [hash],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, u32>(9)?,
                    row.get::<_, u32>(10)?,
                    row.get::<_, u32>(11)?,
                    row.get::<_, String>(12)?,
                ))
            },
        )?;
        let warnings =
            serde_json::from_str(&source.12).map_err(|_| ObservatoryError::StorageUnavailable)?;
        let coverage = CoverageReport {
            status: if source.8 == "complete" {
                CoverageStatus::Complete
            } else {
                CoverageStatus::Partial
            },
            history_records: source.9,
            chartable_records: source.10,
            dropped_records: source.11,
            warnings,
        };

        let points = load_history(connection, hash)?
            .into_iter()
            .map(|record| ReceiverHistoryPoint {
                record_id: record.record_id,
                year: record.year,
                day: record.day,
                game_day: record.game_day,
                classified_total: record.classified_total,
                none: record.none,
                radio: record.radio,
                television: record.television,
                computer: record.computer,
            })
            .collect::<Vec<_>>();

        let source_fields = if points.is_empty() {
            Vec::new()
        } else {
            self.load_metric_evidence(connection, hash)?
        };

        Ok(ReceiverDataset {
            payload_hash: hash.to_owned(),
            source_file_name: source.0,
            source_file_size: from_sql_integer(source.1)?,
            source_modified_ms: source.2,
            imported_at_ms: source.3,
            parser_version: source.4,
            format_profile: source.5,
            branch_id: source.6,
            geographic_scope: source.7,
            coverage,
            source_fields,
            points,
        })
    }

    fn load_metric_evidence(
        &self,
        connection: &Connection,
        hash: &str,
    ) -> Result<Vec<MetricEvidence>, ObservatoryError> {
        let mut statement = connection.prepare(
            r#"SELECT metric_id, source_field, latest_source_line
               FROM observation_metric_evidence
               WHERE payload_hash = ?1
               ORDER BY metric_id"#,
        )?;
        statement
            .query_map([hash], |row| {
                Ok(MetricEvidence {
                    metric_id: row.get(0)?,
                    source_field: row.get(1)?,
                    latest_source_line: from_sql_integer(row.get(2)?)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}
