use std::path::{Path, PathBuf};

use rusqlite::params;

use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;

#[derive(Clone, Debug)]
pub struct StoredResearchSetup {
    pub tesmio_checkout_path: Option<PathBuf>,
    pub tesmio_source_origin: Option<String>,
    pub accepted_notice_revision: u32,
    pub last_probe_hash: Option<String>,
    pub last_built_at_ms: Option<i64>,
}

impl ObservatoryStorage {
    pub fn research_setup(&self) -> Result<StoredResearchSetup, ObservatoryError> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT tesmio_checkout_path, tesmio_source_origin, accepted_notice_revision, \
                        last_probe_hash, last_built_at_ms \
                 FROM research_setup_state WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(StoredResearchSetup {
                        tesmio_checkout_path: row.get::<_, Option<String>>(0)?.map(PathBuf::from),
                        tesmio_source_origin: row.get(1)?,
                        accepted_notice_revision: row.get(2)?,
                        last_probe_hash: row.get(3)?,
                        last_built_at_ms: row.get(4)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn set_research_notice_revision(&self, revision: u32) -> Result<(), ObservatoryError> {
        if revision > 1_000_000 {
            return Err(ObservatoryError::InvalidResearchSetup);
        }
        let connection = self.connect()?;
        connection.execute(
            "UPDATE research_setup_state SET accepted_notice_revision = ?1, \
                    updated_at_ms = ?2 WHERE singleton_id = 1",
            params![revision, now_ms()],
        )?;
        Ok(())
    }

    pub fn set_research_tesmio_checkout(
        &self,
        path: &Path,
        source_origin: &str,
    ) -> Result<(), ObservatoryError> {
        if !matches!(source_origin, "manual_checkout" | "observatory_downloaded") {
            return Err(ObservatoryError::InvalidResearchSetup);
        }
        let connection = self.connect()?;
        connection.execute(
            "UPDATE research_setup_state SET tesmio_checkout_path = ?1, \
                    tesmio_source_origin = ?2, updated_at_ms = ?3 WHERE singleton_id = 1",
            params![path.to_string_lossy(), source_origin, now_ms()],
        )?;
        Ok(())
    }

    pub fn record_research_probe_build(&self, content_hash: &str) -> Result<(), ObservatoryError> {
        if content_hash.len() != 64
            || !content_hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ObservatoryError::InvalidResearchSetup);
        }
        let timestamp = now_ms();
        let connection = self.connect()?;
        connection.execute(
            "UPDATE research_setup_state SET last_probe_hash = ?1, \
                    last_built_at_ms = ?2, updated_at_ms = ?2 \
             WHERE singleton_id = 1",
            params![content_hash, timestamp],
        )?;
        Ok(())
    }
}
