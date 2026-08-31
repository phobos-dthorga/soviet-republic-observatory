<script lang="ts">
  import { translation } from "../i18n/runtime";
  import type { MetricContext } from "../observations/types";
  import {
    metricContextHelpFor,
    type MetricLabelResolver,
  } from "../presentation/metricContext";
  import ContextHelp from "./ContextHelp.svelte";

  let {
    metricId,
    context,
    metricLabel,
    placement = "left",
  }: {
    metricId: string;
    context: MetricContext;
    metricLabel: MetricLabelResolver;
    placement?: "left" | "right" | "below";
  } = $props();

  const help = $derived(
    metricContextHelpFor(metricId, context, $translation, metricLabel),
  );
</script>

<ContextHelp {...help} {placement} />
