import type {
  WorkspaceName,
  WorkspaceSection,
} from "../navigation/relatedData";

export const workspaceTaskRegistry = {
  "environment-carbon-study": {
    workspace: "environment",
    section: "environment-carbon",
  },
  "environment-recording-management": {
    workspace: "environment",
    section: "environment-recording",
  },
  "broadcast-outcome-laboratory": {
    workspace: "broadcast",
    section: "outcomes",
  },
  "plan-editor": { workspace: "plan", section: "plan-editor" },
  "markets-basket-laboratory": {
    workspace: "markets",
    section: "markets-labs",
  },
  "markets-scenario-laboratory": {
    workspace: "markets",
    section: "markets-labs",
  },
  "materials-pathway-study": {
    workspace: "materials",
    section: "material-flow-laboratory",
  },
  "materials-overlay-editor": {
    workspace: "materials",
    section: "overlay-laboratory",
  },
  "archive-comparison": {
    workspace: "archive",
    section: "archive-comparison",
  },
} as const satisfies Record<
  string,
  { workspace: WorkspaceName; section: WorkspaceSection }
>;

export type WorkspaceTaskRoute = keyof typeof workspaceTaskRegistry;

export function pushWorkspaceTaskRoute(
  trail: WorkspaceTaskRoute[],
  route: WorkspaceTaskRoute,
): WorkspaceTaskRoute[] {
  const existing = trail.indexOf(route);
  if (existing >= 0) return trail.slice(0, existing + 1);
  return [...trail, route];
}

export function popWorkspaceTaskRoute(
  trail: WorkspaceTaskRoute[],
): WorkspaceTaskRoute[] {
  return trail.slice(0, -1);
}

export function topWorkspaceTaskRoute(
  trail: WorkspaceTaskRoute[],
): WorkspaceTaskRoute | null {
  return trail.at(-1) ?? null;
}

export function taskBelongsToWorkspace(
  route: WorkspaceTaskRoute,
  workspace: WorkspaceName,
): boolean {
  return workspaceTaskRegistry[route].workspace === workspace;
}
