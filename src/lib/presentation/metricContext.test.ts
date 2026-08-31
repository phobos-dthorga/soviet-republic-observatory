import { describe, expect, it } from "vitest";
import { translate } from "../i18n/runtime";
import { reviewRepublicBrief } from "../ui-review/fixtures";
import {
  metricContextDetails,
  metricContextHelp,
  metricContextSummary,
  publishedMetricContext,
} from "./metricContext";
import { briefMetricLabel } from "./republicBrief";

describe("metric context presentation", () => {
  const brief = reviewRepublicBrief();
  const label = (metricId: string) => briefMetricLabel(metricId, translate);

  it("distinguishes the all-citizen education source from the workers panel", () => {
    const metric = brief.metrics.find(
      (candidate) =>
        candidate.metric_id === "source.stats.citizens.basic_education",
    );
    expect(metric).toBeDefined();
    const help = metricContextHelp(metric!, translate, label);

    expect(help.details).toContainEqual({
      label: "Counted population",
      value: "All recorded citizens",
    });
    expect(help.details).toContainEqual({
      label: "Important boundary",
      value: "Not the workers-only education breakdown shown in the game panel",
    });
  });

  it("names the classified receiver denominator and its exclusion", () => {
    const metric = brief.metrics.find(
      (candidate) => candidate.metric_id === "core.citizens.electronics.radio",
    );
    expect(metric).toBeDefined();
    const details = metricContextDetails(metric!.context, translate, label);

    expect(details).toContainEqual({
      label: "Denominator",
      value: "Classified receiver population",
    });
    expect(details).toContainEqual({
      label: "Important boundary",
      value: "Citizens outside the four recorded receiver classes are excluded",
    });
  });

  it("keeps scope visible without requiring the tooltip", () => {
    const adults = brief.metrics.find(
      (candidate) => candidate.metric_id === "source.stats.citizens.adults",
    );
    expect(adults).toBeDefined();
    expect(
      metricContextSummary(adults!.context, translate).replace(
        /[\u2068\u2069]/g,
        "",
      ),
    ).toBe("W&R's source-defined adult class · Whole republic");
  });

  it("uses the same published contract for exact cards and historical charts", () => {
    const catalogue = [
      {
        metric_id: "source.stats.citizens.born",
        exact: {
          population_basis: "source_defined_movement_counter" as const,
          time_basis: "exact_selected_observation" as const,
          geographic_scope: "whole_republic" as const,
          denominator_metric_id: null,
          comparison_basis: "proven_preceding_same_branch_and_profile" as const,
          limitations: [
            "source_window_unverified" as const,
            "not_interval_flow" as const,
          ],
        },
        history: {
          population_basis: "source_defined_movement_counter" as const,
          time_basis: "branch_observations_through_selected_head" as const,
          geographic_scope: "whole_republic" as const,
          denominator_metric_id: null,
          comparison_basis: "proven_preceding_same_branch_and_profile" as const,
          limitations: [
            "source_window_unverified" as const,
            "not_interval_flow" as const,
          ],
        },
      },
    ];

    const context = publishedMetricContext(
      catalogue,
      "source.stats.citizens.born",
      "history",
    );
    expect(context?.time_basis).toBe(
      "branch_observations_through_selected_head",
    );
    expect(context?.limitations).toContain("not_interval_flow");
  });
});
