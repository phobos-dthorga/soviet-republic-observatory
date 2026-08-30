<script lang="ts">
  import type { TranslationKey } from "../i18n/catalog";
  import { formatDate, formatNumber, formatSignedNumber } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import type {
    ArchiveObservation,
    ArchiveComparison,
    ArchiveOverview,
    TimelineBranch,
  } from "../observations/types";

  let {
    archive,
    desktopAvailable,
    onselect,
    oninspect,
    oncontinue,
    onrename,
    onreturn,
    oncompare,
  }: {
    archive: ArchiveOverview | null;
    desktopAvailable: boolean;
    onselect: (branchId: string) => Promise<void>;
    oninspect: (interpretationId: string) => Promise<void>;
    oncontinue: (interpretationId: string, label: string) => Promise<void>;
    onrename: (branchId: string, label: string | null) => Promise<void>;
    onreturn: () => Promise<void>;
    oncompare: (
      fromPayloadHash: string,
      toPayloadHash: string,
    ) => Promise<ArchiveComparison>;
  } = $props();

  let selecting = $state(false);
  let selectionError = $state(false);
  let comparisonBusy = $state(false);
  let comparisonError = $state(false);
  let comparisonFrom = $state("");
  let comparisonTo = $state("");
  let comparisonBranch = $state("");
  let comparison = $state<ArchiveComparison | null>(null);
  let contextBusy = $state(false);
  let contextError = $state(false);
  let branchLabelDraft = $state("");
  const selectedBranch = $derived(
    archive?.branches.find((branch) => branch.selected) ?? null,
  );
  const branchObservations = $derived(
    (archive?.observations ?? [])
      .filter(
        (observation) =>
          observation.included_in_context &&
          archive?.selected_branch_id !== "unassigned",
      )
      .sort(
        (left, right) =>
          (left.context_sequence ?? Number.MAX_SAFE_INTEGER) -
          (right.context_sequence ?? Number.MAX_SAFE_INTEGER),
      ),
  );

  $effect(() => {
    const nextBranch = archive?.selected_branch_id ?? "";
    if (nextBranch === comparisonBranch) return;
    comparisonBranch = nextBranch;
    comparisonFrom = branchObservations.at(-1)?.interpretation_id ?? "";
    comparisonTo = branchObservations[0]?.interpretation_id ?? "";
    comparison = null;
    comparisonError = false;
  });

  $effect(() => {
    branchLabelDraft = selectedBranch?.player_label ?? "";
  });

  const relationshipKeys: Record<
    ArchiveObservation["relationship"],
    TranslationKey
  > = {
    root: "archive-relationship-root",
    successor: "archive-relationship-successor",
    equivalent_history: "archive-relationship-equivalent",
    rollback_fork: "archive-relationship-rollback-fork",
    divergent_fork: "archive-relationship-divergent-fork",
    ambiguous: "archive-relationship-ambiguous",
    continuation_anchor: "archive-relationship-continuation-anchor",
  };

  function branchLabel(branch: TimelineBranch): string {
    if (branch.player_label) return branch.player_label;
    if (branch.branch_kind === "main")
      return $translation("archive-branch-main");
    if (branch.branch_kind === "unassigned")
      return $translation("archive-branch-unassigned");
    return $translation(
      branch.origin === "manual_continuation"
        ? "archive-branch-continuation"
        : "archive-branch-fork",
      {
        identity: branch.short_identity,
      },
    );
  }

  async function inspectObservation(interpretationId: string): Promise<void> {
    if (contextBusy) return;
    contextBusy = true;
    contextError = false;
    try {
      await oninspect(interpretationId);
    } catch {
      contextError = true;
    } finally {
      contextBusy = false;
    }
  }

  async function continueFrom(observation: ArchiveObservation): Promise<void> {
    if (contextBusy) return;
    const ordinal =
      (archive?.branches ?? []).filter(
        (branch) =>
          branch.origin === "manual_continuation" &&
          branch.anchor_interpretation_id === observation.interpretation_id,
      ).length + 1;
    const label = $translation("archive-continuation-default-label", {
      date: gameDate(observation.latest_year, observation.latest_day),
      ordinal,
    });
    if (!window.confirm($translation("archive-continuation-confirm"))) return;
    contextBusy = true;
    contextError = false;
    try {
      await oncontinue(observation.interpretation_id, label);
    } catch {
      contextError = true;
    } finally {
      contextBusy = false;
    }
  }

  async function returnLatest(): Promise<void> {
    if (contextBusy) return;
    contextBusy = true;
    contextError = false;
    try {
      await onreturn();
    } catch {
      contextError = true;
    } finally {
      contextBusy = false;
    }
  }

  async function saveBranchLabel(): Promise<void> {
    if (!selectedBranch || contextBusy) return;
    contextBusy = true;
    contextError = false;
    try {
      await onrename(selectedBranch.branch_id, branchLabelDraft.trim() || null);
    } catch {
      contextError = true;
    } finally {
      contextBusy = false;
    }
  }

  function gameDate(year: number | null, day: number | null): string {
    if (year === null || day === null)
      return $translation("archive-date-unavailable");
    return $translation("observation-game-date-compact", {
      year,
      day: String(day).padStart(3, "0"),
    });
  }

  async function selectBranch(branchId: string): Promise<void> {
    if (selecting || branchId === archive?.selected_branch_id) return;
    selecting = true;
    selectionError = false;
    try {
      await onselect(branchId);
    } catch {
      selectionError = true;
    } finally {
      selecting = false;
    }
  }

  async function compareObservations(): Promise<void> {
    if (
      comparisonBusy ||
      !comparisonFrom ||
      !comparisonTo ||
      comparisonFrom === comparisonTo
    )
      return;
    comparisonBusy = true;
    comparisonError = false;
    try {
      comparison = await oncompare(comparisonFrom, comparisonTo);
    } catch {
      comparison = null;
      comparisonError = true;
    } finally {
      comparisonBusy = false;
    }
  }

  function observationOption(observation: ArchiveObservation): string {
    return `${gameDate(observation.latest_year, observation.latest_day)} · ${observation.source_file_name}`;
  }

  const receiverLabels: Record<string, TranslationKey> = {
    "core.citizens.electronics.none": "receiver-none",
    "core.citizens.electronics.radio": "receiver-radio",
    "core.citizens.electronics.television": "receiver-television",
    "core.citizens.electronics.computer": "receiver-computer",
  };
</script>

<section class="workspace archive-workspace">
  <aside
    class="navigator"
    aria-label={$translation("archive-navigation-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("archive-directorate")}</span>
        <h2>{$translation("nav-archive")}</h2>
      </div>
      <span class="edition">{$translation("archive-local")}</span>
    </div>

    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("archive-files")}</span>
        <strong
          >{formatNumber(
            archive?.file_observation_count ?? 0,
            $activeLocale,
          )}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("archive-distinct-states")}</span>
        <strong
          >{formatNumber(
            archive?.distinct_state_count ?? 0,
            $activeLocale,
          )}</strong
        >
      </div>
      <div class="lens-row">
        <span>{$translation("archive-branches")}</span>
        <strong
          >{formatNumber(archive?.branches.length ?? 0, $activeLocale)}</strong
        >
      </div>
    </div>

    <div
      class="archive-branch-list"
      role="group"
      aria-label={$translation("archive-branch-list-label")}
    >
      {#each archive?.branches ?? [] as branch}
        <button
          type="button"
          class:active={branch.selected}
          aria-pressed={branch.selected}
          disabled={selecting}
          onclick={() => void selectBranch(branch.branch_id)}
        >
          <span aria-hidden="true"
            >{branch.branch_kind === "fork" ? "↳" : "◆"}</span
          >
          <span>
            <strong>{branchLabel(branch)}</strong>
            <small>{gameDate(branch.latest_year, branch.latest_day)}</small>
          </span>
          <b>{formatNumber(branch.observation_count, $activeLocale)}</b>
        </button>
      {/each}
    </div>

    <div class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("archive-branch-safety-note")}</p>
    </div>
  </aside>

  <section class="canvas">
    <div class="preview-banner archive-banner" role="status">
      <strong>{$translation("archive-evidence-prefix")}</strong>
      <span>{$translation("archive-evidence-explanation")}</span>
    </div>

    {#if archive?.analysis_context.mode === "historical_preview"}
      <div class="archive-context-banner" role="status">
        <div>
          <strong>{$translation("archive-historical-preview")}</strong>
          <span>{$translation("archive-historical-preview-detail")}</span>
        </div>
        <button
          type="button"
          disabled={contextBusy}
          onclick={() => void returnLatest()}
        >
          {$translation("return-latest")}
        </button>
      </div>
    {/if}

    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("archive-heading-eyebrow")}</span>
        <h2>{$translation("archive-heading-title")}</h2>
        <p>{$translation("archive-heading-description")}</p>
      </div>
      {#if selectedBranch}
        <div class="date-stamp">
          <span>{$translation("archive-selected-branch")}</span>
          <strong>{branchLabel(selectedBranch)}</strong>
          <small
            >{gameDate(
              selectedBranch.latest_year,
              selectedBranch.latest_day,
            )}</small
          >
        </div>
      {/if}
    </header>

    {#if selectionError}
      <p class="language-error" role="alert">
        {$translation("error-observer-unknown-branch")}
      </p>
    {/if}
    {#if contextError}
      <p class="language-error" role="alert">
        {$translation("archive-context-action-failed")}
      </p>
    {/if}

    {#if !desktopAvailable}
      <section class="archive-empty-state">
        <strong>{$translation("archive-desktop-required")}</strong>
        <p>{$translation("archive-desktop-required-detail")}</p>
      </section>
    {:else if !archive || archive.distinct_state_count === 0}
      <section class="archive-empty-state">
        <strong>{$translation("archive-empty")}</strong>
        <p>{$translation("archive-empty-detail")}</p>
      </section>
    {:else}
      <section
        class="kpi-grid archive-kpis"
        aria-label={$translation("archive-summary-label")}
      >
        <article class="kpi-card">
          <header><span>{$translation("archive-files")}</span></header>
          <strong
            >{formatNumber(
              archive.file_observation_count,
              $activeLocale,
            )}</strong
          >
          <p>{$translation("archive-files-description")}</p>
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("archive-distinct-states")}</span>
          </header>
          <strong
            >{formatNumber(archive.distinct_state_count, $activeLocale)}</strong
          >
          <p>{$translation("archive-states-description")}</p>
        </article>
        <article class="kpi-card">
          <header><span>{$translation("archive-unresolved")}</span></header>
          <strong
            >{formatNumber(
              archive.unresolved_state_count,
              $activeLocale,
            )}</strong
          >
          <p>{$translation("archive-unresolved-description")}</p>
        </article>
      </section>

      <section
        class="archive-comparison"
        aria-labelledby="archive-comparison-title"
      >
        <header>
          <div>
            <span class="eyebrow"
              >{$translation("archive-comparison-eyebrow")}</span
            >
            <h3 id="archive-comparison-title">
              {$translation("archive-comparison-title")}
            </h3>
          </div>
          <span class="status-chip" data-status="stable">
            {$translation("evidence-calculation")}
          </span>
        </header>

        {#if archive.selected_branch_id === "unassigned"}
          <p>{$translation("archive-comparison-unassigned")}</p>
        {:else if branchObservations.length < 2}
          <p>{$translation("archive-comparison-needs-two")}</p>
        {:else}
          <div class="archive-comparison-controls">
            <label>
              <span>{$translation("archive-comparison-baseline")}</span>
              <select bind:value={comparisonFrom} disabled={comparisonBusy}>
                {#each branchObservations as observation}
                  <option value={observation.interpretation_id}>
                    {observationOption(observation)}
                  </option>
                {/each}
              </select>
            </label>
            <span aria-hidden="true">→</span>
            <label>
              <span>{$translation("archive-comparison-target")}</span>
              <select bind:value={comparisonTo} disabled={comparisonBusy}>
                {#each branchObservations as observation}
                  <option value={observation.interpretation_id}>
                    {observationOption(observation)}
                  </option>
                {/each}
              </select>
            </label>
            <button
              type="button"
              disabled={comparisonBusy || comparisonFrom === comparisonTo}
              onclick={() => void compareObservations()}
            >
              {comparisonBusy
                ? $translation("archive-comparison-working")
                : $translation("archive-comparison-action")}
            </button>
          </div>
          {#if comparisonError}
            <p class="language-error" role="alert">
              {$translation("error-observer-incompatible-comparison")}
            </p>
          {/if}
          {#if comparison}
            <div class="archive-comparison-result" aria-live="polite">
              <div class="archive-comparison-window">
                <div>
                  <span>{$translation("archive-comparison-baseline")}</span>
                  <strong
                    >{gameDate(
                      comparison.from.year,
                      comparison.from.day,
                    )}</strong
                  >
                  <small>{comparison.from.source_file_name}</small>
                </div>
                <b aria-hidden="true">→</b>
                <div>
                  <span>{$translation("archive-comparison-target")}</span>
                  <strong
                    >{gameDate(comparison.to.year, comparison.to.day)}</strong
                  >
                  <small>{comparison.to.source_file_name}</small>
                </div>
                <div>
                  <span>{$translation("archive-comparison-elapsed")}</span>
                  <strong
                    >{formatSignedNumber(
                      comparison.elapsed_game_days,
                      $activeLocale,
                    )}</strong
                  >
                  <small>{$translation("unit-game-days")}</small>
                </div>
              </div>
              <div
                class="archive-change-grid"
                aria-label={$translation("archive-comparison-receiver-changes")}
              >
                {#each comparison.receiver_changes as change}
                  <article
                    data-movement={change.delta > 0
                      ? "increase"
                      : change.delta < 0
                        ? "decrease"
                        : "steady"}
                  >
                    <span
                      >{$translation(
                        receiverLabels[change.metric_id] ?? "receiver-none",
                      )}</span
                    >
                    <strong
                      >{formatSignedNumber(change.delta, $activeLocale)}</strong
                    >
                    <small
                      >{formatNumber(change.from_value, $activeLocale)} →
                      {formatNumber(change.to_value, $activeLocale)}</small
                    >
                  </article>
                {/each}
                <article
                  data-movement={comparison.classified_total_change.delta > 0
                    ? "increase"
                    : comparison.classified_total_change.delta < 0
                      ? "decrease"
                      : "steady"}
                >
                  <span
                    >{$translation("archive-comparison-classified-total")}</span
                  >
                  <strong
                    >{formatSignedNumber(
                      comparison.classified_total_change.delta,
                      $activeLocale,
                    )}</strong
                  >
                  <small
                    >{formatNumber(
                      comparison.classified_total_change.from_value,
                      $activeLocale,
                    )} → {formatNumber(
                      comparison.classified_total_change.to_value,
                      $activeLocale,
                    )}</small
                  >
                </article>
              </div>
              <p class="archive-snapshot-summary">
                {$translation("archive-comparison-snapshot-summary", {
                  republic: comparison.to.republic_snapshot_fields,
                  cities: comparison.to.city_snapshot_count,
                  fields: comparison.to.city_snapshot_fields,
                })}
              </p>
            </div>
          {/if}
        {/if}
      </section>

      <section
        class="archive-ledger"
        aria-label={$translation("archive-ledger-label")}
      >
        <header>
          <div>
            <span class="eyebrow">{$translation("archive-ledger-eyebrow")}</span
            >
            <h3>{$translation("archive-ledger-title")}</h3>
          </div>
          <span class="status-chip" data-status="stable">
            {$translation("archive-read-only")}
          </span>
        </header>
        <div class="archive-ledger-head" aria-hidden="true">
          <span>{$translation("archive-column-save")}</span>
          <span>{$translation("archive-column-game-date")}</span>
          <span>{$translation("archive-column-lineage")}</span>
          <span>{$translation("archive-column-evidence")}</span>
        </div>
        {#each archive.observations as observation}
          <article
            class:selected={observation.included_in_context}
            class:active-head={observation.active_head}
          >
            <div>
              <strong>{observation.source_file_name}</strong>
              <small
                >{formatDate(observation.imported_at_ms, $activeLocale, {
                  dateStyle: "medium",
                  timeStyle: "short",
                })}</small
              >
              <div class="archive-row-actions">
                <button
                  type="button"
                  disabled={contextBusy || observation.active_head}
                  onclick={() =>
                    void inspectObservation(observation.interpretation_id)}
                  >{$translation("archive-inspect-save")}</button
                >
                <button
                  type="button"
                  disabled={contextBusy ||
                    observation.branch_id === "unassigned"}
                  onclick={() => void continueFrom(observation)}
                  >{$translation("archive-continue-save")}</button
                >
              </div>
            </div>
            <div>
              <strong
                >{gameDate(
                  observation.latest_year,
                  observation.latest_day,
                )}</strong
              >
              <small
                >{$translation("observation-records", {
                  count: observation.history_records,
                })}</small
              >
            </div>
            <div>
              <strong
                >{$translation(
                  relationshipKeys[observation.relationship],
                )}</strong
              >
              <small
                >{$translation("archive-shared-records", {
                  count: observation.shared_record_count,
                })}</small
              >
            </div>
            <div>
              <span
                class="status-chip"
                data-status={observation.mapping_classification ===
                "player_mapped"
                  ? "watch"
                  : observation.coverage_status === "complete"
                    ? "stable"
                    : "watch"}
              >
                {$translation(
                  observation.mapping_classification === "player_mapped"
                    ? "compatibility-player-mapped"
                    : "compatibility-reviewed",
                )}
              </span>
              <code title={observation.interpretation_id}
                >{observation.interpretation_id.slice(0, 12)}</code
              >
              <small
                >{$translation("archive-snapshot-evidence", {
                  republic: observation.republic_snapshot_fields,
                  cities: observation.city_snapshot_count,
                })} · {observation.profile_id} v{observation.profile_version}</small
              >
            </div>
          </article>
        {/each}
      </section>
    {/if}
  </section>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-focusable scroll region) -->
  <aside
    class="inspector"
    role="region"
    tabindex="0"
    aria-label={$translation("archive-inspector-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("archive-inspector-eyebrow")}</span>
        <h2>{$translation("archive-inspector-title")}</h2>
      </div>
      <span class="edition">{$translation("archive-deterministic")}</span>
    </div>

    {#if selectedBranch}
      <div class="archive-selected-card">
        <span>{$translation("archive-selected-branch")}</span>
        <strong>{branchLabel(selectedBranch)}</strong>
        <small><code>{selectedBranch.branch_id}</code></small>
      </div>
      <div class="evidence-ledger archive-evidence-ledger">
        <div>
          <span>{$translation("archive-branch-kind")}</span>
          <strong
            >{$translation(
              selectedBranch.branch_kind === "main"
                ? "archive-kind-main"
                : selectedBranch.branch_kind === "fork"
                  ? "archive-kind-fork"
                  : "archive-kind-unassigned",
            )}</strong
          >
        </div>
        <div>
          <span>{$translation("archive-parent-branch")}</span>
          <strong
            >{selectedBranch.parent_branch_id ??
              $translation("archive-none")}</strong
          >
        </div>
        <div>
          <span>{$translation("archive-fork-record")}</span>
          <strong
            >{selectedBranch.fork_record_id ??
              $translation("archive-none")}</strong
          >
        </div>
        <div>
          <span>{$translation("archive-states")}</span>
          <strong
            >{formatNumber(
              selectedBranch.observation_count,
              $activeLocale,
            )}</strong
          >
        </div>
      </div>
      {#if selectedBranch.origin === "manual_continuation"}
        <form
          class="archive-label-editor"
          onsubmit={(event) => {
            event.preventDefault();
            void saveBranchLabel();
          }}
        >
          <label>
            <span>{$translation("archive-branch-label")}</span>
            <input
              maxlength="120"
              value={branchLabelDraft || selectedBranch.player_label || ""}
              oninput={(event) =>
                (branchLabelDraft = event.currentTarget.value)}
            />
          </label>
          <button type="submit" disabled={contextBusy}
            >{$translation("archive-save-label")}</button
          >
        </form>
      {/if}
    {:else}
      <p class="archive-inspector-empty">
        {$translation("archive-no-selected-branch")}
      </p>
    {/if}

    <section class="archive-rule-key">
      <span class="eyebrow">{$translation("archive-rule-key")}</span>
      <div><i>→</i><span>{$translation("archive-rule-successor")}</span></div>
      <div><i>↳</i><span>{$translation("archive-rule-fork")}</span></div>
      <div><i>?</i><span>{$translation("archive-rule-ambiguous")}</span></div>
    </section>
  </aside>
</section>
