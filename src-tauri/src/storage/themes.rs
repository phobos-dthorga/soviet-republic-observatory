use rusqlite::{Connection, OptionalExtension, params};

use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;
use crate::theme::{
    AvailableThemeRevision, DEFAULT_THEME_ID, DEFAULT_THEME_VERSION, ThemeSource,
    ThemeStatus, built_in_themes, canonical_theme_json, parse_community_theme, parse_stored_theme,
    same_theme_appearance, theme_content_hash, validate_contrast,
};

impl ObservatoryStorage {
    pub fn theme_status(&self) -> Result<ThemeStatus, ObservatoryError> {
        theme_status_from_connection(&self.connect()?)
    }

    pub fn import_theme(&self, document: &str) -> Result<ThemeStatus, ObservatoryError> {
        let manifest = parse_community_theme(document)?;
        let canonical_json = canonical_theme_json(&manifest)?;
        let content_hash = theme_content_hash(&canonical_json);
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let existing_hash = transaction
            .query_row(
                "SELECT content_hash FROM theme_revisions \
                 WHERE theme_id = ?1 AND semantic_version = ?2",
                params![manifest.id, manifest.version],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(existing_hash) = existing_hash {
            return Err(if existing_hash == content_hash {
                ObservatoryError::DuplicateTheme
            } else {
                ObservatoryError::ThemeRevisionConflict
            });
        }
        let duplicate_hash = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM theme_revisions WHERE content_hash = ?1)",
            [&content_hash],
            |row| row.get::<_, bool>(0),
        )?;
        if duplicate_hash {
            return Err(ObservatoryError::DuplicateTheme);
        }
        let available = available_themes_from_connection(&transaction)?;
        if available.iter().any(|theme| {
            theme.manifest.id != manifest.id && same_theme_appearance(&theme.manifest, &manifest)
        }) {
            return Err(ObservatoryError::DuplicateTheme);
        }
        let timestamp = now_ms();
        transaction.execute(
            r#"INSERT INTO theme_revisions(
                   theme_id, semantic_version, content_hash, manifest_json, display_name,
                   author, description, installed_at_ms, updated_at_ms
               ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)"#,
            params![
                manifest.id,
                manifest.version,
                content_hash,
                canonical_json,
                manifest.name,
                manifest.author,
                manifest.description,
                timestamp,
            ],
        )?;
        transaction.commit()?;
        self.theme_status()
    }

    pub fn select_theme(
        &self,
        theme_id: &str,
        version: &str,
        content_hash: &str,
    ) -> Result<ThemeStatus, ObservatoryError> {
        let connection = self.connect()?;
        let available = available_themes_from_connection(&connection)?;
        if !available.iter().any(|theme| {
            theme.manifest.id == theme_id
                && theme.manifest.version == version
                && theme.content_hash == content_hash
        }) {
            return Err(ObservatoryError::UnknownTheme);
        }
        connection.execute(
            "UPDATE theme_preferences SET selected_theme_id = ?1, selected_version = ?2, \
                 selected_content_hash = ?3, updated_at_ms = ?4 WHERE singleton_id = 1",
            params![theme_id, version, content_hash, now_ms()],
        )?;
        theme_status_from_connection(&connection)
    }

    pub fn remove_theme(
        &self,
        theme_id: &str,
        version: &str,
    ) -> Result<ThemeStatus, ObservatoryError> {
        if theme_id.starts_with("org.republic-observatory.") {
            return Err(ObservatoryError::BuiltInThemeRemove);
        }
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let selected = transaction.query_row(
            "SELECT selected_theme_id = ?1 AND selected_version = ?2 \
             FROM theme_preferences WHERE singleton_id = 1",
            params![theme_id, version],
            |row| row.get::<_, bool>(0),
        )?;
        if selected {
            return Err(ObservatoryError::ActiveThemeRemove);
        }
        let removed = transaction.execute(
            "DELETE FROM theme_revisions WHERE theme_id = ?1 AND semantic_version = ?2",
            params![theme_id, version],
        )?;
        if removed == 0 {
            return Err(ObservatoryError::UnknownTheme);
        }
        transaction.commit()?;
        self.theme_status()
    }

    pub fn export_theme(
        &self,
        theme_id: &str,
        version: &str,
        content_hash: &str,
    ) -> Result<String, ObservatoryError> {
        let connection = self.connect()?;
        let available = available_themes_from_connection(&connection)?;
        let theme = available
            .into_iter()
            .find(|theme| {
                theme.manifest.id == theme_id
                    && theme.manifest.version == version
                    && theme.content_hash == content_hash
            })
            .ok_or(ObservatoryError::UnknownTheme)?;
        let mut document = serde_json::to_string_pretty(&theme.manifest)
            .map_err(|_| ObservatoryError::InvalidThemeManifest)?;
        document.push('\n');
        Ok(document)
    }
}

fn theme_status_from_connection(connection: &Connection) -> Result<ThemeStatus, ObservatoryError> {
    let mut themes = available_themes_from_connection(connection)?;
    let requested = connection.query_row(
        "SELECT selected_theme_id, selected_version, selected_content_hash \
         FROM theme_preferences WHERE singleton_id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    )?;
    let requested_index = themes.iter().position(|theme| {
        theme.manifest.id == requested.0
            && theme.manifest.version == requested.1
            && (requested.2.is_empty() || theme.content_hash == requested.2)
    });
    let fallback_applied = requested_index.is_none();
    let selected_index = requested_index
        .or_else(|| {
            themes.iter().position(|theme| {
                theme.manifest.id == DEFAULT_THEME_ID
                    && theme.manifest.version == DEFAULT_THEME_VERSION
            })
        })
        .expect("the validated default theme must always be available");
    let active = themes[selected_index].clone();
    themes[selected_index].selected = true;
    if fallback_applied || requested.2.is_empty() {
        connection.execute(
            "UPDATE theme_preferences SET selected_theme_id = ?1, selected_version = ?2, \
                 selected_content_hash = ?3, updated_at_ms = ?4 WHERE singleton_id = 1",
            params![
                active.manifest.id,
                active.manifest.version,
                active.content_hash,
                now_ms(),
            ],
        )?;
    }
    Ok(ThemeStatus {
        selected_theme_id: active.manifest.id.clone(),
        selected_version: active.manifest.version.clone(),
        selected_content_hash: active.content_hash.clone(),
        active_theme: active.manifest,
        active_report: active.report,
        themes,
        fallback_applied,
        storage_authority: "native_sqlite",
    })
}

fn available_themes_from_connection(
    connection: &Connection,
) -> Result<Vec<AvailableThemeRevision>, ObservatoryError> {
    let mut themes = built_in_themes()
        .iter()
        .map(|manifest| {
            let canonical =
                canonical_theme_json(manifest).expect("validated built-in themes must serialise");
            AvailableThemeRevision {
                manifest: manifest.clone(),
                content_hash: theme_content_hash(&canonical),
                source: ThemeSource::BuiltIn,
                installed_at_ms: None,
                updated_at_ms: None,
                selected: false,
                report: validate_contrast(manifest),
            }
        })
        .collect::<Vec<_>>();
    let mut statement = connection.prepare(
        "SELECT manifest_json, content_hash, installed_at_ms, updated_at_ms \
         FROM theme_revisions ORDER BY display_name COLLATE NOCASE, semantic_version, theme_id",
    )?;
    let stored = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (document, content_hash, installed_at_ms, updated_at_ms) in stored {
        if let Ok(manifest) = parse_stored_theme(&document) {
            let report = crate::theme::inspect_theme_document(&document)
                .report
                .expect("stored validated themes must have a report");
            themes.push(AvailableThemeRevision {
                manifest,
                content_hash,
                source: ThemeSource::LocalImport,
                installed_at_ms: Some(installed_at_ms),
                updated_at_ms: Some(updated_at_ms),
                selected: false,
                report,
            });
        }
    }
    Ok(themes)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::theme::{THEME_SCHEMA_VERSION, ThemeColours, ThemeManifest};

    fn community_theme(id: &str, version: &str) -> ThemeManifest {
        let source = built_in_themes()[0].clone();
        ThemeManifest {
            schema_version: THEME_SCHEMA_VERSION,
            id: id.into(),
            version: version.into(),
            name: "Community Laboratory".into(),
            author: Some("Unverified author".into()),
            description: Some("Synthetic test theme.".into()),
            colours: ThemeColours {
                comparison: "#C5B5E0".into(),
                ..source.colours
            },
            chart_palette: source.chart_palette,
        }
    }

    #[test]
    fn imports_without_selecting_and_pins_an_explicit_revision() {
        let directory = tempdir().unwrap();
        let storage =
            ObservatoryStorage::initialise(directory.path().join("themes.sqlite3")).unwrap();
        let document =
            serde_json::to_string(&community_theme("org.example.laboratory", "1.0.0")).unwrap();
        let imported = storage.import_theme(&document).unwrap();
        assert_eq!(imported.selected_theme_id, DEFAULT_THEME_ID);
        let local = imported
            .themes
            .iter()
            .find(|theme| theme.manifest.id == "org.example.laboratory")
            .unwrap();
        let selected = storage
            .select_theme(
                &local.manifest.id,
                &local.manifest.version,
                &local.content_hash,
            )
            .unwrap();
        assert_eq!(selected.selected_theme_id, "org.example.laboratory");
        assert_eq!(selected.selected_content_hash, local.content_hash);
    }

    #[test]
    fn revisions_are_immutable_and_active_revisions_cannot_be_removed() {
        let directory = tempdir().unwrap();
        let storage =
            ObservatoryStorage::initialise(directory.path().join("themes.sqlite3")).unwrap();
        let first = community_theme("org.example.laboratory", "1.0.0");
        let document = serde_json::to_string(&first).unwrap();
        let imported = storage.import_theme(&document).unwrap();
        assert!(matches!(
            storage.import_theme(&document),
            Err(ObservatoryError::DuplicateTheme)
        ));
        let local = imported
            .themes
            .iter()
            .find(|theme| theme.manifest.id == first.id)
            .unwrap();
        storage
            .select_theme(&first.id, &first.version, &local.content_hash)
            .unwrap();
        assert!(matches!(
            storage.remove_theme(&first.id, &first.version),
            Err(ObservatoryError::ActiveThemeRemove)
        ));
    }

    #[test]
    fn appearance_duplicates_fail_and_corrupt_selected_revisions_fall_back_intact() {
        let directory = tempdir().unwrap();
        let storage =
            ObservatoryStorage::initialise(directory.path().join("themes.sqlite3")).unwrap();

        let mut duplicate = built_in_themes()[0].clone();
        duplicate.id = "org.example.classic-copy".into();
        duplicate.name = "Rebadged classic".into();
        assert!(matches!(
            storage.import_theme(&serde_json::to_string(&duplicate).unwrap()),
            Err(ObservatoryError::DuplicateTheme)
        ));

        let local = community_theme("org.example.recovery", "1.0.0");
        let imported = storage
            .import_theme(&serde_json::to_string(&local).unwrap())
            .unwrap();
        let revision = imported
            .themes
            .iter()
            .find(|theme| theme.manifest.id == local.id)
            .unwrap();
        storage
            .select_theme(&local.id, &local.version, &revision.content_hash)
            .unwrap();
        storage
            .connect()
            .unwrap()
            .execute(
                "UPDATE theme_revisions SET manifest_json = '{\"corrupt\":true}' WHERE theme_id = ?1",
                [&local.id],
            )
            .unwrap();

        let recovered = storage.theme_status().unwrap();
        assert!(recovered.fallback_applied);
        assert_eq!(recovered.selected_theme_id, DEFAULT_THEME_ID);
        assert_eq!(
            storage
                .connect()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM theme_revisions WHERE theme_id = ?1",
                    [&local.id],
                    |row| row.get::<_, u32>(0),
                )
                .unwrap(),
            1
        );
    }
}
