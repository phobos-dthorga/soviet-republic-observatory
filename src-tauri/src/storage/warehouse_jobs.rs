use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

use super::{ObservatoryStorage, now_ms};
use crate::error::ObservatoryError;

#[derive(Clone, Debug)]
pub(crate) struct StoredProjectionJob {
    pub projection_id: String,
    pub projection_kind: String,
    pub source_identity: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProjectionQueueStatus {
    pub pending_jobs: u32,
    pub failed_jobs: u32,
    pub oldest_unresolved_at_ms: Option<i64>,
}

pub(crate) fn content_derived_projection_id(prefix: &str, content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{prefix}:{digest}")
}

pub(crate) fn recover_interrupted_projection_jobs(
    connection: &Connection,
) -> Result<(), ObservatoryError> {
    connection.execute(
        "UPDATE warehouse_projection_jobs \
         SET status = 'pending', error_code = 'interrupted' \
         WHERE status = 'running'",
        [],
    )?;
    Ok(())
}

pub(crate) fn enqueue_projection_job(
    transaction: &Transaction<'_>,
    projection_id: &str,
    projection_kind: &str,
    source_identity: &str,
    requested_at_ms: i64,
) -> Result<(), ObservatoryError> {
    transaction.execute(
        "INSERT OR IGNORE INTO warehouse_projection_jobs(\
             projection_id, projection_kind, source_identity, status, requested_at_ms\
         ) VALUES(?1, ?2, ?3, 'pending', ?4)",
        params![
            projection_id,
            projection_kind,
            source_identity,
            requested_at_ms
        ],
    )?;
    Ok(())
}

impl ObservatoryStorage {
    pub(crate) fn claim_projection_job(
        &self,
    ) -> Result<Option<StoredProjectionJob>, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let job = transaction
            .query_row(
                "SELECT projection_id, projection_kind, source_identity \
                 FROM warehouse_projection_jobs WHERE status = 'pending' \
                 ORDER BY CASE projection_kind \
                            WHEN 'rebuild' THEN 0 WHEN 'observation' THEN 1 \
                            WHEN 'overlay_state' THEN 2 ELSE 3 END, \
                          requested_at_ms, projection_id LIMIT 1",
                [],
                |row| {
                    Ok(StoredProjectionJob {
                        projection_id: row.get(0)?,
                        projection_kind: row.get(1)?,
                        source_identity: row.get(2)?,
                    })
                },
            )
            .optional()?;
        if let Some(job) = &job {
            let claimed = transaction.execute(
                "UPDATE warehouse_projection_jobs \
                 SET status = 'running', started_at_ms = ?1, \
                     attempt_count = attempt_count + 1, error_code = NULL \
                 WHERE projection_id = ?2 AND status = 'pending'",
                params![now_ms(), job.projection_id],
            )?;
            if claimed == 0 {
                transaction.commit()?;
                return Ok(None);
            }
        }
        transaction.commit()?;
        Ok(job)
    }

    pub(crate) fn complete_projection_job(
        &self,
        projection_id: &str,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "UPDATE warehouse_projection_jobs \
             SET status = 'applied', applied_at_ms = ?1, error_code = NULL \
             WHERE projection_id = ?2",
            params![now_ms(), projection_id],
        )?;
        Ok(())
    }

    pub(crate) fn fail_projection_job(
        &self,
        projection_id: &str,
        error_code: &str,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "UPDATE warehouse_projection_jobs \
             SET status = 'failed', error_code = ?1 WHERE projection_id = ?2",
            params![error_code, projection_id],
        )?;
        Ok(())
    }

    pub(crate) fn projection_queue_status(
        &self,
    ) -> Result<ProjectionQueueStatus, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT \
                     SUM(CASE WHEN status IN ('pending', 'running') THEN 1 ELSE 0 END), \
                     SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), \
                     MIN(CASE WHEN status <> 'applied' THEN requested_at_ms END) \
                 FROM warehouse_projection_jobs",
                [],
                |row| {
                    Ok(ProjectionQueueStatus {
                        pending_jobs: row.get::<_, Option<u32>>(0)?.unwrap_or(0),
                        failed_jobs: row.get::<_, Option<u32>>(1)?.unwrap_or(0),
                        oldest_unresolved_at_ms: row.get(2)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn enqueue_warehouse_rebuild(&self) -> Result<(), ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let requested_at = now_ms();
        transaction.execute(
            "UPDATE warehouse_projection_jobs SET status = 'pending', error_code = NULL, \
                 applied_at_ms = NULL \
             WHERE projection_kind IN ('observation', 'market_observation', 'overlay_state', 'branch_membership')",
            [],
        )?;
        enqueue_projection_job(
            &transaction,
            &content_derived_projection_id("rebuild", &format!("all_observations:{requested_at}")),
            "rebuild",
            "all_observations",
            requested_at,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn warehouse_rebuild_running(&self) -> Result<bool, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM warehouse_projection_jobs \
                 WHERE projection_kind = 'rebuild' AND status IN ('pending', 'running'))",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn note_catalogue_refresh_request(
        &self,
        requested_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "UPDATE catalogue_runtime_state SET last_refresh_requested_ms = ?1, \
             last_refresh_error_code = NULL WHERE singleton_id = 1",
            [requested_at_ms],
        )?;
        Ok(())
    }

    pub(crate) fn note_catalogue_filesystem_event(
        &self,
        event_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "UPDATE catalogue_runtime_state SET last_filesystem_event_ms = ?1 \
             WHERE singleton_id = 1",
            [event_at_ms],
        )?;
        Ok(())
    }

    pub(crate) fn note_catalogue_refresh_failure(
        &self,
        error_code: &str,
    ) -> Result<(), ObservatoryError> {
        self.connect()?.execute(
            "UPDATE catalogue_runtime_state SET last_refresh_error_code = ?1 \
             WHERE singleton_id = 1",
            [error_code],
        )?;
        Ok(())
    }

    pub(crate) fn catalogue_runtime_state(
        &self,
    ) -> Result<(Option<i64>, Option<String>), ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT last_filesystem_event_ms, last_refresh_error_code \
                 FROM catalogue_runtime_state WHERE singleton_id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(Into::into)
    }

    pub(crate) fn enqueue_current_overlay_projection(
        &self,
        source_identity: &str,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let requested_at = now_ms();
        enqueue_projection_job(
            &transaction,
            &content_derived_projection_id("overlay-refresh", source_identity),
            "overlay_state",
            source_identity,
            requested_at,
        )?;
        transaction.commit()?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn projection_job_status(
        &self,
        projection_id: &str,
    ) -> Result<Option<String>, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT status FROM warehouse_projection_jobs WHERE projection_id = ?1",
                [projection_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
}
