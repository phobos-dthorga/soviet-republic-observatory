use std::collections::HashMap;
use std::fmt::Write;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::compatibility_profile::ResolvedCompatibilityProfile;
use crate::error::ObservatoryError;
use crate::fixed_binary::decode_layout;
use crate::model::SaveInspection;
use crate::stats_parser::parse_stats;

const MAX_ARCHIVE_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_STATS_BYTES: u64 = 128 * 1024 * 1024;
const MAX_COMPRESSED_STATS_BYTES: u64 = 64 * 1024 * 1024;

pub fn inspect_save_archive(
    path: &Path,
    profile: &ResolvedCompatibilityProfile,
) -> Result<SaveInspection, ObservatoryError> {
    let before = fs::metadata(path).map_err(|_| ObservatoryError::InvalidSaveCandidate)?;
    if !before.is_file()
        || before.len() == 0
        || before.len() > MAX_ARCHIVE_BYTES
        || !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err(ObservatoryError::InvalidSaveCandidate);
    }

    let file = File::open(path).map_err(|_| ObservatoryError::InvalidArchive)?;
    let mut archive =
        ZipArchive::new(BufReader::new(file)).map_err(|_| ObservatoryError::InvalidArchive)?;
    let mut stats_index = None;
    let mut binary_indices = HashMap::<String, usize>::new();

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| ObservatoryError::InvalidArchive)?;
        let normalized_name = entry.name().replace('\\', "/");
        let is_stats = profile
            .stats_archive_aliases()
            .iter()
            .any(|alias| alias == &normalized_name);
        if is_stats {
            if stats_index.replace(index).is_some() {
                return Err(ObservatoryError::DuplicateStatsPayload);
            }
            if entry.size() == 0 {
                return Err(ObservatoryError::MissingStatsPayload);
            }
            if entry.size() > MAX_STATS_BYTES
                || entry.compressed_size() > MAX_COMPRESSED_STATS_BYTES
            {
                return Err(ObservatoryError::StatsPayloadTooLarge);
            }
        }
        if profile
            .binary_layouts()
            .iter()
            .any(|layout| layout.entry_name == normalized_name)
        {
            if binary_indices.insert(normalized_name, index).is_some() {
                return Err(ObservatoryError::BinaryCompatibilityMismatch(
                    "duplicate_entry",
                ));
            }
            if entry.size() == 0
                || entry.size() > MAX_STATS_BYTES
                || entry.compressed_size() > MAX_COMPRESSED_STATS_BYTES
            {
                return Err(ObservatoryError::BinaryCompatibilityMismatch("entry_size"));
            }
        }
    }

    let stats_index = stats_index.ok_or(ObservatoryError::MissingStatsPayload)?;
    let parsed = {
        let entry = archive
            .by_index(stats_index)
            .map_err(|_| ObservatoryError::InvalidArchive)?;
        parse_stats(BufReader::new(entry), profile)?
    };
    let mut binary_facts = Vec::new();
    for layout in profile.binary_layouts() {
        let Some(index) = binary_indices.get(&layout.entry_name).copied() else {
            continue;
        };
        let mut entry = archive
            .by_index(index)
            .map_err(|_| ObservatoryError::InvalidArchive)?;
        let capacity = usize::try_from(entry.size())
            .map_err(|_| ObservatoryError::BinaryCompatibilityMismatch("entry_size"))?;
        let mut bytes = Vec::with_capacity(capacity);
        entry
            .read_to_end(&mut bytes)
            .map_err(|_| ObservatoryError::InvalidArchive)?;
        if bytes.len() != capacity {
            return Err(ObservatoryError::BinaryCompatibilityMismatch(
                "truncated_entry",
            ));
        }
        binary_facts.extend(decode_layout(layout, &bytes)?);
    }
    drop(archive);

    let after = fs::metadata(path).map_err(|_| ObservatoryError::SaveChangedDuringRead)?;
    if before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || !after.is_file()
    {
        return Err(ObservatoryError::SaveChangedDuringRead);
    }

    let source_file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ObservatoryError::InvalidSaveCandidate)?
        .to_owned();

    let interpretation_id = profile.interpretation_id(&parsed.payload_hash);
    Ok(SaveInspection {
        payload_hash: parsed.payload_hash,
        interpretation_id,
        compatibility: profile.provenance(),
        source_file_name,
        source_file_size: after.len(),
        source_modified_ms: system_time_ms(after.modified().unwrap_or(UNIX_EPOCH)),
        source_directory_identity: directory_identity(
            path.parent()
                .ok_or(ObservatoryError::InvalidSaveCandidate)?,
        )?,
        records: parsed.records,
        coverage: parsed.coverage,
        snapshots: parsed.snapshots,
        market: parsed.market,
        citizen_status: parsed.citizen_status,
        environment: parsed.environment,
        binary_facts,
    })
}

pub fn hash_save_stats_payload(
    path: &Path,
    profile: &ResolvedCompatibilityProfile,
) -> Result<String, ObservatoryError> {
    let before = fs::metadata(path).map_err(|_| ObservatoryError::InvalidSaveCandidate)?;
    if !before.is_file()
        || before.len() == 0
        || before.len() > MAX_ARCHIVE_BYTES
        || !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err(ObservatoryError::InvalidSaveCandidate);
    }
    let file = File::open(path).map_err(|_| ObservatoryError::InvalidArchive)?;
    let mut archive =
        ZipArchive::new(BufReader::new(file)).map_err(|_| ObservatoryError::InvalidArchive)?;
    let mut stats_index = None;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| ObservatoryError::InvalidArchive)?;
        let normalized_name = entry.name().replace('\\', "/");
        if profile
            .stats_archive_aliases()
            .iter()
            .any(|alias| alias == &normalized_name)
        {
            if stats_index.replace(index).is_some() {
                return Err(ObservatoryError::DuplicateStatsPayload);
            }
            if entry.size() == 0 {
                return Err(ObservatoryError::MissingStatsPayload);
            }
            if entry.size() > MAX_STATS_BYTES
                || entry.compressed_size() > MAX_COMPRESSED_STATS_BYTES
            {
                return Err(ObservatoryError::StatsPayloadTooLarge);
            }
        }
    }
    let stats_index = stats_index.ok_or(ObservatoryError::MissingStatsPayload)?;
    let mut entry = archive
        .by_index(stats_index)
        .map_err(|_| ObservatoryError::InvalidArchive)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut bytes_read = 0_u64;
    loop {
        let count = entry
            .read(&mut buffer)
            .map_err(|_| ObservatoryError::InvalidArchive)?;
        if count == 0 {
            break;
        }
        bytes_read = bytes_read
            .checked_add(count as u64)
            .ok_or(ObservatoryError::StatsPayloadTooLarge)?;
        if bytes_read > MAX_STATS_BYTES {
            return Err(ObservatoryError::StatsPayloadTooLarge);
        }
        hasher.update(&buffer[..count]);
    }
    drop(entry);
    drop(archive);
    let after = fs::metadata(path).map_err(|_| ObservatoryError::SaveChangedDuringRead)?;
    if before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || !after.is_file()
    {
        return Err(ObservatoryError::SaveChangedDuringRead);
    }
    let digest = hasher.finalize();
    let mut payload_hash = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut payload_hash, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(payload_hash)
}

pub fn directory_identity(path: &Path) -> Result<String, ObservatoryError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| ObservatoryError::InvalidDirectory)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let mut identity = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut identity, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(identity)
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

    use super::{hash_save_stats_payload, inspect_save_archive};
    use crate::compatibility_profile::ResolvedCompatibilityProfile;
    use crate::error::ObservatoryError;

    #[test]
    fn reads_stats_directly_from_a_synthetic_archive() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("synthetic-save.zip");
        let file = File::create(&path).expect("fixture archive");
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                "stats.ini",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .expect("stats entry");
        archive
            .write_all(include_bytes!("../fixtures/valid.receiver-stats.txt"))
            .expect("stats content");
        archive.finish().expect("finish archive");

        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let inspection = inspect_save_archive(&path, &profile).expect("read-only inspection");
        let payload_hash = hash_save_stats_payload(&path, &profile).expect("hash-only inspection");
        assert_eq!(inspection.records.len(), 3);
        assert_eq!(payload_hash, inspection.payload_hash);
        assert_eq!(inspection.source_file_name, "synthetic-save.zip");
    }

    #[test]
    fn rejects_an_archive_without_stats() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("no-stats.zip");
        let file = File::create(&path).expect("fixture archive");
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("header.bin", SimpleFileOptions::default())
            .expect("header entry");
        archive.write_all(b"synthetic").expect("header content");
        archive.finish().expect("finish archive");

        assert!(matches!(
            inspect_save_archive(
                &path,
                &ResolvedCompatibilityProfile::reviewed_builtin().expect("profile")
            ),
            Err(ObservatoryError::MissingStatsPayload)
        ));
    }

    #[test]
    fn optional_local_save_conformance() {
        let Ok(path) = std::env::var("RO_LIVE_SAVE") else {
            return;
        };

        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let inspection = inspect_save_archive(Path::new(&path), &profile)
            .expect("configured local save should satisfy the supported receiver profile");

        assert!(!inspection.records.is_empty());
        assert_eq!(
            inspection.coverage.chartable_records as usize,
            inspection.records.len()
        );
        assert_eq!(inspection.coverage.dropped_records, 0);
        assert!(
            inspection
                .snapshots
                .iter()
                .any(|snapshot| snapshot.scope_kind == crate::model::SnapshotScopeKind::Republic)
        );
        assert!(
            inspection
                .snapshots
                .iter()
                .any(|snapshot| snapshot.scope_kind == crate::model::SnapshotScopeKind::City)
        );
    }
}
