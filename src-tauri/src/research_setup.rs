//! Bounded native setup and build service for the optional GPL research companion.
//!
//! The only network operation retrieves source from one reviewed upstream commit.
//! It never downloads binaries, installs, injects, launches W&R, or accepts an
//! arbitrary command. The build still requires the exact reviewed header pair.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::diagnostics;
use crate::error::ObservatoryError;
use crate::research_source_download::{DownloadedResearchSource, download_reviewed_source};
use crate::storage::{StoredResearchSetup, now_ms};

pub const RESEARCH_NOTICE_REVISION: u32 = 2;
pub const REVIEWED_TESMIO_REVISION: &str = "3baa141f9f08921aea9c95f0a400289cabd9960a";
pub(crate) const REVIEWED_PLUGIN_HEADER_HASH: &str =
    "d886ac6550dd84031ee2ed3afab13a7f75e4ddf920d23183b93395440d3cff49";
pub(crate) const REVIEWED_API_HEADER_HASH: &str =
    "33c9fae4acb1041708c7b1b4675b0eb4740f0af737e7a1968c0acb0c325fff3c";
const MAX_HEADER_BYTES: u64 = 256 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LOG_LINES: usize = 80;
const MAX_LOG_LINE_CHARS: usize = 240;

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

#[derive(Clone, Debug, Serialize)]
pub struct ResearchSourceDownloadProgress {
    pub state: ResearchSourceDownloadState,
    pub progress_percent: Option<u8>,
    pub transferred_bytes: u64,
    pub expected_bytes: Option<u64>,
    pub updated_at_ms: Option<i64>,
    pub error_code: Option<String>,
}

impl Default for ResearchSourceDownloadProgress {
    fn default() -> Self {
        Self {
            state: ResearchSourceDownloadState::Idle,
            progress_percent: None,
            transferred_bytes: 0,
            expected_bytes: None,
            updated_at_ms: None,
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
    building: AtomicBool,
    downloading: AtomicBool,
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
            building: AtomicBool::new(false),
            downloading: AtomicBool::new(false),
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

    pub fn status(&self, stored: &StoredResearchSetup) -> ResearchSetupStatus {
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
            && checkout_state != ResearchCheckoutState::Reviewed
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
        }
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
        self.update_download_progress(
            app,
            ResearchSourceDownloadProgress {
                state: ResearchSourceDownloadState::Running,
                progress_percent: Some(0),
                transferred_bytes: 0,
                expected_bytes: None,
                updated_at_ms: Some(now_ms()),
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
            download_reviewed_source(&self.managed_source_root, |transferred, expected| {
                let percent = expected
                    .filter(|expected| *expected > 0)
                    .map(|expected| ((transferred.saturating_mul(100) / expected).min(99)) as u8);
                self.update_download_progress(
                    app,
                    ResearchSourceDownloadProgress {
                        state: ResearchSourceDownloadState::Running,
                        progress_percent: percent,
                        transferred_bytes: transferred,
                        expected_bytes: expected,
                        updated_at_ms: Some(now_ms()),
                        error_code: None,
                    },
                );
            });
        self.downloading.store(false, Ordering::Release);
        match result {
            Ok(source) => {
                let mut progress = self.download_progress();
                progress.state = ResearchSourceDownloadState::Complete;
                progress.progress_percent = Some(100);
                progress.updated_at_ms = Some(now_ms());
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
                progress.updated_at_ms = Some(now_ms());
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
        bounded_reviewed_header_hash(&plugin, MAX_HEADER_BYTES),
        bounded_reviewed_header_hash(&api, MAX_HEADER_BYTES),
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

fn bounded_hash(path: &Path, max_bytes: u64) -> Option<String> {
    let bytes = bounded_read(path, max_bytes)?;
    Some(sha256(&bytes))
}

fn bounded_reviewed_header_hash(path: &Path, max_bytes: u64) -> Option<String> {
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
        MAX_HEADER_BYTES, REVIEWED_API_HEADER_HASH, REVIEWED_PLUGIN_HEADER_HASH,
        ResearchCheckoutState, bounded_hash, canonical_checkout_path, checkout_state,
        compiler_checkout_path, reviewed_header_hash, sanitise_build_output,
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
}
