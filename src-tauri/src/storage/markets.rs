use std::collections::HashSet;
use std::fmt::Write;

use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

use super::{ObservatoryStorage, from_sql_integer, now_ms};
use crate::error::ObservatoryError;
use crate::model::{
    MarketFactRows, MarketIndexCandidate, MarketIndexingProgress, ParsedMarketData, SaveInspection,
};

impl ObservatoryStorage {
    pub(crate) fn market_coverage_exists(
        &self,
        interpretation_id: &str,
    ) -> Result<bool, ObservatoryError> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT EXISTS(\
                     SELECT 1 FROM market_observation_coverage coverage\
                     JOIN observation_sources source ON source.payload_hash = coverage.payload_hash\
                     WHERE source.interpretation_id = ?1\
                 )",
                [interpretation_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn market_index_candidates(
        &self,
        source_directory_identity: &str,
    ) -> Result<Vec<MarketIndexCandidate>, ObservatoryError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT os.interpretation_id, ao.source_file_name, ao.source_file_size,\
                    ao.source_modified_ms, ao.source_directory_identity, os.raw_payload_hash\
             FROM archive_observations ao\
             JOIN observation_sources os ON os.payload_hash = ao.payload_hash\
             WHERE ao.source_directory_identity = ?1\
             ORDER BY os.history_records DESC, ao.observed_at_ms DESC",
        )?;
        let rows = statement.query_map([source_directory_identity], |row| {
            Ok(MarketIndexCandidate {
                interpretation_id: row.get(0)?,
                source_file_name: row.get(1)?,
                source_file_size: from_sql_integer(row.get(2)?)?,
                source_modified_ms: row.get(3)?,
                source_directory_identity: row.get(4)?,
                raw_payload_hash: row.get(5)?,
            })
        })?;
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        for candidate in rows {
            let candidate = candidate?;
            let identity = (
                candidate.source_file_name.clone(),
                candidate.source_file_size,
                candidate.source_modified_ms,
                candidate.raw_payload_hash.clone(),
            );
            if seen.insert(identity) {
                candidates.push(candidate);
            }
        }
        Ok(candidates)
    }

    pub(crate) fn start_market_index_job(
        &self,
        job_id: &str,
        candidates: &[MarketIndexCandidate],
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO market_indexing_jobs(job_id, state, started_at_ms, total_archives)\
             VALUES(?1, 'running', ?2, ?3)",
            params![
                job_id,
                now_ms(),
                candidates.len().min(u32::MAX as usize) as u32
            ],
        )?;
        for candidate in candidates {
            transaction.execute(
                "INSERT OR IGNORE INTO market_indexing_items(job_id, payload_hash, state)\
                 VALUES(?1, ?2, 'pending')",
                params![job_id, candidate.interpretation_id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn update_market_index_item(
        &self,
        job_id: &str,
        interpretation_id: &str,
        state: &str,
        records_processed: u32,
        rows_processed: u32,
        error_code: Option<&str>,
    ) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        connection.execute(
            "UPDATE market_indexing_items SET state = ?1, records_processed = ?2,\
                    rows_processed = ?3, error_code = ?4\
             WHERE job_id = ?5 AND payload_hash = ?6",
            params![
                state,
                records_processed,
                rows_processed,
                error_code,
                job_id,
                interpretation_id,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn finish_market_index_job(
        &self,
        progress: &MarketIndexingProgress,
    ) -> Result<(), ObservatoryError> {
        let Some(job_id) = progress.job_id.as_deref() else {
            return Err(ObservatoryError::StorageUnavailable);
        };
        let connection = self.connect()?;
        connection.execute(
            "UPDATE market_indexing_jobs SET state = ?1, completed_at_ms = ?2,\
                    completed_archives = ?3, missing_archives = ?4, changed_archives = ?5,\
                    failed_archives = ?6, duplicate_archives = ?7, last_error_code = ?8\
             WHERE job_id = ?9",
            params![
                if progress.error_code.is_some() {
                    "failed"
                } else {
                    "complete"
                },
                now_ms(),
                progress.completed_archives,
                progress.missing_archives,
                progress.changed_archives,
                progress.failed_archives,
                progress.duplicate_archives,
                progress.error_code,
                job_id,
            ],
        )?;
        Ok(())
    }
}

pub(crate) fn persist_market_data(
    connection: &Connection,
    storage_key: &str,
    inspection: &SaveInspection,
) -> Result<(), ObservatoryError> {
    let warnings_json = serde_json::to_string(&inspection.market.warnings)
        .map_err(|_| ObservatoryError::StorageUnavailable)?;
    connection.execute(
        "INSERT OR REPLACE INTO market_observation_coverage(\
             payload_hash, coverage_status, history_records, snapshot_scopes, row_count, warnings_json\
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            storage_key,
            inspection.market.coverage_status().as_str(),
            inspection.market.records.len().min(u32::MAX as usize) as u32,
            inspection.market.snapshots.len().min(u32::MAX as usize) as u32,
            inspection.market.row_count,
            warnings_json,
        ],
    )?;

    for (ordinal, record) in inspection.market.records.iter().enumerate() {
        let record_hash = record_hash(record, &inspection.market)?;
        connection.execute(
            "INSERT OR IGNORE INTO market_records(record_hash, record_id, year, day, game_day) \
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                record_hash,
                record.record_id,
                record.year,
                record.day,
                record.game_day
            ],
        )?;
        persist_record_rows(connection, &record_hash, &record.rows, &inspection.market)?;
        connection.execute(
            "INSERT OR IGNORE INTO market_observation_records(payload_hash, ordinal, record_hash) \
             VALUES(?1, ?2, ?3)",
            params![storage_key, ordinal as u32, record_hash],
        )?;
    }

    for snapshot in &inspection.market.snapshots {
        let scope_kind = snapshot.scope_kind.as_str();
        connection.execute(
            "INSERT OR IGNORE INTO market_snapshot_scopes(payload_hash, scope_kind, scope_id) \
             VALUES(?1, ?2, ?3)",
            params![storage_key, scope_kind, snapshot.scope_id],
        )?;
        persist_snapshot_rows(
            connection,
            storage_key,
            scope_kind,
            &snapshot.scope_id,
            &snapshot.rows,
            &inspection.market,
        )?;
    }

    connection.execute(
        "INSERT OR IGNORE INTO market_interpretation_variants(\
             raw_payload_hash, interpretation_id, profile_id, profile_version, resolved_profile_hash, indexed_at_ms\
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            inspection.payload_hash,
            inspection.interpretation_id,
            inspection.compatibility.profile_id,
            inspection.compatibility.profile_version,
            inspection.compatibility.resolved_profile_hash,
            now_ms(),
        ],
    )?;
    Ok(())
}

fn persist_record_rows(
    connection: &Connection,
    record_hash: &str,
    rows: &MarketFactRows,
    market: &ParsedMarketData,
) -> Result<(), ObservatoryError> {
    for row in &rows.prices {
        connection.execute(
            "INSERT OR IGNORE INTO market_price_facts(\
                 record_hash, currency, price_side, resource_token, value_real, modifier_real,\
                 source_field, source_line, mapping_id\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record_hash,
                row.currency.as_str(),
                row.side.as_str(),
                resource(market, row.resource_index)?,
                row.value,
                row.modifier,
                source_field(market, row.source_field_index)?,
                row.source_line,
                format!(
                    "market.price.{}.{}",
                    row.side.as_str(),
                    row.currency.as_str()
                ),
            ],
        )?;
    }
    for row in &rows.trades {
        connection.execute(
            "INSERT OR IGNORE INTO market_trade_facts(\
                 record_hash, currency, direction, channel, resource_token, quantity_real,\
                 account_value_real, source_field, source_line, mapping_id\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                record_hash,
                row.currency.as_str(),
                row.direction.as_str(),
                row.channel.as_str(),
                resource(market, row.resource_index)?,
                row.quantity,
                row.account_value,
                source_field(market, row.source_field_index)?,
                row.source_line,
                format!(
                    "market.trade.{}.{}.{}",
                    row.direction.as_str(),
                    row.channel.as_str(),
                    row.currency.as_str()
                ),
            ],
        )?;
    }
    for row in &rows.scalars {
        connection.execute(
            "INSERT OR IGNORE INTO market_scalar_facts(\
                 record_hash, fact_id, currency, category, value_real, source_field, source_line, mapping_id\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record_hash,
                row.fact_id,
                row.currency.map(|currency| currency.as_str()),
                row.category,
                row.value,
                source_field(market, row.source_field_index)?,
                row.source_line,
                row.fact_id,
            ],
        )?;
    }
    Ok(())
}

fn persist_snapshot_rows(
    connection: &Connection,
    storage_key: &str,
    scope_kind: &str,
    scope_id: &str,
    rows: &MarketFactRows,
    market: &ParsedMarketData,
) -> Result<(), ObservatoryError> {
    for row in &rows.prices {
        connection.execute(
            "INSERT OR IGNORE INTO market_snapshot_price_facts(\
                 payload_hash, scope_kind, scope_id, currency, price_side, resource_token,\
                 value_real, modifier_real, source_field, source_line, mapping_id\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                storage_key,
                scope_kind,
                scope_id,
                row.currency.as_str(),
                row.side.as_str(),
                resource(market, row.resource_index)?,
                row.value,
                row.modifier,
                source_field(market, row.source_field_index)?,
                row.source_line,
                format!(
                    "market.price.{}.{}",
                    row.side.as_str(),
                    row.currency.as_str()
                ),
            ],
        )?;
    }
    for row in &rows.trades {
        connection.execute(
            "INSERT OR IGNORE INTO market_snapshot_trade_facts(\
                 payload_hash, scope_kind, scope_id, currency, direction, channel, resource_token,\
                 quantity_real, account_value_real, source_field, source_line, mapping_id\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                storage_key,
                scope_kind,
                scope_id,
                row.currency.as_str(),
                row.direction.as_str(),
                row.channel.as_str(),
                resource(market, row.resource_index)?,
                row.quantity,
                row.account_value,
                source_field(market, row.source_field_index)?,
                row.source_line,
                format!(
                    "market.trade.{}.{}.{}",
                    row.direction.as_str(),
                    row.channel.as_str(),
                    row.currency.as_str()
                ),
            ],
        )?;
    }
    for row in &rows.scalars {
        connection.execute(
            "INSERT OR IGNORE INTO market_snapshot_scalar_facts(\
                 payload_hash, scope_kind, scope_id, fact_id, currency, category, value_real,\
                 source_field, source_line, mapping_id\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                storage_key,
                scope_kind,
                scope_id,
                row.fact_id,
                row.currency.map(|currency| currency.as_str()),
                row.category,
                row.value,
                source_field(market, row.source_field_index)?,
                row.source_line,
                row.fact_id,
            ],
        )?;
    }
    Ok(())
}

fn record_hash(
    record: &crate::model::MarketHistoryRecord,
    market: &ParsedMarketData,
) -> Result<String, ObservatoryError> {
    let mut hasher = Sha256::new();
    hasher.update(record.record_id.to_le_bytes());
    hasher.update(record.year.to_le_bytes());
    hasher.update(record.day.to_le_bytes());
    for row in &record.rows.prices {
        hasher.update(b"price\0");
        hasher.update(resource(market, row.resource_index)?.as_bytes());
        hasher.update(source_field(market, row.source_field_index)?.as_bytes());
        hasher.update([row.currency as u8, row.side as u8]);
        hasher.update(row.value.to_bits().to_le_bytes());
        hasher.update(row.modifier.to_bits().to_le_bytes());
    }
    for row in &record.rows.trades {
        hasher.update(b"trade\0");
        hasher.update(resource(market, row.resource_index)?.as_bytes());
        hasher.update(source_field(market, row.source_field_index)?.as_bytes());
        hasher.update([row.currency as u8, row.direction as u8, row.channel as u8]);
        hasher.update(row.quantity.to_bits().to_le_bytes());
        hasher.update(row.account_value.to_bits().to_le_bytes());
    }
    for row in &record.rows.scalars {
        hasher.update(b"scalar\0");
        hasher.update(row.fact_id.as_bytes());
        hasher.update(source_field(market, row.source_field_index)?.as_bytes());
        hasher.update(row.category.unwrap_or(i32::MIN).to_le_bytes());
        hasher.update(row.value.to_bits().to_le_bytes());
    }
    let digest = hasher.finalize();
    let mut result = String::with_capacity(64);
    for byte in digest {
        write!(&mut result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(result)
}

fn resource(market: &ParsedMarketData, index: u16) -> Result<&str, ObservatoryError> {
    market
        .resources
        .get(index as usize)
        .map(String::as_str)
        .ok_or(ObservatoryError::StorageUnavailable)
}

fn source_field(market: &ParsedMarketData, index: u16) -> Result<&str, ObservatoryError> {
    market
        .source_fields
        .get(index as usize)
        .map(String::as_str)
        .ok_or(ObservatoryError::StorageUnavailable)
}
