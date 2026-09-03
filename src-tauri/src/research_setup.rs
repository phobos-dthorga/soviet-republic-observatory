//! Bounded native setup and build service for the optional GPL research companion.
//!
//! The only network operation retrieves source from one reviewed upstream commit.
//! It never downloads binaries or accepts an arbitrary command. Game-folder
//! preparation and live launch use separate, typed consent boundaries.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::diagnostics;
use crate::error::ObservatoryError;
use crate::research_source_download::{
    DownloadedResearchSource, ResearchSourceDownloadPhase as DownloadPhase,
    download_reviewed_source, reviewed_session_source_is_available,
};
use crate::storage::{StoredResearchSetup, now_ms};

pub const RESEARCH_NOTICE_REVISION: u32 = 4;
pub const REVIEWED_TESMIO_REVISION: &str = "3baa141f9f08921aea9c95f0a400289cabd9960a";
pub(crate) const REVIEWED_PLUGIN_HEADER_HASH: &str =
    "d886ac6550dd84031ee2ed3afab13a7f75e4ddf920d23183b93395440d3cff49";
pub(crate) const REVIEWED_API_HEADER_HASH: &str =
    "33c9fae4acb1041708c7b1b4675b0eb4740f0af737e7a1968c0acb0c325fff3c";
const MAX_HEADER_BYTES: u64 = 256 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LOG_LINES: usize = 80;
const MAX_LOG_LINE_CHARS: usize = 240;
const PROBE_BUILD_PROVENANCE: &str = "observatory_probe.provenance.json";
const PROBE_SOURCE_CONTRACT_FILES: [&str; 5] = [
    "build.ps1",
    "observatory_probe.cpp",
    "observatory_probe.ini",
    "COPYING",
    "verify-observation-only.ps1",
];
const MANAGED_SESSION_DIRECTORY: &str = "observatory";
const SESSION_MANIFEST: &str = "observatory-install.json";
const MAX_SESSION_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024;
const SESSION_FILES: [&str; 6] = [
    "tesmioloader.dll",
    "tesmiolauncher.exe",
    "tesmioloader.ini",
    "COPYING",
    "plugins/observatory_probe.dll",
    "plugins/observatory_probe.ini",
];

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchCheckoutState {
    NotSelected,
    Missing,
    Reviewed,
    Unsupported,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchArtifactState {
    Absent,
    Unrecorded,
    Verified,
    Changed,
    Missing,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSourceOrigin {
    ManualCheckout,
    ObservatoryDownloaded,
}

impl ResearchSourceOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ManualCheckout => "manual_checkout",
            Self::ObservatoryDownloaded => "observatory_downloaded",
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSourceDownloadState {
    Idle,
    Running,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSourceDownloadPhase {
    Idle,
    Connecting,
    Downloading,
    CheckingArchive,
    Installing,
    Verifying,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResearchSourceDownloadProgress {
    pub task_id: String,
    pub run_id: String,
    pub state: ResearchSourceDownloadState,
    pub phase: ResearchSourceDownloadPhase,
    pub progress_percent: Option<u8>,
    pub transferred_bytes: u64,
    pub expected_bytes: Option<u64>,
    pub started_at_ms: Option<i64>,
    pub updated_at_ms: Option<i64>,
    pub current_item: Option<String>,
    pub error_code: Option<String>,
}

impl Default for ResearchSourceDownloadProgress {
    fn default() -> Self {
        Self {
            task_id: "research_source_download".to_owned(),
            run_id: "not_started".to_owned(),
            state: ResearchSourceDownloadState::Idle,
            phase: ResearchSourceDownloadPhase::Idle,
            progress_percent: None,
            transferred_bytes: 0,
            expected_bytes: None,
            started_at_ms: None,
            updated_at_ms: None,
            current_item: None,
            error_code: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ResearchSetupStatus {
    pub notice_revision: u32,
    pub notice_accepted: bool,
    pub source_available: bool,
    pub compiler_available: bool,
    pub checkout_state: ResearchCheckoutState,
    pub source_origin: Option<ResearchSourceOrigin>,
    pub checkout_name: Option<String>,
    pub reviewed_tesmio_revision: String,
    pub probe_built: bool,
    pub artifact_state: ResearchArtifactState,
    pub probe_content_hash: Option<String>,
    pub probe_size_bytes: Option<u64>,
    pub output_display_path: Option<String>,
    pub last_built_at_ms: Option<i64>,
    pub can_build: bool,
    pub can_download: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub progress: ResearchBuildProgress,
    pub download_progress: ResearchSourceDownloadProgress,
    pub session: ResearchSessionStatus,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSessionState {
    GameNotConfigured,
    PrerequisitesRequired,
    ReadyToPrepare,
    Prepared,
    ReportAvailable,
    Invalid,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSessionTaskState {
    Idle,
    Running,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSessionPhase {
    Idle,
    Preflight,
    BuildingHost,
    Installing,
    Verifying,
    CheckingSetup,
    StartingGame,
    LoadingTesmio,
    GameResumed,
    WaitingForReport,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSessionLaunchState {
    Idle,
    CheckingSetup,
    StartingGame,
    LoadingTesmio,
    GameResumed,
    WaitingForReport,
    ReportReady,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResearchSessionProgress {
    pub task_id: String,
    pub run_id: String,
    pub state: ResearchSessionTaskState,
    pub phase: ResearchSessionPhase,
    pub progress_percent: Option<u8>,
    pub started_at_ms: Option<i64>,
    pub updated_at_ms: Option<i64>,
    pub current_item: Option<String>,
    pub log_lines: Vec<String>,
    pub error_code: Option<String>,
}

impl Default for ResearchSessionProgress {
    fn default() -> Self {
        Self {
            task_id: "research_session_preparation".to_owned(),
            run_id: "not_started".to_owned(),
            state: ResearchSessionTaskState::Idle,
            phase: ResearchSessionPhase::Idle,
            progress_percent: None,
            started_at_ms: None,
            updated_at_ms: None,
            current_item: None,
            log_lines: Vec::new(),
            error_code: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ResearchSessionStatus {
    pub state: ResearchSessionState,
    pub launch_state: ResearchSessionLaunchState,
    pub game_configured: bool,
    pub reviewed_loader_source_available: bool,
    pub probe_ready: bool,
    pub report_snapshot_count: u32,
    pub report_collection_stage: Option<String>,
    pub people_readings_ready: bool,
    pub resource_readings_ready: bool,
    pub environment_readings_ready: bool,
    pub facility_contract_version: Option<u32>,
    pub last_report_at_ms: Option<i64>,
    pub managed_folder: String,
    pub can_prepare: bool,
    pub can_launch: bool,
    pub writes_game_directory: bool,
    pub writes_save_data: bool,
    pub changes_running_game_memory: bool,
    pub progress: ResearchSessionProgress,
}

struct SessionBuildCleanup(PathBuf);

impl Drop for SessionBuildCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResearchSessionManifest {
    schema_version: u32,
    reviewed_revision: String,
    installed_at_ms: i64,
    files: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchBuildState {
    Idle,
    Running,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchBuildPhase {
    Idle,
    Preflight,
    Toolchain,
    Compiling,
    Verifying,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResearchBuildProgress {
    pub task_id: String,
    pub run_id: String,
    pub state: ResearchBuildState,
    pub phase: ResearchBuildPhase,
    pub progress_percent: Option<u8>,
    pub started_at_ms: Option<i64>,
    pub updated_at_ms: Option<i64>,
    pub current_item: Option<String>,
    pub log_lines: Vec<String>,
    pub error_code: Option<String>,
    pub failed_stage: Option<String>,
    pub compiler_exit_code: Option<i32>,
    pub remediation_code: Option<String>,
}

impl Default for ResearchBuildProgress {
    fn default() -> Self {
        Self {
            task_id: "research_probe_build".to_owned(),
            run_id: "not_started".to_owned(),
            state: ResearchBuildState::Idle,
            phase: ResearchBuildPhase::Idle,
            progress_percent: None,
            started_at_ms: None,
            updated_at_ms: None,
            current_item: None,
            log_lines: Vec::new(),
            error_code: None,
            failed_stage: None,
            compiler_exit_code: None,
            remediation_code: None,
        }
    }
}

#[derive(Debug)]
pub struct ResearchSetupService {
    source_root: Option<PathBuf>,
    managed_source_root: PathBuf,
    progress: Mutex<ResearchBuildProgress>,
    download_progress: Mutex<ResearchSourceDownloadProgress>,
    session_progress: Mutex<ResearchSessionProgress>,
    building: AtomicBool,
    downloading: AtomicBool,
    preparing_session: AtomicBool,
    launching_session: AtomicBool,
}

impl ResearchSetupService {
    pub fn discover(data_directory: &Path) -> Self {
        Self {
            source_root: discover_source_root(),
            managed_source_root: data_directory
                .join("research")
                .join("tesmioloader-reviewed"),
            progress: Mutex::new(ResearchBuildProgress::default()),
            download_progress: Mutex::new(ResearchSourceDownloadProgress::default()),
            session_progress: Mutex::new(ResearchSessionProgress::default()),
            building: AtomicBool::new(false),
            downloading: AtomicBool::new(false),
            preparing_session: AtomicBool::new(false),
            launching_session: AtomicBool::new(false),
        }
    }

    pub fn progress(&self) -> ResearchBuildProgress {
        self.progress
            .lock()
            .map(|progress| progress.clone())
            .unwrap_or_default()
    }

    pub fn download_progress(&self) -> ResearchSourceDownloadProgress {
        self.download_progress
            .lock()
            .map(|progress| progress.clone())
            .unwrap_or_default()
    }

    pub fn session_progress(&self) -> ResearchSessionProgress {
        self.session_progress
            .lock()
            .map(|progress| progress.clone())
            .unwrap_or_default()
    }

    pub fn validate_checkout(&self, path: &Path) -> Result<PathBuf, ObservatoryError> {
        let metadata =
            fs::symlink_metadata(path).map_err(|_| ObservatoryError::InvalidResearchCheckout)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(ObservatoryError::InvalidResearchCheckout);
        }
        let canonical = canonical_checkout_path(path)?;
        if checkout_state(Some(&canonical)) != ResearchCheckoutState::Reviewed {
            return Err(ObservatoryError::InvalidResearchCheckout);
        }
        Ok(canonical)
    }

    pub fn status(
        &self,
        stored: &StoredResearchSetup,
        game_media_directory: Option<&Path>,
    ) -> ResearchSetupStatus {
        let source_available = self.source_root.as_deref().is_some_and(source_ready);
        let compiler_available = compiler_ready();
        let checkout_state = match stored.tesmio_checkout_path.as_deref() {
            None => ResearchCheckoutState::NotSelected,
            Some(path) if !path.is_dir() => ResearchCheckoutState::Missing,
            Some(path) if self.validate_checkout(path).is_ok() => ResearchCheckoutState::Reviewed,
            Some(_) => ResearchCheckoutState::Unsupported,
        };
        let notice_accepted = stored.accepted_notice_revision == RESEARCH_NOTICE_REVISION;
        let source_origin = match stored.tesmio_source_origin.as_deref() {
            Some("observatory_downloaded") => Some(ResearchSourceOrigin::ObservatoryDownloaded),
            Some("manual_checkout") => Some(ResearchSourceOrigin::ManualCheckout),
            _ if stored.tesmio_checkout_path.is_some() => {
                Some(ResearchSourceOrigin::ManualCheckout)
            }
            _ => None,
        };
        let artifact = self.source_root.as_deref().and_then(inspect_artifact);
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();
        if !notice_accepted {
            blockers.push("notice_required".to_owned());
        }
        if !source_available {
            blockers.push("probe_source_unavailable".to_owned());
        }
        if !compiler_available {
            blockers.push("compiler_unavailable".to_owned());
        }
        match checkout_state {
            ResearchCheckoutState::NotSelected => blockers.push("checkout_required".to_owned()),
            ResearchCheckoutState::Missing => blockers.push("checkout_missing".to_owned()),
            ResearchCheckoutState::Unsupported => blockers.push("checkout_unsupported".to_owned()),
            ResearchCheckoutState::Reviewed => {}
        }
        if source_origin == Some(ResearchSourceOrigin::ObservatoryDownloaded)
            && (checkout_state != ResearchCheckoutState::Reviewed
                || !stored
                    .tesmio_checkout_path
                    .as_deref()
                    .is_some_and(reviewed_session_source_is_available))
        {
            warnings.push("downloaded_source_needs_repair".to_owned());
        }
        let is_building = self.building.load(Ordering::Acquire);
        if is_building {
            blockers.push("build_running".to_owned());
        }
        let artifact_state = match (stored.last_probe_hash.as_deref(), artifact.as_ref()) {
            (None, None) => ResearchArtifactState::Absent,
            (None, Some(_)) => ResearchArtifactState::Unrecorded,
            (Some(recorded), Some(current)) if recorded == current.hash => {
                ResearchArtifactState::Verified
            }
            (Some(_), Some(_)) => ResearchArtifactState::Changed,
            (Some(_), None) => ResearchArtifactState::Missing,
        };
        match artifact_state {
            ResearchArtifactState::Unrecorded => {
                warnings.push("artifact_not_recorded_by_assistant".to_owned())
            }
            ResearchArtifactState::Changed => {
                warnings.push("artifact_changed_outside_assistant".to_owned())
            }
            ResearchArtifactState::Missing => {
                warnings.push("artifact_missing_after_verified_build".to_owned())
            }
            ResearchArtifactState::Absent | ResearchArtifactState::Verified => {}
        }
        let session = self.session_status(
            stored,
            game_media_directory,
            notice_accepted,
            source_available,
            compiler_available,
            artifact_state == ResearchArtifactState::Verified,
        );
        ResearchSetupStatus {
            notice_revision: RESEARCH_NOTICE_REVISION,
            notice_accepted,
            source_available,
            compiler_available,
            checkout_state,
            source_origin,
            checkout_name: stored
                .tesmio_checkout_path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned()),
            reviewed_tesmio_revision: REVIEWED_TESMIO_REVISION.to_owned(),
            probe_built: artifact_state == ResearchArtifactState::Verified,
            artifact_state,
            probe_content_hash: artifact.as_ref().map(|artifact| artifact.hash.clone()),
            probe_size_bytes: artifact.as_ref().map(|artifact| artifact.size),
            output_display_path: artifact
                .as_ref()
                .map(|_| "research/tesmioloader-probe/build/observatory_probe.dll".to_owned()),
            last_built_at_ms: stored.last_built_at_ms,
            can_build: blockers.is_empty(),
            can_download: notice_accepted && !self.downloading.load(Ordering::Acquire),
            blockers,
            warnings,
            progress: self.progress(),
            download_progress: self.download_progress(),
            session,
        }
    }

    fn session_status(
        &self,
        stored: &StoredResearchSetup,
        game_media_directory: Option<&Path>,
        notice_accepted: bool,
        source_available: bool,
        compiler_available: bool,
        probe_ready: bool,
    ) -> ResearchSessionStatus {
        let reviewed_loader_source_available = stored
            .tesmio_checkout_path
            .as_deref()
            .is_some_and(reviewed_session_source_is_available);
        let paths = game_media_directory.and_then(managed_session_paths);
        let game_configured = paths.is_some();
        let preparing = self.preparing_session.load(Ordering::Acquire);
        let prerequisites_ready = notice_accepted
            && source_available
            && compiler_available
            && probe_ready
            && reviewed_loader_source_available
            && game_configured;
        let mut report_snapshot_count = 0;
        let mut report_collection_stage = None;
        let mut people_readings_ready = false;
        let mut resource_readings_ready = false;
        let mut environment_readings_ready = false;
        let mut facility_contract_version = None;
        let mut last_report_at_ms = None;
        let (state, installed) = match paths.as_ref() {
            None => (ResearchSessionState::GameNotConfigured, false),
            Some(paths)
                if managed_session_is_valid(
                    &paths.session_root,
                    &paths.game_executable,
                    stored.last_probe_hash.as_deref(),
                ) =>
            {
                let report = crate::tesmio_probe::inspect(game_media_directory);
                let report_available = matches!(
                    report,
                    crate::model::TesmioProbeStatus {
                        state: crate::model::TesmioProbeState::Available
                            | crate::model::TesmioProbeState::Warning,
                        ..
                    }
                );
                if report_available {
                    report_snapshot_count = report.snapshot_count;
                    report_collection_stage = report.collection_stage;
                    people_readings_ready = report.people_readings_ready;
                    resource_readings_ready = report.resource_readings_ready;
                    environment_readings_ready = report.environment_readings_ready;
                    facility_contract_version = report.facility_contract_version;
                    last_report_at_ms = report.last_report_at_ms;
                }
                (
                    if report_available {
                        ResearchSessionState::ReportAvailable
                    } else {
                        ResearchSessionState::Prepared
                    },
                    true,
                )
            }
            Some(paths) if paths.session_root.exists() => (ResearchSessionState::Invalid, false),
            Some(_) if prerequisites_ready => (ResearchSessionState::ReadyToPrepare, false),
            Some(_) => (ResearchSessionState::PrerequisitesRequired, false),
        };
        let progress = self.session_progress();
        let report_is_current_launch = progress.task_id != "research_session_launch"
            || progress.started_at_ms.is_some_and(|started| {
                last_report_at_ms.is_some_and(|reported| reported >= started)
            });
        let launch_state = if report_snapshot_count > 0 && report_is_current_launch {
            ResearchSessionLaunchState::ReportReady
        } else {
            match progress.phase {
                ResearchSessionPhase::CheckingSetup => ResearchSessionLaunchState::CheckingSetup,
                ResearchSessionPhase::StartingGame => ResearchSessionLaunchState::StartingGame,
                ResearchSessionPhase::LoadingTesmio => ResearchSessionLaunchState::LoadingTesmio,
                ResearchSessionPhase::GameResumed => ResearchSessionLaunchState::GameResumed,
                ResearchSessionPhase::WaitingForReport => {
                    ResearchSessionLaunchState::WaitingForReport
                }
                ResearchSessionPhase::Failed if progress.task_id == "research_session_launch" => {
                    ResearchSessionLaunchState::Failed
                }
                _ => ResearchSessionLaunchState::Idle,
            }
        };
        ResearchSessionStatus {
            state,
            launch_state,
            game_configured,
            reviewed_loader_source_available,
            probe_ready,
            report_snapshot_count,
            report_collection_stage,
            people_readings_ready,
            resource_readings_ready,
            environment_readings_ready,
            facility_contract_version,
            last_report_at_ms,
            managed_folder: "W&R/tesmioloader/observatory".to_owned(),
            can_prepare: prerequisites_ready && !preparing,
            can_launch: installed
                && notice_accepted
                && !preparing
                && !self.launching_session.load(Ordering::Acquire),
            writes_game_directory: true,
            writes_save_data: false,
            changes_running_game_memory: true,
            progress,
        }
    }

    pub fn prepare_observation_session(
        &self,
        app: &AppHandle,
        stored: &StoredResearchSetup,
        game_media_directory: Option<&Path>,
        game_directory_write_confirmed: bool,
    ) -> Result<(), ObservatoryError> {
        if !game_directory_write_confirmed {
            return Err(ObservatoryError::ResearchSessionConsentRequired);
        }
        if self
            .preparing_session
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ObservatoryError::CriticalTaskBusy);
        }
        let result = self.prepare_observation_session_inner(app, stored, game_media_directory);
        self.preparing_session.store(false, Ordering::Release);
        result
    }

    fn prepare_observation_session_inner(
        &self,
        app: &AppHandle,
        stored: &StoredResearchSetup,
        game_media_directory: Option<&Path>,
    ) -> Result<(), ObservatoryError> {
        let started = now_ms();
        self.update_session_progress(
            app,
            ResearchSessionProgress {
                task_id: "research_session_preparation".to_owned(),
                run_id: format!("research-session-{started}"),
                state: ResearchSessionTaskState::Running,
                phase: ResearchSessionPhase::Preflight,
                progress_percent: Some(10),
                started_at_ms: Some(started),
                updated_at_ms: Some(started),
                current_item: Some("consent_and_paths".to_owned()),
                log_lines: Vec::new(),
                error_code: None,
            },
        );
        if stored.accepted_notice_revision != RESEARCH_NOTICE_REVISION {
            return self.fail_session(app, ObservatoryError::ResearchNoticeRequired);
        }
        if !compiler_ready() {
            return self.fail_session(app, ObservatoryError::ResearchToolchainUnavailable);
        }
        let Some(source_root) = self
            .source_root
            .as_deref()
            .filter(|path| source_ready(path))
        else {
            return self.fail_session(app, ObservatoryError::ResearchSourceUnavailable);
        };
        let Some(checkout) = stored
            .tesmio_checkout_path
            .as_deref()
            .filter(|path| reviewed_session_source_is_available(path))
        else {
            return self.fail_session(app, ObservatoryError::InvalidResearchCheckout);
        };
        let Some(probe) = inspect_artifact(source_root)
            .filter(|artifact| stored.last_probe_hash.as_deref() == Some(artifact.hash.as_str()))
        else {
            return self.fail_session(app, ObservatoryError::ResearchSessionNotReady);
        };
        let Some(paths) = game_media_directory.and_then(managed_session_paths) else {
            return self.fail_session(app, ObservatoryError::InvalidGameDirectory);
        };
        if paths.session_root.exists() && !managed_session_owned(&paths.session_root) {
            return self.fail_session(app, ObservatoryError::ResearchSessionConflict);
        }
        if managed_session_is_valid(
            &paths.session_root,
            &paths.game_executable,
            stored.last_probe_hash.as_deref(),
        ) {
            self.complete_session_progress(app, "existing_checked_setup");
            return Ok(());
        }

        self.advance_session(
            app,
            ResearchSessionPhase::BuildingHost,
            35,
            "reviewed_tesmio_host",
        );
        let Some(build_parent) = self.managed_source_root.parent() else {
            return self.fail_session(app, ObservatoryError::ResearchSessionPreparationFailed);
        };
        let build_root = build_parent
            .join("observation-session-build")
            .join(format!("{}-{started}", std::process::id()));
        if fs::create_dir_all(&build_root).is_err() {
            return self.fail_session(app, ObservatoryError::ResearchSessionPreparationFailed);
        }
        let _build_cleanup = SessionBuildCleanup(build_root.clone());
        let Some(powershell) = find_powershell() else {
            return self.fail_session(app, ObservatoryError::ResearchToolchainUnavailable);
        };
        let script = source_root.join("build-observation-session.ps1");
        let output = Command::new(powershell)
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&script)
            .arg("-TesmioLoaderRoot")
            .arg(compiler_checkout_path(checkout))
            .arg("-OutputRoot")
            .arg(&build_root)
            .current_dir(source_root)
            .output();
        let Ok(output) = output else {
            return self.fail_session(app, ObservatoryError::ResearchSessionPreparationFailed);
        };
        let mut logs = sanitise_build_output(&output.stdout, &output.stderr, source_root, checkout);
        let private_build = build_root.to_string_lossy();
        for line in &mut logs {
            *line = line.replace(private_build.as_ref(), "<managed-session-build>");
        }
        self.set_session_logs(app, logs);
        if !output.status.success()
            || checked_artifact(
                &build_root.join("tesmioloader.dll"),
                MAX_SESSION_ARTIFACT_BYTES,
            )
            .is_none()
            || checked_artifact(
                &build_root.join("tesmiolauncher.exe"),
                MAX_SESSION_ARTIFACT_BYTES,
            )
            .is_none()
        {
            return self.fail_session(app, ObservatoryError::ResearchSessionPreparationFailed);
        }

        self.advance_session(
            app,
            ResearchSessionPhase::Installing,
            70,
            "isolated_game_folder",
        );
        if let Err(error) =
            install_managed_session(&paths, &build_root, source_root, checkout, &probe)
        {
            return self.fail_session(app, error);
        }
        self.advance_session(
            app,
            ResearchSessionPhase::Verifying,
            92,
            "read_only_contract",
        );
        if !managed_session_is_valid(
            &paths.session_root,
            &paths.game_executable,
            Some(&probe.hash),
        ) {
            return self.fail_session(app, ObservatoryError::ResearchSessionPreparationFailed);
        }
        self.complete_session_progress(app, "ready_for_confirmed_launch");
        diagnostics::record(
            "info",
            "research_session.prepared",
            "prepare_observation_only_session",
            "Prepared the isolated observation-only Tesmio session after explicit consent. No save file was read or changed.",
        );
        Ok(())
    }

    pub fn launch_observation_session(
        self: &Arc<Self>,
        app: &AppHandle,
        stored: &StoredResearchSetup,
        game_media_directory: Option<&Path>,
        running_game_memory_confirmed: bool,
    ) -> Result<(), ObservatoryError> {
        let started = now_ms();
        self.update_session_progress(
            app,
            ResearchSessionProgress {
                task_id: "research_session_launch".to_owned(),
                run_id: format!("research-launch-{started}"),
                state: ResearchSessionTaskState::Running,
                phase: ResearchSessionPhase::CheckingSetup,
                progress_percent: Some(10),
                started_at_ms: Some(started),
                updated_at_ms: Some(started),
                current_item: Some("checking_checked_setup".to_owned()),
                log_lines: Vec::new(),
                error_code: None,
            },
        );
        if !running_game_memory_confirmed {
            return self.fail_session(app, ObservatoryError::ResearchSessionConsentRequired);
        }
        if stored.accepted_notice_revision != RESEARCH_NOTICE_REVISION {
            return self.fail_session(app, ObservatoryError::ResearchNoticeRequired);
        }
        let Some(paths) = game_media_directory.and_then(managed_session_paths) else {
            return self.fail_session(app, ObservatoryError::InvalidGameDirectory);
        };
        if !managed_session_is_valid(
            &paths.session_root,
            &paths.game_executable,
            stored.last_probe_hash.as_deref(),
        ) {
            return self.fail_session(app, ObservatoryError::ResearchSessionNotReady);
        }
        if self
            .launching_session
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ObservatoryError::CriticalTaskBusy);
        }
        self.advance_session(app, ResearchSessionPhase::StartingGame, 30, "starting_wr");
        let mut command = Command::new(paths.session_root.join("tesmiolauncher.exe"));
        command
            .arg("--game")
            .arg(&paths.game_executable)
            .arg("--nogui")
            .current_dir(&paths.session_root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let child = match command.spawn() {
            Ok(child) => child,
            Err(_) => {
                self.launching_session.store(false, Ordering::Release);
                return self.fail_session(app, ObservatoryError::ResearchSessionLaunchFailed);
            }
        };
        self.advance_session(
            app,
            ResearchSessionPhase::LoadingTesmio,
            55,
            "loading_tesmio",
        );
        let service = Arc::clone(self);
        let app = app.clone();
        let session_root = paths.session_root.clone();
        let game_root = paths
            .game_executable
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        std::thread::Builder::new()
            .name("observatory-tesmio-launch".to_owned())
            .spawn(move || match child.wait_with_output() {
                Ok(output) if output.status.success() => {
                    service.launching_session.store(false, Ordering::Release);
                    let logs = sanitise_build_output(
                        &output.stdout,
                        &output.stderr,
                        &session_root,
                        &game_root,
                    );
                    service.set_session_logs(&app, logs);
                    service.advance_session(
                        &app,
                        ResearchSessionPhase::GameResumed,
                        80,
                        "game_resumed",
                    );
                    service.advance_session(
                        &app,
                        ResearchSessionPhase::WaitingForReport,
                        90,
                        "waiting_for_checked_report",
                    );
                    diagnostics::record(
                        "info",
                        "research_session.launcher_complete",
                        "launch_observation_only_session",
                        "The checked launcher resumed W&R and exited normally. Observatory is waiting for a report.",
                    );
                }
                Ok(output) => {
                    service.launching_session.store(false, Ordering::Release);
                    let logs = sanitise_build_output(
                        &output.stdout,
                        &output.stderr,
                        &session_root,
                        &game_root,
                    );
                    service.set_session_logs(&app, logs);
                    let _ = service.fail_session::<()>(
                        &app,
                        ObservatoryError::ResearchSessionLaunchFailed,
                    );
                }
                Err(_) => {
                    service.launching_session.store(false, Ordering::Release);
                    let _ = service.fail_session::<()>(
                        &app,
                        ObservatoryError::ResearchSessionLaunchFailed,
                    );
                }
            })
            .map_err(|_| {
                self.launching_session.store(false, Ordering::Release);
                ObservatoryError::ResearchSessionLaunchFailed
            })?;
        diagnostics::record(
            "info",
            "research_session.launch_requested",
            "launch_observation_only_session",
            "The player explicitly launched the checked observation-only session. Launch progress is continuing in the background; no save write was requested.",
        );
        Ok(())
    }

    fn advance_session(
        &self,
        app: &AppHandle,
        phase: ResearchSessionPhase,
        percent: u8,
        item: &str,
    ) {
        let mut progress = self.session_progress();
        progress.phase = phase;
        progress.progress_percent = Some(percent);
        progress.updated_at_ms = Some(now_ms());
        progress.current_item = Some(item.to_owned());
        self.update_session_progress(app, progress);
    }

    fn set_session_logs(&self, app: &AppHandle, logs: Vec<String>) {
        let mut progress = self.session_progress();
        progress.log_lines = logs;
        progress.updated_at_ms = Some(now_ms());
        self.update_session_progress(app, progress);
    }

    fn complete_session_progress(&self, app: &AppHandle, item: &str) {
        let mut progress = self.session_progress();
        progress.state = ResearchSessionTaskState::Complete;
        progress.phase = ResearchSessionPhase::Complete;
        progress.progress_percent = Some(100);
        progress.updated_at_ms = Some(now_ms());
        progress.current_item = Some(item.to_owned());
        self.update_session_progress(app, progress);
    }

    fn fail_session<T>(
        &self,
        app: &AppHandle,
        error: ObservatoryError,
    ) -> Result<T, ObservatoryError> {
        let mut progress = self.session_progress();
        let operation = if progress.task_id == "research_session_launch" {
            "launch_observation_only_session"
        } else {
            "prepare_observation_only_session"
        };
        progress.state = ResearchSessionTaskState::Failed;
        progress.phase = ResearchSessionPhase::Failed;
        progress.updated_at_ms = Some(now_ms());
        progress.error_code = Some(error.code().to_owned());
        self.update_session_progress(app, progress);
        diagnostics::record(
            "error",
            error.code(),
            operation,
            "The checked-session preparation stopped safely. No save file was changed.",
        );
        Err(error)
    }

    pub fn download_source(
        &self,
        app: &AppHandle,
        stored: &StoredResearchSetup,
    ) -> Result<DownloadedResearchSource, ObservatoryError> {
        if stored.accepted_notice_revision != RESEARCH_NOTICE_REVISION {
            return Err(ObservatoryError::ResearchNoticeRequired);
        }
        if self
            .downloading
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ObservatoryError::CriticalTaskBusy);
        }
        let started_at_ms = now_ms();
        self.update_download_progress(
            app,
            ResearchSourceDownloadProgress {
                task_id: "research_source_download".to_owned(),
                run_id: format!("research-source-{started_at_ms}"),
                state: ResearchSourceDownloadState::Running,
                phase: ResearchSourceDownloadPhase::Connecting,
                progress_percent: Some(5),
                transferred_bytes: 0,
                expected_bytes: None,
                started_at_ms: Some(started_at_ms),
                updated_at_ms: Some(started_at_ms),
                current_item: Some("github_connection".to_owned()),
                error_code: None,
            },
        );
        diagnostics::record(
            "info",
            "research_source_download_started",
            "download_reviewed_tesmio_source",
            "The reviewed TesmioLoader source download started.",
        );
        let result =
            download_reviewed_source(&self.managed_source_root, |phase, transferred, expected| {
                let (phase, percent, current_item) = match phase {
                    DownloadPhase::Connecting => (
                        ResearchSourceDownloadPhase::Connecting,
                        Some(5),
                        "github_connection",
                    ),
                    DownloadPhase::Downloading => {
                        let percent = expected.filter(|expected| *expected > 0).map(|expected| {
                            (10 + transferred.saturating_mul(60) / expected).min(70) as u8
                        });
                        (
                            ResearchSourceDownloadPhase::Downloading,
                            percent,
                            "reviewed_source_archive",
                        )
                    }
                    DownloadPhase::CheckingArchive => (
                        ResearchSourceDownloadPhase::CheckingArchive,
                        Some(76),
                        "archive_safety_checks",
                    ),
                    DownloadPhase::Installing => (
                        ResearchSourceDownloadPhase::Installing,
                        Some(88),
                        "reviewed_source_files",
                    ),
                    DownloadPhase::Verifying => (
                        ResearchSourceDownloadPhase::Verifying,
                        Some(96),
                        "reviewed_header_identity",
                    ),
                };
                let mut current = self.download_progress();
                current.state = ResearchSourceDownloadState::Running;
                current.phase = phase;
                current.progress_percent = percent;
                if transferred > 0 {
                    current.transferred_bytes = transferred;
                }
                if expected.is_some() {
                    current.expected_bytes = expected;
                }
                current.updated_at_ms = Some(now_ms());
                current.current_item = Some(current_item.to_owned());
                current.error_code = None;
                self.update_download_progress(app, current);
            });
        self.downloading.store(false, Ordering::Release);
        match result {
            Ok(source) => {
                let mut progress = self.download_progress();
                progress.state = ResearchSourceDownloadState::Complete;
                progress.phase = ResearchSourceDownloadPhase::Complete;
                progress.progress_percent = Some(100);
                progress.updated_at_ms = Some(now_ms());
                progress.current_item = Some("download_complete".to_owned());
                self.update_download_progress(app, progress);
                diagnostics::record(
                    "info",
                    "research_source_download_complete",
                    "download_reviewed_tesmio_source",
                    &format!(
                        "The reviewed source was validated and stored (archive {}, reused {}).",
                        source.archive_hash, source.reused
                    ),
                );
                Ok(source)
            }
            Err(error) => {
                let mut progress = self.download_progress();
                progress.state = ResearchSourceDownloadState::Failed;
                progress.phase = ResearchSourceDownloadPhase::Failed;
                progress.updated_at_ms = Some(now_ms());
                progress.current_item = Some("download_stopped".to_owned());
                progress.error_code = Some(error.code().to_owned());
                self.update_download_progress(app, progress);
                diagnostics::record(
                    "warning",
                    error.code(),
                    "download_reviewed_tesmio_source",
                    "The reviewed source download stopped safely. Existing source and probe files were unchanged.",
                );
                Err(error)
            }
        }
    }

    pub fn build_probe(
        &self,
        app: &AppHandle,
        stored: &StoredResearchSetup,
    ) -> Result<BuildArtifact, ObservatoryError> {
        if self
            .building
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ObservatoryError::CriticalTaskBusy);
        }
        let result = self.build_probe_inner(app, stored);
        self.building.store(false, Ordering::Release);
        result
    }

    fn build_probe_inner(
        &self,
        app: &AppHandle,
        stored: &StoredResearchSetup,
    ) -> Result<BuildArtifact, ObservatoryError> {
        let started = now_ms();
        self.update_progress(
            app,
            ResearchBuildProgress {
                task_id: "research_probe_build".to_owned(),
                run_id: format!("research-probe-build-{started}"),
                state: ResearchBuildState::Running,
                phase: ResearchBuildPhase::Preflight,
                progress_percent: Some(10),
                started_at_ms: Some(started),
                updated_at_ms: Some(started),
                current_item: Some("reviewed_contract".to_owned()),
                log_lines: Vec::new(),
                error_code: None,
                failed_stage: None,
                compiler_exit_code: None,
                remediation_code: None,
            },
        );
        diagnostics::record(
            "info",
            "research_build_started",
            "build_research_probe",
            "The bounded research-probe build entered preflight.",
        );
        if stored.accepted_notice_revision != RESEARCH_NOTICE_REVISION {
            return self.fail(
                app,
                "research_notice_required",
                "review_research_notice",
                ObservatoryError::ResearchNoticeRequired,
            );
        }
        let Some(source_root) = self
            .source_root
            .as_deref()
            .filter(|path| source_ready(path))
        else {
            return self.fail(
                app,
                "research_source_unavailable",
                "repair_application_installation",
                ObservatoryError::ResearchSourceUnavailable,
            );
        };
        let Some(checkout_path) = stored.tesmio_checkout_path.as_deref() else {
            return self.fail(
                app,
                "research_checkout_required",
                "choose_reviewed_checkout",
                ObservatoryError::InvalidResearchCheckout,
            );
        };
        let checkout = match self.validate_checkout(checkout_path) {
            Ok(checkout) => checkout,
            Err(error) => {
                let code = if checkout_path.is_dir() {
                    "research_checkout_unsupported"
                } else {
                    "research_checkout_missing"
                };
                return self.fail(app, code, "choose_reviewed_checkout", error);
            }
        };
        let compiler_checkout = compiler_checkout_path(&checkout);
        let Some(powershell) = find_powershell() else {
            return self.fail(
                app,
                "research_toolchain_unavailable",
                "install_visual_cpp_build_tools",
                ObservatoryError::ResearchToolchainUnavailable,
            );
        };
        if !compiler_ready() {
            return self.fail(
                app,
                "research_toolchain_unavailable",
                "install_visual_cpp_build_tools",
                ObservatoryError::ResearchToolchainUnavailable,
            );
        }
        self.advance(
            app,
            ResearchBuildPhase::Toolchain,
            30,
            "visual_cpp_toolchain",
        );
        let script = source_root.join("build.ps1");
        self.advance(
            app,
            ResearchBuildPhase::Compiling,
            55,
            "observatory_probe_cpp",
        );
        let output = match Command::new(powershell)
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&script)
            .arg("-TesmioLoaderRoot")
            .arg(&compiler_checkout)
            .current_dir(source_root)
            .output()
        {
            Ok(output) => output,
            Err(_) => {
                return self.fail(
                    app,
                    "research_build_failed",
                    "inspect_build_diagnostics",
                    ObservatoryError::ResearchBuildFailed,
                );
            }
        };
        let logs = sanitise_build_output(
            &output.stdout,
            &output.stderr,
            source_root,
            &compiler_checkout,
        );
        if !output.status.success() {
            self.set_logs_and_exit_code(app, logs, output.status.code());
            return self.fail(
                app,
                "research_build_failed",
                "inspect_build_diagnostics",
                ObservatoryError::ResearchBuildFailed,
            );
        }
        self.set_logs(app, logs);
        self.advance(app, ResearchBuildPhase::Verifying, 90, "probe_artifact");
        let Some(artifact) = inspect_unrecorded_artifact(source_root) else {
            return self.fail(
                app,
                "research_artifact_invalid",
                "inspect_build_diagnostics",
                ObservatoryError::ResearchBuildFailed,
            );
        };
        if record_probe_build_provenance(source_root, &artifact).is_err() {
            return self.fail(
                app,
                "research_artifact_invalid",
                "inspect_build_diagnostics",
                ObservatoryError::ResearchBuildFailed,
            );
        }
        let Some(artifact) = inspect_artifact(source_root) else {
            return self.fail(
                app,
                "research_artifact_invalid",
                "inspect_build_diagnostics",
                ObservatoryError::ResearchBuildFailed,
            );
        };
        let mut progress = self.progress();
        progress.state = ResearchBuildState::Complete;
        progress.phase = ResearchBuildPhase::Complete;
        progress.progress_percent = Some(100);
        progress.updated_at_ms = Some(now_ms());
        progress.current_item = Some("build_complete".to_owned());
        self.update_progress(app, progress);
        diagnostics::record(
            "info",
            "research_build_complete",
            "build_research_probe",
            &format!(
                "The bounded research probe was verified ({} bytes).",
                artifact.size
            ),
        );
        Ok(artifact)
    }

    fn advance(&self, app: &AppHandle, phase: ResearchBuildPhase, percent: u8, item: &str) {
        let mut progress = self.progress();
        progress.phase = phase;
        progress.progress_percent = Some(percent);
        progress.updated_at_ms = Some(now_ms());
        progress.current_item = Some(item.to_owned());
        self.update_progress(app, progress);
    }

    fn set_logs(&self, app: &AppHandle, logs: Vec<String>) {
        let mut progress = self.progress();
        progress.log_lines = logs;
        progress.updated_at_ms = Some(now_ms());
        self.update_progress(app, progress);
    }

    fn set_logs_and_exit_code(&self, app: &AppHandle, logs: Vec<String>, exit_code: Option<i32>) {
        let mut progress = self.progress();
        progress.log_lines = logs;
        progress.compiler_exit_code = exit_code;
        progress.updated_at_ms = Some(now_ms());
        self.update_progress(app, progress);
    }

    fn fail<T>(
        &self,
        app: &AppHandle,
        error_code: &str,
        remediation_code: &str,
        error: ObservatoryError,
    ) -> Result<T, ObservatoryError> {
        let mut progress = self.progress();
        let failed_stage = format!("{:?}", progress.phase).to_ascii_lowercase();
        progress.state = ResearchBuildState::Failed;
        progress.phase = ResearchBuildPhase::Failed;
        progress.updated_at_ms = Some(now_ms());
        progress.error_code = Some(error_code.to_owned());
        progress.failed_stage = Some(failed_stage.clone());
        progress.remediation_code = Some(remediation_code.to_owned());
        self.update_progress(app, progress);
        let failed = self.progress();
        let exit = failed
            .compiler_exit_code
            .map(|code| format!(" Compiler exit code: {code}."))
            .unwrap_or_default();
        let excerpt = failed
            .log_lines
            .iter()
            .rev()
            .take(2)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        let excerpt = if excerpt.is_empty() {
            String::new()
        } else {
            format!(" Sanitised output: {excerpt}")
        };
        diagnostics::record(
            "error",
            error_code,
            "build_research_probe",
            &format!(
                "The research build stopped during {failed_stage}. Remediation: {remediation_code}.{exit}{excerpt}"
            ),
        );
        Err(error)
    }

    fn update_progress(&self, app: &AppHandle, progress: ResearchBuildProgress) {
        if let Ok(mut current) = self.progress.lock() {
            *current = progress.clone();
        }
        let _ = app.emit("research-setup-progress", progress);
    }

    fn update_download_progress(&self, app: &AppHandle, progress: ResearchSourceDownloadProgress) {
        if let Ok(mut current) = self.download_progress.lock() {
            *current = progress.clone();
        }
        let _ = app.emit("research-source-download-progress", progress);
    }

    fn update_session_progress(&self, app: &AppHandle, progress: ResearchSessionProgress) {
        if let Ok(mut current) = self.session_progress.lock() {
            *current = progress.clone();
        }
        let _ = app.emit("research-session-progress", progress);
    }
}

#[derive(Debug)]
struct ManagedSessionPaths {
    game_executable: PathBuf,
    session_root: PathBuf,
}

fn managed_session_paths(media_directory: &Path) -> Option<ManagedSessionPaths> {
    let canonical_media = media_directory.canonicalize().ok()?;
    let game_root = canonical_media.parent()?.to_path_buf();
    let game_executable = game_root.join("SOVIET64.exe");
    let metadata = fs::symlink_metadata(&game_executable).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return None;
    }
    Some(ManagedSessionPaths {
        game_executable,
        session_root: game_root
            .join("tesmioloader")
            .join(MANAGED_SESSION_DIRECTORY),
    })
}

fn session_configuration(game_executable: &Path) -> Option<Vec<u8>> {
    let game_executable = compiler_checkout_path(game_executable);
    let game_executable = game_executable.to_string_lossy();
    if game_executable.len() > 2_048
        || game_executable
            .chars()
            .any(|character| character.is_control())
    {
        return None;
    }
    Some(
        format!(
            "; Prepared by Republic Observatory after explicit player consent.\r\n\
[tesmioloader]\r\n\
version = observatory-{REVIEWED_TESMIO_REVISION}\r\n\
game_exe = {game_executable}\r\n\
trace_reads = 0\r\n\
log_game = 0\r\n\
vfs = 0\r\n\
probe_map = 0\r\n\
probe_texel = 0\r\n\
save_manifest = 0\r\n\
plugins = 1\r\n\
menu_patch = 0\r\n\
version_check = 1\r\n\
\r\n\
[plugins]\r\n\
observatory_probe = 1\r\n"
        )
        .into_bytes(),
    )
}

fn read_session_manifest(root: &Path) -> Option<ResearchSessionManifest> {
    let path = root.join(SESSION_MANIFEST);
    let metadata = fs::symlink_metadata(&path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 64 * 1024 {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn managed_session_owned(root: &Path) -> bool {
    read_session_manifest(root).is_some_and(|manifest| {
        manifest.schema_version == 1
            && manifest.reviewed_revision == REVIEWED_TESMIO_REVISION
            && manifest.files.len() == SESSION_FILES.len()
            && SESSION_FILES
                .iter()
                .all(|path| manifest.files.contains_key(*path))
    })
}

fn managed_session_is_valid(
    root: &Path,
    game_executable: &Path,
    expected_probe_hash: Option<&str>,
) -> bool {
    let Some(manifest) = read_session_manifest(root) else {
        return false;
    };
    if manifest.schema_version != 1
        || manifest.reviewed_revision != REVIEWED_TESMIO_REVISION
        || manifest.files.len() != SESSION_FILES.len()
        || !crate::tesmio_probe::verify_observation_only_build_root(root)
    {
        return false;
    }
    let Some(expected_configuration) = session_configuration(game_executable) else {
        return false;
    };
    if !crate::tesmio_probe::observation_only_configuration_matches(root, &expected_configuration) {
        return false;
    }
    if manifest
        .files
        .get("plugins/observatory_probe.dll")
        .map(String::as_str)
        != expected_probe_hash
    {
        return false;
    }
    SESSION_FILES
        .iter()
        .filter(|relative| **relative != "tesmioloader.ini")
        .all(|relative| {
            manifest.files.get(*relative).is_some_and(|expected| {
                bounded_hash(&root.join(relative), MAX_SESSION_ARTIFACT_BYTES).as_deref()
                    == Some(expected.as_str())
            })
        })
}

fn checked_artifact(path: &Path, max_bytes: u64) -> Option<BuildArtifact> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > max_bytes
    {
        return None;
    }
    Some(BuildArtifact {
        hash: bounded_hash(path, max_bytes)?,
        size: metadata.len(),
    })
}

fn install_managed_session(
    paths: &ManagedSessionPaths,
    build_root: &Path,
    source_root: &Path,
    checkout: &Path,
    probe: &BuildArtifact,
) -> Result<(), ObservatoryError> {
    let parent = paths
        .session_root
        .parent()
        .ok_or(ObservatoryError::ResearchSessionPreparationFailed)?;
    fs::create_dir_all(parent).map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)?;
    let parent_metadata = fs::symlink_metadata(parent)
        .map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)?;
    if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
        return Err(ObservatoryError::ResearchSessionPreparationFailed);
    }
    if paths.session_root.exists() && !managed_session_owned(&paths.session_root) {
        return Err(ObservatoryError::ResearchSessionConflict);
    }

    let nonce = format!("{}-{}", std::process::id(), now_ms());
    let staging = parent.join(format!(".{MANAGED_SESSION_DIRECTORY}.{nonce}.staging"));
    let backup = parent.join(format!(".{MANAGED_SESSION_DIRECTORY}.{nonce}.backup"));
    fs::create_dir(&staging).map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)?;
    let result = (|| {
        fs::create_dir(staging.join("plugins"))
            .map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)?;
        copy_session_file(
            &build_root.join("tesmioloader.dll"),
            &staging.join("tesmioloader.dll"),
        )?;
        copy_session_file(
            &build_root.join("tesmiolauncher.exe"),
            &staging.join("tesmiolauncher.exe"),
        )?;
        copy_session_file(&checkout.join("LICENSE"), &staging.join("COPYING"))?;
        copy_session_file(
            &source_root.join("build/observatory_probe.dll"),
            &staging.join("plugins/observatory_probe.dll"),
        )?;
        copy_session_file(
            &source_root.join("observatory_probe.ini"),
            &staging.join("plugins/observatory_probe.ini"),
        )?;
        if bounded_hash(
            &staging.join("plugins/observatory_probe.dll"),
            MAX_SESSION_ARTIFACT_BYTES,
        )
        .as_deref()
            != Some(probe.hash.as_str())
        {
            return Err(ObservatoryError::ResearchSessionPreparationFailed);
        }
        let configuration = session_configuration(&paths.game_executable)
            .ok_or(ObservatoryError::ResearchSessionPreparationFailed)?;
        fs::write(staging.join("tesmioloader.ini"), configuration)
            .map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)?;

        let files = SESSION_FILES
            .iter()
            .map(|relative| {
                bounded_hash(&staging.join(relative), MAX_SESSION_ARTIFACT_BYTES)
                    .map(|hash| ((*relative).to_owned(), hash))
                    .ok_or(ObservatoryError::ResearchSessionPreparationFailed)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let manifest = ResearchSessionManifest {
            schema_version: 1,
            reviewed_revision: REVIEWED_TESMIO_REVISION.to_owned(),
            installed_at_ms: now_ms(),
            files,
        };
        let manifest = serde_json::to_vec_pretty(&manifest)
            .map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)?;
        fs::write(staging.join(SESSION_MANIFEST), manifest)
            .map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)?;
        if !managed_session_is_valid(&staging, &paths.game_executable, Some(&probe.hash)) {
            return Err(ObservatoryError::ResearchSessionPreparationFailed);
        }

        if paths.session_root.exists() {
            fs::rename(&paths.session_root, &backup)
                .map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)?;
        }
        if fs::rename(&staging, &paths.session_root).is_err() {
            if backup.exists() {
                let _ = fs::rename(&backup, &paths.session_root);
            }
            return Err(ObservatoryError::ResearchSessionPreparationFailed);
        }
        if backup.exists() && managed_session_owned(&backup) {
            let _ = fs::remove_dir_all(&backup);
        }
        Ok(())
    })();
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn copy_session_file(source: &Path, destination: &Path) -> Result<(), ObservatoryError> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > MAX_SESSION_ARTIFACT_BYTES
    {
        return Err(ObservatoryError::ResearchSessionPreparationFailed);
    }
    fs::copy(source, destination)
        .map(|_| ())
        .map_err(|_| ObservatoryError::ResearchSessionPreparationFailed)
}

fn canonical_checkout_path(path: &Path) -> Result<PathBuf, ObservatoryError> {
    // Tauri's Windows directory picker already returns an extended-length,
    // canonical path. Canonicalising that form a second time is not stable
    // across all Windows filesystem providers, so retain it after the same
    // bounded metadata checks performed by the caller.
    #[cfg(windows)]
    if path.to_string_lossy().starts_with(r"\\?\") {
        return Ok(path.to_path_buf());
    }
    path.canonicalize()
        .map_err(|_| ObservatoryError::InvalidResearchCheckout)
}

fn compiler_checkout_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let display = path.to_string_lossy();
        if let Some(unc) = display.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        if let Some(drive) = display.strip_prefix(r"\\?\")
            && drive.as_bytes().get(1) == Some(&b':')
            && drive
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic)
        {
            return PathBuf::from(drive);
        }
    }
    path.to_path_buf()
}

#[derive(Clone, Debug)]
pub struct BuildArtifact {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResearchProbeBuildProvenance {
    schema_version: u32,
    source_contract_hash: String,
    artifact_hash: String,
}

fn discover_source_root() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    current
        .ancestors()
        .take(5)
        .map(|ancestor| ancestor.join("research").join("tesmioloader-probe"))
        .find(|candidate| source_ready(candidate))
}

fn source_ready(path: &Path) -> bool {
    [
        "build.ps1",
        "build-observation-session.ps1",
        "observatory_probe.cpp",
        "observatory_probe.ini",
        "COPYING",
        "verify-observation-only.ps1",
    ]
    .iter()
    .all(|name| path.join(name).is_file())
}

fn checkout_state(path: Option<&Path>) -> ResearchCheckoutState {
    let Some(path) = path else {
        return ResearchCheckoutState::NotSelected;
    };
    if !path.is_dir() {
        return ResearchCheckoutState::Missing;
    }
    if checkout_matches_reviewed_headers(path) {
        ResearchCheckoutState::Reviewed
    } else {
        ResearchCheckoutState::Unsupported
    }
}

pub(crate) fn checkout_matches_reviewed_headers(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let plugin = path.join("src").join("tesmio_plugin.h");
    let api = path.join("src").join("tesmio_api.h");
    matches!(
        (
        bounded_reviewed_file_hash(&plugin, MAX_HEADER_BYTES),
        bounded_reviewed_file_hash(&api, MAX_HEADER_BYTES),
        ),
        (Some(plugin_hash), Some(api_hash))
            if plugin_hash == REVIEWED_PLUGIN_HEADER_HASH
                && api_hash == REVIEWED_API_HEADER_HASH
    )
}

fn compiler_ready() -> bool {
    let Some(program_files) = std::env::var_os("ProgramFiles(x86)") else {
        return false;
    };
    PathBuf::from(program_files)
        .join("Microsoft Visual Studio")
        .join("Installer")
        .join("vswhere.exe")
        .is_file()
        && find_powershell().is_some()
}

fn find_powershell() -> Option<PathBuf> {
    let modern = PathBuf::from(r"C:\Program Files\PowerShell\7\pwsh.exe");
    if modern.is_file() {
        return Some(modern);
    }
    let system_root = std::env::var_os("SystemRoot")?;
    let legacy = PathBuf::from(system_root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    legacy.is_file().then_some(legacy)
}

fn inspect_artifact(source_root: &Path) -> Option<BuildArtifact> {
    let artifact = inspect_unrecorded_artifact(source_root)?;
    let path = source_root.join("build").join(PROBE_BUILD_PROVENANCE);
    let metadata = fs::symlink_metadata(&path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 64 * 1024 {
        return None;
    }
    let provenance: ResearchProbeBuildProvenance =
        serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    (provenance.schema_version == 1
        && provenance.artifact_hash == artifact.hash
        && provenance.source_contract_hash == probe_source_contract_hash(source_root)?)
    .then_some(artifact)
}

fn inspect_unrecorded_artifact(source_root: &Path) -> Option<BuildArtifact> {
    let path = source_root.join("build").join("observatory_probe.dll");
    let metadata = fs::symlink_metadata(&path).ok()?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > MAX_ARTIFACT_BYTES
    {
        return None;
    }
    Some(BuildArtifact {
        hash: bounded_hash(&path, MAX_ARTIFACT_BYTES)?,
        size: metadata.len(),
    })
}

fn record_probe_build_provenance(
    source_root: &Path,
    artifact: &BuildArtifact,
) -> Result<(), ObservatoryError> {
    let provenance = ResearchProbeBuildProvenance {
        schema_version: 1,
        source_contract_hash: probe_source_contract_hash(source_root)
            .ok_or(ObservatoryError::ResearchBuildFailed)?,
        artifact_hash: artifact.hash.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&provenance)
        .map_err(|_| ObservatoryError::ResearchBuildFailed)?;
    fs::write(
        source_root.join("build").join(PROBE_BUILD_PROVENANCE),
        bytes,
    )
    .map_err(|_| ObservatoryError::ResearchBuildFailed)
}

fn probe_source_contract_hash(source_root: &Path) -> Option<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"republic-observatory-probe-source.v1\0");
    for relative in PROBE_SOURCE_CONTRACT_FILES {
        let bytes = bounded_read(&source_root.join(relative), MAX_ARTIFACT_BYTES)?;
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Some(
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

pub(crate) fn bounded_hash(path: &Path, max_bytes: u64) -> Option<String> {
    let bytes = bounded_read(path, max_bytes)?;
    Some(sha256(&bytes))
}

pub(crate) fn bounded_reviewed_file_hash(path: &Path, max_bytes: u64) -> Option<String> {
    let bytes = bounded_read(path, max_bytes)?;
    Some(reviewed_header_hash(&bytes))
}

fn bounded_read(path: &Path, max_bytes: u64) -> Option<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > max_bytes {
        return None;
    }
    let mut file = fs::File::open(path).ok()?;
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).ok()?);
    file.by_ref()
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > max_bytes {
        return None;
    }
    Some(bytes)
}

pub(crate) fn reviewed_header_hash(bytes: &[u8]) -> String {
    let mut canonical = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            canonical.push(b'\n');
            index += 2;
        } else {
            canonical.push(bytes[index]);
            index += 1;
        }
    }
    sha256(&canonical)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sanitise_build_output(
    stdout: &[u8],
    stderr: &[u8],
    source_root: &Path,
    checkout: &Path,
) -> Vec<String> {
    let source = source_root.to_string_lossy();
    let checkout = checkout.to_string_lossy();
    String::from_utf8_lossy(stdout)
        .lines()
        .chain(String::from_utf8_lossy(stderr).lines())
        .map(|line| {
            line.replace(source.as_ref(), "<observatory-source>")
                .replace(checkout.as_ref(), "<tesmioloader-checkout>")
                .chars()
                .filter(|character| !character.is_control() || *character == '\t')
                .take(MAX_LOG_LINE_CHARS)
                .collect::<String>()
        })
        .filter(|line| !line.trim().is_empty())
        .take(MAX_LOG_LINES)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        BuildArtifact, MAX_ARTIFACT_BYTES, MAX_HEADER_BYTES, ManagedSessionPaths,
        REVIEWED_API_HEADER_HASH, REVIEWED_PLUGIN_HEADER_HASH, REVIEWED_TESMIO_REVISION,
        ResearchCheckoutState, SESSION_FILES, bounded_hash, canonical_checkout_path,
        checkout_state, compiler_checkout_path, inspect_artifact, inspect_unrecorded_artifact,
        install_managed_session, managed_session_is_valid, managed_session_owned,
        record_probe_build_provenance, reviewed_header_hash, sanitise_build_output,
        session_configuration,
    };

    #[test]
    fn rejects_missing_and_unreviewed_checkouts() {
        assert_eq!(checkout_state(None), ResearchCheckoutState::NotSelected);
        let directory = tempdir().expect("checkout fixture");
        assert_eq!(
            checkout_state(Some(&directory.path().join("missing"))),
            ResearchCheckoutState::Missing
        );
        fs::create_dir(directory.path().join("src")).expect("source directory");
        fs::write(directory.path().join("src/tesmio_plugin.h"), "unreviewed")
            .expect("plugin header");
        fs::write(directory.path().join("src/tesmio_api.h"), "unreviewed").expect("api header");
        assert_eq!(
            checkout_state(Some(directory.path())),
            ResearchCheckoutState::Unsupported
        );
    }

    #[test]
    fn reviewed_hash_constants_are_full_sha256_values() {
        assert_eq!(REVIEWED_PLUGIN_HEADER_HASH.len(), 64);
        assert_eq!(REVIEWED_API_HEADER_HASH.len(), 64);
        let directory = tempdir().expect("hash fixture");
        let oversized = directory.path().join("oversized");
        fs::write(&oversized, vec![0_u8; MAX_HEADER_BYTES as usize + 1]).expect("large fixture");
        assert!(bounded_hash(&oversized, MAX_HEADER_BYTES).is_none());
    }

    #[test]
    fn reviewed_header_identity_is_stable_across_checkout_line_endings() {
        assert_eq!(
            reviewed_header_hash(b"first\nsecond\n"),
            reviewed_header_hash(b"first\r\nsecond\r\n")
        );
        assert_ne!(
            reviewed_header_hash(b"first\nsecond\n"),
            reviewed_header_hash(b"first\nchanged\n")
        );
    }

    #[test]
    fn probe_build_provenance_rejects_stale_compiled_source() {
        let directory = tempdir().expect("probe source fixture");
        for relative in super::PROBE_SOURCE_CONTRACT_FILES {
            fs::write(
                directory.path().join(relative),
                format!("fixture {relative}"),
            )
            .expect("source contract file");
        }
        fs::create_dir(directory.path().join("build")).expect("build directory");
        fs::write(
            directory.path().join("build/observatory_probe.dll"),
            b"compiled probe",
        )
        .expect("probe artifact");
        assert!(inspect_artifact(directory.path()).is_none());
        let artifact = inspect_unrecorded_artifact(directory.path()).expect("unrecorded artifact");
        record_probe_build_provenance(directory.path(), &artifact).expect("build provenance");
        assert!(inspect_artifact(directory.path()).is_some());

        fs::write(
            directory.path().join("observatory_probe.cpp"),
            b"corrected probe source",
        )
        .expect("updated probe source");
        assert!(
            inspect_artifact(directory.path()).is_none(),
            "a source correction must force one fresh local probe build"
        );
    }

    #[cfg(windows)]
    #[test]
    fn preserves_windows_extended_length_checkout_paths() {
        let path = std::path::Path::new(r"\\?\D:\reviewed\TesmioLoader");
        assert_eq!(canonical_checkout_path(path).expect("verbatim path"), path);
    }

    #[cfg(windows)]
    #[test]
    fn supplies_native_tools_with_a_non_verbatim_windows_path() {
        assert_eq!(
            compiler_checkout_path(std::path::Path::new(r"\\?\D:\reviewed\TesmioLoader")),
            std::path::Path::new(r"D:\reviewed\TesmioLoader")
        );
        assert_eq!(
            compiler_checkout_path(std::path::Path::new(r"\\?\UNC\server\share\TesmioLoader")),
            std::path::Path::new(r"\\server\share\TesmioLoader")
        );
    }

    #[test]
    fn build_logs_remove_private_roots_and_control_characters() {
        let source = std::path::Path::new(r"C:\private\observatory");
        let checkout = std::path::Path::new(r"D:\private\tesmio");
        let logs = sanitise_build_output(
            b"C:\\private\\observatory\\probe.cpp\n",
            b"D:\\private\\tesmio\\header.h\x07\n",
            source,
            checkout,
        );
        assert_eq!(logs[0], r"<observatory-source>\probe.cpp");
        assert_eq!(logs[1], r"<tesmioloader-checkout>\header.h");
    }

    #[test]
    fn generated_session_configuration_keeps_every_write_surface_disabled() {
        let configuration = session_configuration(std::path::Path::new(
            r"C:\Games\SovietRepublic\SOVIET64.exe",
        ))
        .expect("bounded configuration");
        let configuration = String::from_utf8(configuration).expect("UTF-8 configuration");
        for required in [
            "trace_reads = 0",
            "log_game = 0",
            "vfs = 0",
            "save_manifest = 0",
            "menu_patch = 0",
            "version_check = 1",
            "observatory_probe = 1",
        ] {
            assert!(configuration.contains(required), "missing {required}");
        }
        assert!(!configuration.contains("save_manifest = 1"));
        assert!(!configuration.contains("version_check = 0"));
    }

    #[test]
    fn replacement_requires_a_complete_observatory_owned_manifest() {
        let directory = tempdir().expect("managed session fixture");
        fs::write(
            directory.path().join(super::SESSION_MANIFEST),
            format!(
                r#"{{"schema_version":1,"reviewed_revision":"{REVIEWED_TESMIO_REVISION}","installed_at_ms":1,"files":{{"tesmioloader.dll":"hash"}}}}"#,
            ),
        )
        .expect("partial manifest");
        assert!(!managed_session_owned(directory.path()));

        let files = SESSION_FILES
            .iter()
            .map(|path| format!(r#""{path}":"hash""#))
            .collect::<Vec<_>>()
            .join(",");
        fs::write(
            directory.path().join(super::SESSION_MANIFEST),
            format!(
                r#"{{"schema_version":1,"reviewed_revision":"{REVIEWED_TESMIO_REVISION}","installed_at_ms":1,"files":{{{files}}}}}"#,
            ),
        )
        .expect("complete manifest");
        assert!(managed_session_owned(directory.path()));
    }

    #[test]
    fn managed_session_install_changes_only_its_dedicated_folder() {
        let fixture = tempdir().expect("session fixture");
        let game_root = fixture.path().join("SovietRepublic");
        let media = game_root.join("media_soviet");
        let save = media.join("save_cloud/test-save.zip");
        fs::create_dir_all(save.parent().expect("save parent")).expect("save directory");
        fs::write(&save, b"save bytes stay unchanged").expect("save fixture");
        let game_executable = game_root.join("SOVIET64.exe");
        fs::write(&game_executable, b"game bytes stay unchanged").expect("game fixture");

        let build_root = fixture.path().join("host-build");
        fs::create_dir_all(&build_root).expect("host build");
        fs::write(build_root.join("tesmioloader.dll"), b"reviewed loader").expect("loader fixture");
        fs::write(build_root.join("tesmiolauncher.exe"), b"reviewed launcher")
            .expect("launcher fixture");

        let source_root = fixture.path().join("probe-source");
        fs::create_dir_all(source_root.join("build")).expect("probe build");
        let probe_path = source_root.join("build/observatory_probe.dll");
        fs::write(&probe_path, b"reviewed Observatory probe").expect("probe fixture");
        fs::write(
            source_root.join("observatory_probe.ini"),
            b"[observatory]\n",
        )
        .expect("probe settings");
        let checkout = fixture.path().join("reviewed-checkout");
        fs::create_dir_all(&checkout).expect("checkout fixture");
        fs::write(checkout.join("LICENSE"), b"GPL fixture").expect("licence fixture");
        let probe = BuildArtifact {
            hash: bounded_hash(&probe_path, MAX_ARTIFACT_BYTES).expect("probe hash"),
            size: fs::metadata(&probe_path).expect("probe metadata").len(),
        };
        let paths = ManagedSessionPaths {
            game_executable: game_executable.clone(),
            session_root: game_root.join("tesmioloader/observatory"),
        };

        install_managed_session(&paths, &build_root, &source_root, &checkout, &probe)
            .expect("managed installation");

        assert!(managed_session_is_valid(
            &paths.session_root,
            &game_executable,
            Some(&probe.hash),
        ));
        assert!(
            !managed_session_is_valid(&paths.session_root, &game_executable, None),
            "a prepared session cannot become trusted without its recorded probe identity"
        );
        assert!(
            !managed_session_is_valid(
                &paths.session_root,
                &game_executable,
                Some("changed-probe-hash"),
            ),
            "a different recorded probe must require a new checked preparation"
        );
        let rewritten_configuration = String::from_utf8(
            session_configuration(&game_executable).expect("generated configuration"),
        )
        .expect("UTF-8 configuration")
        .replace(" = ", "=")
        .replace(
            "\r\n\r\n[plugins]",
            &format!(
                "\r\nmenu_tag=tesmioloader v. observatory-{REVIEWED_TESMIO_REVISION}\r\n\r\n[plugins]"
            ),
        );
        #[cfg(windows)]
        let rewritten_configuration = rewritten_configuration.replace(
            &format!(
                "game_exe={}",
                compiler_checkout_path(&game_executable).to_string_lossy()
            ),
            &format!(
                "game_exe={}",
                game_executable
                    .canonicalize()
                    .expect("extended game executable path")
                    .to_string_lossy()
            ),
        );
        fs::write(
            paths.session_root.join("tesmioloader.ini"),
            rewritten_configuration,
        )
        .expect("loader-rewritten configuration");
        assert!(
            managed_session_is_valid(&paths.session_root, &game_executable, Some(&probe.hash)),
            "TesmioLoader's safe first-launch rewrite must not create a repair loop"
        );
        let unsafe_configuration = fs::read_to_string(paths.session_root.join("tesmioloader.ini"))
            .expect("rewritten configuration")
            .replace("menu_patch=0", "menu_patch=1");
        fs::write(
            paths.session_root.join("tesmioloader.ini"),
            unsafe_configuration,
        )
        .expect("unsafe configuration fixture");
        assert!(!managed_session_is_valid(
            &paths.session_root,
            &game_executable,
            Some(&probe.hash),
        ));
        assert_eq!(
            fs::read(&save).expect("save after setup"),
            b"save bytes stay unchanged"
        );
        assert_eq!(
            fs::read(&game_executable).expect("game after setup"),
            b"game bytes stay unchanged"
        );
        assert_eq!(
            fs::read_dir(game_root.join("tesmioloader"))
                .expect("Tesmio directory")
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec!["observatory"]
        );
    }

    #[test]
    #[ignore = "requires an explicitly supplied local reviewed checkout"]
    fn live_reviewed_checkout_uses_the_same_status_and_build_validation() {
        let path = std::env::var_os("RO_REVIEWED_TESMIO_CHECKOUT")
            .map(std::path::PathBuf::from)
            .expect("RO_REVIEWED_TESMIO_CHECKOUT");
        assert_eq!(checkout_state(Some(&path)), ResearchCheckoutState::Reviewed);
        let data = tempfile::tempdir().expect("managed source root");
        let service = super::ResearchSetupService::discover(data.path());
        service
            .validate_checkout(&path)
            .expect("status-approved checkout must pass build validation");
    }

    #[test]
    #[ignore = "requires an explicitly supplied prepared-session folder"]
    fn live_prepared_session_uses_the_same_read_only_validity_check() {
        let root = std::env::var_os("RO_PREPARED_SESSION_ROOT")
            .map(std::path::PathBuf::from)
            .expect("RO_PREPARED_SESSION_ROOT");
        let game_executable = std::env::var_os("RO_GAME_EXECUTABLE")
            .map(std::path::PathBuf::from)
            .expect("RO_GAME_EXECUTABLE");
        let expected_probe_hash =
            std::env::var("RO_EXPECTED_PROBE_HASH").expect("RO_EXPECTED_PROBE_HASH");
        assert!(managed_session_is_valid(
            &root,
            &game_executable,
            Some(&expected_probe_hash),
        ));
    }
}
