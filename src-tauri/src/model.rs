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

#[derive(Clone, Copy, Debug)]
pub struct MetricDefinition {
    pub id: &'static str,
    pub source_field: &'static str,
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
}

#[derive(Clone, Debug)]
pub struct SaveInspection {
    pub payload_hash: String,
    pub source_file_name: String,
    pub source_file_size: u64,
    pub source_modified_ms: i64,
    pub records: Vec<ReceiverRecord>,
    pub coverage: CoverageReport,
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
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryKind {
    Save,
    Game,
}
