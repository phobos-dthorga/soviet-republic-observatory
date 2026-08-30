use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::compatibility_profile::{
    CatalogueScopeUpdatePolicy, DefinitionOperation, ResolvedCompatibilityProfile,
};
use crate::error::ObservatoryError;
use crate::model::{
    CompatibilityCatalogueScopeState, CompatibilityCatalogueScopeStatus, CompatibilityProvenance,
};

pub const DEFINITION_PARSER_VERSION: &str = "wrsr-definition-directives.v3";
const MAX_FILES: usize = 100_000;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 4_096;
const PROGRESS_REPORT_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Debug)]
pub struct CatalogueReuseEntry {
    pub display_name: String,
    pub coverage: String,
    pub resource_targets: Vec<String>,
    pub has_production_route: bool,
}

#[derive(Clone, Debug)]
pub struct CatalogueGeneration {
    pub generation_id: String,
    pub game_build_id: Option<String>,
    pub created_at_ms: i64,
    pub compatibility: CompatibilityProvenance,
    pub compatibility_scopes: Vec<CompatibilityCatalogueScopeStatus>,
    pub sources: Vec<CatalogueSource>,
    pub files: Vec<CatalogueFile>,
    pub entities: Vec<ParsedDefinition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogueDiscoveryPhase {
    Discovering,
    Scanning,
    Complete,
}

#[derive(Clone, Debug)]
pub struct CatalogueDiscoveryProgress {
    pub phase: CatalogueDiscoveryPhase,
    pub current_source: Option<String>,
    pub current_file: Option<String>,
    pub current_file_index: Option<u32>,
    pub sources_discovered: u32,
    pub sources_total: u32,
    pub files_discovered: u32,
    pub files_processed: u32,
    pub files_reused: u32,
    pub files_parsed: u32,
    pub entities_prepared: u32,
}

#[derive(Clone, Debug)]
pub struct CatalogueSource {
    pub source_id: String,
    pub source_kind: String,
    pub package_name: String,
    pub package_version: Option<String>,
    pub content_hash: String,
    pub file_count: u32,
}

#[derive(Clone, Debug)]
pub struct CatalogueFile {
    pub source_id: String,
    pub logical_path: String,
    pub content_hash: String,
    pub byte_size: u64,
    pub warning_count: u32,
}

#[derive(Clone, Debug)]
pub struct ParsedDefinition {
    pub entity_id: String,
    pub revision_hash: String,
    pub entity_kind: String,
    pub source_id: String,
    pub source_object_id: String,
    pub display_name: String,
    pub coverage: String,
    pub properties: Vec<ParsedProperty>,
    pub relations: Vec<ParsedRelation>,
    pub unknown_directives: Vec<(String, u32)>,
}

#[derive(Clone, Debug)]
pub struct ParsedProperty {
    pub field_id: String,
    pub occurrence: u32,
    pub value_kind: String,
    pub value_number: Option<f64>,
    pub value_text: Option<String>,
    pub unit: Option<String>,
    pub source_directive: String,
    pub source_line: u32,
    pub raw_arguments: String,
    pub resolution: String,
    pub mapping_id: String,
    pub catalogue_scope_id: Option<String>,
    pub mapping_classification: String,
}

#[derive(Clone, Debug)]
pub struct ParsedRelation {
    pub relation_kind: String,
    pub occurrence: u32,
    pub target_id: String,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub phase_id: Option<String>,
    pub source_directive: String,
    pub source_line: u32,
    pub raw_arguments: String,
    pub resolution: String,
    pub mapping_id: String,
    pub catalogue_scope_id: Option<String>,
    pub mapping_classification: String,
}

#[derive(Clone, Debug)]
struct SourceRoot {
    source_id: String,
    source_kind: String,
    package_name: String,
    package_version: Option<String>,
    root: PathBuf,
    mode: ScanMode,
}

#[derive(Clone, Copy, Debug)]
enum ScanMode {
    BaseBuildings,
    Vehicles,
    Package,
}

#[cfg(test)]
fn discover_catalogue(
    media_directory: &Path,
    created_at_ms: i64,
) -> Result<CatalogueGeneration, ObservatoryError> {
    discover_catalogue_with_reuse(media_directory, None, created_at_ms, &HashMap::new())
}

#[cfg(test)]
pub fn discover_catalogue_with_reuse(
    media_directory: &Path,
    optional_workshop_directory: Option<&Path>,
    created_at_ms: i64,
    reuse: &HashMap<String, CatalogueReuseEntry>,
) -> Result<CatalogueGeneration, ObservatoryError> {
    let profile = ResolvedCompatibilityProfile::reviewed_builtin()?;
    discover_catalogue_with_reuse_and_progress(
        media_directory,
        optional_workshop_directory,
        created_at_ms,
        reuse,
        &profile,
        |_| {},
    )
}

pub fn discover_catalogue_with_reuse_and_progress(
    media_directory: &Path,
    optional_workshop_directory: Option<&Path>,
    created_at_ms: i64,
    reuse: &HashMap<String, CatalogueReuseEntry>,
    profile: &ResolvedCompatibilityProfile,
    mut report: impl FnMut(CatalogueDiscoveryProgress),
) -> Result<CatalogueGeneration, ObservatoryError> {
    let roots = discover_source_roots(media_directory, optional_workshop_directory)?;
    let sources_total = roots.len().min(u32::MAX as usize) as u32;
    let mut files = Vec::new();
    let mut entities = Vec::new();
    let mut source_entries = BTreeMap::<String, Vec<(String, String)>>::new();
    let mut total_bytes = 0_u64;
    let mut source_candidates = Vec::with_capacity(roots.len());
    let mut files_discovered = 0_u32;

    report(CatalogueDiscoveryProgress {
        phase: CatalogueDiscoveryPhase::Discovering,
        current_source: None,
        current_file: None,
        current_file_index: None,
        sources_discovered: 0,
        sources_total,
        files_discovered: 0,
        files_processed: 0,
        files_reused: 0,
        files_parsed: 0,
        entities_prepared: 0,
    });

    let mut last_discovery_report = Instant::now() - PROGRESS_REPORT_INTERVAL;
    for (source_index, source) in roots.iter().enumerate() {
        let source_name = bounded_source_name(&source.package_name);
        report(CatalogueDiscoveryProgress {
            phase: CatalogueDiscoveryPhase::Discovering,
            current_source: Some(source_name.clone()),
            current_file: None,
            current_file_index: None,
            sources_discovered: source_index.min(u32::MAX as usize) as u32,
            sources_total,
            files_discovered,
            files_processed: 0,
            files_reused: 0,
            files_parsed: 0,
            entities_prepared: 0,
        });
        let previously_discovered = files_discovered;
        let candidates = candidate_files(source, |path, source_file_count| {
            if last_discovery_report.elapsed() < PROGRESS_REPORT_INTERVAL {
                return;
            }
            last_discovery_report = Instant::now();
            report(CatalogueDiscoveryProgress {
                phase: CatalogueDiscoveryPhase::Discovering,
                current_source: Some(source_name.clone()),
                current_file: Some(bounded_file_label(&source.root, path)),
                current_file_index: None,
                sources_discovered: source_index.min(u32::MAX as usize) as u32,
                sources_total,
                files_discovered: previously_discovered.saturating_add(source_file_count),
                files_processed: 0,
                files_reused: 0,
                files_parsed: 0,
                entities_prepared: 0,
            });
        })?;
        files_discovered =
            files_discovered.saturating_add(candidates.len().min(u32::MAX as usize) as u32);
        if files_discovered as usize > MAX_FILES {
            return Err(ObservatoryError::InvalidCatalogueRequest);
        }
        report(CatalogueDiscoveryProgress {
            phase: CatalogueDiscoveryPhase::Discovering,
            current_source: Some(source_name),
            current_file: candidates
                .last()
                .map(|path| bounded_file_label(&source.root, path)),
            current_file_index: None,
            sources_discovered: (source_index + 1).min(u32::MAX as usize) as u32,
            sources_total,
            files_discovered,
            files_processed: 0,
            files_reused: 0,
            files_parsed: 0,
            entities_prepared: 0,
        });
        source_candidates.push((source, candidates));
    }

    let mut files_processed = 0_u32;
    let mut files_reused = 0_u32;
    let mut files_parsed = 0_u32;
    let mut last_scan_report = Instant::now() - PROGRESS_REPORT_INTERVAL;
    for (source, candidates) in source_candidates {
        for candidate in candidates {
            if files_processed as usize >= MAX_FILES {
                return Err(ObservatoryError::InvalidCatalogueRequest);
            }
            let file_label = bounded_file_label(&source.root, &candidate);
            let current_file_index = files_processed.saturating_add(1);
            if last_scan_report.elapsed() >= PROGRESS_REPORT_INTERVAL {
                last_scan_report = Instant::now();
                report(CatalogueDiscoveryProgress {
                    phase: CatalogueDiscoveryPhase::Scanning,
                    current_source: Some(bounded_source_name(&source.package_name)),
                    current_file: Some(file_label.clone()),
                    current_file_index: Some(current_file_index),
                    sources_discovered: sources_total,
                    sources_total,
                    files_discovered,
                    files_processed,
                    files_reused,
                    files_parsed,
                    entities_prepared: entities.len().min(u32::MAX as usize) as u32,
                });
            }
            let metadata = fs::symlink_metadata(&candidate)
                .map_err(|_| ObservatoryError::InvalidGameDirectory)?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_FILE_BYTES
            {
                continue;
            }
            total_bytes = total_bytes.saturating_add(metadata.len());
            if total_bytes > MAX_TOTAL_BYTES {
                return Err(ObservatoryError::InvalidCatalogueRequest);
            }
            let bytes = fs::read(&candidate).map_err(|_| ObservatoryError::InvalidGameDirectory)?;
            let content_hash = hex_hash(&bytes);
            let logical_path = logical_path(&source.root, &candidate)?;
            let (entity_kind, source_object_id, entity_id, revision_hash) = definition_identity(
                source,
                &logical_path,
                &content_hash,
                profile.resolved_hash(),
            );
            let (definition, warning_count) = if let Some(cached) = reuse.get(&revision_hash) {
                files_reused = files_reused.saturating_add(1);
                (
                    ParsedDefinition {
                        entity_id,
                        revision_hash,
                        entity_kind,
                        source_id: source.source_id.clone(),
                        source_object_id,
                        display_name: cached.display_name.clone(),
                        coverage: cached.coverage.clone(),
                        properties: Vec::new(),
                        relations: Vec::new(),
                        unknown_directives: Vec::new(),
                    },
                    0,
                )
            } else {
                files_parsed = files_parsed.saturating_add(1);
                parse_definition(source, &logical_path, &content_hash, &bytes, profile)?
            };
            source_entries
                .entry(source.source_id.clone())
                .or_default()
                .push((logical_path.clone(), content_hash.clone()));
            files.push(CatalogueFile {
                source_id: source.source_id.clone(),
                logical_path,
                content_hash,
                byte_size: metadata.len(),
                warning_count,
            });
            entities.push(definition);
            files_processed = files_processed.saturating_add(1);
            if files_processed == files_discovered
                || last_scan_report.elapsed() >= PROGRESS_REPORT_INTERVAL
            {
                last_scan_report = Instant::now();
                report(CatalogueDiscoveryProgress {
                    phase: CatalogueDiscoveryPhase::Scanning,
                    current_source: Some(bounded_source_name(&source.package_name)),
                    current_file: Some(file_label),
                    current_file_index: Some(files_processed),
                    sources_discovered: sources_total,
                    sources_total,
                    files_discovered,
                    files_processed,
                    files_reused,
                    files_parsed,
                    entities_prepared: entities.len().min(u32::MAX as usize) as u32,
                });
            }
        }
    }

    let cached_resource_targets = reuse
        .values()
        .flat_map(|entry| entry.resource_targets.iter().cloned())
        .collect::<BTreeSet<_>>();
    add_recipe_entities(&mut entities, reuse);
    add_resource_entities(&mut entities, cached_resource_targets);
    files.sort_by(|left, right| {
        (&left.source_id, &left.logical_path).cmp(&(&right.source_id, &right.logical_path))
    });
    entities.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));

    let mut sources = roots
        .iter()
        .filter_map(|source| {
            let mut entries = source_entries.remove(&source.source_id)?;
            entries.sort();
            let mut hasher = Sha256::new();
            for (path, hash) in &entries {
                hasher.update(path.as_bytes());
                hasher.update([0]);
                hasher.update(hash.as_bytes());
                hasher.update([0]);
            }
            Some(CatalogueSource {
                source_id: source.source_id.clone(),
                source_kind: source.source_kind.clone(),
                package_name: source.package_name.clone(),
                package_version: source.package_version.clone(),
                content_hash: hex_bytes(&hasher.finalize()),
                file_count: entries.len().min(u32::MAX as usize) as u32,
            })
        })
        .collect::<Vec<_>>();
    let resource_entries = entities
        .iter()
        .filter(|entity| entity.source_id == "game.resources")
        .map(|entity| entity.revision_hash.as_str())
        .collect::<Vec<_>>();
    if !resource_entries.is_empty() {
        let mut hasher = Sha256::new();
        for revision_hash in &resource_entries {
            hasher.update(revision_hash.as_bytes());
        }
        sources.push(CatalogueSource {
            source_id: "game.resources".to_owned(),
            source_kind: "derived".to_owned(),
            package_name: "Observed resource tokens".to_owned(),
            package_version: Some(DEFINITION_PARSER_VERSION.to_owned()),
            content_hash: hex_bytes(&hasher.finalize()),
            file_count: 0,
        });
    }
    sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    let compatibility_scopes = evaluate_catalogue_scopes(profile, &sources);

    let game_build_id = discover_game_build(media_directory);
    let mut generation_hasher = Sha256::new();
    generation_hasher.update(DEFINITION_PARSER_VERSION.as_bytes());
    generation_hasher.update(profile.resolved_hash().as_bytes());
    if let Some(build) = &game_build_id {
        generation_hasher.update(build.as_bytes());
    }
    for file in &files {
        generation_hasher.update(file.source_id.as_bytes());
        generation_hasher.update([0]);
        generation_hasher.update(file.logical_path.as_bytes());
        generation_hasher.update([0]);
        generation_hasher.update(file.content_hash.as_bytes());
    }

    report(CatalogueDiscoveryProgress {
        phase: CatalogueDiscoveryPhase::Complete,
        current_source: None,
        current_file: None,
        current_file_index: None,
        sources_discovered: sources_total,
        sources_total,
        files_discovered,
        files_processed,
        files_reused,
        files_parsed,
        entities_prepared: entities.len().min(u32::MAX as usize) as u32,
    });

    Ok(CatalogueGeneration {
        generation_id: hex_bytes(&generation_hasher.finalize()),
        game_build_id,
        created_at_ms,
        compatibility: profile.provenance(),
        compatibility_scopes,
        sources,
        files,
        entities,
    })
}

fn bounded_source_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(120)
        .collect()
}

fn bounded_file_label(root: &Path, path: &Path) -> String {
    logical_path(root, path)
        .ok()
        .or_else(|| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_default()
        .chars()
        .filter(|character| !character.is_control())
        .take(180)
        .collect()
}

pub fn catalogue_watch_roots(
    media_directory: &Path,
    optional_workshop_directory: Option<&Path>,
) -> Vec<PathBuf> {
    let Ok(canonical) = media_directory.canonicalize() else {
        return Vec::new();
    };
    let mut paths = vec![canonical.clone()];
    if let Some(workshop) = external_workshop_root(&canonical) {
        paths.push(workshop);
    }
    if let Some(workshop) = optional_workshop_directory
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| path.is_dir())
    {
        paths.push(workshop);
    }
    paths.sort();
    paths.dedup();
    paths
}

fn discover_source_roots(
    media_directory: &Path,
    optional_workshop_directory: Option<&Path>,
) -> Result<Vec<SourceRoot>, ObservatoryError> {
    let canonical = media_directory
        .canonicalize()
        .map_err(|_| ObservatoryError::InvalidGameDirectory)?;
    let mut roots = Vec::new();
    add_root(
        &mut roots,
        "base.buildings",
        "base",
        "Base-game buildings",
        canonical.join("buildings_types"),
        ScanMode::BaseBuildings,
        None,
    );
    add_root(
        &mut roots,
        "base.vehicles",
        "base",
        "Base-game vehicles",
        canonical.join("vehicles"),
        ScanMode::Vehicles,
        None,
    );

    for entry in fs::read_dir(&canonical).map_err(|_| ObservatoryError::InvalidGameDirectory)? {
        let Ok(entry) = entry else { continue };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if entry.file_type().is_ok_and(|kind| kind.is_dir())
            && (lower.starts_with("dlc") || lower.starts_with("elc"))
        {
            add_root(
                &mut roots,
                &format!("dlc.{lower}"),
                "dlc",
                &format!("Installed content {name}"),
                entry.path(),
                ScanMode::Package,
                None,
            );
        }
    }

    add_package_children(
        &mut roots,
        &canonical.join("workshop_subscribed"),
        "workshop",
    );
    add_package_children(&mut roots, &canonical.join("workshop_wip"), "wip");
    if let Some(workshop) = external_workshop_root(&canonical) {
        add_package_children(&mut roots, &workshop, "workshop");
    }
    if let Some(workshop) = optional_workshop_directory
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| path.is_dir())
    {
        add_package_children(&mut roots, &workshop, "workshop");
    }
    roots.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    roots.dedup_by(|left, right| left.source_id == right.source_id);
    Ok(roots)
}

fn add_package_children(roots: &mut Vec<SourceRoot>, parent: &Path, kind: &str) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        if !entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
            continue;
        }
        let Some(folder_id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let (item_id, package_name) = workshop_metadata(&entry.path())
            .unwrap_or_else(|| (folder_id.clone(), format!("Local item {folder_id}")));
        let source_id = format!("{kind}.{item_id}");
        if roots.iter().any(|source| source.source_id == source_id) {
            continue;
        }
        add_root(
            roots,
            &source_id,
            kind,
            &package_name,
            entry.path(),
            ScanMode::Package,
            None,
        );
    }
}

fn add_root(
    roots: &mut Vec<SourceRoot>,
    source_id: &str,
    source_kind: &str,
    package_name: &str,
    root: PathBuf,
    mode: ScanMode,
    package_version: Option<String>,
) {
    if root.is_dir() {
        roots.push(SourceRoot {
            source_id: source_id.to_owned(),
            source_kind: source_kind.to_owned(),
            package_name: package_name.to_owned(),
            package_version,
            root,
            mode,
        });
    }
}

fn external_workshop_root(media_directory: &Path) -> Option<PathBuf> {
    let game_root = media_directory.parent()?;
    let common = game_root.parent()?;
    let steamapps = common.parent()?;
    let workshop = steamapps.join("workshop").join("content").join("784150");
    workshop.is_dir().then_some(workshop)
}

fn candidate_files(
    source: &SourceRoot,
    mut observe: impl FnMut(&Path, u32),
) -> Result<Vec<PathBuf>, ObservatoryError> {
    let mut files = Vec::new();
    walk_directory(&source.root, &mut |path| {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let matches = match source.mode {
            ScanMode::BaseBuildings => path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ini")),
            ScanMode::Vehicles | ScanMode::Package => {
                file_name.eq_ignore_ascii_case("script.ini")
                    || file_name.eq_ignore_ascii_case("building.ini")
            }
        };
        if matches {
            files.push(path.to_path_buf());
        }
        observe(path, files.len().min(u32::MAX as usize) as u32);
    })?;
    files.sort();
    Ok(files)
}

fn walk_directory(
    directory: &Path,
    accept: &mut impl FnMut(&Path),
) -> Result<(), ObservatoryError> {
    for entry in fs::read_dir(directory).map_err(|_| ObservatoryError::InvalidGameDirectory)? {
        let Ok(entry) = entry else { continue };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            walk_directory(&entry.path(), accept)?;
        } else if file_type.is_file() {
            accept(&entry.path());
        }
    }
    Ok(())
}

fn parse_definition(
    source: &SourceRoot,
    logical_path: &str,
    content_hash: &str,
    bytes: &[u8],
    profile: &ResolvedCompatibilityProfile,
) -> Result<(ParsedDefinition, u32), ObservatoryError> {
    let text = String::from_utf8_lossy(bytes);
    let lossy = matches!(&text, std::borrow::Cow::Owned(_));
    let (entity_kind, source_object_id, entity_id, revision_hash) =
        definition_identity(source, logical_path, content_hash, profile.resolved_hash());
    let mut display_name = fallback_display_name(&source_object_id);
    let mut properties = Vec::new();
    let mut relations = Vec::new();
    let mut property_counts = HashMap::<String, u32>::new();
    let mut relation_counts = HashMap::<String, u32>::new();
    let mut unknown = BTreeMap::<String, u32>::new();
    let mut warning_count = u32::from(lossy);
    let mut current_phase: Option<String> = None;
    let mut phase_ordinal = 0_u32;

    for (line_index, raw_line) in text.lines().enumerate() {
        if raw_line.len() > MAX_LINE_BYTES {
            warning_count = warning_count.saturating_add(1);
            continue;
        }
        let line = raw_line.trim();
        if !line.starts_with('$') || line.starts_with("$END") {
            continue;
        }
        let mut split = line.splitn(2, char::is_whitespace);
        let directive = split.next().unwrap_or_default().to_ascii_uppercase();
        let raw_arguments = truncate(split.next().unwrap_or_default().trim(), 1_024);
        let arguments = split_arguments(&raw_arguments);
        let source_line = (line_index + 1).min(u32::MAX as usize) as u32;

        let mapping = profile.definition_mapping(&source.source_id, &directive)?;
        let operation = mapping.as_ref().map(|mapping| mapping.operation);
        if operation == Some(DefinitionOperation::BuildingConstructionPhase) {
            phase_ordinal = phase_ordinal.saturating_add(1);
            current_phase = arguments
                .first()
                .map(|value| format!("{}:{phase_ordinal}", normalized_token(value)));
            push_text_property(
                &mut properties,
                &mut property_counts,
                "building.construction.phase",
                current_phase.as_deref().unwrap_or("unknown"),
                None,
                &directive,
                source_line,
                &raw_arguments,
                "source_directive",
            );
            if let (Some(property), Some(mapping)) = (properties.last_mut(), mapping) {
                property.mapping_id = mapping.id;
                property.catalogue_scope_id = mapping.catalogue_scope;
                property.mapping_classification = mapping.mapping_classification;
            }
            continue;
        }

        let property_start = properties.len();
        let relation_start = relations.len();
        let recognized = match operation {
            Some(DefinitionOperation::DefinitionDisplayName) => {
                if let Some(value) = arguments.first() {
                    display_name = truncate(value, 120);
                    push_text_property(
                        &mut properties,
                        &mut property_counts,
                        "definition.display_name",
                        value,
                        None,
                        &directive,
                        source_line,
                        &raw_arguments,
                        "verified",
                    );
                }
                true
            }
            Some(DefinitionOperation::DefinitionNameToken) => {
                if let Some(value) = arguments.first() {
                    display_name = format!("Name token {}", truncate(value, 32));
                    push_text_property(
                        &mut properties,
                        &mut property_counts,
                        "definition.name_token",
                        value,
                        None,
                        &directive,
                        source_line,
                        &raw_arguments,
                        "source_token",
                    );
                }
                true
            }
            Some(DefinitionOperation::BuildingStyle) => text_property(
                &mut properties,
                &mut property_counts,
                "building.style",
                &arguments,
                None,
                &directive,
                source_line,
                &raw_arguments,
            ),
            Some(DefinitionOperation::BuildingWorkersRequired) => number_property(
                &mut properties,
                &mut property_counts,
                "building.workers.required",
                &arguments,
                Some("workers"),
                &directive,
                source_line,
                &raw_arguments,
                "verified",
            ),
            Some(DefinitionOperation::BuildingProfessorsRequired) => number_property(
                &mut properties,
                &mut property_counts,
                "building.professors.required",
                &arguments,
                Some("professors"),
                &directive,
                source_line,
                &raw_arguments,
                "verified",
            ),
            Some(DefinitionOperation::BuildingServiceCapacity) => number_property(
                &mut properties,
                &mut property_counts,
                "building.citizens.service_capacity_per_worker",
                &arguments,
                Some("citizens"),
                &directive,
                source_line,
                &raw_arguments,
                "source_coefficient",
            ),
            Some(DefinitionOperation::BuildingQualityOfLiving) => number_property(
                &mut properties,
                &mut property_counts,
                "building.quality_of_living",
                &arguments,
                Some("ratio"),
                &directive,
                source_line,
                &raw_arguments,
                "verified",
            ),
            Some(DefinitionOperation::BuildingVehicleCapacity) => number_property(
                &mut properties,
                &mut property_counts,
                "building.vehicles.capacity",
                &arguments,
                Some("vehicles"),
                &directive,
                source_line,
                &raw_arguments,
                "verified",
            ),
            Some(DefinitionOperation::VehicleType) if entity_kind == "vehicle" => text_property(
                &mut properties,
                &mut property_counts,
                "vehicle.type",
                &arguments,
                None,
                &directive,
                source_line,
                &raw_arguments,
            ),
            Some(DefinitionOperation::VehicleFamily) => text_property(
                &mut properties,
                &mut property_counts,
                "vehicle.family",
                &arguments,
                None,
                &directive,
                source_line,
                &raw_arguments,
            ),
            Some(DefinitionOperation::VehicleCountry) => text_property(
                &mut properties,
                &mut property_counts,
                "vehicle.country_token",
                &arguments,
                None,
                &directive,
                source_line,
                &raw_arguments,
            ),
            Some(DefinitionOperation::DefinitionAvailability) => multi_number_properties(
                &mut properties,
                &mut property_counts,
                [
                    "definition.available.from_year",
                    "definition.available.to_year",
                ],
                &arguments,
                Some("year"),
                &directive,
                source_line,
                &raw_arguments,
            ),
            Some(DefinitionOperation::VehicleLifespan) => number_property(
                &mut properties,
                &mut property_counts,
                "vehicle.lifespan",
                &arguments,
                Some("years"),
                &directive,
                source_line,
                &raw_arguments,
                "verified",
            ),
            Some(DefinitionOperation::VehicleCostRub) => number_property(
                &mut properties,
                &mut property_counts,
                "vehicle.purchase_cost.rubles",
                &arguments,
                Some("RUB"),
                &directive,
                source_line,
                &raw_arguments,
                "game_definition",
            ),
            Some(DefinitionOperation::VehicleCostUsd) => number_property(
                &mut properties,
                &mut property_counts,
                "vehicle.purchase_cost.dollars",
                &arguments,
                Some("USD"),
                &directive,
                source_line,
                &raw_arguments,
                "game_definition",
            ),
            Some(DefinitionOperation::VehicleSpeed) => number_property(
                &mut properties,
                &mut property_counts,
                "vehicle.speed.maximum",
                &arguments,
                Some("km/h"),
                &directive,
                source_line,
                &raw_arguments,
                "game_definition",
            ),
            Some(DefinitionOperation::VehiclePower) => number_property(
                &mut properties,
                &mut property_counts,
                "vehicle.power",
                &arguments,
                Some("kW"),
                &directive,
                source_line,
                &raw_arguments,
                "game_definition",
            ),
            Some(DefinitionOperation::VehicleEmptyWeight) => number_property(
                &mut properties,
                &mut property_counts,
                "vehicle.empty_weight",
                &arguments,
                Some("t"),
                &directive,
                source_line,
                &raw_arguments,
                "game_definition",
            ),
            Some(DefinitionOperation::VehicleConsumption) => number_property(
                &mut properties,
                &mut property_counts,
                "vehicle.consumption.coefficient",
                &arguments,
                None,
                &directive,
                source_line,
                &raw_arguments,
                "source_coefficient",
            ),
            Some(DefinitionOperation::VehicleResourceCapacity) => number_property(
                &mut properties,
                &mut property_counts,
                "vehicle.resource_capacity",
                &arguments,
                None,
                &directive,
                source_line,
                &raw_arguments,
                "game_definition",
            ),
            Some(DefinitionOperation::VehicleTransportType) => text_property(
                &mut properties,
                &mut property_counts,
                "vehicle.transport_type",
                &arguments,
                None,
                &directive,
                source_line,
                &raw_arguments,
            ),
            Some(DefinitionOperation::VehicleResourceAllowed) => text_property(
                &mut properties,
                &mut property_counts,
                "vehicle.resource_allowed",
                &arguments,
                None,
                &directive,
                source_line,
                &raw_arguments,
            ),
            Some(DefinitionOperation::ProductionOutput) => relation(
                &mut relations,
                &mut relation_counts,
                "production_output",
                &arguments,
                "resource",
                Some("source_rate"),
                current_phase.as_deref(),
                &directive,
                source_line,
                &raw_arguments,
                "source_coefficient",
            ),
            Some(DefinitionOperation::ProductionInput) => relation(
                &mut relations,
                &mut relation_counts,
                "production_input",
                &arguments,
                "resource",
                Some("source_rate"),
                current_phase.as_deref(),
                &directive,
                source_line,
                &raw_arguments,
                "source_coefficient",
            ),
            Some(DefinitionOperation::ProductionInputPerSecond) => relation(
                &mut relations,
                &mut relation_counts,
                "production_input",
                &arguments,
                "resource",
                Some("per_second"),
                current_phase.as_deref(),
                &directive,
                source_line,
                &raw_arguments,
                "verified_time_basis",
            ),
            Some(DefinitionOperation::ProductionWasteInput) => relation(
                &mut relations,
                &mut relation_counts,
                "waste_input",
                &arguments,
                "resource",
                Some("source_rate"),
                current_phase.as_deref(),
                &directive,
                source_line,
                &raw_arguments,
                "source_coefficient",
            ),
            Some(DefinitionOperation::ConstructionMaterialExplicit) => relation(
                &mut relations,
                &mut relation_counts,
                "construction_material_explicit",
                &arguments,
                "resource",
                Some("source_quantity"),
                current_phase.as_deref(),
                &directive,
                source_line,
                &raw_arguments,
                "explicit_quantity",
            ),
            Some(DefinitionOperation::ConstructionMaterialAuto) => relation(
                &mut relations,
                &mut relation_counts,
                "construction_material_auto",
                &arguments,
                "construction_rule",
                Some("coefficient"),
                current_phase.as_deref(),
                &directive,
                source_line,
                &raw_arguments,
                "unresolved_auto",
            ),
            Some(DefinitionOperation::ConstructionNode) => relation(
                &mut relations,
                &mut relation_counts,
                "construction_node",
                &arguments,
                "construction_node",
                None,
                current_phase.as_deref(),
                &directive,
                source_line,
                &raw_arguments,
                "source_directive",
            ),
            Some(DefinitionOperation::ConstructionKeyword) => relation(
                &mut relations,
                &mut relation_counts,
                "construction_keyword",
                &arguments,
                "construction_keyword",
                None,
                current_phase.as_deref(),
                &directive,
                source_line,
                &raw_arguments,
                "source_directive",
            ),
            Some(DefinitionOperation::BuildingType) if entity_kind == "building" => {
                push_text_property(
                    &mut properties,
                    &mut property_counts,
                    "building.type",
                    directive.trim_start_matches('$'),
                    None,
                    &directive,
                    source_line,
                    &raw_arguments,
                    "game_definition",
                );
                true
            }
            Some(DefinitionOperation::VehicleSkill) if entity_kind == "vehicle" => {
                let field = format!(
                    "vehicle.skill.{}",
                    normalized_token(directive.trim_start_matches("$SKILL_"))
                );
                number_property(
                    &mut properties,
                    &mut property_counts,
                    &field,
                    &arguments,
                    None,
                    &directive,
                    source_line,
                    &raw_arguments,
                    "game_definition",
                )
            }
            Some(DefinitionOperation::StorageCapacity) => relation(
                &mut relations,
                &mut relation_counts,
                "storage_capacity",
                &arguments,
                "transport",
                Some("source_capacity"),
                current_phase.as_deref(),
                &directive,
                source_line,
                &raw_arguments,
                "game_definition",
            ),
            Some(DefinitionOperation::DefinitionRepairOrMaintenance) => {
                let field = format!(
                    "definition.repair.{}",
                    normalized_token(directive.trim_start_matches('$'))
                );
                if arguments
                    .first()
                    .and_then(|value| value.parse::<f64>().ok())
                    .is_some()
                {
                    number_property(
                        &mut properties,
                        &mut property_counts,
                        &field,
                        &arguments,
                        None,
                        &directive,
                        source_line,
                        &raw_arguments,
                        "explicit_only",
                    )
                } else {
                    text_property(
                        &mut properties,
                        &mut property_counts,
                        &field,
                        &arguments,
                        None,
                        &directive,
                        source_line,
                        &raw_arguments,
                    )
                }
            }
            _ => false,
        };
        if !recognized {
            *unknown.entry(directive).or_default() += 1;
        } else if let Some(mapping) = mapping {
            for property in &mut properties[property_start..] {
                property.mapping_id = mapping.id.clone();
                property.catalogue_scope_id = mapping.catalogue_scope.clone();
                property.mapping_classification = mapping.mapping_classification.clone();
            }
            for relation in &mut relations[relation_start..] {
                relation.mapping_id = mapping.id.clone();
                relation.catalogue_scope_id = mapping.catalogue_scope.clone();
                relation.mapping_classification = mapping.mapping_classification.clone();
            }
        }
    }

    let coverage = if warning_count == 0 {
        "complete"
    } else {
        "partial"
    };
    Ok((
        ParsedDefinition {
            entity_id,
            revision_hash,
            entity_kind: entity_kind.to_owned(),
            source_id: source.source_id.clone(),
            source_object_id,
            display_name,
            coverage: coverage.to_owned(),
            properties,
            relations,
            unknown_directives: unknown.into_iter().collect(),
        },
        warning_count,
    ))
}

fn evaluate_catalogue_scopes(
    profile: &ResolvedCompatibilityProfile,
    sources: &[CatalogueSource],
) -> Vec<CompatibilityCatalogueScopeStatus> {
    profile
        .catalogue_scopes()
        .iter()
        .map(|scope| {
            let source = sources
                .iter()
                .find(|source| source.source_id == scope.source_id);
            let state = match source {
                None => CompatibilityCatalogueScopeState::Dormant,
                Some(source) if source.content_hash == scope.acknowledged_content_hash => {
                    CompatibilityCatalogueScopeState::Matched
                }
                Some(_) if scope.update_policy == CatalogueScopeUpdatePolicy::Exact => {
                    CompatibilityCatalogueScopeState::Conflict
                }
                Some(_) => CompatibilityCatalogueScopeState::UpdatedUnreviewed,
            };
            CompatibilityCatalogueScopeStatus {
                id: scope.id.clone(),
                source_id: scope.source_id.clone(),
                package_name: source.map(|source| source.package_name.clone()),
                update_policy: match scope.update_policy {
                    CatalogueScopeUpdatePolicy::Exact => "exact",
                    CatalogueScopeUpdatePolicy::TrackUpdates => "track_updates",
                }
                .to_owned(),
                acknowledged_content_hash: scope.acknowledged_content_hash.clone(),
                current_content_hash: source.map(|source| source.content_hash.clone()),
                mapping_count: profile.catalogue_scope_mapping_count(&scope.id),
                state,
            }
        })
        .collect()
}

fn definition_identity(
    source: &SourceRoot,
    logical_path: &str,
    content_hash: &str,
    compatibility_hash: &str,
) -> (String, String, String, String) {
    let entity_kind = if logical_path
        .rsplit('/')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("script.ini"))
        || matches!(source.mode, ScanMode::Vehicles)
    {
        "vehicle".to_owned()
    } else {
        "building".to_owned()
    };
    let source_object_id = logical_path
        .trim_end_matches(".ini")
        .trim_end_matches("/building")
        .trim_end_matches("/script")
        .to_ascii_lowercase();
    let entity_id = format!(
        "{}::{}::{}",
        source.source_id, entity_kind, source_object_id
    );
    let revision_hash = hex_hash(
        format!("{content_hash}\0{entity_id}\0{DEFINITION_PARSER_VERSION}\0{compatibility_hash}")
            .as_bytes(),
    );
    (entity_kind, source_object_id, entity_id, revision_hash)
}

#[allow(clippy::too_many_arguments)]
fn push_text_property(
    properties: &mut Vec<ParsedProperty>,
    counts: &mut HashMap<String, u32>,
    field_id: &str,
    value: &str,
    unit: Option<&str>,
    directive: &str,
    line: u32,
    raw: &str,
    resolution: &str,
) {
    let occurrence = next_occurrence(counts, field_id);
    properties.push(ParsedProperty {
        field_id: field_id.to_owned(),
        occurrence,
        value_kind: "text".to_owned(),
        value_number: None,
        value_text: Some(truncate(value, 240)),
        unit: unit.map(str::to_owned),
        source_directive: directive.to_owned(),
        source_line: line,
        raw_arguments: raw.to_owned(),
        resolution: resolution.to_owned(),
        mapping_id: String::new(),
        catalogue_scope_id: None,
        mapping_classification: "reviewed_mapping".to_owned(),
    });
}

#[allow(clippy::too_many_arguments)]
fn text_property(
    properties: &mut Vec<ParsedProperty>,
    counts: &mut HashMap<String, u32>,
    field_id: &str,
    arguments: &[String],
    unit: Option<&str>,
    directive: &str,
    line: u32,
    raw: &str,
) -> bool {
    let Some(value) = arguments.first() else {
        return false;
    };
    push_text_property(
        properties,
        counts,
        field_id,
        value,
        unit,
        directive,
        line,
        raw,
        "game_definition",
    );
    true
}

#[allow(clippy::too_many_arguments)]
fn number_property(
    properties: &mut Vec<ParsedProperty>,
    counts: &mut HashMap<String, u32>,
    field_id: &str,
    arguments: &[String],
    unit: Option<&str>,
    directive: &str,
    line: u32,
    raw: &str,
    resolution: &str,
) -> bool {
    let Some(number) = arguments
        .first()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
    else {
        return false;
    };
    let occurrence = next_occurrence(counts, field_id);
    properties.push(ParsedProperty {
        field_id: field_id.to_owned(),
        occurrence,
        value_kind: "number".to_owned(),
        value_number: Some(number),
        value_text: None,
        unit: unit.map(str::to_owned),
        source_directive: directive.to_owned(),
        source_line: line,
        raw_arguments: raw.to_owned(),
        resolution: resolution.to_owned(),
        mapping_id: String::new(),
        catalogue_scope_id: None,
        mapping_classification: "reviewed_mapping".to_owned(),
    });
    true
}

#[allow(clippy::too_many_arguments)]
fn multi_number_properties<const N: usize>(
    properties: &mut Vec<ParsedProperty>,
    counts: &mut HashMap<String, u32>,
    fields: [&str; N],
    arguments: &[String],
    unit: Option<&str>,
    directive: &str,
    line: u32,
    raw: &str,
) -> bool {
    let mut added = false;
    for (field, value) in fields.into_iter().zip(arguments) {
        added |= number_property(
            properties,
            counts,
            field,
            std::slice::from_ref(value),
            unit,
            directive,
            line,
            raw,
            "game_definition",
        );
    }
    added
}

#[allow(clippy::too_many_arguments)]
fn relation(
    relations: &mut Vec<ParsedRelation>,
    counts: &mut HashMap<String, u32>,
    relation_kind: &str,
    arguments: &[String],
    target_kind: &str,
    unit: Option<&str>,
    phase: Option<&str>,
    directive: &str,
    line: u32,
    raw: &str,
    resolution: &str,
) -> bool {
    let Some(target) = arguments.first() else {
        return false;
    };
    let quantity = arguments
        .get(1)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite());
    let occurrence = next_occurrence(counts, relation_kind);
    relations.push(ParsedRelation {
        relation_kind: relation_kind.to_owned(),
        occurrence,
        target_id: format!("{target_kind}::{}", normalized_token(target)),
        quantity,
        unit: unit.map(str::to_owned),
        phase_id: phase.map(str::to_owned),
        source_directive: directive.to_owned(),
        source_line: line,
        raw_arguments: raw.to_owned(),
        resolution: resolution.to_owned(),
        mapping_id: String::new(),
        catalogue_scope_id: None,
        mapping_classification: "reviewed_mapping".to_owned(),
    });
    true
}

fn next_occurrence(counts: &mut HashMap<String, u32>, key: &str) -> u32 {
    let occurrence = counts.entry(key.to_owned()).or_default();
    let result = *occurrence;
    *occurrence = occurrence.saturating_add(1);
    result
}

fn add_resource_entities(entities: &mut Vec<ParsedDefinition>, mut resources: BTreeSet<String>) {
    resources.extend(
        entities
            .iter()
            .flat_map(|entity| entity.relations.iter())
            .filter_map(|relation| relation.target_id.strip_prefix("resource::"))
            .map(str::to_owned),
    );
    for resource in resources {
        let entity_id = format!("game.resources::resource::{resource}");
        entities.push(ParsedDefinition {
            revision_hash: hex_hash(format!("{entity_id}\0{DEFINITION_PARSER_VERSION}").as_bytes()),
            entity_id,
            entity_kind: "resource".to_owned(),
            source_id: "game.resources".to_owned(),
            source_object_id: resource.clone(),
            display_name: resource.replace('_', " "),
            coverage: "partial".to_owned(),
            properties: Vec::new(),
            relations: Vec::new(),
            unknown_directives: Vec::new(),
        });
    }
}

fn add_recipe_entities(
    entities: &mut Vec<ParsedDefinition>,
    reuse: &HashMap<String, CatalogueReuseEntry>,
) {
    let recipes = entities
        .iter()
        .filter(|entity| entity.entity_kind == "building")
        .filter(|entity| {
            entity.relations.iter().any(|relation| {
                matches!(
                    relation.relation_kind.as_str(),
                    "production_input" | "production_output" | "waste_input"
                )
            }) || reuse
                .get(&entity.revision_hash)
                .is_some_and(|entry| entry.has_production_route)
        })
        .map(|entity| {
            let entity_id = format!("{}::recipe::{}", entity.source_id, entity.source_object_id);
            let revision_hash = hex_hash(
                format!(
                    "{}\0{}\0{}",
                    entity.revision_hash, entity_id, DEFINITION_PARSER_VERSION
                )
                .as_bytes(),
            );
            ParsedDefinition {
                entity_id,
                revision_hash,
                entity_kind: "recipe".to_owned(),
                source_id: entity.source_id.clone(),
                source_object_id: entity.source_object_id.clone(),
                display_name: format!("{} production route", entity.display_name),
                coverage: entity.coverage.clone(),
                properties: vec![ParsedProperty {
                    field_id: "recipe.building.entity_id".to_owned(),
                    occurrence: 0,
                    value_kind: "text".to_owned(),
                    value_number: None,
                    value_text: Some(entity.entity_id.clone()),
                    unit: None,
                    source_directive: "$DERIVED_RECIPE".to_owned(),
                    source_line: 0,
                    raw_arguments: String::new(),
                    resolution: "derived_from_definition".to_owned(),
                    mapping_id: "host.derived.recipe".to_owned(),
                    catalogue_scope_id: None,
                    mapping_classification: "reviewed_mapping".to_owned(),
                }],
                relations: entity
                    .relations
                    .iter()
                    .filter(|relation| {
                        matches!(
                            relation.relation_kind.as_str(),
                            "production_input" | "production_output" | "waste_input"
                        )
                    })
                    .cloned()
                    .collect(),
                unknown_directives: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    entities.extend(recipes);
}

fn workshop_metadata(root: &Path) -> Option<(String, String)> {
    let bytes = fs::read(root.join("workshopconfig.ini")).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let mut item_id = None;
    let mut item_name = None;
    for line in text.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("$ITEM_ID") {
            item_id = split_arguments(value.trim()).into_iter().next();
        } else if let Some(value) = line.strip_prefix("$ITEM_NAME") {
            item_name = split_arguments(value.trim()).into_iter().next();
        }
    }
    Some((
        item_id?,
        item_name.unwrap_or_else(|| "Workshop item".to_owned()),
    ))
}

fn discover_game_build(media_directory: &Path) -> Option<String> {
    let steamapps = media_directory.parent()?.parent()?.parent()?;
    let manifest = fs::read_to_string(steamapps.join("appmanifest_784150.acf")).ok()?;
    manifest.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("\"buildid\"")
            .and_then(|value| split_arguments(value.trim()).into_iter().next())
    })
}

fn logical_path(root: &Path, path: &Path) -> Result<String, ObservatoryError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ObservatoryError::InvalidCatalogueRequest)?;
    let value = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/");
    if value.is_empty() || value.contains("..") {
        return Err(ObservatoryError::InvalidCatalogueRequest);
    }
    Ok(value)
}

fn fallback_display_name(source_object_id: &str) -> String {
    source_object_id
        .rsplit('/')
        .next()
        .unwrap_or(source_object_id)
        .replace(['_', '-'], " ")
}

fn normalized_token(value: &str) -> String {
    value
        .trim_matches('"')
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn split_arguments(value: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in value.chars() {
        match character {
            '"' => quoted = !quoted,
            _ if character.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    arguments.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(character),
        }
    }
    if !current.is_empty() {
        arguments.push(current);
    }
    arguments
}

fn truncate(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

fn hex_hash(bytes: &[u8]) -> String {
    hex_bytes(&Sha256::digest(bytes))
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, Instant};

    use tempfile::tempdir;

    use super::*;
    use crate::compatibility_profile::{
        CatalogueScopeMapping, CatalogueScopeUpdatePolicy, CompatibilityProfileDocument,
        DefinitionDirectiveMapping, DirectiveMatch, DirectiveMatchKind,
    };

    fn reuse_entries(generation: &CatalogueGeneration) -> HashMap<String, CatalogueReuseEntry> {
        generation
            .entities
            .iter()
            .filter(|entity| matches!(entity.entity_kind.as_str(), "building" | "vehicle"))
            .map(|entity| {
                (
                    entity.revision_hash.clone(),
                    CatalogueReuseEntry {
                        display_name: entity.display_name.clone(),
                        coverage: entity.coverage.clone(),
                        resource_targets: entity
                            .relations
                            .iter()
                            .filter_map(|relation| {
                                relation
                                    .target_id
                                    .strip_prefix("resource::")
                                    .map(str::to_owned)
                            })
                            .collect(),
                        has_production_route: entity.relations.iter().any(|relation| {
                            matches!(
                                relation.relation_kind.as_str(),
                                "production_input" | "production_output" | "waste_input"
                            )
                        }),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn separates_explicit_and_automatic_construction_evidence() {
        let directory = tempdir().expect("temporary directory");
        let media = directory.path().join("media_soviet");
        let buildings = media.join("buildings_types");
        let vehicles = media.join("vehicles");
        fs::create_dir_all(&buildings).expect("buildings");
        fs::create_dir_all(&vehicles).expect("vehicles");
        fs::write(
            buildings.join("plant.ini"),
            b"$NAME_STR \"Test Plant\"\n$TYPE_FACTORY\n$WORKERS_NEEDED 20\n\
              $PRODUCTION chemicals 0.5\n$CONSUMPTION oil 1.25\n\
              $COST_WORK SOVIET_CONSTRUCTION_GROUNDWORKS 0\n\
              $COST_RESOURCE concrete 12\n$COST_RESOURCE_AUTO wall_steel 0.7\nend\n",
        )
        .expect("definition");

        let generation = discover_catalogue(&media, 1).expect("catalogue");
        let plant = generation
            .entities
            .iter()
            .find(|entity| entity.display_name == "Test Plant")
            .expect("plant");
        assert!(plant.relations.iter().any(|relation| {
            relation.relation_kind == "construction_material_explicit"
                && relation.target_id == "resource::concrete"
                && relation.quantity == Some(12.0)
        }));
        assert!(plant.relations.iter().any(|relation| {
            relation.relation_kind == "construction_material_auto"
                && relation.target_id == "construction_rule::wall_steel"
                && relation.resolution == "unresolved_auto"
        }));
        let recipe = generation
            .entities
            .iter()
            .find(|entity| entity.entity_kind == "recipe")
            .expect("derived recipe");
        assert!(recipe.relations.iter().any(|relation| {
            relation.relation_kind == "production_output"
                && relation.target_id == "resource::chemicals"
        }));
        assert!(recipe.relations.iter().any(|relation| {
            relation.relation_kind == "production_input" && relation.target_id == "resource::oil"
        }));
    }

    #[test]
    fn discovers_workshop_sources_without_colliding_with_base_identity() {
        let directory = tempdir().expect("temporary directory");
        let media = directory.path().join("media_soviet");
        fs::create_dir_all(media.join("buildings_types")).expect("buildings");
        fs::create_dir_all(media.join("vehicles")).expect("vehicles");
        let item = media.join("workshop_wip").join("900000001");
        fs::create_dir_all(item.join("asset")).expect("item");
        fs::write(
            item.join("workshopconfig.ini"),
            b"$ITEM_ID 900000001\n$ITEM_NAME \"Test Mod\"\n$END\n",
        )
        .expect("metadata");
        fs::write(
            item.join("asset").join("building.ini"),
            b"$NAME_STR \"Mod Building\"\n$TYPE_FACTORY\nend\n",
        )
        .expect("definition");

        let generation = discover_catalogue(&media, 1).expect("catalogue");
        assert!(
            generation
                .sources
                .iter()
                .any(|source| source.source_id == "wip.900000001")
        );
        assert!(
            generation
                .entities
                .iter()
                .any(|entity| entity.entity_id.starts_with("wip.900000001::building::"))
        );
    }

    #[test]
    fn changed_file_refresh_reuses_unchanged_revisions_and_retires_removed_sources() {
        let directory = tempdir().expect("temporary directory");
        let media = directory.path().join("media_soviet");
        let buildings = media.join("buildings_types");
        fs::create_dir_all(&buildings).expect("buildings");
        fs::create_dir_all(media.join("vehicles")).expect("vehicles");
        fs::write(
            buildings.join("one.ini"),
            b"$NAME_STR \"One\"\n$TYPE_FACTORY\n$PRODUCTION steel 1\n",
        )
        .expect("first file");
        fs::write(
            buildings.join("two.ini"),
            b"$NAME_STR \"Two\"\n$TYPE_FACTORY\n$PRODUCTION chemicals 1\n",
        )
        .expect("second file");
        let first = discover_catalogue(&media, 1).expect("first catalogue");
        let cache = reuse_entries(&first);
        fs::remove_file(buildings.join("two.ini")).expect("remove source file");
        let second =
            discover_catalogue_with_reuse(&media, None, 2, &cache).expect("incremental catalogue");
        let one = second
            .entities
            .iter()
            .find(|entity| entity.display_name == "One")
            .expect("retained entity");
        assert!(
            one.properties.is_empty(),
            "unchanged entity was not reparsed"
        );
        assert!(second.entities.iter().any(|entity| {
            entity.entity_kind == "recipe" && entity.source_object_id.ends_with("one")
        }));
        assert!(
            !second
                .entities
                .iter()
                .any(|entity| entity.display_name == "Two")
        );
        assert_ne!(first.generation_id, second.generation_id);
    }

    #[test]
    fn lossy_and_unsupported_directives_are_diagnostics_not_source_copies() {
        let directory = tempdir().expect("temporary directory");
        let media = directory.path().join("media_soviet");
        let buildings = media.join("buildings_types");
        fs::create_dir_all(&buildings).expect("buildings");
        fs::create_dir_all(media.join("vehicles")).expect("vehicles");
        fs::write(
            buildings.join("lossy.ini"),
            b"$NAME_STR \"Lossy\xff\"\n$UNSUPPORTED secret-value\n",
        )
        .expect("lossy file");
        let generation = discover_catalogue(&media, 1).expect("catalogue");
        let entity = generation
            .entities
            .iter()
            .find(|entity| entity.entity_kind == "building")
            .expect("entity");
        assert_eq!(entity.coverage, "partial");
        assert_eq!(
            entity.unknown_directives,
            vec![("$UNSUPPORTED".to_owned(), 1)]
        );
        assert!(generation.files[0].warning_count > 0);
    }

    #[test]
    fn containment_and_parser_limits_fail_closed() {
        let directory = tempdir().expect("temporary directory");
        let root = directory.path().join("source");
        let outside = directory.path().join("outside.ini");
        fs::create_dir(&root).expect("source root");
        fs::write(&outside, b"$NAME_STR Outside").expect("outside definition");
        assert!(logical_path(&root, &outside).is_err());

        let source = SourceRoot {
            source_id: "base.buildings".to_owned(),
            source_kind: "base".to_owned(),
            package_name: "Base buildings".to_owned(),
            package_version: None,
            root,
            mode: ScanMode::BaseBuildings,
        };
        let oversized_line = format!("$NAME_STR {}\n$TYPE_FACTORY\n", "x".repeat(MAX_LINE_BYTES));
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let (definition, warning_count) = parse_definition(
            &source,
            "limited.ini",
            "content",
            oversized_line.as_bytes(),
            &profile,
        )
        .expect("definition");
        assert_eq!(warning_count, 1);
        assert!(definition.display_name.contains("limited"));
        assert!(
            definition
                .properties
                .iter()
                .all(|property| property.raw_arguments.len() <= 1_024)
        );
    }

    #[test]
    fn catalogue_scopes_distinguish_exact_conflicts_tracked_updates_and_dormancy() {
        let base = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        let mut local = CompatibilityProfileDocument::starter_override(&base);
        local.mappings.catalogue_scopes = Some(vec![
            CatalogueScopeMapping {
                id: "local.scope.exact".to_owned(),
                source_id: "workshop.1".to_owned(),
                acknowledged_content_hash: "a".repeat(64),
                update_policy: CatalogueScopeUpdatePolicy::Exact,
            },
            CatalogueScopeMapping {
                id: "local.scope.tracked".to_owned(),
                source_id: "wip.2".to_owned(),
                acknowledged_content_hash: "b".repeat(64),
                update_policy: CatalogueScopeUpdatePolicy::TrackUpdates,
            },
            CatalogueScopeMapping {
                id: "local.scope.dormant".to_owned(),
                source_id: "workshop.3".to_owned(),
                acknowledged_content_hash: "c".repeat(64),
                update_policy: CatalogueScopeUpdatePolicy::Exact,
            },
        ]);
        local.mappings.definition_directives = Some(
            ["exact", "tracked", "dormant"]
                .into_iter()
                .map(|name| DefinitionDirectiveMapping {
                    id: format!("local.mapping.{name}"),
                    operation: DefinitionOperation::BuildingStyle,
                    matches: vec![DirectiveMatch {
                        kind: DirectiveMatchKind::Exact,
                        value: format!("$MOD_{}", name.to_ascii_uppercase()),
                    }],
                    catalogue_scope: Some(format!("local.scope.{name}")),
                })
                .collect(),
        );
        let profile =
            ResolvedCompatibilityProfile::resolve_override(&base, local).expect("resolved profile");
        let sources = vec![
            CatalogueSource {
                source_id: "workshop.1".to_owned(),
                source_kind: "workshop".to_owned(),
                package_name: "Exact package".to_owned(),
                package_version: None,
                content_hash: "d".repeat(64),
                file_count: 1,
            },
            CatalogueSource {
                source_id: "wip.2".to_owned(),
                source_kind: "wip".to_owned(),
                package_name: "Tracked package".to_owned(),
                package_version: None,
                content_hash: "e".repeat(64),
                file_count: 1,
            },
        ];
        let statuses = evaluate_catalogue_scopes(&profile, &sources);
        assert_eq!(
            statuses[0].state,
            CompatibilityCatalogueScopeState::Conflict
        );
        assert_eq!(
            statuses[1].state,
            CompatibilityCatalogueScopeState::UpdatedUnreviewed
        );
        assert_eq!(statuses[2].state, CompatibilityCatalogueScopeState::Dormant);
    }

    #[test]
    fn progress_reports_bounded_source_relative_files() {
        let directory = tempdir().expect("temporary directory");
        let media = directory.path().join("media_soviet");
        let buildings = media.join("buildings_types").join("district");
        fs::create_dir_all(&buildings).expect("buildings");
        fs::create_dir_all(media.join("vehicles")).expect("vehicles");
        fs::write(
            buildings.join("plant.ini"),
            b"$NAME_STR Plant\n$TYPE_FACTORY\n",
        )
        .expect("definition");
        let mut updates = Vec::new();

        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("profile");
        discover_catalogue_with_reuse_and_progress(
            &media,
            None,
            1,
            &HashMap::new(),
            &profile,
            |progress| updates.push(progress),
        )
        .expect("catalogue");

        let scanning = updates
            .iter()
            .find(|progress| progress.phase == CatalogueDiscoveryPhase::Scanning)
            .expect("scanning progress");
        assert_eq!(scanning.current_file.as_deref(), Some("district/plant.ini"));
        assert_eq!(scanning.current_file_index, Some(1));
        assert!(updates.iter().all(|progress| {
            progress.current_file.as_ref().is_none_or(|file| {
                !file.contains(directory.path().to_string_lossy().as_ref())
                    && !file.contains('\\')
                    && file.len() <= 180
            })
        }));
    }

    #[test]
    #[ignore = "requires a private local W&R installation and is a reference-machine benchmark"]
    fn presently_installed_catalogue_meets_reference_target() {
        let media = std::env::var_os("RO_GAME_MEDIA").expect("set RO_GAME_MEDIA privately");
        let started = Instant::now();
        let generation = discover_catalogue(Path::new(&media), 1).expect("local catalogue");
        eprintln!(
            "catalogue files={} entities={} elapsed={:?}",
            generation.files.len(),
            generation.entities.len(),
            started.elapsed()
        );
        assert!(generation.files.len() >= 1_000);
        assert!(started.elapsed() < Duration::from_secs(30));
    }

    #[test]
    #[ignore = "reference-machine repeated Workshop update benchmark"]
    fn synthetic_large_workshop_update_batch_meets_incremental_target() {
        let directory = tempdir().expect("temporary directory");
        let media = directory.path().join("media_soviet");
        fs::create_dir_all(media.join("buildings_types")).expect("buildings");
        fs::create_dir_all(media.join("vehicles")).expect("vehicles");
        let item = media.join("workshop_subscribed").join("900000002");
        fs::create_dir_all(&item).expect("Workshop item");
        fs::write(
            item.join("workshopconfig.ini"),
            b"$ITEM_ID 900000002\n$ITEM_NAME Synthetic Batch\n$END\n",
        )
        .expect("metadata");
        for index in 0..5_000 {
            let asset = item.join(format!("asset-{index}"));
            fs::create_dir(&asset).expect("asset");
            fs::write(
                asset.join("building.ini"),
                format!("$NAME_STR Asset {index}\n$TYPE_FACTORY\n$PRODUCTION steel 1\n"),
            )
            .expect("definition");
        }
        let first = discover_catalogue(&media, 1).expect("initial catalogue");
        let cache = reuse_entries(&first);
        for index in 0..100 {
            fs::write(
                item.join(format!("asset-{index}")).join("building.ini"),
                format!("$NAME_STR Updated Asset {index}\n$TYPE_FACTORY\n$PRODUCTION steel 2\n"),
            )
            .expect("updated definition");
        }
        let started = Instant::now();
        let second =
            discover_catalogue_with_reuse(&media, None, 2, &cache).expect("incremental catalogue");
        eprintln!(
            "incremental files={} elapsed={:?}",
            second.files.len(),
            started.elapsed()
        );
        assert_eq!(second.files.len(), 5_000);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
