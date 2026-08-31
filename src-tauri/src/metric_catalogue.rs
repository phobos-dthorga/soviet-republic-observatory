use crate::model::{
    MetricComparisonBasis, MetricContext, MetricContextLimitation, MetricGeographicScope,
    MetricPopulationBasis, MetricTimeBasis, PublishedMetricContext,
};

pub const ADULTS: &str = "source.stats.citizens.adults";
pub const SMALL_CHILDREN: &str = "source.stats.citizens.small_children";
pub const UNEMPLOYED: &str = "source.stats.citizens.unemployed";
pub const NO_EDUCATION: &str = "source.stats.citizens.no_education";
pub const BASIC_EDUCATION: &str = "source.stats.citizens.basic_education";
pub const HIGHER_EDUCATION: &str = "source.stats.citizens.higher_education";
pub const RECEIVER_NONE: &str = "core.citizens.electronics.none";
pub const RECEIVER_RADIO: &str = "core.citizens.electronics.radio";
pub const RECEIVER_TELEVISION: &str = "core.citizens.electronics.television";
pub const RECEIVER_COMPUTER: &str = "core.citizens.electronics.computer";
pub const RECEIVER_TOTAL: &str = "core.citizens.electronics.classified_total";
pub const BORN: &str = "source.stats.citizens.born";
pub const DEAD: &str = "source.stats.citizens.dead";
pub const ESCAPED: &str = "source.stats.citizens.escaped";
pub const IMMIGRANT_SOVIET: &str = "source.stats.citizens.immigrant_soviet";
pub const IMMIGRANT_AFRICA: &str = "source.stats.citizens.immigrant_africa";

pub const PLAN_METRIC_IDS: [&str; 11] = [
    ADULTS,
    SMALL_CHILDREN,
    UNEMPLOYED,
    NO_EDUCATION,
    BASIC_EDUCATION,
    HIGHER_EDUCATION,
    RECEIVER_NONE,
    RECEIVER_RADIO,
    RECEIVER_TELEVISION,
    RECEIVER_COMPUTER,
    RECEIVER_TOTAL,
];

const PUBLISHED_METRIC_IDS: [&str; 16] = [
    ADULTS,
    SMALL_CHILDREN,
    UNEMPLOYED,
    NO_EDUCATION,
    BASIC_EDUCATION,
    HIGHER_EDUCATION,
    RECEIVER_NONE,
    RECEIVER_RADIO,
    RECEIVER_TELEVISION,
    RECEIVER_COMPUTER,
    RECEIVER_TOTAL,
    BORN,
    DEAD,
    ESCAPED,
    IMMIGRANT_SOVIET,
    IMMIGRANT_AFRICA,
];

pub fn published_metric_contexts() -> Vec<PublishedMetricContext> {
    PUBLISHED_METRIC_IDS
        .iter()
        .filter_map(|metric_id| {
            Some(PublishedMetricContext {
                metric_id: (*metric_id).to_owned(),
                exact: context(
                    metric_id,
                    MetricTimeBasis::ExactSelectedObservation,
                    MetricComparisonBasis::ProvenPrecedingSameBranchAndProfile,
                )?,
                history: context(
                    metric_id,
                    MetricTimeBasis::BranchObservationsThroughSelectedHead,
                    MetricComparisonBasis::ProvenPrecedingSameBranchAndProfile,
                )?,
            })
        })
        .collect()
}

pub fn exact_context(metric_id: &str) -> Option<MetricContext> {
    context(
        metric_id,
        MetricTimeBasis::ExactSelectedObservation,
        MetricComparisonBasis::ProvenPrecedingSameBranchAndProfile,
    )
}

pub fn plan_context(metric_id: &str) -> Option<MetricContext> {
    context(
        metric_id,
        MetricTimeBasis::BranchObservationsThroughSelectedHead,
        MetricComparisonBasis::PlayerPlanSchedule,
    )
}

pub fn is_plan_metric(metric_id: &str) -> bool {
    PLAN_METRIC_IDS.contains(&metric_id)
}

fn context(
    metric_id: &str,
    time_basis: MetricTimeBasis,
    comparison_basis: MetricComparisonBasis,
) -> Option<MetricContext> {
    let (population_basis, denominator_metric_id, limitations) = match metric_id {
        ADULTS => (
            MetricPopulationBasis::SourceDefinedAdults,
            None,
            vec![MetricContextLimitation::NotEmploymentCount],
        ),
        SMALL_CHILDREN => (
            MetricPopulationBasis::SourceDefinedSmallChildren,
            None,
            vec![MetricContextLimitation::SourceAgeBoundaryUnverified],
        ),
        UNEMPLOYED => (
            MetricPopulationBasis::SourceDefinedUnemployed,
            None,
            vec![MetricContextLimitation::SourceWindowUnverified],
        ),
        NO_EDUCATION | BASIC_EDUCATION | HIGHER_EDUCATION => (
            MetricPopulationBasis::AllRecordedCitizens,
            None,
            vec![MetricContextLimitation::NotWorkersOnly],
        ),
        RECEIVER_NONE | RECEIVER_RADIO | RECEIVER_TELEVISION | RECEIVER_COMPUTER => (
            MetricPopulationBasis::ClassifiedReceiverPopulation,
            Some(RECEIVER_TOTAL.to_owned()),
            vec![MetricContextLimitation::ExcludesUnclassifiedCitizens],
        ),
        RECEIVER_TOTAL => (
            MetricPopulationBasis::ClassifiedReceiverPopulation,
            None,
            vec![MetricContextLimitation::ExcludesUnclassifiedCitizens],
        ),
        BORN | DEAD | ESCAPED | IMMIGRANT_SOVIET | IMMIGRANT_AFRICA => (
            MetricPopulationBasis::SourceDefinedMovementCounter,
            None,
            vec![
                MetricContextLimitation::SourceWindowUnverified,
                MetricContextLimitation::NotIntervalFlow,
            ],
        ),
        _ => return None,
    };
    Some(MetricContext {
        population_basis,
        time_basis,
        geographic_scope: MetricGeographicScope::WholeRepublic,
        denominator_metric_id,
        comparison_basis,
        limitations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_contexts_cover_every_plan_metric() {
        let published = published_metric_contexts();
        for metric_id in PLAN_METRIC_IDS {
            assert!(published.iter().any(|entry| entry.metric_id == metric_id));
            assert_eq!(
                plan_context(metric_id)
                    .expect("plan context")
                    .comparison_basis,
                MetricComparisonBasis::PlayerPlanSchedule
            );
        }
    }
}
