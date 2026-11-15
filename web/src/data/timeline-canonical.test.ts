import { expect, test } from "vitest";
import { fixtureId } from "../testing/ids";
import type { AgentEvent, ToolCallScope } from "../wire/proto";
import { reduce } from "./timeline";
import { EMPTY } from "./timeline-model";
import { toolKey } from "./timeline-tool";
import { waitingQuestions } from "./conversation";

function fold(events: AgentEvent[]) {
  return reduce(
    EMPTY,
    events.map((event, seq) => ({
      seq,
      at: "2026-11-15T00:00:00Z",
      event: { type: "agent" as const, event },
    })),
  );
}
const a: ToolCallScope = { kind: "session_turn", started: 1 };
const b: ToolCallScope = { kind: "session_turn", started: 2 };
test("same native id with late A update cannot change B or its permission context", () => {
  const id = fixtureId("ToolCallId", "same");
  const call = {
    event: "tool_call",
    id,
    title: "A",
    kind: "execute",
    status: "pending",
    content: [],
    locations: [],
  } as const;
  const model = fold([
    {
      ...call,
      scope: a,
      content: [],
      locations: [],
      detail: { type: "shell", command: "A" },
    },
    {
      ...call,
      title: "B",
      scope: b,
      content: [],
      locations: [],
      detail: { type: "shell", command: "B" },
    },
    {
      event: "tool_call_update",
      id,
      scope: a,
      title: "late A",
      detail: { type: "unknown" },
    },
  ]);
  const first = model.entries[model.tools.get(toolKey(id, a)) ?? -1];
  const second = model.entries[model.tools.get(toolKey(id, b)) ?? -1];
  expect(first?.kind === "tool" ? first.tool.title : null).toBe("late A");
  expect(first?.kind === "tool" ? first.tool.detail : null).toEqual({
    type: "shell",
    command: "A",
  });
  expect(second?.kind === "tool" ? second.tool.title : null).toBe("B");
  expect(toolKey(toolKey(id, a))).not.toBe(toolKey(id, a));
});
test("compaction resolves earliest outstanding marker across intervening rows", () => {
  const model = fold([
    { event: "compaction", status: "loading" },
    { event: "message_chunk", content: { type: "text", text: "work" } },
    { event: "compaction", status: "loading" },
    {
      event: "compaction",
      status: "completed",
      trigger: "manual",
      pre_tokens: 123,
    },
  ]);
  expect(model.entries.map((entry) => entry.kind)).toEqual([
    "compaction",
    "message",
    "compaction",
  ]);
  expect(model.entries[0]).toMatchObject({
    seq: 0,
    item: { status: "completed", trigger: "manual", pre_tokens: 123 },
  });
  expect(model.entries[2]).toMatchObject({ item: { status: "loading" } });
});
test("session question survives turn end and pending-state replay updates one card", () => {
  const id = fixtureId("QuestionId", "question");
  const model = fold([
    {
      event: "question_asked",
      id,
      message: "choose",
      fields: [],
      lifetime: "session",
    },
    { event: "turn_ended", reason: "end_turn" },
    {
      event: "question_asked",
      id,
      message: "choose",
      fields: [],
      lifetime: "session",
      delivery_state: "outcome_unknown",
    },
  ]);
  expect(
    model.entries.filter((entry) => entry.kind === "question"),
  ).toHaveLength(1);
  expect(waitingQuestions(model.conversation)).toHaveLength(1);
  expect(waitingQuestions(model.conversation)[0]?.question.delivery_state).toBe(
    "outcome_unknown",
  );
});

test("unrecognised permission resolution never claims an allowance", async () => {
  const { settled } = await import("./approval");
  expect(
    settled([], {
      outcome: "selected",
      option: fixtureId("PermissionOptionId", "unoffered"),
    }).state,
  ).toBe("unknown");
});

test("command acceptance retains caller identity without changing the running turn", () => {
  const message = fixtureId("ClientMessageId", "command");
  const model = fold([
    {
      event: "command_accepted",
      client_message_id: message,
      content: [{ type: "text", text: "/compact" }],
      status: "compact_started",
    },
  ]);
  expect(model.turns).toBe(0);
  expect(model.inTurn).toBe(false);
  expect(model.turn).toBeNull();
  const entry = model.entries[0];
  expect(entry?.kind).toBe("command");
  if (entry?.kind !== "command") throw new Error("command echo");
  expect(entry.item.client_message_id).toBe(message);
  expect(entry.text).toBe("/compact");
});
