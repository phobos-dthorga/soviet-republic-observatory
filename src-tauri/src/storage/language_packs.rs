use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, params};

use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;
use crate::language_pack::{
    AvailableLanguagePack, DEFAULT_LANGUAGE_PACK_ID, LanguagePackTrust, LanguageStatus,
    LegacyLanguageHandover, MAX_LEGACY_HANDOVER_BYTES, MAX_LEGACY_LANGUAGE_PACKS, TextDirection,
    canonical_manifest_json, eligible_message_count, manifest_content_hash,
    parse_community_manifest, source_catalog,
};

impl ObservatoryStorage {
    pub fn language_status(&self) -> Result<LanguageStatus, ObservatoryError> {
        language_status_from_connection(&self.connect()?)
    }

    pub fn install_language_pack(
        &self,
        manifest_json: &str,
    ) -> Result<LanguageStatus, ObservatoryError> {
        let manifest = parse_community_manifest(manifest_json)?;
        let canonical_json = canonical_manifest_json(&manifest)?;
        let content_hash = manifest_content_hash(&canonical_json);
        let timestamp = now_ms();
        self.connect()?.execute(
            r#"INSERT INTO language_pack_manifests(
                   pack_id, content_hash, manifest_json, locale, display_name, author,
                   source_catalog_version, source_catalog_revision, direction,
                   translated_message_count, installed_at_ms, updated_at_ms
               ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
               ON CONFLICT(pack_id) DO UPDATE SET
                   content_hash = excluded.content_hash,
                   manifest_json = excluded.manifest_json,
                   locale = excluded.locale,
                   display_name = excluded.display_name,
                   author = excluded.author,
                   source_catalog_version = excluded.source_catalog_version,
                   source_catalog_revision = excluded.source_catalog_revision,
                   direction = excluded.direction,
                   translated_message_count = excluded.translated_message_count,
                   updated_at_ms = excluded.updated_at_ms"#,
            params![
                manifest.id,
                content_hash,
                canonical_json,
                manifest.locale,
                manifest.name,
                manifest.author,
                manifest.source_catalog_version,
                manifest.source_catalog_revision,
                direction_value(manifest.direction),
                u32::try_from(manifest.messages.len()).unwrap_or(u32::MAX),
                timestamp,
            ],
        )?;
        self.language_status()
    }

    pub fn select_language_pack(&self, pack_id: &str) -> Result<LanguageStatus, ObservatoryError> {
        let connection = self.connect()?;
        if pack_id != DEFAULT_LANGUAGE_PACK_ID {
            let exists = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM language_pack_manifests WHERE pack_id = ?1)",
                [pack_id],
                |row| row.get::<_, bool>(0),
            )?;
            if !exists {
                return Err(ObservatoryError::UnknownLanguagePack);
            }
        }
        connection.execute(
            "UPDATE language_preferences SET selected_pack_id = ?1, updated_at_ms = ?2 \
             WHERE singleton_id = 1",
            params![pack_id, now_ms()],
        )?;
        language_status_from_connection(&connection)
    }

    pub fn remove_language_pack(&self, pack_id: &str) -> Result<LanguageStatus, ObservatoryError> {
        if pack_id == DEFAULT_LANGUAGE_PACK_ID {
            return Err(ObservatoryError::BuiltInLanguagePackRemove);
        }
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let removed = transaction.execute(
            "DELETE FROM language_pack_manifests WHERE pack_id = ?1",
            [pack_id],
        )?;
        if removed == 0 {
            return Err(ObservatoryError::UnknownLanguagePack);
        }
        transaction.execute(
            "UPDATE language_preferences SET \
                 selected_pack_id = CASE WHEN selected_pack_id = ?1 \
                     THEN ?2 ELSE selected_pack_id END, \
                 updated_at_ms = ?3 \
             WHERE singleton_id = 1",
            params![pack_id, DEFAULT_LANGUAGE_PACK_ID, now_ms()],
        )?;
        transaction.commit()?;
        self.language_status()
    }

    pub fn export_language_pack(&self, pack_id: &str) -> Result<String, ObservatoryError> {
        if pack_id == DEFAULT_LANGUAGE_PACK_ID {
            return canonical_manifest_json(source_catalog());
        }
        self.connect()?
            .query_row(
                "SELECT manifest_json FROM language_pack_manifests WHERE pack_id = ?1",
                [pack_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownLanguagePack)
    }

    pub fn handover_legacy_language_packs(
        &self,
        handover: &LegacyLanguageHandover,
    ) -> Result<LanguageStatus, ObservatoryError> {
        let mut connection = self.connect()?;
        let (already_completed, native_choice_made) = connection.query_row(
            "SELECT legacy_handover_completed_at_ms IS NOT NULL, updated_at_ms > 0 \
             FROM language_preferences WHERE singleton_id = 1",
            [],
            |row| Ok((row.get::<_, bool>(0)?, row.get::<_, bool>(1)?)),
        )?;
        if already_completed {
            return language_status_from_connection(&connection);
        }
        if native_choice_made {
            connection.execute(
                "UPDATE language_preferences SET legacy_handover_completed_at_ms = ?1 \
                 WHERE singleton_id = 1",
                [now_ms()],
            )?;
            return language_status_from_connection(&connection);
        }
        if handover.manifests.len() > MAX_LEGACY_LANGUAGE_PACKS
            || handover.manifests.iter().map(String::len).sum::<usize>() > MAX_LEGACY_HANDOVER_BYTES
        {
            return Err(ObservatoryError::LanguageManifestTooLarge);
        }

        let mut parsed = Vec::with_capacity(handover.manifests.len());
        let mut ids = BTreeSet::new();
        for document in &handover.manifests {
            let manifest = parse_community_manifest(document)?;
            if !ids.insert(manifest.id.clone()) {
                return Err(ObservatoryError::InvalidLanguageManifest);
            }
            let canonical_json = canonical_manifest_json(&manifest)?;
            let content_hash = manifest_content_hash(&canonical_json);
            parsed.push((manifest, canonical_json, content_hash));
        }

        let transaction = connection.transaction()?;
        let (already_completed, native_choice_made) = transaction.query_row(
            "SELECT legacy_handover_completed_at_ms IS NOT NULL, updated_at_ms > 0 \
             FROM language_preferences WHERE singleton_id = 1",
            [],
            |row| Ok((row.get::<_, bool>(0)?, row.get::<_, bool>(1)?)),
        )?;
        if already_completed {
            transaction.commit()?;
            return language_status_from_connection(&connection);
        }

        let timestamp = now_ms();
        if native_choice_made {
            transaction.execute(
                "UPDATE language_preferences SET legacy_handover_completed_at_ms = ?1 \
                 WHERE singleton_id = 1",
                [timestamp],
            )?;
            transaction.commit()?;
            return language_status_from_connection(&connection);
        }
        for (manifest, canonical_json, content_hash) in &parsed {
            transaction.execute(
                r#"INSERT INTO language_pack_manifests(
                       pack_id, content_hash, manifest_json, locale, display_name, author,
                       source_catalog_version, source_catalog_revision, direction,
                       translated_message_count, installed_at_ms, updated_at_ms
                   ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
                   ON CONFLICT(pack_id) DO NOTHING"#,
                params![
                    manifest.id,
                    content_hash,
                    canonical_json,
                    manifest.locale,
                    manifest.name,
                    manifest.author,
                    manifest.source_catalog_version,
                    manifest.source_catalog_revision,
                    direction_value(manifest.direction),
                    u32::try_from(manifest.messages.len()).unwrap_or(u32::MAX),
                    timestamp,
                ],
            )?;
        }
        let selected = handover
            .selected_language_pack_id
            .as_deref()
            .filter(|candidate| *candidate == DEFAULT_LANGUAGE_PACK_ID || ids.contains(*candidate))
            .unwrap_or(DEFAULT_LANGUAGE_PACK_ID);
        transaction.execute(
            "UPDATE language_preferences SET selected_pack_id = ?1, \
                 legacy_handover_completed_at_ms = ?2, updated_at_ms = ?2 \
             WHERE singleton_id = 1",
            params![selected, timestamp],
        )?;
        transaction.commit()?;
        self.language_status()
    }
}

fn direction_value(direction: TextDirection) -> &'static str {
    match direction {
        TextDirection::LeftToRight => "left_to_right",
        TextDirection::RightToLeft => "right_to_left",
    }
}

fn language_status_from_connection(
    connection: &Connection,
) -> Result<LanguageStatus, ObservatoryError> {
    let eligible_messages = eligible_message_count();
    let source = source_catalog().clone();
    let mut packs = vec![AvailableLanguagePack {
        manifest: source.clone(),
        trust: LanguagePackTrust::BuiltIn,
        translated_messages: eligible_messages,
        eligible_messages,
    }];

    let mut statement = connection.prepare(
        "SELECT pack_id, manifest_json FROM language_pack_manifests \
         ORDER BY display_name COLLATE NOCASE, pack_id",
    )?;
    let stored = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (pack_id, manifest_json) in stored {
        if let Ok(manifest) = parse_community_manifest(&manifest_json)
            && manifest.id == pack_id
        {
            packs.push(AvailableLanguagePack {
                translated_messages: manifest.messages.len(),
                eligible_messages,
                manifest,
                trust: LanguagePackTrust::Community,
            });
        }
    }

    let requested_id = connection.query_row(
        "SELECT selected_pack_id FROM language_preferences WHERE singleton_id = 1",
        [],
        |row| row.get::<_, String>(0),
    )?;
    let active_pack = packs
        .iter()
        .find(|pack| pack.manifest.id == requested_id)
        .unwrap_or(&packs[0])
        .manifest
        .clone();
    Ok(LanguageStatus {
        selected_language_pack_id: active_pack.id.clone(),
        active_pack,
        packs,
        storage_authority: "native_sqlite",
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::tempdir;

    use super::*;
    use crate::language_pack::{
        LANGUAGE_PACK_SCHEMA_VERSION, LanguagePackManifest, SOURCE_CATALOG_REVISION,
        SOURCE_CATALOG_VERSION, SOURCE_LOCALE, TextDirection,
    };

    fn manifest(id: &str) -> String {
        canonical_manifest_json(&LanguagePackManifest {
            schema_version: LANGUAGE_PACK_SCHEMA_VERSION,
            id: id.to_owned(),
            locale: "fr-FR".to_owned(),
            name: "Français".to_owned(),
            author: None,
            source_locale: SOURCE_LOCALE.to_owned(),
            source_catalog_version: SOURCE_CATALOG_VERSION,
            source_catalog_revision: SOURCE_CATALOG_REVISION,
            direction: TextDirection::LeftToRight,
            messages: BTreeMap::from([("action-close".to_owned(), "Fermer".to_owned())]),
        })
        .expect("manifest")
    }

    #[test]
    fn installation_selection_export_and_removal_are_distinct_and_persistent() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("observatory.sqlite3");
        let storage = ObservatoryStorage::initialise(path.clone()).expect("storage");
        let installed = storage
            .install_language_pack(&manifest("community-fr"))
            .expect("install");
        assert_eq!(installed.packs.len(), 2);
        assert_eq!(
            installed.selected_language_pack_id,
            DEFAULT_LANGUAGE_PACK_ID
        );
        assert!(
            storage
                .export_language_pack("community-fr")
                .expect("export")
                .contains("community-fr")
        );
        let selected = storage
            .select_language_pack("community-fr")
            .expect("select");
        assert_eq!(selected.selected_language_pack_id, "community-fr");
        drop(storage);

        let reopened = ObservatoryStorage::initialise(path).expect("reopen");
        assert_eq!(
            reopened
                .language_status()
                .expect("status")
                .selected_language_pack_id,
            "community-fr"
        );
        let removed = reopened
            .remove_language_pack("community-fr")
            .expect("remove");
        assert_eq!(removed.selected_language_pack_id, DEFAULT_LANGUAGE_PACK_ID);
    }

    #[test]
    fn legacy_handover_is_atomic_and_idempotent() {
        let directory = tempdir().expect("temporary directory");
        let storage =
            ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");
        let first = LegacyLanguageHandover {
            manifests: vec![manifest("community-fr")],
            selected_language_pack_id: Some("community-fr".to_owned()),
        };
        assert_eq!(
            storage
                .handover_legacy_language_packs(&first)
                .expect("handover")
                .selected_language_pack_id,
            "community-fr"
        );
        let ignored = LegacyLanguageHandover {
            manifests: vec![manifest("community-other")],
            selected_language_pack_id: Some("community-other".to_owned()),
        };
        let status = storage
            .handover_legacy_language_packs(&ignored)
            .expect("idempotent handover");
        assert_eq!(status.selected_language_pack_id, "community-fr");
        assert_eq!(status.packs.len(), 2);
    }

    #[test]
    fn corrupt_stored_pack_and_stale_selection_fall_back_without_blocking_startup() {
        let directory = tempdir().expect("temporary directory");
        let storage =
            ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");
        storage
            .install_language_pack(&manifest("community-fr"))
            .expect("install");
        storage
            .select_language_pack("community-fr")
            .expect("select");
        storage
            .connect()
            .expect("connection")
            .execute(
                "UPDATE language_pack_manifests SET manifest_json = '{' WHERE pack_id = ?1",
                ["community-fr"],
            )
            .expect("corrupt test record");
        let status = storage.language_status().expect("resilient status");
        assert_eq!(status.selected_language_pack_id, DEFAULT_LANGUAGE_PACK_ID);
        assert_eq!(status.packs.len(), 1);

        let oversized = LegacyLanguageHandover {
            manifests: vec!["x".repeat(MAX_LEGACY_HANDOVER_BYTES + 1)],
            selected_language_pack_id: None,
        };
        storage
            .connect()
            .expect("connection")
            .execute(
                "UPDATE language_preferences SET legacy_handover_completed_at_ms = 1",
                [],
            )
            .expect("mark handover complete");
        storage
            .handover_legacy_language_packs(&oversized)
            .expect("completed handover ignores stale payload");
    }

    #[test]
    fn a_native_selection_wins_a_startup_handover_race() {
        let directory = tempdir().expect("temporary directory");
        let storage =
            ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");
        storage
            .install_language_pack(&manifest("community-native"))
            .expect("native install");
        storage
            .select_language_pack("community-native")
            .expect("native selection");
        let stale_legacy = LegacyLanguageHandover {
            manifests: vec![manifest("community-legacy")],
            selected_language_pack_id: Some("community-legacy".to_owned()),
        };
        let status = storage
            .handover_legacy_language_packs(&stale_legacy)
            .expect("handover");
        assert_eq!(status.selected_language_pack_id, "community-native");
        assert!(
            status
                .packs
                .iter()
                .all(|pack| pack.manifest.id != "community-legacy")
        );
    }
}
