use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::ObservatoryError;
use crate::model::{CompatibilityProvenance, RECEIVER_METRICS, SNAPSHOT_FACTS};

pub const PARSER_ENGINE_API_VERSION: &str = "1.0.0";
pub const PARSER_ENGINE_VERSION: &str = "compatibility-profile-engine.v2";
pub const BUILTIN_PROFILE_ID: &str = "org.republic-observatory.wrsr-1.1.1.9";
const LEGACY_BUILTIN_PROFILE_VERSION: &str = "1.0.0";
const LEGACY_BUILTIN_PROFILE_HASH: &str =
    "0f2737d29ddb50aa22a32d6fb1747e7c0ec5aa00227464a38d68b4ae1bac522e";

const BUILTIN_PROFILE_BYTES: &[u8] =
    include_bytes!("../../compatibility/wrsr-1.1.1.9.rocompat.json");
const MAX_PROFILE_BYTES: usize = 1024 * 1024;
const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

const REQUIRED_STATS_SLOTS: &[&str] = &[
    "core.citizens.electronics.none",
    "core.citizens.electronics.radio",
    "core.citizens.electronics.television",
    "core.citizens.electronics.computer",
    "source.stats.citizens.born",
    "source.stats.citizens.dead",
    "source.stats.citizens.escaped",
    "source.stats.citizens.immigrant_soviet",
    "source.stats.citizens.immigrant_africa",
    "source.stats.citizens.small_children",
    "source.stats.citizens.medium_children",
    "source.stats.citizens.adults_parent",
    "source.stats.citizens.adults",
    "source.stats.citizens.unemployed",
    "source.stats.citizens.no_education",
    "source.stats.citizens.basic_education",
    "source.stats.citizens.higher_education",
    "source.stats.citizens.car_owners",
];

const MARKET_STATS_SLOTS: &[&str] = &[
    "market.price.purchase.rub",
    "market.price.purchase.usd",
    "market.price.sell.rub",
    "market.price.sell.usd",
    "market.price.base.rub",
    "market.price.base.usd",
    "market.trade.import.standard.rub",
    "market.trade.import.standard.usd",
    "market.trade.export.standard.rub",
    "market.trade.export.standard.usd",
    "market.trade.import.international.rub",
    "market.trade.import.international.usd",
    "market.trade.export.international.rub",
    "market.trade.export.international.usd",
    "market.cost.delivery.rub",
    "market.cost.delivery.usd",
    "market.cost.labour.rub",
    "market.cost.labour.usd",
    "market.cost.immigrant.rub",
    "market.cost.immigrant.usd",
    "market.tourism.visitors",
    "market.tourism.hotel_nights",
    "market.tourism.spending.rub",
    "market.tourism.spending.usd",
    "market.loan.balance.rub",
    "market.loan.balance.usd",
    "market.loan.interest.rub",
    "market.loan.interest.usd",
    "market.vehicle.import.rub",
    "market.vehicle.import.usd",
    "market.vehicle.export.rub",
    "market.vehicle.export.usd",
];

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityProfileSource {
    ReviewedBuiltin,
    LocalOverride,
}

impl CompatibilityProfileSource {
    pub fn evidence_classification(self) -> &'static str {
        match self {
            Self::ReviewedBuiltin => "reviewed_mapping",
            Self::LocalOverride => "player_mapped",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityProfileDocument {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub parser_engine_api_version: String,
    pub author: String,
    pub description: String,
    pub targets: ProfileTargets,
    pub content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<BaseProfileReference>,
    pub mappings: ProfileMappings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileTargets {
    pub game_versions: Vec<String>,
    pub build_ids: Vec<String>,
    pub stats_formats: Vec<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseProfileReference {
    pub id: String,
    pub version: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileMappings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_scopes: Option<Vec<CatalogueScopeMapping>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_entries: Option<Vec<ArchiveEntryMapping>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_markers: Option<Vec<StatsMarkerMapping>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_fields: Option<Vec<StatsFieldMapping>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_directives: Option<Vec<DefinitionDirectiveMapping>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_layouts: Option<Vec<BinaryLayout>>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveSlot {
    Stats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveEntryMapping {
    pub slot: ArchiveSlot,
    pub aliases: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StatsMarkerSlot {
    Format,
    HistoryRecord,
    CurrentState,
    City,
    DateYear,
    DateDay,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatsMarkerMapping {
    pub slot: StatsMarkerSlot,
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_values: Option<Vec<u16>>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StatsContext {
    History,
    Republic,
    City,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatsFieldMapping {
    pub host_slot: String,
    pub aliases: Vec<String>,
    pub contexts: Vec<StatsContext>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DirectiveMatchKind {
    Exact,
    Prefix,
    Contains,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectiveMatch {
    pub kind: DirectiveMatchKind,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DefinitionOperation {
    #[serde(rename = "building.construction_phase")]
    BuildingConstructionPhase,
    #[serde(rename = "definition.display_name")]
    DefinitionDisplayName,
    #[serde(rename = "definition.name_token")]
    DefinitionNameToken,
    #[serde(rename = "building.style")]
    BuildingStyle,
    #[serde(rename = "building.workers_required")]
    BuildingWorkersRequired,
    #[serde(rename = "building.professors_required")]
    BuildingProfessorsRequired,
    #[serde(rename = "building.service_capacity")]
    BuildingServiceCapacity,
    #[serde(rename = "building.quality_of_living")]
    BuildingQualityOfLiving,
    #[serde(rename = "building.vehicle_capacity")]
    BuildingVehicleCapacity,
    #[serde(rename = "vehicle.type")]
    VehicleType,
    #[serde(rename = "vehicle.family")]
    VehicleFamily,
    #[serde(rename = "vehicle.country")]
    VehicleCountry,
    #[serde(rename = "definition.availability")]
    DefinitionAvailability,
    #[serde(rename = "vehicle.lifespan")]
    VehicleLifespan,
    #[serde(rename = "vehicle.cost_rub")]
    VehicleCostRub,
    #[serde(rename = "vehicle.cost_usd")]
    VehicleCostUsd,
    #[serde(rename = "vehicle.speed")]
    VehicleSpeed,
    #[serde(rename = "vehicle.power")]
    VehiclePower,
    #[serde(rename = "vehicle.empty_weight")]
    VehicleEmptyWeight,
    #[serde(rename = "vehicle.consumption")]
    VehicleConsumption,
    #[serde(rename = "vehicle.resource_capacity")]
    VehicleResourceCapacity,
    #[serde(rename = "vehicle.transport_type")]
    VehicleTransportType,
    #[serde(rename = "vehicle.resource_allowed")]
    VehicleResourceAllowed,
    #[serde(rename = "production.output")]
    ProductionOutput,
    #[serde(rename = "production.input")]
    ProductionInput,
    #[serde(rename = "production.input_per_second")]
    ProductionInputPerSecond,
    #[serde(rename = "production.waste_input")]
    ProductionWasteInput,
    #[serde(rename = "construction.material_explicit")]
    ConstructionMaterialExplicit,
    #[serde(rename = "construction.material_auto")]
    ConstructionMaterialAuto,
    #[serde(rename = "construction.node")]
    ConstructionNode,
    #[serde(rename = "construction.keyword")]
    ConstructionKeyword,
    #[serde(rename = "building.type")]
    BuildingType,
    #[serde(rename = "vehicle.skill")]
    VehicleSkill,
    #[serde(rename = "storage.capacity")]
    StorageCapacity,
    #[serde(rename = "definition.repair_or_maintenance")]
    DefinitionRepairOrMaintenance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionDirectiveMapping {
    pub id: String,
    pub operation: DefinitionOperation,
    pub matches: Vec<DirectiveMatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_scope: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CatalogueScopeUpdatePolicy {
    Exact,
    TrackUpdates,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueScopeMapping {
    pub id: String,
    pub source_id: String,
    pub acknowledged_content_hash: String,
    pub update_policy: CatalogueScopeUpdatePolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedDefinitionMapping {
    pub id: String,
    pub operation: DefinitionOperation,
    pub catalogue_scope: Option<String>,
    pub mapping_classification: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ByteOrder {
    Little,
    Big,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BinaryPrimitive {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
    F64,
}

impl BinaryPrimitive {
    pub fn size(self) -> usize {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 | Self::F32 => 4,
            Self::U64 | Self::I64 | Self::F64 => 8,
        }
    }

    pub fn is_integer(self) -> bool {
        !matches!(self, Self::F32 | Self::F64)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BinaryRecordCount {
    Fixed {
        value: u32,
    },
    Field {
        offset: u64,
        primitive: BinaryPrimitive,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryMagicCheck {
    pub offset: u64,
    pub bytes_hex: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryField {
    pub host_slot: String,
    pub offset: u32,
    pub primitive: BinaryPrimitive,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_values: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryLayout {
    pub id: String,
    pub entry_name: String,
    pub byte_order: ByteOrder,
    pub base_offset: u64,
    pub record_count: BinaryRecordCount,
    pub stride: u32,
    pub max_records: u32,
    pub magic_checks: Vec<BinaryMagicCheck>,
    pub fields: Vec<BinaryField>,
}

#[derive(Clone, Debug)]
pub struct ResolvedCompatibilityProfile {
    document: CompatibilityProfileDocument,
    mappings: ResolvedMappings,
    resolved_hash: String,
    source: CompatibilityProfileSource,
    base: Option<BaseProfileReference>,
    local_definition_mapping_ids: HashSet<String>,
}

#[derive(Clone, Debug)]
struct ResolvedMappings {
    catalogue_scopes: Vec<CatalogueScopeMapping>,
    archive_entries: Vec<ArchiveEntryMapping>,
    stats_markers: Vec<StatsMarkerMapping>,
    stats_fields: Vec<StatsFieldMapping>,
    definition_directives: Vec<DefinitionDirectiveMapping>,
    binary_layouts: Vec<BinaryLayout>,
}

impl CompatibilityProfileDocument {
    pub fn parse(bytes: &[u8]) -> Result<Self, ObservatoryError> {
        if bytes.is_empty() || bytes.len() > MAX_PROFILE_BYTES {
            return Err(ObservatoryError::InvalidCompatibilityProfile(
                "profile_size",
            ));
        }
        let mut document = serde_json::from_slice::<Self>(bytes).map_err(|_| {
            ObservatoryError::InvalidCompatibilityProfile("schema_or_unknown_field")
        })?;
        if document.extends.is_some() && document.content_hash == ZERO_HASH {
            document.content_hash = document.calculated_content_hash()?;
        }
        document.validate()?;
        Ok(document)
    }

    pub fn canonical_json(&self) -> Result<String, ObservatoryError> {
        serde_json::to_string_pretty(self)
            .map_err(|_| ObservatoryError::InvalidCompatibilityProfile("serialization"))
    }

    pub fn starter_override(base: &ResolvedCompatibilityProfile) -> Self {
        Self {
            schema_version: 1,
            id: "local.republic-observatory.wrsr-override".to_owned(),
            version: "1.0.0".to_owned(),
            parser_engine_api_version: PARSER_ENGINE_API_VERSION.to_owned(),
            author: "Local player".to_owned(),
            description: "Local compatibility repairs for this installation.".to_owned(),
            targets: base.document.targets.clone(),
            content_hash: ZERO_HASH.to_owned(),
            extends: Some(BaseProfileReference {
                id: base.document.id.clone(),
                version: base.document.version.clone(),
                content_hash: base.document.content_hash.clone(),
            }),
            mappings: ProfileMappings::default(),
        }
    }

    fn validate(&self) -> Result<(), ObservatoryError> {
        if self.schema_version != 1 {
            return invalid("schema_version");
        }
        if !valid_reverse_domain_id(&self.id) {
            return invalid("profile_id");
        }
        if !valid_semantic_version(&self.version) {
            return invalid("profile_version");
        }
        if self.parser_engine_api_version != PARSER_ENGINE_API_VERSION {
            return invalid("parser_engine_api_version");
        }
        validate_text(&self.author, 1, 120, "author")?;
        validate_text(&self.description, 1, 500, "description")?;
        if self.targets.game_versions.len() > 32
            || self.targets.build_ids.len() > 64
            || self.targets.stats_formats.len() > 16
            || (self.targets.game_versions.is_empty()
                && self.targets.build_ids.is_empty()
                && self.targets.stats_formats.is_empty())
        {
            return invalid("targets");
        }
        ensure_unique(&self.targets.game_versions, "duplicate_target")?;
        ensure_unique(&self.targets.build_ids, "duplicate_target")?;
        ensure_unique(&self.targets.stats_formats, "duplicate_target")?;
        if self
            .targets
            .game_versions
            .iter()
            .any(|value| value.is_empty() || value.len() > 32 || contains_forbidden_text(value))
            || self.targets.build_ids.iter().any(|value| {
                value.is_empty() || value.len() > 20 || !value.bytes().all(|b| b.is_ascii_digit())
            })
        {
            return invalid("targets");
        }
        if let Some(base) = &self.extends
            && (!valid_reverse_domain_id(&base.id)
                || !valid_semantic_version(&base.version)
                || !is_lower_hex_hash(&base.content_hash))
        {
            return invalid("base_reference");
        }
        if !is_lower_hex_hash(&self.content_hash)
            || self.calculated_content_hash()? != self.content_hash
        {
            return invalid("content_hash");
        }
        validate_mappings(&self.mappings)?;
        Ok(())
    }

    fn calculated_content_hash(&self) -> Result<String, ObservatoryError> {
        let mut value = serde_json::to_value(self)
            .map_err(|_| ObservatoryError::InvalidCompatibilityProfile("serialization"))?;
        let Some(object) = value.as_object_mut() else {
            return invalid("serialization");
        };
        object.insert(
            "content_hash".to_owned(),
            Value::String(ZERO_HASH.to_owned()),
        );
        Ok(hex_hash(&canonical_json_bytes(&value)?))
    }
}

impl ResolvedCompatibilityProfile {
    pub fn reviewed_builtin() -> Result<Self, ObservatoryError> {
        let document = CompatibilityProfileDocument::parse(BUILTIN_PROFILE_BYTES)?;
        if document.id != BUILTIN_PROFILE_ID || document.extends.is_some() {
            return invalid("builtin_identity");
        }
        let mappings = complete_mappings(&document.mappings)?;
        validate_resolved_mappings(&mappings)?;
        Ok(Self {
            resolved_hash: document.content_hash.clone(),
            document,
            mappings,
            source: CompatibilityProfileSource::ReviewedBuiltin,
            base: None,
            local_definition_mapping_ids: HashSet::new(),
        })
    }

    pub fn legacy_reviewed_builtins() -> Result<Vec<Self>, ObservatoryError> {
        let mut document = CompatibilityProfileDocument::parse(BUILTIN_PROFILE_BYTES)?;
        document.version = LEGACY_BUILTIN_PROFILE_VERSION.to_owned();
        document.content_hash = LEGACY_BUILTIN_PROFILE_HASH.to_owned();
        if let Some(stats_fields) = document.mappings.stats_fields.as_mut() {
            stats_fields.retain(|mapping| !mapping.host_slot.starts_with("market."));
        }
        if document.calculated_content_hash()? != LEGACY_BUILTIN_PROFILE_HASH {
            return invalid("legacy_builtin_identity");
        }
        let mappings = complete_mappings(&document.mappings)?;
        validate_resolved_mappings(&mappings)?;
        Ok(vec![Self {
            resolved_hash: document.content_hash.clone(),
            document,
            mappings,
            source: CompatibilityProfileSource::ReviewedBuiltin,
            base: None,
            local_definition_mapping_ids: HashSet::new(),
        }])
    }

    pub fn resolve_override(
        base: &ResolvedCompatibilityProfile,
        override_document: CompatibilityProfileDocument,
    ) -> Result<Self, ObservatoryError> {
        let reference = override_document.extends.as_ref().ok_or(
            ObservatoryError::InvalidCompatibilityProfile("override_requires_base"),
        )?;
        if reference.id != base.document.id
            || reference.version != base.document.version
            || reference.content_hash != base.document.content_hash
        {
            return invalid("base_reference_mismatch");
        }
        let mut mappings = base.mappings.clone();
        let local_definition_mapping_ids = override_document
            .mappings
            .definition_directives
            .iter()
            .flatten()
            .map(|mapping| mapping.id.clone())
            .collect();
        replace_by_key(
            &mut mappings.catalogue_scopes,
            override_document.mappings.catalogue_scopes.as_deref(),
            |mapping| mapping.id.clone(),
        );
        replace_by_key(
            &mut mappings.archive_entries,
            override_document.mappings.archive_entries.as_deref(),
            |mapping| format!("{:?}", mapping.slot),
        );
        replace_by_key(
            &mut mappings.stats_markers,
            override_document.mappings.stats_markers.as_deref(),
            |mapping| format!("{:?}", mapping.slot),
        );
        replace_by_key(
            &mut mappings.stats_fields,
            override_document.mappings.stats_fields.as_deref(),
            |mapping| mapping.host_slot.clone(),
        );
        replace_by_key(
            &mut mappings.definition_directives,
            override_document.mappings.definition_directives.as_deref(),
            |mapping| mapping.id.clone(),
        );
        replace_by_key(
            &mut mappings.binary_layouts,
            override_document.mappings.binary_layouts.as_deref(),
            |mapping| mapping.id.clone(),
        );
        validate_resolved_mappings(&mappings)?;
        let resolved_hash = hex_hash(
            format!(
                "{}\0{}\0{}",
                base.resolved_hash, override_document.content_hash, PARSER_ENGINE_API_VERSION
            )
            .as_bytes(),
        );
        Ok(Self {
            base: override_document.extends.clone(),
            document: override_document,
            mappings,
            resolved_hash,
            source: CompatibilityProfileSource::LocalOverride,
            local_definition_mapping_ids,
        })
    }

    pub fn id(&self) -> &str {
        &self.document.id
    }

    pub fn version(&self) -> &str {
        &self.document.version
    }

    pub fn content_hash(&self) -> &str {
        &self.document.content_hash
    }

    pub fn resolved_hash(&self) -> &str {
        &self.resolved_hash
    }

    pub fn source(&self) -> CompatibilityProfileSource {
        self.source
    }

    pub fn base(&self) -> Option<&BaseProfileReference> {
        self.base.as_ref()
    }

    pub fn targets(&self) -> &ProfileTargets {
        &self.document.targets
    }

    pub fn stats_archive_aliases(&self) -> &[String] {
        self.mappings
            .archive_entries
            .iter()
            .find(|mapping| mapping.slot == ArchiveSlot::Stats)
            .map(|mapping| mapping.aliases.as_slice())
            .unwrap_or_default()
    }

    pub fn marker_for(&self, directive: &str) -> Option<&StatsMarkerMapping> {
        self.mappings
            .stats_markers
            .iter()
            .find(|mapping| mapping.aliases.iter().any(|alias| alias == directive))
    }

    pub fn field_for(&self, directive: &str, context: StatsContext) -> Option<&StatsFieldMapping> {
        self.mappings.stats_fields.iter().find(|mapping| {
            mapping.contexts.contains(&context)
                && mapping.aliases.iter().any(|alias| alias == directive)
        })
    }

    pub fn expected_snapshot_fields(&self, context: StatsContext) -> u32 {
        self.mappings
            .stats_fields
            .iter()
            .filter(|mapping| {
                mapping.contexts.contains(&context) && !mapping.host_slot.starts_with("market.")
            })
            .count()
            .min(u32::MAX as usize) as u32
    }

    pub fn definition_mapping(
        &self,
        source_id: &str,
        directive: &str,
    ) -> Result<Option<ResolvedDefinitionMapping>, ObservatoryError> {
        let mut candidates = Vec::new();
        for mapping in &self.mappings.definition_directives {
            let scoped = if let Some(scope_id) = &mapping.catalogue_scope {
                self.mappings
                    .catalogue_scopes
                    .iter()
                    .find(|scope| &scope.id == scope_id)
                    .is_some_and(|scope| scope.source_id == source_id)
            } else {
                true
            };
            if !scoped {
                continue;
            }
            let best_match = mapping
                .matches
                .iter()
                .filter(|candidate| match candidate.kind {
                    DirectiveMatchKind::Exact => directive == candidate.value,
                    DirectiveMatchKind::Prefix => directive.starts_with(&candidate.value),
                    DirectiveMatchKind::Contains => directive.contains(&candidate.value),
                })
                .map(|candidate| {
                    let match_rank = match candidate.kind {
                        DirectiveMatchKind::Exact => 3_u8,
                        DirectiveMatchKind::Prefix => 2,
                        DirectiveMatchKind::Contains => 1,
                    };
                    (
                        u8::from(mapping.catalogue_scope.is_some()),
                        match_rank,
                        candidate.value.len(),
                    )
                })
                .max();
            if let Some(rank) = best_match {
                candidates.push((rank, mapping));
            }
        }
        let Some(best_rank) = candidates.iter().map(|(rank, _)| *rank).max() else {
            return Ok(None);
        };
        let mut best = candidates
            .into_iter()
            .filter(|(rank, _)| *rank == best_rank)
            .map(|(_, mapping)| mapping);
        let selected = best.next().expect("best mapping candidate");
        if best.next().is_some() {
            return invalid("ambiguous_definition_mapping");
        }
        Ok(Some(ResolvedDefinitionMapping {
            id: selected.id.clone(),
            operation: selected.operation,
            catalogue_scope: selected.catalogue_scope.clone(),
            mapping_classification: if self.local_definition_mapping_ids.contains(&selected.id) {
                "player_mapped"
            } else {
                "reviewed_mapping"
            }
            .to_owned(),
        }))
    }

    pub fn catalogue_scopes(&self) -> &[CatalogueScopeMapping] {
        &self.mappings.catalogue_scopes
    }

    pub fn catalogue_scope_mapping_count(&self, scope_id: &str) -> u32 {
        self.mappings
            .definition_directives
            .iter()
            .filter(|mapping| mapping.catalogue_scope.as_deref() == Some(scope_id))
            .count()
            .min(u32::MAX as usize) as u32
    }

    pub fn binary_layouts(&self) -> &[BinaryLayout] {
        &self.mappings.binary_layouts
    }

    pub fn mapping_counts(&self) -> (u32, u32, u32, u32, u32) {
        (
            self.mappings.stats_markers.len().min(u32::MAX as usize) as u32,
            self.mappings.stats_fields.len().min(u32::MAX as usize) as u32,
            self.mappings
                .definition_directives
                .iter()
                .map(|mapping| mapping.operation)
                .collect::<HashSet<_>>()
                .len()
                .min(u32::MAX as usize) as u32,
            self.mappings.binary_layouts.len().min(u32::MAX as usize) as u32,
            self.mappings.catalogue_scopes.len().min(u32::MAX as usize) as u32,
        )
    }

    pub fn canonical_document_json(&self) -> Result<String, ObservatoryError> {
        self.document.canonical_json()
    }

    pub fn interpretation_id(&self, raw_payload_hash: &str) -> String {
        hex_hash(
            format!(
                "{raw_payload_hash}\0{PARSER_ENGINE_VERSION}\0{}",
                self.resolved_hash
            )
            .as_bytes(),
        )
    }

    pub fn provenance(&self) -> CompatibilityProvenance {
        CompatibilityProvenance {
            profile_id: self.id().to_owned(),
            profile_version: self.version().to_owned(),
            profile_content_hash: self.content_hash().to_owned(),
            resolved_profile_hash: self.resolved_hash().to_owned(),
            base_profile_hash: self.base().map(|reference| reference.content_hash.clone()),
            profile_source: match self.source() {
                CompatibilityProfileSource::ReviewedBuiltin => "reviewed_builtin",
                CompatibilityProfileSource::LocalOverride => "local_override",
            }
            .to_owned(),
            mapping_classification: self.source().evidence_classification().to_owned(),
            parser_engine_version: PARSER_ENGINE_VERSION.to_owned(),
        }
    }
}

fn complete_mappings(mappings: &ProfileMappings) -> Result<ResolvedMappings, ObservatoryError> {
    Ok(ResolvedMappings {
        catalogue_scopes: mappings.catalogue_scopes.clone().ok_or(
            ObservatoryError::InvalidCompatibilityProfile("missing_catalogue_scopes"),
        )?,
        archive_entries: mappings.archive_entries.clone().ok_or(
            ObservatoryError::InvalidCompatibilityProfile("missing_archive_entries"),
        )?,
        stats_markers: mappings.stats_markers.clone().ok_or(
            ObservatoryError::InvalidCompatibilityProfile("missing_stats_markers"),
        )?,
        stats_fields: mappings.stats_fields.clone().ok_or(
            ObservatoryError::InvalidCompatibilityProfile("missing_stats_fields"),
        )?,
        definition_directives: mappings.definition_directives.clone().ok_or(
            ObservatoryError::InvalidCompatibilityProfile("missing_definition_directives"),
        )?,
        binary_layouts: mappings.binary_layouts.clone().ok_or(
            ObservatoryError::InvalidCompatibilityProfile("missing_binary_layouts"),
        )?,
    })
}

fn validate_mappings(mappings: &ProfileMappings) -> Result<(), ObservatoryError> {
    if let Some(scopes) = &mappings.catalogue_scopes {
        if scopes.len() > 64 {
            return invalid("catalogue_scope_limit");
        }
        unique_keys(scopes.iter().map(|scope| scope.id.clone()))?;
        unique_keys(scopes.iter().map(|scope| scope.source_id.clone()))?;
        for scope in scopes {
            if !valid_mapping_id(&scope.id)
                || !valid_catalogue_source_id(&scope.source_id)
                || !is_lower_hex_hash(&scope.acknowledged_content_hash)
            {
                return invalid("catalogue_scope");
            }
        }
    }
    if let Some(entries) = &mappings.archive_entries {
        if entries.len() > 16 {
            return invalid("archive_entry_limit");
        }
        unique_keys(entries.iter().map(|entry| format!("{:?}", entry.slot)))?;
        for entry in entries {
            validate_aliases(&entry.aliases, false)?;
            for alias in &entry.aliases {
                validate_archive_entry_name(alias)?;
            }
        }
    }
    if let Some(markers) = &mappings.stats_markers {
        if markers.len() > 16 {
            return invalid("stats_marker_limit");
        }
        unique_keys(markers.iter().map(|marker| format!("{:?}", marker.slot)))?;
        for marker in markers {
            validate_aliases(&marker.aliases, true)?;
            if marker.slot == StatsMarkerSlot::Format {
                if marker.accepted_values.as_ref().is_none_or(Vec::is_empty) {
                    return invalid("format_values");
                }
            } else if marker.accepted_values.is_some() {
                return invalid("unexpected_marker_values");
            }
        }
    }
    if let Some(fields) = &mappings.stats_fields {
        if fields.len() > 128 {
            return invalid("stats_field_limit");
        }
        unique_keys(fields.iter().map(|field| field.host_slot.clone()))?;
        let mut context_aliases = HashSet::new();
        for field in fields {
            if !REQUIRED_STATS_SLOTS.contains(&field.host_slot.as_str())
                && !MARKET_STATS_SLOTS.contains(&field.host_slot.as_str())
            {
                return invalid("unknown_host_slot");
            }
            validate_aliases(&field.aliases, true)?;
            if field.contexts.is_empty() || field.contexts.len() > 3 {
                return invalid("stats_contexts");
            }
            ensure_unique(&field.contexts, "duplicate_context")?;
            for context in &field.contexts {
                if !slot_supports_context(&field.host_slot, *context) {
                    return invalid("slot_scope_restriction");
                }
                for alias in &field.aliases {
                    if !context_aliases.insert((*context, alias.to_ascii_uppercase())) {
                        return invalid("duplicate_mapping");
                    }
                }
            }
        }
    }
    if let Some(directives) = &mappings.definition_directives {
        if directives.len() > 128 {
            return invalid("definition_directive_limit");
        }
        unique_keys(directives.iter().map(|directive| directive.id.clone()))?;
        let mut matchers = HashSet::new();
        for directive in directives {
            if !valid_mapping_id(&directive.id)
                || directive
                    .catalogue_scope
                    .as_ref()
                    .is_some_and(|scope| !valid_mapping_id(scope))
            {
                return invalid("definition_mapping_id");
            }
            if directive.matches.is_empty() || directive.matches.len() > 16 {
                return invalid("definition_match_limit");
            }
            for matcher in &directive.matches {
                validate_directive_match(matcher)?;
                if !matchers.insert((
                    directive.catalogue_scope.clone(),
                    matcher.kind,
                    matcher.value.clone(),
                )) {
                    return invalid("duplicate_mapping");
                }
            }
        }
    }
    if let Some(layouts) = &mappings.binary_layouts {
        if layouts.len() > 32 {
            return invalid("binary_layout_limit");
        }
        unique_keys(layouts.iter().map(|layout| layout.id.clone()))?;
        for layout in layouts {
            validate_binary_layout(layout)?;
        }
    }
    Ok(())
}

fn validate_resolved_mappings(mappings: &ResolvedMappings) -> Result<(), ObservatoryError> {
    validate_mappings(&ProfileMappings {
        catalogue_scopes: Some(mappings.catalogue_scopes.clone()),
        archive_entries: Some(mappings.archive_entries.clone()),
        stats_markers: Some(mappings.stats_markers.clone()),
        stats_fields: Some(mappings.stats_fields.clone()),
        definition_directives: Some(mappings.definition_directives.clone()),
        binary_layouts: Some(mappings.binary_layouts.clone()),
    })?;
    let scope_ids = mappings
        .catalogue_scopes
        .iter()
        .map(|scope| scope.id.as_str())
        .collect::<HashSet<_>>();
    if mappings.definition_directives.iter().any(|mapping| {
        mapping
            .catalogue_scope
            .as_deref()
            .is_some_and(|scope| !scope_ids.contains(scope))
    }) {
        return invalid("unknown_catalogue_scope");
    }
    if scope_ids.iter().any(|scope| {
        !mappings
            .definition_directives
            .iter()
            .any(|mapping| mapping.catalogue_scope.as_deref() == Some(*scope))
    }) {
        return invalid("unused_catalogue_scope");
    }
    let archive_slots = mappings
        .archive_entries
        .iter()
        .map(|mapping| mapping.slot)
        .collect::<HashSet<_>>();
    if !archive_slots.contains(&ArchiveSlot::Stats) {
        return invalid("missing_stats_archive_alias");
    }
    let marker_slots = mappings
        .stats_markers
        .iter()
        .map(|mapping| mapping.slot)
        .collect::<HashSet<_>>();
    for required in [
        StatsMarkerSlot::Format,
        StatsMarkerSlot::HistoryRecord,
        StatsMarkerSlot::CurrentState,
        StatsMarkerSlot::City,
        StatsMarkerSlot::DateYear,
        StatsMarkerSlot::DateDay,
    ] {
        if !marker_slots.contains(&required) {
            return invalid("missing_stats_marker");
        }
    }
    let slots = mappings
        .stats_fields
        .iter()
        .map(|field| field.host_slot.as_str())
        .collect::<HashSet<_>>();
    if REQUIRED_STATS_SLOTS
        .iter()
        .any(|required| !slots.contains(required))
    {
        return invalid("missing_host_slot");
    }
    Ok(())
}

fn validate_binary_layout(layout: &BinaryLayout) -> Result<(), ObservatoryError> {
    if layout.id.len() < 3
        || layout.id.len() > 64
        || !layout
            .id
            .starts_with(|character: char| character.is_ascii_lowercase())
        || !layout.id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_.-".contains(&byte)
        })
    {
        return invalid("binary_layout_id");
    }
    validate_archive_entry_name(&layout.entry_name)?;
    if layout.stride == 0
        || layout.stride > 65_536
        || layout.max_records == 0
        || layout.max_records > 1_000_000
        || layout.base_offset > 1_073_741_824
        || layout.fields.is_empty()
        || layout.fields.len() > 64
        || layout.magic_checks.len() > 16
    {
        return invalid("binary_bounds");
    }
    match layout.record_count {
        BinaryRecordCount::Fixed { value } => {
            if value == 0 || value > layout.max_records {
                return invalid("binary_record_count");
            }
        }
        BinaryRecordCount::Field { offset, primitive } => {
            if offset > 1_073_741_824
                || !matches!(
                    primitive,
                    BinaryPrimitive::U8
                        | BinaryPrimitive::U16
                        | BinaryPrimitive::U32
                        | BinaryPrimitive::U64
                )
            {
                return invalid("binary_record_count");
            }
        }
    }
    for check in &layout.magic_checks {
        if check.offset > 1_073_741_824
            || check.bytes_hex.is_empty()
            || check.bytes_hex.len() > 128
            || check.bytes_hex.len() % 2 != 0
            || !check.bytes_hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return invalid("binary_magic");
        }
    }
    let mut slots = HashSet::new();
    for field in &layout.fields {
        if !REQUIRED_STATS_SLOTS.contains(&field.host_slot.as_str())
            || !slots.insert(field.host_slot.as_str())
            || field.offset as usize + field.primitive.size() > layout.stride as usize
            || field.mask.is_some() && !field.primitive.is_integer()
            || field
                .scale
                .is_some_and(|value| !value.is_finite() || value == 0.0)
            || field.missing_values.len() > 16
            || field.missing_values.iter().any(|value| !value.is_finite())
        {
            return invalid("binary_field");
        }
    }
    Ok(())
}

fn validate_aliases(aliases: &[String], directive: bool) -> Result<(), ObservatoryError> {
    if aliases.is_empty() || aliases.len() > 32 {
        return invalid("alias_limit");
    }
    ensure_unique(aliases, "duplicate_alias")?;
    for alias in aliases {
        if alias.is_empty()
            || alias.len() > 128
            || contains_forbidden_text(alias)
            || directive
                && (!alias.starts_with('$')
                    || !alias
                        .bytes()
                        .skip(1)
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
        {
            return invalid("alias");
        }
    }
    Ok(())
}

fn validate_directive_match(matcher: &DirectiveMatch) -> Result<(), ObservatoryError> {
    if matcher.value.len() < 2
        || matcher.value.len() > 96
        || contains_forbidden_text(&matcher.value)
        || !matcher.value.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'$'
        })
        || matches!(
            matcher.kind,
            DirectiveMatchKind::Exact | DirectiveMatchKind::Prefix
        ) && !matcher.value.starts_with('$')
    {
        return invalid("directive_match");
    }
    Ok(())
}

fn validate_archive_entry_name(name: &str) -> Result<(), ObservatoryError> {
    let normalized = name.replace('\\', "/");
    let path = Path::new(&normalized);
    if normalized.is_empty()
        || normalized.len() > 160
        || normalized.starts_with('/')
        || normalized.contains(':')
        || normalized.contains('*')
        || normalized.contains('?')
        || contains_forbidden_text(&normalized)
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return invalid("archive_entry_name");
    }
    Ok(())
}

fn slot_supports_context(slot: &str, context: StatsContext) -> bool {
    if MARKET_STATS_SLOTS.contains(&slot) {
        return true;
    }
    if context == StatsContext::History {
        return RECEIVER_METRICS.iter().any(|metric| metric.id == slot);
    }
    SNAPSHOT_FACTS
        .iter()
        .find(|fact| fact.id == slot)
        .is_some_and(|fact| match context {
            StatsContext::History => false,
            StatsContext::Republic => fact.republic,
            StatsContext::City => fact.city,
        })
}

fn replace_by_key<T: Clone>(
    base: &mut Vec<T>,
    replacements: Option<&[T]>,
    key: impl Fn(&T) -> String,
) {
    let Some(replacements) = replacements else {
        return;
    };
    let replacement_keys = replacements.iter().map(&key).collect::<HashSet<_>>();
    base.retain(|item| !replacement_keys.contains(&key(item)));
    base.extend_from_slice(replacements);
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, ObservatoryError> {
    fn canonical(value: &Value) -> Value {
        match value {
            Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
            Value::Object(values) => {
                let sorted = values
                    .iter()
                    .map(|(key, value)| (key.clone(), canonical(value)))
                    .collect::<BTreeMap<_, _>>();
                Value::Object(sorted.into_iter().collect())
            }
            _ => value.clone(),
        }
    }
    serde_json::to_vec(&canonical(value))
        .map_err(|_| ObservatoryError::InvalidCompatibilityProfile("serialization"))
}

fn valid_reverse_domain_id(value: &str) -> bool {
    value.len() >= 3
        && value.len() <= 96
        && value.contains('.')
        && value.split(['.', '-']).all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn valid_mapping_id(value: &str) -> bool {
    value.len() >= 3
        && value.len() <= 96
        && value.contains('.')
        && value.split(['.', '-', '_']).all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn valid_catalogue_source_id(value: &str) -> bool {
    let Some((kind, identity)) = value.split_once('.') else {
        return false;
    };
    matches!(kind, "workshop" | "wip")
        && !identity.is_empty()
        && identity.len() <= 96
        && identity.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'_' || byte == b'-'
        })
}

fn valid_semantic_version(value: &str) -> bool {
    let core = value.split_once('-').map_or(value, |(core, _)| core);
    let parts = core.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
}

fn validate_text(
    value: &str,
    minimum: usize,
    maximum: usize,
    reason: &'static str,
) -> Result<(), ObservatoryError> {
    if value.len() < minimum || value.len() > maximum || contains_forbidden_text(value) {
        return invalid(reason);
    }
    Ok(())
}

fn contains_forbidden_text(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.chars().any(char::is_control)
        || value.contains('<')
        || value.contains('>')
        || lower.contains("javascript:")
        || lower.contains("http://")
        || lower.contains("https://")
        || lower.contains("file://")
        || lower.contains("select ")
        || lower.contains("insert ")
        || lower.contains("update ")
        || lower.contains("delete ")
        || lower.contains("function(")
        || value.contains("=>")
        || value.as_bytes().windows(3).any(|window| {
            window[0].is_ascii_alphabetic()
                && window[1] == b':'
                && matches!(window[2], b'/' | b'\\')
        })
}

fn is_lower_hex_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn unique_keys(values: impl Iterator<Item = String>) -> Result<(), ObservatoryError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return invalid("duplicate_mapping");
        }
    }
    Ok(())
}

fn ensure_unique<T: Eq + std::hash::Hash>(
    values: &[T],
    reason: &'static str,
) -> Result<(), ObservatoryError> {
    let mut seen = HashSet::new();
    if values.iter().any(|value| !seen.insert(value)) {
        return invalid(reason);
    }
    Ok(())
}

fn invalid<T>(reason: &'static str) -> Result<T, ObservatoryError> {
    Err(ObservatoryError::InvalidCompatibilityProfile(reason))
}

fn hex_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hash = String::with_capacity(64);
    for byte in digest {
        write!(&mut hash, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{
        CatalogueScopeMapping, CatalogueScopeUpdatePolicy, CompatibilityProfileDocument,
        DefinitionDirectiveMapping, DefinitionOperation, DirectiveMatch, DirectiveMatchKind,
        ResolvedCompatibilityProfile, StatsContext, StatsFieldMapping,
    };
    use crate::error::ObservatoryError;

    #[test]
    fn reviewed_profile_is_strict_and_complete() {
        let profile = ResolvedCompatibilityProfile::reviewed_builtin().expect("reviewed profile");
        assert_eq!(profile.stats_archive_aliases(), ["stats.ini"]);
        assert_eq!(profile.mapping_counts(), (6, 50, 35, 0, 0));
    }

    #[test]
    fn legacy_reviewed_profile_remains_an_exact_inheritance_base() {
        let legacy = ResolvedCompatibilityProfile::legacy_reviewed_builtins()
            .expect("legacy profiles")
            .pop()
            .expect("legacy profile");
        assert_eq!(legacy.version(), "1.0.0");
        assert_eq!(
            legacy.content_hash(),
            "0f2737d29ddb50aa22a32d6fb1747e7c0ec5aa00227464a38d68b4ae1bac522e"
        );
        assert_eq!(legacy.mapping_counts(), (6, 18, 35, 0, 0));
        let local = CompatibilityProfileDocument::starter_override(&legacy);
        let resolved = ResolvedCompatibilityProfile::resolve_override(&legacy, local)
            .expect("legacy-based override");
        assert_eq!(
            resolved.base().expect("base").content_hash,
            legacy.content_hash()
        );
    }

    #[test]
    fn override_requires_the_exact_reviewed_base() {
        let base = ResolvedCompatibilityProfile::reviewed_builtin().expect("reviewed profile");
        let mut local = CompatibilityProfileDocument::starter_override(&base);
        local.extends.as_mut().expect("base").content_hash = "a".repeat(64);
        local.content_hash = local.calculated_content_hash().expect("hash");
        assert!(matches!(
            ResolvedCompatibilityProfile::resolve_override(&base, local),
            Err(ObservatoryError::InvalidCompatibilityProfile(
                "base_reference_mismatch"
            ))
        ));
    }

    #[test]
    fn unknown_fields_and_executable_payloads_fail_closed() {
        let base = ResolvedCompatibilityProfile::reviewed_builtin().expect("reviewed profile");
        let json = CompatibilityProfileDocument::starter_override(&base)
            .canonical_json()
            .expect("json");
        let unknown = json.replacen(
            "\"mappings\": {",
            "\"script\": \"alert(1)\",\n  \"mappings\": {",
            1,
        );
        assert!(CompatibilityProfileDocument::parse(unknown.as_bytes()).is_err());
    }

    #[test]
    fn local_alias_replacement_changes_only_the_declared_stable_slot() {
        let base = ResolvedCompatibilityProfile::reviewed_builtin().expect("reviewed profile");
        let mut local = CompatibilityProfileDocument::starter_override(&base);
        local.mappings.stats_fields = Some(vec![StatsFieldMapping {
            host_slot: "core.citizens.electronics.radio".to_owned(),
            aliases: vec!["$Citizens_Radio_NewBuild".to_owned()],
            contexts: vec![StatsContext::History, StatsContext::Republic],
        }]);
        local.content_hash = local.calculated_content_hash().expect("hash");
        local.validate().expect("valid replacement");
        let resolved = ResolvedCompatibilityProfile::resolve_override(&base, local)
            .expect("resolved replacement");
        assert!(
            resolved
                .field_for("$Citizens_Radio_NewBuild", StatsContext::History)
                .is_some()
        );
        assert!(
            resolved
                .field_for("$Citizens_EletrinicRadio", StatsContext::History)
                .is_none()
        );
        assert!(
            resolved
                .field_for("$Citizens_EletronicTV", StatsContext::History)
                .is_some()
        );
    }

    #[test]
    fn unknown_slots_duplicate_aliases_and_scope_expansion_fail_closed() {
        let base = ResolvedCompatibilityProfile::reviewed_builtin().expect("reviewed profile");
        for mapping in [
            StatsFieldMapping {
                host_slot: "private.parser.memory".to_owned(),
                aliases: vec!["$Private".to_owned()],
                contexts: vec![StatsContext::Republic],
            },
            StatsFieldMapping {
                host_slot: "source.stats.citizens.car_owners".to_owned(),
                aliases: vec!["$Citizens_Born".to_owned()],
                contexts: vec![StatsContext::City],
            },
        ] {
            let mut local = CompatibilityProfileDocument::starter_override(&base);
            local.mappings.stats_fields = Some(vec![mapping]);
            local.content_hash = local.calculated_content_hash().expect("hash");
            assert!(local.validate().is_err());
        }
    }

    #[test]
    fn markup_urls_sql_callbacks_and_absolute_paths_are_inertly_rejected() {
        let base = ResolvedCompatibilityProfile::reviewed_builtin().expect("reviewed profile");
        for payload in [
            "<script>alert(1)</script>",
            "https://example.invalid/profile",
            "select * from private_table",
            "value => callback(value)",
            "C:\\private\\mapping.json",
        ] {
            let mut local = CompatibilityProfileDocument::starter_override(&base);
            local.description = payload.to_owned();
            local.content_hash = local.calculated_content_hash().expect("hash");
            assert!(local.validate().is_err(), "accepted {payload}");
        }
    }

    #[test]
    fn scoped_mod_mapping_coexists_with_reviewed_operations_and_is_source_bounded() {
        let base = ResolvedCompatibilityProfile::reviewed_builtin().expect("reviewed profile");
        let mut local = CompatibilityProfileDocument::starter_override(&base);
        local.mappings.catalogue_scopes = Some(vec![CatalogueScopeMapping {
            id: "local.example.factory".to_owned(),
            source_id: "workshop.1234567890".to_owned(),
            acknowledged_content_hash: "a".repeat(64),
            update_policy: CatalogueScopeUpdatePolicy::Exact,
        }]);
        local.mappings.definition_directives = Some(vec![DefinitionDirectiveMapping {
            id: "local.example.factory.workers".to_owned(),
            operation: DefinitionOperation::BuildingWorkersRequired,
            matches: vec![DirectiveMatch {
                kind: DirectiveMatchKind::Exact,
                value: "$MOD_WORKERS".to_owned(),
            }],
            catalogue_scope: Some("local.example.factory".to_owned()),
        }]);
        local.content_hash = local.calculated_content_hash().expect("hash");
        let resolved = ResolvedCompatibilityProfile::resolve_override(&base, local)
            .expect("resolved scoped mapping");

        let scoped = resolved
            .definition_mapping("workshop.1234567890", "$MOD_WORKERS")
            .expect("mapping")
            .expect("scoped mapping");
        assert_eq!(scoped.id, "local.example.factory.workers");
        assert_eq!(scoped.mapping_classification, "player_mapped");
        assert!(
            resolved
                .definition_mapping("wip.1234567890", "$MOD_WORKERS")
                .expect("mapping")
                .is_none()
        );
        assert_eq!(
            resolved
                .definition_mapping("base.buildings", "$WORKERS_NEEDED")
                .expect("mapping")
                .expect("reviewed mapping")
                .mapping_classification,
            "reviewed_mapping"
        );
    }

    #[test]
    fn scoped_mapping_precedes_global_and_runtime_ambiguity_fails_closed() {
        let base = ResolvedCompatibilityProfile::reviewed_builtin().expect("reviewed profile");
        let mut local = CompatibilityProfileDocument::starter_override(&base);
        local.mappings.catalogue_scopes = Some(vec![CatalogueScopeMapping {
            id: "local.example.remap".to_owned(),
            source_id: "workshop.42".to_owned(),
            acknowledged_content_hash: "b".repeat(64),
            update_policy: CatalogueScopeUpdatePolicy::TrackUpdates,
        }]);
        local.mappings.definition_directives = Some(vec![
            DefinitionDirectiveMapping {
                id: "local.example.remap.type".to_owned(),
                operation: DefinitionOperation::DefinitionDisplayName,
                matches: vec![DirectiveMatch {
                    kind: DirectiveMatchKind::Exact,
                    value: "$TYPE_FACTORY".to_owned(),
                }],
                catalogue_scope: Some("local.example.remap".to_owned()),
            },
            DefinitionDirectiveMapping {
                id: "local.example.remap.repair".to_owned(),
                operation: DefinitionOperation::BuildingStyle,
                matches: vec![DirectiveMatch {
                    kind: DirectiveMatchKind::Contains,
                    value: "REPAIR".to_owned(),
                }],
                catalogue_scope: Some("local.example.remap".to_owned()),
            },
            DefinitionDirectiveMapping {
                id: "local.example.remap.update".to_owned(),
                operation: DefinitionOperation::BuildingType,
                matches: vec![DirectiveMatch {
                    kind: DirectiveMatchKind::Contains,
                    value: "UPDATE".to_owned(),
                }],
                catalogue_scope: Some("local.example.remap".to_owned()),
            },
        ]);
        local.content_hash = local.calculated_content_hash().expect("hash");
        let resolved = ResolvedCompatibilityProfile::resolve_override(&base, local)
            .expect("resolved scoped mapping");
        assert_eq!(
            resolved
                .definition_mapping("workshop.42", "$TYPE_FACTORY")
                .expect("mapping")
                .expect("scoped mapping")
                .operation,
            DefinitionOperation::DefinitionDisplayName
        );
        assert!(matches!(
            resolved.definition_mapping("workshop.42", "$REPAIR_UPDATE"),
            Err(ObservatoryError::InvalidCompatibilityProfile(
                "ambiguous_definition_mapping"
            ))
        ));
    }

    #[test]
    fn unknown_scopes_and_unsafe_source_identities_fail_closed() {
        let base = ResolvedCompatibilityProfile::reviewed_builtin().expect("reviewed profile");
        let mut unknown = CompatibilityProfileDocument::starter_override(&base);
        unknown.mappings.definition_directives = Some(vec![DefinitionDirectiveMapping {
            id: "local.example.unknown".to_owned(),
            operation: DefinitionOperation::BuildingStyle,
            matches: vec![DirectiveMatch {
                kind: DirectiveMatchKind::Exact,
                value: "$MOD_STYLE".to_owned(),
            }],
            catalogue_scope: Some("local.example.missing".to_owned()),
        }]);
        unknown.content_hash = unknown.calculated_content_hash().expect("hash");
        assert!(matches!(
            ResolvedCompatibilityProfile::resolve_override(&base, unknown),
            Err(ObservatoryError::InvalidCompatibilityProfile(
                "unknown_catalogue_scope"
            ))
        ));

        let mut unsafe_source = CompatibilityProfileDocument::starter_override(&base);
        unsafe_source.mappings.catalogue_scopes = Some(vec![CatalogueScopeMapping {
            id: "local.example.unsafe".to_owned(),
            source_id: "workshop.C:\\private".to_owned(),
            acknowledged_content_hash: "c".repeat(64),
            update_policy: CatalogueScopeUpdatePolicy::Exact,
        }]);
        unsafe_source.content_hash = unsafe_source.calculated_content_hash().expect("hash");
        assert!(unsafe_source.validate().is_err());
    }
}
