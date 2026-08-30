import { beforeEach, describe, expect, it } from "vitest";
import {
  dismissAttentionCue,
  getAttentionCueStatus,
  replayAttentionCue,
} from "./service";

describe("attention cue lifecycle", () => {
  beforeEach(async () => {
    await replayAttentionCue("research.setup.entry", 1);
  });

  it("separates dismissal from replay by content revision", async () => {
    expect(
      (await getAttentionCueStatus("research.setup.entry", 1)).dismissed,
    ).toBe(false);
    await dismissAttentionCue("research.setup.entry", 1);
    expect(
      (await getAttentionCueStatus("research.setup.entry", 1)).dismissed,
    ).toBe(true);
    expect(
      (await getAttentionCueStatus("research.setup.entry", 2)).dismissed,
    ).toBe(false);
    await replayAttentionCue("research.setup.entry", 1);
    expect(
      (await getAttentionCueStatus("research.setup.entry", 1)).dismissed,
    ).toBe(false);
  });
});
