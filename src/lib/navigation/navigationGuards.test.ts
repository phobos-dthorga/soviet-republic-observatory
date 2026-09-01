import { afterEach, describe, expect, it } from "vitest";
import {
  hasUnsavedNavigationChanges,
  registerNavigationGuard,
} from "./navigationGuards";

const removals: Array<() => void> = [];

afterEach(() => removals.splice(0).forEach((remove) => remove()));

describe("related navigation draft guards", () => {
  it("blocks while any mounted workspace reports unsaved changes", () => {
    removals.push(registerNavigationGuard("clean", () => false));
    removals.push(registerNavigationGuard("dirty", () => true));
    expect(hasUnsavedNavigationChanges()).toBe(true);
  });

  it("removes a guard only when its own registration is disposed", () => {
    const first = registerNavigationGuard("plan", () => true);
    const second = registerNavigationGuard("plan", () => false);
    removals.push(first, second);
    first();
    expect(hasUnsavedNavigationChanges()).toBe(false);
  });
});
