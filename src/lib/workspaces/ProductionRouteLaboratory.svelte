<script lang="ts">
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatNumber } from "../i18n/format";
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import ProductionPathwayLaboratory from "./ProductionPathwayLaboratory.svelte";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import {
    destinationsForSubject,
    type ChartNavigationBinding,
    type RelatedDataDestination,
    type WorkspaceFilters,
  } from "../navigation/relatedData";
  import {
    createProductionRouteChart,
    productionResourceLabel,
    productionRouteUnit,
  } from "../presentation/productionRoute";
  import {
    getProductionRoute,
    getProductionRouteCoverage,
    searchCatalogue,
  } from "../observations/desktopClient";
  import type {
    DefinitionSummary,
    ProductionPathwayModel,
    ProductionRouteFlow,
    ProductionRouteCoverage,
    ProductionRouteModel,
    ProductionRouteStatus,
  } from "../observations/types";

  let {
    desktopAvailable,
    gameConfigured,
    generationId,
    overlayProfileName,
    overlayRevision,
    requestedResourceToken,
    onlocationchange,
    onrelatednavigate,
    reviewRoute = null,
    reviewPathway = null,
  } = $props<{
    desktopAvailable: boolean;
    gameConfigured: boolean;
    generationId: string | null;
    overlayProfileName: string | null;
    overlayRevision: number | null;
    requestedResourceToken?: string;
    onlocationchange?: (filters: WorkspaceFilters) => void;
    onrelatednavigate?: (
      destinations: RelatedDataDestination[],
      origin: HTMLElement | null,
    ) => void;
    reviewRoute?: ProductionRouteModel | null;
    reviewPathway?: ProductionPathwayModel | null;
  }>();

  let recipes = $state<DefinitionSummary[]>([]);
  let route = $state<ProductionRouteModel | null>(null);
  let coverage = $state<ProductionRouteCoverage | null>(null);
  let selectedRouteId = $state("");
  let selectedOutputId = $state("");
  let targetValue = $state("");
  let query = $state("");
  let busy = $state(false);
  let error = $state(false);
  let loadedSnapshot = $state<string | null>(null);
  let requestSequence = 0;
  let appliedResourceToken = "";
  const chart = $derived(
    route
      ? createProductionRouteChart(route, $translation, $activeLocale)
      : null,
  );
  const outputs = $derived(
    route?.flows.filter((flow) => flow.direction === "production_output") ?? [],
  );
  const auxiliaryFlows = $derived(
    route?.flows.filter((flow) => flow.basis_role === "auxiliary") ?? [],
  );
  const chartNavigation = $derived.by((): ChartNavigationBinding[] => {
    if (!chart || !route) return [];
    return route.flows
      .filter((flow) => flow.basis_role === "primary")
      .map((flow, pointIndex) => ({
        seriesId: chart.id,
        pointIndex,
        destinations: destinationsForSubject({
          kind: "resource",
          resourceToken: flow.resource_id,
        }),
      }));
  });

  $effect(() => {
    const snapshotIdentity = currentSnapshotIdentity();
    if (reviewRoute) {
      requestSequence += 1;
      loadedSnapshot = snapshotIdentity;
      recipes = [];
      route = reviewRoute;
      coverage = null;
      selectedRouteId = reviewRoute.route_id;
      selectedOutputId = reviewRoute.selected_output_resource_id ?? "";
      targetValue = reviewRoute.target_quantity?.toString() ?? "";
      busy = false;
      error = false;
    } else if (!desktopAvailable || !gameConfigured || !snapshotIdentity) {
      requestSequence += 1;
      loadedSnapshot = null;
      recipes = [];
      route = null;
      coverage = null;
      selectedRouteId = "";
      selectedOutputId = "";
      targetValue = "";
      busy = false;
      error = false;
    } else if (snapshotIdentity !== loadedSnapshot) {
      requestSequence += 1;
      loadedSnapshot = snapshotIdentity;
      recipes = [];
      route = null;
      coverage = null;
      selectedRouteId = "";
      selectedOutputId = "";
      targetValue = "";
      void loadRecipes(snapshotIdentity);
    }
  });

  $effect(() => {
    const requested = requestedResourceToken ?? "";
    const snapshotIdentity = currentSnapshotIdentity();
    if (
      !requested ||
      requested === appliedResourceToken ||
      !desktopAvailable ||
      !gameConfigured ||
      !snapshotIdentity
    )
      return;
    appliedResourceToken = requested;
    query = "";
    void loadRecipes(snapshotIdentity, normaliseResourceId(requested));
  });

  function normaliseResourceId(resourceToken: string): string {
    return resourceToken.startsWith("resource::")
      ? resourceToken
      : `resource::${resourceToken}`;
  }

  function currentSnapshotIdentity(): string | null {
    return generationId
      ? `${generationId}|${overlayProfileName ?? "none"}|${overlayRevision ?? 0}`
      : null;
  }

  function requestIsCurrent(
    request: number,
    snapshotIdentity: string,
  ): boolean {
    return (
      request === requestSequence &&
      currentSnapshotIdentity() === snapshotIdentity
    );
  }

  function statusLabel(status: ProductionRouteStatus): string {
    switch (status) {
      case "ready":
        return $translation("production-route-status-ready");
      case "ready_with_auxiliary":
        return $translation("production-route-status-ready-with-auxiliary");
      case "too_complex":
        return $translation("production-route-status-too-complex");
      case "no_output":
        return $translation("production-route-status-no-output");
      case "no_input":
        return $translation("production-route-status-no-input");
      case "missing_quantity":
        return $translation("production-route-status-missing-quantity");
      case "invalid_quantity":
        return $translation("production-route-status-invalid-quantity");
      case "missing_unit":
        return $translation("production-route-status-missing-unit");
      case "no_comparable_input":
        return $translation("production-route-status-no-comparable-input");
      case "duplicate_endpoint":
        return $translation("production-route-status-duplicate-endpoint");
    }
  }

  function directionLabel(flow: ProductionRouteFlow): string {
    if (flow.direction === "production_output")
      return $translation("production-route-direction-output");
    if (flow.direction === "waste_input")
      return $translation("production-route-direction-waste-input");
    return $translation("production-route-direction-input");
  }

  function quantity(value: number | null): string {
    return value == null ? "—" : formatNumber(value, $activeLocale);
  }

  function unitLabel(unit: string | null): string {
    return unit ? productionRouteUnit(unit, $translation) : "—";
  }

  function resourceLabel(flow: ProductionRouteFlow): string {
    return productionResourceLabel(
      flow.resource_id,
      flow.display_name,
      $translation,
    );
  }

  function basisLabel(flow: ProductionRouteFlow): string {
    return flow.basis_role === "primary"
      ? $translation("production-route-basis-primary")
      : $translation("production-route-basis-auxiliary");
  }

  function basisReason(flow: ProductionRouteFlow): string {
    if (flow.basis_role === "primary")
      return $translation("production-route-basis-primary-reason");
    return flow.basis_exclusion === "missing_unit"
      ? $translation("production-route-basis-missing-unit")
      : $translation("production-route-basis-different-unit");
  }

  async function loadRecipes(
    expectedSnapshot = currentSnapshotIdentity(),
    outputResourceId?: string,
  ): Promise<void> {
    if (!desktopAvailable || !generationId || !expectedSnapshot) return;
    const request = ++requestSequence;
    busy = true;
    error = false;
    try {
      const [page, nextCoverage] = await Promise.all([
        searchCatalogue({
          query: query || undefined,
          output_resource_id: outputResourceId,
          entity_kind: "recipe",
          limit: 100,
        }),
        getProductionRouteCoverage(),
      ]);
      if (!requestIsCurrent(request, expectedSnapshot)) return;
      coverage = nextCoverage;
      recipes = page.items;
      if (!recipes.some((item) => item.entity_id === selectedRouteId)) {
        selectedRouteId = recipes[0]?.entity_id ?? "";
      }
      if (selectedRouteId) {
        selectedOutputId = outputResourceId ?? "";
        await loadRoute(Boolean(outputResourceId), expectedSnapshot);
      } else route = null;
    } catch {
      if (requestIsCurrent(request, expectedSnapshot)) error = true;
    } finally {
      if (requestIsCurrent(request, expectedSnapshot)) busy = false;
    }
  }

  async function loadRoute(
    useSelection = true,
    expectedSnapshot = currentSnapshotIdentity(),
  ): Promise<void> {
    if (!selectedRouteId || !expectedSnapshot) return;
    const request = ++requestSequence;
    busy = true;
    error = false;
    try {
      const target = Number(targetValue);
      const next = await getProductionRoute({
        entity_id: selectedRouteId,
        output_resource_id:
          useSelection && selectedOutputId ? selectedOutputId : undefined,
        target_quantity:
          useSelection && Number.isFinite(target) && target > 0
            ? target
            : undefined,
      });
      if (!requestIsCurrent(request, expectedSnapshot)) return;
      route = next;
      selectedOutputId = next.selected_output_resource_id ?? "";
      targetValue = next.target_quantity?.toString() ?? "";
    } catch {
      if (requestIsCurrent(request, expectedSnapshot)) error = true;
    } finally {
      if (requestIsCurrent(request, expectedSnapshot)) busy = false;
    }
  }

  async function selectOutput(): Promise<void> {
    const selected = outputs.find(
      (flow) => flow.resource_id === selectedOutputId,
    );
    targetValue = selected?.source_quantity?.toString() ?? "";
    await loadRoute(true);
  }
</script>

<section class="production-route-laboratory" aria-labelledby="route-title">
  <header class="laboratory-heading">
    <div>
      <span class="eyebrow">{$translation("production-route-eyebrow")}</span>
      <h2 id="route-title">{$translation("production-route-title")}</h2>
      <p>{$translation("production-route-description")}</p>
    </div>
    {#if route}
      <span
        class:attention={!["ready", "ready_with_auxiliary"].includes(
          route.status,
        )}
        class="route-status"
      >
        {statusLabel(route.status)}
      </span>
    {/if}
  </header>

  {#if !reviewRoute && (!desktopAvailable || !gameConfigured)}
    <GuidanceSurface kind="help" layout="block" semanticRole="status">
      <strong>{$translation("production-route-no-catalogue")}</strong>
      <span>{$translation("production-route-unavailable-note")}</span>
    </GuidanceSurface>
  {:else if !generationId}
    <div class="route-empty">
      {$translation("production-route-no-catalogue")}
    </div>
  {:else}
    <form
      class="route-search"
      onsubmit={(event) => {
        event.preventDefault();
        void loadRecipes();
      }}
    >
      <label>
        <span>{$translation("production-route-search-label")}</span>
        <input
          bind:value={query}
          placeholder={$translation("production-route-search-placeholder")}
          maxlength="120"
        />
      </label>
      <button type="submit" disabled={busy}
        >{$translation("production-route-search-action")}</button
      >
    </form>

    {#if coverage}
      <div
        class="route-coverage"
        aria-label={$translation("production-route-coverage-label")}
      >
        <article>
          <span>{$translation("production-route-coverage-routes")}</span>
          <strong>{coverage.route_count}</strong>
        </article>
        <article>
          <span>{$translation("production-route-coverage-diagrammable")}</span>
          <strong>{coverage.diagrammable_count}</strong>
        </article>
        <article>
          <span>{$translation("production-route-coverage-auxiliary")}</span>
          <strong>{coverage.routes_with_auxiliary}</strong>
        </article>
        <article>
          <span>{$translation("production-route-coverage-unresolved")}</span>
          <strong
            >{coverage.unresolved_basis_relation_count} / {coverage.unquantified_relation_count}</strong
          >
        </article>
      </div>
    {/if}

    {#if recipes.length}
      <div class="route-controls">
        <label>
          <span>{$translation("production-route-selector")}</span>
          <select
            bind:value={selectedRouteId}
            disabled={busy}
            onchange={() => {
              onlocationchange?.({ catalogueEntityId: selectedRouteId });
              void loadRoute(false);
            }}
          >
            {#each recipes as recipe}
              <option value={recipe.entity_id}
                >{recipe.display_name} · {recipe.package_name}</option
              >
            {/each}
          </select>
        </label>
        <label>
          <span>{$translation("production-route-output")}</span>
          <select
            bind:value={selectedOutputId}
            disabled={busy || outputs.length === 0}
            onchange={() => {
              onlocationchange?.({ resourceToken: selectedOutputId });
              void selectOutput();
            }}
          >
            {#each outputs as output}
              <option value={output.resource_id}>{resourceLabel(output)}</option
              >
            {/each}
          </select>
        </label>
        <label>
          <span>{$translation("production-route-target")}</span>
          <input
            type="number"
            bind:value={targetValue}
            min="0.000001"
            max="1000000000"
            step="any"
            disabled={busy || outputs.length === 0}
          />
        </label>
        <button
          type="button"
          disabled={busy || !selectedOutputId || !targetValue}
          onclick={() => void loadRoute(true)}
          >{$translation("production-route-apply")}</button
        >
      </div>
    {/if}

    {#if busy}
      <div class="route-empty" aria-live="polite">
        {$translation("production-route-loading")}
      </div>
    {:else if error}
      <div class="route-notice attention" role="alert">
        {$translation("production-route-error")}
      </div>
    {:else if recipes.length === 0 && !route}
      <div class="route-empty">
        {$translation("production-route-no-routes")}
      </div>
    {:else if route}
      <div
        class="route-boundary guidance-surface"
        data-guidance-surface="boundary"
        data-guidance-layout="block"
        role="note"
      >
        <strong>{$translation("production-route-boundary-title")}</strong>
        <span>{$translation("production-route-boundary")}</span>
        <span>{$translation("production-route-overlay-boundary")}</span>
      </div>

      <div class="route-snapshot">
        <span>
          {$translation("production-route-snapshot", {
            generation: route.snapshot.catalogue_generation_id.slice(0, 12),
            profile: route.snapshot.compatibility_profile_id,
            version: route.snapshot.compatibility_profile_version,
          })}
        </span>
        <span>
          {overlayProfileName
            ? $translation("production-route-overlay", {
                profile: overlayProfileName,
                revision: overlayRevision ?? 0,
              })
            : $translation("production-route-no-overlay")}
        </span>
      </div>

      {#if chart}
        <ObservatoryChart
          spec={chart}
          eyebrow={$translation("production-route-chart-eyebrow")}
          navigation={chartNavigation}
          {onrelatednavigate}
        />
      {:else}
        <div class="route-notice attention">
          {statusLabel(route.status)} · {$translation(
            "production-route-table-fallback",
          )}
        </div>
      {/if}

      {#if auxiliaryFlows.length}
        <section
          class="auxiliary-requirements"
          aria-labelledby="route-auxiliary-title"
        >
          <header>
            <div>
              <span class="eyebrow"
                >{$translation("production-route-auxiliary-eyebrow")}</span
              >
              <h3 id="route-auxiliary-title">
                {$translation("production-route-auxiliary-heading")}
              </h3>
              <p>{$translation("production-route-auxiliary-description")}</p>
            </div>
            <span class="evidence-badge">{auxiliaryFlows.length}</span>
          </header>
          <div class="auxiliary-grid">
            {#each auxiliaryFlows as flow}
              <article>
                <div>
                  <button
                    type="button"
                    class="related-data-link"
                    onclick={(event) =>
                      onrelatednavigate?.(
                        destinationsForSubject({
                          kind: "resource",
                          resourceToken: flow.resource_id,
                        }),
                        event.currentTarget,
                      )}>{resourceLabel(flow)}</button
                  >
                  <code>{flow.resource_id}</code>
                </div>
                <span
                  >{quantity(flow.scaled_quantity ?? flow.source_quantity)}
                  {unitLabel(flow.unit)}</span
                >
                <small>{basisReason(flow)}</small>
              </article>
            {/each}
          </div>
        </section>
      {/if}

      <section class="route-evidence" aria-labelledby="route-evidence-title">
        <header>
          <div>
            <span class="eyebrow"
              >{$translation("production-route-evidence-eyebrow")}</span
            >
            <h3 id="route-evidence-title">
              {$translation("production-route-evidence-heading")}
            </h3>
            <p>{$translation("production-route-evidence-description")}</p>
          </div>
          <span class="evidence-badge">
            {route.mapping_classification === "player_mapped"
              ? $translation("compatibility-player-mapped")
              : $translation("compatibility-reviewed")}
          </span>
        </header>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard access for an intentionally scrollable evidence region) -->
        <div
          class="route-table-scroll"
          role="region"
          tabindex="0"
          aria-labelledby="route-evidence-title"
        >
          <table>
            <thead>
              <tr>
                <th>{$translation("production-route-column-direction")}</th>
                <th>{$translation("production-route-column-resource")}</th>
                <th>{$translation("production-route-column-source")}</th>
                <th>{$translation("production-route-column-scaled")}</th>
                <th>{$translation("production-route-column-unit")}</th>
                <th>{$translation("production-route-column-basis-role")}</th>
                <th>{$translation("production-route-column-evidence")}</th>
              </tr>
            </thead>
            <tbody>
              {#each route.flows as flow}
                <tr>
                  <td>{directionLabel(flow)}</td>
                  <th scope="row">
                    <button
                      type="button"
                      class="related-data-link"
                      onclick={(event) =>
                        onrelatednavigate?.(
                          destinationsForSubject({
                            kind: "resource",
                            resourceToken: flow.resource_id,
                          }),
                          event.currentTarget,
                        )}>{resourceLabel(flow)}</button
                    >
                    <code>{flow.resource_id}</code>
                  </th>
                  <td>{quantity(flow.source_quantity)}</td>
                  <td>{quantity(flow.scaled_quantity)}</td>
                  <td>{unitLabel(flow.unit)}</td>
                  <td>
                    <strong>{basisLabel(flow)}</strong>
                    <span>{basisReason(flow)}</span>
                  </td>
                  <td>
                    <strong>{flow.source_directive}</strong>
                    <span>
                      {$translation("production-route-line-mapping", {
                        line: flow.source_line,
                        mapping: flow.mapping.mapping_id,
                      })}
                    </span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>

      {#if ["ready", "ready_with_auxiliary"].includes(route.status)}
        <ProductionPathwayLaboratory
          rootRoute={route}
          initialPathway={reviewPathway}
        />
      {/if}
    {/if}
  {/if}
</section>

<style>
  .production-route-laboratory {
    display: grid;
    gap: 0.65rem;
    min-width: 0;
    max-width: 100%;
  }

  .production-route-laboratory > * {
    min-width: 0;
  }

  .laboratory-heading,
  .route-evidence header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  h2 {
    margin-top: 0.16rem;
    font-family: var(--font-display);
    font-size: clamp(1.12rem, 2vw, 1.55rem);
    font-weight: 500;
  }

  h3 {
    margin-top: 0.14rem;
    font-family: var(--font-display);
    font-size: 1.03rem;
    font-weight: 500;
  }

  p {
    margin-top: 0.22rem;
    color: var(--colour-muted);
    font-size: 0.8rem;
    line-height: 1.5;
  }

  .eyebrow {
    color: var(--colour-gold);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .route-status,
  .evidence-badge {
    flex: 0 0 auto;
    border: 1px solid rgba(123, 198, 216, 0.36);
    padding: 0.24rem 0.42rem;
    color: var(--colour-observed);
    font-size: 0.75rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .route-status.attention,
  .route-notice.attention {
    border-color: rgba(216, 184, 106, 0.55);
    color: var(--colour-gold);
  }

  .route-search,
  .route-controls {
    display: grid;
    gap: 0.55rem;
    align-items: end;
  }

  .route-search {
    grid-template-columns: minmax(12rem, 52rem) auto;
    justify-content: start;
    width: min(100%, 64rem);
  }

  .route-controls {
    grid-template-columns:
      minmax(14rem, 1.6fr) minmax(10rem, 1fr) minmax(8rem, 0.55fr)
      auto;
    border: 1px solid var(--colour-line-faint);
    background: rgba(13, 29, 39, 0.72);
    padding: 0.65rem;
    width: min(100%, 96rem);
  }

  .route-coverage {
    display: grid;
    grid-template-columns: repeat(4, minmax(8rem, 1fr));
    gap: 0.45rem;
  }

  .route-coverage article {
    border: 1px solid var(--colour-line-faint);
    background: rgba(18, 41, 55, 0.72);
    padding: 0.55rem 0.65rem;
  }

  .route-coverage span,
  .route-coverage strong {
    display: block;
  }

  .route-coverage span {
    color: var(--colour-muted);
    font-size: 0.75rem;
  }

  .route-coverage strong {
    margin-top: 0.18rem;
    color: var(--colour-observed);
    font-family: var(--font-display);
    font-size: 1.05rem;
    font-weight: 500;
  }

  label {
    display: grid;
    gap: 0.28rem;
    min-width: 0;
  }

  label span {
    color: var(--colour-muted);
    font-size: 0.75rem;
    font-weight: 650;
    letter-spacing: 0.04em;
  }

  input,
  select,
  button {
    min-height: 2.15rem;
    border: 1px solid var(--colour-line);
    border-radius: 0;
    background: var(--colour-surface-raised);
    color: var(--colour-text);
    font: inherit;
    font-size: 0.8rem;
  }

  input,
  select {
    min-width: 0;
    padding: 0.38rem 0.48rem;
  }

  button {
    padding: 0.38rem 0.72rem;
    color: var(--colour-gold);
    cursor: pointer;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .route-notice,
  .route-empty,
  .route-boundary,
  .route-snapshot {
    border: 1px solid var(--colour-line-faint);
    background: rgba(18, 41, 55, 0.72);
    padding: 0.6rem 0.7rem;
    color: var(--colour-muted);
    font-size: 0.8rem;
    line-height: 1.45;
  }

  .route-empty {
    text-align: center;
    padding: 1.2rem;
  }

  .route-boundary,
  .route-snapshot {
    display: grid;
    gap: 0.18rem;
  }

  .route-boundary strong {
    color: var(--colour-guidance);
  }

  .route-boundary.guidance-surface {
    border-color: var(--colour-guidance);
    border-inline-start: 3px solid var(--colour-guidance);
    background:
      linear-gradient(110deg, var(--colour-guidance-soft), transparent 76%),
      var(--colour-surface);
  }

  .route-snapshot {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    background: transparent;
  }

  .route-evidence,
  .auxiliary-requirements {
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface);
    padding: 0.7rem;
  }

  .auxiliary-requirements header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .auxiliary-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
    gap: 0.45rem;
    margin-top: 0.6rem;
  }

  .auxiliary-grid article {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.2rem 0.8rem;
    border-inline-start: 2px solid var(--colour-gold);
    background: var(--colour-surface-raised);
    padding: 0.55rem 0.65rem;
  }

  .auxiliary-grid div,
  .auxiliary-grid .related-data-link,
  .auxiliary-grid code,
  .auxiliary-grid small {
    display: block;
  }

  .auxiliary-grid code,
  tbody th code,
  .auxiliary-grid small {
    margin-top: 0.12rem;
    color: var(--colour-muted);
    font-size: 0.75rem;
    overflow-wrap: anywhere;
  }

  .auxiliary-grid small {
    grid-column: 1 / -1;
  }

  .route-table-scroll {
    overflow-x: auto;
    margin-top: 0.6rem;
  }

  table {
    width: 100%;
    min-width: 52rem;
    border-collapse: collapse;
    font-size: 0.78rem;
  }

  th,
  td {
    border-top: 1px solid var(--colour-line-faint);
    padding: 0.48rem;
    text-align: left;
    vertical-align: top;
  }

  thead th {
    border-top: 0;
    color: var(--colour-muted);
    font-size: 0.75rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  tbody th {
    color: var(--colour-text);
    font-weight: 600;
  }

  tbody th .related-data-link,
  tbody th code {
    display: block;
  }

  td strong,
  td span {
    display: block;
  }

  td span {
    margin-top: 0.16rem;
    color: var(--colour-muted);
    overflow-wrap: anywhere;
  }

  @media (max-width: 920px) {
    .route-controls {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .route-coverage {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 620px) {
    .laboratory-heading,
    .route-evidence header {
      display: grid;
    }

    .route-search,
    .route-controls,
    .route-snapshot,
    .route-coverage {
      grid-template-columns: 1fr;
    }

    button {
      width: 100%;
    }
  }
</style>
