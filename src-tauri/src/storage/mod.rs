mod analysis_context;
mod analysis_packs;
mod archive;
mod attention;
mod broadcast;
mod comparison;
mod compatibility;
mod connection;
mod history;
mod language_packs;
mod markets;
mod migrations;
mod observations;
mod planning_overlays;
mod population;
mod recorder;
mod related_navigation;
mod republic_plans;
mod research_setup;
mod settings;
mod snapshots;
mod themes;
mod warehouse_jobs;

pub(crate) use broadcast::BROADCAST_STATUS_STORAGE_CONTRACT_VERSION;
pub(crate) use markets::MARKET_STORAGE_CONTRACT_VERSION;
pub(crate) use research_setup::StoredResearchSetup;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::ObservatoryError;

#[derive(Debug)]
pub struct ObservatoryStorage {
    database_path: PathBuf,
}

impl ObservatoryStorage {
    pub fn initialise(database_path: PathBuf) -> Result<Self, ObservatoryError> {
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(|_| ObservatoryError::StorageUnavailable)?;
        }
        let storage = Self { database_path };
        let mut connection = storage.connect()?;
        migrations::apply(&mut connection)?;
        archive::backfill_missing_history_signatures(&mut connection)?;
        history::backfill_compacted_histories(&mut connection)?;
        archive::reconcile_unassigned_observations(&mut connection)?;
        recorder::recover_interrupted_candidates(&connection)?;
        warehouse_jobs::recover_interrupted_projection_jobs(&connection)?;
        Ok(storage)
    }

    pub fn verify_and_repair_known_contracts(&self) -> Result<(), ObservatoryError> {
        let mut connection = self.connect()?;
        migrations::apply(&mut connection)?;
        warehouse_jobs::recover_interrupted_projection_jobs(&connection)?;
        Ok(())
    }
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

pub(crate) fn to_sql_integer(value: u64) -> Result<i64, ObservatoryError> {
    i64::try_from(value).map_err(|_| ObservatoryError::StorageUnavailable)
}

pub(crate) fn from_sql_integer(value: i64) -> Result<u64, rusqlite::Error> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
