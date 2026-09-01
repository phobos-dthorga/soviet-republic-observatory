import { describe, expect, it } from "vitest";
import {
  defaultWorkspaceLocation,
  destinationsForSubject,
  isWorkspaceSection,
  pushNavigationTrail,
  workspaceDestination,
  type NavigationTrailEntry,
} from "./relatedData";

describe("related data navigation", () => {
  it("resolves known metric identities without using display labels", () => {
    expect(
      destinationsForSubject({
        kind: "metric",
        metricId: "source.stats.citizens.adults",
      }).map((item) => item.location.workspace),
    ).toEqual(["population", "plan"]);
  });

  it("keeps resource currencies and channels in the destination", () => {
    const destinations = destinationsForSubject({
      kind: "resource",
      resourceToken: "alcohol",
      currency: "rub",
      channel: "standard",
    });
    expect(destinations[0]?.location.filters).toMatchObject({
      resourceToken: "alcohol",
      currency: "rub",
      channel: "standard",
    });
  });

  it("offers exact saves to every supported analytical view", () => {
    const destinations = destinationsForSubject({
      kind: "observation",
      reference: {
        interpretation_id: "saved-moment",
        branch_id: "main",
        year: 2018,
        day: 333,
      },
    });
    expect(destinations.map((item) => item.location.workspace)).toEqual([
      "briefing",
      "broadcast",
      "population",
      "markets",
      "archive",
    ]);
    expect(destinations.every((item) => item.exactObservation)).toBe(true);
  });

  it("keeps every admitted destination unique and inside its workspace", () => {
    const subjects = [
      {
        kind: "metric" as const,
        metricId: "source.stats.citizens.adults",
      },
      {
        kind: "resource" as const,
        resourceToken: "alcohol",
        currency: "rub" as const,
        channel: "standard" as const,
      },
      { kind: "city" as const, cityId: "97" },
      { kind: "catalogue_entity" as const, entityId: "resource::oil" },
      {
        kind: "plan_target" as const,
        metricId: "source.stats.citizens.adults",
        revision: 3,
      },
    ];
    for (const subject of subjects) {
      const destinations = destinationsForSubject(subject);
      expect(new Set(destinations.map((item) => item.id)).size).toBe(
        destinations.length,
      );
      expect(
        destinations.every((item) =>
          isWorkspaceSection(item.location.workspace, item.location.section),
        ),
      ).toBe(true);
    }
  });

  it("lets the host relate only a known extension metric", () => {
    expect(
      destinationsForSubject({
        kind: "extension_contribution",
        metricId: "core.citizens.electronics.radio",
      }).map((item) => item.location.workspace),
    ).toEqual(["broadcast"]);
    expect(
      destinationsForSubject({
        kind: "extension_contribution",
        metricId: "local.author-invented.metric",
      }),
    ).toEqual([]);
  });

  it("rejects sections outside the workspace allowlist", () => {
    expect(isWorkspaceSection("markets", "markets-prices")).toBe(true);
    expect(isWorkspaceSection("markets", "population-status")).toBe(false);
    expect(() => workspaceDestination("markets", "population-status")).toThrow(
      "invalid_related_destination",
    );
  });

  it("bounds history and avoids adjacent duplicate locations", () => {
    const entry = {
      location: defaultWorkspaceLocation("briefing"),
      context: null,
    };
    expect(pushNavigationTrail([entry], entry)).toHaveLength(1);
    let trail: NavigationTrailEntry[] = [entry];
    for (let index = 0; index < 25; index += 1) {
      trail = pushNavigationTrail(trail, {
        location: {
          ...defaultWorkspaceLocation("markets"),
          focusId: `resource-${index}`,
        },
        context: null,
      });
    }
    expect(trail).toHaveLength(20);
  });
});
