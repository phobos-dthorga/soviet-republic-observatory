use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

use super::{ObservatoryStorage, from_sql_integer, now_ms};
use crate::error::ObservatoryError;
use crate::model::{
    ArchiveObservation, ArchiveOverview, CoverageStatus, RECEIVER_METRICS, ReceiverRecord,
    TimelineBranch,
};

const MAIN_BRANCH_ID: &str = "main";
const UNASSIGNED_BRANCH_ID: &str = "unassigned";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryRecord {
    record_id: u32,
    year: i32,
    day: u16,
    game_day: i64,
    none: u64,
    radio: u64,
    television: u64,
    computer: u64,
    classified_total: u64,
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

#[derive(Debug)]
struct StoredHistorySummary {
    payload_hash: String,
    branch_id: String,
    imported_at_ms: i64,
    record_count: usize,
    tip_fingerprint: String,
}

#[derive(Debug)]
struct StoredHistory {
    branch_id: String,
    records: Vec<HistoryRecord>,
}

#[derive(Debug)]
pub(crate) struct BranchResolution {
    pub branch_id: String,
    branch_kind: &'static str,
    parent_branch_id: Option<String>,
    fork_record_id: Option<u32>,
    parent_payload_hash: Option<String>,
    relationship: &'static str,
    shared_record_count: u32,
}

impl BranchResolution {
    fn main_root() -> Self {
        Self {
            branch_id: MAIN_BRANCH_ID.to_owned(),
            branch_kind: "main",
            parent_branch_id: None,
            fork_record_id: None,
            parent_payload_hash: None,
            relationship: "root",
            shared_record_count: 0,
        }
    }

    fn ambiguous(shared_record_count: usize) -> Self {
        Self {
            branch_id: UNASSIGNED_BRANCH_ID.to_owned(),
            branch_kind: "unassigned",
            parent_branch_id: None,
            fork_record_id: None,
            parent_payload_hash: None,
            relationship: "ambiguous",
            shared_record_count: bounded_count(shared_record_count),
        }
    }

    fn fork(
        payload_hash: &str,
        parent_branch_id: String,
        fork_record_id: Option<u32>,
        relationship: &'static str,
        shared_record_count: usize,
        parent_payload_hash: Option<String>,
    ) -> Self {
        Self {
            branch_id: fork_branch_id(payload_hash),
            branch_kind: "fork",
            parent_branch_id: Some(parent_branch_id),
            fork_record_id,
            parent_payload_hash,
            relationship,
            shared_record_count: bounded_count(shared_record_count),
        }
    }
}

pub(crate) fn resolve_branch(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    records: &[ReceiverRecord],
) -> Result<BranchResolution, ObservatoryError> {
    let incoming = records.iter().map(HistoryRecord::from).collect::<Vec<_>>();
    let prefix_fingerprints = history_prefix_fingerprints(&incoming);
    resolve_history(transaction, payload_hash, &incoming, &prefix_fingerprints)
}

fn resolve_history(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    incoming: &[HistoryRecord],
    prefix_fingerprints: &[String],
) -> Result<BranchResolution, ObservatoryError> {
    let summaries = load_resolved_summaries(transaction, payload_hash)?;
    if summaries.is_empty() {
        return Ok(BranchResolution::main_root());
    }
    let incoming_tip = prefix_fingerprints
        .last()
        .ok_or(ObservatoryError::StorageUnavailable)?;

    let exact = summaries
        .iter()
        .filter(|history| {
            history.record_count == incoming.len() && history.tip_fingerprint == *incoming_tip
        })
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        let Some(branch_id) = unique_summary_branch(&exact) else {
            return Ok(BranchResolution::ambiguous(incoming.len()));
        };
        let parent = latest_summary(exact).expect("non-empty exact histories");
        return Ok(BranchResolution {
            branch_id,
            branch_kind: "main",
            parent_branch_id: None,
            fork_record_id: None,
            parent_payload_hash: Some(parent.payload_hash.clone()),
            relationship: "equivalent_history",
            shared_record_count: bounded_count(incoming.len()),
        });
    }

    let prefix_candidates = summaries
        .iter()
        .filter(|history| summary_is_prefix(history, prefix_fingerprints))
        .collect::<Vec<_>>();
    if let Some(maximum_length) = prefix_candidates
        .iter()
        .map(|history| history.record_count)
        .max()
    {
        let nearest = prefix_candidates
            .into_iter()
            .filter(|history| history.record_count == maximum_length)
            .collect::<Vec<_>>();
        let Some(branch_id) = unique_summary_branch(&nearest) else {
            return Ok(BranchResolution::ambiguous(maximum_length));
        };
        let parent = latest_summary(nearest).expect("non-empty prefix histories");
        let incompatible_successor = summaries.iter().any(|history| {
            history.branch_id == branch_id
                && history.record_count > parent.record_count
                && !summary_is_prefix(history, prefix_fingerprints)
        });
        if incompatible_successor {
            return Ok(BranchResolution::fork(
                payload_hash,
                branch_id,
                incoming
                    .get(maximum_length.saturating_sub(1))
                    .map(|record| record.record_id),
                "rollback_fork",
                maximum_length,
                Some(parent.payload_hash.clone()),
            ));
        }
        return Ok(BranchResolution {
            branch_id,
            branch_kind: "main",
            parent_branch_id: None,
            fork_record_id: None,
            parent_payload_hash: Some(parent.payload_hash.clone()),
            relationship: "successor",
            shared_record_count: bounded_count(maximum_length),
        });
    }

    let branch_tips = load_branch_tips(transaction, &summaries)?;
    let reverse_prefix = branch_tips
        .iter()
        .filter(|history| is_prefix(incoming, &history.records))
        .collect::<Vec<_>>();
    if !reverse_prefix.is_empty() {
        let Some(parent_branch_id) = unique_history_branch(&reverse_prefix) else {
            return Ok(BranchResolution::ambiguous(incoming.len()));
        };
        return Ok(BranchResolution::fork(
            payload_hash,
            parent_branch_id,
            incoming.last().map(|record| record.record_id),
            "rollback_fork",
            incoming.len(),
            None,
        ));
    }

    let maximum_shared = branch_tips
        .iter()
        .map(|history| common_prefix_len(&history.records, incoming))
        .max()
        .unwrap_or_default();
    if maximum_shared == 0 {
        return Ok(BranchResolution::ambiguous(0));
    }
    let divergent = branch_tips
        .iter()
        .filter(|history| common_prefix_len(&history.records, incoming) == maximum_shared)
        .collect::<Vec<_>>();
    let Some(parent_branch_id) = unique_history_branch(&divergent) else {
        return Ok(BranchResolution::ambiguous(maximum_shared));
    };
    Ok(BranchResolution::fork(
        payload_hash,
        parent_branch_id,
        incoming
            .get(maximum_shared.saturating_sub(1))
            .map(|record| record.record_id),
        "divergent_fork",
        maximum_shared,
        None,
    ))
}

pub(crate) fn persist_history_signature(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    records: &[ReceiverRecord],
) -> Result<(), ObservatoryError> {
    let history = records.iter().map(HistoryRecord::from).collect::<Vec<_>>();
    persist_history_signature_records(transaction, payload_hash, &history)
}

fn persist_history_signature_records(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    records: &[HistoryRecord],
) -> Result<(), ObservatoryError> {
    let prefixes = history_prefix_fingerprints(records);
    let tip = prefixes
        .last()
        .ok_or(ObservatoryError::StorageUnavailable)?;
    transaction.execute(
        "INSERT OR REPLACE INTO observation_history_signatures(\
             payload_hash, record_count, tip_fingerprint\
         ) VALUES(?1, ?2, ?3)",
        params![payload_hash, bounded_count(records.len()), tip],
    )?;
    Ok(())
}

fn history_prefix_fingerprints(records: &[HistoryRecord]) -> Vec<String> {
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

pub(crate) fn persist_resolution(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    resolution: &BranchResolution,
) -> Result<(), ObservatoryError> {
    transaction.execute(
        "INSERT OR IGNORE INTO timeline_branches(\
             branch_id, branch_kind, created_at_ms, parent_branch_id, fork_record_id\
         ) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![
            resolution.branch_id,
            resolution.branch_kind,
            now_ms(),
            resolution.parent_branch_id,
            resolution.fork_record_id,
        ],
    )?;
    transaction.execute(
        "INSERT OR REPLACE INTO observation_lineage(\
             payload_hash, parent_payload_hash, relationship, shared_record_count, resolved_at_ms\
         ) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![
            payload_hash,
            resolution.parent_payload_hash,
            resolution.relationship,
            resolution.shared_record_count,
            now_ms(),
        ],
    )?;
    transaction.execute(
        "UPDATE archive_state SET selected_branch_id = ?1 WHERE singleton_id = 1",
        [&resolution.branch_id],
    )?;
    Ok(())
}

pub(crate) fn backfill_missing_history_signatures(
    connection: &mut Connection,
) -> Result<(), ObservatoryError> {
    let hashes = {
        let mut statement = connection.prepare(
            "SELECT os.payload_hash FROM observation_sources os \
             LEFT JOIN observation_history_signatures signature \
               ON signature.payload_hash = os.payload_hash \
             WHERE signature.payload_hash IS NULL \
             ORDER BY os.imported_at_ms, os.payload_hash",
        )?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    for hash in hashes {
        let transaction = connection.transaction()?;
        let records = load_history(&transaction, &hash)?;
        persist_history_signature_records(&transaction, &hash, &records)?;
        transaction.commit()?;
    }
    Ok(())
}

pub(crate) fn reconcile_unassigned_observations(
    connection: &mut Connection,
) -> Result<(), ObservatoryError> {
    let hashes = {
        let mut statement = connection.prepare(
            "SELECT payload_hash FROM observation_sources \
             WHERE branch_id = 'unassigned' ORDER BY imported_at_ms, payload_hash",
        )?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    for hash in hashes {
        let transaction = connection.transaction()?;
        let records = load_history(&transaction, &hash)?;
        let prefixes = history_prefix_fingerprints(&records);
        let resolution = resolve_history(&transaction, &hash, &records, &prefixes)?;
        transaction.execute(
            "UPDATE observation_sources SET branch_id = ?1 WHERE payload_hash = ?2",
            params![resolution.branch_id, hash],
        )?;
        persist_resolution(&transaction, &hash, &resolution)?;
        transaction.commit()?;
    }
    Ok(())
}

impl ObservatoryStorage {
    pub fn load_archive_overview(&self) -> Result<ArchiveOverview, ObservatoryError> {
        let connection = self.connect()?;
        let selected_branch_id = selected_branch_id(&connection)?;
        let file_observation_count = count(&connection, "archive_observations")?;
        let distinct_state_count = count(&connection, "observation_sources")?;
        let unresolved_state_count = connection.query_row(
            "SELECT COUNT(*) FROM observation_sources WHERE branch_id = 'unassigned'",
            [],
            |row| row.get::<_, u32>(0),
        )?;
        let branches = load_branches(&connection, &selected_branch_id)?;
        let observations = load_archive_observations(&connection)?;
        Ok(ArchiveOverview {
            selected_branch_id,
            file_observation_count,
            distinct_state_count,
            unresolved_state_count,
            branches,
            observations,
        })
    }

    pub fn select_branch(&self, branch_id: &str) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        let exists = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM timeline_branches WHERE branch_id = ?1)",
            [branch_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(ObservatoryError::UnknownBranch);
        }
        connection.execute(
            "UPDATE archive_state SET selected_branch_id = ?1 WHERE singleton_id = 1",
            [branch_id],
        )?;
        Ok(())
    }

    pub fn file_observation_count(&self) -> Result<u32, ObservatoryError> {
        let connection = self.connect()?;
        count(&connection, "archive_observations")
    }

    pub fn distinct_state_count(&self) -> Result<u32, ObservatoryError> {
        let connection = self.connect()?;
        count(&connection, "observation_sources")
    }
}

pub(crate) fn selected_branch_id(connection: &Connection) -> Result<String, ObservatoryError> {
    connection
        .query_row(
            "SELECT selected_branch_id FROM archive_state WHERE singleton_id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn load_resolved_summaries(
    connection: &Connection,
    excluded_hash: &str,
) -> Result<Vec<StoredHistorySummary>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT os.payload_hash, os.branch_id, os.imported_at_ms, \
                signature.record_count, signature.tip_fingerprint \
         FROM observation_sources os \
         JOIN observation_history_signatures signature \
           ON signature.payload_hash = os.payload_hash \
         WHERE os.payload_hash <> ?1 AND os.branch_id <> 'unassigned' \
         ORDER BY os.imported_at_ms, os.payload_hash",
    )?;
    statement
        .query_map([excluded_hash], |row| {
            let record_count = row.get::<_, u32>(3)?;
            Ok(StoredHistorySummary {
                payload_hash: row.get(0)?,
                branch_id: row.get(1)?,
                imported_at_ms: row.get(2)?,
                record_count: record_count as usize,
                tip_fingerprint: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_branch_tips(
    connection: &Connection,
    summaries: &[StoredHistorySummary],
) -> Result<Vec<StoredHistory>, ObservatoryError> {
    let mut tips = BTreeMap::<&str, &StoredHistorySummary>::new();
    for summary in summaries {
        let replace = tips.get(summary.branch_id.as_str()).is_none_or(|current| {
            summary.record_count > current.record_count
                || (summary.record_count == current.record_count
                    && summary.imported_at_ms > current.imported_at_ms)
        });
        if replace {
            tips.insert(&summary.branch_id, summary);
        }
    }
    tips.into_values()
        .map(|summary| {
            Ok(StoredHistory {
                branch_id: summary.branch_id.clone(),
                records: load_history(connection, &summary.payload_hash)?,
            })
        })
        .collect()
}

fn load_history(
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
            |row| {
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
            },
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_branches(
    connection: &Connection,
    selected_branch_id: &str,
) -> Result<Vec<TimelineBranch>, ObservatoryError> {
    let rows = {
        let mut statement = connection.prepare(
            "SELECT branch_id, branch_kind, parent_branch_id, fork_record_id \
             FROM timeline_branches ORDER BY created_at_ms, branch_id",
        )?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<u32>>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut branches = Vec::new();
    for (branch_id, branch_kind, parent_branch_id, fork_record_id) in rows {
        let observation_count = connection.query_row(
            "SELECT COUNT(*) FROM observation_sources WHERE branch_id = ?1",
            [&branch_id],
            |row| row.get::<_, u32>(0),
        )?;
        if observation_count == 0 && branch_id != selected_branch_id {
            continue;
        }
        let latest = latest_date_for_branch(connection, &branch_id)?;
        branches.push(TimelineBranch {
            selected: branch_id == selected_branch_id,
            branch_id,
            branch_kind,
            parent_branch_id,
            fork_record_id,
            observation_count,
            latest_year: latest.map(|date| date.0),
            latest_day: latest.map(|date| date.1),
        });
    }
    Ok(branches)
}

fn load_archive_observations(
    connection: &Connection,
) -> Result<Vec<ArchiveObservation>, ObservatoryError> {
    let mut statement = connection.prepare(
        r#"SELECT os.payload_hash, os.source_file_name, os.imported_at_ms, os.branch_id,
                  ol.relationship, ol.parent_payload_hash, ol.shared_record_count,
                  os.history_records, os.coverage_status,
                  (SELECT er.year FROM embedded_records er
                   WHERE er.payload_hash = os.payload_hash
                   ORDER BY er.game_day DESC, er.record_id DESC LIMIT 1),
                  (SELECT er.day FROM embedded_records er
                   WHERE er.payload_hash = os.payload_hash
                   ORDER BY er.game_day DESC, er.record_id DESC LIMIT 1),
                  (SELECT COUNT(*) FROM archive_observations ao
                   WHERE ao.payload_hash = os.payload_hash)
           FROM observation_sources os
           JOIN observation_lineage ol ON ol.payload_hash = os.payload_hash
           ORDER BY os.imported_at_ms DESC, os.payload_hash DESC
           LIMIT 256"#,
    )?;
    statement
        .query_map([], |row| {
            let coverage = match row.get::<_, String>(8)?.as_str() {
                "complete" => CoverageStatus::Complete,
                _ => CoverageStatus::Partial,
            };
            Ok(ArchiveObservation {
                payload_hash: row.get(0)?,
                source_file_name: row.get(1)?,
                imported_at_ms: row.get(2)?,
                branch_id: row.get(3)?,
                relationship: row.get(4)?,
                parent_payload_hash: row.get(5)?,
                shared_record_count: row.get(6)?,
                history_records: row.get(7)?,
                coverage_status: coverage,
                latest_year: row.get(9)?,
                latest_day: row.get(10)?,
                file_observation_count: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn latest_date_for_branch(
    connection: &Connection,
    branch_id: &str,
) -> Result<Option<(i32, u16)>, ObservatoryError> {
    connection
        .query_row(
            "SELECT er.year, er.day FROM embedded_records er \
             JOIN observation_sources os ON os.payload_hash = er.payload_hash \
             WHERE os.branch_id = ?1 \
             ORDER BY er.game_day DESC, er.record_id DESC LIMIT 1",
            [branch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(Into::into)
}

fn count(connection: &Connection, table: &str) -> Result<u32, ObservatoryError> {
    let sql = match table {
        "archive_observations" => "SELECT COUNT(*) FROM archive_observations",
        "observation_sources" => "SELECT COUNT(*) FROM observation_sources",
        _ => return Err(ObservatoryError::StorageUnavailable),
    };
    connection
        .query_row(sql, [], |row| row.get::<_, u32>(0))
        .map_err(Into::into)
}

fn unique_summary_branch(histories: &[&StoredHistorySummary]) -> Option<String> {
    let branches = histories
        .iter()
        .map(|history| history.branch_id.as_str())
        .collect::<BTreeSet<_>>();
    (branches.len() == 1).then(|| branches.into_iter().next().unwrap().to_owned())
}

fn unique_history_branch(histories: &[&StoredHistory]) -> Option<String> {
    let branches = histories
        .iter()
        .map(|history| history.branch_id.as_str())
        .collect::<BTreeSet<_>>();
    (branches.len() == 1).then(|| branches.into_iter().next().unwrap().to_owned())
}

fn latest_summary(histories: Vec<&StoredHistorySummary>) -> Option<&StoredHistorySummary> {
    histories.into_iter().max_by(|left, right| {
        left.imported_at_ms
            .cmp(&right.imported_at_ms)
            .then_with(|| left.payload_hash.cmp(&right.payload_hash))
    })
}

fn summary_is_prefix(summary: &StoredHistorySummary, incoming_prefixes: &[String]) -> bool {
    summary.record_count < incoming_prefixes.len()
        && incoming_prefixes
            .get(summary.record_count.saturating_sub(1))
            .is_some_and(|fingerprint| *fingerprint == summary.tip_fingerprint)
}

fn is_prefix(prefix: &[HistoryRecord], history: &[HistoryRecord]) -> bool {
    prefix.len() < history.len() && history.starts_with(prefix)
}

fn common_prefix_len(left: &[HistoryRecord], right: &[HistoryRecord]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

fn fork_branch_id(payload_hash: &str) -> String {
    format!("fork-{payload_hash}")
}

fn bounded_count(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}
