<script lang="ts">
  import { onMount } from "svelte";
  import type { TranslationKey } from "../i18n/catalog";
  import { translation } from "../i18n/runtime";
  import TechnicalDetails from "../ui/TechnicalDetails.svelte";
  import {
    dismissNotification,
    openRecoveryProposal,
    type AppNotification,
    type NotificationTone,
  } from "./service";

  let { notification }: { notification: AppNotification } = $props();

  const toneLabels: Record<NotificationTone, TranslationKey> = {
    info: "notification-info-label",
    success: "notification-success-label",
    warning: "notification-warning-label",
    error: "notification-error-label",
  };
  const toneGlyphs: Record<NotificationTone, string> = {
    info: "i",
    success: "✓",
    warning: "!",
    error: "×",
  };

  onMount(() => {
    if (notification.timeoutMs <= 0) return;
    const timer = window.setTimeout(
      () => dismissNotification(notification.id),
      notification.timeoutMs,
    );
    return () => window.clearTimeout(timer);
  });
</script>

<article
  class="notification-toast"
  data-tone={notification.tone}
  role={notification.tone === "error" || notification.tone === "warning"
    ? "alert"
    : "status"}
  aria-atomic="true"
>
  <span class="notification-glyph" aria-hidden="true"
    >{toneGlyphs[notification.tone]}</span
  >
  <div class="notification-copy">
    <strong
      >{notification.title ??
        $translation(toneLabels[notification.tone])}</strong
    >
    <p>{notification.message}</p>
    {#if notification.technicalDetails}
      <TechnicalDetails {...notification.technicalDetails} />
    {/if}
    {#if notification.recovery}
      <button
        class="notification-recovery"
        type="button"
        onclick={() => {
          if (!notification.recovery) return;
          openRecoveryProposal(notification.recovery);
          dismissNotification(notification.id);
        }}>{$translation("notification-review-recovery")}</button
      >
    {/if}
  </div>
  <button
    class="notification-dismiss"
    type="button"
    aria-label={$translation("notification-dismiss")}
    title={$translation("notification-dismiss")}
    onclick={() => dismissNotification(notification.id)}>×</button
  >
</article>
