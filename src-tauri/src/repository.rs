use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::ObservatoryError;
use crate::model::{
    CoverageReport, CoverageStatus, FORMAT_PROFILE, INITIAL_BRANCH_ID, MetricEvidence,
    PARSER_VERSION, RECEIVER_METRICS, REPUBLIC_SCOPE, ReceiverDataset, ReceiverHistoryPoint,
    SaveInspection,
};

const MIGRATION_0001: &str = include_str!("../migrations/0001_observations.sql");

#[derive(Debug)]
pub struct ObservationRepository {
    database_path: PathBuf,
}

impl ObservationRepository {
    pub fn initialise(database_path: PathBuf) -> Result<Self, ObservatoryError> {
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(|_| ObservatoryError::StorageUnavailable)?;
        }
        let repository = Self { database_path };
        let mut connection = repository.connect()?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (\
                 version INTEGER PRIMARY KEY,\
                 name TEXT NOT NULL,\
                 applied_at_ms INTEGER NOT NULL\
             ) STRICT;",
        )?;
        let applied = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 1)",
            [],
            |row| row.get::<_, bool>(0),
        )?;
        if !applied {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_0001)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, name, applied_at_ms) VALUES(1, ?1, ?2)",
                params!["observation foundation", now_ms()],
            )?;
            transaction.commit()?;
        }
        Ok(repository)
    }

    pub fn set_setting(&self, key: &str, value: &Path) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        connection.execute(
            r#"INSERT INTO private_settings(setting_key, setting_value) VALUES(?1, ?2)
               ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value"#,
            params![key, value.to_string_lossy()],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<PathBuf>, ObservatoryError> {
        let connection = self.connect()?;
        let value = connection
            .query_row(
                "SELECT setting_value FROM private_settings WHERE setting_key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value.map(PathBuf::from))
    }

    pub fn save_inspection(&self, inspection: &SaveInspection) -> Result<bool, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let warnings_json = serde_json::to_string(&inspection.coverage.warnings)
            .map_err(|_| ObservatoryError::StorageUnavailable)?;
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO observation_sources(\
                 payload_hash, source_file_name, source_file_size, source_modified_ms,\
                 imported_at_ms, parser_version, format_profile, branch_id, geographic_scope,\
                 coverage_status, history_records, chartable_records, dropped_records, warnings_json\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                inspection.payload_hash,
                inspection.source_file_name,
                i64::try_from(inspection.source_file_size)
                    .map_err(|_| ObservatoryError::StorageUnavailable)?,
                inspection.source_modified_ms,
                now_ms(),
                PARSER_VERSION,
                FORMAT_PROFILE,
                INITIAL_BRANCH_ID,
                REPUBLIC_SCOPE,
                inspection.coverage.status.as_str(),
                inspection.coverage.history_records,
                inspection.coverage.chartable_records,
                inspection.coverage.dropped_records,
                warnings_json,
            ],
        )? > 0;

        if inserted {
            {
                let mut insert_record = transaction.prepare(
                    "INSERT INTO embedded_records(\
                         payload_hash, record_id, year, day, game_day, classified_total\
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                )?;
                let mut insert_metric = transaction.prepare(
                    "INSERT INTO metric_observations(\
                         payload_hash, record_id, metric_id, value_integer, source_field,\
                         source_line, evidence_kind, coverage\
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, 'save_fact', 'complete')",
                )?;

                for record in &inspection.records {
                    insert_record.execute(params![
                        inspection.payload_hash,
                        record.record_id,
                        record.year,
                        record.day,
                        record.game_day,
                        to_sql_integer(record.classified_total)?,
                    ])?;
                    let values = [
                        record.none,
                        record.radio,
                        record.television,
                        record.computer,
                    ];
                    let lines = [
                        record.source_lines.none,
                        record.source_lines.radio,
                        record.source_lines.television,
                        record.source_lines.computer,
                    ];
                    for ((metric, value), line) in RECEIVER_METRICS.iter().zip(values).zip(lines) {
                        insert_metric.execute(params![
                            inspection.payload_hash,
                            record.record_id,
                            metric.id,
                            to_sql_integer(value)?,
                            metric.source_field,
                            to_sql_integer(line)?,
                        ])?;
                    }
                }
            }
        }

        transaction.commit()?;
        Ok(inserted)
    }

    pub fn load_latest_dataset(&self) -> Result<Option<ReceiverDataset>, ObservatoryError> {
        let connection = self.connect()?;
        let hash = connection
            .query_row(
                "SELECT payload_hash FROM observation_sources ORDER BY imported_at_ms DESC LIMIT 1",
                [],
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

    pub fn observation_count(&self) -> Result<u32, ObservatoryError> {
        let connection = self.connect()?;
        connection
            .query_row("SELECT COUNT(*) FROM observation_sources", [], |row| {
                row.get::<_, u32>(0)
            })
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

        let mut statement = connection.prepare(
            r#"SELECT er.record_id, er.year, er.day, er.game_day, er.classified_total,
                      MAX(CASE WHEN mo.metric_id = ?2 THEN mo.value_integer END),
                      MAX(CASE WHEN mo.metric_id = ?3 THEN mo.value_integer END),
                      MAX(CASE WHEN mo.metric_id = ?4 THEN mo.value_integer END),
                      MAX(CASE WHEN mo.metric_id = ?5 THEN mo.value_integer END)
               FROM embedded_records er
               JOIN metric_observations mo
                 ON mo.payload_hash = er.payload_hash AND mo.record_id = er.record_id
               WHERE er.payload_hash = ?1
               GROUP BY er.record_id, er.year, er.day, er.game_day, er.classified_total
               ORDER BY er.record_id"#,
        )?;
        let points = statement
            .query_map(
                params![
                    hash,
                    RECEIVER_METRICS[0].id,
                    RECEIVER_METRICS[1].id,
                    RECEIVER_METRICS[2].id,
                    RECEIVER_METRICS[3].id,
                ],
                |row| {
                    Ok(ReceiverHistoryPoint {
                        record_id: row.get(0)?,
                        year: row.get(1)?,
                        day: row.get(2)?,
                        game_day: row.get(3)?,
                        classified_total: from_sql_integer(row.get(4)?)?,
                        none: from_sql_integer(row.get(5)?)?,
                        radio: from_sql_integer(row.get(6)?)?,
                        television: from_sql_integer(row.get(7)?)?,
                        computer: from_sql_integer(row.get(8)?)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        let latest_record_id = points.last().map(|point| point.record_id);
        let source_fields = latest_record_id
            .map(|record_id| self.load_metric_evidence(connection, hash, record_id))
            .transpose()?
            .unwrap_or_default();

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
        record_id: u32,
    ) -> Result<Vec<MetricEvidence>, ObservatoryError> {
        let mut statement = connection.prepare(
            r#"SELECT metric_id, source_field, source_line
               FROM metric_observations
               WHERE payload_hash = ?1 AND record_id = ?2
               ORDER BY metric_id"#,
        )?;
        statement
            .query_map(params![hash, record_id], |row| {
                Ok(MetricEvidence {
                    metric_id: row.get(0)?,
                    source_field: row.get(1)?,
                    latest_source_line: from_sql_integer(row.get(2)?)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn connect(&self) -> Result<Connection, ObservatoryError> {
        let connection = Connection::open(&self.database_path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(connection)
    }
}

fn to_sql_integer(value: u64) -> Result<i64, ObservatoryError> {
    i64::try_from(value).map_err(|_| ObservatoryError::StorageUnavailable)
}

fn from_sql_integer(value: i64) -> Result<u64, rusqlite::Error> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tempfile::tempdir;

    use super::ObservationRepository;
    use crate::model::SaveInspection;
    use crate::stats_parser::parse_stats;

    #[test]
    fn stores_normalised_metrics_and_deduplicates_by_payload_hash() {
        let directory = tempdir().expect("temporary directory");
        let repository = ObservationRepository::initialise(directory.path().join("test.sqlite3"))
            .expect("repository");
        let parsed = parse_stats(Cursor::new(include_bytes!(
            "../fixtures/valid.receiver-stats.txt"
        )))
        .expect("fixture");
        let inspection = SaveInspection {
            payload_hash: parsed.payload_hash,
            source_file_name: "synthetic.zip".to_owned(),
            source_file_size: 100,
            source_modified_ms: 1,
            records: parsed.records,
            coverage: parsed.coverage,
        };

        assert!(
            repository
                .save_inspection(&inspection)
                .expect("first import")
        );
        assert!(
            !repository
                .save_inspection(&inspection)
                .expect("duplicate import")
        );
        assert_eq!(repository.observation_count().expect("count"), 1);

        let dataset = repository
            .load_latest_dataset()
            .expect("load")
            .expect("dataset");
        assert_eq!(dataset.points.len(), 3);
        assert_eq!(dataset.source_fields.len(), 4);
        assert_eq!(dataset.branch_id, "unassigned");
        assert_eq!(dataset.geographic_scope, "republic");
    }
}
