<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import {
    audienceReachSpec,
    broadcastNotebookPreview,
    influenceAssaySpec,
    outcomeLaboratorySpec,
    programmeMixPreview,
    receiverLadderSpec,
    stationPreview,
  } from "../data/broadcastPreview";

  const sections = [
    { label: "Receiver ladder", href: "#receivers", marker: "01" },
    { label: "Audience desk", href: "#audience", marker: "02" },
    { label: "Programme assay", href: "#programme", marker: "03" },
    { label: "Outcome laboratory", href: "#outcomes", marker: "04" },
    { label: "Evening bulletin", href: "#bulletin", marker: "05" },
  ];

  let selectedStation = $state<keyof typeof stationPreview>("Radio");
  const station = $derived(stationPreview[selectedStation]);
</script>

<section class="workspace broadcast-workspace">
  <aside class="navigator" aria-label="Broadcast workspace navigation">
    <div class="aside-heading">
      <div>
        <span class="eyebrow">Editorial desk</span>
        <h2>Broadcast</h2>
      </div>
      <span class="edition">Concept</span>
    </div>

    <div class="lens-card">
      <div class="lens-row">
        <span>Branch</span><strong>planning-preview</strong>
      </div>
      <div class="lens-row">
        <span>Window</span><strong>Rolling 360 days</strong>
      </div>
      <div class="lens-row">
        <span>Scope</span><strong>Whole republic</strong>
      </div>
    </div>

    <div class="section-list">
      {#each sections as section}
        <a href={section.href}><span>{section.marker}</span>{section.label}</a>
      {/each}
    </div>

    <div class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>
        Receiver classes are available as plain-text save facts. Station reach,
        ratings, staffing, and outcomes shown here are synthetic research
        concepts.
      </p>
    </div>
  </aside>

  <section class="canvas">
    <div class="preview-banner" role="status">
      <strong>Synthetic Broadcast Desk</strong>
      <span>No station telemetry has been decoded or loaded.</span>
    </div>

    <header class="page-heading">
      <div>
        <span class="eyebrow">Signals, schedules, and citizen outcomes</span>
        <h2>Broadcast Desk</h2>
        <p>
          Treat radio and television as measurable public institutions—without
          mistaking a handsome correlation for a confession.
        </p>
      </div>
      <div class="date-stamp">
        <span>Bulletin</span><strong>20:00</strong><small>Draft ready</small>
      </div>
    </header>

    <section id="receivers" class="broadcast-chart-wide">
      <ObservatoryChart
        spec={receiverLadderSpec}
        height="285px"
        eyebrow="Receiver ladder"
      />
    </section>

    <section id="audience" class="broadcast-chart-wide">
      <div class="research-flag">
        <strong>Binary-research candidate</strong>
        <span
          >Potential and current listeners/viewers remain unavailable from
          supported saves.</span
        >
      </div>
      <ObservatoryChart
        spec={audienceReachSpec}
        height="280px"
        eyebrow="Audience desk"
      />
    </section>

    <section
      id="programme"
      class="broadcast-grid"
      aria-label="Programming analysis"
    >
      <article class="laboratory-card programme-card">
        <header>
          <div>
            <span class="eyebrow">Programme formulation</span>
            <h3>Illustrative intended influence</h3>
          </div>
          <span class="coverage">100% allocated</span>
        </header>
        <p>
          Six game-facing preferences shown as a formulation, not a
          prescription. Values are synthetic and do not claim the game applies a
          linear mixture.
        </p>
        <div class="programme-list">
          {#each programmeMixPreview as programme}
            <div>
              <span>{programme.label}</span><strong>{programme.value}%</strong>
              <i aria-hidden="true"
                ><b style={`width: ${programme.value}%`}></b></i
              >
            </div>
          {/each}
        </div>
      </article>
      <ObservatoryChart spec={influenceAssaySpec} eyebrow="Influence assay" />
    </section>

    <section id="outcomes" class="outcome-block">
      <div class="causation-warning" role="note">
        <strong>Association is not causation.</strong>
        <span>
          Compare aligned pre/post windows, contemporaneous changes, coverage,
          and plausible lags before attributing any citizen outcome to
          broadcasting.
        </span>
      </div>
      <ObservatoryChart
        spec={outcomeLaboratorySpec}
        height="285px"
        eyebrow="Outcome laboratory"
      />
    </section>

    <section class="notebook-panel" aria-labelledby="notebook-title">
      <header class="panel-heading">
        <div>
          <span class="eyebrow">Broadcast Notebook</span>
          <h2 id="notebook-title">Intervention ledger</h2>
          <p>
            Hypotheses and player changes remain annotations, not manufactured
            evidence.
          </p>
        </div>
        <span class="coverage">2 open notes</span>
      </header>
      <div
        class="notebook-table"
        role="table"
        aria-label="Synthetic broadcast experiments"
      >
        <div class="notebook-row notebook-head" role="row">
          <span role="columnheader">Hypothesis</span>
          <span role="columnheader">Intervention</span>
          <span role="columnheader">Window</span>
          <span role="columnheader">Status</span>
        </div>
        {#each broadcastNotebookPreview as note}
          <div class="notebook-row" role="row">
            <span role="cell">{note.hypothesis}</span>
            <span role="cell">{note.intervention}</span>
            <span role="cell">{note.window}</span>
            <span class="notebook-status" role="cell">{note.status}</span>
          </div>
        {/each}
      </div>
    </section>

    <section class="bulletin-panel" id="bulletin">
      <div class="bulletin-masthead">
        <span>РО · Evening service</span><strong>Republic signal</strong><time
          >Y4 · D050</time
        >
      </div>
      <div class="bulletin-body">
        <div class="dispatch-seal" aria-hidden="true">20</div>
        <div>
          <span class="eyebrow">Evening Bulletin · deterministic preview</span>
          <h2>Television advances; the wireless remains indispensable</h2>
          <p>
            Television is the largest receiver class in this synthetic
            observation, while radio still accounts for one quarter of
            classified citizens. The illustrated education and propaganda
            schedule coincides with a gradual loyalty rise, but the Ministry
            declines to award the transmitter a medal until decoded audience
            evidence and a longer comparison window arrive.
          </p>
          <div class="dispatch-links">
            <a href="#receivers">Receiver evidence</a>
            <a href="#outcomes">Outcome caveats</a>
          </div>
        </div>
      </div>
    </section>
  </section>

  <aside class="inspector" aria-label="Broadcast station inspector">
    <div class="aside-heading">
      <div>
        <span class="eyebrow">Station inspector</span>
        <h2>{selectedStation}</h2>
      </div>
      <span class="status-chip" data-status="watch">Research</span>
    </div>

    <div class="station-switch" aria-label="Select station">
      {#each Object.keys(stationPreview) as stationName}
        <button
          type="button"
          aria-pressed={selectedStation === stationName}
          class:active={selectedStation === stationName}
          onclick={() =>
            (selectedStation = stationName as keyof typeof stationPreview)}
          >{stationName}</button
        >
      {/each}
    </div>

    <div class="selected-reading">
      <span>Synthetic rating</span>
      <strong>{station.rating}</strong>
      <small>{station.availability}</small>
      <p>
        Displayed station values demonstrate the intended inspector only. They
        are not save facts and are not estimates safe for administration.
      </p>
    </div>

    <div class="fact-grid">
      <article><span>Workers</span><strong>{station.workers}</strong></article>
      <article>
        <span>Professors</span><strong>{station.professors}</strong>
      </article>
      <article>
        <span>Potential reach</span><strong>{station.potential}</strong>
      </article>
      <article>
        <span>Current audience</span><strong>{station.current}</strong>
      </article>
    </div>

    <section class="evidence-ledger">
      <span class="eyebrow">Evidence ledger</span>
      <div>
        <strong>Receiver class</strong><span
          >Plain-text save fact available</span
        >
      </div>
      <div>
        <strong>Staffing capacity</strong><span>Game-definition fact</span>
      </div>
      <div>
        <strong>Station state</strong><span>Binary research candidate</span>
      </div>
      <div>
        <strong>Outcome attribution</strong><span
          >Experimental; causal claim prohibited</span
        >
      </div>
    </section>
  </aside>
</section>
