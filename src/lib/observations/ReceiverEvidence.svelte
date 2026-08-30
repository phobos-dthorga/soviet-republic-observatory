<script lang="ts">
  import { formatNumber } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import type {
    MetricEvidence,
    ReceiverDataset,
    ReceiverHistoryPoint,
  } from "./types";

  let { dataset }: { dataset: ReceiverDataset } = $props();
  const latest = $derived(dataset.points.at(-1));

  function latestValue(
    evidence: MetricEvidence,
    point: ReceiverHistoryPoint | undefined,
  ): string {
    if (!point) return $translation("chart-unavailable");
    const values: Record<string, number> = {
      "core.citizens.electronics.none": point.none,
      "core.citizens.electronics.radio": point.radio,
      "core.citizens.electronics.television": point.television,
      "core.citizens.electronics.computer": point.computer,
    };
    const value = values[evidence.metric_id];
    return value === undefined
      ? $translation("chart-unavailable")
      : formatNumber(value, $activeLocale);
  }
</script>

<article class="receiver-evidence-panel">
  <header class="panel-heading">
    <div>
      <span class="eyebrow">{$translation("evidence-save-fact")}</span>
      <h2>{$translation("evidence-receiver-observation-title")}</h2>
      <p>{$translation("evidence-receiver-observation-description")}</p>
    </div>
    <span class="coverage"
      >{$translation(
        dataset.coverage.status === "complete"
          ? "coverage-complete"
          : "coverage-partial",
      )}</span
    >
  </header>

  <div class="receiver-evidence-facts">
    <div>
      <span>{$translation("evidence-source-archive")}</span>
      <strong>{dataset.source_file_name}</strong>
    </div>
    <div>
      <span>{$translation("evidence-parser-version")}</span>
      <strong>{dataset.parser_version}</strong>
    </div>
    <div>
      <span>{$translation("compatibility-profile-evidence")}</span>
      <strong
        >{dataset.compatibility.mapping_classification === "player_mapped"
          ? $translation("compatibility-player-mapped")
          : $translation("compatibility-reviewed")}</strong
      >
      <small
        >{dataset.compatibility.profile_id} v{dataset.compatibility
          .profile_version}</small
      >
    </div>
    <div>
      <span>{$translation("evidence-branch-pending")}</span>
      <strong
        >{dataset.branch_id === "unassigned"
          ? $translation("observation-branch-unassigned")
          : dataset.branch_id}</strong
      >
    </div>
    <div>
      <span>{$translation("evidence-geographic-scope")}</span>
      <strong
        >{dataset.geographic_scope === "republic"
          ? $translation("observation-scope-republic")
          : dataset.geographic_scope}</strong
      >
    </div>
    <div class="receiver-evidence-wide">
      <span>{$translation("evidence-payload-sha")}</span>
      <code>{dataset.payload_hash}</code>
    </div>
    <div class="receiver-evidence-wide">
      <span>{$translation("compatibility-interpretation-id")}</span>
      <code>{dataset.interpretation_id}</code>
      <small>{$translation("compatibility-change-note")}</small>
    </div>
    <div class="receiver-evidence-wide">
      <span>{$translation("evidence-record-coverage")}</span>
      <strong
        >{$translation("coverage-receiver-records", {
          chartable: dataset.coverage.chartable_records,
          history: dataset.coverage.history_records,
          dropped: dataset.coverage.dropped_records,
        })}</strong
      >
    </div>
    <div class="receiver-evidence-wide">
      <span>{$translation("evidence-format-profile")}</span>
      <strong>{dataset.format_profile}</strong>
      <small>{$translation("evidence-format-profile-note")}</small>
    </div>
  </div>

  <section
    class="receiver-metric-table"
    aria-label={$translation("evidence-source-mapping")}
  >
    <header>
      <span>{$translation("evidence-metric-id")}</span>
      <span>{$translation("evidence-source-field")}</span>
      <span>{$translation("evidence-latest-value")}</span>
    </header>
    {#each dataset.source_fields as evidence}
      <div>
        <code>{evidence.metric_id}</code>
        <code>{evidence.source_field}</code>
        <strong>{latestValue(evidence, latest)}</strong>
        <small
          >{$translation("evidence-source-line", {
            line: evidence.latest_source_line,
          })}</small
        >
      </div>
    {/each}
  </section>

  <p class="receiver-evidence-caveat">
    {$translation("coverage-no-fields-inferred")}
  </p>
</article>
