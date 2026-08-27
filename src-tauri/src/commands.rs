use std::sync::Arc;

use tauri::State;

use crate::application::ObservatoryApplication;
use crate::error::CommandError;
use crate::model::{
    ArchiveComparison, ArchiveOverview, BranchSelectionResult, DirectoryKind,
    ObservationImportResult, ReceiverDataset, RecorderHealth, SetupState,
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
