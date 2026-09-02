use std::collections::{BTreeMap, HashSet};
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
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 8 * 1024 * 1024;
const MAX_RETAINED_ENTRY_BYTES: u64 = 2 * 1024 * 1024;
const MAX_ENTRY_NAME_BYTES: usize = 512;

const REVIEWED_RETAINED_FILES: [(&str, &str); 7] = [
    ("src/tesmio_plugin.h", REVIEWED_PLUGIN_HEADER_HASH),
    ("src/tesmio_api.h", REVIEWED_API_HEADER_HASH),
    (
        "src/tesmioloader.cpp",
        "d88d42412c6614935db160e6358283ade9c327d03a752e3662b0841badcdc418",
    ),
    (
        "src/tesmiolauncher.cpp",
        "dd2753220759d11323e7336d5671b012ef2e53c62d677443edf1318196bfac04",
    ),
    (
        "src/tesmiolauncher.rc",
        "e2272842f7264f570db7eafae2d73d6ccf21268974745396f739bc07263f779a",
    ),
    (
        "logo.ico",
        "981a6cc3ac0b6339986d711da95ddb7b24c43f73c4245dac8d8fcfb09463b179",
    ),
    (
        "LICENSE",
        "3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986",
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResearchSourceDownloadPhase {
    Connecting,
    Downloading,
    CheckingArchive,
    Installing,
    Verifying,
}

#[derive(Clone, Debug)]
pub(crate) struct DownloadedResearchSource {
    pub checkout_path: PathBuf,
    pub archive_hash: String,
    pub reused: bool,
}

#[derive(Debug)]
struct ReviewedArchive {
    files: BTreeMap<String, Vec<u8>>,
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
    retained_files: Vec<String>,
}

pub(crate) fn download_reviewed_source(
    managed_root: &Path,
    mut progress: impl FnMut(ResearchSourceDownloadPhase, u64, Option<u64>),
) -> Result<DownloadedResearchSource, ObservatoryError> {
    let destination = managed_root.join(REVIEWED_TESMIO_REVISION);
    if managed_copy_is_reviewed(&destination) {
        progress(ResearchSourceDownloadPhase::Verifying, 0, None);
        let archive_hash = read_provenance_archive_hash(&destination)
            .unwrap_or_else(|| "unknown_revalidated_managed_copy".to_owned());
        return Ok(DownloadedResearchSource {
            checkout_path: destination,
            archive_hash,
            reused: true,
        });
    }

    progress(ResearchSourceDownloadPhase::Connecting, 0, None);
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
    progress(
        ResearchSourceDownloadPhase::CheckingArchive,
        bytes.len() as u64,
        Some(bytes.len() as u64),
    );
    let archive = inspect_reviewed_archive(bytes, &REVIEWED_RETAINED_FILES)?;
    progress(ResearchSourceDownloadPhase::Installing, 0, None);
    let installed = install_reviewed_archive(managed_root, archive)?;
    progress(ResearchSourceDownloadPhase::Verifying, 0, None);
    if !managed_copy_is_reviewed(&installed.checkout_path) {
        return Err(ObservatoryError::ResearchSourceInstallFailed);
    }
    Ok(installed)
}

fn reviewed_download_url() -> String {
    format!("https://{DOWNLOAD_HOST}{DOWNLOAD_PATH_PREFIX}{REVIEWED_TESMIO_REVISION}")
}

fn read_bounded_response(
    mut response: Response,
    expected_url: &str,
    progress: &mut impl FnMut(ResearchSourceDownloadPhase, u64, Option<u64>),
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
        progress(
            ResearchSourceDownloadPhase::Downloading,
            bytes.len() as u64,
            content_length,
        );
    }
    Ok(bytes)
}

fn inspect_reviewed_archive(
    bytes: Vec<u8>,
    expected_files: &[(&str, &str)],
) -> Result<ReviewedArchive, ObservatoryError> {
    let archive_hash = sha256(&bytes);
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| ObservatoryError::ResearchSourceArchiveInvalid)?;
    if archive.is_empty() || archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(ObservatoryError::ResearchSourceArchiveInvalid);
    }

    let mut names = HashSet::new();
    let mut expanded_bytes = 0_u64;
    let mut retained = BTreeMap::<String, Vec<u8>>::new();
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
        if expanded_bytes > MAX_EXPANDED_BYTES || entry.size() > MAX_ARCHIVE_ENTRY_BYTES {
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

        if !expected_files
            .iter()
            .any(|(expected, _)| *expected == relative)
        {
            continue;
        }
        if entry.size() > MAX_RETAINED_ENTRY_BYTES {
            return Err(ObservatoryError::ResearchSourceArchiveInvalid);
        }
        let mut contents = Vec::with_capacity(
            usize::try_from(entry.size())
                .map_err(|_| ObservatoryError::ResearchSourceArchiveInvalid)?,
        );
        entry
            .read_to_end(&mut contents)
            .map_err(|_| ObservatoryError::ResearchSourceArchiveInvalid)?;
        if retained.insert(relative.to_owned(), contents).is_some() {
            return Err(ObservatoryError::ResearchSourceArchiveInvalid);
        }
    }

    for (path, expected_hash) in expected_files {
        let contents = retained
            .get(*path)
            .ok_or(ObservatoryError::ResearchSourceArchiveInvalid)?;
        let actual_hash = if path.ends_with(".ico") {
            sha256(contents)
        } else {
            reviewed_header_hash(contents)
        };
        if actual_hash != *expected_hash {
            return Err(ObservatoryError::ResearchSourceArchiveInvalid);
        }
    }

    Ok(ReviewedArchive {
        files: retained,
        archive_hash,
    })
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
        for (relative, contents) in &archive.files {
            let target = staging.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
            }
            fs::write(target, contents)
                .map_err(|_| ObservatoryError::ResearchSourceInstallFailed)?;
        }
        let provenance = ResearchSourceProvenance {
            schema_version: 1,
            upstream_repository: "https://github.com/MaxLegend/TesmioLoader",
            reviewed_revision: REVIEWED_TESMIO_REVISION,
            archive_sha256: &archive.archive_hash,
            plugin_header_sha256: REVIEWED_PLUGIN_HEADER_HASH,
            api_header_sha256: REVIEWED_API_HEADER_HASH,
            retained_files: archive.files.keys().cloned().collect(),
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
    reviewed_session_source_is_available(path) && read_provenance_archive_hash(path).is_some()
}

pub(crate) fn reviewed_session_source_is_available(path: &Path) -> bool {
    path.is_dir()
        && REVIEWED_RETAINED_FILES.iter().all(|(relative, expected)| {
            let actual = if relative.ends_with(".ico") {
                super::research_setup::bounded_hash(&path.join(relative), MAX_RETAINED_ENTRY_BYTES)
            } else {
                super::research_setup::bounded_reviewed_file_hash(
                    &path.join(relative),
                    MAX_RETAINED_ENTRY_BYTES,
                )
            };
            actual.as_deref() == Some(*expected)
        })
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
        DOWNLOAD_HOST, DOWNLOAD_PATH_PREFIX, DOWNLOAD_TIMEOUT, MAX_RETAINED_ENTRY_BYTES,
        MAX_TRANSFER_BYTES, REVIEWED_RETAINED_FILES, inspect_reviewed_archive,
        install_reviewed_archive, reviewed_download_url, sha256,
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

    fn valid_archive() -> (Vec<u8>, String, String, String) {
        let plugin = b"reviewed plugin fixture";
        let api = b"reviewed api fixture";
        let licence = b"GPL fixture";
        (
            archive(&[
                ("TesmioLoader-reviewed/src/tesmio_plugin.h", plugin),
                ("TesmioLoader-reviewed/src/tesmio_api.h", api),
                ("TesmioLoader-reviewed/LICENSE", licence),
                ("TesmioLoader-reviewed/ignored.cpp", b"not retained"),
            ]),
            sha256(plugin),
            sha256(api),
            sha256(licence),
        )
    }

    #[test]
    fn retains_only_the_supplied_exact_allowlist_and_provenance() {
        let (bytes, plugin_hash, api_hash, licence_hash) = valid_archive();
        let expected = [
            ("src/tesmio_plugin.h", plugin_hash.as_str()),
            ("src/tesmio_api.h", api_hash.as_str()),
            ("LICENSE", licence_hash.as_str()),
        ];
        let archive = inspect_reviewed_archive(bytes, &expected).expect("reviewed");
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
        let (bytes, _plugin_hash, _api_hash, _licence_hash) = valid_archive();
        assert!(inspect_reviewed_archive(bytes, &[("src/tesmio_plugin.h", "0")]).is_err());

        let traversal = archive(&[("../src/tesmio_plugin.h", b"x")]);
        assert!(inspect_reviewed_archive(traversal, &[("src/tesmio_plugin.h", "x")]).is_err());

        let duplicate = archive(&[
            ("root/src/tesmio_plugin.h", b"one"),
            ("root/./src/tesmio_plugin.h", b"two"),
        ]);
        assert!(inspect_reviewed_archive(duplicate, &[("src/tesmio_plugin.h", "x")]).is_err());
    }

    #[test]
    fn rejects_incomplete_absolute_and_oversized_archives() {
        let plugin = b"reviewed plugin fixture";
        let api = b"reviewed api fixture";
        let missing_licence = archive(&[
            ("root/src/tesmio_plugin.h", plugin),
            ("root/src/tesmio_api.h", api),
        ]);
        let plugin_hash = sha256(plugin);
        let api_hash = sha256(api);
        assert!(
            inspect_reviewed_archive(
                missing_licence,
                &[
                    ("src/tesmio_plugin.h", plugin_hash.as_str()),
                    ("src/tesmio_api.h", api_hash.as_str()),
                    ("LICENSE", "missing"),
                ],
            )
            .is_err()
        );

        let absolute = archive(&[("/root/src/tesmio_plugin.h", b"x")]);
        assert!(inspect_reviewed_archive(absolute, &[("src/tesmio_plugin.h", "x")]).is_err());

        let oversized_contents = vec![b'x'; MAX_RETAINED_ENTRY_BYTES as usize + 1];
        let oversized = archive(&[("root/LICENSE", oversized_contents.as_slice())]);
        assert!(inspect_reviewed_archive(oversized, &[("LICENSE", "x")]).is_err());
    }

    #[test]
    fn ignores_large_unretained_assets_from_the_reviewed_upstream_archive() {
        let plugin = b"reviewed plugin fixture";
        let api = b"reviewed api fixture";
        // The reviewed upstream ZIP includes 4 MiB texture assets. They are
        // inspected for archive safety, then ignored rather than extracted.
        let ignored_asset = vec![b'x'; 4_194_432];
        let bytes = archive(&[
            ("root/src/tesmio_plugin.h", plugin),
            ("root/src/tesmio_api.h", api),
            ("root/LICENSE", b"GPL fixture"),
            ("root/res/irrelevant-texture.dds", ignored_asset.as_slice()),
        ]);
        let plugin_hash = sha256(plugin);
        let api_hash = sha256(api);
        let licence_hash = sha256(b"GPL fixture");
        assert!(
            inspect_reviewed_archive(
                bytes,
                &[
                    ("src/tesmio_plugin.h", plugin_hash.as_str()),
                    ("src/tesmio_api.h", api_hash.as_str()),
                    ("LICENSE", licence_hash.as_str()),
                ],
            )
            .is_ok()
        );
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

    #[test]
    fn automatic_session_source_is_an_exact_allowlist() {
        assert_eq!(
            REVIEWED_RETAINED_FILES
                .iter()
                .map(|(path, _)| *path)
                .collect::<Vec<_>>(),
            vec![
                "src/tesmio_plugin.h",
                "src/tesmio_api.h",
                "src/tesmioloader.cpp",
                "src/tesmiolauncher.cpp",
                "src/tesmiolauncher.rc",
                "logo.ico",
                "LICENSE",
            ]
        );
        assert!(
            REVIEWED_RETAINED_FILES
                .iter()
                .all(|(_, hash)| hash.len() == 64
                    && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        );
    }
}
