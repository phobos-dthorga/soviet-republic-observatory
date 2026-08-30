use rusqlite::{Connection, OptionalExtension};

use super::ObservatoryStorage;
use super::history::{HistoryRecord, load_history};
use crate::error::ObservatoryError;
use crate::model::{
    ArchiveComparison, ComparisonObservation, CoverageStatus, RECEIVER_METRICS, ReceiverClassChange,
};

impl ObservatoryStorage {
    pub fn compare_observations(
        &self,
        from_interpretation_id: &str,
        to_interpretation_id: &str,
    ) -> Result<ArchiveComparison, ObservatoryError> {
        if from_interpretation_id == to_interpretation_id {
            return Err(ObservatoryError::SameObservationComparison);
        }
        let connection = self.connect()?;
        let (from, from_record) = load_comparison_observation(&connection, from_interpretation_id)?;
        let (to, to_record) = load_comparison_observation(&connection, to_interpretation_id)?;
        if from.branch_id != to.branch_id || from.branch_id == "unassigned" {
            return Err(ObservatoryError::IncompatibleComparison);
        }

        let from_values = [
            from_record.none,
            from_record.radio,
            from_record.television,
            from_record.computer,
        ];
        let to_values = [
            to_record.none,
            to_record.radio,
            to_record.television,
            to_record.computer,
        ];
        let receiver_changes = RECEIVER_METRICS
            .iter()
            .zip(from_values)
            .zip(to_values)
            .map(|((metric, from_value), to_value)| {
                receiver_change(metric.id, from_value, to_value)
            })
            .collect();
        let classified_total_change = receiver_change(
            "core.citizens.electronics.classified_total",
            from_record.classified_total,
            to_record.classified_total,
        );

        Ok(ArchiveComparison {
            branch_id: from.branch_id.clone(),
            elapsed_game_days: to.game_day - from.game_day,
            from,
            to,
            receiver_changes,
            classified_total_change,
        })
    }
}

fn load_comparison_observation(
    connection: &Connection,
    interpretation_id: &str,
) -> Result<(ComparisonObservation, HistoryRecord), ObservatoryError> {
    let source = connection
        .query_row(
            "SELECT payload_hash, raw_payload_hash, source_file_name, branch_id, coverage_status \
             FROM observation_sources WHERE interpretation_id = ?1",
            [interpretation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?
        .ok_or(ObservatoryError::UnknownObservation)?;
    let record = load_history(connection, &source.0)?
        .pop()
        .ok_or(ObservatoryError::UnknownObservation)?;
    let snapshot = connection.query_row(
        "SELECT \
             COALESCE((SELECT supported_fact_count FROM snapshot_scopes \
               WHERE payload_hash = ?1 AND scope_kind = 'republic' AND scope_id = 'republic'), 0), \
             (SELECT COUNT(*) FROM snapshot_scopes \
               WHERE payload_hash = ?1 AND scope_kind = 'city'), \
             COALESCE((SELECT SUM(supported_fact_count) FROM snapshot_scopes \
               WHERE payload_hash = ?1 AND scope_kind = 'city'), 0)",
        [&source.0],
        |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
            ))
        },
    )?;
    Ok((
        ComparisonObservation {
            payload_hash: source.1,
            interpretation_id: interpretation_id.to_owned(),
            source_file_name: source.2,
            branch_id: source.3,
            year: record.year,
            day: record.day,
            game_day: record.game_day,
            coverage_status: if source.4 == "complete" {
                CoverageStatus::Complete
            } else {
                CoverageStatus::Partial
            },
            republic_snapshot_fields: snapshot.0,
            city_snapshot_count: snapshot.1,
            city_snapshot_fields: snapshot.2,
        },
        record,
    ))
}

fn receiver_change(metric_id: &str, from_value: u64, to_value: u64) -> ReceiverClassChange {
    ReceiverClassChange {
        metric_id: metric_id.to_owned(),
        from_value,
        to_value,
        delta: to_value as i64 - from_value as i64,
    }
}
