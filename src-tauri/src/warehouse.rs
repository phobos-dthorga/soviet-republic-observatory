use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use duckdb::{Connection, OptionalExt, params};

use crate::definition_catalogue::{
    CatalogueGeneration, CatalogueReuseEntry, DEFINITION_PARSER_VERSION,
};
use crate::error::ObservatoryError;
use crate::model::{
    CatalogueGenerationSummary, CataloguePage, CatalogueSearchFilter,
    CompatibilityCatalogueScopeState, CompatibilityCatalogueScopeStatus, DefinitionDossier,
    DefinitionFact, DefinitionMappingProvenance, DefinitionRelation, DefinitionSummary,
    DefinitionValue, ProductionRouteCoverage, ProductionRouteFlow, ProductionRouteModel,
    ProductionRouteRequest, ReceiverDataset, UnknownDirectiveSummary, WarehouseHealth,
    WarehousePhase, WarehouseSnapshot, WarehouseWriteKind, WarehouseWriteStage,
};
use crate::planning_overlay::{
    OverlayOperationKind, OverlayValue, OverlayValueKind, PlanningOverlayDocument,
};
use crate::warehouse_governor::{
    WarehouseGovernor, WarehouseGovernorSnapshot, WarehouseWritePermit,
};

pub const WAREHOUSE_SCHEMA_VERSION: u32 = 4;
pub const PROJECTOR_VERSION: &str = "republic-observatory-projector.v1";
const MAX_PRODUCTION_ROUTE_RELATIONS: usize = 63;
pub type CatalogueRuntime = (Option<i64>, Option<i64>, Option<String>);

#[derive(Clone, Debug)]
pub struct WarehousePublishProgress {
    pub rows_written: u64,
    pub rows_total: u64,
}

fn catalogue_publish_row_count(generation: &CatalogueGeneration) -> u64 {
    let entity_rows = generation.entities.len().saturating_mul(2);
    let detail_rows = generation.entities.iter().fold(0_usize, |count, entity| {
        count
            .saturating_add(entity.properties.len())
            .saturating_add(entity.relations.len())
            .saturating_add(entity.unknown_directives.len())
    });
    generation
        .sources
        .len()
        .saturating_add(generation.files.len())
        .saturating_add(generation.compatibility_scopes.len())
        .saturating_add(entity_rows)
        .saturating_add(detail_rows)
        .min(u64::MAX as usize) as u64
}

fn note_published_row(
    report: &mut impl FnMut(WarehousePublishProgress),
    rows_written: &mut u64,
    rows_total: u64,
    permit: &WarehouseWritePermit<'_>,
) {
    *rows_written = rows_written.saturating_add(1);
    if *rows_written == rows_total || rows_written.is_multiple_of(512) {
        permit.progress(WarehouseWriteStage::Staging, *rows_written);
        report(WarehousePublishProgress {
            rows_written: *rows_written,
            rows_total,
        });
    }
}

fn scope_state_name(state: CompatibilityCatalogueScopeState) -> &'static str {
    match state {
        CompatibilityCatalogueScopeState::Matched => "matched",
        CompatibilityCatalogueScopeState::Dormant => "dormant",
        CompatibilityCatalogueScopeState::UpdatedUnreviewed => "updated_unreviewed",
        CompatibilityCatalogueScopeState::Conflict => "conflict",
    }
}

fn parse_scope_state(value: Option<String>) -> Option<CompatibilityCatalogueScopeState> {
    match value.as_deref() {
        Some("matched") => Some(CompatibilityCatalogueScopeState::Matched),
        Some("dormant") => Some(CompatibilityCatalogueScopeState::Dormant),
        Some("updated_unreviewed") => Some(CompatibilityCatalogueScopeState::UpdatedUnreviewed),
        Some("conflict") => Some(CompatibilityCatalogueScopeState::Conflict),
        _ => None,
    }
}

const MIGRATIONS: &[(u32, &str)] = &[
    (
        1,
        include_str!("../warehouse_migrations/0001_catalogue_and_analytics.sql"),
    ),
    (
        2,
        include_str!("../warehouse_migrations/0002_planning_projections.sql"),
    ),
    (
        3,
        include_str!("../warehouse_migrations/0003_compatibility_provenance.sql"),
    ),
    (
        4,
        include_str!("../warehouse_migrations/0004_definition_mapping_provenance.sql"),
    ),
];

pub struct AnalyticalWarehouse {
    database_path: PathBuf,
    connection: Mutex<Connection>,
    governor: WarehouseGovernor,
    available: bool,
}

impl std::fmt::Debug for AnalyticalWarehouse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AnalyticalWarehouse")
            .field("database_path", &"<app-local warehouse>")
            .finish_non_exhaustive()
    }
}

impl AnalyticalWarehouse {
    pub fn is_available(&self) -> bool {
        self.available
    }

    pub fn initialise(database_path: PathBuf) -> Result<Self, ObservatoryError> {
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(|_| ObservatoryError::WarehouseUnavailable)?;
        }
        let mut connection = Connection::open(&database_path)?;
        connection.execute_batch(
            "SET autoinstall_known_extensions = false;\
             SET autoload_known_extensions = false;\
             SET enable_external_access = false;",
        )?;
        migrate(&mut connection)?;
        Ok(Self {
            database_path,
            connection: Mutex::new(connection),
            governor: WarehouseGovernor::default(),
            available: true,
        })
    }

    /// Keeps the operational SQLite recorder available when the analytical
    /// warehouse cannot be opened. All analytical operations fail closed until
    /// the next application start, while `health` remains queryable.
    pub fn unavailable(database_path: PathBuf) -> Self {
        Self {
            database_path,
            connection: Mutex::new(
                Connection::open_in_memory().expect("an in-memory DuckDB connection must open"),
            ),
            governor: WarehouseGovernor::default(),
            available: false,
        }
    }

    #[cfg(test)]
    pub fn publish_catalogue(
        &self,
        generation: &CatalogueGeneration,
    ) -> Result<bool, ObservatoryError> {
        self.publish_catalogue_with_progress(generation, |_| {})
    }

    pub fn publish_catalogue_with_progress(
        &self,
        generation: &CatalogueGeneration,
        mut report: impl FnMut(WarehousePublishProgress),
    ) -> Result<bool, ObservatoryError> {
        let mut connection = self.lock()?;
        let rows_total = catalogue_publish_row_count(generation);
        let permit = self
            .governor
            .begin(WarehouseWriteKind::CataloguePublication, rows_total)?;
        let current = connection.query_row(
            "SELECT current_catalogue_generation_id FROM warehouse_metadata WHERE singleton_id = 1",
            [],
            |row| row.get::<_, Option<String>>(0),
        )?;
        connection.execute(
            "UPDATE warehouse_metadata SET last_catalogue_check_ms = ?1 WHERE singleton_id = 1",
            [generation.created_at_ms],
        )?;
        if current.as_deref() == Some(&generation.generation_id) {
            permit.complete();
            return Ok(false);
        }

        let generation_exists = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM catalogue_generations WHERE generation_id = ?1)",
            [&generation.generation_id],
            |row| row.get::<_, bool>(0),
        )?;
        if generation_exists {
            permit.progress(WarehouseWriteStage::Committing, rows_total);
            connection.execute(
                "UPDATE warehouse_metadata SET current_catalogue_generation_id = ?1, \
                 last_catalogue_refresh_ms = ?2, last_catalogue_error_code = NULL \
                 WHERE singleton_id = 1",
                params![generation.generation_id, generation.created_at_ms],
            )?;
            permit.complete();
            return Ok(true);
        }

        let transaction = connection.transaction()?;
        let warning_count = generation
            .files
            .iter()
            .map(|file| u64::from(file.warning_count))
            .sum::<u64>();
        let mut rows_written = 0_u64;
        report(WarehousePublishProgress {
            rows_written,
            rows_total,
        });

        transaction.execute_batch(
            "CREATE OR REPLACE TEMP TABLE incoming_definition_entity_revisions AS \
                 SELECT * FROM definition_entity_revisions WHERE FALSE; \
             CREATE OR REPLACE TEMP TABLE incoming_definition_properties AS \
                 SELECT * FROM definition_properties WHERE FALSE; \
             CREATE OR REPLACE TEMP TABLE incoming_definition_relations AS \
                 SELECT * FROM definition_relations WHERE FALSE; \
             CREATE OR REPLACE TEMP TABLE incoming_definition_unknown_directives AS \
                 SELECT * FROM definition_unknown_directives WHERE FALSE;",
        )?;

        transaction.execute(
            "INSERT INTO catalogue_generations(\
                 generation_id, game_build_id, parser_version, created_at_ms, source_count,\
                 file_count, entity_count, property_count, relation_count, warning_count,\
                 compatibility_profile_id, compatibility_profile_version,\
                 compatibility_profile_hash, mapping_classification\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                generation.generation_id,
                generation.game_build_id,
                DEFINITION_PARSER_VERSION,
                generation.created_at_ms,
                generation.sources.len() as u64,
                generation.files.len() as u64,
                generation.entities.len() as u64,
                0_u64,
                0_u64,
                warning_count,
                generation.compatibility.profile_id,
                generation.compatibility.profile_version,
                generation.compatibility.resolved_profile_hash,
                generation.compatibility.mapping_classification,
            ],
        )?;

        {
            let mut appender = transaction.appender("catalogue_sources")?;
            for source in &generation.sources {
                appender.append_row(params![
                    generation.generation_id,
                    source.source_id,
                    source.source_kind,
                    source.package_name,
                    source.package_version,
                    source.content_hash,
                    source.file_count,
                ])?;
                note_published_row(&mut report, &mut rows_written, rows_total, &permit);
            }
        }
        {
            let mut appender = transaction.appender("catalogue_files")?;
            for file in &generation.files {
                appender.append_row(params![
                    generation.generation_id,
                    file.source_id,
                    file.logical_path,
                    file.content_hash,
                    file.byte_size,
                    DEFINITION_PARSER_VERSION,
                    file.warning_count,
                ])?;
                note_published_row(&mut report, &mut rows_written, rows_total, &permit);
            }
        }
        {
            let mut appender = transaction.appender("catalogue_scope_evaluations")?;
            for scope in &generation.compatibility_scopes {
                appender.append_row(params![
                    generation.generation_id,
                    scope.id,
                    scope.source_id,
                    scope.package_name,
                    scope.update_policy,
                    scope.acknowledged_content_hash,
                    scope.current_content_hash,
                    scope.mapping_count,
                    scope_state_name(scope.state),
                ])?;
                note_published_row(&mut report, &mut rows_written, rows_total, &permit);
            }
        }

        {
            let mut appender = transaction.appender("incoming_definition_entity_revisions")?;
            for entity in &generation.entities {
                appender.append_row(params![
                    entity.revision_hash,
                    entity.entity_kind,
                    entity.source_id,
                    entity.source_object_id,
                    entity.display_name,
                    entity.coverage,
                ])?;
                note_published_row(&mut report, &mut rows_written, rows_total, &permit);
            }
        }
        {
            let mut appender = transaction.appender("incoming_definition_properties")?;
            for entity in &generation.entities {
                for property in &entity.properties {
                    appender.append_row(params![
                        entity.revision_hash,
                        property.field_id,
                        property.occurrence,
                        property.value_kind,
                        property.value_number,
                        property.value_text,
                        property.unit,
                        property.source_directive,
                        property.source_line,
                        property.raw_arguments,
                        "game_definition",
                        property.resolution,
                        property.mapping_id,
                        property.catalogue_scope_id,
                        property.mapping_classification,
                    ])?;
                    note_published_row(&mut report, &mut rows_written, rows_total, &permit);
                }
            }
        }
        {
            let mut appender = transaction.appender("incoming_definition_relations")?;
            for entity in &generation.entities {
                for relation in &entity.relations {
                    appender.append_row(params![
                        entity.revision_hash,
                        relation.relation_kind,
                        relation.occurrence,
                        relation.target_id,
                        relation.quantity,
                        relation.unit,
                        relation.phase_id,
                        relation.source_directive,
                        relation.source_line,
                        relation.raw_arguments,
                        relation.resolution,
                        relation.mapping_id,
                        relation.catalogue_scope_id,
                        relation.mapping_classification,
                    ])?;
                    note_published_row(&mut report, &mut rows_written, rows_total, &permit);
                }
            }
        }
        {
            let mut appender = transaction.appender("incoming_definition_unknown_directives")?;
            for entity in &generation.entities {
                for (directive, count) in &entity.unknown_directives {
                    appender.append_row(params![entity.revision_hash, directive, count])?;
                    note_published_row(&mut report, &mut rows_written, rows_total, &permit);
                }
            }
        }

        {
            let mut appender = transaction.appender("catalogue_generation_entities")?;
            for entity in &generation.entities {
                appender.append_row(params![
                    generation.generation_id,
                    entity.entity_id,
                    entity.revision_hash
                ])?;
                note_published_row(&mut report, &mut rows_written, rows_total, &permit);
            }
        }

        permit.progress(WarehouseWriteStage::Merging, rows_written);
        transaction.execute_batch(
            "CREATE OR REPLACE TEMP TABLE incoming_new_revisions AS \
                 SELECT incoming.revision_hash \
                 FROM incoming_definition_entity_revisions incoming \
                 LEFT JOIN definition_entity_revisions existing USING(revision_hash) \
                 WHERE existing.revision_hash IS NULL; \
             INSERT INTO definition_entity_revisions \
                 SELECT incoming.* FROM incoming_definition_entity_revisions incoming \
                 JOIN incoming_new_revisions USING(revision_hash); \
             INSERT INTO definition_properties \
                 SELECT incoming.* FROM incoming_definition_properties incoming \
                 JOIN incoming_new_revisions USING(revision_hash); \
             INSERT INTO definition_relations \
                 SELECT incoming.* FROM incoming_definition_relations incoming \
                 JOIN incoming_new_revisions USING(revision_hash); \
             INSERT INTO definition_unknown_directives \
                 SELECT incoming.* FROM incoming_definition_unknown_directives incoming \
                 JOIN incoming_new_revisions USING(revision_hash);",
        )?;

        report(WarehousePublishProgress {
            rows_written,
            rows_total,
        });
        permit.progress(WarehouseWriteStage::Committing, rows_written);
        transaction.execute(
            "UPDATE catalogue_generations SET \
                 property_count = (SELECT COUNT(*) FROM catalogue_generation_entities membership \
                     JOIN definition_properties properties USING(revision_hash) \
                     WHERE membership.generation_id = ?1), \
                 relation_count = (SELECT COUNT(*) FROM catalogue_generation_entities membership \
                     JOIN definition_relations relations USING(revision_hash) \
                     WHERE membership.generation_id = ?1) \
             WHERE generation_id = ?1",
            [&generation.generation_id],
        )?;
        transaction.execute(
            "UPDATE warehouse_metadata SET current_catalogue_generation_id = ?1, \
             last_catalogue_refresh_ms = ?2, last_catalogue_error_code = NULL WHERE singleton_id = 1",
            params![generation.generation_id, generation.created_at_ms],
        )?;
        transaction.commit()?;
        permit.complete();
        Ok(true)
    }

    pub fn note_catalogue_failure(&self, checked_at_ms: i64, code: &str) {
        if let Ok(connection) = self.lock() {
            let _ = connection.execute(
                "UPDATE warehouse_metadata SET last_catalogue_check_ms = ?1, \
                 last_catalogue_error_code = ?2 WHERE singleton_id = 1",
                params![checked_at_ms, code],
            );
        }
    }

    pub fn note_projection_failure(&self) {
        self.governor.note_failure();
    }

    pub fn note_catalogue_write_failure(&self) {
        self.governor.note_failure();
    }

    pub fn retry_delay(&self) -> std::time::Duration {
        self.governor.retry_delay()
    }

    pub fn catalogue_reuse_cache(
        &self,
    ) -> Result<HashMap<String, CatalogueReuseEntry>, ObservatoryError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT revisions.revision_hash, revisions.display_name, revisions.coverage, \
                    relations.target_id, relations.relation_kind \
             FROM catalogue_generation_entities membership \
             JOIN warehouse_metadata metadata ON membership.generation_id = metadata.current_catalogue_generation_id \
             JOIN definition_entity_revisions revisions USING(revision_hash) \
             LEFT JOIN definition_relations relations ON relations.revision_hash = revisions.revision_hash \
                  AND relations.target_id LIKE 'resource::%' \
             WHERE metadata.singleton_id = 1 \
               AND revisions.entity_kind IN ('building', 'vehicle')",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        let mut cache = HashMap::<String, CatalogueReuseEntry>::new();
        for row in rows {
            let (revision_hash, display_name, coverage, resource_target, relation_kind) = row?;
            let entry = cache.entry(revision_hash).or_insert(CatalogueReuseEntry {
                display_name,
                coverage,
                resource_targets: Vec::new(),
                has_production_route: false,
            });
            entry.has_production_route |= relation_kind.is_some_and(|kind| {
                matches!(
                    kind.as_str(),
                    "production_input" | "production_output" | "waste_input"
                )
            });
            if let Some(resource) = resource_target
                .and_then(|target| target.strip_prefix("resource::").map(str::to_owned))
                && !entry.resource_targets.contains(&resource)
            {
                entry.resource_targets.push(resource);
            }
        }
        Ok(cache)
    }

    pub fn project_observation(
        &self,
        projection_id: &str,
        dataset: &ReceiverDataset,
        applied_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.lock()?;
        if receipt_exists(&connection, projection_id)? {
            self.governor.note_success();
            return Ok(());
        }
        let rows_total = u64::try_from(dataset.points.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(4);
        let permit = self
            .governor
            .begin(WarehouseWriteKind::ObservationProjection, rows_total)?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(
            "CREATE OR REPLACE TEMP TABLE incoming_observation_metrics AS \
                 SELECT * FROM observation_metrics WHERE FALSE;",
        )?;
        {
            let mut appender = transaction.appender("incoming_observation_metrics")?;
            let mut rows_written = 0_u64;
            for point in &dataset.points {
                for (metric_id, value) in [
                    ("core.citizens.electronics.none", point.none),
                    ("core.citizens.electronics.radio", point.radio),
                    ("core.citizens.electronics.television", point.television),
                    ("core.citizens.electronics.computer", point.computer),
                ] {
                    appender.append_row(params![
                        dataset.interpretation_id,
                        dataset.branch_id,
                        point.record_id,
                        point.year,
                        point.day,
                        point.game_day,
                        metric_id,
                        value,
                        dataset.interpretation_id,
                        dataset.payload_hash,
                        dataset.compatibility.profile_id,
                        dataset.compatibility.profile_version,
                        dataset.compatibility.profile_content_hash,
                        dataset.compatibility.resolved_profile_hash,
                        dataset.compatibility.mapping_classification,
                    ])?;
                    rows_written = rows_written.saturating_add(1);
                    if rows_written == rows_total || rows_written.is_multiple_of(512) {
                        permit.progress(WarehouseWriteStage::Staging, rows_written);
                    }
                }
            }
        }
        permit.progress(WarehouseWriteStage::Merging, rows_total);
        transaction.execute_batch(
            "INSERT INTO observation_metrics \
                 SELECT * FROM incoming_observation_metrics ON CONFLICT DO NOTHING;",
        )?;
        record_receipt(
            &transaction,
            projection_id,
            "observation",
            &dataset.interpretation_id,
            applied_at_ms,
        )?;
        transaction.execute(
            "UPDATE warehouse_metadata SET last_projection_ms = ?1, observation_watermark = ?2 \
             WHERE singleton_id = 1",
            params![applied_at_ms, dataset.interpretation_id],
        )?;
        permit.progress(WarehouseWriteStage::Committing, rows_total);
        transaction.commit()?;
        permit.complete();
        Ok(())
    }

    pub fn project_overlay(
        &self,
        projection_id: &str,
        active: Option<(&str, u32, &PlanningOverlayDocument)>,
        applied_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.lock()?;
        if receipt_exists(&connection, projection_id)? {
            self.governor.note_success();
            return Ok(());
        }
        let rows_total = active
            .map(|(_, _, document)| {
                document
                    .operations
                    .len()
                    .saturating_add(document.supplements.len())
            })
            .and_then(|rows| u64::try_from(rows).ok())
            .unwrap_or_default();
        let permit = self
            .governor
            .begin(WarehouseWriteKind::OverlayProjection, rows_total)?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(
            r#"CREATE OR REPLACE TEMP TABLE incoming_overlay_operations(
                   profile_id VARCHAR, revision BIGINT, operation_index BIGINT, operation VARCHAR,
                   entity_id VARCHAR, field_id VARCHAR, occurrence BIGINT,
                   expected_revision_hash VARCHAR, expected_value_kind VARCHAR,
                   expected_value_number DOUBLE, expected_value_text VARCHAR, expected_value_unit VARCHAR,
                   value_kind VARCHAR, value_number DOUBLE, value_text VARCHAR, unit VARCHAR, reason VARCHAR);
               CREATE OR REPLACE TEMP TABLE incoming_overlay_entities AS
                   SELECT * FROM active_overlay_entities WHERE FALSE;
               DELETE FROM active_overlay_operations;
               DELETE FROM active_overlay_entities;"#,
        )?;
        if let Some((profile_id, revision, document)) = active {
            let mut rows_written = 0_u64;
            {
                let mut appender = transaction.appender("incoming_overlay_operations")?;
                for (index, operation) in document.operations.iter().enumerate() {
                    let expected = operation.expected_value.as_ref();
                    let value = operation.value.as_ref();
                    appender.append_row(params![
                        profile_id,
                        revision,
                        index as u64,
                        match operation.operation {
                            OverlayOperationKind::Set => "set",
                            OverlayOperationKind::Unset => "unset",
                            OverlayOperationKind::Add => "add",
                        },
                        operation.entity_id,
                        operation.field_id,
                        operation.occurrence,
                        operation.expected_revision_hash,
                        expected.map(|value| value.kind.as_str()),
                        expected.and_then(|value| value.number),
                        expected.and_then(overlay_text),
                        expected.and_then(|value| value.unit.as_deref()),
                        value.map(|value| value.kind.as_str()),
                        value.and_then(|value| value.number),
                        value.and_then(overlay_text),
                        value.and_then(|value| value.unit.as_deref()),
                        operation.reason,
                    ])?;
                    rows_written = rows_written.saturating_add(1);
                    if rows_written == rows_total || rows_written.is_multiple_of(512) {
                        permit.progress(WarehouseWriteStage::Staging, rows_written);
                    }
                }
            }
            {
                let mut appender = transaction.appender("incoming_overlay_entities")?;
                for supplement in &document.supplements {
                    let entity_id = format!(
                        "overlay::{profile_id}::{}::{}",
                        supplement.entity_kind, supplement.local_id
                    );
                    let properties_json = serde_json::to_string(&supplement.properties)
                        .map_err(|_| ObservatoryError::InvalidPlanningOverlay("invalid_json"))?;
                    appender.append_row(params![
                        profile_id,
                        revision,
                        entity_id,
                        supplement.entity_kind,
                        supplement.display_name,
                        supplement.reason,
                        properties_json,
                    ])?;
                    rows_written = rows_written.saturating_add(1);
                    if rows_written == rows_total || rows_written.is_multiple_of(512) {
                        permit.progress(WarehouseWriteStage::Staging, rows_written);
                    }
                }
            }
            permit.progress(WarehouseWriteStage::Merging, rows_written);
            transaction.execute_batch(
                r#"INSERT INTO active_overlay_operations
                   SELECT incoming.profile_id, incoming.revision, incoming.operation_index,
                          incoming.operation, incoming.entity_id, incoming.field_id, incoming.occurrence,
                          incoming.expected_revision_hash, incoming.value_kind, incoming.value_number,
                          incoming.value_text, incoming.unit, incoming.reason,
                          CASE
                            WHEN membership.revision_hash IS NULL THEN 'target_missing'
                            WHEN membership.revision_hash <> incoming.expected_revision_hash THEN 'revision_changed'
                            WHEN incoming.expected_value_kind IS NOT NULL AND NOT EXISTS (
                              SELECT 1 FROM definition_properties property
                              WHERE property.revision_hash = membership.revision_hash
                                AND property.field_id = incoming.field_id
                                AND property.occurrence = COALESCE(incoming.occurrence, 0)
                                AND property.value_kind = incoming.expected_value_kind
                                AND property.value_number IS NOT DISTINCT FROM incoming.expected_value_number
                                AND property.value_text IS NOT DISTINCT FROM incoming.expected_value_text
                                AND property.unit IS NOT DISTINCT FROM incoming.expected_value_unit
                            ) THEN 'value_changed'
                            ELSE NULL
                          END
                   FROM incoming_overlay_operations incoming
                   CROSS JOIN warehouse_metadata metadata
                   LEFT JOIN catalogue_generation_entities membership
                     ON membership.generation_id = metadata.current_catalogue_generation_id
                    AND membership.entity_id = incoming.entity_id
                   WHERE metadata.singleton_id = 1;
                   INSERT INTO active_overlay_entities SELECT * FROM incoming_overlay_entities;"#,
            )?;
            transaction.execute(
                "UPDATE warehouse_metadata SET active_overlay_profile_id = ?1, \
                 active_overlay_revision = ?2 WHERE singleton_id = 1",
                params![profile_id, revision],
            )?;
        } else {
            transaction.execute(
                "UPDATE warehouse_metadata SET active_overlay_profile_id = NULL, \
                 active_overlay_revision = NULL WHERE singleton_id = 1",
                [],
            )?;
        }
        let source_identity = active
            .map(|(profile, revision, _)| format!("{profile}:{revision}"))
            .unwrap_or_else(|| "none".to_owned());
        record_receipt(
            &transaction,
            projection_id,
            "overlay_state",
            &source_identity,
            applied_at_ms,
        )?;
        transaction.execute(
            "UPDATE warehouse_metadata SET last_projection_ms = ?1 WHERE singleton_id = 1",
            [applied_at_ms],
        )?;
        permit.progress(WarehouseWriteStage::Committing, rows_total);
        transaction.commit()?;
        permit.complete();
        Ok(())
    }

    pub fn rebuild_observations(
        &self,
        projection_id: &str,
        applied_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.lock()?;
        if receipt_exists(&connection, projection_id)? {
            self.governor.note_success();
            return Ok(());
        }
        let permit = self
            .governor
            .begin(WarehouseWriteKind::ObservationRebuild, 0)?;
        permit.progress(WarehouseWriteStage::Rebuilding, 0);
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM observation_metrics", [])?;
        transaction.execute(
            "DELETE FROM projection_receipts WHERE projection_kind = 'observation'",
            [],
        )?;
        record_receipt(
            &transaction,
            projection_id,
            "rebuild",
            "all_observations",
            applied_at_ms,
        )?;
        transaction.execute(
            "UPDATE warehouse_metadata SET observation_watermark = NULL, last_projection_ms = ?1 \
             WHERE singleton_id = 1",
            [applied_at_ms],
        )?;
        permit.progress(WarehouseWriteStage::Committing, 0);
        transaction.commit()?;
        permit.complete();
        Ok(())
    }

    #[cfg(test)]
    pub fn health(
        &self,
        pending_jobs: u32,
        failed_jobs: u32,
        lag_ms: Option<i64>,
        rebuilding: bool,
    ) -> Result<WarehouseHealth, ObservatoryError> {
        if !self.available {
            return Ok(self.health_shell(
                WarehousePhase::Attention,
                pending_jobs,
                failed_jobs,
                lag_ms,
            ));
        }
        let connection = self.lock()?;
        let (last_projected_at_ms, observation_watermark) = connection.query_row(
            "SELECT last_projection_ms, observation_watermark FROM warehouse_metadata WHERE singleton_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let phase = if failed_jobs > 0 {
            WarehousePhase::Attention
        } else if rebuilding {
            WarehousePhase::Rebuilding
        } else if pending_jobs > 0 {
            WarehousePhase::Lagging
        } else {
            WarehousePhase::Ready
        };
        Ok(WarehouseHealth {
            last_projected_at_ms,
            observation_watermark,
            ..self.health_shell(phase, pending_jobs, failed_jobs, lag_ms)
        })
    }

    pub fn health_snapshot(
        &self,
        pending_jobs: u32,
        failed_jobs: u32,
        lag_ms: Option<i64>,
        rebuilding: bool,
    ) -> WarehouseHealth {
        let phase = if !self.available || failed_jobs > 0 {
            WarehousePhase::Attention
        } else if rebuilding {
            WarehousePhase::Rebuilding
        } else if pending_jobs > 0 {
            WarehousePhase::Lagging
        } else {
            WarehousePhase::Ready
        };
        let fallback = self.health_shell(phase, pending_jobs, failed_jobs, lag_ms);
        if !self.available {
            return fallback;
        }
        let Ok(connection) = self.connection.try_lock() else {
            return WarehouseHealth {
                phase: if fallback.phase == WarehousePhase::Attention {
                    WarehousePhase::Attention
                } else if rebuilding {
                    WarehousePhase::Rebuilding
                } else {
                    WarehousePhase::Lagging
                },
                ..fallback
            };
        };
        let Ok((last_projected_at_ms, observation_watermark)) = connection.query_row(
            "SELECT last_projection_ms, observation_watermark FROM warehouse_metadata WHERE singleton_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ) else {
            return fallback;
        };
        WarehouseHealth {
            last_projected_at_ms,
            observation_watermark,
            ..fallback
        }
    }

    fn health_shell(
        &self,
        phase: WarehousePhase,
        pending_jobs: u32,
        failed_jobs: u32,
        lag_ms: Option<i64>,
    ) -> WarehouseHealth {
        let WarehouseGovernorSnapshot {
            active_write,
            consecutive_failures,
            retry_after_ms,
        } = self.governor.snapshot();
        WarehouseHealth {
            phase: if consecutive_failures > 0 {
                WarehousePhase::Attention
            } else {
                phase
            },
            schema_version: WAREHOUSE_SCHEMA_VERSION,
            pending_jobs,
            failed_jobs,
            lag_ms,
            last_projected_at_ms: None,
            observation_watermark: None,
            database_size_bytes: fs::metadata(&self.database_path)
                .map(|metadata| metadata.len())
                .unwrap_or_default(),
            active_write,
            consecutive_write_failures: consecutive_failures,
            retry_after_ms,
        }
    }

    pub fn catalogue_generation_if_ready(&self) -> Option<CatalogueGenerationSummary> {
        if !self.available {
            return None;
        }
        let connection = self.connection.try_lock().ok()?;
        catalogue_generation_from(&connection).ok().flatten()
    }

    pub fn catalogue_scope_statuses_if_ready(
        &self,
    ) -> Option<Vec<CompatibilityCatalogueScopeStatus>> {
        if !self.available {
            return None;
        }
        let connection = self.connection.try_lock().ok()?;
        catalogue_scope_statuses_from(&connection).ok()
    }

    pub fn catalogue_runtime_if_ready(&self) -> Option<CatalogueRuntime> {
        if !self.available {
            return None;
        }
        let connection = self.connection.try_lock().ok()?;
        catalogue_runtime_from(&connection).ok()
    }

    #[cfg(test)]
    pub fn catalogue_scope_statuses(
        &self,
    ) -> Result<Vec<CompatibilityCatalogueScopeStatus>, ObservatoryError> {
        let connection = self.lock()?;
        catalogue_scope_statuses_from(&connection)
    }

    pub fn search(
        &self,
        filter: &CatalogueSearchFilter,
    ) -> Result<CataloguePage, ObservatoryError> {
        let query = filter.query.as_deref().unwrap_or("").trim();
        if query.len() > 120 {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }
        let kind = filter.entity_kind.as_deref().unwrap_or("");
        if !kind.is_empty() && !matches!(kind, "resource" | "building" | "vehicle" | "recipe") {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }
        let source_kind = filter.source_kind.as_deref().unwrap_or("");
        if !source_kind.is_empty()
            && !matches!(source_kind, "base" | "dlc" | "workshop" | "wip" | "derived")
        {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }
        let package_query = filter.package_query.as_deref().unwrap_or("").trim();
        if package_query.len() > 120 {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }
        let coverage = filter.coverage.as_deref().unwrap_or("");
        if !coverage.is_empty() && !matches!(coverage, "complete" | "partial") {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }
        let available_year = filter.available_year.map(i64::from);
        if available_year.is_some_and(|year| !(1800..=3000).contains(&year)) {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }
        let limit = filter.limit.unwrap_or(50).clamp(1, 100);
        let offset = filter.offset.unwrap_or(0).min(1_000_000);
        let connection = self.lock()?;
        let base = " FROM catalogue_generation_entities membership \
                    JOIN warehouse_metadata metadata ON membership.generation_id = metadata.current_catalogue_generation_id \
                    JOIN definition_entity_revisions revisions USING(revision_hash) \
                    JOIN catalogue_sources sources ON sources.generation_id = membership.generation_id \
                         AND sources.source_id = revisions.source_id \
                    WHERE metadata.singleton_id = 1 \
                      AND (?1 = '' OR lower(revisions.display_name) LIKE concat('%', lower(?1), '%') \
                           OR lower(membership.entity_id) LIKE concat('%', lower(?1), '%')) \
                      AND (?2 = '' OR revisions.entity_kind = ?2) \
                      AND (?3 = '' OR sources.source_kind = ?3) \
                      AND (?4 = '' OR lower(sources.package_name) LIKE concat('%', lower(?4), '%')) \
                      AND (?5 = '' OR revisions.coverage = ?5) \
                      AND (?6 IS NULL OR ( \
                          EXISTS(SELECT 1 FROM definition_properties available_from \
                            WHERE available_from.revision_hash = revisions.revision_hash \
                              AND available_from.field_id = 'definition.available.from_year' \
                              AND available_from.value_number <= ?6) \
                          AND EXISTS(SELECT 1 FROM definition_properties available_to \
                            WHERE available_to.revision_hash = revisions.revision_hash \
                              AND available_to.field_id = 'definition.available.to_year' \
                              AND available_to.value_number >= ?6)))";
        let total = connection.query_row(
            &format!("SELECT COUNT(*){base}"),
            params![
                query,
                kind,
                source_kind,
                package_query,
                coverage,
                available_year
            ],
            |row| row.get::<_, u32>(0),
        )?;
        let mut statement = connection.prepare(&format!(
            "SELECT membership.entity_id, revisions.revision_hash, revisions.entity_kind, \
                    revisions.source_id, sources.source_kind, sources.package_name, \
                    revisions.display_name, revisions.coverage, \
                    (SELECT COUNT(*) FROM definition_properties properties WHERE properties.revision_hash = revisions.revision_hash), \
                    (SELECT COUNT(*) FROM definition_relations relations WHERE relations.revision_hash = revisions.revision_hash) \
             {base} ORDER BY revisions.display_name, membership.entity_id LIMIT ?7 OFFSET ?8"
        ))?;
        let items = statement
            .query_map(
                params![
                    query,
                    kind,
                    source_kind,
                    package_query,
                    coverage,
                    available_year,
                    limit,
                    offset
                ],
                summary_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CataloguePage {
            total,
            limit,
            offset,
            items,
        })
    }

    pub fn production_route(
        &self,
        request: &ProductionRouteRequest,
    ) -> Result<ProductionRouteModel, ObservatoryError> {
        if request.entity_id.is_empty()
            || request.entity_id.len() > 320
            || request.output_resource_id.as_deref().is_some_and(|value| {
                value.len() > 160
                    || !value.starts_with("resource::")
                    || !value.chars().all(|character| {
                        character.is_ascii_alphanumeric()
                            || matches!(character, ':' | '_' | '-' | '.')
                    })
            })
            || request
                .target_quantity
                .is_some_and(|value| !value.is_finite() || value <= 0.0 || value > 1_000_000_000.0)
        {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }

        let connection = self.lock()?;
        let (route_id, revision_hash, display_name, package_name, coverage, relation_count) =
            connection
                .query_row(
                    "SELECT membership.entity_id, revisions.revision_hash, revisions.display_name, \
                            sources.package_name, revisions.coverage, \
                            (SELECT COUNT(*) FROM definition_relations relations \
                             WHERE relations.revision_hash = revisions.revision_hash \
                               AND relations.relation_kind IN \
                                   ('production_input', 'production_output', 'waste_input')) \
                     FROM catalogue_generation_entities membership \
                     JOIN warehouse_metadata metadata \
                       ON membership.generation_id = metadata.current_catalogue_generation_id \
                     JOIN definition_entity_revisions revisions USING(revision_hash) \
                     JOIN catalogue_sources sources ON sources.generation_id = membership.generation_id \
                          AND sources.source_id = revisions.source_id \
                     WHERE metadata.singleton_id = 1 AND membership.entity_id = ?1 \
                       AND revisions.entity_kind = 'recipe'",
                    [&request.entity_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, u32>(5)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(ObservatoryError::InvalidCatalogueRequest)?;
        let building_entity_id = connection
            .query_row(
                "SELECT value_text FROM definition_properties \
                 WHERE revision_hash = ?1 AND field_id = 'recipe.building.entity_id' \
                   AND occurrence = 0",
                [&revision_hash],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();

        let mut statement = connection.prepare(
            "SELECT relation_kind, occurrence, target_id, quantity, unit, resolution, \
                    source_directive, source_line, relations.mapping_id, \
                    relations.catalogue_scope_id, relations.mapping_classification, \
                    scope.state, scope.update_policy, scope.acknowledged_content_hash, \
                    scope.current_content_hash \
             FROM definition_relations relations \
             JOIN warehouse_metadata metadata ON metadata.singleton_id = 1 \
             LEFT JOIN catalogue_scope_evaluations scope \
               ON scope.generation_id = metadata.current_catalogue_generation_id \
              AND scope.scope_id = relations.catalogue_scope_id \
             WHERE relations.revision_hash = ?1 \
               AND relations.relation_kind IN \
                   ('production_input', 'production_output', 'waste_input') \
             ORDER BY CASE relation_kind WHEN 'production_input' THEN 0 \
                       WHEN 'waste_input' THEN 1 ELSE 2 END, occurrence \
             LIMIT ?2",
        )?;
        let mut flows = statement
            .query_map(
                params![&revision_hash, MAX_PRODUCTION_ROUTE_RELATIONS as u32],
                |row| {
                    let direction = row.get::<_, String>(0)?;
                    let occurrence = row.get::<_, u32>(1)?;
                    let resource_id = row.get::<_, String>(2)?;
                    Ok(ProductionRouteFlow {
                        id: format!("{direction}-{occurrence}"),
                        direction,
                        display_name: production_resource_name(&resource_id),
                        resource_id,
                        source_quantity: row.get(3)?,
                        scaled_quantity: None,
                        unit: row.get(4)?,
                        basis_role: String::new(),
                        basis_exclusion: None,
                        resolution: row.get(5)?,
                        source_directive: row.get(6)?,
                        source_line: row.get(7)?,
                        mapping: DefinitionMappingProvenance {
                            mapping_id: row.get(8)?,
                            catalogue_scope_id: row.get(9)?,
                            mapping_classification: row.get(10)?,
                            scope_state: parse_scope_state(row.get(11)?),
                            update_policy: row.get(12)?,
                            acknowledged_content_hash: row.get(13)?,
                            current_content_hash: row.get(14)?,
                        },
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);

        let output_count = flows
            .iter()
            .filter(|flow| flow.direction == "production_output")
            .count();
        let input_count = flows
            .iter()
            .filter(|flow| matches!(flow.direction.as_str(), "production_input" | "waste_input"))
            .count();
        let selected_output_resource_id = request.output_resource_id.clone().or_else(|| {
            flows
                .iter()
                .find(|flow| flow.direction == "production_output")
                .map(|flow| flow.resource_id.clone())
        });
        if request.output_resource_id.is_some()
            && !flows.iter().any(|flow| {
                flow.direction == "production_output"
                    && Some(flow.resource_id.as_str()) == request.output_resource_id.as_deref()
            })
        {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }

        let selected_output = selected_output_resource_id.as_deref().and_then(|selected| {
            flows
                .iter()
                .find(|flow| flow.direction == "production_output" && flow.resource_id == selected)
        });
        let selected_output_match_count = selected_output_resource_id
            .as_deref()
            .map(|selected| {
                flows
                    .iter()
                    .filter(|flow| {
                        flow.direction == "production_output" && flow.resource_id == selected
                    })
                    .count()
            })
            .unwrap_or_default();
        let primary_unit = selected_output.and_then(|flow| flow.unit.clone());
        let selected_output_quantity = selected_output.and_then(|flow| flow.source_quantity);
        for flow in &mut flows {
            if primary_unit.is_some() && flow.unit == primary_unit {
                flow.basis_role = "primary".to_owned();
            } else {
                flow.basis_role = "auxiliary".to_owned();
                flow.basis_exclusion = Some(
                    if flow.unit.is_none() {
                        "missing_unit"
                    } else {
                        "different_unit"
                    }
                    .to_owned(),
                );
            }
        }
        let primary_flows = flows
            .iter()
            .filter(|flow| flow.basis_role == "primary")
            .collect::<Vec<_>>();
        let primary_inputs = primary_flows
            .iter()
            .filter(|flow| matches!(flow.direction.as_str(), "production_input" | "waste_input"))
            .count();
        let primary_flow_count = primary_flows.len();
        let auxiliary_flow_count = flows
            .iter()
            .filter(|flow| flow.basis_role == "auxiliary")
            .count();
        let primary_endpoints = primary_flows
            .iter()
            .map(|flow| (flow.direction.clone(), flow.resource_id.clone()))
            .collect::<std::collections::BTreeSet<_>>();

        let status = if relation_count as usize > MAX_PRODUCTION_ROUTE_RELATIONS {
            "too_complex"
        } else if output_count == 0 {
            "no_output"
        } else if input_count == 0 {
            "no_input"
        } else if selected_output_match_count != 1 {
            "duplicate_endpoint"
        } else if primary_unit.is_none() {
            "missing_unit"
        } else if primary_inputs == 0 {
            "no_comparable_input"
        } else if primary_flows
            .iter()
            .any(|flow| flow.source_quantity.is_none())
        {
            "missing_quantity"
        } else if primary_flows.iter().any(|flow| {
            flow.source_quantity
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
        }) {
            "invalid_quantity"
        } else if primary_endpoints.len() != primary_flows.len() {
            "duplicate_endpoint"
        } else if auxiliary_flow_count > 0 {
            "ready_with_auxiliary"
        } else {
            "ready"
        };
        let target_quantity = request.target_quantity.or(selected_output_quantity);
        let diagrammable = matches!(status, "ready" | "ready_with_auxiliary");
        let scale_factor = if diagrammable {
            selected_output_quantity
                .zip(target_quantity)
                .map(|(source, target)| target / source)
        } else {
            None
        };
        if let Some(scale) = scale_factor {
            for flow in &mut flows {
                flow.scaled_quantity = flow
                    .source_quantity
                    .filter(|quantity| quantity.is_finite() && *quantity > 0.0)
                    .map(|quantity| quantity * scale);
            }
        }
        let mapping_classification = if flows
            .iter()
            .any(|flow| flow.mapping.mapping_classification == "player_mapped")
        {
            "player_mapped"
        } else {
            "reviewed_mapping"
        };
        let snapshot = snapshot_from(&connection)?;

        Ok(ProductionRouteModel {
            schema_version: 2,
            route_id,
            revision_hash,
            building_entity_id,
            display_name,
            package_name,
            coverage,
            status: status.to_owned(),
            relation_count,
            primary_flow_count: primary_flow_count.min(u32::MAX as usize) as u32,
            auxiliary_flow_count: auxiliary_flow_count.min(u32::MAX as usize) as u32,
            unit: primary_unit,
            selected_output_resource_id,
            target_quantity,
            scale_factor,
            mapping_classification: mapping_classification.to_owned(),
            flows,
            snapshot,
        })
    }

    pub fn production_route_coverage(&self) -> Result<ProductionRouteCoverage, ObservatoryError> {
        let connection = self.lock()?;
        let (
            route_count,
            diagrammable_count,
            routes_with_auxiliary,
            relation_count,
            auxiliary_relation_count,
            unresolved_basis_relation_count,
            unquantified_relation_count,
        ) = connection.query_row(
            "WITH route_relations AS (\
                 SELECT membership.entity_id, revisions.revision_hash, relations.relation_kind, \
                        relations.occurrence, relations.target_id, relations.quantity, \
                        relations.unit \
                 FROM catalogue_generation_entities membership \
                 JOIN warehouse_metadata metadata \
                   ON membership.generation_id = metadata.current_catalogue_generation_id \
                 JOIN definition_entity_revisions revisions USING(revision_hash) \
                 JOIN definition_relations relations USING(revision_hash) \
                 WHERE metadata.singleton_id = 1 \
                   AND revisions.entity_kind = 'recipe' \
                   AND relations.relation_kind IN \
                       ('production_input', 'production_output', 'waste_input')\
             ), selected_outputs AS (\
                 SELECT entity_id, revision_hash, target_id AS selected_target, \
                        unit AS selected_unit, quantity AS selected_quantity \
                 FROM (\
                     SELECT route_relations.*, \
                            ROW_NUMBER() OVER (PARTITION BY entity_id ORDER BY occurrence) AS rank \
                     FROM route_relations \
                     WHERE relation_kind = 'production_output'\
                 ) ranked \
                 WHERE rank = 1\
             ), route_stats AS (\
                 SELECT relations.entity_id, relations.revision_hash, outputs.selected_unit, \
                        outputs.selected_quantity, COUNT(*) AS relation_count, \
                        SUM(CASE WHEN relations.relation_kind IN \
                                      ('production_input', 'waste_input') \
                                 THEN 1 ELSE 0 END) AS input_count, \
                        SUM(CASE WHEN relations.relation_kind = 'production_output' \
                                 THEN 1 ELSE 0 END) AS output_count, \
                        SUM(CASE WHEN relations.relation_kind = 'production_output' \
                                      AND relations.target_id = outputs.selected_target \
                                 THEN 1 ELSE 0 END) AS selected_output_match_count, \
                        SUM(CASE WHEN outputs.selected_unit IS NOT NULL \
                                      AND relations.unit = outputs.selected_unit \
                                      AND relations.relation_kind IN \
                                          ('production_input', 'waste_input') \
                                 THEN 1 ELSE 0 END) AS primary_input_count, \
                        SUM(CASE WHEN outputs.selected_unit IS NOT NULL \
                                      AND relations.unit = outputs.selected_unit \
                                 THEN 1 ELSE 0 END) AS primary_relation_count, \
                        SUM(CASE WHEN outputs.selected_unit IS NOT NULL \
                                      AND relations.unit = outputs.selected_unit \
                                      AND (relations.quantity IS NULL OR relations.quantity <= 0) \
                                 THEN 1 ELSE 0 END) AS invalid_primary_count, \
                        COUNT(DISTINCT CASE WHEN outputs.selected_unit IS NOT NULL \
                                                 AND relations.unit = outputs.selected_unit \
                                            THEN relations.relation_kind || CHR(31) || relations.target_id \
                                       END) AS primary_endpoint_count, \
                        SUM(CASE WHEN outputs.selected_unit IS NOT NULL \
                                      AND relations.unit IS DISTINCT FROM outputs.selected_unit \
                                 THEN 1 ELSE 0 END) AS auxiliary_count, \
                        SUM(CASE WHEN relations.unit IS NULL THEN 1 ELSE 0 END) \
                            AS unresolved_basis_count, \
                        SUM(CASE WHEN relations.quantity IS NULL OR relations.quantity <= 0 \
                                 THEN 1 ELSE 0 END) AS unquantified_count \
                 FROM route_relations relations \
                 LEFT JOIN selected_outputs outputs USING(entity_id, revision_hash) \
                 GROUP BY relations.entity_id, relations.revision_hash, outputs.selected_target, outputs.selected_unit, \
                          outputs.selected_quantity\
             ), classified AS (\
                 SELECT *, relation_count <= ?1 \
                        AND output_count > 0 AND input_count > 0 \
                        AND selected_output_match_count = 1 \
                        AND selected_unit IS NOT NULL \
                        AND selected_quantity IS NOT NULL AND selected_quantity > 0 \
                        AND primary_input_count > 0 AND invalid_primary_count = 0 \
                        AND primary_relation_count = primary_endpoint_count AS diagrammable \
                 FROM route_stats\
             ) \
             SELECT CAST(COUNT(*) AS INTEGER), \
                    CAST(COALESCE(SUM(CASE WHEN diagrammable THEN 1 ELSE 0 END), 0) AS INTEGER), \
                    CAST(COALESCE(SUM(CASE WHEN diagrammable AND auxiliary_count > 0 \
                                           THEN 1 ELSE 0 END), 0) AS INTEGER), \
                    CAST(COALESCE(SUM(relation_count), 0) AS INTEGER), \
                    CAST(COALESCE(SUM(CASE WHEN diagrammable THEN auxiliary_count ELSE 0 END), 0) AS INTEGER), \
                    CAST(COALESCE(SUM(unresolved_basis_count), 0) AS INTEGER), \
                    CAST(COALESCE(SUM(unquantified_count), 0) AS INTEGER) \
             FROM classified",
            [MAX_PRODUCTION_ROUTE_RELATIONS as u32],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, u32>(3)?,
                    row.get::<_, u32>(4)?,
                    row.get::<_, u32>(5)?,
                    row.get::<_, u32>(6)?,
                ))
            },
        )?;
        let snapshot = snapshot_from(&connection)?;
        Ok(ProductionRouteCoverage {
            schema_version: 1,
            route_count,
            diagrammable_count,
            routes_with_auxiliary,
            unavailable_count: route_count.saturating_sub(diagrammable_count),
            relation_count,
            auxiliary_relation_count,
            unresolved_basis_relation_count,
            unquantified_relation_count,
            snapshot,
        })
    }

    pub fn dossier(&self, entity_id: &str) -> Result<DefinitionDossier, ObservatoryError> {
        if entity_id.is_empty() || entity_id.len() > 320 {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }
        let connection = self.lock()?;
        let summary = connection
            .query_row(
                "SELECT membership.entity_id, revisions.revision_hash, revisions.entity_kind, \
                        revisions.source_id, sources.source_kind, sources.package_name, \
                        revisions.display_name, revisions.coverage, \
                        (SELECT COUNT(*) FROM definition_properties properties WHERE properties.revision_hash = revisions.revision_hash), \
                        (SELECT COUNT(*) FROM definition_relations relations WHERE relations.revision_hash = revisions.revision_hash) \
                 FROM catalogue_generation_entities membership \
                 JOIN warehouse_metadata metadata ON membership.generation_id = metadata.current_catalogue_generation_id \
                 JOIN definition_entity_revisions revisions USING(revision_hash) \
                 JOIN catalogue_sources sources ON sources.generation_id = membership.generation_id \
                      AND sources.source_id = revisions.source_id \
                 WHERE metadata.singleton_id = 1 AND membership.entity_id = ?1",
                [entity_id],
                summary_from_row,
            )
            .optional()?
            .ok_or(ObservatoryError::CatalogueUnavailable)?;

        let mut facts = BTreeMap::<(String, u32), DefinitionFact>::new();
        {
            let mut statement = connection.prepare(
                "SELECT field_id, occurrence, value_kind, value_number, value_text, unit, \
                        source_directive, source_line, raw_arguments, evidence_kind, resolution, \
                        properties.mapping_id, properties.catalogue_scope_id, \
                        properties.mapping_classification, scope.state, scope.update_policy, \
                        scope.acknowledged_content_hash, scope.current_content_hash \
                 FROM definition_properties properties \
                 JOIN warehouse_metadata metadata ON metadata.singleton_id = 1 \
                 LEFT JOIN catalogue_scope_evaluations scope \
                   ON scope.generation_id = metadata.current_catalogue_generation_id \
                  AND scope.scope_id = properties.catalogue_scope_id \
                 WHERE revision_hash = ?1 ORDER BY field_id, occurrence",
            )?;
            for row in statement.query_map([&summary.revision_hash], |row| {
                let field_id = row.get::<_, String>(0)?;
                let occurrence = row.get::<_, u32>(1)?;
                let value = DefinitionValue {
                    value_kind: row.get(2)?,
                    number: row.get(3)?,
                    text: row.get(4)?,
                    unit: row.get(5)?,
                };
                Ok((
                    (field_id.clone(), occurrence),
                    DefinitionFact {
                        field_id,
                        occurrence,
                        original: Some(value.clone()),
                        override_value: None,
                        effective: Some(value),
                        source_directive: row.get(6)?,
                        source_line: row.get(7)?,
                        raw_arguments: row.get(8)?,
                        evidence_kind: row.get(9)?,
                        resolution: row.get(10)?,
                        conflict_code: None,
                        mapping: DefinitionMappingProvenance {
                            mapping_id: row.get(11)?,
                            catalogue_scope_id: row.get(12)?,
                            mapping_classification: row.get(13)?,
                            scope_state: parse_scope_state(row.get(14)?),
                            update_policy: row.get(15)?,
                            acknowledged_content_hash: row.get(16)?,
                            current_content_hash: row.get(17)?,
                        },
                    },
                ))
            })? {
                let (key, fact) = row?;
                facts.insert(key, fact);
            }
        }
        {
            let mut statement = connection.prepare(
                "SELECT operation, field_id, occurrence, value_kind, value_number, value_text, unit, \
                        reason, conflict_code FROM active_overlay_operations \
                 WHERE entity_id = ?1 ORDER BY operation_index",
            )?;
            for row in statement.query_map([entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<u32>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<f64>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                ))
            })? {
                let (operation, field, occurrence, kind, number, text, unit, reason, conflict) =
                    row?;
                let occurrence = occurrence.unwrap_or_else(|| next_occurrence(&facts, &field));
                let key = (field.clone(), occurrence);
                let override_value = kind.map(|value_kind| DefinitionValue {
                    value_kind,
                    number,
                    text,
                    unit,
                });
                let entry = facts.entry(key).or_insert_with(|| DefinitionFact {
                    field_id: field,
                    occurrence,
                    original: None,
                    override_value: None,
                    effective: None,
                    source_directive: "planning overlay".to_owned(),
                    source_line: 0,
                    raw_arguments: reason,
                    evidence_kind: "player_definition".to_owned(),
                    resolution: "overlay".to_owned(),
                    conflict_code: conflict.clone(),
                    mapping: DefinitionMappingProvenance {
                        mapping_id: "host.planning_overlay".to_owned(),
                        catalogue_scope_id: None,
                        mapping_classification: "player_override".to_owned(),
                        scope_state: None,
                        update_policy: None,
                        acknowledged_content_hash: None,
                        current_content_hash: None,
                    },
                });
                entry.override_value = override_value.clone();
                entry.conflict_code = conflict.clone();
                entry.evidence_kind = "player_override".to_owned();
                if conflict.is_none() {
                    entry.effective = if operation == "unset" {
                        None
                    } else {
                        override_value
                    };
                }
            }
        }
        let relations = {
            let mut statement = connection.prepare(
                "SELECT relation_kind, occurrence, target_id, quantity, unit, phase_id, \
                        source_directive, source_line, raw_arguments, resolution, \
                        relations.mapping_id, relations.catalogue_scope_id, \
                        relations.mapping_classification, scope.state, scope.update_policy, \
                        scope.acknowledged_content_hash, scope.current_content_hash \
                 FROM definition_relations relations \
                 JOIN warehouse_metadata metadata ON metadata.singleton_id = 1 \
                 LEFT JOIN catalogue_scope_evaluations scope \
                   ON scope.generation_id = metadata.current_catalogue_generation_id \
                  AND scope.scope_id = relations.catalogue_scope_id \
                 WHERE revision_hash = ?1 \
                 ORDER BY relation_kind, occurrence",
            )?;
            statement
                .query_map([&summary.revision_hash], |row| {
                    Ok(DefinitionRelation {
                        relation_kind: row.get(0)?,
                        occurrence: row.get(1)?,
                        target_id: row.get(2)?,
                        quantity: row.get(3)?,
                        unit: row.get(4)?,
                        phase_id: row.get(5)?,
                        source_directive: row.get(6)?,
                        source_line: row.get(7)?,
                        raw_arguments: row.get(8)?,
                        resolution: row.get(9)?,
                        mapping: DefinitionMappingProvenance {
                            mapping_id: row.get(10)?,
                            catalogue_scope_id: row.get(11)?,
                            mapping_classification: row.get(12)?,
                            scope_state: parse_scope_state(row.get(13)?),
                            update_policy: row.get(14)?,
                            acknowledged_content_hash: row.get(15)?,
                            current_content_hash: row.get(16)?,
                        },
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let unknown_directives = {
            let mut statement = connection.prepare(
                "SELECT directive, occurrence_count FROM definition_unknown_directives \
                 WHERE revision_hash = ?1 ORDER BY occurrence_count DESC, directive LIMIT 100",
            )?;
            statement
                .query_map([&summary.revision_hash], |row| {
                    Ok(UnknownDirectiveSummary {
                        directive: row.get(0)?,
                        occurrence_count: row.get(1)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(DefinitionDossier {
            summary,
            facts: facts.into_values().collect(),
            relations,
            unknown_directives,
        })
    }

    pub fn overlay_conflict_count(&self, profile_id: &str, revision: u32) -> u32 {
        self.connection
            .try_lock()
            .ok()
            .and_then(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM active_overlay_operations WHERE profile_id = ?1 \
                         AND revision = ?2 AND conflict_code IS NOT NULL",
                        params![profile_id, revision],
                        |row| row.get(0),
                    )
                    .ok()
            })
            .unwrap_or_default()
    }

    pub fn snapshot(&self) -> Result<WarehouseSnapshot, ObservatoryError> {
        let connection = self.lock()?;
        snapshot_from(&connection)
    }

    #[cfg(test)]
    pub(crate) fn test_writer_lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Connection>, ObservatoryError> {
        self.lock()
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ObservatoryError> {
        if !self.available {
            return Err(ObservatoryError::WarehouseUnavailable);
        }
        self.connection
            .lock()
            .map_err(|_| ObservatoryError::WarehouseUnavailable)
    }
}

fn snapshot_from(connection: &Connection) -> Result<WarehouseSnapshot, ObservatoryError> {
    connection
        .query_row(
            "SELECT metadata.current_catalogue_generation_id, \
                    generation.compatibility_profile_id, \
                    generation.compatibility_profile_version, \
                    generation.compatibility_profile_hash, generation.mapping_classification, \
                    metadata.active_overlay_profile_id, metadata.active_overlay_revision, \
                    metadata.observation_watermark \
             FROM warehouse_metadata metadata \
             JOIN catalogue_generations generation \
               ON generation.generation_id = metadata.current_catalogue_generation_id \
             WHERE metadata.singleton_id = 1",
            [],
            |row| {
                Ok(WarehouseSnapshot {
                    catalogue_generation_id: row.get(0)?,
                    compatibility_profile_id: row.get(1)?,
                    compatibility_profile_version: row.get(2)?,
                    compatibility_profile_hash: row.get(3)?,
                    mapping_classification: row.get(4)?,
                    overlay_profile_id: row.get(5)?,
                    overlay_revision: row.get(6)?,
                    observation_watermark: row.get(7)?,
                    warehouse_schema_version: WAREHOUSE_SCHEMA_VERSION,
                    projector_version: PROJECTOR_VERSION.to_owned(),
                })
            },
        )
        .map_err(Into::into)
}

fn production_resource_name(resource_id: &str) -> String {
    resource_id
        .strip_prefix("resource::")
        .unwrap_or(resource_id)
        .replace('_', " ")
}

fn migrate(connection: &mut Connection) -> Result<(), ObservatoryError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS warehouse_schema_migrations(\
             version INTEGER PRIMARY KEY, applied_at TIMESTAMP DEFAULT current_timestamp\
         );",
    )?;
    let version = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM warehouse_schema_migrations",
        [],
        |row| row.get::<_, u32>(0),
    )?;
    if version > WAREHOUSE_SCHEMA_VERSION {
        return Err(ObservatoryError::WarehouseUnavailable);
    }
    for (migration_version, sql) in MIGRATIONS {
        if *migration_version > version {
            let transaction = connection.transaction()?;
            transaction.execute_batch(sql)?;
            transaction.execute(
                "INSERT INTO warehouse_schema_migrations(version) VALUES(?1)",
                [migration_version],
            )?;
            transaction.commit()?;
        }
    }
    Ok(())
}

fn catalogue_generation_from(
    connection: &Connection,
) -> Result<Option<CatalogueGenerationSummary>, ObservatoryError> {
    connection
        .query_row(
            "SELECT generation.generation_id, generation.game_build_id, generation.parser_version, \
                    generation.created_at_ms, generation.source_count, generation.file_count, \
                    generation.entity_count, generation.property_count, generation.relation_count, \
                    generation.warning_count, generation.compatibility_profile_id,\
                    generation.compatibility_profile_version, generation.compatibility_profile_hash,\
                    generation.mapping_classification \
             FROM catalogue_generations generation JOIN warehouse_metadata metadata \
               ON generation.generation_id = metadata.current_catalogue_generation_id \
             WHERE metadata.singleton_id = 1",
            [],
            |row| {
                Ok(CatalogueGenerationSummary {
                    generation_id: row.get(0)?,
                    game_build_id: row.get(1)?,
                    parser_version: row.get(2)?,
                    created_at_ms: row.get(3)?,
                    source_count: row.get(4)?,
                    file_count: row.get(5)?,
                    entity_count: row.get(6)?,
                    property_count: row.get(7)?,
                    relation_count: row.get(8)?,
                    warning_count: row.get(9)?,
                    compatibility_profile_id: row.get(10)?,
                    compatibility_profile_version: row.get(11)?,
                    compatibility_profile_hash: row.get(12)?,
                    mapping_classification: row.get(13)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn catalogue_scope_statuses_from(
    connection: &Connection,
) -> Result<Vec<CompatibilityCatalogueScopeStatus>, ObservatoryError> {
    let mut statement = connection.prepare(
        "SELECT scope.scope_id, scope.source_id, scope.package_name, scope.update_policy, \
                scope.acknowledged_content_hash, scope.current_content_hash, \
                scope.mapping_count, scope.state \
         FROM catalogue_scope_evaluations scope \
         JOIN warehouse_metadata metadata \
           ON scope.generation_id = metadata.current_catalogue_generation_id \
         WHERE metadata.singleton_id = 1 ORDER BY scope.scope_id",
    )?;
    statement
        .query_map([], |row| {
            Ok(CompatibilityCatalogueScopeStatus {
                id: row.get(0)?,
                source_id: row.get(1)?,
                package_name: row.get(2)?,
                update_policy: row.get(3)?,
                acknowledged_content_hash: row.get(4)?,
                current_content_hash: row.get(5)?,
                mapping_count: row.get(6)?,
                state: parse_scope_state(row.get(7)?)
                    .unwrap_or(CompatibilityCatalogueScopeState::Conflict),
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn catalogue_runtime_from(connection: &Connection) -> Result<CatalogueRuntime, ObservatoryError> {
    connection
        .query_row(
            "SELECT last_catalogue_check_ms, last_catalogue_refresh_ms, last_catalogue_error_code \
             FROM warehouse_metadata WHERE singleton_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(Into::into)
}

fn summary_from_row(row: &duckdb::Row<'_>) -> Result<DefinitionSummary, duckdb::Error> {
    Ok(DefinitionSummary {
        entity_id: row.get(0)?,
        revision_hash: row.get(1)?,
        entity_kind: row.get(2)?,
        source_id: row.get(3)?,
        source_kind: row.get(4)?,
        package_name: row.get(5)?,
        display_name: row.get(6)?,
        coverage: row.get(7)?,
        property_count: row.get(8)?,
        relation_count: row.get(9)?,
    })
}

fn receipt_exists(connection: &Connection, projection_id: &str) -> Result<bool, ObservatoryError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projection_receipts WHERE projection_id = ?1)",
            [projection_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn record_receipt(
    transaction: &duckdb::Transaction<'_>,
    projection_id: &str,
    kind: &str,
    source_identity: &str,
    applied_at_ms: i64,
) -> Result<(), ObservatoryError> {
    transaction.execute(
        "INSERT INTO projection_receipts VALUES(?1, ?2, ?3, ?4)",
        params![projection_id, kind, source_identity, applied_at_ms],
    )?;
    Ok(())
}

fn overlay_text(value: &OverlayValue) -> Option<&str> {
    match value.kind {
        OverlayValueKind::Text => value.text.as_deref(),
        OverlayValueKind::Boolean => Some(if value.boolean == Some(true) {
            "true"
        } else {
            "false"
        }),
        OverlayValueKind::Number => None,
    }
}

fn next_occurrence(facts: &BTreeMap<(String, u32), DefinitionFact>, field_id: &str) -> u32 {
    facts
        .keys()
        .filter(|(field, _)| field == field_id)
        .map(|(_, occurrence)| occurrence.saturating_add(1))
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::definition_catalogue::{
        CatalogueFile, CatalogueSource, ParsedDefinition, ParsedProperty, ParsedRelation,
    };
    use crate::model::{CoverageReport, CoverageStatus, ReceiverHistoryPoint};
    use crate::planning_overlay::{OverlayOperation, OverlayOperationKind, OverlaySupplement};
    use tempfile::tempdir;

    fn observation_dataset(point_count: u32) -> ReceiverDataset {
        ReceiverDataset {
            payload_hash: "a".repeat(64),
            interpretation_id: "b".repeat(64),
            source_file_name: "synthetic.zip".to_owned(),
            source_file_size: 1,
            source_modified_ms: 1,
            imported_at_ms: 1,
            parser_version: "test".to_owned(),
            format_profile: "test".to_owned(),
            compatibility:
                crate::compatibility_profile::ResolvedCompatibilityProfile::reviewed_builtin()
                    .expect("profile")
                    .provenance(),
            branch_id: "main".to_owned(),
            geographic_scope: "republic".to_owned(),
            coverage: CoverageReport {
                status: CoverageStatus::Complete,
                history_records: point_count,
                chartable_records: point_count,
                dropped_records: 0,
                warnings: Vec::new(),
            },
            source_fields: Vec::new(),
            points: (0..point_count)
                .map(|record_id| ReceiverHistoryPoint {
                    record_id,
                    year: 2000 + (record_id / 365) as i32,
                    day: (record_id % 365) as u16,
                    game_day: i64::from(record_id),
                    none: u64::from(record_id) + 10,
                    radio: u64::from(record_id) + 20,
                    television: u64::from(record_id) + 30,
                    computer: u64::from(record_id) + 40,
                    classified_total: u64::from(record_id) * 4 + 100,
                })
                .collect(),
        }
    }

    #[test]
    fn creates_and_reopens_warehouse() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("test.duckdb");
        let warehouse = AnalyticalWarehouse::initialise(path.clone()).expect("create warehouse");
        assert_eq!(
            warehouse
                .health(0, 0, None, false)
                .expect("health")
                .schema_version,
            WAREHOUSE_SCHEMA_VERSION
        );
        drop(warehouse);
        AnalyticalWarehouse::initialise(path).expect("reopen warehouse");
    }

    #[test]
    fn realistic_observation_projection_completes_as_one_bounded_batch() {
        let directory = tempdir().expect("temporary directory");
        let warehouse =
            AnalyticalWarehouse::initialise(directory.path().join("observations.duckdb"))
                .expect("warehouse");
        let dataset = observation_dataset(2_000);
        let started = Instant::now();

        warehouse
            .project_observation("observation:batch", &dataset, 10)
            .expect("projection");

        assert!(
            started.elapsed() < Duration::from_secs(5),
            "realistic projection took {:?}",
            started.elapsed()
        );
        let row_count = warehouse
            .lock()
            .expect("connection")
            .query_row("SELECT COUNT(*) FROM observation_metrics", [], |row| {
                row.get::<_, u64>(0)
            })
            .expect("observation row count");
        assert_eq!(row_count, 8_000);
        warehouse
            .project_observation("observation:batch", &dataset, 11)
            .expect("idempotent retry");
    }

    #[test]
    fn status_snapshots_do_not_wait_for_an_active_warehouse_writer() {
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("busy.duckdb"))
            .expect("warehouse");
        let _writer = warehouse.lock().expect("writer lock");
        let permit = warehouse
            .governor
            .begin(WarehouseWriteKind::ObservationProjection, 8_000)
            .expect("governed write");
        permit.progress(WarehouseWriteStage::Staging, 2_000);
        let started = Instant::now();

        let health = warehouse.health_snapshot(1, 0, Some(10), false);
        let generation = warehouse.catalogue_generation_if_ready();
        let runtime = warehouse.catalogue_runtime_if_ready();

        assert!(started.elapsed() < Duration::from_millis(100));
        assert_eq!(health.phase, WarehousePhase::Lagging);
        assert_eq!(
            health
                .active_write
                .as_ref()
                .map(|activity| activity.rows_processed),
            Some(2_000)
        );
        assert!(generation.is_none());
        assert!(runtime.is_none());
        permit.complete();
    }

    #[test]
    fn upgrades_a_version_one_warehouse_independently() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("version-one.duckdb");
        {
            let connection = Connection::open(&path).expect("version one warehouse");
            connection
                .execute_batch(
                    "CREATE TABLE warehouse_schema_migrations(\
                         version INTEGER PRIMARY KEY, \
                         applied_at TIMESTAMP DEFAULT current_timestamp\
                     );",
                )
                .expect("migration ledger");
            connection
                .execute_batch(include_str!(
                    "../warehouse_migrations/0001_catalogue_and_analytics.sql"
                ))
                .expect("version one schema");
            connection
                .execute(
                    "INSERT INTO warehouse_schema_migrations VALUES(1, now())",
                    [],
                )
                .expect("version one marker");
        }

        let warehouse = AnalyticalWarehouse::initialise(path).expect("upgrade warehouse");
        let view_count = warehouse
            .lock()
            .expect("connection")
            .query_row(
                "SELECT COUNT(*) FROM duckdb_views() \
                 WHERE view_name = 'effective_value_projection'",
                [],
                |row| row.get::<_, u32>(0),
            )
            .expect("planning projection view");
        assert_eq!(view_count, 1);
    }

    #[test]
    fn refuses_a_newer_schema() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("newer.duckdb");
        {
            let connection = Connection::open(&path).expect("open database");
            connection
                .execute_batch(
                    "CREATE TABLE warehouse_schema_migrations(version INTEGER PRIMARY KEY);\
                     INSERT INTO warehouse_schema_migrations VALUES(999);",
                )
                .expect("future marker");
        }
        assert!(AnalyticalWarehouse::initialise(path).is_err());
    }

    #[test]
    fn catalogue_publication_overlay_projection_and_retries_are_idempotent() {
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("catalogue.duckdb"))
            .expect("warehouse");
        let revision_hash = "a".repeat(64);
        let entity_id = "workshop.123::building::factory".to_owned();
        let mut compatibility =
            crate::compatibility_profile::ResolvedCompatibilityProfile::reviewed_builtin()
                .expect("profile")
                .provenance();
        compatibility.mapping_classification = "player_mapped".to_owned();
        let generation = CatalogueGeneration {
            generation_id: "b".repeat(64),
            game_build_id: Some("test-build".to_owned()),
            created_at_ms: 10,
            compatibility,
            compatibility_scopes: vec![CompatibilityCatalogueScopeStatus {
                id: "local.example.factory".to_owned(),
                source_id: "workshop.123".to_owned(),
                package_name: Some("Scoped factory".to_owned()),
                update_policy: "exact".to_owned(),
                acknowledged_content_hash: "c".repeat(64),
                current_content_hash: Some("c".repeat(64)),
                mapping_count: 1,
                state: CompatibilityCatalogueScopeState::Matched,
            }],
            sources: vec![CatalogueSource {
                source_id: "workshop.123".to_owned(),
                source_kind: "workshop".to_owned(),
                package_name: "Scoped factory".to_owned(),
                package_version: None,
                content_hash: "c".repeat(64),
                file_count: 1,
            }],
            files: vec![CatalogueFile {
                source_id: "workshop.123".to_owned(),
                logical_path: "factory.ini".to_owned(),
                content_hash: "d".repeat(64),
                byte_size: 10,
                warning_count: 0,
            }],
            entities: vec![ParsedDefinition {
                entity_id: entity_id.clone(),
                revision_hash: revision_hash.clone(),
                entity_kind: "building".to_owned(),
                source_id: "workshop.123".to_owned(),
                source_object_id: "factory".to_owned(),
                display_name: "Test factory".to_owned(),
                coverage: "complete".to_owned(),
                properties: vec![ParsedProperty {
                    field_id: "building.workers.required".to_owned(),
                    occurrence: 0,
                    value_kind: "number".to_owned(),
                    value_number: Some(20.0),
                    value_text: None,
                    unit: Some("workers".to_owned()),
                    source_directive: "$WORKERS_NEEDED".to_owned(),
                    source_line: 3,
                    raw_arguments: "20".to_owned(),
                    resolution: "verified".to_owned(),
                    mapping_id: "local.example.factory.workers".to_owned(),
                    catalogue_scope_id: Some("local.example.factory".to_owned()),
                    mapping_classification: "player_mapped".to_owned(),
                }],
                relations: vec![ParsedRelation {
                    relation_kind: "construction_material_explicit".to_owned(),
                    occurrence: 0,
                    target_id: "resource::steel".to_owned(),
                    quantity: Some(12.0),
                    unit: Some("source_quantity".to_owned()),
                    phase_id: Some("groundworks:1".to_owned()),
                    source_directive: "$COST_RESOURCE".to_owned(),
                    source_line: 5,
                    raw_arguments: "steel 12".to_owned(),
                    resolution: "explicit_quantity".to_owned(),
                    mapping_id: "core.construction.material_explicit".to_owned(),
                    catalogue_scope_id: None,
                    mapping_classification: "reviewed_mapping".to_owned(),
                }],
                unknown_directives: vec![("$UNREVIEWED".to_owned(), 1)],
            }],
        };
        assert!(warehouse.publish_catalogue(&generation).expect("publish"));
        assert!(
            !warehouse
                .publish_catalogue(&generation)
                .expect("same generation")
        );
        assert_eq!(warehouse.catalogue_reuse_cache().expect("cache").len(), 1);

        let document = PlanningOverlayDocument {
            schema_version: 1,
            id: "org.example.factory-plan".to_owned(),
            version: "1.0.0".to_owned(),
            name: "Factory plan".to_owned(),
            author: "Planner".to_owned(),
            default_locale: "en-AU".to_owned(),
            description: "A local staffing assumption".to_owned(),
            target_game_build: None,
            operations: vec![OverlayOperation {
                operation: OverlayOperationKind::Set,
                entity_id: entity_id.clone(),
                field_id: "building.workers.required".to_owned(),
                occurrence: Some(0),
                expected_revision_hash: revision_hash,
                expected_value: Some(OverlayValue {
                    kind: OverlayValueKind::Number,
                    number: Some(20.0),
                    text: None,
                    boolean: None,
                    unit: Some("workers".to_owned()),
                }),
                value: Some(OverlayValue {
                    kind: OverlayValueKind::Number,
                    number: Some(25.0),
                    text: None,
                    boolean: None,
                    unit: Some("workers".to_owned()),
                }),
                reason: "Planning allowance".to_owned(),
            }],
            supplements: Vec::new(),
        };
        warehouse
            .project_overlay("overlay:test", Some((&document.id, 1, &document)), 20)
            .expect("overlay projection");
        warehouse
            .project_overlay("overlay:test", Some((&document.id, 1, &document)), 21)
            .expect("duplicate projection");
        let dossier = warehouse.dossier(&entity_id).expect("dossier");
        assert_eq!(dossier.facts.len(), 1);
        assert_eq!(
            dossier.facts[0]
                .effective
                .as_ref()
                .and_then(|value| value.number),
            Some(25.0)
        );
        assert_eq!(dossier.facts[0].evidence_kind, "player_override");
        assert_eq!(
            dossier.facts[0].mapping.mapping_id,
            "local.example.factory.workers"
        );
        assert_eq!(
            dossier.facts[0].mapping.scope_state,
            Some(CompatibilityCatalogueScopeState::Matched)
        );
        assert_eq!(
            warehouse.catalogue_scope_statuses().expect("scopes").len(),
            1
        );
        assert_eq!(
            dossier.relations[0].mapping.mapping_id,
            "core.construction.material_explicit"
        );
        let snapshot = warehouse.snapshot().expect("pinned model snapshot");
        assert_eq!(snapshot.catalogue_generation_id, generation.generation_id);
        assert_eq!(
            snapshot.overlay_profile_id.as_deref(),
            Some(document.id.as_str())
        );
        assert_eq!(snapshot.overlay_revision, Some(1));
        assert_eq!(snapshot.warehouse_schema_version, WAREHOUSE_SCHEMA_VERSION);
        assert_eq!(snapshot.projector_version, PROJECTOR_VERSION);
        let effective = warehouse
            .lock()
            .expect("connection")
            .query_row(
                "SELECT value_number, evidence_kind FROM effective_value_projection \
                 WHERE entity_id = ?1 AND field_id = 'building.workers.required'",
                [&entity_id],
                |row| Ok((row.get::<_, f64>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("effective planning projection");
        assert_eq!(effective, (25.0, "player_override".to_owned()));
    }

    #[test]
    fn production_routes_scale_definition_coefficients_and_pin_their_snapshot() {
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("routes.duckdb"))
            .expect("warehouse");
        let compatibility =
            crate::compatibility_profile::ResolvedCompatibilityProfile::reviewed_builtin()
                .expect("profile")
                .provenance();
        let mapping =
            |relation_kind: &str, occurrence: u32, resource: &str, quantity: f64, unit: &str| {
                ParsedRelation {
                    relation_kind: relation_kind.to_owned(),
                    occurrence,
                    target_id: format!("resource::{resource}"),
                    quantity: Some(quantity),
                    unit: Some(unit.to_owned()),
                    phase_id: None,
                    source_directive: if relation_kind == "production_output" {
                        "$PRODUCTION"
                    } else {
                        "$CONSUMPTION"
                    }
                    .to_owned(),
                    source_line: occurrence + 10,
                    raw_arguments: format!("{resource} {quantity}"),
                    resolution: "source_coefficient".to_owned(),
                    mapping_id: format!("core.definition.{relation_kind}"),
                    catalogue_scope_id: None,
                    mapping_classification: "reviewed_mapping".to_owned(),
                }
            };
        let recipe = |entity_id: &str, revision_hash: &str, relations| ParsedDefinition {
            entity_id: entity_id.to_owned(),
            revision_hash: revision_hash.to_owned(),
            entity_kind: "recipe".to_owned(),
            source_id: "base".to_owned(),
            source_object_id: entity_id.rsplit("::").next().unwrap_or_default().to_owned(),
            display_name: "Chemical plant route".to_owned(),
            coverage: "complete".to_owned(),
            properties: vec![ParsedProperty {
                field_id: "recipe.building.entity_id".to_owned(),
                occurrence: 0,
                value_kind: "text".to_owned(),
                value_number: None,
                value_text: Some("base::building::chemical-plant".to_owned()),
                unit: None,
                source_directive: "$TYPE_FACTORY".to_owned(),
                source_line: 1,
                raw_arguments: "chemical-plant".to_owned(),
                resolution: "derived_reference".to_owned(),
                mapping_id: "core.recipe.building".to_owned(),
                catalogue_scope_id: None,
                mapping_classification: "reviewed_mapping".to_owned(),
            }],
            relations,
            unknown_directives: Vec::new(),
        };
        let generation = CatalogueGeneration {
            generation_id: "9".repeat(64),
            game_build_id: Some("test-build".to_owned()),
            created_at_ms: 10,
            compatibility,
            compatibility_scopes: Vec::new(),
            sources: vec![CatalogueSource {
                source_id: "base".to_owned(),
                source_kind: "base".to_owned(),
                package_name: "Workers & Resources".to_owned(),
                package_version: Some("test".to_owned()),
                content_hash: "8".repeat(64),
                file_count: 1,
            }],
            files: vec![CatalogueFile {
                source_id: "base".to_owned(),
                logical_path: "buildings.ini".to_owned(),
                content_hash: "7".repeat(64),
                byte_size: 100,
                warning_count: 0,
            }],
            entities: vec![
                recipe(
                    "base::recipe::chemical-plant",
                    &"6".repeat(64),
                    vec![
                        mapping("production_input", 0, "oil", 2.0, "source_rate"),
                        mapping("production_input", 1, "power", 1.0, "source_rate"),
                        mapping("production_output", 0, "chemicals", 0.5, "source_rate"),
                    ],
                ),
                recipe(
                    "base::recipe::mixed-units",
                    &"5".repeat(64),
                    vec![
                        mapping("production_input", 0, "oil", 2.0, "source_rate"),
                        mapping("production_input", 1, "eletric", 0.01, "per_second"),
                        mapping("production_output", 0, "fuel", 1.0, "source_rate"),
                    ],
                ),
                recipe(
                    "base::recipe::no-comparable-input",
                    &"4".repeat(64),
                    vec![
                        mapping("production_input", 0, "oil", 2.0, "source_rate"),
                        mapping("production_output", 0, "fuel", 1.0, "per_second"),
                    ],
                ),
            ],
        };
        assert!(warehouse.publish_catalogue(&generation).expect("publish"));

        let route = warehouse
            .production_route(&ProductionRouteRequest {
                entity_id: "base::recipe::chemical-plant".to_owned(),
                output_resource_id: Some("resource::chemicals".to_owned()),
                target_quantity: Some(10.0),
            })
            .expect("production route");
        assert_eq!(route.status, "ready");
        assert_eq!(route.unit.as_deref(), Some("source_rate"));
        assert_eq!(route.scale_factor, Some(20.0));
        assert_eq!(route.target_quantity, Some(10.0));
        assert_eq!(route.mapping_classification, "reviewed_mapping");
        assert_eq!(
            route.snapshot.catalogue_generation_id,
            generation.generation_id
        );
        assert_eq!(
            route.snapshot.warehouse_schema_version,
            WAREHOUSE_SCHEMA_VERSION
        );
        assert_eq!(
            route
                .flows
                .iter()
                .find(|flow| flow.resource_id == "resource::oil")
                .and_then(|flow| flow.scaled_quantity),
            Some(40.0)
        );
        assert_eq!(
            route
                .flows
                .iter()
                .find(|flow| flow.resource_id == "resource::chemicals")
                .and_then(|flow| flow.scaled_quantity),
            Some(10.0)
        );

        let mixed = warehouse
            .production_route(&ProductionRouteRequest {
                entity_id: "base::recipe::mixed-units".to_owned(),
                output_resource_id: None,
                target_quantity: None,
            })
            .expect("mixed-unit route");
        assert_eq!(mixed.status, "ready_with_auxiliary");
        assert_eq!(mixed.unit.as_deref(), Some("source_rate"));
        assert_eq!(mixed.primary_flow_count, 2);
        assert_eq!(mixed.auxiliary_flow_count, 1);
        let electricity = mixed
            .flows
            .iter()
            .find(|flow| flow.resource_id == "resource::eletric")
            .expect("auxiliary electricity requirement");
        assert_eq!(electricity.basis_role, "auxiliary");
        assert_eq!(
            electricity.basis_exclusion.as_deref(),
            Some("different_unit")
        );
        assert_eq!(electricity.scaled_quantity, Some(0.01));

        let no_comparable = warehouse
            .production_route(&ProductionRouteRequest {
                entity_id: "base::recipe::no-comparable-input".to_owned(),
                output_resource_id: None,
                target_quantity: None,
            })
            .expect("route without a comparable input");
        assert_eq!(no_comparable.status, "no_comparable_input");
        assert_eq!(no_comparable.scale_factor, None);

        let coverage = warehouse
            .production_route_coverage()
            .expect("route coverage");
        assert_eq!(coverage.route_count, 3);
        assert_eq!(coverage.diagrammable_count, 2);
        assert_eq!(coverage.routes_with_auxiliary, 1);
        assert_eq!(coverage.unavailable_count, 1);
        assert_eq!(coverage.relation_count, 8);
        assert_eq!(coverage.auxiliary_relation_count, 1);

        assert!(
            warehouse
                .production_route(&ProductionRouteRequest {
                    entity_id: "base::recipe::chemical-plant".to_owned(),
                    output_resource_id: Some("resource::steel".to_owned()),
                    target_quantity: Some(1.0),
                })
                .is_err()
        );
    }

    #[test]
    fn maximum_overlay_projection_uses_one_bounded_bulk_merge() {
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("overlay.duckdb"))
            .expect("warehouse");
        let document = PlanningOverlayDocument {
            schema_version: 1,
            id: "org.example.maximum-overlay".to_owned(),
            version: "1.0.0".to_owned(),
            name: "Maximum overlay".to_owned(),
            author: "Planner".to_owned(),
            default_locale: "en-AU".to_owned(),
            description: "Bulk projection regression fixture".to_owned(),
            target_game_build: None,
            operations: (0..crate::planning_overlay::MAX_OPERATIONS)
                .map(|index| OverlayOperation {
                    operation: OverlayOperationKind::Set,
                    entity_id: format!("base::building::missing-{index}"),
                    field_id: "building.workers.required".to_owned(),
                    occurrence: Some(0),
                    expected_revision_hash: "a".repeat(64),
                    expected_value: None,
                    value: Some(OverlayValue {
                        kind: OverlayValueKind::Number,
                        number: Some(index as f64),
                        text: None,
                        boolean: None,
                        unit: Some("workers".to_owned()),
                    }),
                    reason: "Maximum-size bulk regression".to_owned(),
                })
                .collect(),
            supplements: (0..crate::planning_overlay::MAX_SUPPLEMENTS)
                .map(|index| OverlaySupplement {
                    local_id: format!("supplement-{index}"),
                    entity_kind: "building".to_owned(),
                    display_name: format!("Supplement {index}"),
                    reason: "Maximum-size bulk regression".to_owned(),
                    properties: Vec::new(),
                })
                .collect(),
        };
        let started = Instant::now();

        warehouse
            .project_overlay("overlay:maximum", Some((&document.id, 1, &document)), 20)
            .expect("maximum overlay projection");

        assert!(
            started.elapsed() < Duration::from_secs(5),
            "maximum overlay projection took {:?}",
            started.elapsed()
        );
        let connection = warehouse.lock().expect("connection");
        let (operation_count, conflict_count, supplement_count) = connection
            .query_row(
                "SELECT COUNT(*), COUNT(*) FILTER (WHERE conflict_code = 'target_missing'), \
                        (SELECT COUNT(*) FROM active_overlay_entities) \
                 FROM active_overlay_operations",
                [],
                |row| {
                    Ok((
                        row.get::<_, u64>(0)?,
                        row.get::<_, u64>(1)?,
                        row.get::<_, u64>(2)?,
                    ))
                },
            )
            .expect("bulk overlay counts");
        assert_eq!(
            operation_count,
            crate::planning_overlay::MAX_OPERATIONS as u64
        );
        assert_eq!(conflict_count, operation_count);
        assert_eq!(
            supplement_count,
            crate::planning_overlay::MAX_SUPPLEMENTS as u64
        );
    }

    #[test]
    #[ignore = "requires a private local W&R installation and is a reference-machine benchmark"]
    fn presently_installed_catalogue_publishes_in_bounded_batches() {
        let media = std::env::var_os("RO_GAME_MEDIA").expect("set RO_GAME_MEDIA privately");
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("local.duckdb"))
            .expect("warehouse");
        let started = Instant::now();
        let generation = crate::definition_catalogue::discover_catalogue_with_reuse(
            std::path::Path::new(&media),
            None,
            1,
            &std::collections::HashMap::new(),
        )
        .expect("local catalogue");
        let mut latest = None;
        assert!(
            warehouse
                .publish_catalogue_with_progress(&generation, |progress| latest = Some(progress))
                .expect("publish")
        );
        let latest = latest.expect("publication progress");
        eprintln!(
            "catalogue files={} entities={} rows={} elapsed={:?}",
            generation.files.len(),
            generation.entities.len(),
            latest.rows_written,
            started.elapsed()
        );
        assert_eq!(latest.rows_written, latest.rows_total);
        assert!(started.elapsed() < Duration::from_secs(30));
    }

    #[test]
    #[ignore = "reference-machine scale and growth benchmark"]
    fn synthetic_catalogue_and_observation_scale_targets() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("scale.duckdb");
        let warehouse = AnalyticalWarehouse::initialise(path.clone()).expect("warehouse");
        let started = Instant::now();
        {
            let connection = warehouse.lock().expect("connection");
            connection
                .execute_batch(
                    r#"
                    INSERT INTO catalogue_generations
                    VALUES('scale', NULL, 'scale.v1', 1, 1, 100000, 100000, 2000000, 200000, 0);
                    INSERT INTO catalogue_sources
                    VALUES('scale', 'synthetic', 'test', 'Synthetic', NULL, 'hash', 100000);
                    INSERT INTO definition_entity_revisions
                    SELECT 'revision-' || range::VARCHAR,
                           CASE WHEN range % 5 = 0 THEN 'vehicle' ELSE 'building' END,
                           'synthetic',
                           range::VARCHAR, 'Entity ' || range::VARCHAR, 'complete'
                    FROM range(100000);
                    INSERT INTO catalogue_generation_entities
                    SELECT 'scale', 'synthetic::building::' || range::VARCHAR,
                           'revision-' || range::VARCHAR
                    FROM range(100000);
                    INSERT INTO definition_properties
                    SELECT 'revision-' || (range % 100000)::VARCHAR,
                           CASE WHEN range % 100000 % 5 = 0
                                THEN 'vehicle.speed.maximum'
                                ELSE 'building.synthetic.field' END,
                           floor(range / 100000.0)::BIGINT,
                           'number', range::DOUBLE, NULL, 'unit', '$SYNTHETIC', 1,
                           'bounded', 'game_definition', 'synthetic'
                    FROM range(2000000);
                    INSERT INTO definition_relations
                    SELECT 'revision-' || range::VARCHAR, 'production_input', 0,
                           'resource::steel', 1.0, 't/day', NULL, '$CONSUMPTION', 1,
                           'steel 1', 'verified_time_basis'
                    FROM range(100000)
                    UNION ALL
                    SELECT 'revision-' || range::VARCHAR, 'construction_material_explicit', 0,
                           'resource::concrete', 10.0, 't', 'groundworks:1',
                           '$COST_RESOURCE', 2, 'concrete 10', 'explicit_quantity'
                    FROM range(100000);
                    UPDATE warehouse_metadata
                    SET current_catalogue_generation_id = 'scale'
                    WHERE singleton_id = 1;
                    INSERT INTO observation_metrics
                    SELECT 'payload-' || floor(range / 1000.0)::BIGINT::VARCHAR,
                           'main', range % 1000,
                           2000, range % 365, range, 'core.synthetic.metric', range % 10000
                    FROM range(5000000);
                    "#,
                )
                .expect("synthetic load");
        }
        assert!(started.elapsed() < Duration::from_secs(90));
        let connection = warehouse.lock().expect("connection");
        let query_cases = [
            (
                "filtered aggregate",
                "SELECT SUM(value)::DOUBLE FROM observation_metrics \
                 WHERE game_day BETWEEN 100000 AND 200000",
            ),
            (
                "material demand",
                "SELECT SUM(quantity)::DOUBLE FROM construction_demand \
                 WHERE target_id = 'resource::concrete'",
            ),
            (
                "production chain",
                "SELECT SUM(quantity)::DOUBLE FROM material_flows \
                 WHERE target_id = 'resource::steel'",
            ),
            (
                "fleet capability",
                "SELECT AVG(value_number) FROM fleet_capabilities \
                 WHERE field_id = 'vehicle.speed.maximum'",
            ),
        ];
        let mut query_measurements = Vec::new();
        for (name, query) in query_cases {
            let query_started = Instant::now();
            let total = connection
                .query_row(query, [], |row| row.get::<_, f64>(0))
                .expect(name);
            let elapsed = query_started.elapsed();
            assert!(total > 0.0);
            assert!(elapsed < Duration::from_millis(500), "{name}: {elapsed:?}");
            query_measurements.push((name, elapsed));
        }
        drop(connection);
        let database_size = fs::metadata(path).expect("database size").len();
        eprintln!(
            "synthetic warehouse facts=2200000 observations=5000000 size_bytes={database_size} load={:?} queries={query_measurements:?}",
            started.elapsed(),
        );
        assert!(database_size > 0);
    }
}
