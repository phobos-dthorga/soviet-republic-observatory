import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AnalysisPackContribution,
  AnalysisPackInspection,
  AnalysisPackSummary,
} from "../extensions/runtime";
import type {
  ArchiveOverview,
  ArchiveComparison,
  BranchSelectionResult,
  CataloguePage,
  CatalogueRefreshProgress,
  CatalogueSearchFilter,
  CatalogueStatus,
  CompatibilityStatus,
  CompatibilityUpdate,
  DefinitionDossier,
  DiagnosticLogView,
  DirectoryKind,
  ObservationImportResult,
  ObserverErrorCode,
  ReceiverDataset,
  RecorderHealth,
  RecorderUpdate,
  ReinterpretationProgress,
  SetupState,
  OverlayInspection,
  OverlayProfileSummary,
  ProductionRouteModel,
  ProductionRouteCoverage,
  ProductionRouteRequest,
} from "./types";

export function desktopHostAvailable(): boolean {
  return isTauri();
}

export async function chooseDirectory(title: string): Promise<string | null> {
  if (!desktopHostAvailable()) return null;
  const selected = await open({ directory: true, multiple: false, title });
  return typeof selected === "string" ? selected : null;
}

export function configureDirectory(
  kind: DirectoryKind,
  path: string,
): Promise<SetupState> {
  return invoke<SetupState>("configure_directory", { kind, path });
}

export function getSetupState(): Promise<SetupState> {
  return invoke<SetupState>("get_setup_state");
}

export function getLatestReceiverDataset(): Promise<ReceiverDataset | null> {
  return invoke<ReceiverDataset | null>("get_latest_receiver_dataset");
}

export function getArchiveOverview(): Promise<ArchiveOverview> {
  return invoke<ArchiveOverview>("get_archive_overview");
}

export function getRecorderHealth(): Promise<RecorderHealth> {
  return invoke<RecorderHealth>("get_recorder_health");
}

export function listenForRecorderUpdates(
  accept: (update: RecorderUpdate) => void,
): Promise<UnlistenFn> {
  return listen<RecorderUpdate>("recorder-update", (event) =>
    accept(event.payload),
  );
}

export function selectTimelineBranch(
  branchId: string,
): Promise<BranchSelectionResult> {
  return invoke<BranchSelectionResult>("select_timeline_branch", {
    branchId,
  });
}

export function observeLatestSave(): Promise<ObservationImportResult> {
  return invoke<ObservationImportResult>("observe_latest_save");
}

export function setAutomaticObservation(enabled: boolean): Promise<SetupState> {
  return invoke<SetupState>("set_automatic_observation", { enabled });
}

export function compareArchiveObservations(
  fromInterpretationId: string,
  toInterpretationId: string,
): Promise<ArchiveComparison> {
  return invoke<ArchiveComparison>("compare_archive_observations", {
    fromInterpretationId,
    toInterpretationId,
  });
}

export function getCompatibilityStatus(): Promise<CompatibilityStatus> {
  return invoke<CompatibilityStatus>("get_compatibility_status");
}

export function createLocalCompatibilityOverride(): Promise<CompatibilityUpdate> {
  return invoke<CompatibilityUpdate>("create_local_compatibility_override");
}

export function reloadLocalCompatibilityOverride(): Promise<CompatibilityUpdate> {
  return invoke<CompatibilityUpdate>("reload_local_compatibility_override");
}

export function reinterpretLatestSave(): Promise<ObservationImportResult> {
  return invoke<ObservationImportResult>("reinterpret_latest_save");
}

export function getReinterpretationProgress(): Promise<ReinterpretationProgress> {
  return invoke<ReinterpretationProgress>("get_reinterpretation_progress");
}

export function listenForCompatibilityUpdates(
  accept: (update: CompatibilityUpdate) => void,
): Promise<UnlistenFn> {
  return listen<CompatibilityUpdate>("compatibility-update", (event) =>
    accept(event.payload),
  );
}

export function listenForReinterpretationProgress(
  accept: (progress: ReinterpretationProgress) => void,
): Promise<UnlistenFn> {
  return listen<ReinterpretationProgress>(
    "compatibility-reinterpretation-progress",
    (event) => accept(event.payload),
  );
}

export function observerErrorCode(error: unknown): ObserverErrorCode {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string"
  ) {
    return error.code as ObserverErrorCode;
  }
  return "unknown";
}

export function getCatalogueStatus(): Promise<CatalogueStatus> {
  return invoke<CatalogueStatus>("get_catalogue_status");
}

export function getDiagnosticLog(): Promise<DiagnosticLogView> {
  return invoke<DiagnosticLogView>("diagnostic_log");
}

export function clearDiagnosticLog(): Promise<DiagnosticLogView> {
  return invoke<DiagnosticLogView>("clear_diagnostic_log");
}

export function refreshDefinitions(): Promise<CatalogueStatus> {
  return invoke<CatalogueStatus>("refresh_definitions");
}

export function rebuildWarehouse(): Promise<CatalogueStatus> {
  return invoke<CatalogueStatus>("rebuild_warehouse");
}

export function searchCatalogue(
  filter: CatalogueSearchFilter,
): Promise<CataloguePage> {
  return invoke<CataloguePage>("search_catalogue", { filter });
}

export function getDefinitionDossier(
  entityId: string,
): Promise<DefinitionDossier> {
  return invoke<DefinitionDossier>("get_definition_dossier", { entityId });
}

export function getProductionRoute(
  request: ProductionRouteRequest,
): Promise<ProductionRouteModel> {
  return invoke<ProductionRouteModel>("get_production_route", { request });
}

export function getProductionRouteCoverage(): Promise<ProductionRouteCoverage> {
  return invoke<ProductionRouteCoverage>("get_production_route_coverage");
}

export function inspectPlanningOverlay(
  json: string,
): Promise<OverlayInspection> {
  return invoke<OverlayInspection>("inspect_planning_overlay", { json });
}

export function importPlanningOverlay(
  json: string,
): Promise<OverlayProfileSummary> {
  return invoke<OverlayProfileSummary>("import_planning_overlay", { json });
}

export function exportPlanningOverlay(
  profileId: string,
  revision: number,
): Promise<string> {
  return invoke<string>("export_planning_overlay", { profileId, revision });
}

export function listPlanningOverlays(): Promise<OverlayProfileSummary[]> {
  return invoke<OverlayProfileSummary[]>("list_planning_overlays");
}

export function activatePlanningOverlay(
  profileId: string,
  revision?: number,
): Promise<OverlayProfileSummary> {
  return invoke<OverlayProfileSummary>("activate_planning_overlay", {
    profileId,
    revision,
  });
}

export function rollbackPlanningOverlay(
  profileId: string,
): Promise<OverlayProfileSummary> {
  return invoke<OverlayProfileSummary>("rollback_planning_overlay", {
    profileId,
  });
}

export function deactivatePlanningOverlay(): Promise<void> {
  return invoke<void>("deactivate_planning_overlay");
}

export function removePlanningOverlay(profileId: string): Promise<void> {
  return invoke<void>("remove_planning_overlay", { profileId });
}

export function listenForCatalogueUpdates(
  accept: (status: CatalogueStatus) => void,
): Promise<UnlistenFn> {
  return listen<CatalogueStatus>("catalogue-update", (event) =>
    accept(event.payload),
  );
}

export function listenForCatalogueProgress(
  accept: (progress: CatalogueRefreshProgress) => void,
): Promise<UnlistenFn> {
  return listen<CatalogueRefreshProgress>("catalogue-progress", (event) =>
    accept(event.payload),
  );
}

export function listenForWarehouseUpdates(
  accept: (status: CatalogueStatus) => void,
): Promise<UnlistenFn> {
  return listen<CatalogueStatus>("warehouse-update", (event) =>
    accept(event.payload),
  );
}

export function inspectAnalysisPack(
  json: string,
): Promise<AnalysisPackInspection> {
  return invoke<AnalysisPackInspection>("inspect_analysis_pack", { json });
}

export function importAnalysisPack(json: string): Promise<AnalysisPackSummary> {
  return invoke<AnalysisPackSummary>("import_analysis_pack", { json });
}

export function exportAnalysisPack(
  packId: string,
  revision: number,
): Promise<string> {
  return invoke<string>("export_analysis_pack", { packId, revision });
}

export function listAnalysisPacks(): Promise<AnalysisPackSummary[]> {
  return invoke<AnalysisPackSummary[]>("list_analysis_packs");
}

export function enableAnalysisPack(
  packId: string,
  revision?: number,
): Promise<AnalysisPackSummary> {
  return invoke<AnalysisPackSummary>("enable_analysis_pack", {
    packId,
    revision,
  });
}

export function disableAnalysisPack(
  packId: string,
): Promise<AnalysisPackSummary> {
  return invoke<AnalysisPackSummary>("disable_analysis_pack", { packId });
}

export function rollbackAnalysisPack(
  packId: string,
): Promise<AnalysisPackSummary> {
  return invoke<AnalysisPackSummary>("rollback_analysis_pack", { packId });
}

export function removeAnalysisPack(packId: string): Promise<void> {
  return invoke<void>("remove_analysis_pack", { packId });
}

export function getAnalysisPackContributions(): Promise<
  AnalysisPackContribution[]
> {
  return invoke<AnalysisPackContribution[]>("get_analysis_pack_contributions");
}
