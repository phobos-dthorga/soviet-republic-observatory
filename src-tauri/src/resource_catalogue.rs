use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::error::ObservatoryError;
use crate::model::{
    ResourceCatalogueEntry, ResourceCatalogueOriginFilter, ResourceCatalogueRequest,
    ResourceCatalogueRevision, ResourceCatalogueView, ResourceDetails, ResourceOriginEvidence,
};
use crate::storage::StoredLiveResources;
use crate::warehouse::InstalledResourceEvidence;

const MAX_RESOURCE_ENTRIES: usize = 8_192;

#[derive(Clone, Debug)]
struct ResourceBuildEntry {
    entry: ResourceCatalogueEntry,
    installed_sources: Vec<String>,
}

pub(crate) fn catalogue(
    installed: Vec<InstalledResourceEvidence>,
    recorded_tokens: Vec<String>,
    definition_generation_id: Option<String>,
    overlay_revision: Option<String>,
    live: Option<StoredLiveResources>,
    request: &ResourceCatalogueRequest,
) -> Result<ResourceCatalogueView, ObservatoryError> {
    let live_snapshot_id = live.as_ref().map(|live| live.summary.snapshot_id.clone());
    let all = build_entries(installed, recorded_tokens, live.map(|live| live.entries))?;
    let revision = revision(
        &all,
        definition_generation_id,
        overlay_revision,
        live_snapshot_id,
    );
    let query = request.query.as_deref().unwrap_or("").trim();
    if query.len() > 120 {
        return Err(ObservatoryError::InvalidCatalogueRequest);
    }
    let query = query.to_ascii_lowercase();
    let limit = request.limit.unwrap_or(100).clamp(1, 250);
    let offset = request.offset.unwrap_or(0).min(1_000_000);
    let filtered = all
        .into_values()
        .filter(|candidate| {
            (query.is_empty()
                || candidate
                    .entry
                    .source_token
                    .to_ascii_lowercase()
                    .contains(&query)
                || candidate
                    .entry
                    .display_name
                    .to_ascii_lowercase()
                    .contains(&query))
                && origin_matches(&candidate.entry.origin, request.origin)
        })
        .collect::<Vec<_>>();
    let total = filtered.len().min(u32::MAX as usize) as u32;
    let entries = filtered
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|candidate| candidate.entry)
        .collect();
    Ok(ResourceCatalogueView {
        revision: ResourceCatalogueRevision {
            entry_count: revision.entry_count,
            ..revision
        },
        total,
        limit,
        offset,
        entries,
    })
}

pub(crate) fn details(
    installed: Vec<InstalledResourceEvidence>,
    recorded_tokens: Vec<String>,
    definition_generation_id: Option<String>,
    overlay_revision: Option<String>,
    live: Option<StoredLiveResources>,
    resource_id: &str,
) -> Result<ResourceDetails, ObservatoryError> {
    if resource_id.len() > 160 || !resource_id.starts_with("resource::") {
        return Err(ObservatoryError::InvalidCatalogueRequest);
    }
    let live_summary = live.as_ref().map(|live| live.summary.clone());
    let live_snapshot_id = live_summary
        .as_ref()
        .map(|summary| summary.snapshot_id.clone());
    let all = build_entries(installed, recorded_tokens, live.map(|live| live.entries))?;
    let revision = revision(
        &all,
        definition_generation_id,
        overlay_revision,
        live_snapshot_id,
    );
    let candidate = all
        .get(resource_id)
        .ok_or(ObservatoryError::InvalidCatalogueRequest)?;
    Ok(ResourceDetails {
        revision_id: revision.revision_id,
        entry: candidate.entry.clone(),
        installed_sources: candidate.installed_sources.clone(),
        recorded_profile_count: u32::from(candidate.entry.origin.recorded_save),
        live_snapshot: candidate
            .entry
            .origin
            .live_game
            .then_some(live_summary)
            .flatten(),
    })
}

fn build_entries(
    installed: Vec<InstalledResourceEvidence>,
    recorded_tokens: Vec<String>,
    live_entries: Option<Vec<ResourceCatalogueEntry>>,
) -> Result<BTreeMap<String, ResourceBuildEntry>, ObservatoryError> {
    let mut entries = BTreeMap::<String, ResourceBuildEntry>::new();
    for evidence in installed {
        validate_token(&evidence.source_token)?;
        let resource_id = format!("resource::{}", evidence.source_token);
        let candidate = entries
            .entry(resource_id.clone())
            .or_insert_with(|| ResourceBuildEntry {
                entry: empty_entry(&evidence.source_token),
                installed_sources: Vec::new(),
            });
        if evidence.player_overlay {
            candidate.entry.origin.player_overlay = true;
            if !candidate.entry.origin.installed_content {
                candidate.entry.display_name = evidence.display_name;
                candidate.entry.label_source = "player_overlay".to_owned();
            }
        } else {
            candidate.entry.origin.installed_content = true;
            candidate.entry.origin.installed_reference_count = candidate
                .entry
                .origin
                .installed_reference_count
                .saturating_add(evidence.installed_reference_count);
            candidate.entry.display_name = evidence.display_name;
            candidate.entry.label_source = "installed_definition".to_owned();
            candidate
                .installed_sources
                .extend(evidence.installed_sources);
            candidate.installed_sources.sort();
            candidate.installed_sources.dedup();
        }
    }
    for source_token in recorded_tokens {
        validate_token(&source_token)?;
        let resource_id = format!("resource::{source_token}");
        entries
            .entry(resource_id)
            .or_insert_with(|| ResourceBuildEntry {
                entry: empty_entry(&source_token),
                installed_sources: Vec::new(),
            })
            .entry
            .origin
            .recorded_save = true;
    }
    for live in live_entries.unwrap_or_default() {
        validate_token(&live.source_token)?;
        let resource_id = live.resource_id.clone();
        let candidate = entries
            .entry(resource_id)
            .or_insert_with(|| ResourceBuildEntry {
                entry: empty_entry(&live.source_token),
                installed_sources: Vec::new(),
            });
        candidate.entry.origin.live_game = true;
        candidate.entry.origin.runtime_extension = live.origin.runtime_extension;
        candidate.entry.caption_id = live.caption_id;
        candidate.entry.live_index = live.live_index;
        candidate.entry.resource_kind = live.resource_kind;
        candidate.entry.transport_classes = live.transport_classes;
        candidate.entry.material_family = live.material_family;
        candidate.entry.live_prices = live.live_prices;
        candidate.entry.latest_live_snapshot_id = live.latest_live_snapshot_id;
        if live.label_source != "source_token" {
            candidate.entry.display_name = live.display_name;
            candidate.entry.label_source = live.label_source;
        }
    }
    if entries.len() > MAX_RESOURCE_ENTRIES {
        return Err(ObservatoryError::InvalidCatalogueRequest);
    }
    Ok(entries)
}

fn empty_entry(source_token: &str) -> ResourceCatalogueEntry {
    ResourceCatalogueEntry {
        resource_id: format!("resource::{source_token}"),
        source_token: source_token.to_owned(),
        display_name: readable_token(source_token),
        label_source: "source_token".to_owned(),
        caption_id: None,
        live_index: None,
        resource_kind: None,
        transport_classes: Vec::new(),
        material_family: None,
        origin: ResourceOriginEvidence {
            installed_content: false,
            recorded_save: false,
            live_game: false,
            runtime_extension: false,
            player_overlay: false,
            installed_reference_count: 0,
        },
        live_prices: Vec::new(),
        latest_live_snapshot_id: None,
    }
}

fn revision(
    entries: &BTreeMap<String, ResourceBuildEntry>,
    definition_generation_id: Option<String>,
    overlay_revision: Option<String>,
    live_snapshot_id: Option<String>,
) -> ResourceCatalogueRevision {
    let mut hasher = Sha256::new();
    hasher.update(b"republic-observatory-resource-catalogue.v1\0");
    if let Some(value) = &definition_generation_id {
        hasher.update(value.as_bytes());
    }
    hasher.update([0]);
    if let Some(value) = &overlay_revision {
        hasher.update(value.as_bytes());
    }
    hasher.update([0]);
    if let Some(value) = &live_snapshot_id {
        hasher.update(value.as_bytes());
    }
    for candidate in entries.values() {
        let entry = &candidate.entry;
        hasher.update([0]);
        hasher.update(entry.source_token.as_bytes());
        hasher.update([u8::from(entry.origin.installed_content)]);
        hasher.update([u8::from(entry.origin.recorded_save)]);
        hasher.update([u8::from(entry.origin.player_overlay)]);
    }
    let revision_id = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ResourceCatalogueRevision {
        revision_id,
        definition_generation_id,
        overlay_revision,
        live_snapshot_id,
        entry_count: entries.len().min(u32::MAX as usize) as u32,
    }
}

fn origin_matches(
    origin: &ResourceOriginEvidence,
    filter: Option<ResourceCatalogueOriginFilter>,
) -> bool {
    match filter {
        None => true,
        Some(ResourceCatalogueOriginFilter::InstalledContent) => origin.installed_content,
        Some(ResourceCatalogueOriginFilter::RecordedSave) => origin.recorded_save,
        Some(ResourceCatalogueOriginFilter::LiveGame) => origin.live_game,
        Some(ResourceCatalogueOriginFilter::PlayerOverlay) => origin.player_overlay,
    }
}

fn validate_token(value: &str) -> Result<(), ObservatoryError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '/' | '\\'))
    {
        return Err(ObservatoryError::InvalidCatalogueRequest);
    }
    Ok(())
}

fn readable_token(value: &str) -> String {
    let mut label = value.replace(['_', '-'], " ");
    if let Some(first) = label.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(token: &str) -> InstalledResourceEvidence {
        InstalledResourceEvidence {
            source_token: token.to_owned(),
            display_name: readable_token(token),
            installed_reference_count: 2,
            installed_sources: vec!["workshop.42".to_owned()],
            player_overlay: false,
        }
    }

    #[test]
    fn merges_exact_tokens_without_a_fixed_inventory() {
        let view = catalogue(
            vec![installed("unobtainium_crystal")],
            vec![
                "save_only_fluid".to_owned(),
                "unobtainium_crystal".to_owned(),
            ],
            Some("definitions".to_owned()),
            None,
            None,
            &ResourceCatalogueRequest {
                query: None,
                origin: None,
                limit: None,
                offset: None,
            },
        )
        .expect("catalogue");
        assert_eq!(view.total, 2);
        let crystal = view
            .entries
            .iter()
            .find(|entry| entry.source_token == "unobtainium_crystal")
            .expect("synthetic resource");
        assert!(crystal.origin.installed_content);
        assert!(crystal.origin.recorded_save);
    }

    #[test]
    fn treats_similar_spellings_as_distinct_identities() {
        let view = catalogue(
            vec![installed("electronics")],
            vec!["eletronics".to_owned()],
            None,
            None,
            None,
            &ResourceCatalogueRequest {
                query: None,
                origin: None,
                limit: None,
                offset: None,
            },
        )
        .expect("catalogue");
        assert_eq!(view.total, 2);
    }
}
