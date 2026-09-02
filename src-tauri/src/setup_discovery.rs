use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::model::DirectoryKind;

const STEAM_APP_ID: &str = "784150";
const MAX_LIBRARY_FILE_BYTES: u64 = 512 * 1024;
const MAX_MANIFEST_BYTES: u64 = 128 * 1024;
const MAX_QUOTED_FIELDS: usize = 16_384;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SetupDirectorySuggestion {
    pub path: PathBuf,
}

pub(crate) fn suggest_directory(
    kind: DirectoryKind,
    configured: Option<&Path>,
) -> Option<SetupDirectorySuggestion> {
    if let Some(path) = configured.filter(|path| path.is_dir()) {
        return Some(SetupDirectorySuggestion {
            path: path.to_path_buf(),
        });
    }

    suggest_from_steam_roots(kind, system_steam_roots())
}

/// Windows file dialogs do not consistently honour extended-length paths as
/// their initial folder. Stored paths remain canonical; only the picker-facing
/// copy is converted back to the ordinary form.
pub(crate) fn picker_start_directory(path: &Path) -> PathBuf {
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

fn suggest_from_steam_roots(
    kind: DirectoryKind,
    roots: impl IntoIterator<Item = PathBuf>,
) -> Option<SetupDirectorySuggestion> {
    let libraries = discover_libraries(roots);
    let installations = libraries
        .iter()
        .filter_map(|library| confirmed_installation(library))
        .collect::<Vec<_>>();

    let path = match kind {
        DirectoryKind::Game => installations
            .iter()
            .map(|installation| installation.join("media_soviet"))
            .find(|path| path.is_dir()),
        DirectoryKind::Save => installations
            .iter()
            .flat_map(|installation| {
                let media = installation.join("media_soviet");
                [media.join("save_cloud"), media.join("save")]
            })
            .filter(|path| path.is_dir())
            .max_by_key(|path| {
                let modified = newest_zip_modified(path);
                (
                    modified.is_some(),
                    modified.unwrap_or(0),
                    path.file_name().is_some_and(|name| name == "save_cloud"),
                )
            }),
        DirectoryKind::Workshop => libraries
            .iter()
            .filter(|library| confirmed_installation(library).is_some())
            .map(|library| {
                library
                    .join("steamapps")
                    .join("workshop")
                    .join("content")
                    .join(STEAM_APP_ID)
            })
            .find(|path| path.is_dir()),
    }?;

    Some(SetupDirectorySuggestion { path })
}

fn discover_libraries(roots: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut libraries = Vec::new();
    let mut seen = HashSet::new();
    for root in roots {
        add_existing_directory(&mut libraries, &mut seen, root.clone());
        let library_file = root.join("steamapps").join("libraryfolders.vdf");
        for library in parse_library_file(&library_file) {
            add_existing_directory(&mut libraries, &mut seen, library);
        }
    }
    libraries
}

fn add_existing_directory(paths: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, path: PathBuf) {
    if !path.is_dir() {
        return;
    }
    let identity = path.canonicalize().unwrap_or_else(|_| path.clone());
    if seen.insert(identity) {
        paths.push(path);
    }
}

fn parse_library_file(path: &Path) -> Vec<PathBuf> {
    let Some(fields) = read_bounded_quoted_fields(path, MAX_LIBRARY_FILE_BYTES) else {
        return Vec::new();
    };
    fields
        .windows(2)
        .filter_map(|pair| {
            let key = &pair[0];
            let value = &pair[1];
            ((key.eq_ignore_ascii_case("path")
                || key.chars().all(|character| character.is_ascii_digit()))
                && looks_like_absolute_path(value))
            .then(|| PathBuf::from(value))
        })
        .collect()
}

fn confirmed_installation(library: &Path) -> Option<PathBuf> {
    let steamapps = library.join("steamapps");
    let manifest = steamapps.join(format!("appmanifest_{STEAM_APP_ID}.acf"));
    let fields = read_bounded_quoted_fields(&manifest, MAX_MANIFEST_BYTES)?;
    let install_dir = fields.windows(2).find_map(|pair| {
        pair[0]
            .eq_ignore_ascii_case("installdir")
            .then_some(pair[1].as_str())
    })?;
    if install_dir.is_empty()
        || install_dir.contains('/')
        || install_dir.contains('\\')
        || install_dir == "."
        || install_dir == ".."
    {
        return None;
    }
    let installation = steamapps.join("common").join(install_dir);
    installation
        .join("media_soviet")
        .is_dir()
        .then_some(installation)
}

fn newest_zip_modified(directory: &Path) -> Option<u128> {
    fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        })
        .filter_map(|entry| entry.metadata().ok()?.modified().ok())
        .filter_map(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .max()
}

fn read_bounded_quoted_fields(path: &Path, limit: u64) -> Option<Vec<String>> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > limit {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    parse_quoted_fields(&bytes)
}

fn parse_quoted_fields(bytes: &[u8]) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }
        index += 1;
        let mut field = Vec::new();
        let mut terminated = false;
        while index < bytes.len() {
            match bytes[index] {
                b'"' => {
                    index += 1;
                    terminated = true;
                    break;
                }
                b'\\' if index + 1 < bytes.len() && matches!(bytes[index + 1], b'\\' | b'"') => {
                    field.push(bytes[index + 1]);
                    index += 2;
                }
                byte => {
                    field.push(byte);
                    index += 1;
                }
            }
        }
        if !terminated || fields.len() >= MAX_QUOTED_FIELDS {
            return None;
        }
        fields.push(String::from_utf8(field).ok()?);
    }
    Some(fields)
}

fn looks_like_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/'))
        || value.starts_with("/")
}

fn system_steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

        if let Ok(steam) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam") {
            if let Ok(path) = steam.get_value::<String, _>("SteamPath") {
                roots.push(PathBuf::from(path));
            } else if let Ok(path) = steam.get_value::<String, _>("InstallPath") {
                roots.push(PathBuf::from(path));
            }
        }
        for key in [
            "SOFTWARE\\WOW6432Node\\Valve\\Steam",
            "SOFTWARE\\Valve\\Steam",
        ] {
            if let Ok(steam) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(key)
                && let Ok(path) = steam.get_value::<String, _>("InstallPath")
            {
                roots.push(PathBuf::from(path));
            }
        }
    }

    for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(root) = std::env::var_os(variable) {
            roots.push(PathBuf::from(root).join("Steam"));
        }
    }
    roots
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use tempfile::TempDir;

    use super::{
        MAX_LIBRARY_FILE_BYTES, STEAM_APP_ID, parse_library_file, picker_start_directory,
        suggest_from_steam_roots,
    };
    use crate::model::DirectoryKind;

    fn installation(root: &TempDir, library_name: &str) -> std::path::PathBuf {
        let library = root.path().join(library_name);
        let steamapps = library.join("steamapps");
        let game = steamapps.join("common").join("SovietRepublic");
        fs::create_dir_all(game.join("media_soviet").join("save_cloud")).expect("cloud saves");
        fs::create_dir_all(game.join("media_soviet").join("save")).expect("local saves");
        fs::write(
            steamapps.join(format!("appmanifest_{STEAM_APP_ID}.acf")),
            r#""AppState" { "appid" "784150" "installdir" "SovietRepublic" }"#,
        )
        .expect("manifest");
        library
    }

    #[test]
    fn discovers_game_save_and_workshop_from_confirmed_library() {
        let root = TempDir::new().expect("temp root");
        let library = installation(&root, "Steam");
        let workshop = library
            .join("steamapps")
            .join("workshop")
            .join("content")
            .join(STEAM_APP_ID);
        fs::create_dir_all(&workshop).expect("workshop");

        assert!(
            suggest_from_steam_roots(DirectoryKind::Game, [library.clone()])
                .expect("game")
                .path
                .ends_with("media_soviet")
        );
        assert!(
            suggest_from_steam_roots(DirectoryKind::Save, [library.clone()])
                .expect("save")
                .path
                .ends_with("save_cloud")
        );
        assert_eq!(
            suggest_from_steam_roots(DirectoryKind::Workshop, [library])
                .expect("workshop")
                .path,
            workshop
        );
    }

    #[test]
    fn follows_secondary_libraries_and_prefers_the_newest_zip_save() {
        let root = TempDir::new().expect("temp root");
        let primary = root.path().join("Steam");
        fs::create_dir_all(primary.join("steamapps")).expect("primary");
        let secondary = installation(&root, "Secondary");
        fs::write(
            primary.join("steamapps").join("libraryfolders.vdf"),
            format!(
                r#""libraryfolders" {{ "1" {{ "path" "{}" }} }}"#,
                secondary.display()
            ),
        )
        .expect("libraries");
        let cloud = secondary.join("steamapps/common/SovietRepublic/media_soviet/save_cloud");
        let local = secondary.join("steamapps/common/SovietRepublic/media_soviet/save");
        fs::write(cloud.join("older.zip"), b"not opened").expect("older save");
        std::thread::sleep(Duration::from_millis(20));
        fs::write(local.join("newer.ZIP"), b"not opened").expect("newer save");

        let suggestion = suggest_from_steam_roots(DirectoryKind::Save, [primary]).expect("save");
        assert_eq!(suggestion.path, local);
    }

    #[test]
    fn malformed_and_oversized_library_files_fail_closed() {
        let root = TempDir::new().expect("temp root");
        let malformed = root.path().join("malformed.vdf");
        fs::write(&malformed, b"\"path\" \"unterminated").expect("malformed");
        assert!(parse_library_file(&malformed).is_empty());

        let oversized = root.path().join("oversized.vdf");
        let file = fs::File::create(&oversized).expect("oversized");
        file.set_len(MAX_LIBRARY_FILE_BYTES + 1).expect("length");
        assert!(parse_library_file(&oversized).is_empty());

        let relative = root.path().join("relative.vdf");
        fs::write(&relative, r#""path" "..\\untrusted""#).expect("relative path");
        assert!(parse_library_file(&relative).is_empty());
    }

    #[test]
    fn configured_directory_wins_without_reading_steam() {
        let root = TempDir::new().expect("temp root");
        let configured = root.path().join("chosen");
        fs::create_dir(&configured).expect("configured");
        let suggestion = super::suggest_directory(DirectoryKind::Save, Some(&configured))
            .expect("configured suggestion");
        assert_eq!(suggestion.path, configured);
    }

    #[test]
    fn picker_start_paths_remain_specific_to_each_directory_kind() {
        let root = TempDir::new().expect("temp root");
        let save = root.path().join("save_cloud");
        let game = root.path().join("media_soviet");
        let workshop = root.path().join("workshop").join(STEAM_APP_ID);
        for path in [&save, &game, &workshop] {
            fs::create_dir_all(path).expect("configured directory");
        }

        assert_eq!(
            picker_start_directory(
                &super::suggest_directory(DirectoryKind::Save, Some(&save))
                    .expect("save suggestion")
                    .path,
            ),
            save
        );
        assert_eq!(
            picker_start_directory(
                &super::suggest_directory(DirectoryKind::Game, Some(&game))
                    .expect("game suggestion")
                    .path,
            ),
            game
        );
        assert_eq!(
            picker_start_directory(
                &super::suggest_directory(DirectoryKind::Workshop, Some(&workshop))
                    .expect("workshop suggestion")
                    .path,
            ),
            workshop
        );
    }

    #[cfg(windows)]
    #[test]
    fn picker_converts_extended_windows_paths_without_changing_storage_identity() {
        assert_eq!(
            picker_start_directory(std::path::Path::new(
                r"\\?\C:\Games\SovietRepublic\media_soviet"
            )),
            std::path::PathBuf::from(r"C:\Games\SovietRepublic\media_soviet")
        );
        assert_eq!(
            picker_start_directory(std::path::Path::new(r"\\?\UNC\server\share\SovietRepublic")),
            std::path::PathBuf::from(r"\\server\share\SovietRepublic")
        );
    }
}
