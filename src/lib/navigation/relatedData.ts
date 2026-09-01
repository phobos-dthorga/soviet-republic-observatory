import type { TranslationKey } from "../i18n/catalog";

export const workspaceSections = {
  briefing: ["briefing", "assays", "capabilities", "dispatch"],
  monitor: ["monitor-health", "monitor-pulse", "monitor-ledger"],
  broadcast: ["receivers", "audience", "programme", "outcomes", "bulletin"],
  extensions: [
    "pack-inspection",
    "pack-library",
    "pack-charts",
    "model-plugins",
  ],
  plan: ["plan-status", "plan-trajectory", "plan-editor", "plan-revisions"],
  materials: [
    "material-flow-laboratory",
    "catalogue-browser",
    "definition-dossier",
    "overlay-laboratory",
  ],
  population: [
    "population-status",
    "population-movement",
    "population-cities",
    "population-identity",
  ],
  markets: [
    "markets-pulse",
    "markets-trade",
    "markets-prices",
    "markets-cities",
    "markets-labs",
  ],
  archive: ["archive-overview", "archive-comparison"],
} as const;

export type WorkspaceName = keyof typeof workspaceSections;
export type WorkspaceSection =
  (typeof workspaceSections)[WorkspaceName][number];

export type WorkspaceFilters = {
  metricId?: string;
  stationId?: "none" | "radio" | "television" | "computer";
  cityId?: string;
  currency?: "rub" | "usd";
  channel?: "standard" | "international";
  resourceToken?: string;
  catalogueEntityId?: string;
  planRevision?: number;
  interpretationId?: string;
};

export type WorkspaceLocation = {
  workspace: WorkspaceName;
  section: WorkspaceSection;
  focusId?: string;
  filters: WorkspaceFilters;
};

export type ExactObservationReference = {
  interpretationId: string;
  branchId: string;
  year: number;
  day: number;
};

export type RelatedDataRelationship =
  "details" | "composition" | "history" | "source" | "planning";

export type RelatedDataSubject =
  | { kind: "metric"; metricId: string }
  | { kind: "observation"; interpretationId: string }
  | {
      kind: "resource";
      resourceToken: string;
      currency?: "rub" | "usd";
      channel?: "standard" | "international";
    }
  | { kind: "city"; cityId: string }
  | { kind: "catalogue_entity"; entityId: string }
  | { kind: "plan_target"; metricId: string; revision: number }
  | { kind: "extension_contribution"; metricId: string };

export type RelatedDataDestination = {
  id: string;
  labelKey: TranslationKey;
  relationship: RelatedDataRelationship;
  location: WorkspaceLocation;
  exactObservation?: ExactObservationReference;
};

export type AnalysisContextReference = {
  branchId: string;
  headInterpretationId: string | null;
  isTip: boolean;
};

export type NavigationTrailEntry = {
  location: WorkspaceLocation;
  context: AnalysisContextReference | null;
};

export type ChartNavigationBinding = {
  seriesId: string;
  pointIndex: number;
  destinations: RelatedDataDestination[];
};

const workspaceLabels = {
  briefing: "nav-briefing",
  monitor: "nav-monitor",
  broadcast: "nav-broadcast",
  extensions: "nav-extensions",
  plan: "nav-plan",
  materials: "nav-materials",
  population: "nav-population",
  markets: "nav-markets",
  archive: "nav-archive",
} as const satisfies Record<WorkspaceName, TranslationKey>;

export function workspaceLabelKey(workspace: WorkspaceName): TranslationKey {
  return workspaceLabels[workspace];
}

const relationshipLabels = {
  details: "related-nav-details",
  composition: "related-nav-composition",
  history: "related-nav-history",
  source: "related-nav-source",
  planning: "related-nav-planning",
} as const satisfies Record<RelatedDataRelationship, TranslationKey>;

export function relationshipLabelKey(
  relationship: RelatedDataRelationship,
): TranslationKey {
  return relationshipLabels[relationship];
}

export function defaultWorkspaceLocation(
  workspace: WorkspaceName,
): WorkspaceLocation {
  return {
    workspace,
    section: workspaceSections[workspace][0],
    filters: {},
  };
}

export function workspaceDestination(
  workspace: WorkspaceName,
  section: WorkspaceSection = workspaceSections[workspace][0],
  filters: WorkspaceFilters = {},
): RelatedDataDestination {
  if (!isWorkspaceSection(workspace, section)) {
    throw new Error("invalid_related_destination");
  }
  return {
    id: `${workspace}:${section}`,
    labelKey: workspaceLabelKey(workspace),
    relationship: "details",
    location: { workspace, section, filters },
  };
}

export function isWorkspaceSection(
  workspace: WorkspaceName,
  section: string,
): section is WorkspaceSection {
  return (workspaceSections[workspace] as readonly string[]).includes(section);
}

export function destinationsForSubject(
  subject: RelatedDataSubject,
): RelatedDataDestination[] {
  switch (subject.kind) {
    case "metric":
    case "extension_contribution":
      return destinationsForMetric(subject.metricId);
    case "observation":
      return [
        workspaceDestination("archive", "archive-overview", {
          interpretationId: subject.interpretationId,
        }),
      ];
    case "resource":
      return [
        workspaceDestination("markets", "markets-trade", {
          resourceToken: subject.resourceToken,
          currency: subject.currency,
          channel: subject.channel,
        }),
        workspaceDestination("markets", "markets-prices", {
          resourceToken: subject.resourceToken,
          currency: subject.currency,
        }),
        workspaceDestination("materials", "catalogue-browser", {
          catalogueEntityId: subject.resourceToken,
        }),
      ];
    case "city":
      return [
        workspaceDestination("population", "population-cities", {
          cityId: subject.cityId,
        }),
        workspaceDestination("markets", "markets-cities", {
          cityId: subject.cityId,
        }),
      ];
    case "catalogue_entity":
      return [
        workspaceDestination("materials", "definition-dossier", {
          catalogueEntityId: subject.entityId,
        }),
      ];
    case "plan_target":
      return [
        ...destinationsForMetric(subject.metricId),
        workspaceDestination("plan", "plan-trajectory", {
          metricId: subject.metricId,
          planRevision: subject.revision,
        }),
      ];
  }
}

function destinationsForMetric(metricId: string): RelatedDataDestination[] {
  if (metricId.includes("receiver") || metricId.includes("electronic")) {
    return [workspaceDestination("broadcast", "receivers", { metricId })];
  }
  if (metricId.startsWith("market.")) {
    return [workspaceDestination("markets", "markets-pulse", { metricId })];
  }
  if (
    metricId.includes("adults") ||
    metricId.includes("children") ||
    metricId.includes("education") ||
    metricId.includes("unemployed") ||
    metricId.includes("born") ||
    metricId.includes("dead") ||
    metricId.includes("escaped") ||
    metricId.includes("immigrant")
  ) {
    return [
      workspaceDestination("population", "population-status", { metricId }),
      workspaceDestination("plan", "plan-trajectory", { metricId }),
    ];
  }
  return [];
}

export function pushNavigationTrail(
  trail: NavigationTrailEntry[],
  entry: NavigationTrailEntry,
): NavigationTrailEntry[] {
  const previous = trail.at(-1);
  if (previous && locationsEqual(previous.location, entry.location))
    return trail;
  return [...trail, entry].slice(-20);
}

export function locationsEqual(
  left: WorkspaceLocation,
  right: WorkspaceLocation,
): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}
