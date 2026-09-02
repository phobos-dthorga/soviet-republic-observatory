use std::collections::BTreeMap;

use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};

use super::{ObservatoryStorage, now_ms, to_sql_integer};
use crate::error::ObservatoryError;
use crate::model::{
    GameVocabularySource, ResourceCatalogueEntry, ResourceLivePrice, ResourceOriginEvidence,
    ResourceRegistryAssurance, ResourceRegistrySnapshotSummary,
};
use crate::tesmio_probe::ValidatedResourceRegistry;

pub(crate) const RESOURCE_REGISTRY_STORAGE_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub(crate) struct StoredResourceRegistrySettings {
    pub enabled: bool,
    pub assurance: Option<ResourceRegistryAssurance>,
    pub acknowledged_notice_revision: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct StoredLiveResources {
    pub summary: ResourceRegistrySnapshotSummary,
    pub entries: Vec<ResourceCatalogueEntry>,
}

impl ObservatoryStorage {
    pub(crate) fn resource_registry_settings(
        &self,
    ) -> Result<StoredResourceRegistrySettings, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT enabled, assurance, acknowledged_notice_revision \
                 FROM resource_registry_ingestion_state WHERE singleton_id = 1",
                [],
                |row| {
                    let assurance = row
                        .get::<_, Option<String>>(1)?
                        .and_then(|value| ResourceRegistryAssurance::parse(&value));
                    Ok(StoredResourceRegistrySettings {
                        enabled: row.get(0)?,
                        assurance,
                        acknowledged_notice_revision: row.get(2)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub(crate) fn configure_resource_registry_ingestion(
        &self,
        enabled: bool,
        assurance: Option<ResourceRegistryAssurance>,
        notice_revision: u32,
    ) -> Result<(), ObservatoryError> {
        if enabled && assurance.is_none() {
            return Err(ObservatoryError::InvalidResearchSetup);
        }
        self.connect()?.execute(
            "UPDATE resource_registry_ingestion_state SET enabled = ?1, assurance = ?2, \
                    acknowledged_notice_revision = ?3, updated_at_ms = ?4 \
             WHERE singleton_id = 1",
            params![
                enabled,
                assurance.map(ResourceRegistryAssurance::as_str),
                notice_revision,
                now_ms()
            ],
        )?;
        Ok(())
    }

    pub(crate) fn persist_resource_registry(
        &self,
        registry: &ValidatedResourceRegistry,
        assurance: ResourceRegistryAssurance,
        labels: &BTreeMap<u32, (String, String)>,
        vocabulary_revisions: &[GameVocabularySource],
    ) -> Result<ResourceRegistrySnapshotSummary, ObservatoryError> {
        let snapshot_id = snapshot_id(registry, assurance);
        let captured_at_ms = now_ms();
        let game_build_id = format!(
            "{}:{:08x}:{}",
            registry.target_game_version, registry.executable_timestamp, registry.executable_size
        );
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT OR IGNORE INTO resource_registry_snapshots(\
                 snapshot_id, source_content_hash, assurance, game_build_id, probe_version, \
                 loader_api_version, executable_timestamp, executable_size, captured_year, \
                 captured_day, captured_at_ms, resource_count, storage_contract_version\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                snapshot_id,
                registry.source_content_hash,
                assurance.as_str(),
                game_build_id,
                registry.probe_version,
                registry.loader_api_version,
                to_sql_integer(registry.executable_timestamp)?,
                to_sql_integer(registry.executable_size)?,
                registry.year,
                registry.day,
                captured_at_ms,
                registry.entries.len() as u32,
                RESOURCE_REGISTRY_STORAGE_CONTRACT_VERSION,
            ],
        )?;
        for entry in &registry.entries {
            let (resolved_caption, label_source_id) = labels
                .get(&entry.caption_id)
                .map(|(label, source)| (Some(label.as_str()), Some(source.as_str())))
                .unwrap_or((None, None));
            transaction.execute(
                "INSERT OR IGNORE INTO resource_registry_entries(\
                     snapshot_id, live_index, source_token, caption_id, resolved_caption, \
                     label_source_id, resource_kind, transport_class_mask, material_family\
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    snapshot_id,
                    entry.live_index,
                    entry.source_token,
                    entry.caption_id,
                    resolved_caption,
                    label_source_id,
                    entry.resource_kind,
                    entry.transport_class_mask,
                    entry.material_family,
                ],
            )?;
            for (currency, finished, base, buy, sell) in [
                (
                    "RUB",
                    entry.finished_price_rub,
                    entry.base_price_rub,
                    entry.buy_multiplier_rub,
                    entry.sell_multiplier_rub,
                ),
                (
                    "USD",
                    entry.finished_price_usd,
                    entry.base_price_usd,
                    entry.buy_multiplier_usd,
                    entry.sell_multiplier_usd,
                ),
            ] {
                transaction.execute(
                    "INSERT OR IGNORE INTO resource_registry_prices(\
                         snapshot_id, live_index, currency, finished_price, base_price, \
                         buy_multiplier, sell_multiplier, buy_quote, sell_quote\
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        snapshot_id,
                        entry.live_index,
                        currency,
                        finished,
                        base,
                        buy,
                        sell,
                        finished * buy,
                        finished * sell,
                    ],
                )?;
            }
        }
        for revision in vocabulary_revisions {
            let (Some(content_hash), Some(entry_count)) =
                (revision.content_hash.as_deref(), revision.entry_count)
            else {
                continue;
            };
            transaction.execute(
                "INSERT OR IGNORE INTO resource_vocabulary_revisions(\
                     snapshot_id, source_id, content_hash, entry_count, warning_count\
                 ) VALUES(?1, ?2, ?3, ?4, ?5)",
                params![
                    snapshot_id,
                    revision.source_id,
                    content_hash,
                    entry_count,
                    revision.warning_count,
                ],
            )?;
        }
        transaction.execute(
            "INSERT OR IGNORE INTO resource_registry_ingestion_receipts(\
                 snapshot_id, source_content_hash, ingested_at_ms\
             ) VALUES(?1, ?2, ?3)",
            params![snapshot_id, registry.source_content_hash, captured_at_ms],
        )?;
        transaction.execute(
            "UPDATE resource_registry_ingestion_state \
             SET last_ingested_snapshot_id = ?1, updated_at_ms = ?2 WHERE singleton_id = 1",
            params![snapshot_id, captured_at_ms],
        )?;
        super::warehouse_jobs::enqueue_projection_job(
            &transaction,
            &format!("resource-registry:{snapshot_id}"),
            "resource_registry_snapshot",
            &snapshot_id,
            captured_at_ms,
        )?;
        transaction.commit()?;
        self.resource_registry_snapshot(&snapshot_id)?
            .ok_or(ObservatoryError::StorageContractViolation)
    }

    pub(crate) fn resource_registry_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<Option<ResourceRegistrySnapshotSummary>, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT snapshot_id, assurance, game_build_id, probe_version, loader_api_version, \
                        captured_year, captured_day, captured_at_ms, resource_count \
                 FROM resource_registry_snapshots WHERE snapshot_id = ?1",
                [snapshot_id],
                summary_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn latest_resource_registry_snapshot(
        &self,
    ) -> Result<Option<ResourceRegistrySnapshotSummary>, ObservatoryError> {
        self.connect()?
            .query_row(
                "SELECT snapshot_id, assurance, game_build_id, probe_version, loader_api_version, \
                        captured_year, captured_day, captured_at_ms, resource_count \
                 FROM resource_registry_snapshots ORDER BY captured_at_ms DESC, snapshot_id DESC LIMIT 1",
                [],
                summary_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn list_resource_registry_snapshots(
        &self,
    ) -> Result<Vec<ResourceRegistrySnapshotSummary>, ObservatoryError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT snapshot_id, assurance, game_build_id, probe_version, loader_api_version, \
                    captured_year, captured_day, captured_at_ms, resource_count \
             FROM resource_registry_snapshots ORDER BY captured_at_ms DESC, snapshot_id DESC LIMIT 100",
        )?;
        statement
            .query_map([], summary_from_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub(crate) fn latest_live_resources(
        &self,
    ) -> Result<Option<StoredLiveResources>, ObservatoryError> {
        let Some(summary) = self.latest_resource_registry_snapshot()? else {
            return Ok(None);
        };
        self.live_resources(&summary.snapshot_id)
    }

    pub(crate) fn live_resources(
        &self,
        snapshot_id: &str,
    ) -> Result<Option<StoredLiveResources>, ObservatoryError> {
        let Some(summary) = self.resource_registry_snapshot(snapshot_id)? else {
            return Ok(None);
        };
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT live_index, source_token, caption_id, resolved_caption, label_source_id, \
                    resource_kind, transport_class_mask, material_family \
             FROM resource_registry_entries WHERE snapshot_id = ?1 ORDER BY live_index",
        )?;
        let rows = statement
            .query_map([&summary.snapshot_id], |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i32>(5)?,
                    row.get::<_, u32>(6)?,
                    row.get::<_, i32>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut price_statement = connection.prepare(
            "SELECT currency, finished_price, base_price, buy_multiplier, sell_multiplier, \
                    buy_quote, sell_quote FROM resource_registry_prices \
             WHERE snapshot_id = ?1 AND live_index = ?2 ORDER BY currency",
        )?;
        let mut entries = Vec::with_capacity(rows.len());
        for (index, token, caption_id, label, label_source, kind, mask, family) in rows {
            let prices = price_statement
                .query_map(params![summary.snapshot_id, index], |row| {
                    Ok(ResourceLivePrice {
                        currency: row.get(0)?,
                        finished_price: row.get(1)?,
                        base_price: row.get(2)?,
                        buy_multiplier: row.get(3)?,
                        sell_multiplier: row.get(4)?,
                        buy_quote: row.get(5)?,
                        sell_quote: row.get(6)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            let display_name = label.unwrap_or_else(|| readable_token(&token));
            entries.push(ResourceCatalogueEntry {
                resource_id: format!("resource::{token}"),
                source_token: token,
                display_name,
                label_source: label_source.unwrap_or_else(|| "source_token".to_owned()),
                caption_id: Some(caption_id),
                live_index: Some(index),
                resource_kind: Some(kind),
                transport_classes: (0..18).filter(|class| mask & (1 << class) != 0).collect(),
                material_family: Some(family),
                origin: ResourceOriginEvidence {
                    installed_content: false,
                    recorded_save: false,
                    live_game: true,
                    runtime_extension: index >= 57,
                    player_overlay: false,
                    installed_reference_count: 0,
                },
                live_prices: prices,
                latest_live_snapshot_id: Some(summary.snapshot_id.clone()),
            });
        }
        Ok(Some(StoredLiveResources { summary, entries }))
    }
}

fn summary_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<ResourceRegistrySnapshotSummary, rusqlite::Error> {
    let assurance_value = row.get::<_, String>(1)?;
    let assurance = ResourceRegistryAssurance::parse(&assurance_value).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            "invalid resource registry assurance".into(),
        )
    })?;
    Ok(ResourceRegistrySnapshotSummary {
        snapshot_id: row.get(0)?,
        assurance,
        game_build_id: row.get(2)?,
        probe_version: row.get(3)?,
        loader_api_version: row.get(4)?,
        captured_year: row.get(5)?,
        captured_day: row.get(6)?,
        captured_at_ms: row.get(7)?,
        resource_count: row.get(8)?,
    })
}

fn snapshot_id(
    registry: &ValidatedResourceRegistry,
    assurance: ResourceRegistryAssurance,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"republic-observatory-resource-registry.v1\0");
    hasher.update(registry.source_content_hash.as_bytes());
    hasher.update(registry.probe_version.as_bytes());
    hasher.update(registry.loader_api_version.to_le_bytes());
    hasher.update(registry.executable_timestamp.to_le_bytes());
    hasher.update(registry.executable_size.to_le_bytes());
    hasher.update(registry.registry_fingerprint.as_bytes());
    hasher.update(assurance.as_str().as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn readable_token(value: &str) -> String {
    let mut label = value.replace(['_', '-'], " ");
    if let Some(first) = label.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    label
}
