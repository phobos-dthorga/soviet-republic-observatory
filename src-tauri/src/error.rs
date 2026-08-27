use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObservatoryError {
    #[error("The selected folder is not available.")]
    InvalidDirectory,
    #[error("The selected game folder does not contain media_soviet.")]
    InvalidGameDirectory,
    #[error("No save folder has been configured.")]
    SaveDirectoryNotConfigured,
    #[error("No ZIP save was found in the configured folder.")]
    NoSaveCandidate,
    #[error("The save candidate is not a regular ZIP file.")]
    InvalidSaveCandidate,
    #[error("The save changed while it was being observed. Try again after saving finishes.")]
    SaveChangedDuringRead,
    #[error("The save archive could not be read.")]
    InvalidArchive,
    #[error("The save archive contains no usable stats.ini payload.")]
    MissingStatsPayload,
    #[error("The save archive contains more than one stats.ini payload.")]
    DuplicateStatsPayload,
    #[error("The stats.ini payload is larger than the observer safety limit.")]
    StatsPayloadTooLarge,
    #[error("The stats.ini payload is not valid UTF-8 text.")]
    InvalidStatsEncoding,
    #[error("A stats.ini line exceeds the observer safety limit.")]
    StatsLineTooLong,
    #[error("This explicit stats format is not supported.")]
    UnsupportedStatsFormat,
    #[error("The receiver history is malformed: {0}")]
    MalformedReceiverHistory(&'static str),
    #[error("A current or city snapshot is malformed: {0}")]
    MalformedSnapshot(&'static str),
    #[error("The save contains no receiver history that this parser can use.")]
    ReceiverHistoryUnavailable,
    #[error("Local observation storage is unavailable.")]
    StorageUnavailable,
    #[error("The selected timeline branch does not exist.")]
    UnknownBranch,
    #[error("The selected observations cannot be compared on one resolved branch.")]
    IncompatibleComparison,
    #[error("Choose two different observations to compare.")]
    SameObservationComparison,
    #[error("One of the selected observations no longer exists.")]
    UnknownObservation,
    #[error("The analytical warehouse is unavailable.")]
    WarehouseUnavailable,
    #[error("No installed-game definition catalogue is available.")]
    CatalogueUnavailable,
    #[error("The definition catalogue request is invalid.")]
    InvalidCatalogueRequest,
    #[error("The planning overlay is invalid: {0}")]
    InvalidPlanningOverlay(&'static str),
    #[error("The selected planning overlay profile does not exist.")]
    UnknownPlanningOverlay,
}

impl ObservatoryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidDirectory => "invalid_directory",
            Self::InvalidGameDirectory => "invalid_game_directory",
            Self::SaveDirectoryNotConfigured => "save_directory_not_configured",
            Self::NoSaveCandidate => "no_save_candidate",
            Self::InvalidSaveCandidate => "invalid_save_candidate",
            Self::SaveChangedDuringRead => "save_changed_during_read",
            Self::InvalidArchive => "invalid_archive",
            Self::MissingStatsPayload => "missing_stats_payload",
            Self::DuplicateStatsPayload => "duplicate_stats_payload",
            Self::StatsPayloadTooLarge => "stats_payload_too_large",
            Self::InvalidStatsEncoding => "invalid_stats_encoding",
            Self::StatsLineTooLong => "stats_line_too_long",
            Self::UnsupportedStatsFormat => "unsupported_stats_format",
            Self::MalformedReceiverHistory(_) => "malformed_receiver_history",
            Self::MalformedSnapshot(_) => "malformed_snapshot",
            Self::ReceiverHistoryUnavailable => "receiver_history_unavailable",
            Self::StorageUnavailable => "storage_unavailable",
            Self::UnknownBranch => "unknown_branch",
            Self::IncompatibleComparison => "incompatible_comparison",
            Self::SameObservationComparison => "same_observation_comparison",
            Self::UnknownObservation => "unknown_observation",
            Self::WarehouseUnavailable => "warehouse_unavailable",
            Self::CatalogueUnavailable => "catalogue_unavailable",
            Self::InvalidCatalogueRequest => "invalid_catalogue_request",
            Self::InvalidPlanningOverlay(_) => "invalid_planning_overlay",
            Self::UnknownPlanningOverlay => "unknown_planning_overlay",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub diagnostic: String,
}

impl From<ObservatoryError> for CommandError {
    fn from(error: ObservatoryError) -> Self {
        Self {
            code: error.code().to_owned(),
            diagnostic: error.to_string(),
        }
    }
}

impl From<rusqlite::Error> for ObservatoryError {
    fn from(_error: rusqlite::Error) -> Self {
        #[cfg(debug_assertions)]
        eprintln!("SQLite diagnostic: {_error}");
        Self::StorageUnavailable
    }
}

impl From<duckdb::Error> for ObservatoryError {
    fn from(_: duckdb::Error) -> Self {
        Self::WarehouseUnavailable
    }
}

impl From<std::io::Error> for ObservatoryError {
    fn from(_: std::io::Error) -> Self {
        Self::InvalidArchive
    }
}

impl From<zip::result::ZipError> for ObservatoryError {
    fn from(_: zip::result::ZipError) -> Self {
        Self::InvalidArchive
    }
}
