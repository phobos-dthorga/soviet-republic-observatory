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
    #[error("That timeline branch label is invalid.")]
    InvalidBranchLabel,
    #[error("The analytical warehouse is unavailable.")]
    WarehouseUnavailable,
    #[error("The analytical warehouse write exceeds its bounded workload limit.")]
    WarehouseWriteLimit,
    #[error("No installed-game definition catalogue is available.")]
    CatalogueUnavailable,
    #[error("The definition catalogue request is invalid.")]
    InvalidCatalogueRequest,
    #[error(
        "An exact mod compatibility scope no longer matches its acknowledged definition content."
    )]
    CatalogueCompatibilityConflict,
    #[error("The planning overlay is invalid: {0}")]
    InvalidPlanningOverlay(&'static str),
    #[error("The selected planning overlay profile does not exist.")]
    UnknownPlanningOverlay,
    #[error("The republic plan is invalid: {0}")]
    InvalidRepublicPlan(&'static str),
    #[error("The selected republic plan does not exist.")]
    UnknownRepublicPlan,
    #[error("That republic plan belongs to a different timeline branch.")]
    RepublicPlanBranchMismatch,
    #[error("The market basket or scenario is invalid: {0}")]
    InvalidMarketDefinition(&'static str),
    #[error("The selected market basket or scenario does not exist.")]
    UnknownMarketDefinition,
    #[error("The active market basket or scenario cannot be removed.")]
    ActiveMarketDefinitionRemove,
    #[error("The Analysis Pack is invalid: {0}")]
    InvalidAnalysisPack(&'static str),
    #[error("The selected Analysis Pack does not exist.")]
    UnknownAnalysisPack,
    #[error("The compatibility profile is invalid: {0}")]
    InvalidCompatibilityProfile(&'static str),
    #[error("A fixed binary compatibility layout did not match the save: {0}")]
    BinaryCompatibilityMismatch(&'static str),
    #[error("Another critical task of this type is already running.")]
    CriticalTaskBusy,
    #[error("That attention cue identity or revision is invalid.")]
    InvalidAttentionCue,
    #[error("The experimental research setup request is invalid.")]
    InvalidResearchSetup,
    #[error("The selected TesmioLoader checkout does not match the reviewed interface.")]
    InvalidResearchCheckout,
    #[error("Review and accept the current native-research notice before building.")]
    ResearchNoticeRequired,
    #[error("The reviewed probe source is unavailable in this application checkout.")]
    ResearchSourceUnavailable,
    #[error("The required Windows C++ build toolchain is unavailable.")]
    ResearchToolchainUnavailable,
    #[error("The bounded research-probe build failed.")]
    ResearchBuildFailed,
    #[error("That language pack is larger than the 256 KiB safety limit.")]
    LanguageManifestTooLarge,
    #[error("That file does not contain valid JSON.")]
    InvalidLanguageJson,
    #[error("That file is not a valid Republic Observatory language-pack manifest.")]
    InvalidLanguageManifest,
    #[error("That language pack targets an unsupported schema or source-catalogue version.")]
    UnsupportedLanguageVersion,
    #[error("That language-pack identifier is invalid or reserved by Republic Observatory.")]
    InvalidLanguageIdentifier,
    #[error("That language pack contains invalid locale, name, author, or direction metadata.")]
    InvalidLanguageMetadata,
    #[error("That language pack contains an unknown, malformed, or incompatible message.")]
    InvalidLanguageMessage,
    #[error("Community language packs cannot replace protected Observatory messages.")]
    ProtectedLanguageMessage,
    #[error("Choose a language pack that is currently installed.")]
    UnknownLanguagePack,
    #[error("The built-in English catalogue cannot be removed.")]
    BuiltInLanguagePackRemove,
    #[error("That theme is larger than the 32 KiB safety limit.")]
    ThemeManifestTooLarge,
    #[error("That file is not a valid Republic Observatory theme manifest.")]
    InvalidThemeManifest,
    #[error("That theme targets an unsupported schema version.")]
    UnsupportedThemeVersion,
    #[error("That theme identifier is invalid or reserved by Republic Observatory.")]
    InvalidThemeIdentifier,
    #[error("That theme contains invalid version, name, author, or description metadata.")]
    InvalidThemeMetadata,
    #[error("Theme colours must use the #RRGGBB format and provide a bounded chart palette.")]
    InvalidThemeColour,
    #[error("That theme does not provide enough contrast for the Observatory interface.")]
    ThemeInsufficientContrast,
    #[error(
        "That exact theme revision is already installed or duplicates another theme's appearance."
    )]
    DuplicateTheme,
    #[error("That theme ID and version already identify different content.")]
    ThemeRevisionConflict,
    #[error("Choose a theme revision that is currently available.")]
    UnknownTheme,
    #[error("The active theme revision cannot be removed.")]
    ActiveThemeRemove,
    #[error("Built-in themes cannot be removed.")]
    BuiltInThemeRemove,
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
            Self::InvalidBranchLabel => "invalid_branch_label",
            Self::WarehouseUnavailable => "warehouse_unavailable",
            Self::WarehouseWriteLimit => "warehouse_write_limit",
            Self::CatalogueUnavailable => "catalogue_unavailable",
            Self::InvalidCatalogueRequest => "invalid_catalogue_request",
            Self::CatalogueCompatibilityConflict => "catalogue_compatibility_conflict",
            Self::InvalidPlanningOverlay(_) => "invalid_planning_overlay",
            Self::UnknownPlanningOverlay => "unknown_planning_overlay",
            Self::InvalidRepublicPlan(reason) => match *reason {
                "invalid_name" => "invalid_republic_plan_name",
                "invalid_end_date" => "invalid_republic_plan_end_date",
                "invalid_target_count" => "invalid_republic_plan_target_count",
                "unknown_metric" => "invalid_republic_plan_unknown_metric",
                "duplicate_metric" => "invalid_republic_plan_duplicate_metric",
                "invalid_guardrail" => "invalid_republic_plan_guardrail",
                "direction_mismatch" => "invalid_republic_plan_direction_mismatch",
                "invalid_plan_window" => "invalid_republic_plan_window",
                "metric_unavailable" => "invalid_republic_plan_metric_unavailable",
                _ => "invalid_republic_plan",
            },
            Self::UnknownRepublicPlan => "unknown_republic_plan",
            Self::RepublicPlanBranchMismatch => "republic_plan_branch_mismatch",
            Self::InvalidMarketDefinition(_) => "invalid_market_definition",
            Self::UnknownMarketDefinition => "unknown_market_definition",
            Self::ActiveMarketDefinitionRemove => "active_market_definition_remove",
            Self::InvalidAnalysisPack(_) => "invalid_analysis_pack",
            Self::UnknownAnalysisPack => "unknown_analysis_pack",
            Self::InvalidCompatibilityProfile(_) => "invalid_compatibility_profile",
            Self::BinaryCompatibilityMismatch(_) => "binary_compatibility_mismatch",
            Self::CriticalTaskBusy => "critical_task_busy",
            Self::InvalidAttentionCue => "invalid_attention_cue",
            Self::InvalidResearchSetup => "invalid_research_setup",
            Self::InvalidResearchCheckout => "invalid_research_checkout",
            Self::ResearchNoticeRequired => "research_notice_required",
            Self::ResearchSourceUnavailable => "research_source_unavailable",
            Self::ResearchToolchainUnavailable => "research_toolchain_unavailable",
            Self::ResearchBuildFailed => "research_build_failed",
            Self::LanguageManifestTooLarge => "manifest_too_large",
            Self::InvalidLanguageJson => "invalid_json",
            Self::InvalidLanguageManifest => "invalid_manifest",
            Self::UnsupportedLanguageVersion => "unsupported_version",
            Self::InvalidLanguageIdentifier => "invalid_identifier",
            Self::InvalidLanguageMetadata => "invalid_metadata",
            Self::InvalidLanguageMessage => "invalid_message",
            Self::ProtectedLanguageMessage => "protected_message",
            Self::UnknownLanguagePack => "unknown_pack",
            Self::BuiltInLanguagePackRemove => "built_in_remove",
            Self::ThemeManifestTooLarge => "theme_manifest_too_large",
            Self::InvalidThemeManifest => "invalid_theme_manifest",
            Self::UnsupportedThemeVersion => "unsupported_theme_version",
            Self::InvalidThemeIdentifier => "invalid_theme_identifier",
            Self::InvalidThemeMetadata => "invalid_theme_metadata",
            Self::InvalidThemeColour => "invalid_theme_colour",
            Self::ThemeInsufficientContrast => "theme_insufficient_contrast",
            Self::DuplicateTheme => "duplicate_theme",
            Self::ThemeRevisionConflict => "theme_revision_conflict",
            Self::UnknownTheme => "unknown_theme",
            Self::ActiveThemeRemove => "active_theme_remove",
            Self::BuiltInThemeRemove => "built_in_theme_remove",
        }
    }

    pub fn analysis_pack_reason(&self) -> Option<&'static str> {
        match self {
            Self::InvalidAnalysisPack(reason) => Some(reason),
            Self::UnknownAnalysisPack => Some("unknown_analysis_pack"),
            _ => None,
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
    fn from(_error: duckdb::Error) -> Self {
        #[cfg(debug_assertions)]
        eprintln!("DuckDB diagnostic: {_error}");
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
