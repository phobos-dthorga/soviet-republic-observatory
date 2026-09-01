import { describe, expect, it } from "vitest";
import {
  destinationsForSubject,
  electronicsEconomyDestinations,
} from "./relatedData";

describe("electronics-economy related navigation", () => {
  it("offers explicit currency and channel choices from Broadcast", () => {
    const destinations = electronicsEconomyDestinations("eletronics");

    expect(destinations).toHaveLength(7);
    expect(
      destinations.map((destination) => destination.location.filters),
    ).toContainEqual({
      resourceToken: "eletronics",
      currency: "rub",
      channel: "standard",
    });
    expect(
      destinations.map((destination) => destination.location.filters),
    ).toContainEqual({
      resourceToken: "eletronics",
      currency: "usd",
    });
    expect(destinations.at(-1)?.location).toEqual({
      workspace: "materials",
      section: "material-flow-laboratory",
      filters: { resourceToken: "resource::eletronics" },
    });
  });

  it("links exact electronics source rows back to receiver uptake", () => {
    const destinations = destinationsForSubject({
      kind: "resource",
      resourceToken: "resource::ecomponents",
      currency: "rub",
      channel: "standard",
    });

    expect(
      destinations.some(
        (destination) =>
          destination.location.workspace === "broadcast" &&
          destination.location.section === "receivers",
      ),
    ).toBe(true);
    expect(destinations[0].location.filters.resourceToken).toBe("ecomponents");
  });

  it("does not invent receiver or demographic links for other resources", () => {
    const destinations = destinationsForSubject({
      kind: "resource",
      resourceToken: "resource::steel",
    });

    expect(
      destinations.some(
        (destination) => destination.location.workspace === "broadcast",
      ),
    ).toBe(false);
    expect(
      destinations.some(
        (destination) => destination.location.workspace === "population",
      ),
    ).toBe(false);
  });
});
