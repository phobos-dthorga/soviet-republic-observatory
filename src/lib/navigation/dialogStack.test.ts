import { describe, expect, it } from "vitest";
import {
  dialogLayer,
  pushDialogRoute,
  removeDialogRoute,
  topDialogRoute,
  type DialogRoute,
} from "./dialogStack";

describe("dialog navigation stack", () => {
  it("returns from a child to its retained parent", () => {
    let stack: DialogRoute[] = [];
    stack = pushDialogRoute(stack, "settings");
    stack = pushDialogRoute(stack, "theme");

    expect(topDialogRoute(stack)).toBe("theme");
    stack = removeDialogRoute(stack, "theme");
    expect(stack).toEqual(["settings"]);
  });

  it("navigates back to an existing route instead of duplicating it", () => {
    const stack: DialogRoute[] = ["settings", "legal", "research"];
    expect(pushDialogRoute(stack, "settings")).toEqual(["settings"]);
  });

  it("places recovery above the dialog which requested it", () => {
    const stack = pushDialogRoute(["settings"], "recovery");
    expect(topDialogRoute(stack)).toBe("recovery");
    expect(dialogLayer(stack, "settings")).toBe(0);
    expect(dialogLayer(stack, "recovery")).toBe(1);
  });

  it("can remove a suspended route while a confirmation is active", () => {
    expect(removeDialogRoute(["settings", "recovery"], "settings")).toEqual([
      "recovery",
    ]);
  });
});
