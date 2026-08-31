use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::io::BufRead;

use sha2::{Digest, Sha256};

use crate::compatibility_profile::{ResolvedCompatibilityProfile, StatsContext, StatsMarkerSlot};
use crate::error::ObservatoryError;
use crate::model::{
    CoverageReport, CoverageStatus, CoverageWarning, MarketCurrency, MarketFactRows,
    MarketHistoryRecord, MarketPriceRow, MarketPriceSide, MarketScalarRow, MarketSnapshot,
    MarketTradeChannel, MarketTradeDirection, MarketTradeRow, ParsedMarketData, ParsedStats,
    ReceiverRecord, SNAPSHOT_FACTS, SaveSnapshot, SnapshotFact, SnapshotScopeKind, SourceFieldSet,
    SourceLineSet,
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
    let mut records = Vec::new();
    let mut snapshots = Vec::new();
    let mut seen_city_ids = HashSet::new();
    let mut warnings = BTreeMap::<String, u32>::new();
    let mut seen_record_ids = HashSet::new();
    let mut last_record_id = None;
    let mut history_records = 0_u32;
    let mut dropped_records = 0_u32;
    let mut chartable_records = 0_u32;
    let mut market = MarketCollector::default();
    let mut active_market_block: Option<ActiveMarketBlock> = None;

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
                &mut records,
                &mut market.records,
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

        if marker.is_some_and(|marker| marker.slot == StatsMarkerSlot::CurrentState) {
            finalise_record(
                current.take(),
                &mut records,
                &mut market.records,
                &mut warnings,
                &mut history_records,
                &mut dropped_records,
                &mut chartable_records,
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
                &mut records,
                &mut market.records,
                &mut warnings,
                &mut history_records,
                &mut dropped_records,
                &mut chartable_records,
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

    finalise_record(
        current,
        &mut records,
        &mut market.records,
        &mut warnings,
        &mut history_records,
        &mut dropped_records,
        &mut chartable_records,
    )?;
    if active_market_block.is_some() {
        add_warning(&mut market.warnings, "unterminated_market_block");
    }
    finalise_snapshot(
        profile,
        current_snapshot,
        &mut snapshots,
        &mut market.snapshots,
    );

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
    })
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
    records: &mut Vec<ReceiverRecord>,
    market_records: &mut Vec<MarketHistoryRecord>,
    warnings: &mut BTreeMap<String, u32>,
    history_records: &mut u32,
    dropped_records: &mut u32,
    chartable_records: &mut u32,
) -> Result<(), ObservatoryError> {
    let Some(draft) = draft else {
        return Ok(());
    };
    *history_records += 1;

    if (!draft.market.prices.is_empty()
        || !draft.market.trades.is_empty()
        || !draft.market.scalars.is_empty())
        && let (Some(year), Some(day)) = (&draft.year, &draft.day)
    {
        market_records.push(MarketHistoryRecord {
            record_id: draft.record_id,
            year: year.value,
            day: day.value,
            game_day: i64::from(year.value) * DAYS_PER_GAME_YEAR + i64::from(day.value),
            rows: draft.market.clone(),
        });
    }

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
        source_fields: SourceFieldSet {
            none: none.source_field,
            radio: radio.source_field,
            television: television.source_field,
            computer: computer.source_field,
        },
    });
    Ok(())
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
