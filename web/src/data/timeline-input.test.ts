import { expect, test } from "vitest";
import { fixtureId } from "../testing/ids";
import type { AgentEvent } from "../wire/proto";
import { searchable } from "./find";
import { outline } from "./outline";
import { rowsOf } from "./rows";
import { reduce } from "./timeline";
import { isInternalInput } from "./timeline-input";
import { EMPTY } from "./timeline-model";
import { spoken } from "./timeline-text";
import { toolRuns } from "./tool-runs";

function fold(events: AgentEvent[]) {
  return reduce(
    EMPTY,
    events.map((event, seq) => ({
      seq,
      at: "2026-11-16T00:00:00Z",
      event: { type: "agent" as const, event },
    })),
  );
}

const provenance = {
  kind: "internal_notice",
  notice: 2,
} as const;

function tool(id: string): AgentEvent {
  return {
    event: "tool_call",
    id: fixtureId("ToolCallId", id),
    title: id,
    kind: "read",
    status: "completed",
    content: [],
    locations: [],
  };
}

test("notice input stays a real hidden boundary without joining an earlier tool run or reply", () => {
  const model = fold([
    { event: "turn_started", content: [{ type: "text", text: "Person" }] },
    { event: "message_chunk", content: { type: "text", text: "First reply" } },
    { event: "notification", level: "info", message: "Child finished" },
    tool("before"),
    {
      event: "turn_started",
      content: [{ type: "text", text: "private provider instruction" }],
      provenance,
    },
    tool("after"),
    { event: "message_chunk", content: { type: "text", text: "Notice reply" } },
    { event: "turn_ended", reason: "end_turn" },
  ]);
  const hidden = model.entries.filter(isInternalInput);
  expect(hidden).toHaveLength(1);
  expect(hidden[0]?.seq).toBe(4);
  expect(model.turns).toBe(2);
  expect(model.inTurn).toBe(false);
  expect(model.turn).toBeNull();
  expect(model.next).toBe(8);
  expect(JSON.stringify(model.entries)).not.toContain(
    "private provider instruction",
  );
  const before = model.entries.find(
    (entry) => entry.kind === "tool" && entry.tool.title === "before",
  );
  const after = model.entries.find(
    (entry) => entry.kind === "tool" && entry.tool.title === "after",
  );
  const folds = toolRuns(model.entries);
  expect(before && folds.get(before.key)?.members).toEqual([before?.key]);
  expect(after && folds.get(after.key)?.members).toEqual([after?.key]);
  expect(outline(model.entries)).toEqual([
    { key: "r0", prompt: "Person", reply: "First reply" },
  ]);
  expect(rowsOf(model.entries, null, new Set()).some(isInternalInput)).toBe(
    false,
  );
  expect(hidden.map(spoken)).toEqual([""]);
  expect(hidden.map(searchable)).toEqual([null]);
  expect(
    model.entries
      .filter((entry) => entry.kind === "notification")
      .map((entry) => entry.seq),
  ).toEqual([2]);
});

test("tagged steer and chunk disappear without hiding later untagged user words", () => {
  const model = fold([
    {
      event: "turn_started",
      content: [{ type: "text", text: "internal start" }],
      provenance,
    },
    {
      event: "user_message_chunk",
      content: { type: "text", text: "internal chunk" },
      provenance,
    },
    {
      event: "user_message_chunk",
      content: { type: "text", text: "foreign visible" },
    },
    {
      event: "steered",
      content: [{ type: "text", text: "internal steer" }],
      provenance,
    },
    { event: "steered", content: [{ type: "text", text: "ordinary steer" }] },
  ]);
  expect(model.turns).toBe(1);
  expect(model.turn).toBe(0);
  expect(model.inTurn).toBe(true);
  expect(model.entries.filter(isInternalInput)).toHaveLength(2);
  const visible = rowsOf(model.entries, toolRuns(model.entries), new Set());
  expect(visible.map(spoken)).toEqual([
    "Prompt: foreign visible",
    "Steered: ordinary steer",
  ]);
  expect(JSON.stringify(model.entries)).not.toContain("internal start");
  expect(JSON.stringify(model.entries)).not.toContain("internal chunk");
  expect(JSON.stringify(model.entries)).not.toContain("internal steer");
});
