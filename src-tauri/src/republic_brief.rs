use crate::metric_catalogue::{
    ADULTS, BASIC_EDUCATION, HIGHER_EDUCATION, NO_EDUCATION, RECEIVER_COMPUTER, RECEIVER_NONE,
    RECEIVER_RADIO, RECEIVER_TELEVISION, RECEIVER_TOTAL, SMALL_CHILDREN, UNEMPLOYED, exact_context,
};
use crate::model::{
    AnalysisContextMode, BriefComparisonAnchor, BriefEvidenceKind, BriefFinding,
    BriefFindingSeverity, BriefMetric, BriefMetricRole, BriefObservation, BriefOperations,
    BriefSourceEvidence, CatalogueStatus, CoverageStatus, PopulationDataset, PopulationFact,
    PopulationObservation, RecorderHealth, RepublicBrief, RepublicPlanBrief, WarehousePhase,
};

const SCHEMA_VERSION: u32 = 1;
const RAW_METRICS: [(&str, BriefMetricRole); 9] = [
    (ADULTS, BriefMetricRole::Headline),
    (SMALL_CHILDREN, BriefMetricRole::Headline),
    (UNEMPLOYED, BriefMetricRole::Headline),
    (NO_EDUCATION, BriefMetricRole::Education),
    (BASIC_EDUCATION, BriefMetricRole::Education),
    (HIGHER_EDUCATION, BriefMetricRole::Education),
    (RECEIVER_NONE, BriefMetricRole::ReceiverClass),
    (RECEIVER_RADIO, BriefMetricRole::ReceiverClass),
    (RECEIVER_TELEVISION, BriefMetricRole::ReceiverClass),
];

const UNAVAILABLE_CAPABILITIES: [&str; 2] = ["import_exposure", "observed_material_reliance"];

pub fn build_republic_brief(
    dataset: &PopulationDataset,
    recorder: Option<&RecorderHealth>,
    catalogue: Option<&CatalogueStatus>,
    plan: Option<RepublicPlanBrief>,
) -> RepublicBrief {
    let current = dataset.observations.last();
    let previous = dataset
        .observations
        .len()
        .checked_sub(2)
        .and_then(|index| dataset.observations.get(index));
    let mut findings = operational_findings(recorder, catalogue);
    let operations = operations(dataset, recorder, catalogue);

    let Some(current) = current else {
        findings.push(finding(
            "no_observation",
            BriefFindingSeverity::Information,
            None,
            None,
        ));
        sort_findings(&mut findings);
        return RepublicBrief {
            schema_version: SCHEMA_VERSION,
            analysis_context: dataset.analysis_context.clone(),
            observation: None,
            comparison: None,
            metrics: Vec::new(),
            dispatch_code: dispatch_code(&findings),
            findings,
            operations,
            unavailable_capabilities: unavailable_capabilities(plan.is_none()),
            plan,
        };
    };

    if dataset.analysis_context.mode == AnalysisContextMode::HistoricalPreview {
        findings.push(finding(
            "historical_preview",
            BriefFindingSeverity::Information,
            None,
            None,
        ));
    }
    if current.coverage_status == CoverageStatus::Partial {
        findings.push(finding(
            "partial_coverage",
            BriefFindingSeverity::Watch,
            None,
            None,
        ));
    }
    if current.mapping_classification == "player_mapped" {
        findings.push(finding(
            "player_mapping",
            BriefFindingSeverity::Watch,
            None,
            None,
        ));
    }
    if previous.is_none() {
        findings.push(finding(
            "no_prior_observation",
            BriefFindingSeverity::Information,
            None,
            None,
        ));
    } else if previous
        .is_some_and(|prior| prior.resolved_profile_hash != current.resolved_profile_hash)
    {
        findings.push(finding(
            "mapping_changed",
            BriefFindingSeverity::Watch,
            None,
            None,
        ));
    }

    let comparable_previous =
        previous.filter(|prior| prior.resolved_profile_hash == current.resolved_profile_hash);
    let current_receiver_total = receiver_total(current);
    let previous_receiver_total = comparable_previous.and_then(receiver_total);
    let mut metrics = RAW_METRICS
        .iter()
        .chain([(RECEIVER_COMPUTER, BriefMetricRole::ReceiverClass)].iter())
        .filter_map(|(metric_id, role)| {
            raw_metric(
                current,
                comparable_previous,
                metric_id,
                *role,
                current_receiver_total,
            )
        })
        .collect::<Vec<_>>();
    if let Some(value) = current_receiver_total {
        metrics.insert(
            3,
            BriefMetric {
                metric_id: RECEIVER_TOTAL.to_owned(),
                role: BriefMetricRole::Headline,
                value,
                previous_value: previous_receiver_total,
                delta: previous_receiver_total.and_then(|prior| difference(value, prior)),
                share_basis_points: None,
                evidence_kind: BriefEvidenceKind::Calculation,
                sources: receiver_sources(current),
                context: exact_context(RECEIVER_TOTAL)
                    .expect("receiver total is a published metric"),
            },
        );
    }

    let expected_metric_count = RAW_METRICS.len() + 1;
    let raw_metric_count = metrics
        .iter()
        .filter(|metric| metric.evidence_kind == BriefEvidenceKind::SaveFact)
        .count();
    let missing_metric_count = expected_metric_count.saturating_sub(raw_metric_count);
    if missing_metric_count > 0 {
        findings.push(finding(
            "missing_metrics",
            BriefFindingSeverity::Watch,
            Some(missing_metric_count as u64),
            None,
        ));
    }
    sort_findings(&mut findings);

    RepublicBrief {
        schema_version: SCHEMA_VERSION,
        analysis_context: dataset.analysis_context.clone(),
        observation: Some(observation(current)),
        comparison: previous.map(comparison),
        metrics,
        dispatch_code: dispatch_code(&findings),
        findings,
        operations,
        unavailable_capabilities: unavailable_capabilities(plan.is_none()),
        plan,
    }
}

fn raw_metric(
    current: &PopulationObservation,
    previous: Option<&PopulationObservation>,
    metric_id: &str,
    role: BriefMetricRole,
    receiver_total: Option<u64>,
) -> Option<BriefMetric> {
    let current_fact = fact(current, metric_id)?;
    let previous_value = previous
        .and_then(|observation| fact(observation, metric_id))
        .map(|fact| fact.value);
    Some(BriefMetric {
        metric_id: metric_id.to_owned(),
        role,
        value: current_fact.value,
        previous_value,
        delta: previous_value.and_then(|prior| difference(current_fact.value, prior)),
        share_basis_points: (role == BriefMetricRole::ReceiverClass)
            .then(|| receiver_total.and_then(|total| ratio_basis_points(current_fact.value, total)))
            .flatten(),
        evidence_kind: BriefEvidenceKind::SaveFact,
        sources: vec![BriefSourceEvidence {
            source_field: current_fact.source_field.clone(),
            source_line: current_fact.source_line,
        }],
        context: exact_context(metric_id).expect("brief metrics are published by the host"),
    })
}

fn unavailable_capabilities(plan_unavailable: bool) -> Vec<String> {
    let mut capabilities = UNAVAILABLE_CAPABILITIES
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    if plan_unavailable {
        capabilities.insert(0, "plan_attainment".to_owned());
    }
    capabilities
}

fn fact<'a>(observation: &'a PopulationObservation, metric_id: &str) -> Option<&'a PopulationFact> {
    observation
        .facts
        .iter()
        .find(|fact| fact.fact_id == metric_id)
}

fn receiver_total(observation: &PopulationObservation) -> Option<u64> {
    [
        RECEIVER_NONE,
        RECEIVER_RADIO,
        RECEIVER_TELEVISION,
        RECEIVER_COMPUTER,
    ]
    .iter()
    .try_fold(0_u64, |total, metric_id| {
        total.checked_add(fact(observation, metric_id)?.value)
    })
}

fn receiver_sources(observation: &PopulationObservation) -> Vec<BriefSourceEvidence> {
    [
        RECEIVER_NONE,
        RECEIVER_RADIO,
        RECEIVER_TELEVISION,
        RECEIVER_COMPUTER,
    ]
    .iter()
    .filter_map(|metric_id| fact(observation, metric_id))
    .map(|fact| BriefSourceEvidence {
        source_field: fact.source_field.clone(),
        source_line: fact.source_line,
    })
    .collect()
}

fn difference(current: u64, previous: u64) -> Option<i64> {
    let current = i64::try_from(current).ok()?;
    let previous = i64::try_from(previous).ok()?;
    current.checked_sub(previous)
}

fn ratio_basis_points(value: u64, total: u64) -> Option<u16> {
    if total == 0 {
        return None;
    }
    let numerator = u128::from(value)
        .checked_mul(10_000)?
        .checked_add(u128::from(total) / 2)?;
    u16::try_from(numerator / u128::from(total)).ok()
}

fn observation(observation: &PopulationObservation) -> BriefObservation {
    BriefObservation {
        interpretation_id: observation.interpretation_id.clone(),
        source_file_name: observation.source_file_name.clone(),
        year: observation.sampled_year,
        day: observation.sampled_day,
        game_day: observation.sampled_game_day,
        coverage_status: observation.coverage_status.clone(),
        mapping_classification: observation.mapping_classification.clone(),
        profile_id: observation.profile_id.clone(),
        profile_version: observation.profile_version.clone(),
        resolved_profile_hash: observation.resolved_profile_hash.clone(),
    }
}

fn comparison(observation: &PopulationObservation) -> BriefComparisonAnchor {
    BriefComparisonAnchor {
        interpretation_id: observation.interpretation_id.clone(),
        source_file_name: observation.source_file_name.clone(),
        year: observation.sampled_year,
        day: observation.sampled_day,
        game_day: observation.sampled_game_day,
    }
}

fn operations(
    dataset: &PopulationDataset,
    recorder: Option<&RecorderHealth>,
    catalogue: Option<&CatalogueStatus>,
) -> BriefOperations {
    BriefOperations {
        recorder_phase: recorder.map(|health| health.observer.phase.clone()),
        recorder_queue_depth: recorder.map(|health| health.queue_depth),
        recorder_attention_count: recorder.map(|health| health.attention_count),
        warehouse_phase: catalogue.map(|status| status.warehouse.phase),
        warehouse_pending_jobs: catalogue.map(|status| status.warehouse.pending_jobs),
        warehouse_failed_jobs: catalogue.map(|status| status.warehouse.failed_jobs),
        warehouse_lag_ms: catalogue.and_then(|status| status.warehouse.lag_ms),
        catalogue_generation_id: catalogue
            .and_then(|status| status.generation.as_ref())
            .map(|generation| generation.generation_id.clone()),
        catalogue_entity_count: catalogue
            .and_then(|status| status.generation.as_ref())
            .map(|generation| generation.entity_count),
        city_scope_count: dataset.cities.len().try_into().unwrap_or(u32::MAX),
    }
}

fn operational_findings(
    recorder: Option<&RecorderHealth>,
    catalogue: Option<&CatalogueStatus>,
) -> Vec<BriefFinding> {
    let mut findings = Vec::new();
    if let Some(recorder) = recorder {
        if recorder.attention_count > 0 {
            findings.push(finding(
                "recorder_attention",
                BriefFindingSeverity::Attention,
                Some(u64::from(recorder.attention_count)),
                None,
            ));
        } else if recorder.queue_depth > 0 {
            findings.push(finding(
                "recorder_queue",
                BriefFindingSeverity::Information,
                Some(u64::from(recorder.queue_depth)),
                None,
            ));
        }
    }
    if let Some(catalogue) = catalogue {
        if catalogue.warehouse.phase == WarehousePhase::Attention {
            findings.push(finding(
                "warehouse_attention",
                BriefFindingSeverity::Attention,
                Some(u64::from(catalogue.warehouse.failed_jobs)),
                None,
            ));
        } else if catalogue.warehouse.phase == WarehousePhase::Lagging {
            findings.push(finding(
                "warehouse_lagging",
                BriefFindingSeverity::Watch,
                Some(u64::from(catalogue.warehouse.pending_jobs)),
                None,
            ));
        }
        if catalogue.generation.is_none() {
            findings.push(finding(
                "catalogue_unavailable",
                BriefFindingSeverity::Information,
                None,
                None,
            ));
        }
    }
    findings
}

fn finding(
    code: &str,
    severity: BriefFindingSeverity,
    value: Option<u64>,
    metric_id: Option<&str>,
) -> BriefFinding {
    BriefFinding {
        code: code.to_owned(),
        severity,
        value,
        metric_id: metric_id.map(str::to_owned),
    }
}

fn sort_findings(findings: &mut [BriefFinding]) {
    findings.sort_by_key(|finding| match finding.severity {
        BriefFindingSeverity::Attention => 0,
        BriefFindingSeverity::Watch => 1,
        BriefFindingSeverity::Information => 2,
    });
}

fn dispatch_code(findings: &[BriefFinding]) -> String {
    findings
        .first()
        .map(|finding| finding.code.clone())
        .unwrap_or_else(|| "observation_ready".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AnalysisContext, AnalysisContextOrigin, AutomaticObserverPhase, AutomaticObserverStatus,
        CatalogueRefreshProgress, CatalogueStatus, MetricContextLimitation, MetricPopulationBasis,
        PopulationCitySnapshot, RecorderHealth, WarehouseHealth,
    };

    #[test]
    fn builds_exact_head_metrics_deltas_shares_and_provenance() {
        let dataset = dataset_with_two_observations();
        let brief = build_republic_brief(&dataset, Some(&recorder()), Some(&catalogue()), None);

        assert_eq!(brief.schema_version, 1);
        assert_eq!(brief.observation.as_ref().map(|value| value.day), Some(20));
        assert_eq!(brief.comparison.as_ref().map(|value| value.day), Some(10));
        let adults = metric(&brief, ADULTS);
        assert_eq!(adults.value, 1_100);
        assert_eq!(adults.previous_value, Some(1_000));
        assert_eq!(adults.delta, Some(100));
        assert_eq!(adults.sources[0].source_field, "$Citizens_Adults");
        assert_eq!(
            adults.context.population_basis,
            MetricPopulationBasis::SourceDefinedAdults
        );
        assert_eq!(
            adults.context.limitations,
            vec![MetricContextLimitation::NotEmploymentCount]
        );
        assert_eq!(
            metric(&brief, BASIC_EDUCATION).context.population_basis,
            MetricPopulationBasis::AllRecordedCitizens
        );
        assert_eq!(
            metric(&brief, BASIC_EDUCATION).context.limitations,
            vec![MetricContextLimitation::NotWorkersOnly]
        );
        let total = metric(&brief, RECEIVER_TOTAL);
        assert_eq!(total.value, 1_100);
        assert_eq!(total.evidence_kind, BriefEvidenceKind::Calculation);
        assert_eq!(
            metric(&brief, RECEIVER_RADIO).share_basis_points,
            Some(3_000)
        );
        assert_eq!(
            metric(&brief, RECEIVER_RADIO)
                .context
                .denominator_metric_id
                .as_deref(),
            Some(RECEIVER_TOTAL)
        );
        assert_eq!(brief.dispatch_code, "observation_ready");
    }

    #[test]
    fn active_plan_replaces_only_the_plan_unavailable_capability() {
        let plan = RepublicPlanBrief {
            plan_id: "plan-test".to_owned(),
            name: "Five-Year Plan".to_owned(),
            revision: 2,
            target_count: 3,
            end_year: 2020,
            end_day: 20,
            state: crate::model::PlanTargetState::OnTrack,
            attainment_basis_points: Some(9_500),
            guardrail_breach_count: 0,
        };

        let brief = build_republic_brief(
            &dataset_with_two_observations(),
            Some(&recorder()),
            Some(&catalogue()),
            Some(plan.clone()),
        );

        assert_eq!(brief.plan, Some(plan));
        assert!(
            !brief
                .unavailable_capabilities
                .contains(&"plan_attainment".to_owned())
        );
        assert_eq!(brief.unavailable_capabilities.len(), 2);
    }

    #[test]
    fn keeps_missing_and_operational_evidence_explicit() {
        let mut dataset = dataset_with_two_observations();
        dataset.analysis_context.mode = AnalysisContextMode::HistoricalPreview;
        dataset.observations.last_mut().unwrap().coverage_status = CoverageStatus::Partial;
        dataset.observations.last_mut().unwrap().facts.clear();
        let mut recorder = recorder();
        recorder.attention_count = 2;
        let mut catalogue = catalogue();
        catalogue.warehouse.phase = WarehousePhase::Attention;
        catalogue.warehouse.failed_jobs = 1;

        let brief = build_republic_brief(&dataset, Some(&recorder), Some(&catalogue), None);

        assert!(brief.metrics.is_empty());
        assert_eq!(brief.dispatch_code, "recorder_attention");
        assert!(
            brief
                .findings
                .iter()
                .any(|item| item.code == "partial_coverage")
        );
        assert!(
            brief
                .findings
                .iter()
                .any(|item| item.code == "missing_metrics")
        );
        assert!(
            brief
                .findings
                .iter()
                .any(|item| item.code == "historical_preview")
        );
        assert_eq!(brief.unavailable_capabilities.len(), 3);
    }

    #[test]
    fn suppresses_changes_across_compatibility_mapping_boundaries() {
        let mut dataset = dataset_with_two_observations();
        dataset
            .observations
            .last_mut()
            .unwrap()
            .resolved_profile_hash = "b".repeat(64);

        let brief = build_republic_brief(&dataset, Some(&recorder()), Some(&catalogue()), None);

        assert_eq!(metric(&brief, ADULTS).previous_value, None);
        assert_eq!(metric(&brief, ADULTS).delta, None);
        assert!(
            brief
                .findings
                .iter()
                .any(|finding| finding.code == "mapping_changed")
        );
    }

    #[test]
    fn empty_context_does_not_invent_an_observation() {
        let mut dataset = dataset_with_two_observations();
        dataset.observations.clear();
        let brief = build_republic_brief(&dataset, None, None, None);

        assert!(brief.observation.is_none());
        assert!(brief.metrics.is_empty());
        assert_eq!(brief.dispatch_code, "no_observation");
    }

    fn metric<'a>(brief: &'a RepublicBrief, id: &str) -> &'a BriefMetric {
        brief
            .metrics
            .iter()
            .find(|metric| metric.metric_id == id)
            .unwrap()
    }

    fn dataset_with_two_observations() -> PopulationDataset {
        PopulationDataset {
            analysis_context: AnalysisContext {
                context_id: "context".to_owned(),
                selected_branch_id: "main".to_owned(),
                head_interpretation_id: Some("second".to_owned()),
                original_branch_id: Some("main".to_owned()),
                mode: AnalysisContextMode::Latest,
                origin: AnalysisContextOrigin::Automatic,
                is_tip: true,
                membership_revision: 2,
                compatibility_profile_id: Some("profile".to_owned()),
                compatibility_profile_hash: Some("hash".to_owned()),
                observation_watermark: Some("watermark".to_owned()),
                catalogue_generation_id: Some("generation".to_owned()),
                resource_catalogue_revision_id: None,
                overlay_revision: None,
            },
            observations: vec![
                observation("first", 10, 1_000),
                observation("second", 20, 1_100),
            ],
            cities: vec![PopulationCitySnapshot {
                scope_id: "1".to_owned(),
                sampled_year: 2015,
                sampled_day: 20,
                sampled_game_day: 5_495,
                coverage_status: CoverageStatus::Complete,
                facts: Vec::new(),
            }],
            observation_limit: 256,
            city_limit: 512,
            tesmio_probe: crate::model::TesmioProbeStatus::not_configured(),
        }
    }

    fn observation(id: &str, day: u16, scale: u64) -> PopulationObservation {
        let values = [
            (ADULTS, scale),
            (SMALL_CHILDREN, scale / 10),
            (UNEMPLOYED, scale / 5),
            (NO_EDUCATION, scale / 10),
            (BASIC_EDUCATION, scale / 2),
            (HIGHER_EDUCATION, scale / 4),
            (RECEIVER_NONE, scale / 10),
            (RECEIVER_RADIO, scale * 3 / 10),
            (RECEIVER_TELEVISION, scale * 2 / 5),
            (RECEIVER_COMPUTER, scale / 5),
        ];
        PopulationObservation {
            interpretation_id: id.to_owned(),
            source_file_name: format!("{id}.zip"),
            membership_revision: u32::from(day / 10),
            sampled_year: 2015,
            sampled_day: day,
            sampled_game_day: 5_475 + i64::from(day),
            coverage_status: CoverageStatus::Complete,
            mapping_classification: "reviewed_mapping".to_owned(),
            profile_id: "profile".to_owned(),
            profile_version: "1.0.0".to_owned(),
            resolved_profile_hash: "hash".to_owned(),
            exact_observation: None,
            facts: values
                .into_iter()
                .enumerate()
                .map(|(index, (fact_id, value))| PopulationFact {
                    fact_id: fact_id.to_owned(),
                    value,
                    source_field: if fact_id == ADULTS {
                        "$Citizens_Adults".to_owned()
                    } else {
                        format!("$Fixture_{index}")
                    },
                    source_line: 100 + index as u64,
                })
                .collect(),
        }
    }

    fn recorder() -> RecorderHealth {
        RecorderHealth {
            observer: AutomaticObserverStatus {
                enabled: true,
                phase: AutomaticObserverPhase::Watching,
                candidate_file_name: None,
                retry_attempt: 0,
                error_code: None,
                last_observed_file_name: Some("second.zip".to_owned()),
                last_observed_at_ms: Some(1),
            },
            last_scan_ms: Some(1),
            last_filesystem_event_ms: Some(1),
            last_completed_at_ms: Some(1),
            last_completed_file_name: Some("second.zip".to_owned()),
            last_processing_latency_ms: Some(20),
            queue_depth: 0,
            attention_count: 0,
            completed_count: 2,
            latest_entries: Vec::new(),
        }
    }

    fn catalogue() -> CatalogueStatus {
        CatalogueStatus {
            warehouse: WarehouseHealth {
                phase: WarehousePhase::Ready,
                schema_version: 5,
                pending_jobs: 0,
                failed_jobs: 0,
                lag_ms: Some(0),
                last_projected_at_ms: Some(1),
                observation_watermark: Some("watermark".to_owned()),
                database_size_bytes: 1,
                active_write: None,
                consecutive_write_failures: 0,
                retry_after_ms: None,
            },
            generation: Some(crate::model::CatalogueGenerationSummary {
                generation_id: "generation".to_owned(),
                game_build_id: None,
                parser_version: "parser".to_owned(),
                created_at_ms: 1,
                source_count: 1,
                file_count: 1,
                entity_count: 42,
                property_count: 1,
                relation_count: 1,
                warning_count: 0,
                compatibility_profile_id: "profile".to_owned(),
                compatibility_profile_version: "1.0.0".to_owned(),
                compatibility_profile_hash: "hash".to_owned(),
                mapping_classification: "reviewed_mapping".to_owned(),
            }),
            last_checked_at_ms: Some(1),
            last_refreshed_at_ms: Some(1),
            last_filesystem_event_ms: None,
            error_code: None,
            active_overlay: None,
            refresh: CatalogueRefreshProgress::default(),
        }
    }
}
