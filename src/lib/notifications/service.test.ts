import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  clearNotifications,
  dismissNotification,
  notifications,
  notify,
} from "./service";

describe("application notifications", () => {
  beforeEach(() => {
    clearNotifications();
    vi.restoreAllMocks();
  });

  it("assigns a tone-specific default lifetime", () => {
    notify({ message: "Catalogue ready", tone: "success" });
    notify({ message: "Review required", tone: "error" });

    expect(
      get(notifications).map(({ tone, timeoutMs }) => [tone, timeoutMs]),
    ).toEqual([
      ["success", 6_000],
      ["error", 0],
    ]);
  });

  it("keeps a bounded visible queue", () => {
    for (let index = 0; index < 7; index += 1) {
      notify({ message: `Notice ${index}` });
    }

    expect(get(notifications).map(({ message }) => message)).toEqual([
      "Notice 2",
      "Notice 3",
      "Notice 4",
      "Notice 5",
      "Notice 6",
    ]);
  });

  it("dismisses one notice without disturbing its neighbours", () => {
    const first = notify({ message: "First" });
    notify({ message: "Second" });

    dismissNotification(first);

    expect(get(notifications).map(({ message }) => message)).toEqual([
      "Second",
    ]);
  });
});
