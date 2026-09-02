import type { TechnicalDetailsView } from "./service";

export function detailsFromError(
  error: unknown,
  fallback?: TechnicalDetailsView,
): TechnicalDetailsView {
  if (typeof error !== "object" || error === null) return fallback ?? {};
  const candidate = error as { code?: unknown; diagnostic?: unknown };
  return {
    code: typeof candidate.code === "string" ? candidate.code : fallback?.code,
    operation: fallback?.operation,
    detail:
      typeof candidate.diagnostic === "string"
        ? candidate.diagnostic
        : fallback?.detail,
  };
}
