use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::compatibility_profile::{
    CompatibilityProfileDocument, CompatibilityProfileSource, ResolvedCompatibilityProfile,
};
use crate::diagnostics;
use crate::error::ObservatoryError;
use crate::model::{
    CompatibilityCatalogueScopeState, CompatibilityCatalogueScopeStatus,
    CompatibilityMappingCoverage, CompatibilityProfileSummary, CompatibilityStatus,
    CompatibilityUpdate, CompatibilityValidationState,
};
use crate::storage::now_ms;

pub const LOCAL_PROFILE_RELATIVE_PATH: &str = "compatibility/local.rocompat.json";

#[derive(Debug)]
pub struct CompatibilityRuntime {
    reviewed: ResolvedCompatibilityProfile,
    legacy_reviewed: Vec<ResolvedCompatibilityProfile>,
    local_path: PathBuf,
    state: Mutex<RuntimeState>,
}

#[derive(Debug)]
struct RuntimeState {
    active: ResolvedCompatibilityProfile,
    local_file_exists: bool,
    local_validation: CompatibilityValidationState,
    last_validation_error: Option<String>,
    last_validated_at_ms: Option<i64>,
    catalogue_scopes: Vec<CompatibilityCatalogueScopeStatus>,
}

impl CompatibilityRuntime {
    pub fn initialise(data_directory: &Path) -> Result<Self, ObservatoryError> {
        let reviewed = ResolvedCompatibilityProfile::reviewed_builtin()?;
        let legacy_reviewed = ResolvedCompatibilityProfile::legacy_reviewed_builtins()?;
        fs::create_dir_all(data_directory.join("compatibility"))
            .map_err(|_| ObservatoryError::InvalidDirectory)?;
        let runtime = Self {
            local_path: data_directory.join(LOCAL_PROFILE_RELATIVE_PATH),
            state: Mutex::new(RuntimeState {
                active: reviewed.clone(),
                local_file_exists: false,
                local_validation: CompatibilityValidationState::Missing,
                last_validation_error: None,
                last_validated_at_ms: None,
                catalogue_scopes: dormant_scope_statuses(&reviewed),
            }),
            reviewed,
            legacy_reviewed,
        };
        runtime.reload()?;
        Ok(runtime)
    }

    pub fn active(&self) -> Result<ResolvedCompatibilityProfile, ObservatoryError> {
        self.state
            .lock()
            .map(|state| state.active.clone())
            .map_err(|_| ObservatoryError::StorageUnavailable)
    }

    pub fn status(&self) -> Result<CompatibilityStatus, ObservatoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?;
        let (stats_markers, stats_fields, definition_operations, binary_layouts, catalogue_scopes) =
            state.active.mapping_counts();
        Ok(CompatibilityStatus {
            active: summary(&state.active),
            reviewed_base: summary(&self.reviewed),
            local_file_path: self.local_path.to_string_lossy().into_owned(),
            local_file_exists: state.local_file_exists,
            local_validation: state.local_validation,
            last_validation_error: state.last_validation_error.clone(),
            last_validated_at_ms: state.last_validated_at_ms,
            detected_game_version: None,
            detected_build_id: None,
            coverage: CompatibilityMappingCoverage {
                stats_markers,
                stats_fields,
                definition_operations,
                binary_layouts,
                catalogue_scopes,
            },
            catalogue_scopes: state.catalogue_scopes.clone(),
        })
    }

    pub fn create_starter_override(&self) -> Result<CompatibilityUpdate, ObservatoryError> {
        if !self.local_path.exists() {
            let parent = self
                .local_path
                .parent()
                .ok_or(ObservatoryError::InvalidDirectory)?;
            fs::create_dir_all(parent).map_err(|_| ObservatoryError::InvalidDirectory)?;
            let document = CompatibilityProfileDocument::starter_override(&self.reviewed);
            let json = document.canonical_json()?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&self.local_path)
                .map_err(|_| ObservatoryError::InvalidDirectory)?;
            file.write_all(json.as_bytes())
                .and_then(|_| file.write_all(b"\n"))
                .and_then(|_| file.sync_all())
                .map_err(|_| ObservatoryError::InvalidDirectory)?;
        }
        self.reload()
    }

    pub fn reload(&self) -> Result<CompatibilityUpdate, ObservatoryError> {
        let validated_at = now_ms();
        let mut state = self
            .state
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?;
        let previous_hash = state.active.resolved_hash().to_owned();
        if !self.local_path.is_file() {
            if state.active.resolved_hash() != self.reviewed.resolved_hash() {
                state.catalogue_scopes = dormant_scope_statuses(&self.reviewed);
            }
            state.active = self.reviewed.clone();
            state.local_file_exists = false;
            state.local_validation = CompatibilityValidationState::Missing;
            state.last_validation_error = None;
            state.last_validated_at_ms = Some(validated_at);
        } else {
            state.local_file_exists = true;
            let result = fs::read(&self.local_path)
                .map_err(|_| ObservatoryError::InvalidCompatibilityProfile("local_read"))
                .and_then(|bytes| CompatibilityProfileDocument::parse(&bytes))
                .and_then(|document| {
                    let reference = document.extends.as_ref().ok_or(
                        ObservatoryError::InvalidCompatibilityProfile("override_requires_base"),
                    )?;
                    let base = std::iter::once(&self.reviewed)
                        .chain(self.legacy_reviewed.iter())
                        .find(|candidate| {
                            candidate.id() == reference.id
                                && candidate.version() == reference.version
                                && candidate.content_hash() == reference.content_hash
                        })
                        .ok_or(ObservatoryError::InvalidCompatibilityProfile(
                            "base_reference_mismatch",
                        ))?;
                    ResolvedCompatibilityProfile::resolve_override(base, document)
                });
            match result {
                Ok(profile) => {
                    if profile.resolved_hash() != state.active.resolved_hash() {
                        state.catalogue_scopes = dormant_scope_statuses(&profile);
                    }
                    state.active = profile;
                    state.local_validation = CompatibilityValidationState::Valid;
                    state.last_validation_error = None;
                    diagnostics::record(
                        "info",
                        "compatibility.override_valid",
                        "compatibility_reload",
                        "The local compatibility override was validated and activated.",
                    );
                }
                Err(error) => {
                    state.local_validation = CompatibilityValidationState::Invalid;
                    state.last_validation_error = Some(profile_error_reason(&error).to_owned());
                    diagnostics::record(
                        "warning",
                        "compatibility.override_invalid",
                        "compatibility_reload",
                        "The local compatibility override was rejected; the last valid profile remains active.",
                    );
                }
            }
            state.last_validated_at_ms = Some(validated_at);
        }
        let profile_changed = previous_hash != state.active.resolved_hash();
        drop(state);
        Ok(CompatibilityUpdate {
            status: self.status()?,
            profile_changed,
            definition_mapping_changed: profile_changed,
        })
    }

    pub fn local_path(&self) -> &Path {
        &self.local_path
    }

    pub fn record_catalogue_scopes(
        &self,
        scopes: Vec<CompatibilityCatalogueScopeStatus>,
    ) -> Result<(), ObservatoryError> {
        self.state
            .lock()
            .map(|mut state| state.catalogue_scopes = scopes)
            .map_err(|_| ObservatoryError::StorageUnavailable)
    }
}

fn dormant_scope_statuses(
    profile: &ResolvedCompatibilityProfile,
) -> Vec<CompatibilityCatalogueScopeStatus> {
    profile
        .catalogue_scopes()
        .iter()
        .map(|scope| CompatibilityCatalogueScopeStatus {
            id: scope.id.clone(),
            source_id: scope.source_id.clone(),
            package_name: None,
            update_policy: match scope.update_policy {
                crate::compatibility_profile::CatalogueScopeUpdatePolicy::Exact => "exact",
                crate::compatibility_profile::CatalogueScopeUpdatePolicy::TrackUpdates => {
                    "track_updates"
                }
            }
            .to_owned(),
            acknowledged_content_hash: scope.acknowledged_content_hash.clone(),
            current_content_hash: None,
            mapping_count: profile.catalogue_scope_mapping_count(&scope.id),
            state: CompatibilityCatalogueScopeState::Dormant,
        })
        .collect()
}

fn summary(profile: &ResolvedCompatibilityProfile) -> CompatibilityProfileSummary {
    let base = profile.base();
    CompatibilityProfileSummary {
        id: profile.id().to_owned(),
        version: profile.version().to_owned(),
        content_hash: profile.content_hash().to_owned(),
        resolved_hash: profile.resolved_hash().to_owned(),
        source: match profile.source() {
            CompatibilityProfileSource::ReviewedBuiltin => "reviewed_builtin",
            CompatibilityProfileSource::LocalOverride => "local_override",
        }
        .to_owned(),
        mapping_classification: profile.source().evidence_classification().to_owned(),
        base_profile_id: base.map(|reference| reference.id.clone()),
        base_profile_version: base.map(|reference| reference.version.clone()),
        base_profile_hash: base.map(|reference| reference.content_hash.clone()),
        target_game_versions: profile.targets().game_versions.clone(),
        target_build_ids: profile.targets().build_ids.clone(),
        target_stats_formats: profile.targets().stats_formats.clone(),
    }
}

fn profile_error_reason(error: &ObservatoryError) -> &'static str {
    match error {
        ObservatoryError::InvalidCompatibilityProfile(reason) => reason,
        _ => error.code(),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::CompatibilityRuntime;
    use crate::model::CompatibilityValidationState;

    #[test]
    fn invalid_local_edit_keeps_the_last_valid_profile_active() {
        let directory = tempdir().expect("directory");
        let runtime = CompatibilityRuntime::initialise(directory.path()).expect("runtime");
        let created = runtime.create_starter_override().expect("starter");
        assert_eq!(
            created.status.local_validation,
            CompatibilityValidationState::Valid
        );
        let active_hash = created.status.active.resolved_hash;
        std::fs::write(runtime.local_path(), b"{\"script\":\"alert(1)\"}").expect("invalid edit");
        let invalid = runtime.reload().expect("reload status");
        assert_eq!(
            invalid.status.local_validation,
            CompatibilityValidationState::Invalid
        );
        assert_eq!(invalid.status.active.resolved_hash, active_hash);
    }
}
