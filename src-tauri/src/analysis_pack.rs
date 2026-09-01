use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ObservatoryError;
use crate::model::{CoverageStatus, ReceiverDataset};

pub const ANALYSIS_PACK_SCHEMA_VERSION: u32 = 1;
pub const ANALYSIS_PACK_HOST_API_VERSION: u32 = 1;
pub const MAX_ANALYSIS_PACK_BYTES: usize = 512 * 1024;
pub const MAX_DERIVED_METRICS: usize = 64;
pub const MAX_CHARTS: usize = 16;
pub const MAX_SERIES_PER_CHART: usize = 12;
pub const MAX_OPERANDS: usize = 16;

const CORE_METRICS: [&str; 4] = [
    "core.citizens.electronics.none",
    "core.citizens.electronics.radio",
    "core.citizens.electronics.television",
    "core.citizens.electronics.computer",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisPackDocument {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub host_api_version: u32,
    #[serde(default)]
    pub default_locale: Option<String>,
    pub name: String,
    pub author: String,
    pub description: String,
    pub derived_metrics: Vec<DerivedMetricDeclaration>,
    pub charts: Vec<AnalysisChartTemplate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedMetricDeclaration {
    pub id: String,
    pub label: String,
    pub unit: String,
    #[serde(default)]
    pub description: Option<String>,
    pub operation: AnalysisOperation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnalysisOperation {
    Sum {
        operands: Vec<MetricReference>,
    },
    Difference {
        minuend: MetricReference,
        subtrahend: MetricReference,
    },
    Product {
        operands: Vec<MetricReference>,
    },
    SafeRatio {
        numerator: MetricReference,
        denominator: MetricReference,
        #[serde(default)]
        scale: Option<f64>,
    },
    Scale {
        operand: MetricReference,
        factor: f64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricReference {
    Core(CoreMetricReference),
    Derived(DerivedMetricReference),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreMetricReference {
    pub core_metric: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedMetricReference {
    pub derived_metric: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisChartTemplate {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: AnalysisChartKind,
    #[serde(default)]
    pub orientation: Option<AnalysisChartOrientation>,
    #[serde(default)]
    pub category_axis_label: Option<String>,
    #[serde(default)]
    pub value_axis_label: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub value_domain: Option<AnalysisValueDomain>,
    pub series: Vec<AnalysisChartSeriesTemplate>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisChartKind {
    Line,
    Area,
    Bar,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisChartOrientation {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisSeriesStyle {
    Solid,
    Dashed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisValueDomain {
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisChartSeriesTemplate {
    pub id: String,
    pub label: String,
    pub metric: MetricReference,
    #[serde(default)]
    pub style: Option<AnalysisSeriesStyle>,
    #[serde(default)]
    pub stack_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalysisPackInspection {
    pub valid: bool,
    pub code: Option<String>,
    pub pack_id: Option<String>,
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub host_api_version: Option<u32>,
    pub default_locale: Option<String>,
    pub description: Option<String>,
    pub content_hash: Option<String>,
    pub consumed_metrics: Vec<String>,
    pub derived_metrics: Vec<String>,
    pub charts: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalysisPackSummary {
    pub pack_id: String,
    pub display_name: String,
    pub author: String,
    pub default_locale: String,
    pub description: String,
    pub active_revision: Option<u32>,
    pub latest_revision: u32,
    pub revision_count: u32,
    pub semantic_version: String,
    pub host_api_version: u32,
    pub content_hash: String,
    pub derived_metric_count: u32,
    pub chart_count: u32,
    pub enabled: bool,
    pub validation_state: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalysisPackContribution {
    pub pack_id: String,
    pub version: String,
    pub content_hash: String,
    pub default_locale: String,
    pub charts: Vec<ResolvedAnalysisChart>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedAnalysisChart {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: AnalysisChartKind,
    pub orientation: Option<AnalysisChartOrientation>,
    pub category_axis_label: Option<String>,
    pub value_axis_label: Option<String>,
    pub unit: Option<String>,
    pub value_domain: Option<AnalysisValueDomain>,
    pub series: Vec<ResolvedAnalysisSeries>,
    pub provenance: AnalysisProvenance,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedAnalysisSeries {
    pub id: String,
    pub label: String,
    pub published_metric_id: Option<String>,
    pub style: Option<AnalysisSeriesStyle>,
    pub stack_id: Option<String>,
    pub points: Vec<ResolvedAnalysisPoint>,
    pub provenance: AnalysisProvenance,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedAnalysisPoint {
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub value: f64,
    pub gap_before: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalysisProvenance {
    pub kind: String,
    pub source: String,
    pub observed_at: String,
    pub coverage: String,
}

impl AnalysisPackDocument {
    pub fn parse(bytes: &[u8]) -> Result<Self, ObservatoryError> {
        if bytes.is_empty() || bytes.len() > MAX_ANALYSIS_PACK_BYTES {
            return Err(ObservatoryError::InvalidAnalysisPack("size_limit"));
        }
        let document = serde_json::from_slice::<Self>(bytes)
            .map_err(|_| ObservatoryError::InvalidAnalysisPack("invalid_json"))?;
        document.validate()?;
        Ok(document)
    }

    pub fn validate(&self) -> Result<(), ObservatoryError> {
        if self.schema_version != ANALYSIS_PACK_SCHEMA_VERSION {
            return Err(ObservatoryError::InvalidAnalysisPack(
                "unsupported_schema_version",
            ));
        }
        if self.host_api_version != ANALYSIS_PACK_HOST_API_VERSION {
            return Err(ObservatoryError::InvalidAnalysisPack(
                "unsupported_host_api_version",
            ));
        }
        if !valid_pack_id(&self.id) || !valid_semver(&self.version) {
            return Err(ObservatoryError::InvalidAnalysisPack(
                "invalid_identifier_or_version",
            ));
        }
        if !safe_text(&self.name, 80)
            || !safe_text(&self.author, 120)
            || !safe_text(&self.description, 500)
            || self
                .default_locale
                .as_deref()
                .is_some_and(|locale| !valid_locale(locale))
        {
            return Err(ObservatoryError::InvalidAnalysisPack("invalid_metadata"));
        }
        if self.derived_metrics.len() > MAX_DERIVED_METRICS || self.charts.len() > MAX_CHARTS {
            return Err(ObservatoryError::InvalidAnalysisPack("limit_exceeded"));
        }

        let mut available_derived = BTreeSet::new();
        for metric in &self.derived_metrics {
            if !valid_local_id(&metric.id)
                || !available_derived.insert(metric.id.clone())
                || !safe_text(&metric.label, 80)
                || !safe_text(&metric.unit, 32)
                || metric
                    .description
                    .as_deref()
                    .is_some_and(|description| !safe_text(description, 240))
            {
                return Err(ObservatoryError::InvalidAnalysisPack(
                    "invalid_derived_metric",
                ));
            }
            self.validate_operation(&metric.operation, &available_derived, &metric.id)?;
        }

        let declared_metrics = available_derived;
        let mut chart_ids = BTreeSet::new();
        for chart in &self.charts {
            if chart.schema_version != ANALYSIS_PACK_SCHEMA_VERSION
                || !valid_local_id(&chart.id)
                || !chart_ids.insert(chart.id.clone())
                || !safe_text(&chart.title, 100)
                || !safe_text(&chart.description, 500)
                || chart.series.is_empty()
                || chart.series.len() > MAX_SERIES_PER_CHART
                || chart
                    .category_axis_label
                    .as_deref()
                    .is_some_and(|label| !safe_text(label, 80))
                || chart
                    .value_axis_label
                    .as_deref()
                    .is_some_and(|label| !safe_text(label, 80))
                || chart
                    .unit
                    .as_deref()
                    .is_some_and(|unit| !safe_text(unit, 32))
            {
                return Err(ObservatoryError::InvalidAnalysisPack(
                    "invalid_chart_template",
                ));
            }
            if chart.value_domain.as_ref().is_some_and(|domain| {
                !domain.min.is_finite() || !domain.max.is_finite() || domain.min >= domain.max
            }) {
                return Err(ObservatoryError::InvalidAnalysisPack(
                    "invalid_chart_domain",
                ));
            }
            let mut series_ids = BTreeSet::new();
            for series in &chart.series {
                if !valid_local_id(&series.id)
                    || !series_ids.insert(series.id.clone())
                    || !safe_text(&series.label, 80)
                    || series
                        .stack_id
                        .as_deref()
                        .is_some_and(|id| !valid_local_id(id))
                {
                    return Err(ObservatoryError::InvalidAnalysisPack(
                        "invalid_chart_series",
                    ));
                }
                validate_reference(&series.metric, &declared_metrics, false)?;
            }
        }
        Ok(())
    }

    fn validate_operation(
        &self,
        operation: &AnalysisOperation,
        available_derived: &BTreeSet<String>,
        current_metric: &str,
    ) -> Result<(), ObservatoryError> {
        let references = operation.references();
        if references.len() > MAX_OPERANDS
            || matches!(
                operation,
                AnalysisOperation::Sum { .. } | AnalysisOperation::Product { .. }
            ) && references.len() < 2
        {
            return Err(ObservatoryError::InvalidAnalysisPack("invalid_operation"));
        }
        if operation.scalar().is_some_and(|value| !value.is_finite()) {
            return Err(ObservatoryError::InvalidAnalysisPack("invalid_operation"));
        }
        for reference in references {
            validate_reference(reference, available_derived, true)?;
            if matches!(reference, MetricReference::Derived(value) if value.derived_metric == current_metric)
            {
                return Err(ObservatoryError::InvalidAnalysisPack(
                    "forward_or_cyclic_reference",
                ));
            }
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> Result<String, ObservatoryError> {
        serde_json::to_string_pretty(self)
            .map_err(|_| ObservatoryError::InvalidAnalysisPack("invalid_json"))
    }

    pub fn content_hash(&self) -> Result<String, ObservatoryError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|_| ObservatoryError::InvalidAnalysisPack("invalid_json"))?;
        Ok(Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }

    pub fn default_locale(&self) -> &str {
        self.default_locale.as_deref().unwrap_or("en-AU")
    }

    pub fn inspection(&self) -> Result<AnalysisPackInspection, ObservatoryError> {
        let mut consumed_metrics = BTreeSet::new();
        for metric in &self.derived_metrics {
            for reference in metric.operation.references() {
                if let MetricReference::Core(reference) = reference {
                    consumed_metrics.insert(reference.core_metric.clone());
                }
            }
        }
        for chart in &self.charts {
            for series in &chart.series {
                if let MetricReference::Core(reference) = &series.metric {
                    consumed_metrics.insert(reference.core_metric.clone());
                }
            }
        }
        Ok(AnalysisPackInspection {
            valid: true,
            code: None,
            pack_id: Some(self.id.clone()),
            name: Some(self.name.clone()),
            author: Some(self.author.clone()),
            version: Some(self.version.clone()),
            host_api_version: Some(self.host_api_version),
            default_locale: Some(self.default_locale().to_owned()),
            description: Some(self.description.clone()),
            content_hash: Some(self.content_hash()?),
            consumed_metrics: consumed_metrics.into_iter().collect(),
            derived_metrics: self
                .derived_metrics
                .iter()
                .map(|metric| metric.id.clone())
                .collect(),
            charts: self
                .charts
                .iter()
                .map(|chart| chart.title.clone())
                .collect(),
        })
    }

    pub fn resolve(
        &self,
        content_hash: &str,
        dataset: &ReceiverDataset,
    ) -> AnalysisPackContribution {
        const MAX_CHART_POINTS: usize = 10_000;
        let first_chart_point = dataset.points.len().saturating_sub(MAX_CHART_POINTS);
        let chart_points = &dataset.points[first_chart_point..];
        let evaluated = chart_points
            .iter()
            .map(|point| {
                let mut values = BTreeMap::from([
                    (CORE_METRICS[0].to_owned(), Some(point.none as f64)),
                    (CORE_METRICS[1].to_owned(), Some(point.radio as f64)),
                    (CORE_METRICS[2].to_owned(), Some(point.television as f64)),
                    (CORE_METRICS[3].to_owned(), Some(point.computer as f64)),
                ]);
                for metric in &self.derived_metrics {
                    let value = evaluate_operation(&metric.operation, &values);
                    values.insert(metric.id.clone(), value);
                }
                values
            })
            .collect::<Vec<_>>();

        let observed_at = dataset
            .points
            .last()
            .map(|point| format!("Y{} D{:03}", point.year, point.day))
            .unwrap_or_else(|| "No observations".to_owned());
        let base_coverage_partial =
            dataset.coverage.status == CoverageStatus::Partial || first_chart_point > 0;
        let hash_label = &content_hash[..content_hash.len().min(12)];
        let chart_source = bounded_source(format!(
            "{} {} · {} · {} selected observations · branch {} · head {} · context {} · {}",
            self.id,
            self.version,
            hash_label,
            chart_points.len(),
            dataset.branch_id,
            &dataset.interpretation_id[..dataset.interpretation_id.len().min(12)],
            dataset.analysis_context_id.as_deref().unwrap_or("unbound"),
            dataset.geographic_scope
        ));

        let charts = self
            .charts
            .iter()
            .map(|chart| {
                let series = chart
                    .series
                    .iter()
                    .map(|series| {
                        let mut last_included_index: Option<usize> = None;
                        let points = chart_points
                            .iter()
                            .enumerate()
                            .filter_map(|(index, point)| {
                                let value = lookup_reference(&series.metric, &evaluated[index])?;
                                let gap_before = last_included_index.is_some_and(|previous| {
                                    index != previous + 1
                                        || point.game_day - chart_points[previous].game_day > 14
                                });
                                last_included_index = Some(index);
                                Some(ResolvedAnalysisPoint {
                                    year: point.year,
                                    day: point.day,
                                    game_day: point.game_day,
                                    value,
                                    gap_before,
                                })
                            })
                            .collect::<Vec<_>>();
                        let series_coverage =
                            if base_coverage_partial || points.len() < chart_points.len() {
                                "partial"
                            } else {
                                "complete"
                            };
                        ResolvedAnalysisSeries {
                            id: series.id.clone(),
                            label: series.label.clone(),
                            published_metric_id: match &series.metric {
                                MetricReference::Core(reference) => {
                                    Some(reference.core_metric.clone())
                                }
                                MetricReference::Derived(_) => None,
                            },
                            style: series.style,
                            stack_id: series.stack_id.clone(),
                            points,
                            provenance: AnalysisProvenance {
                                kind: "extension_calculation".to_owned(),
                                source: bounded_source(format!(
                                    "{chart_source} · rule {}",
                                    series.metric.id()
                                )),
                                observed_at: observed_at.clone(),
                                coverage: series_coverage.to_owned(),
                            },
                        }
                    })
                    .collect::<Vec<_>>();
                let chart_coverage = if series
                    .iter()
                    .any(|series| series.provenance.coverage == "partial")
                {
                    "partial"
                } else {
                    "complete"
                };
                ResolvedAnalysisChart {
                    schema_version: chart.schema_version,
                    id: chart.id.clone(),
                    title: chart.title.clone(),
                    description: chart.description.clone(),
                    kind: chart.kind,
                    orientation: chart.orientation,
                    category_axis_label: chart.category_axis_label.clone(),
                    value_axis_label: chart.value_axis_label.clone(),
                    unit: chart.unit.clone(),
                    value_domain: chart.value_domain.clone(),
                    series,
                    provenance: AnalysisProvenance {
                        kind: "extension_calculation".to_owned(),
                        source: chart_source.clone(),
                        observed_at: observed_at.clone(),
                        coverage: chart_coverage.to_owned(),
                    },
                }
            })
            .collect();

        AnalysisPackContribution {
            pack_id: self.id.clone(),
            version: self.version.clone(),
            content_hash: content_hash.to_owned(),
            default_locale: self.default_locale().to_owned(),
            charts,
        }
    }
}

impl AnalysisOperation {
    fn references(&self) -> Vec<&MetricReference> {
        match self {
            Self::Sum { operands } | Self::Product { operands } => operands.iter().collect(),
            Self::Difference {
                minuend,
                subtrahend,
            } => vec![minuend, subtrahend],
            Self::SafeRatio {
                numerator,
                denominator,
                ..
            } => vec![numerator, denominator],
            Self::Scale { operand, .. } => vec![operand],
        }
    }

    fn scalar(&self) -> Option<f64> {
        match self {
            Self::SafeRatio { scale, .. } => *scale,
            Self::Scale { factor, .. } => Some(*factor),
            _ => None,
        }
    }
}

impl MetricReference {
    fn id(&self) -> &str {
        match self {
            Self::Core(reference) => &reference.core_metric,
            Self::Derived(reference) => &reference.derived_metric,
        }
    }
}

fn validate_reference(
    reference: &MetricReference,
    available_derived: &BTreeSet<String>,
    enforce_prior_declaration: bool,
) -> Result<(), ObservatoryError> {
    match reference {
        MetricReference::Core(reference) => {
            if !CORE_METRICS.contains(&reference.core_metric.as_str()) {
                return Err(ObservatoryError::InvalidAnalysisPack("unknown_core_metric"));
            }
        }
        MetricReference::Derived(reference) => {
            if !valid_local_id(&reference.derived_metric)
                || !available_derived.contains(&reference.derived_metric)
            {
                return Err(ObservatoryError::InvalidAnalysisPack(
                    if enforce_prior_declaration {
                        "forward_or_cyclic_reference"
                    } else {
                        "unknown_derived_metric"
                    },
                ));
            }
        }
    }
    Ok(())
}

fn evaluate_operation(
    operation: &AnalysisOperation,
    values: &BTreeMap<String, Option<f64>>,
) -> Option<f64> {
    let operands = operation
        .references()
        .into_iter()
        .map(|reference| lookup_reference(reference, values))
        .collect::<Option<Vec<_>>>()?;
    let result = match operation {
        AnalysisOperation::Sum { .. } => operands.iter().sum(),
        AnalysisOperation::Difference { .. } => operands[0] - operands[1],
        AnalysisOperation::Product { .. } => operands.iter().product(),
        AnalysisOperation::SafeRatio { scale, .. } => {
            let denominator = operands[1];
            if denominator == 0.0 || !denominator.is_finite() {
                return None;
            }
            operands[0] / denominator * scale.unwrap_or(1.0)
        }
        AnalysisOperation::Scale { factor, .. } => operands[0] * factor,
    };
    result.is_finite().then_some(result)
}

fn lookup_reference(
    reference: &MetricReference,
    values: &BTreeMap<String, Option<f64>>,
) -> Option<f64> {
    values
        .get(reference.id())
        .copied()
        .flatten()
        .filter(|value| value.is_finite())
}

fn bounded_source(mut value: String) -> String {
    const MAXIMUM: usize = 240;
    if value.len() <= MAXIMUM {
        return value;
    }
    let mut boundary = MAXIMUM.saturating_sub(1);
    while !value.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    value.truncate(boundary);
    value.push('…');
    value
}

fn valid_pack_id(value: &str) -> bool {
    value.len() <= 128
        && value.split(['.', '-']).count() >= 2
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
        && value.split(['.', '-']).all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        })
}

fn valid_local_id(value: &str) -> bool {
    value.len() <= 64
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
        && value.split(['.', '_', '-']).all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        })
}

fn safe_text(value: &str, maximum: usize) -> bool {
    let lower = value.to_ascii_lowercase();
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.contains(['<', '>', '\\'])
        && !value.contains("://")
        && !lower.contains("file:")
        && !lower.contains("javascript:")
        && !lower.contains("data:")
        && !value.split_ascii_whitespace().any(|token| {
            token.starts_with('/')
                || token.starts_with("./")
                || token.starts_with("../")
                || (token.len() >= 3
                    && token.as_bytes()[0].is_ascii_alphabetic()
                    && token.as_bytes()[1] == b':')
        })
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

fn valid_locale(value: &str) -> bool {
    if value.len() > 64 {
        return false;
    }
    let mut parts = value.split('-');
    let Some(language) = parts.next() else {
        return false;
    };
    matches!(language.len(), 2 | 3 | 5..=8)
        && language
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        && parts.all(|part| {
            !part.is_empty()
                && part.len() <= 8
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
}

fn valid_semver(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 {
        return false;
    }
    let mut build_split = value.split('+');
    let without_build = build_split.next().unwrap_or_default();
    let build = build_split.next();
    if build_split.next().is_some()
        || build.is_some_and(|part| !valid_semver_identifiers(part, false))
    {
        return false;
    }
    let mut pre_split = without_build.split('-');
    let core = pre_split.next().unwrap_or_default();
    let pre_release = pre_split.next();
    if pre_split.next().is_some()
        || pre_release.is_some_and(|part| !valid_semver_identifiers(part, true))
    {
        return false;
    }
    let mut parts = core.split('.');
    let valid = (0..3).all(|_| {
        parts.next().is_some_and(|part| {
            !part.is_empty()
                && part.chars().all(|character| character.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'))
        })
    });
    valid && parts.next().is_none()
}

fn valid_semver_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
                && (!reject_numeric_leading_zero
                    || !identifier
                        .chars()
                        .all(|character| character.is_ascii_digit())
                    || identifier == "0"
                    || !identifier.starts_with('0'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CoverageReport, ReceiverHistoryPoint};

    fn example() -> AnalysisPackDocument {
        AnalysisPackDocument::parse(include_bytes!(
            "../../examples/analysis-packs/receiver-adoption-laboratory.roanalysis.json"
        ))
        .expect("published example")
    }

    fn dataset() -> ReceiverDataset {
        ReceiverDataset {
            payload_hash: "a".repeat(64),
            interpretation_id: "b".repeat(64),
            source_file_name: "synthetic.zip".to_owned(),
            source_file_size: 1,
            source_modified_ms: 1,
            imported_at_ms: 1,
            parser_version: "test".to_owned(),
            format_profile: "test".to_owned(),
            compatibility:
                crate::compatibility_profile::ResolvedCompatibilityProfile::reviewed_builtin()
                    .expect("profile")
                    .provenance(),
            branch_id: "main".to_owned(),
            original_branch_id: "main".to_owned(),
            analysis_context_id: Some("ctx-test".to_owned()),
            geographic_scope: "republic".to_owned(),
            coverage: CoverageReport {
                status: CoverageStatus::Complete,
                history_records: 3,
                chartable_records: 3,
                dropped_records: 0,
                warnings: Vec::new(),
            },
            source_fields: Vec::new(),
            points: vec![
                ReceiverHistoryPoint {
                    record_id: 1,
                    year: 1,
                    day: 1,
                    game_day: 366,
                    none: 10,
                    radio: 20,
                    television: 30,
                    computer: 40,
                    classified_total: 100,
                    exact_observation: None,
                },
                ReceiverHistoryPoint {
                    record_id: 2,
                    year: 1,
                    day: 2,
                    game_day: 367,
                    none: 0,
                    radio: 0,
                    television: 0,
                    computer: 0,
                    classified_total: 0,
                    exact_observation: None,
                },
                ReceiverHistoryPoint {
                    record_id: 3,
                    year: 1,
                    day: 20,
                    game_day: 385,
                    none: 25,
                    radio: 25,
                    television: 25,
                    computer: 25,
                    classified_total: 100,
                    exact_observation: None,
                },
            ],
        }
    }

    #[test]
    fn published_example_is_accepted_by_the_authoritative_validator() {
        let pack = example();
        assert_eq!(pack.schema_version, 1);
        assert_eq!(
            pack.inspection()
                .expect("inspection")
                .consumed_metrics
                .len(),
            4
        );
    }

    #[test]
    fn resolves_shares_without_interpolating_zero_denominators() {
        let pack = example();
        let hash = pack.content_hash().expect("hash");
        let contribution = pack.resolve(&hash, &dataset());
        let chart = &contribution.charts[0];
        assert_eq!(chart.series.len(), 4);
        assert_eq!(chart.series[0].points.len(), 2);
        assert_eq!(chart.series[0].points[0].value, 10.0);
        assert_eq!(chart.series[0].points[1].value, 25.0);
        assert!(chart.series[0].points[1].gap_before);
        assert_eq!(chart.provenance.kind, "extension_calculation");
    }

    #[test]
    fn bounds_concrete_chart_points_and_marks_truncation_partial() {
        let pack = example();
        let mut observations = dataset();
        observations.points = (0..10_001)
            .map(|index| ReceiverHistoryPoint {
                record_id: index,
                year: i32::try_from(index / 365).expect("year"),
                day: u16::try_from(index % 365).expect("day"),
                game_day: i64::from(index),
                none: 10,
                radio: 20,
                television: 30,
                computer: 40,
                classified_total: 100,
                exact_observation: None,
            })
            .collect();
        observations.coverage.history_records = 10_001;
        observations.coverage.chartable_records = 10_001;

        let contribution = pack.resolve(&pack.content_hash().expect("hash"), &observations);
        assert_eq!(contribution.charts[0].series[0].points.len(), 10_000);
        assert_eq!(contribution.charts[0].provenance.coverage, "partial");
    }

    #[test]
    fn rejects_unknown_fields_forward_references_and_unpublished_metrics() {
        let injected = br#"{
          "schema_version":1,"id":"org.example.pack","version":"1.0.0",
          "host_api_version":1,"name":"Example","author":"Planner",
          "description":"Example pack","derived_metrics":[],"charts":[],
          "script":"alert(1)"
        }"#;
        assert!(AnalysisPackDocument::parse(injected).is_err());

        let mut pack = example();
        pack.derived_metrics[0].operation = AnalysisOperation::Scale {
            operand: MetricReference::Derived(DerivedMetricReference {
                derived_metric: "radio_share".to_owned(),
            }),
            factor: 1.0,
        };
        assert_eq!(
            pack.validate()
                .expect_err("forward reference")
                .analysis_pack_reason(),
            Some("forward_or_cyclic_reference")
        );

        let mut pack = example();
        pack.charts[0].series[0].metric = MetricReference::Core(CoreMetricReference {
            core_metric: "core.citizens.electronics.pager".to_owned(),
        });
        assert_eq!(
            pack.validate()
                .expect_err("unknown metric")
                .analysis_pack_reason(),
            Some("unknown_core_metric")
        );
    }
}
