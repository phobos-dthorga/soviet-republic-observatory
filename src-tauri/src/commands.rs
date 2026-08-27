use std::sync::Arc;

use tauri::State;

use crate::application::ObservatoryApplication;
use crate::error::CommandError;
use crate::model::{
    ArchiveComparison, ArchiveOverview, BranchSelectionResult, CataloguePage,
    CatalogueSearchFilter, CatalogueStatus, DefinitionDossier, DirectoryKind,
    ObservationImportResult, OverlayInspection, OverlayProfileSummary, ReceiverDataset,
    RecorderHealth, SetupState, WarehouseSnapshot,
};

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
) -> Result<BranchSelectionResult, CommandError> {
    state
        .application
        .select_branch(&branch_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn compare_archive_observations(
    from_payload_hash: String,
    to_payload_hash: String,
    state: State<'_, AppState>,
) -> Result<ArchiveComparison, CommandError> {
    state
        .application
        .compare_observations(&from_payload_hash, &to_payload_hash)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_catalogue_status(state: State<'_, AppState>) -> Result<CatalogueStatus, CommandError> {
    state.application.catalogue_status().map_err(Into::into)
}

#[tauri::command]
pub fn refresh_definitions(state: State<'_, AppState>) -> Result<CatalogueStatus, CommandError> {
    state.application.refresh_catalogue().map_err(Into::into)
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
