use rusqlite::{Connection, OptionalExtension, params};

use super::archive::{persist_history_signature, persist_resolution, resolve_branch};
use super::history::{load_history, persist_compacted_history, persist_latest_metric_evidence};
use super::snapshots::persist_snapshots;
use super::{ObservatoryStorage, from_sql_integer, now_ms, to_sql_integer};
use crate::error::ObservatoryError;
use crate::model::{
    CompatibilityProvenance, CoverageReport, CoverageStatus, MetricEvidence, PARSER_VERSION,
    REPUBLIC_SCOPE, ReceiverDataset, ReceiverHistoryPoint, SaveInspection,
};

impl ObservatoryStorage {
    pub fn save_inspection(&self, inspection: &SaveInspection) -> Result<bool, ObservatoryError> {
        self.save_inspection_internal(inspection, true)
    }

    pub fn save_reinterpretation(
        &self,
        inspection: &SaveInspection,
    ) -> Result<bool, ObservatoryError> {
        self.save_inspection_internal(inspection, false)
    }

    fn save_inspection_internal(
        &self,
        inspection: &SaveInspection,
        record_file_observation: bool,
    ) -> Result<bool, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let existing_storage_key = transaction
            .query_row(
                "SELECT payload_hash FROM observation_sources WHERE interpretation_id = ?1",
                [&inspection.interpretation_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let exists = existing_storage_key.is_some();
        let storage_key =
            existing_storage_key.unwrap_or_else(|| inspection.interpretation_id.clone());

        if !exists {
            let resolution = resolve_branch(&transaction, &storage_key, &inspection.records)?;
            let warnings_json = serde_json::to_string(&inspection.coverage.warnings)
                .map_err(|_| ObservatoryError::StorageUnavailable)?;
            transaction.execute(
                "INSERT INTO observation_sources(\
                     payload_hash, source_file_name, source_file_size, source_modified_ms,\
                     imported_at_ms, parser_version, format_profile, branch_id, geographic_scope,\
                     coverage_status, history_records, chartable_records, dropped_records, warnings_json,\
                     raw_payload_hash, interpretation_id, profile_id, profile_semantic_version,\
                     profile_content_hash, resolved_profile_hash, base_profile_hash, profile_source,\
                     mapping_classification, parser_engine_version\
                 ) VALUES(\
                     ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,\
                     ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24\
                 )",
                params![
                    storage_key,
                    inspection.source_file_name,
                    to_sql_integer(inspection.source_file_size)?,
                    inspection.source_modified_ms,
                    now_ms(),
                    PARSER_VERSION,
                    format!(
                        "{}@{}",
                        inspection.compatibility.profile_id,
                        inspection.compatibility.profile_version
                    ),
                    resolution.branch_id,
                    REPUBLIC_SCOPE,
                    inspection.coverage.status.as_str(),
                    inspection.coverage.history_records,
                    inspection.coverage.chartable_records,
                    inspection.coverage.dropped_records,
                    warnings_json,
                    inspection.payload_hash,
                    inspection.interpretation_id,
                    inspection.compatibility.profile_id,
                    inspection.compatibility.profile_version,
                    inspection.compatibility.profile_content_hash,
                    inspection.compatibility.resolved_profile_hash,
                    inspection.compatibility.base_profile_hash,
                    inspection.compatibility.profile_source,
                    inspection.compatibility.mapping_classification,
                    inspection.compatibility.parser_engine_version,
                ],
            )?;

            persist_compacted_history(
                &transaction,
                &storage_key,
                &inspection.records,
                resolution.shared_record_count as usize,
            )?;
            persist_latest_metric_evidence(&transaction, &storage_key, &inspection.records)?;
            persist_snapshots(
                &transaction,
                &storage_key,
                &inspection.snapshots,
                &inspection.records,
            )?;
            for fact in &inspection.binary_facts {
                transaction.execute(
                    "INSERT INTO binary_mapped_facts(\
                         payload_hash, layout_id, record_index, host_slot, value_real, available,\
                         source_offset, evidence_kind\
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 'save_fact')",
                    params![
                        storage_key,
                        fact.layout_id,
                        fact.record_index,
                        fact.host_slot,
                        fact.value,
                        i64::from(fact.value.is_some()),
                        to_sql_integer(fact.source_offset)?,
                    ],
                )?;
            }
            persist_history_signature(&transaction, &storage_key, &inspection.records)?;
            persist_resolution(&transaction, &storage_key, &resolution)?;
            super::analysis_context::record_observation_memberships(
                &transaction,
                &storage_key,
                &inspection.interpretation_id,
                &resolution.branch_id,
                resolution.relationship,
                resolution.parent_payload_hash.as_deref(),
                resolution.shared_record_count,
            )?;
            super::warehouse_jobs::enqueue_projection_job(
                &transaction,
                &format!("observation:{}", inspection.interpretation_id),
                "observation",
                &inspection.interpretation_id,
                now_ms(),
            )?;
        } else {
            let snapshots_exist = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM snapshot_scopes WHERE payload_hash = ?1)",
                [&storage_key],
                |row| row.get::<_, bool>(0),
            )?;
            if !snapshots_exist && !inspection.snapshots.is_empty() {
                persist_snapshots(
                    &transaction,
                    &storage_key,
                    &inspection.snapshots,
                    &inspection.records,
                )?;
            }
        }

        if record_file_observation {
            transaction.execute(
                "INSERT OR IGNORE INTO archive_observations(\
                     payload_hash, source_file_name, source_file_size, source_modified_ms,\
                     observed_at_ms, source_directory_identity\
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    storage_key,
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
                    storage_key,
                    inspection.source_file_name,
                    to_sql_integer(inspection.source_file_size)?,
                    inspection.source_modified_ms,
                ],
            )?;
        }

        transaction.commit()?;
        Ok(!exists)
    }

    pub fn load_latest_dataset(&self) -> Result<Option<ReceiverDataset>, ObservatoryError> {
        self.load_context_dataset()
    }

    pub fn load_dataset(
        &self,
        interpretation_id: &str,
    ) -> Result<ReceiverDataset, ObservatoryError> {
        let connection = self.connect()?;
        self.load_dataset_with_connection(&connection, interpretation_id)
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

    pub(crate) fn load_dataset_with_connection(
        &self,
        connection: &Connection,
        interpretation_id: &str,
    ) -> Result<ReceiverDataset, ObservatoryError> {
        let source = connection.query_row(
            r#"SELECT payload_hash, raw_payload_hash, source_file_name, source_file_size, source_modified_ms, imported_at_ms,
                      parser_version, format_profile, branch_id, geographic_scope, coverage_status,
                      history_records, chartable_records, dropped_records, warnings_json,
                      profile_id, profile_semantic_version, profile_content_hash,
                      resolved_profile_hash, base_profile_hash, profile_source,
                      mapping_classification, parser_engine_version
               FROM observation_sources WHERE interpretation_id = ?1"#,
            [interpretation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, u32>(11)?,
                    row.get::<_, u32>(12)?,
                    row.get::<_, u32>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, String>(16)?,
                    row.get::<_, String>(17)?,
                    row.get::<_, String>(18)?,
                    row.get::<_, Option<String>>(19)?,
                    row.get::<_, String>(20)?,
                    row.get::<_, String>(21)?,
                    row.get::<_, String>(22)?,
                ))
            },
        )?;
        let warnings =
            serde_json::from_str(&source.14).map_err(|_| ObservatoryError::StorageUnavailable)?;
        let coverage = CoverageReport {
            status: if source.10 == "complete" {
                CoverageStatus::Complete
            } else {
                CoverageStatus::Partial
            },
            history_records: source.11,
            chartable_records: source.12,
            dropped_records: source.13,
            warnings,
        };

        let points = load_history(connection, &source.0)?
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
            self.load_metric_evidence(connection, &source.0)?
        };

        Ok(ReceiverDataset {
            payload_hash: source.1,
            interpretation_id: interpretation_id.to_owned(),
            source_file_name: source.2,
            source_file_size: from_sql_integer(source.3)?,
            source_modified_ms: source.4,
            imported_at_ms: source.5,
            parser_version: source.6,
            format_profile: source.7,
            compatibility: CompatibilityProvenance {
                profile_id: source.15,
                profile_version: source.16,
                profile_content_hash: source.17,
                resolved_profile_hash: source.18,
                base_profile_hash: source.19,
                profile_source: source.20,
                mapping_classification: source.21,
                parser_engine_version: source.22,
            },
            branch_id: source.8.clone(),
            original_branch_id: source.8,
            analysis_context_id: None,
            geographic_scope: source.9,
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
