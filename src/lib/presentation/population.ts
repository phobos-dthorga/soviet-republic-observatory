import type { ChartPoint, ChartSpec, Provenance } from "../charts/types";
import type { TranslationKey } from "../i18n/catalog";
import type { Translator } from "../i18n/runtime";
import type {
  PopulationCitySnapshot,
  PopulationDataset,
  PopulationFact,
  PopulationObservation,
} from "../observations/types";

export const POPULATION_FACT_LABELS = {
  "source.stats.citizens.born": "population-fact-born",
  "source.stats.citizens.dead": "population-fact-dead",
  "source.stats.citizens.escaped": "population-fact-escaped",
  "source.stats.citizens.immigrant_soviet": "population-fact-immigrant-soviet",
  "source.stats.citizens.immigrant_africa": "population-fact-immigrant-africa",
  "source.stats.citizens.small_children": "population-fact-small-children",
  "source.stats.citizens.medium_children": "population-fact-medium-children",
  "source.stats.citizens.adults_parent": "population-fact-adults-parent",
  "source.stats.citizens.adults": "population-fact-adults",
  "source.stats.citizens.unemployed": "population-fact-unemployed",
  "source.stats.citizens.no_education": "population-fact-no-education",
  "source.stats.citizens.basic_education": "population-fact-basic-education",
  "source.stats.citizens.higher_education": "population-fact-higher-education",
  "source.stats.citizens.car_owners": "population-fact-car-owners",
  "core.citizens.electronics.none": "receiver-none",
  "core.citizens.electronics.radio": "receiver-radio",
  "core.citizens.electronics.television": "receiver-television",
  "core.citizens.electronics.computer": "receiver-computer",
} as const satisfies Record<string, TranslationKey>;

const STATUS_FACTS = [
  "source.stats.citizens.small_children",
  "source.stats.citizens.medium_children",
  "source.stats.citizens.adults",
  "source.stats.citizens.unemployed",
] as const;
const MOVEMENT_FACTS = [
  "source.stats.citizens.born",
  "source.stats.citizens.dead",
  "source.stats.citizens.escaped",
  "source.stats.citizens.immigrant_soviet",
  "source.stats.citizens.immigrant_africa",
] as const;
const EDUCATION_FACTS = [
  "source.stats.citizens.no_education",
  "source.stats.citizens.basic_education",
  "source.stats.citizens.higher_education",
] as const;

export function populationFact(
  facts: PopulationFact[],
  factId: string,
): PopulationFact | null {
  return facts.find((fact) => fact.fact_id === factId) ?? null;
}

export function populationFactLabel(
  factId: string,
  translate: Translator,
): string {
  const key = (
    POPULATION_FACT_LABELS as Partial<Record<string, TranslationKey>>
  )[factId];
  return key
    ? translate(key)
    : translate("population-fact-unrecognised", { id: factId });
}

function dateLabel(
  observation: Pick<PopulationObservation, "sampled_year" | "sampled_day">,
  translate: Translator,
): string {
  return translate("observation-game-date-compact", {
    year: observation.sampled_year,
    day: String(observation.sampled_day).padStart(3, "0"),
  });
}

function provenance(
  dataset: PopulationDataset,
  translate: Translator,
): Provenance {
  const latest = dataset.observations.at(-1);
  return {
    kind: "save_fact",
    source: latest
      ? translate("population-source-snapshot", {
          file: latest.source_file_name,
          profile: `${latest.profile_id}@${latest.profile_version}`,
        })
      : translate("population-source-no-snapshot"),
    observed_at: latest
      ? dateLabel(latest, translate)
      : translate("chart-unavailable"),
    coverage:
      dataset.observations.length > 0 &&
      dataset.observations.every(
        (observation) => observation.coverage_status === "complete",
      )
        ? "complete"
        : "partial",
  };
}

function observationPoints(
  dataset: PopulationDataset,
  factId: string,
  translate: Translator,
): ChartPoint[] {
  const points: ChartPoint[] = [];
  let previousIncluded: PopulationObservation | undefined;
  let missingSincePrevious = false;
  for (const observation of dataset.observations) {
    const fact = populationFact(observation.facts, factId);
    if (!fact) {
      if (previousIncluded) missingSincePrevious = true;
      continue;
    }
    points.push({
      category: dateLabel(observation, translate),
      category_value: observation.sampled_game_day,
      value: fact.value,
      gap_before:
        previousIncluded !== undefined &&
        (missingSincePrevious ||
          observation.membership_revision !==
            previousIncluded.membership_revision + 1 ||
          observation.sampled_game_day <= previousIncluded.sampled_game_day),
    });
    previousIncluded = observation;
    missingSincePrevious = false;
  }
  return points;
}

function trendChart(
  dataset: PopulationDataset,
  factIds: readonly string[],
  id: string,
  title: TranslationKey,
  description: TranslationKey,
  translate: Translator,
): ChartSpec {
  return {
    schema_version: 1,
    id,
    title: translate(title),
    description: translate(description),
    kind: "line",
    category_axis_scale: "game_day",
    category_axis_label: translate("chart-axis-game-date"),
    value_axis_label: translate("population-axis-recorded-citizens"),
    unit: translate("unit-citizens"),
    series: factIds.flatMap((factId) => {
      const points = observationPoints(dataset, factId, translate);
      return points.length
        ? [
            {
              id: factId,
              label: populationFactLabel(factId, translate),
              points,
            },
          ]
        : [];
    }),
    provenance: provenance(dataset, translate),
  };
}

export function createPopulationStatusChart(
  dataset: PopulationDataset,
  translate: Translator,
): ChartSpec {
  return trendChart(
    dataset,
    STATUS_FACTS,
    "core.population.save_sampled_status",
    "population-chart-status-title",
    "population-chart-status-description",
    translate,
  );
}

export function createPopulationMovementChart(
  dataset: PopulationDataset,
  translate: Translator,
): ChartSpec {
  return trendChart(
    dataset,
    MOVEMENT_FACTS,
    "core.population.recorded_movement_counters",
    "population-chart-movement-title",
    "population-chart-movement-description",
    translate,
  );
}

function factBarChart(
  facts: PopulationFact[],
  factIds: readonly string[],
  id: string,
  title: TranslationKey,
  description: TranslationKey,
  chartProvenance: Provenance,
  translate: Translator,
): ChartSpec {
  const points = factIds.flatMap((factId) => {
    const fact = populationFact(facts, factId);
    return fact
      ? [
          {
            category: populationFactLabel(factId, translate),
            value: fact.value,
          },
        ]
      : [];
  });
  return {
    schema_version: 1,
    id,
    title: translate(title),
    description: translate(description),
    kind: "bar",
    orientation: "horizontal",
    category_axis_label: translate("population-axis-source-category"),
    value_axis_label: translate("population-axis-recorded-citizens"),
    unit: translate("unit-citizens"),
    series: points.length
      ? [
          {
            id: `${id}.values`,
            label: translate("population-series-recorded-count"),
            points,
          },
        ]
      : [],
    provenance: chartProvenance,
  };
}

export function createEducationProfileChart(
  dataset: PopulationDataset,
  translate: Translator,
): ChartSpec {
  return factBarChart(
    dataset.observations.at(-1)?.facts ?? [],
    EDUCATION_FACTS,
    "core.population.education_profile",
    "population-chart-education-title",
    "population-chart-education-description",
    provenance(dataset, translate),
    translate,
  );
}

export function createCityMovementChart(
  city: PopulationCitySnapshot | null,
  dataset: PopulationDataset,
  translate: Translator,
): ChartSpec {
  const latest = dataset.observations.at(-1);
  const cityProvenance: Provenance = {
    ...provenance(dataset, translate),
    observed_at: city
      ? dateLabel(city, translate)
      : latest
        ? dateLabel(latest, translate)
        : translate("chart-unavailable"),
    coverage: city?.coverage_status ?? "partial",
  };
  return factBarChart(
    city?.facts ?? [],
    MOVEMENT_FACTS,
    "core.population.city_movement_snapshot",
    "population-chart-city-title",
    "population-chart-city-description",
    cityProvenance,
    translate,
  );
}
