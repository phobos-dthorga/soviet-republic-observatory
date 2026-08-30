use rusqlite::{Connection, OptionalExtension, params};

use super::{ObservatoryStorage, from_sql_integer};
use crate::error::ObservatoryError;
use crate::model::{
    CoverageStatus, PopulationCitySnapshot, PopulationDataset, PopulationFact,
    PopulationObservation, TesmioProbeStatus,
};

const POPULATION_OBSERVATION_LIMIT: u32 = 256;
const POPULATION_CITY_LIMIT: u32 = 512;

impl ObservatoryStorage {
    pub fn load_population_dataset(&self) -> Result<PopulationDataset, ObservatoryError> {
        let connection = self.connect()?;
        let context = super::analysis_context::load_analysis_context_from(&connection)?;
        let Some(head) = context.head_interpretation_id.as_deref() else {
            return Ok(PopulationDataset {
                analysis_context: context,
                observations: Vec::new(),
                cities: Vec::new(),
                observation_limit: POPULATION_OBSERVATION_LIMIT,
                city_limit: POPULATION_CITY_LIMIT,
                tesmio_probe: TesmioProbeStatus::not_configured(),
            });
        };
        let head_revision = connection
            .query_row(
                "SELECT membership_revision FROM timeline_branch_memberships \
                 WHERE branch_id = ?1 AND interpretation_id = ?2",
                params![context.selected_branch_id, head],
                |row| row.get::<_, u32>(0),
            )
            .optional()?
            .ok_or(ObservatoryError::UnknownObservation)?;
        let observations = load_republic_observations(
            &connection,
            &context.selected_branch_id,
            head_revision,
            context.selected_branch_id == "unassigned",
        )?;
        let cities = load_head_cities(&connection, head)?;
        Ok(PopulationDataset {
            analysis_context: context,
            observations,
            cities,
            observation_limit: POPULATION_OBSERVATION_LIMIT,
            city_limit: POPULATION_CITY_LIMIT,
            tesmio_probe: TesmioProbeStatus::not_configured(),
        })
    }
}

fn coverage(value: &str) -> CoverageStatus {
    if value == "complete" {
        CoverageStatus::Complete
    } else {
        CoverageStatus::Partial
    }
}

fn load_republic_observations(
    connection: &Connection,
    branch_id: &str,
    head_revision: u32,
    exact_head_only: bool,
) -> Result<Vec<PopulationObservation>, ObservatoryError> {
    let mut statement = connection.prepare(
        "WITH recent AS (\
             SELECT membership.interpretation_id, membership.payload_hash, \
                    membership.membership_revision, source.source_file_name, \
                    source.mapping_classification, source.profile_id, \
                    source.profile_semantic_version, source.resolved_profile_hash \
             FROM timeline_branch_memberships membership \
             JOIN observation_sources source \
               ON source.interpretation_id = membership.interpretation_id \
             WHERE membership.branch_id = ?1 AND membership.membership_revision <= ?2 \
               AND membership.membership_revision >= ?3 \
             ORDER BY membership.membership_revision DESC \
             LIMIT ?4\
         ) \
         SELECT recent.interpretation_id, recent.source_file_name, \
                recent.membership_revision, scope.sampled_year, scope.sampled_day, \
                scope.sampled_game_day, scope.coverage_status, \
                recent.mapping_classification, recent.profile_id, \
                recent.profile_semantic_version, recent.resolved_profile_hash, \
                fact.fact_id, fact.value_integer, fact.source_field, fact.source_line \
         FROM recent \
         JOIN snapshot_scopes scope ON scope.payload_hash = recent.payload_hash \
         JOIN snapshot_scalar_facts fact \
           ON fact.payload_hash = scope.payload_hash \
          AND fact.scope_kind = scope.scope_kind AND fact.scope_id = scope.scope_id \
         WHERE scope.scope_kind = 'republic' AND scope.scope_id = 'republic' \
         ORDER BY recent.membership_revision, fact.fact_id",
    )?;
    let minimum_revision = if exact_head_only { head_revision } else { 0 };
    let mut rows = statement.query(params![
        branch_id,
        head_revision,
        minimum_revision,
        POPULATION_OBSERVATION_LIMIT
    ])?;
    let mut observations = Vec::<PopulationObservation>::new();
    while let Some(row) = rows.next()? {
        let interpretation_id = row.get::<_, String>(0)?;
        if observations
            .last()
            .is_none_or(|observation| observation.interpretation_id != interpretation_id)
        {
            observations.push(PopulationObservation {
                interpretation_id,
                source_file_name: row.get(1)?,
                membership_revision: row.get(2)?,
                sampled_year: row.get(3)?,
                sampled_day: row.get(4)?,
                sampled_game_day: row.get(5)?,
                coverage_status: coverage(&row.get::<_, String>(6)?),
                mapping_classification: row.get(7)?,
                profile_id: row.get(8)?,
                profile_version: row.get(9)?,
                resolved_profile_hash: row.get(10)?,
                facts: Vec::new(),
            });
        }
        observations
            .last_mut()
            .expect("an observation was inserted above")
            .facts
            .push(PopulationFact {
                fact_id: row.get(11)?,
                value: from_sql_integer(row.get(12)?)?,
                source_field: row.get(13)?,
                source_line: from_sql_integer(row.get(14)?)?,
            });
    }
    Ok(observations)
}

fn load_head_cities(
    connection: &Connection,
    head_interpretation_id: &str,
) -> Result<Vec<PopulationCitySnapshot>, ObservatoryError> {
    let mut statement = connection.prepare(
        "WITH bounded_scopes AS (\
             SELECT scope.payload_hash, scope.scope_kind, scope.scope_id, \
                    scope.sampled_year, scope.sampled_day, scope.sampled_game_day, \
                    scope.coverage_status \
             FROM observation_sources source \
             JOIN snapshot_scopes scope ON scope.payload_hash = source.payload_hash \
             WHERE source.interpretation_id = ?1 AND scope.scope_kind = 'city' \
             ORDER BY length(scope.scope_id), scope.scope_id \
             LIMIT ?2\
         ) \
         SELECT scope.scope_id, scope.sampled_year, scope.sampled_day, \
                scope.sampled_game_day, scope.coverage_status, fact.fact_id, \
                fact.value_integer, fact.source_field, fact.source_line \
         FROM bounded_scopes scope \
         JOIN snapshot_scalar_facts fact \
           ON fact.payload_hash = scope.payload_hash \
          AND fact.scope_kind = scope.scope_kind AND fact.scope_id = scope.scope_id \
         ORDER BY length(scope.scope_id), scope.scope_id, fact.fact_id",
    )?;
    let mut rows = statement.query(params![head_interpretation_id, POPULATION_CITY_LIMIT])?;
    let mut cities = Vec::<PopulationCitySnapshot>::new();
    while let Some(row) = rows.next()? {
        let scope_id = row.get::<_, String>(0)?;
        if cities
            .last()
            .is_none_or(|snapshot| snapshot.scope_id != scope_id)
        {
            cities.push(PopulationCitySnapshot {
                scope_id,
                sampled_year: row.get(1)?,
                sampled_day: row.get(2)?,
                sampled_game_day: row.get(3)?,
                coverage_status: coverage(&row.get::<_, String>(4)?),
                facts: Vec::new(),
            });
        }
        cities
            .last_mut()
            .expect("a city snapshot was inserted above")
            .facts
            .push(PopulationFact {
                fact_id: row.get(5)?,
                value: from_sql_integer(row.get(6)?)?,
                source_field: row.get(7)?,
                source_line: from_sql_integer(row.get(8)?)?,
            });
    }
    Ok(cities)
}
