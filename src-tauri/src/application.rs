use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::automatic_observer::{AutomaticObserver, latest_save_candidate_path};
use crate::definition_catalogue::{catalogue_watch_roots, discover_catalogue_with_reuse};
use crate::error::ObservatoryError;
use crate::game_vocabulary::{discover_game_vocabularies, resolve_game_media_directory};
use crate::model::{
    ArchiveComparison, ArchiveOverview, AutomaticObservationUpdate, BranchSelectionResult,
    CataloguePage, CatalogueSearchFilter, CatalogueStatus, ConfiguredDirectorySummary,
    DefinitionDossier, DirectoryKind, ImportOutcome, ObservationImportResult, OverlayInspection,
    OverlayProfileSummary, ReceiverDataset, RecorderDiscoverySource, RecorderHealth,
    RecorderUpdate, SetupState, WarehouseSnapshot,
};
use crate::planning_overlay::PlanningOverlayDocument;
use crate::save_archive::inspect_save_archive;
use crate::storage::{ObservatoryStorage, now_ms};
use crate::warehouse::AnalyticalWarehouse;

const SAVE_DIRECTORY_KEY: &str = "save_directory";
const GAME_MEDIA_DIRECTORY_KEY: &str = "game_media_directory";
const WORKSHOP_DIRECTORY_KEY: &str = "workshop_directory";
const AUTOMATIC_OBSERVATION_KEY: &str = "automatic_observation_enabled";

#[derive(Debug)]
pub struct ObservatoryApplication {
    storage: ObservatoryStorage,
    warehouse: AnalyticalWarehouse,
    automatic_observer: Mutex<AutomaticObserver>,
    catalogue_refresh: Mutex<()>,
}

impl ObservatoryApplication {
    pub fn initialise(
        database_path: PathBuf,
        warehouse_path: PathBuf,
    ) -> Result<Self, ObservatoryError> {
        let storage = ObservatoryStorage::initialise(database_path)?;
        let warehouse = AnalyticalWarehouse::initialise(warehouse_path.clone())
            .unwrap_or_else(|_| AnalyticalWarehouse::unavailable(warehouse_path));
        let automatic_observation_enabled = storage.get_bool_setting(AUTOMATIC_OBSERVATION_KEY)?;
        let directory_configured = storage
            .get_setting(SAVE_DIRECTORY_KEY)?
            .is_some_and(|path| path.is_dir());
        let mut automatic_observer = AutomaticObserver::new(automatic_observation_enabled);
        automatic_observer.set_enabled(automatic_observation_enabled, directory_configured);
        Ok(Self {
            storage,
            warehouse,
            automatic_observer: Mutex::new(automatic_observer),
            catalogue_refresh: Mutex::new(()),
        })
    }

    pub fn setup_state(&self) -> Result<SetupState, ObservatoryError> {
        let save_path = self.storage.get_setting(SAVE_DIRECTORY_KEY)?;
        let game_path = self.storage.get_setting(GAME_MEDIA_DIRECTORY_KEY)?;
        let workshop_path = self.storage.get_setting(WORKSHOP_DIRECTORY_KEY)?;
        let save_directory = save_path
            .as_deref()
            .filter(|path| path.is_dir())
            .map(|path| ConfiguredDirectorySummary {
                name: directory_display_name(path, false),
            });
        let game_directory = game_path
            .as_deref()
            .filter(|path| path.is_dir())
            .map(|path| ConfiguredDirectorySummary {
                name: directory_display_name(path, true),
            });
        let workshop_directory =
            workshop_path
                .as_deref()
                .filter(|path| path.is_dir())
                .map(|path| ConfiguredDirectorySummary {
                    name: directory_display_name(path, false),
                });
        let save_candidates = save_path
            .as_deref()
            .filter(|path| path.is_dir())
            .map(count_save_candidates)
            .transpose()?
            .unwrap_or_default();
        let game_vocabularies = game_path
            .as_deref()
            .filter(|path| path.is_dir())
            .map(discover_game_vocabularies)
            .transpose()?
            .unwrap_or_default();

        Ok(SetupState {
            save_directory,
            game_directory,
            workshop_directory,
            save_candidates,
            observed_saves: self.storage.file_observation_count()?,
            distinct_states: self.storage.distinct_state_count()?,
            game_vocabularies,
            automatic_observer: self.observer_status()?,
        })
    }

    pub fn latest_receiver_dataset(&self) -> Result<Option<ReceiverDataset>, ObservatoryError> {
        self.storage.load_latest_dataset()
    }

    pub fn archive_overview(&self) -> Result<ArchiveOverview, ObservatoryError> {
        self.storage.load_archive_overview()
    }

    pub fn configure_directory(
        &self,
        kind: DirectoryKind,
        path: String,
    ) -> Result<SetupState, ObservatoryError> {
        let selected = PathBuf::from(path);
        match kind {
            DirectoryKind::Save => {
                let canonical = selected
                    .canonicalize()
                    .map_err(|_| ObservatoryError::InvalidDirectory)?;
                if !canonical.is_dir() {
                    return Err(ObservatoryError::InvalidDirectory);
                }
                self.storage.set_setting(SAVE_DIRECTORY_KEY, &canonical)?;
                let mut observer = self
                    .automatic_observer
                    .lock()
                    .map_err(|_| ObservatoryError::StorageUnavailable)?;
                let enabled = observer.status().enabled;
                observer.set_enabled(enabled, true);
            }
            DirectoryKind::Game => {
                let media = resolve_game_media_directory(&selected)?;
                self.storage.set_setting(GAME_MEDIA_DIRECTORY_KEY, &media)?;
            }
            DirectoryKind::Workshop => {
                let canonical = selected
                    .canonicalize()
                    .map_err(|_| ObservatoryError::InvalidDirectory)?;
                if !canonical.is_dir() {
                    return Err(ObservatoryError::InvalidDirectory);
                }
                self.storage
                    .set_setting(WORKSHOP_DIRECTORY_KEY, &canonical)?;
            }
        }
        self.setup_state()
    }

    pub fn observe_latest_save(&self) -> Result<ObservationImportResult, ObservatoryError> {
        let directory = self
            .storage
            .get_setting(SAVE_DIRECTORY_KEY)?
            .ok_or(ObservatoryError::SaveDirectoryNotConfigured)?;
        let path = latest_save_candidate_path(&directory)?;
        let inspection = inspect_save_archive(&path)?;
        let inserted = self.storage.save_inspection(&inspection)?;
        let dataset = self.storage.load_dataset(&inspection.payload_hash)?;
        self.automatic_observer
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?
            .record_observation(&inspection.source_file_name, now_ms());
        Ok(ObservationImportResult {
            outcome: if inserted {
                ImportOutcome::Imported
            } else {
                ImportOutcome::Duplicate
            },
            dataset,
        })
    }

    pub fn select_branch(
        &self,
        branch_id: &str,
    ) -> Result<BranchSelectionResult, ObservatoryError> {
        self.storage.select_branch(branch_id)?;
        Ok(BranchSelectionResult {
            archive: self.storage.load_archive_overview()?,
            dataset: self.storage.load_latest_dataset()?,
        })
    }

    pub fn set_automatic_observation(&self, enabled: bool) -> Result<SetupState, ObservatoryError> {
        self.storage
            .set_bool_setting(AUTOMATIC_OBSERVATION_KEY, enabled)?;
        let directory_configured = self
            .storage
            .get_setting(SAVE_DIRECTORY_KEY)?
            .is_some_and(|path| path.is_dir());
        self.automatic_observer
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?
            .set_enabled(enabled, directory_configured);
        self.setup_state()
    }

    pub(crate) fn poll_automatic_observation_from(
        &self,
        discovery_source: RecorderDiscoverySource,
    ) -> Result<AutomaticObservationUpdate, ObservatoryError> {
        let directory = self.storage.get_setting(SAVE_DIRECTORY_KEY)?;
        self.automatic_observer
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?
            .poll(
                &self.storage,
                directory.as_deref(),
                now_ms(),
                discovery_source,
            )
    }

    pub fn recorder_health(&self) -> Result<RecorderHealth, ObservatoryError> {
        self.storage.load_recorder_health(self.observer_status()?)
    }

    pub(crate) fn recorder_update(
        &self,
        import_result: Option<ObservationImportResult>,
    ) -> Result<RecorderUpdate, ObservatoryError> {
        Ok(RecorderUpdate {
            health: self.recorder_health()?,
            import_result,
        })
    }

    pub(crate) fn recorder_configuration(
        &self,
    ) -> Result<(bool, Option<PathBuf>), ObservatoryError> {
        Ok((
            self.observer_status()?.enabled,
            self.storage.get_setting(SAVE_DIRECTORY_KEY)?,
        ))
    }

    pub(crate) fn note_recorder_filesystem_event(
        &self,
        event_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.storage.note_recorder_filesystem_event(event_at_ms)
    }

    pub fn compare_observations(
        &self,
        from_payload_hash: &str,
        to_payload_hash: &str,
    ) -> Result<ArchiveComparison, ObservatoryError> {
        self.storage
            .compare_observations(from_payload_hash, to_payload_hash)
    }

    pub fn catalogue_status(&self) -> Result<CatalogueStatus, ObservatoryError> {
        let queue = self.storage.projection_queue_status()?;
        let rebuilding = self.storage.warehouse_rebuild_running()?;
        let (last_checked_at_ms, last_refreshed_at_ms, warehouse_error) = self
            .warehouse
            .catalogue_runtime()
            .unwrap_or((None, None, Some("warehouse_unavailable".to_owned())));
        let (last_filesystem_event_ms, sqlite_error) = self.storage.catalogue_runtime_state()?;
        let mut active_overlay = self.storage.active_overlay_summary()?;
        if let Some(profile) = &mut active_overlay
            && let Some(revision) = profile.active_revision
        {
            profile.conflict_count = self
                .warehouse
                .overlay_conflict_count(&profile.profile_id, revision);
        }
        Ok(CatalogueStatus {
            warehouse: self
                .warehouse
                .health(
                    queue.pending_jobs,
                    queue.failed_jobs,
                    queue
                        .oldest_unresolved_at_ms
                        .map(|requested_at| now_ms().saturating_sub(requested_at)),
                    rebuilding,
                )?,
            generation: self.warehouse.catalogue_generation().unwrap_or(None),
            last_checked_at_ms,
            last_refreshed_at_ms,
            last_filesystem_event_ms,
            error_code: sqlite_error.or(warehouse_error),
            active_overlay,
        })
    }

    pub fn refresh_catalogue(&self) -> Result<CatalogueStatus, ObservatoryError> {
        let _guard = self
            .catalogue_refresh
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?;
        let requested_at = now_ms();
        self.storage.note_catalogue_refresh_request(requested_at)?;
        let result: Result<(), ObservatoryError> = (|| {
            let game_directory = self
                .storage
                .get_setting(GAME_MEDIA_DIRECTORY_KEY)?
                .filter(|path| path.is_dir())
                .ok_or(ObservatoryError::InvalidGameDirectory)?;
            let workshop_directory = self
                .storage
                .get_setting(WORKSHOP_DIRECTORY_KEY)?
                .filter(|path| path.is_dir());
            let reuse = self.warehouse.catalogue_reuse_cache()?;
            let generation = discover_catalogue_with_reuse(
                &game_directory,
                workshop_directory.as_deref(),
                requested_at,
                &reuse,
            )?;
            let changed = self.warehouse.publish_catalogue(&generation)?;
            if changed {
                let active = self.storage.active_overlay_document()?;
                let overlay_identity = active
                    .as_ref()
                    .map(|(profile, revision, _)| format!("{profile}:{revision}"))
                    .unwrap_or_else(|| "none".to_owned());
                let identity = format!("{}:{overlay_identity}", generation.generation_id);
                self.storage.enqueue_current_overlay_projection(&identity)?;
            }
            Ok(())
        })();
        if let Err(error) = &result {
            self.warehouse
                .note_catalogue_failure(requested_at, error.code());
            let _ = self.storage.note_catalogue_refresh_failure(error.code());
        }
        result?;
        self.catalogue_status()
    }

    pub fn catalogue_search(
        &self,
        filter: &CatalogueSearchFilter,
    ) -> Result<CataloguePage, ObservatoryError> {
        self.warehouse.search(filter)
    }

    pub fn catalogue_dossier(
        &self,
        entity_id: &str,
    ) -> Result<DefinitionDossier, ObservatoryError> {
        self.warehouse.dossier(entity_id)
    }

    pub fn rebuild_warehouse(&self) -> Result<CatalogueStatus, ObservatoryError> {
        self.storage.enqueue_warehouse_rebuild()?;
        self.catalogue_status()
    }

    pub fn inspect_overlay(&self, json: &str) -> OverlayInspection {
        match PlanningOverlayDocument::parse(json.as_bytes()) {
            Ok(document) => OverlayInspection {
                valid: true,
                code: None,
                profile: None,
                operation_count: document.operations.len().min(u32::MAX as usize) as u32,
                supplement_count: document.supplements.len().min(u32::MAX as usize) as u32,
                document: serde_json::to_value(document).ok(),
            },
            Err(error) => OverlayInspection {
                valid: false,
                code: Some(error.code().to_owned()),
                profile: None,
                operation_count: 0,
                supplement_count: 0,
                document: None,
            },
        }
    }

    pub fn import_overlay(&self, json: &str) -> Result<OverlayProfileSummary, ObservatoryError> {
        let document = PlanningOverlayDocument::parse(json.as_bytes())?;
        self.storage.install_overlay(&document)
    }

    pub fn export_overlay(
        &self,
        profile_id: &str,
        revision: u32,
    ) -> Result<String, ObservatoryError> {
        self.storage
            .overlay_document(profile_id, revision)?
            .canonical_json()
    }

    pub fn overlay_profiles(&self) -> Result<Vec<OverlayProfileSummary>, ObservatoryError> {
        let mut profiles = self.storage.list_overlay_profiles()?;
        for profile in &mut profiles {
            if let Some(revision) = profile.active_revision {
                profile.conflict_count = self
                    .warehouse
                    .overlay_conflict_count(&profile.profile_id, revision);
            }
        }
        Ok(profiles)
    }

    pub fn activate_overlay(
        &self,
        profile_id: &str,
        revision: Option<u32>,
    ) -> Result<OverlayProfileSummary, ObservatoryError> {
        self.storage.activate_overlay(profile_id, revision)
    }

    pub fn rollback_overlay(
        &self,
        profile_id: &str,
    ) -> Result<OverlayProfileSummary, ObservatoryError> {
        self.storage.rollback_overlay(profile_id)
    }

    pub fn deactivate_overlay(&self) -> Result<(), ObservatoryError> {
        self.storage.deactivate_overlay()
    }

    pub fn remove_overlay(&self, profile_id: &str) -> Result<(), ObservatoryError> {
        self.storage.remove_overlay(profile_id)
    }

    pub fn warehouse_snapshot(&self) -> Result<WarehouseSnapshot, ObservatoryError> {
        self.warehouse.snapshot()
    }

    pub(crate) fn process_next_projection_job(&self) -> Result<bool, ObservatoryError> {
        let Some(job) = self.storage.claim_projection_job()? else {
            return Ok(false);
        };
        let result = match job.projection_kind.as_str() {
            "observation" => self
                .storage
                .load_dataset(&job.source_identity)
                .and_then(|dataset| {
                    self.warehouse
                        .project_observation(&job.projection_id, &dataset, now_ms())
                }),
            "overlay_state" => self.storage.active_overlay_document().and_then(|active| {
                self.warehouse.project_overlay(
                    &job.projection_id,
                    active.as_ref().map(|(profile, revision, document)| {
                        (profile.as_str(), *revision, document)
                    }),
                    now_ms(),
                )
            }),
            "rebuild" => self
                .warehouse
                .rebuild_observations(&job.projection_id, now_ms()),
            _ => Err(ObservatoryError::WarehouseUnavailable),
        };
        match result {
            Ok(()) => self.storage.complete_projection_job(&job.projection_id)?,
            Err(error) => {
                self.storage
                    .fail_projection_job(&job.projection_id, error.code())?;
                return Err(error);
            }
        }
        Ok(true)
    }

    pub(crate) fn catalogue_configuration(&self) -> Result<Option<PathBuf>, ObservatoryError> {
        self.storage.get_setting(GAME_MEDIA_DIRECTORY_KEY)
    }

    pub(crate) fn catalogue_watch_roots(&self) -> Result<Vec<PathBuf>, ObservatoryError> {
        let game = self.catalogue_configuration()?;
        let workshop = self
            .storage
            .get_setting(WORKSHOP_DIRECTORY_KEY)?
            .filter(|path| path.is_dir());
        Ok(game
            .as_deref()
            .map(|path| catalogue_watch_roots(path, workshop.as_deref()))
            .unwrap_or_default())
    }

    pub(crate) fn note_catalogue_filesystem_event(
        &self,
        event_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.storage.note_catalogue_filesystem_event(event_at_ms)
    }

    fn observer_status(&self) -> Result<crate::model::AutomaticObserverStatus, ObservatoryError> {
        self.automatic_observer
            .lock()
            .map(|observer| observer.status())
            .map_err(|_| ObservatoryError::StorageUnavailable)
    }
}

fn count_save_candidates(directory: &Path) -> Result<u32, ObservatoryError> {
    let count = fs::read_dir(directory)
        .map_err(|_| ObservatoryError::InvalidDirectory)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_file()) && has_zip_extension(&entry.path())
        })
        .count();
    Ok(count.min(u32::MAX as usize) as u32)
}

fn has_zip_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn directory_display_name(path: &Path, game_media: bool) -> String {
    let display_path = if game_media
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("media_soviet"))
    {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    display_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Configured folder")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{ObservatoryApplication, count_save_candidates};
    use crate::automatic_observer::latest_save_candidate_path;
    use crate::model::WarehousePhase;

    #[test]
    fn candidates_are_zip_files_only() {
        let directory = tempdir().expect("temporary directory");
        fs::write(directory.path().join("one.zip"), b"one").expect("zip candidate");
        fs::write(directory.path().join("notes.txt"), b"notes").expect("ignored file");

        assert_eq!(count_save_candidates(directory.path()).expect("count"), 1);
        assert_eq!(
            latest_save_candidate_path(directory.path())
                .expect("candidate")
                .file_name()
                .and_then(|name| name.to_str()),
            Some("one.zip")
        );
    }

    #[test]
    fn unavailable_warehouse_does_not_block_operational_startup() {
        let directory = tempdir().expect("temporary directory");
        let unavailable_path = directory.path().join("warehouse-is-a-directory");
        fs::create_dir(&unavailable_path).expect("blocking directory");
        let application = ObservatoryApplication::initialise(
            directory.path().join("operational.sqlite3"),
            unavailable_path,
        )
        .expect("SQLite startup must survive a warehouse failure");

        application.setup_state().expect("operational state");
        let status = application.catalogue_status().expect("degraded status");
        assert_eq!(status.warehouse.phase, WarehousePhase::Attention);
        assert_eq!(status.error_code.as_deref(), Some("warehouse_unavailable"));
    }
}
