use rusqlite::{OptionalExtension, params};

use super::{ObservatoryStorage, now_ms};
use crate::analysis_pack::{AnalysisPackDocument, AnalysisPackSummary};
use crate::error::ObservatoryError;

#[derive(Clone, Debug)]
pub(crate) struct InstalledAnalysisPackRevision {
    pub pack_id: String,
    pub revision: u32,
    pub content_hash: String,
    pub document_json: String,
}

impl ObservatoryStorage {
    pub fn install_analysis_pack(
        &self,
        document: &AnalysisPackDocument,
    ) -> Result<AnalysisPackSummary, ObservatoryError> {
        document.validate()?;
        let content_hash = document.content_hash()?;
        let document_json = document.canonical_json()?;
        let installed_at = now_ms();
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO analysis_pack_profiles(\
                 pack_id, display_name, active_revision, removed_at_ms, created_at_ms, updated_at_ms\
             ) VALUES(?1, ?2, NULL, NULL, ?3, ?3) \
             ON CONFLICT(pack_id) DO UPDATE SET \
                 display_name = excluded.display_name, removed_at_ms = NULL, \
                 updated_at_ms = excluded.updated_at_ms",
            params![document.id, document.name, installed_at],
        )?;
        let existing = transaction
            .query_row(
                "SELECT revision FROM analysis_pack_revisions \
                 WHERE pack_id = ?1 AND content_hash = ?2",
                params![document.id, content_hash],
                |row| row.get::<_, u32>(0),
            )
            .optional()?;
        if existing.is_none() {
            let next = transaction.query_row(
                "SELECT COALESCE(MAX(revision), 0) + 1 FROM analysis_pack_revisions \
                 WHERE pack_id = ?1",
                [&document.id],
                |row| row.get::<_, u32>(0),
            )?;
            transaction.execute(
                "INSERT INTO analysis_pack_revisions(\
                     pack_id, revision, content_hash, semantic_version, host_api_version, author, \
                     default_locale, description, derived_metric_count, chart_count, document_json, \
                     installed_at_ms\
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    document.id,
                    next,
                    content_hash,
                    document.version,
                    document.host_api_version,
                    document.author,
                    document.default_locale(),
                    document.description,
                    u32::try_from(document.derived_metrics.len()).unwrap_or(u32::MAX),
                    u32::try_from(document.charts.len()).unwrap_or(u32::MAX),
                    document_json,
                    installed_at,
                ],
            )?;
        }
        transaction.commit()?;
        self.analysis_pack_summary(&document.id)
    }

    pub fn enable_analysis_pack(
        &self,
        pack_id: &str,
        revision: Option<u32>,
    ) -> Result<AnalysisPackSummary, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let selected_revision = if let Some(revision) = revision {
            revision
        } else {
            transaction
                .query_row(
                    "SELECT MAX(revision) FROM analysis_pack_revisions WHERE pack_id = ?1",
                    [pack_id],
                    |row| row.get::<_, Option<u32>>(0),
                )?
                .ok_or(ObservatoryError::UnknownAnalysisPack)?
        };
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM analysis_pack_revisions \
             WHERE pack_id = ?1 AND revision = ?2)",
            params![pack_id, selected_revision],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(ObservatoryError::UnknownAnalysisPack);
        }
        let updated = transaction.execute(
            "UPDATE analysis_pack_profiles SET active_revision = ?1, updated_at_ms = ?2 \
             WHERE pack_id = ?3 AND removed_at_ms IS NULL",
            params![selected_revision, now_ms(), pack_id],
        )?;
        if updated == 0 {
            return Err(ObservatoryError::UnknownAnalysisPack);
        }
        transaction.commit()?;
        self.analysis_pack_summary(pack_id)
    }

    pub fn disable_analysis_pack(
        &self,
        pack_id: &str,
    ) -> Result<AnalysisPackSummary, ObservatoryError> {
        let updated = self.connect()?.execute(
            "UPDATE analysis_pack_profiles SET active_revision = NULL, updated_at_ms = ?1 \
             WHERE pack_id = ?2 AND removed_at_ms IS NULL",
            params![now_ms(), pack_id],
        )?;
        if updated == 0 {
            return Err(ObservatoryError::UnknownAnalysisPack);
        }
        self.analysis_pack_summary(pack_id)
    }

    pub fn rollback_analysis_pack(
        &self,
        pack_id: &str,
    ) -> Result<AnalysisPackSummary, ObservatoryError> {
        let summary = self.analysis_pack_summary(pack_id)?;
        let current = summary
            .active_revision
            .ok_or(ObservatoryError::UnknownAnalysisPack)?;
        let previous = self
            .connect()?
            .query_row(
                "SELECT MAX(revision) FROM analysis_pack_revisions \
                 WHERE pack_id = ?1 AND revision < ?2",
                params![pack_id, current],
                |row| row.get::<_, Option<u32>>(0),
            )?
            .ok_or(ObservatoryError::UnknownAnalysisPack)?;
        self.enable_analysis_pack(pack_id, Some(previous))
    }

    pub fn remove_analysis_pack(&self, pack_id: &str) -> Result<(), ObservatoryError> {
        let updated = self.connect()?.execute(
            "UPDATE analysis_pack_profiles SET active_revision = NULL, removed_at_ms = ?1, \
             updated_at_ms = ?1 WHERE pack_id = ?2 AND removed_at_ms IS NULL",
            params![now_ms(), pack_id],
        )?;
        if updated == 0 {
            return Err(ObservatoryError::UnknownAnalysisPack);
        }
        Ok(())
    }

    pub fn list_analysis_packs(&self) -> Result<Vec<AnalysisPackSummary>, ObservatoryError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT pack_id FROM analysis_pack_profiles \
             WHERE removed_at_ms IS NULL ORDER BY display_name, pack_id",
        )?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|pack_id| self.analysis_pack_summary(pack_id))
            .collect()
    }

    pub fn analysis_pack_document(
        &self,
        pack_id: &str,
        revision: u32,
    ) -> Result<AnalysisPackDocument, ObservatoryError> {
        let json = self
            .connect()?
            .query_row(
                "SELECT document_json FROM analysis_pack_revisions \
                 WHERE pack_id = ?1 AND revision = ?2",
                params![pack_id, revision],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownAnalysisPack)?;
        AnalysisPackDocument::parse(json.as_bytes())
    }

    pub(crate) fn enabled_analysis_pack_revisions(
        &self,
    ) -> Result<Vec<InstalledAnalysisPackRevision>, ObservatoryError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT profile.pack_id, profile.active_revision, revision.content_hash, \
                    revision.document_json \
             FROM analysis_pack_profiles profile \
             JOIN analysis_pack_revisions revision ON revision.pack_id = profile.pack_id \
                  AND revision.revision = profile.active_revision \
             WHERE profile.removed_at_ms IS NULL AND profile.active_revision IS NOT NULL \
             ORDER BY profile.pack_id",
        )?;
        statement
            .query_map([], |row| {
                Ok(InstalledAnalysisPackRevision {
                    pack_id: row.get(0)?,
                    revision: row.get(1)?,
                    content_hash: row.get(2)?,
                    document_json: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn analysis_pack_summary(
        &self,
        pack_id: &str,
    ) -> Result<AnalysisPackSummary, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT profile.display_name, profile.active_revision, MAX(revision.revision), \
                        COUNT(*), latest.semantic_version, latest.host_api_version, \
                        latest.content_hash, latest.author, latest.default_locale, \
                        latest.description, latest.derived_metric_count, latest.chart_count \
                 FROM analysis_pack_profiles profile \
                 JOIN analysis_pack_revisions revision ON revision.pack_id = profile.pack_id \
                 JOIN analysis_pack_revisions latest ON latest.pack_id = profile.pack_id \
                      AND latest.revision = (SELECT MAX(r.revision) FROM analysis_pack_revisions r \
                                             WHERE r.pack_id = profile.pack_id) \
                 WHERE profile.pack_id = ?1 AND profile.removed_at_ms IS NULL \
                 GROUP BY profile.display_name, profile.active_revision, latest.semantic_version, \
                          latest.host_api_version, latest.content_hash, latest.author, \
                          latest.default_locale, latest.description, latest.derived_metric_count, \
                          latest.chart_count",
                [pack_id],
                |row| {
                    let active_revision = row.get::<_, Option<u32>>(1)?;
                    Ok(AnalysisPackSummary {
                        pack_id: pack_id.to_owned(),
                        display_name: row.get(0)?,
                        active_revision,
                        latest_revision: row.get(2)?,
                        revision_count: row.get(3)?,
                        semantic_version: row.get(4)?,
                        host_api_version: row.get(5)?,
                        content_hash: row.get(6)?,
                        author: row.get(7)?,
                        default_locale: row.get(8)?,
                        description: row.get(9)?,
                        derived_metric_count: row.get(10)?,
                        chart_count: row.get(11)?,
                        enabled: active_revision.is_some(),
                        validation_state: "valid".to_owned(),
                    })
                },
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownAnalysisPack)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn document() -> AnalysisPackDocument {
        AnalysisPackDocument::parse(include_bytes!(
            "../../../examples/analysis-packs/receiver-adoption-laboratory.roanalysis.json"
        ))
        .expect("example")
    }

    #[test]
    fn lifecycle_separates_import_enable_disable_and_removal() {
        let directory = tempdir().expect("temporary directory");
        let storage = ObservatoryStorage::initialise(directory.path().join("packs.sqlite3"))
            .expect("storage");
        let imported = storage.install_analysis_pack(&document()).expect("import");
        assert_eq!(imported.latest_revision, 1);
        assert!(!imported.enabled);

        let enabled = storage
            .enable_analysis_pack(&imported.pack_id, None)
            .expect("enable");
        assert!(enabled.enabled);
        assert_eq!(
            storage
                .enabled_analysis_pack_revisions()
                .expect("active")
                .len(),
            1
        );

        let disabled = storage
            .disable_analysis_pack(&imported.pack_id)
            .expect("disable");
        assert!(!disabled.enabled);
        storage
            .remove_analysis_pack(&imported.pack_id)
            .expect("remove");
        assert!(storage.list_analysis_packs().expect("list").is_empty());
    }

    #[test]
    fn duplicate_import_is_idempotent() {
        let directory = tempdir().expect("temporary directory");
        let storage = ObservatoryStorage::initialise(directory.path().join("packs.sqlite3"))
            .expect("storage");
        storage.install_analysis_pack(&document()).expect("first");
        let duplicate = storage
            .install_analysis_pack(&document())
            .expect("duplicate");
        assert_eq!(duplicate.revision_count, 1);
    }
}
