use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::history::{HistoryRecord, history_prefix_fingerprints, load_history};
use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;
use crate::model::{
    ArchiveObservation, ArchiveOverview, CoverageStatus, ReceiverRecord, TimelineBranch,
};

const MAIN_BRANCH_ID: &str = "main";
const UNASSIGNED_BRANCH_ID: &str = "unassigned";

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
    pub parent_payload_hash: Option<String>,
    pub relationship: &'static str,
    pub shared_record_count: u32,
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
        "INSERT OR IGNORE INTO timeline_branch_metadata(\
             branch_id, origin, short_identity, player_label, anchor_interpretation_id,\
             membership_revision, created_at_ms, updated_at_ms\
         ) VALUES(?1, 'automatic', ?2, NULL, NULL, 0, ?3, ?3)",
        params![
            resolution.branch_id,
            resolution.branch_id.chars().take(24).collect::<String>(),
            now_ms(),
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
        let interpretation_id = transaction.query_row(
            "SELECT interpretation_id FROM observation_sources WHERE payload_hash = ?1",
            [&hash],
            |row| row.get::<_, String>(0),
        )?;
        transaction.execute(
            "DELETE FROM timeline_branch_memberships WHERE interpretation_id = ?1",
            [&interpretation_id],
        )?;
        transaction.execute(
            "UPDATE timeline_branch_metadata SET membership_revision = membership_revision + 1, \
             updated_at_ms = ?1 WHERE branch_id = 'unassigned'",
            [now_ms()],
        )?;
        let unassigned_revision = transaction.query_row(
            "SELECT membership_revision FROM timeline_branch_metadata \
             WHERE branch_id = 'unassigned'",
            [],
            |row| row.get::<_, u32>(0),
        )?;
        super::warehouse_jobs::enqueue_projection_job(
            &transaction,
            &format!("branch_membership:unassigned:{unassigned_revision}"),
            "branch_membership",
            "unassigned",
            now_ms(),
        )?;
        transaction.execute(
            "UPDATE observation_sources SET branch_id = ?1 WHERE payload_hash = ?2",
            params![resolution.branch_id, hash],
        )?;
        persist_resolution(&transaction, &hash, &resolution)?;
        super::analysis_context::record_observation_memberships(
            &transaction,
            &hash,
            &interpretation_id,
            &resolution.branch_id,
            resolution.relationship,
            resolution.parent_payload_hash.as_deref(),
            resolution.shared_record_count,
        )?;
        transaction.commit()?;
    }
    Ok(())
}

impl ObservatoryStorage {
    pub fn load_archive_overview(&self) -> Result<ArchiveOverview, ObservatoryError> {
        let connection = self.connect()?;
        let context = super::analysis_context::load_analysis_context_from(&connection)?;
        let selected_branch_id = context.selected_branch_id.clone();
        let file_observation_count = count(&connection, "archive_observations")?;
        let distinct_state_count = count(&connection, "observation_sources")?;
        let unresolved_state_count = connection.query_row(
            "SELECT COUNT(*) FROM observation_sources WHERE branch_id = 'unassigned'",
            [],
            |row| row.get::<_, u32>(0),
        )?;
        let branches = load_branches(&connection, &selected_branch_id)?;
        let mut observations = load_archive_observations(&connection)?;
        mark_context_observations(&connection, &context, &mut observations)?;
        Ok(ArchiveOverview {
            selected_branch_id,
            file_observation_count,
            distinct_state_count,
            unresolved_state_count,
            branches,
            observations,
            analysis_context: context,
        })
    }

    pub fn select_branch(&self, branch_id: &str) -> Result<(), ObservatoryError> {
        self.select_analysis_branch(branch_id)
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

fn load_branches(
    connection: &Connection,
    selected_branch_id: &str,
) -> Result<Vec<TimelineBranch>, ObservatoryError> {
    let rows = {
        let mut statement = connection.prepare(
            "SELECT branch.branch_id, branch.branch_kind, branch.parent_branch_id, \
                    branch.fork_record_id, metadata.origin, metadata.short_identity, \
                    metadata.player_label, metadata.anchor_interpretation_id, \
                    metadata.membership_revision \
             FROM timeline_branches branch \
             JOIN timeline_branch_metadata metadata USING (branch_id) \
             ORDER BY branch.created_at_ms, branch.branch_id",
        )?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<u32>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, u32>(8)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut branches = Vec::new();
    for (
        branch_id,
        branch_kind,
        parent_branch_id,
        fork_record_id,
        origin,
        short_identity,
        player_label,
        anchor_interpretation_id,
        membership_revision,
    ) in rows
    {
        let observation_count = connection.query_row(
            "SELECT COUNT(*) FROM timeline_branch_memberships WHERE branch_id = ?1",
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
            origin: if origin == "manual_continuation" {
                crate::model::AnalysisContextOrigin::ManualContinuation
            } else {
                crate::model::AnalysisContextOrigin::Automatic
            },
            short_identity,
            player_label,
            anchor_interpretation_id,
            membership_revision,
        });
    }
    Ok(branches)
}

fn load_archive_observations(
    connection: &Connection,
) -> Result<Vec<ArchiveObservation>, ObservatoryError> {
    let mut statement = connection.prepare(
        r#"SELECT os.raw_payload_hash, os.interpretation_id, os.source_file_name,
                  os.imported_at_ms, os.branch_id, ol.relationship,
                  (SELECT parent.raw_payload_hash FROM observation_sources parent
                   WHERE parent.payload_hash = ol.parent_payload_hash),
                  ol.shared_record_count,
                  os.history_records, os.coverage_status,
                  (SELECT node.year FROM observation_history_tips tip
                   JOIN receiver_history_nodes node ON node.node_id = tip.tip_node_id
                   WHERE tip.payload_hash = os.payload_hash),
                  (SELECT node.day FROM observation_history_tips tip
                   JOIN receiver_history_nodes node ON node.node_id = tip.tip_node_id
                   WHERE tip.payload_hash = os.payload_hash),
                  (SELECT COUNT(*) FROM archive_observations ao
                   WHERE ao.payload_hash = os.payload_hash),
                  COALESCE((SELECT scope.supported_fact_count FROM snapshot_scopes scope
                   WHERE scope.payload_hash = os.payload_hash
                     AND scope.scope_kind = 'republic' AND scope.scope_id = 'republic'), 0),
                  (SELECT COUNT(*) FROM snapshot_scopes scope
                   WHERE scope.payload_hash = os.payload_hash AND scope.scope_kind = 'city'),
                  COALESCE((SELECT SUM(scope.supported_fact_count) FROM snapshot_scopes scope
                   WHERE scope.payload_hash = os.payload_hash AND scope.scope_kind = 'city'), 0),
                  os.mapping_classification, os.profile_id, os.profile_semantic_version,
                  os.resolved_profile_hash
           FROM observation_sources os
           JOIN observation_lineage ol ON ol.payload_hash = os.payload_hash
           ORDER BY os.imported_at_ms DESC, os.payload_hash DESC
           LIMIT 256"#,
    )?;
    statement
        .query_map([], |row| {
            let coverage = match row.get::<_, String>(9)?.as_str() {
                "complete" => CoverageStatus::Complete,
                _ => CoverageStatus::Partial,
            };
            Ok(ArchiveObservation {
                payload_hash: row.get(0)?,
                interpretation_id: row.get(1)?,
                source_file_name: row.get(2)?,
                imported_at_ms: row.get(3)?,
                branch_id: row.get(4)?,
                relationship: row.get(5)?,
                parent_payload_hash: row.get(6)?,
                shared_record_count: row.get(7)?,
                history_records: row.get(8)?,
                coverage_status: coverage,
                latest_year: row.get(10)?,
                latest_day: row.get(11)?,
                file_observation_count: row.get(12)?,
                republic_snapshot_fields: row.get(13)?,
                city_snapshot_count: row.get(14)?,
                city_snapshot_fields: row.get(15)?,
                mapping_classification: row.get(16)?,
                profile_id: row.get(17)?,
                profile_version: row.get(18)?,
                resolved_profile_hash: row.get(19)?,
                included_in_context: false,
                active_head: false,
                context_sequence: None,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn mark_context_observations(
    connection: &Connection,
    context: &crate::model::AnalysisContext,
    observations: &mut [ArchiveObservation],
) -> Result<(), ObservatoryError> {
    let Some(head) = context.head_interpretation_id.as_deref() else {
        return Ok(());
    };
    let head_revision = connection
        .query_row(
            "SELECT membership_revision FROM timeline_branch_memberships \
             WHERE branch_id = ?1 AND interpretation_id = ?2",
            params![context.selected_branch_id, head],
            |row| row.get::<_, u32>(0),
        )
        .optional()?;
    let Some(head_revision) = head_revision else {
        return Ok(());
    };
    let mut statement = connection.prepare(
        "SELECT interpretation_id, membership_revision FROM timeline_branch_memberships \
         WHERE branch_id = ?1 AND membership_revision <= ?2",
    )?;
    let included = statement
        .query_map(params![context.selected_branch_id, head_revision], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    for observation in observations {
        observation.context_sequence = included.get(&observation.interpretation_id).copied();
        observation.included_in_context = observation.context_sequence.is_some();
        observation.active_head = observation.interpretation_id == head;
    }
    Ok(())
}

fn latest_date_for_branch(
    connection: &Connection,
    branch_id: &str,
) -> Result<Option<(i32, u16)>, ObservatoryError> {
    connection
        .query_row(
            "SELECT node.year, node.day FROM timeline_branch_memberships membership \
             JOIN observation_history_tips tip ON tip.payload_hash = membership.payload_hash \
             JOIN receiver_history_nodes node ON node.node_id = tip.tip_node_id \
             WHERE membership.branch_id = ?1 \
             ORDER BY membership.membership_revision DESC LIMIT 1",
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
