<script lang="ts">
  import receiverExampleJson from "../../../examples/analysis-packs/receiver-adoption-laboratory.roanalysis.json?raw";
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import { receiverPackPreview } from "../presentation/extensionPreview";
  import {
    chartSpecForAnalysisContribution,
    type AnalysisPackContribution,
    type AnalysisPackInspection,
    type AnalysisPackSummary,
    type ResolvedAnalysisChart,
  } from "../extensions/runtime";
  import type { TranslationKey } from "../i18n/catalog";
  import { translation } from "../i18n/runtime";
  import { containedSectionNavigation } from "../navigation/containedSectionNavigation";
  import {
    destinationsForSubject,
    type ChartNavigationBinding,
    type RelatedDataDestination,
  } from "../navigation/relatedData";
  import {
    notify,
    type NotificationTone,
    type TechnicalDetailsView,
  } from "../notifications/service";
  import ContextHelp from "../ui/ContextHelp.svelte";
  import FilePicker from "../ui/FilePicker.svelte";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import {
    disableAnalysisPack,
    enableAnalysisPack,
    exportAnalysisPack,
    getAnalysisPackContributions,
    importAnalysisPack,
    inspectAnalysisPack,
    listAnalysisPacks,
    removeAnalysisPack,
    rollbackAnalysisPack,
  } from "../observations/desktopClient";
  import WorkspaceSectionHeader from "./WorkspaceSectionHeader.svelte";

  let {
    desktopAvailable = false,
    observationContext = "",
    onrelatednavigate,
  }: {
    desktopAvailable?: boolean;
    observationContext?: string;
    onrelatednavigate?: (
      destinations: RelatedDataDestination[],
      origin: HTMLElement | null,
    ) => void;
  } = $props();
  let packs = $state<AnalysisPackSummary[]>([]);
  let contributions = $state<AnalysisPackContribution[]>([]);
  let inspection = $state<AnalysisPackInspection | null>(null);
  let inspectedJson = $state("");
  let busy = $state(false);
  let loadedObservationContext = $state("");
  let runtimeInitialised = $state(false);

  const sections: Array<{
    label: TranslationKey;
    href: string;
    marker: string;
  }> = [
    {
      label: "extensions-section-inspection",
      href: "#pack-inspection",
      marker: "01",
    },
    { label: "extensions-library", href: "#pack-library", marker: "02" },
    {
      label: "extensions-chart-contribution",
      href: "#pack-charts",
      marker: "03",
    },
    {
      label: "extensions-section-models",
      href: "#model-plugins",
      marker: "04",
    },
  ];
  const deniedCapabilities: TranslationKey[] = [
    "extension-permission-denied-executable",
    "extension-permission-denied-network",
    "extension-permission-denied-raw-save",
    "extension-permission-denied-custom-interface",
    "extension-permission-denied-echarts",
  ];
  const selectedPack = $derived(
    inspection?.valid
      ? inspection
      : packs[0]
        ? {
            valid: true,
            pack_id: packs[0].pack_id,
            name: packs[0].display_name,
            author: packs[0].author,
            version: packs[0].semantic_version,
            host_api_version: packs[0].host_api_version,
            default_locale: packs[0].default_locale,
            description: packs[0].description,
            content_hash: packs[0].content_hash,
            consumed_metrics: [],
            derived_metrics: [],
            charts: [],
            code: null,
          }
        : null,
  );

  async function refreshRuntime(): Promise<void> {
    if (!desktopAvailable) return;
    [packs, contributions] = await Promise.all([
      listAnalysisPacks(),
      getAnalysisPackContributions(),
    ]);
    loadedObservationContext = observationContext;
  }

  async function runAction(action: () => Promise<void>): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await action();
    } catch (error) {
      const message =
        typeof error === "object" && error && "diagnostic" in error
          ? String(error.diagnostic)
          : $translation("extensions-action-failed");
      reportAction(message, "error");
    } finally {
      busy = false;
    }
  }

  function reportAction(
    message: string,
    tone: NotificationTone,
    technicalDetails?: TechnicalDetailsView,
  ): void {
    notify({
      title: $translation("extensions-local-manager"),
      message,
      tone,
      technicalDetails,
    });
  }

  async function inspectJson(json: string): Promise<void> {
    await runAction(async () => {
      inspection = await inspectAnalysisPack(json);
      inspectedJson = inspection.valid ? json : "";
      reportAction(
        inspection.valid
          ? $translation("extensions-inspection-valid")
          : $translation("extensions-inspection-invalid-summary"),
        inspection.valid ? "success" : "warning",
        inspection.valid
          ? undefined
          : {
              code: inspection.code ?? "invalid_analysis_pack",
              operation: "analysis_pack_inspection",
            },
      );
    });
  }

  async function choosePack(file: File | null): Promise<void> {
    if (!file) return;
    if (file.size > 512 * 1024) {
      reportAction($translation("extensions-file-too-large"), "error");
      return;
    }
    await inspectJson(await file.text());
  }

  async function importInspected(): Promise<void> {
    if (!inspection?.valid || !inspectedJson) return;
    await runAction(async () => {
      const imported = await importAnalysisPack(inspectedJson);
      reportAction(
        $translation("extensions-imported", {
          name: imported.display_name,
        }),
        "success",
      );
      await refreshRuntime();
    });
  }

  async function setEnabled(pack: AnalysisPackSummary): Promise<void> {
    await runAction(async () => {
      if (pack.enabled) await disableAnalysisPack(pack.pack_id);
      else await enableAnalysisPack(pack.pack_id, pack.latest_revision);
      await refreshRuntime();
      reportAction(
        $translation(
          pack.enabled
            ? "extensions-disabled-notice"
            : "extensions-enabled-notice",
          { name: pack.display_name },
        ),
        "success",
      );
    });
  }

  async function rollback(pack: AnalysisPackSummary): Promise<void> {
    await runAction(async () => {
      await rollbackAnalysisPack(pack.pack_id);
      await refreshRuntime();
      reportAction(
        $translation("extensions-rollback-notice", {
          name: pack.display_name,
        }),
        "success",
      );
    });
  }

  async function removePack(pack: AnalysisPackSummary): Promise<void> {
    if (
      !window.confirm(
        $translation("extensions-remove-confirm", {
          name: pack.display_name,
        }),
      )
    )
      return;
    await runAction(async () => {
      await removeAnalysisPack(pack.pack_id);
      await refreshRuntime();
      reportAction(
        $translation("extensions-removed-notice", {
          name: pack.display_name,
        }),
        "success",
      );
    });
  }

  async function exportPack(pack: AnalysisPackSummary): Promise<void> {
    await runAction(async () => {
      const json = await exportAnalysisPack(pack.pack_id, pack.latest_revision);
      const url = URL.createObjectURL(
        new Blob([json], { type: "application/json" }),
      );
      const link = document.createElement("a");
      link.href = url;
      link.download = `${pack.pack_id}-${pack.semantic_version}.roanalysis.json`;
      link.click();
      URL.revokeObjectURL(url);
      reportAction(
        $translation("extensions-exported-notice", {
          name: pack.display_name,
        }),
        "success",
      );
    });
  }

  function chartSpec(
    contribution: AnalysisPackContribution,
    chart: ResolvedAnalysisChart,
  ) {
    return chartSpecForAnalysisContribution(contribution, chart, $translation);
  }

  function chartNavigation(
    chart: ResolvedAnalysisChart,
  ): ChartNavigationBinding[] {
    return chart.series.flatMap((series) =>
      series.published_metric_id
        ? series.points.map((_, pointIndex) => ({
            seriesId: series.id,
            pointIndex,
            destinations: destinationsForSubject({
              kind: "extension_contribution",
              metricId: series.published_metric_id!,
            }),
          }))
        : [],
    );
  }

  $effect(() => {
    const context = observationContext;
    if (!desktopAvailable || busy) return;
    if (!runtimeInitialised) {
      void runAction(async () => {
        await refreshRuntime();
        runtimeInitialised = true;
      });
      return;
    }
    if (context === loadedObservationContext) return;
    void runAction(async () => {
      contributions = await getAnalysisPackContributions();
      loadedObservationContext = context;
    });
  });
</script>

<section class="workspace extensions-workspace">
  <aside
    class="navigator"
    aria-label={$translation("extensions-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow"
          >{$translation("extensions-community-laboratory")}</span
        >
        <h2>{$translation("nav-extensions")}</h2>
      </div>
      <span class="edition"
        >{$translation(
          desktopAvailable ? "extensions-live" : "extensions-preview",
        )}</span
      >
    </div>
    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("extensions-host-api")}</span><strong>1</strong>
      </div>
      <div class="lens-row">
        <span>{$translation("extensions-schema")}</span><strong
          >Analysis Pack v1</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("extensions-runtime")}</span>
        <strong
          >{$translation(
            desktopAvailable
              ? "extensions-runtime-local"
              : "extension-permission-not-implemented",
          )}</strong
        >
      </div>
    </div>
    <div class="section-list">
      {#each sections as section}
        <a href={section.href} use:containedSectionNavigation
          ><span>{section.marker}</span>{$translation(section.label)}</a
        >
      {/each}
    </div>
    <GuidanceSurface kind="boundary" layout="compact" class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("extensions-boundary-note")}</p>
    </GuidanceSurface>
  </aside>

  <section class="canvas">
    <GuidanceSurface
      kind="preview"
      layout="inline"
      semanticRole="status"
      class="preview-banner"
    >
      <strong
        >{$translation(
          desktopAvailable
            ? "extensions-local-manager"
            : "synthetic-extensions-concept",
        )}</strong
      >
      <span
        >{$translation(
          desktopAvailable
            ? "extensions-manager-description"
            : "extension-permission-no-manager-host",
        )}</span
      >
    </GuidanceSurface>
    <WorkspaceSectionHeader
      level="page"
      eyebrow={$translation("extensions-heading-eyebrow")}
      title={$translation("extensions-heading-title")}
      description={$translation("security-extension-host-boundary")}
    >
      {#snippet actions()}
        <div class="date-stamp">
          <span
            >{$translation("extensions-installed-count", {
              count: packs.length,
            })}</span
          >
          <strong>{packs.filter((pack) => pack.enabled).length}</strong>
          <small>{$translation("extensions-enabled-count")}</small>
        </div>
      {/snippet}
    </WorkspaceSectionHeader>

    <section class="extension-hero" id="pack-inspection">
      <div class="extension-title">
        <span class="extension-glyph" aria-hidden="true">Ra</span>
        <div>
          <span class="eyebrow"
            >{$translation("extensions-analysis-pack-inspection")}</span
          >
          <h2
            lang={selectedPack?.default_locale ??
              receiverPackPreview.defaultLocale}
          >
            {selectedPack?.name ?? receiverPackPreview.name}
          </h2>
          <code>{selectedPack?.pack_id ?? receiverPackPreview.id}</code>
        </div>
      </div>
      <span class="validation-chip">
        {inspection
          ? $translation(
              inspection.valid ? "extensions-valid" : "extensions-invalid",
            )
          : $translation("extensions-awaiting-inspection")}
      </span>
    </section>

    <section
      class="extension-actions"
      aria-label={$translation("extensions-import-controls")}
    >
      <FilePicker
        id="analysis-pack-file-input"
        accept=".json,.roanalysis.json,application/json"
        disabled={!desktopAvailable || busy}
        label={$translation("extensions-choose-file")}
        showFileName={false}
        onselect={choosePack}
      />
      <button
        type="button"
        disabled={!desktopAvailable || busy}
        onclick={() => void inspectJson(receiverExampleJson)}
        >{$translation("extensions-inspect-example")}</button
      >
      <button
        type="button"
        class="primary-action"
        disabled={!inspection?.valid || !inspectedJson || busy}
        onclick={() => void importInspected()}
        >{$translation("extensions-import-inspected")}</button
      >
      <span>{$translation("extensions-import-separation")}</span>
      <ContextHelp
        topic="analysis-pack-actions"
        title={$translation("help-extension-actions-title")}
        text={$translation("help-extension-actions-text")}
        placement="left"
      />
    </section>

    {#if inspection}
      <section
        class="extension-facts"
        aria-label={$translation("extensions-identity-label")}
      >
        <article>
          <span>{$translation("extensions-author")}</span><strong
            lang={inspection.default_locale ?? "en-AU"}
            >{inspection.author ?? "—"}</strong
          >
        </article>
        <article>
          <span>{$translation("extensions-version")}</span><strong
            >{inspection.version ?? "—"}</strong
          >
        </article>
        <article>
          <span>{$translation("extensions-host-api")}</span><strong
            >{inspection.host_api_version ?? "—"}</strong
          >
        </article>
        <article>
          <span>{$translation("extensions-validation")}</span><strong
            >{inspection.valid
              ? $translation("extensions-valid")
              : inspection.code}</strong
          >
        </article>
      </section>
      {#if inspection.valid}
        <section class="contract-grid">
          <article class="contract-card">
            <header>
              <span class="eyebrow"
                >{$translation("extensions-normalised-inputs")}</span
              ><strong>{inspection.consumed_metrics.length}</strong>
            </header>
            <ul>
              {#each inspection.consumed_metrics as input}<li>
                  <code>{input}</code>
                </li>{/each}
            </ul>
          </article>
          <article class="contract-card">
            <header>
              <span class="eyebrow"
                >{$translation("extensions-derived-metrics")}</span
              ><strong>{inspection.derived_metrics.length}</strong>
            </header>
            <ul>
              {#each inspection.derived_metrics as metric}<li>
                  <code>{metric}</code>
                </li>{/each}
            </ul>
          </article>
          <article class="contract-card">
            <header>
              <span class="eyebrow"
                >{$translation("extensions-chart-contribution")}</span
              ><strong>{inspection.charts.length}</strong>
            </header>
            <ul>
              {#each inspection.charts as chart}<li
                  lang={inspection.default_locale ?? "en-AU"}
                >
                  {chart}
                </li>{/each}
            </ul>
            <small
              >{$translation("extension-permission-host-resolves-chart")}</small
            >
          </article>
        </section>
      {/if}
    {/if}

    <section class="lifecycle-panel extension-library" id="pack-library">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >{$translation("extensions-library-eyebrow")}</span
          >
          <h2>{$translation("extensions-library")}</h2>
          <p>{$translation("extensions-library-description")}</p>
        </div>
      </header>
      {#if packs.length === 0}
        <div class="extension-empty">
          {$translation("extensions-library-empty")}
        </div>
      {:else}
        <div class="extension-pack-list">
          {#each packs as pack (pack.pack_id)}
            <article>
              <div class="extension-pack-heading">
                <div lang={pack.default_locale}>
                  <strong>{pack.display_name}</strong><code>{pack.pack_id}</code
                  >
                </div>
                <span
                  class="status-chip"
                  data-status={pack.enabled ? "stable" : "watch"}
                  >{$translation(
                    pack.enabled ? "extensions-enabled" : "extensions-disabled",
                  )}</span
                >
              </div>
              <p lang={pack.default_locale}>{pack.description}</p>
              <div class="extension-pack-meta">
                <span>{pack.semantic_version}</span><span
                  >{$translation("extensions-revision", {
                    revision: pack.active_revision ?? pack.latest_revision,
                  })}</span
                ><span>{pack.content_hash.slice(0, 12)}</span>
              </div>
              <div class="extension-pack-actions">
                <button
                  type="button"
                  disabled={busy}
                  onclick={() => void setEnabled(pack)}
                  >{$translation(
                    pack.enabled ? "extensions-disable" : "extensions-enable",
                  )}</button
                >
                <button
                  type="button"
                  disabled={busy ||
                    !pack.enabled ||
                    (pack.active_revision ?? 0) <= 1}
                  onclick={() => void rollback(pack)}
                  >{$translation("extensions-rollback")}</button
                >
                <button
                  type="button"
                  disabled={busy}
                  onclick={() => void exportPack(pack)}
                  >{$translation("extensions-export")}</button
                >
                <button
                  type="button"
                  class="danger-action"
                  disabled={busy}
                  onclick={() => void removePack(pack)}
                  >{$translation("extensions-remove")}</button
                >
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </section>

    <section class="analysis-contributions" id="pack-charts">
      <header class="panel-heading">
        <div>
          <span class="eyebrow">{$translation("extensions-host-rendered")}</span
          >
          <h2>{$translation("extensions-analysis-contributions")}</h2>
          <p>{$translation("extensions-analysis-description")}</p>
        </div>
      </header>
      {#if contributions.length === 0}
        <div class="extension-empty">
          {$translation("extensions-analysis-empty")}
        </div>
      {:else}
        {#each contributions as contribution (contribution.pack_id)}
          {#each contribution.charts as chart (chart.id)}
            <div lang={contribution.default_locale}>
              <ObservatoryChart
                spec={chartSpec(contribution, chart)}
                eyebrow={$translation("evidence-extension-calculation")}
                height="320px"
                navigation={chartNavigation(chart)}
                {onrelatednavigate}
              />
            </div>
          {/each}
        {/each}
      {/if}
    </section>

    <section class="capability-panel">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >{$translation("extensions-deliberately-absent")}</span
          >
          <h2>{$translation("extensions-no-hidden-capabilities")}</h2>
          <p>{$translation("security-extension-unknown-fields")}</p>
        </div>
      </header>
      <div class="denied-grid">
        {#each deniedCapabilities as capability}<span
            ><i aria-hidden="true">×</i>{$translation(capability)}</span
          >{/each}
      </div>
    </section>

    <section class="model-plugin-panel" id="model-plugins">
      <div>
        <span class="eyebrow"
          >{$translation("extension-permission-planned-unavailable")}</span
        >
        <h2>{$translation("extensions-model-plugins")}</h2>
        <p>{$translation("security-model-plugin-boundary")}</p>
      </div>
      <span class="planned-stamp"
        >{$translation("extension-permission-not-implemented")}</span
      >
    </section>
  </section>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-focusable scroll region) -->
  <aside
    class="inspector"
    role="region"
    tabindex="0"
    aria-label={$translation("extensions-contract-inspector-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow"
          >{$translation("extensions-contract-inspector")}</span
        >
        <h2>Analysis Pack v1</h2>
      </div>
      <span class="status-chip" data-status="stable"
        >{$translation("extension-permission-inert")}</span
      >
    </div>
    <div class="selected-reading">
      <span>{$translation("extensions-file-suffix")}</span><strong
        >.roanalysis.json</strong
      >
      <small>{$translation("security-human-json-no-code")}</small>
      <p>{$translation("extensions-authoritative-validator")}</p>
    </div>
    <section class="evidence-ledger">
      <span class="eyebrow"
        >{$translation("extensions-host-responsibilities")}</span
      >
      <div>
        <strong>{$translation("extensions-data")}</strong><span
          >{$translation("extensions-data-responsibility")}</span
        >
      </div>
      <div>
        <strong>{$translation("extensions-calculation")}</strong><span
          >{$translation("extensions-calculation-responsibility")}</span
        >
      </div>
      <div>
        <strong>{$translation("extensions-presentation")}</strong><span
          >{$translation("extensions-presentation-responsibility")}</span
        >
      </div>
      <div>
        <strong>{$translation("extensions-trust")}</strong><span
          >{$translation("extensions-trust-responsibility")}</span
        >
      </div>
    </section>
    <section
      class="inspector-notes lifecycle-inspector"
      aria-labelledby="analysis-pack-lifecycle-title"
    >
      <header>
        <span class="eyebrow" id="analysis-pack-lifecycle-title"
          >{$translation("extensions-lifecycle-label")}</span
        >
        <ContextHelp
          topic="analysis-pack-lifecycle"
          title={$translation("help-extension-lifecycle-title")}
          text={$translation("help-extension-lifecycle-text")}
          placement="left"
        />
      </header>
      <ol>
        <li>
          <span aria-hidden="true">01</span>
          <div>
            <strong>{$translation("extensions-inspect")}</strong>
            <p>{$translation("extensions-inspect-description")}</p>
          </div>
        </li>
        <li>
          <span aria-hidden="true">02</span>
          <div>
            <strong>{$translation("extensions-validate")}</strong>
            <p>{$translation("extensions-validate-description")}</p>
          </div>
        </li>
        <li>
          <span aria-hidden="true">03</span>
          <div>
            <strong>{$translation("extensions-enable")}</strong>
            <p>{$translation("extensions-enable-description-live")}</p>
          </div>
        </li>
      </ol>
    </section>
  </aside>
</section>
