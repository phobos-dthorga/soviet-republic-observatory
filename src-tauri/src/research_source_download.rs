use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use serde::Serialize;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::error::ObservatoryError;
use crate::research_setup::{
    REVIEWED_API_HEADER_HASH, REVIEWED_PLUGIN_HEADER_HASH, REVIEWED_TESMIO_REVISION,
    reviewed_header_hash,
};

const DOWNLOAD_HOST: &str = "codeload.github.com";
const DOWNLOAD_PATH_PREFIX: &str = "/MaxLegend/TesmioLoader/zip/";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_TRANSFER_BYTES: u64 = 8 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 1_024;
const MAX_EXPANDED_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 2 * 1024 * 1024;
const MAX_ENTRY_NAME_BYTES: usize = 512;

#[derive(Clone, Debug)]
pub(crate) struct DownloadedResearchSource {
    pub checkout_path: PathBuf,
    pub archive_hash: String,
    pub reused: bool,
}

#[derive(Debug)]
struct ReviewedArchive {
    plugin_header: Vec<u8>,
    api_header: Vec<u8>,
    licence_name: String,
    licence: Vec<u8>,
    archive_hash: String,
}

#[derive(Serialize)]
struct ResearchSourceProvenance<'a> {
    schema_version: u32,
    upstream_repository: &'a str,
    reviewed_revision: &'a str,
    archive_sha256: &'a str,
    plugin_header_sha256: &'a str,
    api_header_sha256: &'a str,
    retained_files: [&'a str; 3],
}

pub(crate) fn download_reviewed_source(
    managed_root: &Path,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<DownloadedResearchSource, ObservatoryError> {
    let destination = managed_root.join(REVIEWED_TESMIO_REVISION);
    if managed_copy_is_reviewed(&destination) {
        let archive_hash = read_provenance_archive_hash(&destination)
            .unwrap_or_else(|| "unknown_revalidated_managed_copy".to_owned());
        return Ok(DownloadedResearchSource {
            checkout_path: destination,
            archive_hash,
            reused: true,
        });
    }

    let client = Client::builder()
        .https_only(true)
        .redirect(Policy::none())
        .timeout(DOWNLOAD_TIMEOUT)
        .user_agent("Republic-Observatory/reviewed-source-fetch")
        .build()
        .map_err(|_| ObservatoryError::ResearchSourceDownloadFailed)?;
    let url = reviewed_download_url();
    let response = client
        .get(&url)
        .send()
        .map_err(|_| ObservatoryError::ResearchSourceDownloadFailed)?;
    let bytes = read_bounded_response(response, &url, &mut progress)?;
    let archive =
        inspect_reviewed_archive(bytes, REVIEWED_PLUGIN_HEADER_HASH, REVIEWED_API_HEADER_HASH)?;
    install_reviewed_archive(managed_root, archive)
}

fn reviewed_download_url() -> String {
    format!("https://{DOWNLOAD_HOST}{DOWNLOAD_PATH_PREFIX}{REVIEWED_TESMIO_REVISION}")
}

fn read_bounded_response(
    mut response: Response,
    expected_url: &str,
    progress: &mut impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, ObservatoryError> {
    if response.url().as_str() != expected_url
        || response.url().scheme() != "https"
        || response.url().host_str() != Some(DOWNLOAD_HOST)
        || response.url().query().is_some()
        || !response.status().is_success()
    {
        return Err(ObservatoryError::ResearchSourceDownloadFailed);
    }
    let content_length = response.content_length();
    if content_length.is_some_and(|length| length > MAX_TRANSFER_BYTES) {
        return Err(ObservatoryError::ResearchSourceArchiveInvalid);
    }
    let mut bytes = Vec::with_capacity(
        content_length
            .and_then(|length| usize::try_from(length).ok())
            .unwrap_or(64 * 1024),
    );
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|_| ObservatoryError::ResearchSourceDownloadFailed)?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) as u64 > MAX_TRANSFER_BYTES {
            return Err(ObservatoryError::ResearchSourceArchiveInvalid);
        }
        bytes.extend_from_slice(&buffer[..read]);
        progress(bytes.len() as u64, content_length);
    }
    Ok(bytes)
}

fn inspect_reviewed_archive(
    bytes: Vec<u8>,
    expected_plugin_hash: &str,
    expected_api_hash: &str,
) -> Result<ReviewedArchive, ObservatoryError> {
    let archive_hash = sha256(&bytes);
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| ObservatoryError::ResearchSourceArchiveInvalid)?;
    if archive.is_empty() || archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(ObservatoryError::ResearchSourceArchiveInvalid);
    }

    let mut names = HashSet::new();
    let mut expanded_bytes = 0_u64;
    let mut plugin_header = None;
    let mut api_header = None;
    let mut licence = None;
    let mut archive_prefix = None::<String>;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| ObservatoryError::ResearchSourceArchiveInvalid)?;
        let name = entry.name().to_owned();
        if name.len() > MAX_ENTRY_NAME_BYTES
            || name.contains('\0')
            || name.contains('\\')
            || !names.insert(name.clone())
            || !safe_archive_name(&name)
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(ObservatoryError::ResearchSourceArchiveInvalid);
        }
        expanded_bytes = expanded_bytes
            .checked_add(entry.size())
            .ok_or(ObservatoryError::ResearchSourceArchiveInvalid)?;
        if expanded_bytes > MAX_EXPANDED_BYTES || entry.size() > MAX_ENTRY_BYTES {
            return Err(ObservatoryError::ResearchSourceArchiveInvalid);
        }
        if entry.is_dir() {
            continue;
        }

        let Some((prefix, relative)) = name.split_once('/') else {
            return Err(ObservatoryError::ResearchSourceArchiveInvalid);
        };
        if prefix.is_empty() {
            return Err(ObservatoryError::ResearchSourceArchiveInvalid);
        }
        if archive_prefix
            .as_deref()
            .is_some_and(|known| known != prefix)
        {
            return Err(ObservatoryError::ResearchSourceArchiveInvalid);
        }
        archive_prefix.get_or_insert_with(|| prefix.to_owned());

        let target = match relative {
            "src/tesmio_plugin.h" => Some("plugin"),
            "src/tesmio_api.h" => Some("api"),
            "LICENSE" => Some("licence"),
            _ => None,
        };
        let Some(target) = target else {
            continue;
        };
        let mut contents = Vec::with_capacity(
            usize::try_from(entry.size())
                .map_err(|_| ObservatoryError::ResearchSourceArchiveInvalid)?,
        );
        entry
            .read_to_end(&mut contents)
            .map_err(|_| ObservatoryError::ResearchSourceArchiveInvalid)?;
        match target {
            "plugin" => assign_once(&mut plugin_header, contents)?,
            "api" => assign_once(&mut api_header, contents)?,
            "licence" => assign_once(&mut licence, contents)?,
            _ => {}
        }
    }

    let plugin_header = plugin_header.ok_or(ObservatoryError::ResearchSourceArchiveInvalid)?;
    let api_header = api_header.ok_or(ObservatoryError::ResearchSourceArchiveInvalid)?;
    let licence = licence.ok_or(ObservatoryError::ResearchSourceArchiveInvalid)?;
    if reviewed_header_hash(&plugin_header) != expected_plugin_hash
        || reviewed_header_hash(&api_header) != expected_api_hash
    {
        return Err(ObservatoryError::ResearchSourceArchiveInvalid);
    }

    Ok(ReviewedArchive {
        plugin_header,
        api_header,
        licence_name: "LICENSE".to_owned(),
        licence,
        archive_hash,
    })
}

fn assign_once(slot: &mut Option<Vec<u8>>, contents: Vec<u8>) -> Result<(), ObservatoryError> {
    if slot.replace(contents).is_some() {
        return Err(ObservatoryError::ResearchSourceArchiveInvalid);
    }
    Ok(())
}

fn safe_archive_name(name: &str) -> bool {
    let path = Path::new(name);
    !path.is_absolute()
        && path.components().all(|component| {
            matches!(component, Component::Normal(_) | Component::CurDir)
                && component != Component::CurDir
        })
}

fn install_reviewed_archive(
    managed_root: &Path,
    archive: ReviewedArchive,
) -> Result<DownloadedResearchSource, ObservatoryError> {
    fs::create_dir_all(managed_root).map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
    let root_metadata = fs::symlink_metadata(managed_root)
        .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
    if !root_metadata.is_dir() || root_metadata.file_type().is_symlink() {
        return Err(ObservatoryError::ResearchSourceInstallFailed);
    }

    let destination = managed_root.join(REVIEWED_TESMIO_REVISION);
    let nonce = format!("{}-{}", std::process::id(), crate::storage::now_ms());
    let staging = managed_root.join(format!(".{REVIEWED_TESMIO_REVISION}.{nonce}.staging"));
    let backup = managed_root.join(format!(".{REVIEWED_TESMIO_REVISION}.{nonce}.backup"));
    fs::create_dir(&staging).map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
    let result = (|| {
        fs::create_dir(staging.join("src"))
            .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
        fs::write(staging.join("src/tesmio_plugin.h"), &archive.plugin_header)
            .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
        fs::write(staging.join("src/tesmio_api.h"), &archive.api_header)
            .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
        fs::write(staging.join(&archive.licence_name), &archive.licence)
            .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
        let provenance = ResearchSourceProvenance {
            schema_version: 1,
            upstream_repository: "https://github.com/MaxLegend/TesmioLoader",
            reviewed_revision: REVIEWED_TESMIO_REVISION,
            archive_sha256: &archive.archive_hash,
            plugin_header_sha256: REVIEWED_PLUGIN_HEADER_HASH,
            api_header_sha256: REVIEWED_API_HEADER_HASH,
            retained_files: ["src/tesmio_plugin.h", "src/tesmio_api.h", "LICENSE"],
        };
        let provenance = serde_json::to_vec_pretty(&provenance)
            .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
        fs::write(staging.join("observatory-provenance.json"), provenance)
            .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;

        if destination.exists() {
            let metadata = fs::symlink_metadata(&destination)
                .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(ObservatoryError::ResearchSourceInstallFailed);
            }
            fs::rename(&destination, &backup)
                .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
        }
        if let Err(_error) = fs::rename(&staging, &destination) {
            if backup.exists() {
                let _ = fs::rename(&backup, &destination);
            }
            return Err(ObservatoryError::ResearchSourceInstallFailed);
        }
        if backup.exists() {
            fs::remove_dir_all(&backup)
                .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
        }
        Ok(DownloadedResearchSource {
            checkout_path: destination,
            archive_hash: archive.archive_hash,
            reused: false,
        })
    })();
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

pub(crate) fn managed_copy_is_reviewed(path: &Path) -> bool {
    super::research_setup::checkout_matches_reviewed_headers(path)
        && read_provenance_archive_hash(path).is_some()
}

fn read_provenance_archive_hash(path: &Path) -> Option<String> {
    let bytes = fs::read(path.join("observatory-provenance.json")).ok()?;
    if bytes.len() > 16 * 1024 {
        return None;
    }
    let value = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
    if value.get("reviewed_revision")?.as_str()? != REVIEWED_TESMIO_REVISION {
        return None;
    }
    let hash = value.get("archive_sha256")?.as_str()?;
    (hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())).then(|| hash.to_owned())
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    use super::{
        DOWNLOAD_HOST, DOWNLOAD_PATH_PREFIX, DOWNLOAD_TIMEOUT, MAX_ENTRY_BYTES, MAX_TRANSFER_BYTES,
        inspect_reviewed_archive, install_reviewed_archive, reviewed_download_url, sha256,
    };

    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, contents) in entries {
            writer
                .start_file(name, SimpleFileOptions::default())
                .expect("entry");
            writer.write_all(contents).expect("contents");
        }
        writer.finish().expect("archive").into_inner()
    }

    fn valid_archive() -> (Vec<u8>, String, String) {
        let plugin = b"reviewed plugin fixture";
        let api = b"reviewed api fixture";
        (
            archive(&[
                ("TesmioLoader-reviewed/src/tesmio_plugin.h", plugin),
                ("TesmioLoader-reviewed/src/tesmio_api.h", api),
                ("TesmioLoader-reviewed/LICENSE", b"GPL fixture"),
                ("TesmioLoader-reviewed/ignored.cpp", b"not retained"),
            ]),
            sha256(plugin),
            sha256(api),
        )
    }

    #[test]
    fn retains_only_reviewed_headers_licence_and_provenance() {
        let (bytes, plugin_hash, api_hash) = valid_archive();
        let archive = inspect_reviewed_archive(bytes, &plugin_hash, &api_hash).expect("reviewed");
        let root = tempdir().expect("managed root");
        let installed = install_reviewed_archive(root.path(), archive).expect("install");
        assert!(
            installed
                .checkout_path
                .join("src/tesmio_plugin.h")
                .is_file()
        );
        assert!(installed.checkout_path.join("src/tesmio_api.h").is_file());
        assert!(installed.checkout_path.join("LICENSE").is_file());
        assert!(
            installed
                .checkout_path
                .join("observatory-provenance.json")
                .is_file()
        );
        assert!(!installed.checkout_path.join("ignored.cpp").exists());
    }

    #[test]
    fn rejects_traversal_duplicate_aliases_and_wrong_header_identity() {
        let (bytes, plugin_hash, _api_hash) = valid_archive();
        assert!(inspect_reviewed_archive(bytes, &plugin_hash, "0").is_err());

        let traversal = archive(&[("../src/tesmio_plugin.h", b"x")]);
        assert!(inspect_reviewed_archive(traversal, "x", "y").is_err());

        let duplicate = archive(&[
            ("root/src/tesmio_plugin.h", b"one"),
            ("root/./src/tesmio_plugin.h", b"two"),
        ]);
        assert!(inspect_reviewed_archive(duplicate, "x", "y").is_err());
    }

    #[test]
    fn rejects_incomplete_absolute_and_oversized_archives() {
        let plugin = b"reviewed plugin fixture";
        let api = b"reviewed api fixture";
        let missing_licence = archive(&[
            ("root/src/tesmio_plugin.h", plugin),
            ("root/src/tesmio_api.h", api),
        ]);
        assert!(inspect_reviewed_archive(missing_licence, &sha256(plugin), &sha256(api)).is_err());

        let absolute = archive(&[("/root/src/tesmio_plugin.h", b"x")]);
        assert!(inspect_reviewed_archive(absolute, "x", "y").is_err());

        let oversized_contents = vec![b'x'; MAX_ENTRY_BYTES as usize + 1];
        let oversized = archive(&[("root/LICENSE", oversized_contents.as_slice())]);
        assert!(inspect_reviewed_archive(oversized, "x", "y").is_err());
    }

    #[test]
    fn network_boundary_is_fixed_to_the_reviewed_source_revision() {
        assert_eq!(DOWNLOAD_HOST, "codeload.github.com");
        assert_eq!(DOWNLOAD_PATH_PREFIX, "/MaxLegend/TesmioLoader/zip/");
        assert_eq!(DOWNLOAD_TIMEOUT.as_secs(), 30);
        assert_eq!(MAX_TRANSFER_BYTES, 8 * 1024 * 1024);
        assert_eq!(
            reviewed_download_url(),
            "https://codeload.github.com/MaxLegend/TesmioLoader/zip/3baa141f9f08921aea9c95f0a400289cabd9960a"
        );
    }
}
