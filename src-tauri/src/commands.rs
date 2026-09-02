use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

use crate::analysis_pack::{AnalysisPackContribution, AnalysisPackInspection, AnalysisPackSummary};
use crate::application::ObservatoryApplication;
use crate::error::{CommandError, ObservatoryError};
use crate::language_pack::{LanguagePackInspection, LanguageStatus, LegacyLanguageHandover};
use crate::model::{
    AnalysisContextResult, ApplicationPreferencesDraft, ApplicationSettingsView, ArchiveComparison,
    ArchiveOverview, BroadcastIndexingProgress, BroadcastOutcomeModel, BroadcastOutcomeRequest,
    BroadcastWorkspaceModel, CarbonFactorImportPreview, CarbonFactorSetDraft, CataloguePage,
    CatalogueSearchFilter, CatalogueStatus, CompatibilityStatus, CompatibilityUpdate,
    DefinitionDossier, DiagnosticLogView, DirectoryKind, EnvironmentCaptureResult,
    EnvironmentHistoryModel, EnvironmentIndexingProgress, EnvironmentSnapshot,
    EnvironmentWorkspaceModel, MarketBasketDraft, MarketIndexingProgress, MarketPriceSeries,
    MarketScenarioDraft, MarketWorkspace, ObservationImportResult, OverlayInspection,
    OverlayProfileSummary, PopulationDataset, ProductionPathwayModel, ProductionPathwayRequest,
    ProductionRouteCoverage, ProductionRouteModel, ProductionRouteRequest, PublishedMetricContext,
    ReceiverDataset, RecorderHealth, ReinterpretationProgress, RepublicBrief, RepublicPlanDraft,
    RepublicPlanWorkspace, ResourceCatalogueRequest, ResourceCatalogueView, ResourceDetails,
    ResourceRegistryAssurance, ResourceRegistrySnapshotSummary, ResourceRegistryStatus, SetupState,
    TesmioProbeStatus, WarehouseSnapshot,
};
use crate::research_setup::{
    RESEARCH_NOTICE_REVISION, ResearchBuildProgress, ResearchSessionProgress, ResearchSetupService,
    ResearchSetupStatus, ResearchSourceDownloadProgress, ResearchSourceOrigin,
};
use crate::setup_discovery::{picker_start_directory, suggest_directory};
use crate::theme::{ThemeInspection, ThemeStatus};
use crate::ui_review::UiReviewContext;

#[derive(Debug)]
pub struct AppState {
    pub application: Arc<ObservatoryApplication>,
    pub research_setup: Arc<ResearchSetupService>,
    pub ui_review: UiReviewContext,
    pub data_directory: PathBuf,
}

pub const MARKET_INDEXING_PROGRESS_EVENT: &str = "market-indexing-progress";
pub const BROADCAST_INDEXING_PROGRESS_EVENT: &str = "broadcast-indexing-progress";
pub const ENVIRONMENT_INDEXING_PROGRESS_EVENT: &str = "environment-indexing-progress";

#[tauri::command]
pub fn erase_application_databases(
    confirmation: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    if state.ui_review.enabled {
        return Err(ObservatoryError::DatabaseResetFailed.into());
    }
    crate::database_reset::schedule_database_reset(&state.data_directory, &confirmation)?;
    crate::diagnostics::record(
        "warning",
        "database_reset_scheduled",
        "erase_application_databases",
        "A confirmed restart-time reset was scheduled for Observatory-owned databases only.",
    );
    app.restart()
}

async fn run_market_workspace_command(
    application: Arc<ObservatoryApplication>,
    operation: impl FnOnce(&ObservatoryApplication) -> Result<MarketWorkspace, ObservatoryError>
    + Send
    + 'static,
) -> Result<MarketWorkspace, CommandError> {
    tauri::async_runtime::spawn_blocking(move || operation(&application))
        .await
        .map_err(|_| CommandError {
            code: "market_workspace_worker_unavailable".to_owned(),
            diagnostic: "The Markets workspace worker stopped unexpectedly.".to_owned(),
        })?
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_ui_review_context(state: State<'_, AppState>) -> UiReviewContext {
    state.ui_review.clone()
}

#[derive(Debug, Serialize)]
pub struct AttentionCueStatus {
    pub cue_id: String,
    pub content_revision: u32,
    pub dismissed: bool,
}

#[tauri::command]
pub fn attention_cue_status(
    cue_id: String,
    content_revision: u32,
    state: State<'_, AppState>,
) -> Result<AttentionCueStatus, CommandError> {
    let dismissed = state
        .application
        .attention_cue_dismissed(&cue_id, content_revision)?;
    Ok(AttentionCueStatus {
        cue_id,
        content_revision,
        dismissed,
    })
}

#[tauri::command]
pub fn dismiss_attention_cue(
    cue_id: String,
    content_revision: u32,
    state: State<'_, AppState>,
) -> Result<AttentionCueStatus, CommandError> {
    state
        .application
        .dismiss_attention_cue(&cue_id, content_revision)?;
    Ok(AttentionCueStatus {
        cue_id,
        content_revision,
        dismissed: true,
    })
}

#[tauri::command]
pub fn replay_attention_cue(
    cue_id: String,
    content_revision: u32,
    state: State<'_, AppState>,
) -> Result<AttentionCueStatus, CommandError> {
    state
        .application
        .replay_attention_cue(&cue_id, content_revision)?;
    Ok(AttentionCueStatus {
        cue_id,
        content_revision,
        dismissed: false,
    })
}

#[tauri::command]
pub fn replay_all_attention_cues(state: State<'_, AppState>) -> Result<u32, CommandError> {
    state
        .application
        .replay_all_attention_cues()
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_application_settings(
    state: State<'_, AppState>,
) -> Result<ApplicationSettingsView, CommandError> {
    state.application.application_settings().map_err(Into::into)
}

#[tauri::command]
pub fn update_application_preferences(
    preferences: ApplicationPreferencesDraft,
    state: State<'_, AppState>,
) -> Result<ApplicationSettingsView, CommandError> {
    state
        .application
        .update_application_preferences(&preferences)
        .map_err(Into::into)
}

#[tauri::command]
pub fn reset_application_preferences(
    state: State<'_, AppState>,
) -> Result<ApplicationSettingsView, CommandError> {
    state
        .application
        .reset_application_preferences()
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_research_setup(state: State<'_, AppState>) -> Result<ResearchSetupStatus, CommandError> {
    let stored = state.application.research_setup()?;
    let media = state.application.catalogue_configuration()?;
    Ok(state.research_setup.status(&stored, media.as_deref()))
}

#[tauri::command]
pub fn get_research_report_status(
    state: State<'_, AppState>,
) -> Result<TesmioProbeStatus, CommandError> {
    let media = state.application.catalogue_configuration()?;
    Ok(crate::tesmio_probe::inspect(media.as_deref()))
}

#[tauri::command]
pub fn set_research_notice_accepted(
    accepted: bool,
    state: State<'_, AppState>,
) -> Result<ResearchSetupStatus, CommandError> {
    state
        .application
        .set_research_notice_revision(if accepted {
            RESEARCH_NOTICE_REVISION
        } else {
            0
        })?;
    let stored = state.application.research_setup()?;
    let media = state.application.catalogue_configuration()?;
    Ok(state.research_setup.status(&stored, media.as_deref()))
}

#[tauri::command]
pub fn configure_research_tesmio_checkout(
    path: String,
    state: State<'_, AppState>,
) -> Result<ResearchSetupStatus, CommandError> {
    if path.is_empty() || path.len() > 4_096 {
        return Err(crate::error::ObservatoryError::InvalidResearchSetup.into());
    }
    let canonical = match state.research_setup.validate_checkout(&PathBuf::from(path)) {
        Ok(canonical) => canonical,
        Err(error) => {
            crate::diagnostics::record(
                "warning",
                "research_checkout_rejected",
                "configure_research_tesmio_checkout",
                "The selected checkout did not match the bounded reviewed-header contract.",
            );
            return Err(error.into());
        }
    };
    state
        .application
        .set_research_tesmio_checkout(&canonical, ResearchSourceOrigin::ManualCheckout.as_str())?;
    crate::diagnostics::record(
        "info",
        "research_checkout_reviewed",
        "configure_research_tesmio_checkout",
        "The selected checkout matched the bounded reviewed-header contract.",
    );
    let stored = state.application.research_setup()?;
    let media = state.application.catalogue_configuration()?;
    Ok(state.research_setup.status(&stored, media.as_deref()))
}

#[tauri::command]
pub fn get_research_build_progress(state: State<'_, AppState>) -> ResearchBuildProgress {
    state.research_setup.progress()
}

#[tauri::command]
pub fn get_research_source_download_progress(
    state: State<'_, AppState>,
) -> ResearchSourceDownloadProgress {
    state.research_setup.download_progress()
}

#[tauri::command]
pub async fn download_reviewed_tesmio_source(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ResearchSetupStatus, CommandError> {
    let application = Arc::clone(&state.application);
    let service = Arc::clone(&state.research_setup);
    tauri::async_runtime::spawn_blocking(move || {
        let stored = application.research_setup()?;
        let source = service.download_source(&app, &stored)?;
        application.set_research_tesmio_checkout(
            &source.checkout_path,
            ResearchSourceOrigin::ObservatoryDownloaded.as_str(),
        )?;
        let stored = application.research_setup()?;
        let media = application.catalogue_configuration()?;
        Ok::<ResearchSetupStatus, ObservatoryError>(service.status(&stored, media.as_deref()))
    })
    .await
    .map_err(|_| CommandError::from(ObservatoryError::ResearchSourceDownloadFailed))?
    .map_err(Into::into)
}

#[tauri::command]
pub async fn build_research_probe(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ResearchSetupStatus, CommandError> {
    let application = Arc::clone(&state.application);
    let service = Arc::clone(&state.research_setup);
    tauri::async_runtime::spawn_blocking(move || {
        let stored = application.research_setup()?;
        let artifact = service.build_probe(&app, &stored)?;
        application.record_research_probe_build(&artifact.hash)?;
        let stored = application.research_setup()?;
        let media = application.catalogue_configuration()?;
        Ok::<ResearchSetupStatus, crate::error::ObservatoryError>(
            service.status(&stored, media.as_deref()),
        )
    })
    .await
    .map_err(|_| CommandError::from(crate::error::ObservatoryError::ResearchBuildFailed))?
    .map_err(Into::into)
}

#[tauri::command]
pub fn get_research_session_progress(state: State<'_, AppState>) -> ResearchSessionProgress {
    state.research_setup.session_progress()
}

#[tauri::command]
pub async fn prepare_observation_only_session(
    game_directory_write_confirmed: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ResearchSetupStatus, CommandError> {
    let application = Arc::clone(&state.application);
    let service = Arc::clone(&state.research_setup);
    tauri::async_runtime::spawn_blocking(move || {
        let stored = application.research_setup()?;
        let media = application.catalogue_configuration()?;
        service.prepare_observation_session(
            &app,
            &stored,
            media.as_deref(),
            game_directory_write_confirmed,
        )?;
        application.enable_resource_registry_ingestion(
            ResourceRegistryAssurance::VerifiedObservationOnly,
            true,
        )?;
        let stored = application.research_setup()?;
        Ok::<ResearchSetupStatus, ObservatoryError>(service.status(&stored, media.as_deref()))
    })
    .await
    .map_err(|_| CommandError::from(ObservatoryError::ResearchSessionPreparationFailed))?
    .map_err(Into::into)
}

#[tauri::command]
pub async fn launch_observation_only_session(
    running_game_memory_confirmed: bool,
    state: State<'_, AppState>,
) -> Result<ResearchSetupStatus, CommandError> {
    let application = Arc::clone(&state.application);
    let service = Arc::clone(&state.research_setup);
    tauri::async_runtime::spawn_blocking(move || {
        let media = application.catalogue_configuration()?;
        let stored = application.research_setup()?;
        service.launch_observation_session(
            &stored,
            media.as_deref(),
            running_game_memory_confirmed,
        )?;
        Ok::<ResearchSetupStatus, ObservatoryError>(service.status(&stored, media.as_deref()))
    })
    .await
    .map_err(|_| CommandError::from(ObservatoryError::ResearchSessionLaunchFailed))?
    .map_err(Into::into)
}

#[tauri::command]
pub fn get_setup_state(state: State<'_, AppState>) -> Result<SetupState, CommandError> {
    state.application.setup_state().map_err(Into::into)
}

#[tauri::command]
pub fn get_latest_receiver_dataset(
    state: State<'_, AppState>,
) -> Result<Option<ReceiverDataset>, CommandError> {
    state
        .application
        .latest_receiver_dataset()
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_archive_overview(state: State<'_, AppState>) -> Result<ArchiveOverview, CommandError> {
    state.application.archive_overview().map_err(Into::into)
}

#[tauri::command]
pub fn get_population_dataset(
    state: State<'_, AppState>,
) -> Result<PopulationDataset, CommandError> {
    state.application.population_dataset().map_err(Into::into)
}

#[tauri::command]
pub fn get_republic_brief(state: State<'_, AppState>) -> Result<RepublicBrief, CommandError> {
    state.application.republic_brief().map_err(Into::into)
}

#[tauri::command]
pub async fn get_broadcast_workspace(
    state: State<'_, AppState>,
) -> Result<BroadcastWorkspaceModel, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || application.broadcast_workspace())
        .await
        .map_err(|_| CommandError {
            code: "broadcast_workspace_worker_unavailable".to_owned(),
            diagnostic: "The Broadcast workspace worker stopped unexpectedly.".to_owned(),
        })?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_broadcast_outcome(
    request: BroadcastOutcomeRequest,
    state: State<'_, AppState>,
) -> Result<BroadcastOutcomeModel, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || application.broadcast_outcome(&request))
        .await
        .map_err(|_| CommandError {
            code: "broadcast_outcome_worker_unavailable".to_owned(),
            diagnostic: "The Broadcast comparison worker stopped unexpectedly.".to_owned(),
        })?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_environment_workspace(
    state: State<'_, AppState>,
) -> Result<EnvironmentWorkspaceModel, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || application.environment_workspace())
        .await
        .map_err(|_| CommandError {
            code: "environment_workspace_worker_unavailable".to_owned(),
            diagnostic: "The Environment workspace worker stopped unexpectedly.".to_owned(),
        })?
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_environment_history(
    state: State<'_, AppState>,
) -> Result<EnvironmentHistoryModel, CommandError> {
    state.application.environment_history().map_err(Into::into)
}

#[tauri::command]
pub fn get_environment_snapshot(
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<Option<EnvironmentSnapshot>, CommandError> {
    state
        .application
        .environment_snapshot(&snapshot_id)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn save_carbon_factor_set(
    draft: CarbonFactorSetDraft,
    state: State<'_, AppState>,
) -> Result<EnvironmentWorkspaceModel, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || application.save_carbon_factor_set(&draft))
        .await
        .map_err(|_| CommandError {
            code: "carbon_factor_worker_unavailable".to_owned(),
            diagnostic: "The Carbon study worker stopped unexpectedly.".to_owned(),
        })?
        .map_err(Into::into)
}

#[tauri::command]
pub fn select_carbon_factor_set(
    factor_set_id: String,
    revision: u32,
    state: State<'_, AppState>,
) -> Result<EnvironmentWorkspaceModel, CommandError> {
    state
        .application
        .select_carbon_factor_set(&factor_set_id, revision)
        .map_err(Into::into)
}

#[tauri::command]
pub fn rollback_carbon_factor_set(
    factor_set_id: String,
    state: State<'_, AppState>,
) -> Result<EnvironmentWorkspaceModel, CommandError> {
    state
        .application
        .rollback_carbon_factor_set(&factor_set_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn remove_carbon_factor_set(
    factor_set_id: String,
    state: State<'_, AppState>,
) -> Result<EnvironmentWorkspaceModel, CommandError> {
    state
        .application
        .remove_carbon_factor_set(&factor_set_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn export_carbon_factor_set(
    factor_set_id: String,
    revision: u32,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    state
        .application
        .export_carbon_factor_set(&factor_set_id, revision)
        .map_err(Into::into)
}

#[tauri::command]
pub fn preview_carbon_factor_import(
    csv: String,
    state: State<'_, AppState>,
) -> Result<CarbonFactorImportPreview, CommandError> {
    state
        .application
        .preview_carbon_factor_import(&csv)
        .map_err(Into::into)
}

#[tauri::command]
pub fn apply_carbon_factor_import(
    csv: String,
    state: State<'_, AppState>,
) -> Result<EnvironmentWorkspaceModel, CommandError> {
    state
        .application
        .apply_carbon_factor_import(&csv)
        .map_err(Into::into)
}

#[tauri::command]
pub fn enable_environment_recording(
    consent: bool,
    state: State<'_, AppState>,
) -> Result<EnvironmentWorkspaceModel, CommandError> {
    if !consent {
        return Err(ObservatoryError::EnvironmentRecordingConsentRequired.into());
    }
    state
        .application
        .enable_environment_recording(crate::environment::ENVIRONMENT_RECORDING_NOTICE_REVISION)?;
    state
        .application
        .environment_workspace()
        .map_err(Into::into)
}

#[tauri::command]
pub fn disable_environment_recording(
    state: State<'_, AppState>,
) -> Result<EnvironmentWorkspaceModel, CommandError> {
    state.application.disable_environment_recording()?;
    state
        .application
        .environment_workspace()
        .map_err(Into::into)
}

#[tauri::command]
pub fn capture_environment_snapshot(
    state: State<'_, AppState>,
) -> Result<EnvironmentCaptureResult, CommandError> {
    state
        .application
        .capture_environment_snapshot()
        .map_err(Into::into)
}

#[tauri::command]
pub fn delete_live_environmental_recordings(
    confirmation: String,
    state: State<'_, AppState>,
) -> Result<EnvironmentWorkspaceModel, CommandError> {
    if confirmation != "DELETE LIVE ENVIRONMENT RECORDINGS" {
        return Err(ObservatoryError::InvalidApplicationPreferences.into());
    }
    state.application.delete_live_environmental_recordings()?;
    state
        .application
        .environment_workspace()
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_published_metric_contexts() -> Vec<PublishedMetricContext> {
    crate::metric_catalogue::published_metric_contexts()
}

#[tauri::command]
pub fn get_republic_plan_workspace(
    state: State<'_, AppState>,
) -> Result<RepublicPlanWorkspace, CommandError> {
    state
        .application
        .republic_plan_workspace()
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_market_workspace(
    state: State<'_, AppState>,
) -> Result<MarketWorkspace, CommandError> {
    run_market_workspace_command(Arc::clone(&state.application), |application| {
        application.market_workspace()
    })
    .await
}

#[tauri::command]
pub async fn get_market_price_series(
    currency: String,
    resource_token: String,
    state: State<'_, AppState>,
) -> Result<MarketPriceSeries, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || {
        application.market_price_series(&currency, &resource_token)
    })
    .await
    .map_err(|_| CommandError {
        code: "market_workspace_worker_unavailable".to_owned(),
        diagnostic: "The Markets workspace worker stopped unexpectedly.".to_owned(),
    })?
    .map_err(Into::into)
}

#[tauri::command]
pub async fn save_market_basket(
    draft: MarketBasketDraft,
    state: State<'_, AppState>,
) -> Result<MarketWorkspace, CommandError> {
    run_market_workspace_command(Arc::clone(&state.application), move |application| {
        application.save_market_basket(&draft)
    })
    .await
}

#[tauri::command]
pub async fn save_market_scenario(
    draft: MarketScenarioDraft,
    state: State<'_, AppState>,
) -> Result<MarketWorkspace, CommandError> {
    run_market_workspace_command(Arc::clone(&state.application), move |application| {
        application.save_market_scenario(&draft)
    })
    .await
}

#[tauri::command]
pub async fn select_market_definition(
    kind: String,
    definition_id: String,
    revision: u32,
    state: State<'_, AppState>,
) -> Result<MarketWorkspace, CommandError> {
    run_market_workspace_command(Arc::clone(&state.application), move |application| {
        application.select_market_definition(&kind, &definition_id, revision)
    })
    .await
}

#[tauri::command]
pub async fn rollback_market_definition(
    kind: String,
    definition_id: String,
    state: State<'_, AppState>,
) -> Result<MarketWorkspace, CommandError> {
    run_market_workspace_command(Arc::clone(&state.application), move |application| {
        application.rollback_market_definition(&kind, &definition_id)
    })
    .await
}

#[tauri::command]
pub async fn clear_market_selection(
    kind: String,
    state: State<'_, AppState>,
) -> Result<MarketWorkspace, CommandError> {
    run_market_workspace_command(Arc::clone(&state.application), move |application| {
        application.clear_market_selection(&kind)
    })
    .await
}

#[tauri::command]
pub async fn remove_market_definition(
    kind: String,
    definition_id: String,
    state: State<'_, AppState>,
) -> Result<MarketWorkspace, CommandError> {
    run_market_workspace_command(Arc::clone(&state.application), move |application| {
        application.remove_market_definition(&kind, &definition_id)
    })
    .await
}

#[tauri::command]
pub fn save_republic_plan(
    draft: RepublicPlanDraft,
    state: State<'_, AppState>,
) -> Result<RepublicPlanWorkspace, CommandError> {
    state
        .application
        .save_republic_plan(&draft)
        .map_err(Into::into)
}

#[tauri::command]
pub fn activate_republic_plan(
    plan_id: String,
    revision: Option<u32>,
    state: State<'_, AppState>,
) -> Result<RepublicPlanWorkspace, CommandError> {
    state
        .application
        .activate_republic_plan(&plan_id, revision)
        .map_err(Into::into)
}

#[tauri::command]
pub fn rollback_republic_plan(
    plan_id: String,
    state: State<'_, AppState>,
) -> Result<RepublicPlanWorkspace, CommandError> {
    state
        .application
        .rollback_republic_plan(&plan_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn remove_republic_plan(
    plan_id: String,
    state: State<'_, AppState>,
) -> Result<RepublicPlanWorkspace, CommandError> {
    state
        .application
        .remove_republic_plan(&plan_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_recorder_health(state: State<'_, AppState>) -> Result<RecorderHealth, CommandError> {
    state.application.recorder_health().map_err(Into::into)
}

#[tauri::command]
pub async fn choose_and_configure_directory(
    kind: DirectoryKind,
    title: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SetupState, CommandError> {
    if title.trim().is_empty() || title.chars().count() > 160 || title.chars().any(char::is_control)
    {
        return Err(ObservatoryError::InvalidDirectory.into());
    }
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || {
        let configured = application.configured_directory_path(kind)?;
        let suggestion = suggest_directory(kind, configured.as_deref());
        let mut picker = app.dialog().file().set_title(title);
        if let Some(suggestion) = suggestion {
            picker = picker.set_directory(picker_start_directory(&suggestion.path));
        }
        let Some(selected) = picker.blocking_pick_folder() else {
            return application.setup_state();
        };
        let path = selected
            .into_path()
            .map_err(|_| ObservatoryError::InvalidDirectory)?;
        application.configure_directory(kind, path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|_| CommandError {
        code: "directory_picker_unavailable".to_owned(),
        diagnostic: "The folder picker stopped unexpectedly.".to_owned(),
    })?
    .map_err(Into::into)
}

#[tauri::command]
pub fn observe_latest_save(
    state: State<'_, AppState>,
) -> Result<ObservationImportResult, CommandError> {
    state.application.observe_latest_save().map_err(Into::into)
}

#[tauri::command]
pub fn set_automatic_observation(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<SetupState, CommandError> {
    state
        .application
        .set_automatic_observation(enabled)
        .map_err(Into::into)
}

#[tauri::command]
pub fn select_timeline_branch(
    branch_id: String,
    state: State<'_, AppState>,
) -> Result<AnalysisContextResult, CommandError> {
    state
        .application
        .select_branch(&branch_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn inspect_archive_observation(
    interpretation_id: String,
    state: State<'_, AppState>,
) -> Result<AnalysisContextResult, CommandError> {
    state
        .application
        .inspect_archive_observation(&interpretation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn return_to_branch_tip(
    state: State<'_, AppState>,
) -> Result<AnalysisContextResult, CommandError> {
    state.application.return_to_branch_tip().map_err(Into::into)
}

#[tauri::command]
pub fn create_timeline_continuation(
    interpretation_id: String,
    label: Option<String>,
    state: State<'_, AppState>,
) -> Result<AnalysisContextResult, CommandError> {
    state
        .application
        .create_continuation(&interpretation_id, label.as_deref())
        .map_err(Into::into)
}

#[tauri::command]
pub fn set_timeline_branch_label(
    branch_id: String,
    label: Option<String>,
    state: State<'_, AppState>,
) -> Result<AnalysisContextResult, CommandError> {
    state
        .application
        .set_branch_label(&branch_id, label.as_deref())
        .map_err(Into::into)
}

#[tauri::command]
pub fn compare_archive_observations(
    from_interpretation_id: String,
    to_interpretation_id: String,
    state: State<'_, AppState>,
) -> Result<ArchiveComparison, CommandError> {
    state
        .application
        .compare_observations(&from_interpretation_id, &to_interpretation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_compatibility_status(
    state: State<'_, AppState>,
) -> Result<CompatibilityStatus, CommandError> {
    state.application.compatibility_status().map_err(Into::into)
}

#[tauri::command]
pub fn create_local_compatibility_override(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CompatibilityUpdate, CommandError> {
    let update = state.application.create_compatibility_override()?;
    let _ = app.emit(
        crate::compatibility_service::COMPATIBILITY_UPDATE_EVENT,
        &update,
    );
    crate::compatibility_service::schedule_catalogue_refresh(
        app.clone(),
        Arc::clone(&state.application),
        &update,
    );
    Ok(update)
}

#[tauri::command]
pub fn reload_local_compatibility_override(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CompatibilityUpdate, CommandError> {
    let update = state.application.reload_compatibility()?;
    let _ = app.emit(
        crate::compatibility_service::COMPATIBILITY_UPDATE_EVENT,
        &update,
    );
    crate::compatibility_service::schedule_catalogue_refresh(
        app.clone(),
        Arc::clone(&state.application),
        &update,
    );
    Ok(update)
}

#[tauri::command]
pub fn get_reinterpretation_progress(
    state: State<'_, AppState>,
) -> Result<ReinterpretationProgress, CommandError> {
    state
        .application
        .reinterpretation_progress()
        .map_err(Into::into)
}

#[tauri::command]
pub async fn reinterpret_latest_save(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ObservationImportResult, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || {
        application.reinterpret_latest_save(|progress| {
            let _ = app.emit(
                crate::compatibility_service::REINTERPRETATION_PROGRESS_EVENT,
                progress,
            );
        })
    })
    .await
    .map_err(|_| CommandError {
        code: "reinterpretation_worker_unavailable".to_owned(),
        diagnostic: "The save reinterpretation worker stopped unexpectedly.".to_owned(),
    })?
    .map_err(Into::into)
}

#[tauri::command]
pub fn get_market_indexing_progress(
    state: State<'_, AppState>,
) -> Result<MarketIndexingProgress, CommandError> {
    state
        .application
        .market_indexing_progress()
        .map_err(Into::into)
}

#[tauri::command]
pub fn recover_market_indexing(state: State<'_, AppState>) -> Result<(), CommandError> {
    state
        .application
        .recover_market_indexing()
        .map_err(Into::into)
}

#[tauri::command]
pub async fn index_available_saves_for_markets(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<MarketIndexingProgress, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || {
        application.index_available_saves_for_markets(|progress| {
            let _ = app.emit(MARKET_INDEXING_PROGRESS_EVENT, progress);
        })
    })
    .await
    .map_err(|_| CommandError {
        code: "market_indexing_worker_unavailable".to_owned(),
        diagnostic: "The Markets indexing worker stopped unexpectedly.".to_owned(),
    })?
    .map_err(Into::into)
}

#[tauri::command]
pub async fn refresh_changed_market_data(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<MarketIndexingProgress, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || {
        application.refresh_changed_market_data(|progress| {
            let _ = app.emit(MARKET_INDEXING_PROGRESS_EVENT, progress);
        })
    })
    .await
    .map_err(|_| CommandError {
        code: "market_refresh_worker_unavailable".to_owned(),
        diagnostic: "The Markets refresh worker stopped unexpectedly.".to_owned(),
    })?
    .map_err(Into::into)
}

#[tauri::command]
pub fn get_broadcast_indexing_progress(
    state: State<'_, AppState>,
) -> Result<BroadcastIndexingProgress, CommandError> {
    state
        .application
        .broadcast_indexing_progress()
        .map_err(Into::into)
}

#[tauri::command]
pub async fn index_available_saves_for_broadcast(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BroadcastIndexingProgress, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || {
        application.index_available_saves_for_broadcast(|progress| {
            let _ = app.emit(BROADCAST_INDEXING_PROGRESS_EVENT, progress);
        })
    })
    .await
    .map_err(|_| CommandError {
        code: "broadcast_indexing_worker_unavailable".to_owned(),
        diagnostic: "The Broadcast indexing worker stopped unexpectedly.".to_owned(),
    })?
    .map_err(Into::into)
}

#[tauri::command]
pub async fn resume_broadcast_indexing(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BroadcastIndexingProgress, CommandError> {
    index_available_saves_for_broadcast(app, state).await
}

#[tauri::command]
pub fn get_environment_indexing_progress(
    state: State<'_, AppState>,
) -> Result<EnvironmentIndexingProgress, CommandError> {
    state
        .application
        .environment_indexing_progress()
        .map_err(Into::into)
}

#[tauri::command]
pub async fn index_available_saves_for_environment(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EnvironmentIndexingProgress, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || {
        application.index_available_saves_for_environment(|progress| {
            let _ = app.emit(ENVIRONMENT_INDEXING_PROGRESS_EVENT, progress);
        })
    })
    .await
    .map_err(|_| CommandError {
        code: "environment_indexing_worker_unavailable".to_owned(),
        diagnostic: "The Environment indexing worker stopped unexpectedly.".to_owned(),
    })?
    .map_err(Into::into)
}

#[tauri::command]
pub async fn resume_environment_indexing(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EnvironmentIndexingProgress, CommandError> {
    index_available_saves_for_environment(app, state).await
}

#[tauri::command]
pub fn get_catalogue_status(state: State<'_, AppState>) -> Result<CatalogueStatus, CommandError> {
    state.application.catalogue_status().map_err(Into::into)
}

#[tauri::command]
pub async fn refresh_definitions(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CatalogueStatus, CommandError> {
    let application = Arc::clone(&state.application);
    tauri::async_runtime::spawn_blocking(move || {
        application.refresh_catalogue(crate::model::CatalogueRefreshTrigger::Manual, |progress| {
            let _ = app.emit(crate::catalogue_service::CATALOGUE_PROGRESS_EVENT, progress);
        })
    })
    .await
    .map_err(|_| CommandError {
        code: "catalogue_worker_unavailable".to_owned(),
        diagnostic: "The definition refresh worker stopped unexpectedly.".to_owned(),
    })?
    .map_err(Into::into)
}

#[tauri::command]
pub fn diagnostic_log() -> DiagnosticLogView {
    crate::diagnostics::view()
}

#[tauri::command]
pub fn clear_diagnostic_log() -> DiagnosticLogView {
    crate::diagnostics::clear()
}

#[tauri::command]
pub fn rebuild_warehouse(state: State<'_, AppState>) -> Result<CatalogueStatus, CommandError> {
    state.application.rebuild_warehouse().map_err(Into::into)
}

#[tauri::command]
pub fn search_catalogue(
    filter: CatalogueSearchFilter,
    state: State<'_, AppState>,
) -> Result<CataloguePage, CommandError> {
    state
        .application
        .catalogue_search(&filter)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_resource_catalogue(
    request: ResourceCatalogueRequest,
    state: State<'_, AppState>,
) -> Result<ResourceCatalogueView, CommandError> {
    state
        .application
        .resource_catalogue(&request)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_resource_details(
    resource_id: String,
    state: State<'_, AppState>,
) -> Result<ResourceDetails, CommandError> {
    state
        .application
        .resource_details(&resource_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_resource_registry_status(
    state: State<'_, AppState>,
) -> Result<ResourceRegistryStatus, CommandError> {
    state
        .application
        .resource_registry_status()
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_resource_registry_snapshots(
    state: State<'_, AppState>,
) -> Result<Vec<ResourceRegistrySnapshotSummary>, CommandError> {
    state
        .application
        .list_resource_registry_snapshots()
        .map_err(Into::into)
}

#[tauri::command]
pub fn enable_resource_registry_ingestion(
    assurance: ResourceRegistryAssurance,
    acknowledged: bool,
    state: State<'_, AppState>,
) -> Result<ResourceRegistryStatus, CommandError> {
    state
        .application
        .enable_resource_registry_ingestion(assurance, acknowledged)
        .map_err(Into::into)
}

#[tauri::command]
pub fn disable_resource_registry_ingestion(
    state: State<'_, AppState>,
) -> Result<ResourceRegistryStatus, CommandError> {
    state
        .application
        .disable_resource_registry_ingestion()
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_definition_dossier(
    entity_id: String,
    state: State<'_, AppState>,
) -> Result<DefinitionDossier, CommandError> {
    state
        .application
        .catalogue_dossier(&entity_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_production_route(
    request: ProductionRouteRequest,
    state: State<'_, AppState>,
) -> Result<ProductionRouteModel, CommandError> {
    state
        .application
        .production_route(&request)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_production_pathway(
    request: ProductionPathwayRequest,
    state: State<'_, AppState>,
) -> Result<ProductionPathwayModel, CommandError> {
    state
        .application
        .production_pathway(&request)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_production_route_coverage(
    state: State<'_, AppState>,
) -> Result<ProductionRouteCoverage, CommandError> {
    state
        .application
        .production_route_coverage()
        .map_err(Into::into)
}

#[tauri::command]
pub fn inspect_planning_overlay(json: String, state: State<'_, AppState>) -> OverlayInspection {
    state.application.inspect_overlay(&json)
}

#[tauri::command]
pub fn import_planning_overlay(
    json: String,
    state: State<'_, AppState>,
) -> Result<OverlayProfileSummary, CommandError> {
    state.application.import_overlay(&json).map_err(Into::into)
}

#[tauri::command]
pub fn export_planning_overlay(
    profile_id: String,
    revision: u32,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    state
        .application
        .export_overlay(&profile_id, revision)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_planning_overlays(
    state: State<'_, AppState>,
) -> Result<Vec<OverlayProfileSummary>, CommandError> {
    state.application.overlay_profiles().map_err(Into::into)
}

#[tauri::command]
pub fn activate_planning_overlay(
    profile_id: String,
    revision: Option<u32>,
    state: State<'_, AppState>,
) -> Result<OverlayProfileSummary, CommandError> {
    state
        .application
        .activate_overlay(&profile_id, revision)
        .map_err(Into::into)
}

#[tauri::command]
pub fn rollback_planning_overlay(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<OverlayProfileSummary, CommandError> {
    state
        .application
        .rollback_overlay(&profile_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn deactivate_planning_overlay(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.application.deactivate_overlay().map_err(Into::into)
}

#[tauri::command]
pub fn remove_planning_overlay(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    state
        .application
        .remove_overlay(&profile_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_warehouse_snapshot(
    state: State<'_, AppState>,
) -> Result<WarehouseSnapshot, CommandError> {
    state.application.warehouse_snapshot().map_err(Into::into)
}

#[tauri::command]
pub fn inspect_analysis_pack(json: String, state: State<'_, AppState>) -> AnalysisPackInspection {
    state.application.inspect_analysis_pack(&json)
}

#[tauri::command]
pub fn import_analysis_pack(
    json: String,
    state: State<'_, AppState>,
) -> Result<AnalysisPackSummary, CommandError> {
    state
        .application
        .import_analysis_pack(&json)
        .map_err(Into::into)
}

#[tauri::command]
pub fn export_analysis_pack(
    pack_id: String,
    revision: u32,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    state
        .application
        .export_analysis_pack(&pack_id, revision)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_analysis_packs(
    state: State<'_, AppState>,
) -> Result<Vec<AnalysisPackSummary>, CommandError> {
    state.application.analysis_packs().map_err(Into::into)
}

#[tauri::command]
pub fn enable_analysis_pack(
    pack_id: String,
    revision: Option<u32>,
    state: State<'_, AppState>,
) -> Result<AnalysisPackSummary, CommandError> {
    state
        .application
        .enable_analysis_pack(&pack_id, revision)
        .map_err(Into::into)
}

#[tauri::command]
pub fn disable_analysis_pack(
    pack_id: String,
    state: State<'_, AppState>,
) -> Result<AnalysisPackSummary, CommandError> {
    state
        .application
        .disable_analysis_pack(&pack_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn rollback_analysis_pack(
    pack_id: String,
    state: State<'_, AppState>,
) -> Result<AnalysisPackSummary, CommandError> {
    state
        .application
        .rollback_analysis_pack(&pack_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn remove_analysis_pack(
    pack_id: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    state
        .application
        .remove_analysis_pack(&pack_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_analysis_pack_contributions(
    state: State<'_, AppState>,
) -> Result<Vec<AnalysisPackContribution>, CommandError> {
    state
        .application
        .analysis_pack_contributions()
        .map_err(Into::into)
}

#[tauri::command]
pub fn language_status(state: State<'_, AppState>) -> Result<LanguageStatus, CommandError> {
    state.application.language_status().map_err(Into::into)
}

#[tauri::command]
pub fn inspect_language_pack(json: String, state: State<'_, AppState>) -> LanguagePackInspection {
    state.application.inspect_language_pack(&json)
}

#[tauri::command]
pub fn install_language_pack(
    json: String,
    state: State<'_, AppState>,
) -> Result<LanguageStatus, CommandError> {
    state
        .application
        .install_language_pack(&json)
        .map_err(Into::into)
}

#[tauri::command]
pub fn select_language_pack(
    pack_id: String,
    state: State<'_, AppState>,
) -> Result<LanguageStatus, CommandError> {
    state
        .application
        .select_language_pack(&pack_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn remove_language_pack(
    pack_id: String,
    state: State<'_, AppState>,
) -> Result<LanguageStatus, CommandError> {
    state
        .application
        .remove_language_pack(&pack_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn export_language_pack(
    pack_id: String,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    state
        .application
        .export_language_pack(&pack_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn handover_legacy_language_packs(
    handover: LegacyLanguageHandover,
    state: State<'_, AppState>,
) -> Result<LanguageStatus, CommandError> {
    state
        .application
        .handover_legacy_language_packs(&handover)
        .map_err(Into::into)
}

#[tauri::command]
pub fn theme_status(state: State<'_, AppState>) -> Result<ThemeStatus, CommandError> {
    state.application.theme_status().map_err(Into::into)
}

#[tauri::command]
pub fn inspect_theme(document: String, state: State<'_, AppState>) -> ThemeInspection {
    state.application.inspect_theme(&document)
}

#[tauri::command]
pub fn import_theme(
    document: String,
    state: State<'_, AppState>,
) -> Result<ThemeStatus, CommandError> {
    state
        .application
        .import_theme(&document)
        .map_err(Into::into)
}

#[tauri::command]
pub fn select_theme(
    theme_id: String,
    version: String,
    content_hash: String,
    state: State<'_, AppState>,
) -> Result<ThemeStatus, CommandError> {
    state
        .application
        .select_theme(&theme_id, &version, &content_hash)
        .map_err(Into::into)
}

#[tauri::command]
pub fn export_theme(
    theme_id: String,
    version: String,
    content_hash: String,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    state
        .application
        .export_theme(&theme_id, &version, &content_hash)
        .map_err(Into::into)
}

#[tauri::command]
pub fn remove_theme(
    theme_id: String,
    version: String,
    state: State<'_, AppState>,
) -> Result<ThemeStatus, CommandError> {
    state
        .application
        .remove_theme(&theme_id, &version)
        .map_err(Into::into)
}
