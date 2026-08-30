<script lang="ts">
  let {
    label,
    detail,
    percent,
    failed = false,
    currentItem = null,
    onclick,
  } = $props<{
    label: string;
    detail: string;
    percent: number | null;
    failed?: boolean;
    currentItem?: string | null;
    onclick: () => void;
  }>();
</script>

<button
  type="button"
  class="task-indicator"
  class:failed
  class:indeterminate={percent == null && !failed}
  title={currentItem ?? label}
  {onclick}
>
  <span>{label}</span>
  <strong>{detail}</strong>
  <i aria-hidden="true"
    ><b style:width={percent == null ? "35%" : `${percent}%`}></b></i
  >
</button>

<style>
  .task-indicator {
    min-width: 122px;
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 4px 7px;
    border: 1px solid rgba(128, 198, 216, 0.32);
    padding: 6px 8px;
    color: var(--colour-muted);
    background: var(--colour-observed-soft);
    cursor: pointer;
    font-size: 8px;
    letter-spacing: 0.06em;
    text-align: start;
    text-transform: uppercase;
  }
  strong {
    color: var(--colour-observed);
  }
  i {
    height: 2px;
    grid-column: 1 / -1;
    overflow: hidden;
    background: var(--colour-line-faint);
  }
  b {
    display: block;
    height: 100%;
    background: var(--colour-observed);
  }
  .failed {
    border-color: rgba(216, 132, 116, 0.5);
    background: var(--colour-risk-soft);
  }
  .failed strong,
  .failed b {
    color: var(--colour-risk);
    background: var(--colour-risk);
  }
  @media (prefers-reduced-motion: no-preference) {
    .task-indicator:not(.failed) i b {
      transition: width 120ms linear;
    }
    .task-indicator.indeterminate i b {
      animation: task-indicator-scan 1.4s ease-in-out infinite alternate;
    }
  }
  @keyframes task-indicator-scan {
    from {
      transform: translateX(-20%);
    }
    to {
      transform: translateX(210%);
    }
  }
</style>
