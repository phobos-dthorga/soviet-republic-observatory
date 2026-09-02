use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::ObservatoryError;
use crate::model::GameVocabularySource;

const MAX_VOCABULARY_BYTES: u64 = 32 * 1024 * 1024;
const MAX_VOCABULARY_ENTRIES: usize = 100_000;
const MAX_UTF16_UNITS: usize = 16 * 1024 * 1024;
const MAX_RESOLVED_LABEL_CHARS: usize = 160;
const BTF_HEADER_BYTES: usize = 12;
const BTF_INDEX_BYTES: usize = 10;

#[derive(Clone, Debug)]
pub struct VocabularyRevision {
    pub source: GameVocabularySource,
    entries: BTreeMap<u32, String>,
}

#[derive(Clone, Debug, Default)]
pub struct GameVocabularyCatalogue {
    primary: Option<VocabularyRevision>,
    english: Option<VocabularyRevision>,
}

impl GameVocabularyCatalogue {
    pub fn resolve(&self, caption_id: u32) -> Option<(String, String)> {
        self.primary
            .as_ref()
            .and_then(|revision| {
                revision
                    .entries
                    .get(&caption_id)
                    .map(|label| (label.clone(), revision.source.source_id.clone()))
            })
            .or_else(|| {
                self.english.as_ref().and_then(|revision| {
                    revision
                        .entries
                        .get(&caption_id)
                        .map(|label| (label.clone(), revision.source.source_id.clone()))
                })
            })
    }

    pub fn revisions(&self) -> impl Iterator<Item = &GameVocabularySource> {
        self.primary
            .iter()
            .chain(self.english.iter())
            .map(|revision| &revision.source)
    }
}

pub fn resolve_game_media_directory(selected: &Path) -> Result<PathBuf, ObservatoryError> {
    let canonical = selected
        .canonicalize()
        .map_err(|_| ObservatoryError::InvalidGameDirectory)?;
    if !canonical.is_dir() {
        return Err(ObservatoryError::InvalidGameDirectory);
    }

    let media = if canonical
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("media_soviet"))
    {
        canonical
    } else {
        canonical.join("media_soviet")
    };
    if !media.is_dir() {
        return Err(ObservatoryError::InvalidGameDirectory);
    }
    Ok(media)
}

pub fn discover_game_vocabularies(
    media_directory: &Path,
) -> Result<Vec<GameVocabularySource>, ObservatoryError> {
    let mut sources = fs::read_dir(media_directory)
        .map_err(|_| ObservatoryError::InvalidGameDirectory)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_file() || file_type.is_symlink() {
                return None;
            }
            let file_name = entry.file_name().to_str()?.to_owned();
            let suffix = file_name
                .strip_prefix("soviet")?
                .strip_suffix(".btf")?
                .to_owned();
            if suffix.is_empty() {
                return None;
            }
            Some((entry.path(), suffix, file_name))
        })
        .map(|(path, suffix, file_name)| {
            inspect_vocabulary(&path, &suffix, file_name.clone())
                .unwrap_or_else(|| vocabulary_source(&suffix, file_name, false, None, None, 0))
        })
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    Ok(sources)
}

pub fn load_game_vocabulary_catalogue(
    media_directory: &Path,
    requested_locale: &str,
    requested_caption_ids: &BTreeSet<u32>,
) -> Result<GameVocabularyCatalogue, ObservatoryError> {
    if requested_caption_ids.len() > 512 {
        return Err(ObservatoryError::InvalidCatalogueRequest);
    }

    let sources = discover_game_vocabularies(media_directory)?;
    let primary_source = sources
        .iter()
        .find(|source| locale_matches(source.locale_hint.as_deref(), requested_locale))
        .cloned();
    let english_source = sources
        .iter()
        .find(|source| source.locale_hint.as_deref() == Some("en"))
        .filter(|source| {
            primary_source
                .as_ref()
                .is_none_or(|primary| primary.source_id != source.source_id)
        })
        .cloned();

    let primary = primary_source.and_then(|source| {
        load_vocabulary_revision(media_directory, source, requested_caption_ids).ok()
    });
    let english = english_source.and_then(|source| {
        load_vocabulary_revision(media_directory, source, requested_caption_ids).ok()
    });

    Ok(GameVocabularyCatalogue { primary, english })
}

fn inspect_vocabulary(
    path: &Path,
    suffix: &str,
    file_name: String,
) -> Option<GameVocabularySource> {
    let bytes = read_bounded(path).ok()?;
    let decoded = decode_btf(&bytes, &BTreeSet::new()).ok()?;
    Some(vocabulary_source(
        suffix,
        file_name,
        true,
        Some(sha256(&bytes)),
        Some(decoded.declared_entry_count),
        decoded.warning_count,
    ))
}

fn load_vocabulary_revision(
    media_directory: &Path,
    mut source: GameVocabularySource,
    requested_caption_ids: &BTreeSet<u32>,
) -> Result<VocabularyRevision, ObservatoryError> {
    let path = media_directory.join(&source.file_name);
    let canonical_media = media_directory
        .canonicalize()
        .map_err(|_| ObservatoryError::InvalidGameDirectory)?;
    let canonical_path = path
        .canonicalize()
        .map_err(|_| ObservatoryError::InvalidGameDirectory)?;
    if !canonical_path.starts_with(&canonical_media) || !canonical_path.is_file() {
        return Err(ObservatoryError::InvalidGameDirectory);
    }
    let bytes = read_bounded(&canonical_path)?;
    let decoded = decode_btf(&bytes, requested_caption_ids)
        .map_err(|_| ObservatoryError::InvalidCatalogueRequest)?;
    source.readable = true;
    source.content_hash = Some(sha256(&bytes));
    source.entry_count = Some(decoded.declared_entry_count);
    source.warning_count = decoded.warning_count;
    Ok(VocabularyRevision {
        source,
        entries: decoded.entries,
    })
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, ObservatoryError> {
    let metadata = fs::metadata(path).map_err(|_| ObservatoryError::InvalidGameDirectory)?;
    if !metadata.is_file() || metadata.len() > MAX_VOCABULARY_BYTES {
        return Err(ObservatoryError::InvalidCatalogueRequest);
    }
    fs::read(path).map_err(|_| ObservatoryError::InvalidGameDirectory)
}

#[derive(Debug)]
struct DecodedBtf {
    declared_entry_count: u32,
    warning_count: u32,
    entries: BTreeMap<u32, String>,
}

fn decode_btf(bytes: &[u8], requested_caption_ids: &BTreeSet<u32>) -> Result<DecodedBtf, ()> {
    if bytes.len() < BTF_HEADER_BYTES || bytes.len() as u64 > MAX_VOCABULARY_BYTES {
        return Err(());
    }
    let declared_entry_count = read_u32(bytes, 0)?;
    let declared_file_size = read_u32(bytes, 4)? as usize;
    let payload_units = read_u32(bytes, 8)? as usize;
    let entry_count = declared_entry_count as usize;
    if entry_count > MAX_VOCABULARY_ENTRIES || payload_units > MAX_UTF16_UNITS {
        return Err(());
    }
    let index_bytes = entry_count.checked_mul(BTF_INDEX_BYTES).ok_or(())?;
    let payload_bytes = payload_units.checked_mul(2).ok_or(())?;
    let payload_start = BTF_HEADER_BYTES.checked_add(index_bytes).ok_or(())?;
    let expected_size = payload_start.checked_add(payload_bytes).ok_or(())?;
    if declared_file_size != expected_size || bytes.len() != expected_size {
        return Err(());
    }

    let mut entries = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut warning_count = 0_u32;
    for index in 0..entry_count {
        let offset = BTF_HEADER_BYTES + index * BTF_INDEX_BYTES;
        let caption_id = read_u32(bytes, offset)?;
        let string_offset = read_u32(bytes, offset + 4)? as usize;
        let string_length = read_u16(bytes, offset + 8)? as usize;
        let end = string_offset.checked_add(string_length).ok_or(())?;
        if end > payload_units || string_length > MAX_RESOLVED_LABEL_CHARS {
            return Err(());
        }
        if !seen.insert(caption_id) {
            warning_count = warning_count.saturating_add(1);
            continue;
        }
        if !requested_caption_ids.is_empty() && !requested_caption_ids.contains(&caption_id) {
            continue;
        }

        let byte_start = payload_start + string_offset * 2;
        let byte_end = payload_start + end * 2;
        let mut units = Vec::with_capacity(string_length);
        for pair in bytes[byte_start..byte_end].chunks_exact(2) {
            units.push(u16::from_be_bytes([pair[0], pair[1]]));
        }
        let label = String::from_utf16(&units).map_err(|_| ())?;
        let label = sanitize_label(&label)?;
        entries.insert(caption_id, label);
    }

    Ok(DecodedBtf {
        declared_entry_count,
        warning_count,
        entries,
    })
}

fn sanitize_label(label: &str) -> Result<String, ()> {
    let trimmed = label.trim_matches(char::from(0)).trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > MAX_RESOLVED_LABEL_CHARS
        || trimmed
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(());
    }
    Ok(trimmed.to_owned())
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ()> {
    let slice = bytes.get(offset..offset + 4).ok_or(())?;
    Ok(u32::from_be_bytes(slice.try_into().map_err(|_| ())?))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ()> {
    let slice = bytes.get(offset..offset + 2).ok_or(())?;
    Ok(u16::from_be_bytes(slice.try_into().map_err(|_| ())?))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn vocabulary_source(
    suffix: &str,
    file_name: String,
    readable: bool,
    content_hash: Option<String>,
    entry_count: Option<u32>,
    warning_count: u32,
) -> GameVocabularySource {
    GameVocabularySource {
        source_id: format!("installed-game.{suffix}"),
        file_name,
        locale_hint: locale_hint(suffix).map(str::to_owned),
        format: "btf-be-v1".to_owned(),
        readable,
        content_hash,
        entry_count,
        warning_count,
    }
}

fn locale_matches(candidate: Option<&str>, requested: &str) -> bool {
    let requested = requested.to_ascii_lowercase();
    candidate.is_some_and(|candidate| {
        let candidate = candidate.to_ascii_lowercase();
        requested == candidate || requested.starts_with(&format!("{candidate}-"))
    })
}

fn locale_hint(suffix: &str) -> Option<&'static str> {
    match suffix {
        "English" => Some("en"),
        "Bulgarian" => Some("bg"),
        "Chinese" => Some("zh-Hans"),
        "ChineseTraditional" => Some("zh-Hant"),
        "Czech" => Some("cs"),
        "French" => Some("fr"),
        "German" => Some("de"),
        "Hungarian" => Some("hu"),
        "Italian" => Some("it"),
        "Japanese" => Some("ja"),
        "Korean" => Some("ko"),
        "Polish" => Some("pl"),
        "PortugueseBrazil" => Some("pt-BR"),
        "Romanian" => Some("ro"),
        "Russian" => Some("ru"),
        "Serbian" => Some("sr"),
        "Slovak" => Some("sk"),
        "Spanish" => Some("es"),
        "Turkish" => Some("tr"),
        "Ukrainian" => Some("uk"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;

    use tempfile::tempdir;

    use super::{
        decode_btf, discover_game_vocabularies, load_game_vocabulary_catalogue,
        resolve_game_media_directory,
    };

    fn fixture(entries: &[(u32, &str)]) -> Vec<u8> {
        let mut index = Vec::new();
        let mut payload = Vec::new();
        let mut offset = 0_u32;
        for (id, value) in entries {
            let units = value.encode_utf16().collect::<Vec<_>>();
            index.extend_from_slice(&id.to_be_bytes());
            index.extend_from_slice(&offset.to_be_bytes());
            index.extend_from_slice(&(units.len() as u16).to_be_bytes());
            for unit in &units {
                payload.extend_from_slice(&unit.to_be_bytes());
            }
            offset += units.len() as u32;
        }
        let size = 12 + index.len() + payload.len();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&(size as u32).to_be_bytes());
        bytes.extend_from_slice(&(payload.len() as u32 / 2).to_be_bytes());
        bytes.extend_from_slice(&index);
        bytes.extend_from_slice(&payload);
        bytes
    }

    #[test]
    fn decodes_only_requested_utf16_labels() {
        let bytes = fixture(&[(10, "Electronics"), (20, "Café")]);
        let requested = BTreeSet::from([20]);
        let decoded = decode_btf(&bytes, &requested).expect("valid BTF");
        assert_eq!(decoded.declared_entry_count, 2);
        assert_eq!(decoded.entries.len(), 1);
        assert_eq!(decoded.entries.get(&20).map(String::as_str), Some("Café"));
    }

    #[test]
    fn duplicate_caption_ids_keep_first_value_and_report_warning() {
        let bytes = fixture(&[(7, "First"), (7, "Second")]);
        let decoded = decode_btf(&bytes, &BTreeSet::new()).expect("valid BTF");
        assert_eq!(decoded.entries.get(&7).map(String::as_str), Some("First"));
        assert_eq!(decoded.warning_count, 1);
    }

    #[test]
    fn rejects_bad_declared_size_and_offsets() {
        let mut bad_size = fixture(&[(1, "A")]);
        bad_size[7] = bad_size[7].saturating_add(1);
        assert!(decode_btf(&bad_size, &BTreeSet::new()).is_err());

        let mut bad_offset = fixture(&[(1, "A")]);
        bad_offset[19] = 100;
        assert!(decode_btf(&bad_offset, &BTreeSet::new()).is_err());
    }

    #[test]
    fn discovers_and_resolves_valid_installed_vocabularies() {
        let directory = tempdir().expect("temporary directory");
        let media = directory.path().join("media_soviet");
        fs::create_dir(&media).expect("media directory");
        fs::write(
            media.join("sovietEnglish.btf"),
            fixture(&[(42, "Electronic components")]),
        )
        .expect("synthetic vocabulary");
        fs::write(media.join("unrelated.ini"), b"ignored").expect("unrelated file");

        let resolved = resolve_game_media_directory(directory.path()).expect("game root");
        let sources = discover_game_vocabularies(&resolved).expect("catalogue");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_id, "installed-game.English");
        assert_eq!(sources[0].locale_hint.as_deref(), Some("en"));
        assert!(sources[0].readable);
        assert_eq!(sources[0].entry_count, Some(1));
        assert!(sources[0].content_hash.is_some());

        let catalogue = load_game_vocabulary_catalogue(&resolved, "en-AU", &BTreeSet::from([42]))
            .expect("loaded catalogue");
        assert_eq!(
            catalogue.resolve(42).map(|value| value.0),
            Some("Electronic components".to_owned())
        );
        assert_eq!(catalogue.revisions().count(), 1);
    }

    #[test]
    fn malformed_vocabulary_is_discovered_but_never_readable() {
        let directory = tempdir().expect("temporary directory");
        let media = directory.path().join("media_soviet");
        fs::create_dir(&media).expect("media directory");
        fs::write(media.join("sovietEnglish.btf"), b"not a BTF").expect("synthetic vocabulary");
        let sources = discover_game_vocabularies(&media).expect("catalogue");
        assert_eq!(sources.len(), 1);
        assert!(!sources[0].readable);
        assert_eq!(sources[0].entry_count, None);
    }
}
