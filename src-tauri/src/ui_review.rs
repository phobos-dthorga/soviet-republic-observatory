//! Bounded startup contract for deterministic native interface review.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::theme::{ThemeManifest, ThemeValidationReport, built_in_themes, validate_contrast};

const REVIEW_DIRECTORY_NAME: &str = "republic-observatory-ui-review";
pub const REVIEW_MARKER_NAME: &str = ".observatory-ui-review.json";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UiReviewDataState {
    Fixture,
    Live,
}

#[derive(Clone, Debug, Serialize)]
pub struct UiReviewContext {
    pub enabled: bool,
    pub run_id: Option<String>,
    pub data_state: Option<UiReviewDataState>,
    pub background_work_suppressed: bool,
    pub validator_boundary_theme: Option<ThemeManifest>,
    pub validator_boundary_report: Option<ThemeValidationReport>,
}

impl UiReviewContext {
    pub fn ordinary() -> Self {
        Self {
            enabled: false,
            run_id: None,
            data_state: None,
            background_work_suppressed: false,
            validator_boundary_theme: None,
            validator_boundary_report: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UiReviewStartup {
    pub context: UiReviewContext,
    pub data_directory: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReviewMarker {
    run_id: String,
    data_state: UiReviewDataState,
}

impl UiReviewStartup {
    pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut review_enabled = false;
        let mut run_id: Option<String> = None;
        let mut root: Option<PathBuf> = None;
        let mut data_state: Option<UiReviewDataState> = None;
        let mut arguments = arguments.into_iter();
        let _executable = arguments.next();

        while let Some(argument) = arguments.next() {
            let argument = argument
                .into_string()
                .map_err(|_| "UI review arguments must be valid Unicode.".to_string())?;
            match argument.as_str() {
                "--ui-review" => {
                    if review_enabled {
                        return Err("--ui-review may only be supplied once.".into());
                    }
                    review_enabled = true;
                }
                "--ui-review-run" => {
                    set_once(
                        &mut run_id,
                        next_value(&mut arguments, "--ui-review-run")?,
                        "--ui-review-run",
                    )?;
                }
                "--ui-review-root" => {
                    let value = next_value(&mut arguments, "--ui-review-root")?;
                    set_once(&mut root, PathBuf::from(value), "--ui-review-root")?;
                }
                "--ui-review-state" => {
                    let value = next_value(&mut arguments, "--ui-review-state")?;
                    let parsed = parse_data_state(&value)?;
                    set_once(&mut data_state, parsed, "--ui-review-state")?;
                }
                value if value.starts_with("--ui-review-run=") => {
                    set_once(
                        &mut run_id,
                        value["--ui-review-run=".len()..].to_owned(),
                        "--ui-review-run",
                    )?;
                }
                value if value.starts_with("--ui-review-root=") => {
                    set_once(
                        &mut root,
                        PathBuf::from(&value["--ui-review-root=".len()..]),
                        "--ui-review-root",
                    )?;
                }
                value if value.starts_with("--ui-review-state=") => {
                    set_once(
                        &mut data_state,
                        parse_data_state(&value["--ui-review-state=".len()..])?,
                        "--ui-review-state",
                    )?;
                }
                value if value.starts_with("--ui-review") => {
                    return Err(format!("Unknown UI review option '{value}'."));
                }
                _ => {}
            }
        }

        if !review_enabled {
            if run_id.is_some() || root.is_some() || data_state.is_some() {
                return Err("UI review options require --ui-review.".into());
            }
            return Ok(Self {
                context: UiReviewContext::ordinary(),
                data_directory: None,
            });
        }

        let run_id = run_id.ok_or("--ui-review-run is required in UI review mode.")?;
        if !safe_run_id(&run_id) {
            return Err(
                "--ui-review-run must contain 8-64 ASCII letters, digits, '-' or '_'.".into(),
            );
        }
        let data_state = data_state.ok_or("--ui-review-state is required in UI review mode.")?;
        let root = validate_review_root(
            &root.ok_or("--ui-review-root is required in UI review mode.")?,
            &run_id,
            data_state,
        )?;
        let data_directory = root.join("data");
        std::fs::create_dir_all(&data_directory)
            .map_err(|error| format!("Could not prepare the UI review data directory: {error}"))?;

        let (validator_boundary_theme, validator_boundary_report) = validator_boundary_theme();
        Ok(Self {
            context: UiReviewContext {
                enabled: true,
                run_id: Some(run_id),
                data_state: Some(data_state),
                background_work_suppressed: true,
                validator_boundary_theme: Some(validator_boundary_theme),
                validator_boundary_report: Some(validator_boundary_report),
            },
            data_directory: Some(data_directory),
        })
    }
}

fn parse_data_state(value: &str) -> Result<UiReviewDataState, String> {
    match value {
        "fixture" => Ok(UiReviewDataState::Fixture),
        "live" => Ok(UiReviewDataState::Live),
        _ => Err("--ui-review-state must be either 'fixture' or 'live'.".into()),
    }
}

fn validator_boundary_theme() -> (ThemeManifest, ThemeValidationReport) {
    let mut theme = built_in_themes()[0].clone();
    theme.id = "org.republic-observatory.ui-review-boundary".into();
    theme.version = "1.0.0".into();
    theme.name = "Generated validator-boundary dark".into();
    theme.author = Some("Republic Observatory native review".into());
    theme.description =
        Some("A host-validated boundary fixture for native interface review.".into());
    let original_muted = theme.colours.text_muted.clone();
    let surface = theme.colours.surface.clone();
    let mut last_valid = original_muted.clone();
    for step in 1..=100 {
        theme.colours.text_muted = blend_hex(&original_muted, &surface, step);
        if validate_contrast(&theme).valid {
            last_valid = theme.colours.text_muted.clone();
        } else {
            break;
        }
    }
    theme.colours.text_muted = last_valid;
    let report = validate_contrast(&theme);
    assert!(
        report.valid,
        "the native review boundary theme must remain valid"
    );
    (theme, report)
}

fn blend_hex(foreground: &str, background: &str, percent: u16) -> String {
    let parse = |value: &str, offset: usize| {
        u16::from_str_radix(&value[offset..offset + 2], 16)
            .expect("built-in theme colours must remain six-digit hexadecimal values")
    };
    let mix = |offset| {
        let start = parse(foreground, offset);
        let end = parse(background, offset);
        (start * (100 - percent) + end * percent + 50) / 100
    };
    format!("#{:02X}{:02X}{:02X}", mix(1), mix(3), mix(5))
}

fn next_value(
    arguments: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("{option} requires a value."))?
        .into_string()
        .map_err(|_| format!("{option} must be valid Unicode."))
}

fn set_once<T>(slot: &mut Option<T>, value: T, option: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("{option} may only be supplied once."));
    }
    *slot = Some(value);
    Ok(())
}

fn safe_run_id(value: &str) -> bool {
    (8..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn validate_review_root(
    requested: &Path,
    run_id: &str,
    data_state: UiReviewDataState,
) -> Result<PathBuf, String> {
    let base = std::env::temp_dir().join(REVIEW_DIRECTORY_NAME);
    validate_review_root_under(&base, requested, run_id, data_state)
}

fn validate_review_root_under(
    base: &Path,
    requested: &Path,
    run_id: &str,
    data_state: UiReviewDataState,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(base)
        .map_err(|error| format!("Could not prepare the UI review temporary directory: {error}"))?;
    let current_directory = std::env::current_dir()
        .map_err(|error| format!("Could not resolve the current directory: {error}"))?;
    let base_absolute = if base.is_absolute() {
        base.to_path_buf()
    } else {
        current_directory.join(base)
    };
    let requested_absolute = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        current_directory.join(requested)
    };
    if !requested_absolute.starts_with(&base_absolute) || requested_absolute == base_absolute {
        return Err(
            "--ui-review-root must be a child of the Observatory UI review temporary directory."
                .into(),
        );
    }
    reject_reparse_points(&base_absolute, &requested_absolute)?;
    let base = base_absolute
        .canonicalize()
        .map_err(|error| format!("Could not resolve the UI review temporary directory: {error}"))?;
    let requested = requested_absolute
        .canonicalize()
        .map_err(|error| format!("Could not resolve --ui-review-root: {error}"))?;
    if !requested.starts_with(&base) || requested == base {
        return Err(
            "--ui-review-root must be a child of the Observatory UI review temporary directory."
                .into(),
        );
    }
    let marker_path = requested.join(REVIEW_MARKER_NAME);
    let marker_document = std::fs::read_to_string(&marker_path)
        .map_err(|error| format!("The UI review root has no readable CLI marker: {error}"))?;
    let marker: ReviewMarker = serde_json::from_str(&marker_document)
        .map_err(|error| format!("The UI review root marker is invalid: {error}"))?;
    if marker.run_id != run_id || marker.data_state != data_state {
        return Err("The UI review root marker does not match this review run.".into());
    }
    Ok(requested)
}

#[cfg(windows)]
fn reject_reparse_points(base: &Path, requested: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    let relative = requested
        .strip_prefix(base)
        .map_err(|_| "The UI review root is outside its temporary boundary.".to_string())?;
    let mut current = base.to_path_buf();
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err("UI review roots may not contain path traversal.".into());
        }
        current.push(component.as_os_str());
        let metadata = std::fs::symlink_metadata(&current)
            .map_err(|error| format!("Could not inspect the UI review root: {error}"))?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err("UI review roots may not contain symlinks or reparse points.".into());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn reject_reparse_points(base: &Path, requested: &Path) -> Result<(), String> {
    let relative = requested
        .strip_prefix(base)
        .map_err(|_| "The UI review root is outside its temporary boundary.".to_string())?;
    let mut current = base.to_path_buf();
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err("UI review roots may not contain path traversal.".into());
        }
        current.push(component.as_os_str());
        if std::fs::symlink_metadata(&current)
            .map_err(|error| format!("Could not inspect the UI review root: {error}"))?
            .file_type()
            .is_symlink()
        {
            return Err("UI review roots may not contain symlinks.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordinary(arguments: &[&str]) -> Result<UiReviewStartup, String> {
        UiReviewStartup::parse(arguments.iter().map(OsString::from))
    }

    #[test]
    fn ordinary_startup_does_not_enable_review_mode() {
        let startup = ordinary(&["observatory", "--ordinary-option"]).unwrap();
        assert!(!startup.context.enabled);
        assert!(startup.data_directory.is_none());
    }

    #[test]
    fn review_parameters_without_review_mode_fail_closed() {
        assert!(ordinary(&["observatory", "--ui-review-state", "fixture"]).is_err());
    }

    #[test]
    fn unsafe_run_identifiers_are_rejected_before_filesystem_access() {
        let error = ordinary(&[
            "observatory",
            "--ui-review",
            "--ui-review-run",
            "../escape",
            "--ui-review-root",
            "missing",
            "--ui-review-state",
            "fixture",
        ])
        .unwrap_err();
        assert!(error.contains("8-64 ASCII"));
    }

    #[test]
    fn unknown_review_options_are_rejected() {
        assert!(ordinary(&["observatory", "--ui-review-script", "alert(1)"]).is_err());
    }

    #[test]
    fn generated_boundary_theme_is_valid_and_exercises_a_boundary() {
        let classic = &built_in_themes()[0];
        let (theme, report) = validator_boundary_theme();
        assert!(report.valid);
        assert_ne!(theme.colours.text_muted, classic.colours.text_muted);
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.passes && check.measured - check.minimum < 0.15 })
        );
    }

    #[test]
    fn marked_review_roots_are_bounded_and_exact() {
        let temporary = tempfile::tempdir().unwrap();
        let base = temporary.path().join(REVIEW_DIRECTORY_NAME);
        let requested = base.join("review-safe123");
        std::fs::create_dir_all(&requested).unwrap();
        std::fs::write(
            requested.join(REVIEW_MARKER_NAME),
            serde_json::to_vec(&ReviewMarker {
                run_id: "review-safe123".into(),
                data_state: UiReviewDataState::Fixture,
            })
            .unwrap(),
        )
        .unwrap();

        let resolved = validate_review_root_under(
            &base,
            &requested,
            "review-safe123",
            UiReviewDataState::Fixture,
        )
        .unwrap();
        assert_eq!(resolved, requested.canonicalize().unwrap());
        assert!(
            validate_review_root_under(
                &base,
                &requested,
                "review-stale123",
                UiReviewDataState::Fixture,
            )
            .is_err()
        );
        assert!(
            validate_review_root_under(
                &base,
                temporary.path(),
                "review-safe123",
                UiReviewDataState::Fixture,
            )
            .is_err()
        );
        assert!(
            validate_review_root_under(
                &base,
                &requested.join("..").join("review-safe123"),
                "review-safe123",
                UiReviewDataState::Fixture,
            )
            .is_err()
        );
    }

    #[cfg(windows)]
    #[test]
    fn review_roots_reject_available_windows_reparse_links() {
        use std::os::windows::fs::symlink_dir;

        let temporary = tempfile::tempdir().unwrap();
        let base = temporary.path().join(REVIEW_DIRECTORY_NAME);
        let requested = base.join("review-target123");
        let linked = base.join("review-linked123");
        std::fs::create_dir_all(&requested).unwrap();
        std::fs::write(
            requested.join(REVIEW_MARKER_NAME),
            serde_json::to_vec(&ReviewMarker {
                run_id: "review-linked123".into(),
                data_state: UiReviewDataState::Fixture,
            })
            .unwrap(),
        )
        .unwrap();
        if symlink_dir(&requested, &linked).is_ok() {
            assert!(
                validate_review_root_under(
                    &base,
                    &linked,
                    "review-linked123",
                    UiReviewDataState::Fixture,
                )
                .is_err()
            );
        }
    }
}
