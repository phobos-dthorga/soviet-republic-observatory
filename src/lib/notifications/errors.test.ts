import { describe, expect, it } from "vitest";
import { detailsFromError } from "./errors";

describe("command error details", () => {
  it("keeps the host code and diagnostic behind the expandable summary", () => {
    expect(
      detailsFromError(
        {
          code: "research_source_archive_invalid",
          diagnostic: "The archive contained an invalid entry.",
        },
        { operation: "research_source_download" },
      ),
    ).toEqual({
      code: "research_source_archive_invalid",
      operation: "research_source_download",
      detail: "The archive contained an invalid entry.",
    });
  });

  it("uses safe fallback details for non-command failures", () => {
    expect(
      detailsFromError("network stopped", {
        code: "unknown",
        operation: "background_task",
      }),
    ).toEqual({ code: "unknown", operation: "background_task" });
  });
});
