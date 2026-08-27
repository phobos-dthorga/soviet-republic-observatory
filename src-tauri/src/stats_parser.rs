use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;
use std::io::BufRead;

use sha2::{Digest, Sha256};

use crate::error::ObservatoryError;
use crate::model::{
    CoverageReport, CoverageStatus, CoverageWarning, ParsedStats, ReceiverRecord, SNAPSHOT_FACTS,
    SaveSnapshot, SnapshotFact, SnapshotScopeKind, SourceLineSet,
};

const MAX_STATS_BYTES: u64 = 128 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 16 * 1024;
const DAYS_PER_GAME_YEAR: i64 = 365;

#[derive(Clone, Copy, Debug)]
struct FieldValue<T> {
    value: T,
    line: u64,
}

#[derive(Debug)]
struct RecordDraft {
    record_id: u32,
    year: Option<FieldValue<i32>>,
    day: Option<FieldValue<u16>>,
    none: Option<FieldValue<u64>>,
    radio: Option<FieldValue<u64>>,
    television: Option<FieldValue<u64>>,
    computer: Option<FieldValue<u64>>,
    malformed: bool,
}

#[derive(Debug)]
struct SnapshotDraft {
    scope_kind: SnapshotScopeKind,
    scope_id: String,
    facts: BTreeMap<&'static str, SnapshotFact>,
}

impl SnapshotDraft {
    fn republic() -> Self {
        Self {
            scope_kind: SnapshotScopeKind::Republic,
            scope_id: "republic".to_owned(),
            facts: BTreeMap::new(),
        }
    }

    fn city(city_source_id: u32) -> Self {
        Self {
            scope_kind: SnapshotScopeKind::City,
            scope_id: city_source_id.to_string(),
            facts: BTreeMap::new(),
        }
    }
}

impl RecordDraft {
    fn new(record_id: u32) -> Self {
        Self {
            record_id,
            year: None,
            day: None,
            none: None,
            radio: None,
            television: None,
            computer: None,
            malformed: false,
        }
    }
}

pub fn parse_stats<R: BufRead>(mut reader: R) -> Result<ParsedStats, ObservatoryError> {
    let mut hash = Sha256::new();
    let mut bytes_read = 0_u64;
    let mut line_number = 0_u64;
    let mut line_buffer = Vec::new();
    let mut current: Option<RecordDraft> = None;
    let mut current_snapshot: Option<SnapshotDraft> = None;
    let mut records = Vec::new();
    let mut snapshots = Vec::new();
    let mut seen_city_ids = HashSet::new();
    let mut warnings = BTreeMap::<String, u32>::new();
    let mut seen_record_ids = HashSet::new();
    let mut last_record_id = None;
    let mut history_records = 0_u32;
    let mut dropped_records = 0_u32;
    let mut chartable_records = 0_u32;

    loop {
        line_buffer.clear();
        let count = reader
            .read_until(b'\n', &mut line_buffer)
            .map_err(|_| ObservatoryError::InvalidArchive)?;
        if count == 0 {
            break;
        }

        bytes_read = bytes_read
            .checked_add(count as u64)
            .ok_or(ObservatoryError::StatsPayloadTooLarge)?;
        if bytes_read > MAX_STATS_BYTES {
            return Err(ObservatoryError::StatsPayloadTooLarge);
        }
        if line_buffer.len() > MAX_LINE_BYTES {
            return Err(ObservatoryError::StatsLineTooLong);
        }

        hash.update(&line_buffer);
        line_number += 1;
        let line = std::str::from_utf8(&line_buffer)
            .map_err(|_| ObservatoryError::InvalidStatsEncoding)?
            .trim_end_matches(['\r', '\n']);

        if line.starts_with("$STATS_FORMAT") {
            let version: u32 = parse_single_value(line).ok_or(
                ObservatoryError::MalformedReceiverHistory("invalid stats format marker"),
            )?;
            if version != 1 {
                return Err(ObservatoryError::UnsupportedStatsFormat);
            }
            continue;
        }

        if line.starts_with("$STAT_RECORD") {
            finalise_snapshot(current_snapshot.take(), &mut snapshots);
            finalise_record(
                current.take(),
                &mut records,
                &mut warnings,
                &mut history_records,
                &mut dropped_records,
                &mut chartable_records,
            )?;
            let record_id: u32 = parse_single_value(line).ok_or(
                ObservatoryError::MalformedReceiverHistory("invalid record identifier"),
            )?;
            if !seen_record_ids.insert(record_id) {
                return Err(ObservatoryError::MalformedReceiverHistory(
                    "duplicate record identifier",
                ));
            }
            if last_record_id.is_some_and(|previous| record_id <= previous) {
                return Err(ObservatoryError::MalformedReceiverHistory(
                    "record identifiers are not increasing",
                ));
            }
            last_record_id = Some(record_id);
            current = Some(RecordDraft::new(record_id));
            continue;
        }

        if line.starts_with("$STAT_CURRENT") {
            finalise_record(
                current.take(),
                &mut records,
                &mut warnings,
                &mut history_records,
                &mut dropped_records,
                &mut chartable_records,
            )?;
            finalise_snapshot(current_snapshot.take(), &mut snapshots);
            current_snapshot = Some(SnapshotDraft::republic());
            continue;
        }

        if line.starts_with("$STAT_CITY") {
            finalise_record(
                current.take(),
                &mut records,
                &mut warnings,
                &mut history_records,
                &mut dropped_records,
                &mut chartable_records,
            )?;
            finalise_snapshot(current_snapshot.take(), &mut snapshots);
            let city_source_id: u32 = parse_single_value(line).ok_or(
                ObservatoryError::MalformedSnapshot("invalid city identifier"),
            )?;
            if !seen_city_ids.insert(city_source_id) {
                return Err(ObservatoryError::MalformedSnapshot(
                    "duplicate city identifier",
                ));
            }
            current_snapshot = Some(SnapshotDraft::city(city_source_id));
            continue;
        }

        if let Some(snapshot) = current_snapshot.as_mut() {
            assign_snapshot_fact(snapshot, line, line_number)?;
            continue;
        }

        let Some(record) = current.as_mut() else {
            continue;
        };

        match directive_name(line) {
            Some("$DATE_YEAR") => assign_field(
                &mut record.year,
                parse_single_value::<i32>(line),
                line_number,
                &mut record.malformed,
            )?,
            Some("$DATE_DAY") => assign_field(
                &mut record.day,
                parse_single_value::<u16>(line).filter(|day| *day < DAYS_PER_GAME_YEAR as u16),
                line_number,
                &mut record.malformed,
            )?,
            Some("$Citizens_EletronicNone") => assign_field(
                &mut record.none,
                parse_single_value::<u64>(line),
                line_number,
                &mut record.malformed,
            )?,
            Some("$Citizens_EletrinicRadio") => assign_field(
                &mut record.radio,
                parse_single_value::<u64>(line),
                line_number,
                &mut record.malformed,
            )?,
            Some("$Citizens_EletronicTV") => assign_field(
                &mut record.television,
                parse_single_value::<u64>(line),
                line_number,
                &mut record.malformed,
            )?,
            Some("$Citizens_EletronicComputer") => assign_field(
                &mut record.computer,
                parse_single_value::<u64>(line),
                line_number,
                &mut record.malformed,
            )?,
            _ => {}
        }
    }

    finalise_record(
        current,
        &mut records,
        &mut warnings,
        &mut history_records,
        &mut dropped_records,
        &mut chartable_records,
    )?;
    finalise_snapshot(current_snapshot, &mut snapshots);

    if records.is_empty() {
        return Err(ObservatoryError::ReceiverHistoryUnavailable);
    }

    let warnings = warnings
        .into_iter()
        .map(|(code, count)| CoverageWarning { code, count })
        .collect::<Vec<_>>();
    let status = if warnings.is_empty() {
        CoverageStatus::Complete
    } else {
        CoverageStatus::Partial
    };

    let digest = hash.finalize();
    let mut payload_hash = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut payload_hash, "{byte:02x}").expect("writing to a String cannot fail");
    }

    Ok(ParsedStats {
        payload_hash,
        records,
        coverage: CoverageReport {
            status,
            history_records,
            chartable_records,
            dropped_records,
            warnings,
        },
        snapshots,
    })
}

fn assign_snapshot_fact(
    snapshot: &mut SnapshotDraft,
    line: &str,
    line_number: u64,
) -> Result<(), ObservatoryError> {
    let Some(source_field) = directive_name(line) else {
        return Ok(());
    };
    let Some(definition) = SNAPSHOT_FACTS.iter().find(|definition| {
        definition.source_field == source_field
            && match snapshot.scope_kind {
                SnapshotScopeKind::Republic => definition.republic,
                SnapshotScopeKind::City => definition.city,
            }
    }) else {
        return Ok(());
    };
    if snapshot.facts.contains_key(definition.id) {
        return Err(ObservatoryError::MalformedSnapshot(
            "duplicate supported scalar field",
        ));
    }
    let Some(value) = parse_single_value::<u64>(line) else {
        return Ok(());
    };
    snapshot.facts.insert(
        definition.id,
        SnapshotFact {
            fact_id: definition.id,
            source_field: definition.source_field,
            value,
            source_line: line_number,
        },
    );
    Ok(())
}

fn finalise_snapshot(draft: Option<SnapshotDraft>, snapshots: &mut Vec<SaveSnapshot>) {
    let Some(draft) = draft else {
        return;
    };
    let expected_fact_count = SNAPSHOT_FACTS
        .iter()
        .filter(|definition| match draft.scope_kind {
            SnapshotScopeKind::Republic => definition.republic,
            SnapshotScopeKind::City => definition.city,
        })
        .count()
        .min(u32::MAX as usize) as u32;
    let coverage = if draft.facts.len() == expected_fact_count as usize {
        CoverageStatus::Complete
    } else {
        CoverageStatus::Partial
    };
    snapshots.push(SaveSnapshot {
        scope_kind: draft.scope_kind,
        scope_id: draft.scope_id,
        facts: draft.facts.into_values().collect(),
        expected_fact_count,
        coverage,
    });
}

fn finalise_record(
    draft: Option<RecordDraft>,
    records: &mut Vec<ReceiverRecord>,
    warnings: &mut BTreeMap<String, u32>,
    history_records: &mut u32,
    dropped_records: &mut u32,
    chartable_records: &mut u32,
) -> Result<(), ObservatoryError> {
    let Some(draft) = draft else {
        return Ok(());
    };
    *history_records += 1;

    if draft.malformed {
        add_warning(warnings, "malformed_record");
        *dropped_records += 1;
        return Ok(());
    }

    let (Some(year), Some(day), Some(none), Some(radio), Some(television), Some(computer)) = (
        draft.year,
        draft.day,
        draft.none,
        draft.radio,
        draft.television,
        draft.computer,
    ) else {
        add_warning(warnings, "incomplete_record");
        *dropped_records += 1;
        return Ok(());
    };

    let classified_total = none
        .value
        .checked_add(radio.value)
        .and_then(|value| value.checked_add(television.value))
        .and_then(|value| value.checked_add(computer.value));
    let Some(classified_total) = classified_total else {
        add_warning(warnings, "classified_total_overflow");
        *dropped_records += 1;
        return Ok(());
    };

    if classified_total == 0 {
        add_warning(warnings, "zero_classified_population");
    } else {
        *chartable_records += 1;
    }

    records.push(ReceiverRecord {
        record_id: draft.record_id,
        year: year.value,
        day: day.value,
        game_day: i64::from(year.value) * DAYS_PER_GAME_YEAR + i64::from(day.value),
        none: none.value,
        radio: radio.value,
        television: television.value,
        computer: computer.value,
        classified_total,
        source_lines: SourceLineSet {
            none: none.line,
            radio: radio.line,
            television: television.line,
            computer: computer.line,
        },
    });
    Ok(())
}

fn assign_field<T: Copy>(
    target: &mut Option<FieldValue<T>>,
    value: Option<T>,
    line: u64,
    malformed: &mut bool,
) -> Result<(), ObservatoryError> {
    if target.is_some() {
        return Err(ObservatoryError::MalformedReceiverHistory(
            "duplicate field in one record",
        ));
    }
    match value {
        Some(value) => *target = Some(FieldValue { value, line }),
        None => *malformed = true,
    }
    Ok(())
}

fn directive_name(line: &str) -> Option<&str> {
    line.split_ascii_whitespace()
        .next()
        .filter(|part| part.starts_with('$'))
}

fn parse_single_value<T: std::str::FromStr>(line: &str) -> Option<T> {
    let mut parts = line.split_ascii_whitespace();
    parts.next()?;
    let value = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some(value)
}

fn add_warning(warnings: &mut BTreeMap<String, u32>, code: &str) {
    *warnings.entry(code.to_owned()).or_default() += 1;
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::parse_stats;
    use crate::error::ObservatoryError;
    use crate::model::{CoverageStatus, SnapshotScopeKind};

    #[test]
    fn parses_complete_receiver_history_and_closes_it_at_current_block() {
        let parsed = parse_stats(Cursor::new(include_bytes!(
            "../fixtures/valid.receiver-stats.txt"
        )))
        .expect("valid fixture");

        assert_eq!(parsed.records.len(), 3);
        assert_eq!(parsed.coverage.status, CoverageStatus::Complete);
        assert_eq!(parsed.records[2].year, 1981);
        assert_eq!(parsed.records[2].day, 5);
        assert_eq!(parsed.records[2].classified_total, 100);
        assert_eq!(parsed.payload_hash.len(), 64);
        assert_eq!(parsed.snapshots.len(), 1);
        assert_eq!(parsed.snapshots[0].scope_kind, SnapshotScopeKind::Republic);
        assert_eq!(parsed.snapshots[0].facts.len(), 4);
        assert_eq!(parsed.snapshots[0].coverage, CoverageStatus::Partial);
    }

    #[test]
    fn reports_partial_coverage_without_inventing_a_missing_metric() {
        let parsed = parse_stats(Cursor::new(include_bytes!(
            "../fixtures/partial.receiver-stats.txt"
        )))
        .expect("partially usable fixture");

        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.coverage.history_records, 2);
        assert_eq!(parsed.coverage.dropped_records, 1);
        assert_eq!(parsed.coverage.status, CoverageStatus::Partial);
        assert_eq!(parsed.coverage.warnings[0].code, "incomplete_record");
    }

    #[test]
    fn drops_a_malformed_record_but_preserves_a_later_valid_record() {
        let parsed = parse_stats(Cursor::new(include_bytes!(
            "../fixtures/malformed.receiver-stats.txt"
        )))
        .expect("one record remains usable");

        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].record_id, 1);
        assert_eq!(parsed.coverage.warnings[0].code, "malformed_record");
    }

    #[test]
    fn rejects_duplicate_record_identifiers() {
        let error = parse_stats(Cursor::new(include_bytes!(
            "../fixtures/duplicate-ids.receiver-stats.txt"
        )))
        .expect_err("duplicate ids must fail");
        assert!(matches!(
            error,
            ObservatoryError::MalformedReceiverHistory("duplicate record identifier")
        ));
    }

    #[test]
    fn rejects_an_explicit_unsupported_format() {
        let error = parse_stats(Cursor::new(include_bytes!(
            "../fixtures/unsupported-version.receiver-stats.txt"
        )))
        .expect_err("unsupported format must fail");
        assert!(matches!(error, ObservatoryError::UnsupportedStatsFormat));
    }

    #[test]
    fn captures_supported_current_and_city_snapshot_facts() {
        let parsed = parse_stats(Cursor::new(include_bytes!(
            "../fixtures/current-city.receiver-stats.txt"
        )))
        .expect("snapshot fixture");

        assert_eq!(parsed.snapshots.len(), 3);
        let republic = &parsed.snapshots[0];
        assert_eq!(republic.scope_kind, SnapshotScopeKind::Republic);
        assert_eq!(republic.scope_id, "republic");
        assert_eq!(republic.facts.len(), 18);
        assert_eq!(republic.coverage, CoverageStatus::Complete);
        let cities = &parsed.snapshots[1..];
        assert!(
            cities
                .iter()
                .all(|snapshot| snapshot.scope_kind == SnapshotScopeKind::City)
        );
        assert!(cities.iter().all(|snapshot| snapshot.facts.len() == 5));
        assert!(
            cities
                .iter()
                .all(|snapshot| snapshot.coverage == CoverageStatus::Complete)
        );
    }
}
