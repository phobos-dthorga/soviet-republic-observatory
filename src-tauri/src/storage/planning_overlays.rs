use rusqlite::{OptionalExtension, params};

use super::warehouse_jobs::enqueue_projection_job;
use super::warehouse_jobs::content_derived_projection_id;
use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;
use crate::model::OverlayProfileSummary;
use crate::planning_overlay::PlanningOverlayDocument;

impl ObservatoryStorage {
    pub fn install_overlay(
        &self,
        document: &PlanningOverlayDocument,
    ) -> Result<OverlayProfileSummary, ObservatoryError> {
        document.validate()?;
        let content_hash = document.content_hash()?;
        let document_json = document.canonical_json()?;
        let installed_at = now_ms();
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO planning_overlay_profiles(\
                 profile_id, display_name, active_revision, removed_at_ms, created_at_ms, updated_at_ms\
             ) VALUES(?1, ?2, NULL, NULL, ?3, ?3) \
             ON CONFLICT(profile_id) DO UPDATE SET \
                 display_name = excluded.display_name, removed_at_ms = NULL, updated_at_ms = excluded.updated_at_ms",
            params![document.id, document.name, installed_at],
        )?;
        let existing = transaction
            .query_row(
                "SELECT revision FROM planning_overlay_revisions WHERE content_hash = ?1",
                [&content_hash],
                |row| row.get::<_, u32>(0),
            )
            .optional()?;
        let revision = if let Some(existing) = existing {
            existing
        } else {
            let next = transaction.query_row(
                "SELECT COALESCE(MAX(revision), 0) + 1 FROM planning_overlay_revisions \
                 WHERE profile_id = ?1",
                [&document.id],
                |row| row.get::<_, u32>(0),
            )?;
            transaction.execute(
                "INSERT INTO planning_overlay_revisions(\
                     profile_id, revision, content_hash, semantic_version, author, default_locale, \
                     description, document_json, installed_at_ms\
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    document.id,
                    next,
                    content_hash,
                    document.version,
                    document.author,
                    document.default_locale,
                    document.description,
                    document_json,
                    installed_at,
                ],
            )?;
            next
        };
        transaction.execute(
            "UPDATE planning_overlay_profiles SET active_revision = COALESCE(active_revision, ?1), \
                 updated_at_ms = ?2 WHERE profile_id = ?3",
            params![revision, installed_at, document.id],
        )?;
        transaction.commit()?;
        self.overlay_profile_summary(&document.id)
    }

    pub fn activate_overlay(
        &self,
        profile_id: &str,
        revision: Option<u32>,
    ) -> Result<OverlayProfileSummary, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let selected_revision = if let Some(revision) = revision {
            revision
        } else {
            transaction
                .query_row(
                    "SELECT MAX(revision) FROM planning_overlay_revisions WHERE profile_id = ?1",
                    [profile_id],
                    |row| row.get::<_, Option<u32>>(0),
                )?
                .ok_or(ObservatoryError::UnknownPlanningOverlay)?
        };
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM planning_overlay_revisions \
             WHERE profile_id = ?1 AND revision = ?2)",
            params![profile_id, selected_revision],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(ObservatoryError::UnknownPlanningOverlay);
        }
        let changed_at = now_ms();
        transaction.execute(
            "UPDATE planning_overlay_state SET active_profile_id = ?1, active_revision = ?2 \
             WHERE singleton_id = 1",
            params![profile_id, selected_revision],
        )?;
        transaction.execute(
            "UPDATE planning_overlay_profiles SET active_revision = ?1, updated_at_ms = ?2 \
             WHERE profile_id = ?3",
            params![selected_revision, changed_at, profile_id],
        )?;
        enqueue_overlay_state_job(
            &transaction,
            profile_id,
            Some(selected_revision),
            changed_at,
        )?;
        transaction.commit()?;
        self.overlay_profile_summary(profile_id)
    }

    pub fn deactivate_overlay(&self) -> Result<(), ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let changed_at = now_ms();
        transaction.execute(
            "UPDATE planning_overlay_state SET active_profile_id = NULL, active_revision = NULL \
             WHERE singleton_id = 1",
            [],
        )?;
        enqueue_overlay_state_job(&transaction, "none", None, changed_at)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn rollback_overlay(
        &self,
        profile_id: &str,
    ) -> Result<OverlayProfileSummary, ObservatoryError> {
        let summary = self.overlay_profile_summary(profile_id)?;
        let current = summary
            .active_revision
            .ok_or(ObservatoryError::UnknownPlanningOverlay)?;
        let previous = self
            .connect()?
            .query_row(
                "SELECT MAX(revision) FROM planning_overlay_revisions \
             WHERE profile_id = ?1 AND revision < ?2",
                params![profile_id, current],
                |row| row.get::<_, Option<u32>>(0),
            )?
            .ok_or(ObservatoryError::UnknownPlanningOverlay)?;
        self.activate_overlay(profile_id, Some(previous))
    }

    pub fn remove_overlay(&self, profile_id: &str) -> Result<(), ObservatoryError> {
        let active = self.active_overlay_identity()?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let removed_at = now_ms();
        let updated = transaction.execute(
            "UPDATE planning_overlay_profiles SET removed_at_ms = ?1, updated_at_ms = ?1 \
             WHERE profile_id = ?2 AND removed_at_ms IS NULL",
            params![removed_at, profile_id],
        )?;
        if updated == 0 {
            return Err(ObservatoryError::UnknownPlanningOverlay);
        }
        if active.as_ref().is_some_and(|value| value.0 == profile_id) {
            transaction.execute(
                "UPDATE planning_overlay_state SET active_profile_id = NULL, active_revision = NULL \
                 WHERE singleton_id = 1",
                [],
            )?;
            enqueue_overlay_state_job(&transaction, "none", None, removed_at)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn list_overlay_profiles(&self) -> Result<Vec<OverlayProfileSummary>, ObservatoryError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT profile_id FROM planning_overlay_profiles \
             WHERE removed_at_ms IS NULL ORDER BY display_name, profile_id",
        )?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|profile_id| self.overlay_profile_summary(profile_id))
            .collect()
    }

    pub fn active_overlay_document(
        &self,
    ) -> Result<Option<(String, u32, PlanningOverlayDocument)>, ObservatoryError> {
        let Some((profile_id, revision)) = self.active_overlay_identity()? else {
            return Ok(None);
        };
        Ok(Some((
            profile_id.clone(),
            revision,
            self.overlay_document(&profile_id, revision)?,
        )))
    }

    pub fn overlay_document(
        &self,
        profile_id: &str,
        revision: u32,
    ) -> Result<PlanningOverlayDocument, ObservatoryError> {
        let json = self
            .connect()?
            .query_row(
                "SELECT document_json FROM planning_overlay_revisions \
                 WHERE profile_id = ?1 AND revision = ?2",
                params![profile_id, revision],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownPlanningOverlay)?;
        PlanningOverlayDocument::parse(json.as_bytes())
    }

    pub fn active_overlay_summary(
        &self,
    ) -> Result<Option<OverlayProfileSummary>, ObservatoryError> {
        self.active_overlay_identity()?
            .map(|(profile_id, _)| self.overlay_profile_summary(&profile_id))
            .transpose()
    }

    fn active_overlay_identity(&self) -> Result<Option<(String, u32)>, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT active_profile_id, active_revision FROM planning_overlay_state \
                 WHERE singleton_id = 1 AND active_profile_id IS NOT NULL \
                   AND active_revision IS NOT NULL",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
            )
            .optional()
            .map_err(Into::into)
    }

    fn overlay_profile_summary(
        &self,
        profile_id: &str,
    ) -> Result<OverlayProfileSummary, ObservatoryError> {
        let connection = self.connect()?;
        let active = self.active_overlay_identity()?;
        connection
            .query_row(
                "SELECT profile.display_name, profile.active_revision, \
                        MAX(revision.revision), COUNT(*), latest.semantic_version, latest.content_hash \
                 FROM planning_overlay_profiles profile \
                 JOIN planning_overlay_revisions revision ON revision.profile_id = profile.profile_id \
                 JOIN planning_overlay_revisions latest ON latest.profile_id = profile.profile_id \
                      AND latest.revision = (SELECT MAX(r.revision) FROM planning_overlay_revisions r \
                                             WHERE r.profile_id = profile.profile_id) \
                 WHERE profile.profile_id = ?1 AND profile.removed_at_ms IS NULL \
                 GROUP BY profile.display_name, profile.active_revision, \
                          latest.semantic_version, latest.content_hash",
                [profile_id],
                |row| {
                    Ok(OverlayProfileSummary {
                        profile_id: profile_id.to_owned(),
                        display_name: row.get(0)?,
                        active_revision: row.get(1)?,
                        latest_revision: row.get(2)?,
                        revision_count: row.get(3)?,
                        semantic_version: row.get(4)?,
                        content_hash: row.get(5)?,
                        conflict_count: 0,
                        active: active.as_ref().is_some_and(|value| value.0 == profile_id),
                    })
                },
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownPlanningOverlay)
    }
}

fn enqueue_overlay_state_job(
    transaction: &rusqlite::Transaction<'_>,
    profile_id: &str,
    revision: Option<u32>,
    requested_at_ms: i64,
) -> Result<(), ObservatoryError> {
    let source_identity = revision
        .map(|revision| format!("{profile_id}:{revision}"))
        .unwrap_or_else(|| "none".to_owned());
    let projection_id = content_derived_projection_id(
        "overlay",
        &format!("{source_identity}:{requested_at_ms}"),
    );
    enqueue_projection_job(
        transaction,
        &projection_id,
        "overlay_state",
        &source_identity,
        requested_at_ms,
    )
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn document(version: &str, name: &str) -> PlanningOverlayDocument {
        PlanningOverlayDocument::parse(
            format!(
                r#"{{
                  "schema_version": 1,
                  "id": "org.example.planning-profile",
                  "version": "{version}",
                  "name": "{name}",
                  "author": "Planner",
                  "default_locale": "en-AU",
                  "description": "A local planning assumption",
                  "operations": [],
                  "supplements": []
                }}"#
            )
            .as_bytes(),
        )
        .expect("valid document")
    }

    #[test]
    fn overlay_lifecycle_keeps_immutable_revisions_and_separate_activation() {
        let directory = tempdir().expect("temporary directory");
        let storage = ObservatoryStorage::initialise(directory.path().join("overlay.sqlite3"))
            .expect("storage");
        let first = storage
            .install_overlay(&document("1.0.0", "Planning profile"))
            .expect("first revision");
        assert_eq!(first.latest_revision, 1);
        assert!(!first.active);
        storage
            .activate_overlay(&first.profile_id, Some(1))
            .expect("activate");
        let second_document = document("1.1.0", "Planning profile revised");
        let second = storage
            .install_overlay(&second_document)
            .expect("second revision");
        assert_eq!(second.latest_revision, 2);
        assert_eq!(second.revision_count, 2);
        assert_eq!(
            storage
                .overlay_document(&second.profile_id, 2)
                .expect("export")
                .canonical_json()
                .expect("canonical JSON"),
            second_document.canonical_json().expect("input JSON")
        );
        storage
            .activate_overlay(&second.profile_id, Some(2))
            .expect("activate latest");
        let rolled_back = storage
            .rollback_overlay(&second.profile_id)
            .expect("rollback");
        assert_eq!(rolled_back.active_revision, Some(1));
        storage.deactivate_overlay().expect("deactivate");
        assert!(storage.active_overlay_summary().expect("state").is_none());
        storage.remove_overlay(&second.profile_id).expect("remove");
        assert!(storage.list_overlay_profiles().expect("profiles").is_empty());
    }
}
