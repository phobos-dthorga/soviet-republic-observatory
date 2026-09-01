use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Mutex, TryLockError};
use std::time::{Duration, Instant, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::analysis_pack::{
    AnalysisPackContribution, AnalysisPackDocument, AnalysisPackInspection, AnalysisPackSummary,
};
use crate::automatic_observer::{AutomaticObserver, latest_save_candidate_path};
use crate::bulk_work::BulkWorkCoordinator;
use crate::compatibility_runtime::CompatibilityRuntime;
use crate::definition_catalogue::{
    CatalogueDiscoveryPhase, catalogue_watch_roots, discover_catalogue_with_reuse_and_progress,
};
use crate::diagnostics;
use crate::error::ObservatoryError;
use crate::game_vocabulary::{discover_game_vocabularies, resolve_game_media_directory};
use crate::language_pack::{
    LanguagePackInspection, LanguageStatus, LegacyLanguageHandover, inspect_community_manifest,
};
use crate::model::{
    AnalysisContextResult, ApplicationPreferences, ApplicationPreferencesDraft,
    ApplicationSettingsView, ArchiveComparison, ArchiveOverview, AutomaticObservationUpdate,
    BackgroundWorkPriority, CataloguePage, CatalogueRefreshPhase, CatalogueRefreshProgress,
    CatalogueRefreshTrigger, CatalogueSearchFilter, CatalogueStatus, CompatibilityStatus,
    CompatibilityUpdate, ConfiguredDirectorySummary, DefinitionDossier, DirectoryKind,
    ImportOutcome, MaintenanceDiagnostics, MarketBasketDraft, MarketIndexingPhase,
    MarketIndexingProgress, MarketPriceSeries, MarketScenarioDraft, MarketWorkspace,
    ObservationImportResult, OverlayInspection, OverlayProfileSummary, PopulationDataset,
    ProductionPathwayModel, ProductionPathwayRequest, ProductionRouteCoverage,
    ProductionRouteModel, ProductionRouteRequest, ReceiverDataset, RecorderDiscoverySource,
    RecorderHealth, RecorderUpdate, ReinterpretationPhase, ReinterpretationProgress, RepublicBrief,
    RepublicPlanBrief, RepublicPlanDraft, RepublicPlanWorkspace, SetupState, WarehouseSnapshot,
};
use crate::planning_overlay::PlanningOverlayDocument;
use crate::save_archive::{hash_save_stats_payload, inspect_save_archive};
use crate::storage::{ObservatoryStorage, now_ms};
use crate::tesmio_probe;
use crate::theme::{ThemeInspection, ThemeStatus, inspect_theme_document};
use crate::warehouse::AnalyticalWarehouse;

const SAVE_DIRECTORY_KEY: &str = "save_directory";
const GAME_MEDIA_DIRECTORY_KEY: &str = "game_media_directory";
const WORKSHOP_DIRECTORY_KEY: &str = "workshop_directory";
const AUTOMATIC_OBSERVATION_KEY: &str = "automatic_observation_enabled";

fn background_priority_code(priority: BackgroundWorkPriority) -> u8 {
    match priority {
        BackgroundWorkPriority::Gentle => 0,
        BackgroundWorkPriority::Balanced => 1,
        BackgroundWorkPriority::FinishSooner => 2,
    }
}

fn background_priority_from_code(code: u8) -> BackgroundWorkPriority {
    match code {
        1 => BackgroundWorkPriority::Balanced,
        2 => BackgroundWorkPriority::FinishSooner,
        _ => BackgroundWorkPriority::Gentle,
    }
}

fn catalogue_trigger_name(trigger: CatalogueRefreshTrigger) -> &'static str {
    match trigger {
        CatalogueRefreshTrigger::Startup => "startup",
        CatalogueRefreshTrigger::Filesystem => "filesystem change",
        CatalogueRefreshTrigger::Manual => "manual request",
    }
}

fn ratio_percent(
    completed: impl Into<u64>,
    total: impl Into<u64>,
    range_start: u8,
    range_end: u8,
) -> Option<u8> {
    let completed = completed.into();
    let total = total.into();
    if total == 0 {
        return None;
    }
    let span = u64::from(range_end.saturating_sub(range_start));
    Some(
        u64::from(range_start)
            .saturating_add(completed.min(total).saturating_mul(span) / total)
            .min(u64::from(range_end)) as u8,
    )
}

fn market_index_job_id(
    source_directory_identity: &str,
    resolved_profile_hash: &str,
    candidates: &[crate::model::MarketIndexCandidate],
) -> String {
    let mut payload_hashes = candidates
        .iter()
        .map(|candidate| candidate.raw_payload_hash.as_str())
        .collect::<Vec<_>>();
    payload_hashes.sort_unstable();
    payload_hashes.dedup();
    let mut hasher = Sha256::new();
    hasher.update(b"republic-observatory-market-index-v1\0");
    hasher.update(source_directory_identity.as_bytes());
    hasher.update(b"\0");
    hasher.update(resolved_profile_hash.as_bytes());
    for payload_hash in payload_hashes {
        hasher.update(b"\0");
        hasher.update(payload_hash.as_bytes());
    }
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("market-index-{}", &suffix[..24])
}

#[derive(Debug)]
pub struct ObservatoryApplication {
    storage: ObservatoryStorage,
    warehouse: AnalyticalWarehouse,
    automatic_observer: Mutex<AutomaticObserver>,
    compatibility: CompatibilityRuntime,
    reinterpretation: Mutex<()>,
    reinterpretation_progress: Mutex<ReinterpretationProgress>,
    market_indexing_progress: Mutex<MarketIndexingProgress>,
    bulk_work: BulkWorkCoordinator,
    storage_patience_seconds: AtomicU64,
    background_work_priority: AtomicU8,
    catalogue_refresh: Mutex<()>,
    catalogue_progress: Mutex<CatalogueRefreshProgress>,
}

impl ObservatoryApplication {
    pub fn initialise(
        database_path: PathBuf,
        warehouse_path: PathBuf,
    ) -> Result<Self, ObservatoryError> {
        let data_directory = database_path
            .parent()
            .ok_or(ObservatoryError::InvalidDirectory)?;
        let compatibility = CompatibilityRuntime::initialise(data_directory)?;
        let storage = ObservatoryStorage::initialise(database_path)?;
        storage.record_compatibility_runtime(&compatibility.active()?, &compatibility.status()?)?;
        let preferences = storage.load_application_preferences()?;
        let market_indexing_progress = storage.latest_market_index_progress()?;
        let warehouse = match AnalyticalWarehouse::initialise(warehouse_path.clone()) {
            Ok(warehouse) => warehouse,
            Err(_) => {
                diagnostics::record(
                    "error",
                    "warehouse.startup_unavailable",
                    "application_startup",
                    "The analytical warehouse could not be opened; save recording remains available.",
                );
                AnalyticalWarehouse::unavailable(warehouse_path)
            }
        };
        let automatic_observation_enabled = preferences.automatic_observation_enabled;
        let directory_configured = storage
            .get_setting(SAVE_DIRECTORY_KEY)?
            .is_some_and(|path| path.is_dir());
        let mut automatic_observer = AutomaticObserver::new(automatic_observation_enabled);
        automatic_observer.set_enabled(automatic_observation_enabled, directory_configured);
        Ok(Self {
            storage,
            warehouse,
            automatic_observer: Mutex::new(automatic_observer),
            compatibility,
            reinterpretation: Mutex::new(()),
            reinterpretation_progress: Mutex::new(ReinterpretationProgress::default()),
            market_indexing_progress: Mutex::new(market_indexing_progress),
            bulk_work: BulkWorkCoordinator::default(),
            storage_patience_seconds: AtomicU64::new(u64::from(
                preferences.effective_storage_patience_seconds,
            )),
            background_work_priority: AtomicU8::new(background_priority_code(
                preferences.background_work_priority,
            )),
            catalogue_refresh: Mutex::new(()),
            catalogue_progress: Mutex::new(CatalogueRefreshProgress::default()),
        })
    }

    pub fn setup_state(&self) -> Result<SetupState, ObservatoryError> {
        let save_path = self.storage.get_setting(SAVE_DIRECTORY_KEY)?;
        let game_path = self.storage.get_setting(GAME_MEDIA_DIRECTORY_KEY)?;
        let workshop_path = self.storage.get_setting(WORKSHOP_DIRECTORY_KEY)?;
        let save_directory = save_path
            .as_deref()
            .filter(|path| path.is_dir())
            .map(|path| ConfiguredDirectorySummary {
                name: directory_display_name(path, false),
            });
        let game_directory = game_path
            .as_deref()
            .filter(|path| path.is_dir())
            .map(|path| ConfiguredDirectorySummary {
                name: directory_display_name(path, true),
            });
        let workshop_directory =
            workshop_path
                .as_deref()
                .filter(|path| path.is_dir())
                .map(|path| ConfiguredDirectorySummary {
                    name: directory_display_name(path, false),
                });
        let save_candidates = save_path
            .as_deref()
            .filter(|path| path.is_dir())
            .map(count_save_candidates)
            .transpose()?
            .unwrap_or_default();
        let game_vocabularies = game_path
            .as_deref()
            .filter(|path| path.is_dir())
            .map(discover_game_vocabularies)
            .transpose()?
            .unwrap_or_default();

        Ok(SetupState {
            save_directory,
            game_directory,
            workshop_directory,
            save_candidates,
            observed_saves: self.storage.file_observation_count()?,
            distinct_states: self.storage.distinct_state_count()?,
            game_vocabularies,
            automatic_observer: self.observer_status()?,
            compatibility: self.compatibility_status()?,
        })
    }

    pub fn application_settings(&self) -> Result<ApplicationSettingsView, ObservatoryError> {
        self.application_settings_view(self.storage.load_application_preferences()?)
    }

    pub fn update_application_preferences(
        &self,
        draft: &ApplicationPreferencesDraft,
    ) -> Result<ApplicationSettingsView, ObservatoryError> {
        let preferences = self.storage.save_application_preferences(draft)?;
        self.apply_observation_preference(&preferences)?;
        diagnostics::record(
            "info",
            "settings.updated",
            "application_settings",
            "Application preferences were validated and saved locally.",
        );
        self.application_settings_view(preferences)
    }

    pub fn reset_application_preferences(
        &self,
    ) -> Result<ApplicationSettingsView, ObservatoryError> {
        let preferences = self.storage.reset_application_preferences()?;
        self.apply_observation_preference(&preferences)?;
        diagnostics::record(
            "info",
            "settings.defaults_restored",
            "application_settings",
            "Interface and background-work preferences were restored to safe defaults.",
        );
        self.application_settings_view(preferences)
    }

    fn application_settings_view(
        &self,
        preferences: ApplicationPreferences,
    ) -> Result<ApplicationSettingsView, ObservatoryError> {
        let (cached_market_records, cached_market_fact_rows, market_interpretation_memberships) =
            self.storage.market_cache_totals()?;
        let progress = self.market_indexing_progress()?;
        Ok(ApplicationSettingsView {
            preferences,
            setup: self.setup_state()?,
            maintenance: MaintenanceDiagnostics {
                market_storage_contract_version: crate::storage::MARKET_STORAGE_CONTRACT_VERSION,
                cached_market_records,
                cached_market_fact_rows,
                market_interpretation_memberships,
                latest_indexing_phase: progress.phase,
                latest_cache_records_reused: progress.cache_records_reused,
                latest_cache_rows_avoided: progress.cache_rows_avoided,
                latest_contention_retries: progress.contention_retries,
                latest_contention_wait_ms: progress.contention_wait_ms,
                latest_resume_count: progress.resume_count,
            },
        })
    }

    fn apply_observation_preference(
        &self,
        preferences: &ApplicationPreferences,
    ) -> Result<(), ObservatoryError> {
        self.storage_patience_seconds.store(
            u64::from(preferences.effective_storage_patience_seconds),
            Ordering::Release,
        );
        self.background_work_priority.store(
            background_priority_code(preferences.background_work_priority),
            Ordering::Release,
        );
        let directory_configured = self
            .storage
            .get_setting(SAVE_DIRECTORY_KEY)?
            .is_some_and(|path| path.is_dir());
        self.automatic_observer
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?
            .set_enabled(
                preferences.automatic_observation_enabled,
                directory_configured,
            );
        Ok(())
    }

    pub fn latest_receiver_dataset(&self) -> Result<Option<ReceiverDataset>, ObservatoryError> {
        self.storage.load_latest_dataset()
    }

    pub fn population_dataset(&self) -> Result<PopulationDataset, ObservatoryError> {
        let mut dataset = self.storage.load_population_dataset()?;
        dataset.analysis_context.catalogue_generation_id = self
            .warehouse
            .catalogue_generation_if_ready()
            .map(|generation| generation.generation_id);
        let game_media_directory = self.storage.get_setting(GAME_MEDIA_DIRECTORY_KEY)?;
        dataset.tesmio_probe = tesmio_probe::inspect(game_media_directory.as_deref());
        Ok(dataset)
    }

    pub fn republic_brief(&self) -> Result<RepublicBrief, ObservatoryError> {
        let population = self.population_dataset()?;
        let recorder = self.recorder_health().ok();
        let catalogue = self.catalogue_status().ok();
        let plan = self
            .storage
            .republic_plan_workspace(&population)?
            .active_plan
            .map(|evaluation| RepublicPlanBrief {
                plan_id: evaluation.revision.plan_id,
                name: evaluation.revision.name,
                revision: evaluation.revision.revision,
                target_count: evaluation.targets.len().min(u32::MAX as usize) as u32,
                end_year: evaluation.revision.end_year,
                end_day: evaluation.revision.end_day,
                state: evaluation.state,
                attainment_basis_points: evaluation.attainment_basis_points,
                guardrail_breach_count: evaluation.guardrail_breach_count,
            });
        Ok(crate::republic_brief::build_republic_brief(
            &population,
            recorder.as_ref(),
            catalogue.as_ref(),
            plan,
        ))
    }

    pub fn republic_plan_workspace(&self) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        let population = self.population_dataset()?;
        self.storage.republic_plan_workspace(&population)
    }

    pub fn market_workspace(&self) -> Result<MarketWorkspace, ObservatoryError> {
        let context = self.archive_overview()?.analysis_context;
        let mut evidence = self.storage.market_evidence(context)?;
        let mut warehouse_history_available = false;
        if let Some(sqlite_projection) = evidence.projection.as_ref()
            && let Ok(Some(warehouse_projection)) =
                self.warehouse.market_projection(sqlite_projection)
        {
            evidence.projection = Some(warehouse_projection);
            warehouse_history_available = true;
        }
        Ok(crate::market::build_workspace(
            evidence,
            warehouse_history_available,
        ))
    }

    pub fn market_price_series(
        &self,
        currency: &str,
        resource_token: &str,
    ) -> Result<MarketPriceSeries, ObservatoryError> {
        if !matches!(currency, "rub" | "usd")
            || resource_token.is_empty()
            || resource_token.len() > 128
            || resource_token.chars().any(char::is_control)
        {
            return Err(ObservatoryError::InvalidMarketDefinition(
                "invalid_price_series_request",
            ));
        }
        let workspace = self.market_workspace()?;
        let context = workspace
            .metric_contexts
            .iter()
            .find(|context| context.metric_id == format!("market.price.{currency}"))
            .cloned()
            .ok_or(ObservatoryError::InvalidMarketDefinition(
                "price_series_unavailable",
            ))?;
        let Some(interpretation_id) = workspace.analysis_context.head_interpretation_id.as_deref()
        else {
            return Ok(MarketPriceSeries {
                available: false,
                currency: currency.to_owned(),
                resource_token: resource_token.to_owned(),
                points: Vec::new(),
                context,
                limitation: Some("selected_head_unavailable".to_owned()),
            });
        };
        let points =
            match self
                .warehouse
                .market_price_series(interpretation_id, currency, resource_token)
            {
                Ok(Some(points)) => points,
                Ok(None) | Err(ObservatoryError::WarehouseUnavailable) => {
                    return Ok(MarketPriceSeries {
                        available: false,
                        currency: currency.to_owned(),
                        resource_token: resource_token.to_owned(),
                        points: Vec::new(),
                        context,
                        limitation: Some("warehouse_history_unavailable".to_owned()),
                    });
                }
                Err(error) => return Err(error),
            };
        Ok(MarketPriceSeries {
            available: !points.is_empty(),
            currency: currency.to_owned(),
            resource_token: resource_token.to_owned(),
            points,
            context,
            limitation: None,
        })
    }

    pub fn save_market_basket(
        &self,
        draft: &MarketBasketDraft,
    ) -> Result<MarketWorkspace, ObservatoryError> {
        self.storage.save_market_basket(draft)?;
        self.market_workspace()
    }

    pub fn save_market_scenario(
        &self,
        draft: &MarketScenarioDraft,
    ) -> Result<MarketWorkspace, ObservatoryError> {
        self.storage.save_market_scenario(draft)?;
        self.market_workspace()
    }

    pub fn select_market_definition(
        &self,
        kind: &str,
        definition_id: &str,
        revision: u32,
    ) -> Result<MarketWorkspace, ObservatoryError> {
        self.storage
            .select_market_definition(kind, definition_id, revision)?;
        self.market_workspace()
    }

    pub fn rollback_market_definition(
        &self,
        kind: &str,
        definition_id: &str,
    ) -> Result<MarketWorkspace, ObservatoryError> {
        self.storage
            .rollback_market_definition(kind, definition_id)?;
        self.market_workspace()
    }

    pub fn clear_market_selection(&self, kind: &str) -> Result<MarketWorkspace, ObservatoryError> {
        self.storage.clear_market_selection(kind)?;
        self.market_workspace()
    }

    pub fn remove_market_definition(
        &self,
        kind: &str,
        definition_id: &str,
    ) -> Result<MarketWorkspace, ObservatoryError> {
        self.storage.remove_market_definition(kind, definition_id)?;
        self.market_workspace()
    }

    pub fn save_republic_plan(
        &self,
        draft: &RepublicPlanDraft,
    ) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        let population = self.population_dataset()?;
        self.storage.save_republic_plan(draft, &population)
    }

    pub fn activate_republic_plan(
        &self,
        plan_id: &str,
        revision: Option<u32>,
    ) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        let population = self.population_dataset()?;
        self.storage
            .activate_republic_plan(plan_id, revision, &population)
    }

    pub fn rollback_republic_plan(
        &self,
        plan_id: &str,
    ) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        let population = self.population_dataset()?;
        self.storage.rollback_republic_plan(plan_id, &population)
    }

    pub fn remove_republic_plan(
        &self,
        plan_id: &str,
    ) -> Result<RepublicPlanWorkspace, ObservatoryError> {
        let population = self.population_dataset()?;
        self.storage.remove_republic_plan(plan_id, &population)
    }

    pub fn language_status(&self) -> Result<LanguageStatus, ObservatoryError> {
        self.storage.language_status()
    }

    pub fn theme_status(&self) -> Result<ThemeStatus, ObservatoryError> {
        self.storage.theme_status()
    }

    pub fn inspect_theme(&self, document: &str) -> ThemeInspection {
        inspect_theme_document(document)
    }

    pub fn import_theme(&self, document: &str) -> Result<ThemeStatus, ObservatoryError> {
        self.storage.import_theme(document)
    }

    pub fn select_theme(
        &self,
        theme_id: &str,
        version: &str,
        content_hash: &str,
    ) -> Result<ThemeStatus, ObservatoryError> {
        self.storage.select_theme(theme_id, version, content_hash)
    }

    pub fn export_theme(
        &self,
        theme_id: &str,
        version: &str,
        content_hash: &str,
    ) -> Result<String, ObservatoryError> {
        self.storage.export_theme(theme_id, version, content_hash)
    }

    pub fn remove_theme(
        &self,
        theme_id: &str,
        version: &str,
    ) -> Result<ThemeStatus, ObservatoryError> {
        self.storage.remove_theme(theme_id, version)
    }

    pub fn inspect_language_pack(&self, json: &str) -> LanguagePackInspection {
        inspect_community_manifest(json)
    }

    pub fn install_language_pack(&self, json: &str) -> Result<LanguageStatus, ObservatoryError> {
        self.storage.install_language_pack(json)
    }

    pub fn select_language_pack(&self, pack_id: &str) -> Result<LanguageStatus, ObservatoryError> {
        self.storage.select_language_pack(pack_id)
    }

    pub fn remove_language_pack(&self, pack_id: &str) -> Result<LanguageStatus, ObservatoryError> {
        self.storage.remove_language_pack(pack_id)
    }

    pub fn export_language_pack(&self, pack_id: &str) -> Result<String, ObservatoryError> {
        self.storage.export_language_pack(pack_id)
    }

    pub fn handover_legacy_language_packs(
        &self,
        handover: &LegacyLanguageHandover,
    ) -> Result<LanguageStatus, ObservatoryError> {
        self.storage.handover_legacy_language_packs(handover)
    }

    pub fn compatibility_status(&self) -> Result<CompatibilityStatus, ObservatoryError> {
        let mut status = self.compatibility.status()?;
        if let Some(generation) = self.warehouse.catalogue_generation_if_ready() {
            status.detected_build_id = generation.game_build_id;
            if generation.compatibility_profile_hash == status.active.resolved_hash
                && status.catalogue_scopes.iter().all(|scope| {
                    scope.state == crate::model::CompatibilityCatalogueScopeState::Dormant
                })
                && let Some(scopes) = self.warehouse.catalogue_scope_statuses_if_ready()
                && !scopes.is_empty()
            {
                status.catalogue_scopes = scopes;
            }
        }
        Ok(status)
    }

    pub fn create_compatibility_override(&self) -> Result<CompatibilityUpdate, ObservatoryError> {
        let mut update = self.compatibility.create_starter_override()?;
        update.status = self.compatibility_status()?;
        self.storage
            .record_compatibility_runtime(&self.compatibility.active()?, &update.status)?;
        Ok(update)
    }

    pub fn reload_compatibility(&self) -> Result<CompatibilityUpdate, ObservatoryError> {
        let mut update = self.compatibility.reload()?;
        update.status = self.compatibility_status()?;
        self.storage
            .record_compatibility_runtime(&self.compatibility.active()?, &update.status)?;
        Ok(update)
    }

    pub fn reinterpretation_progress(&self) -> Result<ReinterpretationProgress, ObservatoryError> {
        self.reinterpretation_progress
            .lock()
            .map(|progress| progress.clone())
            .map_err(|_| ObservatoryError::StorageUnavailable)
    }

    pub fn reinterpret_latest_save(
        &self,
        mut notify: impl FnMut(ReinterpretationProgress),
    ) -> Result<ObservationImportResult, ObservatoryError> {
        let _guard = match self.reinterpretation.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return Err(ObservatoryError::CriticalTaskBusy),
            Err(TryLockError::Poisoned(_)) => return Err(ObservatoryError::StorageUnavailable),
        };
        let started_at_ms = now_ms();
        diagnostics::record(
            "info",
            "compatibility.reinterpret_started",
            "save_reinterpretation",
            "Save reinterpretation started with the active compatibility profile.",
        );
        let directory = self
            .storage
            .get_setting(SAVE_DIRECTORY_KEY)?
            .ok_or(ObservatoryError::SaveDirectoryNotConfigured)?;
        let path = latest_save_candidate_path(&directory)?;
        let current_file = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("save.zip")
            .to_owned();
        let mut progress = ReinterpretationProgress {
            phase: ReinterpretationPhase::Reading,
            progress_percent: Some(10),
            started_at_ms: Some(started_at_ms),
            updated_at_ms: Some(started_at_ms),
            current_file: Some(current_file),
            interpretation_id: None,
            error_code: None,
        };
        self.report_reinterpretation_progress(&progress, &mut notify);
        let result: Result<ObservationImportResult, ObservatoryError> = (|| {
            progress.phase = ReinterpretationPhase::Parsing;
            progress.progress_percent = Some(40);
            progress.updated_at_ms = Some(now_ms());
            self.report_reinterpretation_progress(&progress, &mut notify);
            let profile = self.compatibility.active()?;
            let inspection = inspect_save_archive(&path, &profile)?;
            progress.phase = ReinterpretationPhase::Persisting;
            progress.progress_percent = Some(75);
            progress.updated_at_ms = Some(now_ms());
            progress.interpretation_id = Some(inspection.interpretation_id.clone());
            self.report_reinterpretation_progress(&progress, &mut notify);
            let inserted = self.run_coordinated_background_storage(|| {
                self.storage.save_reinterpretation(&inspection)
            })?;
            progress.phase = ReinterpretationPhase::QueueingWarehouse;
            progress.progress_percent = Some(92);
            progress.updated_at_ms = Some(now_ms());
            self.report_reinterpretation_progress(&progress, &mut notify);
            let dataset = self
                .storage
                .load_context_dataset()?
                .ok_or(ObservatoryError::UnknownObservation)?;
            let active_context_id = self
                .storage
                .load_archive_overview()?
                .analysis_context
                .context_id;
            Ok(ObservationImportResult {
                outcome: if inserted {
                    ImportOutcome::Imported
                } else {
                    ImportOutcome::Duplicate
                },
                recorded_interpretation_id: inspection.interpretation_id,
                active_context_id,
                dataset,
            })
        })();
        match result {
            Ok(result) => {
                progress.phase = ReinterpretationPhase::Complete;
                progress.progress_percent = Some(100);
                progress.updated_at_ms = Some(now_ms());
                self.report_reinterpretation_progress(&progress, &mut notify);
                diagnostics::record(
                    "info",
                    "compatibility.reinterpret_complete",
                    "save_reinterpretation",
                    "Save reinterpretation completed and its analytical projection was queued.",
                );
                Ok(result)
            }
            Err(error) => {
                progress.phase = ReinterpretationPhase::Failed;
                progress.progress_percent = None;
                progress.updated_at_ms = Some(now_ms());
                progress.error_code = Some(error.code().to_owned());
                self.report_reinterpretation_progress(&progress, &mut notify);
                diagnostics::record(
                    "error",
                    "compatibility.reinterpret_failed",
                    "save_reinterpretation",
                    "Save reinterpretation stopped; earlier observations remain available.",
                );
                Err(error)
            }
        }
    }

    fn report_reinterpretation_progress(
        &self,
        progress: &ReinterpretationProgress,
        notify: &mut impl FnMut(ReinterpretationProgress),
    ) {
        if let Ok(mut current) = self.reinterpretation_progress.lock() {
            *current = progress.clone();
        }
        notify(progress.clone());
    }

    pub fn market_indexing_progress(&self) -> Result<MarketIndexingProgress, ObservatoryError> {
        self.market_indexing_progress
            .lock()
            .map(|progress| progress.clone())
            .map_err(|_| ObservatoryError::StorageUnavailable)
    }

    pub fn recover_market_indexing(&self) -> Result<(), ObservatoryError> {
        self.storage.verify_and_repair_known_contracts()?;
        self.storage.enqueue_warehouse_rebuild()?;
        diagnostics::record(
            "info",
            "markets.recovery_queued",
            "market_indexing_recovery",
            "Known storage contracts were verified and retained analytical evidence was queued for reprojection.",
        );
        Ok(())
    }

    fn background_priority(&self) -> BackgroundWorkPriority {
        background_priority_from_code(self.background_work_priority.load(Ordering::Acquire))
    }

    fn run_background_storage_step<T>(
        &self,
        progress: &mut MarketIndexingProgress,
        mut operation: impl FnMut() -> Result<T, ObservatoryError>,
    ) -> Result<T, ObservatoryError> {
        let started = Instant::now();
        let patience = Duration::from_secs(
            self.storage_patience_seconds
                .load(Ordering::Acquire)
                .clamp(5, 300),
        );
        loop {
            let priority = self.background_priority();
            let remaining = patience.saturating_sub(started.elapsed());
            let (lease, contention) = self.bulk_work.background_write(remaining, priority)?;
            progress.contention_retries = progress
                .contention_retries
                .saturating_add(contention.retries);
            progress.contention_wait_ms = progress
                .contention_wait_ms
                .saturating_add(contention.waited_ms);
            let attempted_at = Instant::now();
            let result = operation();
            drop(lease);
            match result {
                Ok(value) => return Ok(value),
                Err(ObservatoryError::StorageBusy) if started.elapsed() < patience => {
                    progress.contention_retries = progress.contention_retries.saturating_add(1);
                    progress.contention_wait_ms = progress.contention_wait_ms.saturating_add(
                        attempted_at.elapsed().as_millis().min(u64::MAX as u128) as u64,
                    );
                    self.bulk_work.cooperative_yield(priority);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn run_coordinated_background_storage<T>(
        &self,
        operation: impl FnMut() -> Result<T, ObservatoryError>,
    ) -> Result<T, ObservatoryError> {
        let mut progress = MarketIndexingProgress::default();
        self.run_background_storage_step(&mut progress, operation)
    }

    pub fn index_available_saves_for_markets(
        &self,
        mut notify: impl FnMut(MarketIndexingProgress),
    ) -> Result<MarketIndexingProgress, ObservatoryError> {
        self.run_market_indexing(false, &mut notify)
    }

    pub fn refresh_changed_market_data(
        &self,
        mut notify: impl FnMut(MarketIndexingProgress),
    ) -> Result<MarketIndexingProgress, ObservatoryError> {
        self.run_market_indexing(true, &mut notify)
    }

    fn run_market_indexing(
        &self,
        refresh_all: bool,
        notify: &mut impl FnMut(MarketIndexingProgress),
    ) -> Result<MarketIndexingProgress, ObservatoryError> {
        let _guard = match self.reinterpretation.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return Err(ObservatoryError::CriticalTaskBusy),
            Err(TryLockError::Poisoned(_)) => return Err(ObservatoryError::StorageUnavailable),
        };
        let result: Result<MarketIndexingProgress, ObservatoryError> = (|| {
            let started_at_ms = now_ms();
            let directory = self
                .storage
                .get_setting(SAVE_DIRECTORY_KEY)?
                .ok_or(ObservatoryError::SaveDirectoryNotConfigured)?;
            let source_directory_identity = crate::save_archive::directory_identity(&directory)?;
            let mut progress = MarketIndexingProgress {
                job_id: None,
                storage_contract_version: crate::storage::MARKET_STORAGE_CONTRACT_VERSION,
                phase: MarketIndexingPhase::Discovering,
                started_at_ms: Some(started_at_ms),
                updated_at_ms: Some(started_at_ms),
                ..MarketIndexingProgress::default()
            };
            self.report_market_indexing_progress(&progress, notify);
            let candidates = self.run_background_storage_step(&mut progress, || {
                self.storage
                    .market_index_candidates(&source_directory_identity)
            })?;
            let profile = self.compatibility.active()?;
            let job_id = market_index_job_id(
                &source_directory_identity,
                profile.resolved_hash(),
                &candidates,
            );
            progress.job_id = Some(job_id.clone());
            progress.total_archives = candidates.len().min(u32::MAX as usize) as u32;
            progress.phase = MarketIndexingPhase::Matching;
            progress.progress_percent = Some(2);
            progress.updated_at_ms = Some(now_ms());
            let durable = self.run_background_storage_step(&mut progress, || {
                self.storage
                    .start_market_index_job(&job_id, &candidates, refresh_all)
            })?;
            progress.completed_archives = durable.completed_archives;
            progress.missing_archives = durable.missing_archives;
            progress.changed_archives = durable.changed_archives;
            progress.failed_archives = durable.failed_archives;
            progress.duplicate_archives = durable.duplicate_archives;
            progress.resume_count = durable.resume_count;
            if progress.resume_count > 0 {
                diagnostics::record(
                    "info",
                    "markets.index_resumed",
                    "market_indexing",
                    "Markets maintenance resumed from durable archive checkpoints under the background-work coordinator.",
                );
            }
            let item_states = self.run_background_storage_step(&mut progress, || {
                self.storage.market_index_item_states(&job_id)
            })?;
            self.report_market_indexing_progress(&progress, notify);

            for (index, candidate) in candidates.iter().enumerate() {
                if item_states
                    .get(&candidate.payload_hash)
                    .is_some_and(|state| {
                        matches!(
                            state.as_str(),
                            "complete" | "missing" | "changed" | "duplicate"
                        )
                    })
                {
                    continue;
                }
                progress.current_archive = (index + 1).min(u32::MAX as usize) as u32;
                progress.current_file = Some(candidate.source_file_name.clone());
                progress.records_processed = 0;
                progress.rows_processed = 0;
                progress.phase = MarketIndexingPhase::ReadingArchive;
                progress.progress_percent =
                    ratio_percent(index as u64, candidates.len() as u64, 3, 94);
                progress.updated_at_ms = Some(now_ms());
                self.report_market_indexing_progress(&progress, notify);

                let relative = Path::new(&candidate.source_file_name);
                let path = directory.join(relative);
                if relative.components().count() != 1 || !path.is_file() {
                    progress.missing_archives += 1;
                    self.run_background_storage_step(&mut progress, || {
                        self.storage.update_market_index_item(
                            &job_id,
                            &candidate.payload_hash,
                            "missing",
                            0,
                            0,
                            Some("archive_missing"),
                        )
                    })?;
                    self.bulk_work.cooperative_yield(self.background_priority());
                    continue;
                }
                let metadata = fs::metadata(&path).map_err(|_| ObservatoryError::InvalidArchive)?;
                let modified_ms = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
                    .unwrap_or_default();
                if metadata.len() != candidate.source_file_size
                    || modified_ms != candidate.source_modified_ms
                    || candidate.source_directory_identity != source_directory_identity
                {
                    progress.changed_archives += 1;
                    self.run_background_storage_step(&mut progress, || {
                        self.storage.update_market_index_item(
                            &job_id,
                            &candidate.payload_hash,
                            "changed",
                            0,
                            0,
                            Some("archive_changed"),
                        )
                    })?;
                    self.bulk_work.cooperative_yield(self.background_priority());
                    continue;
                }

                let cached_counts = self.run_background_storage_step(&mut progress, || {
                    self.storage.cached_market_variant_counts(
                        &candidate.raw_payload_hash,
                        profile.resolved_hash(),
                    )
                })?;
                if let Some((records, rows)) = cached_counts {
                    match hash_save_stats_payload(&path, &profile) {
                        Ok(payload_hash) if payload_hash == candidate.raw_payload_hash => {
                            progress.records_processed = records;
                            progress.rows_processed = rows;
                            progress.duplicate_archives =
                                progress.duplicate_archives.saturating_add(1);
                            progress.cache_records_reused =
                                progress.cache_records_reused.saturating_add(records);
                            progress.cache_rows_avoided =
                                progress.cache_rows_avoided.saturating_add(u64::from(rows));
                            self.run_background_storage_step(&mut progress, || {
                                self.storage.update_market_index_item(
                                    &job_id,
                                    &candidate.payload_hash,
                                    "duplicate",
                                    records,
                                    rows,
                                    None,
                                )
                            })?;
                        }
                        Ok(_) => {
                            progress.changed_archives = progress.changed_archives.saturating_add(1);
                            self.run_background_storage_step(&mut progress, || {
                                self.storage.update_market_index_item(
                                    &job_id,
                                    &candidate.payload_hash,
                                    "changed",
                                    0,
                                    0,
                                    Some("raw_payload_mismatch"),
                                )
                            })?;
                        }
                        Err(error) => {
                            progress.failed_archives = progress.failed_archives.saturating_add(1);
                            self.run_background_storage_step(&mut progress, || {
                                self.storage.update_market_index_item(
                                    &job_id,
                                    &candidate.payload_hash,
                                    "failed",
                                    0,
                                    0,
                                    Some(error.code()),
                                )
                            })?;
                        }
                    }
                    self.bulk_work.cooperative_yield(self.background_priority());
                    continue;
                }

                progress.phase = MarketIndexingPhase::ParsingRecords;
                progress.updated_at_ms = Some(now_ms());
                self.report_market_indexing_progress(&progress, notify);
                match inspect_save_archive(&path, &profile) {
                    Ok(inspection) if inspection.payload_hash == candidate.raw_payload_hash => {
                        progress.records_processed = inspection.market.records.len() as u32;
                        progress.rows_processed = inspection.market.row_count;
                        progress.phase = MarketIndexingPhase::Persisting;
                        progress.updated_at_ms = Some(now_ms());
                        self.report_market_indexing_progress(&progress, notify);
                        let already_indexed =
                            self.run_background_storage_step(&mut progress, || {
                                self.storage
                                    .market_coverage_exists(&inspection.interpretation_id)
                            })?;
                        let (_, cache) = self.run_background_storage_step(&mut progress, || {
                            self.storage
                                .save_reinterpretation_with_market_stats(&inspection)
                        })?;
                        progress.cache_records_reused = progress
                            .cache_records_reused
                            .saturating_add(cache.records_reused);
                        progress.cache_rows_avoided = progress
                            .cache_rows_avoided
                            .saturating_add(cache.rows_avoided);
                        if already_indexed {
                            progress.duplicate_archives += 1;
                        } else {
                            progress.completed_archives += 1;
                        }
                        let records_processed = progress.records_processed;
                        let rows_processed = progress.rows_processed;
                        self.run_background_storage_step(&mut progress, || {
                            self.storage.update_market_index_item(
                                &job_id,
                                &candidate.payload_hash,
                                if already_indexed {
                                    "duplicate"
                                } else {
                                    "complete"
                                },
                                records_processed,
                                rows_processed,
                                None,
                            )
                        })?;
                    }
                    Ok(_) => {
                        progress.changed_archives += 1;
                        self.run_background_storage_step(&mut progress, || {
                            self.storage.update_market_index_item(
                                &job_id,
                                &candidate.payload_hash,
                                "changed",
                                0,
                                0,
                                Some("raw_payload_mismatch"),
                            )
                        })?;
                    }
                    Err(error) => {
                        progress.failed_archives += 1;
                        self.run_background_storage_step(&mut progress, || {
                            self.storage.update_market_index_item(
                                &job_id,
                                &candidate.payload_hash,
                                "failed",
                                0,
                                0,
                                Some(error.code()),
                            )
                        })?;
                    }
                }
                self.bulk_work.cooperative_yield(self.background_priority());
            }

            progress.phase = MarketIndexingPhase::QueueingWarehouse;
            progress.progress_percent = Some(97);
            progress.current_file = None;
            progress.updated_at_ms = Some(now_ms());
            self.report_market_indexing_progress(&progress, notify);
            progress.phase = MarketIndexingPhase::Complete;
            progress.progress_percent = Some(100);
            progress.updated_at_ms = Some(now_ms());
            let finish_snapshot = progress.clone();
            self.run_background_storage_step(&mut progress, || {
                self.storage.finish_market_index_job(&finish_snapshot)
            })?;
            self.report_market_indexing_progress(&progress, notify);
            diagnostics::record(
                "info",
                "markets.index_complete",
                "market_indexing",
                &format!(
                    "Markets maintenance completed without changing the analytical head; {} cached records and {} cached rows were reused after {} contention retries and {} ms waiting.",
                    progress.cache_records_reused,
                    progress.cache_rows_avoided,
                    progress.contention_retries,
                    progress.contention_wait_ms,
                ),
            );
            Ok(progress)
        })();
        match result {
            Ok(progress) => Ok(progress),
            Err(ObservatoryError::StorageBusy) => {
                let mut paused = self.market_indexing_progress().unwrap_or_default();
                paused.phase = MarketIndexingPhase::Paused;
                paused.updated_at_ms = Some(now_ms());
                paused.error_code = Some("storage_occupied".to_owned());
                self.report_market_indexing_progress(&paused, notify);
                if paused.job_id.is_some() {
                    let _ = self.storage.finish_market_index_job(&paused);
                }
                diagnostics::record(
                    "warning",
                    "markets.index_paused_storage",
                    "market_indexing",
                    &format!(
                        "Markets indexing paused after {} contention retries and {} ms waiting; completed archive checkpoints remain reusable.",
                        paused.contention_retries, paused.contention_wait_ms
                    ),
                );
                Ok(paused)
            }
            Err(error) => {
                let mut failed = self.market_indexing_progress().unwrap_or_default();
                failed.phase = MarketIndexingPhase::Failed;
                failed.progress_percent = None;
                failed.updated_at_ms = Some(now_ms());
                failed.error_code = Some(error.code().to_owned());
                self.report_market_indexing_progress(&failed, notify);
                if failed.job_id.is_some() {
                    let _ = self.storage.finish_market_index_job(&failed);
                }
                Err(error)
            }
        }
    }

    fn report_market_indexing_progress(
        &self,
        progress: &MarketIndexingProgress,
        notify: &mut impl FnMut(MarketIndexingProgress),
    ) {
        if let Ok(mut current) = self.market_indexing_progress.lock() {
            *current = progress.clone();
        }
        notify(progress.clone());
    }

    pub fn inspect_analysis_pack(&self, json: &str) -> AnalysisPackInspection {
        match AnalysisPackDocument::parse(json.as_bytes())
            .and_then(|document| document.inspection())
        {
            Ok(inspection) => inspection,
            Err(error) => AnalysisPackInspection {
                valid: false,
                code: Some(
                    error
                        .analysis_pack_reason()
                        .unwrap_or(error.code())
                        .to_owned(),
                ),
                pack_id: None,
                name: None,
                author: None,
                version: None,
                host_api_version: None,
                default_locale: None,
                description: None,
                content_hash: None,
                consumed_metrics: Vec::new(),
                derived_metrics: Vec::new(),
                charts: Vec::new(),
            },
        }
    }

    pub fn import_analysis_pack(
        &self,
        json: &str,
    ) -> Result<AnalysisPackSummary, ObservatoryError> {
        let document = AnalysisPackDocument::parse(json.as_bytes())?;
        self.storage.install_analysis_pack(&document)
    }

    pub fn export_analysis_pack(
        &self,
        pack_id: &str,
        revision: u32,
    ) -> Result<String, ObservatoryError> {
        self.storage
            .analysis_pack_document(pack_id, revision)?
            .canonical_json()
    }

    pub fn analysis_packs(&self) -> Result<Vec<AnalysisPackSummary>, ObservatoryError> {
        self.storage.list_analysis_packs()
    }

    pub fn enable_analysis_pack(
        &self,
        pack_id: &str,
        revision: Option<u32>,
    ) -> Result<AnalysisPackSummary, ObservatoryError> {
        self.storage.enable_analysis_pack(pack_id, revision)
    }

    pub fn disable_analysis_pack(
        &self,
        pack_id: &str,
    ) -> Result<AnalysisPackSummary, ObservatoryError> {
        self.storage.disable_analysis_pack(pack_id)
    }

    pub fn rollback_analysis_pack(
        &self,
        pack_id: &str,
    ) -> Result<AnalysisPackSummary, ObservatoryError> {
        self.storage.rollback_analysis_pack(pack_id)
    }

    pub fn remove_analysis_pack(&self, pack_id: &str) -> Result<(), ObservatoryError> {
        self.storage.remove_analysis_pack(pack_id)
    }

    pub fn analysis_pack_contributions(
        &self,
    ) -> Result<Vec<AnalysisPackContribution>, ObservatoryError> {
        let Some(dataset) = self.storage.load_latest_dataset()? else {
            return Ok(Vec::new());
        };
        let installed = self.storage.enabled_analysis_pack_revisions()?;
        let mut contributions = Vec::with_capacity(installed.len());
        for revision in installed {
            match AnalysisPackDocument::parse(revision.document_json.as_bytes()) {
                Ok(document) => {
                    contributions.push(document.resolve(&revision.content_hash, &dataset));
                }
                Err(error) => diagnostics::record(
                    "error",
                    "analysis_pack.evaluation_failed",
                    "analysis_pack_evaluation",
                    &format!(
                        "Analysis Pack {} revision {} was isolated after validation failed: {}",
                        revision.pack_id, revision.revision, error
                    ),
                ),
            }
        }
        Ok(contributions)
    }

    pub fn archive_overview(&self) -> Result<ArchiveOverview, ObservatoryError> {
        let mut archive = self.storage.load_archive_overview()?;
        archive.analysis_context.catalogue_generation_id = self
            .warehouse
            .catalogue_generation_if_ready()
            .map(|generation| generation.generation_id);
        Ok(archive)
    }

    pub fn configure_directory(
        &self,
        kind: DirectoryKind,
        path: String,
    ) -> Result<SetupState, ObservatoryError> {
        let selected = PathBuf::from(path);
        match kind {
            DirectoryKind::Save => {
                let canonical = selected
                    .canonicalize()
                    .map_err(|_| ObservatoryError::InvalidDirectory)?;
                if !canonical.is_dir() {
                    return Err(ObservatoryError::InvalidDirectory);
                }
                self.storage.set_setting(SAVE_DIRECTORY_KEY, &canonical)?;
                let mut observer = self
                    .automatic_observer
                    .lock()
                    .map_err(|_| ObservatoryError::StorageUnavailable)?;
                let enabled = observer.status().enabled;
                observer.set_enabled(enabled, true);
            }
            DirectoryKind::Game => {
                let media = resolve_game_media_directory(&selected)?;
                self.storage.set_setting(GAME_MEDIA_DIRECTORY_KEY, &media)?;
            }
            DirectoryKind::Workshop => {
                let canonical = selected
                    .canonicalize()
                    .map_err(|_| ObservatoryError::InvalidDirectory)?;
                if !canonical.is_dir() {
                    return Err(ObservatoryError::InvalidDirectory);
                }
                self.storage
                    .set_setting(WORKSHOP_DIRECTORY_KEY, &canonical)?;
            }
        }
        self.setup_state()
    }

    pub(crate) fn configured_directory_path(
        &self,
        kind: DirectoryKind,
    ) -> Result<Option<PathBuf>, ObservatoryError> {
        let key = match kind {
            DirectoryKind::Save => SAVE_DIRECTORY_KEY,
            DirectoryKind::Game => GAME_MEDIA_DIRECTORY_KEY,
            DirectoryKind::Workshop => WORKSHOP_DIRECTORY_KEY,
        };
        self.storage.get_setting(key)
    }

    pub fn observe_latest_save(&self) -> Result<ObservationImportResult, ObservatoryError> {
        let directory = self
            .storage
            .get_setting(SAVE_DIRECTORY_KEY)?
            .ok_or(ObservatoryError::SaveDirectoryNotConfigured)?;
        let path = latest_save_candidate_path(&directory)?;
        let profile = self.compatibility.active()?;
        let inspection = inspect_save_archive(&path, &profile)?;
        let _recorder_lease = self.bulk_work.recorder_write()?;
        let inserted = self.storage.save_inspection(&inspection)?;
        let dataset = self
            .storage
            .load_context_dataset()?
            .ok_or(ObservatoryError::UnknownObservation)?;
        self.automatic_observer
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?
            .record_observation(&inspection.source_file_name, now_ms());
        Ok(ObservationImportResult {
            outcome: if inserted {
                ImportOutcome::Imported
            } else {
                ImportOutcome::Duplicate
            },
            recorded_interpretation_id: inspection.interpretation_id,
            active_context_id: self
                .storage
                .load_archive_overview()?
                .analysis_context
                .context_id,
            dataset,
        })
    }

    pub fn select_branch(
        &self,
        branch_id: &str,
    ) -> Result<AnalysisContextResult, ObservatoryError> {
        self.storage.select_branch(branch_id)?;
        self.analysis_context_result()
    }

    pub fn inspect_archive_observation(
        &self,
        interpretation_id: &str,
    ) -> Result<AnalysisContextResult, ObservatoryError> {
        self.storage.inspect_observation(interpretation_id)?;
        self.analysis_context_result()
    }

    pub fn return_to_branch_tip(&self) -> Result<AnalysisContextResult, ObservatoryError> {
        self.storage.return_to_branch_tip()?;
        self.analysis_context_result()
    }

    pub fn create_continuation(
        &self,
        interpretation_id: &str,
        label: Option<&str>,
    ) -> Result<AnalysisContextResult, ObservatoryError> {
        self.storage.create_continuation(interpretation_id, label)?;
        self.analysis_context_result()
    }

    pub fn set_branch_label(
        &self,
        branch_id: &str,
        label: Option<&str>,
    ) -> Result<AnalysisContextResult, ObservatoryError> {
        self.storage.set_branch_label(branch_id, label)?;
        self.analysis_context_result()
    }

    fn analysis_context_result(&self) -> Result<AnalysisContextResult, ObservatoryError> {
        let archive = self.archive_overview()?;
        Ok(AnalysisContextResult {
            context: archive.analysis_context.clone(),
            archive,
            dataset: self.storage.load_context_dataset()?,
        })
    }

    pub fn set_automatic_observation(&self, enabled: bool) -> Result<SetupState, ObservatoryError> {
        self.storage
            .set_bool_setting(AUTOMATIC_OBSERVATION_KEY, enabled)?;
        let directory_configured = self
            .storage
            .get_setting(SAVE_DIRECTORY_KEY)?
            .is_some_and(|path| path.is_dir());
        self.automatic_observer
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?
            .set_enabled(enabled, directory_configured);
        self.setup_state()
    }

    pub(crate) fn poll_automatic_observation_from(
        &self,
        discovery_source: RecorderDiscoverySource,
    ) -> Result<AutomaticObservationUpdate, ObservatoryError> {
        let directory = self.storage.get_setting(SAVE_DIRECTORY_KEY)?;
        let _recorder_lease = self.bulk_work.recorder_write()?;
        self.automatic_observer
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?
            .poll_with_profile(
                &self.storage,
                directory.as_deref(),
                now_ms(),
                discovery_source,
                &self.compatibility.active()?,
            )
    }

    pub fn recorder_health(&self) -> Result<RecorderHealth, ObservatoryError> {
        self.storage.load_recorder_health(self.observer_status()?)
    }

    pub(crate) fn recorder_update(
        &self,
        import_result: Option<ObservationImportResult>,
    ) -> Result<RecorderUpdate, ObservatoryError> {
        Ok(RecorderUpdate {
            health: self.recorder_health()?,
            import_result,
        })
    }

    pub(crate) fn recorder_configuration(
        &self,
    ) -> Result<(bool, Option<PathBuf>), ObservatoryError> {
        Ok((
            self.observer_status()?.enabled,
            self.storage.get_setting(SAVE_DIRECTORY_KEY)?,
        ))
    }

    pub(crate) fn note_recorder_filesystem_event(
        &self,
        event_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.storage.note_recorder_filesystem_event(event_at_ms)
    }

    pub fn compare_observations(
        &self,
        from_payload_hash: &str,
        to_payload_hash: &str,
    ) -> Result<ArchiveComparison, ObservatoryError> {
        self.storage
            .compare_observations(from_payload_hash, to_payload_hash)
    }

    pub fn catalogue_status(&self) -> Result<CatalogueStatus, ObservatoryError> {
        let queue = self.storage.projection_queue_status()?;
        let rebuilding = self.storage.warehouse_rebuild_running()?;
        let (last_checked_at_ms, last_refreshed_at_ms, warehouse_error) =
            self.warehouse.catalogue_runtime_if_ready().unwrap_or((
                None,
                None,
                (!self.warehouse.is_available()).then(|| "warehouse_unavailable".to_owned()),
            ));
        let (last_filesystem_event_ms, sqlite_error) = self.storage.catalogue_runtime_state()?;
        let mut active_overlay = self.storage.active_overlay_summary()?;
        if let Some(profile) = &mut active_overlay
            && let Some(revision) = profile.active_revision
        {
            profile.conflict_count = self
                .warehouse
                .overlay_conflict_count(&profile.profile_id, revision);
        }
        Ok(CatalogueStatus {
            warehouse: self.warehouse.health_snapshot(
                queue.pending_jobs,
                queue.failed_jobs,
                queue
                    .oldest_unresolved_at_ms
                    .map(|requested_at| now_ms().saturating_sub(requested_at)),
                rebuilding,
            ),
            generation: self.warehouse.catalogue_generation_if_ready(),
            last_checked_at_ms,
            last_refreshed_at_ms,
            last_filesystem_event_ms,
            error_code: sqlite_error.or(warehouse_error),
            active_overlay,
            refresh: self
                .catalogue_progress
                .lock()
                .map(|progress| progress.clone())
                .unwrap_or_default(),
        })
    }

    pub fn refresh_catalogue(
        &self,
        trigger: CatalogueRefreshTrigger,
        mut notify: impl FnMut(CatalogueRefreshProgress),
    ) -> Result<CatalogueStatus, ObservatoryError> {
        let _guard = match self.catalogue_refresh.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return self.catalogue_status(),
            Err(TryLockError::Poisoned(_)) => return Err(ObservatoryError::StorageUnavailable),
        };
        let requested_at = now_ms();
        let timer = Instant::now();
        let mut progress = CatalogueRefreshProgress {
            phase: CatalogueRefreshPhase::Discovering,
            trigger,
            progress_percent: None,
            started_at_ms: Some(requested_at),
            updated_at_ms: Some(requested_at),
            ..CatalogueRefreshProgress::default()
        };
        self.report_catalogue_progress(&progress, &mut notify);
        diagnostics::record(
            "info",
            "catalogue.refresh_started",
            "refresh_catalogue",
            &format!(
                "Definition refresh started ({}).",
                catalogue_trigger_name(trigger)
            ),
        );
        let result: Result<(), ObservatoryError> = (|| {
            self.storage.note_catalogue_refresh_request(requested_at)?;
            let game_directory = self
                .storage
                .get_setting(GAME_MEDIA_DIRECTORY_KEY)?
                .filter(|path| path.is_dir())
                .ok_or(ObservatoryError::InvalidGameDirectory)?;
            let workshop_directory = self
                .storage
                .get_setting(WORKSHOP_DIRECTORY_KEY)?
                .filter(|path| path.is_dir());
            let reuse = self.warehouse.catalogue_reuse_cache()?;
            let compatibility = self.compatibility.active()?;
            let generation = discover_catalogue_with_reuse_and_progress(
                &game_directory,
                workshop_directory.as_deref(),
                requested_at,
                &reuse,
                &compatibility,
                |discovery| {
                    progress.phase = match discovery.phase {
                        CatalogueDiscoveryPhase::Discovering => CatalogueRefreshPhase::Discovering,
                        CatalogueDiscoveryPhase::Scanning | CatalogueDiscoveryPhase::Complete => {
                            CatalogueRefreshPhase::Scanning
                        }
                    };
                    progress.progress_percent = match discovery.phase {
                        CatalogueDiscoveryPhase::Discovering => ratio_percent(
                            discovery.sources_discovered,
                            discovery.sources_total,
                            0,
                            10,
                        ),
                        CatalogueDiscoveryPhase::Scanning | CatalogueDiscoveryPhase::Complete => {
                            ratio_percent(
                                discovery.files_processed,
                                discovery.files_discovered,
                                10,
                                55,
                            )
                        }
                    };
                    progress.updated_at_ms = Some(now_ms());
                    progress.current_source = discovery.current_source;
                    progress.current_file = discovery.current_file;
                    progress.current_file_index = discovery.current_file_index;
                    progress.sources_discovered = discovery.sources_discovered;
                    progress.sources_total = discovery.sources_total;
                    progress.files_discovered = discovery.files_discovered;
                    progress.files_processed = discovery.files_processed;
                    progress.files_reused = discovery.files_reused;
                    progress.files_parsed = discovery.files_parsed;
                    progress.entities_prepared = discovery.entities_prepared;
                    self.report_catalogue_progress(&progress, &mut notify);
                },
            )?;
            self.compatibility
                .record_catalogue_scopes(generation.compatibility_scopes.clone())?;
            if generation.compatibility_scopes.iter().any(|scope| {
                scope.state == crate::model::CompatibilityCatalogueScopeState::Conflict
            }) {
                diagnostics::record(
                    "warning",
                    "compatibility.catalogue_scope_conflict",
                    "refresh_catalogue",
                    "An exact mod compatibility scope no longer matches its acknowledged definition content; the previous catalogue generation remains active.",
                );
                return Err(ObservatoryError::CatalogueCompatibilityConflict);
            }
            if generation.compatibility_scopes.iter().any(|scope| {
                scope.state == crate::model::CompatibilityCatalogueScopeState::UpdatedUnreviewed
            }) {
                diagnostics::record(
                    "warning",
                    "compatibility.catalogue_scope_updated",
                    "refresh_catalogue",
                    "A tracked mod compatibility scope was applied to updated definition content that has not yet been acknowledged.",
                );
            }
            diagnostics::record(
                "info",
                "catalogue.scan_complete",
                "refresh_catalogue",
                &format!(
                    "Definition scan prepared {} entities from {} files ({} reused; {} parsed) in {} ms.",
                    progress.entities_prepared,
                    progress.files_processed,
                    progress.files_reused,
                    progress.files_parsed,
                    timer.elapsed().as_millis()
                ),
            );
            progress.phase = CatalogueRefreshPhase::Publishing;
            progress.progress_percent = Some(55);
            progress.current_source = None;
            progress.current_file = None;
            progress.current_file_index = None;
            progress.updated_at_ms = Some(now_ms());
            self.report_catalogue_progress(&progress, &mut notify);
            let changed =
                self.warehouse
                    .publish_catalogue_with_progress(&generation, |publication| {
                        progress.rows_written = publication.rows_written;
                        progress.rows_total = publication.rows_total;
                        progress.progress_percent =
                            ratio_percent(publication.rows_written, publication.rows_total, 55, 95);
                        progress.updated_at_ms = Some(now_ms());
                        self.report_catalogue_progress(&progress, &mut notify);
                    })?;
            progress.phase = CatalogueRefreshPhase::Finalising;
            progress.progress_percent = Some(98);
            progress.updated_at_ms = Some(now_ms());
            self.report_catalogue_progress(&progress, &mut notify);
            if changed {
                let active = self.storage.active_overlay_document()?;
                let overlay_identity = active
                    .as_ref()
                    .map(|(profile, revision, _)| format!("{profile}:{revision}"))
                    .unwrap_or_else(|| "none".to_owned());
                let identity = format!("{}:{overlay_identity}", generation.generation_id);
                self.storage.enqueue_current_overlay_projection(&identity)?;
            }
            Ok(())
        })();
        if let Err(error) = &result {
            if progress.phase == CatalogueRefreshPhase::Publishing {
                self.warehouse.note_catalogue_write_failure();
            }
            self.warehouse
                .note_catalogue_failure(requested_at, error.code());
            let _ = self.storage.note_catalogue_refresh_failure(error.code());
            progress.phase = CatalogueRefreshPhase::Failed;
            progress.progress_percent = None;
            progress.current_source = None;
            progress.current_file = None;
            progress.current_file_index = None;
            progress.updated_at_ms = Some(now_ms());
            progress.error_code = Some(error.code().to_owned());
            self.report_catalogue_progress(&progress, &mut notify);
            diagnostics::record(
                "error",
                "catalogue.refresh_failed",
                "refresh_catalogue",
                &format!(
                    "Definition refresh failed with code {} after {} ms.",
                    error.code(),
                    timer.elapsed().as_millis()
                ),
            );
        }
        result?;
        progress.phase = CatalogueRefreshPhase::Complete;
        progress.progress_percent = Some(100);
        progress.current_source = None;
        progress.current_file = None;
        progress.current_file_index = None;
        progress.updated_at_ms = Some(now_ms());
        self.report_catalogue_progress(&progress, &mut notify);
        diagnostics::record(
            "info",
            "catalogue.refresh_complete",
            "refresh_catalogue",
            &format!(
                "Definition refresh completed: {} files, {} entities and {} warehouse rows in {} ms.",
                progress.files_processed,
                progress.entities_prepared,
                progress.rows_written,
                timer.elapsed().as_millis()
            ),
        );
        self.catalogue_status()
    }

    fn report_catalogue_progress(
        &self,
        progress: &CatalogueRefreshProgress,
        notify: &mut impl FnMut(CatalogueRefreshProgress),
    ) {
        if let Ok(mut current) = self.catalogue_progress.lock() {
            *current = progress.clone();
        }
        notify(progress.clone());
    }

    pub fn catalogue_search(
        &self,
        filter: &CatalogueSearchFilter,
    ) -> Result<CataloguePage, ObservatoryError> {
        self.warehouse.search(filter)
    }

    pub fn catalogue_dossier(
        &self,
        entity_id: &str,
    ) -> Result<DefinitionDossier, ObservatoryError> {
        self.warehouse.dossier(entity_id)
    }

    pub fn production_route(
        &self,
        request: &ProductionRouteRequest,
    ) -> Result<ProductionRouteModel, ObservatoryError> {
        self.warehouse.production_route(request)
    }

    pub fn production_pathway(
        &self,
        request: &ProductionPathwayRequest,
    ) -> Result<ProductionPathwayModel, ObservatoryError> {
        self.warehouse.production_pathway(request)
    }

    pub fn production_route_coverage(&self) -> Result<ProductionRouteCoverage, ObservatoryError> {
        self.warehouse.production_route_coverage()
    }

    pub fn rebuild_warehouse(&self) -> Result<CatalogueStatus, ObservatoryError> {
        self.storage.enqueue_warehouse_rebuild()?;
        diagnostics::record(
            "info",
            "warehouse.rebuild_queued",
            "warehouse_rebuild",
            "A full analytical warehouse rebuild was queued; SQLite evidence and save observation remain available.",
        );
        self.catalogue_status()
    }

    pub fn inspect_overlay(&self, json: &str) -> OverlayInspection {
        match PlanningOverlayDocument::parse(json.as_bytes()) {
            Ok(document) => OverlayInspection {
                valid: true,
                code: None,
                profile: None,
                operation_count: document.operations.len().min(u32::MAX as usize) as u32,
                supplement_count: document.supplements.len().min(u32::MAX as usize) as u32,
                document: serde_json::to_value(document).ok(),
            },
            Err(error) => OverlayInspection {
                valid: false,
                code: Some(error.code().to_owned()),
                profile: None,
                operation_count: 0,
                supplement_count: 0,
                document: None,
            },
        }
    }

    pub fn import_overlay(&self, json: &str) -> Result<OverlayProfileSummary, ObservatoryError> {
        let document = PlanningOverlayDocument::parse(json.as_bytes())?;
        self.storage.install_overlay(&document)
    }

    pub fn export_overlay(
        &self,
        profile_id: &str,
        revision: u32,
    ) -> Result<String, ObservatoryError> {
        self.storage
            .overlay_document(profile_id, revision)?
            .canonical_json()
    }

    pub fn overlay_profiles(&self) -> Result<Vec<OverlayProfileSummary>, ObservatoryError> {
        let mut profiles = self.storage.list_overlay_profiles()?;
        for profile in &mut profiles {
            if let Some(revision) = profile.active_revision {
                profile.conflict_count = self
                    .warehouse
                    .overlay_conflict_count(&profile.profile_id, revision);
            }
        }
        Ok(profiles)
    }

    pub fn activate_overlay(
        &self,
        profile_id: &str,
        revision: Option<u32>,
    ) -> Result<OverlayProfileSummary, ObservatoryError> {
        self.storage.activate_overlay(profile_id, revision)
    }

    pub fn rollback_overlay(
        &self,
        profile_id: &str,
    ) -> Result<OverlayProfileSummary, ObservatoryError> {
        self.storage.rollback_overlay(profile_id)
    }

    pub fn deactivate_overlay(&self) -> Result<(), ObservatoryError> {
        self.storage.deactivate_overlay()
    }

    pub fn remove_overlay(&self, profile_id: &str) -> Result<(), ObservatoryError> {
        self.storage.remove_overlay(profile_id)
    }

    pub fn warehouse_snapshot(&self) -> Result<WarehouseSnapshot, ObservatoryError> {
        self.warehouse.snapshot()
    }

    pub(crate) fn process_next_projection_job(&self) -> Result<bool, ObservatoryError> {
        let Some(job) =
            self.run_coordinated_background_storage(|| self.storage.claim_projection_job())?
        else {
            return Ok(false);
        };
        let timer = Instant::now();
        diagnostics::record(
            "info",
            "warehouse.projection_started",
            "project_warehouse",
            &format!(
                "Analytical warehouse projection started ({}).",
                job.projection_kind
            ),
        );
        let result = match job.projection_kind.as_str() {
            "observation" => self
                .run_coordinated_background_storage(|| {
                    self.storage.load_dataset(&job.source_identity)
                })
                .and_then(|dataset| {
                    self.warehouse
                        .project_observation(&job.projection_id, &dataset, now_ms())
                }),
            "market_observation" => self
                .run_coordinated_background_storage(|| {
                    self.storage
                        .market_warehouse_projection(&job.source_identity)
                })
                .and_then(|projection| {
                    self.warehouse.project_market_observation(
                        &job.projection_id,
                        &projection,
                        now_ms(),
                    )
                }),
            "overlay_state" => self
                .run_coordinated_background_storage(|| self.storage.active_overlay_document())
                .and_then(|active| {
                    self.warehouse.project_overlay(
                        &job.projection_id,
                        active.as_ref().map(|(profile, revision, document)| {
                            (profile.as_str(), *revision, document)
                        }),
                        now_ms(),
                    )
                }),
            "branch_membership" => self
                .run_coordinated_background_storage(|| {
                    self.storage
                        .branch_membership_projection(&job.source_identity)
                })
                .and_then(|(revision, memberships)| {
                    self.warehouse.project_branch_memberships(
                        &job.projection_id,
                        &memberships,
                        &job.source_identity,
                        revision,
                        now_ms(),
                    )
                }),
            "rebuild" => self
                .warehouse
                .rebuild_observations(&job.projection_id, now_ms()),
            _ => Err(ObservatoryError::WarehouseUnavailable),
        };
        match result {
            Ok(()) => {
                self.run_coordinated_background_storage(|| {
                    self.storage.complete_projection_job(&job.projection_id)
                })?;
                diagnostics::record(
                    "info",
                    "warehouse.projection_complete",
                    "project_warehouse",
                    &format!(
                        "Analytical warehouse projection completed ({}) in {} ms.",
                        job.projection_kind,
                        timer.elapsed().as_millis()
                    ),
                );
            }
            Err(error) => {
                self.warehouse.note_projection_failure();
                let retry_delay_ms = self.warehouse.retry_delay().as_millis();
                self.run_coordinated_background_storage(|| {
                    self.storage
                        .fail_projection_job(&job.projection_id, error.code())
                })?;
                diagnostics::record(
                    "error",
                    "warehouse.projection_failed",
                    "project_warehouse",
                    &format!(
                        "Analytical warehouse projection failed ({}) with code {} after {} ms; write protection will wait {} ms.",
                        job.projection_kind,
                        error.code(),
                        timer.elapsed().as_millis(),
                        retry_delay_ms,
                    ),
                );
                return Err(error);
            }
        }
        Ok(true)
    }

    pub(crate) fn warehouse_retry_delay(&self) -> std::time::Duration {
        self.warehouse.retry_delay()
    }

    pub fn attention_cue_dismissed(
        &self,
        cue_id: &str,
        content_revision: u32,
    ) -> Result<bool, ObservatoryError> {
        self.storage
            .attention_cue_dismissed(cue_id, content_revision)
    }

    pub fn dismiss_attention_cue(
        &self,
        cue_id: &str,
        content_revision: u32,
    ) -> Result<(), ObservatoryError> {
        self.storage.dismiss_attention_cue(cue_id, content_revision)
    }

    pub fn replay_attention_cue(
        &self,
        cue_id: &str,
        content_revision: u32,
    ) -> Result<(), ObservatoryError> {
        self.storage.replay_attention_cue(cue_id, content_revision)
    }

    pub fn replay_all_attention_cues(&self) -> Result<u32, ObservatoryError> {
        let replayed = self.storage.replay_all_attention_cues()?;
        diagnostics::record(
            "info",
            "guidance.replayed",
            "application_settings",
            "Dismissed introductions and guidance cues were made available for replay.",
        );
        Ok(replayed)
    }

    pub(crate) fn research_setup(
        &self,
    ) -> Result<crate::storage::StoredResearchSetup, ObservatoryError> {
        self.storage.research_setup()
    }

    pub fn set_research_notice_revision(&self, revision: u32) -> Result<(), ObservatoryError> {
        self.storage.set_research_notice_revision(revision)
    }

    pub fn set_research_tesmio_checkout(
        &self,
        path: &Path,
        source_origin: &str,
    ) -> Result<(), ObservatoryError> {
        self.storage
            .set_research_tesmio_checkout(path, source_origin)
    }

    pub(crate) fn record_research_probe_build(
        &self,
        content_hash: &str,
    ) -> Result<(), ObservatoryError> {
        self.storage.record_research_probe_build(content_hash)
    }

    pub(crate) fn catalogue_configuration(&self) -> Result<Option<PathBuf>, ObservatoryError> {
        self.storage.get_setting(GAME_MEDIA_DIRECTORY_KEY)
    }

    pub(crate) fn compatibility_watch_root(&self) -> Option<PathBuf> {
        self.compatibility
            .local_path()
            .parent()
            .map(Path::to_path_buf)
    }

    pub(crate) fn catalogue_watch_roots(&self) -> Result<Vec<PathBuf>, ObservatoryError> {
        let game = self.catalogue_configuration()?;
        let workshop = self
            .storage
            .get_setting(WORKSHOP_DIRECTORY_KEY)?
            .filter(|path| path.is_dir());
        Ok(game
            .as_deref()
            .map(|path| catalogue_watch_roots(path, workshop.as_deref()))
            .unwrap_or_default())
    }

    pub(crate) fn note_catalogue_filesystem_event(
        &self,
        event_at_ms: i64,
    ) -> Result<(), ObservatoryError> {
        self.storage.note_catalogue_filesystem_event(event_at_ms)
    }

    fn observer_status(&self) -> Result<crate::model::AutomaticObserverStatus, ObservatoryError> {
        self.automatic_observer
            .lock()
            .map(|observer| observer.status())
            .map_err(|_| ObservatoryError::StorageUnavailable)
    }
}

fn count_save_candidates(directory: &Path) -> Result<u32, ObservatoryError> {
    let count = fs::read_dir(directory)
        .map_err(|_| ObservatoryError::InvalidDirectory)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_file()) && has_zip_extension(&entry.path())
        })
        .count();
    Ok(count.min(u32::MAX as usize) as u32)
}

fn has_zip_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn directory_display_name(path: &Path, game_media: bool) -> String {
    let display_path = if game_media
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("media_soviet"))
    {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    display_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Configured folder")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Write;

    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    use super::{ObservatoryApplication, count_save_candidates, market_index_job_id};
    use crate::automatic_observer::latest_save_candidate_path;
    use crate::compatibility_profile::ResolvedCompatibilityProfile;
    use crate::model::{DirectoryKind, MarketIndexCandidate, WarehousePhase};
    use crate::save_archive::inspect_save_archive;

    fn market_candidate(raw_payload_hash: &str) -> MarketIndexCandidate {
        MarketIndexCandidate {
            payload_hash: format!("payload-{raw_payload_hash}"),
            source_file_name: format!("{raw_payload_hash}.zip"),
            source_file_size: 100,
            source_modified_ms: 1,
            source_directory_identity: "directory".to_owned(),
            raw_payload_hash: raw_payload_hash.to_owned(),
        }
    }

    fn write_market_archive(path: &std::path::Path, record_id: u32) {
        let file = File::create(path).expect("market archive");
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("stats.ini", SimpleFileOptions::default())
            .expect("stats entry");
        write!(
            archive,
            "$STATS_FORMAT 1\n\
             $STAT_RECORD 0\n\
             $DATE_YEAR 1980\n\
             $DATE_DAY 1\n\
             $Citizens_EletronicNone 100\n\
             $Citizens_EletrinicRadio 20\n\
             $Citizens_EletronicTV 10\n\
             $Citizens_EletronicComputer 5\n\
             $Economy_PurchaseCostRUB\n\
             oil 12.5 1\n\
             $end\n\
             $Resources_ImportRUB\n\
             oil 4 40\n\
             $end\n\
             $STAT_RECORD 1\n\
             $DATE_YEAR 1980\n\
             $DATE_DAY 2\n\
             $Citizens_EletronicNone 101\n\
             $Citizens_EletrinicRadio 21\n\
             $Citizens_EletronicTV 11\n\
             $Citizens_EletronicComputer 6\n\
             $Resources_ImportRUB\n\
             oil 4 41\n\
             $end\n\
             $STAT_RECORD {}\n\
             $DATE_YEAR 1980\n\
             $DATE_DAY {}\n\
             $Citizens_EletronicNone {}\n\
             $Citizens_EletrinicRadio 22\n\
             $Citizens_EletronicTV 12\n\
             $Citizens_EletronicComputer 7\n\
             $Resources_ImportRUB\n\
             oil 4 {}\n\
             $end\n\
             $STAT_CURRENT\n\
             $Citizens_EletronicNone {}\n\
             $Citizens_EletrinicRadio 23\n\
             $Citizens_EletronicTV 13\n\
             $Citizens_EletronicComputer 8\n",
            record_id + 1,
            record_id + 2,
            100 + record_id,
            40 + record_id,
            101 + record_id,
        )
        .expect("stats payload");
        archive.finish().expect("complete archive");
    }

    #[test]
    fn candidates_are_zip_files_only() {
        let directory = tempdir().expect("temporary directory");
        fs::write(directory.path().join("one.zip"), b"one").expect("zip candidate");
        fs::write(directory.path().join("notes.txt"), b"notes").expect("ignored file");

        assert_eq!(count_save_candidates(directory.path()).expect("count"), 1);
        assert_eq!(
            latest_save_candidate_path(directory.path())
                .expect("candidate")
                .file_name()
                .and_then(|name| name.to_str()),
            Some("one.zip")
        );
    }

    #[test]
    fn market_index_job_identity_is_content_derived_and_order_independent() {
        let first = market_candidate("a");
        let second = market_candidate("b");
        let forward = market_index_job_id(
            "directory-identity",
            "profile-one",
            &[first.clone(), second.clone()],
        );
        let reverse = market_index_job_id(
            "directory-identity",
            "profile-one",
            &[second, first.clone(), first],
        );
        assert_eq!(forward, reverse);
        assert_ne!(
            forward,
            market_index_job_id(
                "directory-identity",
                "profile-two",
                &[market_candidate("a"), market_candidate("b")],
            )
        );
    }

    #[test]
    fn guided_market_indexing_handles_a_25_save_batch_without_moving_the_head() {
        let directory = tempdir().expect("temporary directory");
        let saves = directory.path().join("saves");
        fs::create_dir(&saves).expect("save directory");
        let application = ObservatoryApplication::initialise(
            directory.path().join("operational.sqlite3"),
            directory.path().join("analytical.duckdb"),
        )
        .expect("application");
        application
            .configure_directory(DirectoryKind::Save, saves.to_string_lossy().into_owned())
            .expect("configure saves");
        let legacy = ResolvedCompatibilityProfile::legacy_reviewed_builtins()
            .expect("legacy profiles")
            .remove(0);
        let mut source_archives = Vec::new();
        for index in 1..=25 {
            let path = saves.join(format!("market-{index:02}.zip"));
            write_market_archive(&path, index);
            source_archives.push((path.clone(), fs::read(&path).expect("source archive bytes")));
            let inspection = inspect_save_archive(&path, &legacy).expect("legacy observation");
            application
                .storage
                .save_inspection(&inspection)
                .expect("observation");
        }
        let before = application
            .archive_overview()
            .expect("archive before indexing")
            .analysis_context;
        let progress = application
            .index_available_saves_for_markets(|_| {})
            .expect("guided indexing");
        let after = application
            .archive_overview()
            .expect("archive after indexing")
            .analysis_context;

        assert_eq!(progress.total_archives, 25);
        assert_eq!(progress.completed_archives, 25);
        assert_eq!(progress.failed_archives, 0);
        assert_eq!(progress.missing_archives, 0);
        assert_eq!(progress.changed_archives, 0);
        assert_eq!(before.selected_branch_id, after.selected_branch_id);
        assert_eq!(before.head_interpretation_id, after.head_interpretation_id);
        assert_eq!(before.mode, after.mode);
        assert_eq!(before.origin, after.origin);

        let refreshed = application
            .refresh_changed_market_data(|_| {})
            .expect("incremental refresh");
        assert_eq!(refreshed.total_archives, 25);
        assert_eq!(refreshed.duplicate_archives, 25);
        assert!(refreshed.cache_records_reused > 0);
        assert!(refreshed.cache_rows_avoided > 0);
        for (path, before_bytes) in source_archives {
            assert_eq!(
                fs::read(path).expect("source archive after refresh"),
                before_bytes,
                "market maintenance must never modify a save archive"
            );
        }
        let after_refresh = application
            .archive_overview()
            .expect("archive after refresh")
            .analysis_context;
        assert_eq!(after.selected_branch_id, after_refresh.selected_branch_id);
        assert_eq!(
            after.head_interpretation_id,
            after_refresh.head_interpretation_id
        );
    }

    #[test]
    fn unavailable_warehouse_does_not_block_operational_startup() {
        let directory = tempdir().expect("temporary directory");
        let unavailable_path = directory.path().join("warehouse-is-a-directory");
        fs::create_dir(&unavailable_path).expect("blocking directory");
        let application = ObservatoryApplication::initialise(
            directory.path().join("operational.sqlite3"),
            unavailable_path,
        )
        .expect("SQLite startup must survive a warehouse failure");

        application.setup_state().expect("operational state");
        let status = application.catalogue_status().expect("degraded status");
        assert_eq!(status.warehouse.phase, WarehousePhase::Attention);
        assert_eq!(status.error_code.as_deref(), Some("warehouse_unavailable"));
    }

    #[test]
    fn active_warehouse_writer_does_not_block_setup_or_status() {
        let directory = tempdir().expect("temporary directory");
        let application = ObservatoryApplication::initialise(
            directory.path().join("operational.sqlite3"),
            directory.path().join("analytical.duckdb"),
        )
        .expect("application");
        let _writer = application
            .warehouse
            .test_writer_lock()
            .expect("writer lock");
        let started = std::time::Instant::now();

        application.setup_state().expect("setup state");
        let status = application.catalogue_status().expect("catalogue status");

        // This guards against waiting on the held DuckDB writer lock. A one-second
        // ceiling remains far below a blocked writer while tolerating parallel
        // filesystem-heavy tests on slower Windows hosts.
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        assert_eq!(status.warehouse.phase, WarehousePhase::Lagging);
    }

    #[test]
    #[ignore = "requires private copied app-local databases and is a reference-machine benchmark"]
    fn private_projection_backlog_completes_in_bounded_time() {
        let sqlite = std::env::var_os("RO_PROJECTION_SQLITE")
            .expect("set RO_PROJECTION_SQLITE to a disposable database copy");
        let duckdb = std::env::var_os("RO_PROJECTION_DUCKDB")
            .expect("set RO_PROJECTION_DUCKDB to a disposable warehouse copy");
        let application = ObservatoryApplication::initialise(sqlite.into(), duckdb.into())
            .expect("copied application data");
        let started = std::time::Instant::now();
        let mut projected = 0_u32;

        while application
            .process_next_projection_job()
            .expect("projection job")
        {
            projected = projected.saturating_add(1);
            assert!(projected <= 1_000, "projection queue did not converge");
        }

        eprintln!(
            "projected {projected} copied jobs in {:?}",
            started.elapsed()
        );
        assert!(projected > 0);
        assert!(started.elapsed() < std::time::Duration::from_secs(30));
    }
}
