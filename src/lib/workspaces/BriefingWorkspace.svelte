<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import type { EvidenceCoverage, EvidenceKind } from "../charts/types";
  import { createBriefingPreview, type MaterialCell } from "../data/sample";
  import type { TranslationKey } from "../i18n/catalog";
  import { activeLocale, translation } from "../i18n/runtime";

  const sections: Array<{
    label: TranslationKey;
    href: string;
    marker: string;
  }> = [
    { label: "briefing-section-state", href: "#briefing", marker: "01" },
    { label: "briefing-section-plan", href: "#plan", marker: "02" },
    { label: "briefing-section-materials", href: "#materials", marker: "03" },
    { label: "briefing-section-dispatch", href: "#dispatch", marker: "04" },
  ];
  const familyKeys: Record<MaterialCell["family"], TranslationKey> = {
    raw: "material-family-raw",
    industrial: "material-family-industrial",
    construction: "material-family-construction",
    consumer: "material-family-consumer",
    energy: "material-family-energy",
    waste: "material-family-waste",
  };
  const statusKeys: Record<MaterialCell["status"], TranslationKey> = {
    stable: "status-stable",
    watch: "status-watch",
    exposed: "status-exposed",
  };
  const evidenceKeys: Record<EvidenceKind, TranslationKey> = {
    save_fact: "evidence-save-fact",
    game_definition: "evidence-game-definition",
    calculation: "evidence-calculation",
    extension_calculation: "evidence-extension-calculation",
    player_override: "evidence-player-override",
    player_definition: "evidence-player-definition",
    estimate: "evidence-estimate",
    recommendation: "evidence-recommendation",
  };
  const coverageKeys: Record<EvidenceCoverage, TranslationKey> = {
    complete: "coverage-complete",
    partial: "coverage-partial",
    experimental: "coverage-experimental",
  };

  let selectedMaterialCode = $state("Ch");
  const preview = $derived(createBriefingPreview($translation, $activeLocale));
  const selectedMaterial = $derived(
    preview.materialCells.find((item) => item.code === selectedMaterialCode) ??
      preview.materialCells[0],
  );
</script>

<section class="workspace">
  <aside
    class="navigator"
    aria-label={$translation("briefing-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("briefing-directorate")}</span>
        <h2>{$translation("briefing-republic-brief")}</h2>
      </div>
      <span class="edition">v0.1</span>
    </div>

    <div class="lens-card">
      <label>
        {$translation("briefing-plan-label")}
        <select aria-label={$translation("briefing-selected-plan")} disabled>
          <option>{$translation("briefing-plan-name")}</option>
        </select>
      </label>
      <div class="lens-row">
        <span>{$translation("filter-window")}</span><strong
          >{$translation("filter-all-observations")}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("filter-currency")}</span><strong
          >{$translation("filter-separate-currencies")}</strong
        >
      </div>
    </div>

    <div class="section-list">
      {#each sections as section}
        <a href={section.href}
          ><span>{section.marker}</span>{$translation(section.label)}</a
        >
      {/each}
    </div>

    <div class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("synthetic-briefing-sidebar-note")}</p>
    </div>
  </aside>

  <section class="canvas" id="briefing">
    <div class="preview-banner" role="status">
      <strong>{$translation("synthetic-interface-foundation")}</strong>
      <span>{$translation("synthetic-no-real-save-values")}</span>
    </div>

    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("briefing-heading-eyebrow")}</span>
        <h2>{$translation("briefing-heading-title")}</h2>
        <p>{$translation("briefing-heading-description")}</p>
      </div>
      <div class="date-stamp">
        <span>{$translation("briefing-plan-year")}</span><strong>04 / 05</strong
        ><small>{$translation("briefing-plan-elapsed", { percent: 74 })}</small>
      </div>
    </header>

    <section class="kpi-grid" aria-label={$translation("briefing-kpis-label")}>
      {#each preview.kpis as kpi}
        <article class="kpi-card">
          <header>
            <span>{kpi.label}</span><span class="coverage"
              >{$translation(coverageKeys[kpi.coverage])}</span
            >
          </header>
          <strong>{kpi.value}</strong>
          <p>{kpi.change}</p>
          <footer>
            <span>{kpi.context}</span><span class="badge" data-kind={kpi.kind}
              >{$translation(evidenceKeys[kpi.kind])}</span
            >
          </footer>
        </article>
      {/each}
    </section>

    <section
      class="chart-grid"
      id="plan"
      aria-label={$translation("briefing-charts-label")}
    >
      <ObservatoryChart
        spec={preview.planProgress}
        eyebrow={$translation("briefing-section-plan")}
      />
      <ObservatoryChart
        spec={preview.importDependency}
        eyebrow={$translation("briefing-chart-external-dependency")}
      />
    </section>

    <section class="material-panel" id="materials">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >{$translation("briefing-material-eyebrow")}</span
          >
          <h2>{$translation("briefing-material-title")}</h2>
          <p>{$translation("briefing-material-description")}</p>
        </div>
        <div
          class="table-legend"
          aria-label={$translation("briefing-material-legend")}
        >
          <span><i class="stable"></i>{$translation("status-stable")}</span>
          <span><i class="watch"></i>{$translation("status-watch")}</span>
          <span><i class="exposed"></i>{$translation("status-exposed")}</span>
        </div>
      </header>

      <div class="material-table">
        {#each preview.materialCells as material}
          <button
            type="button"
            class="material-cell"
            class:selected={selectedMaterial.code === material.code}
            data-status={material.status}
            aria-pressed={selectedMaterial.code === material.code}
            aria-label={$translation("briefing-material-cell-label", {
              name: material.name,
              value: material.value,
            })}
            onclick={() => (selectedMaterialCode = material.code)}
          >
            <span class="material-code">{material.code}</span>
            <span class="material-value">{material.value}</span>
            <strong>{material.name}</strong>
            <small>{$translation(familyKeys[material.family])}</small>
            <span class="material-meter" aria-hidden="true"
              ><i style={`width: ${material.reliance}%`}></i></span
            >
            <span class="material-delta"
              >{$translation("briefing-points-value", {
                value: material.delta,
              })}</span
            >
          </button>
        {/each}
      </div>
    </section>

    <section class="dispatch-panel" id="dispatch">
      <div class="dispatch-seal" aria-hidden="true">04</div>
      <div>
        <span class="eyebrow"
          >{$translation("synthetic-ministry-dispatch-eyebrow")}</span
        >
        <h2>{$translation("briefing-dispatch-title")}</h2>
        <p>{$translation("briefing-dispatch-body")}</p>
        <div class="dispatch-links">
          <a href="#plan">{$translation("briefing-inspect-variance")}</a>
          <a href="#materials"
            >{$translation("briefing-open-material-evidence")}</a
          >
        </div>
      </div>
    </section>
  </section>

  <aside
    class="inspector"
    aria-label={$translation("briefing-inspector-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow"
          >{$translation("briefing-evidence-inspector")}</span
        >
        <h2>{selectedMaterial.name}</h2>
      </div>
      <span class="status-chip" data-status={selectedMaterial.status}
        >{$translation(statusKeys[selectedMaterial.status])}</span
      >
    </div>

    <div class="selected-reading">
      <span>{$translation("briefing-recorded-import-reliance")}</span>
      <strong>{selectedMaterial.value}</strong>
      <small
        >{$translation("briefing-comparison-points", {
          value: selectedMaterial.delta,
        })}</small
      >
      <p>{selectedMaterial.note}</p>
    </div>

    <div class="fact-grid">
      <article>
        <span>{$translation("briefing-family")}</span><strong
          >{$translation(familyKeys[selectedMaterial.family])}</strong
        >
      </article>
      <article>
        <span>{$translation("briefing-evidence")}</span><strong
          >{$translation("evidence-estimate")}</strong
        >
      </article>
      <article>
        <span>{$translation("briefing-coverage")}</span><strong
          >{$translation("coverage-experimental")}</strong
        >
      </article>
      <article>
        <span>{$translation("briefing-observed")}</span><strong
          >{$translation("synthetic-observed-day-230")}</strong
        >
      </article>
    </div>

    <section class="attention-queue">
      <header>
        <span class="eyebrow">{$translation("briefing-attention-queue")}</span
        ><strong
          >{$translation("briefing-findings-count", {
            count: preview.attention.length,
          })}</strong
        >
      </header>
      {#each preview.attention as item}
        <article>
          <span>{item.level}</span><strong>{item.title}</strong>
          <p>{item.detail}</p>
        </article>
      {/each}
    </section>

    <section class="provenance-key">
      <span class="eyebrow">{$translation("briefing-evidence-key")}</span>
      <div>
        <i data-kind="save_fact"></i><span
          >{$translation("evidence-save-fact")}</span
        >
      </div>
      <div>
        <i data-kind="calculation"></i><span
          >{$translation("evidence-calculation")}</span
        >
      </div>
      <div>
        <i data-kind="estimate"></i><span
          >{$translation("evidence-estimate")}</span
        >
      </div>
      <div>
        <i data-kind="recommendation"></i><span
          >{$translation("evidence-recommendation")}</span
        >
      </div>
    </section>
  </aside>
</section>
