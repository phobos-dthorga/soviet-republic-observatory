use rusqlite::{Transaction, params};

use super::to_sql_integer;
use crate::error::ObservatoryError;
use crate::model::{ReceiverRecord, SaveSnapshot};

pub(crate) fn persist_snapshots(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    snapshots: &[SaveSnapshot],
    records: &[ReceiverRecord],
) -> Result<(), ObservatoryError> {
    let sampled = records.last().ok_or(ObservatoryError::StorageUnavailable)?;
    let mut insert_scope = transaction.prepare(
        "INSERT INTO snapshot_scopes(\
             payload_hash, scope_kind, scope_id, sampled_year, sampled_day, sampled_game_day,\
             coverage_status, supported_fact_count, expected_fact_count\
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    let mut insert_fact = transaction.prepare(
        "INSERT INTO snapshot_scalar_facts(\
             payload_hash, scope_kind, scope_id, fact_id, value_integer, source_field,\
             source_line, evidence_kind, coverage\
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 'save_fact', 'complete')",
    )?;

    for snapshot in snapshots {
        let scope_kind = snapshot.scope_kind.as_str();
        insert_scope.execute(params![
            payload_hash,
            scope_kind,
            snapshot.scope_id,
            sampled.year,
            sampled.day,
            sampled.game_day,
            snapshot.coverage.as_str(),
            snapshot.facts.len().min(u32::MAX as usize) as u32,
            snapshot.expected_fact_count,
        ])?;
        for fact in &snapshot.facts {
            insert_fact.execute(params![
                payload_hash,
                scope_kind,
                snapshot.scope_id,
                fact.fact_id,
                to_sql_integer(fact.value)?,
                fact.source_field,
                to_sql_integer(fact.source_line)?,
            ])?;
        }
    }
    Ok(())
}
