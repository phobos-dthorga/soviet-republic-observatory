import type { ChartSpec } from "../charts/types";
import type { ExactObservationReference } from "../observations/types";
import type {
  ChartNavigationBinding,
  RelatedDataDestination,
  WorkspaceLocation,
} from "./relatedData";

export type ExactObservationPoint = {
  game_day: number;
  exact_observation: ExactObservationReference | null;
};

export function exactObservationDestination(
  reference: ExactObservationReference,
  location: WorkspaceLocation,
): RelatedDataDestination {
  return {
    id: `exact:${reference.interpretation_id}:${location.workspace}:${location.section}`,
    labelKey: "related-nav-history",
    relationship: "history",
    location: {
      ...location,
      filters: {
        ...location.filters,
        interpretationId: reference.interpretation_id,
      },
    },
    exactObservation: reference,
  };
}

export function exactObservationChartBindings(
  spec: ChartSpec,
  points: readonly ExactObservationPoint[],
  location: WorkspaceLocation,
): ChartNavigationBinding[] {
  const byGameDay = uniqueExactReferences(points);
  return spec.series.flatMap((series) =>
    series.points.flatMap((point, pointIndex) => {
      if (point.category_value === undefined) return [];
      const reference = byGameDay.get(point.category_value);
      if (!reference) return [];
      return [
        {
          seriesId: series.id,
          pointIndex,
          destinations: [exactObservationDestination(reference, location)],
        },
      ];
    }),
  );
}

function uniqueExactReferences(
  points: readonly ExactObservationPoint[],
): Map<number, ExactObservationReference> {
  const candidates = new Map<number, ExactObservationReference[]>();
  for (const point of points) {
    if (!point.exact_observation) continue;
    candidates.set(point.game_day, [
      ...(candidates.get(point.game_day) ?? []),
      point.exact_observation,
    ]);
  }
  return new Map(
    [...candidates.entries()].flatMap(([gameDay, references]) => {
      const unique = new Map(
        references.map((reference) => [
          `${reference.branch_id}:${reference.interpretation_id}`,
          reference,
        ]),
      );
      return unique.size === 1 ? [[gameDay, [...unique.values()][0]]] : [];
    }),
  );
}
