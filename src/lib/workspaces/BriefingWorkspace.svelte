<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import {
    attentionPreview,
    importDependencySpec,
    kpiPreview,
    materialCells,
    planProgressSpec,
  } from "../data/sample";

  const sections = [
    { label: "State of the republic", href: "#briefing", marker: "01" },
    { label: "Five-Year Plan", href: "#plan", marker: "02" },
    { label: "Material table", href: "#materials", marker: "03" },
    { label: "Ministry dispatch", href: "#dispatch", marker: "04" },
  ];

  let selectedMaterialCode = $state("Ch");
  const selectedMaterial = $derived(
    materialCells.find((material) => material.code === selectedMaterialCode) ??
      materialCells[0],
  );
</script>

<section class="workspace">
  <aside class="navigator" aria-label="Briefing navigation">
    <div class="aside-heading">
      <div>
        <span class="eyebrow">Directorate</span>
        <h2>Republic brief</h2>
      </div>
      <span class="edition">v0.1</span>
    </div>

    <div class="lens-card">
      <label>
        Plan
        <select aria-label="Selected plan" disabled>
          <option>Industrial independence · 2001–2005</option>
        </select>
      </label>
      <div class="lens-row">
        <span>Window</span><strong>All observations</strong>
      </div>
      <div class="lens-row">
        <span>Currency</span><strong>Separate RUB / USD</strong>
      </div>
    </div>

    <div class="section-list">
      {#each sections as section}
        <a href={section.href}>
          <span>{section.marker}</span>
          {section.label}
        </a>
      {/each}
    </div>

    <div class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>
        This interface uses synthetic values. Connected observations will retain
        source, coverage, parser version, and actual game date.
      </p>
    </div>
  </aside>

  <section class="canvas" id="briefing">
    <div class="preview-banner" role="status">
      <strong>Synthetic interface foundation</strong>
      <span>No displayed number was read from a real save.</span>
    </div>

    <header class="page-heading">
      <div>
        <span class="eyebrow"
          >Central planning brief · latest distinct state</span
        >
        <h2>State of the republic</h2>
        <p>
          Outcomes first, followed by the movement and evidence that explain
          them.
        </p>
      </div>
      <div class="date-stamp">
        <span>Plan year</span><strong>04 / 05</strong><small>74% elapsed</small>
      </div>
    </header>

    <section class="kpi-grid" aria-label="Primary republic outcomes">
      {#each kpiPreview as kpi}
        <article class="kpi-card">
          <header>
            <span>{kpi.label}</span><span class="coverage">{kpi.coverage}</span>
          </header>
          <strong>{kpi.value}</strong>
          <p>{kpi.change}</p>
          <footer>
            <span>{kpi.context}</span>
            <span class="badge" data-kind={kpi.kind}
              >{kpi.kind.replaceAll("_", " ")}</span
            >
          </footer>
        </article>
      {/each}
    </section>

    <section
      class="chart-grid"
      id="plan"
      aria-label="Plan and dependency charts"
    >
      <ObservatoryChart spec={planProgressSpec} eyebrow="Five-Year Plan" />
      <ObservatoryChart
        spec={importDependencySpec}
        eyebrow="External dependency"
      />
    </section>

    <section class="material-panel" id="materials">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >Material Periodic Table · import-reliance lens</span
          >
          <h2>Strategic material field</h2>
          <p>
            Each cell is a resource index and a route into its complete evidence
            dossier.
          </p>
        </div>
        <div class="table-legend" aria-label="Material status legend">
          <span><i class="stable"></i> Stable</span>
          <span><i class="watch"></i> Watch</span>
          <span><i class="exposed"></i> Exposed</span>
        </div>
      </header>

      <div class="material-table">
        {#each materialCells as material}
          <button
            type="button"
            class="material-cell"
            class:selected={selectedMaterial.code === material.code}
            data-status={material.status}
            aria-pressed={selectedMaterial.code === material.code}
            aria-label={`${material.name}, ${material.value} recorded import reliance`}
            onclick={() => (selectedMaterialCode = material.code)}
          >
            <span class="material-code">{material.code}</span>
            <span class="material-value">{material.value}</span>
            <strong>{material.name}</strong>
            <small>{material.family}</small>
            <span class="material-meter" aria-hidden="true">
              <i style={`width: ${material.reliance}%`}></i>
            </span>
            <span class="material-delta">{material.delta} pts</span>
          </button>
        {/each}
      </div>
    </section>

    <section class="dispatch-panel" id="dispatch">
      <div class="dispatch-seal" aria-hidden="true">04</div>
      <div>
        <span class="eyebrow">Ministry Dispatch · deterministic preview</span>
        <h2>Industrial independence is improving unevenly</h2>
        <p>
          Fuel and steel exposure fell across the comparison window, while
          chemicals and electronic components remain the two largest
          constraints. The industrial plan is behind its scheduled path for a
          second consecutive quarter. Demographic balance remains positive,
          although city coverage is not represented in this preview.
        </p>
        <div class="dispatch-links">
          <a href="#plan">Inspect plan variance</a>
          <a href="#materials">Open material evidence</a>
        </div>
      </div>
    </section>
  </section>

  <aside class="inspector" aria-label="Evidence inspector">
    <div class="aside-heading">
      <div>
        <span class="eyebrow">Evidence inspector</span>
        <h2>{selectedMaterial.name}</h2>
      </div>
      <span class="status-chip" data-status={selectedMaterial.status}
        >{selectedMaterial.status}</span
      >
    </div>

    <div class="selected-reading">
      <span>Recorded import reliance</span>
      <strong>{selectedMaterial.value}</strong>
      <small>{selectedMaterial.delta} points against comparison window</small>
      <p>{selectedMaterial.note}</p>
    </div>

    <div class="fact-grid">
      <article>
        <span>Family</span><strong>{selectedMaterial.family}</strong>
      </article>
      <article><span>Evidence</span><strong>Estimate</strong></article>
      <article><span>Coverage</span><strong>Experimental</strong></article>
      <article><span>Observed</span><strong>2004 · day 230</strong></article>
    </div>

    <section class="attention-queue">
      <header>
        <span class="eyebrow">Attention queue</span><strong>3 findings</strong>
      </header>
      {#each attentionPreview as item}
        <article>
          <span>{item.level}</span>
          <strong>{item.title}</strong>
          <p>{item.detail}</p>
        </article>
      {/each}
    </section>

    <section class="provenance-key">
      <span class="eyebrow">Evidence key</span>
      <div><i data-kind="save_fact"></i><span>Parsed save fact</span></div>
      <div>
        <i data-kind="calculation"></i><span>Deterministic calculation</span>
      </div>
      <div>
        <i data-kind="estimate"></i><span>Estimate with assumptions</span>
      </div>
      <div>
        <i data-kind="recommendation"></i><span>Player recommendation</span>
      </div>
    </section>
  </aside>
</section>
