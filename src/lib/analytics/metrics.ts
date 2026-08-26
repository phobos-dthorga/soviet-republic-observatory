export type DemographicFlows = {
  births: number;
  immigration: number;
  deaths: number;
  escapes: number;
};

export function planAttainment(
  actual: number,
  scheduled: number,
): number | null {
  if (
    !Number.isFinite(actual) ||
    !Number.isFinite(scheduled) ||
    scheduled <= 0
  ) {
    return null;
  }

  return actual / scheduled;
}

export function netDemographicChange(flows: DemographicFlows): number | null {
  const values = Object.values(flows);
  if (values.some((value) => !Number.isFinite(value) || value < 0)) return null;

  return flows.births + flows.immigration - flows.deaths - flows.escapes;
}

export function perThousand(
  count: number,
  populationExposure: number,
): number | null {
  if (
    !Number.isFinite(count) ||
    !Number.isFinite(populationExposure) ||
    populationExposure <= 0
  ) {
    return null;
  }

  return (count / populationExposure) * 1_000;
}

export function concentrationHhi(values: number[]): number | null {
  if (
    values.length === 0 ||
    values.some((value) => !Number.isFinite(value) || value < 0)
  ) {
    return null;
  }

  const total = values.reduce((sum, value) => sum + value, 0);
  if (total <= 0) return null;

  return values.reduce((sum, value) => sum + (value / total) ** 2, 0);
}

export function effectiveProductCount(values: number[]): number | null {
  const hhi = concentrationHhi(values);
  return hhi ? 1 / hhi : null;
}
