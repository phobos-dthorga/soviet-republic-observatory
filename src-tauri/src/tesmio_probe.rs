//! Fail-closed reader for the optional TesmioLoader research companion.
//!
//! The probe remains outside Observatory's process. This module derives one
//! fixed path from the configured game directory, validates strict bounded
//! records, and returns aggregate status only. It never stores probe payloads.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::model::{TesmioProbeState, TesmioProbeStatus};

const PROBE_ID: &str = "org.republic-observatory.tesmio-readonly";
const PROBE_FILE: &str = "republic-observatory-probe.jsonl";
const MAX_BYTES: u64 = 4 * 1024 * 1024;
const MAX_LINES: usize = 8_192;
const MAX_LINE_BYTES: usize = 16 * 1024;
const MAX_SAMPLES_PER_SNAPSHOT: u32 = 32;
const MAX_POPULATION: u32 = 500_000;
const MAX_STATUS_VALUE: f32 = 1.5;
const MAX_MONEY_SPENT: f32 = 1.0e12;
const REVIEWED_PROBE_VERSION: &str = "0.1.0";
const REVIEWED_GAME_VERSION: &str = "1.1.1.9";
const REVIEWED_EXECUTABLE_TIMESTAMP: u64 = 0x6A3E_B6AD;
const REVIEWED_EXECUTABLE_SIZE: u64 = 10_308_608;

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
    person_size: u32,
    person_vector_rva: String,
    writes_game_state: bool,
    writes_save_data: bool,
    writes_observatory_databases: bool,
    network_access: bool,
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
    let canonical_media = media_directory
        .canonicalize()
        .map_err(|_| "game_directory_unavailable")?;
    let game_root = canonical_media
        .parent()
        .ok_or("game_directory_unavailable")?
        .to_path_buf();
    let probe_path = probe_path(&game_root);
    if !probe_path.exists() {
        let mut status = TesmioProbeStatus::not_configured();
        status.state = TesmioProbeState::Missing;
        return Ok(status);
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
    let text = std::str::from_utf8(&bytes).map_err(|_| "probe_not_utf8")?;
    parse_records(text, &bytes)
}

fn probe_path(game_root: &Path) -> PathBuf {
    game_root
        .join("tesmioloader")
        .join("build")
        .join(PROBE_FILE)
}

fn parse_records(text: &str, bytes: &[u8]) -> Result<TesmioProbeStatus, &'static str> {
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
    for line in &lines[1..] {
        let value: serde_json::Value = serde_json::from_str(line).map_err(|_| "record_invalid")?;
        match value.get("record_type").and_then(serde_json::Value::as_str) {
            Some("snapshot") => {
                let record: SnapshotRecord =
                    serde_json::from_value(value).map_err(|_| "snapshot_invalid")?;
                validate_snapshot(&record)?;
                if snapshots.insert(record.sequence, record).is_some() {
                    return Err("duplicate_snapshot_sequence");
                }
            }
            Some("person_sample") => {
                let record: PersonSampleRecord =
                    serde_json::from_value(value).map_err(|_| "person_sample_invalid")?;
                validate_person_sample(&record)?;
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

    // Sequence, not game date, is the session ordering authority. Loading an
    // older save may legitimately make the date move backwards.
    let latest = snapshots.values().next_back();
    let warnings = Vec::new();
    Ok(TesmioProbeStatus {
        state: if warnings.is_empty() {
            TesmioProbeState::Available
        } else {
            TesmioProbeState::Warning
        },
        read_only: true,
        optional: true,
        persisted: false,
        probe_id: Some(session.probe_id),
        probe_version: Some(session.probe_version),
        loader_api_version: Some(session.loader_api_version),
        target_game_version: Some(session.target_game_version),
        executable_timestamp: Some(session.executable_timestamp),
        content_hash: Some(hex_hash(bytes)),
        snapshot_count: u32::try_from(snapshots.len()).unwrap_or(u32::MAX),
        sample_count: observed_samples.values().sum(),
        latest_year: latest.map(|record| record.year),
        latest_day: latest.map(|record| record.day),
        latest_population_count: latest.map(|record| record.population_count),
        warnings,
    })
}

fn validate_session(record: &SessionRecord) -> Result<(), &'static str> {
    if record.schema_version != 1
        || record.record_type != "session"
        || record.probe_id != PROBE_ID
        || record.mode != "read_only"
        || record.probe_version != REVIEWED_PROBE_VERSION
        || record.loader_api_version != 4
        || record.target_game_version != REVIEWED_GAME_VERSION
        || record.executable_timestamp != REVIEWED_EXECUTABLE_TIMESTAMP
        || record.executable_size != REVIEWED_EXECUTABLE_SIZE
        || record.person_size != 0x750
        || record.person_vector_rva != "0x9E75B8"
        || record.writes_game_state
        || record.writes_save_data
        || record.writes_observatory_databases
        || record.network_access
    {
        return Err("session_contract_mismatch");
    }
    Ok(())
}

fn validate_snapshot(record: &SnapshotRecord) -> Result<(), &'static str> {
    if record.schema_version != 1
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

fn validate_person_sample(record: &PersonSampleRecord) -> Result<(), &'static str> {
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
    if record.schema_version != 1
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

    use super::{PROBE_ID, inspect, parse_records};
    use crate::model::TesmioProbeState;

    fn session() -> String {
        format!(
            r#"{{"schema_version":1,"record_type":"session","probe_id":"{PROBE_ID}","probe_version":"0.1.0","mode":"read_only","loader_api_version":4,"target_game_version":"1.1.1.9","executable_timestamp":1782494893,"executable_size":10308608,"person_size":1872,"person_vector_rva":"0x9E75B8","writes_game_state":false,"writes_save_data":false,"writes_observatory_databases":false,"network_access":false}}"#
        )
    }

    fn snapshot(sequence: u32, year: i32, day: u16, count: u32) -> String {
        format!(
            r#"{{"schema_version":1,"record_type":"snapshot","sequence":{sequence},"year":{year},"day":{day},"population_count":100,"sample_count":{count}}}"#
        )
    }

    fn sample(sequence: u32, year: i32, day: u16) -> String {
        format!(
            r#"{{"schema_version":1,"record_type":"person_sample","sequence":{sequence},"sample_index":0,"vector_index":4,"year":{year},"day":{day},"current_building_present":true,"age_years":42.0,"education_level":2.0,"status_happiness":0.5,"status_food":0.5,"status_health":0.5,"status_soviet":0.5,"status_alcohol":0.5,"status_culture":0.5,"status_sport":0.5,"status_religion":0.5,"status_clothing":0.5,"status_electronics":0.5,"status_crime":0.5,"citizen_class":0,"money_spent":0.0}}"#
        )
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
}
