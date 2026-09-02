import type { TranslationKey } from "../i18n/catalog";
import type { ExactObservationReference } from "../observations/types";

export const workspaceSections = {
  briefing: ["briefing", "assays", "capabilities", "dispatch"],
  monitor: ["monitor-health", "monitor-pulse", "monitor-ledger"],
  broadcast: [
    "pulse",
    "receivers",
    "audience",
    "programme",
    "outcomes",
    "bulletin",
  ],
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
  environment: [
    "environment-overview",
    "environment-pollution",
    "environment-waste",
    "environment-water",
    "environment-carbon",
    "environment-recording",
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
  activityChannel?: string;
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

export type RelatedDataRelationship =
  "details" | "composition" | "history" | "source" | "planning";

export type RelatedDataSubject =
  | { kind: "metric"; metricId: string }
  | { kind: "observation"; reference: ExactObservationReference }
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
  environment: "nav-environment",
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
    id: workspaceLocationId(workspace, section, filters),
    labelKey: workspaceLabelKey(workspace),
    relationship: "details",
    location: { workspace, section, filters },
  };
}

function withRelationship(
  destination: RelatedDataDestination,
  relationship: RelatedDataRelationship,
): RelatedDataDestination {
  return { ...destination, relationship };
}

function workspaceLocationId(
  workspace: WorkspaceName,
  section: WorkspaceSection,
  filters: WorkspaceFilters,
): string {
  const identity = Object.entries(filters)
    .filter(([, value]) => value !== undefined)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, value]) => `${key}=${encodeURIComponent(String(value))}`)
    .join("&");
  return `${workspace}:${section}${identity ? `?${identity}` : ""}`;
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
        exactWorkspaceDestination("briefing", "briefing", subject.reference),
        exactWorkspaceDestination("broadcast", "receivers", subject.reference),
        exactWorkspaceDestination(
          "population",
          "population-status",
          subject.reference,
        ),
        exactWorkspaceDestination(
          "environment",
          "environment-overview",
          subject.reference,
        ),
        exactWorkspaceDestination(
          "markets",
          "markets-pulse",
          subject.reference,
        ),
        exactWorkspaceDestination(
          "archive",
          "archive-overview",
          subject.reference,
        ),
      ];
    case "resource":
      return resourceDestinations(subject);
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
        withRelationship(
          workspaceDestination("materials", "definition-dossier", {
            catalogueEntityId: subject.entityId,
          }),
          "source",
        ),
      ];
    case "plan_target":
      return [
        ...destinationsForMetric(subject.metricId).filter(
          (destination) => destination.location.workspace !== "plan",
        ),
        withRelationship(
          workspaceDestination("plan", "plan-trajectory", {
            metricId: subject.metricId,
            planRevision: subject.revision,
          }),
          "planning",
        ),
      ];
  }
}

function resourceDestinations(
  subject: Extract<RelatedDataSubject, { kind: "resource" }>,
): RelatedDataDestination[] {
  const sourceToken = sourceResourceToken(subject.resourceToken);
  const definitionId = definitionResourceId(sourceToken);
  const destinations = [
    withRelationship(
      workspaceDestination("markets", "markets-trade", {
        resourceToken: sourceToken,
        currency: subject.currency,
        channel: subject.channel,
      }),
      "details",
    ),
    withRelationship(
      workspaceDestination("markets", "markets-prices", {
        resourceToken: sourceToken,
        currency: subject.currency,
      }),
      "history",
    ),
    withRelationship(
      workspaceDestination("materials", "material-flow-laboratory", {
        resourceToken: definitionId,
      }),
      "source",
    ),
    withRelationship(
      workspaceDestination("environment", "environment-overview", {
        resourceToken: sourceToken,
      }),
      "history",
    ),
  ];
  if (isElectronicsEconomyResource(sourceToken)) {
    destinations.push(
      withRelationship(
        workspaceDestination("broadcast", "receivers", {
          metricId: "core.citizens.electronics.classified_total",
        }),
        "history",
      ),
    );
  }
  return destinations;
}

export function electronicsEconomyDestinations(
  resourceToken: "eletronics" | "ecomponents",
): RelatedDataDestination[] {
  const sourceToken = sourceResourceToken(resourceToken);
  const tradeLabels = {
    rub: {
      standard: "related-nav-market-trade-rub-standard",
      international: "related-nav-market-trade-rub-international",
    },
    usd: {
      standard: "related-nav-market-trade-usd-standard",
      international: "related-nav-market-trade-usd-international",
    },
  } as const satisfies Record<
    "rub" | "usd",
    Record<"standard" | "international", TranslationKey>
  >;
  const priceLabels = {
    rub: "related-nav-market-price-rub",
    usd: "related-nav-market-price-usd",
  } as const satisfies Record<"rub" | "usd", TranslationKey>;
  const tradeChoices = (["rub", "usd"] as const).flatMap((currency) =>
    (["standard", "international"] as const).map((channel) => ({
      ...workspaceDestination("markets", "markets-trade", {
        resourceToken: sourceToken,
        currency,
        channel,
      }),
      labelKey: tradeLabels[currency][channel],
      relationship: "details" as const,
    })),
  );
  const priceChoices = (["rub", "usd"] as const).map((currency) => ({
    ...workspaceDestination("markets", "markets-prices", {
      resourceToken: sourceToken,
      currency,
    }),
    labelKey: priceLabels[currency],
    relationship: "history" as const,
  }));
  return [
    ...tradeChoices,
    ...priceChoices,
    {
      ...workspaceDestination("materials", "material-flow-laboratory", {
        resourceToken: definitionResourceId(sourceToken),
      }),
      labelKey: "related-nav-production-routes",
      relationship: "source",
    },
  ];
}

function sourceResourceToken(value: string): string {
  return value.startsWith("resource::")
    ? value.slice("resource::".length)
    : value;
}

function definitionResourceId(value: string): string {
  return `resource::${sourceResourceToken(value)}`;
}

function isElectronicsEconomyResource(value: string): boolean {
  return value === "eletronics" || value === "ecomponents";
}

function exactWorkspaceDestination(
  workspace: WorkspaceName,
  section: WorkspaceSection,
  reference: ExactObservationReference,
): RelatedDataDestination {
  const destination = workspaceDestination(workspace, section, {
    interpretationId: reference.interpretation_id,
  });
  return {
    ...destination,
    relationship: "history",
    exactObservation: reference,
    location: {
      ...destination.location,
      focusId:
        workspace === "archive"
          ? `archive-observation-${reference.interpretation_id}`
          : destination.location.focusId,
    },
  };
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
      withRelationship(
        workspaceDestination("population", "population-status", { metricId }),
        "details",
      ),
      withRelationship(
        workspaceDestination("plan", "plan-editor", { metricId }),
        "planning",
      ),
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
