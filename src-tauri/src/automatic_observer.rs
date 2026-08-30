use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::compatibility_profile::ResolvedCompatibilityProfile;
use crate::error::ObservatoryError;
use crate::model::{
    AutomaticObservationUpdate, AutomaticObserverPhase, AutomaticObserverStatus, ImportOutcome,
    ObservationImportResult, RecorderCandidateStatus, RecorderDiscoverySource,
};
use crate::save_archive::{directory_identity, inspect_save_archive};
use crate::storage::ObservatoryStorage;

const STABLE_WINDOW_MS: i64 = 1_500;
const MAX_RETRY_ATTEMPTS: u8 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CandidateIdentity {
    path: PathBuf,
    directory_identity: String,
    file_name: String,
    file_size: u64,
    modified_ms: i64,
}

#[derive(Debug)]
struct PendingCandidate {
    candidate_id: i64,
    identity: CandidateIdentity,
    stable_since_ms: i64,
    retry_attempt: u8,
    terminal_failure: bool,
}

#[derive(Debug)]
pub struct AutomaticObserver {
    enabled: bool,
    pending: Option<PendingCandidate>,
    observed_directory_identity: Option<String>,
    status: AutomaticObserverStatus,
}

impl AutomaticObserver {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            pending: None,
            observed_directory_identity: None,
            status: status_for(
                enabled,
                if enabled {
                    AutomaticObserverPhase::NotConfigured
                } else {
                    AutomaticObserverPhase::Disabled
                },
                None,
                0,
                None,
                None,
                None,
            ),
        }
    }

    pub fn status(&self) -> AutomaticObserverStatus {
        self.status.clone()
    }

    pub fn set_enabled(&mut self, enabled: bool, directory_configured: bool) {
        self.enabled = enabled;
        self.pending = None;
        self.observed_directory_identity = None;
        self.status = self.next_status(
            if !enabled {
                AutomaticObserverPhase::Disabled
            } else if directory_configured {
                AutomaticObserverPhase::Watching
            } else {
                AutomaticObserverPhase::NotConfigured
            },
            None,
            0,
            None,
        );
    }

    pub fn record_observation(&mut self, file_name: &str, observed_at_ms: i64) {
        self.pending = None;
        self.status = status_for(
            self.enabled,
            AutomaticObserverPhase::Observed,
            Some(file_name.to_owned()),
            0,
            None,
            Some(file_name.to_owned()),
            Some(observed_at_ms),
        );
    }

    #[cfg(test)]
    pub fn poll(
        &mut self,
        storage: &ObservatoryStorage,
        directory: Option<&Path>,
        observed_at_ms: i64,
        discovery_source: RecorderDiscoverySource,
    ) -> Result<AutomaticObservationUpdate, ObservatoryError> {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin()?;
        self.poll_with_profile(
            storage,
            directory,
            observed_at_ms,
            discovery_source,
            &profile,
        )
    }

    pub fn poll_with_profile(
        &mut self,
        storage: &ObservatoryStorage,
        directory: Option<&Path>,
        observed_at_ms: i64,
        discovery_source: RecorderDiscoverySource,
        profile: &ResolvedCompatibilityProfile,
    ) -> Result<AutomaticObservationUpdate, ObservatoryError> {
        if !self.enabled {
            self.set_enabled(false, directory.is_some());
            return Ok(self.update(None));
        }
        let Some(directory) = directory.filter(|path| path.is_dir()) else {
            self.pending = None;
            self.status = self.next_status(AutomaticObserverPhase::NotConfigured, None, 0, None);
            return Ok(self.update(None));
        };
        storage.note_recorder_scan(observed_at_ms)?;
        let candidates = save_candidates(directory)?;
        if candidates.is_empty() {
            self.pending = None;
            self.status = self.next_status(AutomaticObserverPhase::Watching, None, 0, None);
            return Ok(self.update(None));
        }

        let directory_identity = candidates[0].directory_identity.clone();
        let directory_changed =
            self.observed_directory_identity.as_deref() != Some(&directory_identity);
        let initial_scan =
            directory_changed && !storage.recorder_directory_is_initialised(&directory_identity)?;
        if directory_changed {
            self.pending = None;
            self.observed_directory_identity = Some(directory_identity.clone());
        }

        let mut registered = Vec::with_capacity(candidates.len());
        let baseline_limit = candidates.len().saturating_sub(1);
        for (index, candidate) in candidates.iter().cloned().enumerate() {
            let source = if initial_scan {
                RecorderDiscoverySource::InitialScan
            } else {
                discovery_source
            };
            let mut ledger = storage.discover_recorder_candidate(
                &candidate.directory_identity,
                &candidate.file_name,
                candidate.file_size,
                candidate.modified_ms,
                observed_at_ms,
                source,
            )?;
            if initial_scan && index < baseline_limit && !ledger.status.is_terminal() {
                storage.supersede_recorder_candidate(ledger.candidate_id, observed_at_ms)?;
                ledger.status = RecorderCandidateStatus::Superseded;
            }
            if !ledger.status.is_terminal()
                && let Some(payload_hash) = storage.file_observation_payload_hash(
                    &candidate.directory_identity,
                    &candidate.file_name,
                    candidate.file_size,
                    candidate.modified_ms,
                )?
            {
                storage.complete_recorder_candidate(
                    ledger.candidate_id,
                    ImportOutcome::Duplicate,
                    &payload_hash,
                    observed_at_ms,
                )?;
                ledger.status = RecorderCandidateStatus::Duplicate;
            }
            registered.push((candidate, ledger));
        }
        storage.supersede_unseen_recorder_candidates(&directory_identity, observed_at_ms)?;
        if initial_scan {
            storage.mark_recorder_directory_initialised(&directory_identity, observed_at_ms)?;
        }

        if let Some(pending) = self.pending.as_ref() {
            let still_present = registered
                .iter()
                .any(|(candidate, _)| candidate == &pending.identity);
            let newer_candidate_waits = registered.iter().any(|(candidate, ledger)| {
                !ledger.status.is_terminal() && candidate != &pending.identity
            });
            if !still_present || (pending.terminal_failure && newer_candidate_waits) {
                if !still_present {
                    storage.supersede_recorder_candidate(pending.candidate_id, observed_at_ms)?;
                }
                self.pending = None;
            }
        }

        let (candidate, ledger) = if let Some(pending) = self.pending.as_ref() {
            registered
                .iter()
                .find(|(candidate, _)| candidate == &pending.identity)
                .cloned()
                .expect("a present pending candidate has a ledger record")
        } else {
            let Some(candidate) = registered
                .iter()
                .find(|(_, ledger)| !ledger.status.is_terminal())
                .cloned()
            else {
                self.status = self.next_status(
                    AutomaticObserverPhase::Watching,
                    candidates
                        .last()
                        .map(|candidate| candidate.file_name.clone()),
                    0,
                    None,
                );
                return Ok(self.update(None));
            };
            candidate
        };

        let candidate_changed = self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.identity != candidate);
        if candidate_changed {
            storage.mark_recorder_candidate_stabilising(ledger.candidate_id)?;
            self.pending = Some(PendingCandidate {
                candidate_id: ledger.candidate_id,
                identity: candidate.clone(),
                stable_since_ms: observed_at_ms,
                retry_attempt: ledger.attempt_count.min(u8::MAX as u32) as u8,
                terminal_failure: false,
            });
            self.status = self.next_status(
                AutomaticObserverPhase::WaitingForStability,
                Some(candidate.file_name),
                ledger.attempt_count.min(u8::MAX as u32) as u8,
                None,
            );
            return Ok(self.update(None));
        }

        let pending = self
            .pending
            .as_ref()
            .expect("the matching candidate was established above");
        if pending.terminal_failure {
            return Ok(self.update(None));
        }
        if observed_at_ms - pending.stable_since_ms < STABLE_WINDOW_MS {
            self.status = self.next_status(
                AutomaticObserverPhase::WaitingForStability,
                Some(candidate.file_name),
                pending.retry_attempt,
                None,
            );
            return Ok(self.update(None));
        }

        storage.mark_recorder_candidate_ready(ledger.candidate_id, observed_at_ms)?;
        let attempt = storage
            .mark_recorder_candidate_reading(ledger.candidate_id, observed_at_ms)?
            .min(u8::MAX as u32) as u8;
        match inspect_save_archive(&candidate.path, profile) {
            Ok(inspection) => {
                let inserted = storage.save_inspection(&inspection)?;
                let dataset = storage.load_dataset(&inspection.interpretation_id)?;
                let outcome = if inserted {
                    ImportOutcome::Imported
                } else {
                    ImportOutcome::Duplicate
                };
                storage.complete_recorder_candidate(
                    ledger.candidate_id,
                    outcome,
                    &inspection.interpretation_id,
                    observed_at_ms,
                )?;
                let import_result = ObservationImportResult { outcome, dataset };
                self.record_observation(&candidate.file_name, observed_at_ms);
                Ok(self.update(Some(import_result)))
            }
            Err(error) => {
                let error_code = error.code().to_owned();
                let pending = self
                    .pending
                    .as_mut()
                    .expect("the matching candidate was established above");
                pending.retry_attempt = attempt;
                let retry = retryable(&error) && attempt < MAX_RETRY_ATTEMPTS;
                pending.terminal_failure = !retry;
                pending.stable_since_ms = observed_at_ms;
                storage.fail_recorder_candidate(
                    ledger.candidate_id,
                    retry,
                    &error_code,
                    observed_at_ms,
                )?;
                self.status = self.next_status(
                    if retry {
                        AutomaticObserverPhase::Retrying
                    } else {
                        AutomaticObserverPhase::Failed
                    },
                    Some(candidate.file_name),
                    attempt,
                    Some(error_code),
                );
                Ok(self.update(None))
            }
        }
    }

    fn update(&self, import_result: Option<ObservationImportResult>) -> AutomaticObservationUpdate {
        AutomaticObservationUpdate {
            status: self.status.clone(),
            import_result,
        }
    }

    fn next_status(
        &self,
        phase: AutomaticObserverPhase,
        candidate_file_name: Option<String>,
        retry_attempt: u8,
        error_code: Option<String>,
    ) -> AutomaticObserverStatus {
        status_for(
            self.enabled,
            phase,
            candidate_file_name,
            retry_attempt,
            error_code,
            self.status.last_observed_file_name.clone(),
            self.status.last_observed_at_ms,
        )
    }
}

fn status_for(
    enabled: bool,
    phase: AutomaticObserverPhase,
    candidate_file_name: Option<String>,
    retry_attempt: u8,
    error_code: Option<String>,
    last_observed_file_name: Option<String>,
    last_observed_at_ms: Option<i64>,
) -> AutomaticObserverStatus {
    AutomaticObserverStatus {
        enabled,
        phase,
        candidate_file_name,
        retry_attempt,
        error_code,
        last_observed_file_name,
        last_observed_at_ms,
    }
}

fn retryable(error: &ObservatoryError) -> bool {
    matches!(
        error,
        ObservatoryError::InvalidSaveCandidate
            | ObservatoryError::SaveChangedDuringRead
            | ObservatoryError::InvalidArchive
            | ObservatoryError::MissingStatsPayload
    )
}

fn save_candidates(directory: &Path) -> Result<Vec<CandidateIdentity>, ObservatoryError> {
    let directory_identity = directory_identity(directory)?;
    let mut candidates = fs::read_dir(directory)
        .map_err(|_| ObservatoryError::InvalidDirectory)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let path = entry.path();
            (metadata.is_file() && has_zip_extension(&path)).then(|| CandidateIdentity {
                path,
                directory_identity: directory_identity.clone(),
                file_name: entry.file_name().to_string_lossy().into_owned(),
                file_size: metadata.len(),
                modified_ms: system_time_ms(metadata.modified().unwrap_or(UNIX_EPOCH)),
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.modified_ms
            .cmp(&right.modified_ms)
            .then_with(|| left.file_name.cmp(&right.file_name))
    });
    Ok(candidates)
}

pub(crate) fn latest_save_candidate_path(directory: &Path) -> Result<PathBuf, ObservatoryError> {
    save_candidates(directory)?
        .pop()
        .map(|candidate| candidate.path)
        .ok_or(ObservatoryError::NoSaveCandidate)
}

fn has_zip_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn system_time_ms(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    use super::{AutomaticObserver, AutomaticObserverPhase};
    use crate::model::{RecorderCandidateStatus, RecorderDiscoverySource};
    use crate::storage::ObservatoryStorage;

    #[test]
    fn disabling_clears_pending_work_without_erasing_last_success() {
        let mut observer = AutomaticObserver::new(true);
        observer.record_observation("safe.zip", 42);
        observer.set_enabled(false, true);

        let status = observer.status();
        assert_eq!(status.phase, AutomaticObserverPhase::Disabled);
        assert_eq!(status.last_observed_file_name.as_deref(), Some("safe.zip"));
        assert_eq!(status.last_observed_at_ms, Some(42));
    }

    #[test]
    fn observes_a_candidate_only_after_two_stable_polls() {
        let directory = tempdir().expect("temporary directory");
        let storage = ObservatoryStorage::initialise(directory.path().join("observer.sqlite3"))
            .expect("storage");
        write_save(&directory.path().join("new-save.zip"));
        let mut observer = AutomaticObserver::new(true);

        let first = observer
            .poll(
                &storage,
                Some(directory.path()),
                10_000,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("first poll");
        assert_eq!(
            first.status.phase,
            AutomaticObserverPhase::WaitingForStability
        );
        assert!(first.import_result.is_none());
        let too_soon = observer
            .poll(
                &storage,
                Some(directory.path()),
                11_000,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("second poll");
        assert_eq!(
            too_soon.status.phase,
            AutomaticObserverPhase::WaitingForStability
        );
        let observed = observer
            .poll(
                &storage,
                Some(directory.path()),
                11_500,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("stable poll");
        assert_eq!(observed.status.phase, AutomaticObserverPhase::Observed);
        assert!(observed.import_result.is_some());
        assert_eq!(storage.distinct_state_count().expect("state count"), 1);
        assert_eq!(storage.file_observation_count().expect("file count"), 1);

        let known = observer
            .poll(
                &storage,
                Some(directory.path()),
                13_000,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("known candidate");
        assert_eq!(known.status.phase, AutomaticObserverPhase::Watching);
        assert!(known.import_result.is_none());
        assert_eq!(storage.file_observation_count().expect("file count"), 1);
    }

    #[test]
    fn first_scan_baselines_older_files_and_observes_only_the_newest() {
        let directory = tempdir().expect("temporary directory");
        let storage = ObservatoryStorage::initialise(directory.path().join("observer.sqlite3"))
            .expect("storage");
        write_save(&directory.path().join("001-old.zip"));
        write_save(&directory.path().join("002-middle.zip"));
        write_save(&directory.path().join("003-newest.zip"));
        let mut observer = AutomaticObserver::new(true);

        let first = observer
            .poll(
                &storage,
                Some(directory.path()),
                15_000,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("initial scan");
        assert_eq!(
            first.status.candidate_file_name.as_deref(),
            Some("003-newest.zip")
        );
        observer
            .poll(
                &storage,
                Some(directory.path()),
                16_500,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("newest observed");

        let health = storage
            .load_recorder_health(observer.status())
            .expect("health");
        assert_eq!(health.completed_count, 1);
        assert_eq!(
            health.last_completed_file_name.as_deref(),
            Some("003-newest.zip")
        );
        assert_eq!(health.last_completed_at_ms, Some(16_500));
        assert_eq!(health.last_processing_latency_ms, Some(1_500));
        assert_eq!(
            health
                .latest_entries
                .iter()
                .filter(|entry| entry.status == RecorderCandidateStatus::Superseded)
                .count(),
            2
        );
        assert_eq!(storage.file_observation_count().expect("file count"), 1);
    }

    #[test]
    fn retries_an_incomplete_candidate_and_recovers_after_it_changes() {
        let directory = tempdir().expect("temporary directory");
        let storage = ObservatoryStorage::initialise(directory.path().join("observer.sqlite3"))
            .expect("storage");
        let path = directory.path().join("new-save.zip");
        std::fs::write(&path, b"incomplete").expect("partial candidate");
        let mut observer = AutomaticObserver::new(true);

        observer
            .poll(
                &storage,
                Some(directory.path()),
                20_000,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("waiting poll");
        let retry = observer
            .poll(
                &storage,
                Some(directory.path()),
                21_500,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("retry poll");
        assert_eq!(retry.status.phase, AutomaticObserverPhase::Retrying);
        assert_eq!(retry.status.retry_attempt, 1);

        write_save(&path);
        let changed = observer
            .poll(
                &storage,
                Some(directory.path()),
                22_000,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("changed candidate");
        assert_eq!(
            changed.status.phase,
            AutomaticObserverPhase::WaitingForStability
        );
        let observed = observer
            .poll(
                &storage,
                Some(directory.path()),
                23_500,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("recovered candidate");
        assert_eq!(observed.status.phase, AutomaticObserverPhase::Observed);
        assert!(observed.import_result.is_some());
    }

    #[test]
    fn queues_each_new_candidate_in_creation_order() {
        let directory = tempdir().expect("temporary directory");
        let storage = ObservatoryStorage::initialise(directory.path().join("observer.sqlite3"))
            .expect("storage");
        write_save(&directory.path().join("baseline.zip"));
        let mut observer = AutomaticObserver::new(true);
        observer
            .poll(
                &storage,
                Some(directory.path()),
                30_000,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("baseline waiting");
        observer
            .poll(
                &storage,
                Some(directory.path()),
                31_500,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("baseline observed");

        write_save(&directory.path().join("second.zip"));
        write_save(&directory.path().join("third.zip"));
        let second_wait = observer
            .poll(
                &storage,
                Some(directory.path()),
                32_000,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("second waiting");
        assert_eq!(
            second_wait.status.candidate_file_name.as_deref(),
            Some("second.zip")
        );
        observer
            .poll(
                &storage,
                Some(directory.path()),
                33_500,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("second observed");
        let third_wait = observer
            .poll(
                &storage,
                Some(directory.path()),
                34_000,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("third waiting");
        assert_eq!(
            third_wait.status.candidate_file_name.as_deref(),
            Some("third.zip")
        );
        observer
            .poll(
                &storage,
                Some(directory.path()),
                35_500,
                RecorderDiscoverySource::Reconciliation,
            )
            .expect("third observed");

        assert_eq!(storage.file_observation_count().expect("file count"), 3);
    }

    fn write_save(path: &Path) {
        let file = File::create(path).expect("fixture archive");
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                "stats.ini",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .expect("stats entry");
        archive
            .write_all(include_bytes!(
                "../fixtures/current-city.receiver-stats.txt"
            ))
            .expect("stats content");
        archive.finish().expect("finish archive");
    }
}
