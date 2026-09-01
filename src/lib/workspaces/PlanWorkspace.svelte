<script lang="ts">
  import ObservatoryChart from "../charts/ObservatoryChart.svelte";
  import {
    formatNumber,
    formatPercent,
    formatSignedNumber,
  } from "../i18n/format";
  import { activeLocale, translation } from "../i18n/runtime";
  import type { TranslationKey } from "../i18n/catalog";
  import { containedSectionNavigation } from "../navigation/containedSectionNavigation";
  import { notify } from "../notifications/service";
  import {
    activateRepublicPlan,
    removeRepublicPlan,
    rollbackRepublicPlan,
    saveRepublicPlan,
  } from "../observations/desktopClient";
  import type {
    PlanDirection,
    PlanScheduleKind,
    PlanTargetDraft,
    PlanTargetEvaluation,
    PlanTargetState,
    RepublicPlanDraft,
    RepublicPlanWorkspace,
  } from "../observations/types";
  import { metricContextHelpFor } from "../presentation/metricContext";
  import { briefMetricLabel } from "../presentation/republicBrief";
  import {
    createPlanTargetChart,
    planDirectionForValues,
    planErrorTranslationKey,
  } from "../presentation/republicPlan";
  import GuidanceSurface from "../ui/GuidanceSurface.svelte";
  import MetricContextHelp from "../ui/MetricContextHelp.svelte";

  let {
    workspace = null,
    desktopAvailable,
    onupdate,
  }: {
    workspace?: RepublicPlanWorkspace | null;
    desktopAvailable: boolean;
    onupdate: (workspace: RepublicPlanWorkspace) => void;
  } = $props();

  type EditableTarget = PlanTargetDraft & { guardrail_percent: number };

  const planStateKeys = {
    awaiting_start: "plan-state-awaiting-start",
    ahead: "plan-state-ahead",
    on_track: "plan-state-on-track",
    behind: "plan-state-behind",
    complete: "plan-state-complete",
    unavailable: "plan-state-unavailable",
  } as const satisfies Record<PlanTargetState, TranslationKey>;

  const scheduleKeys = {
    linear: "plan-schedule-linear",
    milestone: "plan-schedule-milestone",
    hold_then_change: "plan-schedule-hold-then-change",
  } as const satisfies Record<PlanScheduleKind, TranslationKey>;

  const directionKeys = {
    increase: "plan-direction-increase",
    decrease: "plan-direction-decrease",
    maintain: "plan-direction-maintain",
  } as const satisfies Record<PlanDirection, TranslationKey>;

  let busy = $state(false);
  let editingPlanId = $state<string | null>(null);
  let name = $state("");
  let endYear = $state(0);
  let endDay = $state(0);
  let schedule = $state<PlanScheduleKind>("linear");
  let targets = $state<EditableTarget[]>([]);
  let selectedMetricId = $state("");

  const activePlan = $derived(workspace?.active_plan ?? null);
  const selectedTarget = $derived(
    activePlan?.targets.find(
      (target) => target.target.metric_id === selectedMetricId,
    ) ??
      activePlan?.targets[0] ??
      null,
  );
  const targetChart = $derived(
    selectedTarget && activePlan
      ? createPlanTargetChart(
          selectedTarget,
          activePlan.revision.name,
          $translation,
        )
      : null,
  );
  const targetHelp = $derived(
    selectedTarget
      ? metricContextHelpFor(
          selectedTarget.target.metric_id,
          selectedTarget.context,
          $translation,
          metricLabel,
        )
      : null,
  );

  $effect(() => {
    const firstMetric = activePlan?.targets[0]?.target.metric_id ?? "";
    if (
      !activePlan?.targets.some(
        (target) => target.target.metric_id === selectedMetricId,
      )
    ) {
      selectedMetricId = firstMetric;
    }
  });

  $effect(() => {
    if (!editingPlanId && !name) resetDraft();
  });

  function metricLabel(metricId: string): string {
    return briefMetricLabel(metricId, $translation);
  }

  function editorMetricContext(metricId: string) {
    return workspace?.available_metrics.find(
      (metric) => metric.metric_id === metricId,
    )?.context;
  }

  function currentValue(metricId: string): number | null {
    return (
      workspace?.available_metrics.find(
        (metric) => metric.metric_id === metricId,
      )?.current_value ?? null
    );
  }

  function baselineValue(metricId: string): number | null {
    if (editingPlanId && activePlan?.revision.plan_id === editingPlanId) {
      return (
        workspace?.available_metrics.find(
          (metric) => metric.metric_id === metricId,
        )?.active_plan_baseline_value ?? null
      );
    }
    return currentValue(metricId);
  }

  function defaultTarget(): EditableTarget | null {
    const used = new Set(targets.map((target) => target.metric_id));
    const option = workspace?.available_metrics.find(
      (metric) => metric.current_value !== null && !used.has(metric.metric_id),
    );
    if (!option || option.current_value === null) return null;
    const baseline = baselineValue(option.metric_id);
    if (baseline === null) return null;
    return {
      metric_id: option.metric_id,
      target_value: baseline,
      direction: "maintain",
      guardrail_basis_points: 500,
      guardrail_percent: 5,
    };
  }

  function resetDraft(): void {
    const current = workspace?.available_metrics.find(
      (metric) => metric.current_value !== null,
    );
    const year = workspace?.current_year ?? 2000;
    editingPlanId = null;
    name = $translation("plan-default-name");
    endYear = year + 5;
    endDay = workspace?.current_day ?? 0;
    schedule = "linear";
    targets =
      current?.current_value == null
        ? []
        : [
            {
              metric_id: current.metric_id,
              target_value: current.current_value,
              direction: "maintain",
              guardrail_basis_points: 500,
              guardrail_percent: 5,
            },
          ];
  }

  function editActivePlan(): void {
    if (!activePlan) return;
    editingPlanId = activePlan.revision.plan_id;
    name = activePlan.revision.name;
    endYear = activePlan.revision.end_year;
    endDay = activePlan.revision.end_day;
    schedule = activePlan.revision.schedule;
    targets = activePlan.revision.targets.map((target) => ({
      metric_id: target.metric_id,
      target_value: target.target_value,
      direction: target.direction,
      guardrail_basis_points: target.guardrail_basis_points,
      guardrail_percent: target.guardrail_basis_points / 100,
    }));
  }

  function addTarget(): void {
    const target = defaultTarget();
    if (target) targets = [...targets, target];
  }

  function removeTarget(index: number): void {
    targets = targets.filter((_, targetIndex) => targetIndex !== index);
  }

  function updateTarget<K extends keyof EditableTarget>(
    index: number,
    key: K,
    value: EditableTarget[K],
  ): void {
    targets = targets.map((target, targetIndex) =>
      targetIndex === index ? { ...target, [key]: value } : target,
    );
  }

  function updateDirectionForValue(index: number, value: number): void {
    const baseline = baselineValue(targets[index].metric_id);
    const direction =
      planDirectionForValues(baseline, value) ?? targets[index].direction;
    targets = targets.map((target, targetIndex) =>
      targetIndex === index
        ? { ...target, target_value: value, direction }
        : target,
    );
  }

  function changeMetric(index: number, metricId: string): void {
    const baseline = baselineValue(metricId) ?? 0;
    targets = targets.map((target, targetIndex) =>
      targetIndex === index
        ? {
            ...target,
            metric_id: metricId,
            target_value: baseline,
            direction: "maintain",
          }
        : target,
    );
  }

  async function submitPlan(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy) return;
    busy = true;
    const draft: RepublicPlanDraft = {
      plan_id: editingPlanId,
      name,
      end_year: endYear,
      end_day: endDay,
      schedule,
      targets: targets.map((target) => ({
        metric_id: target.metric_id,
        target_value: target.target_value,
        direction:
          planDirectionForValues(
            baselineValue(target.metric_id),
            target.target_value,
          ) ?? target.direction,
        guardrail_basis_points: Math.round(target.guardrail_percent * 100),
      })),
    };
    try {
      onupdate(await saveRepublicPlan(draft));
      notify({
        title: $translation("plan-notification-title"),
        message: $translation(
          editingPlanId
            ? "plan-notification-revised"
            : "plan-notification-created",
        ),
        tone: "success",
      });
      editingPlanId = null;
      name = "";
    } catch (error) {
      notify({
        title: $translation("plan-notification-title"),
        message: $translation(planErrorTranslationKey(error)),
        tone: "error",
      });
    } finally {
      busy = false;
    }
  }

  async function activate(planId: string): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      onupdate(await activateRepublicPlan(planId));
    } catch (error) {
      notifyPlanError(error);
    } finally {
      busy = false;
    }
  }

  async function rollback(): Promise<void> {
    if (!activePlan || busy) return;
    busy = true;
    try {
      onupdate(await rollbackRepublicPlan(activePlan.revision.plan_id));
    } catch (error) {
      notifyPlanError(error);
    } finally {
      busy = false;
    }
  }

  async function remove(): Promise<void> {
    if (
      !activePlan ||
      busy ||
      !window.confirm($translation("plan-remove-confirm"))
    )
      return;
    busy = true;
    try {
      onupdate(await removeRepublicPlan(activePlan.revision.plan_id));
      editingPlanId = null;
      name = "";
    } catch (error) {
      notifyPlanError(error);
    } finally {
      busy = false;
    }
  }

  function stateLabel(state: PlanTargetState): string {
    return $translation(planStateKeys[state]);
  }

  function scheduleLabel(value: PlanScheduleKind): string {
    return $translation(scheduleKeys[value]);
  }

  function notifyPlanError(error: unknown): void {
    notify({
      title: $translation("plan-notification-title"),
      message: $translation(planErrorTranslationKey(error)),
      tone: "error",
    });
  }

  function directionLabel(direction: PlanDirection): string {
    return $translation(directionKeys[direction]);
  }

  function reading(value: number | null): string {
    return value == null
      ? $translation("chart-unavailable")
      : formatNumber(value, $activeLocale);
  }

  function attainment(value: number | null): string {
    return value == null
      ? $translation("chart-unavailable")
      : formatPercent(value / 100, $activeLocale);
  }

  function targetVariance(target: PlanTargetEvaluation): string {
    return target.directional_variance == null
      ? $translation("chart-unavailable")
      : formatSignedNumber(target.directional_variance, $activeLocale);
  }
</script>

<section class="workspace plan-workspace">
  <aside class="navigator" aria-label={$translation("plan-navigation-label")}>
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("plan-directorate")}</span>
        <h2>{$translation("nav-plan")}</h2>
      </div>
      <span class="edition">v1</span>
    </div>
    <div class="lens-card">
      <div class="lens-row">
        <span>{$translation("filter-branch")}</span>
        <strong>{workspace?.analysis_context.selected_branch_id ?? "—"}</strong>
      </div>
      <div class="lens-row">
        <span>{$translation("plan-active-revision")}</span>
        <strong>{activePlan?.revision.revision ?? "—"}</strong>
      </div>
      <div class="lens-row">
        <span>{$translation("plan-target-count")}</span>
        <strong>{activePlan?.targets.length ?? 0}</strong>
      </div>
    </div>
    <div class="section-list">
      <a href="#plan-status" use:containedSectionNavigation
        ><span>01</span>{$translation("plan-section-status")}</a
      >
      <a href="#plan-trajectory" use:containedSectionNavigation
        ><span>02</span>{$translation("plan-section-trajectory")}</a
      >
      <a href="#plan-editor" use:containedSectionNavigation
        ><span>03</span>{$translation("plan-section-editor")}</a
      >
      <a href="#plan-revisions" use:containedSectionNavigation
        ><span>04</span>{$translation("plan-section-revisions")}</a
      >
    </div>
    <GuidanceSurface kind="boundary" layout="compact" class="sidebar-note">
      <span aria-hidden="true">◇</span>
      <p>{$translation("plan-sidebar-boundary")}</p>
    </GuidanceSurface>
  </aside>

  <section class="canvas">
    <GuidanceSurface
      kind="instruction"
      layout="inline"
      semanticRole="status"
      class="preview-banner"
    >
      <strong>{$translation("plan-evidence-banner")}</strong>
      <span>{$translation("plan-evidence-banner-detail")}</span>
    </GuidanceSurface>
    <header class="page-heading">
      <div>
        <span class="eyebrow">{$translation("plan-heading-eyebrow")}</span>
        <h2>{$translation("plan-heading-title")}</h2>
        <p>{$translation("plan-heading-description")}</p>
      </div>
      <div class="date-stamp">
        <span>{$translation("plan-active-plan")}</span>
        <strong>{activePlan?.revision.name ?? "—"}</strong>
        <small
          >{activePlan
            ? stateLabel(activePlan.state)
            : $translation("plan-none-active")}</small
        >
      </div>
    </header>

    {#if !desktopAvailable}
      <section class="archive-empty-state">
        <span class="eyebrow">{$translation("archive-desktop-required")}</span>
        <h3>{$translation("plan-desktop-required")}</h3>
        <p>{$translation("plan-desktop-required-detail")}</p>
      </section>
    {:else if !workspace?.analysis_context.head_interpretation_id}
      <section class="archive-empty-state">
        <span class="eyebrow">{$translation("plan-observation-required")}</span>
        <h3>{$translation("plan-no-observation-title")}</h3>
        <p>{$translation("plan-no-observation-detail")}</p>
      </section>
    {:else}
      <section
        id="plan-status"
        class="plan-summary-grid"
        aria-label={$translation("plan-section-status")}
      >
        <article class="kpi-card">
          <header>
            <span>{$translation("plan-overall-attainment")}</span><span
              class="coverage">{$translation("evidence-calculation")}</span
            >
          </header>
          <strong
            >{attainment(activePlan?.attainment_basis_points ?? null)}</strong
          >
          <p>
            {activePlan
              ? stateLabel(activePlan.state)
              : $translation("plan-none-active")}
          </p>
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("plan-deadline")}</span><span class="coverage"
              >{$translation("evidence-player-definition")}</span
            >
          </header>
          <strong
            >{activePlan
              ? `${activePlan.revision.end_year} · ${String(activePlan.revision.end_day).padStart(3, "0")}`
              : "—"}</strong
          >
          <p>
            {activePlan
              ? scheduleLabel(activePlan.revision.schedule)
              : $translation("plan-none-active")}
          </p>
        </article>
        <article class="kpi-card">
          <header>
            <span>{$translation("plan-guardrail-breaches")}</span><span
              class="coverage">{$translation("evidence-calculation")}</span
            >
          </header>
          <strong>{activePlan?.guardrail_breach_count ?? 0}</strong>
          <p>{$translation("plan-guardrail-detail")}</p>
        </article>
      </section>

      {#if activePlan && selectedTarget && targetChart}
        <section id="plan-trajectory" class="plan-trajectory">
          <div
            class="plan-target-tabs"
            role="tablist"
            aria-label={$translation("plan-select-target")}
          >
            {#each activePlan.targets as target}
              <button
                type="button"
                role="tab"
                aria-selected={selectedTarget.target.metric_id ===
                  target.target.metric_id}
                class:active={selectedTarget.target.metric_id ===
                  target.target.metric_id}
                onclick={() => (selectedMetricId = target.target.metric_id)}
                >{metricLabel(target.target.metric_id)}</button
              >
            {/each}
          </div>
          <div class="plan-target-reading">
            <article>
              <span>{$translation("plan-observed-value")}</span>
              <strong>{reading(selectedTarget.current_value)}</strong>
            </article>
            <article>
              <span>{$translation("plan-scheduled-value")}</span>
              <strong>{reading(selectedTarget.scheduled_value)}</strong>
            </article>
            <article data-state={selectedTarget.state}>
              <span>{$translation("plan-directional-variance")}</span>
              <strong>{targetVariance(selectedTarget)}</strong>
            </article>
            <article>
              <span>{$translation("plan-target-attainment")}</span>
              <strong
                >{attainment(selectedTarget.attainment_basis_points)}</strong
              >
            </article>
          </div>
          <ObservatoryChart
            spec={targetChart}
            height="310px"
            eyebrow={$translation("plan-section-trajectory")}
            help={targetHelp}
          />
          <GuidanceSurface kind="boundary" layout="compact">
            <strong>{$translation("plan-interpretation-boundary")}</strong>
            <span>{$translation("plan-interpretation-boundary-detail")}</span>
          </GuidanceSurface>
        </section>
      {:else}
        <GuidanceSurface kind="help" layout="block">
          <strong>{$translation("plan-empty-title")}</strong>
          <span>{$translation("plan-empty-detail")}</span>
        </GuidanceSurface>
      {/if}

      <section id="plan-editor" class="plan-editor-panel">
        <header class="panel-heading">
          <div>
            <span class="eyebrow">{$translation("plan-player-intent")}</span>
            <h2>
              {editingPlanId
                ? $translation("plan-revise-title")
                : $translation("plan-create-title")}
            </h2>
            <p>{$translation("plan-editor-description")}</p>
          </div>
          {#if activePlan}
            <div class="panel-actions">
              <button type="button" onclick={editActivePlan} disabled={busy}
                >{$translation("plan-revise")}</button
              >
              <button type="button" onclick={resetDraft} disabled={busy}
                >{$translation("plan-new")}</button
              >
            </div>
          {/if}
        </header>
        <form onsubmit={submitPlan}>
          <div class="plan-form-grid">
            <label>
              <span>{$translation("plan-name")}</span>
              <input bind:value={name} maxlength="120" required />
            </label>
            <label>
              <span>{$translation("plan-end-year")}</span>
              <input
                type="number"
                bind:value={endYear}
                min="0"
                max="10000"
                required
              />
            </label>
            <label>
              <span>{$translation("plan-end-day")}</span>
              <input
                type="number"
                bind:value={endDay}
                min="0"
                max="364"
                required
              />
            </label>
            <label>
              <span>{$translation("plan-schedule")}</span>
              <select bind:value={schedule}>
                <option value="linear"
                  >{$translation("plan-schedule-linear")}</option
                >
                <option value="milestone"
                  >{$translation("plan-schedule-milestone")}</option
                >
                <option value="hold_then_change"
                  >{$translation("plan-schedule-hold-then-change")}</option
                >
              </select>
            </label>
          </div>
          <div class="plan-target-editor">
            <header>
              <div>
                <span class="eyebrow">{$translation("plan-targets")}</span>
                <h3>{$translation("plan-targets-heading")}</h3>
              </div>
              <button
                type="button"
                onclick={addTarget}
                disabled={busy || targets.length >= 12}
                >{$translation("plan-add-target")}</button
              >
            </header>
            {#each targets as target, index}
              {@const editorContext = editorMetricContext(target.metric_id)}
              <div class="plan-target-row">
                <label>
                  <span>{$translation("plan-metric")}</span>
                  <select
                    value={target.metric_id}
                    onchange={(event) =>
                      changeMetric(index, event.currentTarget.value)}
                  >
                    {#each workspace.available_metrics as metric}
                      <option
                        value={metric.metric_id}
                        disabled={targets.some(
                          (row, rowIndex) =>
                            rowIndex !== index &&
                            row.metric_id === metric.metric_id,
                        )}>{metricLabel(metric.metric_id)}</option
                      >
                    {/each}
                  </select>
                </label>
                <label>
                  <span>{$translation("plan-baseline")}</span>
                  <output>{reading(baselineValue(target.metric_id))}</output>
                </label>
                <label>
                  <span>{$translation("plan-target-value")}</span>
                  <input
                    type="number"
                    min="0"
                    value={target.target_value}
                    oninput={(event) =>
                      updateDirectionForValue(
                        index,
                        event.currentTarget.valueAsNumber,
                      )}
                    required
                  />
                </label>
                <label>
                  <span>{$translation("plan-direction")}</span>
                  <output
                    class="calculated-direction"
                    aria-describedby={`plan-direction-help-${index}`}
                    >{directionLabel(target.direction)}</output
                  >
                  <small id={`plan-direction-help-${index}`}
                    >{$translation("plan-direction-derived")}</small
                  >
                </label>
                <label>
                  <span>{$translation("plan-guardrail")}</span>
                  <input
                    type="number"
                    min="0"
                    max="50"
                    step="0.1"
                    value={target.guardrail_percent}
                    oninput={(event) =>
                      updateTarget(
                        index,
                        "guardrail_percent",
                        event.currentTarget.valueAsNumber,
                      )}
                    required
                  />
                </label>
                <button
                  type="button"
                  class="remove-target"
                  onclick={() => removeTarget(index)}
                  disabled={busy || targets.length === 1}
                  aria-label={$translation("plan-remove-target", {
                    metric: metricLabel(target.metric_id),
                  })}>×</button
                >
                <span class="target-context-help">
                  {#if editorContext}
                    <MetricContextHelp
                      metricId={target.metric_id}
                      context={editorContext}
                      {metricLabel}
                      placement="left"
                    />
                  {/if}
                </span>
              </div>
            {/each}
          </div>
          <div class="form-actions">
            <button
              type="submit"
              class="primary"
              disabled={busy || !targets.length}
              >{busy
                ? $translation("plan-saving")
                : editingPlanId
                  ? $translation("plan-save-revision")
                  : $translation("plan-create")}</button
            >
          </div>
        </form>
      </section>

      <section id="plan-revisions" class="plan-revision-panel">
        <header class="panel-heading">
          <div>
            <span class="eyebrow"
              >{$translation("plan-immutable-revisions")}</span
            >
            <h2>{$translation("plan-saved-plans")}</h2>
            <p>{$translation("plan-revisions-description")}</p>
          </div>
          {#if activePlan}
            <div class="panel-actions">
              <button
                type="button"
                onclick={rollback}
                disabled={busy || activePlan.revision.revision <= 1}
                >{$translation("plan-rollback")}</button
              >
              <button type="button" onclick={remove} disabled={busy}
                >{$translation("plan-remove")}</button
              >
            </div>
          {/if}
        </header>
        <div class="plan-list">
          {#each workspace.plans as plan}
            <article class:active={plan.selected}>
              <div>
                <strong>{plan.name}</strong>
                <span
                  >{$translation("plan-revision-summary", {
                    active: plan.active_revision,
                    latest: plan.latest_revision,
                    count: plan.revision_count,
                  })}</span
                >
              </div>
              {#if plan.selected}
                <span class="status-chip" data-status="stable"
                  >{$translation("plan-selected")}</span
                >
              {:else}
                <button
                  type="button"
                  onclick={() => void activate(plan.plan_id)}
                  disabled={busy}>{$translation("plan-select")}</button
                >
              {/if}
            </article>
          {:else}
            <GuidanceSurface kind="help" layout="compact">
              <strong>{$translation("plan-no-saved-title")}</strong>
              <span>{$translation("plan-no-saved-detail")}</span>
            </GuidanceSurface>
          {/each}
        </div>
      </section>
    {/if}
  </section>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (keyboard-focusable scroll region) -->
  <aside
    class="inspector"
    role="region"
    tabindex="0"
    aria-label={$translation("plan-inspector-label")}
  >
    <div class="aside-heading">
      <div>
        <span class="eyebrow">{$translation("plan-evidence-inspector")}</span>
        <h2>
          {selectedTarget
            ? metricLabel(selectedTarget.target.metric_id)
            : $translation("plan-snapshot-ledger")}
        </h2>
      </div>
      <span
        class="status-chip"
        data-status={selectedTarget?.guardrail_breached ? "watch" : "stable"}
        >{selectedTarget
          ? stateLabel(selectedTarget.state)
          : $translation("plan-none-active")}</span
      >
    </div>
    {#if selectedTarget && activePlan}
      <div class="selected-reading">
        <span>{$translation("plan-target-attainment")}</span>
        <strong>{attainment(selectedTarget.attainment_basis_points)}</strong>
        <small
          >{$translation("plan-plan-revision", {
            revision: activePlan.revision.revision,
          })}</small
        >
        <p>{$translation("plan-inspector-detail")}</p>
      </div>
      <section class="metric-context-ledger">
        <span class="eyebrow">{$translation("metric-context-ledger")}</span>
        <dl>
          {#each targetHelp?.details ?? [] as detail}
            <div>
              <dt>{detail.label}</dt>
              <dd>{detail.value}</dd>
            </div>
          {/each}
        </dl>
      </section>
      <section class="evidence-ledger">
        <span class="eyebrow">{$translation("evidence-ledger")}</span>
        <div>
          <strong>{$translation("plan-observed-value")}</strong><span
            >{$translation("evidence-save-fact")}</span
          >
        </div>
        <div>
          <strong>{$translation("plan-scheduled-value")}</strong><span
            >{$translation("evidence-player-definition")}</span
          >
        </div>
        <div>
          <strong>{$translation("plan-directional-variance")}</strong><span
            >{$translation("evidence-calculation")}</span
          >
        </div>
      </section>
    {:else}
      <GuidanceSurface kind="help" layout="compact">
        <strong>{$translation("plan-inspector-empty-title")}</strong>
        <span>{$translation("plan-inspector-empty-detail")}</span>
      </GuidanceSurface>
    {/if}
  </aside>
</section>

<style>
  .plan-summary-grid,
  .plan-target-reading {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .plan-trajectory,
  .plan-editor-panel,
  .plan-revision-panel {
    margin-top: 10px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-raised);
    padding: 16px;
    scroll-margin-top: 18px;
  }

  .plan-target-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 10px;
  }

  .plan-target-tabs button,
  .panel-actions button,
  .plan-target-editor > header button,
  .remove-target,
  .form-actions button,
  .plan-list button {
    min-height: 38px;
    border: 1px solid var(--colour-line);
    padding: 8px 12px;
    color: var(--colour-text);
    background: var(--colour-surface);
    cursor: pointer;
  }

  .plan-target-tabs button:disabled,
  .panel-actions button:disabled,
  .plan-target-editor > header button:disabled,
  .remove-target:disabled,
  .form-actions button:disabled,
  .plan-list button:disabled {
    cursor: not-allowed;
  }

  .plan-target-tabs button.active {
    border-color: var(--colour-observed);
    color: var(--colour-observed);
    background: var(--colour-surface);
  }

  .plan-target-reading {
    grid-template-columns: repeat(4, minmax(0, 1fr));
    margin-bottom: 8px;
  }

  .plan-target-reading article {
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface-soft);
    padding: 11px;
  }

  .plan-target-reading span,
  .plan-target-reading strong {
    display: block;
  }

  .plan-target-reading span {
    color: var(--colour-muted);
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .plan-target-reading strong {
    margin-top: 5px;
    font-size: 21px;
  }

  .plan-form-grid,
  .plan-target-row {
    display: grid;
    gap: 8px;
  }

  .plan-form-grid {
    grid-template-columns: 2fr repeat(3, minmax(120px, 1fr));
  }

  label {
    display: grid;
    gap: 5px;
    color: var(--colour-muted);
    font-size: 12px;
  }

  input,
  select,
  output {
    min-height: 38px;
  }

  output {
    display: flex;
    align-items: center;
    border: 1px solid var(--colour-line-faint);
    padding: 0 10px;
    color: var(--colour-text);
  }

  .calculated-direction {
    border-color: var(--colour-guidance);
    background: var(--colour-guidance-soft);
  }

  .plan-target-row small {
    color: var(--colour-muted);
    font-size: 11px;
    line-height: 1.4;
  }

  .plan-target-editor {
    margin-top: 14px;
  }

  .plan-target-editor > header {
    display: flex;
    align-items: end;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .plan-target-editor h3 {
    margin: 3px 0 0;
  }

  .plan-target-row {
    position: relative;
    grid-template-columns:
      minmax(180px, 2fr) repeat(4, minmax(105px, 1fr))
      40px;
    align-items: end;
    border-top: 1px solid var(--colour-line-faint);
    padding: 10px 36px 10px 0;
  }

  .target-context-help {
    position: absolute;
    top: 13px;
    right: 0;
  }

  .remove-target {
    min-height: 38px;
    color: var(--colour-risk);
  }

  .form-actions {
    display: flex;
    justify-content: end;
    margin-top: 12px;
  }

  .form-actions .primary {
    border-color: var(--colour-gold);
    color: var(--colour-gold);
    background: var(--colour-surface);
  }

  .plan-list {
    display: grid;
    gap: 7px;
  }

  .plan-list article {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border: 1px solid var(--colour-line-faint);
    padding: 11px;
  }

  .plan-list article.active {
    border-inline-start: 3px solid var(--colour-gold);
  }

  .plan-list article div,
  .plan-list article span {
    display: grid;
    gap: 4px;
  }

  .plan-list article div > span {
    color: var(--colour-muted);
    font-size: 12px;
  }

  @media (max-width: 1200px) {
    .plan-form-grid,
    .plan-target-row {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 760px) {
    .plan-summary-grid,
    .plan-target-reading,
    .plan-form-grid,
    .plan-target-row {
      grid-template-columns: 1fr;
    }
  }
</style>
