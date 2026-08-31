use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

use super::{ObservatoryStorage, from_sql_integer, now_ms, to_sql_integer};
use crate::error::ObservatoryError;
use crate::model::{
    PlanDirection, PlanScheduleKind, PopulationDataset, RepublicPlanDraft, RepublicPlanListItem,
    RepublicPlanRevision, RepublicPlanTarget, RepublicPlanWorkspace,
};
use crate::republic_plan::{
    available_metrics, evaluate, game_day, validate_draft_shape, validate_target_direction,
    validate_window,
};

impl ObservatoryStorage {
    pub fn republic_plan_workspace(
        &self,
        dataset: &PopulationDataset,
    ) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        let connection = self.connect()?;
        let branch_id = &dataset.analysis_context.selected_branch_id;
        let plans = list_plans(&connection, branch_id)?;
        let active_revision = active_plan_identity(&connection, branch_id)?
            .map(|(plan_id, revision)| load_revision(&connection, &plan_id, revision))
            .transpose()?;
        let baseline_values = active_revision
            .as_ref()
            .map(|revision| load_metric_values(&connection, &revision.start_interpretation_id))
            .transpose()?
            .unwrap_or_default();
        let mut available_metrics = available_metrics(dataset);
        for metric in &mut available_metrics {
            metric.active_plan_baseline_value = baseline_values.get(&metric.metric_id).copied();
        }
        let active_plan = active_revision.map(|revision| evaluate(revision, dataset));
        let current = dataset.observations.last();
        Ok(RepublicPlanWorkspace {
            analysis_context: dataset.analysis_context.clone(),
            current_year: current.map(|observation| observation.sampled_year),
            current_day: current.map(|observation| observation.sampled_day),
            available_metrics,
            plans,
            active_plan,
        })
    }

    pub fn save_republic_plan(
        &self,
        draft: &RepublicPlanDraft,
        dataset: &PopulationDataset,
    ) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        validate_draft_shape(draft)?;
        let current = dataset
            .observations
            .last()
            .ok_or(ObservatoryError::InvalidRepublicPlan(
                "observation_required",
            ))?;
        let end_game_day = game_day(draft.end_year, draft.end_day)?;
        let now = now_ms();
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;

        let (
            plan_id,
            branch_id,
            start_interpretation_id,
            start_profile_hash,
            start_year,
            start_day,
            start_game_day,
        ) = if let Some(plan_id) = draft.plan_id.as_deref() {
            let anchor = transaction
                .query_row(
                    "SELECT plan.branch_id, revision.start_interpretation_id, \
                                revision.start_profile_hash, revision.start_year, \
                                revision.start_day, revision.start_game_day \
                         FROM republic_plans plan \
                         JOIN republic_plan_revisions revision \
                           ON revision.plan_id = plan.plan_id AND revision.revision = 1 \
                         WHERE plan.plan_id = ?1 AND plan.removed_at_ms IS NULL",
                    [plan_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i32>(3)?,
                            row.get::<_, u16>(4)?,
                            row.get::<_, i64>(5)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(ObservatoryError::UnknownRepublicPlan)?;
            if anchor.0 != dataset.analysis_context.selected_branch_id {
                return Err(ObservatoryError::RepublicPlanBranchMismatch);
            }
            (
                plan_id.to_owned(),
                anchor.0,
                anchor.1,
                anchor.2,
                anchor.3,
                anchor.4,
                anchor.5,
            )
        } else {
            (
                plan_identity(
                    &draft.name,
                    &dataset.analysis_context.selected_branch_id,
                    &current.interpretation_id,
                    now,
                ),
                dataset.analysis_context.selected_branch_id.clone(),
                current.interpretation_id.clone(),
                current.resolved_profile_hash.clone(),
                current.sampled_year,
                current.sampled_day,
                current.sampled_game_day,
            )
        };
        validate_window(start_game_day, end_game_day)?;
        let baseline_values = load_metric_values(&transaction, &start_interpretation_id)?;
        let targets = draft
            .targets
            .iter()
            .map(|target| {
                let baseline_value = baseline_values
                    .get(&target.metric_id)
                    .copied()
                    .ok_or(ObservatoryError::InvalidRepublicPlan("metric_unavailable"))?;
                let target = RepublicPlanTarget {
                    metric_id: target.metric_id.clone(),
                    baseline_value,
                    target_value: target.target_value,
                    direction: target.direction,
                    guardrail_basis_points: target.guardrail_basis_points,
                };
                validate_target_direction(&target)?;
                Ok(target)
            })
            .collect::<Result<Vec<_>, ObservatoryError>>()?;
        let revision = transaction.query_row(
            "SELECT COALESCE(MAX(revision), 0) + 1 FROM republic_plan_revisions \
             WHERE plan_id = ?1",
            [&plan_id],
            |row| row.get::<_, u32>(0),
        )?;
        transaction.execute(
            "INSERT INTO republic_plans(\
                 plan_id, display_name, branch_id, active_revision, removed_at_ms, \
                 created_at_ms, updated_at_ms\
             ) VALUES(?1, ?2, ?3, ?4, NULL, ?5, ?5) \
             ON CONFLICT(plan_id) DO UPDATE SET \
                 display_name = excluded.display_name, active_revision = excluded.active_revision, \
                 removed_at_ms = NULL, updated_at_ms = excluded.updated_at_ms",
            params![plan_id, draft.name.trim(), branch_id, revision, now],
        )?;
        transaction.execute(
            "INSERT INTO republic_plan_revisions(\
                 plan_id, revision, display_name, branch_id, start_interpretation_id, \
                 start_profile_hash, start_year, start_day, start_game_day, end_year, end_day, \
                 end_game_day, schedule_kind, created_at_ms\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                plan_id,
                revision,
                draft.name.trim(),
                branch_id,
                start_interpretation_id,
                start_profile_hash,
                start_year,
                start_day,
                start_game_day,
                draft.end_year,
                draft.end_day,
                end_game_day,
                draft.schedule.as_str(),
                now,
            ],
        )?;
        for (ordinal, target) in targets.iter().enumerate() {
            transaction.execute(
                "INSERT INTO republic_plan_targets(\
                     plan_id, revision, ordinal, metric_id, baseline_value, target_value, \
                     direction, guardrail_basis_points\
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    plan_id,
                    revision,
                    ordinal as u32,
                    target.metric_id,
                    to_sql_integer(target.baseline_value)?,
                    to_sql_integer(target.target_value)?,
                    target.direction.as_str(),
                    target.guardrail_basis_points,
                ],
            )?;
        }
        transaction.execute(
            "INSERT INTO active_republic_plans(branch_id, plan_id, revision, selected_at_ms) \
             VALUES(?1, ?2, ?3, ?4) \
             ON CONFLICT(branch_id) DO UPDATE SET \
                 plan_id = excluded.plan_id, revision = excluded.revision, \
                 selected_at_ms = excluded.selected_at_ms",
            params![branch_id, plan_id, revision, now],
        )?;
        transaction.commit()?;
        self.republic_plan_workspace(dataset)
    }

    pub fn activate_republic_plan(
        &self,
        plan_id: &str,
        revision: Option<u32>,
        dataset: &PopulationDataset,
    ) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let (branch_id, latest_revision) = transaction
            .query_row(
                "SELECT branch_id, active_revision FROM republic_plans \
                 WHERE plan_id = ?1 AND removed_at_ms IS NULL",
                [plan_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownRepublicPlan)?;
        if branch_id != dataset.analysis_context.selected_branch_id {
            return Err(ObservatoryError::RepublicPlanBranchMismatch);
        }
        let revision = revision.unwrap_or(latest_revision);
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM republic_plan_revisions \
             WHERE plan_id = ?1 AND revision = ?2)",
            params![plan_id, revision],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(ObservatoryError::UnknownRepublicPlan);
        }
        transaction.execute(
            "UPDATE republic_plans SET active_revision = ?1, updated_at_ms = ?2 \
             WHERE plan_id = ?3",
            params![revision, now_ms(), plan_id],
        )?;
        transaction.execute(
            "INSERT INTO active_republic_plans(branch_id, plan_id, revision, selected_at_ms) \
             VALUES(?1, ?2, ?3, ?4) \
             ON CONFLICT(branch_id) DO UPDATE SET \
                 plan_id = excluded.plan_id, revision = excluded.revision, \
                 selected_at_ms = excluded.selected_at_ms",
            params![branch_id, plan_id, revision, now_ms()],
        )?;
        transaction.commit()?;
        self.republic_plan_workspace(dataset)
    }

    pub fn rollback_republic_plan(
        &self,
        plan_id: &str,
        dataset: &PopulationDataset,
    ) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        let connection = self.connect()?;
        let current =
            active_plan_identity(&connection, &dataset.analysis_context.selected_branch_id)?
                .filter(|identity| identity.0 == plan_id)
                .ok_or(ObservatoryError::UnknownRepublicPlan)?;
        let previous = connection
            .query_row(
                "SELECT MAX(revision) FROM republic_plan_revisions \
                 WHERE plan_id = ?1 AND revision < ?2",
                params![plan_id, current.1],
                |row| row.get::<_, Option<u32>>(0),
            )?
            .ok_or(ObservatoryError::UnknownRepublicPlan)?;
        drop(connection);
        self.activate_republic_plan(plan_id, Some(previous), dataset)
    }

    pub fn remove_republic_plan(
        &self,
        plan_id: &str,
        dataset: &PopulationDataset,
    ) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let updated = transaction.execute(
            "UPDATE republic_plans SET removed_at_ms = ?1, updated_at_ms = ?1 \
             WHERE plan_id = ?2 AND branch_id = ?3 AND removed_at_ms IS NULL",
            params![
                now_ms(),
                plan_id,
                dataset.analysis_context.selected_branch_id
            ],
        )?;
        if updated == 0 {
            return Err(ObservatoryError::UnknownRepublicPlan);
        }
        transaction.execute(
            "DELETE FROM active_republic_plans WHERE branch_id = ?1 AND plan_id = ?2",
            params![dataset.analysis_context.selected_branch_id, plan_id],
        )?;
        transaction.commit()?;
        self.republic_plan_workspace(dataset)
    }
}

fn plan_identity(name: &str, branch_id: &str, interpretation_id: &str, now: i64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.trim().as_bytes());
    hasher.update([0]);
    hasher.update(branch_id.as_bytes());
    hasher.update([0]);
    hasher.update(interpretation_id.as_bytes());
    hasher.update(now.to_le_bytes());
    let digest = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("plan-{}", &digest[..24])
}

fn active_plan_identity(
    connection: &Connection,
    branch_id: &str,
) -> Result<Option<(String, u32)>, ObservatoryError> {
    connection
        .query_row(
            "SELECT active.plan_id, active.revision \
             FROM active_republic_plans active \
             JOIN republic_plans plan ON plan.plan_id = active.plan_id \
             WHERE active.branch_id = ?1 AND plan.removed_at_ms IS NULL",
            [branch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(Into::into)
}

fn list_plans(
    connection: &Connection,
    branch_id: &str,
) -> Result<Vec<RepublicPlanListItem>, ObservatoryError> {
    let active = active_plan_identity(connection, branch_id)?;
    let mut statement = connection.prepare(
        "SELECT plan.plan_id, plan.display_name, plan.branch_id, plan.active_revision, \
                MAX(revision.revision), COUNT(*) \
         FROM republic_plans plan \
         JOIN republic_plan_revisions revision ON revision.plan_id = plan.plan_id \
         WHERE plan.branch_id = ?1 AND plan.removed_at_ms IS NULL \
         GROUP BY plan.plan_id, plan.display_name, plan.branch_id, plan.active_revision \
         ORDER BY plan.updated_at_ms DESC, plan.plan_id",
    )?;
    statement
        .query_map([branch_id], |row| {
            let plan_id = row.get::<_, String>(0)?;
            Ok(RepublicPlanListItem {
                selected: active.as_ref().is_some_and(|value| value.0 == plan_id),
                plan_id,
                name: row.get(1)?,
                branch_id: row.get(2)?,
                active_revision: row.get(3)?,
                latest_revision: row.get(4)?,
                revision_count: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_revision(
    connection: &Connection,
    plan_id: &str,
    revision: u32,
) -> Result<RepublicPlanRevision, ObservatoryError> {
    let mut plan = connection
        .query_row(
            "SELECT display_name, branch_id, start_interpretation_id, start_profile_hash, \
                    start_year, start_day, start_game_day, end_year, end_day, end_game_day, \
                    schedule_kind, created_at_ms \
             FROM republic_plan_revisions WHERE plan_id = ?1 AND revision = ?2",
            params![plan_id, revision],
            |row| {
                Ok(RepublicPlanRevision {
                    plan_id: plan_id.to_owned(),
                    name: row.get(0)?,
                    revision,
                    branch_id: row.get(1)?,
                    start_interpretation_id: row.get(2)?,
                    start_profile_hash: row.get(3)?,
                    start_year: row.get(4)?,
                    start_day: row.get(5)?,
                    start_game_day: row.get(6)?,
                    end_year: row.get(7)?,
                    end_day: row.get(8)?,
                    end_game_day: row.get(9)?,
                    schedule: parse_schedule(&row.get::<_, String>(10)?)?,
                    created_at_ms: row.get(11)?,
                    targets: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or(ObservatoryError::UnknownRepublicPlan)?;
    let mut statement = connection.prepare(
        "SELECT metric_id, baseline_value, target_value, direction, guardrail_basis_points \
         FROM republic_plan_targets WHERE plan_id = ?1 AND revision = ?2 ORDER BY ordinal",
    )?;
    plan.targets = statement
        .query_map(params![plan_id, revision], |row| {
            Ok(RepublicPlanTarget {
                metric_id: row.get(0)?,
                baseline_value: from_sql_integer(row.get(1)?)?,
                target_value: from_sql_integer(row.get(2)?)?,
                direction: parse_direction(&row.get::<_, String>(3)?)?,
                guardrail_basis_points: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(plan)
}

fn load_metric_values(
    connection: &Connection,
    interpretation_id: &str,
) -> Result<HashMap<String, u64>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT fact.fact_id, fact.value_integer \
         FROM observation_sources source \
         JOIN snapshot_scalar_facts fact ON fact.payload_hash = source.payload_hash \
         WHERE source.interpretation_id = ?1 AND fact.scope_kind = 'republic' \
           AND fact.scope_id = 'republic'",
    )?;
    let mut values = statement
        .query_map([interpretation_id], |row| {
            Ok((row.get::<_, String>(0)?, from_sql_integer(row.get(1)?)?))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;
    let receiver_total = [
        crate::metric_catalogue::RECEIVER_NONE,
        crate::metric_catalogue::RECEIVER_RADIO,
        crate::metric_catalogue::RECEIVER_TELEVISION,
        crate::metric_catalogue::RECEIVER_COMPUTER,
    ]
    .iter()
    .try_fold(0_u64, |total, metric_id| {
        total.checked_add(*values.get(*metric_id)?)
    });
    if let Some(total) = receiver_total {
        values.insert(crate::metric_catalogue::RECEIVER_TOTAL.to_owned(), total);
    }
    Ok(values)
}

fn parse_schedule(value: &str) -> Result<PlanScheduleKind, rusqlite::Error> {
    match value {
        "linear" => Ok(PlanScheduleKind::Linear),
        "milestone" => Ok(PlanScheduleKind::Milestone),
        "hold_then_change" => Ok(PlanScheduleKind::HoldThenChange),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_direction(value: &str) -> Result<PlanDirection, rusqlite::Error> {
    match value {
        "increase" => Ok(PlanDirection::Increase),
        "decrease" => Ok(PlanDirection::Decrease),
        "maintain" => Ok(PlanDirection::Maintain),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AnalysisContext, AnalysisContextMode, AnalysisContextOrigin, CoverageStatus,
        PlanTargetDraft, PopulationFact, PopulationObservation, TesmioProbeStatus,
    };
    use tempfile::tempdir;

    fn dataset() -> PopulationDataset {
        let profile = "a".repeat(64);
        PopulationDataset {
            analysis_context: AnalysisContext {
                context_id: "context".to_owned(),
                selected_branch_id: "main".to_owned(),
                head_interpretation_id: Some("head".to_owned()),
                original_branch_id: Some("main".to_owned()),
                mode: AnalysisContextMode::Latest,
                origin: AnalysisContextOrigin::Automatic,
                is_tip: true,
                membership_revision: 1,
                compatibility_profile_id: Some("profile".to_owned()),
                compatibility_profile_hash: Some(profile.clone()),
                observation_watermark: Some("head".to_owned()),
                catalogue_generation_id: None,
                overlay_revision: None,
            },
            observations: vec![PopulationObservation {
                interpretation_id: "head".to_owned(),
                source_file_name: "head.zip".to_owned(),
                membership_revision: 1,
                sampled_year: 2000,
                sampled_day: 0,
                sampled_game_day: 730_000,
                coverage_status: CoverageStatus::Complete,
                mapping_classification: "reviewed_mapping".to_owned(),
                profile_id: "profile".to_owned(),
                profile_version: "1.0.0".to_owned(),
                resolved_profile_hash: profile,
                facts: vec![PopulationFact {
                    fact_id: crate::metric_catalogue::ADULTS.to_owned(),
                    value: 100,
                    source_field: "$Citizens_Adults".to_owned(),
                    source_line: 1,
                }],
            }],
            cities: Vec::new(),
            observation_limit: 256,
            city_limit: 512,
            tesmio_probe: TesmioProbeStatus::not_configured(),
        }
    }

    #[test]
    fn plan_lifecycle_keeps_revisions_and_branch_selection_separate() {
        let directory = tempdir().expect("temporary directory");
        let storage = ObservatoryStorage::initialise(directory.path().join("plans.sqlite3"))
            .expect("storage");
        let connection = storage.connect().expect("connection");
        connection
            .execute(
                "INSERT INTO timeline_branches(branch_id, branch_kind, created_at_ms) \
                 VALUES('main', 'main', 1)",
                [],
            )
            .expect("branch");
        connection
            .execute(
                "INSERT INTO timeline_branch_metadata(\
                     branch_id, origin, short_identity, membership_revision, created_at_ms, updated_at_ms\
                 ) VALUES('main', 'automatic', 'main', 1, 1, 1)",
                [],
            )
            .expect("metadata");
        connection
            .execute(
                "INSERT INTO observation_sources(\
                     payload_hash, raw_payload_hash, interpretation_id, source_file_name, \
                     source_file_size, source_modified_ms, imported_at_ms, parser_version, \
                     format_profile, branch_id, geographic_scope, coverage_status, history_records, \
                     chartable_records, dropped_records, warnings_json, profile_id, \
                     profile_semantic_version, profile_content_hash, resolved_profile_hash, \
                     profile_source, mapping_classification, parser_engine_version\
                 ) VALUES('payload', 'payload', 'head', 'head.zip', 1, 1, 1, 'parser', 'profile', \
                          'main', 'republic', 'complete', 1, 1, 0, '[]', 'profile', '1.0.0', \
                          ?1, ?1, 'reviewed_builtin', 'reviewed_mapping', 'engine')",
                ["a".repeat(64)],
            )
            .expect("source");
        connection
            .execute(
                "INSERT INTO snapshot_scopes(\
                     payload_hash, scope_kind, scope_id, sampled_year, sampled_day, sampled_game_day, \
                     coverage_status, supported_fact_count, expected_fact_count\
                 ) VALUES('payload', 'republic', 'republic', 2000, 0, 730000, 'complete', 1, 1)",
                [],
            )
            .expect("scope");
        connection
            .execute(
                "INSERT INTO snapshot_scalar_facts(\
                     payload_hash, scope_kind, scope_id, fact_id, value_integer, source_field, \
                     source_line, evidence_kind, coverage\
                 ) VALUES('payload', 'republic', 'republic', ?1, 100, '$Citizens_Adults', 1, \
                          'save_fact', 'complete')",
                [crate::metric_catalogue::ADULTS],
            )
            .expect("fact");
        drop(connection);

        let first = storage
            .save_republic_plan(
                &RepublicPlanDraft {
                    plan_id: None,
                    name: "First plan".to_owned(),
                    end_year: 2001,
                    end_day: 0,
                    schedule: PlanScheduleKind::Linear,
                    targets: vec![PlanTargetDraft {
                        metric_id: crate::metric_catalogue::ADULTS.to_owned(),
                        target_value: 120,
                        direction: PlanDirection::Increase,
                        guardrail_basis_points: 500,
                    }],
                },
                &dataset(),
            )
            .expect("first plan");
        let plan_id = first
            .active_plan
            .as_ref()
            .expect("active plan")
            .revision
            .plan_id
            .clone();
        let second = storage
            .save_republic_plan(
                &RepublicPlanDraft {
                    plan_id: Some(plan_id.clone()),
                    name: "First plan revised".to_owned(),
                    end_year: 2002,
                    end_day: 0,
                    schedule: PlanScheduleKind::Milestone,
                    targets: vec![PlanTargetDraft {
                        metric_id: crate::metric_catalogue::ADULTS.to_owned(),
                        target_value: 130,
                        direction: PlanDirection::Increase,
                        guardrail_basis_points: 500,
                    }],
                },
                &dataset(),
            )
            .expect("second revision");
        assert_eq!(second.plans[0].revision_count, 2);
        assert_eq!(
            second
                .active_plan
                .as_ref()
                .expect("active")
                .revision
                .revision,
            2
        );
        let rolled_back = storage
            .rollback_republic_plan(&plan_id, &dataset())
            .expect("rollback");
        assert_eq!(
            rolled_back.active_plan.expect("active").revision.revision,
            1
        );
        let removed = storage
            .remove_republic_plan(&plan_id, &dataset())
            .expect("remove");
        assert!(removed.plans.is_empty());
    }
}
