<script lang="ts">
  import { receiverPackPreview } from "../data/extensionPreview";

  const sections = [
    { label: "Pack inspection", href: "#pack-inspection", marker: "01" },
    { label: "Data contract", href: "#pack-contract", marker: "02" },
    { label: "Lifecycle", href: "#pack-lifecycle", marker: "03" },
    { label: "Model Plugins", href: "#model-plugins", marker: "04" },
  ];
</script>

<section class="workspace extensions-workspace">
  <aside class="navigator" aria-label="Extensions concept navigation">
    <div class="aside-heading">
      <div>
        <span class="eyebrow">Community laboratory</span>
        <h2>Extensions</h2>
      </div>
      <span class="edition">Proof</span>
    </div>

    <div class="lens-card">
      <div class="lens-row"><span>Host API</span><strong>1</strong></div>
      <div class="lens-row">
        <span>Schema</span><strong>Analysis Pack v1</strong>
      </div>
      <div class="lens-row">
        <span>Runtime</span><strong>Not implemented</strong>
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
        This is an interface concept for a schema proof. It cannot import,
        install, enable, or execute an extension.
      </p>
    </div>
  </aside>

  <section class="canvas">
    <div class="preview-banner" role="status">
      <strong>Synthetic Extensions concept</strong>
      <span
        >No extension manager or executable plugin host exists in this pass.</span
      >
    </div>

    <header class="page-heading">
      <div>
        <span class="eyebrow">Declarative first · external code later</span>
        <h2>Community Extensions</h2>
        <p>
          The host supplies trusted observations, calculations, provenance, and
          rendering. Packs describe what they need; they do not bring their own
          little web browser.
        </p>
      </div>
      <div class="date-stamp">
        <span>Available now</span><strong>Schema</strong><small
          >Runtime later</small
        >
      </div>
    </header>

    <section class="extension-hero" id="pack-inspection">
      <div class="extension-title">
        <span class="extension-glyph" aria-hidden="true">Ra</span>
        <div>
          <span class="eyebrow">Non-executable Analysis Pack example</span>
          <h2>{receiverPackPreview.name}</h2>
          <code>{receiverPackPreview.id}</code>
        </div>
      </div>
      <span class="validation-chip">Valid proof</span>
    </section>

    <section class="extension-facts" aria-label="Analysis Pack identity">
      <article>
        <span>Author</span><strong>{receiverPackPreview.author}</strong>
      </article>
      <article>
        <span>Version</span><strong>{receiverPackPreview.version}</strong>
      </article>
      <article>
        <span>Host API</span><strong>{receiverPackPreview.hostApi}</strong>
      </article>
      <article>
        <span>Validation</span><strong>{receiverPackPreview.validation}</strong>
      </article>
    </section>

    <section class="contract-grid" id="pack-contract">
      <article class="contract-card">
        <header>
          <span class="eyebrow">Normalised inputs</span><strong>4</strong>
        </header>
        <ul>
          {#each receiverPackPreview.inputs as input}<li>
              <code>{input}</code>
            </li>{/each}
        </ul>
      </article>
      <article class="contract-card">
        <header>
          <span class="eyebrow">Derived metrics</span><strong>5</strong>
        </header>
        <ul>
          {#each receiverPackPreview.derivedMetrics as metric}<li>
              <code>{metric}</code>
            </li>{/each}
        </ul>
      </article>
      <article class="contract-card contribution-card">
        <header>
          <span class="eyebrow">Chart contribution</span><strong>1</strong>
        </header>
        <div
          class="mini-stack"
          aria-label="Illustration of a 100 percent stacked chart"
        >
          <i style="width: 10%"></i><i style="width: 25%"></i><i
            style="width: 45%"
          ></i><i style="width: 20%"></i>
        </div>
        <p>{receiverPackPreview.chart}</p>
        <small
          >Host resolves observations, evidence, colours, motion, and ECharts
          options.</small
        >
      </article>
    </section>

    <section class="capability-panel">
      <header class="panel-heading">
        <div>
          <span class="eyebrow">Deliberately absent</span>
          <h2>No hidden capabilities</h2>
          <p>
            Unknown fields fail validation; plain strings are never rendered as
            markup.
          </p>
        </div>
      </header>
      <div class="denied-grid">
        {#each receiverPackPreview.deniedCapabilities as capability}
          <span><i aria-hidden="true">×</i>{capability}</span>
        {/each}
      </div>
    </section>

    <section class="lifecycle-panel" id="pack-lifecycle">
      <header class="panel-heading">
        <div>
          <span class="eyebrow">Future local lifecycle</span>
          <h2>Inspection is not permission</h2>
        </div>
        <span class="coverage">Concept only</span>
      </header>
      <div class="lifecycle-flow" aria-label="Planned Analysis Pack lifecycle">
        <article>
          <span>01</span><strong>Inspect</strong>
          <p>Read identity, contents, and requested metrics.</p>
        </article>
        <i aria-hidden="true">→</i>
        <article>
          <span>02</span><strong>Validate</strong>
          <p>Check schema, semantics, compatibility, and content identity.</p>
        </article>
        <i aria-hidden="true">→</i>
        <article>
          <span>03</span><strong>Enable</strong>
          <p>A distinct future user choice; no control is implemented here.</p>
        </article>
      </div>
    </section>

    <section class="model-plugin-panel" id="model-plugins">
      <div>
        <span class="eyebrow">Planned · unavailable</span>
        <h2>Executable Model Plugins</h2>
        <p>
          Reserved for a demonstrated statistical model that exceeds the
          declarative operation catalogue. Any future program runs out of
          process and receives bounded normalised observations—not saves,
          binaries, SQLite, parser internals, paths, or rendering access.
        </p>
      </div>
      <span class="planned-stamp">Not implemented</span>
    </section>
  </section>

  <aside class="inspector" aria-label="Extension contract inspector">
    <div class="aside-heading">
      <div>
        <span class="eyebrow">Contract inspector</span>
        <h2>Analysis Pack v1</h2>
      </div>
      <span class="status-chip" data-status="stable">Inert</span>
    </div>

    <div class="selected-reading">
      <span>File suffix</span>
      <strong>.roanalysis.json</strong>
      <small>Human-readable JSON · no executable expressions</small>
      <p>
        The future Rust host remains the authoritative validator. Ajv proves
        conformance during development and keeps examples honest.
      </p>
    </div>

    <section class="evidence-ledger">
      <span class="eyebrow">Host responsibilities</span>
      <div>
        <strong>Data</strong><span
          >Normalisation, branches, dates, scope, gaps</span
        >
      </div>
      <div>
        <strong>Calculation</strong><span>Bounded operation evaluation</span>
      </div>
      <div>
        <strong>Presentation</strong><span
          >Charts, theme, motion, accessibility</span
        >
      </div>
      <div>
        <strong>Trust</strong><span
          >Provenance, limits, failure states, lifecycle</span
        >
      </div>
    </section>

    <section class="attention-queue">
      <header>
        <span class="eyebrow">Compatibility</span><strong>Separate axes</strong>
      </header>
      <article>
        <span>Contract</span><strong>Pack schema ≠ host API</strong>
        <p>Format and data vocabulary evolve independently.</p>
      </article>
      <article>
        <span>Runtime</span><strong>Protocol ≠ application version</strong>
        <p>Future executable compatibility will be explicit.</p>
      </article>
      <article>
        <span>Safety</span><strong>Process ≠ OS sandbox</strong>
        <p>
          Isolation reduces coupling; it is not a security boundary by itself.
        </p>
      </article>
    </section>
  </aside>
</section>
