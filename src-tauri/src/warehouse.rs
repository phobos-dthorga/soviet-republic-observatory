use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use duckdb::{Connection, OptionalExt, params};

use crate::definition_catalogue::{
    CatalogueGeneration, CatalogueReuseEntry, DEFINITION_PARSER_VERSION,
};
use crate::error::ObservatoryError;
use crate::model::{
    BranchMembershipProjection, BroadcastStationRequirement, BroadcastWarehouseProjection,
    CatalogueGenerationSummary, CataloguePage, CatalogueSearchFilter,
    CompatibilityCatalogueScopeState, CompatibilityCatalogueScopeStatus, DefinitionDossier,
    DefinitionFact, DefinitionMappingProvenance, DefinitionRelation, DefinitionSummary,
    DefinitionValue, EnvironmentDefinitionContext, EnvironmentWarehouseProjection,
    MarketPriceSeriesPoint, MarketPriceVolatility, MarketTradePoint, MarketWarehousePriceFact,
    MarketWarehouseProjection, MarketWarehouseRecord, MarketWarehouseScalarFact,
    MarketWarehouseTradeFact, ProductionPathwayAuxiliaryRequirement, ProductionPathwayCandidate,
    ProductionPathwayChoice, ProductionPathwayDiagnostic, ProductionPathwayLink,
    ProductionPathwayModel, ProductionPathwayNode, ProductionPathwayRequest,
    ProductionPathwayRequirement, ProductionRouteCoverage, ProductionRouteFlow,
    ProductionRouteModel, ProductionRouteRequest, ReceiverDataset, UnknownDirectiveSummary,
    WarehouseHealth, WarehousePhase, WarehouseSnapshot, WarehouseWriteKind, WarehouseWriteStage,
};
use crate::planning_overlay::{
    OverlayOperationKind, OverlayValue, OverlayValueKind, PlanningOverlayDocument,
};
use crate::storage::StoredLiveResources;
use crate::warehouse_governor::{
    WarehouseGovernor, WarehouseGovernorSnapshot, WarehouseWritePermit,
};

pub const WAREHOUSE_SCHEMA_VERSION: u32 = 10;
pub const PROJECTOR_VERSION: &str = "republic-observatory-projector.v3";
const MAX_PRODUCTION_ROUTE_RELATIONS: usize = 63;
const MAX_PRODUCTION_PATHWAY_DEPTH: u32 = 6;
const MAX_PRODUCTION_PATHWAY_SELECTIONS: usize = 32;
const MAX_PRODUCTION_PATHWAY_CANDIDATES: usize = 16;
const MAX_PRODUCTION_PATHWAY_NODES: usize = 128;
const MAX_PRODUCTION_PATHWAY_LINKS: usize = 256;
pub type CatalogueRuntime = (Option<i64>, Option<i64>, Option<String>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InstalledResourceEvidence {
    pub source_token: String,
    pub display_name: String,
    pub installed_reference_count: u32,
    pub installed_sources: Vec<String>,
    pub player_overlay: bool,
}

#[derive(Clone, Debug, Default)]
struct WarehouseStatusCache {
    catalogue_generation: Option<CatalogueGenerationSummary>,
    catalogue_scopes: Vec<CompatibilityCatalogueScopeStatus>,
    catalogue_runtime: CatalogueRuntime,
    last_projected_at_ms: Option<i64>,
    observation_watermark: Option<String>,
}

struct PathwayBuild {
    snapshot: WarehouseSnapshot,
    max_depth: u32,
    selections: BTreeMap<String, String>,
    used_selections: BTreeSet<String>,
    nodes: Vec<ProductionPathwayNode>,
    links: Vec<ProductionPathwayLink>,
    choices: Vec<ProductionPathwayChoice>,
    terminal_requirements: BTreeMap<(String, String, String), (String, f64)>,
    auxiliary_requirements: Vec<ProductionPathwayAuxiliaryRequirement>,
    diagnostics: Vec<ProductionPathwayDiagnostic>,
    next_node: u32,
    next_link: u32,
    player_mapped: bool,
}

impl PathwayBuild {
    fn terminal(
        &mut self,
        resource_id: &str,
        display_name: &str,
        quantity: f64,
        unit: &str,
        reason: &str,
    ) {
        let entry = self
            .terminal_requirements
            .entry((resource_id.to_owned(), unit.to_owned(), reason.to_owned()))
            .or_insert_with(|| (display_name.to_owned(), 0.0));
        entry.1 += quantity;
    }

    fn diagnostic(
        &mut self,
        code: &str,
        resource_id: Option<&str>,
        recipe_entity_id: Option<&str>,
        depth: u32,
    ) {
        self.diagnostics.push(ProductionPathwayDiagnostic {
            code: code.to_owned(),
            resource_id: resource_id.map(str::to_owned),
            recipe_entity_id: recipe_entity_id.map(str::to_owned),
            depth,
        });
    }
}

impl WarehouseStatusCache {
    fn load(connection: &Connection) -> Result<Self, ObservatoryError> {
        let (last_projected_at_ms, observation_watermark) = connection.query_row(
            "SELECT last_projection_ms, observation_watermark FROM warehouse_metadata \
             WHERE singleton_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(Self {
            catalogue_generation: catalogue_generation_from(connection)?,
            catalogue_scopes: catalogue_scope_statuses_from(connection)?,
            catalogue_runtime: catalogue_runtime_from(connection)?,
            last_projected_at_ms,
            observation_watermark,
        })
    }
}

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
    (
        5,
        include_str!("../warehouse_migrations/0005_branch_memberships.sql"),
    ),
    (
        6,
        include_str!("../warehouse_migrations/0006_market_analytics.sql"),
    ),
    (
        7,
        include_str!("../warehouse_migrations/0007_normalised_market_records.sql"),
    ),
    (
        8,
        include_str!("../warehouse_migrations/0008_broadcast_status.sql"),
    ),
    (
        9,
        include_str!("../warehouse_migrations/0009_resource_registry.sql"),
    ),
    (
        10,
        include_str!("../warehouse_migrations/0010_environment_activity.sql"),
    ),
];

pub struct AnalyticalWarehouse {
    database_path: PathBuf,
    connection: Mutex<Connection>,
    status_cache: Mutex<WarehouseStatusCache>,
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

    pub fn environment_definition_context(
        &self,
    ) -> Result<EnvironmentDefinitionContext, ObservatoryError> {
        if !self.available {
            return Ok(EnvironmentDefinitionContext::default());
        }
        let connection = self.lock()?;
        connection
            .query_row(
                r#"SELECT COUNT(DISTINCT revisions.revision_hash),
                          COUNT(*) FILTER (WHERE properties.field_id = 'building.environment.pollution_class'),
                          COUNT(*) FILTER (WHERE properties.field_id = 'building.environment.sewage_pollution_factor'),
                          COUNT(*) FILTER (WHERE properties.field_id = 'building.environment.water_required_quality'),
                          COUNT(*) FILTER (WHERE properties.field_id IN (
                              'building.environment.water_industry_substation_disabled',
                              'building.environment.production_sewage_disabled',
                              'building.environment.sewage_disabled'))
                   FROM catalogue_generation_entities membership
                   JOIN warehouse_metadata metadata
                     ON membership.generation_id = metadata.current_catalogue_generation_id
                   JOIN definition_entity_revisions revisions USING(revision_hash)
                   JOIN definition_properties properties USING(revision_hash)
                   WHERE metadata.singleton_id = 1
                     AND revisions.entity_kind = 'building'
                     AND properties.field_id LIKE 'building.environment.%'"#,
                [],
                |row| {
                    let building_count = row.get::<_, u32>(0)?;
                    Ok(EnvironmentDefinitionContext {
                        available: building_count > 0,
                        building_count,
                        pollution_class_facts: row.get(1)?,
                        sewage_pollution_factors: row.get(2)?,
                        water_quality_facts: row.get(3)?,
                        connection_capability_facts: row.get(4)?,
                    })
                },
            )
            .map_err(Into::into)
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
        let status_cache = WarehouseStatusCache::load(&connection)?;
        Ok(Self {
            database_path,
            connection: Mutex::new(connection),
            status_cache: Mutex::new(status_cache),
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
            status_cache: Mutex::new(WarehouseStatusCache::default()),
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

    pub fn project_market_observation(
        &self,
        projection_id: &str,
        projection: &MarketWarehouseProjection,
        applied_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.lock()?;
        if receipt_exists(&connection, projection_id)? {
            self.governor.note_success();
            return Ok(());
        }
        let rows_total = projection.row_count();
        let permit = self
            .governor
            .begin(WarehouseWriteKind::MarketProjection, rows_total)?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(
            "CREATE OR REPLACE TEMP TABLE incoming_market_records AS \
                 SELECT * FROM market_records WHERE FALSE; \
             CREATE OR REPLACE TEMP TABLE incoming_market_price_facts AS \
                 SELECT * FROM market_price_facts WHERE FALSE; \
             CREATE OR REPLACE TEMP TABLE incoming_market_trade_facts AS \
                 SELECT * FROM market_trade_facts WHERE FALSE; \
             CREATE OR REPLACE TEMP TABLE incoming_market_scalar_facts AS \
                 SELECT * FROM market_scalar_facts WHERE FALSE;",
        )?;
        let mut rows_written = 0_u64;
        {
            let mut appender = transaction.appender("market_observation_records")?;
            for record in &projection.records {
                appender.append_row(params![
                    projection.interpretation_id,
                    projection.raw_payload_hash,
                    projection.branch_id,
                    record.record_hash,
                    record.ordinal,
                    projection.profile_id,
                    projection.profile_version,
                    projection.resolved_profile_hash,
                    projection.mapping_classification,
                ])?;
                rows_written += 1;
                if rows_written.is_multiple_of(512) {
                    permit.progress(WarehouseWriteStage::Staging, rows_written);
                }
            }
        }
        {
            let mut appender = transaction.appender("incoming_market_records")?;
            for record in &projection.records {
                appender.append_row(params![
                    record.record_hash,
                    record.record_id,
                    record.year,
                    record.day,
                    record.game_day,
                ])?;
            }
        }
        {
            let mut history = transaction.appender("incoming_market_price_facts")?;
            let mut snapshots = transaction.appender("market_snapshot_price_facts")?;
            for fact in &projection.prices {
                if let Some(record_hash) = fact.record_hash.as_deref() {
                    history.append_row(params![
                        record_hash,
                        fact.currency,
                        fact.price_side,
                        fact.resource_token,
                        fact.value,
                        fact.modifier,
                        fact.source_field,
                        fact.source_line,
                        fact.mapping_id,
                    ])?;
                } else {
                    snapshots.append_row(params![
                        projection.interpretation_id,
                        fact.scope_kind,
                        fact.scope_id,
                        fact.currency,
                        fact.price_side,
                        fact.resource_token,
                        fact.value,
                        fact.modifier,
                        fact.source_field,
                        fact.source_line,
                        fact.mapping_id,
                    ])?;
                }
                rows_written = rows_written.saturating_add(1);
            }
        }
        {
            let mut history = transaction.appender("incoming_market_trade_facts")?;
            let mut snapshots = transaction.appender("market_snapshot_trade_facts")?;
            for fact in &projection.trades {
                if let Some(record_hash) = fact.record_hash.as_deref() {
                    history.append_row(params![
                        record_hash,
                        fact.currency,
                        fact.direction,
                        fact.channel,
                        fact.resource_token,
                        fact.quantity,
                        fact.account_value,
                        fact.source_field,
                        fact.source_line,
                        fact.mapping_id,
                    ])?;
                } else {
                    snapshots.append_row(params![
                        projection.interpretation_id,
                        fact.scope_kind,
                        fact.scope_id,
                        fact.currency,
                        fact.direction,
                        fact.channel,
                        fact.resource_token,
                        fact.quantity,
                        fact.account_value,
                        fact.source_field,
                        fact.source_line,
                        fact.mapping_id,
                    ])?;
                }
                rows_written = rows_written.saturating_add(1);
            }
        }
        {
            let mut history = transaction.appender("incoming_market_scalar_facts")?;
            let mut snapshots = transaction.appender("market_snapshot_scalar_facts")?;
            for fact in &projection.scalars {
                if let Some(record_hash) = fact.record_hash.as_deref() {
                    history.append_row(params![
                        record_hash,
                        fact.fact_id,
                        fact.currency,
                        fact.category,
                        fact.value,
                        fact.source_field,
                        fact.source_line,
                        fact.mapping_id,
                    ])?;
                } else {
                    snapshots.append_row(params![
                        projection.interpretation_id,
                        fact.scope_kind,
                        fact.scope_id,
                        fact.fact_id,
                        fact.currency,
                        fact.category,
                        fact.value,
                        fact.source_field,
                        fact.source_line,
                        fact.mapping_id,
                    ])?;
                }
                rows_written = rows_written.saturating_add(1);
            }
        }
        permit.progress(WarehouseWriteStage::Merging, rows_total);
        transaction.execute_batch(
            "CREATE OR REPLACE TEMP TABLE new_market_records AS \
                 SELECT DISTINCT incoming.* FROM incoming_market_records incoming \
                 WHERE NOT EXISTS (SELECT 1 FROM market_records stored \
                                   WHERE stored.record_hash = incoming.record_hash); \
             INSERT INTO market_records SELECT * FROM new_market_records; \
             INSERT INTO market_price_facts \
                 SELECT DISTINCT fact.* FROM incoming_market_price_facts fact \
                 JOIN new_market_records record USING(record_hash); \
             INSERT INTO market_trade_facts \
                 SELECT DISTINCT fact.* FROM incoming_market_trade_facts fact \
                 JOIN new_market_records record USING(record_hash); \
             INSERT INTO market_scalar_facts \
                 SELECT DISTINCT fact.* FROM incoming_market_scalar_facts fact \
                 JOIN new_market_records record USING(record_hash);",
        )?;
        record_receipt(
            &transaction,
            projection_id,
            "market_observation",
            &projection.interpretation_id,
            applied_at_ms,
        )?;
        transaction.execute(
            "UPDATE warehouse_metadata SET last_projection_ms = ?1, observation_watermark = ?2 \
             WHERE singleton_id = 1",
            params![applied_at_ms, projection.interpretation_id],
        )?;
        permit.progress(WarehouseWriteStage::Committing, rows_total);
        transaction.commit()?;
        permit.complete();
        Ok(())
    }

    pub fn project_broadcast_observation(
        &self,
        projection_id: &str,
        projection: &BroadcastWarehouseProjection,
        applied_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.lock()?;
        if receipt_exists(&connection, projection_id)? {
            self.governor.note_success();
            return Ok(());
        }
        let rows_total = projection.row_count();
        let permit = self
            .governor
            .begin(WarehouseWriteKind::BroadcastProjection, rows_total)?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(
            "CREATE OR REPLACE TEMP TABLE incoming_broadcast_status_records AS \
                 SELECT * FROM broadcast_status_records WHERE FALSE; \
             CREATE OR REPLACE TEMP TABLE incoming_broadcast_status_facts AS \
                 SELECT * FROM broadcast_status_facts WHERE FALSE;",
        )?;
        let mut rows_written = 0_u64;
        {
            let mut membership = transaction.appender("broadcast_status_observation_records")?;
            let mut records = transaction.appender("incoming_broadcast_status_records")?;
            for record in &projection.records {
                membership.append_row(params![
                    projection.interpretation_id,
                    projection.raw_payload_hash,
                    projection.branch_id,
                    record.record_hash,
                    record.ordinal,
                    projection.profile_id,
                    projection.profile_version,
                    projection.resolved_profile_hash,
                    projection.mapping_classification,
                ])?;
                records.append_row(params![
                    record.record_hash,
                    record.record_id,
                    record.year,
                    record.day,
                    record.game_day,
                ])?;
                rows_written = rows_written.saturating_add(1);
            }
        }
        {
            let mut facts = transaction.appender("incoming_broadcast_status_facts")?;
            for fact in &projection.facts {
                facts.append_row(params![
                    fact.record_hash,
                    fact.source_index,
                    fact.metric_id,
                    fact.value,
                    fact.source_field,
                    fact.source_line,
                    fact.mapping_id,
                ])?;
                rows_written = rows_written.saturating_add(1);
                if rows_written == rows_total || rows_written.is_multiple_of(512) {
                    permit.progress(WarehouseWriteStage::Staging, rows_written);
                }
            }
        }
        permit.progress(WarehouseWriteStage::Merging, rows_total);
        transaction.execute_batch(
            "CREATE OR REPLACE TEMP TABLE new_broadcast_status_records AS \
                 SELECT DISTINCT incoming.* FROM incoming_broadcast_status_records incoming \
                 WHERE NOT EXISTS (SELECT 1 FROM broadcast_status_records stored \
                                   WHERE stored.record_hash = incoming.record_hash); \
             INSERT INTO broadcast_status_records \
                 SELECT * FROM new_broadcast_status_records; \
             INSERT INTO broadcast_status_facts \
                 SELECT DISTINCT fact.* FROM incoming_broadcast_status_facts fact \
                 JOIN new_broadcast_status_records record USING(record_hash);",
        )?;
        record_receipt(
            &transaction,
            projection_id,
            "broadcast_observation",
            &projection.interpretation_id,
            applied_at_ms,
        )?;
        transaction.execute(
            "UPDATE warehouse_metadata SET last_projection_ms = ?1, observation_watermark = ?2 \
             WHERE singleton_id = 1",
            params![applied_at_ms, projection.interpretation_id],
        )?;
        permit.progress(WarehouseWriteStage::Committing, rows_total);
        transaction.commit()?;
        permit.complete();
        Ok(())
    }

    pub fn project_environment_observation(
        &self,
        projection_id: &str,
        projection: &EnvironmentWarehouseProjection,
        applied_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.lock()?;
        if receipt_exists(&connection, projection_id)? {
            self.governor.note_success();
            return Ok(());
        }
        let rows_total = projection.row_count();
        let permit = self
            .governor
            .begin(WarehouseWriteKind::EnvironmentProjection, rows_total)?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(
            "CREATE OR REPLACE TEMP TABLE incoming_environment_activity_records AS \
                 SELECT * FROM environment_activity_records WHERE FALSE; \
             CREATE OR REPLACE TEMP TABLE incoming_environment_activity_facts AS \
                 SELECT * FROM environment_activity_facts WHERE FALSE;",
        )?;
        let mut rows_written = 0_u64;
        {
            let mut membership =
                transaction.appender("environment_activity_observation_records")?;
            let mut records = transaction.appender("incoming_environment_activity_records")?;
            for record in &projection.records {
                membership.append_row(params![
                    projection.interpretation_id,
                    projection.raw_payload_hash,
                    projection.branch_id,
                    record.record_hash,
                    record.ordinal,
                    projection.profile_id,
                    projection.profile_version,
                    projection.resolved_profile_hash,
                    projection.mapping_classification,
                ])?;
                records.append_row(params![
                    record.record_hash,
                    record.record_id,
                    record.year,
                    record.day,
                    record.game_day,
                ])?;
                rows_written = rows_written.saturating_add(1);
            }
        }
        {
            let mut facts = transaction.appender("incoming_environment_activity_facts")?;
            for fact in &projection.facts {
                facts.append_row(params![
                    fact.record_hash,
                    fact.source_field,
                    fact.source_line,
                    fact.row_ordinal,
                    fact.resource_token,
                    fact.activity_channel,
                    fact.primary_value,
                    fact.secondary_value,
                    fact.quantity_is_publishable,
                    fact.mapping_id,
                ])?;
                rows_written = rows_written.saturating_add(1);
                if rows_written == rows_total || rows_written.is_multiple_of(512) {
                    permit.progress(WarehouseWriteStage::Staging, rows_written);
                }
            }
        }
        permit.progress(WarehouseWriteStage::Merging, rows_total);
        transaction.execute_batch(
            "CREATE OR REPLACE TEMP TABLE new_environment_activity_records AS \
                 SELECT DISTINCT incoming.* FROM incoming_environment_activity_records incoming \
                 WHERE NOT EXISTS (SELECT 1 FROM environment_activity_records stored \
                                   WHERE stored.record_hash = incoming.record_hash); \
             INSERT INTO environment_activity_records \
                 SELECT * FROM new_environment_activity_records; \
             INSERT INTO environment_activity_facts \
                 SELECT DISTINCT fact.* FROM incoming_environment_activity_facts fact \
                 JOIN new_environment_activity_records record USING(record_hash);",
        )?;
        record_receipt(
            &transaction,
            projection_id,
            "environment_observation",
            &projection.interpretation_id,
            applied_at_ms,
        )?;
        transaction.execute(
            "UPDATE warehouse_metadata SET last_projection_ms = ?1, observation_watermark = ?2 \
             WHERE singleton_id = 1",
            params![applied_at_ms, projection.interpretation_id],
        )?;
        permit.progress(WarehouseWriteStage::Committing, rows_total);
        transaction.commit()?;
        permit.complete();
        Ok(())
    }

    pub fn project_resource_registry(
        &self,
        projection_id: &str,
        resources: &StoredLiveResources,
        applied_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.lock()?;
        if receipt_exists(&connection, projection_id)? {
            self.governor.note_success();
            return Ok(());
        }
        let rows_total = 1_u64.saturating_add(
            resources
                .entries
                .iter()
                .map(|entry| 1_u64.saturating_add(entry.live_prices.len() as u64))
                .sum::<u64>(),
        );
        let permit = self
            .governor
            .begin(WarehouseWriteKind::ResourceRegistryProjection, rows_total)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO resource_registry_snapshots VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
             ON CONFLICT DO NOTHING",
            params![
                resources.summary.snapshot_id,
                resources.summary.assurance.as_str(),
                resources.summary.game_build_id,
                resources.summary.probe_version,
                resources.summary.loader_api_version,
                resources.summary.captured_year,
                resources.summary.captured_day,
                resources.summary.captured_at_ms,
                resources.summary.resource_count,
            ],
        )?;
        let mut rows_written = 1_u64;
        for entry in &resources.entries {
            let class_mask = entry
                .transport_classes
                .iter()
                .filter(|class| **class >= 0 && **class < 18)
                .fold(0_u32, |mask, class| mask | (1_u32 << *class as u32));
            transaction.execute(
                "INSERT INTO resource_registry_entries VALUES(\
                     ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10\
                 ) ON CONFLICT DO NOTHING",
                params![
                    resources.summary.snapshot_id,
                    entry.live_index,
                    entry.source_token,
                    entry.display_name,
                    entry.label_source,
                    entry.caption_id,
                    entry.resource_kind,
                    class_mask,
                    entry.material_family,
                    entry.origin.runtime_extension,
                ],
            )?;
            rows_written = rows_written.saturating_add(1);
            for price in &entry.live_prices {
                transaction.execute(
                    "INSERT INTO resource_registry_prices VALUES(\
                         ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9\
                     ) ON CONFLICT DO NOTHING",
                    params![
                        resources.summary.snapshot_id,
                        entry.live_index,
                        price.currency,
                        price.finished_price,
                        price.base_price,
                        price.buy_multiplier,
                        price.sell_multiplier,
                        price.buy_quote,
                        price.sell_quote,
                    ],
                )?;
                rows_written = rows_written.saturating_add(1);
            }
            if rows_written == rows_total || rows_written.is_multiple_of(256) {
                permit.progress(WarehouseWriteStage::Staging, rows_written);
            }
        }
        permit.progress(WarehouseWriteStage::Merging, rows_total);
        record_receipt(
            &transaction,
            projection_id,
            "resource_registry_snapshot",
            &resources.summary.snapshot_id,
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

    pub(crate) fn broadcast_projection_available(
        &self,
        interpretation_id: &str,
    ) -> Result<bool, ObservatoryError> {
        if !self.available {
            return Ok(false);
        }
        let connection = self
            .connection
            .try_lock()
            .map_err(|_| ObservatoryError::WarehouseUnavailable)?;
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM projection_receipts \
                 WHERE projection_kind = 'broadcast_observation' AND source_identity = ?1)",
                [interpretation_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn broadcast_station_requirements(
        &self,
    ) -> Result<Vec<BroadcastStationRequirement>, ObservatoryError> {
        if !self.available {
            return Ok(Vec::new());
        }
        let connection = self
            .connection
            .try_lock()
            .map_err(|_| ObservatoryError::WarehouseUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT membership.entity_id, type.value_text, workers.value_number, \
                    professors.value_number \
             FROM catalogue_generation_entities membership \
             JOIN warehouse_metadata metadata \
               ON membership.generation_id = metadata.current_catalogue_generation_id \
             JOIN definition_properties type \
               ON type.revision_hash = membership.revision_hash \
              AND type.field_id = 'building.type' AND type.occurrence = 0 \
             JOIN definition_properties workers \
               ON workers.revision_hash = membership.revision_hash \
              AND workers.field_id = 'building.workers.required' AND workers.occurrence = 0 \
             JOIN definition_properties professors \
               ON professors.revision_hash = membership.revision_hash \
              AND professors.field_id = 'building.professors.required' \
              AND professors.occurrence = 0 \
             WHERE metadata.singleton_id = 1 \
               AND type.value_text IN ('TYPE_RADIO_STATION', 'TYPE_TV_STATION', \
                                       'TYPE_TELEVISION_STATION') \
             ORDER BY type.value_text, membership.entity_id",
        )?;
        let candidates = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut by_kind = BTreeMap::<String, Vec<(String, u32, u32)>>::new();
        for (entity_id, source_type, workers, professors) in candidates {
            if !workers.is_finite()
                || !professors.is_finite()
                || workers < 0.0
                || professors < 0.0
                || workers.fract() != 0.0
                || professors.fract() != 0.0
                || workers > f64::from(u32::MAX)
                || professors > f64::from(u32::MAX)
            {
                continue;
            }
            let station_kind = if source_type == "TYPE_RADIO_STATION" {
                "radio"
            } else {
                "television"
            };
            by_kind.entry(station_kind.to_owned()).or_default().push((
                entity_id,
                workers as u32,
                professors as u32,
            ));
        }
        Ok(by_kind
            .into_iter()
            .filter_map(|(station_kind, candidates)| {
                let [(catalogue_entity_id, workers, professors)] = candidates.as_slice() else {
                    return None;
                };
                Some(BroadcastStationRequirement {
                    station_kind,
                    catalogue_entity_id: catalogue_entity_id.clone(),
                    workers: *workers,
                    professors: *professors,
                })
            })
            .collect())
    }

    pub(crate) fn market_projection(
        &self,
        metadata: &MarketWarehouseProjection,
    ) -> Result<Option<MarketWarehouseProjection>, ObservatoryError> {
        if !self.available {
            return Ok(None);
        }
        let connection = self
            .connection
            .try_lock()
            .map_err(|_| ObservatoryError::WarehouseUnavailable)?;
        let projected = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM projection_receipts \
             WHERE projection_kind = 'market_observation' AND source_identity = ?1)",
            [&metadata.interpretation_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !projected {
            return Ok(None);
        }

        let records = {
            let mut statement = connection.prepare(
                "SELECT membership.record_hash, membership.ordinal, record.record_id, \
                        record.year, record.day, record.game_day \
                 FROM market_observation_records membership \
                 JOIN market_records record USING(record_hash) \
                 WHERE membership.interpretation_id = ?1 ORDER BY membership.ordinal",
            )?;
            statement
                .query_map([&metadata.interpretation_id], |row| {
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
        let prices = {
            let mut statement = connection.prepare(
                "SELECT fact.record_hash, NULL AS scope_kind, NULL AS scope_id, fact.currency, \
                        fact.price_side, fact.resource_token, fact.value, fact.modifier, \
                        fact.source_field, fact.source_line, fact.mapping_id \
                 FROM market_price_facts fact \
                 JOIN market_observation_records membership USING(record_hash) \
                 WHERE membership.interpretation_id = ?1 AND ( \
                     fact.record_hash = (SELECT record_hash FROM market_observation_records \
                        WHERE interpretation_id = ?1 ORDER BY ordinal LIMIT 1) OR \
                     fact.record_hash = (SELECT record_hash FROM market_observation_records \
                        WHERE interpretation_id = ?1 ORDER BY ordinal DESC LIMIT 1)) \
                 UNION ALL \
                 SELECT NULL, scope_kind, scope_id, currency, price_side, resource_token, \
                        value, modifier, source_field, source_line, mapping_id \
                 FROM market_snapshot_price_facts WHERE interpretation_id = ?1 \
                 ORDER BY record_hash, scope_kind, scope_id, source_line",
            )?;
            statement
                .query_map([&metadata.interpretation_id], |row| {
                    Ok(MarketWarehousePriceFact {
                        record_hash: row.get(0)?,
                        scope_kind: row.get(1)?,
                        scope_id: row.get(2)?,
                        currency: row.get(3)?,
                        price_side: row.get(4)?,
                        resource_token: row.get(5)?,
                        value: row.get(6)?,
                        modifier: row.get(7)?,
                        source_field: row.get(8)?,
                        source_line: row.get(9)?,
                        mapping_id: row.get(10)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let trades = {
            let mut statement = connection.prepare(
                "SELECT fact.record_hash, NULL AS scope_kind, NULL AS scope_id, fact.currency, \
                        fact.direction, fact.channel, fact.resource_token, fact.quantity, \
                        fact.account_value, fact.source_field, fact.source_line, fact.mapping_id \
                 FROM market_trade_facts fact \
                 JOIN market_observation_records membership USING(record_hash) \
                 WHERE membership.interpretation_id = ?1 AND ( \
                     fact.record_hash = (SELECT record_hash FROM market_observation_records \
                        WHERE interpretation_id = ?1 ORDER BY ordinal LIMIT 1) OR \
                     fact.record_hash = (SELECT record_hash FROM market_observation_records \
                        WHERE interpretation_id = ?1 ORDER BY ordinal DESC LIMIT 1)) \
                 UNION ALL \
                 SELECT NULL, scope_kind, scope_id, currency, direction, channel, resource_token, \
                        quantity, account_value, source_field, source_line, mapping_id \
                 FROM market_snapshot_trade_facts WHERE interpretation_id = ?1 \
                 ORDER BY record_hash, scope_kind, scope_id, source_line",
            )?;
            statement
                .query_map([&metadata.interpretation_id], |row| {
                    Ok(MarketWarehouseTradeFact {
                        record_hash: row.get(0)?,
                        scope_kind: row.get(1)?,
                        scope_id: row.get(2)?,
                        currency: row.get(3)?,
                        direction: row.get(4)?,
                        channel: row.get(5)?,
                        resource_token: row.get(6)?,
                        quantity: row.get(7)?,
                        account_value: row.get(8)?,
                        source_field: row.get(9)?,
                        source_line: row.get(10)?,
                        mapping_id: row.get(11)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let scalars = {
            let mut statement = connection.prepare(
                "SELECT fact.record_hash, NULL AS scope_kind, NULL AS scope_id, fact.fact_id, \
                        fact.currency, fact.category, fact.value, fact.source_field, \
                        fact.source_line, fact.mapping_id \
                 FROM market_scalar_facts fact \
                 JOIN market_observation_records membership USING(record_hash) \
                 WHERE membership.interpretation_id = ?1 AND \
                     fact.record_hash = (SELECT record_hash FROM market_observation_records \
                        WHERE interpretation_id = ?1 ORDER BY ordinal DESC LIMIT 1) \
                 UNION ALL \
                 SELECT NULL, scope_kind, scope_id, fact_id, currency, category, value, \
                        source_field, source_line, mapping_id \
                 FROM market_snapshot_scalar_facts WHERE interpretation_id = ?1 \
                 ORDER BY record_hash, scope_kind, scope_id, source_line",
            )?;
            statement
                .query_map([&metadata.interpretation_id], |row| {
                    Ok(MarketWarehouseScalarFact {
                        record_hash: row.get(0)?,
                        scope_kind: row.get(1)?,
                        scope_id: row.get(2)?,
                        fact_id: row.get(3)?,
                        currency: row.get(4)?,
                        category: row.get(5)?,
                        value: row.get(6)?,
                        source_field: row.get(7)?,
                        source_line: row.get(8)?,
                        mapping_id: row.get(9)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let analytical_trade_history = {
            let mut statement = connection.prepare(
                "SELECT record_hash, year, day, game_day, currency, channel, \
                        SUM(CASE WHEN direction = 'import' THEN account_value ELSE 0 END), \
                        SUM(CASE WHEN direction = 'export' THEN account_value ELSE 0 END) \
                 FROM market_trade_history WHERE interpretation_id = ?1 \
                 GROUP BY record_hash, year, day, game_day, currency, channel \
                 ORDER BY game_day, currency, channel",
            )?;
            statement
                .query_map([&metadata.interpretation_id], |row| {
                    let import_value = row.get::<_, f64>(6)?;
                    let export_value = row.get::<_, f64>(7)?;
                    Ok(MarketTradePoint {
                        record_hash: row.get(0)?,
                        year: row.get(1)?,
                        day: row.get(2)?,
                        game_day: row.get(3)?,
                        currency: row.get(4)?,
                        channel: row.get(5)?,
                        import_value,
                        export_value,
                        trade_result: export_value - import_value,
                        exact_observation: None,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let analytical_price_volatility = {
            let mut statement = connection.prepare(
                "WITH ordered AS ( \
                     SELECT currency, resource_token, game_day, value, \
                            LAG(value) OVER (PARTITION BY currency, resource_token \
                                             ORDER BY game_day) AS previous_value \
                     FROM market_price_history \
                     WHERE interpretation_id = ?1 AND price_side = 'purchase' AND value > 0 \
                 ), movements AS ( \
                     SELECT currency, resource_token, LN(value / previous_value) AS movement \
                     FROM ordered WHERE previous_value > 0 \
                 ), centred AS ( \
                     SELECT currency, resource_token, movement, \
                            MEDIAN(movement) OVER (PARTITION BY currency, resource_token) AS centre \
                     FROM movements WHERE ISFINITE(movement) \
                 ) \
                 SELECT currency, resource_token, \
                        MEDIAN(ABS(movement - centre)) * 1.4826, COUNT(*) \
                 FROM centred GROUP BY currency, resource_token HAVING COUNT(*) >= 2 \
                 ORDER BY currency, resource_token",
            )?;
            statement
                .query_map([&metadata.interpretation_id], |row| {
                    Ok(MarketPriceVolatility {
                        currency: row.get(0)?,
                        resource_token: row.get(1)?,
                        robust_log_volatility: row.get(2)?,
                        observations: row.get(3)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(Some(MarketWarehouseProjection {
            interpretation_id: metadata.interpretation_id.clone(),
            raw_payload_hash: metadata.raw_payload_hash.clone(),
            branch_id: metadata.branch_id.clone(),
            profile_id: metadata.profile_id.clone(),
            profile_version: metadata.profile_version.clone(),
            resolved_profile_hash: metadata.resolved_profile_hash.clone(),
            mapping_classification: metadata.mapping_classification.clone(),
            parser_engine_version: metadata.parser_engine_version.clone(),
            records,
            prices,
            trades,
            scalars,
            analytical_trade_history,
            analytical_price_volatility,
        }))
    }

    pub(crate) fn market_price_series(
        &self,
        interpretation_id: &str,
        currency: &str,
        resource_token: &str,
    ) -> Result<Option<Vec<MarketPriceSeriesPoint>>, ObservatoryError> {
        if !self.available {
            return Ok(None);
        }
        let connection = self
            .connection
            .try_lock()
            .map_err(|_| ObservatoryError::WarehouseUnavailable)?;
        let projected = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM projection_receipts \
             WHERE projection_kind = 'market_observation' AND source_identity = ?1)",
            [interpretation_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !projected {
            return Ok(None);
        }
        let mut statement = connection.prepare(
            "SELECT record_hash, year, day, game_day, \
                    MAX(CASE WHEN price_side = 'purchase' THEN value END), \
                    MAX(CASE WHEN price_side = 'sell' THEN value END), \
                    MAX(CASE WHEN price_side = 'base' THEN value END) \
             FROM market_price_history \
             WHERE interpretation_id = ?1 AND currency = ?2 AND resource_token = ?3 \
             GROUP BY record_hash, year, day, game_day \
             ORDER BY game_day, record_hash LIMIT 10000",
        )?;
        let points = statement
            .query_map(
                params![interpretation_id, currency, resource_token],
                |row| {
                    Ok(MarketPriceSeriesPoint {
                        record_hash: row.get(0)?,
                        year: row.get(1)?,
                        day: row.get(2)?,
                        game_day: row.get(3)?,
                        purchase_price: row.get(4)?,
                        sell_price: row.get(5)?,
                        base_price: row.get(6)?,
                        exact_observation: None,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(points))
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

    pub fn project_branch_memberships(
        &self,
        projection_id: &str,
        memberships: &[BranchMembershipProjection],
        branch_id: &str,
        membership_revision: u32,
        applied_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        let mut connection = self.lock()?;
        let receipt_already_exists = receipt_exists(&connection, projection_id)?;
        let rows_total = u64::try_from(memberships.len()).unwrap_or(u64::MAX);
        let permit = self
            .governor
            .begin(WarehouseWriteKind::BranchMembershipProjection, rows_total)?;
        let transaction = connection.transaction()?;
        let generation_exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM branch_membership_generations \
             WHERE branch_id = ?1 AND membership_revision = ?2)",
            params![branch_id, membership_revision],
            |row| row.get::<_, bool>(0),
        )?;
        if generation_exists {
            let mut statement = transaction.prepare(
                "SELECT branch_id, membership_revision, interpretation_id, payload_hash, \
                        parent_interpretation_id, relationship, shared_record_count \
                 FROM branch_observation_memberships \
                 WHERE branch_id = ?1 AND membership_revision = ?2 \
                 ORDER BY interpretation_id",
            )?;
            let stored = statement
                .query_map(params![branch_id, membership_revision], |row| {
                    Ok(BranchMembershipProjection {
                        branch_id: row.get(0)?,
                        membership_revision: row.get(1)?,
                        interpretation_id: row.get(2)?,
                        payload_hash: row.get(3)?,
                        parent_interpretation_id: row.get(4)?,
                        relationship: row.get(5)?,
                        shared_record_count: row.get(6)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            let mut expected = memberships.to_vec();
            expected.sort_by(|left, right| left.interpretation_id.cmp(&right.interpretation_id));
            if stored != expected {
                return Err(ObservatoryError::StorageContractViolation);
            }
        } else {
            if memberships.iter().any(|membership| {
                membership.branch_id != branch_id
                    || membership.membership_revision != membership_revision
            }) {
                return Err(ObservatoryError::StorageContractViolation);
            }
            transaction.execute(
                "INSERT INTO branch_membership_generations VALUES(?1, ?2, ?3)",
                params![branch_id, membership_revision, applied_at_ms],
            )?;
            {
                let mut appender = transaction.appender("branch_observation_memberships")?;
                for (index, membership) in memberships.iter().enumerate() {
                    appender.append_row(params![
                        membership.branch_id,
                        membership.membership_revision,
                        membership.interpretation_id,
                        membership.payload_hash,
                        membership.parent_interpretation_id,
                        membership.relationship,
                        membership.shared_record_count,
                    ])?;
                    let written = u64::try_from(index + 1).unwrap_or(rows_total);
                    if written == rows_total || written.is_multiple_of(512) {
                        permit.progress(WarehouseWriteStage::Staging, written);
                    }
                }
            }
        }
        permit.progress(WarehouseWriteStage::Merging, rows_total);
        if !receipt_already_exists {
            record_receipt(
                &transaction,
                projection_id,
                "branch_membership",
                branch_id,
                applied_at_ms,
            )?;
        }
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
        transaction.execute("DELETE FROM market_snapshot_price_facts", [])?;
        transaction.execute("DELETE FROM market_snapshot_trade_facts", [])?;
        transaction.execute("DELETE FROM market_snapshot_scalar_facts", [])?;
        transaction.execute("DELETE FROM market_price_facts", [])?;
        transaction.execute("DELETE FROM market_trade_facts", [])?;
        transaction.execute("DELETE FROM market_scalar_facts", [])?;
        transaction.execute("DELETE FROM market_observation_records", [])?;
        transaction.execute("DELETE FROM market_records", [])?;
        transaction.execute("DELETE FROM broadcast_status_facts", [])?;
        transaction.execute("DELETE FROM broadcast_status_observation_records", [])?;
        transaction.execute("DELETE FROM broadcast_status_records", [])?;
        transaction.execute("DELETE FROM environment_activity_facts", [])?;
        transaction.execute("DELETE FROM environment_activity_observation_records", [])?;
        transaction.execute("DELETE FROM environment_activity_records", [])?;
        transaction.execute("DELETE FROM resource_registry_prices", [])?;
        transaction.execute("DELETE FROM resource_registry_entries", [])?;
        transaction.execute("DELETE FROM resource_registry_snapshots", [])?;
        transaction.execute("DELETE FROM branch_observation_memberships", [])?;
        transaction.execute("DELETE FROM branch_membership_generations", [])?;
        transaction.execute(
            "DELETE FROM projection_receipts \
             WHERE projection_kind IN ('observation', 'market_observation', \
                 'broadcast_observation', 'environment_observation', \
                 'resource_registry_snapshot', 'branch_membership')",
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
            let cached = self.status_cache.lock().ok().map(|cache| {
                (
                    cache.last_projected_at_ms,
                    cache.observation_watermark.clone(),
                )
            });
            return WarehouseHealth {
                phase: if fallback.phase == WarehousePhase::Attention {
                    WarehousePhase::Attention
                } else if rebuilding {
                    WarehousePhase::Rebuilding
                } else {
                    WarehousePhase::Lagging
                },
                last_projected_at_ms: cached.as_ref().and_then(|value| value.0),
                observation_watermark: cached.and_then(|value| value.1),
                ..fallback
            };
        };
        let Ok((last_projected_at_ms, observation_watermark)) = connection.query_row(
            "SELECT last_projection_ms, observation_watermark FROM warehouse_metadata WHERE singleton_id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        ) else {
            return fallback;
        };
        drop(connection);
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.last_projected_at_ms = last_projected_at_ms;
            cache.observation_watermark = observation_watermark.clone();
        }
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
        let Ok(connection) = self.connection.try_lock() else {
            return self
                .status_cache
                .lock()
                .ok()
                .and_then(|cache| cache.catalogue_generation.clone());
        };
        let generation = catalogue_generation_from(&connection).ok()?;
        drop(connection);
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.catalogue_generation = generation.clone();
        }
        generation
    }

    pub fn catalogue_scope_statuses_if_ready(
        &self,
    ) -> Option<Vec<CompatibilityCatalogueScopeStatus>> {
        if !self.available {
            return None;
        }
        let Ok(connection) = self.connection.try_lock() else {
            return self
                .status_cache
                .lock()
                .ok()
                .map(|cache| cache.catalogue_scopes.clone());
        };
        let scopes = catalogue_scope_statuses_from(&connection).ok()?;
        drop(connection);
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.catalogue_scopes = scopes.clone();
        }
        Some(scopes)
    }

    pub fn catalogue_runtime_if_ready(&self) -> Option<CatalogueRuntime> {
        if !self.available {
            return None;
        }
        let Ok(connection) = self.connection.try_lock() else {
            return self
                .status_cache
                .lock()
                .ok()
                .map(|cache| cache.catalogue_runtime.clone());
        };
        let runtime = catalogue_runtime_from(&connection).ok()?;
        drop(connection);
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.catalogue_runtime = runtime.clone();
        }
        Some(runtime)
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
        let output_resource_id = filter.output_resource_id.as_deref().unwrap_or("");
        if !output_resource_id.is_empty()
            && (output_resource_id.len() > 160
                || !output_resource_id.starts_with("resource::")
                || !output_resource_id.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, ':' | '_' | '-' | '.')
                }))
        {
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
                              AND available_to.value_number >= ?6))) \
                      AND (?7 = '' OR EXISTS( \
                          SELECT 1 FROM definition_relations output_relation \
                           WHERE output_relation.revision_hash = revisions.revision_hash \
                             AND output_relation.relation_kind = 'production_output' \
                             AND output_relation.target_id = ?7))";
        let total = connection.query_row(
            &format!("SELECT COUNT(*){base}"),
            params![
                query,
                kind,
                source_kind,
                package_query,
                coverage,
                available_year,
                output_resource_id
            ],
            |row| row.get::<_, u32>(0),
        )?;
        let mut statement = connection.prepare(&format!(
            "SELECT membership.entity_id, revisions.revision_hash, revisions.entity_kind, \
                    revisions.source_id, sources.source_kind, sources.package_name, \
                    revisions.display_name, revisions.coverage, \
                    (SELECT COUNT(*) FROM definition_properties properties WHERE properties.revision_hash = revisions.revision_hash), \
                    (SELECT COUNT(*) FROM definition_relations relations WHERE relations.revision_hash = revisions.revision_hash) \
             {base} ORDER BY revisions.display_name, membership.entity_id LIMIT ?8 OFFSET ?9"
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
                    output_resource_id,
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

    pub(crate) fn installed_resource_evidence(
        &self,
    ) -> Result<Vec<InstalledResourceEvidence>, ObservatoryError> {
        let connection = self.lock()?;
        let mut references = BTreeMap::<String, (u32, BTreeSet<String>)>::new();
        let mut reference_statement = connection.prepare(
            "SELECT relations.target_id, revisions.source_id \
             FROM catalogue_generation_entities membership \
             JOIN warehouse_metadata metadata \
               ON membership.generation_id = metadata.current_catalogue_generation_id \
             JOIN definition_entity_revisions revisions USING(revision_hash) \
             JOIN definition_relations relations USING(revision_hash) \
             WHERE metadata.singleton_id = 1 \
               AND relations.target_id LIKE 'resource::%' \
             ORDER BY relations.target_id, revisions.source_id",
        )?;
        for row in reference_statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (resource_id, source_id) = row?;
            let Some(token) = resource_id.strip_prefix("resource::") else {
                continue;
            };
            let entry = references.entry(token.to_owned()).or_default();
            entry.0 = entry.0.saturating_add(1);
            entry.1.insert(source_id);
        }
        drop(reference_statement);

        let mut entries = Vec::new();
        let mut statement = connection.prepare(
            "SELECT revisions.source_object_id, revisions.display_name \
             FROM catalogue_generation_entities membership \
             JOIN warehouse_metadata metadata \
               ON membership.generation_id = metadata.current_catalogue_generation_id \
             JOIN definition_entity_revisions revisions USING(revision_hash) \
             WHERE metadata.singleton_id = 1 \
               AND revisions.entity_kind = 'resource' \
             ORDER BY revisions.source_object_id",
        )?;
        for row in statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (source_token, display_name) = row?;
            let (installed_reference_count, sources) =
                references.remove(&source_token).unwrap_or_default();
            entries.push(InstalledResourceEvidence {
                source_token,
                display_name,
                installed_reference_count,
                installed_sources: sources.into_iter().collect(),
                player_overlay: false,
            });
        }
        drop(statement);

        let mut overlay_statement = connection.prepare(
            "SELECT entity_id, display_name FROM active_overlay_entities \
             WHERE entity_kind = 'resource' ORDER BY entity_id",
        )?;
        for row in overlay_statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (entity_id, display_name) = row?;
            let Some(source_token) = entity_id.rsplit("::").next() else {
                continue;
            };
            entries.push(InstalledResourceEvidence {
                source_token: source_token.to_owned(),
                display_name,
                installed_reference_count: 0,
                installed_sources: Vec::new(),
                player_overlay: true,
            });
        }
        entries.sort_by(|left, right| left.source_token.cmp(&right.source_token));
        Ok(entries)
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

    pub fn production_pathway(
        &self,
        request: &ProductionPathwayRequest,
    ) -> Result<ProductionPathwayModel, ObservatoryError> {
        if request.root_recipe_entity_id.is_empty()
            || request.root_recipe_entity_id.len() > 320
            || !valid_resource_id(&request.output_resource_id)
            || !request.target_quantity.is_finite()
            || request.target_quantity <= 0.0
            || request.target_quantity > 1_000_000_000.0
            || request.max_depth < 2
            || request.max_depth > MAX_PRODUCTION_PATHWAY_DEPTH
            || request.selections.len() > MAX_PRODUCTION_PATHWAY_SELECTIONS
        {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }

        let mut selections = BTreeMap::new();
        for selection in &request.selections {
            if !valid_resource_id(&selection.resource_id)
                || selection.recipe_entity_id.is_empty()
                || selection.recipe_entity_id.len() > 320
                || selections
                    .insert(
                        selection.resource_id.clone(),
                        selection.recipe_entity_id.clone(),
                    )
                    .is_some()
            {
                return Err(ObservatoryError::InvalidCatalogueRequest);
            }
        }

        let root = self.production_route(&ProductionRouteRequest {
            entity_id: request.root_recipe_entity_id.clone(),
            output_resource_id: Some(request.output_resource_id.clone()),
            target_quantity: Some(request.target_quantity),
        })?;
        if !matches!(root.status.as_str(), "ready" | "ready_with_auxiliary") {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }
        let unit = root
            .unit
            .clone()
            .ok_or(ObservatoryError::InvalidCatalogueRequest)?;
        let root_output = root
            .flows
            .iter()
            .find(|flow| {
                flow.direction == "production_output"
                    && flow.resource_id == request.output_resource_id
                    && flow.basis_role == "primary"
            })
            .ok_or(ObservatoryError::InvalidCatalogueRequest)?;

        let mut build = PathwayBuild {
            snapshot: root.snapshot.clone(),
            max_depth: request.max_depth,
            selections,
            used_selections: BTreeSet::new(),
            nodes: vec![
                ProductionPathwayNode {
                    id: "stage-root".to_owned(),
                    kind: "process".to_owned(),
                    display_name: root.display_name.clone(),
                    resource_id: None,
                    recipe_entity_id: Some(root.route_id.clone()),
                    package_name: Some(root.package_name.clone()),
                    depth: 0,
                },
                ProductionPathwayNode {
                    id: "resource-target".to_owned(),
                    kind: "resource".to_owned(),
                    display_name: production_resource_name(&request.output_resource_id),
                    resource_id: Some(request.output_resource_id.clone()),
                    recipe_entity_id: None,
                    package_name: None,
                    depth: 0,
                },
            ],
            links: vec![ProductionPathwayLink {
                id: "pathway-link-0".to_owned(),
                source: "stage-root".to_owned(),
                target: "resource-target".to_owned(),
                resource_id: request.output_resource_id.clone(),
                quantity: request.target_quantity,
                unit: unit.clone(),
                source_directive: root_output.source_directive.clone(),
                source_line: root_output.source_line,
                mapping: root_output.mapping.clone(),
            }],
            choices: Vec::new(),
            terminal_requirements: BTreeMap::new(),
            auxiliary_requirements: Vec::new(),
            diagnostics: Vec::new(),
            next_node: 0,
            next_link: 1,
            player_mapped: root.mapping_classification == "player_mapped",
        };
        let mut route_path = BTreeSet::from([root.route_id.clone()]);
        self.expand_production_pathway_stage(&root, "stage-root", 0, &mut route_path, &mut build)?;
        if build.used_selections.len() != build.selections.len() {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }

        let has_limit = build.diagnostics.iter().any(|item| {
            matches!(
                item.code.as_str(),
                "node_limit" | "link_limit" | "candidate_limit"
            )
        });
        let has_boundary = build.diagnostics.iter().any(|item| {
            matches!(
                item.code.as_str(),
                "depth_limit" | "cycle" | "unsupported_route"
            )
        });
        let status = if has_limit {
            "too_complex"
        } else if build
            .choices
            .iter()
            .any(|choice| choice.selected_recipe_entity_id.is_none())
        {
            "needs_selection"
        } else if has_boundary {
            "bounded"
        } else if !build.auxiliary_requirements.is_empty() {
            "ready_with_auxiliary"
        } else {
            "ready"
        };
        let terminal_requirements = build
            .terminal_requirements
            .into_iter()
            .map(|((resource_id, unit, reason), (display_name, quantity))| {
                ProductionPathwayRequirement {
                    resource_id,
                    display_name,
                    quantity,
                    unit,
                    reason,
                }
            })
            .collect();

        Ok(ProductionPathwayModel {
            schema_version: 1,
            status: status.to_owned(),
            root_recipe_entity_id: request.root_recipe_entity_id.clone(),
            output_resource_id: request.output_resource_id.clone(),
            target_quantity: request.target_quantity,
            unit,
            max_depth: request.max_depth,
            mapping_classification: if build.player_mapped {
                "player_mapped"
            } else {
                "reviewed_mapping"
            }
            .to_owned(),
            nodes: build.nodes,
            links: build.links,
            choices: build.choices,
            terminal_requirements,
            auxiliary_requirements: build.auxiliary_requirements,
            diagnostics: build.diagnostics,
            snapshot: build.snapshot,
        })
    }

    fn production_pathway_candidates(
        &self,
        resource_id: &str,
        unit: &str,
        expected_generation_id: &str,
    ) -> Result<Vec<ProductionPathwayCandidate>, ObservatoryError> {
        let connection = self.lock()?;
        let current_generation_id = snapshot_from(&connection)?.catalogue_generation_id;
        if current_generation_id != expected_generation_id {
            return Err(ObservatoryError::CatalogueUnavailable);
        }
        let mut statement = connection.prepare(
            "SELECT DISTINCT membership.entity_id, revisions.display_name, sources.package_name, \
                    relations.quantity, relations.unit \
             FROM catalogue_generation_entities membership \
             JOIN definition_entity_revisions revisions USING(revision_hash) \
             JOIN catalogue_sources sources ON sources.generation_id = membership.generation_id \
                  AND sources.source_id = revisions.source_id \
             JOIN definition_relations relations USING(revision_hash) \
             WHERE membership.generation_id = ?1 AND revisions.entity_kind = 'recipe' \
               AND relations.relation_kind = 'production_output' \
               AND relations.target_id = ?2 AND relations.unit = ?3 \
               AND relations.quantity IS NOT NULL AND relations.quantity > 0 \
             ORDER BY revisions.display_name, membership.entity_id, relations.occurrence \
             LIMIT ?4",
        )?;
        let rows = statement
            .query_map(
                params![
                    expected_generation_id,
                    resource_id,
                    unit,
                    (MAX_PRODUCTION_PATHWAY_CANDIDATES + 1) as u32
                ],
                |row| {
                    Ok(ProductionPathwayCandidate {
                        recipe_entity_id: row.get(0)?,
                        display_name: row.get(1)?,
                        package_name: row.get(2)?,
                        output_quantity: row.get(3)?,
                        unit: row.get(4)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let mut distinct = BTreeMap::new();
        for candidate in rows {
            distinct
                .entry(candidate.recipe_entity_id.clone())
                .or_insert(candidate);
        }
        Ok(distinct.into_values().collect())
    }

    fn expand_production_pathway_stage(
        &self,
        route: &ProductionRouteModel,
        stage_id: &str,
        depth: u32,
        route_path: &mut BTreeSet<String>,
        build: &mut PathwayBuild,
    ) -> Result<(), ObservatoryError> {
        for flow in route
            .flows
            .iter()
            .filter(|flow| flow.direction != "production_output")
        {
            if flow.mapping.mapping_classification == "player_mapped" {
                build.player_mapped = true;
            }
            if flow.basis_role == "auxiliary" {
                build
                    .auxiliary_requirements
                    .push(ProductionPathwayAuxiliaryRequirement {
                        stage_id: stage_id.to_owned(),
                        recipe_entity_id: route.route_id.clone(),
                        resource_id: flow.resource_id.clone(),
                        display_name: flow.display_name.clone(),
                        quantity: flow.scaled_quantity,
                        unit: flow.unit.clone(),
                        reason: flow
                            .basis_exclusion
                            .clone()
                            .unwrap_or_else(|| "different_unit".to_owned()),
                        source_directive: flow.source_directive.clone(),
                        source_line: flow.source_line,
                        mapping: flow.mapping.clone(),
                    });
                continue;
            }
            let quantity = flow
                .scaled_quantity
                .filter(|value| value.is_finite() && *value > 0.0)
                .ok_or(ObservatoryError::InvalidCatalogueRequest)?;
            let unit = flow
                .unit
                .as_deref()
                .ok_or(ObservatoryError::InvalidCatalogueRequest)?;
            if build.nodes.len() >= MAX_PRODUCTION_PATHWAY_NODES {
                build.terminal(
                    &flow.resource_id,
                    &flow.display_name,
                    quantity,
                    unit,
                    "node_limit",
                );
                build.diagnostic(
                    "node_limit",
                    Some(&flow.resource_id),
                    Some(&route.route_id),
                    depth,
                );
                continue;
            }
            let resource_node_id = format!("pathway-resource-{}", build.next_node);
            build.next_node += 1;
            build.nodes.push(ProductionPathwayNode {
                id: resource_node_id.clone(),
                kind: "resource".to_owned(),
                display_name: production_resource_name(&flow.resource_id),
                resource_id: Some(flow.resource_id.clone()),
                recipe_entity_id: None,
                package_name: None,
                depth: depth + 1,
            });
            if build.links.len() >= MAX_PRODUCTION_PATHWAY_LINKS {
                build.terminal(
                    &flow.resource_id,
                    &flow.display_name,
                    quantity,
                    unit,
                    "link_limit",
                );
                build.diagnostic(
                    "link_limit",
                    Some(&flow.resource_id),
                    Some(&route.route_id),
                    depth,
                );
                continue;
            }
            build.links.push(ProductionPathwayLink {
                id: format!("pathway-link-{}", build.next_link),
                source: resource_node_id.clone(),
                target: stage_id.to_owned(),
                resource_id: flow.resource_id.clone(),
                quantity,
                unit: unit.to_owned(),
                source_directive: flow.source_directive.clone(),
                source_line: flow.source_line,
                mapping: flow.mapping.clone(),
            });
            build.next_link += 1;

            let candidates = self.production_pathway_candidates(
                &flow.resource_id,
                unit,
                &build.snapshot.catalogue_generation_id,
            )?;
            if candidates.len() > MAX_PRODUCTION_PATHWAY_CANDIDATES {
                build.terminal(
                    &flow.resource_id,
                    &flow.display_name,
                    quantity,
                    unit,
                    "candidate_limit",
                );
                build.diagnostic(
                    "candidate_limit",
                    Some(&flow.resource_id),
                    Some(&route.route_id),
                    depth,
                );
                continue;
            }
            if candidates.is_empty() {
                build.terminal(
                    &flow.resource_id,
                    &flow.display_name,
                    quantity,
                    unit,
                    "external_input",
                );
                continue;
            }
            let requested_selection = build.selections.get(&flow.resource_id).cloned();
            if let Some(selection) = requested_selection.as_deref() {
                if !candidates
                    .iter()
                    .any(|candidate| candidate.recipe_entity_id == selection)
                {
                    return Err(ObservatoryError::InvalidCatalogueRequest);
                }
                build.used_selections.insert(flow.resource_id.clone());
            }
            let selected = requested_selection.or_else(|| {
                (candidates.len() == 1).then(|| candidates[0].recipe_entity_id.clone())
            });
            if candidates.len() > 1 {
                build.choices.push(ProductionPathwayChoice {
                    resource_node_id: resource_node_id.clone(),
                    resource_id: flow.resource_id.clone(),
                    display_name: production_resource_name(&flow.resource_id),
                    required_quantity: quantity,
                    unit: unit.to_owned(),
                    selected_recipe_entity_id: selected.clone(),
                    candidates: candidates.clone(),
                });
            }
            let Some(selected_recipe) = selected else {
                build.terminal(
                    &flow.resource_id,
                    &flow.display_name,
                    quantity,
                    unit,
                    "route_selection_required",
                );
                continue;
            };
            if depth + 1 >= build.max_depth {
                build.terminal(
                    &flow.resource_id,
                    &flow.display_name,
                    quantity,
                    unit,
                    "depth_limit",
                );
                build.diagnostic(
                    "depth_limit",
                    Some(&flow.resource_id),
                    Some(&selected_recipe),
                    depth + 1,
                );
                continue;
            }
            if route_path.contains(&selected_recipe) {
                build.terminal(
                    &flow.resource_id,
                    &flow.display_name,
                    quantity,
                    unit,
                    "cycle",
                );
                build.diagnostic(
                    "cycle",
                    Some(&flow.resource_id),
                    Some(&selected_recipe),
                    depth + 1,
                );
                continue;
            }
            let child = self.production_route(&ProductionRouteRequest {
                entity_id: selected_recipe.clone(),
                output_resource_id: Some(flow.resource_id.clone()),
                target_quantity: Some(quantity),
            })?;
            if !same_snapshot(&child.snapshot, &build.snapshot) {
                return Err(ObservatoryError::CatalogueUnavailable);
            }
            if !matches!(child.status.as_str(), "ready" | "ready_with_auxiliary") {
                build.terminal(
                    &flow.resource_id,
                    &flow.display_name,
                    quantity,
                    unit,
                    "unsupported_route",
                );
                build.diagnostic(
                    "unsupported_route",
                    Some(&flow.resource_id),
                    Some(&selected_recipe),
                    depth + 1,
                );
                continue;
            }
            if child.mapping_classification == "player_mapped" {
                build.player_mapped = true;
            }
            if build.nodes.len() >= MAX_PRODUCTION_PATHWAY_NODES
                || build.links.len() >= MAX_PRODUCTION_PATHWAY_LINKS
            {
                let reason = if build.nodes.len() >= MAX_PRODUCTION_PATHWAY_NODES {
                    "node_limit"
                } else {
                    "link_limit"
                };
                build.terminal(
                    &flow.resource_id,
                    &flow.display_name,
                    quantity,
                    unit,
                    reason,
                );
                build.diagnostic(
                    reason,
                    Some(&flow.resource_id),
                    Some(&selected_recipe),
                    depth + 1,
                );
                continue;
            }
            let child_output = child
                .flows
                .iter()
                .find(|candidate| {
                    candidate.direction == "production_output"
                        && candidate.resource_id == flow.resource_id
                        && candidate.basis_role == "primary"
                })
                .ok_or(ObservatoryError::InvalidCatalogueRequest)?;
            let child_stage_id = format!("pathway-stage-{}", build.next_node);
            build.next_node += 1;
            build.nodes.push(ProductionPathwayNode {
                id: child_stage_id.clone(),
                kind: "process".to_owned(),
                display_name: child.display_name.clone(),
                resource_id: None,
                recipe_entity_id: Some(child.route_id.clone()),
                package_name: Some(child.package_name.clone()),
                depth: depth + 1,
            });
            build.links.push(ProductionPathwayLink {
                id: format!("pathway-link-{}", build.next_link),
                source: child_stage_id.clone(),
                target: resource_node_id,
                resource_id: flow.resource_id.clone(),
                quantity,
                unit: unit.to_owned(),
                source_directive: child_output.source_directive.clone(),
                source_line: child_output.source_line,
                mapping: child_output.mapping.clone(),
            });
            build.next_link += 1;
            route_path.insert(child.route_id.clone());
            self.expand_production_pathway_stage(
                &child,
                &child_stage_id,
                depth + 1,
                route_path,
                build,
            )?;
            route_path.remove(&child.route_id);
        }
        Ok(())
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

fn valid_resource_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value.starts_with("resource::")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ':' | '_' | '-' | '.')
        })
}

fn same_snapshot(left: &WarehouseSnapshot, right: &WarehouseSnapshot) -> bool {
    left.catalogue_generation_id == right.catalogue_generation_id
        && left.compatibility_profile_hash == right.compatibility_profile_hash
        && left.overlay_profile_id == right.overlay_profile_id
        && left.overlay_revision == right.overlay_revision
        && left.observation_watermark == right.observation_watermark
        && left.warehouse_schema_version == right.warehouse_schema_version
        && left.projector_version == right.projector_version
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
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::definition_catalogue::{
        CatalogueFile, CatalogueSource, ParsedDefinition, ParsedProperty, ParsedRelation,
    };
    use crate::model::{CoverageReport, CoverageStatus, ReceiverHistoryPoint};
    use crate::planning_overlay::{OverlayOperation, OverlayOperationKind, OverlaySupplement};
    use tempfile::tempdir;

    // These two regression checks deliberately exercise sizeable DuckDB writes.
    // Running them together measures test-runner contention instead of either
    // operation, so keep their reference timings independent and repeatable.
    static BULK_PROJECTION_TIMING_LOCK: Mutex<()> = Mutex::new(());

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
            original_branch_id: "main".to_owned(),
            analysis_context_id: None,
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
                    exact_observation: None,
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
    fn environment_definition_context_reads_the_current_catalogue() {
        let directory = tempdir().expect("temporary directory");
        let warehouse =
            AnalyticalWarehouse::initialise(directory.path().join("environment-context.duckdb"))
                .expect("warehouse");
        {
            let connection = warehouse.lock().expect("connection");
            connection
                .execute_batch(
                    r#"
                    INSERT INTO catalogue_generations(
                        generation_id, game_build_id, parser_version, created_at_ms,
                        source_count, file_count, entity_count, property_count,
                        relation_count, warning_count)
                    VALUES('environment-generation', NULL, 'environment-test', 1, 1, 1, 4, 0, 0, 0);
                    INSERT INTO definition_entity_revisions
                    VALUES('environment-revision', 'building', 'base', 'building-1',
                           'Water treatment plant', 'complete');
                    INSERT INTO catalogue_generation_entities
                    VALUES('environment-generation', 'base::building::building-1',
                           'environment-revision');
                    INSERT INTO definition_properties(
                        revision_hash, field_id, occurrence, value_kind,
                        value_number, value_text, unit, source_directive,
                        source_line, raw_arguments, evidence_kind, resolution)
                    VALUES
                      ('environment-revision', 'building.environment.pollution_class', 0,
                       'number', 1, NULL, NULL, '$POLLUTION', 1, '1', 'from_game_files', 'base'),
                      ('environment-revision', 'building.environment.sewage_pollution_factor', 0,
                       'number', 0.5, NULL, NULL, '$SEWAGE', 2, '0.5', 'from_game_files', 'base'),
                      ('environment-revision', 'building.environment.water_required_quality', 0,
                       'number', 0.9, NULL, NULL, '$WATER', 3, '0.9', 'from_game_files', 'base'),
                      ('environment-revision', 'building.environment.sewage_disabled', 0,
                       'boolean', NULL, NULL, NULL, '$SEWAGE_DISABLED', 4, 'false',
                       'from_game_files', 'base');
                    UPDATE warehouse_metadata
                    SET current_catalogue_generation_id = 'environment-generation'
                    WHERE singleton_id = 1;
                    "#,
                )
                .expect("environment catalogue fixture");
        }

        let context = warehouse
            .environment_definition_context()
            .expect("environment definition context");
        assert!(context.available);
        assert_eq!(context.building_count, 1);
        assert_eq!(context.pollution_class_facts, 1);
        assert_eq!(context.sewage_pollution_factors, 1);
        assert_eq!(context.water_quality_facts, 1);
        assert_eq!(context.connection_capability_facts, 1);
    }

    #[test]
    fn broadcast_projection_is_idempotent_and_reuses_content_addressed_records() {
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("broadcast.duckdb"))
            .expect("warehouse");
        let projection = broadcast_projection_fixture("broadcast-one");
        warehouse
            .project_broadcast_observation("broadcast:one", &projection, 1)
            .expect("first projection");
        warehouse
            .project_broadcast_observation("broadcast:one", &projection, 2)
            .expect("duplicate delivery");

        let second = broadcast_projection_fixture("broadcast-two");
        warehouse
            .project_broadcast_observation("broadcast:two", &second, 3)
            .expect("second interpretation");
        assert!(
            warehouse
                .broadcast_projection_available("broadcast-one")
                .expect("availability")
        );
        let connection = warehouse.lock().expect("connection");
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM broadcast_status_records", [], |row| {
                    row.get::<_, u32>(0)
                })
                .expect("record count"),
            2
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM broadcast_status_facts", [], |row| {
                    row.get::<_, u32>(0)
                })
                .expect("fact count"),
            18
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM broadcast_status_observation_records",
                    [],
                    |row| row.get::<_, u32>(0),
                )
                .expect("membership count"),
            4
        );
    }

    fn broadcast_projection_fixture(interpretation_id: &str) -> BroadcastWarehouseProjection {
        let records = (0..2)
            .map(|ordinal| crate::model::BroadcastWarehouseRecord {
                record_hash: format!("{:064x}", ordinal + 1),
                ordinal,
                record_id: ordinal,
                year: 2017,
                day: ordinal as u16,
                game_day: i64::from(ordinal),
            })
            .collect::<Vec<_>>();
        let facts = records
            .iter()
            .flat_map(|record| {
                crate::model::CITIZEN_STATUS_METRICS
                    .iter()
                    .map(move |metric| crate::model::BroadcastWarehouseFact {
                        record_hash: record.record_hash.clone(),
                        source_index: metric.source_index,
                        metric_id: metric.id.to_owned(),
                        value: f64::from(metric.source_index) / 10.0,
                        source_field: "$Citizens_Status".to_owned(),
                        source_line: u64::from(metric.source_index) + 1,
                        mapping_id: metric.id.to_owned(),
                    })
            })
            .collect();
        BroadcastWarehouseProjection {
            interpretation_id: interpretation_id.to_owned(),
            raw_payload_hash: "a".repeat(64),
            branch_id: "main".to_owned(),
            profile_id: "profile".to_owned(),
            profile_version: "1.2.0".to_owned(),
            resolved_profile_hash: "b".repeat(64),
            mapping_classification: "reviewed_mapping".to_owned(),
            records,
            facts,
        }
    }

    #[test]
    fn environment_projection_is_idempotent_and_reuses_content_addressed_records() {
        let directory = tempdir().expect("temporary directory");
        let warehouse =
            AnalyticalWarehouse::initialise(directory.path().join("environment.duckdb"))
                .expect("warehouse");
        let projection = environment_projection_fixture("environment-one");
        warehouse
            .project_environment_observation("environment:one", &projection, 1)
            .expect("first projection");
        warehouse
            .project_environment_observation("environment:one", &projection, 2)
            .expect("duplicate delivery");
        let second = environment_projection_fixture("environment-two");
        warehouse
            .project_environment_observation("environment:two", &second, 3)
            .expect("second interpretation");

        let connection = warehouse.lock().expect("connection");
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM environment_activity_records",
                    [],
                    |row| { row.get::<_, u32>(0) }
                )
                .expect("record count"),
            2
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM environment_activity_facts",
                    [],
                    |row| { row.get::<_, u32>(0) }
                )
                .expect("fact count"),
            2
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM environment_activity_observation_records",
                    [],
                    |row| row.get::<_, u32>(0),
                )
                .expect("membership count"),
            4
        );
    }

    fn environment_projection_fixture(interpretation_id: &str) -> EnvironmentWarehouseProjection {
        let records = (0..2)
            .map(|ordinal| crate::model::EnvironmentWarehouseRecord {
                record_hash: format!("{:064x}", ordinal + 21),
                ordinal,
                record_id: ordinal,
                year: 2017,
                day: ordinal as u16,
                game_day: i64::from(ordinal),
            })
            .collect::<Vec<_>>();
        let facts = records
            .iter()
            .map(|record| crate::model::EnvironmentWarehouseFact {
                record_hash: record.record_hash.clone(),
                source_field: "$FactoryProductionStats".to_owned(),
                source_line: u64::from(record.ordinal) + 1,
                row_ordinal: 0,
                resource_token: "chemicals".to_owned(),
                activity_channel: "production".to_owned(),
                primary_value: 42.0,
                secondary_value: 0.0,
                quantity_is_publishable: true,
                mapping_id: "environment.activity.production".to_owned(),
            })
            .collect();
        EnvironmentWarehouseProjection {
            interpretation_id: interpretation_id.to_owned(),
            raw_payload_hash: "a".repeat(64),
            branch_id: "main".to_owned(),
            profile_id: "profile".to_owned(),
            profile_version: "1.3.0".to_owned(),
            resolved_profile_hash: "b".repeat(64),
            mapping_classification: "reviewed_mapping".to_owned(),
            records,
            facts,
        }
    }

    #[test]
    fn market_projection_is_idempotent_and_read_back_is_interpretation_bounded() {
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("market.duckdb"))
            .expect("warehouse");
        let projection = MarketWarehouseProjection {
            interpretation_id: "interpretation-market".to_owned(),
            raw_payload_hash: "raw-market".to_owned(),
            branch_id: "main".to_owned(),
            profile_id: "reviewed".to_owned(),
            profile_version: "1.1.0".to_owned(),
            resolved_profile_hash: "profile-hash".to_owned(),
            mapping_classification: "reviewed_mapping".to_owned(),
            parser_engine_version: "compatibility-profile-engine.v2".to_owned(),
            records: vec![MarketWarehouseRecord {
                record_hash: "record-one".to_owned(),
                ordinal: 0,
                record_id: 1,
                year: 1980,
                day: 1,
                game_day: 1,
            }],
            prices: vec![MarketWarehousePriceFact {
                record_hash: Some("record-one".to_owned()),
                scope_kind: None,
                scope_id: None,
                currency: "rub".to_owned(),
                price_side: "purchase".to_owned(),
                resource_token: "steel".to_owned(),
                value: 12.0,
                modifier: 1.0,
                source_field: "$Purchase".to_owned(),
                source_line: 4,
                mapping_id: "market.price.purchase.rub".to_owned(),
            }],
            trades: vec![MarketWarehouseTradeFact {
                record_hash: Some("record-one".to_owned()),
                scope_kind: None,
                scope_id: None,
                currency: "rub".to_owned(),
                direction: "import".to_owned(),
                channel: "standard".to_owned(),
                resource_token: "steel".to_owned(),
                quantity: 2.0,
                account_value: 24.0,
                source_field: "$Import".to_owned(),
                source_line: 5,
                mapping_id: "market.trade.import.standard.rub".to_owned(),
            }],
            scalars: Vec::new(),
            analytical_trade_history: Vec::new(),
            analytical_price_volatility: Vec::new(),
        };
        warehouse
            .project_market_observation("market:one", &projection, 1)
            .expect("first delivery");
        warehouse
            .project_market_observation("market:one", &projection, 2)
            .expect("duplicate delivery");
        let mut second = projection.clone();
        second.interpretation_id = "interpretation-market-two".to_owned();
        warehouse
            .project_market_observation("market:two", &second, 3)
            .expect("shared-record delivery");
        {
            let connection = warehouse.lock().expect("cache evidence");
            assert_eq!(
                connection
                    .query_row("SELECT COUNT(*) FROM market_records", [], |row| row
                        .get::<_, u32>(0))
                    .expect("shared records"),
                1
            );
            assert_eq!(
                connection
                    .query_row("SELECT COUNT(*) FROM market_price_facts", [], |row| row
                        .get::<_, u32>(0))
                    .expect("shared price facts"),
                1
            );
            assert_eq!(
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM market_observation_records",
                        [],
                        |row| row.get::<_, u32>(0)
                    )
                    .expect("interpretation memberships"),
                2
            );
        }
        let loaded = warehouse
            .market_projection(&projection)
            .expect("read")
            .expect("projected");
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.prices.len(), 1);
        assert_eq!(loaded.trades.len(), 1);
        assert_eq!(loaded.trades[0].account_value, 24.0);
        let series = warehouse
            .market_price_series("interpretation-market", "rub", "steel")
            .expect("price series query")
            .expect("projected price series");
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].purchase_price, Some(12.0));
        assert_eq!(series[0].sell_price, None);
    }

    #[test]
    #[ignore = "reference-machine million-row market aggregation and memory-bound check"]
    fn synthetic_market_scale_read_remains_bounded() {
        let directory = tempdir().expect("temporary directory");
        let warehouse =
            AnalyticalWarehouse::initialise(directory.path().join("market-scale.duckdb"))
                .expect("warehouse");
        let projection = MarketWarehouseProjection {
            interpretation_id: "interpretation-market-scale".to_owned(),
            raw_payload_hash: "raw-market-scale".to_owned(),
            branch_id: "main".to_owned(),
            profile_id: "reviewed".to_owned(),
            profile_version: "1.1.0".to_owned(),
            resolved_profile_hash: "profile-hash".to_owned(),
            mapping_classification: "reviewed_mapping".to_owned(),
            parser_engine_version: "compatibility-profile-engine.v2".to_owned(),
            records: vec![MarketWarehouseRecord {
                record_hash: "record-0".to_owned(),
                ordinal: 0,
                record_id: 0,
                year: 1980,
                day: 1,
                game_day: 1,
            }],
            prices: Vec::new(),
            trades: Vec::new(),
            scalars: Vec::new(),
            analytical_trade_history: Vec::new(),
            analytical_price_volatility: Vec::new(),
        };
        warehouse
            .project_market_observation("market:scale", &projection, 1)
            .expect("metadata projection");
        {
            let connection = warehouse.lock().expect("connection");
            connection
                .execute_batch(
                    "INSERT INTO market_records
                     SELECT 'record-' || CAST(i AS VARCHAR), i,
                            1980 + CAST(i / 365 AS INTEGER),
                            CAST(i % 365 AS INTEGER) + 1, i + 1
                     FROM range(1, 2805) values(i);

                     INSERT INTO market_observation_records
                     SELECT 'interpretation-market-scale', 'raw-market-scale', 'main',
                            'record-' || CAST(i AS VARCHAR), i,
                            'reviewed', '1.1.0', 'profile-hash', 'reviewed_mapping'
                     FROM range(1, 2805) values(i);

                     INSERT INTO market_trade_facts
                     SELECT 'record-' || CAST((i % 2804) + 1 AS VARCHAR), 'rub',
                            CASE WHEN i % 2 = 0 THEN 'import' ELSE 'export' END,
                            CASE WHEN i % 4 < 2 THEN 'standard' ELSE 'international' END,
                            'resource_' || CAST(i % 128 AS VARCHAR), 1.0, 2.0,
                            '$SyntheticMarket', CAST(i % 50000 AS BIGINT), 'market.synthetic'
                     FROM range(0, 1000000) values(i);

                     INSERT INTO market_snapshot_trade_facts
                     SELECT 'interpretation-market-scale', 'city', CAST(i AS VARCHAR),
                            'rub', 'export', 'standard', 'resource_city', 1.0, 2.0,
                            '$SyntheticCity', i, 'market.synthetic.city'
                     FROM range(0, 139) values(i);",
                )
                .expect("synthetic rows");
        }

        let started = std::time::Instant::now();
        let loaded = warehouse
            .market_projection(&projection)
            .expect("bounded read")
            .expect("projected");
        let elapsed = started.elapsed();
        assert_eq!(loaded.records.len(), 2_805);
        assert_eq!(loaded.analytical_trade_history.len(), 2_804);
        assert_eq!(
            loaded
                .trades
                .iter()
                .filter(|fact| fact.scope_kind.as_deref() == Some("city"))
                .count(),
            139
        );
        assert!(loaded.trades.len() < 1_000);
        eprintln!("bounded million-row market read completed in {elapsed:?}");
    }

    #[test]
    fn branch_membership_generations_are_idempotent_and_can_project_an_empty_revision() {
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("branches.duckdb"))
            .expect("warehouse");
        let membership = BranchMembershipProjection {
            branch_id: "continuation-test".to_owned(),
            membership_revision: 1,
            interpretation_id: "interpretation-one".to_owned(),
            payload_hash: "payload-one".to_owned(),
            parent_interpretation_id: None,
            relationship: "continuation_anchor".to_owned(),
            shared_record_count: 2,
        };
        warehouse
            .project_branch_memberships(
                "branch_membership:continuation-test:1",
                std::slice::from_ref(&membership),
                "continuation-test",
                1,
                10,
            )
            .expect("first projection");
        warehouse
            .project_branch_memberships(
                "branch_membership:continuation-test:1",
                std::slice::from_ref(&membership),
                "continuation-test",
                1,
                11,
            )
            .expect("duplicate delivery");
        assert_eq!(
            warehouse
                .lock()
                .expect("connection")
                .query_row(
                    "SELECT COUNT(*) FROM current_branch_observation_memberships",
                    [],
                    |row| row.get::<_, u32>(0),
                )
                .expect("current membership"),
            1
        );

        warehouse
            .project_branch_memberships(
                "branch_membership:continuation-test:2",
                &[],
                "continuation-test",
                2,
                12,
            )
            .expect("empty replacement generation");
        assert_eq!(
            warehouse
                .lock()
                .expect("connection")
                .query_row(
                    "SELECT COUNT(*) FROM current_branch_observation_memberships",
                    [],
                    |row| row.get::<_, u32>(0),
                )
                .expect("empty current generation"),
            0
        );
    }

    #[test]
    fn branch_membership_retry_recovers_an_existing_generation_without_a_receipt() {
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("repair.duckdb"))
            .expect("warehouse");
        let membership = BranchMembershipProjection {
            branch_id: "main".to_owned(),
            membership_revision: 7,
            interpretation_id: "interpretation-seven".to_owned(),
            payload_hash: "payload-seven".to_owned(),
            parent_interpretation_id: None,
            relationship: "root".to_owned(),
            shared_record_count: 0,
        };
        {
            let connection = warehouse.lock().expect("connection");
            connection
                .execute(
                    "INSERT INTO branch_membership_generations VALUES('main', 7, 10)",
                    [],
                )
                .expect("orphaned generation");
            connection
                .execute(
                    "INSERT INTO branch_observation_memberships VALUES(
                         'main', 7, 'interpretation-seven', 'payload-seven', NULL, 'root', 0
                     )",
                    [],
                )
                .expect("orphaned membership");
        }

        warehouse
            .project_branch_memberships(
                "branch_membership:main:7",
                std::slice::from_ref(&membership),
                "main",
                7,
                11,
            )
            .expect("repair delivery");

        assert_eq!(
            warehouse
                .lock()
                .expect("connection")
                .query_row(
                    "SELECT COUNT(*) FROM projection_receipts \
                     WHERE projection_id = 'branch_membership:main:7'",
                    [],
                    |row| row.get::<_, u32>(0),
                )
                .expect("repair receipt"),
            1
        );
    }

    #[test]
    fn branch_membership_retry_repairs_a_receipt_written_for_the_wrong_revision() {
        let directory = tempdir().expect("temporary directory");
        let warehouse = AnalyticalWarehouse::initialise(directory.path().join("revision.duckdb"))
            .expect("warehouse");
        let latest = BranchMembershipProjection {
            branch_id: "main".to_owned(),
            membership_revision: 7,
            interpretation_id: "latest".to_owned(),
            payload_hash: "latest-payload".to_owned(),
            parent_interpretation_id: None,
            relationship: "root".to_owned(),
            shared_record_count: 0,
        };
        warehouse
            .project_branch_memberships(
                "branch_membership:main:1",
                std::slice::from_ref(&latest),
                "main",
                7,
                10,
            )
            .expect("simulate old projector defect");

        let first = BranchMembershipProjection {
            membership_revision: 1,
            interpretation_id: "first".to_owned(),
            payload_hash: "first-payload".to_owned(),
            ..latest
        };
        warehouse
            .project_branch_memberships(
                "branch_membership:main:1",
                std::slice::from_ref(&first),
                "main",
                1,
                11,
            )
            .expect("revision-specific retry");

        assert_eq!(
            warehouse
                .lock()
                .expect("connection")
                .query_row(
                    "SELECT COUNT(*) FROM branch_membership_generations \
                     WHERE branch_id = 'main' AND membership_revision IN (1, 7)",
                    [],
                    |row| row.get::<_, u32>(0),
                )
                .expect("both exact generations"),
            2
        );
    }

    #[test]
    fn realistic_observation_projection_completes_as_one_bounded_batch() {
        let _timing_guard = BULK_PROJECTION_TIMING_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
    fn status_snapshots_preserve_confirmed_evidence_during_an_active_warehouse_writer() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("busy.duckdb");
        {
            let warehouse = AnalyticalWarehouse::initialise(path.clone()).expect("warehouse");
            warehouse
                .lock()
                .expect("connection")
                .execute_batch(
                    "INSERT INTO catalogue_generations VALUES(
                         'cached-generation', 'test-build', 'parser.v1', 10,
                         1, 2, 3, 4, 5, 0,
                         'org.example.profile', '1.0.0', 'profile-hash', 'reviewed_mapping'
                     );
                     UPDATE warehouse_metadata SET
                         current_catalogue_generation_id = 'cached-generation',
                         last_catalogue_check_ms = 11,
                         last_catalogue_refresh_ms = 12,
                         last_projection_ms = 13,
                         observation_watermark = 'confirmed-watermark'
                     WHERE singleton_id = 1;",
                )
                .expect("confirmed metadata");
        }
        let warehouse = AnalyticalWarehouse::initialise(path).expect("reopened warehouse");
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
        assert_eq!(
            health.observation_watermark.as_deref(),
            Some("confirmed-watermark")
        );
        assert_eq!(
            generation
                .as_ref()
                .map(|value| value.generation_id.as_str()),
            Some("cached-generation")
        );
        assert_eq!(runtime, Some((Some(11), Some(12), None)));
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
                recipe(
                    "base::recipe::crude-oil",
                    &"3".repeat(64),
                    vec![
                        mapping("production_input", 0, "crude", 1.0, "source_rate"),
                        mapping("production_output", 0, "oil", 1.0, "source_rate"),
                    ],
                ),
                recipe(
                    "base::recipe::bio-oil",
                    &"2".repeat(64),
                    vec![
                        mapping("production_input", 0, "plants", 2.0, "source_rate"),
                        mapping("production_output", 0, "oil", 1.0, "source_rate"),
                    ],
                ),
                recipe(
                    "base::recipe::recycled-oil",
                    &"1".repeat(64),
                    vec![
                        mapping("production_input", 0, "chemicals", 1.0, "source_rate"),
                        mapping("production_output", 0, "oil", 1.0, "source_rate"),
                    ],
                ),
                recipe(
                    "base::recipe::power",
                    &"0".repeat(64),
                    vec![
                        mapping("production_input", 0, "coal", 3.0, "source_rate"),
                        mapping("production_output", 0, "power", 1.0, "source_rate"),
                    ],
                ),
            ],
        };
        assert!(warehouse.publish_catalogue(&generation).expect("publish"));

        let chemical_routes = warehouse
            .search(&CatalogueSearchFilter {
                query: None,
                output_resource_id: Some("resource::chemicals".to_owned()),
                entity_kind: Some("recipe".to_owned()),
                source_kind: None,
                package_query: None,
                coverage: None,
                available_year: None,
                limit: Some(10),
                offset: None,
            })
            .expect("routes filtered by exact output resource");
        assert_eq!(chemical_routes.total, 1);
        assert_eq!(
            chemical_routes.items[0].entity_id,
            "base::recipe::chemical-plant"
        );

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
        assert_eq!(coverage.route_count, 7);
        assert_eq!(coverage.diagrammable_count, 6);
        assert_eq!(coverage.routes_with_auxiliary, 1);
        assert_eq!(coverage.unavailable_count, 1);
        assert_eq!(coverage.relation_count, 16);
        assert_eq!(coverage.auxiliary_relation_count, 1);

        assert!(
            warehouse
                .production_pathway(&ProductionPathwayRequest {
                    root_recipe_entity_id: "base::recipe::chemical-plant".to_owned(),
                    output_resource_id: "resource::chemicals".to_owned(),
                    target_quantity: 10.0,
                    max_depth: 1,
                    selections: Vec::new(),
                })
                .is_err()
        );

        let unresolved = warehouse
            .production_pathway(&ProductionPathwayRequest {
                root_recipe_entity_id: "base::recipe::chemical-plant".to_owned(),
                output_resource_id: "resource::chemicals".to_owned(),
                target_quantity: 10.0,
                max_depth: 4,
                selections: Vec::new(),
            })
            .expect("pathway with an explicit oil-route choice");
        assert_eq!(unresolved.status, "needs_selection");
        assert_eq!(unresolved.choices.len(), 1);
        assert_eq!(unresolved.choices[0].resource_id, "resource::oil");
        assert_eq!(unresolved.choices[0].candidates.len(), 3);
        assert!(unresolved.terminal_requirements.iter().any(|requirement| {
            requirement.resource_id == "resource::coal" && requirement.quantity == 60.0
        }));

        let selected = warehouse
            .production_pathway(&ProductionPathwayRequest {
                root_recipe_entity_id: "base::recipe::chemical-plant".to_owned(),
                output_resource_id: "resource::chemicals".to_owned(),
                target_quantity: 10.0,
                max_depth: 4,
                selections: vec![crate::model::ProductionPathwaySelection {
                    resource_id: "resource::oil".to_owned(),
                    recipe_entity_id: "base::recipe::bio-oil".to_owned(),
                }],
            })
            .expect("selected multi-stage pathway");
        assert_eq!(selected.status, "ready");
        assert_eq!(
            selected
                .nodes
                .iter()
                .filter(|node| node.kind == "process")
                .count(),
            3
        );
        assert_eq!(selected.links.len(), 7);
        assert!(selected.terminal_requirements.iter().any(|requirement| {
            requirement.resource_id == "resource::plants" && requirement.quantity == 80.0
        }));
        assert_eq!(
            selected.snapshot.catalogue_generation_id,
            generation.generation_id
        );

        let cyclic = warehouse
            .production_pathway(&ProductionPathwayRequest {
                root_recipe_entity_id: "base::recipe::chemical-plant".to_owned(),
                output_resource_id: "resource::chemicals".to_owned(),
                target_quantity: 10.0,
                max_depth: 4,
                selections: vec![crate::model::ProductionPathwaySelection {
                    resource_id: "resource::oil".to_owned(),
                    recipe_entity_id: "base::recipe::recycled-oil".to_owned(),
                }],
            })
            .expect("cycle-bounded pathway");
        assert_eq!(cyclic.status, "bounded");
        assert!(cyclic.diagnostics.iter().any(|item| item.code == "cycle"));

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
        let _timing_guard = BULK_PROJECTION_TIMING_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
