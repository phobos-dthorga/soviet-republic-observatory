//! Strict, inert language-pack validation owned by the desktop host.
//!
//! English (Australia) is the canonical source catalogue. Community packs may
//! replace ordinary presentation messages only; operational, evidence, safety,
//! and error namespaces remain host-controlled.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::sync::LazyLock;

use fluent_syntax::{ast, parser};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ObservatoryError;

pub const LANGUAGE_PACK_SCHEMA_VERSION: u32 = 1;
pub const SOURCE_CATALOG_VERSION: u32 = 1;
pub const SOURCE_CATALOG_REVISION: u32 = 38;
pub const SOURCE_LOCALE: &str = "en-AU";
pub const DEFAULT_LANGUAGE_PACK_ID: &str = "observatory-en-au";
pub const MAX_LANGUAGE_PACK_BYTES: usize = 256 * 1024;
pub const MAX_LEGACY_LANGUAGE_PACKS: usize = 32;
pub const MAX_LEGACY_HANDOVER_BYTES: usize = 2 * 1024 * 1024;
const MAX_LANGUAGE_MESSAGES: usize = 2_048;
const MAX_MESSAGE_PATTERN_BYTES: usize = 2_048;
const PROTECTED_MESSAGE_PREFIXES: [&str; 15] = [
    "legal-",
    "privacy-",
    "credential-",
    "save-safety-",
    "extension-permission-",
    "security-",
    "data-protection-",
    "destructive-",
    "error-",
    "evidence-",
    "coverage-",
    "causality-",
    "synthetic-",
    "research-setup-",
    "attention-",
];

static SOURCE_CATALOG: LazyLock<LanguagePackManifest> = LazyLock::new(|| {
    let manifest: LanguagePackManifest =
        serde_json::from_str(include_str!("../../locales/en-AU.json"))
            .expect("the built-in English catalogue must be valid JSON");
    validate_manifest(&manifest, ValidationMode::BuiltIn)
        .expect("the built-in English catalogue must satisfy the language-pack contract");
    manifest
});

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguagePackManifest {
    pub schema_version: u32,
    pub id: String,
    pub locale: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub source_locale: String,
    pub source_catalog_version: u32,
    pub source_catalog_revision: u32,
    pub direction: TextDirection,
    pub messages: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguagePackTrust {
    BuiltIn,
    Community,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AvailableLanguagePack {
    pub manifest: LanguagePackManifest,
    pub trust: LanguagePackTrust,
    pub translated_messages: usize,
    pub eligible_messages: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LanguageStatus {
    pub selected_language_pack_id: String,
    pub active_pack: LanguagePackManifest,
    pub packs: Vec<AvailableLanguagePack>,
    pub storage_authority: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LanguagePackInspection {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<LanguagePackManifest>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyLanguageHandover {
    pub manifests: Vec<String>,
    pub selected_language_pack_id: Option<String>,
}

pub fn source_catalog() -> &'static LanguagePackManifest {
    &SOURCE_CATALOG
}

pub fn eligible_message_count() -> usize {
    source_catalog()
        .messages
        .keys()
        .filter(|key| !protected_message(key))
        .count()
}

pub fn inspect_community_manifest(manifest_json: &str) -> LanguagePackInspection {
    match parse_community_manifest(manifest_json) {
        Ok(manifest) => LanguagePackInspection {
            valid: true,
            code: None,
            detail: None,
            manifest: Some(manifest),
        },
        Err(error) => LanguagePackInspection {
            valid: false,
            code: Some(error.code().to_owned()),
            detail: Some(error.to_string()),
            manifest: None,
        },
    }
}

pub fn parse_community_manifest(
    manifest_json: &str,
) -> Result<LanguagePackManifest, ObservatoryError> {
    if manifest_json.len() > MAX_LANGUAGE_PACK_BYTES {
        return Err(ObservatoryError::LanguageManifestTooLarge);
    }
    let value: serde_json::Value =
        serde_json::from_str(manifest_json).map_err(|_| ObservatoryError::InvalidLanguageJson)?;
    if value
        .as_object()
        .and_then(|object| object.get("author"))
        .is_some_and(|author| !author.is_string())
    {
        return Err(ObservatoryError::InvalidLanguageMetadata);
    }
    let manifest: LanguagePackManifest =
        serde_json::from_value(value).map_err(|_| ObservatoryError::InvalidLanguageManifest)?;
    validate_manifest(&manifest, ValidationMode::Community)?;
    Ok(manifest)
}

pub fn canonical_manifest_json(
    manifest: &LanguagePackManifest,
) -> Result<String, ObservatoryError> {
    serde_json::to_string(manifest).map_err(|_| ObservatoryError::InvalidLanguageManifest)
}

pub fn manifest_content_hash(canonical_json: &str) -> String {
    let digest = Sha256::digest(canonical_json.as_bytes());
    let mut hash = String::with_capacity(64);
    for byte in digest {
        write!(&mut hash, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hash
}

#[derive(Clone, Copy)]
enum ValidationMode {
    BuiltIn,
    Community,
}

fn validate_manifest(
    manifest: &LanguagePackManifest,
    mode: ValidationMode,
) -> Result<(), ObservatoryError> {
    if manifest.schema_version != LANGUAGE_PACK_SCHEMA_VERSION
        || manifest.source_locale != SOURCE_LOCALE
        || manifest.source_catalog_version != SOURCE_CATALOG_VERSION
        || !(1..=SOURCE_CATALOG_REVISION).contains(&manifest.source_catalog_revision)
    {
        return Err(ObservatoryError::UnsupportedLanguageVersion);
    }
    if !valid_pack_id(&manifest.id)
        || (matches!(mode, ValidationMode::Community) && manifest.id.starts_with("observatory-"))
    {
        return Err(ObservatoryError::InvalidLanguageIdentifier);
    }
    if !valid_locale(&manifest.locale)
        || !valid_label(&manifest.name, 80)
        || manifest
            .author
            .as_ref()
            .is_some_and(|author| !valid_label(author, 80))
    {
        return Err(ObservatoryError::InvalidLanguageMetadata);
    }
    if manifest.messages.is_empty() || manifest.messages.len() > MAX_LANGUAGE_MESSAGES {
        return Err(ObservatoryError::InvalidLanguageMessage);
    }

    let source = if matches!(mode, ValidationMode::BuiltIn) {
        manifest
    } else {
        source_catalog()
    };
    for (key, pattern) in &manifest.messages {
        let Some(source_pattern) = source.messages.get(key) else {
            return Err(ObservatoryError::InvalidLanguageMessage);
        };
        if matches!(mode, ValidationMode::Community) && protected_message(key) {
            return Err(ObservatoryError::ProtectedLanguageMessage);
        }
        if !valid_message_key(key)
            || !valid_message_pattern(key, pattern)
            || message_variables(pattern) != message_variables(source_pattern)
        {
            return Err(ObservatoryError::InvalidLanguageMessage);
        }
    }
    Ok(())
}

fn protected_message(key: &str) -> bool {
    PROTECTED_MESSAGE_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
}

fn valid_pack_id(value: &str) -> bool {
    (3..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
}

fn valid_locale(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 || value.contains('_') {
        return false;
    }
    let mut subtags = value.split('-');
    let Some(language) = subtags.next() else {
        return false;
    };
    if (!(2..=3).contains(&language.len()) && !(5..=8).contains(&language.len()))
        || !language.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return false;
    }
    subtags.all(|subtag| {
        (1..=8).contains(&subtag.len()) && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
    })
}

fn valid_label(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.encode_utf16().count() <= maximum
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(character, '<' | '>')
                || matches!(character, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        })
}

fn valid_message_key(value: &str) -> bool {
    (3..=96).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

fn valid_message_pattern(key: &str, pattern: &str) -> bool {
    if pattern.trim().is_empty()
        || pattern.len() > MAX_MESSAGE_PATTERN_BYTES
        || pattern.contains('<')
        || pattern.replace("->", "").contains('>')
        || pattern.chars().any(|character| {
            (character.is_control() && !matches!(character, '\n' | '\t'))
                || matches!(character, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        })
    {
        return false;
    }
    let mut lines = pattern.lines();
    let Some(first) = lines.next() else {
        return false;
    };
    let mut resource = format!("{key} = {first}");
    for line in lines {
        resource.push_str("\n    ");
        resource.push_str(line);
    }
    let Ok(parsed) = parser::parse(resource.as_str()) else {
        return false;
    };
    if parsed.body.len() != 1 {
        return false;
    }
    let ast::Entry::Message(message) = &parsed.body[0] else {
        return false;
    };
    message.attributes.is_empty()
        && message
            .value
            .as_ref()
            .is_some_and(|value| !pattern_has_external_reference(value))
}

fn pattern_has_external_reference(pattern: &ast::Pattern<&str>) -> bool {
    pattern.elements.iter().any(|element| match element {
        ast::PatternElement::TextElement { .. } => false,
        ast::PatternElement::Placeable { expression } => {
            expression_has_external_reference(expression)
        }
    })
}

fn expression_has_external_reference(expression: &ast::Expression<&str>) -> bool {
    match expression {
        ast::Expression::Inline(inline) => inline_has_external_reference(inline),
        ast::Expression::Select { selector, variants } => {
            inline_has_external_reference(selector)
                || variants
                    .iter()
                    .any(|variant| pattern_has_external_reference(&variant.value))
        }
    }
}

fn inline_has_external_reference(expression: &ast::InlineExpression<&str>) -> bool {
    match expression {
        ast::InlineExpression::MessageReference { .. }
        | ast::InlineExpression::TermReference { .. } => true,
        ast::InlineExpression::FunctionReference { id, arguments } => {
            !matches!(id.name, "NUMBER" | "DATETIME")
                || arguments
                    .positional
                    .iter()
                    .any(inline_has_external_reference)
                || arguments
                    .named
                    .iter()
                    .any(|argument| inline_has_external_reference(&argument.value))
        }
        ast::InlineExpression::Placeable { expression } => {
            expression_has_external_reference(expression)
        }
        ast::InlineExpression::StringLiteral { .. }
        | ast::InlineExpression::NumberLiteral { .. }
        | ast::InlineExpression::VariableReference { .. } => false,
    }
}

fn message_variables(pattern: &str) -> BTreeSet<String> {
    let bytes = pattern.as_bytes();
    let mut variables = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        let start = index + 1;
        let mut end = start;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'-' | b'_'))
        {
            end += 1;
        }
        if end > start && bytes[start].is_ascii_alphabetic() {
            variables.insert(pattern[start..end].to_owned());
        }
        index = end.max(index + 1);
    }
    variables
}

#[cfg(test)]
mod tests {
    use super::*;

    fn community_manifest() -> LanguagePackManifest {
        LanguagePackManifest {
            schema_version: LANGUAGE_PACK_SCHEMA_VERSION,
            id: "community-test".to_owned(),
            locale: "fr-FR".to_owned(),
            name: "Français".to_owned(),
            author: Some("Test author".to_owned()),
            source_locale: SOURCE_LOCALE.to_owned(),
            source_catalog_version: SOURCE_CATALOG_VERSION,
            source_catalog_revision: SOURCE_CATALOG_REVISION,
            direction: TextDirection::LeftToRight,
            messages: BTreeMap::from([("action-close".to_owned(), "Fermer".to_owned())]),
        }
    }

    #[test]
    fn built_in_catalogue_satisfies_the_native_contract() {
        assert_eq!(source_catalog().id, DEFAULT_LANGUAGE_PACK_ID);
        assert!(source_catalog().messages.len() > 900);
    }

    #[test]
    fn community_manifest_is_canonical_and_content_addressed() {
        let json = canonical_manifest_json(&community_manifest()).expect("canonical manifest");
        let parsed = parse_community_manifest(&json).expect("community manifest");
        assert_eq!(parsed.id, "community-test");
        assert_eq!(manifest_content_hash(&json).len(), 64);
    }

    #[test]
    fn rejects_unknown_fields_protected_messages_and_variable_drift() {
        let mut value = serde_json::to_value(community_manifest()).expect("manifest value");
        value["script"] = serde_json::json!("alert(1)");
        assert_eq!(
            parse_community_manifest(&value.to_string())
                .expect_err("unknown field")
                .code(),
            "invalid_manifest"
        );

        let mut protected = community_manifest();
        protected.messages =
            BTreeMap::from([("security-language-active".to_owned(), "Actif".to_owned())]);
        assert_eq!(
            parse_community_manifest(&canonical_manifest_json(&protected).expect("json"))
                .expect_err("protected message")
                .code(),
            "protected_message"
        );

        let mut variables = community_manifest();
        variables.messages =
            BTreeMap::from([("language-by-author".to_owned(), "par { $name }".to_owned())]);
        assert_eq!(
            parse_community_manifest(&canonical_manifest_json(&variables).expect("json"))
                .expect_err("variable drift")
                .code(),
            "invalid_message"
        );
    }

    #[test]
    fn rejects_markup_bidi_and_future_catalogues_but_accepts_arrow_text() {
        for pattern in ["<b>Fermer</b>", "Fermer\u{202E}", "Fermer > maintenant"] {
            let mut manifest = community_manifest();
            manifest
                .messages
                .insert("action-close".to_owned(), pattern.to_owned());
            assert_eq!(
                parse_community_manifest(&canonical_manifest_json(&manifest).expect("json"))
                    .expect_err("unsafe pattern")
                    .code(),
                "invalid_message"
            );
        }
        let mut future = community_manifest();
        future.source_catalog_revision = SOURCE_CATALOG_REVISION + 1;
        assert_eq!(
            parse_community_manifest(&canonical_manifest_json(&future).expect("json"))
                .expect_err("future revision")
                .code(),
            "unsupported_version"
        );
        let mut arrow = community_manifest();
        arrow
            .messages
            .insert("action-close".to_owned(), "Avant -> après".to_owned());
        parse_community_manifest(&canonical_manifest_json(&arrow).expect("json"))
            .expect("arrow remains allowed");

        let mut reference = community_manifest();
        reference
            .messages
            .insert("action-close".to_owned(), "{ nav-broadcast }".to_owned());
        assert_eq!(
            parse_community_manifest(&canonical_manifest_json(&reference).expect("json"))
                .expect_err("external message reference")
                .code(),
            "invalid_message"
        );

        let mut callback = community_manifest();
        callback.messages.insert(
            "language-by-author".to_owned(),
            "{ CALLBACK($author) }".to_owned(),
        );
        assert_eq!(
            parse_community_manifest(&canonical_manifest_json(&callback).expect("json"))
                .expect_err("unknown Fluent callback")
                .code(),
            "invalid_message"
        );
    }
}
