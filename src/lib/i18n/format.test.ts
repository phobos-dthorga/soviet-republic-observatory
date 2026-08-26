import { describe, expect, it } from "vitest";
import {
  formatCurrency,
  formatDate,
  formatNumber,
  formatPercent,
  formatSignedNumber,
} from "./format";

describe("locale formatting", () => {
  it("uses the selected locale for numbers, signs, and percentages", () => {
    expect(formatNumber(1234.5, "de-DE")).toBe("1.234,5");
    expect(formatSignedNumber(4.2, "en-AU")).toBe("+4.2");
    expect(formatPercent(12.5, "fr-FR")).toMatch(/12,5\s?%/);
  });

  it("centralises currency and calendar formatting", () => {
    expect(formatCurrency(12, "en-AU", "USD")).toContain("12.00");
    expect(
      formatDate(new Date("2004-08-17T00:00:00Z"), "en-AU", {
        timeZone: "UTC",
        dateStyle: "medium",
      }),
    ).toBe("17 Aug 2004");
  });
});
