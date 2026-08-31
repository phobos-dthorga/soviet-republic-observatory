<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import { createBroadcastPreview } from "../presentation/broadcastPreview";
  import { createObservedReceiverChart } from "../presentation/receiverObservation";
  import type { TranslationKey } from "../i18n/catalog";
  import { activeLocale, translation } from "../i18n/runtime";
  import ReceiverEvidence from "../observations/ReceiverEvidence.svelte";
  import type { ReceiverDataset } from "../observations/types";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";

  let { receiverDataset = null }: { receiverDataset?: ReceiverDataset | null } =
    $props();

  const sections: Array<{
    label: TranslationKey;
    href: string;
    marker: string;
  }> = [
    { label: "broadcast-section-receivers", href: "#receivers", marker: "01" },
    { label: "broadcast-section-audience", href: "#audience", marker: "02" },
    { label: "broadcast-section-programme", href: "#programme", marker: "03" },
    { label: "broadcast-section-outcomes", href: "#outcomes", marker: "04" },
    { label: "broadcast-section-bulletin", href: "#bulletin", marker: "05" },
  ];
  const stationIds = ["radio", "television"] as const;
  let selectedStation = $state<(typeof stationIds)[number]>("radio");
  const preview = $derived(createBroadcastPreview($translation, $activeLocale));
  const receiverLadder = $derived(
    receiverDataset
      ? createObservedReceiverChart(receiverDataset, $translation)
      : preview.receiverLadder,
  );
  const station = $derived(preview.station[selectedStation]);
</script>

<section class="workspace broadcast-workspace">
  <aside
    class="navigator"
    aria-label={$translation("broadcast-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("broadcast-editorial-desk")}</span>
        <h2>{$translation("nav-broadcast")}</h2>
      </div>
      <span class="edition">{$translation("broadcast-concept")}</span>
    </div>
    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("filter-branch")}</span><strong
          >{receiverDataset
            ? $translation("observation-branch-unassigned")
            : "planning-preview"}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("filter-window")}</span><strong
          >{receiverDataset
            ? $translation("observation-records", {
                count: receiverDataset.coverage.chartable_records,
              })
            : $translation("filter-rolling-days", { days: 360 })}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("filter-scope")}</span><strong
          >{$translation("filter-whole-republic")}</strong
        >
      </div>
    </div>
    <div class="section-list">
      {#each sections as section}<a href={section.href}
          ><span>{section.marker}</span>{$translation(section.label)}</a
        >{/each}
    </div>
    <GuidanceSurface kind="help" layout="compact" class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("evidence-broadcast-sidebar-note")}</p>
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
          receiverDataset
            ? "evidence-broadcast-mixed-desk"
            : "synthetic-broadcast-desk",
        )}</strong
      >
      <span
        >{$translation(
          receiverDataset
            ? "evidence-broadcast-mixed-detail"
            : "synthetic-no-station-telemetry",
        )}</span
      >
    </GuidanceSurface>
    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("broadcast-heading-eyebrow")}</span>
        <h2>{$translation("broadcast-heading-title")}</h2>
        <p>{$translation("broadcast-heading-description")}</p>
      </div>
      <div class="date-stamp">
        <span>{$translation("broadcast-bulletin")}</span><strong>20:00</strong
        ><small>{$translation("broadcast-draft-ready")}</small>
      </div>
    </header>

    <section id="receivers" class="broadcast-chart-wide">
      <ObservatoryChart
        spec={receiverLadder}
        height="285px"
        eyebrow={$translation("broadcast-section-receivers")}
      />
      {#if receiverDataset}
        <ReceiverEvidence dataset={receiverDataset} />
      {/if}
    </section>

    <section id="audience" class="broadcast-chart-wide">
      <div class="research-flag">
        <strong>{$translation("evidence-binary-research-candidate")}</strong>
        <span>{$translation("evidence-audience-unavailable")}</span>
      </div>
      <ObservatoryChart
        spec={preview.audienceReach}
        height="280px"
        eyebrow={$translation("broadcast-section-audience")}
      />
    </section>

    <section
      id="programme"
      class="broadcast-grid"
      aria-label={$translation("broadcast-programming-analysis")}
    >
      <article class="laboratory-card programme-card">
        <header>
          <div>
            <span class="eyebrow"
              >{$translation("broadcast-programme-formulation")}</span
            >
            <h3>{$translation("broadcast-intended-influence")}</h3>
          </div>
          <span class="coverage"
            >{$translation("broadcast-allocated", { percent: 100 })}</span
          >
        </header>
        <p>{$translation("synthetic-programme-formulation-note")}</p>
        <div class="programme-list">
          {#each preview.programmeMix as programme}
            <div>
              <span>{programme.label}</span><strong>{programme.value}%</strong
              ><i aria-hidden="true"
                ><b style={`width: ${programme.value}%`}></b></i
              >
            </div>
          {/each}
        </div>
      </article>
      <ObservatoryChart
        spec={preview.influenceAssay}
        eyebrow={$translation("broadcast-influence-assay")}
      />
    </section>

    <section id="outcomes" class="outcome-block">
      <div class="causation-warning" role="note">
        <strong>{$translation("causality-association-not-causation")}</strong>
        <span>{$translation("causality-comparison-warning")}</span>
      </div>
      <ObservatoryChart
        spec={preview.outcomeLaboratory}
        height="285px"
        eyebrow={$translation("broadcast-section-outcomes")}
      />
    </section>

    <section class="notebook-panel" aria-labelledby="notebook-title">
      <header class="panel-heading">
        <div>
          <span class="eyebrow">{$translation("broadcast-notebook")}</span>
          <h2 id="notebook-title">
            {$translation("broadcast-intervention-ledger")}
          </h2>
          <p>{$translation("evidence-annotations-not-evidence")}</p>
        </div>
        <span class="coverage"
          >{$translation("broadcast-open-notes", { count: 2 })}</span
        >
      </header>
      <div
        class="notebook-table"
        role="table"
        aria-label={$translation("synthetic-broadcast-experiments")}
      >
        <div class="notebook-row notebook-head" role="row">
          <span role="columnheader">{$translation("notebook-hypothesis")}</span
          ><span role="columnheader"
            >{$translation("notebook-intervention")}</span
          ><span role="columnheader">{$translation("filter-window")}</span><span
            role="columnheader">{$translation("notebook-status")}</span
          >
        </div>
        {#each preview.notebook as note}
          <div class="notebook-row" role="row">
            <span role="cell">{note.hypothesis}</span><span role="cell"
              >{note.intervention}</span
            ><span role="cell">{note.window}</span><span
              class="notebook-status"
              role="cell">{note.status}</span
            >
          </div>
        {/each}
      </div>
    </section>

    <section class="bulletin-panel" id="bulletin">
      <div class="bulletin-masthead">
        <span>{$translation("broadcast-evening-service")}</span><strong
          >{$translation("broadcast-republic-signal")}</strong
        ><time>Y4 · D050</time>
      </div>
      <div class="bulletin-body">
        <div class="dispatch-seal" aria-hidden="true">20</div>
        <div>
          <span class="eyebrow"
            >{$translation("synthetic-evening-bulletin-eyebrow")}</span
          >
          <h2>{$translation("broadcast-bulletin-title")}</h2>
          <p>{$translation("causality-bulletin-body")}</p>
          <div class="dispatch-links">
            <a href="#receivers"
              >{$translation("broadcast-receiver-evidence")}</a
            ><a href="#outcomes">{$translation("broadcast-outcome-caveats")}</a>
          </div>
        </div>
      </div>
    </section>
  </section>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-focusable scroll region) -->
  <aside
    class="inspector"
    role="region"
    tabindex="0"
    aria-label={$translation("broadcast-station-inspector-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow"
          >{$translation("broadcast-station-inspector")}</span
        >
        <h2>{station.name}</h2>
      </div>
      <span class="status-chip" data-status="watch"
        >{$translation("evidence-research")}</span
      >
    </div>
    <div
      class="station-switch"
      aria-label={$translation("broadcast-select-station")}
    >
      {#each stationIds as stationId}
        <button
          type="button"
          aria-pressed={selectedStation === stationId}
          class:active={selectedStation === stationId}
          onclick={() => (selectedStation = stationId)}
          >{preview.station[stationId].name}</button
        >
      {/each}
    </div>
    <div class="selected-reading">
      <span>{$translation("synthetic-rating")}</span><strong
        >{station.rating}</strong
      ><small>{station.availability}</small>
      <p>{$translation("synthetic-station-values-warning")}</p>
    </div>
    <div class="fact-grid">
      <article>
        <span>{$translation("station-workers")}</span><strong
          >{station.workers}</strong
        >
      </article>
      <article>
        <span>{$translation("station-professors")}</span><strong
          >{station.professors}</strong
        >
      </article>
      <article>
        <span>{$translation("station-potential-reach")}</span><strong
          >{station.potential}</strong
        >
      </article>
      <article>
        <span>{$translation("station-current-audience")}</span><strong
          >{station.current}</strong
        >
      </article>
    </div>
    <section class="evidence-ledger">
      <span class="eyebrow">{$translation("evidence-ledger")}</span>
      <div>
        <strong>{$translation("receiver-class")}</strong><span
          >{$translation("evidence-plain-text-save-fact")}</span
        >
      </div>
      <div>
        <strong>{$translation("station-staffing-capacity")}</strong><span
          >{$translation("evidence-game-definition")}</span
        >
      </div>
      <div>
        <strong>{$translation("station-state")}</strong><span
          >{$translation("evidence-binary-research-candidate")}</span
        >
      </div>
      <div>
        <strong>{$translation("station-outcome-attribution")}</strong><span
          >{$translation("causality-experimental-prohibited")}</span
        >
      </div>
    </section>
  </aside>
</section>
