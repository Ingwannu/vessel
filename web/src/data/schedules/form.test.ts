import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type { OptionChoice } from "../../wire/proto";
import {
  editRequest,
  maxRuns,
  nameError,
  promptError,
  type ScheduleEdit,
  sessionOptions,
} from "./form";

const timestamp = "2026-11-21T00:00:00Z";

const stored: ScheduleEdit = {
  id: fixtureId("ScheduleId", "fixture"),
  definition: {
    name: "Elapsed schedule",
    cadence: { kind: "every", seconds: 3_600, anchor: timestamp },
    target: {
      kind: "new_session",
      after: "keep",
      spec: { mode: "structured", harness: "claude-code" },
    },
    prompt: "A test prompt",
    catch_up: "once",
    max_runs: 2,
    ends_at: "2027-01-01T00:00:00Z",
  },
  defined_by: { kind: "owner" },
  state: { state: "active", next_due: timestamp },
  runs: 0,
  created_at: timestamp,
  updated_at: timestamp,
  revision: 4,
};

describe("schedule edit values", () => {
  it("preserves the interval anchor, expiry, catch-up policy and revision", () => {
    const request = editRequest(stored, {
      name: "Edited",
      prompt: stored.definition.prompt,
      cadence: stored.definition.cadence,
      spec: stored.definition.target.spec,
      archive: false,
      cap: 2,
    });
    expect(request).toEqual({
      schedule: stored.id,
      revision: 4,
      definition: { ...stored.definition, name: "Edited" },
    });
  });

  it("keeps unrelated stored options in application order", () => {
    const first = fixtureId("OptionId", "first");
    const other = fixtureId("OptionId", "other");
    const options: OptionChoice[] = [
      { id: first, kind: "toggle", on: false },
      { id: other, kind: "toggle", on: true },
    ];
    expect(
      sessionOptions(options, [{ id: first, kind: "toggle", on: true }]),
    ).toEqual([
      { id: first, kind: "toggle", on: true },
      { id: other, kind: "toggle", on: true },
    ]);
  });

  it("distinguishes no cap from an invalid cap and validates byte bounds", () => {
    expect(maxRuns("")).toBeNull();
    expect(maxRuns("0")).toBe("invalid");
    expect(maxRuns("1.5")).toBe("invalid");
    expect(maxRuns("9")).toBe(9);
    expect(nameError("한".repeat(27))).toContain("80 bytes");
    expect(promptError("word\n\tmore")).toBeNull();
    expect(promptError("word\u0000more")).toContain("unsupported control");
  });
});
