use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::io::BufRead;

use sha2::{Digest, Sha256};

use crate::compatibility_profile::{ResolvedCompatibilityProfile, StatsContext, StatsMarkerSlot};
use crate::error::ObservatoryError;
use crate::model::{
    CitizenStatusRecord, CoverageReport, CoverageStatus, CoverageWarning, MarketCurrency,
    MarketFactRows, MarketHistoryRecord, MarketPriceRow, MarketPriceSide, MarketScalarRow,
    MarketSnapshot, MarketTradeChannel, MarketTradeDirection, MarketTradeRow,
    ParsedCitizenStatusData, ParsedMarketData, ParsedStats, ReceiverRecord, SNAPSHOT_FACTS,
    SaveSnapshot, SnapshotFact, SnapshotScopeKind, SourceFieldSet, SourceLineSet,
};

const MAX_STATS_BYTES: u64 = 128 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 16 * 1024;
const DAYS_PER_GAME_YEAR: i64 = 365;
const MAX_MARKET_ROWS: u32 = 1_500_000;
const MAX_MARKET_DICTIONARY: usize = 4_096;

#[derive(Clone, Debug)]
struct FieldValue<T> {
    value: T,
    line: u64,
    source_field: String,
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
    citizen_status: [Option<FieldValue<f64>>; 9],
    citizen_status_seen: bool,
    citizen_status_malformed: bool,
    market: MarketFactRows,
    malformed: bool,
}

#[derive(Debug)]
struct SnapshotDraft {
    scope_kind: SnapshotScopeKind,
    scope_id: String,
    facts: BTreeMap<String, SnapshotFact>,
    market: MarketFactRows,
}

#[derive(Clone, Copy, Debug)]
enum MarketBlockKind {
    Price {
        currency: MarketCurrency,
        side: MarketPriceSide,
    },
    Trade {
        currency: MarketCurrency,
        direction: MarketTradeDirection,
        channel: MarketTradeChannel,
    },
}

#[derive(Debug)]
struct ActiveMarketBlock {
    kind: MarketBlockKind,
    source_field_index: u16,
    seen_resources: HashSet<u16>,
}

#[derive(Debug, Default)]
struct MarketCollector {
    resources: Vec<String>,
    resource_lookup: HashMap<String, u16>,
    source_fields: Vec<String>,
    source_field_lookup: HashMap<String, u16>,
    records: Vec<MarketHistoryRecord>,
    snapshots: Vec<MarketSnapshot>,
    row_count: u32,
    warnings: BTreeMap<String, u32>,
}

#[derive(Debug, Default)]
struct RecordCollector {
    records: Vec<ReceiverRecord>,
    warnings: BTreeMap<String, u32>,
    history_records: u32,
    dropped_records: u32,
    chartable_records: u32,
    citizen_status_records: Vec<CitizenStatusRecord>,
    citizen_status_warnings: BTreeMap<String, u32>,
    citizen_status_history_records: u32,
    citizen_status_dropped_records: u32,
}

impl SnapshotDraft {
    fn republic() -> Self {
        Self {
            scope_kind: SnapshotScopeKind::Republic,
            scope_id: "republic".to_owned(),
            facts: BTreeMap::new(),
            market: MarketFactRows::default(),
        }
    }

    fn city(city_source_id: u32) -> Self {
        Self {
            scope_kind: SnapshotScopeKind::City,
            scope_id: city_source_id.to_string(),
            facts: BTreeMap::new(),
            market: MarketFactRows::default(),
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
            citizen_status: std::array::from_fn(|_| None),
            citizen_status_seen: false,
            citizen_status_malformed: false,
            market: MarketFactRows::default(),
            malformed: false,
        }
    }
}

pub fn parse_stats<R: BufRead>(
    mut reader: R,
    profile: &ResolvedCompatibilityProfile,
) -> Result<ParsedStats, ObservatoryError> {
    let mut hash = Sha256::new();
    let mut bytes_read = 0_u64;
    let mut line_number = 0_u64;
    let mut line_buffer = Vec::new();
    let mut current: Option<RecordDraft> = None;
    let mut current_snapshot: Option<SnapshotDraft> = None;
    let mut record_collector = RecordCollector::default();
    let mut snapshots = Vec::new();
    let mut seen_city_ids = HashSet::new();
    let mut seen_record_ids = HashSet::new();
    let mut last_record_id = None;
    let mut market = MarketCollector::default();
    let mut active_market_block: Option<ActiveMarketBlock> = None;
    let status_enabled = profile.has_indexed_fields(StatsContext::History);

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

        let directive = directive_name(line);
        let marker = directive.and_then(|directive| profile.marker_for(directive));

        if let Some(block) = active_market_block.as_mut() {
            if directive == Some("$end") {
                active_market_block = None;
                continue;
            }
            if directive.is_none() {
                if is_market_block_separator(line) {
                    continue;
                }
                parse_market_block_row(
                    block,
                    line,
                    line_number,
                    &mut market,
                    current.as_mut(),
                    current_snapshot.as_mut(),
                );
                continue;
            }
            add_warning(&mut market.warnings, "unterminated_market_block");
            active_market_block = None;
        }

        if marker.is_some_and(|marker| marker.slot == StatsMarkerSlot::Format) {
            let version: u16 = parse_single_value(line).ok_or(
                ObservatoryError::MalformedReceiverHistory("invalid stats format marker"),
            )?;
            if !marker
                .and_then(|marker| marker.accepted_values.as_ref())
                .is_some_and(|values| values.contains(&version))
            {
                return Err(ObservatoryError::UnsupportedStatsFormat);
            }
            continue;
        }

        if marker.is_some_and(|marker| marker.slot == StatsMarkerSlot::HistoryRecord) {
            finalise_snapshot(
                profile,
                current_snapshot.take(),
                &mut snapshots,
                &mut market.snapshots,
            );
            finalise_record(
                current.take(),
                &mut record_collector,
                &mut market,
                status_enabled,
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

        if marker.is_some_and(|marker| marker.slot == StatsMarkerSlot::CurrentState) {
            finalise_record(
                current.take(),
                &mut record_collector,
                &mut market,
                status_enabled,
            )?;
            finalise_snapshot(
                profile,
                current_snapshot.take(),
                &mut snapshots,
                &mut market.snapshots,
            );
            current_snapshot = Some(SnapshotDraft::republic());
            continue;
        }

        if marker.is_some_and(|marker| marker.slot == StatsMarkerSlot::City) {
            finalise_record(
                current.take(),
                &mut record_collector,
                &mut market,
                status_enabled,
            )?;
            finalise_snapshot(
                profile,
                current_snapshot.take(),
                &mut snapshots,
                &mut market.snapshots,
            );
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

        let market_context = if let Some(snapshot) = current_snapshot.as_ref() {
            Some(match snapshot.scope_kind {
                SnapshotScopeKind::Republic => StatsContext::Republic,
                SnapshotScopeKind::City => StatsContext::City,
            })
        } else if current.is_some() {
            Some(StatsContext::History)
        } else {
            None
        };
        if let (Some(directive), Some(context)) = (directive, market_context)
            && let Some(mapping) = profile.field_for(directive, context)
            && mapping.host_slot.starts_with("market.")
        {
            let Some(source_field_index) = market.intern_source_field(directive) else {
                continue;
            };
            if let Some(kind) = market_block_kind(&mapping.host_slot) {
                active_market_block = Some(ActiveMarketBlock {
                    kind,
                    source_field_index,
                    seen_resources: HashSet::new(),
                });
            } else {
                parse_market_scalar(
                    &mapping.host_slot,
                    source_field_index,
                    line,
                    line_number,
                    &mut market,
                    current.as_mut(),
                    current_snapshot.as_mut(),
                );
            }
            continue;
        }

        if let Some(snapshot) = current_snapshot.as_mut() {
            assign_snapshot_fact(profile, snapshot, line, line_number)?;
            continue;
        }

        let Some(record) = current.as_mut() else {
            continue;
        };

        match marker.map(|marker| marker.slot) {
            Some(StatsMarkerSlot::DateYear) => assign_field(
                &mut record.year,
                parse_single_value::<i32>(line),
                line_number,
                directive.unwrap_or_default(),
                &mut record.malformed,
            )?,
            Some(StatsMarkerSlot::DateDay) => assign_field(
                &mut record.day,
                parse_single_value::<u16>(line).filter(|day| *day < DAYS_PER_GAME_YEAR as u16),
                line_number,
                directive.unwrap_or_default(),
                &mut record.malformed,
            )?,
            _ => {
                let Some(directive) = directive else { continue };
                if profile.has_indexed_field_alias(directive, StatsContext::History) {
                    record.citizen_status_seen = true;
                    match parse_indexed_value::<u8, f64>(line) {
                        Some((index, value))
                            if value.is_finite()
                                && (0.0..=1.0).contains(&value)
                                && profile
                                    .indexed_field_for(directive, StatsContext::History, index)
                                    .is_some() =>
                        {
                            let slot = &mut record.citizen_status[index as usize];
                            if slot.is_some() {
                                record.citizen_status_malformed = true;
                                add_warning(
                                    &mut record_collector.citizen_status_warnings,
                                    "duplicate_citizen_status_index",
                                );
                            } else {
                                *slot = Some(FieldValue {
                                    value,
                                    line: line_number,
                                    source_field: directive.to_owned(),
                                });
                            }
                        }
                        Some((index, value)) if index > 8 || !value.is_finite() => {
                            record.citizen_status_malformed = true;
                            add_warning(
                                &mut record_collector.citizen_status_warnings,
                                "invalid_citizen_status_value",
                            );
                        }
                        Some((_index, _value)) => {
                            record.citizen_status_malformed = true;
                            add_warning(
                                &mut record_collector.citizen_status_warnings,
                                "out_of_range_citizen_status_value",
                            );
                        }
                        None => {
                            record.citizen_status_malformed = true;
                            add_warning(
                                &mut record_collector.citizen_status_warnings,
                                "malformed_citizen_status_row",
                            );
                        }
                    }
                    continue;
                }
                let Some(field) = profile.field_for(directive, StatsContext::History) else {
                    continue;
                };
                match field.host_slot.as_str() {
                    "core.citizens.electronics.none" => assign_field(
                        &mut record.none,
                        parse_single_value::<u64>(line),
                        line_number,
                        directive,
                        &mut record.malformed,
                    )?,
                    "core.citizens.electronics.radio" => assign_field(
                        &mut record.radio,
                        parse_single_value::<u64>(line),
                        line_number,
                        directive,
                        &mut record.malformed,
                    )?,
                    "core.citizens.electronics.television" => assign_field(
                        &mut record.television,
                        parse_single_value::<u64>(line),
                        line_number,
                        directive,
                        &mut record.malformed,
                    )?,
                    "core.citizens.electronics.computer" => assign_field(
                        &mut record.computer,
                        parse_single_value::<u64>(line),
                        line_number,
                        directive,
                        &mut record.malformed,
                    )?,
                    _ => {}
                }
            }
        }
    }

    finalise_record(current, &mut record_collector, &mut market, status_enabled)?;
    if active_market_block.is_some() {
        add_warning(&mut market.warnings, "unterminated_market_block");
    }
    finalise_snapshot(
        profile,
        current_snapshot,
        &mut snapshots,
        &mut market.snapshots,
    );

    if record_collector.records.is_empty() {
        return Err(ObservatoryError::ReceiverHistoryUnavailable);
    }

    let warnings = record_collector
        .warnings
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
        records: record_collector.records,
        coverage: CoverageReport {
            status,
            history_records: record_collector.history_records,
            chartable_records: record_collector.chartable_records,
            dropped_records: record_collector.dropped_records,
            warnings,
        },
        snapshots,
        market: ParsedMarketData {
            resources: market.resources,
            source_fields: market.source_fields,
            records: market.records,
            snapshots: market.snapshots,
            row_count: market.row_count,
            warnings: market
                .warnings
                .into_iter()
                .map(|(code, count)| CoverageWarning { code, count })
                .collect(),
        },
        citizen_status: ParsedCitizenStatusData {
            records: record_collector.citizen_status_records,
            history_records: record_collector.citizen_status_history_records,
            dropped_records: record_collector.citizen_status_dropped_records,
            warnings: record_collector
                .citizen_status_warnings
                .into_iter()
                .map(|(code, count)| CoverageWarning { code, count })
                .collect(),
        },
    })
}

fn is_market_block_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && trimmed.bytes().all(|byte| byte == b'-')
}

impl MarketCollector {
    fn intern_source_field(&mut self, value: &str) -> Option<u16> {
        if let Some(index) = self.source_field_lookup.get(value) {
            return Some(*index);
        }
        self.intern(value, false)
    }

    fn intern_resource(&mut self, value: &str) -> Option<u16> {
        if let Some(index) = self.resource_lookup.get(value) {
            return Some(*index);
        }
        self.intern(value, true)
    }

    fn intern(&mut self, value: &str, resource: bool) -> Option<u16> {
        let values = if resource {
            &mut self.resources
        } else {
            &mut self.source_fields
        };
        if values.len() >= MAX_MARKET_DICTIONARY || value.is_empty() || value.len() > 128 {
            add_warning(&mut self.warnings, "market_dictionary_limit");
            return None;
        }
        let Ok(index) = u16::try_from(values.len()) else {
            add_warning(&mut self.warnings, "market_dictionary_limit");
            return None;
        };
        values.push(value.to_owned());
        if resource {
            self.resource_lookup.insert(value.to_owned(), index);
        } else {
            self.source_field_lookup.insert(value.to_owned(), index);
        }
        Some(index)
    }

    fn reserve_row(&mut self) -> bool {
        if self.row_count >= MAX_MARKET_ROWS {
            add_warning(&mut self.warnings, "market_row_limit");
            return false;
        }
        self.row_count += 1;
        true
    }
}

fn market_rows_mut<'a>(
    current: Option<&'a mut RecordDraft>,
    snapshot: Option<&'a mut SnapshotDraft>,
) -> Option<&'a mut MarketFactRows> {
    snapshot
        .map(|draft| &mut draft.market)
        .or_else(|| current.map(|draft| &mut draft.market))
}

fn market_block_kind(host_slot: &str) -> Option<MarketBlockKind> {
    let currency = if host_slot.ends_with(".rub") {
        MarketCurrency::Rub
    } else if host_slot.ends_with(".usd") {
        MarketCurrency::Usd
    } else {
        return None;
    };
    if host_slot.starts_with("market.price.") {
        let side = if host_slot.contains(".purchase.") {
            MarketPriceSide::Purchase
        } else if host_slot.contains(".sell.") {
            MarketPriceSide::Sell
        } else if host_slot.contains(".base.") {
            MarketPriceSide::Base
        } else {
            return None;
        };
        return Some(MarketBlockKind::Price { currency, side });
    }
    if host_slot.starts_with("market.trade.") {
        let direction = if host_slot.contains(".import.") {
            MarketTradeDirection::Import
        } else if host_slot.contains(".export.") {
            MarketTradeDirection::Export
        } else {
            return None;
        };
        let channel = if host_slot.contains(".international.") {
            MarketTradeChannel::International
        } else {
            MarketTradeChannel::Standard
        };
        return Some(MarketBlockKind::Trade {
            currency,
            direction,
            channel,
        });
    }
    None
}

fn parse_market_block_row(
    block: &mut ActiveMarketBlock,
    line: &str,
    line_number: u64,
    collector: &mut MarketCollector,
    current: Option<&mut RecordDraft>,
    snapshot: Option<&mut SnapshotDraft>,
) {
    let mut parts = line.split_ascii_whitespace();
    let Some(resource) = parts.next() else {
        return;
    };
    let Some(first) = parts.next().and_then(|value| value.parse::<f64>().ok()) else {
        add_warning(&mut collector.warnings, "malformed_market_row");
        return;
    };
    let Some(second) = parts.next().and_then(|value| value.parse::<f64>().ok()) else {
        add_warning(&mut collector.warnings, "malformed_market_row");
        return;
    };
    if parts.next().is_some() || !first.is_finite() || !second.is_finite() {
        add_warning(&mut collector.warnings, "malformed_market_row");
        return;
    }
    let Some(resource_index) = collector.intern_resource(resource) else {
        return;
    };
    if !block.seen_resources.insert(resource_index) {
        add_warning(&mut collector.warnings, "duplicate_market_resource");
        return;
    }
    if !collector.reserve_row() {
        return;
    }
    let Some(rows) = market_rows_mut(current, snapshot) else {
        add_warning(&mut collector.warnings, "market_row_without_scope");
        return;
    };
    let source_line = u32::try_from(line_number).unwrap_or(u32::MAX);
    match block.kind {
        MarketBlockKind::Price { currency, side } => rows.prices.push(MarketPriceRow {
            resource_index,
            source_field_index: block.source_field_index,
            source_line,
            currency,
            side,
            value: first,
            modifier: second,
        }),
        MarketBlockKind::Trade {
            currency,
            direction,
            channel,
        } => rows.trades.push(MarketTradeRow {
            resource_index,
            source_field_index: block.source_field_index,
            source_line,
            currency,
            direction,
            channel,
            quantity: first,
            account_value: second,
        }),
    }
}

fn parse_market_scalar(
    host_slot: &str,
    source_field_index: u16,
    line: &str,
    line_number: u64,
    collector: &mut MarketCollector,
    current: Option<&mut RecordDraft>,
    snapshot: Option<&mut SnapshotDraft>,
) {
    let values = line.split_ascii_whitespace().skip(1).collect::<Vec<_>>();
    let (category, value) = match values.as_slice() {
        [value] => (None, value.parse::<f64>().ok()),
        [category, value] => (category.parse::<i32>().ok(), value.parse::<f64>().ok()),
        _ => {
            add_warning(&mut collector.warnings, "malformed_market_scalar");
            return;
        }
    };
    let Some(value) = value.filter(|value| value.is_finite()) else {
        add_warning(&mut collector.warnings, "malformed_market_scalar");
        return;
    };
    if !collector.reserve_row() {
        return;
    }
    let Some(rows) = market_rows_mut(current, snapshot) else {
        add_warning(&mut collector.warnings, "market_row_without_scope");
        return;
    };
    rows.scalars.push(MarketScalarRow {
        fact_id: host_slot.to_owned(),
        source_field_index,
        source_line: u32::try_from(line_number).unwrap_or(u32::MAX),
        currency: if host_slot.ends_with(".rub") {
            Some(MarketCurrency::Rub)
        } else if host_slot.ends_with(".usd") {
            Some(MarketCurrency::Usd)
        } else {
            None
        },
        category,
        value,
    });
}

fn assign_snapshot_fact(
    profile: &ResolvedCompatibilityProfile,
    snapshot: &mut SnapshotDraft,
    line: &str,
    line_number: u64,
) -> Result<(), ObservatoryError> {
    let Some(source_field) = directive_name(line) else {
        return Ok(());
    };
    let context = match snapshot.scope_kind {
        SnapshotScopeKind::Republic => StatsContext::Republic,
        SnapshotScopeKind::City => StatsContext::City,
    };
    let Some(mapping) = profile.field_for(source_field, context) else {
        return Ok(());
    };
    let Some(definition) = SNAPSHOT_FACTS
        .iter()
        .find(|definition| definition.id == mapping.host_slot)
    else {
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
        definition.id.to_owned(),
        SnapshotFact {
            fact_id: definition.id.to_owned(),
            source_field: source_field.to_owned(),
            value,
            source_line: line_number,
        },
    );
    Ok(())
}

fn finalise_snapshot(
    profile: &ResolvedCompatibilityProfile,
    draft: Option<SnapshotDraft>,
    snapshots: &mut Vec<SaveSnapshot>,
    market_snapshots: &mut Vec<MarketSnapshot>,
) {
    let Some(draft) = draft else {
        return;
    };
    let expected_fact_count = profile.expected_snapshot_fields(match draft.scope_kind {
        SnapshotScopeKind::Republic => StatsContext::Republic,
        SnapshotScopeKind::City => StatsContext::City,
    });
    let coverage = if draft.facts.len() == expected_fact_count as usize {
        CoverageStatus::Complete
    } else {
        CoverageStatus::Partial
    };
    let scope_kind = draft.scope_kind;
    let scope_id = draft.scope_id;
    if !draft.market.prices.is_empty()
        || !draft.market.trades.is_empty()
        || !draft.market.scalars.is_empty()
    {
        market_snapshots.push(MarketSnapshot {
            scope_kind,
            scope_id: scope_id.clone(),
            rows: draft.market,
        });
    }
    snapshots.push(SaveSnapshot {
        scope_kind,
        scope_id,
        facts: draft.facts.into_values().collect(),
        expected_fact_count,
        coverage,
    });
}

fn finalise_record(
    draft: Option<RecordDraft>,
    collector: &mut RecordCollector,
    market: &mut MarketCollector,
    status_enabled: bool,
) -> Result<(), ObservatoryError> {
    let Some(draft) = draft else {
        return Ok(());
    };
    collector.history_records += 1;

    if status_enabled {
        collector.citizen_status_history_records += 1;
        finalise_citizen_status_record(
            &draft,
            &mut collector.citizen_status_records,
            &mut collector.citizen_status_warnings,
            &mut collector.citizen_status_dropped_records,
        );
    }

    if (!draft.market.prices.is_empty()
        || !draft.market.trades.is_empty()
        || !draft.market.scalars.is_empty())
        && let (Some(year), Some(day)) = (&draft.year, &draft.day)
    {
        market.records.push(MarketHistoryRecord {
            record_id: draft.record_id,
            year: year.value,
            day: day.value,
            game_day: i64::from(year.value) * DAYS_PER_GAME_YEAR + i64::from(day.value),
            rows: draft.market.clone(),
        });
    }

    if draft.malformed {
        add_warning(&mut collector.warnings, "malformed_record");
        collector.dropped_records += 1;
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
        add_warning(&mut collector.warnings, "incomplete_record");
        collector.dropped_records += 1;
        return Ok(());
    };

    let classified_total = none
        .value
        .checked_add(radio.value)
        .and_then(|value| value.checked_add(television.value))
        .and_then(|value| value.checked_add(computer.value));
    let Some(classified_total) = classified_total else {
        add_warning(&mut collector.warnings, "classified_total_overflow");
        collector.dropped_records += 1;
        return Ok(());
    };

    if classified_total == 0 {
        add_warning(&mut collector.warnings, "zero_classified_population");
    } else {
        collector.chartable_records += 1;
    }

    collector.records.push(ReceiverRecord {
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
        source_fields: SourceFieldSet {
            none: none.source_field,
            radio: radio.source_field,
            television: television.source_field,
            computer: computer.source_field,
        },
    });
    Ok(())
}

fn finalise_citizen_status_record(
    draft: &RecordDraft,
    records: &mut Vec<CitizenStatusRecord>,
    warnings: &mut BTreeMap<String, u32>,
    dropped_records: &mut u32,
) {
    let Some(year) = draft.year.as_ref() else {
        *dropped_records += 1;
        add_warning(warnings, "citizen_status_date_unavailable");
        return;
    };
    let Some(day) = draft.day.as_ref() else {
        *dropped_records += 1;
        add_warning(warnings, "citizen_status_date_unavailable");
        return;
    };
    if !draft.citizen_status_seen
        || draft.citizen_status_malformed
        || draft.citizen_status.iter().any(Option::is_none)
    {
        *dropped_records += 1;
        add_warning(warnings, "incomplete_citizen_status_record");
        return;
    }
    let values = std::array::from_fn(|index| {
        draft.citizen_status[index]
            .as_ref()
            .expect("completeness checked")
            .value
    });
    let source_lines = std::array::from_fn(|index| {
        draft.citizen_status[index]
            .as_ref()
            .expect("completeness checked")
            .line
    });
    let source_fields = std::array::from_fn(|index| {
        draft.citizen_status[index]
            .as_ref()
            .expect("completeness checked")
            .source_field
            .clone()
    });
    records.push(CitizenStatusRecord {
        record_id: draft.record_id,
        year: year.value,
        day: day.value,
        game_day: i64::from(year.value) * DAYS_PER_GAME_YEAR + i64::from(day.value),
        values,
        source_lines,
        source_fields,
    });
}

fn assign_field<T: Copy>(
    target: &mut Option<FieldValue<T>>,
    value: Option<T>,
    line: u64,
    source_field: &str,
    malformed: &mut bool,
) -> Result<(), ObservatoryError> {
    if target.is_some() {
        return Err(ObservatoryError::MalformedReceiverHistory(
            "duplicate field in one record",
        ));
    }
    match value {
        Some(value) => {
            *target = Some(FieldValue {
                value,
                line,
                source_field: source_field.to_owned(),
            })
        }
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

fn parse_indexed_value<I: std::str::FromStr, T: std::str::FromStr>(line: &str) -> Option<(I, T)> {
    let mut parts = line.split_ascii_whitespace();
    parts.next()?;
    let index = parts.next()?.parse().ok()?;
    let value = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((index, value))
}

fn add_warning(warnings: &mut BTreeMap<String, u32>, code: &str) {
    *warnings.entry(code.to_owned()).or_default() += 1;
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::parse_stats;
    use crate::compatibility_profile::ResolvedCompatibilityProfile;
    use crate::error::ObservatoryError;
    use crate::model::{CoverageStatus, SnapshotScopeKind};

    #[test]
    fn parses_complete_receiver_history_and_closes_it_at_current_block() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let parsed = parse_stats(
            Cursor::new(include_bytes!("../fixtures/valid.receiver-stats.txt")),
            &profile,
        )
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
    fn parses_complete_indexed_citizen_status_history() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let parsed = parse_stats(
            Cursor::new(include_bytes!("../fixtures/citizen-status.stats.txt")),
            &profile,
        )
        .expect("valid citizen status fixture");

        assert_eq!(parsed.citizen_status.records.len(), 2);
        assert_eq!(parsed.citizen_status.history_records, 2);
        assert_eq!(parsed.citizen_status.dropped_records, 0);
        assert!(parsed.citizen_status.warnings.is_empty());
        assert_eq!(parsed.citizen_status.records[0].values[0], 0.81);
        assert_eq!(parsed.citizen_status.records[1].values[8], 0.74);
        assert_eq!(parsed.citizen_status.records[0].source_lines.len(), 9);
    }

    #[test]
    fn isolates_incomplete_and_invalid_status_rows_from_receivers() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let source = include_str!("../fixtures/citizen-status.stats.txt")
            .replace("$Citizens_Status 8 0.74", "$Citizens_Status 8 1.25")
            .replace("$Citizens_Status 7 0.43\n", "");
        let parsed = parse_stats(Cursor::new(source.as_bytes()), &profile)
            .expect("receiver evidence remains usable");

        assert_eq!(parsed.records.len(), 2);
        assert_eq!(parsed.records[1].classified_total, 100);
        assert_eq!(parsed.citizen_status.records.len(), 1);
        assert_eq!(parsed.citizen_status.dropped_records, 1);
        assert!(
            parsed
                .citizen_status
                .warnings
                .iter()
                .any(|warning| { warning.code == "out_of_range_citizen_status_value" })
        );
        assert!(
            parsed
                .citizen_status
                .warnings
                .iter()
                .any(|warning| { warning.code == "incomplete_citizen_status_record" })
        );
    }

    #[test]
    fn isolates_duplicate_status_indices_from_receivers() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let source = include_str!("../fixtures/citizen-status.stats.txt").replace(
            "$Citizens_Status 8 0.74",
            "$Citizens_Status 8 0.74\n$Citizens_Status 8 0.75",
        );
        let parsed = parse_stats(Cursor::new(source.as_bytes()), &profile)
            .expect("receiver evidence remains usable");

        assert_eq!(parsed.records.len(), 2);
        assert_eq!(parsed.citizen_status.records.len(), 1);
        assert!(
            parsed
                .citizen_status
                .warnings
                .iter()
                .any(|warning| { warning.code == "duplicate_citizen_status_index" })
        );
    }

    #[test]
    fn reports_partial_coverage_without_inventing_a_missing_metric() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let parsed = parse_stats(
            Cursor::new(include_bytes!("../fixtures/partial.receiver-stats.txt")),
            &profile,
        )
        .expect("partially usable fixture");

        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.coverage.history_records, 2);
        assert_eq!(parsed.coverage.dropped_records, 1);
        assert_eq!(parsed.coverage.status, CoverageStatus::Partial);
        assert_eq!(parsed.coverage.warnings[0].code, "incomplete_record");
    }

    #[test]
    fn parses_market_blocks_without_changing_receiver_results() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let parsed = parse_stats(
            Cursor::new(include_bytes!("../fixtures/market.stats.txt")),
            &profile,
        )
        .expect("market fixture");

        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].classified_total, 100);
        assert_eq!(parsed.market.records.len(), 1);
        assert_eq!(parsed.market.records[0].rows.prices.len(), 2);
        assert_eq!(parsed.market.records[0].rows.trades.len(), 4);
        assert_eq!(parsed.market.records[0].rows.scalars.len(), 2);
        assert_eq!(parsed.market.snapshots.len(), 2);
        assert_eq!(parsed.market.row_count, 10);
        assert!(parsed.market.warnings.is_empty());
        let waste = parsed.market.records[0]
            .rows
            .trades
            .iter()
            .find(|row| parsed.market.resources[row.resource_index as usize] == "waste")
            .expect("signed disposal row");
        assert_eq!(waste.account_value, -8.0);
    }

    #[test]
    fn ignores_the_game_market_block_separator_without_hiding_bad_rows() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let input = include_str!("../fixtures/market.stats.txt")
            .replace("fuel 20 0.95\n$end", "fuel 20 0.95\n-------------\n$end");
        let parsed = parse_stats(Cursor::new(input.as_bytes()), &profile)
            .expect("decorative separator remains valid");

        assert_eq!(parsed.market.records[0].rows.prices.len(), 2);
        assert!(parsed.market.warnings.is_empty());

        let malformed = input.replace("-------------", "not-a-market-row");
        let parsed = parse_stats(Cursor::new(malformed.as_bytes()), &profile)
            .expect("receiver facts remain isolated from a malformed market row");
        assert!(
            parsed
                .market
                .warnings
                .iter()
                .any(|warning| warning.code == "malformed_market_row")
        );
    }

    #[test]
    fn isolates_malformed_market_rows_from_receiver_history() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let input = include_str!("../fixtures/market.stats.txt").replace("oil 12.5 1", "oil NaN 1");
        let parsed =
            parse_stats(Cursor::new(input.as_bytes()), &profile).expect("receiver remains usable");
        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].classified_total, 100);
        assert_eq!(parsed.market.coverage_status(), CoverageStatus::Partial);
        assert!(
            parsed
                .market
                .warnings
                .iter()
                .any(|warning| warning.code == "malformed_market_row")
        );
    }

    #[test]
    fn drops_a_malformed_record_but_preserves_a_later_valid_record() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let parsed = parse_stats(
            Cursor::new(include_bytes!("../fixtures/malformed.receiver-stats.txt")),
            &profile,
        )
        .expect("one record remains usable");

        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].record_id, 1);
        assert_eq!(parsed.coverage.warnings[0].code, "malformed_record");
    }

    #[test]
    fn rejects_duplicate_record_identifiers() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let error = parse_stats(
            Cursor::new(include_bytes!(
                "../fixtures/duplicate-ids.receiver-stats.txt"
            )),
            &profile,
        )
        .expect_err("duplicate ids must fail");
        assert!(matches!(
            error,
            ObservatoryError::MalformedReceiverHistory("duplicate record identifier")
        ));
    }

    #[test]
    fn rejects_an_explicit_unsupported_format() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let error = parse_stats(
            Cursor::new(include_bytes!(
                "../fixtures/unsupported-version.receiver-stats.txt"
            )),
            &profile,
        )
        .expect_err("unsupported format must fail");
        assert!(matches!(error, ObservatoryError::UnsupportedStatsFormat));
    }

    #[test]
    fn captures_supported_current_and_city_snapshot_facts() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let parsed = parse_stats(
            Cursor::new(include_bytes!(
                "../fixtures/current-city.receiver-stats.txt"
            )),
            &profile,
        )
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
