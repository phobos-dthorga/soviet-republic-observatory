use rusqlite::{OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;
use crate::model::{CITIZEN_STATUS_METRICS, SaveInspection};

pub(crate) const BROADCAST_STATUS_STORAGE_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BroadcastPersistenceStats {
    pub records_reused: u32,
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
    pub(crate) fn cached_broadcast_variant_count(
        &self,
        raw_payload_hash: &str,
        resolved_profile_hash: &str,
    ) -> Result<Option<u32>, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT coverage.stored_records\
                 FROM observation_sources source\
                 JOIN broadcast_status_observation_coverage coverage\
                   ON coverage.payload_hash = source.payload_hash\
                 WHERE source.raw_payload_hash = ?1 AND source.resolved_profile_hash = ?2\
                   AND coverage.storage_contract_version = ?3\
                 LIMIT 1",
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
