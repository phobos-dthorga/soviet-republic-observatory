<script lang="ts">
  import type { TaskProgressView } from "./progress";

  let { view, headingId } = $props<{
    view: TaskProgressView;
    headingId: string;
  }>();
</script>

<section
  class="task-progress"
  class:active={view.state === "running"}
  class:paused={view.state === "paused"}
  class:failed={view.state === "failed"}
  class:warning={view.notice?.tone === "warning"}
  aria-live="polite"
  aria-labelledby={headingId}
  data-task-id={view.taskId}
  data-run-id={view.runId}
>
  <header>
    <div>
      <span class="eyebrow">{view.eyebrow}</span>
      <h3 id={headingId}>{view.heading}</h3>
    </div>
    <strong
      >{view.progressPercent == null ? "—" : `${view.progressPercent}%`}</strong
    >
  </header>

  <progress
    max="100"
    value={view.progressPercent ?? undefined}
    aria-label={view.heading}
  ></progress>

  <ol class="task-stages">
    {#each view.stages as stage}
      <li
        class:active={stage.state === "active"}
        class:complete={stage.state === "complete"}
        class:failed={stage.state === "failed"}
      >
        <span class="stage-marker" aria-hidden="true"></span>
        <span>{stage.label}<small>{stage.stateLabel}</small></span>
      </li>
    {/each}
  </ol>

  {#if view.meters.length > 0}
    <div class="task-meters">
      {#each view.meters as meter}
        <div>
          <span
            >{meter.label}<strong>{meter.completed} / {meter.total}</strong
            ></span
          >
          <progress
            max={Math.max(1, meter.total)}
            value={Math.min(meter.completed, meter.total)}
            aria-label={meter.label}
          ></progress>
        </div>
      {/each}
    </div>
  {/if}

  <div class="task-ledger">
    {#each view.metrics as metric}
      <span>{metric.label}<strong>{metric.value}</strong></span>
    {/each}
  </div>

  {#if view.currentItemLabel && view.currentItem}
    <div class="current-item">
      <span>{view.currentItemLabel}</span>
      <strong title={view.currentItem}>{view.currentItem}</strong>
      {#if view.currentItemContext}<small>{view.currentItemContext}</small>{/if}
    </div>
  {/if}

  {#if view.notice}
    <p
      class:warning={view.notice.tone === "warning"}
      class:error={view.notice.tone === "error"}
      role="alert"
    >
      {view.notice.text}
    </p>
  {/if}
</section>

<style>
  .task-progress {
    margin-bottom: 10px;
    padding: 13px;
    border: 1px solid var(--colour-line-faint);
    background: var(--colour-surface);
  }
  .task-progress.active {
    border-color: rgba(128, 198, 216, 0.5);
  }
  .task-progress.warning,
  .task-progress.paused,
  .task-progress.failed {
    border-color: var(--colour-risk);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h3 {
    margin-top: 4px;
    font-size: 18px;
  }
  header > strong {
    color: var(--colour-observed);
    font-family: Georgia, serif;
    font-size: 22px;
  }
  .task-progress > progress {
    width: 100%;
    height: 9px;
    margin: 11px 0;
    accent-color: var(--colour-observed);
  }
  .task-stages {
    display: grid;
    grid-template-columns: repeat(4, minmax(100px, 1fr));
    gap: 5px;
    margin: 0 0 10px;
    padding: 0;
    list-style: none;
  }
  .task-stages li {
    display: flex;
    align-items: center;
    gap: 7px;
    border-top: 1px solid var(--colour-line-faint);
    padding-top: 7px;
    color: var(--colour-muted);
    font-size: var(--type-caption);
  }
  .task-stages li.active {
    border-color: var(--colour-observed);
    color: var(--colour-text);
  }
  .task-stages li.complete {
    border-color: var(--colour-gold);
  }
  .task-stages li.failed {
    border-color: var(--colour-risk);
    color: var(--colour-risk);
  }
  .stage-marker {
    width: 7px;
    height: 7px;
    flex: 0 0 7px;
    border: 1px solid currentColor;
    transform: rotate(45deg);
  }
  li.active .stage-marker {
    background: var(--colour-observed);
  }
  li.complete .stage-marker {
    background: var(--colour-gold);
  }
  li.failed .stage-marker {
    background: var(--colour-risk);
  }
  .task-stages small {
    display: block;
    margin-top: 2px;
    font-size: var(--type-caption);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .task-meters {
    display: grid;
    grid-template-columns: repeat(2, minmax(140px, 1fr));
    gap: 7px;
    margin-bottom: 7px;
  }
  .task-meters > div {
    padding: 7px;
    background: var(--colour-surface-raised);
  }
  .task-meters span {
    display: flex;
    justify-content: space-between;
    color: var(--colour-muted);
    font-size: var(--type-caption);
    text-transform: uppercase;
  }
  .task-meters strong {
    color: var(--colour-text);
  }
  .task-meters progress {
    width: 100%;
    height: 5px;
    margin-top: 6px;
    accent-color: var(--colour-observed);
  }
  .task-ledger {
    display: grid;
    grid-template-columns: repeat(4, minmax(100px, 1fr));
    gap: 6px;
  }
  .task-ledger span {
    padding: 7px;
    color: var(--colour-muted);
    background: var(--colour-surface-raised);
    font-size: var(--type-caption);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .task-ledger strong {
    display: block;
    margin-top: 3px;
    color: var(--colour-text);
    font-size: var(--type-caption);
  }
  .current-item {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr) max-content;
    align-items: baseline;
    gap: 8px;
    margin-top: 9px;
    padding: 8px;
    color: var(--colour-muted);
    background: var(--colour-surface-raised);
    font-size: var(--type-caption);
  }
  .current-item strong {
    overflow: hidden;
    color: var(--colour-text);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .current-item small {
    color: var(--colour-observed);
  }
  p {
    margin-top: 9px;
    padding: 8px;
    font-size: var(--type-caption);
  }
  p.warning,
  p.error {
    color: var(--colour-risk);
    background: var(--colour-risk-soft);
  }
  @media (max-width: 900px) {
    .task-stages,
    .task-ledger {
      grid-template-columns: repeat(2, minmax(100px, 1fr));
    }
    .task-meters {
      grid-template-columns: 1fr;
    }
    .current-item {
      grid-template-columns: 1fr;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .stage-marker {
      transform: none;
    }
  }
</style>
