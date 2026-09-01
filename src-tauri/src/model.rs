use serde::{Deserialize, Serialize};

pub const PARSER_VERSION: &str = "stats-ini.observatory-history.v2";
pub const REPUBLIC_SCOPE: &str = "republic";

pub const RECEIVER_METRICS: [MetricDefinition; 4] = [
    MetricDefinition {
        id: "core.citizens.electronics.none",
    },
    MetricDefinition {
        id: "core.citizens.electronics.radio",
    },
    MetricDefinition {
        id: "core.citizens.electronics.television",
    },
    MetricDefinition {
        id: "core.citizens.electronics.computer",
    },
];

pub const CITIZEN_STATUS_METRICS: [IndexedMetricDefinition; 9] = [
    IndexedMetricDefinition::new(0, "core.citizens.status.happiness"),
    IndexedMetricDefinition::new(1, "core.citizens.status.food_satisfaction"),
    IndexedMetricDefinition::new(2, "core.citizens.status.health"),
    IndexedMetricDefinition::new(3, "core.citizens.status.government_loyalty"),
    IndexedMetricDefinition::new(4, "core.citizens.status.alcohol_addiction"),
    IndexedMetricDefinition::new(5, "core.citizens.status.culture_enjoyment"),
    IndexedMetricDefinition::new(6, "core.citizens.status.sports_enjoyment"),
    IndexedMetricDefinition::new(7, "core.citizens.status.religion_sympathy"),
    IndexedMetricDefinition::new(8, "core.citizens.status.clothing_quality"),
];

pub const SNAPSHOT_FACTS: [SnapshotFactDefinition; 18] = [
    SnapshotFactDefinition::republic("core.citizens.electronics.none"),
    SnapshotFactDefinition::republic("core.citizens.electronics.radio"),
    SnapshotFactDefinition::republic("core.citizens.electronics.television"),
    SnapshotFactDefinition::republic("core.citizens.electronics.computer"),
    SnapshotFactDefinition::shared("source.stats.citizens.born"),
    SnapshotFactDefinition::shared("source.stats.citizens.dead"),
    SnapshotFactDefinition::shared("source.stats.citizens.escaped"),
    SnapshotFactDefinition::shared("source.stats.citizens.immigrant_soviet"),
    SnapshotFactDefinition::shared("source.stats.citizens.immigrant_africa"),
    SnapshotFactDefinition::republic("source.stats.citizens.small_children"),
    SnapshotFactDefinition::republic("source.stats.citizens.medium_children"),
    SnapshotFactDefinition::republic("source.stats.citizens.adults_parent"),
    SnapshotFactDefinition::republic("source.stats.citizens.adults"),
    SnapshotFactDefinition::republic("source.stats.citizens.unemployed"),
    SnapshotFactDefinition::republic("source.stats.citizens.no_education"),
    SnapshotFactDefinition::republic("source.stats.citizens.basic_education"),
    SnapshotFactDefinition::republic("source.stats.citizens.higher_education"),
    SnapshotFactDefinition::republic("source.stats.citizens.car_owners"),
];

#[derive(Clone, Copy, Debug)]
pub struct MetricDefinition {
    pub id: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct IndexedMetricDefinition {
    pub source_index: u8,
    pub id: &'static str,
}

impl IndexedMetricDefinition {
    const fn new(source_index: u8, id: &'static str) -> Self {
        Self { source_index, id }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SnapshotFactDefinition {
    pub id: &'static str,
    pub republic: bool,
    pub city: bool,
}

impl SnapshotFactDefinition {
    const fn republic(id: &'static str) -> Self {
        Self {
            id,
            republic: true,
            city: false,
        }
    }

    const fn shared(id: &'static str) -> Self {
        Self {
            id,
            republic: true,
            city: true,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceLineSet {
    pub none: u64,
    pub radio: u64,
    pub television: u64,
    pub computer: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceFieldSet {
    pub none: String,
    pub radio: String,
    pub television: String,
    pub computer: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReceiverRecord {
    pub record_id: u32,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub none: u64,
    pub radio: u64,
    pub television: u64,
    pub computer: u64,
    pub classified_total: u64,
    pub source_lines: SourceLineSet,
    pub source_fields: SourceFieldSet,
}

#[derive(Clone, Debug)]
pub struct CitizenStatusRecord {
    pub record_id: u32,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub values: [f64; 9],
    pub source_lines: [u64; 9],
    pub source_fields: [String; 9],
}

#[derive(Clone, Debug, Default)]
pub struct ParsedCitizenStatusData {
    pub records: Vec<CitizenStatusRecord>,
    pub history_records: u32,
    pub dropped_records: u32,
    pub warnings: Vec<CoverageWarning>,
}

impl ParsedCitizenStatusData {
    pub fn coverage_status(&self) -> CoverageStatus {
        if self.warnings.is_empty() && self.records.len() == self.history_records as usize {
            CoverageStatus::Complete
        } else {
            CoverageStatus::Partial
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotScopeKind {
    Republic,
    City,
}

impl SnapshotScopeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Republic => "republic",
            Self::City => "city",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SnapshotFact {
    pub fact_id: String,
    pub source_field: String,
    pub value: u64,
    pub source_line: u64,
}

#[derive(Clone, Debug)]
pub struct SaveSnapshot {
    pub scope_kind: SnapshotScopeKind,
    pub scope_id: String,
    pub facts: Vec<SnapshotFact>,
    pub expected_fact_count: u32,
    pub coverage: CoverageStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    Complete,
    Partial,
}

impl CoverageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageWarning {
    pub code: String,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageReport {
    pub status: CoverageStatus,
    pub history_records: u32,
    pub chartable_records: u32,
    pub dropped_records: u32,
    pub warnings: Vec<CoverageWarning>,
}

#[derive(Clone, Debug)]
pub struct ParsedStats {
    pub payload_hash: String,
    pub records: Vec<ReceiverRecord>,
    pub coverage: CoverageReport,
    pub snapshots: Vec<SaveSnapshot>,
    pub market: ParsedMarketData,
    pub citizen_status: ParsedCitizenStatusData,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketCurrency {
    Rub,
    Usd,
}

impl MarketCurrency {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rub => "rub",
            Self::Usd => "usd",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketPriceSide {
    Purchase,
    Sell,
    Base,
}

impl MarketPriceSide {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Purchase => "purchase",
            Self::Sell => "sell",
            Self::Base => "base",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketTradeDirection {
    Import,
    Export,
}

impl MarketTradeDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Export => "export",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketTradeChannel {
    Standard,
    International,
}

impl MarketTradeChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::International => "international",
        }
    }
}

#[derive(Clone, Debug)]
pub struct MarketPriceRow {
    pub resource_index: u16,
    pub source_field_index: u16,
    pub source_line: u32,
    pub currency: MarketCurrency,
    pub side: MarketPriceSide,
    pub value: f64,
    pub modifier: f64,
}

#[derive(Clone, Debug)]
pub struct MarketTradeRow {
    pub resource_index: u16,
    pub source_field_index: u16,
    pub source_line: u32,
    pub currency: MarketCurrency,
    pub direction: MarketTradeDirection,
    pub channel: MarketTradeChannel,
    pub quantity: f64,
    pub account_value: f64,
}

#[derive(Clone, Debug)]
pub struct MarketScalarRow {
    pub fact_id: String,
    pub source_field_index: u16,
    pub source_line: u32,
    pub currency: Option<MarketCurrency>,
    pub category: Option<i32>,
    pub value: f64,
}

#[derive(Clone, Debug, Default)]
pub struct MarketFactRows {
    pub prices: Vec<MarketPriceRow>,
    pub trades: Vec<MarketTradeRow>,
    pub scalars: Vec<MarketScalarRow>,
}

#[derive(Clone, Debug)]
pub struct MarketHistoryRecord {
    pub record_id: u32,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub rows: MarketFactRows,
}

#[derive(Clone, Debug)]
pub struct MarketSnapshot {
    pub scope_kind: SnapshotScopeKind,
    pub scope_id: String,
    pub rows: MarketFactRows,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedMarketData {
    pub resources: Vec<String>,
    pub source_fields: Vec<String>,
    pub records: Vec<MarketHistoryRecord>,
    pub snapshots: Vec<MarketSnapshot>,
    pub row_count: u32,
    pub warnings: Vec<CoverageWarning>,
}

impl ParsedMarketData {
    pub fn coverage_status(&self) -> CoverageStatus {
        if self.warnings.is_empty() {
            CoverageStatus::Complete
        } else {
            CoverageStatus::Partial
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MarketWarehouseRecord {
    pub record_hash: String,
    pub ordinal: u32,
    pub record_id: u32,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct MarketWarehousePriceFact {
    pub record_hash: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub currency: String,
    pub price_side: String,
    pub resource_token: String,
    pub value: f64,
    pub modifier: f64,
    pub source_field: String,
    pub source_line: u32,
    pub mapping_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct MarketWarehouseTradeFact {
    pub record_hash: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub currency: String,
    pub direction: String,
    pub channel: String,
    pub resource_token: String,
    pub quantity: f64,
    pub account_value: f64,
    pub source_field: String,
    pub source_line: u32,
    pub mapping_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct MarketWarehouseScalarFact {
    pub record_hash: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub fact_id: String,
    pub currency: Option<String>,
    pub category: Option<i32>,
    pub value: f64,
    pub source_field: String,
    pub source_line: u32,
    pub mapping_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct MarketPriceVolatility {
    pub currency: String,
    pub resource_token: String,
    pub robust_log_volatility: f64,
    pub observations: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct MarketWarehouseProjection {
    pub interpretation_id: String,
    pub raw_payload_hash: String,
    pub branch_id: String,
    pub profile_id: String,
    pub profile_version: String,
    pub resolved_profile_hash: String,
    pub mapping_classification: String,
    pub parser_engine_version: String,
    pub records: Vec<MarketWarehouseRecord>,
    pub prices: Vec<MarketWarehousePriceFact>,
    pub trades: Vec<MarketWarehouseTradeFact>,
    pub scalars: Vec<MarketWarehouseScalarFact>,
    pub analytical_trade_history: Vec<MarketTradePoint>,
    pub analytical_price_volatility: Vec<MarketPriceVolatility>,
}

impl MarketWarehouseProjection {
    pub fn row_count(&self) -> u64 {
        self.records
            .len()
            .saturating_add(self.prices.len())
            .saturating_add(self.trades.len())
            .saturating_add(self.scalars.len())
            .min(u64::MAX as usize) as u64
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MarketEvidenceDataset {
    pub analysis_context: AnalysisContext,
    pub projection: Option<MarketWarehouseProjection>,
    pub coverage_status: Option<String>,
    pub history_records: u32,
    pub snapshot_scopes: u32,
    pub row_count: u32,
    pub warnings: Vec<CoverageWarning>,
    pub baskets: Vec<MarketBasketSummary>,
    pub scenarios: Vec<MarketScenarioSummary>,
    pub recorded_save_count: u32,
    pub indexed_save_count: u32,
    pub current_engine_indexed_save_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityProvenance {
    pub profile_id: String,
    pub profile_version: String,
    pub profile_content_hash: String,
    pub resolved_profile_hash: String,
    pub base_profile_hash: Option<String>,
    pub profile_source: String,
    pub mapping_classification: String,
    pub parser_engine_version: String,
}

#[derive(Clone, Debug)]
pub struct SaveInspection {
    pub payload_hash: String,
    pub interpretation_id: String,
    pub compatibility: CompatibilityProvenance,
    pub source_file_name: String,
    pub source_file_size: u64,
    pub source_modified_ms: i64,
    pub source_directory_identity: String,
    pub records: Vec<ReceiverRecord>,
    pub coverage: CoverageReport,
    pub snapshots: Vec<SaveSnapshot>,
    pub market: ParsedMarketData,
    pub citizen_status: ParsedCitizenStatusData,
    pub binary_facts: Vec<BinaryMappedFact>,
}

#[derive(Clone, Debug)]
pub struct BinaryMappedFact {
    pub layout_id: String,
    pub record_index: u32,
    pub host_slot: String,
    pub value: Option<f64>,
    pub source_offset: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MetricEvidence {
    pub metric_id: String,
    pub source_field: String,
    pub latest_source_line: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReceiverHistoryPoint {
    pub record_id: u32,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub none: u64,
    pub radio: u64,
    pub television: u64,
    pub computer: u64,
    pub classified_total: u64,
    pub exact_observation: Option<ExactObservationReference>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ExactObservationReference {
    pub interpretation_id: String,
    pub branch_id: String,
    pub year: i32,
    pub day: u16,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReceiverDataset {
    pub payload_hash: String,
    pub interpretation_id: String,
    pub source_file_name: String,
    pub source_file_size: u64,
    pub source_modified_ms: i64,
    pub imported_at_ms: i64,
    pub parser_version: String,
    pub format_profile: String,
    pub compatibility: CompatibilityProvenance,
    pub branch_id: String,
    pub original_branch_id: String,
    pub analysis_context_id: Option<String>,
    pub geographic_scope: String,
    pub coverage: CoverageReport,
    pub source_fields: Vec<MetricEvidence>,
    pub points: Vec<ReceiverHistoryPoint>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BroadcastMetricDefinition {
    pub metric_id: String,
    pub source_index: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct CitizenStatusPoint {
    pub ordinal: u32,
    pub record_id: u32,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub values: [f64; 9],
    pub source_fields: [String; 9],
    pub source_lines: [u64; 9],
    pub exact_observation: Option<ExactObservationReference>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BroadcastReceiverClassPulse {
    pub metric_id: String,
    pub count: u64,
    pub share_percent: f64,
    pub change_from_previous: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BroadcastPulse {
    pub year: i32,
    pub day: u16,
    pub classified_population: u64,
    pub classes: Vec<BroadcastReceiverClassPulse>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BroadcastStationRequirement {
    pub station_kind: String,
    pub catalogue_entity_id: String,
    pub workers: u32,
    pub professors: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct BroadcastAvailability {
    pub potential_audience: bool,
    pub current_audience: bool,
    pub programme_settings: bool,
    pub demographic_receiver_join: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct BroadcastWorkspaceModel {
    pub analysis_context: AnalysisContext,
    pub receiver: Option<ReceiverDataset>,
    pub pulse: Option<BroadcastPulse>,
    pub status_metrics: Vec<BroadcastMetricDefinition>,
    pub status_coverage: Option<CoverageReport>,
    pub citizen_status_points: Vec<CitizenStatusPoint>,
    pub station_requirements: Vec<BroadcastStationRequirement>,
    pub availability: BroadcastAvailability,
    pub warehouse_projection_available: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct BroadcastEvidenceDataset {
    pub analysis_context: AnalysisContext,
    pub receiver: Option<ReceiverDataset>,
    pub status_coverage: Option<CoverageReport>,
    pub citizen_status_points: Vec<CitizenStatusPoint>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BroadcastOutcomeRequest {
    pub receiver_metric_id: String,
    pub status_metric_id: String,
    pub lag_confirmed_records: u8,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BroadcastOutcomeAvailability {
    Available,
    ReceiverUnavailable,
    StatusUnavailable,
    InsufficientPairs,
    ConstantReceiverChanges,
    ConstantStatusChanges,
}

#[derive(Clone, Debug, Serialize)]
pub struct BroadcastOutcomePair {
    pub receiver_record_id: u32,
    pub receiver_year: i32,
    pub receiver_day: u16,
    pub receiver_game_day: i64,
    pub status_record_id: u32,
    pub status_year: i32,
    pub status_day: u16,
    pub status_game_day: i64,
    pub elapsed_game_days: i64,
    pub receiver_share_change: f64,
    pub status_change: f64,
    pub exact_observation: Option<ExactObservationReference>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BroadcastOutcomeModel {
    pub availability: BroadcastOutcomeAvailability,
    pub receiver_metric_id: String,
    pub status_metric_id: String,
    pub lag_confirmed_records: u8,
    pub coefficient: Option<f64>,
    pub pair_count: u32,
    pub start_year: Option<i32>,
    pub start_day: Option<u16>,
    pub end_year: Option<i32>,
    pub end_day: Option<u16>,
    pub elapsed_days_median: Option<f64>,
    pub elapsed_days_min: Option<i64>,
    pub elapsed_days_max: Option<i64>,
    pub pairs: Vec<BroadcastOutcomePair>,
}

#[derive(Clone, Debug)]
pub(crate) struct BroadcastWarehouseRecord {
    pub record_hash: String,
    pub ordinal: u32,
    pub record_id: u32,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct BroadcastWarehouseFact {
    pub record_hash: String,
    pub source_index: u8,
    pub metric_id: String,
    pub value: f64,
    pub source_field: String,
    pub source_line: u64,
    pub mapping_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct BroadcastWarehouseProjection {
    pub interpretation_id: String,
    pub raw_payload_hash: String,
    pub branch_id: String,
    pub profile_id: String,
    pub profile_version: String,
    pub resolved_profile_hash: String,
    pub mapping_classification: String,
    pub records: Vec<BroadcastWarehouseRecord>,
    pub facts: Vec<BroadcastWarehouseFact>,
}

impl BroadcastWarehouseProjection {
    pub fn row_count(&self) -> u64 {
        self.records
            .len()
            .saturating_add(self.facts.len())
            .min(u64::MAX as usize) as u64
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TimelineBranch {
    pub branch_id: String,
    pub branch_kind: String,
    pub parent_branch_id: Option<String>,
    pub fork_record_id: Option<u32>,
    pub observation_count: u32,
    pub latest_year: Option<i32>,
    pub latest_day: Option<u16>,
    pub selected: bool,
    pub origin: AnalysisContextOrigin,
    pub short_identity: String,
    pub player_label: Option<String>,
    pub anchor_interpretation_id: Option<String>,
    pub membership_revision: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArchiveObservation {
    pub payload_hash: String,
    pub interpretation_id: String,
    pub mapping_classification: String,
    pub profile_id: String,
    pub profile_version: String,
    pub resolved_profile_hash: String,
    pub source_file_name: String,
    pub imported_at_ms: i64,
    pub branch_id: String,
    pub relationship: String,
    pub parent_payload_hash: Option<String>,
    pub shared_record_count: u32,
    pub latest_year: Option<i32>,
    pub latest_day: Option<u16>,
    pub history_records: u32,
    pub coverage_status: CoverageStatus,
    pub file_observation_count: u32,
    pub republic_snapshot_fields: u32,
    pub city_snapshot_count: u32,
    pub city_snapshot_fields: u32,
    pub included_in_context: bool,
    pub active_head: bool,
    pub context_sequence: Option<u32>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisContextMode {
    Latest,
    HistoricalPreview,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisContextOrigin {
    Automatic,
    ManualContinuation,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalysisContext {
    pub context_id: String,
    pub selected_branch_id: String,
    pub head_interpretation_id: Option<String>,
    pub original_branch_id: Option<String>,
    pub mode: AnalysisContextMode,
    pub origin: AnalysisContextOrigin,
    pub is_tip: bool,
    pub membership_revision: u32,
    pub compatibility_profile_id: Option<String>,
    pub compatibility_profile_hash: Option<String>,
    pub observation_watermark: Option<String>,
    pub catalogue_generation_id: Option<String>,
    pub overlay_revision: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArchiveOverview {
    pub selected_branch_id: String,
    pub file_observation_count: u32,
    pub distinct_state_count: u32,
    pub unresolved_state_count: u32,
    pub branches: Vec<TimelineBranch>,
    pub observations: Vec<ArchiveObservation>,
    pub analysis_context: AnalysisContext,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalysisContextResult {
    pub archive: ArchiveOverview,
    pub context: AnalysisContext,
    pub dataset: Option<ReceiverDataset>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PopulationFact {
    pub fact_id: String,
    pub value: u64,
    pub source_field: String,
    pub source_line: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct PopulationObservation {
    pub interpretation_id: String,
    pub source_file_name: String,
    pub membership_revision: u32,
    pub sampled_year: i32,
    pub sampled_day: u16,
    pub sampled_game_day: i64,
    pub coverage_status: CoverageStatus,
    pub mapping_classification: String,
    pub profile_id: String,
    pub profile_version: String,
    pub resolved_profile_hash: String,
    pub exact_observation: Option<ExactObservationReference>,
    pub facts: Vec<PopulationFact>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PopulationCitySnapshot {
    pub scope_id: String,
    pub sampled_year: i32,
    pub sampled_day: u16,
    pub sampled_game_day: i64,
    pub coverage_status: CoverageStatus,
    pub facts: Vec<PopulationFact>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PopulationDataset {
    pub analysis_context: AnalysisContext,
    pub observations: Vec<PopulationObservation>,
    pub cities: Vec<PopulationCitySnapshot>,
    pub observation_limit: u32,
    pub city_limit: u32,
    pub tesmio_probe: TesmioProbeStatus,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BriefMetricRole {
    Headline,
    Education,
    ReceiverClass,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BriefEvidenceKind {
    SaveFact,
    Calculation,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BriefSourceEvidence {
    pub source_field: String,
    pub source_line: u64,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricPopulationBasis {
    AllRecordedCitizens,
    SourceDefinedAdults,
    SourceDefinedSmallChildren,
    SourceDefinedUnemployed,
    SourceDefinedMovementCounter,
    SourceDefinedCitizenStatus,
    ClassifiedReceiverPopulation,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricTimeBasis {
    ExactSelectedObservation,
    BranchObservationsThroughSelectedHead,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricGeographicScope {
    WholeRepublic,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricComparisonBasis {
    ProvenPrecedingSameBranchAndProfile,
    PlayerPlanSchedule,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricContextLimitation {
    NotEmploymentCount,
    NotWorkersOnly,
    SourceAgeBoundaryUnverified,
    SourceWindowUnverified,
    ExcludesUnclassifiedCitizens,
    NotIntervalFlow,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MetricContext {
    pub population_basis: MetricPopulationBasis,
    pub time_basis: MetricTimeBasis,
    pub geographic_scope: MetricGeographicScope,
    pub denominator_metric_id: Option<String>,
    pub comparison_basis: MetricComparisonBasis,
    pub limitations: Vec<MetricContextLimitation>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublishedMetricContext {
    pub metric_id: String,
    pub exact: MetricContext,
    pub history: MetricContext,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanScheduleKind {
    Linear,
    Milestone,
    HoldThenChange,
}

impl PlanScheduleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Milestone => "milestone",
            Self::HoldThenChange => "hold_then_change",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanDirection {
    Increase,
    Decrease,
    Maintain,
}

impl PlanDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Increase => "increase",
            Self::Decrease => "decrease",
            Self::Maintain => "maintain",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanTargetState {
    AwaitingStart,
    Ahead,
    OnTrack,
    Behind,
    Complete,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PlanTargetDraft {
    pub metric_id: String,
    pub target_value: u64,
    pub direction: PlanDirection,
    pub guardrail_basis_points: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RepublicPlanDraft {
    pub plan_id: Option<String>,
    pub name: String,
    pub end_year: i32,
    pub end_day: u16,
    pub schedule: PlanScheduleKind,
    pub targets: Vec<PlanTargetDraft>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RepublicPlanTarget {
    pub metric_id: String,
    pub baseline_value: u64,
    pub target_value: u64,
    pub direction: PlanDirection,
    pub guardrail_basis_points: u16,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RepublicPlanRevision {
    pub plan_id: String,
    pub name: String,
    pub revision: u32,
    pub branch_id: String,
    pub start_interpretation_id: String,
    pub start_profile_hash: String,
    pub start_year: i32,
    pub start_day: u16,
    pub start_game_day: i64,
    pub end_year: i32,
    pub end_day: u16,
    pub end_game_day: i64,
    pub schedule: PlanScheduleKind,
    pub created_at_ms: i64,
    pub targets: Vec<RepublicPlanTarget>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RepublicPlanListItem {
    pub plan_id: String,
    pub name: String,
    pub branch_id: String,
    pub active_revision: u32,
    pub latest_revision: u32,
    pub revision_count: u32,
    pub selected: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PlanMetricOption {
    pub metric_id: String,
    pub current_value: Option<u64>,
    pub active_plan_baseline_value: Option<u64>,
    pub context: MetricContext,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PlanSeriesPoint {
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub observed_value: u64,
    pub scheduled_value: u64,
    pub exact_observation: Option<ExactObservationReference>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PlanTargetEvaluation {
    pub target: RepublicPlanTarget,
    pub current_value: Option<u64>,
    pub scheduled_value: Option<u64>,
    pub directional_variance: Option<i64>,
    pub attainment_basis_points: Option<u16>,
    pub guardrail_breached: bool,
    pub state: PlanTargetState,
    pub context: MetricContext,
    pub points: Vec<PlanSeriesPoint>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RepublicPlanEvaluation {
    pub revision: RepublicPlanRevision,
    pub state: PlanTargetState,
    pub attainment_basis_points: Option<u16>,
    pub guardrail_breach_count: u32,
    pub targets: Vec<PlanTargetEvaluation>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RepublicPlanWorkspace {
    pub analysis_context: AnalysisContext,
    pub current_year: Option<i32>,
    pub current_day: Option<u16>,
    pub available_metrics: Vec<PlanMetricOption>,
    pub plans: Vec<RepublicPlanListItem>,
    pub active_plan: Option<RepublicPlanEvaluation>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RepublicPlanBrief {
    pub plan_id: String,
    pub name: String,
    pub revision: u32,
    pub target_count: u32,
    pub end_year: i32,
    pub end_day: u16,
    pub state: PlanTargetState,
    pub attainment_basis_points: Option<u16>,
    pub guardrail_breach_count: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BriefMetric {
    pub metric_id: String,
    pub role: BriefMetricRole,
    pub value: u64,
    pub previous_value: Option<u64>,
    pub delta: Option<i64>,
    pub share_basis_points: Option<u16>,
    pub evidence_kind: BriefEvidenceKind,
    pub sources: Vec<BriefSourceEvidence>,
    pub context: MetricContext,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BriefObservation {
    pub interpretation_id: String,
    pub source_file_name: String,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub coverage_status: CoverageStatus,
    pub mapping_classification: String,
    pub profile_id: String,
    pub profile_version: String,
    pub resolved_profile_hash: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BriefComparisonAnchor {
    pub interpretation_id: String,
    pub source_file_name: String,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BriefFindingSeverity {
    Information,
    Watch,
    Attention,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BriefFinding {
    pub code: String,
    pub severity: BriefFindingSeverity,
    pub value: Option<u64>,
    pub metric_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BriefOperations {
    pub recorder_phase: Option<AutomaticObserverPhase>,
    pub recorder_queue_depth: Option<u32>,
    pub recorder_attention_count: Option<u32>,
    pub warehouse_phase: Option<WarehousePhase>,
    pub warehouse_pending_jobs: Option<u32>,
    pub warehouse_failed_jobs: Option<u32>,
    pub warehouse_lag_ms: Option<i64>,
    pub catalogue_generation_id: Option<String>,
    pub catalogue_entity_count: Option<u32>,
    pub city_scope_count: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct RepublicBrief {
    pub schema_version: u32,
    pub analysis_context: AnalysisContext,
    pub observation: Option<BriefObservation>,
    pub comparison: Option<BriefComparisonAnchor>,
    pub metrics: Vec<BriefMetric>,
    pub findings: Vec<BriefFinding>,
    pub dispatch_code: String,
    pub operations: BriefOperations,
    pub plan: Option<RepublicPlanBrief>,
    pub unavailable_capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TesmioProbeState {
    NotConfigured,
    Missing,
    Available,
    Warning,
    Invalid,
}

#[derive(Clone, Debug, Serialize)]
pub struct TesmioProbeStatus {
    pub state: TesmioProbeState,
    pub read_only: bool,
    pub optional: bool,
    pub persisted: bool,
    pub probe_id: Option<String>,
    pub probe_version: Option<String>,
    pub loader_api_version: Option<u32>,
    pub target_game_version: Option<String>,
    pub executable_timestamp: Option<u64>,
    pub content_hash: Option<String>,
    pub snapshot_count: u32,
    pub sample_count: u32,
    pub latest_year: Option<i32>,
    pub latest_day: Option<u16>,
    pub latest_population_count: Option<u32>,
    pub warnings: Vec<String>,
}

impl TesmioProbeStatus {
    pub fn not_configured() -> Self {
        Self {
            state: TesmioProbeState::NotConfigured,
            read_only: true,
            optional: true,
            persisted: false,
            probe_id: None,
            probe_version: None,
            loader_api_version: None,
            target_game_version: None,
            executable_timestamp: None,
            content_hash: None,
            snapshot_count: 0,
            sample_count: 0,
            latest_year: None,
            latest_day: None,
            latest_population_count: None,
            warnings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BranchMembershipProjection {
    pub branch_id: String,
    pub membership_revision: u32,
    pub interpretation_id: String,
    pub payload_hash: String,
    pub parent_interpretation_id: Option<String>,
    pub relationship: String,
    pub shared_record_count: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ComparisonObservation {
    pub payload_hash: String,
    pub interpretation_id: String,
    pub source_file_name: String,
    pub branch_id: String,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub coverage_status: CoverageStatus,
    pub republic_snapshot_fields: u32,
    pub city_snapshot_count: u32,
    pub city_snapshot_fields: u32,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityValidationState {
    Missing,
    Valid,
    Invalid,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompatibilityProfileSummary {
    pub id: String,
    pub version: String,
    pub content_hash: String,
    pub resolved_hash: String,
    pub source: String,
    pub mapping_classification: String,
    pub base_profile_id: Option<String>,
    pub base_profile_version: Option<String>,
    pub base_profile_hash: Option<String>,
    pub target_game_versions: Vec<String>,
    pub target_build_ids: Vec<String>,
    pub target_stats_formats: Vec<u16>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompatibilityMappingCoverage {
    pub stats_markers: u32,
    pub stats_fields: u32,
    pub definition_operations: u32,
    pub binary_layouts: u32,
    pub catalogue_scopes: u32,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityCatalogueScopeState {
    Matched,
    Dormant,
    UpdatedUnreviewed,
    Conflict,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CompatibilityCatalogueScopeStatus {
    pub id: String,
    pub source_id: String,
    pub package_name: Option<String>,
    pub update_policy: String,
    pub acknowledged_content_hash: String,
    pub current_content_hash: Option<String>,
    pub mapping_count: u32,
    pub state: CompatibilityCatalogueScopeState,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompatibilityStatus {
    pub active: CompatibilityProfileSummary,
    pub reviewed_base: CompatibilityProfileSummary,
    pub local_file_path: String,
    pub local_file_exists: bool,
    pub local_validation: CompatibilityValidationState,
    pub last_validation_error: Option<String>,
    pub last_validated_at_ms: Option<i64>,
    pub detected_game_version: Option<String>,
    pub detected_build_id: Option<String>,
    pub coverage: CompatibilityMappingCoverage,
    pub catalogue_scopes: Vec<CompatibilityCatalogueScopeStatus>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompatibilityUpdate {
    pub status: CompatibilityStatus,
    pub profile_changed: bool,
    pub definition_mapping_changed: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReinterpretationPhase {
    #[default]
    Idle,
    Reading,
    Parsing,
    Persisting,
    QueueingWarehouse,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ReinterpretationProgress {
    pub phase: ReinterpretationPhase,
    pub progress_percent: Option<u8>,
    pub started_at_ms: Option<i64>,
    pub updated_at_ms: Option<i64>,
    pub current_file: Option<String>,
    pub interpretation_id: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketIndexingPhase {
    #[default]
    Idle,
    Discovering,
    Matching,
    ReadingArchive,
    ParsingRecords,
    Persisting,
    QueueingWarehouse,
    Paused,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct MarketIndexingProgress {
    pub job_id: Option<String>,
    pub storage_contract_version: u32,
    pub phase: MarketIndexingPhase,
    pub progress_percent: Option<u8>,
    pub started_at_ms: Option<i64>,
    pub updated_at_ms: Option<i64>,
    pub current_file: Option<String>,
    pub current_archive: u32,
    pub total_archives: u32,
    pub records_processed: u32,
    pub rows_processed: u32,
    pub completed_archives: u32,
    pub missing_archives: u32,
    pub changed_archives: u32,
    pub failed_archives: u32,
    pub duplicate_archives: u32,
    pub cache_records_reused: u32,
    pub cache_rows_avoided: u64,
    pub contention_retries: u32,
    pub contention_wait_ms: u64,
    pub resume_count: u32,
    pub error_code: Option<String>,
}

pub type BroadcastIndexingProgress = MarketIndexingProgress;

#[derive(Clone, Debug, Serialize)]
pub struct MarketMetricContext {
    pub metric_id: String,
    pub formula: String,
    pub currency: Option<String>,
    pub unit: String,
    pub time_basis: String,
    pub exclusions: Vec<String>,
    pub evidence_class: String,
    pub profile_id: String,
    pub profile_version: String,
    pub source_fields: Vec<String>,
    pub analytical_head: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketCurrencyPulse {
    pub currency: String,
    pub standard_import_value: f64,
    pub standard_export_value: f64,
    pub standard_trade_result: f64,
    pub international_import_value: f64,
    pub international_export_value: f64,
    pub international_trade_result: f64,
    pub positive_export_hhi: Option<f64>,
    pub positive_export_resource_count: u32,
    pub context: MarketMetricContext,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketTradePoint {
    pub record_hash: String,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub currency: String,
    pub channel: String,
    pub import_value: f64,
    pub export_value: f64,
    pub trade_result: f64,
    pub exact_observation: Option<ExactObservationReference>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketResourceLedgerRow {
    pub currency: String,
    pub channel: String,
    pub resource_token: String,
    pub import_quantity: f64,
    pub export_quantity: f64,
    pub import_account_value: f64,
    pub export_account_value: f64,
    pub trade_result: f64,
    pub disposal_cost: Option<f64>,
    pub source_fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketPriceLedgerRow {
    pub currency: String,
    pub resource_token: String,
    pub purchase_price: Option<f64>,
    pub sell_price: Option<f64>,
    pub base_price: Option<f64>,
    pub purchase_index: Option<f64>,
    pub sell_index: Option<f64>,
    pub robust_log_volatility: Option<f64>,
    pub volatility_observations: u32,
    pub source_fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketScalarLedgerRow {
    pub fact_id: String,
    pub currency: Option<String>,
    pub category: Option<i32>,
    pub value: f64,
    pub source_field: String,
    pub source_line: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketCityRow {
    pub source_id: String,
    pub currency: String,
    pub channel: String,
    pub import_value: f64,
    pub export_value: f64,
    pub trade_result: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketBasketSummary {
    pub basket_id: String,
    pub revision: u32,
    pub name: String,
    pub currency: String,
    pub price_side: String,
    pub built_in: bool,
    pub selected: bool,
    pub base_record_hash: Option<String>,
    pub resource_count: u32,
    pub coverage_resources: u32,
    pub index_value: Option<f64>,
    pub reason: String,
    pub weights: Vec<MarketBasketWeight>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketTermsOfTradeSummary {
    pub currency: String,
    pub base_record_hash: String,
    pub import_basket_id: String,
    pub import_basket_revision: u32,
    pub export_basket_id: String,
    pub export_basket_revision: u32,
    pub import_index: f64,
    pub export_index: f64,
    pub terms_of_trade_index: f64,
    pub context: MarketMetricContext,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketBasketWeight {
    pub resource_token: String,
    pub weight: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketBasketDraft {
    pub basket_id: String,
    pub name: String,
    pub currency: String,
    pub price_side: String,
    pub base_record_hash: String,
    pub reason: String,
    pub weights: Vec<MarketBasketWeight>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketScenarioSummary {
    pub scenario_id: String,
    pub revision: u32,
    pub name: String,
    pub scenario_kind: String,
    pub reason: String,
    pub assumptions_json: String,
    pub selected: bool,
    pub result_kind: Option<String>,
    pub result_value: Option<f64>,
    pub result_unit: Option<String>,
    pub covered_components: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketScenarioDraft {
    pub scenario_id: String,
    pub name: String,
    pub scenario_kind: String,
    pub currency: String,
    pub reason: String,
    pub domestic_unit_cost: Option<f64>,
    pub delivery_cost: Option<f64>,
    pub operating_efficiency_percent: Option<f64>,
    pub exchange_rate: Option<f64>,
    pub debt_service: Option<f64>,
    pub export_stress_percent: Option<f64>,
    pub tourism_stress_percent: Option<f64>,
    pub included_income_components: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketCoverageFacet {
    pub facet_id: String,
    pub status: String,
    pub observed_slots: u32,
    pub expected_slots: u32,
    pub resource_count: u32,
    pub currencies: Vec<String>,
    pub channels: Vec<String>,
    pub source_fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketCommissioningSummary {
    pub recorded_save_count: u32,
    pub indexed_save_count: u32,
    pub current_engine_indexed_save_count: u32,
    pub pending_current_engine_save_count: u32,
    pub active_engine_current: bool,
    pub active_parser_engine_version: Option<String>,
    pub recommended_currency: Option<String>,
    pub recommended_channel: Option<String>,
    pub recommended_price_resource: Option<String>,
    pub facets: Vec<MarketCoverageFacet>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketPriceSeriesPoint {
    pub record_hash: String,
    pub year: i32,
    pub day: u16,
    pub game_day: i64,
    pub purchase_price: Option<f64>,
    pub sell_price: Option<f64>,
    pub base_price: Option<f64>,
    pub exact_observation: Option<ExactObservationReference>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketPriceSeries {
    pub available: bool,
    pub currency: String,
    pub resource_token: String,
    pub points: Vec<MarketPriceSeriesPoint>,
    pub context: MarketMetricContext,
    pub limitation: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarketWorkspace {
    pub analysis_context: AnalysisContext,
    pub available: bool,
    pub partial: bool,
    pub coverage_status: Option<String>,
    pub history_records: u32,
    pub row_count: u32,
    pub city_scope_count: u32,
    pub warehouse_history_available: bool,
    pub warnings: Vec<CoverageWarning>,
    pub currencies: Vec<MarketCurrencyPulse>,
    pub trade_history: Vec<MarketTradePoint>,
    pub resource_ledger: Vec<MarketResourceLedgerRow>,
    pub price_ledger: Vec<MarketPriceLedgerRow>,
    pub scalar_ledger: Vec<MarketScalarLedgerRow>,
    pub cities: Vec<MarketCityRow>,
    pub baskets: Vec<MarketBasketSummary>,
    pub scenarios: Vec<MarketScenarioSummary>,
    pub metric_contexts: Vec<MarketMetricContext>,
    pub terms_of_trade: Vec<MarketTermsOfTradeSummary>,
    pub reserves_available: bool,
    pub terms_of_trade_available: bool,
    pub limitations: Vec<String>,
    pub commissioning: MarketCommissioningSummary,
}

#[derive(Clone, Debug)]
pub(crate) struct MarketIndexCandidate {
    pub payload_hash: String,
    pub source_file_name: String,
    pub source_file_size: u64,
    pub source_modified_ms: i64,
    pub source_directory_identity: String,
    pub raw_payload_hash: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReceiverClassChange {
    pub metric_id: String,
    pub from_value: u64,
    pub to_value: u64,
    pub delta: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArchiveComparison {
    pub branch_id: String,
    pub elapsed_game_days: i64,
    pub from: ComparisonObservation,
    pub to: ComparisonObservation,
    pub receiver_changes: Vec<ReceiverClassChange>,
    pub classified_total_change: ReceiverClassChange,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportOutcome {
    Imported,
    Duplicate,
}

#[derive(Clone, Debug, Serialize)]
pub struct ObservationImportResult {
    pub outcome: ImportOutcome,
    pub recorded_interpretation_id: String,
    pub active_context_id: String,
    pub dataset: ReceiverDataset,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomaticObserverPhase {
    Disabled,
    NotConfigured,
    Watching,
    WaitingForStability,
    Retrying,
    Observed,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct AutomaticObserverStatus {
    pub enabled: bool,
    pub phase: AutomaticObserverPhase,
    pub candidate_file_name: Option<String>,
    pub retry_attempt: u8,
    pub error_code: Option<String>,
    pub last_observed_file_name: Option<String>,
    pub last_observed_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AutomaticObservationUpdate {
    pub status: AutomaticObserverStatus,
    pub import_result: Option<ObservationImportResult>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecorderCandidateStatus {
    Discovered,
    Stabilising,
    Ready,
    Reading,
    Imported,
    Duplicate,
    RetryableFailure,
    TerminalFailure,
    Superseded,
}

impl RecorderCandidateStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Stabilising => "stabilising",
            Self::Ready => "ready",
            Self::Reading => "reading",
            Self::Imported => "imported",
            Self::Duplicate => "duplicate",
            Self::RetryableFailure => "retryable_failure",
            Self::TerminalFailure => "terminal_failure",
            Self::Superseded => "superseded",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "discovered" => Some(Self::Discovered),
            "stabilising" => Some(Self::Stabilising),
            "ready" => Some(Self::Ready),
            "reading" => Some(Self::Reading),
            "imported" => Some(Self::Imported),
            "duplicate" => Some(Self::Duplicate),
            "retryable_failure" => Some(Self::RetryableFailure),
            "terminal_failure" => Some(Self::TerminalFailure),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Imported | Self::Duplicate | Self::TerminalFailure | Self::Superseded
        )
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecorderDiscoverySource {
    Migration,
    InitialScan,
    FilesystemEvent,
    Reconciliation,
}

impl RecorderDiscoverySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Migration => "migration",
            Self::InitialScan => "initial_scan",
            Self::FilesystemEvent => "filesystem_event",
            Self::Reconciliation => "reconciliation",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "migration" => Some(Self::Migration),
            "initial_scan" => Some(Self::InitialScan),
            "filesystem_event" => Some(Self::FilesystemEvent),
            "reconciliation" => Some(Self::Reconciliation),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RecorderLedgerEntry {
    pub candidate_id: i64,
    pub file_name: String,
    pub file_size: u64,
    pub source_modified_ms: i64,
    pub status: RecorderCandidateStatus,
    pub discovery_source: RecorderDiscoverySource,
    pub discovered_at_ms: i64,
    pub first_stable_at_ms: Option<i64>,
    pub last_attempt_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub attempt_count: u32,
    pub error_code: Option<String>,
    pub import_outcome: Option<ImportOutcome>,
    pub payload_hash: Option<String>,
    pub processing_latency_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecorderHealth {
    pub observer: AutomaticObserverStatus,
    pub last_scan_ms: Option<i64>,
    pub last_filesystem_event_ms: Option<i64>,
    pub last_completed_at_ms: Option<i64>,
    pub last_completed_file_name: Option<String>,
    pub last_processing_latency_ms: Option<i64>,
    pub queue_depth: u32,
    pub attention_count: u32,
    pub completed_count: u32,
    pub latest_entries: Vec<RecorderLedgerEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecorderUpdate {
    pub health: RecorderHealth,
    pub import_result: Option<ObservationImportResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConfiguredDirectorySummary {
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct GameVocabularySource {
    pub source_id: String,
    pub file_name: String,
    pub locale_hint: Option<String>,
    pub format: String,
    pub readable: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SetupState {
    pub save_directory: Option<ConfiguredDirectorySummary>,
    pub game_directory: Option<ConfiguredDirectorySummary>,
    pub workshop_directory: Option<ConfiguredDirectorySummary>,
    pub save_candidates: u32,
    pub observed_saves: u32,
    pub distinct_states: u32,
    pub game_vocabularies: Vec<GameVocabularySource>,
    pub automatic_observer: AutomaticObserverStatus,
    pub compatibility: CompatibilityStatus,
}

pub const APPLICATION_PREFERENCES_SCHEMA_VERSION: u32 = 2;
pub const MIN_STORAGE_PATIENCE_SECONDS: u16 = 5;
pub const MAX_STORAGE_PATIENCE_SECONDS: u16 = 300;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StoragePatiencePreset {
    Short,
    #[default]
    Balanced,
    Patient,
    Custom,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundWorkPriority {
    #[default]
    Gentle,
    Balanced,
    FinishSooner,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MotionPreference {
    #[default]
    System,
    Reduced,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WordingMode {
    #[default]
    PlayerFriendly,
    Technical,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ApplicationPreferencesDraft {
    pub storage_patience_preset: StoragePatiencePreset,
    pub custom_storage_patience_seconds: Option<u16>,
    pub background_work_priority: BackgroundWorkPriority,
    pub text_scale_percent: u16,
    pub motion_preference: MotionPreference,
    pub wording_mode: WordingMode,
    pub automatic_observation_enabled: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ApplicationPreferences {
    pub schema_version: u32,
    pub storage_patience_preset: StoragePatiencePreset,
    pub custom_storage_patience_seconds: Option<u16>,
    pub effective_storage_patience_seconds: u16,
    pub background_work_priority: BackgroundWorkPriority,
    pub text_scale_percent: u16,
    pub motion_preference: MotionPreference,
    pub wording_mode: WordingMode,
    pub automatic_observation_enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApplicationSettingsView {
    pub preferences: ApplicationPreferences,
    pub setup: SetupState,
    pub maintenance: MaintenanceDiagnostics,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MaintenanceDiagnostics {
    pub market_storage_contract_version: u32,
    pub cached_market_records: u64,
    pub cached_market_fact_rows: u64,
    pub market_interpretation_memberships: u64,
    pub latest_indexing_phase: MarketIndexingPhase,
    pub latest_cache_records_reused: u32,
    pub latest_cache_rows_avoided: u64,
    pub latest_contention_retries: u32,
    pub latest_contention_wait_ms: u64,
    pub latest_resume_count: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryKind {
    Save,
    Game,
    Workshop,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WarehousePhase {
    Ready,
    Lagging,
    Rebuilding,
    Attention,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WarehouseWriteKind {
    CataloguePublication,
    ObservationProjection,
    MarketProjection,
    BroadcastProjection,
    OverlayProjection,
    BranchMembershipProjection,
    ObservationRebuild,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WarehouseWriteStage {
    Staging,
    Merging,
    Committing,
    Rebuilding,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct WarehouseWriteActivity {
    pub kind: WarehouseWriteKind,
    pub stage: WarehouseWriteStage,
    pub started_at_ms: i64,
    pub updated_at_ms: i64,
    pub rows_processed: u64,
    pub rows_total: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct WarehouseHealth {
    pub phase: WarehousePhase,
    pub schema_version: u32,
    pub pending_jobs: u32,
    pub failed_jobs: u32,
    pub lag_ms: Option<i64>,
    pub last_projected_at_ms: Option<i64>,
    pub observation_watermark: Option<String>,
    pub database_size_bytes: u64,
    pub active_write: Option<WarehouseWriteActivity>,
    pub consecutive_write_failures: u32,
    pub retry_after_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CatalogueGenerationSummary {
    pub generation_id: String,
    pub game_build_id: Option<String>,
    pub parser_version: String,
    pub created_at_ms: i64,
    pub source_count: u32,
    pub file_count: u32,
    pub entity_count: u32,
    pub property_count: u32,
    pub relation_count: u32,
    pub warning_count: u32,
    pub compatibility_profile_id: String,
    pub compatibility_profile_version: String,
    pub compatibility_profile_hash: String,
    pub mapping_classification: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CatalogueRefreshPhase {
    Idle,
    Discovering,
    Scanning,
    Publishing,
    Finalising,
    Complete,
    Failed,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CatalogueRefreshTrigger {
    Startup,
    Filesystem,
    Manual,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CatalogueRefreshProgress {
    pub phase: CatalogueRefreshPhase,
    pub trigger: CatalogueRefreshTrigger,
    pub progress_percent: Option<u8>,
    pub started_at_ms: Option<i64>,
    pub updated_at_ms: Option<i64>,
    pub current_source: Option<String>,
    pub current_file: Option<String>,
    pub current_file_index: Option<u32>,
    pub sources_discovered: u32,
    pub sources_total: u32,
    pub files_discovered: u32,
    pub files_processed: u32,
    pub files_reused: u32,
    pub files_parsed: u32,
    pub entities_prepared: u32,
    pub rows_written: u64,
    pub rows_total: u64,
    pub error_code: Option<String>,
}

impl Default for CatalogueRefreshProgress {
    fn default() -> Self {
        Self {
            phase: CatalogueRefreshPhase::Idle,
            trigger: CatalogueRefreshTrigger::Startup,
            progress_percent: None,
            started_at_ms: None,
            updated_at_ms: None,
            current_source: None,
            current_file: None,
            current_file_index: None,
            sources_discovered: 0,
            sources_total: 0,
            files_discovered: 0,
            files_processed: 0,
            files_reused: 0,
            files_parsed: 0,
            entities_prepared: 0,
            rows_written: 0,
            rows_total: 0,
            error_code: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CatalogueStatus {
    pub warehouse: WarehouseHealth,
    pub generation: Option<CatalogueGenerationSummary>,
    pub last_checked_at_ms: Option<i64>,
    pub last_refreshed_at_ms: Option<i64>,
    pub last_filesystem_event_ms: Option<i64>,
    pub error_code: Option<String>,
    pub active_overlay: Option<OverlayProfileSummary>,
    pub refresh: CatalogueRefreshProgress,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticEntry {
    pub occurred_at_ms: i64,
    pub level: String,
    pub code: String,
    pub operation: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DiagnosticLogView {
    pub language: &'static str,
    pub storage: &'static str,
    pub entries: Vec<DiagnosticEntry>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CatalogueSearchFilter {
    pub query: Option<String>,
    pub entity_kind: Option<String>,
    pub source_kind: Option<String>,
    pub package_query: Option<String>,
    pub coverage: Option<String>,
    pub available_year: Option<u32>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DefinitionSummary {
    pub entity_id: String,
    pub revision_hash: String,
    pub entity_kind: String,
    pub source_id: String,
    pub source_kind: String,
    pub package_name: String,
    pub display_name: String,
    pub coverage: String,
    pub property_count: u32,
    pub relation_count: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct CataloguePage {
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
    pub items: Vec<DefinitionSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DefinitionValue {
    pub value_kind: String,
    pub number: Option<f64>,
    pub text: Option<String>,
    pub unit: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DefinitionMappingProvenance {
    pub mapping_id: String,
    pub catalogue_scope_id: Option<String>,
    pub mapping_classification: String,
    pub scope_state: Option<CompatibilityCatalogueScopeState>,
    pub update_policy: Option<String>,
    pub acknowledged_content_hash: Option<String>,
    pub current_content_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DefinitionFact {
    pub field_id: String,
    pub occurrence: u32,
    pub original: Option<DefinitionValue>,
    pub override_value: Option<DefinitionValue>,
    pub effective: Option<DefinitionValue>,
    pub source_directive: String,
    pub source_line: u32,
    pub raw_arguments: String,
    pub evidence_kind: String,
    pub resolution: String,
    pub conflict_code: Option<String>,
    pub mapping: DefinitionMappingProvenance,
}

#[derive(Clone, Debug, Serialize)]
pub struct DefinitionRelation {
    pub relation_kind: String,
    pub occurrence: u32,
    pub target_id: String,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub phase_id: Option<String>,
    pub source_directive: String,
    pub source_line: u32,
    pub raw_arguments: String,
    pub resolution: String,
    pub mapping: DefinitionMappingProvenance,
}

#[derive(Clone, Debug, Serialize)]
pub struct DefinitionDossier {
    pub summary: DefinitionSummary,
    pub facts: Vec<DefinitionFact>,
    pub relations: Vec<DefinitionRelation>,
    pub unknown_directives: Vec<UnknownDirectiveSummary>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProductionRouteRequest {
    pub entity_id: String,
    pub output_resource_id: Option<String>,
    pub target_quantity: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionRouteFlow {
    pub id: String,
    pub direction: String,
    pub resource_id: String,
    pub display_name: String,
    pub source_quantity: Option<f64>,
    pub scaled_quantity: Option<f64>,
    pub unit: Option<String>,
    pub basis_role: String,
    pub basis_exclusion: Option<String>,
    pub resolution: String,
    pub source_directive: String,
    pub source_line: u32,
    pub mapping: DefinitionMappingProvenance,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionRouteModel {
    pub schema_version: u32,
    pub route_id: String,
    pub revision_hash: String,
    pub building_entity_id: Option<String>,
    pub display_name: String,
    pub package_name: String,
    pub coverage: String,
    pub status: String,
    pub relation_count: u32,
    pub primary_flow_count: u32,
    pub auxiliary_flow_count: u32,
    pub unit: Option<String>,
    pub selected_output_resource_id: Option<String>,
    pub target_quantity: Option<f64>,
    pub scale_factor: Option<f64>,
    pub mapping_classification: String,
    pub flows: Vec<ProductionRouteFlow>,
    pub snapshot: WarehouseSnapshot,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionRouteCoverage {
    pub schema_version: u32,
    pub route_count: u32,
    pub diagrammable_count: u32,
    pub routes_with_auxiliary: u32,
    pub unavailable_count: u32,
    pub relation_count: u32,
    pub auxiliary_relation_count: u32,
    pub unresolved_basis_relation_count: u32,
    pub unquantified_relation_count: u32,
    pub snapshot: WarehouseSnapshot,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProductionPathwaySelection {
    pub resource_id: String,
    pub recipe_entity_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProductionPathwayRequest {
    pub root_recipe_entity_id: String,
    pub output_resource_id: String,
    pub target_quantity: f64,
    pub max_depth: u32,
    #[serde(default)]
    pub selections: Vec<ProductionPathwaySelection>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionPathwayNode {
    pub id: String,
    pub kind: String,
    pub display_name: String,
    pub resource_id: Option<String>,
    pub recipe_entity_id: Option<String>,
    pub package_name: Option<String>,
    pub depth: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionPathwayLink {
    pub id: String,
    pub source: String,
    pub target: String,
    pub resource_id: String,
    pub quantity: f64,
    pub unit: String,
    pub source_directive: String,
    pub source_line: u32,
    pub mapping: DefinitionMappingProvenance,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionPathwayCandidate {
    pub recipe_entity_id: String,
    pub display_name: String,
    pub package_name: String,
    pub output_quantity: f64,
    pub unit: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionPathwayChoice {
    pub resource_node_id: String,
    pub resource_id: String,
    pub display_name: String,
    pub required_quantity: f64,
    pub unit: String,
    pub selected_recipe_entity_id: Option<String>,
    pub candidates: Vec<ProductionPathwayCandidate>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionPathwayRequirement {
    pub resource_id: String,
    pub display_name: String,
    pub quantity: f64,
    pub unit: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionPathwayAuxiliaryRequirement {
    pub stage_id: String,
    pub recipe_entity_id: String,
    pub resource_id: String,
    pub display_name: String,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub reason: String,
    pub source_directive: String,
    pub source_line: u32,
    pub mapping: DefinitionMappingProvenance,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionPathwayDiagnostic {
    pub code: String,
    pub resource_id: Option<String>,
    pub recipe_entity_id: Option<String>,
    pub depth: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductionPathwayModel {
    pub schema_version: u32,
    pub status: String,
    pub root_recipe_entity_id: String,
    pub output_resource_id: String,
    pub target_quantity: f64,
    pub unit: String,
    pub max_depth: u32,
    pub mapping_classification: String,
    pub nodes: Vec<ProductionPathwayNode>,
    pub links: Vec<ProductionPathwayLink>,
    pub choices: Vec<ProductionPathwayChoice>,
    pub terminal_requirements: Vec<ProductionPathwayRequirement>,
    pub auxiliary_requirements: Vec<ProductionPathwayAuxiliaryRequirement>,
    pub diagnostics: Vec<ProductionPathwayDiagnostic>,
    pub snapshot: WarehouseSnapshot,
}

#[derive(Clone, Debug, Serialize)]
pub struct UnknownDirectiveSummary {
    pub directive: String,
    pub occurrence_count: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct OverlayProfileSummary {
    pub profile_id: String,
    pub display_name: String,
    pub active_revision: Option<u32>,
    pub latest_revision: u32,
    pub revision_count: u32,
    pub semantic_version: String,
    pub content_hash: String,
    pub conflict_count: u32,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct OverlayInspection {
    pub valid: bool,
    pub code: Option<String>,
    pub profile: Option<OverlayProfileSummary>,
    pub operation_count: u32,
    pub supplement_count: u32,
    pub document: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WarehouseSnapshot {
    pub catalogue_generation_id: String,
    pub compatibility_profile_id: String,
    pub compatibility_profile_version: String,
    pub compatibility_profile_hash: String,
    pub mapping_classification: String,
    pub overlay_profile_id: Option<String>,
    pub overlay_revision: Option<u32>,
    pub observation_watermark: Option<String>,
    pub warehouse_schema_version: u32,
    pub projector_version: String,
}
