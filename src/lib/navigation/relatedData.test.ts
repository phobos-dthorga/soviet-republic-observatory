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
