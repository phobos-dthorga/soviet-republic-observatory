import { describe, expect, it } from "vitest";
import {
  concentrationHhi,
  effectiveProductCount,
  netDemographicChange,
  perThousand,
  planAttainment,
} from "./metrics";

describe("planning metrics", () => {
  it("calculates attainment against the scheduled value", () => {
    expect(planAttainment(92, 100)).toBeCloseTo(0.92);
  });

  it("refuses an absent or invalid schedule denominator", () => {
    expect(planAttainment(92, 0)).toBeNull();
    expect(planAttainment(Number.NaN, 100)).toBeNull();
  });
});

describe("demographic metrics", () => {
  it("reconciles positive and negative demographic flows", () => {
    expect(
      netDemographicChange({
        births: 285,
        immigration: 92,
        deaths: 42,
        escapes: 108,
      }),
    ).toBe(227);
  });

  it("normalises a valid count per thousand residents", () => {
    expect(perThousand(227, 50_000)).toBeCloseTo(4.54);
  });

  it("keeps an invalid or unavailable population denominator unavailable", () => {
    expect(perThousand(10, 0)).toBeNull();
    expect(
      netDemographicChange({
        births: -1,
        immigration: 0,
        deaths: 0,
        escapes: 0,
      }),
    ).toBeNull();
  });
});

describe("trade concentration", () => {
  it("reports complete concentration for one export", () => {
    expect(concentrationHhi([100])).toBe(1);
    expect(effectiveProductCount([100])).toBe(1);
  });

  it("reports four effective products for equal shares", () => {
    expect(concentrationHhi([25, 25, 25, 25])).toBeCloseTo(0.25);
    expect(effectiveProductCount([25, 25, 25, 25])).toBeCloseTo(4);
  });

  it("refuses empty, negative, or zero-only evidence", () => {
    expect(concentrationHhi([])).toBeNull();
    expect(concentrationHhi([5, -1])).toBeNull();
    expect(concentrationHhi([0, 0])).toBeNull();
  });
});
