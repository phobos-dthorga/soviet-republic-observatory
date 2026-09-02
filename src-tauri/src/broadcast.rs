use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::error::ObservatoryError;
use crate::model::{
    AnalysisContext, BroadcastAvailability, BroadcastMetricDefinition,
    BroadcastOutcomeAvailability, BroadcastOutcomeModel, BroadcastOutcomePair,
    BroadcastOutcomeRequest, BroadcastPulse, BroadcastReceiverClassPulse,
    BroadcastStationRequirement, BroadcastWorkspaceModel, CITIZEN_STATUS_METRICS,
    CitizenStatusPoint, CoverageReport, RECEIVER_METRICS, ReceiverDataset, ReceiverHistoryPoint,
};

const ALLOWED_LAGS: [u8; 5] = [0, 1, 2, 4, 8];
const MINIMUM_PAIR_COUNT: usize = 12;

pub(crate) fn build_workspace(
    analysis_context: AnalysisContext,
    receiver: Option<ReceiverDataset>,
    status_coverage: Option<CoverageReport>,
    citizen_status_points: Vec<CitizenStatusPoint>,
    station_requirements: Vec<BroadcastStationRequirement>,
    warehouse_projection_available: bool,
) -> BroadcastWorkspaceModel {
    let pulse = receiver.as_ref().and_then(build_pulse);
    BroadcastWorkspaceModel {
        analysis_context,
        receiver,
        pulse,
        status_metrics: CITIZEN_STATUS_METRICS
            .iter()
            .map(|metric| BroadcastMetricDefinition {
                metric_id: metric.id.to_owned(),
                source_index: metric.source_index,
            })
            .collect(),
        status_coverage,
        citizen_status_points,
        station_requirements,
        availability: BroadcastAvailability {
            potential_audience: false,
            current_audience: false,
            programme_settings: false,
            demographic_receiver_join: false,
        },
        warehouse_projection_available,
    }
}

pub(crate) fn calculate_outcome(
    workspace: &BroadcastWorkspaceModel,
    request: &BroadcastOutcomeRequest,
) -> Result<BroadcastOutcomeModel, ObservatoryError> {
    let receiver_index = RECEIVER_METRICS
        .iter()
        .position(|metric| metric.id == request.receiver_metric_id)
        .ok_or(ObservatoryError::InvalidBroadcastOutcome(
            "unknown_receiver_metric",
        ))?;
    let status_index = CITIZEN_STATUS_METRICS
        .iter()
        .position(|metric| metric.id == request.status_metric_id)
        .ok_or(ObservatoryError::InvalidBroadcastOutcome(
            "unknown_status_metric",
        ))?;
    if !ALLOWED_LAGS.contains(&request.lag_confirmed_records) {
        return Err(ObservatoryError::InvalidBroadcastOutcome("invalid_lag"));
    }

    let Some(receiver) = workspace.receiver.as_ref() else {
        return Ok(unavailable_outcome(
            request,
            BroadcastOutcomeAvailability::ReceiverUnavailable,
        ));
    };
    if workspace.citizen_status_points.is_empty() {
        return Ok(unavailable_outcome(
            request,
            BroadcastOutcomeAvailability::StatusUnavailable,
        ));
    }

    let status_by_identity = workspace
        .citizen_status_points
        .iter()
        .map(|point| ((point.record_id, point.game_day), point))
        .collect::<BTreeMap<_, _>>();
    let mut deltas = Vec::new();
    for (ordinal, pair) in receiver.points.windows(2).enumerate() {
        let previous = &pair[0];
        let current = &pair[1];
        let Some(previous_status) = status_by_identity
            .get(&(previous.record_id, previous.game_day))
            .copied()
        else {
            continue;
        };
        let Some(current_status) = status_by_identity
            .get(&(current.record_id, current.game_day))
            .copied()
        else {
            continue;
        };
        let end_ordinal = ordinal + 1;
        if usize::try_from(previous_status.ordinal).ok() != Some(ordinal)
            || usize::try_from(current_status.ordinal).ok() != Some(end_ordinal)
        {
            continue;
        }
        let Some(previous_share) = receiver_share(previous, receiver_index) else {
            continue;
        };
        let Some(current_share) = receiver_share(current, receiver_index) else {
            continue;
        };
        deltas.push(AlignedDelta {
            end_ordinal,
            receiver: current,
            status: current_status,
            receiver_change: current_share - previous_share,
            status_change: current_status.values[status_index]
                - previous_status.values[status_index],
        });
    }

    let deltas_by_ordinal = deltas
        .iter()
        .map(|delta| (delta.end_ordinal, delta))
        .collect::<BTreeMap<_, _>>();
    let lag = usize::from(request.lag_confirmed_records);
    let mut pairs = Vec::new();
    for status_delta in &deltas {
        let Some(receiver_ordinal) = status_delta.end_ordinal.checked_sub(lag) else {
            continue;
        };
        let Some(receiver_delta) = deltas_by_ordinal.get(&receiver_ordinal).copied() else {
            continue;
        };
        pairs.push(BroadcastOutcomePair {
            receiver_record_id: receiver_delta.receiver.record_id,
            receiver_year: receiver_delta.receiver.year,
            receiver_day: receiver_delta.receiver.day,
            receiver_game_day: receiver_delta.receiver.game_day,
            status_record_id: status_delta.status.record_id,
            status_year: status_delta.status.year,
            status_day: status_delta.status.day,
            status_game_day: status_delta.status.game_day,
            elapsed_game_days: status_delta.status.game_day - receiver_delta.receiver.game_day,
            receiver_share_change: receiver_delta.receiver_change,
            status_change: status_delta.status_change,
            exact_observation: status_delta.status.exact_observation.clone(),
        });
    }

    let availability = if pairs.len() < MINIMUM_PAIR_COUNT {
        BroadcastOutcomeAvailability::InsufficientPairs
    } else if is_constant(pairs.iter().map(|pair| pair.receiver_share_change)) {
        BroadcastOutcomeAvailability::ConstantReceiverChanges
    } else if is_constant(pairs.iter().map(|pair| pair.status_change)) {
        BroadcastOutcomeAvailability::ConstantStatusChanges
    } else {
        BroadcastOutcomeAvailability::Available
    };
    let coefficient = (availability == BroadcastOutcomeAvailability::Available)
        .then(|| {
            spearman(
                &pairs
                    .iter()
                    .map(|pair| pair.receiver_share_change)
                    .collect::<Vec<_>>(),
                &pairs
                    .iter()
                    .map(|pair| pair.status_change)
                    .collect::<Vec<_>>(),
            )
        })
        .flatten();
    let mut elapsed_days = pairs
        .iter()
        .map(|pair| pair.elapsed_game_days)
        .collect::<Vec<_>>();
    elapsed_days.sort_unstable();
    let elapsed_days_median = median(&elapsed_days);
    let first = pairs.first();
    let last = pairs.last();

    Ok(BroadcastOutcomeModel {
        availability,
        receiver_metric_id: request.receiver_metric_id.clone(),
        status_metric_id: request.status_metric_id.clone(),
        lag_confirmed_records: request.lag_confirmed_records,
        coefficient,
        pair_count: pairs.len().min(u32::MAX as usize) as u32,
        start_year: first.map(|pair| pair.status_year),
        start_day: first.map(|pair| pair.status_day),
        end_year: last.map(|pair| pair.status_year),
        end_day: last.map(|pair| pair.status_day),
        elapsed_days_median,
        elapsed_days_min: elapsed_days.first().copied(),
        elapsed_days_max: elapsed_days.last().copied(),
        pairs,
    })
}

struct AlignedDelta<'a> {
    end_ordinal: usize,
    receiver: &'a ReceiverHistoryPoint,
    status: &'a CitizenStatusPoint,
    receiver_change: f64,
    status_change: f64,
}

fn build_pulse(receiver: &ReceiverDataset) -> Option<BroadcastPulse> {
    let latest = receiver.points.last()?;
    let previous = receiver.points.iter().rev().nth(1);
    let counts = [
        latest.none,
        latest.radio,
        latest.television,
        latest.computer,
    ];
    let previous_counts =
        previous.map(|point| [point.none, point.radio, point.television, point.computer]);
    let denominator = latest.classified_total as f64;
    let classes = RECEIVER_METRICS
        .iter()
        .zip(counts)
        .enumerate()
        .map(|(index, (metric, count))| BroadcastReceiverClassPulse {
            metric_id: metric.id.to_owned(),
            count,
            share_percent: if denominator > 0.0 {
                count as f64 / denominator * 100.0
            } else {
                0.0
            },
            change_from_previous: previous_counts
                .map(|values| signed_difference(count, values[index])),
        })
        .collect();
    Some(BroadcastPulse {
        year: latest.year,
        day: latest.day,
        classified_population: latest.classified_total,
        classes,
    })
}

fn signed_difference(current: u64, previous: u64) -> i64 {
    let difference = i128::from(current) - i128::from(previous);
    difference.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

fn receiver_share(point: &ReceiverHistoryPoint, index: usize) -> Option<f64> {
    if point.classified_total == 0 {
        return None;
    }
    let count = [point.none, point.radio, point.television, point.computer][index];
    Some(count as f64 / point.classified_total as f64 * 100.0)
}

fn unavailable_outcome(
    request: &BroadcastOutcomeRequest,
    availability: BroadcastOutcomeAvailability,
) -> BroadcastOutcomeModel {
    BroadcastOutcomeModel {
        availability,
        receiver_metric_id: request.receiver_metric_id.clone(),
        status_metric_id: request.status_metric_id.clone(),
        lag_confirmed_records: request.lag_confirmed_records,
        coefficient: None,
        pair_count: 0,
        start_year: None,
        start_day: None,
        end_year: None,
        end_day: None,
        elapsed_days_median: None,
        elapsed_days_min: None,
        elapsed_days_max: None,
        pairs: Vec::new(),
    }
}

fn is_constant(values: impl Iterator<Item = f64>) -> bool {
    let mut values = values;
    let Some(first) = values.next() else {
        return true;
    };
    values.all(|value| {
        let scale = first.abs().max(value.abs()).max(1.0);
        (value - first).abs() <= f64::EPSILON * scale * 16.0
    })
}

fn spearman(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    pearson(&average_ranks(left), &average_ranks(right))
}

fn average_ranks(values: &[f64]) -> Vec<f64> {
    let mut order = values.iter().copied().enumerate().collect::<Vec<_>>();
    order.sort_by(|left, right| left.1.partial_cmp(&right.1).unwrap_or(Ordering::Equal));
    let mut ranks = vec![0.0; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && order[end].1.to_bits() == order[start].1.to_bits() {
            end += 1;
        }
        let rank = ((start + 1 + end) as f64) / 2.0;
        for entry in &order[start..end] {
            ranks[entry.0] = rank;
        }
        start = end;
    }
    ranks
}

fn pearson(left: &[f64], right: &[f64]) -> Option<f64> {
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    let mut covariance = 0.0;
    let mut left_variance = 0.0;
    let mut right_variance = 0.0;
    for (left, right) in left.iter().zip(right) {
        let left_delta = left - left_mean;
        let right_delta = right - right_mean;
        covariance += left_delta * right_delta;
        left_variance += left_delta * left_delta;
        right_variance += right_delta * right_delta;
    }
    let denominator = (left_variance * right_variance).sqrt();
    (denominator > 0.0).then_some(covariance / denominator)
}

fn median(values: &[i64]) -> Option<f64> {
    let middle = values.len().checked_sub(1)? / 2;
    if values.len().is_multiple_of(2) {
        Some((values[middle] as f64 + values[middle + 1] as f64) / 2.0)
    } else {
        Some(values[middle] as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AnalysisContextMode, AnalysisContextOrigin, CompatibilityProvenance, CoverageStatus,
        ExactObservationReference,
    };

    #[test]
    fn calculates_monotonic_first_difference_association() {
        let workspace = workspace_with(
            15,
            |ordinal| (ordinal * ordinal) as f64 / 400.0,
            |ordinal| {
                let radio = ordinal * ordinal + 10;
                [930 - radio, radio, 50, 20]
            },
        );
        let outcome = calculate_outcome(
            &workspace,
            &BroadcastOutcomeRequest {
                receiver_metric_id: RECEIVER_METRICS[1].id.to_owned(),
                status_metric_id: CITIZEN_STATUS_METRICS[0].id.to_owned(),
                lag_confirmed_records: 0,
            },
        )
        .expect("outcome");

        assert_eq!(
            outcome.availability,
            BroadcastOutcomeAvailability::Available
        );
        assert_eq!(outcome.pair_count, 14);
        assert!((outcome.coefficient.expect("coefficient") - 1.0).abs() < 1e-12);
        assert_eq!(outcome.elapsed_days_median, Some(0.0));
    }

    #[test]
    fn reports_insufficient_pairs_without_interpolating_a_missing_status_record() {
        let mut workspace = workspace_with(
            13,
            |ordinal| ordinal as f64,
            |ordinal| [100, 20 + ordinal, 30, 20],
        );
        workspace.citizen_status_points.remove(6);
        let outcome = calculate_outcome(
            &workspace,
            &BroadcastOutcomeRequest {
                receiver_metric_id: RECEIVER_METRICS[1].id.to_owned(),
                status_metric_id: CITIZEN_STATUS_METRICS[0].id.to_owned(),
                lag_confirmed_records: 0,
            },
        )
        .expect("outcome");

        assert_eq!(
            outcome.availability,
            BroadcastOutcomeAvailability::InsufficientPairs
        );
        assert_eq!(outcome.pair_count, 10);
    }

    #[test]
    fn supports_only_the_published_lags() {
        let workspace = workspace_with(
            20,
            |ordinal| ordinal as f64,
            |ordinal| [100, 20 + ordinal, 30, 20],
        );
        let outcome = calculate_outcome(
            &workspace,
            &BroadcastOutcomeRequest {
                receiver_metric_id: RECEIVER_METRICS[1].id.to_owned(),
                status_metric_id: CITIZEN_STATUS_METRICS[0].id.to_owned(),
                lag_confirmed_records: 4,
            },
        )
        .expect("lagged outcome");
        assert_eq!(outcome.pair_count, 15);
        assert_eq!(outcome.elapsed_days_median, Some(68.0));

        let error = calculate_outcome(
            &workspace,
            &BroadcastOutcomeRequest {
                receiver_metric_id: RECEIVER_METRICS[1].id.to_owned(),
                status_metric_id: CITIZEN_STATUS_METRICS[0].id.to_owned(),
                lag_confirmed_records: 3,
            },
        )
        .expect_err("invalid lag");
        assert_eq!(error.code(), "invalid_broadcast_outcome");
    }

    #[test]
    fn reports_constant_inputs_honestly() {
        let workspace = workspace_with(14, |ordinal| ordinal as f64, |_ordinal| [100, 20, 30, 20]);
        let outcome = calculate_outcome(
            &workspace,
            &BroadcastOutcomeRequest {
                receiver_metric_id: RECEIVER_METRICS[1].id.to_owned(),
                status_metric_id: CITIZEN_STATUS_METRICS[0].id.to_owned(),
                lag_confirmed_records: 0,
            },
        )
        .expect("outcome");
        assert_eq!(
            outcome.availability,
            BroadcastOutcomeAvailability::ConstantReceiverChanges
        );
        assert_eq!(outcome.coefficient, None);
    }

    fn workspace_with(
        count: usize,
        status_value: impl Fn(usize) -> f64,
        receiver_counts: impl Fn(u64) -> [u64; 4],
    ) -> BroadcastWorkspaceModel {
        let context = AnalysisContext {
            context_id: "context".to_owned(),
            selected_branch_id: "main".to_owned(),
            head_interpretation_id: Some("head".to_owned()),
            original_branch_id: Some("main".to_owned()),
            mode: AnalysisContextMode::Latest,
            origin: AnalysisContextOrigin::Automatic,
            is_tip: true,
            membership_revision: 1,
            compatibility_profile_id: Some("profile".to_owned()),
            compatibility_profile_hash: Some("hash".to_owned()),
            observation_watermark: Some("watermark".to_owned()),
            catalogue_generation_id: None,
            resource_catalogue_revision_id: None,
            overlay_revision: None,
        };
        let mut receiver_points = Vec::new();
        let mut status_points = Vec::new();
        for ordinal in 0..count {
            let record_id = ordinal as u32;
            let game_day = ordinal as i64 * 17;
            let [none, radio, television, computer] = receiver_counts(ordinal as u64);
            let exact = Some(ExactObservationReference {
                interpretation_id: format!("observation-{ordinal}"),
                branch_id: "main".to_owned(),
                year: 1960,
                day: (ordinal * 17) as u16,
            });
            receiver_points.push(ReceiverHistoryPoint {
                record_id,
                year: 1960,
                day: (ordinal * 17) as u16,
                game_day,
                none,
                radio,
                television,
                computer,
                classified_total: none + radio + television + computer,
                exact_observation: exact.clone(),
            });
            status_points.push(CitizenStatusPoint {
                ordinal: ordinal as u32,
                record_id,
                year: 1960,
                day: (ordinal * 17) as u16,
                game_day,
                values: [status_value(ordinal); 9],
                source_fields: std::array::from_fn(|index| format!("$Citizens_Status[{index}]")),
                source_lines: [1; 9],
                exact_observation: exact,
            });
        }
        build_workspace(
            context,
            Some(ReceiverDataset {
                payload_hash: "payload".to_owned(),
                interpretation_id: "head".to_owned(),
                source_file_name: "save.zip".to_owned(),
                source_file_size: 1,
                source_modified_ms: 1,
                imported_at_ms: 1,
                parser_version: "parser".to_owned(),
                format_profile: "profile".to_owned(),
                compatibility: CompatibilityProvenance {
                    profile_id: "profile".to_owned(),
                    profile_version: "1.0.0".to_owned(),
                    profile_content_hash: "content".to_owned(),
                    resolved_profile_hash: "hash".to_owned(),
                    base_profile_hash: None,
                    profile_source: "built_in".to_owned(),
                    mapping_classification: "reviewed_mapping".to_owned(),
                    parser_engine_version: "parser".to_owned(),
                },
                branch_id: "main".to_owned(),
                original_branch_id: "main".to_owned(),
                analysis_context_id: Some("context".to_owned()),
                geographic_scope: "republic".to_owned(),
                coverage: CoverageReport {
                    status: CoverageStatus::Complete,
                    history_records: count as u32,
                    chartable_records: count as u32,
                    dropped_records: 0,
                    warnings: Vec::new(),
                },
                source_fields: Vec::new(),
                points: receiver_points,
            }),
            Some(CoverageReport {
                status: CoverageStatus::Complete,
                history_records: count as u32,
                chartable_records: count as u32,
                dropped_records: 0,
                warnings: Vec::new(),
            }),
            status_points,
            Vec::new(),
            false,
        )
    }
}
