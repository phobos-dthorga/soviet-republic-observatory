import { describe, expect, it } from "vitest";
import {
  popWorkspaceTaskRoute,
  pushWorkspaceTaskRoute,
  taskBelongsToWorkspace,
  topWorkspaceTaskRoute,
  type WorkspaceTaskRoute,
} from "./workspaceTaskRoutes";

describe("workspace task routes", () => {
  it("keeps a session-only task trail without duplicate layers", () => {
    let trail: WorkspaceTaskRoute[] = [];
    trail = pushWorkspaceTaskRoute(trail, "environment-carbon-study");
    trail = pushWorkspaceTaskRoute(trail, "environment-recording-management");
    expect(topWorkspaceTaskRoute(trail)).toBe(
      "environment-recording-management",
    );
    trail = pushWorkspaceTaskRoute(trail, "environment-carbon-study");
    expect(trail).toEqual(["environment-carbon-study"]);
    expect(topWorkspaceTaskRoute(popWorkspaceTaskRoute(trail))).toBeNull();
  });

  it("allows a task only in its registered workspace", () => {
    expect(
      taskBelongsToWorkspace("environment-carbon-study", "environment"),
    ).toBe(true);
    expect(taskBelongsToWorkspace("environment-carbon-study", "markets")).toBe(
      false,
    );
  });
});
