//! Strict, inert theme manifests and authoritative contrast validation.

use std::fmt::Write;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ObservatoryError;

pub const THEME_SCHEMA_VERSION: u32 = 1;
pub const MAX_THEME_MANIFEST_BYTES: usize = 32 * 1024;
pub const DEFAULT_THEME_ID: &str = "org.republic-observatory.classic";
pub const DEFAULT_THEME_VERSION: &str = "1.0.0";
const RESERVED_THEME_PREFIX: &str = "org.republic-observatory.";
const TEXT_MINIMUM: f64 = 4.5;
const GRAPHIC_MINIMUM: f64 = 3.0;
const DIVIDER_MINIMUM: f64 = 1.5;

static BUILT_INS: LazyLock<Vec<ThemeManifest>> = LazyLock::new(|| {
    let themes = [
        include_str!("../../themes/republic-observatory-classic.rotheme.json"),
        include_str!("../../themes/republic-observatory-high-contrast.rotheme.json"),
    ]
    .into_iter()
    .map(|document| {
        let manifest: ThemeManifest =
            serde_json::from_str(document).expect("built-in theme JSON must remain valid");
        validate_structure(&manifest, true).expect("built-in theme structure must remain valid");
        manifest
    })
    .collect::<Vec<_>>();
    for theme in &themes {
        validate_structure(theme, true).expect("built-in theme structure must remain valid");
        assert!(
            validate_contrast(theme).valid,
            "built-in theme contrast must remain valid: {}",
            theme.id
        );
    }
    themes
});

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeManifest {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub colours: ThemeColours,
    pub chart_palette: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeColours {
    pub canvas: String,
    pub surface: String,
    pub surface_raised: String,
    pub surface_soft: String,
    pub text: String,
    pub text_muted: String,
    pub line: String,
    pub accent: String,
    pub observed: String,
    pub risk: String,
    pub success: String,
    pub comparison: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeCheckSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ThemeContrastCheck {
    pub id: String,
    pub foreground: String,
    pub background: String,
    pub measured: f64,
    pub minimum: f64,
    pub passes: bool,
    pub severity: ThemeCheckSeverity,
    pub remediation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ThemeValidationReport {
    pub valid: bool,
    pub native_colour_scheme: String,
    pub checks: Vec<ThemeContrastCheck>,
    pub errors: u32,
    pub warnings: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ThemeInspection {
    pub structurally_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ThemeManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ThemeValidationReport>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeSource {
    BuiltIn,
    LocalImport,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AvailableThemeRevision {
    pub manifest: ThemeManifest,
    pub content_hash: String,
    pub source: ThemeSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<i64>,
    pub selected: bool,
    pub report: ThemeValidationReport,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ThemeStatus {
    pub selected_theme_id: String,
    pub selected_version: String,
    pub selected_content_hash: String,
    pub active_theme: ThemeManifest,
    pub active_report: ThemeValidationReport,
    pub themes: Vec<AvailableThemeRevision>,
    pub fallback_applied: bool,
    pub storage_authority: &'static str,
}

pub fn built_in_themes() -> &'static [ThemeManifest] {
    &BUILT_INS
}

pub fn inspect_theme_document(document: &str) -> ThemeInspection {
    match parse_theme_draft(document, false) {
        Ok(manifest) => {
            let canonical =
                canonical_theme_json(&manifest).expect("a validated theme manifest must serialise");
            ThemeInspection {
                structurally_valid: true,
                code: None,
                detail: None,
                content_hash: Some(theme_content_hash(&canonical)),
                report: Some(validate_contrast(&manifest)),
                manifest: Some(manifest),
            }
        }
        Err(error) => ThemeInspection {
            structurally_valid: false,
            code: Some(error.code().to_owned()),
            detail: Some(error.to_string()),
            manifest: None,
            content_hash: None,
            report: None,
        },
    }
}

pub fn parse_community_theme(document: &str) -> Result<ThemeManifest, ObservatoryError> {
    let manifest = parse_theme_draft(document, false)?;
    if !validate_contrast(&manifest).valid {
        return Err(ObservatoryError::ThemeInsufficientContrast);
    }
    Ok(manifest)
}

pub fn parse_stored_theme(document: &str) -> Result<ThemeManifest, ObservatoryError> {
    let manifest = parse_theme_draft(document, false)?;
    if !validate_contrast(&manifest).valid {
        return Err(ObservatoryError::ThemeInsufficientContrast);
    }
    Ok(manifest)
}

pub fn canonical_theme_json(manifest: &ThemeManifest) -> Result<String, ObservatoryError> {
    serde_json::to_string(manifest).map_err(|_| ObservatoryError::InvalidThemeManifest)
}

pub fn theme_content_hash(canonical_json: &str) -> String {
    let digest = Sha256::digest(canonical_json.as_bytes());
    let mut hash = String::with_capacity(64);
    for byte in digest {
        write!(&mut hash, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hash
}

pub fn same_theme_appearance(first: &ThemeManifest, second: &ThemeManifest) -> bool {
    colour_values(&first.colours)
        .iter()
        .zip(colour_values(&second.colours))
        .all(|(first, second)| first.eq_ignore_ascii_case(second))
        && first.chart_palette.len() == second.chart_palette.len()
        && first
            .chart_palette
            .iter()
            .zip(&second.chart_palette)
            .all(|(first, second)| first.eq_ignore_ascii_case(second))
}

fn parse_theme_draft(
    document: &str,
    allow_reserved_identifier: bool,
) -> Result<ThemeManifest, ObservatoryError> {
    if document.len() > MAX_THEME_MANIFEST_BYTES {
        return Err(ObservatoryError::ThemeManifestTooLarge);
    }
    let manifest: ThemeManifest =
        serde_json::from_str(document).map_err(|_| ObservatoryError::InvalidThemeManifest)?;
    validate_structure(&manifest, allow_reserved_identifier)?;
    Ok(manifest)
}

fn validate_structure(
    manifest: &ThemeManifest,
    allow_reserved_identifier: bool,
) -> Result<(), ObservatoryError> {
    if manifest.schema_version != THEME_SCHEMA_VERSION {
        return Err(ObservatoryError::UnsupportedThemeVersion);
    }
    if !valid_reverse_domain_id(&manifest.id)
        || (!allow_reserved_identifier && manifest.id.starts_with(RESERVED_THEME_PREFIX))
    {
        return Err(ObservatoryError::InvalidThemeIdentifier);
    }
    if !valid_semver(&manifest.version)
        || !valid_label(&manifest.name, 80)
        || manifest
            .author
            .as_ref()
            .is_some_and(|value| !valid_label(value, 80))
        || manifest
            .description
            .as_ref()
            .is_some_and(|value| !valid_label(value, 320))
    {
        return Err(ObservatoryError::InvalidThemeMetadata);
    }
    if colour_values(&manifest.colours)
        .iter()
        .any(|value| !valid_hex_colour(value))
        || !(3..=8).contains(&manifest.chart_palette.len())
        || manifest
            .chart_palette
            .iter()
            .any(|value| !valid_hex_colour(value))
    {
        return Err(ObservatoryError::InvalidThemeColour);
    }
    Ok(())
}

pub(crate) fn validate_contrast(manifest: &ThemeManifest) -> ThemeValidationReport {
    let colours = &manifest.colours;
    let surfaces = [
        ("canvas", colours.canvas.as_str()),
        ("surface", colours.surface.as_str()),
        ("surface_raised", colours.surface_raised.as_str()),
        ("surface_soft", colours.surface_soft.as_str()),
    ];
    let foregrounds = [
        ("text", colours.text.as_str()),
        ("text_muted", colours.text_muted.as_str()),
        ("accent", colours.accent.as_str()),
        ("observed", colours.observed.as_str()),
        ("risk", colours.risk.as_str()),
        ("success", colours.success.as_str()),
        ("comparison", colours.comparison.as_str()),
    ];
    let mut checks = Vec::new();
    for (foreground_name, foreground) in foregrounds {
        for (background_name, background) in surfaces {
            push_ratio_check(
                &mut checks,
                format!("{foreground_name}-on-{background_name}"),
                foreground_name,
                background_name,
                contrast_ratio(foreground, background),
                TEXT_MINIMUM,
                ThemeCheckSeverity::Error,
                "increase_foreground_surface_difference",
            );
        }
    }
    for (surface_name, surface) in surfaces {
        push_ratio_check(
            &mut checks,
            format!("line-on-{surface_name}"),
            "line",
            surface_name,
            contrast_ratio(&colours.line, surface),
            GRAPHIC_MINIMUM,
            ThemeCheckSeverity::Error,
            "strengthen_control_boundary",
        );
    }
    push_ratio_check(
        &mut checks,
        "decorative-divider".to_owned(),
        "line_faint",
        "surface",
        contrast_ratio(
            &composite_hex(&colours.line, &colours.surface, 0.45),
            &colours.surface,
        ),
        DIVIDER_MINIMUM,
        ThemeCheckSeverity::Error,
        "strengthen_decorative_divider",
    );
    for (index, colour) in manifest.chart_palette.iter().enumerate() {
        for (surface_name, surface) in [
            ("surface", colours.surface.as_str()),
            ("surface_raised", colours.surface_raised.as_str()),
        ] {
            push_ratio_check(
                &mut checks,
                format!("chart-{}-on-{surface_name}", index + 1),
                &format!("chart_palette[{}]", index + 1),
                surface_name,
                contrast_ratio(colour, surface),
                GRAPHIC_MINIMUM,
                ThemeCheckSeverity::Error,
                "strengthen_chart_surface_difference",
            );
        }
    }
    for (name, foreground) in [
        ("observed_soft", colours.observed.as_str()),
        ("accent_soft", colours.accent.as_str()),
        ("risk_soft", colours.risk.as_str()),
        ("success_soft", colours.success.as_str()),
    ] {
        for (surface_name, surface) in surfaces {
            let derived = composite_hex(foreground, surface, 0.11);
            for (text_name, text) in [
                ("text", colours.text.as_str()),
                ("text_muted", colours.text_muted.as_str()),
            ] {
                push_ratio_check(
                    &mut checks,
                    format!("{text_name}-on-{name}-{surface_name}"),
                    text_name,
                    &format!("{name}+{surface_name}"),
                    contrast_ratio(text, &derived),
                    TEXT_MINIMUM,
                    ThemeCheckSeverity::Error,
                    "adjust_derived_soft_fill",
                );
            }
        }
    }
    for deficiency in [
        Deficiency::Protanopia,
        Deficiency::Deuteranopia,
        Deficiency::Tritanopia,
    ] {
        for index in 0..manifest.chart_palette.len() {
            for other in index + 1..manifest.chart_palette.len() {
                let distance = simulated_distance(
                    &manifest.chart_palette[index],
                    &manifest.chart_palette[other],
                    deficiency,
                );
                push_ratio_check(
                    &mut checks,
                    format!(
                        "chart-distinction-{}-{}-{}",
                        deficiency.name(),
                        index + 1,
                        other + 1
                    ),
                    &format!("chart_palette[{}]", index + 1),
                    &format!("chart_palette[{}]", other + 1),
                    distance,
                    0.12,
                    ThemeCheckSeverity::Warning,
                    "increase_chart_series_distinction",
                );
            }
        }
    }
    let errors = checks
        .iter()
        .filter(|check| !check.passes && check.severity == ThemeCheckSeverity::Error)
        .count() as u32;
    let warnings = checks
        .iter()
        .filter(|check| !check.passes && check.severity == ThemeCheckSeverity::Warning)
        .count() as u32;
    ThemeValidationReport {
        valid: errors == 0,
        native_colour_scheme: if relative_luminance(&manifest.colours.canvas) < 0.45 {
            "dark".to_owned()
        } else {
            "light".to_owned()
        },
        checks,
        errors,
        warnings,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_ratio_check(
    checks: &mut Vec<ThemeContrastCheck>,
    id: String,
    foreground: &str,
    background: &str,
    measured: f64,
    minimum: f64,
    severity: ThemeCheckSeverity,
    remediation: &str,
) {
    checks.push(ThemeContrastCheck {
        id,
        foreground: foreground.to_owned(),
        background: background.to_owned(),
        measured,
        minimum,
        passes: measured >= minimum,
        severity,
        remediation: remediation.to_owned(),
    });
}

fn valid_reverse_domain_id(value: &str) -> bool {
    if !(5..=128).contains(&value.len()) || !value.contains('.') {
        return false;
    }
    value.split('.').all(|segment| {
        !segment.is_empty()
            && segment.len() <= 63
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && segment
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && segment
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
    })
}

fn valid_semver(value: &str) -> bool {
    let core = value.split(['-', '+']).next().unwrap_or_default();
    let parts = core.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

fn valid_label(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= maximum
        && !value.chars().any(char::is_control)
}

fn valid_hex_colour(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn colour_values(colours: &ThemeColours) -> [&str; 12] {
    [
        &colours.canvas,
        &colours.surface,
        &colours.surface_raised,
        &colours.surface_soft,
        &colours.text,
        &colours.text_muted,
        &colours.line,
        &colours.accent,
        &colours.observed,
        &colours.risk,
        &colours.success,
        &colours.comparison,
    ]
}

fn contrast_ratio(first: &str, second: &str) -> f64 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn relative_luminance(colour: &str) -> f64 {
    let [red, green, blue] = rgb(colour);
    0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
}

fn linear(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn rgb(colour: &str) -> [f64; 3] {
    let channel = |offset| {
        u8::from_str_radix(&colour[offset..offset + 2], 16)
            .expect("validated colours contain complete hexadecimal channels") as f64
            / 255.0
    };
    [channel(1), channel(3), channel(5)]
}

fn composite_hex(foreground: &str, background: &str, alpha: f64) -> String {
    let foreground = rgb(foreground);
    let background = rgb(background);
    let mut result = String::from("#");
    for index in 0..3 {
        let channel = (foreground[index] * alpha + background[index] * (1.0 - alpha)) * 255.0;
        write!(&mut result, "{:02X}", channel.round() as u8)
            .expect("writing to a String cannot fail");
    }
    result
}

#[derive(Clone, Copy)]
enum Deficiency {
    Protanopia,
    Deuteranopia,
    Tritanopia,
}

impl Deficiency {
    fn name(self) -> &'static str {
        match self {
            Self::Protanopia => "protanopia",
            Self::Deuteranopia => "deuteranopia",
            Self::Tritanopia => "tritanopia",
        }
    }
}

fn simulate(colour: &str, deficiency: Deficiency) -> [f64; 3] {
    let [r, g, b] = rgb(colour);
    let matrix = match deficiency {
        Deficiency::Protanopia => [
            [0.567, 0.433, 0.0],
            [0.558, 0.442, 0.0],
            [0.0, 0.242, 0.758],
        ],
        Deficiency::Deuteranopia => [[0.625, 0.375, 0.0], [0.7, 0.3, 0.0], [0.0, 0.3, 0.7]],
        Deficiency::Tritanopia => [[0.95, 0.05, 0.0], [0.0, 0.433, 0.567], [0.0, 0.475, 0.525]],
    };
    [
        matrix[0][0] * r + matrix[0][1] * g + matrix[0][2] * b,
        matrix[1][0] * r + matrix[1][1] * g + matrix[1][2] * b,
        matrix[2][0] * r + matrix[2][1] * g + matrix[2][2] * b,
    ]
}

fn simulated_distance(first: &str, second: &str, deficiency: Deficiency) -> f64 {
    let first = simulate(first, deficiency);
    let second = simulate(second, deficiency);
    ((first[0] - second[0]).powi(2)
        + (first[1] - second[1]).powi(2)
        + (first[2] - second[2]).powi(2))
    .sqrt()
        / 3.0_f64.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_themes_pass_the_authoritative_validator() {
        for theme in built_in_themes() {
            assert!(validate_contrast(theme).valid, "{}", theme.id);
        }
    }

    #[test]
    fn low_contrast_drafts_are_inspectable_but_not_installable() {
        let mut theme = built_in_themes()[0].clone();
        theme.id = "org.example.low-contrast".into();
        theme.colours.text = theme.colours.canvas.clone();
        let document = serde_json::to_string(&theme).unwrap();
        let inspection = inspect_theme_document(&document);
        assert!(inspection.structurally_valid);
        assert!(!inspection.report.unwrap().valid);
        assert!(matches!(
            parse_community_theme(&document),
            Err(ObservatoryError::ThemeInsufficientContrast)
        ));
    }

    #[test]
    fn unknown_fields_and_reserved_identifiers_fail_closed() {
        let mut value = serde_json::to_value(&built_in_themes()[0]).unwrap();
        value["id"] = serde_json::json!("org.example.safe");
        value["css"] = serde_json::json!("body { display: none }");
        assert!(!inspect_theme_document(&value.to_string()).structurally_valid);

        let document = serde_json::to_string(&built_in_themes()[0]).unwrap();
        assert!(matches!(
            parse_community_theme(&document),
            Err(ObservatoryError::InvalidThemeIdentifier)
        ));
    }

    #[test]
    fn colour_vision_distinction_is_advisory() {
        let mut theme = built_in_themes()[0].clone();
        theme.id = "org.example.repeated-chart-colour".into();
        theme.chart_palette[1] = theme.chart_palette[0].clone();
        let report = validate_contrast(&theme);
        assert!(report.valid);
        assert!(report.warnings > 0);
    }

    #[test]
    fn success_derived_guidance_surfaces_are_authoritatively_checked() {
        for theme in built_in_themes() {
            let report = validate_contrast(theme);
            for check_id in [
                "text-on-success_soft-surface",
                "text_muted-on-success_soft-surface",
            ] {
                let check = report
                    .checks
                    .iter()
                    .find(|check| check.id == check_id)
                    .expect("success-derived guidance check");
                assert!(check.passes, "{}: {check_id}", theme.id);
            }
        }
    }
}
