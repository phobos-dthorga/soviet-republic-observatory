<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import { formatNumber } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import { containedSectionNavigation } from "../navigation/containedSectionNavigation";
  import { notify, type RecoveryProposal } from "../notifications/service";
  import {
    clearMarketSelection,
    getMarketPriceSeries,
    indexAvailableSavesForMarkets,
    refreshChangedMarketData,
    recoverMarketIndexing,
    removeMarketDefinition,
    rollbackMarketDefinition,
    saveMarketBasket,
    saveMarketScenario,
    selectMarketDefinition,
  } from "../observations/desktopClient";
  import type {
    MarketBasketDraft,
    MarketIndexingProgress,
    MarketPriceSeries,
    MarketScenarioDraft,
    MarketWorkspace,
  } from "../observations/types";
  import {
    createCityTradeChart,
    createMarketPriceHistoryChart,
    createMarketTradeChart,
    createPositiveExportChart,
    marketIndexingProgressView,
    marketMetricHelp,
  } from "../presentation/markets";
  import TaskProgressPanel from "../tasks/TaskProgressPanel.svelte";
  import ContextHelp from "../ui/ContextHelp.svelte";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";

  let {
    workspace = null,
    indexingProgress = null,
    desktopAvailable,
    onupdate,
    onprogress,
  }: {
    workspace?: MarketWorkspace | null;
    indexingProgress?: MarketIndexingProgress | null;
    desktopAvailable: boolean;
    onupdate: (workspace: MarketWorkspace) => void;
    onprogress: (progress: MarketIndexingProgress) => void;
  } = $props();

  let busy = $state(false);
  let selectedCurrency = $state<"rub" | "usd">("rub");
  let selectedChannel = $state<"standard" | "international">("standard");
  let ledgerFilter = $state("");
  let basketId = $state("local.market-basket");
  let basketName = $state("");
  let basketReason = $state("");
  let basketSide = $state<"purchase" | "sell">("purchase");
  let basketBase = $state("");
  let basketWeights = $state<Array<{ resource_token: string; weight: number }>>(
    [],
  );
  let weightResource = $state("");
  let weightValue = $state(1);
  let scenarioId = $state("local.market-scenario");
  let scenarioName = $state("");
  let scenarioReason = $state("");
  let scenarioKind = $state<"break_even" | "debt_stress">("break_even");
  let scenarioCurrency = $state<"rub" | "usd">("rub");
  let domesticCost = $state(0);
  let deliveryCost = $state(0);
  let efficiency = $state(100);
  let exchangeRate = $state<number | null>(null);
  let debtService = $state(0);
  let exportStress = $state(0);
  let tourismStress = $state(0);
  let includeStandardExports = $state(true);
  let includeInternationalExports = $state(false);
  let includeTourism = $state(false);
  let priceResource = $state("");
  let priceSeries = $state<MarketPriceSeries | null>(null);
  let priceSeriesLoading = $state(false);
  let priceRequest = 0;
  let smartDefaultContext = $state("");

  const pulse = $derived(
    workspace?.currencies.find(
      (entry) => entry.currency === selectedCurrency,
    ) ?? null,
  );
  const tradeChart = $derived(
    createMarketTradeChart(
      workspace ?? emptyWorkspace(),
      selectedCurrency,
      selectedChannel,
      $translation,
    ),
  );
  const exportChart = $derived(
    createPositiveExportChart(
      workspace ?? emptyWorkspace(),
      selectedCurrency,
      $translation,
    ),
  );
  const cityChart = $derived(
    createCityTradeChart(
      workspace ?? emptyWorkspace(),
      selectedCurrency,
      $translation,
    ),
  );
  const priceChart = $derived(
    createMarketPriceHistoryChart(
      workspace ?? emptyWorkspace(),
      priceSeries,
      $translation,
    ),
  );
  const pulseHelp = $derived(
    pulse ? marketMetricHelp(pulse.context, $translation) : null,
  );
  const concentrationHelp = $derived(
    workspace?.metric_contexts.find(
      (context) =>
        context.metric_id ===
        `market.positive_export_hhi.${selectedCurrency}.standard`,
    ),
  );
  const priceHelp = $derived(
    workspace?.metric_contexts.find(
      (context) => context.metric_id === `market.price.${selectedCurrency}`,
    ),
  );
  const scalarHelp = $derived(
    workspace?.metric_contexts.find(
      (context) => context.metric_id === "market.scalar_accounts",
    ),
  );
  const cityHelp = $derived(
    workspace?.metric_contexts.find(
      (context) =>
        context.metric_id ===
        `market.city_trade_result.${selectedCurrency}.standard`,
    ),
  );
  const indexView = $derived(
    indexingProgress
      ? marketIndexingProgressView(indexingProgress, $translation)
      : null,
  );
  const indexActionKey = $derived(
    indexingProgress?.phase === "paused"
      ? "markets-index-resume-action"
      : (workspace?.commissioning.current_engine_indexed_save_count ?? 0) > 0 ||
          indexingProgress?.phase === "complete"
        ? "markets-index-refresh-action"
        : "markets-index-action",
  );
  const baseRecords = $derived.by(() => {
    const seen = new Map<string, { hash: string; year: number; day: number }>();
    for (const row of workspace?.trade_history ?? []) {
      if (!seen.has(row.record_hash)) {
        seen.set(row.record_hash, {
          hash: row.record_hash,
          year: row.year,
          day: row.day,
        });
      }
    }
    return [...seen.values()];
  });
  const availableWeightResources = $derived(
    (workspace?.price_ledger ?? [])
      .filter((row) => row.currency === selectedCurrency)
      .map((row) => row.resource_token)
      .filter((token, index, values) => values.indexOf(token) === index)
      .sort(),
  );
  const filteredLedger = $derived(
    (workspace?.resource_ledger ?? [])
      .filter(
        (row) =>
          row.currency === selectedCurrency &&
          row.channel === selectedChannel &&
          row.resource_token.toLowerCase().includes(ledgerFilter.toLowerCase()),
      )
      .slice(0, 150),
  );
  const tradeSelectionAvailable = $derived(
    (workspace?.trade_history ?? []).some(
      (row) =>
        row.currency === selectedCurrency && row.channel === selectedChannel,
    ) ||
      (workspace?.resource_ledger ?? []).some(
        (row) =>
          row.currency === selectedCurrency && row.channel === selectedChannel,
      ),
  );

  $effect(() => {
    const contextId = workspace?.analysis_context.context_id ?? "";
    if (!contextId || smartDefaultContext === contextId) return;
    smartDefaultContext = contextId;
    const recommendedCurrency = workspace?.commissioning.recommended_currency;
    const recommendedChannel = workspace?.commissioning.recommended_channel;
    const recommendedResource =
      workspace?.commissioning.recommended_price_resource;
    if (recommendedCurrency === "rub" || recommendedCurrency === "usd") {
      selectedCurrency = recommendedCurrency;
      scenarioCurrency = recommendedCurrency;
    }
    if (
      recommendedChannel === "standard" ||
      recommendedChannel === "international"
    ) {
      selectedChannel = recommendedChannel;
    }
    if (recommendedResource) priceResource = recommendedResource;
  });

  $effect(() => {
    if (!baseRecords.some((record) => record.hash === basketBase)) {
      basketBase = baseRecords[0]?.hash ?? "";
    }
    if (!availableWeightResources.includes(weightResource)) {
      weightResource = availableWeightResources[0] ?? "";
    }
    if (!availableWeightResources.includes(priceResource)) {
      priceResource = availableWeightResources[0] ?? "";
    }
  });

  $effect(() => {
    const contextId = workspace?.analysis_context.context_id ?? "";
    const currency = selectedCurrency;
    const resource = priceResource;
    if (!desktopAvailable || !contextId || !resource) {
      priceSeries = null;
      priceSeriesLoading = false;
      return;
    }
    void loadPriceSeries(currency, resource);
  });

  async function loadPriceSeries(
    currency: "rub" | "usd",
    resource: string,
  ): Promise<void> {
    const request = ++priceRequest;
    priceSeriesLoading = true;
    try {
      const series = await getMarketPriceSeries(currency, resource);
      if (request === priceRequest) priceSeries = series;
    } catch {
      if (request === priceRequest) priceSeries = null;
    } finally {
      if (request === priceRequest) priceSeriesLoading = false;
    }
  }

  function emptyWorkspace(): MarketWorkspace {
    return {
      analysis_context: {
        context_id: "unavailable",
        selected_branch_id: "unassigned",
        head_interpretation_id: null,
        original_branch_id: null,
        mode: "latest",
        origin: "automatic",
        is_tip: true,
        membership_revision: 0,
        compatibility_profile_id: null,
        compatibility_profile_hash: null,
        observation_watermark: null,
        catalogue_generation_id: null,
        overlay_revision: null,
      },
      available: false,
      partial: false,
      coverage_status: null,
      history_records: 0,
      row_count: 0,
      city_scope_count: 0,
      warehouse_history_available: false,
      warnings: [],
      currencies: [],
      trade_history: [],
      resource_ledger: [],
      price_ledger: [],
      scalar_ledger: [],
      cities: [],
      baskets: [],
      scenarios: [],
      metric_contexts: [],
      terms_of_trade: [],
      reserves_available: false,
      terms_of_trade_available: false,
      limitations: [],
      commissioning: {
        recorded_save_count: 0,
        indexed_save_count: 0,
        current_engine_indexed_save_count: 0,
        pending_current_engine_save_count: 0,
        active_engine_current: false,
        active_parser_engine_version: null,
        recommended_currency: null,
        recommended_channel: null,
        recommended_price_resource: null,
        facets: [],
      },
    };
  }

  function money(value: number | null | undefined): string {
    return value == null ? "—" : formatNumber(value, $activeLocale);
  }

  function signed(value: number): string {
    const formatted = formatNumber(Math.abs(value), $activeLocale);
    return value > 0
      ? `+${formatted}`
      : value < 0
        ? `−${formatted}`
        : formatted;
  }

  function scalarLabel(factId: string): string {
    if (factId.startsWith("market.cost.delivery."))
      return $translation("markets-account-delivery-cost");
    if (factId.startsWith("market.cost.labour."))
      return $translation("markets-account-labour-cost");
    if (factId.startsWith("market.cost.immigrant."))
      return $translation("markets-account-immigrant-cost");
    if (factId === "market.tourism.visitors")
      return $translation("markets-account-tourism-visitors");
    if (factId === "market.tourism.hotel_nights")
      return $translation("markets-account-hotel-nights");
    if (factId.startsWith("market.tourism.spending."))
      return $translation("markets-account-tourism-spending");
    if (factId.startsWith("market.loan.balance."))
      return $translation("markets-account-loan-balance");
    if (factId.startsWith("market.loan.interest."))
      return $translation("markets-account-loan-interest");
    if (factId.startsWith("market.vehicle.import."))
      return $translation("markets-account-vehicle-imports");
    if (factId.startsWith("market.vehicle.export."))
      return $translation("markets-account-vehicle-exports");
    return factId;
  }

  function errorCode(error: unknown): string {
    if (typeof error === "object" && error && "code" in error) {
      return String((error as { code: unknown }).code);
    }
    return "market_action_failed";
  }

  function actionFailureMessage(error: unknown): string {
    const code = errorCode(error);
    if (code === "storage_busy")
      return $translation("markets-action-storage-busy");
    if (code === "storage_contract_violation")
      return $translation("markets-action-storage-contract");
    if (code === "warehouse_unavailable")
      return $translation("markets-action-warehouse-unavailable");
    if (code === "storage_unavailable")
      return $translation("markets-action-storage-unavailable");
    return $translation("markets-action-failed-summary");
  }

  function indexingRecovery(error: unknown): RecoveryProposal | undefined {
    const code = errorCode(error);
    if (code === "storage_busy") {
      return {
        title: $translation("markets-recovery-title"),
        message: $translation("markets-recovery-busy-message"),
        consequence: $translation("recovery-retained-evidence-safety"),
        actionLabel: $translation("markets-recovery-retry-action"),
        technicalDetails: { code, operation: "market_indexing" },
        run: () => runIndexing(true),
      };
    }
    if (
      code === "storage_contract_violation" ||
      code === "warehouse_unavailable"
    ) {
      return {
        title: $translation("markets-recovery-title"),
        message: $translation("markets-recovery-contract-message"),
        consequence: $translation("recovery-retained-evidence-safety"),
        actionLabel: $translation("markets-recovery-repair-action"),
        technicalDetails: { code, operation: "market_indexing_recovery" },
        run: async () => {
          await recoverMarketIndexing();
          await runIndexing(true);
        },
      };
    }
    return undefined;
  }

  async function runIndexing(propagateFailure = false): Promise<void> {
    if (busy || !desktopAvailable) return;
    busy = true;
    try {
      const refresh =
        indexingProgress?.phase !== "paused" &&
        ((workspace?.commissioning.current_engine_indexed_save_count ?? 0) >
          0 ||
          indexingProgress?.phase === "complete");
      const progress = await (refresh
        ? refreshChangedMarketData()
        : indexAvailableSavesForMarkets());
      onprogress(progress);
      if (progress.phase === "paused") {
        notify({
          title: $translation("markets-index-notification-title"),
          message: $translation("markets-index-notification-paused"),
          tone: "warning",
        });
        return;
      }
      notify({
        title: $translation("markets-index-notification-title"),
        message: $translation("markets-index-notification-complete", {
          complete: progress.completed_archives,
          missing: progress.missing_archives,
          changed: progress.changed_archives,
          failed: progress.failed_archives,
        }),
        tone: progress.failed_archives ? "warning" : "success",
      });
    } catch (error) {
      if (propagateFailure) throw error;
      notify({
        title: $translation("markets-index-notification-title"),
        message: actionFailureMessage(error),
        tone: "error",
        recovery: indexingRecovery(error),
        technicalDetails: {
          code: errorCode(error),
          operation: "market_indexing",
        },
      });
    } finally {
      busy = false;
    }
  }

  function addWeight(): void {
    if (!weightResource || !Number.isFinite(weightValue) || weightValue <= 0)
      return;
    const existing = basketWeights.findIndex(
      (entry) => entry.resource_token === weightResource,
    );
    basketWeights =
      existing >= 0
        ? basketWeights.map((entry, index) =>
            index === existing ? { ...entry, weight: weightValue } : entry,
          )
        : [
            ...basketWeights,
            { resource_token: weightResource, weight: weightValue },
          ];
  }

  async function saveBasket(): Promise<void> {
    if (busy) return;
    busy = true;
    const draft: MarketBasketDraft = {
      basket_id: basketId,
      name: basketName,
      currency: selectedCurrency,
      price_side: basketSide,
      base_record_hash: basketBase,
      reason: basketReason,
      weights: basketWeights,
    };
    try {
      onupdate(await saveMarketBasket(draft));
      notify({
        title: $translation("markets-baskets-title"),
        message: $translation("markets-basket-saved"),
        tone: "success",
      });
    } catch (error) {
      notify({
        title: $translation("markets-baskets-title"),
        message: actionFailureMessage(error),
        tone: "error",
        technicalDetails: {
          code: errorCode(error),
          operation: "market_basket_save",
        },
      });
    } finally {
      busy = false;
    }
  }

  async function saveScenario(): Promise<void> {
    if (busy) return;
    busy = true;
    const included = [
      includeStandardExports ? "standard_exports" : null,
      includeInternationalExports ? "international_exports" : null,
      includeTourism ? "tourism_spend" : null,
    ].filter((value): value is string => value !== null);
    const draft: MarketScenarioDraft = {
      scenario_id: scenarioId,
      name: scenarioName,
      scenario_kind: scenarioKind,
      currency: scenarioCurrency,
      reason: scenarioReason,
      domestic_unit_cost: scenarioKind === "break_even" ? domesticCost : null,
      delivery_cost: scenarioKind === "break_even" ? deliveryCost : null,
      operating_efficiency_percent:
        scenarioKind === "break_even" ? efficiency : null,
      exchange_rate: exchangeRate,
      debt_service: scenarioKind === "debt_stress" ? debtService : null,
      export_stress_percent:
        scenarioKind === "debt_stress" ? exportStress : null,
      tourism_stress_percent:
        scenarioKind === "debt_stress" ? tourismStress : null,
      included_income_components: included,
    };
    try {
      onupdate(await saveMarketScenario(draft));
      notify({
        title: $translation("markets-scenarios-title"),
        message: $translation("markets-scenario-saved"),
        tone: "success",
      });
    } catch (error) {
      notify({
        title: $translation("markets-scenarios-title"),
        message: actionFailureMessage(error),
        tone: "error",
        technicalDetails: {
          code: errorCode(error),
          operation: "market_scenario_save",
        },
      });
    } finally {
      busy = false;
    }
  }

  async function lifecycle(
    action: "select" | "rollback" | "remove" | "clear",
    kind: "basket" | "scenario",
    id = "",
    revision = 0,
  ): Promise<void> {
    if (busy) return;
    if (
      action === "remove" &&
      !window.confirm($translation("markets-remove-confirm"))
    )
      return;
    busy = true;
    try {
      const updated =
        action === "select"
          ? await selectMarketDefinition(kind, id, revision)
          : action === "rollback"
            ? await rollbackMarketDefinition(kind, id)
            : action === "remove"
              ? await removeMarketDefinition(kind, id)
              : await clearMarketSelection(kind);
      onupdate(updated);
    } catch (error) {
      notify({
        title: $translation("nav-markets"),
        message: actionFailureMessage(error),
        tone: "error",
        technicalDetails: {
          code: errorCode(error),
          operation: `market_${kind}_${action}`,
        },
      });
    } finally {
      busy = false;
    }
  }

  function basketLabel(
    name: string,
    currency: string,
    builtIn: boolean,
  ): string {
    if (!builtIn) return name;
    return $translation(
      name === "observed_positive_exports"
        ? "markets-basket-observed-exports"
        : "markets-basket-observed-imports",
      { currency: currency.toUpperCase() },
    );
  }

  function limitationLabel(code: string): string {
    const keys = {
      reserves_unavailable: "markets-limit-reserves",
      city_republic_windows_separate: "markets-limit-city-window",
      currencies_require_explicit_exchange: "markets-limit-currencies",
      loan_tourism_denominator_required: "markets-limit-denominators",
      no_annualisation_or_interpolation: "markets-limit-interpolation",
    } as const;
    return $translation(
      keys[code as keyof typeof keys] ?? "markets-limit-unknown",
    );
  }

  function coverageFacetLabel(facetId: string): string {
    const keys = {
      prices: "markets-coverage-prices",
      trade: "markets-coverage-trade",
      costs: "markets-coverage-costs",
      tourism: "markets-coverage-tourism",
      loans: "markets-coverage-loans",
      vehicles: "markets-coverage-vehicles",
      cities: "markets-coverage-cities",
    } as const;
    return $translation(
      keys[facetId as keyof typeof keys] ?? "markets-coverage-unknown",
    );
  }

  function coverageStatusLabel(status: string): string {
    const keys = {
      observed: "markets-coverage-status-observed",
      partial: "markets-coverage-status-partial",
      not_observed: "markets-coverage-status-not-observed",
    } as const;
    return $translation(
      keys[status as keyof typeof keys] ??
        "markets-coverage-status-not-observed",
    );
  }
</script>

<section class="workspace markets-workspace">
  <aside
    class="navigator"
    aria-label={$translation("markets-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("markets-directorate")}</span>
        <h2>{$translation("nav-markets")}</h2>
      </div>
      <span class="edition">V1</span>
    </div>
    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("filter-branch")}</span>
        <strong>{workspace?.analysis_context.selected_branch_id ?? "—"}</strong>
      </div>
      <div class="lens-row">
        <span>{$translation("markets-history-records")}</span>
        <strong
          >{formatNumber(
            workspace?.history_records ?? 0,
            $activeLocale,
          )}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("markets-city-scopes")}</span>
        <strong
          >{formatNumber(
            workspace?.city_scope_count ?? 0,
            $activeLocale,
          )}</strong
        >
      </div>
    </div>
    <div class="section-list">
      <a href="#markets-pulse" use:containedSectionNavigation
        ><span>01</span>{$translation("markets-section-pulse")}</a
      >
      <a href="#markets-trade" use:containedSectionNavigation
        ><span>02</span>{$translation("markets-section-trade")}</a
      >
      <a href="#markets-prices" use:containedSectionNavigation
        ><span>03</span>{$translation("markets-section-prices")}</a
      >
      <a href="#markets-cities" use:containedSectionNavigation
        ><span>04</span>{$translation("markets-section-cities")}</a
      >
      <a href="#markets-labs" use:containedSectionNavigation
        ><span>05</span>{$translation("markets-section-labs")}</a
      >
    </div>
    <GuidanceSurface kind="boundary" layout="compact" class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("markets-sidebar-boundary")}</p>
    </GuidanceSurface>
  </aside>

  <section class="canvas">
    <GuidanceSurface
      kind="instruction"
      layout="inline"
      semanticRole="status"
      class="preview-banner"
    >
      <strong>{$translation("markets-evidence-banner")}</strong>
      <span>{$translation("markets-evidence-banner-detail")}</span>
    </GuidanceSurface>
    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("markets-heading-eyebrow")}</span>
        <h2>{$translation("markets-heading-title")}</h2>
        <p>{$translation("markets-heading-description")}</p>
      </div>
      <button
        type="button"
        disabled={!desktopAvailable || busy}
        onclick={() => runIndexing()}
      >
        {$translation(indexActionKey)}
      </button>
    </header>

    {#if indexView && indexingProgress?.phase !== "idle"}
      <TaskProgressPanel view={indexView} headingId="markets-index-progress" />
    {/if}

    {#if workspace}
      <section
        class="market-commissioning"
        aria-labelledby="markets-commissioning-title"
      >
        <header class="panel-heading">
          <div>
            <span class="eyebrow"
              >{$translation("markets-commissioning-eyebrow")}</span
            >
            <h2 id="markets-commissioning-title">
              {$translation("markets-commissioning-title")}
            </h2>
            <p>{$translation("markets-commissioning-detail")}</p>
          </div>
          <span
            class:current={workspace.commissioning.active_engine_current}
            class="status-chip"
          >
            {$translation(
              workspace.commissioning.active_engine_current
                ? "markets-commissioning-current"
                : "markets-commissioning-reindex-required",
            )}
          </span>
        </header>
        <div class="commissioning-counts">
          <article>
            <span>{$translation("markets-commissioning-recorded")}</span><strong
              >{workspace.commissioning.recorded_save_count}</strong
            >
          </article>
          <article>
            <span>{$translation("markets-commissioning-indexed")}</span><strong
              >{workspace.commissioning.indexed_save_count}</strong
            >
          </article>
          <article>
            <span>{$translation("markets-commissioning-current-engine")}</span
            ><strong
              >{workspace.commissioning
                .current_engine_indexed_save_count}</strong
            >
          </article>
          <article>
            <span>{$translation("markets-commissioning-pending")}</span><strong
              >{workspace.commissioning
                .pending_current_engine_save_count}</strong
            >
          </article>
        </div>
        <div class="coverage-grid">
          {#each workspace.commissioning.facets as facet}
            <article
              class:observed={facet.status === "observed"}
              class:partial={facet.status === "partial"}
            >
              <span>{coverageFacetLabel(facet.facet_id)}</span>
              <strong>{coverageStatusLabel(facet.status)}</strong>
              <small
                >{$translation("markets-coverage-slots", {
                  observed: facet.observed_slots,
                  expected: facet.expected_slots,
                  resources: facet.resource_count,
                })}</small
              >
            </article>
          {/each}
        </div>
        <GuidanceSurface kind="boundary" layout="compact">
          <strong>{$translation("markets-commissioning-boundary-title")}</strong
          >
          <span>{$translation("markets-commissioning-boundary-detail")}</span>
        </GuidanceSurface>
      </section>
    {/if}

    {#if !desktopAvailable}
      <section class="archive-empty-state">
        <span class="eyebrow">{$translation("archive-desktop-required")}</span>
        <h3>{$translation("markets-desktop-required")}</h3>
        <p>{$translation("markets-desktop-required-detail")}</p>
      </section>
    {:else if !workspace?.available}
      <section class="archive-empty-state">
        <span class="eyebrow">{$translation("markets-no-evidence")}</span>
        <h3>{$translation("markets-empty-title")}</h3>
        <p>{$translation("markets-empty-detail")}</p>
        <button type="button" disabled={busy} onclick={() => runIndexing()}>
          {$translation(indexActionKey)}
        </button>
      </section>
    {:else}
      {#if workspace.partial}
        <GuidanceSurface kind="boundary" layout="compact">
          <strong>{$translation("markets-partial-title")}</strong>
          <span>{$translation("markets-partial-detail")}</span>
        </GuidanceSurface>
      {/if}
      {#if !workspace.warehouse_history_available}
        <GuidanceSurface kind="boundary" layout="compact">
          <strong>{$translation("markets-warehouse-lag-title")}</strong>
          <span>{$translation("markets-warehouse-lag-detail")}</span>
        </GuidanceSurface>
      {/if}

      <div
        class="market-controls"
        aria-label={$translation("markets-view-controls")}
      >
        <label>
          <span>{$translation("markets-currency")}</span>
          <select bind:value={selectedCurrency}>
            <option value="rub">RUB</option>
            <option value="usd">USD</option>
          </select>
        </label>
        <label>
          <span>{$translation("markets-channel")}</span>
          <select bind:value={selectedChannel}>
            <option value="standard"
              >{$translation("markets-channel-standard")}</option
            >
            <option value="international"
              >{$translation("markets-channel-international")}</option
            >
          </select>
        </label>
      </div>

      {#if !tradeSelectionAvailable}
        <GuidanceSurface kind="boundary" layout="compact">
          <strong>{$translation("markets-channel-unobserved-title")}</strong>
          <span
            >{$translation("markets-channel-unobserved-detail", {
              currency: selectedCurrency.toUpperCase(),
              channel: $translation(
                selectedChannel === "standard"
                  ? "markets-channel-standard"
                  : "markets-channel-international",
              ),
            })}</span
          >
        </GuidanceSurface>
      {/if}

      <section
        id="markets-pulse"
        class="kpi-grid market-kpis"
        aria-label={$translation("markets-section-pulse")}
      >
        <article class="kpi-card">
          <header>
            <span>{$translation("markets-imports")}</span
            >{#if pulseHelp}<ContextHelp {...pulseHelp} placement="left" />{/if}
          </header>
          <strong
            >{money(
              !tradeSelectionAvailable
                ? null
                : selectedChannel === "standard"
                  ? pulse?.standard_import_value
                  : pulse?.international_import_value,
            )}</strong
          >
          <p>
            {selectedCurrency.toUpperCase()} · {$translation(
              selectedChannel === "standard"
                ? "markets-channel-standard"
                : "markets-channel-international",
            )}
          </p>
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("markets-exports")}</span
            >{#if pulseHelp}<ContextHelp {...pulseHelp} placement="left" />{/if}
          </header>
          <strong
            >{money(
              !tradeSelectionAvailable
                ? null
                : selectedChannel === "standard"
                  ? pulse?.standard_export_value
                  : pulse?.international_export_value,
            )}</strong
          >
          <p>{$translation("markets-signed-source-values")}</p>
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("markets-trade-result")}</span
            >{#if pulseHelp}<ContextHelp {...pulseHelp} placement="left" />{/if}
          </header>
          <strong
            >{tradeSelectionAvailable
              ? signed(
                  selectedChannel === "standard"
                    ? (pulse?.standard_trade_result ?? 0)
                    : (pulse?.international_trade_result ?? 0),
                )
              : "—"}</strong
          >
          <p>{$translation("markets-formula-trade-result")}</p>
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("markets-export-hhi")}</span
            >{#if concentrationHelp}<ContextHelp
                {...marketMetricHelp(concentrationHelp, $translation)}
                placement="left"
              />{/if}
          </header>
          <strong
            >{pulse?.positive_export_hhi == null
              ? "—"
              : pulse.positive_export_hhi.toFixed(3)}</strong
          >
          <p>
            {$translation("markets-export-hhi-detail", {
              resources: pulse?.positive_export_resource_count ?? 0,
            })}
          </p>
        </article>
      </section>

      <section id="markets-trade" class="market-chart-grid">
        <ObservatoryChart
          spec={tradeChart}
          height="300px"
          eyebrow={$translation("markets-section-trade")}
          help={pulseHelp}
        />
        <ObservatoryChart
          spec={exportChart}
          eyebrow={$translation("markets-section-concentration")}
          help={concentrationHelp
            ? marketMetricHelp(concentrationHelp, $translation)
            : null}
        />
      </section>

      <section class="market-panel">
        <header class="panel-heading">
          <div>
            <span class="eyebrow"
              >{$translation("markets-resource-ledger-eyebrow")}</span
            >
            <h2>
              {$translation("markets-resource-ledger-title")}
              {#if pulseHelp}<ContextHelp
                  {...pulseHelp}
                  placement="left"
                />{/if}
            </h2>
            <p>{$translation("markets-resource-ledger-detail")}</p>
          </div>
          <label
            ><span>{$translation("markets-filter-resource")}</span><input
              bind:value={ledgerFilter}
              placeholder={$translation("markets-filter-resource-placeholder")}
            /></label
          >
        </header>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          class="table-scroll"
          role="region"
          tabindex="0"
          aria-label={$translation("markets-resource-ledger-scroll-label")}
        >
          <table>
            <thead
              ><tr
                ><th>{$translation("markets-resource-token")}</th><th
                  >{$translation("markets-import-quantity")}</th
                ><th>{$translation("markets-import-value")}</th><th
                  >{$translation("markets-export-quantity")}</th
                ><th>{$translation("markets-export-value")}</th><th
                  >{$translation("markets-trade-result")}</th
                ><th>{$translation("markets-disposal-cost")}</th></tr
              ></thead
            ><tbody
              >{#each filteredLedger as row}<tr
                  ><th scope="row"><code>{row.resource_token}</code></th><td
                    >{money(row.import_quantity)}</td
                  ><td>{money(row.import_account_value)}</td><td
                    >{money(row.export_quantity)}</td
                  ><td>{money(row.export_account_value)}</td><td
                    >{signed(row.trade_result)}</td
                  ><td>{money(row.disposal_cost)}</td></tr
                >{/each}</tbody
            >
          </table>
        </div>
      </section>

      <section id="markets-prices" class="market-panel">
        <header class="panel-heading">
          <div>
            <span class="eyebrow">{$translation("markets-prices-eyebrow")}</span
            >
            <h2>
              {$translation("markets-prices-title")}
              {#if priceHelp}<ContextHelp
                  {...marketMetricHelp(priceHelp, $translation)}
                  placement="left"
                />{/if}
            </h2>
            <p>{$translation("markets-prices-detail")}</p>
          </div>
          <label class="price-resource-selector">
            <span>{$translation("markets-price-resource")}</span>
            <select bind:value={priceResource}>
              {#each availableWeightResources as resource}
                <option value={resource}>{resource}</option>
              {/each}
            </select>
          </label>
        </header>
        {#if priceSeriesLoading}
          <GuidanceSurface kind="instruction" layout="compact">
            <strong>{$translation("markets-price-history-loading")}</strong>
            <span>{$translation("markets-price-history-loading-detail")}</span>
          </GuidanceSurface>
        {:else if priceSeries?.available}
          <ObservatoryChart
            spec={priceChart}
            height="300px"
            eyebrow={$translation("markets-price-history-eyebrow")}
            help={priceHelp ? marketMetricHelp(priceHelp, $translation) : null}
          />
        {:else}
          <GuidanceSurface kind="boundary" layout="compact">
            <strong>{$translation("markets-price-history-unavailable")}</strong>
            <span
              >{$translation("markets-price-history-unavailable-detail")}</span
            >
          </GuidanceSurface>
        {/if}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          class="table-scroll"
          role="region"
          tabindex="0"
          aria-label={$translation("markets-price-ledger-scroll-label")}
        >
          <table>
            <thead
              ><tr
                ><th>{$translation("markets-resource-token")}</th><th
                  >{$translation("markets-purchase-price")}</th
                ><th>{$translation("markets-sell-price")}</th><th
                  >{$translation("markets-base-price")}</th
                ><th>{$translation("markets-purchase-index")}</th><th
                  >{$translation("markets-sell-index")}</th
                ><th>{$translation("markets-volatility")}</th></tr
              ></thead
            ><tbody
              >{#each (workspace.price_ledger ?? [])
                .filter((row) => row.currency === selectedCurrency)
                .slice(0, 150) as row}<tr
                  ><th scope="row"><code>{row.resource_token}</code></th><td
                    >{money(row.purchase_price)}</td
                  ><td>{money(row.sell_price)}</td><td
                    >{money(row.base_price)}</td
                  ><td
                    >{row.purchase_index == null
                      ? "—"
                      : row.purchase_index.toFixed(1)}</td
                  ><td
                    >{row.sell_index == null
                      ? "—"
                      : row.sell_index.toFixed(1)}</td
                  ><td
                    >{row.robust_log_volatility == null
                      ? "—"
                      : row.robust_log_volatility.toFixed(4)}
                    <small>n={row.volatility_observations}</small></td
                  ></tr
                >{/each}</tbody
            >
          </table>
        </div>
        {#if workspace.terms_of_trade.length > 0}
          <div class="terms-grid">
            {#each workspace.terms_of_trade
              .filter((item) => item.currency === selectedCurrency)
              .slice(0, 8) as item}
              <article>
                <header>
                  <strong>{$translation("markets-terms-title")}</strong
                  ><ContextHelp
                    {...marketMetricHelp(item.context, $translation)}
                    placement="left"
                  />
                </header>
                <span>{item.terms_of_trade_index.toFixed(2)}</span>
                <small
                  >{$translation("markets-terms-reading", {
                    imports: item.import_basket_id,
                    exports: item.export_basket_id,
                    base: item.base_record_hash.slice(0, 8),
                  })}</small
                >
              </article>
            {/each}
          </div>
        {:else}
          <GuidanceSurface kind="boundary" layout="compact"
            ><strong>{$translation("markets-terms-unavailable-title")}</strong
            ><span>{$translation("markets-terms-unavailable-detail")}</span
            ></GuidanceSurface
          >
        {/if}
      </section>

      <section class="market-panel">
        <header class="panel-heading">
          <div>
            <span class="eyebrow"
              >{$translation("markets-account-ledger-eyebrow")}</span
            >
            <h2>
              {$translation("markets-account-ledger-title")}
              {#if scalarHelp}<ContextHelp
                  {...marketMetricHelp(scalarHelp, $translation)}
                  placement="left"
                />{/if}
            </h2>
            <p>{$translation("markets-account-ledger-detail")}</p>
          </div>
        </header>
        <div class="scalar-grid">
          {#each workspace.scalar_ledger as fact}<article>
              <span>{scalarLabel(fact.fact_id)}</span><strong
                >{money(fact.value)}</strong
              ><small
                >{fact.currency?.toUpperCase() ??
                  $translation("markets-source-native")} · {fact.fact_id} · {fact.source_field}
                · L{fact.source_line}</small
              >
            </article>{/each}
        </div>
        {#if workspace.scalar_ledger.length === 0}<p class="empty-row">
            {$translation("markets-account-ledger-empty")}
          </p>{/if}
      </section>

      <section id="markets-cities" class="market-panel">
        <header class="panel-heading">
          <div>
            <span class="eyebrow">{$translation("markets-cities-eyebrow")}</span
            >
            <h2>{$translation("markets-cities-title")}</h2>
            <p>{$translation("markets-cities-detail")}</p>
          </div>
        </header>
        <ObservatoryChart
          spec={cityChart}
          eyebrow={$translation("markets-section-cities")}
          help={cityHelp ? marketMetricHelp(cityHelp, $translation) : null}
        />
      </section>

      <section id="markets-labs" class="market-labs">
        <article class="market-lab">
          <header>
            <div>
              <span class="eyebrow"
                >{$translation("markets-baskets-eyebrow")}</span
              >
              <h2>{$translation("markets-baskets-title")}</h2>
              <p>{$translation("markets-baskets-detail")}</p>
            </div>
            <button
              type="button"
              disabled={busy}
              onclick={() => lifecycle("clear", "basket")}
              >{$translation("markets-clear-selection")}</button
            >
          </header>
          <div class="definition-list">
            {#each workspace.baskets as basket}<article
                class:active={basket.selected}
              >
                <div>
                  <strong
                    >{basketLabel(
                      basket.name,
                      basket.currency,
                      basket.built_in,
                    )}</strong
                  ><span
                    >{basket.currency.toUpperCase()} · {$translation(
                      basket.price_side === "purchase"
                        ? "markets-purchase-price"
                        : "markets-sell-price",
                    )} · {$translation("markets-revision", {
                      revision: basket.revision,
                    })}</span
                  ><small
                    >{$translation("markets-basket-index-reading", {
                      index:
                        basket.index_value == null
                          ? "—"
                          : basket.index_value.toFixed(2),
                      covered: basket.coverage_resources,
                      total: basket.resource_count,
                    })}</small
                  >
                </div>
                {#if basket.built_in}<span class="status-chip"
                    >{$translation("markets-built-in")}</span
                  >{:else}<div class="definition-actions">
                    <button
                      type="button"
                      disabled={busy || basket.selected}
                      onclick={() =>
                        lifecycle(
                          "select",
                          "basket",
                          basket.basket_id,
                          basket.revision,
                        )}>{$translation("action-select")}</button
                    ><button
                      type="button"
                      disabled={busy || basket.revision <= 1}
                      onclick={() =>
                        lifecycle("rollback", "basket", basket.basket_id)}
                      >{$translation("action-rollback")}</button
                    ><button
                      type="button"
                      disabled={busy || basket.selected}
                      onclick={() =>
                        lifecycle("remove", "basket", basket.basket_id)}
                      >{$translation("action-remove")}</button
                    >
                  </div>{/if}
              </article>{/each}
          </div>
          <form
            onsubmit={(event) => {
              event.preventDefault();
              void saveBasket();
            }}
          >
            <h3>{$translation("markets-basket-draft-title")}</h3>
            <div class="form-grid">
              <label
                ><span>{$translation("markets-definition-id")}</span><input
                  required
                  bind:value={basketId}
                /></label
              ><label
                ><span>{$translation("markets-name")}</span><input
                  required
                  bind:value={basketName}
                /></label
              ><label
                ><span>{$translation("markets-price-side")}</span><select
                  bind:value={basketSide}
                  ><option value="purchase"
                    >{$translation("markets-purchase-price")}</option
                  ><option value="sell"
                    >{$translation("markets-sell-price")}</option
                  ></select
                ></label
              ><label
                ><span>{$translation("markets-base-record")}</span><select
                  required
                  bind:value={basketBase}
                  >{#each baseRecords as record}<option value={record.hash}
                      >{$translation("observation-game-date-compact", {
                        year: record.year,
                        day: String(record.day).padStart(3, "0"),
                      })} · {record.hash.slice(0, 8)}</option
                    >{/each}</select
                ></label
              >
            </div>
            <label
              ><span>{$translation("markets-reason")}</span><input
                required
                bind:value={basketReason}
              /></label
            >
            <div class="weight-editor">
              <label
                ><span>{$translation("markets-resource-token")}</span><select
                  bind:value={weightResource}
                  >{#each availableWeightResources as resource}<option
                      value={resource}>{resource}</option
                    >{/each}</select
                ></label
              ><label
                ><span>{$translation("markets-weight")}</span><input
                  type="number"
                  min="0.000001"
                  step="any"
                  bind:value={weightValue}
                /></label
              ><button type="button" onclick={addWeight}
                >{$translation("markets-add-weight")}</button
              >
            </div>
            <div class="weight-list">
              {#each basketWeights as weight}<button
                  type="button"
                  onclick={() =>
                    (basketWeights = basketWeights.filter(
                      (entry) => entry.resource_token !== weight.resource_token,
                    ))}>{weight.resource_token} · {weight.weight} ×</button
                >{/each}
            </div>
            <div class="form-actions">
              <button
                type="submit"
                disabled={busy || basketWeights.length === 0}
                >{$translation("markets-save-basket")}</button
              >
            </div>
          </form>
        </article>

        <article class="market-lab">
          <header>
            <div>
              <span class="eyebrow"
                >{$translation("markets-scenarios-eyebrow")}</span
              >
              <h2>{$translation("markets-scenarios-title")}</h2>
              <p>{$translation("markets-scenarios-detail")}</p>
            </div>
            <button
              type="button"
              disabled={busy}
              onclick={() => lifecycle("clear", "scenario")}
              >{$translation("markets-clear-selection")}</button
            >
          </header>
          <div class="definition-list">
            {#each workspace.scenarios as scenario}<article
                class:active={scenario.selected}
              >
                <div>
                  <strong>{scenario.name}</strong><span
                    >{$translation(
                      scenario.scenario_kind === "break_even"
                        ? "markets-break-even"
                        : "markets-debt-stress",
                    )} · {$translation("markets-revision", {
                      revision: scenario.revision,
                    })}</span
                  ><small
                    >{scenario.result_value == null
                      ? $translation("chart-unavailable")
                      : $translation(
                          scenario.result_kind === "debt_service_coverage"
                            ? "markets-scenario-coverage-result"
                            : "markets-scenario-break-even-result",
                          { value: scenario.result_value.toFixed(3) },
                        )}</small
                  >
                </div>
                <div class="definition-actions">
                  <button
                    type="button"
                    disabled={busy || scenario.selected}
                    onclick={() =>
                      lifecycle(
                        "select",
                        "scenario",
                        scenario.scenario_id,
                        scenario.revision,
                      )}>{$translation("action-select")}</button
                  ><button
                    type="button"
                    disabled={busy || scenario.revision <= 1}
                    onclick={() =>
                      lifecycle("rollback", "scenario", scenario.scenario_id)}
                    >{$translation("action-rollback")}</button
                  ><button
                    type="button"
                    disabled={busy || scenario.selected}
                    onclick={() =>
                      lifecycle("remove", "scenario", scenario.scenario_id)}
                    >{$translation("action-remove")}</button
                  >
                </div>
              </article>{/each}
          </div>
          <form
            onsubmit={(event) => {
              event.preventDefault();
              void saveScenario();
            }}
          >
            <h3>{$translation("markets-scenario-draft-title")}</h3>
            <div class="form-grid">
              <label
                ><span>{$translation("markets-definition-id")}</span><input
                  required
                  bind:value={scenarioId}
                /></label
              ><label
                ><span>{$translation("markets-name")}</span><input
                  required
                  bind:value={scenarioName}
                /></label
              ><label
                ><span>{$translation("markets-scenario-kind")}</span><select
                  bind:value={scenarioKind}
                  ><option value="break_even"
                    >{$translation("markets-break-even")}</option
                  ><option value="debt_stress"
                    >{$translation("markets-debt-stress")}</option
                  ></select
                ></label
              ><label
                ><span>{$translation("markets-currency")}</span><select
                  bind:value={scenarioCurrency}
                  ><option value="rub">RUB</option><option value="usd"
                    >USD</option
                  ></select
                ></label
              >
            </div>
            <label
              ><span>{$translation("markets-reason")}</span><input
                required
                bind:value={scenarioReason}
              /></label
            >
            {#if scenarioKind === "break_even"}<div class="form-grid">
                <label
                  ><span>{$translation("markets-domestic-unit-cost")}</span
                  ><input
                    type="number"
                    min="0"
                    step="any"
                    bind:value={domesticCost}
                  /></label
                ><label
                  ><span>{$translation("markets-delivery-cost")}</span><input
                    type="number"
                    min="0"
                    step="any"
                    bind:value={deliveryCost}
                  /></label
                ><label
                  ><span>{$translation("markets-efficiency")}</span><input
                    type="number"
                    min="0.0001"
                    step="any"
                    bind:value={efficiency}
                  /></label
                ><label
                  ><span>{$translation("markets-exchange-rate-optional")}</span
                  ><input
                    type="number"
                    min="0.000001"
                    step="any"
                    bind:value={exchangeRate}
                  /></label
                >
              </div>{:else}<div class="form-grid">
                <label
                  ><span>{$translation("markets-debt-service")}</span><input
                    type="number"
                    min="0.000001"
                    step="any"
                    bind:value={debtService}
                  /></label
                ><label
                  ><span>{$translation("markets-export-stress")}</span><input
                    type="number"
                    min="0"
                    max="100"
                    step="any"
                    bind:value={exportStress}
                  /></label
                ><label
                  ><span>{$translation("markets-tourism-stress")}</span><input
                    type="number"
                    min="0"
                    max="100"
                    step="any"
                    bind:value={tourismStress}
                  /></label
                ><label
                  ><span>{$translation("markets-exchange-rate-optional")}</span
                  ><input
                    type="number"
                    min="0.000001"
                    step="any"
                    bind:value={exchangeRate}
                  /></label
                >
              </div>
              <fieldset>
                <legend>{$translation("markets-income-components")}</legend
                ><label
                  ><input
                    type="checkbox"
                    bind:checked={includeStandardExports}
                  />
                  {$translation("markets-standard-exports")}</label
                ><label
                  ><input
                    type="checkbox"
                    bind:checked={includeInternationalExports}
                  />
                  {$translation("markets-international-exports")}</label
                ><label
                  ><input type="checkbox" bind:checked={includeTourism} />
                  {$translation("markets-tourism-spend")}</label
                >
              </fieldset>{/if}
            <GuidanceSurface kind="boundary" layout="compact"
              ><strong>{$translation("markets-scenario-boundary-title")}</strong
              ><span>{$translation("markets-scenario-boundary-detail")}</span
              ></GuidanceSurface
            >
            <div class="form-actions">
              <button type="submit" disabled={busy}
                >{$translation("markets-save-scenario")}</button
              >
            </div>
          </form>
        </article>
      </section>

      <section class="market-limitations">
        <span class="eyebrow">{$translation("markets-limitations-title")}</span>
        <ul>
          {#each workspace.limitations as limitation}<li>
              {limitationLabel(limitation)}
            </li>{/each}
        </ul>
      </section>
    {/if}
  </section>
</section>

<style>
  .page-heading > button,
  .market-lab button,
  .archive-empty-state button {
    min-height: 38px;
    border: 1px solid var(--colour-line);
    padding: 8px 12px;
    color: var(--colour-text);
    background: var(--colour-surface);
    cursor: pointer;
  }
  button:disabled {
    cursor: not-allowed;
  }
  .market-controls {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin: 8px 0;
  }
  label {
    display: grid;
    gap: 5px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }
  input,
  select {
    min-height: 38px;
  }
  .market-kpis {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
  .market-chart-grid {
    display: grid;
    grid-template-columns: 1.25fr 0.75fr;
    gap: 8px;
  }
  .market-panel,
  .market-lab,
  .market-limitations,
  .market-commissioning {
    margin-top: 9px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
    padding: 14px;
  }
  .commissioning-counts,
  .coverage-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 7px;
    margin: 8px 0;
  }
  .coverage-grid {
    grid-template-columns: repeat(7, minmax(0, 1fr));
  }
  .commissioning-counts article,
  .coverage-grid article {
    display: grid;
    gap: 4px;
    min-width: 0;
    border: 1px solid var(--colour-line-faint);
    padding: 9px;
    background: var(--colour-surface-soft);
  }
  .coverage-grid article.observed {
    border-inline-start: 3px solid var(--colour-success);
  }
  .coverage-grid article.partial {
    border-inline-start: 3px solid var(--colour-risk);
  }
  .commissioning-counts span,
  .coverage-grid span,
  .coverage-grid small {
    color: var(--colour-muted);
    overflow-wrap: anywhere;
  }
  .status-chip.current {
    color: var(--colour-success);
  }
  .table-scroll {
    max-height: 430px;
    overflow: auto;
    border: 1px solid var(--colour-line-faint);
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 8px;
    border-bottom: 1px solid var(--colour-line-faint);
    text-align: right;
    white-space: nowrap;
  }
  th:first-child,
  td:first-child {
    text-align: left;
  }
  code {
    color: var(--colour-observed);
  }
  .scalar-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 7px;
  }
  .terms-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 7px;
    margin-top: 8px;
  }
  .terms-grid article {
    display: grid;
    gap: 4px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-soft);
    padding: 10px;
  }
  .terms-grid header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .terms-grid span {
    font: var(--type-display);
    color: var(--colour-observed);
  }
  .terms-grid small {
    color: var(--colour-muted);
    overflow-wrap: anywhere;
  }
  .scalar-grid article {
    display: grid;
    gap: 4px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-soft);
    padding: 10px;
  }
  .scalar-grid span,
  .scalar-grid small {
    color: var(--colour-muted);
    overflow-wrap: anywhere;
  }
  .market-labs {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .market-lab > header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: start;
  }
  .definition-list {
    display: grid;
    gap: 6px;
    margin: 12px 0;
  }
  .definition-list > article {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    align-items: center;
    border: 1px solid var(--colour-line-faint);
    padding: 10px;
  }
  .definition-list > article.active {
    border-inline-start: 3px solid var(--colour-success);
  }
  .definition-list > article > div:first-child {
    display: grid;
    gap: 3px;
  }
  .definition-list span,
  .definition-list small {
    color: var(--colour-muted);
  }
  .definition-actions {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }
  form {
    display: grid;
    gap: 9px;
    border-top: 1px solid var(--colour-line-faint);
    padding-top: 12px;
  }
  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .weight-editor {
    display: grid;
    grid-template-columns: 2fr 1fr auto;
    gap: 8px;
    align-items: end;
  }
  .weight-list {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }
  .form-actions {
    display: flex;
    justify-content: flex-end;
  }
  fieldset {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    border: 1px solid var(--colour-line-faint);
  }
  fieldset label {
    display: flex;
    align-items: center;
  }
  .market-limitations ul {
    margin: 8px 0 0;
    color: var(--colour-muted);
  }
  .empty-row {
    color: var(--colour-muted);
  }
  @media (max-width: 1300px) {
    .market-chart-grid,
    .market-labs {
      grid-template-columns: 1fr;
    }
    .market-kpis,
    .scalar-grid,
    .terms-grid,
    .commissioning-counts,
    .coverage-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (max-width: 760px) {
    .market-kpis,
    .scalar-grid,
    .terms-grid,
    .form-grid,
    .weight-editor {
      grid-template-columns: 1fr;
    }
    .commissioning-counts,
    .coverage-grid {
      grid-template-columns: 1fr;
    }
    .market-controls {
      justify-content: stretch;
    }
  }
</style>
