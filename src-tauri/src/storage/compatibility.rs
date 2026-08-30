use rusqlite::params;

use super::{ObservatoryStorage, now_ms};
use crate::compatibility_profile::{
    CompatibilityProfileSource, PARSER_ENGINE_VERSION, ResolvedCompatibilityProfile,
};
use crate::error::ObservatoryError;
use crate::model::{CompatibilityStatus, CompatibilityValidationState};

impl ObservatoryStorage {
    pub fn record_compatibility_runtime(
        &self,
        profile: &ResolvedCompatibilityProfile,
        status: &CompatibilityStatus,
    ) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        connection.execute(
            "INSERT OR IGNORE INTO compatibility_profile_revisions(\
                 profile_id, semantic_version, content_hash, resolved_hash, base_profile_hash,\
                 profile_source, mapping_classification, parser_engine_version, document_json,\
                 validated_at_ms\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                profile.id(),
                profile.version(),
                profile.content_hash(),
                profile.resolved_hash(),
                profile
                    .base()
                    .map(|reference| reference.content_hash.as_str()),
                match profile.source() {
                    CompatibilityProfileSource::ReviewedBuiltin => "reviewed_builtin",
                    CompatibilityProfileSource::LocalOverride => "local_override",
                },
                profile.source().evidence_classification(),
                PARSER_ENGINE_VERSION,
                profile.canonical_document_json()?,
                now_ms(),
            ],
        )?;
        connection.execute(
            "UPDATE compatibility_runtime_state SET active_resolved_hash = ?1,\
                 local_file_exists = ?2, local_validation = ?3, last_validation_error = ?4,\
                 last_validated_at_ms = ?5 WHERE singleton_id = 1",
            params![
                profile.resolved_hash(),
                i64::from(status.local_file_exists),
                match status.local_validation {
                    CompatibilityValidationState::Missing => "missing",
                    CompatibilityValidationState::Valid => "valid",
                    CompatibilityValidationState::Invalid => "invalid",
                },
                status.last_validation_error,
                status.last_validated_at_ms,
            ],
        )?;
        Ok(())
    }
}
