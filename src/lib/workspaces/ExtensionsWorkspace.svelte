<script lang="ts">
  import { receiverPackPreview } from "../data/extensionPreview";
  import type { TranslationKey } from "../i18n/catalog";
  import { translation } from "../i18n/runtime";

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
    {
      label: "extensions-section-contract",
      href: "#pack-contract",
      marker: "02",
    },
    {
      label: "extensions-section-lifecycle",
      href: "#pack-lifecycle",
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
      <span class="edition">{$translation("extensions-proof")}</span>
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
        <span>{$translation("extensions-runtime")}</span><strong
          >{$translation("extension-permission-not-implemented")}</strong
        >
      </div>
    </div>
    <div class="section-list">
      {#each sections as section}<a href={section.href}
          ><span>{section.marker}</span>{$translation(section.label)}</a
        >{/each}
    </div>
    <div class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("extension-permission-concept-only")}</p>
    </div>
  </aside>

  <section class="canvas">
    <div class="preview-banner" role="status">
      <strong>{$translation("synthetic-extensions-concept")}</strong><span
        >{$translation("extension-permission-no-manager-host")}</span
      >
    </div>
    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("extensions-heading-eyebrow")}</span
        >
        <h2>{$translation("extensions-heading-title")}</h2>
        <p>{$translation("security-extension-host-boundary")}</p>
      </div>
      <div class="date-stamp">
        <span>{$translation("extensions-available-now")}</span><strong
          >{$translation("extensions-schema")}</strong
        ><small>{$translation("extensions-runtime-later")}</small>
      </div>
    </header>

    <section class="extension-hero" id="pack-inspection">
      <div class="extension-title">
        <span class="extension-glyph" aria-hidden="true">Ra</span>
        <div>
          <span class="eyebrow"
            >{$translation("extensions-analysis-pack-example")}</span
          >
          <h2 lang={receiverPackPreview.defaultLocale}>
            {receiverPackPreview.name}
          </h2>
          <code>{receiverPackPreview.id}</code>
        </div>
      </div>
      <span class="validation-chip"
        >{$translation("evidence-extension-valid-proof")}</span
      >
    </section>

    <section
      class="extension-facts"
      aria-label={$translation("extensions-identity-label")}
    >
      <article>
        <span>{$translation("extensions-author")}</span><strong
          lang={receiverPackPreview.defaultLocale}
          >{receiverPackPreview.author}</strong
        >
      </article>
      <article>
        <span>{$translation("extensions-version")}</span><strong
          >{receiverPackPreview.version}</strong
        >
      </article>
      <article>
        <span>{$translation("extensions-host-api")}</span><strong
          >{receiverPackPreview.hostApi}</strong
        >
      </article>
      <article>
        <span>{$translation("extensions-validation")}</span><strong
          >{$translation("evidence-extension-validation-valid")}</strong
        >
      </article>
    </section>

    <section class="contract-grid" id="pack-contract">
      <article class="contract-card">
        <header>
          <span class="eyebrow"
            >{$translation("extensions-normalised-inputs")}</span
          ><strong>4</strong>
        </header>
        <ul>
          {#each receiverPackPreview.inputs as input}<li>
              <code>{input}</code>
            </li>{/each}
        </ul>
      </article>
      <article class="contract-card">
        <header>
          <span class="eyebrow"
            >{$translation("extensions-derived-metrics")}</span
          ><strong>5</strong>
        </header>
        <ul>
          {#each receiverPackPreview.derivedMetrics as metric}<li>
              <code>{metric}</code>
            </li>{/each}
        </ul>
      </article>
      <article class="contract-card contribution-card">
        <header>
          <span class="eyebrow"
            >{$translation("extensions-chart-contribution")}</span
          ><strong>1</strong>
        </header>
        <div
          class="mini-stack"
          aria-label={$translation("extensions-stack-chart-label")}
        >
          <i style="width: 10%"></i><i style="width: 25%"></i><i
            style="width: 45%"
          ></i><i style="width: 20%"></i>
        </div>
        <p lang={receiverPackPreview.defaultLocale}>
          {receiverPackPreview.chart}
        </p>
        <small>{$translation("extension-permission-host-resolves-chart")}</small
        >
      </article>
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

    <section class="lifecycle-panel" id="pack-lifecycle">
      <header class="panel-heading">
        <div>
          <span class="eyebrow"
            >{$translation("extensions-future-lifecycle")}</span
          >
          <h2>
            {$translation("extension-permission-inspection-not-permission")}
          </h2>
        </div>
        <span class="coverage"
          >{$translation("extension-permission-concept-only-label")}</span
        >
      </header>
      <div
        class="lifecycle-flow"
        aria-label={$translation("extensions-lifecycle-label")}
      >
        <article>
          <span>01</span><strong>{$translation("extensions-inspect")}</strong>
          <p>{$translation("extensions-inspect-description")}</p>
        </article>
        <i aria-hidden="true">→</i>
        <article>
          <span>02</span><strong>{$translation("extensions-validate")}</strong>
          <p>{$translation("extensions-validate-description")}</p>
        </article>
        <i aria-hidden="true">→</i>
        <article>
          <span>03</span><strong>{$translation("extensions-enable")}</strong>
          <p>{$translation("extension-permission-enable-description")}</p>
        </article>
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

  <aside
    class="inspector"
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
      ><small>{$translation("security-human-json-no-code")}</small>
      <p>{$translation("extensions-validator-description")}</p>
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
    <section class="attention-queue">
      <header>
        <span class="eyebrow">{$translation("extensions-compatibility")}</span
        ><strong>{$translation("extensions-separate-axes")}</strong>
      </header>
      <article>
        <span>{$translation("extensions-contract")}</span><strong
          >{$translation("extensions-pack-schema-host-api")}</strong
        >
        <p>{$translation("extensions-format-vocabulary")}</p>
      </article>
      <article>
        <span>{$translation("extensions-runtime")}</span><strong
          >{$translation("extensions-protocol-app-version")}</strong
        >
        <p>{$translation("extensions-runtime-compatibility")}</p>
      </article>
      <article>
        <span>{$translation("extensions-safety")}</span><strong
          >{$translation("security-process-not-sandbox")}</strong
        >
        <p>{$translation("security-isolation-caveat")}</p>
      </article>
    </section>
  </aside>
</section>
