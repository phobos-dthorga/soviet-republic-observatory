use std::fmt::Write;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

use super::{from_sql_integer, to_sql_integer};
use crate::error::ObservatoryError;
use crate::model::{RECEIVER_METRICS, ReceiverRecord};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryRecord {
    pub record_id: u32,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub none: u64,
    pub radio: u64,
    pub television: u64,
    pub computer: u64,
    pub classified_total: u64,
}

impl From<&ReceiverRecord> for HistoryRecord {
    fn from(record: &ReceiverRecord) -> Self {
        Self {
            record_id: record.record_id,
            year: record.year,
            day: record.day,
            game_day: record.game_day,
            none: record.none,
            radio: record.radio,
            television: record.television,
            computer: record.computer,
            classified_total: record.classified_total,
        }
    }
}

pub(crate) fn history_prefix_fingerprints(records: &[HistoryRecord]) -> Vec<String> {
    let mut hasher = Sha256::new();
    records
        .iter()
        .map(|record| {
            hasher.update(record.record_id.to_le_bytes());
            hasher.update(record.year.to_le_bytes());
            hasher.update(record.day.to_le_bytes());
            hasher.update(record.game_day.to_le_bytes());
            hasher.update(record.none.to_le_bytes());
            hasher.update(record.radio.to_le_bytes());
            hasher.update(record.television.to_le_bytes());
            hasher.update(record.computer.to_le_bytes());
            hasher.update(record.classified_total.to_le_bytes());
            let digest = hasher.clone().finalize();
            let mut fingerprint = String::with_capacity(digest.len() * 2);
            for byte in digest {
                write!(&mut fingerprint, "{byte:02x}").expect("writing to a String cannot fail");
            }
            fingerprint
        })
        .collect()
}

pub(crate) fn persist_compacted_history(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    records: &[ReceiverRecord],
    shared_record_count: usize,
) -> Result<(), ObservatoryError> {
    let history = records.iter().map(HistoryRecord::from).collect::<Vec<_>>();
    persist_history_records(transaction, payload_hash, &history, shared_record_count)
}

fn persist_history_records(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    records: &[HistoryRecord],
    requested_shared_count: usize,
) -> Result<(), ObservatoryError> {
    if records.is_empty() {
        return Err(ObservatoryError::StorageUnavailable);
    }
    let fingerprints = history_prefix_fingerprints(records);
    let mut shared_count = requested_shared_count.min(records.len());
    let mut parent_node_id = if shared_count == 0 {
        None
    } else {
        transaction
            .query_row(
                "SELECT node_id FROM receiver_history_nodes WHERE prefix_fingerprint = ?1",
                [&fingerprints[shared_count - 1]],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
    };
    if shared_count > 0 && parent_node_id.is_none() {
        shared_count = 0;
    }

    for (index, record) in records.iter().enumerate().skip(shared_count) {
        transaction.execute(
            "INSERT OR IGNORE INTO receiver_history_nodes(\
                 parent_node_id, depth, prefix_fingerprint, record_id, year, day, game_day,\
                 classified_total, none_value, radio_value, television_value, computer_value\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                parent_node_id,
                (index + 1).min(u32::MAX as usize) as u32,
                fingerprints[index],
                record.record_id,
                record.year,
                record.day,
                record.game_day,
                to_sql_integer(record.classified_total)?,
                to_sql_integer(record.none)?,
                to_sql_integer(record.radio)?,
                to_sql_integer(record.television)?,
                to_sql_integer(record.computer)?,
            ],
        )?;
        let stored = transaction.query_row(
            "SELECT node_id, parent_node_id, depth FROM receiver_history_nodes \
             WHERE prefix_fingerprint = ?1",
            [&fingerprints[index]],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, u32>(2)? as usize,
                ))
            },
        )?;
        if stored.1 != parent_node_id || stored.2 != index + 1 {
            return Err(ObservatoryError::StorageUnavailable);
        }
        parent_node_id = Some(stored.0);
    }

    let tip_node_id = parent_node_id.ok_or(ObservatoryError::StorageUnavailable)?;
    transaction.execute(
        "INSERT OR REPLACE INTO observation_history_tips(\
             payload_hash, tip_node_id, record_count\
         ) VALUES(?1, ?2, ?3)",
        params![
            payload_hash,
            tip_node_id,
            records.len().min(u32::MAX as usize) as u32
        ],
    )?;
    Ok(())
}

pub(crate) fn persist_latest_metric_evidence(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    records: &[ReceiverRecord],
) -> Result<(), ObservatoryError> {
    let record = records.last().ok_or(ObservatoryError::StorageUnavailable)?;
    let lines = [
        record.source_lines.none,
        record.source_lines.radio,
        record.source_lines.television,
        record.source_lines.computer,
    ];
    let source_fields = [
        record.source_fields.none.as_str(),
        record.source_fields.radio.as_str(),
        record.source_fields.television.as_str(),
        record.source_fields.computer.as_str(),
    ];
    for ((metric, line), source_field) in RECEIVER_METRICS.iter().zip(lines).zip(source_fields) {
        transaction.execute(
            "INSERT OR REPLACE INTO observation_metric_evidence(\
                 payload_hash, metric_id, source_field, latest_source_line\
             ) VALUES(?1, ?2, ?3, ?4)",
            params![payload_hash, metric.id, source_field, to_sql_integer(line)?],
        )?;
    }
    Ok(())
}

pub(crate) fn load_history(
    connection: &Connection,
    payload_hash: &str,
) -> Result<Vec<HistoryRecord>, ObservatoryError> {
    let has_compacted = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM observation_history_tips WHERE payload_hash = ?1)",
        [payload_hash],
        |row| row.get::<_, bool>(0),
    )?;
    if has_compacted {
        return load_compacted_history(connection, payload_hash);
    }
    load_legacy_history(connection, payload_hash)
}

fn load_compacted_history(
    connection: &Connection,
    payload_hash: &str,
) -> Result<Vec<HistoryRecord>, ObservatoryError> {
    let mut statement = connection.prepare(
        r#"WITH RECURSIVE history(
               node_id, parent_node_id, depth, record_id, year, day, game_day,
               classified_total, none_value, radio_value, television_value, computer_value
           ) AS (
               SELECT node.node_id, node.parent_node_id, node.depth, node.record_id,
                      node.year, node.day, node.game_day, node.classified_total,
                      node.none_value, node.radio_value, node.television_value,
                      node.computer_value
               FROM observation_history_tips tip
               JOIN receiver_history_nodes node ON node.node_id = tip.tip_node_id
               WHERE tip.payload_hash = ?1
               UNION ALL
               SELECT parent.node_id, parent.parent_node_id, parent.depth, parent.record_id,
                      parent.year, parent.day, parent.game_day, parent.classified_total,
                      parent.none_value, parent.radio_value, parent.television_value,
                      parent.computer_value
               FROM receiver_history_nodes parent
               JOIN history child ON parent.node_id = child.parent_node_id
           )
           SELECT record_id, year, day, game_day, classified_total, none_value,
                  radio_value, television_value, computer_value
           FROM history ORDER BY depth"#,
    )?;
    statement
        .query_map([payload_hash], history_record_from_row)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_legacy_history(
    connection: &Connection,
    payload_hash: &str,
) -> Result<Vec<HistoryRecord>, ObservatoryError> {
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
    statement
        .query_map(
            params![
                payload_hash,
                RECEIVER_METRICS[0].id,
                RECEIVER_METRICS[1].id,
                RECEIVER_METRICS[2].id,
                RECEIVER_METRICS[3].id,
            ],
            history_record_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn history_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryRecord> {
    Ok(HistoryRecord {
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
}

pub(crate) fn backfill_compacted_histories(
    connection: &mut Connection,
) -> Result<(), ObservatoryError> {
    let hashes = {
        let mut statement = connection.prepare(
            "SELECT os.payload_hash FROM observation_sources os \
             LEFT JOIN observation_history_tips tip ON tip.payload_hash = os.payload_hash \
             WHERE tip.payload_hash IS NULL ORDER BY os.imported_at_ms, os.payload_hash",
        )?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    for hash in hashes {
        let transaction = connection.transaction()?;
        let records = load_legacy_history(&transaction, &hash)?;
        persist_history_records(&transaction, &hash, &records, 0)?;
        backfill_latest_metric_evidence(&transaction, &hash, &records)?;
        transaction.commit()?;
    }
    Ok(())
}

fn backfill_latest_metric_evidence(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    records: &[HistoryRecord],
) -> Result<(), ObservatoryError> {
    let latest_record_id = records
        .last()
        .map(|record| record.record_id)
        .ok_or(ObservatoryError::StorageUnavailable)?;
    transaction.execute(
        "INSERT OR IGNORE INTO observation_metric_evidence(\
             payload_hash, metric_id, source_field, latest_source_line\
         ) SELECT payload_hash, metric_id, source_field, source_line \
           FROM metric_observations WHERE payload_hash = ?1 AND record_id = ?2",
        params![payload_hash, latest_record_id],
    )?;
    Ok(())
}
