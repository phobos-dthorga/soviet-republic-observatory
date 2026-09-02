use std::collections::{BTreeMap, HashSet};

use sha2::{Digest, Sha256};

use crate::error::ObservatoryError;
use crate::model::{
    CarbonEstimateContribution, CarbonEstimateModel, CarbonFactorEntry, CarbonFactorImportPreview,
    CarbonFactorRevision, CarbonFactorSetDraft, EnvironmentActivityChannel,
};

pub const ENVIRONMENT_STORAGE_CONTRACT_VERSION: u32 = 1;
pub const ENVIRONMENT_RECORDING_NOTICE_REVISION: u32 = 1;
pub const ENVIRONMENT_RECORDING_INTERVAL_GAME_DAYS: u16 = 7;
const MAX_FACTOR_ENTRIES: usize = 512;
const MAX_CSV_BYTES: usize = 256 * 1024;

pub(crate) fn validate_factor_draft(
    draft: &CarbonFactorSetDraft,
    known_resources: &HashSet<String>,
) -> Result<(), ObservatoryError> {
    validate_text(&draft.name, 1, 100, "invalid_name")?;
    validate_text(
        &draft.accounting_boundary,
        1,
        240,
        "invalid_accounting_boundary",
    )?;
    validate_text(&draft.reason, 1, 500, "invalid_reason")?;
    if draft.entries.is_empty() || draft.entries.len() > MAX_FACTOR_ENTRIES {
        return Err(ObservatoryError::InvalidCarbonFactorSet(
            "invalid_entry_count",
        ));
    }
    if let Some(id) = &draft.factor_set_id
        && !valid_factor_set_id(id)
    {
        return Err(ObservatoryError::InvalidCarbonFactorSet("invalid_id"));
    }
    let mut identities = HashSet::new();
    for entry in &draft.entries {
        if !known_resources.contains(&entry.resource_token) {
            return Err(ObservatoryError::InvalidCarbonFactorSet("unknown_resource"));
        }
        if !entry.activity_channel.quantity_is_publishable() {
            return Err(ObservatoryError::InvalidCarbonFactorSet(
                "unverified_activity_channel",
            ));
        }
        if !identities.insert((entry.resource_token.as_str(), entry.activity_channel)) {
            return Err(ObservatoryError::InvalidCarbonFactorSet(
                "duplicate_resource_channel",
            ));
        }
        if !entry.grams_co2e_per_unit.is_finite()
            || entry.grams_co2e_per_unit < 0.0
            || entry.grams_co2e_per_unit > 1.0e15
        {
            return Err(ObservatoryError::InvalidCarbonFactorSet("invalid_factor"));
        }
        if !(1900..=9999).contains(&entry.source_year) {
            return Err(ObservatoryError::InvalidCarbonFactorSet(
                "invalid_source_year",
            ));
        }
        validate_text(&entry.source_name, 1, 160, "invalid_source_name")?;
        validate_text(&entry.reason, 1, 500, "invalid_entry_reason")?;
        if let Some(reference) = &entry.reference {
            validate_text(reference, 1, 500, "invalid_reference")?;
        }
    }
    Ok(())
}

fn validate_text(
    value: &str,
    min: usize,
    max: usize,
    reason: &'static str,
) -> Result<(), ObservatoryError> {
    let count = value.chars().count();
    if count < min || count > max || value.chars().any(char::is_control) {
        return Err(ObservatoryError::InvalidCarbonFactorSet(reason));
    }
    Ok(())
}

pub(crate) fn valid_factor_set_id(value: &str) -> bool {
    (3..=80).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-_".contains(&byte)
        })
}

pub(crate) fn factor_content_hash(draft: &CarbonFactorSetDraft) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"republic-observatory-carbon-factor-set-v1\0");
    hasher.update(draft.name.as_bytes());
    hasher.update(b"\0");
    hasher.update(draft.accounting_boundary.as_bytes());
    hasher.update(b"\0");
    hasher.update(draft.reason.as_bytes());
    for entry in &draft.entries {
        hasher.update(b"\0");
        hasher.update(entry.resource_token.as_bytes());
        hasher.update(b"\0");
        hasher.update(entry.activity_channel.as_str().as_bytes());
        hasher.update(entry.grams_co2e_per_unit.to_bits().to_le_bytes());
        hasher.update(entry.source_name.as_bytes());
        hasher.update(entry.source_year.to_le_bytes());
        hasher.update(entry.reason.as_bytes());
        if let Some(reference) = &entry.reference {
            hasher.update(reference.as_bytes());
        }
    }
    hex_digest(hasher)
}

pub(crate) fn generated_factor_set_id(name: &str, created_at_ms: i64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(created_at_ms.to_le_bytes());
    format!("carbon-{}", &hex_digest(hasher)[..24])
}

fn hex_digest(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn calculate_estimate(
    revision: Option<&CarbonFactorRevision>,
    quantities: &[(String, EnvironmentActivityChannel, f64, u32)],
) -> CarbonEstimateModel {
    let eligible_rows = quantities
        .iter()
        .map(|entry| entry.3)
        .fold(0_u32, u32::saturating_add);
    let Some(revision) = revision else {
        return CarbonEstimateModel {
            available: false,
            factor_set_id: None,
            factor_set_revision: None,
            estimated_grams_co2e: None,
            covered_rows: 0,
            eligible_rows,
            coverage_percent: 0.0,
            missing_factors: Vec::new(),
            contributions: Vec::new(),
            limitation: Some("select_or_create_factor_set".to_owned()),
        };
    };
    let factors = revision
        .entries
        .iter()
        .map(|entry| {
            (
                (entry.resource_token.as_str(), entry.activity_channel),
                entry.grams_co2e_per_unit,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut total = 0.0;
    let mut covered_rows = 0_u32;
    let mut missing_factors = Vec::new();
    let mut contributions = Vec::new();
    for (resource, channel, quantity, rows) in quantities {
        let Some(factor) = factors.get(&(resource.as_str(), *channel)).copied() else {
            missing_factors.push(format!("{}:{}", channel.as_str(), resource));
            continue;
        };
        let estimated = quantity * factor;
        if !estimated.is_finite() {
            continue;
        }
        total += estimated;
        covered_rows = covered_rows.saturating_add(*rows);
        contributions.push(CarbonEstimateContribution {
            resource_token: resource.clone(),
            activity_channel: *channel,
            recorded_quantity: *quantity,
            grams_co2e_per_unit: factor,
            estimated_grams_co2e: estimated,
        });
    }
    contributions.sort_by(|left, right| {
        right
            .estimated_grams_co2e
            .total_cmp(&left.estimated_grams_co2e)
    });
    missing_factors.sort();
    missing_factors.dedup();
    let available = covered_rows > 0;
    CarbonEstimateModel {
        available,
        factor_set_id: Some(revision.factor_set_id.clone()),
        factor_set_revision: Some(revision.revision),
        estimated_grams_co2e: available.then_some(total),
        covered_rows,
        eligible_rows,
        coverage_percent: if eligible_rows == 0 {
            0.0
        } else {
            f64::from(covered_rows) * 100.0 / f64::from(eligible_rows)
        },
        missing_factors,
        contributions,
        limitation: (!available).then(|| "no_covered_activity".to_owned()),
    }
}

pub fn export_factor_csv(revision: &CarbonFactorRevision) -> String {
    let mut output = String::from(
        "factor_set_name,accounting_boundary,set_reason,resource_token,activity_channel,grams_co2e_per_unit,source_name,source_year,entry_reason,reference\n",
    );
    for entry in &revision.entries {
        let values = [
            revision.name.clone(),
            revision.accounting_boundary.clone(),
            revision.reason.clone(),
            entry.resource_token.clone(),
            entry.activity_channel.as_str().to_owned(),
            entry.grams_co2e_per_unit.to_string(),
            entry.source_name.clone(),
            entry.source_year.to_string(),
            entry.reason.clone(),
            entry.reference.clone().unwrap_or_default(),
        ];
        output.push_str(
            &values
                .iter()
                .map(|value| csv_escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    output
}

pub fn preview_factor_csv(csv: &str) -> CarbonFactorImportPreview {
    match parse_factor_csv(csv) {
        Ok(draft) => CarbonFactorImportPreview {
            valid: true,
            row_count: draft.entries.len().min(u32::MAX as usize) as u32,
            errors: Vec::new(),
            draft: Some(draft),
        },
        Err(reason) => CarbonFactorImportPreview {
            valid: false,
            row_count: 0,
            errors: vec![reason.to_owned()],
            draft: None,
        },
    }
}

fn parse_factor_csv(csv: &str) -> Result<CarbonFactorSetDraft, &'static str> {
    if csv.is_empty() || csv.len() > MAX_CSV_BYTES || csv.contains('\0') {
        return Err("invalid_file_size");
    }
    let rows = parse_csv_rows(csv)?;
    let expected = [
        "factor_set_name",
        "accounting_boundary",
        "set_reason",
        "resource_token",
        "activity_channel",
        "grams_co2e_per_unit",
        "source_name",
        "source_year",
        "entry_reason",
        "reference",
    ];
    if rows
        .first()
        .map(|row| row.iter().map(String::as_str).collect::<Vec<_>>())
        != Some(expected.to_vec())
    {
        return Err("invalid_header");
    }
    if rows.len() < 2 || rows.len() > MAX_FACTOR_ENTRIES + 1 {
        return Err("invalid_entry_count");
    }
    let first = &rows[1];
    let mut entries = Vec::new();
    for row in rows.iter().skip(1) {
        if row.len() != expected.len() || row[..3] != first[..3] {
            return Err("inconsistent_factor_set");
        }
        if row.iter().any(|value| spreadsheet_formula(value)) {
            return Err("spreadsheet_formula_rejected");
        }
        let channel = match row[4].as_str() {
            "production" => EnvironmentActivityChannel::Production,
            "construction_use" => EnvironmentActivityChannel::ConstructionUse,
            "factory_use" => EnvironmentActivityChannel::FactoryUse,
            "shop_use" => EnvironmentActivityChannel::ShopUse,
            "vehicle_use" => EnvironmentActivityChannel::VehicleUse,
            _ => return Err("unknown_activity_channel"),
        };
        let factor = row[5].parse::<f64>().map_err(|_| "invalid_factor")?;
        let source_year = row[7].parse::<u16>().map_err(|_| "invalid_source_year")?;
        entries.push(CarbonFactorEntry {
            resource_token: row[3].clone(),
            activity_channel: channel,
            grams_co2e_per_unit: factor,
            source_name: row[6].clone(),
            source_year,
            reason: row[8].clone(),
            reference: (!row[9].is_empty()).then(|| row[9].clone()),
        });
    }
    Ok(CarbonFactorSetDraft {
        factor_set_id: None,
        name: first[0].clone(),
        accounting_boundary: first[1].clone(),
        reason: first[2].clone(),
        entries,
    })
}

fn spreadsheet_formula(value: &str) -> bool {
    value
        .trim_start()
        .bytes()
        .next()
        .is_some_and(|byte| b"=+-@".contains(&byte))
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn parse_csv_rows(input: &str) -> Result<Vec<Vec<String>>, &'static str> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                if field.ends_with('\r') {
                    field.pop();
                }
                row.push(std::mem::take(&mut field));
                if row.iter().any(|value| !value.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            character => field.push(character),
        }
    }
    if quoted {
        return Err("unterminated_quote");
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field.trim_end_matches('\r').to_owned());
        rows.push(row);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carbon_estimate_uses_only_exact_factor_matches() {
        let revision = CarbonFactorRevision {
            factor_set_id: "carbon-test".to_owned(),
            revision: 1,
            name: "Test".to_owned(),
            accounting_boundary: "Production only".to_owned(),
            reason: "Fixture".to_owned(),
            created_at_ms: 1,
            content_hash: "a".repeat(64),
            selected: true,
            entries: vec![CarbonFactorEntry {
                resource_token: "steel".to_owned(),
                activity_channel: EnvironmentActivityChannel::Production,
                grams_co2e_per_unit: 20.0,
                source_name: "Study".to_owned(),
                source_year: 2025,
                reason: "Fixture".to_owned(),
                reference: None,
            }],
        };
        let estimate = calculate_estimate(
            Some(&revision),
            &[
                (
                    "steel".to_owned(),
                    EnvironmentActivityChannel::Production,
                    4.0,
                    1,
                ),
                (
                    "coal".to_owned(),
                    EnvironmentActivityChannel::Production,
                    2.0,
                    1,
                ),
            ],
        );
        assert_eq!(estimate.estimated_grams_co2e, Some(80.0));
        assert_eq!(estimate.coverage_percent, 50.0);
        assert_eq!(estimate.missing_factors, vec!["production:coal"]);
    }

    #[test]
    fn csv_round_trip_preserves_factor_entries() {
        let revision = CarbonFactorRevision {
            factor_set_id: "carbon-test".to_owned(),
            revision: 1,
            name: "Test, local".to_owned(),
            accounting_boundary: "Production".to_owned(),
            reason: "A \"study\"".to_owned(),
            created_at_ms: 1,
            content_hash: "a".repeat(64),
            selected: true,
            entries: vec![CarbonFactorEntry {
                resource_token: "steel".to_owned(),
                activity_channel: EnvironmentActivityChannel::Production,
                grams_co2e_per_unit: 2.5,
                source_name: "Study".to_owned(),
                source_year: 2025,
                reason: "Fixture".to_owned(),
                reference: Some("local note".to_owned()),
            }],
        };
        let preview = preview_factor_csv(&export_factor_csv(&revision));
        assert!(preview.valid);
        assert_eq!(preview.draft.expect("draft").entries, revision.entries);
    }

    #[test]
    fn csv_rejects_spreadsheet_formulas() {
        let csv = "factor_set_name,accounting_boundary,set_reason,resource_token,activity_channel,grams_co2e_per_unit,source_name,source_year,entry_reason,reference\nTest,Production,Reason,steel,production,2,=FETCH(),2025,Reason,\n";
        assert_eq!(
            preview_factor_csv(csv).errors,
            vec!["spreadsheet_formula_rejected"]
        );
    }

    #[test]
    fn factor_validation_rejects_unknown_resources_duplicates_and_negative_values() {
        let known = HashSet::from(["steel".to_owned()]);
        let entry = CarbonFactorEntry {
            resource_token: "steel".to_owned(),
            activity_channel: EnvironmentActivityChannel::Production,
            grams_co2e_per_unit: 20.0,
            source_name: "Study".to_owned(),
            source_year: 2025,
            reason: "Fixture".to_owned(),
            reference: None,
        };
        let mut draft = CarbonFactorSetDraft {
            factor_set_id: None,
            name: "Factory study".to_owned(),
            accounting_boundary: "Production only".to_owned(),
            reason: "Player comparison".to_owned(),
            entries: vec![entry.clone()],
        };
        validate_factor_draft(&draft, &known).expect("valid draft");

        draft.entries[0].resource_token = "unknown".to_owned();
        assert!(matches!(
            validate_factor_draft(&draft, &known),
            Err(ObservatoryError::InvalidCarbonFactorSet("unknown_resource"))
        ));

        draft.entries = vec![entry.clone(), entry.clone()];
        assert!(matches!(
            validate_factor_draft(&draft, &known),
            Err(ObservatoryError::InvalidCarbonFactorSet(
                "duplicate_resource_channel"
            ))
        ));

        draft.entries = vec![CarbonFactorEntry {
            grams_co2e_per_unit: -1.0,
            ..entry
        }];
        assert!(matches!(
            validate_factor_draft(&draft, &known),
            Err(ObservatoryError::InvalidCarbonFactorSet("invalid_factor"))
        ));
    }
}
