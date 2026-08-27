use rusqlite::{Connection, params};
use tempfile::tempdir;

use super::ObservatoryStorage;
use crate::automatic_observer::AutomaticObserver;
use crate::model::{
    CoverageReport, CoverageStatus, ReceiverRecord, RecorderCandidateStatus,
    RecorderDiscoverySource, SNAPSHOT_FACTS, SaveInspection, SaveSnapshot, SnapshotFact,
    SnapshotScopeKind, SourceLineSet,
};

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
    let storage = ObservatoryStorage::initialise(directory.path().join("retry.sqlite3"))
        .expect("storage");
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

    storage.enqueue_warehouse_rebuild().expect("request rebuild");
    let retry = storage.projection_queue_status().expect("retry health");
    assert_eq!(retry.failed_jobs, 0);
    assert_eq!(retry.pending_jobs, 2);
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
    assert_eq!(archive.selected_branch_id, "fork-fork-state");
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
            .expect("latest fork")
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
            .expect("reselected fork")
            .expect("fork dataset")
            .payload_hash,
        "fork-state"
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
    assert_eq!(archive.selected_branch_id, "unassigned");
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
                    metric.source_field,
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
            }
        })
        .collect::<Vec<_>>();
    SaveInspection {
        payload_hash: hash.to_owned(),
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
    }
}

fn snapshot_fact(fact_id: &'static str, source_field: &'static str, value: u64) -> SnapshotFact {
    SnapshotFact {
        fact_id,
        source_field,
        value,
        source_line: 100,
    }
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
                fact_id: definition.id,
                source_field: definition.source_field,
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
                    fact_id: definition.id,
                    source_field: definition.source_field,
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
