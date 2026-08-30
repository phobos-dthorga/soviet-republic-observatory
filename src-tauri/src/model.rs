use serde::{Deserialize, Serialize};

pub const PARSER_VERSION: &str = "stats-ini.receiver-history.v1";
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
