use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

use super::{ObservatoryStorage, from_sql_integer, now_ms};
use crate::compatibility_profile::PARSER_ENGINE_VERSION;
use crate::error::ObservatoryError;
use crate::model::{
    AnalysisContext, CoverageWarning, MarketBasketDraft, MarketBasketSummary,
    MarketEvidenceDataset, MarketFactRows, MarketIndexCandidate, MarketIndexingProgress,
    MarketScenarioDraft, MarketScenarioSummary, MarketWarehousePriceFact,
    MarketWarehouseProjection, MarketWarehouseRecord, MarketWarehouseScalarFact,
    MarketWarehouseTradeFact, ParsedMarketData, SaveInspection,
};

pub(crate) const MARKET_STORAGE_CONTRACT_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MarketPersistenceStats {
    pub records_reused: u32,
    pub rows_avoided: u64,
}

impl ObservatoryStorage {
    pub(crate) fn recorded_market_resource_tokens(&self) -> Result<Vec<String>, ObservatoryError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT resource_token FROM market_price_facts \
             UNION SELECT resource_token FROM market_trade_facts \
             UNION SELECT resource_token FROM market_snapshot_price_facts \
             UNION SELECT resource_token FROM market_snapshot_trade_facts \
             ORDER BY resource_token LIMIT 8192",
        )?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub(crate) fn market_cache_totals(&self) -> Result<(u64, u64, u64), ObservatoryError> {
        let connection = self.connect()?;
        let counts = connection
            .query_row(
                "SELECT \
                    (SELECT COUNT(*) FROM market_records), \
                    (SELECT COUNT(*) FROM market_price_facts) + \
                    (SELECT COUNT(*) FROM market_trade_facts) + \
                    (SELECT COUNT(*) FROM market_scalar_facts), \
                    (SELECT COUNT(*) FROM market_observation_records)",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .map_err(ObservatoryError::from)?;
        Ok((
            from_sql_integer(counts.0)?,
            from_sql_integer(counts.1)?,
            from_sql_integer(counts.2)?,
        ))
    }

    pub(crate) fn market_evidence(
        &self,
        analysis_context: AnalysisContext,
    ) -> Result<MarketEvidenceDataset, ObservatoryError> {
        let connection = self.connect()?;
        let (recorded_save_count, indexed_save_count, current_engine_indexed_save_count) =
            market_commissioning_counts(&connection)?;
        let Some(interpretation_id) = analysis_context.head_interpretation_id.as_deref() else {
            return Ok(MarketEvidenceDataset {
                analysis_context,
                projection: None,
                coverage_status: None,
                history_records: 0,
                snapshot_scopes: 0,
                row_count: 0,
                warnings: Vec::new(),
                baskets: Vec::new(),
                scenarios: Vec::new(),
                recorded_save_count,
                indexed_save_count,
                current_engine_indexed_save_count,
            });
        };
        let coverage = connection
            .query_row(
                "SELECT coverage.coverage_status, coverage.history_records, coverage.snapshot_scopes, \
                        coverage.row_count, coverage.warnings_json \
                 FROM market_observation_coverage coverage \
                 JOIN observation_sources source ON source.payload_hash = coverage.payload_hash \
                 WHERE source.interpretation_id = ?1",
                [interpretation_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, u32>(2)?,
                        row.get::<_, u32>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;
        let baskets = {
            let mut statement = connection.prepare(
                "SELECT basket.basket_id, basket.revision, basket.name, basket.currency, \
                        basket.price_side, basket.built_in, basket.base_record_hash, basket.reason, \
                        basket.weights_json, active.definition_id IS NOT NULL \
                 FROM market_basket_revisions basket \
                 LEFT JOIN market_active_selections active \
                   ON active.selection_kind = 'basket' AND active.definition_id = basket.basket_id \
                  AND active.revision = basket.revision \
                 LEFT JOIN market_definition_lifecycle lifecycle \
                   ON lifecycle.definition_kind = 'basket' \
                  AND lifecycle.definition_id = basket.basket_id \
                 WHERE lifecycle.removed_at_ms IS NULL \
                 ORDER BY basket.name, basket.revision DESC",
            )?;
            statement
                .query_map([], |row| {
                    let weights_json: String = row.get(8)?;
                    let weights = serde_json::from_str::<BTreeMap<String, f64>>(&weights_json)
                        .unwrap_or_default()
                        .into_iter()
                        .map(
                            |(resource_token, weight)| crate::model::MarketBasketWeight {
                                resource_token,
                                weight,
                            },
                        )
                        .collect::<Vec<_>>();
                    Ok(MarketBasketSummary {
                        basket_id: row.get(0)?,
                        revision: row.get(1)?,
                        name: row.get(2)?,
                        currency: row.get(3)?,
                        price_side: row.get(4)?,
                        built_in: row.get::<_, i64>(5)? != 0,
                        selected: row.get::<_, i64>(9)? != 0,
                        base_record_hash: row.get(6)?,
                        resource_count: weights.len() as u32,
                        coverage_resources: 0,
                        index_value: None,
                        reason: row.get(7)?,
                        weights,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let scenarios = {
            let mut statement = connection.prepare(
                "SELECT scenario.scenario_id, scenario.revision, scenario.name, \
                        scenario.scenario_kind, scenario.reason, scenario.assumptions_json, \
                        active.definition_id IS NOT NULL \
                 FROM market_scenario_revisions scenario \
                 LEFT JOIN market_active_selections active \
                   ON active.selection_kind = 'scenario' \
                  AND active.definition_id = scenario.scenario_id \
                  AND active.revision = scenario.revision \
                 LEFT JOIN market_definition_lifecycle lifecycle \
                   ON lifecycle.definition_kind = 'scenario' \
                  AND lifecycle.definition_id = scenario.scenario_id \
                 WHERE lifecycle.removed_at_ms IS NULL \
                 ORDER BY scenario.name, scenario.revision DESC",
            )?;
            statement
                .query_map([], |row| {
                    Ok(MarketScenarioSummary {
                        scenario_id: row.get(0)?,
                        revision: row.get(1)?,
                        name: row.get(2)?,
                        scenario_kind: row.get(3)?,
                        reason: row.get(4)?,
                        assumptions_json: row.get(5)?,
                        selected: row.get::<_, i64>(6)? != 0,
                        result_kind: None,
                        result_value: None,
                        result_unit: None,
                        covered_components: 0,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let (coverage_status, history_records, snapshot_scopes, row_count, warnings) =
            if let Some(coverage) = coverage {
                (
                    Some(coverage.0),
                    coverage.1,
                    coverage.2,
                    coverage.3,
                    serde_json::from_str::<Vec<CoverageWarning>>(&coverage.4)
                        .map_err(|_| ObservatoryError::StorageUnavailable)?,
                )
            } else {
                (None, 0, 0, 0, Vec::new())
            };
        let projection = if coverage_status.is_some() {
            Some(self.market_selected_head_projection(interpretation_id)?)
        } else {
            None
        };
        Ok(MarketEvidenceDataset {
            analysis_context,
            projection,
            coverage_status,
            history_records,
            snapshot_scopes,
            row_count,
            warnings,
            baskets,
            scenarios,
            recorded_save_count,
            indexed_save_count,
            current_engine_indexed_save_count,
        })
    }

    pub fn save_market_basket(&self, draft: &MarketBasketDraft) -> Result<(), ObservatoryError> {
        validate_market_id(&draft.basket_id)?;
        validate_market_text(&draft.name, 1, 96, "invalid_name")?;
        validate_market_text(&draft.reason, 1, 512, "invalid_reason")?;
        if !matches!(draft.currency.as_str(), "rub" | "usd") {
            return Err(ObservatoryError::InvalidMarketDefinition(
                "invalid_currency",
            ));
        }
        if !matches!(draft.price_side.as_str(), "purchase" | "sell") {
            return Err(ObservatoryError::InvalidMarketDefinition(
                "invalid_price_side",
            ));
        }
        if draft.weights.is_empty() || draft.weights.len() > 128 {
            return Err(ObservatoryError::InvalidMarketDefinition("invalid_weights"));
        }
        let mut weights = BTreeMap::new();
        for entry in &draft.weights {
            validate_market_text(&entry.resource_token, 1, 128, "invalid_resource")?;
            if !entry.weight.is_finite() || entry.weight <= 0.0 || entry.weight > 1_000_000_000.0 {
                return Err(ObservatoryError::InvalidMarketDefinition("invalid_weight"));
            }
            if weights
                .insert(entry.resource_token.clone(), entry.weight)
                .is_some()
            {
                return Err(ObservatoryError::InvalidMarketDefinition(
                    "duplicate_resource",
                ));
            }
        }
        if draft.base_record_hash.len() != 64
            || !draft
                .base_record_hash
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ObservatoryError::InvalidMarketDefinition(
                "invalid_base_record",
            ));
        }
        let weights_json = serde_json::to_string(&weights)
            .map_err(|_| ObservatoryError::InvalidMarketDefinition("invalid_weights"))?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let base_exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM market_records WHERE record_hash = ?1)",
            [&draft.base_record_hash],
            |row| row.get::<_, bool>(0),
        )?;
        if !base_exists {
            return Err(ObservatoryError::InvalidMarketDefinition(
                "unknown_base_record",
            ));
        }
        let revision = transaction.query_row(
            "SELECT COALESCE(MAX(revision), 0) + 1 FROM market_basket_revisions \
             WHERE basket_id = ?1",
            [&draft.basket_id],
            |row| row.get::<_, u32>(0),
        )?;
        let now = now_ms();
        transaction.execute(
            "INSERT INTO market_basket_revisions( \
                 basket_id, revision, name, currency, price_side, base_record_hash, reason, \
                 weights_json, built_in, created_at_ms \
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9)",
            params![
                draft.basket_id,
                revision,
                draft.name.trim(),
                draft.currency,
                draft.price_side,
                draft.base_record_hash,
                draft.reason.trim(),
                weights_json,
                now,
            ],
        )?;
        transaction.execute(
            "INSERT INTO market_definition_lifecycle( \
                 definition_kind, definition_id, removed_at_ms, updated_at_ms \
             ) VALUES('basket', ?1, NULL, ?2) \
             ON CONFLICT(definition_kind, definition_id) DO UPDATE SET \
                 removed_at_ms = NULL, updated_at_ms = excluded.updated_at_ms",
            params![draft.basket_id, now],
        )?;
        transaction.execute(
            "INSERT INTO market_active_selections( \
                 selection_kind, definition_id, revision, selected_at_ms \
             ) VALUES('basket', ?1, ?2, ?3) \
             ON CONFLICT(selection_kind) DO UPDATE SET definition_id = excluded.definition_id, \
                 revision = excluded.revision, selected_at_ms = excluded.selected_at_ms",
            params![draft.basket_id, revision, now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn save_market_scenario(
        &self,
        draft: &MarketScenarioDraft,
    ) -> Result<(), ObservatoryError> {
        validate_market_id(&draft.scenario_id)?;
        validate_market_text(&draft.name, 1, 96, "invalid_name")?;
        validate_market_text(&draft.reason, 1, 512, "invalid_reason")?;
        validate_scenario(draft)?;
        let assumptions_json = serde_json::to_string(draft)
            .map_err(|_| ObservatoryError::InvalidMarketDefinition("invalid_assumptions"))?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let revision = transaction.query_row(
            "SELECT COALESCE(MAX(revision), 0) + 1 FROM market_scenario_revisions \
             WHERE scenario_id = ?1",
            [&draft.scenario_id],
            |row| row.get::<_, u32>(0),
        )?;
        let now = now_ms();
        transaction.execute(
            "INSERT INTO market_scenario_revisions( \
                 scenario_id, revision, name, scenario_kind, reason, assumptions_json, created_at_ms \
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                draft.scenario_id,
                revision,
                draft.name.trim(),
                draft.scenario_kind,
                draft.reason.trim(),
                assumptions_json,
                now,
            ],
        )?;
        transaction.execute(
            "INSERT INTO market_definition_lifecycle( \
                 definition_kind, definition_id, removed_at_ms, updated_at_ms \
             ) VALUES('scenario', ?1, NULL, ?2) \
             ON CONFLICT(definition_kind, definition_id) DO UPDATE SET \
                 removed_at_ms = NULL, updated_at_ms = excluded.updated_at_ms",
            params![draft.scenario_id, now],
        )?;
        transaction.execute(
            "INSERT INTO market_active_selections( \
                 selection_kind, definition_id, revision, selected_at_ms \
             ) VALUES('scenario', ?1, ?2, ?3) \
             ON CONFLICT(selection_kind) DO UPDATE SET definition_id = excluded.definition_id, \
                 revision = excluded.revision, selected_at_ms = excluded.selected_at_ms",
            params![draft.scenario_id, revision, now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn select_market_definition(
        &self,
        kind: &str,
        definition_id: &str,
        revision: u32,
    ) -> Result<(), ObservatoryError> {
        let table = market_definition_table(kind)?;
        let id_column = market_definition_id_column(kind)?;
        let connection = self.connect()?;
        let sql = format!(
            "SELECT EXISTS(SELECT 1 FROM {table} definition \
             LEFT JOIN market_definition_lifecycle lifecycle \
               ON lifecycle.definition_kind = ?1 AND lifecycle.definition_id = definition.{id_column} \
             WHERE definition.{id_column} = ?2 AND definition.revision = ?3 \
               AND lifecycle.removed_at_ms IS NULL)"
        );
        let exists = connection.query_row(&sql, params![kind, definition_id, revision], |row| {
            row.get::<_, bool>(0)
        })?;
        if !exists {
            return Err(ObservatoryError::UnknownMarketDefinition);
        }
        connection.execute(
            "INSERT INTO market_active_selections( \
                 selection_kind, definition_id, revision, selected_at_ms \
             ) VALUES(?1, ?2, ?3, ?4) \
             ON CONFLICT(selection_kind) DO UPDATE SET definition_id = excluded.definition_id, \
                 revision = excluded.revision, selected_at_ms = excluded.selected_at_ms",
            params![kind, definition_id, revision, now_ms()],
        )?;
        Ok(())
    }

    pub fn rollback_market_definition(
        &self,
        kind: &str,
        definition_id: &str,
    ) -> Result<(), ObservatoryError> {
        let table = market_definition_table(kind)?;
        let id_column = market_definition_id_column(kind)?;
        let connection = self.connect()?;
        let current = connection
            .query_row(
                "SELECT revision FROM market_active_selections \
                 WHERE selection_kind = ?1 AND definition_id = ?2",
                params![kind, definition_id],
                |row| row.get::<_, u32>(0),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownMarketDefinition)?;
        let sql =
            format!("SELECT MAX(revision) FROM {table} WHERE {id_column} = ?1 AND revision < ?2");
        let revision = connection
            .query_row(&sql, params![definition_id, current], |row| {
                row.get::<_, Option<u32>>(0)
            })?
            .ok_or(ObservatoryError::UnknownMarketDefinition)?;
        self.select_market_definition(kind, definition_id, revision)
    }

    pub fn remove_market_definition(
        &self,
        kind: &str,
        definition_id: &str,
    ) -> Result<(), ObservatoryError> {
        market_definition_table(kind)?;
        let connection = self.connect()?;
        let active = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM market_active_selections \
             WHERE selection_kind = ?1 AND definition_id = ?2)",
            params![kind, definition_id],
            |row| row.get::<_, bool>(0),
        )?;
        if active {
            return Err(ObservatoryError::ActiveMarketDefinitionRemove);
        }
        let changed = connection.execute(
            "UPDATE market_definition_lifecycle SET removed_at_ms = ?1, updated_at_ms = ?1 \
             WHERE definition_kind = ?2 AND definition_id = ?3 AND removed_at_ms IS NULL",
            params![now_ms(), kind, definition_id],
        )?;
        if changed == 0 {
            return Err(ObservatoryError::UnknownMarketDefinition);
        }
        Ok(())
    }

    pub fn clear_market_selection(&self, kind: &str) -> Result<(), ObservatoryError> {
        market_definition_table(kind)?;
        let connection = self.connect()?;
        connection.execute(
            "DELETE FROM market_active_selections WHERE selection_kind = ?1",
            [kind],
        )?;
        Ok(())
    }

    pub(crate) fn market_warehouse_projection(
        &self,
        interpretation_id: &str,
    ) -> Result<MarketWarehouseProjection, ObservatoryError> {
        self.market_projection(interpretation_id, true)
    }

    fn market_selected_head_projection(
        &self,
        interpretation_id: &str,
    ) -> Result<MarketWarehouseProjection, ObservatoryError> {
        self.market_projection(interpretation_id, false)
    }

    fn market_projection(
        &self,
        interpretation_id: &str,
        include_history: bool,
    ) -> Result<MarketWarehouseProjection, ObservatoryError> {
        let connection = self.connect()?;
        let source = connection.query_row(
            "SELECT raw_payload_hash, branch_id, profile_id, profile_semantic_version, \
                    resolved_profile_hash, mapping_classification, parser_engine_version, payload_hash \
             FROM observation_sources WHERE interpretation_id = ?1",
            [interpretation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )?;
        let records = {
            let mut statement = connection.prepare(
                "SELECT membership.record_hash, membership.ordinal, record.record_id, \
                        record.year, record.day, record.game_day \
                 FROM market_observation_records membership \
                 JOIN market_records record USING(record_hash) \
                 WHERE membership.payload_hash = ?1 ORDER BY membership.ordinal",
            )?;
            statement
                .query_map([&source.7], |row| {
                    Ok(MarketWarehouseRecord {
                        record_hash: row.get(0)?,
                        ordinal: row.get(1)?,
                        record_id: row.get(2)?,
                        year: row.get(3)?,
                        day: row.get(4)?,
                        game_day: row.get(5)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut prices = load_history_prices(&connection, &source.7, include_history)?;
        prices.extend(load_snapshot_prices(&connection, &source.7)?);
        let mut trades = load_history_trades(&connection, &source.7, include_history)?;
        trades.extend(load_snapshot_trades(&connection, &source.7)?);
        let mut scalars = load_history_scalars(&connection, &source.7, include_history)?;
        scalars.extend(load_snapshot_scalars(&connection, &source.7)?);
        Ok(MarketWarehouseProjection {
            interpretation_id: interpretation_id.to_owned(),
            raw_payload_hash: source.0,
            branch_id: source.1,
            profile_id: source.2,
            profile_version: source.3,
            resolved_profile_hash: source.4,
            mapping_classification: source.5,
            parser_engine_version: source.6,
            records,
            prices,
            trades,
            scalars,
            analytical_trade_history: Vec::new(),
            analytical_price_volatility: Vec::new(),
        })
    }

    pub(crate) fn market_coverage_exists(
        &self,
        interpretation_id: &str,
    ) -> Result<bool, ObservatoryError> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT EXISTS( \
                     SELECT 1 FROM market_observation_coverage coverage \
                     JOIN observation_sources source ON source.payload_hash = coverage.payload_hash \
                     WHERE source.interpretation_id = ?1 \
                       AND coverage.storage_contract_version = ?2 \
                  )",
                params![interpretation_id, MARKET_STORAGE_CONTRACT_VERSION],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn cached_market_variant_counts(
        &self,
        raw_payload_hash: &str,
        resolved_profile_hash: &str,
    ) -> Result<Option<(u32, u32)>, ObservatoryError> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT coverage.history_records, coverage.row_count \
                 FROM market_interpretation_variants variant \
                 JOIN observation_sources source \
                   ON source.interpretation_id = variant.interpretation_id \
                 JOIN market_observation_coverage coverage \
                   ON coverage.payload_hash = source.payload_hash \
                 WHERE variant.raw_payload_hash = ?1 \
                   AND variant.resolved_profile_hash = ?2 \
                   AND coverage.storage_contract_version = ?3 \
                 ORDER BY variant.indexed_at_ms DESC LIMIT 1",
                params![
                    raw_payload_hash,
                    resolved_profile_hash,
                    MARKET_STORAGE_CONTRACT_VERSION
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn market_index_candidates(
        &self,
        source_directory_identity: &str,
    ) -> Result<Vec<MarketIndexCandidate>, ObservatoryError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT ao.source_file_name, ao.source_file_size, ao.source_modified_ms, \
                    ao.source_directory_identity, os.payload_hash, \
                    os.raw_payload_hash \
             FROM archive_observations ao \
             JOIN observation_sources os ON os.payload_hash = ao.payload_hash \
             WHERE ao.source_directory_identity = ?1 \
             ORDER BY os.history_records DESC, ao.observed_at_ms DESC",
        )?;
        let rows = statement.query_map([source_directory_identity], |row| {
            Ok(MarketIndexCandidate {
                payload_hash: row.get(4)?,
                source_file_name: row.get(0)?,
                source_file_size: from_sql_integer(row.get(1)?)?,
                source_modified_ms: row.get(2)?,
                source_directory_identity: row.get(3)?,
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
        refresh_all: bool,
    ) -> Result<MarketIndexingProgress, ObservatoryError> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let resumed = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM market_indexing_jobs WHERE job_id = ?1)",
            [job_id],
            |row| row.get::<_, bool>(0),
        )?;
        transaction.execute(
            "INSERT INTO market_indexing_jobs(job_id, state, started_at_ms, total_archives) \
             VALUES(?1, 'running', ?2, ?3) \
             ON CONFLICT(job_id) DO UPDATE SET state = 'running', completed_at_ms = NULL, \
                  total_archives = excluded.total_archives, last_error_code = NULL",
            params![
                job_id,
                now_ms(),
                candidates.len().min(u32::MAX as usize) as u32
            ],
        )?;
        for candidate in candidates {
            transaction.execute(
                "INSERT OR IGNORE INTO market_indexing_items(job_id, payload_hash, state) \
                 VALUES(?1, ?2, 'pending')",
                params![job_id, candidate.payload_hash],
            )?;
        }
        if refresh_all {
            transaction.execute(
                "UPDATE market_indexing_items SET state = 'pending', error_code = NULL, \
                        records_processed = 0, rows_processed = 0 WHERE job_id = ?1",
                [job_id],
            )?;
        } else {
            transaction.execute(
                "UPDATE market_indexing_items SET state = 'pending', error_code = NULL \
                 WHERE job_id = ?1 AND state IN ('running', 'failed')",
                [job_id],
            )?;
        }
        transaction.commit()?;
        let mut progress = load_market_index_job_progress(&connection, job_id)?;
        progress.resume_count = u32::from(resumed);
        Ok(progress)
    }

    pub(crate) fn market_index_item_states(
        &self,
        job_id: &str,
    ) -> Result<HashMap<String, String>, ObservatoryError> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare("SELECT payload_hash, state FROM market_indexing_items WHERE job_id = ?1")?;
        statement
            .query_map([job_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<HashMap<_, _>, _>>()
            .map_err(Into::into)
    }

    pub(crate) fn latest_market_index_progress(
        &self,
    ) -> Result<MarketIndexingProgress, ObservatoryError> {
        self.latest_index_progress("market-index-")
    }

    pub(crate) fn latest_broadcast_index_progress(
        &self,
    ) -> Result<MarketIndexingProgress, ObservatoryError> {
        self.latest_index_progress("broadcast-index-")
    }

    pub(crate) fn latest_environment_index_progress(
        &self,
    ) -> Result<MarketIndexingProgress, ObservatoryError> {
        self.latest_index_progress("environment-index-")
    }

    fn latest_index_progress(
        &self,
        job_prefix: &str,
    ) -> Result<MarketIndexingProgress, ObservatoryError> {
        let connection = self.connect()?;
        let job_id = connection
            .query_row(
                "SELECT job_id FROM market_indexing_jobs \
                 WHERE substr(job_id, 1, length(?1)) = ?1 \
                 ORDER BY started_at_ms DESC LIMIT 1",
                [job_prefix],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        job_id.map_or_else(
            || Ok(MarketIndexingProgress::default()),
            |job_id| load_market_index_job_progress(&connection, &job_id),
        )
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
            "UPDATE market_indexing_items SET state = ?1, records_processed = ?2, \
                    rows_processed = ?3, error_code = ?4 \
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
        let (state, completed_at_ms) = match progress.phase {
            crate::model::MarketIndexingPhase::Complete => ("complete", Some(now_ms())),
            crate::model::MarketIndexingPhase::Paused => ("running", None),
            crate::model::MarketIndexingPhase::Failed => ("failed", Some(now_ms())),
            _ => ("running", None),
        };
        connection.execute(
            "UPDATE market_indexing_jobs SET state = ?1, completed_at_ms = ?2, \
                    completed_archives = ?3, missing_archives = ?4, changed_archives = ?5, \
                    failed_archives = ?6, duplicate_archives = ?7, last_error_code = ?8 \
             WHERE job_id = ?9",
            params![
                state,
                completed_at_ms,
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

fn load_market_index_job_progress(
    connection: &Connection,
    job_id: &str,
) -> Result<MarketIndexingProgress, ObservatoryError> {
    connection
        .query_row(
            "SELECT jobs.state, jobs.started_at_ms, jobs.completed_at_ms, jobs.total_archives, \
                    jobs.last_error_code, \
                    COALESCE(SUM(CASE WHEN items.state = 'complete' THEN 1 ELSE 0 END), 0), \
                    COALESCE(SUM(CASE WHEN items.state = 'missing' THEN 1 ELSE 0 END), 0), \
                    COALESCE(SUM(CASE WHEN items.state = 'changed' THEN 1 ELSE 0 END), 0), \
                    COALESCE(SUM(CASE WHEN items.state = 'failed' THEN 1 ELSE 0 END), 0), \
                    COALESCE(SUM(CASE WHEN items.state = 'duplicate' THEN 1 ELSE 0 END), 0) \
             FROM market_indexing_jobs jobs \
             LEFT JOIN market_indexing_items items ON items.job_id = jobs.job_id \
             WHERE jobs.job_id = ?1 GROUP BY jobs.job_id",
            [job_id],
            |row| {
                let state = row.get::<_, String>(0)?;
                let error_code = row.get::<_, Option<String>>(4)?;
                let phase = match state.as_str() {
                    "complete" => crate::model::MarketIndexingPhase::Complete,
                    "failed" => crate::model::MarketIndexingPhase::Failed,
                    _ => crate::model::MarketIndexingPhase::Paused,
                };
                Ok(MarketIndexingProgress {
                    job_id: Some(job_id.to_owned()),
                    storage_contract_version: MARKET_STORAGE_CONTRACT_VERSION,
                    phase,
                    progress_percent: (phase == crate::model::MarketIndexingPhase::Complete)
                        .then_some(100),
                    started_at_ms: row.get(1)?,
                    updated_at_ms: row.get::<_, Option<i64>>(2)?.or_else(|| row.get(1).ok()),
                    total_archives: row.get(3)?,
                    completed_archives: row.get(5)?,
                    missing_archives: row.get(6)?,
                    changed_archives: row.get(7)?,
                    failed_archives: row.get(8)?,
                    duplicate_archives: row.get(9)?,
                    error_code,
                    ..MarketIndexingProgress::default()
                })
            },
        )
        .map_err(Into::into)
}

fn market_commissioning_counts(
    connection: &Connection,
) -> Result<(u32, u32, u32), ObservatoryError> {
    connection
        .query_row(
            "SELECT \
                 COUNT(DISTINCT source.raw_payload_hash), \
                 COUNT(DISTINCT CASE WHEN coverage.payload_hash IS NOT NULL \
                                     THEN source.raw_payload_hash END), \
                 COUNT(DISTINCT CASE WHEN coverage.payload_hash IS NOT NULL \
                                       AND source.parser_engine_version = ?1 \
                                     THEN source.raw_payload_hash END) \
             FROM observation_sources source \
             LEFT JOIN market_observation_coverage coverage \
               ON coverage.payload_hash = source.payload_hash",
            [PARSER_ENGINE_VERSION],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, u32>(2)?,
                ))
            },
        )
        .map_err(Into::into)
}

fn market_definition_table(kind: &str) -> Result<&'static str, ObservatoryError> {
    match kind {
        "basket" => Ok("market_basket_revisions"),
        "scenario" => Ok("market_scenario_revisions"),
        _ => Err(ObservatoryError::InvalidMarketDefinition("invalid_kind")),
    }
}

fn market_definition_id_column(kind: &str) -> Result<&'static str, ObservatoryError> {
    match kind {
        "basket" => Ok("basket_id"),
        "scenario" => Ok("scenario_id"),
        _ => Err(ObservatoryError::InvalidMarketDefinition("invalid_kind")),
    }
}

fn validate_market_id(value: &str) -> Result<(), ObservatoryError> {
    if value.len() < 3
        || value.len() > 96
        || value.starts_with("org.republic-observatory.")
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
    {
        return Err(ObservatoryError::InvalidMarketDefinition("invalid_id"));
    }
    Ok(())
}

fn validate_market_text(
    value: &str,
    minimum: usize,
    maximum: usize,
    reason: &'static str,
) -> Result<(), ObservatoryError> {
    let value = value.trim();
    if value.len() < minimum
        || value.len() > maximum
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '<' | '>'))
    {
        return Err(ObservatoryError::InvalidMarketDefinition(reason));
    }
    Ok(())
}

fn validate_scenario(draft: &MarketScenarioDraft) -> Result<(), ObservatoryError> {
    fn finite_bounded(value: Option<f64>, minimum: f64, maximum: f64) -> bool {
        value.is_some_and(|value| value.is_finite() && value >= minimum && value <= maximum)
    }

    if !matches!(draft.currency.as_str(), "rub" | "usd") {
        return Err(ObservatoryError::InvalidMarketDefinition(
            "invalid_currency",
        ));
    }
    if draft.included_income_components.len() > 16 {
        return Err(ObservatoryError::InvalidMarketDefinition(
            "invalid_income_components",
        ));
    }
    let mut components = HashSet::new();
    for component in &draft.included_income_components {
        validate_market_text(component, 1, 80, "invalid_income_component")?;
        if !matches!(
            component.as_str(),
            "standard_exports" | "international_exports" | "tourism_spend"
        ) {
            return Err(ObservatoryError::InvalidMarketDefinition(
                "invalid_income_component",
            ));
        }
        if !components.insert(component) {
            return Err(ObservatoryError::InvalidMarketDefinition(
                "duplicate_income_component",
            ));
        }
    }
    if draft.exchange_rate.is_some()
        && !finite_bounded(draft.exchange_rate, f64::EPSILON, 1_000_000_000.0)
    {
        return Err(ObservatoryError::InvalidMarketDefinition(
            "invalid_exchange_rate",
        ));
    }
    match draft.scenario_kind.as_str() {
        "break_even" => {
            if !finite_bounded(draft.domestic_unit_cost, 0.0, 1_000_000_000_000.0)
                || !finite_bounded(draft.delivery_cost, 0.0, 1_000_000_000_000.0)
                || !finite_bounded(draft.operating_efficiency_percent, f64::EPSILON, 1_000.0)
            {
                return Err(ObservatoryError::InvalidMarketDefinition(
                    "invalid_break_even_assumptions",
                ));
            }
        }
        "debt_stress" => {
            if !finite_bounded(draft.debt_service, f64::EPSILON, 1_000_000_000_000_000.0)
                || draft.included_income_components.is_empty()
                || draft.export_stress_percent.is_some()
                    && !finite_bounded(draft.export_stress_percent, 0.0, 100.0)
                || draft.tourism_stress_percent.is_some()
                    && !finite_bounded(draft.tourism_stress_percent, 0.0, 100.0)
            {
                return Err(ObservatoryError::InvalidMarketDefinition(
                    "invalid_debt_stress_assumptions",
                ));
            }
        }
        _ => {
            return Err(ObservatoryError::InvalidMarketDefinition(
                "invalid_scenario_kind",
            ));
        }
    }
    Ok(())
}

fn load_history_prices(
    connection: &Connection,
    payload_hash: &str,
    include_history: bool,
) -> Result<Vec<MarketWarehousePriceFact>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT fact.record_hash, fact.currency, fact.price_side, fact.resource_token, \
                fact.value_real, fact.modifier_real, fact.source_field, fact.source_line, fact.mapping_id \
         FROM market_price_facts fact JOIN market_observation_records membership USING(record_hash) \
         WHERE membership.payload_hash = ?1 AND (
             ?2 <> 0 OR membership.ordinal = (
                 SELECT MAX(latest.ordinal) FROM market_observation_records latest
                 WHERE latest.payload_hash = ?1
             )
         ) ORDER BY membership.ordinal, fact.source_line",
    )?;
    statement
        .query_map(params![payload_hash, i64::from(include_history)], |row| {
            Ok(MarketWarehousePriceFact {
                record_hash: Some(row.get(0)?),
                scope_kind: None,
                scope_id: None,
                currency: row.get(1)?,
                price_side: row.get(2)?,
                resource_token: row.get(3)?,
                value: row.get(4)?,
                modifier: row.get(5)?,
                source_field: row.get(6)?,
                source_line: row.get(7)?,
                mapping_id: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_snapshot_prices(
    connection: &Connection,
    payload_hash: &str,
) -> Result<Vec<MarketWarehousePriceFact>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT scope_kind, scope_id, currency, price_side, resource_token, value_real, \
                modifier_real, source_field, source_line, mapping_id \
         FROM market_snapshot_price_facts WHERE payload_hash = ?1 \
         ORDER BY scope_kind, scope_id, source_line",
    )?;
    statement
        .query_map([payload_hash], |row| {
            Ok(MarketWarehousePriceFact {
                record_hash: None,
                scope_kind: Some(row.get(0)?),
                scope_id: Some(row.get(1)?),
                currency: row.get(2)?,
                price_side: row.get(3)?,
                resource_token: row.get(4)?,
                value: row.get(5)?,
                modifier: row.get(6)?,
                source_field: row.get(7)?,
                source_line: row.get(8)?,
                mapping_id: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_history_trades(
    connection: &Connection,
    payload_hash: &str,
    include_history: bool,
) -> Result<Vec<MarketWarehouseTradeFact>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT fact.record_hash, fact.currency, fact.direction, fact.channel, fact.resource_token, \
                fact.quantity_real, fact.account_value_real, fact.source_field, fact.source_line, fact.mapping_id \
         FROM market_trade_facts fact JOIN market_observation_records membership USING(record_hash) \
         WHERE membership.payload_hash = ?1 AND (
             ?2 <> 0 OR membership.ordinal = (
                 SELECT MAX(latest.ordinal) FROM market_observation_records latest
                 WHERE latest.payload_hash = ?1
             )
         ) ORDER BY membership.ordinal, fact.source_line",
    )?;
    statement
        .query_map(params![payload_hash, i64::from(include_history)], |row| {
            Ok(MarketWarehouseTradeFact {
                record_hash: Some(row.get(0)?),
                scope_kind: None,
                scope_id: None,
                currency: row.get(1)?,
                direction: row.get(2)?,
                channel: row.get(3)?,
                resource_token: row.get(4)?,
                quantity: row.get(5)?,
                account_value: row.get(6)?,
                source_field: row.get(7)?,
                source_line: row.get(8)?,
                mapping_id: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_snapshot_trades(
    connection: &Connection,
    payload_hash: &str,
) -> Result<Vec<MarketWarehouseTradeFact>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT scope_kind, scope_id, currency, direction, channel, resource_token, quantity_real, \
                account_value_real, source_field, source_line, mapping_id \
         FROM market_snapshot_trade_facts WHERE payload_hash = ?1 \
         ORDER BY scope_kind, scope_id, source_line",
    )?;
    statement
        .query_map([payload_hash], |row| {
            Ok(MarketWarehouseTradeFact {
                record_hash: None,
                scope_kind: Some(row.get(0)?),
                scope_id: Some(row.get(1)?),
                currency: row.get(2)?,
                direction: row.get(3)?,
                channel: row.get(4)?,
                resource_token: row.get(5)?,
                quantity: row.get(6)?,
                account_value: row.get(7)?,
                source_field: row.get(8)?,
                source_line: row.get(9)?,
                mapping_id: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_history_scalars(
    connection: &Connection,
    payload_hash: &str,
    include_history: bool,
) -> Result<Vec<MarketWarehouseScalarFact>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT fact.record_hash, fact.fact_id, fact.currency, fact.category, fact.value_real, \
                fact.source_field, fact.source_line, fact.mapping_id \
         FROM market_scalar_facts fact JOIN market_observation_records membership USING(record_hash) \
         WHERE membership.payload_hash = ?1 AND (
             ?2 <> 0 OR membership.ordinal = (
                 SELECT MAX(latest.ordinal) FROM market_observation_records latest
                 WHERE latest.payload_hash = ?1
             )
         ) ORDER BY membership.ordinal, fact.source_line",
    )?;
    statement
        .query_map(params![payload_hash, i64::from(include_history)], |row| {
            Ok(MarketWarehouseScalarFact {
                record_hash: Some(row.get(0)?),
                scope_kind: None,
                scope_id: None,
                fact_id: row.get(1)?,
                currency: row.get(2)?,
                category: row.get(3)?,
                value: row.get(4)?,
                source_field: row.get(5)?,
                source_line: row.get(6)?,
                mapping_id: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn load_snapshot_scalars(
    connection: &Connection,
    payload_hash: &str,
) -> Result<Vec<MarketWarehouseScalarFact>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT scope_kind, scope_id, fact_id, currency, category, value_real, source_field, \
                source_line, mapping_id FROM market_snapshot_scalar_facts WHERE payload_hash = ?1 \
         ORDER BY scope_kind, scope_id, source_line",
    )?;
    statement
        .query_map([payload_hash], |row| {
            Ok(MarketWarehouseScalarFact {
                record_hash: None,
                scope_kind: Some(row.get(0)?),
                scope_id: Some(row.get(1)?),
                fact_id: row.get(2)?,
                currency: row.get(3)?,
                category: row.get(4)?,
                value: row.get(5)?,
                source_field: row.get(6)?,
                source_line: row.get(7)?,
                mapping_id: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub(crate) fn persist_market_data(
    connection: &Connection,
    storage_key: &str,
    inspection: &SaveInspection,
) -> Result<MarketPersistenceStats, ObservatoryError> {
    let mut stats = MarketPersistenceStats::default();
    let warnings_json = serde_json::to_string(&inspection.market.warnings)
        .map_err(|_| ObservatoryError::StorageUnavailable)?;
    connection.execute(
        "INSERT OR REPLACE INTO market_observation_coverage( \
             payload_hash, coverage_status, history_records, snapshot_scopes, row_count, warnings_json, \
             storage_contract_version \
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            storage_key,
            inspection.market.coverage_status().as_str(),
            inspection.market.records.len().min(u32::MAX as usize) as u32,
            inspection.market.snapshots.len().min(u32::MAX as usize) as u32,
            inspection.market.row_count,
            warnings_json,
            MARKET_STORAGE_CONTRACT_VERSION,
        ],
    )?;

    for (ordinal, record) in inspection.market.records.iter().enumerate() {
        let record_hash = record_hash(record, &inspection.market)?;
        let inserted = connection.execute(
            "INSERT OR IGNORE INTO market_records(record_hash, record_id, year, day, game_day) \
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                record_hash,
                record.record_id,
                record.year,
                record.day,
                record.game_day
            ],
        )? > 0;
        if inserted {
            persist_record_rows(connection, &record_hash, &record.rows, &inspection.market)?;
        } else {
            stats.records_reused = stats.records_reused.saturating_add(1);
            stats.rows_avoided = stats
                .rows_avoided
                .saturating_add(market_fact_row_count(&record.rows));
        }
        connection.execute(
            "INSERT INTO market_observation_records(payload_hash, ordinal, record_hash) \
             VALUES(?1, ?2, ?3) \
             ON CONFLICT(payload_hash, ordinal) DO UPDATE SET record_hash = excluded.record_hash",
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
        "INSERT OR IGNORE INTO market_interpretation_variants( \
             raw_payload_hash, interpretation_id, profile_id, profile_version, resolved_profile_hash, indexed_at_ms \
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
    Ok(stats)
}

fn market_fact_row_count(rows: &MarketFactRows) -> u64 {
    rows.prices
        .len()
        .saturating_add(rows.trades.len())
        .saturating_add(rows.scalars.len())
        .min(u64::MAX as usize) as u64
}

fn persist_record_rows(
    connection: &Connection,
    record_hash: &str,
    rows: &MarketFactRows,
    market: &ParsedMarketData,
) -> Result<(), ObservatoryError> {
    for row in &rows.prices {
        connection.execute(
            "INSERT OR IGNORE INTO market_price_facts( \
                 record_hash, currency, price_side, resource_token, value_real, modifier_real, \
                 source_field, source_line, mapping_id \
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
            "INSERT OR IGNORE INTO market_trade_facts( \
                 record_hash, currency, direction, channel, resource_token, quantity_real, \
                 account_value_real, source_field, source_line, mapping_id \
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
            "INSERT OR IGNORE INTO market_scalar_facts( \
                 record_hash, fact_id, currency, category, value_real, source_field, source_line, mapping_id \
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
            "INSERT OR IGNORE INTO market_snapshot_price_facts( \
                 payload_hash, scope_kind, scope_id, currency, price_side, resource_token, \
                 value_real, modifier_real, source_field, source_line, mapping_id \
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
            "INSERT OR IGNORE INTO market_snapshot_trade_facts( \
                 payload_hash, scope_kind, scope_id, currency, direction, channel, resource_token, \
                 quantity_real, account_value_real, source_field, source_line, mapping_id \
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
            "INSERT OR IGNORE INTO market_snapshot_scalar_facts( \
                 payload_hash, scope_kind, scope_id, fact_id, currency, category, value_real, \
                 source_field, source_line, mapping_id \
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
    hasher.update(b"republic-observatory-market-record-v2\0");
    hasher.update(record.record_id.to_le_bytes());
    hasher.update(record.year.to_le_bytes());
    hasher.update(record.day.to_le_bytes());
    hasher.update(record.game_day.to_le_bytes());
    for row in &record.rows.prices {
        hasher.update(b"price\0");
        hasher.update(resource(market, row.resource_index)?.as_bytes());
        hasher.update(source_field(market, row.source_field_index)?.as_bytes());
        hasher.update([row.currency as u8, row.side as u8]);
        hasher.update(row.source_line.to_le_bytes());
        hasher.update(row.value.to_bits().to_le_bytes());
        hasher.update(row.modifier.to_bits().to_le_bytes());
    }
    for row in &record.rows.trades {
        hasher.update(b"trade\0");
        hasher.update(resource(market, row.resource_index)?.as_bytes());
        hasher.update(source_field(market, row.source_field_index)?.as_bytes());
        hasher.update([row.currency as u8, row.direction as u8, row.channel as u8]);
        hasher.update(row.source_line.to_le_bytes());
        hasher.update(row.quantity.to_bits().to_le_bytes());
        hasher.update(row.account_value.to_bits().to_le_bytes());
    }
    for row in &record.rows.scalars {
        hasher.update(b"scalar\0");
        hasher.update(row.fact_id.as_bytes());
        hasher.update(source_field(market, row.source_field_index)?.as_bytes());
        hasher.update([row.currency.map_or(u8::MAX, |currency| currency as u8)]);
        hasher.update(row.source_line.to_le_bytes());
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
