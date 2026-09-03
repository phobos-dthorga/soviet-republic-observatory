use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

use super::{ObservatoryStorage, now_ms};
use crate::environment::{
    ENVIRONMENT_RECORDING_INTERVAL_GAME_DAYS, ENVIRONMENT_RECORDING_NOTICE_REVISION,
    ENVIRONMENT_STORAGE_CONTRACT_VERSION, calculate_estimate, factor_content_hash,
    generated_factor_set_id,
};
use crate::error::ObservatoryError;
use crate::model::{
    AnalysisContext, CarbonFactorRevision, CarbonFactorSetDraft, CoverageStatus,
    EnvironmentActivityChannel, EnvironmentActivityPoint, EnvironmentActivitySummary,
    EnvironmentDefinitionContext, EnvironmentFacilityReading, EnvironmentHistoryModel,
    EnvironmentLiveState, EnvironmentRecordingStatus, EnvironmentSnapshot,
    EnvironmentSourceAvailability, EnvironmentWarehouseFact, EnvironmentWarehouseProjection,
    EnvironmentWarehouseRecord, EnvironmentWorkspaceModel, ExactObservationReference,
    SaveInspection,
};

const MAX_RETURNED_ACTIVITY_ROWS: u32 = 25_000;

pub(crate) fn persist_environment_data(
    transaction: &Transaction<'_>,
    storage_key: &str,
    inspection: &SaveInspection,
) -> Result<(), ObservatoryError> {
    let data = &inspection.environment;
    for (ordinal, record) in data.records.iter().enumerate() {
        let record_hash = environment_record_hash(record, data);
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO environment_records( \
                 record_hash, record_id, year, day, game_day \
             ) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                record_hash,
                record.record_id,
                record.year,
                record.day,
                record.game_day
            ],
        )?;
        if inserted > 0 {
            for row in &record.rows {
                let resource = data
                    .resources
                    .get(usize::from(row.resource_index))
                    .ok_or(ObservatoryError::StorageContractViolation)?;
                let source_field = data
                    .source_fields
                    .get(usize::from(row.source_field_index))
                    .ok_or(ObservatoryError::StorageContractViolation)?;
                transaction.execute(
                    "INSERT INTO environment_activity_facts( \
                         record_hash, source_field, source_line, row_ordinal, resource_token, \
                         activity_channel, primary_value, secondary_value, \
                         quantity_is_publishable, mapping_id \
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        record_hash,
                        source_field,
                        row.source_line,
                        row.row_ordinal,
                        resource,
                        row.channel.as_str(),
                        row.primary_value,
                        row.secondary_value,
                        i64::from(row.channel.quantity_is_publishable()),
                        environment_mapping_id(row.channel),
                    ],
                )?;
            }
        }
        transaction.execute(
            "INSERT INTO environment_observation_records(payload_hash, ordinal, record_hash) \
             VALUES(?1, ?2, ?3)",
            params![storage_key, ordinal as u32, record_hash],
        )?;
    }
    let warnings_json =
        serde_json::to_string(&data.warnings).map_err(|_| ObservatoryError::StorageUnavailable)?;
    transaction.execute(
        "INSERT INTO environment_observation_coverage( \
             payload_hash, storage_contract_version, coverage_status, history_records, \
             stored_records, row_count, warnings_json \
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            storage_key,
            ENVIRONMENT_STORAGE_CONTRACT_VERSION,
            data.coverage_status().as_str(),
            data.history_records,
            data.records.len().min(u32::MAX as usize) as u32,
            data.row_count,
            warnings_json,
        ],
    )?;
    transaction.execute(
        "INSERT OR IGNORE INTO environment_interpretation_variants( \
             raw_payload_hash, interpretation_id, profile_id, profile_version, \
             resolved_profile_hash, indexed_at_ms \
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
    Ok(())
}

impl ObservatoryStorage {
    pub(crate) fn environment_projection(
        &self,
        interpretation_id: &str,
    ) -> Result<Option<EnvironmentWarehouseProjection>, ObservatoryError> {
        let connection = self.connect()?;
        let source = connection
            .query_row(
                "SELECT source.payload_hash, source.raw_payload_hash, source.branch_id, \
                        source.profile_id, source.profile_semantic_version, \
                        source.resolved_profile_hash, source.mapping_classification, \
                        coverage.storage_contract_version, coverage.stored_records, coverage.row_count \
                 FROM observation_sources source \
                 LEFT JOIN environment_observation_coverage coverage \
                   ON coverage.payload_hash = source.payload_hash \
                 WHERE source.interpretation_id = ?1",
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
                        row.get::<_, Option<u32>>(8)?,
                        row.get::<_, Option<u32>>(9)?,
                    ))
                },
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownObservation)?;
        let Some(contract_version) = source.7 else {
            return Ok(None);
        };
        if contract_version != ENVIRONMENT_STORAGE_CONTRACT_VERSION {
            return Err(ObservatoryError::StorageContractViolation);
        }

        let mut record_statement = connection.prepare(
            "SELECT membership.ordinal, record.record_hash, record.record_id, record.year, \
                    record.day, record.game_day \
             FROM environment_observation_records membership \
             JOIN environment_records record USING(record_hash) \
             WHERE membership.payload_hash = ?1 \
             ORDER BY membership.ordinal",
        )?;
        let records = record_statement
            .query_map([&source.0], |row| {
                Ok(EnvironmentWarehouseRecord {
                    ordinal: row.get(0)?,
                    record_hash: row.get(1)?,
                    record_id: row.get(2)?,
                    year: row.get(3)?,
                    day: row.get(4)?,
                    game_day: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut fact_statement = connection.prepare(
            "SELECT fact.record_hash, fact.source_field, fact.source_line, fact.row_ordinal, \
                    fact.resource_token, fact.activity_channel, fact.primary_value, \
                    fact.secondary_value, fact.quantity_is_publishable, fact.mapping_id \
             FROM environment_observation_records membership \
             JOIN environment_activity_facts fact USING(record_hash) \
             WHERE membership.payload_hash = ?1 \
             ORDER BY membership.ordinal, fact.source_field, fact.source_line, fact.row_ordinal",
        )?;
        let facts = fact_statement
            .query_map([&source.0], |row| {
                let source_line = row.get::<_, i64>(2)?;
                Ok(EnvironmentWarehouseFact {
                    record_hash: row.get(0)?,
                    source_field: row.get(1)?,
                    source_line: u64::try_from(source_line)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, source_line))?,
                    row_ordinal: row.get(3)?,
                    resource_token: row.get(4)?,
                    activity_channel: row.get(5)?,
                    primary_value: row.get(6)?,
                    secondary_value: row.get(7)?,
                    quantity_is_publishable: row.get(8)?,
                    mapping_id: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        if records.len() != usize::try_from(source.8.unwrap_or_default()).unwrap_or(usize::MAX)
            || facts.len() != usize::try_from(source.9.unwrap_or_default()).unwrap_or(usize::MAX)
        {
            return Err(ObservatoryError::StorageContractViolation);
        }
        Ok(Some(EnvironmentWarehouseProjection {
            interpretation_id: interpretation_id.to_owned(),
            raw_payload_hash: source.1,
            branch_id: source.2,
            profile_id: source.3,
            profile_version: source.4,
            resolved_profile_hash: source.5,
            mapping_classification: source.6,
            records,
            facts,
        }))
    }

    pub fn environment_history(&self) -> Result<EnvironmentHistoryModel, ObservatoryError> {
        let connection = self.connect()?;
        let recording = load_recording_status(&connection)?;
        let mut statement = connection.prepare(
            "SELECT snapshot_id, session_id, game_day, facility_count, captured_at_ms \
             FROM environment_live_snapshots ORDER BY captured_at_ms DESC LIMIT 101",
        )?;
        let mut snapshots = statement
            .query_map([], |row| {
                Ok(EnvironmentSnapshot {
                    snapshot_id: row.get(0)?,
                    session_id: row.get(1)?,
                    game_day: row.get(2)?,
                    facility_count: row.get(3)?,
                    captured_at_ms: row.get(4)?,
                    readings: Vec::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let truncated = snapshots.len() > 100;
        snapshots.truncate(100);
        Ok(EnvironmentHistoryModel {
            recording,
            snapshots,
            truncated,
        })
    }

    pub fn environment_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<Option<EnvironmentSnapshot>, ObservatoryError> {
        if snapshot_id.len() != 64 || !snapshot_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Ok(None);
        }
        let connection = self.connect()?;
        let Some(mut snapshot) = connection
            .query_row(
                "SELECT snapshot_id, session_id, game_day, facility_count, captured_at_ms \
                 FROM environment_live_snapshots WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| {
                    Ok(EnvironmentSnapshot {
                        snapshot_id: row.get(0)?,
                        session_id: row.get(1)?,
                        game_day: row.get(2)?,
                        facility_count: row.get(3)?,
                        captured_at_ms: row.get(4)?,
                        readings: Vec::new(),
                    })
                },
            )
            .optional()?
        else {
            return Ok(None);
        };
        let mut statement = connection.prepare(
            "SELECT facility_index, position_x, position_z, definition_identity, \
                    pollution_value, radiation_value, water_amount, water_capacity, water_quality, \
                    sewage_amount, sewage_capacity, sewage_quality \
             FROM environment_facility_readings WHERE snapshot_id = ?1 \
             ORDER BY facility_index LIMIT 25000",
        )?;
        snapshot.readings = statement
            .query_map([snapshot_id], |row| {
                Ok(EnvironmentFacilityReading {
                    facility_index: row.get(0)?,
                    position_x: row.get(1)?,
                    position_z: row.get(2)?,
                    definition_identity: row.get(3)?,
                    pollution_value: row.get(4)?,
                    radiation_value: row.get(5)?,
                    water_amount: row.get(6)?,
                    water_capacity: row.get(7)?,
                    water_quality: row.get(8)?,
                    sewage_amount: row.get(9)?,
                    sewage_capacity: row.get(10)?,
                    sewage_quality: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(snapshot))
    }

    pub(crate) fn environment_coverage_exists(
        &self,
        interpretation_id: &str,
    ) -> Result<bool, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM observation_sources source \
                 JOIN environment_observation_coverage coverage ON coverage.payload_hash = source.payload_hash \
                 WHERE source.interpretation_id = ?1 AND coverage.storage_contract_version = ?2)",
                params![interpretation_id, ENVIRONMENT_STORAGE_CONTRACT_VERSION],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn cached_environment_variant_count(
        &self,
        raw_payload_hash: &str,
        resolved_profile_hash: &str,
    ) -> Result<Option<(u32, u32)>, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT coverage.stored_records, coverage.row_count FROM observation_sources source \
                 JOIN environment_observation_coverage coverage ON coverage.payload_hash = source.payload_hash \
                 WHERE source.raw_payload_hash = ?1 AND source.resolved_profile_hash = ?2 \
                   AND coverage.storage_contract_version = ?3 LIMIT 1",
                params![raw_payload_hash, resolved_profile_hash, ENVIRONMENT_STORAGE_CONTRACT_VERSION],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn environment_workspace(
        &self,
        analysis_context: AnalysisContext,
    ) -> Result<EnvironmentWorkspaceModel, ObservatoryError> {
        let connection = self.connect()?;
        let recording = load_recording_status(&connection)?;
        let factor_sets = load_factor_revisions(&connection)?;
        let selected_factor = factor_sets.iter().find(|revision| revision.selected);
        let Some(interpretation_id) = analysis_context.head_interpretation_id.as_deref() else {
            return Ok(empty_workspace(analysis_context, recording, factor_sets));
        };
        let source = connection
            .query_row(
                "SELECT source.payload_hash, coverage.storage_contract_version, \
                        coverage.coverage_status, coverage.history_records, coverage.row_count, \
                        coverage.warnings_json \
                 FROM observation_sources source \
                 LEFT JOIN environment_observation_coverage coverage \
                   ON coverage.payload_hash = source.payload_hash \
                 WHERE source.interpretation_id = ?1",
                [interpretation_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<u32>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<u32>>(3)?,
                        row.get::<_, Option<u32>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownObservation)?;
        let Some(contract) = source.1 else {
            return Ok(empty_workspace(analysis_context, recording, factor_sets));
        };
        if contract != ENVIRONMENT_STORAGE_CONTRACT_VERSION {
            return Err(ObservatoryError::StorageContractViolation);
        }
        let coverage_status = match source.2.as_deref() {
            Some("complete") => Some(CoverageStatus::Complete),
            Some("partial") => Some(CoverageStatus::Partial),
            _ => return Err(ObservatoryError::StorageContractViolation),
        };
        let history_records = source.3.unwrap_or_default();
        let row_count = source.4.unwrap_or_default();
        let warnings = serde_json::from_str(source.5.as_deref().unwrap_or("[]"))
            .map_err(|_| ObservatoryError::StorageContractViolation)?;
        let receiver = self.load_dataset_with_connection(&connection, interpretation_id)?;
        let exact = receiver
            .points
            .into_iter()
            .filter_map(|point| {
                point
                    .exact_observation
                    .map(|exact| ((point.record_id, point.game_day), exact))
            })
            .collect::<BTreeMap<_, _>>();
        let activity = load_activity(&connection, &source.0, &exact)?;
        let summaries = load_summaries(&connection, &source.0)?;
        let resources = load_resources(&connection, &source.0)?;
        let quantities = load_latest_quantities(&connection, &source.0)?;
        let carbon_estimate = calculate_estimate(selected_factor, &quantities);
        Ok(EnvironmentWorkspaceModel {
            analysis_context,
            coverage_status,
            history_records,
            row_count,
            returned_rows: activity.len().min(u32::MAX as usize) as u32,
            truncated: row_count > MAX_RETURNED_ACTIVITY_ROWS,
            warnings,
            resources,
            activity,
            summaries,
            source_availability: source_availability(row_count > 0),
            definition_context: EnvironmentDefinitionContext::default(),
            recording,
            factor_sets,
            carbon_estimate,
        })
    }

    pub fn save_carbon_factor_set(
        &self,
        draft: &CarbonFactorSetDraft,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let created_at = now_ms();
        let content_hash = factor_content_hash(draft);
        if let Some(existing) = transaction
            .query_row(
                "SELECT factor_set_id, revision FROM carbon_factor_revisions \
                 WHERE content_hash = ?1 AND removed_at_ms IS NULL",
                [&content_hash],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
            )
            .optional()?
        {
            select_factor(&transaction, &existing.0, existing.1)?;
            transaction.commit()?;
            return Ok(());
        }
        let factor_set_id = draft
            .factor_set_id
            .clone()
            .unwrap_or_else(|| generated_factor_set_id(&draft.name, created_at));
        let revision = transaction.query_row(
            "SELECT COALESCE(MAX(revision), 0) + 1 FROM carbon_factor_revisions \
             WHERE factor_set_id = ?1",
            [&factor_set_id],
            |row| row.get::<_, u32>(0),
        )?;
        let entries_json = serde_json::to_string(&draft.entries)
            .map_err(|_| ObservatoryError::StorageUnavailable)?;
        transaction.execute(
            "INSERT INTO carbon_factor_revisions( \
                 factor_set_id, revision, display_name, accounting_boundary, reason, entries_json, \
                 content_hash, created_at_ms, removed_at_ms \
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
            params![
                factor_set_id,
                revision,
                draft.name,
                draft.accounting_boundary,
                draft.reason,
                entries_json,
                content_hash,
                created_at
            ],
        )?;
        select_factor(&transaction, &factor_set_id, revision)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn select_carbon_factor_set(
        &self,
        factor_set_id: &str,
        revision: u32,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM carbon_factor_revisions \
             WHERE factor_set_id = ?1 AND revision = ?2 AND removed_at_ms IS NULL)",
            params![factor_set_id, revision],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(ObservatoryError::UnknownCarbonFactorSet);
        }
        select_factor(&transaction, factor_set_id, revision)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn rollback_carbon_factor_set(&self, factor_set_id: &str) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        let current = connection
            .query_row(
                "SELECT revision FROM carbon_factor_selection WHERE singleton_id = 1 \
             AND factor_set_id = ?1",
                [factor_set_id],
                |row| row.get::<_, u32>(0),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownCarbonFactorSet)?;
        let previous = connection
            .query_row(
                "SELECT MAX(revision) FROM carbon_factor_revisions WHERE factor_set_id = ?1 \
             AND revision < ?2 AND removed_at_ms IS NULL",
                params![factor_set_id, current],
                |row| row.get::<_, Option<u32>>(0),
            )?
            .ok_or(ObservatoryError::UnknownCarbonFactorSet)?;
        connection.execute(
            "UPDATE carbon_factor_selection SET revision = ?1, selected_at_ms = ?2 \
             WHERE singleton_id = 1",
            params![previous, now_ms()],
        )?;
        Ok(())
    }

    pub fn remove_carbon_factor_set(&self, factor_set_id: &str) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        let selected = connection
            .query_row(
                "SELECT factor_set_id = ?1 FROM carbon_factor_selection WHERE singleton_id = 1",
                [factor_set_id],
                |row| row.get::<_, Option<bool>>(0),
            )?
            .unwrap_or(false);
        if selected {
            return Err(ObservatoryError::InvalidCarbonFactorSet("active_remove"));
        }
        let changed = connection.execute(
            "UPDATE carbon_factor_revisions SET removed_at_ms = ?1 \
             WHERE factor_set_id = ?2 AND removed_at_ms IS NULL",
            params![now_ms(), factor_set_id],
        )?;
        if changed == 0 {
            return Err(ObservatoryError::UnknownCarbonFactorSet);
        }
        Ok(())
    }

    pub fn carbon_factor_revision(
        &self,
        factor_set_id: &str,
        revision: u32,
    ) -> Result<CarbonFactorRevision, ObservatoryError> {
        load_factor_revisions(&self.connect()?)?
            .into_iter()
            .find(|item| item.factor_set_id == factor_set_id && item.revision == revision)
            .ok_or(ObservatoryError::UnknownCarbonFactorSet)
    }

    pub fn set_environment_recording(
        &self,
        enabled: bool,
        accepted_notice_revision: u32,
    ) -> Result<EnvironmentRecordingStatus, ObservatoryError> {
        if enabled && accepted_notice_revision != ENVIRONMENT_RECORDING_NOTICE_REVISION {
            return Err(ObservatoryError::EnvironmentRecordingConsentRequired);
        }
        let connection = self.connect()?;
        connection.execute(
            "UPDATE environment_recording_state SET enabled = ?1, accepted_notice_revision = ?2, \
             updated_at_ms = ?3 WHERE singleton_id = 1",
            params![i64::from(enabled), accepted_notice_revision, now_ms()],
        )?;
        load_recording_status(&connection)
    }

    #[cfg(test)]
    pub fn environment_recording_status(
        &self,
    ) -> Result<EnvironmentRecordingStatus, ObservatoryError> {
        load_recording_status(&self.connect()?)
    }

    pub fn delete_live_environmental_recordings(&self) -> Result<u32, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let count = transaction.query_row(
            "SELECT COUNT(*) FROM environment_live_snapshots",
            [],
            |row| row.get::<_, u32>(0),
        )?;
        transaction.execute("DELETE FROM environment_facility_readings", [])?;
        transaction.execute("DELETE FROM environment_live_snapshots", [])?;
        transaction.execute("DELETE FROM environment_live_sessions", [])?;
        transaction.commit()?;
        Ok(count)
    }
}

fn select_factor(
    transaction: &Transaction<'_>,
    factor_set_id: &str,
    revision: u32,
) -> Result<(), ObservatoryError> {
    transaction.execute(
        "UPDATE carbon_factor_selection SET factor_set_id = ?1, revision = ?2, selected_at_ms = ?3 \
         WHERE singleton_id = 1",
        params![factor_set_id, revision, now_ms()],
    )?;
    Ok(())
}

fn load_factor_revisions(
    connection: &Connection,
) -> Result<Vec<CarbonFactorRevision>, ObservatoryError> {
    let selected = connection.query_row(
        "SELECT factor_set_id, revision FROM carbon_factor_selection WHERE singleton_id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<u32>>(1)?,
            ))
        },
    )?;
    let mut statement = connection.prepare(
        "SELECT factor_set_id, revision, display_name, accounting_boundary, reason, created_at_ms, \
                content_hash, entries_json FROM carbon_factor_revisions \
         WHERE removed_at_ms IS NULL ORDER BY display_name, revision DESC",
    )?;
    statement
        .query_map([], |row| {
            let factor_set_id = row.get::<_, String>(0)?;
            let revision = row.get::<_, u32>(1)?;
            let entries_json = row.get::<_, String>(7)?;
            let entries = serde_json::from_str(&entries_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(CarbonFactorRevision {
                selected: selected.0.as_deref() == Some(factor_set_id.as_str())
                    && selected.1 == Some(revision),
                factor_set_id,
                revision,
                name: row.get(2)?,
                accounting_boundary: row.get(3)?,
                reason: row.get(4)?,
                created_at_ms: row.get(5)?,
                content_hash: row.get(6)?,
                entries,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_recording_status(
    connection: &Connection,
) -> Result<EnvironmentRecordingStatus, ObservatoryError> {
    let (enabled, interval, accepted) = connection.query_row(
        "SELECT enabled, interval_game_days, accepted_notice_revision \
         FROM environment_recording_state WHERE singleton_id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, bool>(0)?,
                row.get::<_, u16>(1)?,
                row.get::<_, u32>(2)?,
            ))
        },
    )?;
    let latest = connection
        .query_row(
            "SELECT snapshot_id, game_day, facility_count FROM environment_live_snapshots \
         ORDER BY captured_at_ms DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, u32>(2)?,
                ))
            },
        )
        .optional()?;
    Ok(EnvironmentRecordingStatus {
        enabled,
        interval_game_days: interval.max(ENVIRONMENT_RECORDING_INTERVAL_GAME_DAYS),
        state: if enabled && latest.is_some() {
            EnvironmentLiveState::Ready
        } else if enabled {
            EnvironmentLiveState::WaitingForReviewedFacilityContract
        } else {
            EnvironmentLiveState::Disabled
        },
        notice_revision: accepted,
        latest_snapshot_id: latest.as_ref().map(|value| value.0.clone()),
        latest_game_day: latest.as_ref().map(|value| value.1),
        captured_facilities: latest.map(|value| value.2).unwrap_or_default(),
        detail_code: enabled.then(|| "facility_contract_unavailable".to_owned()),
    })
}

fn load_activity(
    connection: &Connection,
    payload_hash: &str,
    exact: &BTreeMap<(u32, i64), ExactObservationReference>,
) -> Result<Vec<EnvironmentActivityPoint>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT record.record_id, record.year, record.day, record.game_day, recent.resource_token, \
                recent.activity_channel, recent.primary_value, recent.secondary_value, \
                recent.source_field, recent.source_line, recent.row_ordinal, recent.quantity_is_publishable \
         FROM (SELECT membership.ordinal, membership.record_hash, fact.resource_token, fact.activity_channel, \
                      fact.primary_value, fact.secondary_value, fact.source_field, fact.source_line, \
                      fact.row_ordinal, fact.quantity_is_publishable \
               FROM environment_observation_records membership \
               JOIN environment_activity_facts fact USING(record_hash) \
               WHERE membership.payload_hash = ?1 \
               ORDER BY membership.ordinal DESC, fact.source_line DESC \
               LIMIT ?2) recent \
         JOIN environment_records record ON record.record_hash = recent.record_hash \
         ORDER BY record.game_day, recent.source_line, recent.row_ordinal",
    )?;
    statement
        .query_map(params![payload_hash, MAX_RETURNED_ACTIVITY_ROWS], |row| {
            let record_id = row.get::<_, u32>(0)?;
            let game_day = row.get::<_, i64>(3)?;
            let channel = parse_channel(&row.get::<_, String>(5)?)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok(EnvironmentActivityPoint {
                record_id,
                year: row.get(1)?,
                day: row.get(2)?,
                game_day,
                resource_token: row.get(4)?,
                activity_channel: channel,
                primary_value: row.get(6)?,
                secondary_value: row.get(7)?,
                source_field: row.get(8)?,
                source_line: row.get(9)?,
                row_ordinal: row.get(10)?,
                quantity_is_publishable: row.get(11)?,
                exact_observation: exact.get(&(record_id, game_day)).cloned(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_summaries(
    connection: &Connection,
    payload_hash: &str,
) -> Result<Vec<EnvironmentActivitySummary>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT fact.activity_channel, COUNT(*), COUNT(DISTINCT fact.resource_token), \
                CASE WHEN fact.quantity_is_publishable = 1 \
                           AND COUNT(DISTINCT fact.resource_token) = 1 \
                     THEN SUM(fact.primary_value) END, \
                fact.quantity_is_publishable \
         FROM environment_observation_records membership \
         JOIN environment_activity_facts fact USING(record_hash) \
         WHERE membership.payload_hash = ?1 AND membership.ordinal = ( \
             SELECT MAX(ordinal) FROM environment_observation_records WHERE payload_hash = ?1) \
         GROUP BY fact.activity_channel, fact.quantity_is_publishable ORDER BY fact.activity_channel",
    )?;
    statement
        .query_map([payload_hash], |row| {
            let channel_text = row.get::<_, String>(0)?;
            Ok(EnvironmentActivitySummary {
                activity_channel: parse_channel(&channel_text)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                row_count: row.get(1)?,
                resource_count: row.get(2)?,
                latest_recorded_value: row.get(3)?,
                quantity_is_publishable: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_resources(
    connection: &Connection,
    payload_hash: &str,
) -> Result<Vec<String>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT fact.resource_token FROM environment_observation_records membership \
         JOIN environment_activity_facts fact USING(record_hash) WHERE membership.payload_hash = ?1 \
         ORDER BY fact.resource_token LIMIT 4096",
    )?;
    statement
        .query_map([payload_hash], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_latest_quantities(
    connection: &Connection,
    payload_hash: &str,
) -> Result<Vec<(String, EnvironmentActivityChannel, f64, u32)>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT fact.resource_token, fact.activity_channel, SUM(fact.primary_value), COUNT(*) \
         FROM environment_observation_records membership \
         JOIN environment_activity_facts fact USING(record_hash) \
         WHERE membership.payload_hash = ?1 AND fact.quantity_is_publishable = 1 \
           AND fact.primary_value >= 0 AND membership.ordinal = ( \
             SELECT MAX(ordinal) FROM environment_observation_records WHERE payload_hash = ?1) \
         GROUP BY fact.resource_token, fact.activity_channel",
    )?;
    statement
        .query_map([payload_hash], |row| {
            let channel = parse_channel(&row.get::<_, String>(1)?)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok((row.get(0)?, channel, row.get(2)?, row.get(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn empty_workspace(
    analysis_context: AnalysisContext,
    recording: EnvironmentRecordingStatus,
    factor_sets: Vec<CarbonFactorRevision>,
) -> EnvironmentWorkspaceModel {
    EnvironmentWorkspaceModel {
        analysis_context,
        coverage_status: None,
        history_records: 0,
        row_count: 0,
        returned_rows: 0,
        truncated: false,
        warnings: Vec::new(),
        resources: Vec::new(),
        activity: Vec::new(),
        summaries: Vec::new(),
        source_availability: source_availability(false),
        definition_context: EnvironmentDefinitionContext::default(),
        recording,
        factor_sets,
        carbon_estimate: calculate_estimate(None, &[]),
    }
}

fn source_availability(save_activity: bool) -> EnvironmentSourceAvailability {
    EnvironmentSourceAvailability {
        save_activity,
        live_pollution: false,
        live_radiation: false,
        live_water_and_sewage: false,
        spatial_pollution_map: false,
        pollution_units: "W&R pollution units".to_owned(),
        radiation_units: "W&R radioactivity units".to_owned(),
    }
}

fn environment_record_hash(
    record: &crate::model::EnvironmentHistoryRecord,
    data: &crate::model::ParsedEnvironmentData,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"republic-observatory-environment-record-v1\0");
    hasher.update(record.record_id.to_le_bytes());
    hasher.update(record.year.to_le_bytes());
    hasher.update(record.day.to_le_bytes());
    hasher.update(record.game_day.to_le_bytes());
    for row in &record.rows {
        if let (Some(resource), Some(source)) = (
            data.resources.get(usize::from(row.resource_index)),
            data.source_fields.get(usize::from(row.source_field_index)),
        ) {
            hasher.update(resource.as_bytes());
            hasher.update(b"\0");
            hasher.update(source.as_bytes());
            hasher.update(b"\0");
        }
        hasher.update(row.source_line.to_le_bytes());
        hasher.update(row.row_ordinal.to_le_bytes());
        hasher.update(row.channel.as_str().as_bytes());
        hasher.update(row.primary_value.to_bits().to_le_bytes());
        hasher.update(row.secondary_value.to_bits().to_le_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn environment_mapping_id(channel: EnvironmentActivityChannel) -> &'static str {
    match channel {
        EnvironmentActivityChannel::Production => "environment.activity.production",
        EnvironmentActivityChannel::ConstructionUse => "environment.activity.construction_use",
        EnvironmentActivityChannel::FactoryUse => "environment.activity.factory_use",
        EnvironmentActivityChannel::ShopUse => "environment.activity.shop_use",
        EnvironmentActivityChannel::VehicleUse => "environment.activity.vehicle_use",
        EnvironmentActivityChannel::FactoryWaste => "environment.waste.factory",
        EnvironmentActivityChannel::CitizenWaste => "environment.waste.citizen",
        EnvironmentActivityChannel::DemolitionWaste => "environment.waste.demolition",
    }
}

fn parse_channel(value: &str) -> Result<EnvironmentActivityChannel, ObservatoryError> {
    Ok(match value {
        "production" => EnvironmentActivityChannel::Production,
        "construction_use" => EnvironmentActivityChannel::ConstructionUse,
        "factory_use" => EnvironmentActivityChannel::FactoryUse,
        "shop_use" => EnvironmentActivityChannel::ShopUse,
        "vehicle_use" => EnvironmentActivityChannel::VehicleUse,
        "factory_waste" => EnvironmentActivityChannel::FactoryWaste,
        "citizen_waste" => EnvironmentActivityChannel::CitizenWaste,
        "demolition_waste" => EnvironmentActivityChannel::DemolitionWaste,
        _ => return Err(ObservatoryError::StorageContractViolation),
    })
}
