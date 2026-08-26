use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ObservatoryError;
use crate::model::GameVocabularySource;

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
            let file_name = entry.file_name().to_str()?.to_owned();
            let suffix = file_name
                .strip_prefix("soviet")?
                .strip_suffix(".btf")?
                .to_owned();
            (file_type.is_file() && !suffix.is_empty()).then(|| GameVocabularySource {
                source_id: format!("installed-game.{suffix}"),
                file_name,
                locale_hint: locale_hint(&suffix).map(str::to_owned),
                format: "btf".to_owned(),
                readable: false,
            })
        })
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    Ok(sources)
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
    use std::fs;

    use tempfile::tempdir;

    use super::{discover_game_vocabularies, resolve_game_media_directory};

    #[test]
    fn discovers_identity_only_without_copying_or_decoding_game_text() {
        let directory = tempdir().expect("temporary directory");
        let media = directory.path().join("media_soviet");
        fs::create_dir(&media).expect("media directory");
        fs::write(media.join("sovietEnglish.btf"), b"synthetic binary")
            .expect("synthetic vocabulary");
        fs::write(media.join("unrelated.ini"), b"ignored").expect("unrelated file");

        let resolved = resolve_game_media_directory(directory.path()).expect("game root");
        let sources = discover_game_vocabularies(&resolved).expect("catalogue");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_id, "installed-game.English");
        assert_eq!(sources[0].locale_hint.as_deref(), Some("en"));
        assert!(!sources[0].readable);
    }
}
