use std::collections::HashSet;

use crate::error::ObservatoryError;
use crate::metric_catalogue::{PLAN_METRIC_IDS, is_plan_metric, plan_context};
use crate::model::{
    PlanDirection, PlanMetricOption, PlanSeriesPoint, PlanTargetEvaluation, PlanTargetState,
    PopulationDataset, PopulationObservation, RepublicPlanDraft, RepublicPlanEvaluation,
    RepublicPlanRevision, RepublicPlanTarget,
};

pub const MAX_PLAN_TARGETS: usize = 12;
pub const MAX_PLAN_YEARS: i64 = 100;
const DAYS_PER_GAME_YEAR: i64 = 365;

pub fn validate_draft_shape(draft: &RepublicPlanDraft) -> Result<(), ObservatoryError> {
    let name = draft.name.trim();
    if !(1..=120).contains(&name.chars().count()) {
        return Err(ObservatoryError::InvalidRepublicPlan("invalid_name"));
    }
    if draft.end_day >= 365 || !(0..=10_000).contains(&draft.end_year) {
        return Err(ObservatoryError::InvalidRepublicPlan("invalid_end_date"));
    }
    if draft.targets.is_empty() || draft.targets.len() > MAX_PLAN_TARGETS {
        return Err(ObservatoryError::InvalidRepublicPlan(
            "invalid_target_count",
        ));
    }
    let mut metrics = HashSet::new();
    for target in &draft.targets {
        if !is_plan_metric(&target.metric_id) {
            return Err(ObservatoryError::InvalidRepublicPlan("unknown_metric"));
        }
        if !metrics.insert(target.metric_id.as_str()) {
            return Err(ObservatoryError::InvalidRepublicPlan("duplicate_metric"));
        }
        if target.guardrail_basis_points > 5_000 {
            return Err(ObservatoryError::InvalidRepublicPlan("invalid_guardrail"));
        }
    }
    Ok(())
}

pub fn validate_target_direction(target: &RepublicPlanTarget) -> Result<(), ObservatoryError> {
    let valid = match target.direction {
        PlanDirection::Increase => target.target_value > target.baseline_value,
        PlanDirection::Decrease => target.target_value < target.baseline_value,
        PlanDirection::Maintain => target.target_value == target.baseline_value,
    };
    if valid {
        Ok(())
    } else {
        Err(ObservatoryError::InvalidRepublicPlan("direction_mismatch"))
    }
}

pub fn game_day(year: i32, day: u16) -> Result<i64, ObservatoryError> {
    i64::from(year)
        .checked_mul(DAYS_PER_GAME_YEAR)
        .and_then(|value| value.checked_add(i64::from(day)))
        .ok_or(ObservatoryError::InvalidRepublicPlan("invalid_end_date"))
}

pub fn validate_window(start_game_day: i64, end_game_day: i64) -> Result<(), ObservatoryError> {
    let duration = end_game_day
        .checked_sub(start_game_day)
        .ok_or(ObservatoryError::InvalidRepublicPlan("invalid_plan_window"))?;
    if duration <= 0 || duration > DAYS_PER_GAME_YEAR * MAX_PLAN_YEARS {
        return Err(ObservatoryError::InvalidRepublicPlan("invalid_plan_window"));
    }
    Ok(())
}

pub fn metric_value(observation: &PopulationObservation, metric_id: &str) -> Option<u64> {
    if metric_id == crate::metric_catalogue::RECEIVER_TOTAL {
        return [
            crate::metric_catalogue::RECEIVER_NONE,
            crate::metric_catalogue::RECEIVER_RADIO,
            crate::metric_catalogue::RECEIVER_TELEVISION,
            crate::metric_catalogue::RECEIVER_COMPUTER,
        ]
        .iter()
        .try_fold(0_u64, |total, id| {
            total.checked_add(
                observation
                    .facts
                    .iter()
                    .find(|fact| fact.fact_id == *id)?
                    .value,
            )
        });
    }
    observation
        .facts
        .iter()
        .find(|fact| fact.fact_id == metric_id)
        .map(|fact| fact.value)
}

pub fn available_metrics(dataset: &PopulationDataset) -> Vec<PlanMetricOption> {
    let current = dataset.observations.last();
    PLAN_METRIC_IDS
        .iter()
        .filter_map(|metric_id| {
            Some(PlanMetricOption {
                metric_id: (*metric_id).to_owned(),
                current_value: current.and_then(|observation| metric_value(observation, metric_id)),
                active_plan_baseline_value: None,
                context: plan_context(metric_id)?,
            })
        })
        .collect()
}

pub fn evaluate(
    revision: RepublicPlanRevision,
    dataset: &PopulationDataset,
) -> RepublicPlanEvaluation {
    let current = dataset.observations.last();
    let profile_matches = current.is_some_and(|observation| {
        observation.resolved_profile_hash == revision.start_profile_hash
    });
    let evaluations = revision
        .targets
        .iter()
        .map(|target| evaluate_target(&revision, target, dataset, profile_matches))
        .collect::<Vec<_>>();
    let attainment_values = evaluations
        .iter()
        .filter_map(|target| target.attainment_basis_points)
        .map(u64::from)
        .collect::<Vec<_>>();
    let attainment_basis_points = (!attainment_values.is_empty()).then(|| {
        u16::try_from(attainment_values.iter().sum::<u64>() / attainment_values.len() as u64)
            .unwrap_or(u16::MAX)
    });
    let guardrail_breach_count = evaluations
        .iter()
        .filter(|target| target.guardrail_breached)
        .count()
        .min(u32::MAX as usize) as u32;
    let state = aggregate_state(&evaluations);
    RepublicPlanEvaluation {
        revision,
        state,
        attainment_basis_points,
        guardrail_breach_count,
        targets: evaluations,
    }
}

fn evaluate_target(
    revision: &RepublicPlanRevision,
    target: &RepublicPlanTarget,
    dataset: &PopulationDataset,
    profile_matches: bool,
) -> PlanTargetEvaluation {
    let context = plan_context(&target.metric_id)
        .expect("stored plan targets are validated against the host metric catalogue");
    if !profile_matches {
        return PlanTargetEvaluation {
            target: target.clone(),
            current_value: None,
            scheduled_value: None,
            directional_variance: None,
            attainment_basis_points: None,
            guardrail_breached: false,
            state: PlanTargetState::Unavailable,
            context,
            points: Vec::new(),
        };
    }
    let points = dataset
        .observations
        .iter()
        .filter(|observation| {
            observation.sampled_game_day >= revision.start_game_day
                && observation.resolved_profile_hash == revision.start_profile_hash
        })
        .filter_map(|observation| {
            let observed_value = metric_value(observation, &target.metric_id)?;
            Some(PlanSeriesPoint {
                year: observation.sampled_year,
                day: observation.sampled_day,
                game_day: observation.sampled_game_day,
                observed_value,
                scheduled_value: scheduled_value(revision, target, observation.sampled_game_day),
                exact_observation: Some(crate::model::ExactObservationReference {
                    interpretation_id: observation.interpretation_id.clone(),
                    branch_id: dataset.analysis_context.selected_branch_id.clone(),
                    year: observation.sampled_year,
                    day: observation.sampled_day,
                }),
            })
        })
        .collect::<Vec<_>>();
    let current_day = dataset
        .observations
        .last()
        .map(|observation| observation.sampled_game_day);
    if current_day.is_none_or(|day| day < revision.start_game_day) {
        return PlanTargetEvaluation {
            target: target.clone(),
            current_value: None,
            scheduled_value: None,
            directional_variance: None,
            attainment_basis_points: None,
            guardrail_breached: false,
            state: PlanTargetState::AwaitingStart,
            context,
            points,
        };
    }
    let current_day = current_day.expect("checked above");
    let current_value = points.last().map(|point| point.observed_value);
    let scheduled = scheduled_value(revision, target, current_day);
    let variance =
        current_value.and_then(|current| directional_variance(target, current, scheduled));
    let allowed = guardrail_amount(target);
    let guardrail_breached = variance.is_some_and(|value| value < -allowed);
    let state = match (current_value, variance) {
        (None, _) => PlanTargetState::Unavailable,
        (Some(_), Some(_)) if current_day >= revision.end_game_day && !guardrail_breached => {
            PlanTargetState::Complete
        }
        (Some(_), Some(value)) if value > allowed => PlanTargetState::Ahead,
        (Some(_), Some(value)) if value < -allowed => PlanTargetState::Behind,
        (Some(_), Some(_)) => PlanTargetState::OnTrack,
        _ => PlanTargetState::Unavailable,
    };
    PlanTargetEvaluation {
        target: target.clone(),
        current_value,
        scheduled_value: Some(scheduled),
        directional_variance: variance,
        attainment_basis_points: current_value.and_then(|value| attainment(target, value)),
        guardrail_breached,
        state,
        context,
        points,
    }
}

fn scheduled_value(
    revision: &RepublicPlanRevision,
    target: &RepublicPlanTarget,
    game_day: i64,
) -> u64 {
    if game_day <= revision.start_game_day {
        return target.baseline_value;
    }
    if game_day >= revision.end_game_day {
        return target.target_value;
    }
    let elapsed = (game_day - revision.start_game_day) as u128;
    let duration = (revision.end_game_day - revision.start_game_day) as u128;
    let progress = match revision.schedule {
        crate::model::PlanScheduleKind::Linear => (elapsed, duration),
        crate::model::PlanScheduleKind::Milestone => {
            let quarter = elapsed.saturating_mul(4) / duration;
            (quarter.min(3), 4)
        }
        crate::model::PlanScheduleKind::HoldThenChange => {
            if elapsed.saturating_mul(2) <= duration {
                (0, 1)
            } else {
                (elapsed.saturating_mul(2).saturating_sub(duration), duration)
            }
        }
    };
    interpolate(
        target.baseline_value,
        target.target_value,
        progress.0,
        progress.1,
    )
}

fn interpolate(start: u64, end: u64, numerator: u128, denominator: u128) -> u64 {
    if denominator == 0 {
        return end;
    }
    let numerator = numerator.min(denominator);
    if end >= start {
        let delta = u128::from(end - start);
        start.saturating_add((delta.saturating_mul(numerator) / denominator) as u64)
    } else {
        let delta = u128::from(start - end);
        start.saturating_sub((delta.saturating_mul(numerator) / denominator) as u64)
    }
}

fn directional_variance(target: &RepublicPlanTarget, current: u64, scheduled: u64) -> Option<i64> {
    let current = i128::from(current);
    let scheduled = i128::from(scheduled);
    let value = match target.direction {
        PlanDirection::Increase => current - scheduled,
        PlanDirection::Decrease => scheduled - current,
        PlanDirection::Maintain => -(current - scheduled).abs(),
    };
    i64::try_from(value).ok()
}

fn guardrail_amount(target: &RepublicPlanTarget) -> i64 {
    let amount = u128::from(target.target_value)
        .saturating_mul(u128::from(target.guardrail_basis_points))
        / 10_000;
    i64::try_from(amount.max(1)).unwrap_or(i64::MAX)
}

fn attainment(target: &RepublicPlanTarget, current: u64) -> Option<u16> {
    let (progress, span) = match target.direction {
        PlanDirection::Increase => (
            current.saturating_sub(target.baseline_value),
            target.target_value.checked_sub(target.baseline_value)?,
        ),
        PlanDirection::Decrease => (
            target.baseline_value.saturating_sub(current),
            target.baseline_value.checked_sub(target.target_value)?,
        ),
        PlanDirection::Maintain => {
            let deviation = current.abs_diff(target.target_value);
            let allowance = u64::try_from(guardrail_amount(target)).ok()?.max(1);
            let score =
                10_000_u64.saturating_sub(deviation.saturating_mul(10_000).checked_div(allowance)?);
            return u16::try_from(score).ok();
        }
    };
    if span == 0 {
        return None;
    }
    let value = u128::from(progress)
        .saturating_mul(10_000)
        .checked_div(u128::from(span))?
        .min(20_000);
    u16::try_from(value).ok()
}

fn aggregate_state(targets: &[PlanTargetEvaluation]) -> PlanTargetState {
    if targets.is_empty()
        || targets
            .iter()
            .all(|target| target.state == PlanTargetState::Unavailable)
    {
        return PlanTargetState::Unavailable;
    }
    if targets
        .iter()
        .any(|target| target.state == PlanTargetState::Behind)
    {
        return PlanTargetState::Behind;
    }
    if targets
        .iter()
        .all(|target| target.state == PlanTargetState::AwaitingStart)
    {
        return PlanTargetState::AwaitingStart;
    }
    if targets
        .iter()
        .all(|target| target.state == PlanTargetState::Complete)
    {
        return PlanTargetState::Complete;
    }
    if targets
        .iter()
        .any(|target| target.state == PlanTargetState::Ahead)
    {
        return PlanTargetState::Ahead;
    }
    PlanTargetState::OnTrack
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AnalysisContext, AnalysisContextMode, AnalysisContextOrigin, CoverageStatus,
        PlanScheduleKind, PopulationDataset, PopulationFact, PopulationObservation,
        RepublicPlanTarget, TesmioProbeStatus,
    };

    fn revision(schedule: PlanScheduleKind) -> RepublicPlanRevision {
        RepublicPlanRevision {
            plan_id: "plan-test".to_owned(),
            name: "Test plan".to_owned(),
            revision: 1,
            branch_id: "main".to_owned(),
            start_interpretation_id: "start".to_owned(),
            start_profile_hash: "profile".to_owned(),
            start_year: 2000,
            start_day: 0,
            start_game_day: 0,
            end_year: 2001,
            end_day: 0,
            end_game_day: 365,
            schedule,
            created_at_ms: 1,
            targets: Vec::new(),
        }
    }

    fn target() -> RepublicPlanTarget {
        RepublicPlanTarget {
            metric_id: crate::metric_catalogue::ADULTS.to_owned(),
            baseline_value: 100,
            target_value: 200,
            direction: PlanDirection::Increase,
            guardrail_basis_points: 500,
        }
    }

    #[test]
    fn schedules_are_deterministic_and_bounded() {
        assert_eq!(
            scheduled_value(&revision(PlanScheduleKind::Linear), &target(), 182),
            149
        );
        assert_eq!(
            scheduled_value(&revision(PlanScheduleKind::Milestone), &target(), 182),
            125
        );
        assert_eq!(
            scheduled_value(&revision(PlanScheduleKind::HoldThenChange), &target(), 182),
            100
        );
        assert_eq!(
            scheduled_value(&revision(PlanScheduleKind::Linear), &target(), 365),
            200
        );
    }

    #[test]
    fn target_direction_must_match_the_immutable_baseline() {
        assert!(validate_target_direction(&target()).is_ok());
        let mut invalid = target();
        invalid.direction = PlanDirection::Decrease;
        let error = validate_target_direction(&invalid).expect_err("direction mismatch");
        assert_eq!(error.code(), "invalid_republic_plan_direction_mismatch");
    }

    #[test]
    fn evaluation_stops_at_the_selected_head_and_profile_boundary() {
        let mut revision = revision(PlanScheduleKind::Linear);
        revision.targets = vec![target()];
        let mut dataset = plan_dataset();

        let latest = evaluate(revision.clone(), &dataset);
        assert_eq!(latest.targets[0].points.len(), 2);
        assert_eq!(latest.targets[0].current_value, Some(130));

        dataset.observations.pop();
        dataset.analysis_context.mode = AnalysisContextMode::HistoricalPreview;
        let historical = evaluate(revision.clone(), &dataset);
        assert_eq!(historical.targets[0].points.len(), 1);
        assert_eq!(historical.targets[0].current_value, Some(100));

        dataset.observations[0].resolved_profile_hash = "changed".to_owned();
        let incompatible = evaluate(revision, &dataset);
        assert_eq!(incompatible.state, PlanTargetState::Unavailable);
        assert!(incompatible.targets[0].points.is_empty());
    }

    fn plan_dataset() -> PopulationDataset {
        let observation = |id: &str, day: u16, value: u64| PopulationObservation {
            interpretation_id: id.to_owned(),
            source_file_name: format!("{id}.zip"),
            membership_revision: u32::from(day) + 1,
            sampled_year: 2000,
            sampled_day: day,
            sampled_game_day: i64::from(day),
            coverage_status: CoverageStatus::Complete,
            mapping_classification: "reviewed_mapping".to_owned(),
            profile_id: "profile".to_owned(),
            profile_version: "1.0.0".to_owned(),
            resolved_profile_hash: "profile".to_owned(),
            facts: vec![PopulationFact {
                fact_id: crate::metric_catalogue::ADULTS.to_owned(),
                value,
                source_field: "$Citizens_Adults".to_owned(),
                source_line: 1,
            }],
        };
        PopulationDataset {
            analysis_context: AnalysisContext {
                context_id: "context".to_owned(),
                selected_branch_id: "main".to_owned(),
                head_interpretation_id: Some("later".to_owned()),
                original_branch_id: Some("main".to_owned()),
                mode: AnalysisContextMode::Latest,
                origin: AnalysisContextOrigin::Automatic,
                is_tip: true,
                membership_revision: 2,
                compatibility_profile_id: Some("profile".to_owned()),
                compatibility_profile_hash: Some("profile".to_owned()),
                observation_watermark: Some("later".to_owned()),
                catalogue_generation_id: None,
                overlay_revision: None,
            },
            observations: vec![observation("start", 0, 100), observation("later", 100, 130)],
            cities: Vec::new(),
            observation_limit: 256,
            city_limit: 512,
            tesmio_probe: TesmioProbeStatus::not_configured(),
        }
    }
}
