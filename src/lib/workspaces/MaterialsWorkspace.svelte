<script lang="ts">
  import { onMount } from "svelte";
  import { activeLocale, translation } from "../i18n/runtime";
  import {
    containedSectionNavigation,
    focusContainedWorkspaceTarget,
  } from "../navigation/containedSectionNavigation";
  import type {
    RelatedDataDestination,
    WorkspaceFilters,
    WorkspaceLocation,
  } from "../navigation/relatedData";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import ProductionRouteLaboratory from "./ProductionRouteLaboratory.svelte";
  import {
    activatePlanningOverlay,
    deactivatePlanningOverlay,
    exportPlanningOverlay,
    getCatalogueStatus,
    getDefinitionDossier,
    getResourceCatalogue,
    getResourceDetails,
    getResourceRegistryStatus,
    importPlanningOverlay,
    inspectPlanningOverlay,
    listenForCatalogueProgress,
    listenForCatalogueUpdates,
    listenForWarehouseUpdates,
    listPlanningOverlays,
    rebuildWarehouse,
    refreshDefinitions,
    removePlanningOverlay,
    rollbackPlanningOverlay,
    searchCatalogue,
    disableResourceRegistryIngestion,
    enableResourceRegistryIngestion,
  } from "../observations/desktopClient";
  import type {
    CataloguePage,
    CatalogueRefreshProgress,
    CatalogueStatus,
    DefinitionDossier,
    DefinitionValue,
    OverlayInspection,
    OverlayProfileSummary,
    ProductionPathwayModel,
    ProductionRouteModel,
    ResourceCatalogueOriginFilter,
    ResourceCatalogueView,
    ResourceDetails,
    ResourceRegistryAssurance,
    ResourceRegistryStatus,
    WarehouseWriteActivity,
  } from "../observations/types";
  import { formatDate, formatNumber } from "../i18n/format";
  import TaskProgressPanel from "../tasks/TaskProgressPanel.svelte";
  import FilePicker from "../ui/FilePicker.svelte";
  import { catalogueProgressView } from "../tasks/catalogueProgress";
  import {
    observeLatestTaskProgress,
    selectLatestTaskProgress,
  } from "../tasks/progress";

  let {
    desktopAvailable,
    gameConfigured,
    location,
    onlocationchange,
    onrelatednavigate,
    onopenresearch,
    reviewRoute = null,
    reviewPathway = null,
    reviewResourceCatalogue = null,
    reviewResourceDetails = null,
    reviewResourceRegistry = null,
  } = $props<{
    desktopAvailable: boolean;
    gameConfigured: boolean;
    location: WorkspaceLocation;
    onlocationchange?: (filters: WorkspaceFilters) => void;
    onrelatednavigate?: (
      destinations: RelatedDataDestination[],
      origin: HTMLElement | null,
    ) => void;
    onopenresearch: () => void;
    reviewRoute?: ProductionRouteModel | null;
    reviewPathway?: ProductionPathwayModel | null;
    reviewResourceCatalogue?: ResourceCatalogueView | null;
    reviewResourceDetails?: ResourceDetails | null;
    reviewResourceRegistry?: ResourceRegistryStatus | null;
  }>();
  let status = $state<CatalogueStatus | null>(null);
  let refreshProgress = $state<CatalogueRefreshProgress | null>(null);
  let page = $state<CataloguePage | null>(null);
  let dossier = $state<DefinitionDossier | null>(null);
  let resourceCatalogue = $state<ResourceCatalogueView | null>(null);
  let resourceDetails = $state<ResourceDetails | null>(null);
  let resourceRegistry = $state<ResourceRegistryStatus | null>(null);
  let resourceQuery = $state("");
  let resourceOrigin = $state<ResourceCatalogueOriginFilter | "">("");
  let resourceBusy = $state(false);
  let resourceMessage = $state("");
  let resourceAssurance = $state<ResourceRegistryAssurance>(
    "verified_observation_only",
  );
  let resourceAcknowledged = $state(false);
  let profiles = $state<OverlayProfileSummary[]>([]);
  let query = $state("");
  let kind = $state("");
  let sourceKind = $state("");
  let packageQuery = $state("");
  let coverage = $state("");
  let availableYear = $state("");
  let busy = $state(false);
  let message = $state("");
  let overlayText = $state("");
  let inspection = $state<OverlayInspection | null>(null);
  let profileId = $state("local.republic.planning");
  let profileName = $state("Republic planning notes");
  let author = $state("Local planner");
  let description = $state("Player-supplied planning assumptions.");
  let supplementKind = $state("resource");
  let supplementId = $state("planning_material");
  let supplementName = $state("Planning material");
  let clockMs = $state(Date.now());
  let searchRequest = 0;
  let dossierRequest = 0;
  let requestedEntityId = "";
  const refreshActive = $derived(
    refreshProgress != null &&
      ["discovering", "scanning", "publishing", "finalising"].includes(
        refreshProgress.phase,
      ),
  );
  const progressView = $derived(
    refreshProgress
      ? catalogueProgressView(refreshProgress, $translation, clockMs)
      : null,
  );
  const hasRepairFacts = $derived(
    dossier?.facts.some((fact) =>
      fact.field_id.startsWith("definition.repair."),
    ) ?? false,
  );

  function formatBytes(bytes = 0): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KiB`;
    return `${(bytes / 1024 ** 2).toFixed(1)} MiB`;
  }

  function formatLag(milliseconds: number | null | undefined): string {
    if (milliseconds == null) return $translation("catalogue-current");
    const minutes = Math.max(0, Math.round(milliseconds / 60_000));
    if (minutes < 60) return `${minutes} min`;
    return `${(minutes / 60).toFixed(1)} h`;
  }

  function formatTimestamp(milliseconds: number | null | undefined): string {
    if (milliseconds == null) return "—";
    return formatDate(milliseconds, $activeLocale, {
      dateStyle: "short",
      timeStyle: "short",
    });
  }

  function warehouseActivityLabel(activity: WarehouseWriteActivity): string {
    const kind =
      activity.kind === "catalogue_publication"
        ? $translation("catalogue-global-progress")
        : activity.kind === "observation_projection"
          ? $translation("nav-monitor")
          : activity.kind === "market_projection"
            ? $translation("nav-markets")
            : activity.kind === "broadcast_projection"
              ? $translation("nav-broadcast")
              : activity.kind === "overlay_projection"
                ? $translation("catalogue-overlays")
                : $translation("catalogue-rebuild");
    const stage =
      activity.stage === "staging"
        ? $translation("catalogue-progress-stage-publish")
        : activity.stage === "rebuilding"
          ? $translation("catalogue-rebuild")
          : $translation("catalogue-progress-stage-finalise");
    const progress = activity.rows_total
      ? ` · ${formatNumber(activity.rows_processed, $activeLocale)} / ${formatNumber(activity.rows_total, $activeLocale)}`
      : "";
    return `${kind} · ${stage}${progress}`;
  }

  function displayValue(value: DefinitionValue | null): string {
    if (!value) return "—";
    const payload = value.number ?? value.text ?? "—";
    return value.unit ? `${payload} ${value.unit}` : String(payload);
  }

  function mappingLabel(classification: string): string {
    return classification === "player_mapped"
      ? $translation("compatibility-player-mapped")
      : $translation("compatibility-reviewed");
  }

  function scopeStateLabel(state: string | null): string {
    switch (state) {
      case "matched":
        return $translation("compatibility-scope-matched");
      case "dormant":
        return $translation("compatibility-scope-dormant");
      case "updated_unreviewed":
        return $translation("compatibility-scope-updated");
      case "conflict":
        return $translation("compatibility-scope-conflict");
      default:
        return "";
    }
  }

  async function loadStatus(): Promise<void> {
    if (!desktopAvailable) return;
    const nextStatus = await getCatalogueStatus();
    const accepted = acceptCatalogueStatus(nextStatus);
    profiles = await listPlanningOverlays();
    if (accepted.reload && accepted.generationId)
      await runSearch(accepted.generationId);
  }

  async function loadResourceCatalogue(): Promise<void> {
    if (reviewResourceCatalogue) {
      resourceCatalogue = reviewResourceCatalogue;
      resourceDetails = reviewResourceDetails;
      return;
    }
    if (!desktopAvailable) return;
    resourceCatalogue = await getResourceCatalogue({
      query: resourceQuery || undefined,
      origin: resourceOrigin || undefined,
      limit: 150,
    });
    const selected = resourceDetails?.entry.resource_id;
    const next = resourceCatalogue.entries.find(
      (entry) => entry.resource_id === selected,
    );
    if (next) await selectResource(next.resource_id);
    else if (resourceCatalogue.entries[0]) {
      await selectResource(resourceCatalogue.entries[0].resource_id);
    } else resourceDetails = null;
  }

  async function loadResourceRegistryStatus(): Promise<void> {
    if (reviewResourceRegistry) {
      resourceRegistry = reviewResourceRegistry;
      resourceAssurance = reviewResourceRegistry.assurance ?? resourceAssurance;
      return;
    }
    if (!desktopAvailable) return;
    resourceRegistry = await getResourceRegistryStatus();
    if (resourceRegistry.assurance) {
      resourceAssurance = resourceRegistry.assurance;
    }
  }

  async function selectResource(resourceId: string): Promise<void> {
    if (reviewResourceCatalogue) {
      const entry = reviewResourceCatalogue.entries.find(
        (candidate: ResourceCatalogueView["entries"][number]) =>
          candidate.resource_id === resourceId,
      );
      if (!entry) return;
      resourceDetails = {
        revision_id: reviewResourceCatalogue.revision.revision_id,
        entry,
        installed_sources: [],
        recorded_profile_count: Number(entry.origin.recorded_save),
        live_snapshot: entry.origin.live_game
          ? (reviewResourceRegistry?.latest_snapshot ?? null)
          : null,
      };
    } else {
      if (!desktopAvailable) return;
      resourceDetails = await getResourceDetails(resourceId);
    }
    onlocationchange?.({ resourceToken: resourceDetails.entry.source_token });
  }

  function resourceErrorCode(error: unknown): string {
    return typeof error === "object" && error !== null && "code" in error
      ? String((error as { code: unknown }).code)
      : "resource_registry_unavailable";
  }

  function resourceRegistryStateLabel(
    state: ResourceRegistryStatus["state"] | undefined,
  ): string {
    if (state === "waiting_for_game") {
      const stage = resourceRegistry?.collection_stage;
      if (stage === "waiting_for_game_state")
        return $translation("resources-live-state-waiting-game-state");
      if (stage === "waiting_for_loaded_republic")
        return $translation("resources-live-state-waiting-republic");
      if (stage === "stopped_at_record_limit")
        return $translation("resources-live-state-record-limit");
    }
    const keys = {
      disabled: "resources-live-state-disabled",
      waiting_for_game: "resources-live-state-waiting-for-game",
      available: "resources-live-state-available",
      invalid: "resources-live-state-invalid",
    } as const;
    return $translation(keys[state ?? "disabled"]);
  }

  async function setResourceRegistryEnabled(enabled: boolean): Promise<void> {
    if (!desktopAvailable || resourceBusy) return;
    resourceBusy = true;
    resourceMessage = "";
    try {
      resourceRegistry = enabled
        ? await enableResourceRegistryIngestion(
            resourceAssurance,
            resourceAcknowledged,
          )
        : await disableResourceRegistryIngestion();
      resourceAcknowledged = false;
      await loadResourceCatalogue();
    } catch (error) {
      const code = resourceErrorCode(error);
      resourceMessage = $translation(
        code === "research_notice_required"
          ? "resources-live-notice-required"
          : code === "invalid_research_setup"
            ? "resources-live-setup-invalid"
            : "resources-live-action-failed",
      );
    } finally {
      resourceBusy = false;
    }
  }

  function catalogueSnapshotIdentity(
    catalogueStatus: CatalogueStatus | null,
  ): string | null {
    const generationId = catalogueStatus?.generation?.generation_id;
    if (!generationId) return null;
    const overlay = catalogueStatus?.active_overlay;
    return `${generationId}|${overlay?.profile_id ?? "none"}|${overlay?.active_revision ?? 0}`;
  }

  function acceptCatalogueStatus(nextStatus: CatalogueStatus): {
    generationId: string | null;
    reload: boolean;
  } {
    const previousIdentity = catalogueSnapshotIdentity(status);
    const nextIdentity = catalogueSnapshotIdentity(nextStatus);
    const nextGenerationId = nextStatus.generation?.generation_id ?? null;
    status = nextStatus;
    acceptRefreshProgress(nextStatus.refresh);
    if (!nextIdentity || previousIdentity !== nextIdentity) {
      searchRequest += 1;
      dossierRequest += 1;
      page = null;
      dossier = null;
    }
    return {
      generationId: nextGenerationId,
      reload: nextIdentity !== null && previousIdentity !== nextIdentity,
    };
  }

  function acceptRefreshProgress(next: CatalogueRefreshProgress): void {
    refreshProgress = selectLatestTaskProgress(refreshProgress, next);
  }

  async function runSearch(
    expectedGenerationId = status?.generation?.generation_id,
  ): Promise<void> {
    if (!desktopAvailable || !expectedGenerationId) return;
    const request = ++searchRequest;
    const nextPage = await searchCatalogue({
      query: query || undefined,
      entity_kind: kind
        ? (kind as "resource" | "building" | "vehicle" | "recipe")
        : undefined,
      source_kind: sourceKind || undefined,
      package_query: packageQuery || undefined,
      coverage: coverage ? (coverage as "complete" | "partial") : undefined,
      available_year: availableYear ? Number(availableYear) : undefined,
      limit: 75,
    });
    if (
      request !== searchRequest ||
      status?.generation?.generation_id !== expectedGenerationId
    )
      return;
    page = nextPage;
    if (!dossier && nextPage.items[0])
      await selectEntity(nextPage.items[0].entity_id, expectedGenerationId);
  }

  async function selectEntity(
    entityId: string,
    expectedGenerationId = status?.generation?.generation_id,
  ): Promise<void> {
    if (!expectedGenerationId) return;
    const request = ++dossierRequest;
    const nextDossier = await getDefinitionDossier(entityId);
    if (
      request === dossierRequest &&
      status?.generation?.generation_id === expectedGenerationId
    ) {
      dossier = nextDossier;
    }
  }

  async function chooseEntity(entityId: string): Promise<void> {
    await selectEntity(entityId);
    onlocationchange?.({ catalogueEntityId: entityId });
  }

  async function openRequestedEntity(entityId: string): Promise<void> {
    try {
      await selectEntity(entityId);
    } catch {
      query = entityId;
      await runSearch();
      const match = page?.items.find(
        (item) => item.entity_id === entityId || item.display_name === entityId,
      );
      if (match) await selectEntity(match.entity_id);
    }
  }

  $effect(() => {
    const next = location.filters.catalogueEntityId ?? "";
    if (!next || next === requestedEntityId || !status?.generation) return;
    requestedEntityId = next;
    void openRequestedEntity(next);
  });

  $effect(() => {
    if (reviewResourceCatalogue) {
      resourceCatalogue = reviewResourceCatalogue;
      resourceDetails = reviewResourceDetails;
    }
    if (reviewResourceRegistry) {
      resourceRegistry = reviewResourceRegistry;
      resourceAssurance = reviewResourceRegistry.assurance ?? resourceAssurance;
    }
  });

  $effect(() => {
    const token = location.filters.resourceToken;
    if (!token || !resourceCatalogue) return;
    const match = resourceCatalogue.entries.find(
      (entry) => entry.source_token === token,
    );
    if (match && resourceDetails?.entry.resource_id !== match.resource_id) {
      void selectResource(match.resource_id);
    }
  });

  async function runAction(action: () => Promise<unknown>): Promise<void> {
    busy = true;
    message = "";
    try {
      await action();
      await loadStatus();
    } catch (error) {
      if (
        typeof error === "object" &&
        error !== null &&
        "code" in error &&
        error.code === "catalogue_compatibility_conflict"
      ) {
        message = $translation("error-catalogue-compatibility-conflict");
      } else {
        message = $translation("catalogue-action-failed");
      }
    } finally {
      busy = false;
    }
  }

  function createSupplementDocument(): void {
    overlayText = JSON.stringify(
      {
        schema_version: 1,
        id: profileId,
        version: "1.0.0",
        name: profileName,
        author,
        default_locale: "en-AU",
        description,
        operations: [],
        supplements: [
          {
            local_id: supplementId,
            entity_kind: supplementKind,
            display_name: supplementName,
            reason: description,
            properties: [],
          },
        ],
      },
      null,
      2,
    );
    inspection = null;
  }

  async function inspectOverlay(): Promise<void> {
    inspection = await inspectPlanningOverlay(overlayText);
  }

  async function importOverlay(): Promise<void> {
    inspection = await inspectPlanningOverlay(overlayText);
    if (!inspection.valid) return;
    await runAction(() => importPlanningOverlay(overlayText));
  }

  async function loadOverlayFile(file: File | null): Promise<void> {
    if (!file || file.size > 1024 * 1024) return;
    overlayText = await file.text();
    inspection = null;
  }

  async function downloadProfile(
    profile: OverlayProfileSummary,
  ): Promise<void> {
    const json = await exportPlanningOverlay(
      profile.profile_id,
      profile.latest_revision,
    );
    const url = URL.createObjectURL(
      new Blob([json], { type: "application/json" }),
    );
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${profile.profile_id}.rooverlay.json`;
    anchor.click();
    URL.revokeObjectURL(url);
  }

  async function reviewProfile(profile: OverlayProfileSummary): Promise<void> {
    overlayText = await exportPlanningOverlay(
      profile.profile_id,
      profile.latest_revision,
    );
    inspection = await inspectPlanningOverlay(overlayText);
    focusContainedWorkspaceTarget(document.getElementById("overlay-workbench"));
  }

  onMount(() => {
    if (!desktopAvailable) return;
    let disposed = false;
    const stops: Array<() => void> = [];
    const clock = window.setInterval(() => (clockMs = Date.now()), 1_000);
    const registryClock = window.setInterval(() => {
      if (resourceRegistry?.enabled && !resourceBusy) {
        const previousSnapshot = resourceRegistry.latest_snapshot?.snapshot_id;
        void loadResourceRegistryStatus().then(() => {
          const nextSnapshot = resourceRegistry?.latest_snapshot?.snapshot_id;
          if (nextSnapshot && nextSnapshot !== previousSnapshot) {
            void loadResourceCatalogue();
          }
        });
      }
    }, 5_000);
    for (const listen of [
      listenForCatalogueUpdates,
      listenForWarehouseUpdates,
    ]) {
      void listen((next) => {
        if (!disposed) {
          const accepted = acceptCatalogueStatus(next);
          if (accepted.reload && accepted.generationId)
            void runSearch(accepted.generationId);
        }
      }).then((stop) => (disposed ? stop() : stops.push(stop)));
    }
    void (async () => {
      const stop = await observeLatestTaskProgress(
        {
          listen: listenForCatalogueProgress,
          read: async () => (await getCatalogueStatus()).refresh,
        },
        (next) => {
          if (disposed) return;
          acceptRefreshProgress(next);
          if (next.phase === "complete" || next.phase === "failed") {
            void loadStatus();
          }
        },
      );
      if (disposed) stop();
      else {
        stops.push(stop);
        await Promise.all([
          loadStatus(),
          loadResourceCatalogue(),
          loadResourceRegistryStatus(),
        ]);
      }
    })();
    return () => {
      disposed = true;
      window.clearInterval(clock);
      window.clearInterval(registryClock);
      stops.forEach((stop) => stop());
    };
  });
</script>

<section class="workspace catalogue-workspace">
  <aside
    class="navigator"
    aria-label={$translation("catalogue-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("catalogue-industrial")}</span>
        <h2>{$translation("nav-materials")}</h2>
      </div>
      <span class="edition">v0.3</span>
    </div>
    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("catalogue-generation")}</span><strong
          >{status?.generation?.generation_id.slice(0, 8) ?? "—"}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("catalogue-warehouse")}</span><strong
          >{status?.warehouse.phase ?? "—"}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("catalogue-overlay")}</span><strong
          >{status?.active_overlay?.display_name ??
            $translation("catalogue-none")}</strong
        >
      </div>
    </div>
    <div class="section-list">
      <a href="#material-flow-laboratory" use:containedSectionNavigation
        ><span>01</span>{$translation("catalogue-flow-laboratory")}</a
      >
      <a href="#resource-catalogue" use:containedSectionNavigation
        ><span>02</span>{$translation("resources-title")}</a
      >
      <a href="#catalogue-browser" use:containedSectionNavigation
        ><span>03</span>{$translation("catalogue-browser")}</a
      >
      <a href="#definition-dossier" use:containedSectionNavigation
        ><span>04</span>{$translation("catalogue-dossier")}</a
      >
      <a href="#overlay-laboratory" use:containedSectionNavigation
        ><span>05</span>{$translation("catalogue-overlays")}</a
      >
    </div>
    <GuidanceSurface kind="boundary" layout="compact" class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("catalogue-boundary-note")}</p>
    </GuidanceSurface>
  </aside>

  <section class="canvas">
    <GuidanceSurface
      kind="instruction"
      layout="inline"
      semanticRole="status"
      class={`preview-banner ${status?.warehouse.phase === "attention" ? "attention" : ""}`}
    >
      <strong>{$translation("catalogue-local-offline")}</strong>
      <span
        >{status?.warehouse.pending_jobs ?? 0}
        {$translation("catalogue-pending-jobs")}</span
      >
    </GuidanceSurface>
    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("catalogue-heading-eyebrow")}</span>
        <h2>{$translation("catalogue-heading-title")}</h2>
        <p>{$translation("catalogue-heading-description")}</p>
      </div>
      <div class="catalogue-actions">
        <button
          disabled={busy ||
            refreshActive ||
            !desktopAvailable ||
            !gameConfigured}
          onclick={() => runAction(refreshDefinitions)}
          >{$translation("catalogue-refresh")}</button
        >
        <button
          disabled={busy || !desktopAvailable}
          onclick={() => runAction(rebuildWarehouse)}
          >{$translation("catalogue-rebuild")}</button
        >
      </div>
    </header>

    {#if progressView && refreshProgress?.phase !== "idle"}
      <TaskProgressPanel
        view={progressView}
        headingId="catalogue-progress-heading"
      />
    {/if}

    <div id="material-flow-laboratory" class="flow-laboratory">
      <ProductionRouteLaboratory
        {desktopAvailable}
        {gameConfigured}
        generationId={reviewRoute?.snapshot.catalogue_generation_id ??
          status?.generation?.generation_id ??
          null}
        overlayProfileName={status?.active_overlay?.display_name ?? null}
        overlayRevision={status?.active_overlay?.active_revision ?? null}
        requestedResourceToken={location.filters.resourceToken}
        {onlocationchange}
        {onrelatednavigate}
        {reviewRoute}
        {reviewPathway}
      />
    </div>

    {#if !desktopAvailable && !reviewResourceCatalogue}
      <section class="empty-catalogue">
        <h2>{$translation("catalogue-setup-title")}</h2>
        <p>{$translation("catalogue-setup-description")}</p>
      </section>
    {:else}
      {#if !gameConfigured}
        <GuidanceSurface kind="instruction" layout="compact">
          <strong>{$translation("catalogue-setup-title")}</strong>
          <span>{$translation("catalogue-setup-description")}</span>
        </GuidanceSurface>
      {/if}
      <section
        class="catalogue-health"
        aria-label={$translation("catalogue-health-label")}
      >
        <article>
          <span>{$translation("catalogue-sources")}</span><strong
            >{status?.generation?.source_count ?? 0}</strong
          >
        </article>
        <article>
          <span>{$translation("catalogue-files")}</span><strong
            >{status?.generation?.file_count ?? 0}</strong
          >
        </article>
        <article>
          <span>{$translation("catalogue-entities")}</span><strong
            >{status?.generation?.entity_count ?? 0}</strong
          >
        </article>
        <article>
          <span>{$translation("catalogue-facts")}</span><strong
            >{status?.generation?.property_count ?? 0}</strong
          >
        </article>
        <article>
          <span>{$translation("catalogue-size")}</span><strong
            >{formatBytes(status?.warehouse.database_size_bytes)}</strong
          >
        </article>
      </section>

      <section class="resource-catalogue" id="resource-catalogue">
        <header class="panel-heading">
          <div>
            <span class="eyebrow">{$translation("resources-eyebrow")}</span>
            <h2>{$translation("resources-title")}</h2>
            <p>{$translation("resources-detail")}</p>
          </div>
          <span class="coverage">{resourceCatalogue?.total ?? 0}</span>
        </header>

        <div class="resource-registry-card">
          <div>
            <span class="eyebrow">{$translation("resources-live-eyebrow")}</span
            >
            <h3>{$translation("resources-live-title")}</h3>
            <p>{$translation("resources-live-detail")}</p>
          </div>
          <span
            class="status-chip"
            data-status={resourceRegistry?.state === "available"
              ? "stable"
              : resourceRegistry?.state === "invalid"
                ? "risk"
                : "watch"}
          >
            {resourceRegistryStateLabel(resourceRegistry?.state)}
          </span>
          {#if resourceRegistry?.latest_snapshot}
            <GuidanceSurface kind="instruction" layout="compact">
              <strong>{$translation("resources-live-retained")}</strong>
              <span
                >{$translation("observation-game-date-compact", {
                  year: resourceRegistry.latest_snapshot.captured_year,
                  day: String(
                    resourceRegistry.latest_snapshot.captured_day,
                  ).padStart(3, "0"),
                })} · {resourceRegistry.latest_snapshot.resource_count}
                {$translation("resources-count-suffix")}</span
              >
            </GuidanceSurface>
          {/if}
          <div class="resource-registry-controls">
            <label>
              <span>{$translation("resources-live-mode")}</span>
              <select
                bind:value={resourceAssurance}
                disabled={resourceRegistry?.enabled || resourceBusy}
              >
                <option value="verified_observation_only"
                  >{$translation("resources-live-mode-verified")}</option
                >
                <option value="player_managed_modded"
                  >{$translation("resources-live-mode-modded")}</option
                >
              </select>
            </label>
            {#if !resourceRegistry?.enabled}
              <label class="resource-acknowledgement">
                <input type="checkbox" bind:checked={resourceAcknowledged} />
                <span
                  >{$translation(
                    resourceAssurance === "verified_observation_only"
                      ? "resources-live-ack-verified"
                      : "resources-live-ack-modded",
                  )}</span
                >
              </label>
              <button
                type="button"
                disabled={resourceBusy || !resourceAcknowledged}
                onclick={() => void setResourceRegistryEnabled(true)}
                >{$translation("resources-live-enable")}</button
              >
            {:else}
              <button
                type="button"
                disabled={resourceBusy}
                onclick={() => void setResourceRegistryEnabled(false)}
                >{$translation("resources-live-disable")}</button
              >
            {/if}
            <button type="button" onclick={onopenresearch}
              >{$translation("research-setup-open")}</button
            >
          </div>
          <GuidanceSurface kind="boundary" layout="compact">
            <strong>{$translation("resources-live-boundary")}</strong>
            <span>{$translation("resources-live-boundary-detail")}</span>
          </GuidanceSurface>
          {#if resourceMessage}
            <p class="resource-message" role="alert">{resourceMessage}</p>
          {/if}
        </div>

        <form
          class="resource-filter"
          onsubmit={(event) => {
            event.preventDefault();
            void loadResourceCatalogue();
          }}
        >
          <label>
            <span>{$translation("resources-search")}</span>
            <input
              bind:value={resourceQuery}
              placeholder={$translation("resources-search-placeholder")}
            />
          </label>
          <label>
            <span>{$translation("resources-origin")}</span>
            <select bind:value={resourceOrigin}>
              <option value="">{$translation("resources-origin-all")}</option>
              <option value="installed_content"
                >{$translation("resources-origin-installed")}</option
              >
              <option value="recorded_save"
                >{$translation("resources-origin-save")}</option
              >
              <option value="live_game"
                >{$translation("resources-origin-live")}</option
              >
              <option value="player_overlay"
                >{$translation("resources-origin-overlay")}</option
              >
            </select>
          </label>
          <button type="submit">{$translation("catalogue-search")}</button>
        </form>

        <div class="resource-browser">
          <div class="resource-list" role="list">
            {#each resourceCatalogue?.entries ?? [] as resource}
              <button
                type="button"
                class:selected={resourceDetails?.entry.resource_id ===
                  resource.resource_id}
                onclick={() => void selectResource(resource.resource_id)}
              >
                <span>
                  <strong>{resource.display_name}</strong>
                  <code>{resource.source_token}</code>
                </span>
                <span class="resource-origins">
                  {#if resource.origin.installed_content}<small
                      >{$translation("resources-origin-installed")}</small
                    >{/if}
                  {#if resource.origin.recorded_save}<small
                      >{$translation("resources-origin-save")}</small
                    >{/if}
                  {#if resource.origin.live_game}<small
                      >{$translation("resources-origin-live")}</small
                    >{/if}
                  {#if resource.origin.player_overlay}<small
                      >{$translation("resources-origin-overlay")}</small
                    >{/if}
                </span>
              </button>
            {/each}
          </div>
          <article class="resource-details">
            {#if resourceDetails}
              <header>
                <div>
                  <span class="eyebrow"
                    >{$translation("resources-details-eyebrow")}</span
                  >
                  <h3>{resourceDetails.entry.display_name}</h3>
                  <code>{resourceDetails.entry.source_token}</code>
                </div>
                {#if resourceDetails.entry.origin.runtime_extension}
                  <span class="status-chip" data-status="watch"
                    >{$translation("resources-runtime-added")}</span
                  >
                {/if}
              </header>
              <dl>
                <div>
                  <dt>{$translation("resources-installed-references")}</dt>
                  <dd>
                    {resourceDetails.entry.origin.installed_reference_count}
                  </dd>
                </div>
                <div>
                  <dt>{$translation("resources-live-index")}</dt>
                  <dd>{resourceDetails.entry.live_index ?? "—"}</dd>
                </div>
                <div>
                  <dt>{$translation("resources-caption-id")}</dt>
                  <dd>{resourceDetails.entry.caption_id ?? "—"}</dd>
                </div>
              </dl>
              {#if resourceDetails.entry.live_prices.length}
                <div class="live-price-grid">
                  {#each resourceDetails.entry.live_prices as price}
                    <article>
                      <strong>{price.currency}</strong>
                      <span
                        >{$translation("resources-live-buy")}: {formatNumber(
                          price.buy_quote,
                          $activeLocale,
                        )}</span
                      >
                      <span
                        >{$translation("resources-live-sell")}: {formatNumber(
                          price.sell_quote,
                          $activeLocale,
                        )}</span
                      >
                      <small
                        >{$translation("resources-live-finished")}: {formatNumber(
                          price.finished_price,
                          $activeLocale,
                        )}</small
                      >
                    </article>
                  {/each}
                </div>
              {:else}
                <GuidanceSurface kind="instruction" layout="compact">
                  <strong>{$translation("resources-live-price-none")}</strong>
                  <span>{$translation("resources-live-price-none-detail")}</span
                  >
                </GuidanceSurface>
              {/if}
            {:else}
              <p>{$translation("resources-empty")}</p>
            {/if}
          </article>
        </div>
      </section>

      <section class="catalogue-browser" id="catalogue-browser">
        <header class="panel-heading">
          <div>
            <span class="eyebrow"
              >{$translation("catalogue-search-eyebrow")}</span
            >
            <h2>{$translation("catalogue-browser")}</h2>
          </div>
          <span class="coverage">{page?.total ?? 0}</span>
        </header>
        <form
          class="catalogue-filter"
          onsubmit={(event) => {
            event.preventDefault();
            void runSearch();
          }}
        >
          <input
            bind:value={query}
            placeholder={$translation("catalogue-search-placeholder")}
            aria-label={$translation("catalogue-search-placeholder")}
          />
          <select
            bind:value={kind}
            aria-label={$translation("catalogue-kind-filter")}
            ><option value="">{$translation("catalogue-all-kinds")}</option
            ><option value="resource"
              >{$translation("catalogue-resources")}</option
            ><option value="building"
              >{$translation("catalogue-buildings")}</option
            ><option value="vehicle"
              >{$translation("catalogue-vehicles")}</option
            ></select
          >
          <select
            bind:value={sourceKind}
            aria-label={$translation("catalogue-source-filter")}
            ><option value="">{$translation("catalogue-all-sources")}</option
            ><option value="base"
              >{$translation("catalogue-source-base")}</option
            ><option value="dlc">{$translation("catalogue-source-dlc")}</option
            ><option value="workshop"
              >{$translation("catalogue-source-workshop")}</option
            ><option value="wip">{$translation("catalogue-source-wip")}</option
            ><option value="derived"
              >{$translation("catalogue-source-derived")}</option
            ></select
          >
          <input
            bind:value={packageQuery}
            placeholder={$translation("catalogue-package-filter")}
            aria-label={$translation("catalogue-package-filter")}
          />
          <select
            bind:value={coverage}
            aria-label={$translation("catalogue-coverage-filter")}
            ><option value="">{$translation("catalogue-all-coverage")}</option
            ><option value="complete"
              >{$translation("coverage-complete")}</option
            ><option value="partial">{$translation("coverage-partial")}</option
            ></select
          >
          <input
            bind:value={availableYear}
            type="number"
            min="1800"
            max="3000"
            placeholder={$translation("catalogue-availability-filter")}
            aria-label={$translation("catalogue-availability-filter")}
          />
          <button type="submit">{$translation("catalogue-search")}</button>
        </form>
        <div class="catalogue-table" role="list">
          {#each page?.items ?? [] as item}
            <button
              type="button"
              class:selected={dossier?.summary.entity_id === item.entity_id}
              onclick={() => void chooseEntity(item.entity_id)}
            >
              <span class="entity-kind">{item.entity_kind}</span><strong
                >{item.display_name}</strong
              ><small>{item.package_name}</small><i
                >{item.property_count + item.relation_count}</i
              >
            </button>
          {/each}
        </div>
      </section>

      <section class="definition-dossier" id="definition-dossier">
        <header class="panel-heading">
          <div>
            <span class="eyebrow">{$translation("catalogue-dossier")}</span>
            <h2>
              {dossier?.summary.display_name ??
                $translation("catalogue-select-entity")}
            </h2>
            <p>{dossier?.summary.package_name ?? ""}</p>
          </div>
          {#if dossier}<span
              class="status-chip"
              data-status={dossier.summary.coverage === "complete"
                ? "stable"
                : "watch"}>{dossier.summary.coverage}</span
            >{/if}
        </header>
        {#if dossier}
          <div class="fact-ledger">
            {#each dossier.facts as fact}
              <article class:conflict={fact.conflict_code}>
                <code
                  >{fact.field_id}{fact.occurrence
                    ? ` [${fact.occurrence + 1}]`
                    : ""}</code
                >
                <div>
                  <span>{displayValue(fact.original)}</span><b
                    aria-hidden="true">→</b
                  ><span>{displayValue(fact.override_value)}</span><b
                    aria-hidden="true">→</b
                  ><strong>{displayValue(fact.effective)}</strong>
                </div>
                <small
                  >{fact.evidence_kind} · {fact.source_directive}{fact.source_line
                    ? `:${fact.source_line}`
                    : ""}{fact.conflict_code
                    ? ` · ${fact.conflict_code}`
                    : ""}</small
                >
                <small
                  class:mapping-warning={fact.mapping.scope_state ===
                    "updated_unreviewed"}
                  >{mappingLabel(fact.mapping.mapping_classification)} ·
                  {fact.mapping.mapping_id}{fact.mapping.catalogue_scope_id
                    ? ` · ${fact.mapping.catalogue_scope_id} · ${scopeStateLabel(fact.mapping.scope_state)}`
                    : ""}</small
                >
              </article>
            {/each}
          </div>
          <div class="relation-ledger">
            {#each dossier.relations as relation}<article
                class:unresolved={relation.resolution === "unresolved_auto"}
              >
                <span>{relation.relation_kind}</span><strong
                  >{relation.target_id}</strong
                ><small
                  >{relation.quantity ?? "?"}
                  {relation.unit ?? ""} · {relation.phase_id ??
                    $translation("catalogue-no-phase")}</small
                >
                <small
                  class:mapping-warning={relation.mapping.scope_state ===
                    "updated_unreviewed"}
                  >{mappingLabel(relation.mapping.mapping_classification)} ·
                  {relation.mapping.mapping_id}{relation.mapping
                    .catalogue_scope_id
                    ? ` · ${relation.mapping.catalogue_scope_id} · ${scopeStateLabel(relation.mapping.scope_state)}`
                    : ""}</small
                >
              </article>{/each}
          </div>
          {#if ["building", "vehicle"].includes(dossier.summary.entity_kind) && !hasRepairFacts}<p
              class="unavailable-fact"
            >
              {$translation("catalogue-repair-unavailable")}
            </p>{/if}
          {#if dossier.unknown_directives.length}<details>
              <summary
                >{$translation("catalogue-unknown-directives")} ({dossier
                  .unknown_directives.length})</summary
              >
              <div class="directive-list">
                {#each dossier.unknown_directives as item}<code
                    >{item.directive} × {item.occurrence_count}</code
                  >{/each}
              </div>
            </details>{/if}
        {/if}
      </section>

      <section class="overlay-laboratory" id="overlay-laboratory">
        <header class="panel-heading">
          <div>
            <span class="eyebrow"
              >{$translation("catalogue-player-definitions")}</span
            >
            <h2>{$translation("catalogue-overlays")}</h2>
            <p>{$translation("catalogue-overlay-description")}</p>
          </div>
        </header>
        <div class="overlay-profiles">
          {#each profiles as profile}<article class:active={profile.active}>
              <div>
                <strong>{profile.display_name}</strong><code
                  >{profile.profile_id} · v{profile.latest_revision}</code
                >
              </div>
              <span
                >{profile.conflict_count}
                {$translation("catalogue-conflicts")}</span
              >
              <div class="profile-actions">
                <button
                  disabled={busy || profile.active}
                  onclick={() =>
                    runAction(() =>
                      activatePlanningOverlay(profile.profile_id),
                    )}>{$translation("catalogue-activate")}</button
                ><button
                  disabled={busy || profile.revision_count < 2}
                  onclick={() =>
                    runAction(() =>
                      rollbackPlanningOverlay(profile.profile_id),
                    )}>{$translation("catalogue-rollback")}</button
                ><button disabled={busy} onclick={() => reviewProfile(profile)}
                  >{$translation("catalogue-review-rebase")}</button
                ><button
                  disabled={busy}
                  onclick={() => downloadProfile(profile)}
                  >{$translation("catalogue-export")}</button
                ><button
                  disabled={busy}
                  onclick={() =>
                    runAction(() => removePlanningOverlay(profile.profile_id))}
                  >{$translation("catalogue-remove")}</button
                >
              </div>
            </article>{/each}
          {#if status?.active_overlay}<button
              class="deactivate"
              disabled={busy}
              onclick={() => runAction(deactivatePlanningOverlay)}
              >{$translation("catalogue-deactivate")}</button
            >{/if}
        </div>
        <div class="overlay-editor">
          <section id="overlay-workbench">
            <h3>{$translation("catalogue-guided-supplement")}</h3>
            <div class="overlay-fields">
              <label
                >{$translation("catalogue-profile-id")}<input
                  bind:value={profileId}
                /></label
              ><label
                >{$translation("catalogue-profile-name")}<input
                  bind:value={profileName}
                /></label
              ><label
                >{$translation("catalogue-author")}<input
                  bind:value={author}
                /></label
              ><label
                >{$translation("catalogue-description")}<input
                  bind:value={description}
                /></label
              ><label
                >{$translation("catalogue-entity-kind")}<select
                  bind:value={supplementKind}
                  ><option value="resource"
                    >{$translation("catalogue-resources")}</option
                  ><option value="building"
                    >{$translation("catalogue-buildings")}</option
                  ><option value="vehicle"
                    >{$translation("catalogue-vehicles")}</option
                  ><option value="recipe"
                    >{$translation("catalogue-recipes")}</option
                  ></select
                ></label
              ><label
                >{$translation("catalogue-local-id")}<input
                  bind:value={supplementId}
                /></label
              ><label
                >{$translation("catalogue-display-name")}<input
                  bind:value={supplementName}
                /></label
              >
            </div>
            <button onclick={createSupplementDocument}
              >{$translation("catalogue-create-draft")}</button
            >
          </section>
          <section>
            <h3>{$translation("catalogue-json-workbench")}</h3>
            <div class="file-picker-row">
              <FilePicker
                id="overlay-file-input"
                accept=".json,.rooverlay.json,application/json"
                label={$translation("catalogue-choose-overlay-file")}
                emptyLabel={$translation("catalogue-no-file-selected")}
                onselect={loadOverlayFile}
              />
            </div>
            <textarea
              bind:value={overlayText}
              spellcheck="false"
              aria-label={$translation("catalogue-json-workbench")}></textarea>
            <div class="workbench-actions">
              <button disabled={!overlayText} onclick={inspectOverlay}
                >{$translation("catalogue-inspect")}</button
              ><button
                disabled={!inspection?.valid || busy}
                onclick={importOverlay}
                >{$translation("catalogue-import")}</button
              >
            </div>
            {#if inspection}<p class:valid={inspection.valid}>
                {inspection.valid
                  ? $translation("catalogue-valid-overlay")
                  : `${$translation("catalogue-invalid-overlay")}: ${inspection.code}`}
              </p>{/if}
          </section>
        </div>
      </section>
    {/if}
    {#if message}<p class="catalogue-message" role="alert">{message}</p>{/if}
  </section>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-focusable scroll region) -->
  <aside
    class="inspector"
    role="region"
    tabindex="0"
    aria-label={$translation("catalogue-health-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("catalogue-observer-health")}</span>
        <h2>SQLite → DuckDB</h2>
      </div>
      <span
        class="status-chip"
        data-status={status?.warehouse.phase === "ready" ? "stable" : "watch"}
        >{status?.warehouse.phase ?? "—"}</span
      >
    </div>
    <div class="selected-reading">
      <span>{$translation("catalogue-analytical-lag")}</span><strong
        >{formatLag(status?.warehouse.lag_ms)}</strong
      ><small
        >{status?.warehouse.pending_jobs ?? 0}
        {$translation("catalogue-pending-jobs")} ·
        {status?.warehouse.failed_jobs ?? 0}
        {$translation("catalogue-failed-jobs")}</small
      >
      <p>{$translation("catalogue-recorder-independent")}</p>
    </div>
    <section class="evidence-ledger">
      <span class="eyebrow"
        >{$translation("catalogue-generation-evidence")}</span
      >
      <div>
        <strong>{$translation("catalogue-game-build")}</strong><span
          >{status?.generation?.game_build_id ??
            $translation("catalogue-unavailable")}</span
        >
      </div>
      <div>
        <strong>{$translation("catalogue-parser")}</strong><span
          >{status?.generation?.parser_version ?? "—"}</span
        >
      </div>
      <div>
        <strong>{$translation("compatibility-profile-evidence")}</strong><span
          >{status?.generation
            ? `${status.generation.mapping_classification === "player_mapped" ? $translation("compatibility-player-mapped") : $translation("compatibility-reviewed")} · ${status.generation.compatibility_profile_id} v${status.generation.compatibility_profile_version}`
            : "—"}</span
        >
      </div>
      <div>
        <strong>{$translation("catalogue-watermark")}</strong><span
          >{status?.warehouse.observation_watermark?.slice(0, 12) ?? "—"}</span
        >
      </div>
      <div>
        <strong>{$translation("catalogue-last-projection")}</strong><span
          >{formatTimestamp(status?.warehouse.last_projected_at_ms)}</span
        >
      </div>
      <div>
        <strong>{$translation("catalogue-active-write")}</strong><span
          >{status?.warehouse.active_write
            ? warehouseActivityLabel(status.warehouse.active_write)
            : $translation("catalogue-none")}</span
        >
      </div>
      <div>
        <strong>{$translation("catalogue-write-failures")}</strong><span
          >{status?.warehouse.consecutive_write_failures ?? 0}</span
        >
      </div>
      <div>
        <strong>{$translation("catalogue-retry-protection")}</strong><span
          >{status?.warehouse.retry_after_ms
            ? `${formatNumber(Math.ceil(status.warehouse.retry_after_ms / 1000), $activeLocale)} s`
            : $translation("catalogue-current")}</span
        >
      </div>
    </section>
    <div class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("catalogue-auto-cost-warning")}</p>
    </div>
  </aside>
</section>

<style>
  .catalogue-actions,
  .profile-actions,
  .workbench-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  button,
  input,
  select,
  textarea {
    border: 1px solid var(--colour-line-faint);
    color: var(--colour-text);
    background: var(--colour-surface-raised);
  }
  button {
    padding: 8px 11px;
    cursor: pointer;
  }
  button:disabled {
    cursor: default;
    opacity: 0.45;
  }
  .catalogue-health {
    display: grid;
    grid-template-columns: repeat(5, minmax(100px, 1fr));
    gap: 8px;
    margin-bottom: 10px;
  }
  .catalogue-health article {
    padding: 12px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface);
  }
  .catalogue-health span,
  .catalogue-health strong {
    display: block;
  }
  .catalogue-health span {
    color: var(--colour-muted);
    font-size: var(--type-caption);
    text-transform: uppercase;
  }
  .catalogue-health strong {
    margin-top: 5px;
    color: var(--colour-observed);
    font-family: Georgia, serif;
    font-size: 20px;
  }
  .catalogue-browser,
  .resource-catalogue,
  .definition-dossier,
  .overlay-laboratory,
  .empty-catalogue {
    margin-bottom: 10px;
    padding: 14px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface);
  }
  .resource-registry-card {
    display: grid;
    grid-template-columns: minmax(280px, 1fr) auto;
    gap: 12px;
    align-items: start;
    margin: 12px 0;
    padding: 12px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
  }
  .resource-registry-card h3,
  .resource-registry-card p {
    margin: 3px 0 0;
  }
  .resource-registry-card :global(.guidance-surface),
  .resource-registry-controls,
  .resource-message {
    grid-column: 1 / -1;
  }
  .resource-registry-controls {
    display: flex;
    gap: 8px;
    align-items: end;
    flex-wrap: wrap;
  }
  .resource-registry-controls label:not(.resource-acknowledgement),
  .resource-filter label {
    display: grid;
    gap: 4px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }
  .resource-registry-controls select,
  .resource-filter input,
  .resource-filter select {
    min-height: 34px;
    padding: 7px;
  }
  .resource-acknowledgement {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 600px;
  }
  .resource-acknowledgement input {
    width: 18px;
    height: 18px;
    flex: 0 0 auto;
  }
  .resource-message {
    padding: 9px;
    border: 1px solid var(--colour-risk);
    color: var(--colour-risk);
  }
  .resource-filter {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) 220px auto;
    gap: 8px;
    align-items: end;
    margin: 12px 0;
  }
  .resource-browser {
    display: grid;
    grid-template-columns: minmax(280px, 0.8fr) minmax(360px, 1.2fr);
    gap: 10px;
  }
  .resource-list {
    max-height: 420px;
    overflow: auto;
    border: 1px solid var(--colour-line-faint);
  }
  .resource-list > button {
    display: grid;
    grid-template-columns: minmax(150px, 1fr) minmax(120px, auto);
    gap: 8px;
    width: 100%;
    border: 0;
    border-bottom: 1px solid var(--colour-line-faint);
    text-align: start;
  }
  .resource-list > button.selected {
    box-shadow: inset 3px 0 var(--colour-gold);
    background: var(--colour-gold-soft);
  }
  .resource-list strong,
  .resource-list code {
    display: block;
  }
  .resource-list code {
    margin-top: 4px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }
  .resource-origins {
    display: flex;
    justify-content: flex-end;
    gap: 4px;
    flex-wrap: wrap;
  }
  .resource-origins small {
    align-self: center;
    padding: 3px 5px;
    border: 1px solid var(--colour-line-faint);
    color: var(--colour-observed);
  }
  .resource-details {
    min-width: 0;
    padding: 12px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
  }
  .resource-details > header {
    display: flex;
    justify-content: space-between;
    gap: 10px;
  }
  .resource-details h3 {
    margin: 3px 0;
  }
  .resource-details dl,
  .live-price-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(100px, 1fr));
    gap: 7px;
    margin: 12px 0 0;
  }
  .resource-details dl div,
  .live-price-grid article {
    padding: 9px;
    border: 1px solid var(--colour-line-faint);
  }
  .resource-details dt,
  .live-price-grid small {
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }
  .resource-details dd {
    margin: 4px 0 0;
  }
  .live-price-grid article > * {
    display: block;
    margin-top: 3px;
  }
  .flow-laboratory {
    margin-bottom: 10px;
    scroll-margin-top: 18px;
  }
  .catalogue-filter {
    display: grid;
    grid-template-columns: minmax(200px, 1fr) 180px auto;
    gap: 8px;
    margin: 12px 0;
  }
  .catalogue-filter input,
  .catalogue-filter select,
  .overlay-fields input,
  .overlay-fields select {
    min-width: 0;
    padding: 8px;
  }
  .catalogue-table {
    max-height: 280px;
    overflow: auto;
    border-block: 1px solid var(--colour-line-faint);
  }
  .unavailable-fact {
    padding: 10px;
    border: 1px dashed var(--colour-line-faint);
    color: var(--colour-text-muted);
  }
  .catalogue-table button {
    width: 100%;
    display: grid;
    grid-template-columns: 80px minmax(180px, 1.2fr) minmax(160px, 1fr) 50px;
    align-items: center;
    gap: 10px;
    border: 0;
    border-bottom: 1px solid var(--colour-line-faint);
    text-align: start;
  }
  .catalogue-table button.selected {
    box-shadow: inset 3px 0 var(--colour-gold);
    background: var(--colour-gold-soft);
  }
  .catalogue-table small,
  .catalogue-table i,
  .entity-kind {
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }
  .entity-kind {
    text-transform: uppercase;
  }
  .catalogue-table i {
    font-style: normal;
    text-align: end;
  }
  .fact-ledger {
    display: grid;
    grid-template-columns: repeat(2, minmax(250px, 1fr));
    gap: 7px;
    margin-top: 12px;
  }
  .fact-ledger article {
    padding: 10px;
    border: 1px solid var(--colour-line-faint);
  }
  .fact-ledger article.conflict {
    border-color: var(--colour-risk);
  }
  .fact-ledger code,
  .fact-ledger small {
    display: block;
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }
  .fact-ledger div {
    display: grid;
    grid-template-columns: 1fr auto 1fr auto 1fr;
    gap: 7px;
    align-items: center;
    margin: 7px 0;
  }
  .fact-ledger b {
    color: var(--colour-gold);
  }
  .relation-ledger {
    display: grid;
    grid-template-columns: repeat(3, minmax(180px, 1fr));
    gap: 7px;
    margin-top: 10px;
  }
  .relation-ledger article {
    padding: 9px;
    border-inline-start: 2px solid var(--colour-observed);
    background: var(--colour-surface-raised);
  }
  .relation-ledger article.unresolved {
    border-inline-start-color: var(--colour-gold);
  }
  .relation-ledger span,
  .relation-ledger strong,
  .relation-ledger small {
    display: block;
  }
  .relation-ledger span,
  .relation-ledger small {
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }
  .fact-ledger small.mapping-warning,
  .relation-ledger small.mapping-warning {
    margin-top: 5px;
    color: var(--colour-gold);
  }
  details {
    margin-top: 10px;
    color: var(--colour-muted);
  }
  .directive-list {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 8px;
  }
  .directive-list code {
    padding: 5px;
    background: var(--colour-surface-raised);
  }
  .overlay-profiles {
    display: grid;
    gap: 7px;
    margin: 12px 0;
  }
  .overlay-profiles article {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) auto minmax(250px, auto);
    gap: 10px;
    align-items: center;
    padding: 10px;
    border: 1px solid var(--colour-line-faint);
  }
  .overlay-profiles article.active {
    border-inline-start: 3px solid var(--colour-gold);
  }
  .overlay-profiles code {
    display: block;
    margin-top: 3px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }
  .overlay-editor {
    display: grid;
    grid-template-columns: minmax(280px, 0.8fr) minmax(360px, 1.2fr);
    gap: 10px;
  }
  .overlay-editor > section {
    padding: 12px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
  }
  .overlay-fields {
    display: grid;
    grid-template-columns: repeat(2, minmax(130px, 1fr));
    gap: 8px;
    margin: 10px 0;
  }
  .overlay-fields label {
    display: grid;
    gap: 4px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
    text-transform: uppercase;
  }
  .file-picker-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 0 8px;
    min-width: 0;
  }
  textarea {
    width: 100%;
    height: 220px;
    margin: 8px 0;
    padding: 9px;
    resize: vertical;
    font:
      12px ui-monospace,
      monospace;
  }
  .overlay-editor p {
    margin-top: 8px;
    color: var(--colour-risk);
  }
  .overlay-editor p.valid {
    color: var(--colour-observed);
  }
  .catalogue-message {
    position: sticky;
    bottom: 0;
    padding: 10px;
    color: var(--colour-risk);
    background: var(--colour-surface);
    border: 1px solid var(--colour-risk);
  }
  .empty-catalogue {
    min-height: 240px;
    display: grid;
    place-content: center;
    gap: 8px;
    text-align: center;
  }
  @media (max-width: 1180px) {
    .catalogue-health {
      grid-template-columns: repeat(3, 1fr);
    }
    .fact-ledger,
    .overlay-editor,
    .resource-browser {
      grid-template-columns: 1fr;
    }
    .relation-ledger {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  @media (max-width: 760px) {
    .catalogue-filter,
    .resource-filter,
    .catalogue-table button,
    .overlay-profiles article {
      grid-template-columns: 1fr;
    }
    .catalogue-health,
    .relation-ledger,
    .overlay-fields,
    .resource-details dl,
    .live-price-grid,
    .resource-registry-card {
      grid-template-columns: 1fr;
    }
  }
</style>
