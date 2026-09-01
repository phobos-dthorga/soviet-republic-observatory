<script lang="ts">
  import { translation } from "../i18n/runtime";
  import NotificationToast from "./NotificationToast.svelte";
  import RecoveryDialog from "./RecoveryDialog.svelte";
  import { notifications } from "./service";
</script>

{#if $notifications.length > 0}
  <section
    class="notification-region"
    aria-label={$translation("notifications-region-label")}
  >
    {#each $notifications as notification (notification.id)}
      <NotificationToast {notification} />
    {/each}
  </section>
{/if}

<RecoveryDialog />
