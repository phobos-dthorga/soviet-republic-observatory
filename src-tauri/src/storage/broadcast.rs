use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;
use crate::model::{
    BroadcastEvidenceDataset, BroadcastWarehouseFact, BroadcastWarehouseProjection,
    BroadcastWarehouseRecord, CITIZEN_STATUS_METRICS, CitizenStatusPoint, CoverageReport,
    CoverageStatus, ExactObservationReference, SaveInspection,
};

pub(crate) const BROADCAST_STATUS_STORAGE_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BroadcastPersistenceStats {
    pub records_reused: u32,
}

#[derive(Debug, Default)]
struct LoadedStatusHistory {
    coverage: Option<CoverageReport>,
    points: Vec<CitizenStatusPoint>,
    projection: Option<BroadcastWarehouseProjection>,
}

pub(crate) fn persist_citizen_status_data(
    transaction: &Transaction<'_>,
    storage_key: &str,
    inspection: &SaveInspection,
) -> Result<BroadcastPersistenceStats, ObservatoryError> {
    let data = &inspection.citizen_status;
    let warnings_json =
        serde_json::to_string(&data.warnings).map_err(|_| ObservatoryError::StorageUnavailable)?;
    let mut stats = BroadcastPersistenceStats::default();

    for (ordinal, record) in data.records.iter().enumerate() {
        let record_hash = citizen_status_record_hash(record);
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO citizen_status_records(\
                 record_hash, record_id, year, day, game_day\
             ) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                record_hash,
                record.record_id,
                record.year,
                record.day,
                record.game_day,
            ],
        )?;
        if inserted == 0 {
            stats.records_reused = stats.records_reused.saturating_add(1);
        } else {
            for metric in CITIZEN_STATUS_METRICS {
                let index = usize::from(metric.source_index);
                let source_line = i64::try_from(record.source_lines[index])
                    .map_err(|_| ObservatoryError::StorageContractViolation)?;
                transaction.execute(
                    "INSERT INTO citizen_status_facts(\
                         record_hash, source_index, metric_id, value_real, source_field,\
                         source_line, mapping_id\
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        record_hash,
                        metric.source_index,
                        metric.id,
                        record.values[index],
                        record.source_fields[index],
                        source_line,
                        metric.id,
                    ],
                )?;
            }
        }
        transaction.execute(
            "INSERT OR IGNORE INTO broadcast_status_observation_records(\
                 payload_hash, ordinal, record_hash\
             ) VALUES(?1, ?2, ?3)",
            params![storage_key, ordinal as u32, record_hash],
        )?;
    }

    transaction.execute(
        "INSERT INTO broadcast_status_observation_coverage(\
             payload_hash, storage_contract_version, coverage_status, history_records,\
             stored_records, dropped_records, warnings_json\
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            storage_key,
            BROADCAST_STATUS_STORAGE_CONTRACT_VERSION,
            data.coverage_status().as_str(),
            data.history_records,
            data.records.len().min(u32::MAX as usize) as u32,
            data.dropped_records,
            warnings_json,
        ],
    )?;
    transaction.execute(
        "INSERT OR IGNORE INTO broadcast_status_interpretation_variants(\
             raw_payload_hash, interpretation_id, profile_id, profile_version,\
             resolved_profile_hash, indexed_at_ms\
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            inspection.payload_hash,
            inspection.interpretation_id,
            inspection.compatibility.profile_id,
            inspection.compatibility.profile_version,
            inspection.compatibility.resolved_profile_hash,
            now_ms(),
        ],
    )?;
    Ok(stats)
}

impl ObservatoryStorage {
    pub(crate) fn load_broadcast_evidence(
        &self,
    ) -> Result<BroadcastEvidenceDataset, ObservatoryError> {
        let connection = self.connect()?;
        let analysis_context = super::analysis_context::load_analysis_context_from(&connection)?;
        let Some(interpretation_id) = analysis_context.head_interpretation_id.as_deref() else {
            return Ok(BroadcastEvidenceDataset {
                analysis_context,
                receiver: None,
                status_coverage: None,
                citizen_status_points: Vec::new(),
            });
        };
        let mut receiver = self.load_dataset_with_connection(&connection, interpretation_id)?;
        receiver
            .branch_id
            .clone_from(&analysis_context.selected_branch_id);
        receiver.analysis_context_id = Some(analysis_context.context_id.clone());
        let status_history = load_status_history(&connection, interpretation_id, &receiver)?;
        Ok(BroadcastEvidenceDataset {
            analysis_context,
            receiver: Some(receiver),
            status_coverage: status_history.coverage,
            citizen_status_points: status_history.points,
        })
    }

    pub(crate) fn broadcast_projection(
        &self,
        interpretation_id: &str,
    ) -> Result<Option<BroadcastWarehouseProjection>, ObservatoryError> {
        let connection = self.connect()?;
        let receiver = self.load_dataset_with_connection(&connection, interpretation_id)?;
        Ok(load_status_history(&connection, interpretation_id, &receiver)?.projection)
    }

    pub(crate) fn cached_broadcast_variant_count(
        &self,
        raw_payload_hash: &str,
        resolved_profile_hash: &str,
    ) -> Result<Option<u32>, ObservatoryError> {
        self.connect()?
            .query_row(
                r#"SELECT coverage.stored_records
                   FROM observation_sources source
                   JOIN broadcast_status_observation_coverage coverage
                     ON coverage.payload_hash = source.payload_hash
                  WHERE source.raw_payload_hash = ?1
                    AND source.resolved_profile_hash = ?2
                    AND coverage.storage_contract_version = ?3
                  LIMIT 1"#,
                params![
                    raw_payload_hash,
                    resolved_profile_hash,
                    BROADCAST_STATUS_STORAGE_CONTRACT_VERSION,
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn broadcast_coverage_exists(
        &self,
        interpretation_id: &str,
    ) -> Result<bool, ObservatoryError> {
        let connection = self.connect()?;
        connection
            .query_row(
                r#"SELECT EXISTS(
                       SELECT 1
                         FROM observation_sources source
                         JOIN broadcast_status_observation_coverage coverage
                           ON coverage.payload_hash = source.payload_hash
                        WHERE source.interpretation_id = ?1
                          AND coverage.storage_contract_version = ?2
                   )"#,
                params![interpretation_id, BROADCAST_STATUS_STORAGE_CONTRACT_VERSION],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }
}

fn load_status_history(
    connection: &Connection,
    interpretation_id: &str,
    receiver: &crate::model::ReceiverDataset,
) -> Result<LoadedStatusHistory, ObservatoryError> {
    let source = connection
        .query_row(
            r#"SELECT source.payload_hash, source.raw_payload_hash, source.branch_id,
                      source.profile_id, source.profile_semantic_version,
                      source.resolved_profile_hash, source.mapping_classification,
                      coverage.storage_contract_version,
                      coverage.coverage_status, coverage.history_records, coverage.stored_records,
                      coverage.dropped_records, coverage.warnings_json
               FROM observation_sources source
               LEFT JOIN broadcast_status_observation_coverage coverage
                 ON coverage.payload_hash = source.payload_hash
               WHERE source.interpretation_id = ?1"#,
            [interpretation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<u32>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<u32>>(9)?,
                    row.get::<_, Option<u32>>(10)?,
                    row.get::<_, Option<u32>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                ))
            },
        )
        .optional()?
        .ok_or(ObservatoryError::UnknownObservation)?;
    let Some(contract_version) = source.7 else {
        return Ok(LoadedStatusHistory::default());
    };
    if contract_version != BROADCAST_STATUS_STORAGE_CONTRACT_VERSION {
        return Err(ObservatoryError::StorageContractViolation);
    }
    let warnings = serde_json::from_str(
        source
            .12
            .as_deref()
            .ok_or(ObservatoryError::StorageContractViolation)?,
    )
    .map_err(|_| ObservatoryError::StorageContractViolation)?;
    let coverage = CoverageReport {
        status: match source.8.as_deref() {
            Some("complete") => CoverageStatus::Complete,
            Some("partial") => CoverageStatus::Partial,
            _ => return Err(ObservatoryError::StorageContractViolation),
        },
        history_records: source.9.ok_or(ObservatoryError::StorageContractViolation)?,
        chartable_records: source
            .10
            .ok_or(ObservatoryError::StorageContractViolation)?,
        dropped_records: source
            .11
            .ok_or(ObservatoryError::StorageContractViolation)?,
        warnings,
    };
    let exact_by_identity = receiver
        .points
        .iter()
        .filter_map(|point| {
            point
                .exact_observation
                .clone()
                .map(|exact| ((point.record_id, point.game_day), exact))
        })
        .collect::<BTreeMap<(u32, i64), ExactObservationReference>>();
    let mut statement = connection.prepare(
        r#"SELECT membership.ordinal, record.record_hash, record.record_id, record.year,
                  record.day, record.game_day, fact.source_index, fact.metric_id, fact.value_real,
                  fact.source_field, fact.source_line, fact.mapping_id
           FROM broadcast_status_observation_records membership
           JOIN citizen_status_records record USING(record_hash)
           JOIN citizen_status_facts fact USING(record_hash)
           WHERE membership.payload_hash = ?1
           ORDER BY membership.ordinal, fact.source_index"#,
    )?;
    let rows = statement
        .query_map([&source.0], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, u16>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, u8>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, f64>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, String>(11)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if rows.len() != usize::try_from(coverage.chartable_records).unwrap_or(usize::MAX) * 9 {
        return Err(ObservatoryError::StorageContractViolation);
    }

    let mut points = Vec::new();
    let mut warehouse_records = Vec::new();
    let mut warehouse_facts = Vec::with_capacity(rows.len());
    for chunk in rows.chunks_exact(9) {
        let (ordinal, record_hash, record_id, year, day, game_day, ..) = &chunk[0];
        let mut values = [0.0; 9];
        let mut source_fields = std::array::from_fn(|_| String::new());
        let mut source_lines = [0; 9];
        for row in chunk {
            if row.0 != *ordinal
                || row.1 != *record_hash
                || row.2 != *record_id
                || row.3 != *year
                || row.4 != *day
                || row.5 != *game_day
            {
                return Err(ObservatoryError::StorageContractViolation);
            }
            let index = usize::from(row.6);
            let expected = CITIZEN_STATUS_METRICS
                .get(index)
                .ok_or(ObservatoryError::StorageContractViolation)?;
            if expected.source_index != row.6 || expected.id != row.7 || expected.id != row.11 {
                return Err(ObservatoryError::StorageContractViolation);
            }
            values[index] = row.8;
            source_fields[index].clone_from(&row.9);
            let source_line =
                u64::try_from(row.10).map_err(|_| ObservatoryError::StorageContractViolation)?;
            source_lines[index] = source_line;
            warehouse_facts.push(BroadcastWarehouseFact {
                record_hash: row.1.clone(),
                source_index: row.6,
                metric_id: row.7.clone(),
                value: row.8,
                source_field: row.9.clone(),
                source_line,
                mapping_id: row.11.clone(),
            });
        }
        points.push(CitizenStatusPoint {
            ordinal: *ordinal,
            record_id: *record_id,
            year: *year,
            day: *day,
            game_day: *game_day,
            values,
            source_fields,
            source_lines,
            exact_observation: exact_by_identity.get(&(*record_id, *game_day)).cloned(),
        });
        warehouse_records.push(BroadcastWarehouseRecord {
            record_hash: record_hash.clone(),
            ordinal: *ordinal,
            record_id: *record_id,
            year: *year,
            day: *day,
            game_day: *game_day,
        });
    }
    let projection = BroadcastWarehouseProjection {
        interpretation_id: interpretation_id.to_owned(),
        raw_payload_hash: source.1,
        branch_id: source.2,
        profile_id: source.3,
        profile_version: source.4,
        resolved_profile_hash: source.5,
        mapping_classification: source.6,
        records: warehouse_records,
        facts: warehouse_facts,
    };
    Ok(LoadedStatusHistory {
        coverage: Some(coverage),
        points,
        projection: Some(projection),
    })
}

fn citizen_status_record_hash(record: &crate::model::CitizenStatusRecord) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"republic-observatory-citizen-status-record-v1\0");
    hasher.update(record.record_id.to_le_bytes());
    hasher.update(record.year.to_le_bytes());
    hasher.update(record.day.to_le_bytes());
    hasher.update(record.game_day.to_le_bytes());
    for index in 0..record.values.len() {
        hasher.update([index as u8]);
        hasher.update(record.values[index].to_bits().to_le_bytes());
        hasher.update(record.source_fields[index].as_bytes());
        hasher.update([0]);
        hasher.update(record.source_lines[index].to_le_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
