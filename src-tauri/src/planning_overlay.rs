use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::error::ObservatoryError;

pub const OVERLAY_SCHEMA_VERSION: u32 = 1;
pub const MAX_OVERLAY_BYTES: usize = 1024 * 1024;
pub const MAX_OPERATIONS: usize = 4_096;
pub const MAX_SUPPLEMENTS: usize = 512;
pub const MAX_SUPPLEMENT_PROPERTIES: usize = 256;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningOverlayDocument {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub name: String,
    pub author: String,
    pub default_locale: String,
    pub description: String,
    #[serde(default)]
    pub target_game_build: Option<String>,
    #[serde(default)]
    pub operations: Vec<OverlayOperation>,
    #[serde(default)]
    pub supplements: Vec<OverlaySupplement>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayOperationKind {
    Set,
    Unset,
    Add,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayOperation {
    pub operation: OverlayOperationKind,
    pub entity_id: String,
    pub field_id: String,
    #[serde(default)]
    pub occurrence: Option<u32>,
    pub expected_revision_hash: String,
    #[serde(default)]
    pub expected_value: Option<OverlayValue>,
    #[serde(default)]
    pub value: Option<OverlayValue>,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayValueKind {
    Number,
    Text,
    Boolean,
}

impl OverlayValueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Text => "text",
            Self::Boolean => "boolean",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OverlayValue {
    pub kind: OverlayValueKind,
    #[serde(default)]
    pub number: Option<f64>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub boolean: Option<bool>,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlaySupplement {
    pub local_id: String,
    pub entity_kind: String,
    pub display_name: String,
    pub reason: String,
    #[serde(default)]
    pub properties: Vec<OverlaySupplementProperty>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlaySupplementProperty {
    pub field_id: String,
    #[serde(default)]
    pub occurrence: u32,
    pub value: OverlayValue,
}

impl PlanningOverlayDocument {
    pub fn parse(bytes: &[u8]) -> Result<Self, ObservatoryError> {
        if bytes.is_empty() || bytes.len() > MAX_OVERLAY_BYTES {
            return Err(ObservatoryError::InvalidPlanningOverlay("size_limit"));
        }
        let document = serde_json::from_slice::<Self>(bytes)
            .map_err(|_| ObservatoryError::InvalidPlanningOverlay("invalid_json"))?;
        document.validate()?;
        Ok(document)
    }

    pub fn validate(&self) -> Result<(), ObservatoryError> {
        if self.schema_version != OVERLAY_SCHEMA_VERSION {
            return Err(ObservatoryError::InvalidPlanningOverlay(
                "unsupported_version",
            ));
        }
        if !valid_reverse_domain_id(&self.id) || !valid_semver(&self.version) {
            return Err(ObservatoryError::InvalidPlanningOverlay(
                "invalid_identifier",
            ));
        }
        if !safe_text(&self.name, 120)
            || !safe_text(&self.author, 120)
            || !valid_locale(&self.default_locale)
            || !safe_text(&self.description, 500)
            || self
                .target_game_build
                .as_deref()
                .is_some_and(|value| !safe_identifier(value, 80))
        {
            return Err(ObservatoryError::InvalidPlanningOverlay("invalid_metadata"));
        }
        if self.operations.len() > MAX_OPERATIONS || self.supplements.len() > MAX_SUPPLEMENTS {
            return Err(ObservatoryError::InvalidPlanningOverlay("limit_exceeded"));
        }
        for operation in &self.operations {
            if !valid_entity_id(&operation.entity_id)
                || !valid_field_id(&operation.field_id)
                || !valid_hash(&operation.expected_revision_hash)
                || !safe_text(&operation.reason, 300)
            {
                return Err(ObservatoryError::InvalidPlanningOverlay(
                    "invalid_operation",
                ));
            }
            let requires_value = operation.operation != OverlayOperationKind::Unset;
            if requires_value != operation.value.is_some()
                || operation
                    .value
                    .as_ref()
                    .is_some_and(|value| !valid_value(value))
                || operation
                    .expected_value
                    .as_ref()
                    .is_some_and(|value| !valid_value(value))
            {
                return Err(ObservatoryError::InvalidPlanningOverlay("invalid_value"));
            }
        }
        let mut supplement_ids = BTreeSet::new();
        for supplement in &self.supplements {
            if !safe_local_id(&supplement.local_id)
                || !supplement_ids.insert(&supplement.local_id)
                || !matches!(
                    supplement.entity_kind.as_str(),
                    "resource" | "building" | "vehicle" | "recipe"
                )
                || !safe_text(&supplement.display_name, 120)
                || !safe_text(&supplement.reason, 300)
                || supplement.properties.len() > MAX_SUPPLEMENT_PROPERTIES
            {
                return Err(ObservatoryError::InvalidPlanningOverlay(
                    "invalid_supplement",
                ));
            }
            let mut property_ids = BTreeSet::new();
            if supplement.properties.iter().any(|property| {
                !valid_field_id(&property.field_id)
                    || !valid_value(&property.value)
                    || !property_ids.insert((&property.field_id, property.occurrence))
            }) {
                return Err(ObservatoryError::InvalidPlanningOverlay(
                    "invalid_supplement",
                ));
            }
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> Result<String, ObservatoryError> {
        serde_json::to_string_pretty(self)
            .map_err(|_| ObservatoryError::InvalidPlanningOverlay("invalid_json"))
    }

    pub fn content_hash(&self) -> Result<String, ObservatoryError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|_| ObservatoryError::InvalidPlanningOverlay("invalid_json"))?;
        Ok(Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }
}

fn valid_value(value: &OverlayValue) -> bool {
    let payload_valid = match value.kind {
        OverlayValueKind::Number => {
            value.number.is_some_and(f64::is_finite)
                && value.text.is_none()
                && value.boolean.is_none()
        }
        OverlayValueKind::Text => {
            value
                .text
                .as_deref()
                .is_some_and(|text| safe_text(text, 240))
                && value.number.is_none()
                && value.boolean.is_none()
        }
        OverlayValueKind::Boolean => {
            value.boolean.is_some() && value.number.is_none() && value.text.is_none()
        }
    };
    payload_valid && value.unit.as_deref().is_none_or(valid_unit)
}

fn valid_reverse_domain_id(value: &str) -> bool {
    value.len() <= 96
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
        && value.split('.').count() >= 3
        && value.split('.').all(safe_segment)
}

fn valid_field_id(value: &str) -> bool {
    value.len() <= 128
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
        && value.split('.').count() >= 2
        && value.split('.').all(safe_segment)
}

fn valid_entity_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 320
        && !value.contains(['\\', '<', '>', '\0'])
        && !value.contains("..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | ':' | '/' | '_' | '-')
        })
}

fn safe_local_id(value: &str) -> bool {
    value.len() <= 64
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
        && value.split(['.', '_', '-']).all(safe_segment)
}

fn safe_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        && value
            .chars()
            .last()
            .is_some_and(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
}

fn safe_identifier(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn valid_unit(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || character == ' '
                || matches!(character, '%' | '/' | '.' | '_' | '*' | '^' | '-')
        })
}

fn safe_text(value: &str, maximum: usize) -> bool {
    let lower = value.to_ascii_lowercase();
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.contains(['<', '>', '\\'])
        && !value.contains("://")
        && !lower.contains("file:")
        && !lower.contains("javascript:")
        && !lower.contains("data:")
        && !value.split_ascii_whitespace().any(|token| {
            token.starts_with('/')
                || token.starts_with("./")
                || token.starts_with("../")
                || (token.len() >= 3
                    && token.as_bytes()[0].is_ascii_alphabetic()
                    && token.as_bytes()[1] == b':')
        })
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

fn valid_locale(value: &str) -> bool {
    value.len() <= 32
        && value.split('-').all(|part| {
            !part.is_empty()
                && part.len() <= 8
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
}

fn valid_semver(value: &str) -> bool {
    if value.is_empty() || value.len() > 32 {
        return false;
    }
    let mut build_split = value.split('+');
    let without_build = build_split.next().unwrap_or_default();
    let build = build_split.next();
    if build_split.next().is_some()
        || build.is_some_and(|part| !valid_semver_identifiers(part, false))
    {
        return false;
    }
    let mut pre_split = without_build.split('-');
    let core = pre_split.next().unwrap_or_default();
    let pre_release = pre_split.next();
    if pre_split.next().is_some()
        || pre_release.is_some_and(|part| !valid_semver_identifiers(part, true))
    {
        return false;
    }
    let mut parts = core.split('.');
    let valid = (0..3).all(|_| {
        parts.next().is_some_and(|part| {
            !part.is_empty()
                && part.chars().all(|character| character.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'))
        })
    });
    valid && parts.next().is_none()
}

fn valid_semver_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
                && (!reject_numeric_leading_zero
                    || !identifier
                        .chars()
                        .all(|character| character.is_ascii_digit())
                    || identifier == "0"
                    || !identifier.starts_with('0'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_document() -> PlanningOverlayDocument {
        PlanningOverlayDocument {
            schema_version: 1,
            id: "org.example.heavy-industry".to_owned(),
            version: "1.0.0".to_owned(),
            name: "Heavy industry assumptions".to_owned(),
            author: "Example planner".to_owned(),
            default_locale: "en-AU".to_owned(),
            description: "A synthetic planning overlay.".to_owned(),
            target_game_build: None,
            operations: vec![OverlayOperation {
                operation: OverlayOperationKind::Set,
                entity_id: "base::building::buildings_types/chemical_plant".to_owned(),
                field_id: "building.workers.required".to_owned(),
                occurrence: Some(0),
                expected_revision_hash: "a".repeat(64),
                expected_value: None,
                value: Some(OverlayValue {
                    kind: OverlayValueKind::Number,
                    number: Some(72.0),
                    text: None,
                    boolean: None,
                    unit: Some("workers".to_owned()),
                }),
                reason: "Conservative staffing allowance".to_owned(),
            }],
            supplements: Vec::new(),
        }
    }

    #[test]
    fn accepts_a_bounded_inert_overlay() {
        let document = valid_document();
        document.validate().expect("valid overlay");
        assert_eq!(document.content_hash().expect("hash").len(), 64);
    }

    #[test]
    fn published_example_matches_the_authoritative_validator() {
        let document = PlanningOverlayDocument::parse(include_bytes!(
            "../../examples/planning-overlays/supplemental-planning-material.rooverlay.json"
        ))
        .expect("published overlay example");
        assert_eq!(document.schema_version, 1);
    }

    #[test]
    fn rejects_markup_paths_and_missing_values() {
        let mut document = valid_document();
        document.operations[0].reason = "<script>bad</script>".to_owned();
        assert!(document.validate().is_err());
        document = valid_document();
        document.operations[0].value = None;
        assert!(document.validate().is_err());
    }

    #[test]
    fn rejects_unknown_json_fields() {
        let bytes = br#"{
            "schema_version":1,
            "id":"org.example.overlay",
            "version":"1.0.0",
            "name":"Example",
            "author":"Planner",
            "default_locale":"en-AU",
            "description":"Example overlay",
            "operations":[],
            "supplements":[],
            "javascript":"alert(1)"
        }"#;
        assert!(PlanningOverlayDocument::parse(bytes).is_err());
    }

    #[test]
    fn rejects_uppercase_precondition_hashes() {
        let mut document = valid_document();
        document.operations[0].expected_revision_hash = "A".repeat(64);
        assert!(document.validate().is_err());
    }

    #[test]
    fn semantic_versions_and_reverse_domain_ids_match_the_public_contract() {
        let mut document = valid_document();
        document.version = "1.2.3-beta.1+local.7".to_owned();
        document.validate().expect("full semantic version");

        for invalid in ["1.0.0-", "01.0.0", "1.0.0+", "1.0"] {
            document.version = invalid.to_owned();
            assert!(document.validate().is_err(), "accepted {invalid}");
        }
        document = valid_document();
        document.id = "org.example".to_owned();
        assert!(document.validate().is_err());
    }
}
