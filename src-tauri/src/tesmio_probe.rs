//! Fail-closed reader for the optional TesmioLoader research companion.
//!
//! The probe remains outside Observatory's process. This module derives one
//! fixed path from the configured game directory, validates strict bounded
//! records, and returns aggregate status. Resource snapshots are stored only
//! after the application has explicit permission and validates every entry.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::model::{
    EnvironmentValidationFacility, EnvironmentValidationSnapshot, TesmioProbeState,
    TesmioProbeStatus,
};

const PROBE_ID: &str = "org.republic-observatory.tesmio-readonly";
const PROBE_FILE: &str = "republic-observatory-probe.jsonl";
const MAX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_LINES: usize = 40_000;
const MAX_LINE_BYTES: usize = 16 * 1024;
const MAX_SAMPLES_PER_SNAPSHOT: u32 = 32;
const MAX_POPULATION: u32 = 500_000;
const MAX_STATUS_VALUE: f32 = 1.5;
const MAX_MONEY_SPENT: f32 = 1.0e12;
const PROBE_SCHEMA_VERSION: u32 = 4;
const LEGACY_PROBE_SCHEMA_VERSION: u32 = 3;
const REVIEWED_PROBE_VERSION: &str = "0.3.0";
const LEGACY_PROBE_VERSION: &str = "0.2.3";
const REVIEWED_GAME_VERSION: &str = "1.1.1.9";
const REVIEWED_EXECUTABLE_TIMESTAMP: u64 = 0x6A3E_B6AD;
// TesmioLoader reports the PE Optional Header's SizeOfImage for the loaded
// module, not the executable's on-disk byte length (10,308,608 bytes).
const REVIEWED_EXECUTABLE_SIZE: u64 = 11_128_832;
const REVIEWED_TESMIO_REVISION: &str = "3baa141f9f08921aea9c95f0a400289cabd9960a";
const REPORT_READ_ATTEMPTS: usize = 3;
const REPORT_READ_RETRY_DELAY: Duration = Duration::from_millis(15);

type ProbeFileContents = (String, Vec<u8>, Option<i64>);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionRecord {
    schema_version: u32,
    record_type: String,
    probe_id: String,
    probe_version: String,
    mode: String,
    loader_api_version: u32,
    target_game_version: String,
    executable_timestamp: u64,
    executable_size: u64,
    game_state_rva: String,
    person_size: u32,
    person_vector_rva: String,
    resource_stride: u32,
    resource_vector_rva: String,
    #[serde(default)]
    checked_session_id: Option<String>,
    #[serde(default)]
    building_vector_rva: Option<String>,
    #[serde(default)]
    facility_candidate_contract_version: Option<u32>,
    #[serde(default)]
    facility_contract_version: Option<u32>,
    writes_game_state: bool,
    writes_save_data: bool,
    writes_observatory_databases: bool,
    network_access: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeStatusRecord {
    schema_version: u32,
    record_type: String,
    stage: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotRecord {
    schema_version: u32,
    record_type: String,
    sequence: u32,
    year: i32,
    day: u16,
    population_count: u32,
    sample_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersonSampleRecord {
    schema_version: u32,
    record_type: String,
    sequence: u32,
    sample_index: u32,
    vector_index: u32,
    year: i32,
    day: u16,
    current_building_present: bool,
    age_years: f32,
    education_level: f32,
    status_happiness: f32,
    status_food: f32,
    status_health: f32,
    status_soviet: f32,
    status_alcohol: f32,
    status_culture: f32,
    status_sport: f32,
    status_religion: f32,
    status_clothing: f32,
    status_electronics: f32,
    status_crime: f32,
    citizen_class: u32,
    money_spent: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceRegistryRecord {
    schema_version: u32,
    record_type: String,
    sequence: u32,
    year: i32,
    day: u16,
    resource_count: u32,
    registry_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceEntryRecord {
    schema_version: u32,
    record_type: String,
    sequence: u32,
    live_index: u32,
    source_token: String,
    caption_id: u32,
    resource_kind: i32,
    transport_class_mask: u32,
    material_family: i32,
    finished_price_rub: f64,
    finished_price_usd: f64,
    base_price_rub: f64,
    base_price_usd: f64,
    sell_multiplier_rub: f64,
    buy_multiplier_rub: f64,
    sell_multiplier_usd: f64,
    buy_multiplier_usd: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FacilityCandidateSnapshotRecord {
    schema_version: u32,
    record_type: String,
    sequence: u32,
    year: i32,
    day: u16,
    facility_count: u32,
    candidate_contract_version: u32,
    collection_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FacilityCandidateRecord {
    schema_version: u32,
    record_type: String,
    sequence: u32,
    facility_index: u32,
    building_type: i32,
    building_subtype: i32,
    finished: bool,
    going_away: bool,
    position_x: Option<f64>,
    position_z: Option<f64>,
    production: Option<f64>,
    pollution: Option<f64>,
    radiation: Option<f64>,
    water_amount: Option<f64>,
    water_capacity: Option<f64>,
    water_quality: Option<f64>,
    sewage_amount: Option<f64>,
    sewage_capacity: Option<f64>,
    sewage_quality: Option<f64>,
    field_mask: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FacilityCandidateCompleteRecord {
    schema_version: u32,
    record_type: String,
    sequence: u32,
    facility_count: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct ValidatedResourceEntry {
    pub live_index: u32,
    pub source_token: String,
    pub caption_id: u32,
    pub resource_kind: i32,
    pub transport_class_mask: u32,
    pub material_family: i32,
    pub finished_price_rub: f64,
    pub finished_price_usd: f64,
    pub base_price_rub: f64,
    pub base_price_usd: f64,
    pub sell_multiplier_rub: f64,
    pub buy_multiplier_rub: f64,
    pub sell_multiplier_usd: f64,
    pub buy_multiplier_usd: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct ValidatedResourceRegistry {
    pub source_content_hash: String,
    pub probe_version: String,
    pub loader_api_version: u32,
    pub target_game_version: String,
    pub executable_timestamp: u64,
    pub executable_size: u64,
    pub year: i32,
    pub day: u16,
    pub registry_fingerprint: String,
    pub entries: Vec<ValidatedResourceEntry>,
}

#[derive(Debug)]
struct ParsedProbe {
    status: TesmioProbeStatus,
    resource_registry: Option<ValidatedResourceRegistry>,
    environment_candidate: Option<EnvironmentValidationSnapshot>,
}

pub fn inspect(media_directory: Option<&Path>) -> TesmioProbeStatus {
    let Some(media_directory) = media_directory else {
        return TesmioProbeStatus::not_configured();
    };
    match inspect_inner(media_directory) {
        Ok(status) => status,
        Err(code) => invalid_status(code),
    }
}

fn inspect_inner(media_directory: &Path) -> Result<TesmioProbeStatus, &'static str> {
    let Some(parsed) = read_parsed_probe(media_directory)? else {
        let mut status = TesmioProbeStatus::not_configured();
        status.state = TesmioProbeState::Missing;
        return Ok(status);
    };
    Ok(parsed.status)
}

pub(crate) fn inspect_resource_registry(
    media_directory: Option<&Path>,
) -> Result<Option<ValidatedResourceRegistry>, &'static str> {
    let Some(media_directory) = media_directory else {
        return Ok(None);
    };
    Ok(read_parsed_probe(media_directory)?.and_then(|parsed| parsed.resource_registry))
}

pub(crate) fn inspect_environment_report(
    media_directory: Option<&Path>,
) -> Result<(TesmioProbeStatus, Option<EnvironmentValidationSnapshot>), &'static str> {
    let Some(media_directory) = media_directory else {
        return Ok((TesmioProbeStatus::not_configured(), None));
    };
    let Some(parsed) = read_parsed_probe(media_directory)? else {
        let mut status = TesmioProbeStatus::not_configured();
        status.state = TesmioProbeState::Missing;
        return Ok((status, None));
    };
    Ok((parsed.status, parsed.environment_candidate))
}

fn read_parsed_probe(media_directory: &Path) -> Result<Option<ParsedProbe>, &'static str> {
    let mut last_error = "probe_unreadable";
    for attempt in 0..REPORT_READ_ATTEMPTS {
        let Some((text, bytes, modified_at_ms)) = read_probe(media_directory)? else {
            return Ok(None);
        };
        match parse_records_full(&text, &bytes) {
            Ok(mut parsed) => {
                parsed.status.last_report_at_ms = modified_at_ms;
                if let Some(candidate) = parsed.environment_candidate.as_mut() {
                    candidate.captured_at_ms = modified_at_ms.unwrap_or(0);
                }
                return Ok(Some(parsed));
            }
            Err(error) => last_error = error,
        }
        // The probe flushes bounded records while W&R is rendering and briefly
        // rewrites this same file during rollover. A second read prevents that
        // normal boundary from appearing as a persistent invalid report.
        if attempt + 1 < REPORT_READ_ATTEMPTS {
            thread::sleep(REPORT_READ_RETRY_DELAY);
        }
    }
    Err(last_error)
}

fn read_probe(media_directory: &Path) -> Result<Option<ProbeFileContents>, &'static str> {
    let canonical_media = media_directory
        .canonicalize()
        .map_err(|_| "game_directory_unavailable")?;
    let game_root = canonical_media
        .parent()
        .ok_or("game_directory_unavailable")?
        .to_path_buf();
    let probe_path = probe_path(&game_root);
    if !probe_path.exists() {
        return Ok(None);
    }
    let link_metadata = fs::symlink_metadata(&probe_path).map_err(|_| "probe_unreadable")?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        return Err("probe_path_not_regular");
    }
    if link_metadata.len() > MAX_BYTES {
        return Err("probe_file_too_large");
    }
    let canonical_probe = probe_path.canonicalize().map_err(|_| "probe_unreadable")?;
    if !canonical_probe.starts_with(&game_root) {
        return Err("probe_outside_game_directory");
    }
    let mut bytes = Vec::with_capacity(usize::try_from(link_metadata.len()).unwrap_or(0));
    fs::File::open(&canonical_probe)
        .map_err(|_| "probe_unreadable")?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "probe_unreadable")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("probe_file_too_large");
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| "probe_not_utf8")?
        .to_owned();
    let modified_at_ms = link_metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok());
    Ok(Some((text, bytes, modified_at_ms)))
}

fn probe_path(game_root: &Path) -> PathBuf {
    let managed = managed_build_root(game_root);
    if managed.exists() {
        managed.join(PROBE_FILE)
    } else {
        game_root
            .join("tesmioloader")
            .join("build")
            .join(PROBE_FILE)
    }
}

fn managed_build_root(game_root: &Path) -> PathBuf {
    game_root.join("tesmioloader").join("observatory")
}

pub(crate) fn verify_observation_only_session(media_directory: Option<&Path>) -> bool {
    let Some(media_directory) = media_directory else {
        return false;
    };
    let Ok(canonical_media) = media_directory.canonicalize() else {
        return false;
    };
    let Some(game_root) = canonical_media.parent() else {
        return false;
    };
    let managed = managed_build_root(game_root);
    let build_root = if managed.exists() {
        managed
    } else {
        game_root.join("tesmioloader").join("build")
    };
    verify_observation_only_build_root(&build_root)
}

pub(crate) fn verify_observation_only_build_root(build_root: &Path) -> bool {
    let config_path = build_root.join("tesmioloader.ini");
    let plugin_root = build_root.join("plugins");
    let Ok(metadata) = fs::symlink_metadata(&config_path) else {
        return false;
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 65_536 {
        return false;
    }
    let Ok(config) = fs::read_to_string(&config_path) else {
        return false;
    };
    if config.starts_with('\u{feff}') {
        return false;
    }
    let Some(values) = parse_loader_configuration(&config) else {
        return false;
    };
    if !observation_only_configuration_is_safe(&values) {
        return false;
    }
    for (key, expected) in [
        ("trace_reads", "0"),
        ("log_game", "0"),
        ("vfs", "0"),
        ("probe_map", "0"),
        ("probe_texel", "0"),
        ("save_manifest", "0"),
        ("plugins", "1"),
        ("menu_patch", "0"),
        ("version_check", "1"),
    ] {
        if values
            .get(&("tesmioloader".to_owned(), key.to_owned()))
            .map(String::as_str)
            != Some(expected)
        {
            return false;
        }
    }
    if values
        .get(&("plugins".to_owned(), "observatory_probe".to_owned()))
        .map(String::as_str)
        != Some("1")
    {
        return false;
    }
    let Ok(entries) = fs::read_dir(&plugin_root) else {
        return false;
    };
    let plugin_dlls = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("dll"))
        })
        .collect::<Vec<_>>();
    plugin_dlls.len() == 1
        && plugin_dlls[0]
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case("observatory_probe.dll")
        && plugin_root.join("observatory_probe.ini").is_file()
}

pub(crate) fn observation_only_configuration_matches(build_root: &Path, expected: &[u8]) -> bool {
    let Ok(actual) = fs::read_to_string(build_root.join("tesmioloader.ini")) else {
        return false;
    };
    let Ok(expected) = std::str::from_utf8(expected) else {
        return false;
    };
    let (Some(mut actual), Some(mut expected)) = (
        parse_loader_configuration(&actual),
        parse_loader_configuration(expected),
    ) else {
        return false;
    };
    if !observation_only_configuration_is_safe(&actual)
        || !observation_only_configuration_is_safe(&expected)
    {
        return false;
    }
    // TesmioLoader writes its own harmless display tag, normalises spacing
    // around '=', and may rewrite the game path to Windows' extended-length
    // spelling on first launch. All operational keys remain allowlisted and
    // must still match the configuration Observatory prepared.
    let game_executable_key = ("tesmioloader".to_owned(), "game_exe".to_owned());
    let (Some(actual_game_executable), Some(expected_game_executable)) = (
        actual.remove(&game_executable_key),
        expected.remove(&game_executable_key),
    ) else {
        return false;
    };
    if !configured_paths_match(&actual_game_executable, &expected_game_executable) {
        return false;
    }
    actual.remove(&("tesmioloader".to_owned(), "menu_tag".to_owned()));
    actual == expected
}

fn configured_paths_match(actual: &str, expected: &str) -> bool {
    let actual = Path::new(actual);
    let expected = Path::new(expected);
    if !actual.is_absolute() || !expected.is_absolute() {
        return false;
    }
    let (Ok(actual), Ok(expected)) = (actual.canonicalize(), expected.canonicalize()) else {
        return false;
    };
    #[cfg(windows)]
    {
        actual
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        actual == expected
    }
}

fn parse_loader_configuration(config: &str) -> Option<BTreeMap<(String, String), String>> {
    if config.starts_with('\u{feff}') {
        return None;
    }
    let mut section = String::new();
    let mut values = BTreeMap::<(String, String), String>::new();
    for raw_line in config.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            section = name.trim().to_ascii_lowercase();
            if section != "tesmioloader" && section != "plugins" {
                return None;
            }
            continue;
        }
        let (key, value) = line.split_once('=')?;
        let identity = (section.clone(), key.trim().to_ascii_lowercase());
        let value = value.trim();
        if section.is_empty()
            || value.chars().any(char::is_control)
            || values.insert(identity, value.to_owned()).is_some()
        {
            return None;
        }
    }
    Some(values)
}

fn observation_only_configuration_is_safe(values: &BTreeMap<(String, String), String>) -> bool {
    if values.keys().any(|(section, key)| match section.as_str() {
        "tesmioloader" => !matches!(
            key.as_str(),
            "version"
                | "game_exe"
                | "trace_reads"
                | "log_game"
                | "vfs"
                | "probe_map"
                | "probe_texel"
                | "save_manifest"
                | "plugins"
                | "menu_patch"
                | "version_check"
                | "menu_tag"
        ),
        "plugins" => key != "observatory_probe",
        _ => true,
    }) {
        return false;
    }
    let value = |section: &str, key: &str| {
        values
            .get(&(section.to_owned(), key.to_owned()))
            .map(String::as_str)
    };
    let expected_version = format!("observatory-{REVIEWED_TESMIO_REVISION}");
    if value("tesmioloader", "version") != Some(expected_version.as_str())
        || value("plugins", "observatory_probe") != Some("1")
    {
        return false;
    }
    let Some(game_executable) = value("tesmioloader", "game_exe") else {
        return false;
    };
    if game_executable.is_empty() || !Path::new(game_executable).is_absolute() {
        return false;
    }
    if let Some(menu_tag) = value("tesmioloader", "menu_tag")
        && menu_tag != format!("tesmioloader v. {expected_version}")
    {
        return false;
    }
    true
}

#[cfg(test)]
fn parse_records(text: &str, bytes: &[u8]) -> Result<TesmioProbeStatus, &'static str> {
    Ok(parse_records_full(text, bytes)?.status)
}

fn parse_records_full(text: &str, bytes: &[u8]) -> Result<ParsedProbe, &'static str> {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.is_empty() || lines.len() > MAX_LINES {
        return Err("probe_line_count_invalid");
    }
    if lines.iter().any(|line| line.len() > MAX_LINE_BYTES) {
        return Err("probe_line_too_long");
    }
    let session: SessionRecord = serde_json::from_str(lines[0]).map_err(|_| "session_invalid")?;
    validate_session(&session)?;

    let mut snapshots = BTreeMap::<u32, SnapshotRecord>::new();
    let mut observed_samples = BTreeMap::<u32, u32>::new();
    let mut sample_indices = BTreeMap::<u32, BTreeSet<u32>>::new();
    let mut sample_memberships = Vec::<(u32, u32, u32, i32, u16)>::new();
    let mut registries = BTreeMap::<u32, ResourceRegistryRecord>::new();
    let mut resource_entries = BTreeMap::<u32, Vec<ResourceEntryRecord>>::new();
    let mut resource_tokens = BTreeMap::<u32, BTreeSet<String>>::new();
    let mut resource_indices = BTreeMap::<u32, BTreeSet<u32>>::new();
    let mut facility_snapshots = BTreeMap::<u32, FacilityCandidateSnapshotRecord>::new();
    let mut facility_rows = BTreeMap::<u32, Vec<FacilityCandidateRecord>>::new();
    let mut facility_indices = BTreeMap::<u32, BTreeSet<u32>>::new();
    let mut facility_completions = BTreeMap::<u32, FacilityCandidateCompleteRecord>::new();
    let mut collection_stage = None;
    for line in &lines[1..] {
        let value: serde_json::Value = serde_json::from_str(line).map_err(|_| "record_invalid")?;
        match value.get("record_type").and_then(serde_json::Value::as_str) {
            Some("snapshot") => {
                let record: SnapshotRecord =
                    serde_json::from_value(value).map_err(|_| "snapshot_invalid")?;
                validate_snapshot(&record, session.schema_version)?;
                if snapshots.insert(record.sequence, record).is_some() {
                    return Err("duplicate_snapshot_sequence");
                }
            }
            Some("person_sample") => {
                let record: PersonSampleRecord =
                    serde_json::from_value(value).map_err(|_| "person_sample_invalid")?;
                validate_person_sample(&record, session.schema_version)?;
                if !sample_indices
                    .entry(record.sequence)
                    .or_default()
                    .insert(record.sample_index)
                {
                    return Err("duplicate_sample_index");
                }
                *observed_samples.entry(record.sequence).or_default() += 1;
                sample_memberships.push((
                    record.sequence,
                    record.sample_index,
                    record.vector_index,
                    record.year,
                    record.day,
                ));
            }
            Some("resource_registry") => {
                let record: ResourceRegistryRecord =
                    serde_json::from_value(value).map_err(|_| "resource_registry_invalid")?;
                validate_resource_registry(&record, session.schema_version)?;
                if registries.insert(record.sequence, record).is_some() {
                    return Err("duplicate_resource_registry_sequence");
                }
            }
            Some("resource_entry") => {
                let record: ResourceEntryRecord =
                    serde_json::from_value(value).map_err(|_| "resource_entry_invalid")?;
                validate_resource_entry(&record, session.schema_version)?;
                if !resource_tokens
                    .entry(record.sequence)
                    .or_default()
                    .insert(record.source_token.clone())
                    || !resource_indices
                        .entry(record.sequence)
                        .or_default()
                        .insert(record.live_index)
                {
                    return Err("duplicate_resource_identity");
                }
                resource_entries
                    .entry(record.sequence)
                    .or_default()
                    .push(record);
            }
            Some("facility_candidate_snapshot") => {
                let record: FacilityCandidateSnapshotRecord = serde_json::from_value(value)
                    .map_err(|_| "facility_candidate_snapshot_invalid")?;
                validate_facility_snapshot(&record, session.schema_version)?;
                if facility_snapshots.insert(record.sequence, record).is_some() {
                    return Err("duplicate_facility_snapshot_sequence");
                }
            }
            Some("facility_candidate") => {
                let record: FacilityCandidateRecord =
                    serde_json::from_value(value).map_err(|_| "facility_candidate_invalid")?;
                validate_facility_candidate(&record, session.schema_version)?;
                if !facility_indices
                    .entry(record.sequence)
                    .or_default()
                    .insert(record.facility_index)
                {
                    return Err("duplicate_facility_index");
                }
                facility_rows
                    .entry(record.sequence)
                    .or_default()
                    .push(record);
            }
            Some("facility_candidate_complete") => {
                let record: FacilityCandidateCompleteRecord = serde_json::from_value(value)
                    .map_err(|_| "facility_candidate_complete_invalid")?;
                validate_facility_complete(&record, session.schema_version)?;
                if facility_completions
                    .insert(record.sequence, record)
                    .is_some()
                {
                    return Err("duplicate_facility_complete_sequence");
                }
            }
            Some("probe_status") => {
                let record: ProbeStatusRecord =
                    serde_json::from_value(value).map_err(|_| "probe_status_invalid")?;
                validate_probe_status(&record, session.schema_version)?;
                collection_stage = Some(record.stage);
            }
            _ => return Err("unknown_record_type"),
        }
    }
    for (sequence, observed) in &observed_samples {
        let snapshot = snapshots.get(sequence).ok_or("sample_without_snapshot")?;
        if *observed != snapshot.sample_count {
            return Err("sample_count_mismatch");
        }
    }
    if snapshots.values().any(|snapshot| {
        observed_samples
            .get(&snapshot.sequence)
            .copied()
            .unwrap_or(0)
            != snapshot.sample_count
    }) {
        return Err("sample_count_mismatch");
    }
    for (sequence, sample_index, vector_index, year, day) in sample_memberships {
        let snapshot = snapshots.get(&sequence).ok_or("sample_without_snapshot")?;
        if sample_index >= snapshot.sample_count
            || vector_index >= snapshot.population_count
            || year != snapshot.year
            || day != snapshot.day
        {
            return Err("sample_snapshot_mismatch");
        }
    }

    for (sequence, registry) in &registries {
        let entries = resource_entries
            .get(sequence)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if entries.len() != registry.resource_count as usize
            || entries
                .iter()
                .any(|entry| entry.live_index >= registry.resource_count)
        {
            return Err("resource_count_mismatch");
        }
    }
    if resource_entries
        .keys()
        .any(|sequence| !registries.contains_key(sequence))
    {
        return Err("resource_entry_without_registry");
    }

    // Candidate rows are deliberately ignored until their matching completion
    // marker is present. This keeps a report being written by W&R from becoming
    // apparent evidence in Observatory.
    for (sequence, completion) in &facility_completions {
        let snapshot = facility_snapshots
            .get(sequence)
            .ok_or("facility_complete_without_snapshot")?;
        let rows = facility_rows
            .get(sequence)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if completion.facility_count != snapshot.facility_count
            || rows.len() != snapshot.facility_count as usize
            || rows
                .iter()
                .any(|row| row.facility_index >= snapshot.facility_count)
        {
            return Err("facility_count_mismatch");
        }
    }
    if facility_rows
        .keys()
        .any(|sequence| !facility_snapshots.contains_key(sequence))
    {
        return Err("facility_row_without_snapshot");
    }

    // Sequence, not game date, is the session ordering authority. Loading an
    // older save may legitimately make the date move backwards.
    let latest = snapshots.values().next_back();
    let mut warnings = Vec::new();
    if collection_stage.as_deref() == Some("checked_report_ready_without_resources") {
        warnings.push("resource_registry_unavailable".to_owned());
    } else if collection_stage.as_deref() == Some("stopped_at_record_limit") {
        warnings.push("probe_record_limit_reached".to_owned());
    } else if collection_stage.as_deref() == Some("facility_candidate_rejected") {
        warnings.push("facility_candidate_rejected".to_owned());
    }
    let telemetry_content_hash = hex_hash(bytes);
    let status = TesmioProbeStatus {
        state: if warnings.is_empty() {
            TesmioProbeState::Available
        } else {
            TesmioProbeState::Warning
        },
        read_only: true,
        optional: true,
        persisted: false,
        probe_id: Some(session.probe_id.clone()),
        probe_version: Some(session.probe_version.clone()),
        loader_api_version: Some(session.loader_api_version),
        target_game_version: Some(session.target_game_version.clone()),
        executable_timestamp: Some(session.executable_timestamp),
        content_hash: Some(telemetry_content_hash),
        snapshot_count: u32::try_from(snapshots.len()).unwrap_or(u32::MAX),
        sample_count: observed_samples.values().sum(),
        latest_year: latest.map(|record| record.year),
        latest_day: latest.map(|record| record.day),
        latest_population_count: latest.map(|record| record.population_count),
        collection_stage,
        people_readings_ready: !snapshots.is_empty(),
        resource_readings_ready: !registries.is_empty(),
        environment_readings_ready: session.facility_contract_version.is_some(),
        facility_contract_version: session.facility_contract_version,
        last_report_at_ms: None,
        warnings,
    };
    let resource_registry = registries.iter().next_back().map(|(sequence, registry)| {
        let entries: Vec<ValidatedResourceEntry> = resource_entries
            .remove(sequence)
            .unwrap_or_default()
            .into_iter()
            .map(|entry| ValidatedResourceEntry {
                live_index: entry.live_index,
                source_token: entry.source_token,
                caption_id: entry.caption_id,
                resource_kind: entry.resource_kind,
                transport_class_mask: entry.transport_class_mask,
                material_family: entry.material_family,
                finished_price_rub: entry.finished_price_rub,
                finished_price_usd: entry.finished_price_usd,
                base_price_rub: entry.base_price_rub,
                base_price_usd: entry.base_price_usd,
                sell_multiplier_rub: entry.sell_multiplier_rub,
                buy_multiplier_rub: entry.buy_multiplier_rub,
                sell_multiplier_usd: entry.sell_multiplier_usd,
                buy_multiplier_usd: entry.buy_multiplier_usd,
            })
            .collect();
        let source_content_hash = resource_content_hash(&session, registry, &entries);
        ValidatedResourceRegistry {
            source_content_hash,
            probe_version: session.probe_version.clone(),
            loader_api_version: session.loader_api_version,
            target_game_version: session.target_game_version.clone(),
            executable_timestamp: session.executable_timestamp,
            executable_size: session.executable_size,
            year: registry.year,
            day: registry.day,
            registry_fingerprint: registry.registry_fingerprint.clone(),
            entries,
        }
    });
    let environment_candidate = facility_completions
        .keys()
        .next_back()
        .and_then(|sequence| {
            let snapshot = facility_snapshots.get(sequence)?;
            let rows = facility_rows.get(sequence)?;
            Some(environment_candidate_snapshot(&session, snapshot, rows))
        });
    Ok(ParsedProbe {
        status,
        resource_registry,
        environment_candidate,
    })
}

fn environment_candidate_snapshot(
    session: &SessionRecord,
    snapshot: &FacilityCandidateSnapshotRecord,
    rows: &[FacilityCandidateRecord],
) -> EnvironmentValidationSnapshot {
    let facilities = rows
        .iter()
        .map(|row| EnvironmentValidationFacility {
            facility_index: row.facility_index,
            building_type: row.building_type,
            building_subtype: row.building_subtype,
            finished: row.finished,
            going_away: row.going_away,
            position_x: row.position_x,
            position_z: row.position_z,
            production: row.production,
            pollution: row.pollution,
            radiation: row.radiation,
            water_amount: row.water_amount,
            water_capacity: row.water_capacity,
            water_quality: row.water_quality,
            sewage_amount: row.sewage_amount,
            sewage_capacity: row.sewage_capacity,
            sewage_quality: row.sewage_quality,
        })
        .collect::<Vec<_>>();
    let mut hasher = Sha256::new();
    hasher.update(b"republic-observatory-environment-candidate.v1\0");
    hasher.update(session.probe_id.as_bytes());
    hasher.update(session.probe_version.as_bytes());
    hasher.update(session.target_game_version.as_bytes());
    hasher.update(session.executable_timestamp.to_le_bytes());
    hasher.update(session.executable_size.to_le_bytes());
    hasher.update(snapshot.year.to_le_bytes());
    hasher.update(snapshot.day.to_le_bytes());
    hasher.update(snapshot.candidate_contract_version.to_le_bytes());
    hasher.update(snapshot.collection_fingerprint.as_bytes());
    for row in rows {
        hasher.update(row.facility_index.to_le_bytes());
        hasher.update(row.building_type.to_le_bytes());
        hasher.update(row.building_subtype.to_le_bytes());
        hasher.update([u8::from(row.finished), u8::from(row.going_away)]);
        for value in [
            row.position_x,
            row.position_z,
            row.production,
            row.pollution,
            row.radiation,
            row.water_amount,
            row.water_capacity,
            row.water_quality,
            row.sewage_amount,
            row.sewage_capacity,
            row.sewage_quality,
        ] {
            hasher.update(value.map(f64::to_bits).unwrap_or(u64::MAX).to_le_bytes());
        }
    }
    let snapshot_id = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    EnvironmentValidationSnapshot {
        snapshot_id,
        checked_session_id: session
            .checked_session_id
            .clone()
            .unwrap_or_else(|| "legacy-session".to_owned()),
        candidate_contract_version: snapshot.candidate_contract_version,
        probe_version: session.probe_version.clone(),
        game_build_id: format!(
            "{}:{:08X}:{}",
            session.target_game_version, session.executable_timestamp, session.executable_size
        ),
        year: snapshot.year,
        day: snapshot.day,
        game_day: i64::from(snapshot.year) * 365 + i64::from(snapshot.day),
        captured_at_ms: 0,
        collection_fingerprint: snapshot.collection_fingerprint.clone(),
        facilities,
    }
}

fn resource_content_hash(
    session: &SessionRecord,
    registry: &ResourceRegistryRecord,
    entries: &[ValidatedResourceEntry],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"republic-observatory-resource-registry-content.v1\0");
    hasher.update(session.probe_id.as_bytes());
    hasher.update(session.probe_version.as_bytes());
    hasher.update(session.loader_api_version.to_le_bytes());
    hasher.update(session.target_game_version.as_bytes());
    hasher.update(session.executable_timestamp.to_le_bytes());
    hasher.update(session.executable_size.to_le_bytes());
    hasher.update(registry.year.to_le_bytes());
    hasher.update(registry.day.to_le_bytes());
    hasher.update(registry.registry_fingerprint.as_bytes());
    hasher.update((entries.len() as u32).to_le_bytes());
    for entry in entries {
        hasher.update(entry.live_index.to_le_bytes());
        hasher.update((entry.source_token.len() as u32).to_le_bytes());
        hasher.update(entry.source_token.as_bytes());
        hasher.update(entry.caption_id.to_le_bytes());
        hasher.update(entry.resource_kind.to_le_bytes());
        hasher.update(entry.transport_class_mask.to_le_bytes());
        hasher.update(entry.material_family.to_le_bytes());
        for value in [
            entry.finished_price_rub,
            entry.finished_price_usd,
            entry.base_price_rub,
            entry.base_price_usd,
            entry.sell_multiplier_rub,
            entry.buy_multiplier_rub,
            entry.sell_multiplier_usd,
            entry.buy_multiplier_usd,
        ] {
            hasher.update(value.to_bits().to_le_bytes());
        }
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_session(record: &SessionRecord) -> Result<(), &'static str> {
    let legacy = record.schema_version == LEGACY_PROBE_SCHEMA_VERSION;
    let current = record.schema_version == PROBE_SCHEMA_VERSION;
    if (!legacy && !current)
        || record.record_type != "session"
        || record.probe_id != PROBE_ID
        || record.mode != "read_only"
        || (legacy && record.probe_version != LEGACY_PROBE_VERSION)
        || (current && record.probe_version != REVIEWED_PROBE_VERSION)
        || record.loader_api_version != 4
        || record.target_game_version != REVIEWED_GAME_VERSION
        || record.executable_timestamp != REVIEWED_EXECUTABLE_TIMESTAMP
        || record.executable_size != REVIEWED_EXECUTABLE_SIZE
        || record.game_state_rva != "0x9D4F10"
        || record.person_size != 0x750
        || record.person_vector_rva != "0x9E75B8"
        || record.resource_stride != 0x340
        || record.resource_vector_rva != "0x9E11C0"
        || (legacy
            && (record.checked_session_id.is_some()
                || record.building_vector_rva.is_some()
                || record.facility_candidate_contract_version.is_some()
                || record.facility_contract_version.is_some()))
        || (current
            && (record.checked_session_id.as_deref().is_none_or(|value| {
                value.len() != 24 || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
            }) || record.building_vector_rva.as_deref() != Some("0x9E6A18")
                || record.facility_candidate_contract_version != Some(1)))
        || record.writes_game_state
        || record.writes_save_data
        || record.writes_observatory_databases
        || record.network_access
    {
        return Err("session_contract_mismatch");
    }
    Ok(())
}

fn validate_probe_status(
    record: &ProbeStatusRecord,
    schema_version: u32,
) -> Result<(), &'static str> {
    if record.schema_version != schema_version
        || record.record_type != "probe_status"
        || !matches!(
            record.stage.as_str(),
            "waiting_for_game_state"
                | "waiting_for_loaded_republic"
                | "checked_report_ready"
                | "checked_report_ready_without_resources"
                | "checked_report_ready_without_facilities"
                | "facility_candidate_collecting"
                | "facility_candidate_publishing"
                | "facility_candidate_ready"
                | "facility_candidate_rejected"
                | "stopped_at_record_limit"
        )
    {
        return Err("probe_status_out_of_bounds");
    }
    Ok(())
}

fn validate_snapshot(record: &SnapshotRecord, schema_version: u32) -> Result<(), &'static str> {
    if record.schema_version != schema_version
        || record.record_type != "snapshot"
        || record.sequence == 0
        || !(1900..=10_000).contains(&record.year)
        || record.day > 365
        || record.population_count > MAX_POPULATION
        || record.sample_count > MAX_SAMPLES_PER_SNAPSHOT
        || record.sample_count > record.population_count
    {
        return Err("snapshot_out_of_bounds");
    }
    Ok(())
}

fn validate_person_sample(
    record: &PersonSampleRecord,
    schema_version: u32,
) -> Result<(), &'static str> {
    let status_values = [
        record.status_happiness,
        record.status_food,
        record.status_health,
        record.status_soviet,
        record.status_alcohol,
        record.status_culture,
        record.status_sport,
        record.status_religion,
        record.status_clothing,
        record.status_electronics,
        record.status_crime,
    ];
    if record.schema_version != schema_version
        || record.record_type != "person_sample"
        || record.sequence == 0
        || record.sample_index >= MAX_SAMPLES_PER_SNAPSHOT
        || !(1900..=10_000).contains(&record.year)
        || record.day > 365
        || record.citizen_class > 2
        || !record.age_years.is_finite()
        || !record.education_level.is_finite()
        || !record.money_spent.is_finite()
        || status_values
            .iter()
            .any(|value| !(0.0..=MAX_STATUS_VALUE).contains(value))
        || !(0.0..=200.0).contains(&record.age_years)
        || !(0.0..=3.0).contains(&record.education_level)
        || !(0.0..=MAX_MONEY_SPENT).contains(&record.money_spent)
    {
        return Err("person_sample_out_of_bounds");
    }
    let _ = (
        record.vector_index,
        record.year,
        record.current_building_present,
    );
    Ok(())
}

fn validate_resource_registry(
    record: &ResourceRegistryRecord,
    schema_version: u32,
) -> Result<(), &'static str> {
    if record.schema_version != schema_version
        || record.record_type != "resource_registry"
        || record.sequence == 0
        || !(1900..=10_000).contains(&record.year)
        || record.day > 365
        || !(1..=512).contains(&record.resource_count)
        || record.registry_fingerprint.len() != 16
        || !record
            .registry_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("resource_registry_out_of_bounds");
    }
    Ok(())
}

fn validate_resource_entry(
    record: &ResourceEntryRecord,
    schema_version: u32,
) -> Result<(), &'static str> {
    let numeric = [
        record.finished_price_rub,
        record.finished_price_usd,
        record.base_price_rub,
        record.base_price_usd,
        record.sell_multiplier_rub,
        record.buy_multiplier_rub,
        record.sell_multiplier_usd,
        record.buy_multiplier_usd,
    ];
    if record.schema_version != schema_version
        || record.record_type != "resource_entry"
        || record.sequence == 0
        || record.live_index >= 512
        || record.source_token.is_empty()
        || record.source_token.len() > 128
        || !record
            .source_token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        || !(-64..=64).contains(&record.resource_kind)
        || record.transport_class_mask > 0x3ffff
        || !(-1..=255).contains(&record.material_family)
        || numeric.iter().any(|value| !value.is_finite())
        || numeric[..4]
            .iter()
            .any(|value| !(0.0..=1.0e12).contains(value))
        || numeric[4..]
            .iter()
            .any(|value| !(0.0..=100.0).contains(value))
    {
        return Err("resource_entry_out_of_bounds");
    }
    let _ = record.caption_id;
    Ok(())
}

fn validate_facility_snapshot(
    record: &FacilityCandidateSnapshotRecord,
    schema_version: u32,
) -> Result<(), &'static str> {
    if record.schema_version != schema_version
        || schema_version != PROBE_SCHEMA_VERSION
        || record.record_type != "facility_candidate_snapshot"
        || record.sequence == 0
        || !(1900..=10_000).contains(&record.year)
        || record.day > 365
        || !(1..=25_000).contains(&record.facility_count)
        || record.candidate_contract_version != 1
        || record.collection_fingerprint.len() != 16
        || !record
            .collection_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("facility_candidate_snapshot_out_of_bounds");
    }
    Ok(())
}

fn validate_facility_candidate(
    record: &FacilityCandidateRecord,
    schema_version: u32,
) -> Result<(), &'static str> {
    const PRODUCTION: u32 = 1;
    const POLLUTION: u32 = 2;
    const WATER: u32 = 4;
    const SEWAGE: u32 = 8;
    let value_in = |value: Option<f64>, maximum: f64| {
        value.is_none_or(|value| value.is_finite() && (0.0..=maximum).contains(&value))
    };
    let present =
        |value: Option<f64>, mask: u32| value.is_some() == (record.field_mask & mask != 0);
    if record.schema_version != schema_version
        || schema_version != PROBE_SCHEMA_VERSION
        || record.record_type != "facility_candidate"
        || record.sequence == 0
        || record.facility_index >= 25_000
        || !(0..=255).contains(&record.building_type)
        || !(-1..=4096).contains(&record.building_subtype)
        || record.position_x.is_some()
        || record.position_z.is_some()
        || record.radiation.is_some()
        || record.field_mask & !(PRODUCTION | POLLUTION | WATER | SEWAGE) != 0
        || !present(record.production, PRODUCTION)
        || !present(record.pollution, POLLUTION)
        || !present(record.water_amount, WATER)
        || !present(record.water_capacity, WATER)
        || !present(record.water_quality, WATER)
        || !present(record.sewage_amount, SEWAGE)
        || !present(record.sewage_capacity, SEWAGE)
        || !present(record.sewage_quality, SEWAGE)
        || !value_in(record.production, 1.0e12)
        || !value_in(record.pollution, 3.0)
        || !value_in(record.water_amount, 1.0e12)
        || !value_in(record.water_capacity, 1.0e12)
        || !value_in(record.water_quality, 1.5)
        || !value_in(record.sewage_amount, 1.0e12)
        || !value_in(record.sewage_capacity, 1.0e12)
        || !value_in(record.sewage_quality, 1.5)
    {
        return Err("facility_candidate_out_of_bounds");
    }
    let _ = (record.finished, record.going_away);
    Ok(())
}

fn validate_facility_complete(
    record: &FacilityCandidateCompleteRecord,
    schema_version: u32,
) -> Result<(), &'static str> {
    if record.schema_version != schema_version
        || schema_version != PROBE_SCHEMA_VERSION
        || record.record_type != "facility_candidate_complete"
        || record.sequence == 0
        || !(1..=25_000).contains(&record.facility_count)
    {
        return Err("facility_candidate_complete_out_of_bounds");
    }
    Ok(())
}

fn invalid_status(code: &'static str) -> TesmioProbeStatus {
    let mut status = TesmioProbeStatus::not_configured();
    status.state = TesmioProbeState::Invalid;
    status.warnings.push(code.to_owned());
    status
}

fn hex_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{PROBE_ID, inspect, parse_records, parse_records_full};
    use crate::model::TesmioProbeState;

    fn session() -> String {
        format!(
            r#"{{"schema_version":3,"record_type":"session","probe_id":"{PROBE_ID}","probe_version":"0.2.3","mode":"read_only","loader_api_version":4,"target_game_version":"1.1.1.9","executable_timestamp":1782494893,"executable_size":11128832,"game_state_rva":"0x9D4F10","person_size":1872,"person_vector_rva":"0x9E75B8","resource_stride":832,"resource_vector_rva":"0x9E11C0","writes_game_state":false,"writes_save_data":false,"writes_observatory_databases":false,"network_access":false}}"#
        )
    }

    fn probe_status(stage: &str) -> String {
        format!(r#"{{"schema_version":3,"record_type":"probe_status","stage":"{stage}"}}"#)
    }

    fn snapshot(sequence: u32, year: i32, day: u16, count: u32) -> String {
        format!(
            r#"{{"schema_version":3,"record_type":"snapshot","sequence":{sequence},"year":{year},"day":{day},"population_count":100,"sample_count":{count}}}"#
        )
    }

    fn sample(sequence: u32, year: i32, day: u16) -> String {
        format!(
            r#"{{"schema_version":3,"record_type":"person_sample","sequence":{sequence},"sample_index":0,"vector_index":4,"year":{year},"day":{day},"current_building_present":true,"age_years":42.0,"education_level":2.0,"status_happiness":0.5,"status_food":0.5,"status_health":0.5,"status_soviet":0.5,"status_alcohol":0.5,"status_culture":0.5,"status_sport":0.5,"status_religion":0.5,"status_clothing":0.5,"status_electronics":0.5,"status_crime":0.5,"citizen_class":0,"money_spent":0.0}}"#
        )
    }

    fn resource_registry(sequence: u32, count: u32) -> String {
        format!(
            r#"{{"schema_version":3,"record_type":"resource_registry","sequence":{sequence},"year":2018,"day":42,"resource_count":{count},"registry_fingerprint":"0123456789abcdef"}}"#
        )
    }

    fn resource_entry(sequence: u32, index: u32, token: &str) -> String {
        format!(
            r#"{{"schema_version":3,"record_type":"resource_entry","sequence":{sequence},"live_index":{index},"source_token":"{token}","caption_id":500,"resource_kind":2,"transport_class_mask":3,"material_family":13,"finished_price_rub":12.5,"finished_price_usd":9.5,"base_price_rub":3.0,"base_price_usd":2.0,"sell_multiplier_rub":0.95,"buy_multiplier_rub":1.05,"sell_multiplier_usd":0.95,"buy_multiplier_usd":1.05}}"#
        )
    }

    fn session_v4() -> String {
        format!(
            r#"{{"schema_version":4,"record_type":"session","probe_id":"{PROBE_ID}","probe_version":"0.3.0","mode":"read_only","loader_api_version":4,"target_game_version":"1.1.1.9","executable_timestamp":1782494893,"executable_size":11128832,"checked_session_id":"000012340000000000ABCDEF","game_state_rva":"0x9D4F10","person_size":1872,"person_vector_rva":"0x9E75B8","resource_stride":832,"resource_vector_rva":"0x9E11C0","building_vector_rva":"0x9E6A18","facility_candidate_contract_version":1,"facility_contract_version":null,"writes_game_state":false,"writes_save_data":false,"writes_observatory_databases":false,"network_access":false}}"#
        )
    }

    fn facility_header(count: u32) -> String {
        format!(
            r#"{{"schema_version":4,"record_type":"facility_candidate_snapshot","sequence":3,"year":2022,"day":42,"facility_count":{count},"candidate_contract_version":1,"collection_fingerprint":"0123456789abcdef"}}"#
        )
    }

    fn facility_row(index: u32) -> String {
        format!(
            r#"{{"schema_version":4,"record_type":"facility_candidate","sequence":3,"facility_index":{index},"building_type":2,"building_subtype":1,"finished":true,"going_away":false,"position_x":null,"position_z":null,"production":null,"pollution":0.25,"radiation":null,"water_amount":12.5,"water_capacity":20.0,"water_quality":0.98,"sewage_amount":null,"sewage_capacity":null,"sewage_quality":null,"field_mask":6}}"#
        )
    }

    fn facility_complete(count: u32) -> String {
        format!(
            r#"{{"schema_version":4,"record_type":"facility_candidate_complete","sequence":3,"facility_count":{count}}}"#
        )
    }

    #[test]
    fn reports_the_latest_bounded_collection_stage() {
        let text = [
            session(),
            probe_status("waiting_for_loaded_republic"),
            snapshot(1, 2022, 16, 1),
            sample(1, 2022, 16),
            probe_status("checked_report_ready"),
        ]
        .join("\n");
        let status = parse_records(&text, text.as_bytes()).expect("valid probe");
        assert_eq!(
            status.collection_stage.as_deref(),
            Some("checked_report_ready")
        );
        assert_eq!(status.snapshot_count, 1);
    }

    #[test]
    fn rejects_an_unreviewed_collection_stage() {
        let text = [session(), probe_status("read_arbitrary_memory")].join("\n");
        assert_eq!(
            parse_records(&text, text.as_bytes()).expect_err("stage must be rejected"),
            "probe_status_out_of_bounds"
        );
    }

    #[test]
    fn accepts_bounded_read_only_records_and_date_regression() {
        let text = [
            session(),
            snapshot(1, 2014, 50, 1),
            sample(1, 2014, 50),
            snapshot(2, 2013, 20, 1),
            sample(2, 2013, 20),
        ]
        .join("\n");
        let status = parse_records(&text, text.as_bytes()).expect("valid probe");
        assert_eq!(status.state, TesmioProbeState::Available);
        assert_eq!(status.snapshot_count, 2);
        assert_eq!(status.latest_year, Some(2013));
        assert!(!status.persisted);
    }

    #[test]
    fn rejects_write_capabilities_unknown_fields_and_mismatched_samples() {
        let writing =
            session().replace("\"writes_game_state\":false", "\"writes_game_state\":true");
        assert_eq!(
            parse_records(&writing, writing.as_bytes()).unwrap_err(),
            "session_contract_mismatch"
        );

        let unknown = session().replace(
            "\"network_access\":false",
            "\"network_access\":false,\"script\":\"x\"",
        );
        assert_eq!(
            parse_records(&unknown, unknown.as_bytes()).unwrap_err(),
            "session_invalid"
        );

        let mismatched = [session(), snapshot(1, 2014, 50, 1)].join("\n");
        assert_eq!(
            parse_records(&mismatched, mismatched.as_bytes()).unwrap_err(),
            "sample_count_mismatch"
        );

        let implausible_status = [
            session(),
            snapshot(1, 2014, 50, 1),
            sample(1, 2014, 50).replace("\"status_health\":0.5", "\"status_health\":2.0"),
        ]
        .join("\n");
        assert_eq!(
            parse_records(&implausible_status, implausible_status.as_bytes()).unwrap_err(),
            "person_sample_out_of_bounds"
        );
    }

    #[test]
    fn accepts_one_complete_dynamic_resource_registry_and_rejects_duplicates() {
        let text = [
            session(),
            snapshot(1, 2018, 42, 0),
            resource_registry(2, 2),
            resource_entry(2, 0, "eletronics"),
            resource_entry(2, 1, "runtime_crystal"),
        ]
        .join("\n");
        let parsed = parse_records_full(&text, text.as_bytes()).expect("valid registry");
        let registry = parsed.resource_registry.expect("resource registry");
        assert_eq!(registry.entries.len(), 2);
        assert_eq!(registry.entries[1].source_token, "runtime_crystal");

        let duplicate = [
            session(),
            resource_registry(2, 2),
            resource_entry(2, 0, "same"),
            resource_entry(2, 1, "same"),
        ]
        .join("\n");
        assert_eq!(
            parse_records_full(&duplicate, duplicate.as_bytes()).unwrap_err(),
            "duplicate_resource_identity"
        );
    }

    #[test]
    fn derives_one_fixed_location_and_keeps_an_absent_probe_optional() {
        let directory = tempdir().expect("temporary game directory");
        let media = directory.path().join("media_soviet");
        fs::create_dir(&media).expect("media directory");
        assert_eq!(inspect(Some(&media)).state, TesmioProbeState::Missing);

        let build = directory.path().join("tesmioloader").join("build");
        fs::create_dir_all(&build).expect("probe directory");
        let text = [session(), snapshot(1, 2014, 50, 0)].join("\n");
        fs::write(build.join("republic-observatory-probe.jsonl"), text)
            .expect("synthetic probe stream");
        let status = inspect(Some(&media));
        assert_eq!(status.state, TesmioProbeState::Available);
        assert_eq!(status.snapshot_count, 1);
        assert!(!status.persisted);
    }

    #[test]
    fn publishes_only_a_complete_bounded_facility_candidate_snapshot() {
        let incomplete = [session_v4(), facility_header(1), facility_row(0)].join("\n");
        let parsed = parse_records_full(&incomplete, incomplete.as_bytes())
            .expect("a report being written remains valid");
        assert!(parsed.environment_candidate.is_none());

        let complete = [
            session_v4(),
            facility_header(1),
            facility_row(0),
            facility_complete(1),
            r#"{"schema_version":4,"record_type":"probe_status","stage":"facility_candidate_ready"}"#.to_owned(),
        ]
        .join("\n");
        let parsed =
            parse_records_full(&complete, complete.as_bytes()).expect("candidate snapshot");
        let candidate = parsed.environment_candidate.expect("complete candidate");
        assert_eq!(candidate.facilities.len(), 1);
        assert_eq!(candidate.facilities[0].pollution, Some(0.25));
        assert!(!parsed.status.environment_readings_ready);
        assert_eq!(parsed.status.facility_contract_version, None);
    }

    #[test]
    fn rejects_duplicate_or_unbounded_facility_candidate_rows() {
        let duplicate = [
            session_v4(),
            facility_header(2),
            facility_row(0),
            facility_row(0),
            facility_complete(2),
        ]
        .join("\n");
        assert_eq!(
            parse_records_full(&duplicate, duplicate.as_bytes()).unwrap_err(),
            "duplicate_facility_index"
        );

        let invalid = [
            session_v4(),
            facility_header(1),
            facility_row(0).replace("\"pollution\":0.25", "\"pollution\":9.0"),
            facility_complete(1),
        ]
        .join("\n");
        assert_eq!(
            parse_records_full(&invalid, invalid.as_bytes()).unwrap_err(),
            "facility_candidate_out_of_bounds"
        );
    }
}
