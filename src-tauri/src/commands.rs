use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::analysis_pack::{AnalysisPackContribution, AnalysisPackInspection, AnalysisPackSummary};
use crate::application::ObservatoryApplication;
use crate::error::CommandError;
use crate::language_pack::{LanguagePackInspection, LanguageStatus, LegacyLanguageHandover};
use crate::model::{
    AnalysisContextResult, ArchiveComparison, ArchiveOverview, CataloguePage,
    CatalogueSearchFilter, CatalogueStatus, CompatibilityStatus, CompatibilityUpdate,
    DefinitionDossier, DiagnosticLogView, DirectoryKind, ObservationImportResult,
    OverlayInspection, OverlayProfileSummary, PopulationDataset, ProductionRouteCoverage,
    ProductionRouteModel, ProductionRouteRequest, ReceiverDataset, RecorderHealth,
    ReinterpretationProgress, SetupState, WarehouseSnapshot,
};
use crate::theme::{ThemeInspection, ThemeStatus};

#[derive(Debug)]
pub struct AppState {
    pub application: Arc<ObservatoryApplication>,
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
pub fn get_recorder_health(state: State<'_, AppState>) -> Result<RecorderHealth, CommandError> {
    state.application.recorder_health().map_err(Into::into)
}

#[tauri::command]
pub fn configure_directory(
    kind: DirectoryKind,
    path: String,
    state: State<'_, AppState>,
) -> Result<SetupState, CommandError> {
    state
        .application
        .configure_directory(kind, path)
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
