use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

use super::history::load_history;
use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;
use crate::model::{
    AnalysisContext, AnalysisContextMode, AnalysisContextOrigin, BranchMembershipProjection,
    ReceiverDataset,
};

const UNASSIGNED_BRANCH_ID: &str = "unassigned";

pub(crate) fn record_observation_memberships(
    transaction: &Transaction<'_>,
    payload_hash: &str,
    interpretation_id: &str,
    original_branch_id: &str,
    relationship: &str,
    parent_payload_hash: Option<&str>,
    shared_record_count: u32,
) -> Result<(), ObservatoryError> {
    let parent_interpretation_id = parent_payload_hash
        .map(|hash| {
            transaction
                .query_row(
                    "SELECT interpretation_id FROM observation_sources WHERE payload_hash = ?1",
                    [hash],
                    |row| row.get::<_, String>(0),
                )
                .optional()
        })
        .transpose()?
        .flatten();
    insert_membership(
        transaction,
        original_branch_id,
        interpretation_id,
        payload_hash,
        parent_interpretation_id.as_deref(),
        relationship,
        shared_record_count,
    )?;

    let state = transaction.query_row(
        "SELECT selected_branch_id, head_interpretation_id, mode, origin \
         FROM analysis_context_state WHERE singleton_id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;

    if state.1.is_none() || state.0 == UNASSIGNED_BRANCH_ID {
        select_context(
            transaction,
            original_branch_id,
            Some(interpretation_id),
            "latest",
            "automatic",
        )?;
        return Ok(());
    }
    if state.2 != "latest" {
        return Ok(());
    }
    let head = state.1.as_deref().expect("checked above");
    let may_advance = if state.0 == original_branch_id {
        is_strict_descendant(transaction, head, interpretation_id)? || head == interpretation_id
    } else if state.3 == "manual_continuation" {
        is_strict_descendant(transaction, head, interpretation_id)?
    } else {
        false
    };
    if !may_advance {
        return Ok(());
    }
    if state.0 != original_branch_id {
        let head_shared = record_count(transaction, head)?;
        insert_membership(
            transaction,
            &state.0,
            interpretation_id,
            payload_hash,
            Some(head),
            "successor",
            head_shared,
        )?;
    }
    select_context(
        transaction,
        &state.0,
        Some(interpretation_id),
        "latest",
        &state.3,
    )?;
    Ok(())
}

fn insert_membership(
    transaction: &Transaction<'_>,
    branch_id: &str,
    interpretation_id: &str,
    payload_hash: &str,
    parent_interpretation_id: Option<&str>,
    relationship: &str,
    shared_record_count: u32,
) -> Result<(), ObservatoryError> {
    let exists = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM timeline_branch_memberships \
         WHERE branch_id = ?1 AND interpretation_id = ?2)",
        params![branch_id, interpretation_id],
        |row| row.get::<_, bool>(0),
    )?;
    if exists {
        return Ok(());
    }
    transaction.execute(
        "UPDATE timeline_branch_metadata SET membership_revision = membership_revision + 1, \
         updated_at_ms = ?1 WHERE branch_id = ?2",
        params![now_ms(), branch_id],
    )?;
    let revision = transaction.query_row(
        "SELECT membership_revision FROM timeline_branch_metadata WHERE branch_id = ?1",
        [branch_id],
        |row| row.get::<_, u32>(0),
    )?;
    transaction.execute(
        "INSERT INTO timeline_branch_memberships(\
             branch_id, interpretation_id, payload_hash, parent_interpretation_id, relationship,\
             shared_record_count, membership_revision, added_at_ms\
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            branch_id,
            interpretation_id,
            payload_hash,
            parent_interpretation_id,
            relationship,
            shared_record_count,
            revision,
            now_ms(),
        ],
    )?;
    super::warehouse_jobs::enqueue_projection_job(
        transaction,
        &format!("branch_membership:{branch_id}:{revision}"),
        "branch_membership",
        branch_id,
        now_ms(),
    )?;
    Ok(())
}

fn is_strict_descendant(
    connection: &Connection,
    ancestor_interpretation_id: &str,
    candidate_interpretation_id: &str,
) -> Result<bool, ObservatoryError> {
    let ancestor_hash = payload_for_interpretation(connection, ancestor_interpretation_id)?;
    let candidate_hash = payload_for_interpretation(connection, candidate_interpretation_id)?;
    let ancestor = load_history(connection, &ancestor_hash)?;
    let candidate = load_history(connection, &candidate_hash)?;
    Ok(candidate.len() > ancestor.len() && candidate.starts_with(&ancestor))
}

fn payload_for_interpretation(
    connection: &Connection,
    interpretation_id: &str,
) -> Result<String, ObservatoryError> {
    connection
        .query_row(
            "SELECT payload_hash FROM observation_sources WHERE interpretation_id = ?1",
            [interpretation_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(ObservatoryError::UnknownObservation)
}

fn record_count(connection: &Connection, interpretation_id: &str) -> Result<u32, ObservatoryError> {
    connection
        .query_row(
            "SELECT signature.record_count FROM observation_sources source \
             JOIN observation_history_signatures signature \
               ON signature.payload_hash = source.payload_hash \
             WHERE source.interpretation_id = ?1",
            [interpretation_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn select_context(
    connection: &Connection,
    branch_id: &str,
    head: Option<&str>,
    mode: &str,
    origin: &str,
) -> Result<(), ObservatoryError> {
    connection.execute(
        "UPDATE archive_state SET selected_branch_id = ?1 WHERE singleton_id = 1",
        [branch_id],
    )?;
    connection.execute(
        "UPDATE analysis_context_state SET selected_branch_id = ?1, \
         head_interpretation_id = ?2, mode = ?3, origin = ?4, updated_at_ms = ?5 \
         WHERE singleton_id = 1",
        params![branch_id, head, mode, origin, now_ms()],
    )?;
    Ok(())
}

fn branch_tip(
    connection: &Connection,
    branch_id: &str,
) -> Result<Option<String>, ObservatoryError> {
    connection
        .query_row(
            "SELECT membership.interpretation_id \
             FROM timeline_branch_memberships membership \
             JOIN observation_history_signatures signature \
               ON signature.payload_hash = membership.payload_hash \
             WHERE membership.branch_id = ?1 \
             ORDER BY signature.record_count DESC, membership.interpretation_id DESC LIMIT 1",
            [branch_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

pub(crate) fn load_analysis_context_from(
    connection: &Connection,
) -> Result<AnalysisContext, ObservatoryError> {
    let (branch_id, head, mode, origin, revision) = connection.query_row(
        "SELECT state.selected_branch_id, state.head_interpretation_id, state.mode, state.origin, \
                metadata.membership_revision \
         FROM analysis_context_state state \
         JOIN timeline_branch_metadata metadata \
           ON metadata.branch_id = state.selected_branch_id \
         WHERE state.singleton_id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, u32>(4)?,
            ))
        },
    )?;
    let profile = head
        .as_deref()
        .map(|interpretation| {
            connection
                .query_row(
                    "SELECT branch_id, profile_id, resolved_profile_hash \
                     FROM observation_sources WHERE interpretation_id = ?1",
                    [interpretation],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()
        })
        .transpose()?
        .flatten();
    let tip = branch_tip(connection, &branch_id)?;
    let overlay_revision = connection
        .query_row(
            "SELECT CASE WHEN active_profile_id IS NULL THEN NULL \
                    ELSE active_profile_id || ':' || active_revision END \
             FROM planning_overlay_state WHERE singleton_id = 1",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    let mode_value = if mode == "historical_preview" {
        AnalysisContextMode::HistoricalPreview
    } else {
        AnalysisContextMode::Latest
    };
    let origin_value = if origin == "manual_continuation" {
        AnalysisContextOrigin::ManualContinuation
    } else {
        AnalysisContextOrigin::Automatic
    };
    let context_material = format!(
        "{branch_id}|{}|{mode}|{origin}|{revision}",
        head.as_deref().unwrap_or("none")
    );
    let context_id = format!("ctx-{}", hex_digest(context_material.as_bytes()));
    Ok(AnalysisContext {
        context_id: context_id[..20].to_owned(),
        selected_branch_id: branch_id,
        head_interpretation_id: head.clone(),
        original_branch_id: profile.as_ref().map(|value| value.0.clone()),
        mode: mode_value,
        origin: origin_value,
        is_tip: head == tip,
        membership_revision: revision,
        compatibility_profile_id: profile.as_ref().map(|value| value.1.clone()),
        compatibility_profile_hash: profile.as_ref().map(|value| value.2.clone()),
        observation_watermark: head,
        catalogue_generation_id: None,
        resource_catalogue_revision_id: None,
        overlay_revision,
    })
}

impl ObservatoryStorage {
    pub fn load_context_dataset(&self) -> Result<Option<ReceiverDataset>, ObservatoryError> {
        let connection = self.connect()?;
        let context = load_analysis_context_from(&connection)?;
        context
            .head_interpretation_id
            .as_deref()
            .map(|head| {
                let mut dataset = self.load_dataset_with_connection(&connection, head)?;
                dataset.branch_id.clone_from(&context.selected_branch_id);
                dataset.analysis_context_id = Some(context.context_id.clone());
                Ok(dataset)
            })
            .transpose()
    }

    pub fn inspect_observation(&self, interpretation_id: &str) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        let current_branch = load_analysis_context_from(&connection)?.selected_branch_id;
        let branch_id = connection
            .query_row(
                "SELECT membership.branch_id FROM timeline_branch_memberships membership \
                 JOIN observation_sources source USING (interpretation_id) \
                 WHERE membership.interpretation_id = ?1 \
                 ORDER BY CASE WHEN membership.branch_id = ?2 THEN 0 \
                               WHEN membership.branch_id = source.branch_id THEN 1 ELSE 2 END, \
                          membership.branch_id LIMIT 1",
                params![interpretation_id, current_branch],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownObservation)?;
        let origin = branch_origin(&connection, &branch_id)?;
        let tip = branch_tip(&connection, &branch_id)?;
        let mode = if tip.as_deref() == Some(interpretation_id) {
            "latest"
        } else {
            "historical_preview"
        };
        select_context(
            &connection,
            &branch_id,
            Some(interpretation_id),
            mode,
            &origin,
        )
    }

    pub fn return_to_branch_tip(&self) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        let context = load_analysis_context_from(&connection)?;
        let tip = branch_tip(&connection, &context.selected_branch_id)?;
        let origin = branch_origin(&connection, &context.selected_branch_id)?;
        select_context(
            &connection,
            &context.selected_branch_id,
            tip.as_deref(),
            "latest",
            &origin,
        )
    }

    pub fn select_analysis_branch(&self, branch_id: &str) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        let origin = branch_origin(&connection, branch_id)?;
        let tip = branch_tip(&connection, branch_id)?;
        select_context(&connection, branch_id, tip.as_deref(), "latest", &origin)
    }

    pub fn create_continuation(
        &self,
        interpretation_id: &str,
        requested_label: Option<&str>,
    ) -> Result<String, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let current_branch = load_analysis_context_from(&transaction)?.selected_branch_id;
        let parent_branch = transaction
            .query_row(
                "SELECT membership.branch_id \
                 FROM timeline_branch_memberships membership \
                 JOIN observation_sources source USING (interpretation_id) \
                 WHERE membership.interpretation_id = ?1 \
                 ORDER BY CASE WHEN membership.branch_id = ?2 THEN 0 \
                               WHEN membership.branch_id = source.branch_id THEN 1 ELSE 2 END, \
                          membership.branch_id LIMIT 1",
                params![interpretation_id, current_branch],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownObservation)?;
        let payload_hash = payload_for_interpretation(&transaction, interpretation_id)?;
        let ordinal = transaction.query_row(
            "SELECT COUNT(*) + 1 FROM timeline_branch_metadata \
             WHERE origin = 'manual_continuation' AND anchor_interpretation_id = ?1",
            [interpretation_id],
            |row| row.get::<_, u32>(0),
        )?;
        let digest = hex_digest(format!("{interpretation_id}:{ordinal}").as_bytes());
        let branch_id = format!("continuation-{}", &digest[..12]);
        let short_identity = format!("c-{}", &digest[..6]);
        let (year, day, fork_record_id) = transaction.query_row(
            "SELECT node.year, node.day, node.record_id \
             FROM observation_history_tips tip \
             JOIN receiver_history_nodes node ON node.node_id = tip.tip_node_id \
             WHERE tip.payload_hash = ?1",
            [&payload_hash],
            |row| {
                Ok((
                    row.get::<_, i32>(0)?,
                    row.get::<_, u16>(1)?,
                    row.get::<_, u32>(2)?,
                ))
            },
        )?;
        let default_label = format!("Fork from Y{year} D{day:03} · {ordinal}");
        let player_label = validate_label(requested_label.unwrap_or(&default_label))?;
        let timestamp = now_ms();
        transaction.execute(
            "INSERT INTO timeline_branches(\
                 branch_id, branch_kind, created_at_ms, parent_branch_id, fork_record_id\
             ) VALUES(?1, 'fork', ?2, ?3, ?4)",
            params![branch_id, timestamp, parent_branch, fork_record_id],
        )?;
        transaction.execute(
            "INSERT INTO timeline_branch_metadata(\
                 branch_id, origin, short_identity, player_label, anchor_interpretation_id,\
                 membership_revision, created_at_ms, updated_at_ms\
             ) VALUES(?1, 'manual_continuation', ?2, ?3, ?4, 0, ?5, ?5)",
            params![
                branch_id,
                short_identity,
                player_label,
                interpretation_id,
                timestamp
            ],
        )?;
        insert_membership(
            &transaction,
            &branch_id,
            interpretation_id,
            &payload_hash,
            None,
            "continuation_anchor",
            record_count(&transaction, interpretation_id)?,
        )?;
        select_context(
            &transaction,
            &branch_id,
            Some(interpretation_id),
            "latest",
            "manual_continuation",
        )?;
        transaction.commit()?;
        Ok(branch_id)
    }

    pub fn set_branch_label(
        &self,
        branch_id: &str,
        label: Option<&str>,
    ) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        branch_origin(&connection, branch_id)?;
        let label = label.map(validate_label).transpose()?;
        connection.execute(
            "UPDATE timeline_branch_metadata SET player_label = ?1, updated_at_ms = ?2 \
             WHERE branch_id = ?3",
            params![label, now_ms(), branch_id],
        )?;
        Ok(())
    }

    pub(crate) fn branch_membership_projection_at(
        &self,
        branch_id: &str,
        revision: u32,
    ) -> Result<(u32, Vec<BranchMembershipProjection>), ObservatoryError> {
        let connection = self.connect()?;
        let current_revision = connection
            .query_row(
                "SELECT membership_revision FROM timeline_branch_metadata WHERE branch_id = ?1",
                [branch_id],
                |row| row.get::<_, u32>(0),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownBranch)?;
        if revision == 0 || revision > current_revision {
            return Err(ObservatoryError::StorageContractViolation);
        }
        let mut statement = connection.prepare(
            "SELECT interpretation_id, payload_hash, parent_interpretation_id, relationship, \
                    shared_record_count \
             FROM timeline_branch_memberships WHERE branch_id = ?1 AND membership_revision <= ?2 \
             ORDER BY membership_revision, interpretation_id",
        )?;
        let memberships = statement
            .query_map(params![branch_id, revision], |row| {
                Ok(BranchMembershipProjection {
                    branch_id: branch_id.to_owned(),
                    membership_revision: revision,
                    interpretation_id: row.get(0)?,
                    payload_hash: row.get(1)?,
                    parent_interpretation_id: row.get(2)?,
                    relationship: row.get(3)?,
                    shared_record_count: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ObservatoryError::from)?;
        Ok((revision, memberships))
    }
}

fn branch_origin(connection: &Connection, branch_id: &str) -> Result<String, ObservatoryError> {
    connection
        .query_row(
            "SELECT origin FROM timeline_branch_metadata WHERE branch_id = ?1",
            [branch_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(ObservatoryError::UnknownBranch)
}

fn validate_label(label: &str) -> Result<String, ObservatoryError> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 120 || trimmed.chars().any(char::is_control)
    {
        return Err(ObservatoryError::InvalidBranchLabel);
    }
    Ok(trimmed.to_owned())
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
