use rusqlite::params;

use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;

impl ObservatoryStorage {
    pub fn attention_cue_dismissed(
        &self,
        cue_id: &str,
        content_revision: u32,
    ) -> Result<bool, ObservatoryError> {
        validate_cue_identity(cue_id, content_revision)?;
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM attention_cue_dismissals \
                 WHERE cue_id = ?1 AND content_revision = ?2)",
                params![cue_id, content_revision],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn dismiss_attention_cue(
        &self,
        cue_id: &str,
        content_revision: u32,
    ) -> Result<(), ObservatoryError> {
        validate_cue_identity(cue_id, content_revision)?;
        let connection = self.connect()?;
        connection.execute(
            "INSERT OR IGNORE INTO attention_cue_dismissals(\
                 cue_id, content_revision, dismissed_at_ms\
             ) VALUES(?1, ?2, ?3)",
            params![cue_id, content_revision, now_ms()],
        )?;
        Ok(())
    }

    pub fn replay_attention_cue(
        &self,
        cue_id: &str,
        content_revision: u32,
    ) -> Result<(), ObservatoryError> {
        validate_cue_identity(cue_id, content_revision)?;
        let connection = self.connect()?;
        connection.execute(
            "DELETE FROM attention_cue_dismissals \
             WHERE cue_id = ?1 AND content_revision = ?2",
            params![cue_id, content_revision],
        )?;
        Ok(())
    }
}

fn validate_cue_identity(cue_id: &str, content_revision: u32) -> Result<(), ObservatoryError> {
    let valid_id = (3..=96).contains(&cue_id.len())
        && cue_id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte)
        })
        && cue_id
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase);
    if !valid_id || !(1..=1_000_000).contains(&content_revision) {
        return Err(ObservatoryError::InvalidAttentionCue);
    }
    Ok(())
}
