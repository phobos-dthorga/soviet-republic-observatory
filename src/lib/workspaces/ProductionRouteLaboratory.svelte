<script lang="ts">
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatNumber } from "../i18n/format";
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import { createMaterialFlowPreview } from "../data/materialFlowPreview";
  import {
    createProductionRouteChart,
    productionResourceLabel,
    productionRouteUnit,
  } from "../data/productionRoute";
  import {
    getProductionRoute,
    getProductionRouteCoverage,
    searchCatalogue,
  } from "../observations/desktopClient";
  import type {
    DefinitionSummary,
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
  } = $props<{
    desktopAvailable: boolean;
    gameConfigured: boolean;
    generationId: string | null;
    overlayProfileName: string | null;
    overlayRevision: number | null;
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
  const chart = $derived(
    route
      ? createProductionRouteChart(route, $translation, $activeLocale)
      : null,
  );
  const syntheticPreview = $derived(createMaterialFlowPreview($translation));
  const outputs = $derived(
    route?.flows.filter((flow) => flow.direction === "production_output") ?? [],
  );
  const auxiliaryFlows = $derived(
    route?.flows.filter((flow) => flow.basis_role === "auxiliary") ?? [],
  );

  $effect(() => {
    const snapshotIdentity = currentSnapshotIdentity();
    if (!desktopAvailable || !gameConfigured || !snapshotIdentity) {
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
  ): Promise<void> {
    if (!desktopAvailable || !generationId || !expectedSnapshot) return;
    const request = ++requestSequence;
    busy = true;
    error = false;
    try {
      const [page, nextCoverage] = await Promise.all([
        searchCatalogue({
          query: query || undefined,
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
      if (selectedRouteId) await loadRoute(false, expectedSnapshot);
      else route = null;
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

  {#if !desktopAvailable || !gameConfigured}
    <div class="route-notice">
      {$translation("production-route-synthetic-note")}
    </div>
    <ObservatoryChart
      spec={syntheticPreview}
      eyebrow={$translation("catalogue-flow-eyebrow")}
    />
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
            onchange={() => void loadRoute(false)}
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
            onchange={() => void selectOutput()}
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
    {:else if recipes.length === 0}
      <div class="route-empty">
        {$translation("production-route-no-routes")}
      </div>
    {:else if route}
      <div class="route-boundary">
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
                  <strong>{resourceLabel(flow)}</strong>
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
        <div class="route-table-scroll">
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
                    <strong>{resourceLabel(flow)}</strong>
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
    {/if}
  {/if}
</section>

<style>
  .production-route-laboratory {
    display: grid;
    gap: 0.65rem;
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
    color: var(--muted);
    font-size: 0.8rem;
    line-height: 1.5;
  }

  .eyebrow {
    color: var(--gold);
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
    color: var(--cyan);
    font-size: 0.75rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .route-status.attention,
  .route-notice.attention {
    border-color: rgba(216, 184, 106, 0.55);
    color: var(--gold);
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
    border: 1px solid var(--line);
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
    border: 1px solid var(--line);
    background: rgba(18, 41, 55, 0.72);
    padding: 0.55rem 0.65rem;
  }

  .route-coverage span,
  .route-coverage strong {
    display: block;
  }

  .route-coverage span {
    color: var(--muted);
    font-size: 0.75rem;
  }

  .route-coverage strong {
    margin-top: 0.18rem;
    color: var(--cyan);
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
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 650;
    letter-spacing: 0.04em;
  }

  input,
  select,
  button {
    min-height: 2.15rem;
    border: 1px solid var(--line-strong);
    border-radius: 0;
    background: var(--panel-raised);
    color: var(--text);
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
    color: var(--gold);
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
    border: 1px solid var(--line);
    background: rgba(18, 41, 55, 0.72);
    padding: 0.6rem 0.7rem;
    color: var(--muted);
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
    color: var(--gold);
  }

  .route-snapshot {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    background: transparent;
  }

  .route-evidence,
  .auxiliary-requirements {
    border: 1px solid var(--line);
    background: var(--panel);
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
    border-inline-start: 2px solid var(--gold);
    background: var(--panel-raised);
    padding: 0.55rem 0.65rem;
  }

  .auxiliary-grid div,
  .auxiliary-grid strong,
  .auxiliary-grid code,
  .auxiliary-grid small {
    display: block;
  }

  .auxiliary-grid code,
  tbody th code,
  .auxiliary-grid small {
    margin-top: 0.12rem;
    color: var(--muted);
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
    border-top: 1px solid var(--line);
    padding: 0.48rem;
    text-align: left;
    vertical-align: top;
  }

  thead th {
    border-top: 0;
    color: var(--muted);
    font-size: 0.75rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  tbody th {
    color: var(--text);
    font-weight: 600;
  }

  tbody th strong,
  tbody th code {
    display: block;
  }

  td strong,
  td span {
    display: block;
  }

  td span {
    margin-top: 0.16rem;
    color: var(--muted);
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
