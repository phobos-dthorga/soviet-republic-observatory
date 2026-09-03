use rusqlite::{Connection, params};

use super::now_ms;
use crate::error::ObservatoryError;

const MIGRATIONS: &[(i64, &str, &str)] = &[
    (
        1,
        "observation foundation",
        include_str!("../../migrations/0001_observations.sql"),
    ),
    (
        2,
        "branch-aware archive",
        include_str!("../../migrations/0002_branch_archive.sql"),
    ),
    (
        3,
        "compacted history and save-sampled snapshots",
        include_str!("../../migrations/0003_compacted_history_and_snapshots.sql"),
    ),
    (
        4,
        "native recorder candidate ledger",
        include_str!("../../migrations/0004_recorder_ledger.sql"),
    ),
    (
        5,
        "recorder directory baseline state",
        include_str!("../../migrations/0005_recorder_directories.sql"),
    ),
    (
        6,
        "warehouse projection outbox and planning overlays",
        include_str!("../../migrations/0006_warehouse_outbox_and_overlays.sql"),
    ),
    (
        7,
        "analysis pack revisions and lifecycle",
        include_str!("../../migrations/0007_analysis_packs.sql"),
    ),
    (
        8,
        "versioned game compatibility profiles and immutable interpretations",
        include_str!("../../migrations/0008_compatibility_profiles.sql"),
    ),
    (
        9,
        "authoritative language packs and language preference",
        include_str!("../../migrations/0009_language_packs.sql"),
    ),
    (
        10,
        "safe immutable theme revisions and selected theme preference",
        include_str!("../../migrations/0010_theme_revisions.sql"),
    ),
    (
        11,
        "historical analytical heads and continuation memberships",
        include_str!("../../migrations/0011_historical_analysis_contexts.sql"),
    ),
    (
        12,
        "first-class attention cues and native research setup",
        include_str!("../../migrations/0012_attention_and_research_setup.sql"),
    ),
    (
        13,
        "immutable branch-aware republic plans",
        include_str!("../../migrations/0013_republic_plans.sql"),
    ),
    (
        14,
        "source-backed market observations and analytical definitions",
        include_str!("../../migrations/0014_market_observations.sql"),
    ),
    (
        15,
        "market basket and scenario lifecycle",
        include_str!("../../migrations/0015_market_definition_lifecycle.sql"),
    ),
    (
        16,
        "market observation warehouse projection jobs",
        include_str!("../../migrations/0016_market_projection_jobs.sql"),
    ),
    (
        17,
        "parser-engine-aware immutable interpretations",
        include_str!("../../migrations/0017_parser_engine_interpretations.sql"),
    ),
    (
        18,
        "versioned content-addressed market storage",
        include_str!("../../migrations/0018_market_storage_contract.sql"),
    ),
    (
        19,
        "reviewed research source origin",
        include_str!("../../migrations/0019_research_source_origin.sql"),
    ),
    (
        20,
        "content-addressed citizen status history",
        include_str!("../../migrations/0020_broadcast_status_history.sql"),
    ),
    (
        21,
        "Broadcast observation warehouse projection jobs",
        include_str!("../../migrations/0021_broadcast_projection_jobs.sql"),
    ),
    (
        22,
        "content-addressed live resource registry snapshots",
        include_str!("../../migrations/0022_resource_registry_snapshots.sql"),
    ),
    (
        23,
        "environmental observations, live recording state, and carbon factor revisions",
        include_str!("../../migrations/0023_environment_observations.sql"),
    ),
    (
        24,
        "environmental observation warehouse projection jobs",
        include_str!("../../migrations/0024_environment_projection_jobs.sql"),
    ),
    (
        25,
        "recover revision-specific warehouse jobs after environment rollout",
        include_str!("../../migrations/0025_recover_revision_specific_warehouse_jobs.sql"),
    ),
];

pub(crate) fn apply(connection: &mut Connection) -> Result<(), ObservatoryError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (\
             version INTEGER PRIMARY KEY,\
             name TEXT NOT NULL,\
             applied_at_ms INTEGER NOT NULL\
         ) STRICT;",
    )?;

    for (version, name, sql) in MIGRATIONS {
        let applied = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |row| row.get::<_, bool>(0),
        )?;
        if applied {
            continue;
        }
        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, name, applied_at_ms) VALUES(?1, ?2, ?3)",
            params![version, name, now_ms()],
        )?;
        transaction.commit()?;
    }
    backfill_compatibility_provenance(connection)?;
    Ok(())
}

fn backfill_compatibility_provenance(connection: &mut Connection) -> Result<(), ObservatoryError> {
    use crate::compatibility_profile::{PARSER_ENGINE_VERSION, ResolvedCompatibilityProfile};

    let profile = ResolvedCompatibilityProfile::reviewed_builtin()?;
    let document_json = profile.canonical_document_json()?;
    connection.execute(
        "INSERT OR IGNORE INTO compatibility_profile_revisions(\
             profile_id, semantic_version, content_hash, resolved_hash, base_profile_hash,\
             profile_source, mapping_classification, parser_engine_version, document_json,\
             validated_at_ms\
         ) VALUES(?1, ?2, ?3, ?4, NULL, 'reviewed_builtin', 'reviewed_mapping', ?5, ?6, ?7)",
        params![
            profile.id(),
            profile.version(),
            profile.content_hash(),
            profile.resolved_hash(),
            PARSER_ENGINE_VERSION,
            document_json,
            now_ms(),
        ],
    )?;
    let legacy = {
        let mut statement = connection.prepare(
            "SELECT payload_hash FROM observation_sources WHERE interpretation_id IS NULL",
        )?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    let had_legacy_observations = !legacy.is_empty();
    let transaction = connection.transaction()?;
    for raw_payload_hash in legacy {
        let interpretation_id = profile.interpretation_id(&raw_payload_hash);
        transaction.execute(
            r#"UPDATE observation_sources SET
                   raw_payload_hash = ?1,
                   interpretation_id = ?2,
                   profile_id = ?3,
                   profile_semantic_version = ?4,
                   profile_content_hash = ?5,
                   resolved_profile_hash = ?6,
                   base_profile_hash = NULL,
                   profile_source = 'reviewed_builtin',
                   mapping_classification = 'reviewed_mapping',
                   parser_engine_version = ?7
               WHERE payload_hash = ?1 AND interpretation_id IS NULL"#,
            params![
                raw_payload_hash,
                interpretation_id,
                profile.id(),
                profile.version(),
                profile.content_hash(),
                profile.resolved_hash(),
                PARSER_ENGINE_VERSION,
            ],
        )?;
        transaction.execute(
            "UPDATE warehouse_projection_jobs SET source_identity = ?1, status = 'pending', \
                 started_at_ms = NULL, applied_at_ms = NULL, error_code = NULL \
             WHERE projection_kind = 'observation' AND source_identity = ?2",
            params![interpretation_id, raw_payload_hash],
        )?;
    }
    transaction.execute(
        "UPDATE compatibility_runtime_state SET active_resolved_hash = ?1 \
         WHERE singleton_id = 1 AND active_resolved_hash IS NULL",
        [profile.resolved_hash()],
    )?;
    if had_legacy_observations {
        let requested_at_ms = now_ms();
        transaction.execute(
            "INSERT OR IGNORE INTO warehouse_projection_jobs(\
                 projection_id, projection_kind, source_identity, status, requested_at_ms\
             ) VALUES(?1, 'rebuild', 'compatibility_migration', 'pending', ?2)",
            params![
                format!("rebuild:compatibility:{}", &profile.resolved_hash()[..32]),
                requested_at_ms
            ],
        )?;
    }
    transaction.commit()?;
    Ok(())
}
