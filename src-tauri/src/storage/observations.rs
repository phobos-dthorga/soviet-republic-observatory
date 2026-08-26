use rusqlite::{Connection, OptionalExtension, params};

use super::archive::{
    persist_history_signature, persist_resolution, resolve_branch, selected_branch_id,
};
use super::{ObservatoryStorage, from_sql_integer, now_ms, to_sql_integer};
use crate::error::ObservatoryError;
use crate::model::{
    CoverageReport, CoverageStatus, FORMAT_PROFILE, MetricEvidence, PARSER_VERSION,
    RECEIVER_METRICS, REPUBLIC_SCOPE, ReceiverDataset, ReceiverHistoryPoint, SaveInspection,
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
            persist_history_signature(&transaction, &inspection.payload_hash, &inspection.records)?;
            persist_resolution(&transaction, &inspection.payload_hash, &resolution)?;
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
        }

        transaction.execute(
            "INSERT OR IGNORE INTO archive_observations(\
                 payload_hash, source_file_name, source_file_size, source_modified_ms, observed_at_ms\
             ) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                inspection.payload_hash,
                inspection.source_file_name,
                to_sql_integer(inspection.source_file_size)?,
                inspection.source_modified_ms,
                now_ms(),
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
}
