import { describe, expect, it } from "vitest";
import {
  observeLatestTaskProgress,
  selectLatestTaskProgress,
  type TimestampedTaskProgress,
} from "./progress";

type TestProgress = TimestampedTaskProgress & { phase: string };

describe("critical task progress", () => {
  it("keeps a live startup update when an older snapshot arrives later", async () => {
    let listener: ((progress: TestProgress) => void) | undefined;
    let resolveSnapshot: ((progress: TestProgress) => void) | undefined;
    const accepted: string[] = [];
    const stop = await observeLatestTaskProgress<TestProgress>(
      {
        listen: async (accept) => {
          listener = accept;
          return () => undefined;
        },
        read: () =>
          new Promise((resolve) => {
            resolveSnapshot = resolve;
          }),
      },
      (progress) => accepted.push(progress.phase),
    );

    listener?.({
      phase: "scanning",
      started_at_ms: 200,
      updated_at_ms: 240,
    });
    resolveSnapshot?.({
      phase: "idle",
      started_at_ms: 100,
      updated_at_ms: 150,
    });
    await Promise.resolve();

    expect(accepted).toEqual(["scanning"]);
    stop();
  });

  it("accepts later updates from the same run and a newer run", () => {
    const scanning: TestProgress = {
      phase: "scanning",
      started_at_ms: 200,
      updated_at_ms: 240,
    };
    const publishing: TestProgress = {
      phase: "publishing",
      started_at_ms: 200,
      updated_at_ms: 260,
    };
    const nextRun: TestProgress = {
      phase: "discovering",
      started_at_ms: 300,
      updated_at_ms: 300,
    };

    expect(selectLatestTaskProgress(scanning, publishing)).toBe(publishing);
    expect(selectLatestTaskProgress(publishing, nextRun)).toBe(nextRun);
  });
});
