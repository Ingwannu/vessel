import { expect, test } from "vitest";

import type { ToolKind } from "../wire/proto";
import type { Entry } from "./timeline-model";
import { runSummary, toolRuns } from "./tool-runs";

function tool(seq: number, kind: ToolKind): Entry {
  return {
    kind: "tool",
    key: `t${String(seq)}`,
    seq,
    at: "",
    tool: {
      id: `c${String(seq)}`,
      title: "x",
      kind,
      status: "completed",
      content: [],
      locations: [],
      input: null,
      output: null,
    },
  };
}

test("Paseo's sentence", () => {
  expect(runSummary(["edit", "edit", "execute", "execute", "execute"], 0)).toBe(
    "Edited 2 files and ran 3 commands",
  );
  expect(runSummary(["read", "search", "fetch"], 1)).toBe(
    "Read 1 file, searched 1 time and used 1 other tool",
  );
  expect(runSummary([], 2)).toBe("Thought");
});

test("runs of two or more are folded; a message ends a run", () => {
  const folds = toolRuns([
    tool(1, "read"),
    tool(2, "edit"),
    { kind: "message", key: "m3", seq: 3, at: "", text: "done" },
    tool(4, "execute"),
  ]);
  expect(folds.get("t1")?.members).toEqual(["t1", "t2"]);
  expect(folds.get("t2")?.lead).toBe("t1");
  expect(folds.has("t4")).toBe(false);
  expect(folds.has("m3")).toBe(false);
});
