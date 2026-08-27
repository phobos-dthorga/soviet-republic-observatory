use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::automatic_observer::{AutomaticObserver, latest_save_candidate_path};
use crate::error::ObservatoryError;
use crate::game_vocabulary::{discover_game_vocabularies, resolve_game_media_directory};
use crate::model::{
    ArchiveComparison, ArchiveOverview, AutomaticObservationUpdate, BranchSelectionResult,
    ConfiguredDirectorySummary, DirectoryKind, ImportOutcome, ObservationImportResult,
    ReceiverDataset, RecorderDiscoverySource, RecorderHealth, RecorderUpdate, SetupState,
};
use crate::save_archive::inspect_save_archive;
use crate::storage::{ObservatoryStorage, now_ms};

const SAVE_DIRECTORY_KEY: &str = "save_directory";
const GAME_MEDIA_DIRECTORY_KEY: &str = "game_media_directory";
const AUTOMATIC_OBSERVATION_KEY: &str = "automatic_observation_enabled";

#[derive(Debug)]
pub struct ObservatoryApplication {
    storage: ObservatoryStorage,
    automatic_observer: Mutex<AutomaticObserver>,
}

impl ObservatoryApplication {
    pub fn initialise(database_path: PathBuf) -> Result<Self, ObservatoryError> {
        let storage = ObservatoryStorage::initialise(database_path)?;
        let automatic_observation_enabled = storage.get_bool_setting(AUTOMATIC_OBSERVATION_KEY)?;
        let directory_configured = storage
            .get_setting(SAVE_DIRECTORY_KEY)?
            .is_some_and(|path| path.is_dir());
        let mut automatic_observer = AutomaticObserver::new(automatic_observation_enabled);
        automatic_observer.set_enabled(automatic_observation_enabled, directory_configured);
        Ok(Self {
            storage,
            automatic_observer: Mutex::new(automatic_observer),
        })
    }

    pub fn setup_state(&self) -> Result<SetupState, ObservatoryError> {
        let save_path = self.storage.get_setting(SAVE_DIRECTORY_KEY)?;
        let game_path = self.storage.get_setting(GAME_MEDIA_DIRECTORY_KEY)?;
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

    use super::count_save_candidates;
    use crate::automatic_observer::latest_save_candidate_path;

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
}
