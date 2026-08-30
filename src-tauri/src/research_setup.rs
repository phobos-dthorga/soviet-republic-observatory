//! Bounded native build service for the optional GPL research companion.
//!
//! This service never downloads, installs, injects, launches W&R, or accepts an
//! arbitrary command. It validates one reviewed TesmioLoader header pair and
//! invokes the repository-owned probe build script with one canonical path.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::error::ObservatoryError;
use crate::storage::{StoredResearchSetup, now_ms};

pub const RESEARCH_NOTICE_REVISION: u32 = 1;
pub const REVIEWED_TESMIO_REVISION: &str = "3baa141f9f08921aea9c95f0a400289cabd9960a";
const REVIEWED_PLUGIN_HEADER_HASH: &str =
    "f31fa216c8cdf3a3cfe1122857dc9d2794f756adb8ae248a51905ca395de3c6a";
const REVIEWED_API_HEADER_HASH: &str =
    "5daaf51ec1f6a5f279868bb039f01719931dfba0be69312a75863073ceac04a6";
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

#[derive(Clone, Debug, Serialize)]
pub struct ResearchSetupStatus {
    pub notice_revision: u32,
    pub notice_accepted: bool,
    pub source_available: bool,
    pub compiler_available: bool,
    pub checkout_state: ResearchCheckoutState,
    pub checkout_path: Option<String>,
    pub reviewed_tesmio_revision: String,
    pub probe_built: bool,
    pub probe_content_hash: Option<String>,
    pub probe_size_bytes: Option<u64>,
    pub output_path: Option<String>,
    pub last_built_at_ms: Option<i64>,
    pub can_build: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub progress: ResearchBuildProgress,
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
        }
    }
}

#[derive(Debug)]
pub struct ResearchSetupService {
    source_root: Option<PathBuf>,
    progress: Mutex<ResearchBuildProgress>,
    building: AtomicBool,
}

impl ResearchSetupService {
    pub fn discover() -> Self {
        Self {
            source_root: discover_source_root(),
            progress: Mutex::new(ResearchBuildProgress::default()),
            building: AtomicBool::new(false),
        }
    }

    pub fn progress(&self) -> ResearchBuildProgress {
        self.progress
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
        let canonical = path
            .canonicalize()
            .map_err(|_| ObservatoryError::InvalidResearchCheckout)?;
        if checkout_state(Some(&canonical)) != ResearchCheckoutState::Reviewed {
            return Err(ObservatoryError::InvalidResearchCheckout);
        }
        Ok(canonical)
    }

    pub fn status(&self, stored: &StoredResearchSetup) -> ResearchSetupStatus {
        let source_available = self.source_root.as_deref().is_some_and(source_ready);
        let compiler_available = compiler_ready();
        let checkout_state = checkout_state(stored.tesmio_checkout_path.as_deref());
        let notice_accepted = stored.accepted_notice_revision == RESEARCH_NOTICE_REVISION;
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
        let is_building = self.building.load(Ordering::Acquire);
        if is_building {
            blockers.push("build_running".to_owned());
        }
        if let (Some(recorded), Some(current)) =
            (stored.last_probe_hash.as_deref(), artifact.as_ref())
            && recorded != current.hash
        {
            warnings.push("artifact_changed_outside_assistant".to_owned());
        }
        ResearchSetupStatus {
            notice_revision: RESEARCH_NOTICE_REVISION,
            notice_accepted,
            source_available,
            compiler_available,
            checkout_state,
            checkout_path: stored
                .tesmio_checkout_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            reviewed_tesmio_revision: REVIEWED_TESMIO_REVISION.to_owned(),
            probe_built: artifact.is_some(),
            probe_content_hash: artifact.as_ref().map(|artifact| artifact.hash.clone()),
            probe_size_bytes: artifact.as_ref().map(|artifact| artifact.size),
            output_path: artifact
                .as_ref()
                .map(|artifact| artifact.path.to_string_lossy().into_owned()),
            last_built_at_ms: stored.last_built_at_ms,
            can_build: blockers.is_empty(),
            blockers,
            warnings,
            progress: self.progress(),
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
            },
        );
        if stored.accepted_notice_revision != RESEARCH_NOTICE_REVISION {
            return self.fail(
                app,
                "research_notice_required",
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
                ObservatoryError::ResearchSourceUnavailable,
            );
        };
        let Some(checkout_path) = stored.tesmio_checkout_path.as_deref() else {
            return self.fail(
                app,
                "research_checkout_required",
                ObservatoryError::InvalidResearchCheckout,
            );
        };
        let checkout = match self.validate_checkout(checkout_path) {
            Ok(checkout) => checkout,
            Err(error) => return self.fail(app, "research_checkout_invalid", error),
        };
        let Some(powershell) = find_powershell() else {
            return self.fail(
                app,
                "research_toolchain_unavailable",
                ObservatoryError::ResearchToolchainUnavailable,
            );
        };
        if !compiler_ready() {
            return self.fail(
                app,
                "research_toolchain_unavailable",
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
            .arg(&checkout)
            .current_dir(source_root)
            .output()
        {
            Ok(output) => output,
            Err(_) => {
                return self.fail(
                    app,
                    "research_build_failed",
                    ObservatoryError::ResearchBuildFailed,
                );
            }
        };
        let logs = sanitise_build_output(&output.stdout, &output.stderr, source_root, &checkout);
        if !output.status.success() {
            self.set_logs(app, logs);
            return self.fail(
                app,
                "research_build_failed",
                ObservatoryError::ResearchBuildFailed,
            );
        }
        self.set_logs(app, logs);
        self.advance(app, ResearchBuildPhase::Verifying, 90, "probe_artifact");
        let Some(artifact) = inspect_artifact(source_root) else {
            return self.fail(
                app,
                "research_artifact_invalid",
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

    fn fail<T>(
        &self,
        app: &AppHandle,
        error_code: &str,
        error: ObservatoryError,
    ) -> Result<T, ObservatoryError> {
        let mut progress = self.progress();
        progress.state = ResearchBuildState::Failed;
        progress.phase = ResearchBuildPhase::Failed;
        progress.updated_at_ms = Some(now_ms());
        progress.error_code = Some(error_code.to_owned());
        self.update_progress(app, progress);
        Err(error)
    }

    fn update_progress(&self, app: &AppHandle, progress: ResearchBuildProgress) {
        if let Ok(mut current) = self.progress.lock() {
            *current = progress.clone();
        }
        let _ = app.emit("research-setup-progress", progress);
    }
}

#[derive(Clone, Debug)]
pub struct BuildArtifact {
    pub path: PathBuf,
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
    let plugin = path.join("src").join("tesmio_plugin.h");
    let api = path.join("src").join("tesmio_api.h");
    match (
        bounded_hash(&plugin, MAX_HEADER_BYTES),
        bounded_hash(&api, MAX_HEADER_BYTES),
    ) {
        (Some(plugin_hash), Some(api_hash))
            if plugin_hash == REVIEWED_PLUGIN_HEADER_HASH
                && api_hash == REVIEWED_API_HEADER_HASH =>
        {
            ResearchCheckoutState::Reviewed
        }
        _ => ResearchCheckoutState::Unsupported,
    }
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
        path,
    })
}

fn bounded_hash(path: &Path, max_bytes: u64) -> Option<String> {
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
    Some(
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
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
        ResearchCheckoutState, bounded_hash, checkout_state, sanitise_build_output,
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
}
