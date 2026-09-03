<script lang="ts">
  import { activeLocale, translation } from "../i18n/runtime";
  import { formatNumber } from "../i18n/format";
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import { getProductionPathway } from "../observations/desktopClient";
  import type {
    ProductionPathwayChoice,
    ProductionPathwayModel,
    ProductionPathwayStatus,
    ProductionRouteModel,
  } from "../observations/types";
  import { createProductionPathwayChart } from "../presentation/productionPathway";
  import {
    productionResourceLabel,
    productionRouteUnit,
  } from "../presentation/productionRoute";

  let { rootRoute, initialPathway = null } = $props<{
    rootRoute: ProductionRouteModel;
    initialPathway?: ProductionPathwayModel | null;
  }>();

  let pathway = $state<ProductionPathwayModel | null>(null);
  let selections = $state<Record<string, string>>({});
  let maxDepth = $state(4);
  let active = $state(false);
  let busy = $state(false);
  let error = $state(false);
  let loadedRootIdentity = $state("");
  let requestSequence = 0;

  const rootIdentity = $derived(
    `${rootRoute.snapshot.catalogue_generation_id}|${rootRoute.route_id}|${rootRoute.selected_output_resource_id ?? ""}|${rootRoute.target_quantity ?? 0}`,
  );
  const chart = $derived(
    pathway
      ? createProductionPathwayChart(pathway, $translation, $activeLocale)
      : null,
  );
  const choices = $derived.by(() => {
    const unique = new Map<string, ProductionPathwayChoice>();
    for (const choice of pathway?.choices ?? []) {
      const existing = unique.get(choice.resource_id);
      if (existing) {
        existing.required_quantity += choice.required_quantity;
      } else {
        unique.set(choice.resource_id, { ...choice });
      }
    }
    return [...unique.values()];
  });

  $effect(() => {
    if (rootIdentity !== loadedRootIdentity) {
      requestSequence += 1;
      loadedRootIdentity = rootIdentity;
      pathway = initialPathway;
      selections = {};
      active = initialPathway != null;
      busy = false;
      error = false;
    }
  });

  function statusLabel(status: ProductionPathwayStatus): string {
    switch (status) {
      case "ready":
        return $translation("production-pathway-status-ready");
      case "ready_with_auxiliary":
        return $translation("production-pathway-status-ready-with-auxiliary");
      case "needs_selection":
        return $translation("production-pathway-status-needs-selection");
      case "bounded":
        return $translation("production-pathway-status-bounded");
      case "too_complex":
        return $translation("production-pathway-status-too-complex");
    }
  }

  function reasonLabel(reason: string): string {
    switch (reason) {
      case "external_input":
        return $translation("production-pathway-reason-external-input");
      case "route_selection_required":
        return $translation(
          "production-pathway-reason-route-selection-required",
        );
      case "depth_limit":
        return $translation("production-pathway-reason-depth-limit");
      case "cycle":
        return $translation("production-pathway-reason-cycle");
      case "unsupported_route":
        return $translation("production-pathway-reason-unsupported-route");
      case "candidate_limit":
        return $translation("production-pathway-reason-candidate-limit");
      case "node_limit":
        return $translation("production-pathway-reason-node-limit");
      case "link_limit":
        return $translation("production-pathway-reason-link-limit");
      case "different_unit":
        return $translation("production-pathway-reason-different-unit");
      case "missing_unit":
        return $translation("production-pathway-reason-missing-unit");
      default:
        return $translation("production-pathway-reason-unsupported-route");
    }
  }

  function quantity(value: number | null, unit: string | null): string {
    if (value == null) return "—";
    return `${formatNumber(value, $activeLocale)} ${unit ? productionRouteUnit(unit, $translation) : ""}`.trim();
  }

  function resourceLabel(resourceId: string, fallback: string): string {
    return productionResourceLabel(resourceId, fallback, $translation);
  }

  async function loadPathway(): Promise<void> {
    if (
      !rootRoute.selected_output_resource_id ||
      rootRoute.target_quantity == null ||
      !["ready", "ready_with_auxiliary"].includes(rootRoute.status)
    )
      return;
    const request = ++requestSequence;
    active = true;
    busy = true;
    error = false;
    try {
      const next = await getProductionPathway({
        root_recipe_entity_id: rootRoute.route_id,
        output_resource_id: rootRoute.selected_output_resource_id,
        target_quantity: rootRoute.target_quantity,
        max_depth: maxDepth,
        selections: Object.entries(selections)
          .filter(([, recipe]) => recipe)
          .map(([resource_id, recipe_entity_id]) => ({
            resource_id,
            recipe_entity_id,
          })),
      });
      if (request !== requestSequence || rootIdentity !== loadedRootIdentity)
        return;
      pathway = next;
      for (const choice of next.choices) {
        if (choice.selected_recipe_entity_id) {
          selections[choice.resource_id] = choice.selected_recipe_entity_id;
        }
      }
      selections = { ...selections };
    } catch {
      if (request === requestSequence) error = true;
    } finally {
      if (request === requestSequence) busy = false;
    }
  }

  function chooseRoute(resourceId: string, recipeEntityId: string): void {
    if (recipeEntityId) selections[resourceId] = recipeEntityId;
    else delete selections[resourceId];
    selections = { ...selections };
    void loadPathway();
  }
</script>

<section class="pathway-laboratory" aria-labelledby="pathway-title">
  <header class="pathway-heading">
    <div>
      <span class="eyebrow">{$translation("production-pathway-eyebrow")}</span>
      <h3 id="pathway-title">{$translation("production-pathway-title")}</h3>
      <p>{$translation("production-pathway-description")}</p>
    </div>
    {#if pathway}
      <span
        class:attention={!pathway.status.startsWith("ready")}
        class="pathway-status"
      >
        {statusLabel(pathway.status)}
      </span>
    {/if}
  </header>

  <div
    class="pathway-boundary guidance-surface"
    data-guidance-surface="boundary"
    data-guidance-layout="compact"
    role="note"
  >
    <strong>{$translation("production-pathway-boundary-title")}</strong>
    <span>{$translation("production-pathway-boundary")}</span>
  </div>

  <div class="pathway-controls">
    <label>
      <span>{$translation("production-pathway-depth")}</span>
      <select
        bind:value={maxDepth}
        disabled={busy || initialPathway != null}
        onchange={() => active && void loadPathway()}
      >
        {#each [2, 3, 4, 5, 6] as depth}
          <option value={depth}>{depth}</option>
        {/each}
      </select>
    </label>
    <button
      type="button"
      disabled={busy || initialPathway != null}
      onclick={() => void loadPathway()}
    >
      {active
        ? $translation("production-pathway-rebuild")
        : $translation("production-pathway-build")}
    </button>
  </div>

  {#if !active}
    <div class="pathway-empty">
      {$translation("production-pathway-inactive")}
    </div>
  {:else if busy}
    <div class="pathway-empty" aria-live="polite">
      {$translation("production-pathway-loading")}
    </div>
  {:else if error}
    <div class="pathway-notice attention" role="alert">
      {$translation("production-pathway-error")}
    </div>
  {:else if pathway}
    {#if choices.length}
      <section class="route-choices" aria-labelledby="pathway-choice-title">
        <header>
          <div>
            <span class="eyebrow"
              >{$translation("production-pathway-choice-eyebrow")}</span
            >
            <h4 id="pathway-choice-title">
              {$translation("production-pathway-choice-title")}
            </h4>
            <p>{$translation("production-pathway-choice-description")}</p>
          </div>
          <span class="evidence-badge">{choices.length}</span>
        </header>
        <div class="choice-grid">
          {#each choices as choice}
            <label>
              <span>
                <strong
                  >{resourceLabel(
                    choice.resource_id,
                    choice.display_name,
                  )}</strong
                >
                <small>{quantity(choice.required_quantity, choice.unit)}</small>
              </span>
              <select
                value={selections[choice.resource_id] ?? ""}
                disabled={initialPathway != null}
                onchange={(event) =>
                  chooseRoute(choice.resource_id, event.currentTarget.value)}
              >
                <option value=""
                  >{$translation("production-pathway-choice-required")}</option
                >
                {#each choice.candidates as candidate}
                  <option value={candidate.recipe_entity_id}>
                    {candidate.display_name} · {candidate.package_name}
                  </option>
                {/each}
              </select>
            </label>
          {/each}
        </div>
      </section>
    {/if}

    {#if chart}
      <ObservatoryChart
        spec={chart}
        eyebrow={$translation("production-pathway-chart-eyebrow")}
      />
    {:else}
      <div class="pathway-notice attention">
        {$translation("production-pathway-chart-unavailable")}
      </div>
    {/if}

    <div class="pathway-ledgers">
      <section aria-labelledby="pathway-terminal-title">
        <header>
          <div>
            <span class="eyebrow"
              >{$translation("production-pathway-terminal-eyebrow")}</span
            >
            <h4 id="pathway-terminal-title">
              {$translation("production-pathway-terminal-title")}
            </h4>
          </div>
          <span class="evidence-badge"
            >{pathway.terminal_requirements.length}</span
          >
        </header>
        {#if pathway.terminal_requirements.length}
          <div class="ledger-list">
            {#each pathway.terminal_requirements as requirement}
              <article>
                <div>
                  <strong
                    >{resourceLabel(
                      requirement.resource_id,
                      requirement.display_name,
                    )}</strong
                  >
                  <code>{requirement.resource_id}</code>
                </div>
                <span>{quantity(requirement.quantity, requirement.unit)}</span>
                <small>{reasonLabel(requirement.reason)}</small>
              </article>
            {/each}
          </div>
        {:else}
          <p class="pathway-empty">
            {$translation("production-pathway-terminal-none")}
          </p>
        {/if}
      </section>

      <section aria-labelledby="pathway-auxiliary-title">
        <header>
          <div>
            <span class="eyebrow"
              >{$translation("production-pathway-auxiliary-eyebrow")}</span
            >
            <h4 id="pathway-auxiliary-title">
              {$translation("production-pathway-auxiliary-title")}
            </h4>
          </div>
          <span class="evidence-badge"
            >{pathway.auxiliary_requirements.length}</span
          >
        </header>
        {#if pathway.auxiliary_requirements.length}
          <div class="ledger-list">
            {#each pathway.auxiliary_requirements as requirement}
              <article>
                <div>
                  <strong
                    >{resourceLabel(
                      requirement.resource_id,
                      requirement.display_name,
                    )}</strong
                  >
                  <code>{requirement.recipe_entity_id}</code>
                </div>
                <span>{quantity(requirement.quantity, requirement.unit)}</span>
                <small>{reasonLabel(requirement.reason)}</small>
              </article>
            {/each}
          </div>
        {:else}
          <p class="pathway-empty">
            {$translation("production-pathway-auxiliary-none")}
          </p>
        {/if}
      </section>
    </div>

    {#if pathway.diagnostics.length}
      <div class="pathway-notice attention" role="status">
        <strong>{$translation("production-pathway-stops-title")}</strong>
        <span>
          {pathway.diagnostics
            .map((diagnostic) => reasonLabel(diagnostic.code))
            .join(" · ")}
        </span>
      </div>
    {/if}

    <details class="pathway-evidence">
      <summary
        >{$translation("production-pathway-evidence-title", {
          count: pathway.links.length,
        })}</summary
      >
      <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard access for an intentionally scrollable evidence region) -->
      <div
        class="pathway-table-scroll"
        role="region"
        tabindex="0"
        aria-labelledby="pathway-title"
      >
        <table>
          <thead>
            <tr>
              <th>{$translation("production-pathway-column-from")}</th>
              <th>{$translation("production-pathway-column-to")}</th>
              <th>{$translation("production-route-column-resource")}</th>
              <th>{$translation("production-route-column-scaled")}</th>
              <th>{$translation("production-route-column-evidence")}</th>
            </tr>
          </thead>
          <tbody>
            {#each pathway.links as link}
              <tr>
                <td><code>{link.source}</code></td>
                <td><code>{link.target}</code></td>
                <th scope="row"
                  >{resourceLabel(link.resource_id, link.resource_id)}</th
                >
                <td>{quantity(link.quantity, link.unit)}</td>
                <td>
                  <strong>{link.source_directive}</strong>
                  <span
                    >{$translation("production-route-line-mapping", {
                      line: link.source_line,
                      mapping: link.mapping.mapping_id,
                    })}</span
                  >
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </details>
  {/if}
</section>

<style>
  .pathway-laboratory {
    display: grid;
    gap: 0.75rem;
    min-width: 0;
    max-width: 100%;
    margin-top: 0.75rem;
    padding-top: 0.85rem;
    border-top: 1px solid var(--colour-line-faint);
  }

  .pathway-laboratory > *,
  .pathway-heading > div,
  .route-choices header > div,
  .pathway-ledgers header > div {
    min-width: 0;
  }

  .pathway-heading,
  .route-choices header,
  .pathway-ledgers header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  h3,
  h4,
  p {
    margin: 0;
  }

  .pathway-heading p,
  .route-choices p {
    color: var(--colour-muted);
    margin-top: 0.2rem;
    overflow-wrap: anywhere;
  }

  .pathway-status,
  .evidence-badge {
    flex: 0 0 auto;
    border: 1px solid var(--colour-observed);
    color: var(--colour-observed);
    padding: 0.25rem 0.45rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: var(--type-caption);
  }

  .pathway-status.attention,
  .pathway-notice.attention {
    border-color: var(--colour-risk);
    color: var(--colour-risk);
  }

  .pathway-boundary {
    display: grid;
    gap: 0.2rem;
  }

  .pathway-controls {
    display: flex;
    align-items: end;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .pathway-controls label,
  .choice-grid label {
    display: grid;
    gap: 0.3rem;
  }

  .pathway-controls label > span,
  .choice-grid small {
    color: var(--colour-muted);
  }

  .pathway-controls select {
    min-width: 8rem;
  }

  .pathway-empty,
  .pathway-notice {
    padding: 0.75rem;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-soft);
    color: var(--colour-muted);
  }

  .route-choices,
  .pathway-ledgers > section,
  .pathway-evidence {
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface);
    padding: 0.75rem;
  }

  .choice-grid,
  .pathway-ledgers {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.5rem;
    margin-top: 0.65rem;
  }

  .choice-grid label,
  .ledger-list article {
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
    padding: 0.65rem;
  }

  .choice-grid label > span,
  .ledger-list article,
  .ledger-list article > div {
    display: grid;
    gap: 0.2rem;
  }

  .ledger-list {
    display: grid;
    gap: 0.4rem;
    margin-top: 0.6rem;
  }

  code,
  small {
    overflow-wrap: anywhere;
    color: var(--colour-muted);
  }

  .pathway-notice {
    display: grid;
    gap: 0.25rem;
  }

  .pathway-evidence summary {
    cursor: pointer;
    font-weight: 700;
  }

  .pathway-table-scroll {
    overflow-x: auto;
    margin-top: 0.65rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: 0.55rem;
    text-align: left;
    vertical-align: top;
    border-bottom: 1px solid var(--colour-line-faint);
  }

  td:last-child {
    display: grid;
    gap: 0.2rem;
  }

  @media (max-width: 880px) {
    .choice-grid,
    .pathway-ledgers {
      grid-template-columns: 1fr;
    }

    .pathway-heading,
    .route-choices header,
    .pathway-ledgers header {
      flex-direction: column;
    }
  }
</style>
