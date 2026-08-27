use serde::{Deserialize, Serialize};

pub const PARSER_VERSION: &str = "stats-ini.receiver-history.v1";
pub const FORMAT_PROFILE: &str = "wrsr-stats-implicit-v1";
pub const REPUBLIC_SCOPE: &str = "republic";

pub const RECEIVER_METRICS: [MetricDefinition; 4] = [
    MetricDefinition {
        id: "core.citizens.electronics.none",
        source_field: "$Citizens_EletronicNone",
    },
    MetricDefinition {
        id: "core.citizens.electronics.radio",
        source_field: "$Citizens_EletrinicRadio",
    },
    MetricDefinition {
        id: "core.citizens.electronics.television",
        source_field: "$Citizens_EletronicTV",
    },
    MetricDefinition {
        id: "core.citizens.electronics.computer",
        source_field: "$Citizens_EletronicComputer",
    },
];

pub const SNAPSHOT_FACTS: [SnapshotFactDefinition; 18] = [
    SnapshotFactDefinition::republic("core.citizens.electronics.none", "$Citizens_EletronicNone"),
    SnapshotFactDefinition::republic(
        "core.citizens.electronics.radio",
        "$Citizens_EletrinicRadio",
    ),
    SnapshotFactDefinition::republic(
        "core.citizens.electronics.television",
        "$Citizens_EletronicTV",
    ),
    SnapshotFactDefinition::republic(
        "core.citizens.electronics.computer",
        "$Citizens_EletronicComputer",
    ),
    SnapshotFactDefinition::shared("source.stats.citizens.born", "$Citizens_Born"),
    SnapshotFactDefinition::shared("source.stats.citizens.dead", "$Citizens_Dead"),
    SnapshotFactDefinition::shared("source.stats.citizens.escaped", "$Citizens_Escaped"),
    SnapshotFactDefinition::shared(
        "source.stats.citizens.immigrant_soviet",
        "$Citizens_ImigrantSoviet",
    ),
    SnapshotFactDefinition::shared(
        "source.stats.citizens.immigrant_africa",
        "$Citizens_ImigrantAfrica",
    ),
    SnapshotFactDefinition::republic(
        "source.stats.citizens.small_children",
        "$Citizens_SmallChilds",
    ),
    SnapshotFactDefinition::republic(
        "source.stats.citizens.medium_children",
        "$Citizens_MediumChilds",
    ),
    SnapshotFactDefinition::republic(
        "source.stats.citizens.adults_parent",
        "$Citizens_AdultsParent",
    ),
    SnapshotFactDefinition::republic("source.stats.citizens.adults", "$Citizens_Adults"),
    SnapshotFactDefinition::republic("source.stats.citizens.unemployed", "$Citizens_Unemployed"),
    SnapshotFactDefinition::republic(
        "source.stats.citizens.no_education",
        "$Citizens_NoEducation",
    ),
    SnapshotFactDefinition::republic(
        "source.stats.citizens.basic_education",
        "$Citizens_BasicEducationNum",
    ),
    SnapshotFactDefinition::republic(
        "source.stats.citizens.higher_education",
        "$Citizens_HighEducationNum",
    ),
    SnapshotFactDefinition::republic("source.stats.citizens.car_owners", "$Citizens_CarOwners"),
];

#[derive(Clone, Copy, Debug)]
pub struct MetricDefinition {
    pub id: &'static str,
    pub source_field: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct SnapshotFactDefinition {
    pub id: &'static str,
    pub source_field: &'static str,
    pub republic: bool,
    pub city: bool,
}

impl SnapshotFactDefinition {
    const fn republic(id: &'static str, source_field: &'static str) -> Self {
        Self {
            id,
            source_field,
            republic: true,
            city: false,
        }
    }

    const fn shared(id: &'static str, source_field: &'static str) -> Self {
        Self {
            id,
            source_field,
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
    pub fact_id: &'static str,
    pub source_field: &'static str,
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

#[derive(Clone, Debug)]
pub struct SaveInspection {
    pub payload_hash: String,
    pub source_file_name: String,
    pub source_file_size: u64,
    pub source_modified_ms: i64,
    pub source_directory_identity: String,
    pub records: Vec<ReceiverRecord>,
    pub coverage: CoverageReport,
    pub snapshots: Vec<SaveSnapshot>,
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
    pub source_file_name: String,
    pub source_file_size: u64,
    pub source_modified_ms: i64,
    pub imported_at_ms: i64,
    pub parser_version: String,
    pub format_profile: String,
    pub branch_id: String,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct ArchiveObservation {
    pub payload_hash: String,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct ArchiveOverview {
    pub selected_branch_id: String,
    pub file_observation_count: u32,
    pub distinct_state_count: u32,
    pub unresolved_state_count: u32,
    pub branches: Vec<TimelineBranch>,
    pub observations: Vec<ArchiveObservation>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BranchSelectionResult {
    pub archive: ArchiveOverview,
    pub dataset: Option<ReceiverDataset>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ComparisonObservation {
    pub payload_hash: String,
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportOutcome {
    Imported,
    Duplicate,
}

#[derive(Clone, Debug, Serialize)]
pub struct ObservationImportResult {
    pub outcome: ImportOutcome,
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
    pub save_candidates: u32,
    pub observed_saves: u32,
    pub distinct_states: u32,
    pub game_vocabularies: Vec<GameVocabularySource>,
    pub automatic_observer: AutomaticObserverStatus,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryKind {
    Save,
    Game,
}
