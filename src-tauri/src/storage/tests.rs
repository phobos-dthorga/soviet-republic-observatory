use rusqlite::{Connection, params};
use tempfile::tempdir;

use super::ObservatoryStorage;
use crate::automatic_observer::AutomaticObserver;
use crate::model::{
    AnalysisContextMode, AnalysisContextOrigin, CoverageReport, CoverageStatus, ReceiverRecord,
    RecorderCandidateStatus, RecorderDiscoverySource, SNAPSHOT_FACTS, SaveInspection, SaveSnapshot,
    SnapshotFact, SnapshotScopeKind, SourceFieldSet, SourceLineSet,
};

#[test]
fn exact_historical_preview_excludes_later_states_and_returns_to_the_proven_tip() {
    let directory = tempdir().expect("temporary directory");
    let storage = ObservatoryStorage::initialise(directory.path().join("historical.sqlite3"))
        .expect("storage");
    storage
        .save_inspection(&inspection("anchor-state", "anchor.zip", &[1, 2]))
        .expect("anchor");
    storage
        .save_inspection(&inspection("later-state", "later.zip", &[1, 2, 3]))
        .expect("later");

    storage
        .inspect_observation("anchor-state")
        .expect("inspect anchor");
    let archive = storage.load_archive_overview().expect("historical archive");
    assert_eq!(
        archive.analysis_context.mode,
        AnalysisContextMode::HistoricalPreview
    );
    assert!(!archive.analysis_context.is_tip);
    assert_eq!(
        archive.analysis_context.head_interpretation_id.as_deref(),
        Some("anchor-state")
    );
    assert!(archive.observations.iter().any(|observation| {
        observation.interpretation_id == "anchor-state"
            && observation.included_in_context
            && observation.active_head
    }));
    assert!(archive.observations.iter().any(|observation| {
        observation.interpretation_id == "later-state" && !observation.included_in_context
    }));
    assert_eq!(
        storage
            .load_latest_dataset()
            .expect("dataset")
            .expect("head")
            .payload_hash,
        "anchor-state"
    );

    storage.return_to_branch_tip().expect("return to tip");
    assert_eq!(
        storage
            .load_latest_dataset()
            .expect("dataset")
            .expect("tip")
            .payload_hash,
        "later-state"
    );
}

#[test]
fn population_dataset_is_bounded_to_the_exact_analysis_head_and_keeps_source_evidence() {
    let directory = tempdir().expect("temporary directory");
    let storage = ObservatoryStorage::initialise(directory.path().join("population.sqlite3"))
        .expect("storage");
    storage
        .save_inspection(&population_inspection(
            "population-anchor",
            "anchor.zip",
            &[1, 2],
            120,
            7,
        ))
        .expect("anchor");
    storage
        .save_inspection(&population_inspection(
            "population-later",
            "later.zip",
            &[1, 2, 3],
            135,
            11,
        ))
        .expect("later");

    let latest = storage
        .load_population_dataset()
        .expect("latest population");
    assert_eq!(latest.observations.len(), 2);
    assert_eq!(latest.cities.len(), 1);
    assert_eq!(latest.cities[0].scope_id, "17");
    assert_eq!(latest.cities[0].facts[0].source_line, 211);
    assert_eq!(
        latest.observations[1]
            .facts
            .iter()
            .find(|fact| fact.fact_id == "source.stats.citizens.small_children")
            .map(|fact| fact.value),
        Some(135)
    );

    storage
        .inspect_observation("population-anchor")
        .expect("inspect anchor");
    let preview = storage
        .load_population_dataset()
        .expect("historical population");
    assert_eq!(
        preview.analysis_context.mode,
        AnalysisContextMode::HistoricalPreview
    );
    assert_eq!(preview.observations.len(), 1);
    assert_eq!(
        preview.observations[0].interpretation_id,
        "population-anchor"
    );
    assert_eq!(
        preview.cities[0]
            .facts
            .iter()
            .find(|fact| fact.fact_id == "source.stats.citizens.born")
            .map(|fact| fact.value),
        Some(7)
    );
}

#[test]
fn population_dataset_never_splices_unrelated_unassigned_histories() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("unassigned-population.sqlite3"))
            .expect("storage");
    storage
        .save_inspection(&population_inspection(
            "main-population",
            "main.zip",
            &[1, 2],
            120,
            7,
        ))
        .expect("main");
    storage
        .save_inspection(&population_inspection(
            "unrelated-population-a",
            "unrelated-a.zip",
            &[90, 91],
            900,
            70,
        ))
        .expect("first unrelated");
    storage
        .save_inspection(&population_inspection(
            "unrelated-population-b",
            "unrelated-b.zip",
            &[80, 81],
            800,
            60,
        ))
        .expect("second unrelated");

    storage
        .select_branch("unassigned")
        .expect("select unresolved histories");
    let dataset = storage.load_population_dataset().expect("population");
    assert_eq!(dataset.analysis_context.selected_branch_id, "unassigned");
    assert_eq!(dataset.observations.len(), 1);
    assert_eq!(
        dataset.observations[0].interpretation_id,
        dataset
            .analysis_context
            .head_interpretation_id
            .as_deref()
            .expect("selected head")
    );
}

#[test]
fn continuations_are_durable_reusable_forks_and_only_strict_descendants_advance_them() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("continuations.sqlite3");
    let first_branch;
    {
        let storage = ObservatoryStorage::initialise(path.clone()).expect("storage");
        storage
            .save_inspection(&inspection("anchor-state", "anchor.zip", &[1, 2]))
            .expect("anchor");
        storage
            .save_inspection(&inspection("abandoned-future", "future.zip", &[1, 2, 3]))
            .expect("future");
        first_branch = storage
            .create_continuation("anchor-state", Some("Steel-first continuation"))
            .expect("first continuation");
        let second_branch = storage
            .create_continuation("anchor-state", None)
            .expect("second continuation");
        assert_ne!(first_branch, second_branch);
        let archive = storage.load_archive_overview().expect("archive");
        let manual = archive
            .branches
            .iter()
            .filter(|branch| branch.origin == AnalysisContextOrigin::ManualContinuation)
            .collect::<Vec<_>>();
        assert_eq!(manual.len(), 2);
        assert_eq!(
            manual
                .iter()
                .find(|branch| branch.branch_id == first_branch)
                .and_then(|branch| branch.parent_branch_id.as_deref()),
            Some("main")
        );
        assert_eq!(
            manual
                .iter()
                .find(|branch| branch.branch_id == second_branch)
                .and_then(|branch| branch.parent_branch_id.as_deref()),
            Some(first_branch.as_str())
        );

        storage
            .select_branch(&first_branch)
            .expect("select first continuation");
        storage
            .save_inspection(&inspection("continued-state", "continued.zip", &[1, 2, 9]))
            .expect("strict continuation descendant");
        let dataset = storage
            .load_latest_dataset()
            .expect("dataset")
            .expect("continued head");
        assert_eq!(dataset.payload_hash, "continued-state");
        assert_eq!(dataset.branch_id, first_branch);
        assert_ne!(dataset.original_branch_id, dataset.branch_id);

        let nested_branch = storage
            .create_continuation("continued-state", Some("Nested continuation"))
            .expect("nested continuation");
        assert_eq!(
            storage
                .load_archive_overview()
                .expect("nested archive")
                .branches
                .iter()
                .find(|branch| branch.branch_id == nested_branch)
                .and_then(|branch| branch.parent_branch_id.as_deref()),
            Some(first_branch.as_str())
        );
        storage
            .select_branch(&first_branch)
            .expect("restore first continuation");

        storage
            .save_inspection(&inspection("unrelated-state", "unrelated.zip", &[7, 8]))
            .expect("unrelated observation");
        assert_eq!(
            storage
                .load_latest_dataset()
                .expect("dataset")
                .expect("unchanged head")
                .payload_hash,
            "continued-state"
        );
        storage
            .set_branch_label(&first_branch, Some("Renamed continuation"))
            .expect("rename");
    }

    let reopened = ObservatoryStorage::initialise(path).expect("reopen");
    let archive = reopened.load_archive_overview().expect("persisted archive");
    assert_eq!(archive.selected_branch_id, first_branch);
    assert_eq!(
        archive.analysis_context.origin,
        AnalysisContextOrigin::ManualContinuation
    );
    assert_eq!(
        archive.analysis_context.head_interpretation_id.as_deref(),
        Some("continued-state")
    );
    assert_eq!(
        archive
            .branches
            .iter()
            .find(|branch| branch.branch_id == first_branch)
            .and_then(|branch| branch.player_label.as_deref()),
        Some("Renamed continuation")
    );
}

#[test]
fn stores_normalised_metrics_and_separates_files_from_distinct_states() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");
    let first = inspection("aaa111", "first.zip", &[1, 2, 3]);

    assert!(storage.save_inspection(&first).expect("first import"));
    assert!(!storage.save_inspection(&first).expect("same file"));
    let second_file = inspection("aaa111", "copy.zip", &[1, 2, 3]);
    assert!(!storage.save_inspection(&second_file).expect("copied state"));

    assert_eq!(storage.distinct_state_count().expect("distinct count"), 1);
    assert_eq!(storage.file_observation_count().expect("file count"), 2);
    let dataset = storage
        .load_latest_dataset()
        .expect("load")
        .expect("dataset");
    assert_eq!(dataset.points.len(), 3);
    assert_eq!(dataset.source_fields.len(), 4);
    assert_eq!(dataset.branch_id, "main");
}

#[test]
fn reinterpretation_is_idempotent_per_profile_and_immutable_across_profiles() {
    let directory = tempdir().expect("temporary directory");
    let storage = ObservatoryStorage::initialise(directory.path().join("reinterpret.sqlite3"))
        .expect("storage");
    let reviewed = inspection("same-raw-save", "state.zip", &[1, 2, 3]);
    assert!(storage.save_inspection(&reviewed).expect("reviewed import"));
    assert!(
        !storage
            .save_reinterpretation(&reviewed)
            .expect("same profile remains idempotent")
    );

    let mut local = reviewed.clone();
    local.interpretation_id = "local-profile-interpretation".to_owned();
    local.compatibility.profile_id = "local.republic-observatory.override".to_owned();
    local.compatibility.profile_version = "1.0.0".to_owned();
    local.compatibility.profile_content_hash = "b".repeat(64);
    local.compatibility.resolved_profile_hash = "c".repeat(64);
    local.compatibility.base_profile_hash = Some(reviewed.compatibility.profile_content_hash);
    local.compatibility.profile_source = "local_override".to_owned();
    local.compatibility.mapping_classification = "player_mapped".to_owned();
    assert!(
        storage
            .save_reinterpretation(&local)
            .expect("alternate profile interpretation")
    );

    assert_eq!(storage.file_observation_count().expect("file count"), 1);
    assert_eq!(storage.distinct_state_count().expect("state count"), 2);
    let archive = storage.load_archive_overview().expect("archive");
    assert_eq!(archive.observations.len(), 2);
    assert!(archive.observations.iter().any(|observation| {
        observation.interpretation_id == "local-profile-interpretation"
            && observation.mapping_classification == "player_mapped"
    }));
}

#[test]
fn interrupted_warehouse_jobs_return_to_pending_without_losing_the_observation() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("recovery.sqlite3");
    {
        let storage = ObservatoryStorage::initialise(path.clone()).expect("storage");
        storage
            .save_inspection(&inspection("projection-state", "state.zip", &[1, 2]))
            .expect("observation");
        let claimed = storage
            .claim_projection_job()
            .expect("claim")
            .expect("pending job");
        assert_eq!(claimed.source_identity, "projection-state");
        assert_eq!(
            storage
                .projection_job_status(&claimed.projection_id)
                .expect("running status")
                .as_deref(),
            Some("running")
        );
    }
    let reopened = ObservatoryStorage::initialise(path).expect("reopen storage");
    assert_eq!(
        reopened
            .projection_job_status("observation:projection-state")
            .expect("recovered status")
            .as_deref(),
        Some("pending")
    );
    assert_eq!(reopened.distinct_state_count().expect("state count"), 1);
}

#[test]
fn failed_projection_is_visible_and_rebuild_redelivers_retained_observations() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("retry.sqlite3")).expect("storage");
    storage
        .save_inspection(&inspection("warehouse-outage", "outage.zip", &[1, 2]))
        .expect("observation remains committed");
    let job = storage
        .claim_projection_job()
        .expect("claim")
        .expect("observation projection");
    storage
        .fail_projection_job(&job.projection_id, "warehouse_unavailable")
        .expect("record analytical failure");
    let failed = storage.projection_queue_status().expect("queue health");
    assert_eq!(failed.failed_jobs, 1);
    assert_eq!(storage.distinct_state_count().expect("retained state"), 1);

    storage
        .enqueue_warehouse_rebuild()
        .expect("request rebuild");
    let retry = storage.projection_queue_status().expect("retry health");
    assert_eq!(retry.failed_jobs, 0);
    assert_eq!(retry.pending_jobs, 4);
    assert!(retry.oldest_unresolved_at_ms.is_some());
}

#[test]
fn strict_prefix_successors_remain_on_main() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");

    storage
        .save_inspection(&inspection("main-one", "one.zip", &[1, 2]))
        .expect("root");
    storage
        .save_inspection(&inspection("main-two", "two.zip", &[1, 2, 3]))
        .expect("successor");

    let archive = storage.load_archive_overview().expect("archive");
    assert_eq!(archive.distinct_state_count, 2);
    assert_eq!(archive.branches.len(), 1);
    assert_eq!(archive.branches[0].branch_id, "main");
    assert_eq!(archive.branches[0].observation_count, 2);
    assert_eq!(archive.observations[0].relationship, "successor");
    assert_eq!(archive.observations[0].shared_record_count, 2);
}

#[test]
fn strict_prefixes_share_compacted_history_nodes() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");

    storage
        .save_inspection(&inspection("first-state", "first.zip", &[1, 2]))
        .expect("first state");
    storage
        .save_inspection(&inspection("second-state", "second.zip", &[1, 2, 3]))
        .expect("second state");

    let connection = storage.connect().expect("connection");
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM receiver_history_nodes", [], |row| row
                .get::<_, u32>(0))
            .expect("node count"),
        3
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM observation_history_tips", [], |row| {
                row.get::<_, u32>(0)
            })
            .expect("tip count"),
        2
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM embedded_records", [], |row| row
                .get::<_, u32>(0))
            .expect("legacy row count"),
        0
    );
}

#[test]
fn stores_save_sampled_republic_and_city_facts() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");
    let mut save = inspection("snapshot-state", "snapshot.zip", &[1, 2]);
    save.snapshots = vec![
        SaveSnapshot {
            scope_kind: SnapshotScopeKind::Republic,
            scope_id: "republic".to_owned(),
            facts: vec![snapshot_fact(
                "core.citizens.electronics.radio",
                "$Citizens_EletrinicRadio",
                22,
            )],
            expected_fact_count: 18,
            coverage: CoverageStatus::Partial,
        },
        SaveSnapshot {
            scope_kind: SnapshotScopeKind::City,
            scope_id: "7".to_owned(),
            facts: vec![snapshot_fact(
                "source.stats.citizens.born",
                "$Citizens_Born",
                3,
            )],
            expected_fact_count: 5,
            coverage: CoverageStatus::Partial,
        },
    ];

    storage.save_inspection(&save).expect("snapshot import");
    let archive = storage.load_archive_overview().expect("archive");
    assert_eq!(archive.observations[0].republic_snapshot_fields, 1);
    assert_eq!(archive.observations[0].city_snapshot_count, 1);
    assert_eq!(archive.observations[0].city_snapshot_fields, 1);
    let connection = storage.connect().expect("connection");
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM snapshot_scalar_facts", [], |row| row
                .get::<_, u32>(
                0
            ))
            .expect("fact count"),
        2
    );
}

#[test]
fn reobserving_a_legacy_state_backfills_its_snapshots_without_a_new_state() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");
    let original = inspection("legacy-snapshot-state", "legacy.zip", &[1, 2]);
    storage.save_inspection(&original).expect("legacy import");
    let mut reobserved = original;
    reobserved.source_file_name = "legacy-copy.zip".to_owned();
    reobserved.snapshots = vec![SaveSnapshot {
        scope_kind: SnapshotScopeKind::Republic,
        scope_id: "republic".to_owned(),
        facts: vec![snapshot_fact(
            "core.citizens.electronics.radio",
            "$Citizens_EletrinicRadio",
            22,
        )],
        expected_fact_count: 18,
        coverage: CoverageStatus::Partial,
    }];

    assert!(
        !storage
            .save_inspection(&reobserved)
            .expect("snapshot backfill")
    );
    assert_eq!(storage.distinct_state_count().expect("state count"), 1);
    assert_eq!(
        storage
            .load_archive_overview()
            .expect("archive")
            .observations[0]
            .republic_snapshot_fields,
        1
    );
}

#[test]
fn compares_two_distinct_states_only_within_one_resolved_branch() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");
    storage
        .save_inspection(&inspection("from-state", "from.zip", &[1, 2]))
        .expect("from state");
    storage
        .save_inspection(&inspection("to-state", "to.zip", &[1, 2, 3]))
        .expect("to state");

    let comparison = storage
        .compare_observations("from-state", "to-state")
        .expect("comparison");
    assert_eq!(comparison.branch_id, "main");
    assert_eq!(comparison.elapsed_game_days, 1);
    assert!(
        comparison
            .receiver_changes
            .iter()
            .all(|change| change.delta == 1)
    );
    assert_eq!(comparison.classified_total_change.delta, 4);
    assert_eq!(
        storage
            .compare_observations("from-state", "from-state")
            .expect_err("same state must fail")
            .code(),
        "same_observation_comparison"
    );

    storage
        .save_inspection(&inspection("unrelated", "unrelated.zip", &[9, 10]))
        .expect("unrelated state");
    assert_eq!(
        storage
            .compare_observations("from-state", "unrelated")
            .expect_err("cross-branch comparison must fail")
            .code(),
        "incompatible_comparison"
    );
}

#[test]
fn extending_an_older_state_after_an_incompatible_tip_creates_a_fork() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");

    storage
        .save_inspection(&inspection("root-state", "root.zip", &[1, 2]))
        .expect("root");
    storage
        .save_inspection(&inspection("long-state", "long.zip", &[1, 2, 3, 4]))
        .expect("main tip");
    storage
        .save_inspection(&inspection("fork-state", "fork.zip", &[1, 2, 9]))
        .expect("fork");

    let archive = storage.load_archive_overview().expect("archive");
    assert_eq!(archive.branches.len(), 2);
    assert_eq!(archive.selected_branch_id, "main");
    let fork = archive
        .branches
        .iter()
        .find(|branch| branch.branch_kind == "fork")
        .expect("fork branch");
    assert_eq!(fork.parent_branch_id.as_deref(), Some("main"));
    assert_eq!(fork.fork_record_id, Some(1));
    assert_eq!(archive.observations[0].relationship, "rollback_fork");
    assert_eq!(
        archive.observations[0].parent_payload_hash.as_deref(),
        Some("root-state")
    );
}

#[test]
fn partial_divergence_without_an_observed_fork_point_stays_explicit() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");

    storage
        .save_inspection(&inspection("main-state", "main.zip", &[1, 2, 3]))
        .expect("main");
    storage
        .save_inspection(&inspection("diverged-state", "diverged.zip", &[1, 2, 8]))
        .expect("divergence");

    let archive = storage.load_archive_overview().expect("archive");
    assert_eq!(archive.observations[0].relationship, "divergent_fork");
    assert_eq!(archive.observations[0].shared_record_count, 2);
    assert!(archive.observations[0].parent_payload_hash.is_none());
}

#[test]
fn selecting_a_branch_changes_the_latest_dataset_without_rewriting_history() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");

    storage
        .save_inspection(&inspection("main-state", "main.zip", &[1, 2, 3]))
        .expect("main");
    storage
        .save_inspection(&inspection("fork-state", "fork.zip", &[1, 2]))
        .expect("rollback");
    assert_eq!(
        storage
            .load_latest_dataset()
            .expect("persistent main context")
            .expect("main dataset")
            .payload_hash,
        "main-state"
    );

    storage
        .select_branch("fork-fork-state")
        .expect("select fork");
    assert_eq!(
        storage
            .load_latest_dataset()
            .expect("selected fork")
            .expect("fork dataset")
            .payload_hash,
        "fork-state"
    );

    storage.select_branch("main").expect("select main");
    assert_eq!(
        storage
            .load_latest_dataset()
            .expect("latest main")
            .expect("main dataset")
            .payload_hash,
        "main-state"
    );
    assert_eq!(storage.distinct_state_count().expect("count"), 2);

    storage
        .save_inspection(&inspection("fork-state", "fork.zip", &[1, 2]))
        .expect("observe known fork");
    assert_eq!(
        storage
            .load_latest_dataset()
            .expect("persistent selected branch")
            .expect("main dataset")
            .payload_hash,
        "main-state"
    );
}

#[test]
fn unknown_branch_selection_is_rejected() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");

    let error = storage
        .select_branch("not-a-branch")
        .expect_err("unknown branch");
    assert_eq!(error.code(), "unknown_branch");
}

#[test]
fn unrelated_history_remains_unassigned() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("test.sqlite3")).expect("storage");

    storage
        .save_inspection(&inspection("main-state", "main.zip", &[1, 2]))
        .expect("main");
    storage
        .save_inspection(&inspection("unknown-state", "unknown.zip", &[7, 8]))
        .expect("unrelated");

    let archive = storage.load_archive_overview().expect("archive");
    assert_eq!(archive.unresolved_state_count, 1);
    assert_eq!(archive.selected_branch_id, "main");
    let unresolved = archive
        .observations
        .iter()
        .find(|observation| observation.payload_hash == "unknown-state")
        .expect("unresolved observation");
    assert_eq!(unresolved.branch_id, "unassigned");
    assert_eq!(unresolved.relationship, "ambiguous");
}

#[test]
fn version_one_database_is_migrated_and_backfilled_without_reimport() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("version-one.sqlite3");
    let connection = Connection::open(&path).expect("version one connection");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE schema_migrations (
                 version INTEGER PRIMARY KEY,
                 name TEXT NOT NULL,
                 applied_at_ms INTEGER NOT NULL
             ) STRICT;",
        )
        .expect("migration catalogue");
    connection
        .execute_batch(include_str!("../../migrations/0001_observations.sql"))
        .expect("version one schema");
    connection
        .execute(
            "INSERT INTO schema_migrations(version, name, applied_at_ms)
             VALUES(1, 'observation foundation', 1)",
            [],
        )
        .expect("version one migration row");
    connection
        .execute(
            "INSERT INTO observation_sources(
                 payload_hash, source_file_name, source_file_size, source_modified_ms,
                 imported_at_ms, parser_version, format_profile, branch_id, geographic_scope,
                 coverage_status, history_records, chartable_records, dropped_records, warnings_json
             ) VALUES('legacy-state', 'legacy.zip', 100, 1, 2, 'parser', 'profile',
                      'unassigned', 'republic', 'complete', 1, 1, 0, '[]')",
            [],
        )
        .expect("legacy source");
    connection
        .execute(
            "INSERT INTO embedded_records(
                 payload_hash, record_id, year, day, game_day, classified_total
             ) VALUES('legacy-state', 0, 2000, 1, 0, 104)",
            [],
        )
        .expect("legacy record");
    let legacy_source_fields = [
        "$Citizens_EletronicNone",
        "$Citizens_EletrinicRadio",
        "$Citizens_EletronicTV",
        "$Citizens_EletronicComputer",
    ];
    for (index, metric) in crate::model::RECEIVER_METRICS.iter().enumerate() {
        connection
            .execute(
                "INSERT INTO metric_observations(
                     payload_hash, record_id, metric_id, value_integer, source_field,
                     source_line, evidence_kind, coverage
                 ) VALUES('legacy-state', 0, ?1, ?2, ?3, ?4, 'save_fact', 'complete')",
                params![
                    metric.id,
                    11 + index as i64 * 10,
                    legacy_source_fields[index],
                    index as i64 + 1
                ],
            )
            .expect("legacy metric");
    }
    drop(connection);

    let storage = ObservatoryStorage::initialise(path).expect("migrated storage");
    let archive = storage.load_archive_overview().expect("archive");
    assert_eq!(archive.file_observation_count, 1);
    assert_eq!(archive.distinct_state_count, 1);
    assert_eq!(archive.unresolved_state_count, 0);
    assert_eq!(archive.selected_branch_id, "main");
    assert_eq!(archive.observations[0].relationship, "root");
    assert_ne!(archive.observations[0].interpretation_id, "legacy-state");
    assert_eq!(
        archive.observations[0].profile_id,
        "org.republic-observatory.wrsr-1.1.1.9"
    );
    let migrated = storage.connect().expect("migration evidence");
    assert_eq!(
        migrated
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, u32>(0)
            })
            .expect("latest migration"),
        17
    );
    assert_eq!(
        migrated
            .query_row(
                "SELECT COUNT(*) FROM warehouse_projection_jobs \
                 WHERE projection_kind = 'rebuild' AND status = 'pending'",
                [],
                |row| row.get::<_, u32>(0),
            )
            .expect("compatibility rebuild"),
        1
    );
}

#[test]
fn version_fifteen_projection_queue_accepts_market_jobs_after_upgrade() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("version-fifteen.sqlite3");
    let storage = ObservatoryStorage::initialise(path.clone()).expect("current storage");
    let connection = storage.connect().expect("current connection");
    connection
        .execute(
            "INSERT INTO warehouse_projection_jobs(\
                 projection_id, projection_kind, source_identity, status, requested_at_ms\
             ) VALUES('observation:preserved', 'observation', 'preserved', 'pending', 1)",
            [],
        )
        .expect("preserved projection");
    connection
        .execute("DELETE FROM schema_migrations WHERE version >= 16", [])
        .expect("remove current migration markers");
    connection
        .execute_batch(
            "DROP INDEX warehouse_projection_jobs_queue;
             ALTER TABLE warehouse_projection_jobs RENAME TO warehouse_projection_jobs_v16;
             CREATE TABLE warehouse_projection_jobs (
                 projection_id TEXT PRIMARY KEY
                     CHECK (length(projection_id) BETWEEN 3 AND 160),
                 projection_kind TEXT NOT NULL CHECK (
                     projection_kind IN (
                         'observation', 'overlay_state', 'branch_membership', 'rebuild'
                     )
                 ),
                 source_identity TEXT NOT NULL
                     CHECK (length(source_identity) BETWEEN 1 AND 256),
                 status TEXT NOT NULL CHECK (
                     status IN ('pending', 'running', 'applied', 'failed')
                 ),
                 requested_at_ms INTEGER NOT NULL,
                 started_at_ms INTEGER,
                 applied_at_ms INTEGER,
                 attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
                 error_code TEXT,
                 CHECK (
                     (status = 'applied' AND applied_at_ms IS NOT NULL) OR
                     status <> 'applied'
                 )
             ) STRICT;
             INSERT INTO warehouse_projection_jobs
             SELECT * FROM warehouse_projection_jobs_v16;
             DROP TABLE warehouse_projection_jobs_v16;
             CREATE INDEX warehouse_projection_jobs_queue
                 ON warehouse_projection_jobs(status, requested_at_ms, projection_id);",
        )
        .expect("version fifteen projection queue");
    assert!(
        connection
            .execute(
                "INSERT INTO warehouse_projection_jobs(\
                     projection_id, projection_kind, source_identity, status, requested_at_ms\
                 ) VALUES('market:blocked', 'market_observation', 'blocked', 'pending', 2)",
                [],
            )
            .is_err(),
        "the version fifteen queue must reproduce the missing market kind"
    );
    drop(connection);
    drop(storage);

    let migrated = ObservatoryStorage::initialise(path).expect("migrated storage");
    let connection = migrated.connect().expect("migrated connection");
    assert_eq!(
        connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, u32>(0)
            })
            .expect("latest migration"),
        17
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT projection_kind FROM warehouse_projection_jobs \
                 WHERE projection_id = 'observation:preserved'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("preserved projection kind"),
        "observation"
    );
    connection
        .execute(
            "INSERT INTO warehouse_projection_jobs(\
                 projection_id, projection_kind, source_identity, status, requested_at_ms\
             ) VALUES('market:accepted', 'market_observation', 'accepted', 'pending', 3)",
            [],
        )
        .expect("market projection after migration");
}

#[test]
fn parser_engine_upgrade_can_add_an_immutable_interpretation_for_the_same_raw_save() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("parser-engine-upgrade.sqlite3");
    let storage = ObservatoryStorage::initialise(path.clone()).expect("current storage");

    let mut first = inspection("engine-v1", "same-save.zip", &[1, 2, 3]);
    first.payload_hash = "same-raw-save".to_owned();
    first.compatibility.parser_engine_version = "compatibility-profile-engine.v1".to_owned();
    storage
        .save_inspection(&first)
        .expect("first interpretation");

    let connection = storage.connect().expect("current connection");
    connection
        .execute_batch(
            "DELETE FROM schema_migrations WHERE version = 17;
             DROP INDEX observation_sources_raw_engine_profile;
             CREATE UNIQUE INDEX observation_sources_raw_profile
                 ON observation_sources(raw_payload_hash, resolved_profile_hash);",
        )
        .expect("legacy interpretation uniqueness");

    let mut second = inspection("engine-v2", "same-save.zip", &[1, 2, 3]);
    second.payload_hash = "same-raw-save".to_owned();
    second.compatibility.parser_engine_version = "compatibility-profile-engine.v2".to_owned();
    let blocked = storage
        .save_reinterpretation(&second)
        .expect_err("legacy uniqueness must reproduce the storage contract failure");
    assert_eq!(blocked.code(), "storage_contract_violation");
    drop(connection);
    drop(storage);

    let repaired = ObservatoryStorage::initialise(path).expect("repaired storage");
    repaired
        .save_reinterpretation(&second)
        .expect("new parser interpretation");
    let connection = repaired.connect().expect("repaired connection");
    let interpretations = connection
        .query_row(
            "SELECT COUNT(*) FROM observation_sources
             WHERE raw_payload_hash = 'same-raw-save'",
            [],
            |row| row.get::<_, u32>(0),
        )
        .expect("interpretation count");
    assert_eq!(interpretations, 2);
    let engines = connection
        .prepare(
            "SELECT parser_engine_version FROM observation_sources
             WHERE raw_payload_hash = 'same-raw-save'
             ORDER BY parser_engine_version",
        )
        .expect("engine query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("engine rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("engine values");
    assert_eq!(
        engines,
        vec![
            "compatibility-profile-engine.v1".to_owned(),
            "compatibility-profile-engine.v2".to_owned(),
        ]
    );
}

#[test]
fn attention_cues_are_revision_specific_and_replayable() {
    let directory = tempdir().expect("temporary directory");
    let storage = ObservatoryStorage::initialise(directory.path().join("attention.sqlite3"))
        .expect("storage");

    assert!(
        !storage
            .attention_cue_dismissed("research.setup.entry", 1)
            .expect("status")
    );
    storage
        .dismiss_attention_cue("research.setup.entry", 1)
        .expect("dismiss");
    assert!(
        storage
            .attention_cue_dismissed("research.setup.entry", 1)
            .expect("status")
    );
    assert!(
        !storage
            .attention_cue_dismissed("research.setup.entry", 2)
            .expect("new revision")
    );
    storage
        .replay_attention_cue("research.setup.entry", 1)
        .expect("replay");
    assert!(
        !storage
            .attention_cue_dismissed("research.setup.entry", 1)
            .expect("replayed")
    );
    assert!(storage.dismiss_attention_cue("Unsafe cue", 1).is_err());
}

#[test]
fn research_setup_state_persists_consent_checkout_and_build_identity() {
    let directory = tempdir().expect("temporary directory");
    let database_path = directory.path().join("research.sqlite3");
    let checkout = directory.path().join("reviewed-checkout");
    let storage = ObservatoryStorage::initialise(database_path.clone()).expect("storage");
    storage.set_research_notice_revision(1).expect("notice");
    storage
        .set_research_tesmio_checkout(&checkout)
        .expect("checkout");
    let hash = "a".repeat(64);
    storage
        .record_research_probe_build(&hash)
        .expect("build identity");
    drop(storage);

    let reopened = ObservatoryStorage::initialise(database_path).expect("reopened storage");
    let setup = reopened.research_setup().expect("setup");
    assert_eq!(setup.accepted_notice_revision, 1);
    assert_eq!(
        setup.tesmio_checkout_path.as_deref(),
        Some(checkout.as_path())
    );
    assert_eq!(setup.last_probe_hash.as_deref(), Some(hash.as_str()));
    assert!(setup.last_built_at_ms.is_some());
}

fn inspection(hash: &str, file_name: &str, values: &[u64]) -> SaveInspection {
    let records = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let base_line = (index as u64 * 5) + 1;
            ReceiverRecord {
                record_id: index as u32,
                year: 2000 + (index / 365) as i32,
                day: (index % 365) as u16,
                game_day: index as i64,
                none: value + 10,
                radio: value + 20,
                television: value + 30,
                computer: value + 40,
                classified_total: (value * 4) + 100,
                source_lines: SourceLineSet {
                    none: base_line,
                    radio: base_line + 1,
                    television: base_line + 2,
                    computer: base_line + 3,
                },
                source_fields: SourceFieldSet {
                    none: "$Citizens_EletronicNone".to_owned(),
                    radio: "$Citizens_EletrinicRadio".to_owned(),
                    television: "$Citizens_EletronicTV".to_owned(),
                    computer: "$Citizens_EletronicComputer".to_owned(),
                },
            }
        })
        .collect::<Vec<_>>();
    SaveInspection {
        payload_hash: hash.to_owned(),
        interpretation_id: hash.to_owned(),
        compatibility:
            crate::compatibility_profile::ResolvedCompatibilityProfile::reviewed_builtin()
                .expect("profile")
                .provenance(),
        source_file_name: file_name.to_owned(),
        source_file_size: 100,
        source_modified_ms: 1,
        source_directory_identity: "fixture-directory".to_owned(),
        coverage: CoverageReport {
            status: CoverageStatus::Complete,
            history_records: records.len() as u32,
            chartable_records: records.len() as u32,
            dropped_records: 0,
            warnings: Vec::new(),
        },
        records,
        snapshots: Vec::new(),
        market: crate::model::ParsedMarketData::default(),
        binary_facts: Vec::new(),
    }
}

fn snapshot_fact(fact_id: &'static str, source_field: &'static str, value: u64) -> SnapshotFact {
    SnapshotFact {
        fact_id: fact_id.to_owned(),
        source_field: source_field.to_owned(),
        value,
        source_line: 100,
    }
}

fn population_inspection(
    hash: &str,
    file_name: &str,
    history: &[u64],
    small_children: u64,
    city_births: u64,
) -> SaveInspection {
    let mut result = inspection(hash, file_name, history);
    result.snapshots = vec![
        SaveSnapshot {
            scope_kind: SnapshotScopeKind::Republic,
            scope_id: "republic".to_owned(),
            facts: vec![
                snapshot_fact(
                    "source.stats.citizens.small_children",
                    "$Citizens_SmallChilds",
                    small_children,
                ),
                snapshot_fact(
                    "source.stats.citizens.unemployed",
                    "$Citizens_Unemployed",
                    small_children / 2,
                ),
            ],
            expected_fact_count: 18,
            coverage: CoverageStatus::Partial,
        },
        SaveSnapshot {
            scope_kind: SnapshotScopeKind::City,
            scope_id: "17".to_owned(),
            facts: vec![SnapshotFact {
                fact_id: "source.stats.citizens.born".to_owned(),
                source_field: "$Citizens_Born".to_owned(),
                value: city_births,
                source_line: 211,
            }],
            expected_fact_count: 5,
            coverage: CoverageStatus::Partial,
        },
    ];
    result
}

#[test]
fn recorder_ledger_recovers_an_interrupted_read_after_restart() {
    let directory = tempdir().expect("temporary directory");
    let database_path = directory.path().join("recorder.sqlite3");
    let storage = ObservatoryStorage::initialise(database_path.clone()).expect("storage");
    let identity = "a".repeat(64);
    let candidate = storage
        .discover_recorder_candidate(
            &identity,
            "recover.zip",
            42,
            10,
            20,
            RecorderDiscoverySource::FilesystemEvent,
        )
        .expect("discover candidate");
    storage
        .mark_recorder_candidate_stabilising(candidate.candidate_id)
        .expect("stabilising");
    storage
        .mark_recorder_candidate_ready(candidate.candidate_id, 1_520)
        .expect("ready");
    assert_eq!(
        storage
            .mark_recorder_candidate_reading(candidate.candidate_id, 1_521)
            .expect("reading"),
        1
    );
    assert_eq!(storage.recorder_candidate_count().expect("count"), 1);
    drop(storage);

    let reopened = ObservatoryStorage::initialise(database_path).expect("reopened storage");
    let health = reopened
        .load_recorder_health(AutomaticObserver::new(true).status())
        .expect("health");
    assert_eq!(health.queue_depth, 1);
    assert_eq!(
        health.latest_entries[0].status,
        RecorderCandidateStatus::Discovered
    );
    assert_eq!(health.latest_entries[0].attempt_count, 1);
    assert_eq!(
        health.latest_entries[0].error_code.as_deref(),
        Some("interrupted")
    );
    assert_eq!(
        health.latest_entries[0].discovery_source,
        RecorderDiscoverySource::FilesystemEvent
    );
}

#[test]
fn terminal_recorder_failure_moves_from_queue_to_attention_without_losing_evidence() {
    let directory = tempdir().expect("temporary directory");
    let storage =
        ObservatoryStorage::initialise(directory.path().join("recorder.sqlite3")).expect("storage");
    let identity = "b".repeat(64);
    let candidate = storage
        .discover_recorder_candidate(
            &identity,
            "broken.zip",
            12,
            30,
            40,
            RecorderDiscoverySource::Reconciliation,
        )
        .expect("discover candidate");
    storage
        .fail_recorder_candidate(candidate.candidate_id, false, "invalid_archive", 50)
        .expect("terminal failure");

    let health = storage
        .load_recorder_health(AutomaticObserver::new(true).status())
        .expect("health");
    assert_eq!(health.queue_depth, 0);
    assert_eq!(health.attention_count, 1);
    assert_eq!(
        health.latest_entries[0].status,
        RecorderCandidateStatus::TerminalFailure
    );
    assert_eq!(health.latest_entries[0].completed_at_ms, Some(50));

    storage
        .discover_recorder_candidate(
            &identity,
            "broken.zip",
            13,
            31,
            60,
            RecorderDiscoverySource::FilesystemEvent,
        )
        .expect("replacement candidate");
    let recovered = storage
        .load_recorder_health(AutomaticObserver::new(true).status())
        .expect("recovered health");
    assert_eq!(recovered.attention_count, 0);
    assert_eq!(recovered.queue_depth, 1);
    assert!(
        recovered
            .latest_entries
            .iter()
            .any(|entry| entry.status == RecorderCandidateStatus::Superseded)
    );
}

#[test]
#[ignore = "manual storage-growth benchmark"]
fn benchmark_compacted_archive_growth() {
    use std::time::Instant;

    let directory = tempdir().expect("temporary directory");
    let database_path = directory.path().join("growth.sqlite3");
    let storage = ObservatoryStorage::initialise(database_path.clone()).expect("storage");
    let started = Instant::now();
    let baseline_records = 1_900_u64;
    let save_count = 128_u64;
    for save_index in 0..save_count {
        let values = (0..baseline_records + save_index)
            .map(|record| record + 1)
            .collect::<Vec<_>>();
        let mut save = inspection(
            &format!("benchmark-{save_index:03}"),
            &format!("benchmark-{save_index:03}.zip"),
            &values,
        );
        save.snapshots = benchmark_snapshots(save_index);
        storage
            .save_inspection(&save)
            .unwrap_or_else(|error| panic!("benchmark import {save_index}: {error:?}"));
    }
    let connection = storage.connect().expect("connection");
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .expect("checkpoint");
    let nodes = connection
        .query_row("SELECT COUNT(*) FROM receiver_history_nodes", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("node count") as u64;
    let bytes = std::fs::metadata(database_path)
        .expect("database metadata")
        .len();
    eprintln!(
        "archive benchmark: {save_count} states, {nodes} shared nodes, {bytes} bytes, {:?}",
        started.elapsed()
    );
    assert_eq!(nodes, baseline_records + save_count - 1);
    assert!(bytes < 32 * 1024 * 1024);
}

#[test]
#[ignore = "manual recorder-ledger growth benchmark"]
fn benchmark_recorder_ledger_growth() {
    use std::time::Instant;

    let directory = tempdir().expect("temporary directory");
    let database_path = directory.path().join("recorder-growth.sqlite3");
    let storage = ObservatoryStorage::initialise(database_path.clone()).expect("storage");
    let identity = "c".repeat(64);
    storage
        .mark_recorder_directory_initialised(&identity, 1)
        .expect("initialised directory");
    let started = Instant::now();
    let candidate_count = 1_000_u32;
    for index in 0..candidate_count {
        let candidate = storage
            .discover_recorder_candidate(
                &identity,
                &format!("save-{index:04}.zip"),
                1_000 + index as u64,
                index as i64,
                index as i64,
                RecorderDiscoverySource::FilesystemEvent,
            )
            .expect("discover candidate");
        storage
            .supersede_recorder_candidate(candidate.candidate_id, index as i64 + 1)
            .expect("complete lifecycle");
    }
    let connection = storage.connect().expect("connection");
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .expect("checkpoint");
    let bytes = std::fs::metadata(database_path)
        .expect("database metadata")
        .len();
    eprintln!(
        "recorder benchmark: {candidate_count} candidates, {bytes} bytes, {:?}",
        started.elapsed()
    );
    assert_eq!(
        storage.recorder_candidate_count().expect("candidate count"),
        candidate_count
    );
    assert!(bytes < 8 * 1024 * 1024);
}

fn benchmark_snapshots(save_index: u64) -> Vec<SaveSnapshot> {
    let mut snapshots = Vec::with_capacity(140);
    snapshots.push(SaveSnapshot {
        scope_kind: SnapshotScopeKind::Republic,
        scope_id: "republic".to_owned(),
        facts: SNAPSHOT_FACTS
            .iter()
            .filter(|definition| definition.republic)
            .enumerate()
            .map(|(index, definition)| SnapshotFact {
                fact_id: definition.id.to_owned(),
                source_field: definition.id.to_owned(),
                value: save_index + index as u64,
                source_line: index as u64 + 1,
            })
            .collect(),
        expected_fact_count: 18,
        coverage: CoverageStatus::Complete,
    });
    for city_id in 0..139_u32 {
        snapshots.push(SaveSnapshot {
            scope_kind: SnapshotScopeKind::City,
            scope_id: city_id.to_string(),
            facts: SNAPSHOT_FACTS
                .iter()
                .filter(|definition| definition.city)
                .enumerate()
                .map(|(index, definition)| SnapshotFact {
                    fact_id: definition.id.to_owned(),
                    source_field: definition.id.to_owned(),
                    value: save_index + city_id as u64 + index as u64,
                    source_line: 100 + index as u64,
                })
                .collect(),
            expected_fact_count: 5,
            coverage: CoverageStatus::Complete,
        });
    }
    snapshots
}
