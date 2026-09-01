import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  clearNotifications,
  dismissNotification,
  notifications,
  notify,
  openRecoveryProposal,
  recoveryProposal,
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

  it("updates a keyed notification instead of flooding the queue", () => {
    const first = notify({
      message: "Preflight stopped",
      tone: "error",
      dedupeKey: "research.build.failure",
    });
    const second = notify({
      message: "Compile stopped",
      tone: "error",
      dedupeKey: "research.build.failure",
    });

    expect(second).toBe(first);
    expect(get(notifications)).toHaveLength(1);
    expect(get(notifications)[0]?.message).toBe("Compile stopped");
  });

  it("keeps a bounded recovery proposal behind an explicit review action", () => {
    const run = vi.fn();
    notify({
      message: "Storage contract stopped safely",
      tone: "error",
      recovery: {
        title: "Recover indexing",
        message: "Verify known contracts and retry.",
        actionLabel: "Repair and retry",
        run,
      },
    });

    const proposal = get(notifications)[0]?.recovery;
    expect(proposal?.actionLabel).toBe("Repair and retry");
    expect(get(recoveryProposal)).toBeNull();
    if (!proposal) throw new Error("expected recovery proposal");
    openRecoveryProposal(proposal);
    expect(get(recoveryProposal)?.run).toBe(run);

    clearNotifications();
    expect(get(recoveryProposal)).toBeNull();
  });
});
