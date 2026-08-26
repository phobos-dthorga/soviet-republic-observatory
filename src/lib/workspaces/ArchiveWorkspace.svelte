<script lang="ts">
  import type { TranslationKey } from "../i18n/catalog";
  import { formatDate, formatNumber } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import type {
    ArchiveObservation,
    ArchiveOverview,
    TimelineBranch,
  } from "../observations/types";

  let {
    archive,
    desktopAvailable,
    onselect,
  }: {
    archive: ArchiveOverview | null;
    desktopAvailable: boolean;
    onselect: (branchId: string) => Promise<void>;
  } = $props();

  let selecting = $state(false);
  let selectionError = $state(false);
  const selectedBranch = $derived(
    archive?.branches.find((branch) => branch.selected) ?? null,
  );

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
  };

  function branchLabel(branch: TimelineBranch): string {
    if (branch.branch_kind === "main")
      return $translation("archive-branch-main");
    if (branch.branch_kind === "unassigned")
      return $translation("archive-branch-unassigned");
    return $translation("archive-branch-fork", {
      identity: branch.branch_id.replace("fork-", "").slice(0, 6),
    });
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
            class:selected={observation.branch_id ===
              archive.selected_branch_id}
          >
            <div>
              <strong>{observation.source_file_name}</strong>
              <small
                >{formatDate(observation.imported_at_ms, $activeLocale, {
                  dateStyle: "medium",
                  timeStyle: "short",
                })}</small
              >
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
                data-status={observation.coverage_status === "complete"
                  ? "stable"
                  : "watch"}
              >
                {$translation(
                  observation.coverage_status === "complete"
                    ? "coverage-complete"
                    : "coverage-partial",
                )}
              </span>
              <code>{observation.payload_hash.slice(0, 12)}</code>
            </div>
          </article>
        {/each}
      </section>
    {/if}
  </section>

  <aside class="inspector" aria-label={$translation("archive-inspector-label")}>
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
