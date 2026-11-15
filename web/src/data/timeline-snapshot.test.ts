import { expect, test } from "vitest";
import type { Conversation, PermissionRequest } from "../wire/proto";
import { fixtureId } from "../testing/ids";
import { EMPTY } from "./timeline-model";
import { reconcileSnapshot } from "./timeline-snapshot";
import { reduce } from "./timeline";
const request: PermissionRequest = {
  id: fixtureId("PermissionId", "approval"),
  tool_call: fixtureId("ToolCallId", "actual"),
  options: [],
};
function snapshot(
  through: number | null,
  permissions: PermissionRequest[] = [],
): Conversation {
  return {
    through,
    permissions,
    questions: [],
    commands: [],
    queue: { entries: [] },
    usage: null,
    abilities: null,
    limits: {
      image: 1,
      audio: 1,
      file: 1,
      per_prompt: 1,
      inline_per_prompt: 1,
      per_session: 1,
      chunk: 1,
    },
  };
}
function records(events: Parameters<typeof reduce>[1][number]["event"][]) {
  return events.map((event, seq) => ({
    seq,
    at: "2026-11-15T00:00:00Z",
    event,
  }));
}
test("covered absence retires a card without a fabricated resolution outcome", () => {
  const timeline = reduce(
    EMPTY,
    records([
      { type: "agent", event: { event: "permission_requested", ...request } },
    ]),
  );
  const retired = reconcileSnapshot(timeline, snapshot(1));
  expect(retired.pending).toBeNull();
  expect(retired.entries[0]).toMatchObject({ available: false, outcome: null });
  expect(retired.next).toBe(timeline.next);
  expect(reconcileSnapshot(timeline, snapshot(null))).toBe(timeline);
});
test("stale and ahead-of-log snapshots cannot replace newer metadata", () => {
  const timeline = reduce(
    EMPTY,
    records([
      { type: "agent", event: { event: "permission_requested", ...request } },
      {
        type: "agent",
        event: {
          event: "permission_requested",
          ...request,
          delivery_state: "outcome_unknown",
        },
      },
    ]),
  );
  const reconciled = reconcileSnapshot(timeline, snapshot(1));
  expect(reconciled.pending?.request.delivery_state).toBe("outcome_unknown");
  expect(reconcileSnapshot(timeline, snapshot(3))).toBe(timeline);
});
test("a pruned announcement supplies only a dock request and real resolution retires it", () => {
  const timeline = reconcileSnapshot(
    { ...EMPTY, next: 4 },
    snapshot(4, [request]),
  );
  expect(timeline.pending?.request.id).toBe(request.id);
  expect(timeline.entries).toEqual([]);
  const resolved = reduce(timeline, [
    {
      seq: 4,
      at: "2026-11-15T00:00:00Z",
      event: {
        type: "agent",
        event: {
          event: "permission_resolved",
          id: request.id,
          outcome: { outcome: "cancelled" },
        },
      },
    },
  ]);
  expect(resolved.pending).toBeNull();
});

test("read order and coverage cannot reopen admitted snapshot-only requests", () => {
  let timeline = reconcileSnapshot(
    { ...EMPTY, next: 8 },
    snapshot(4, [{ ...request, delivery_state: "answering" }]),
    1,
  );
  timeline = reconcileSnapshot(
    timeline,
    snapshot(4, [{ ...request, delivery_state: "outcome_unknown" }]),
    2,
  );
  expect(timeline.pending?.request.delivery_state).toBe("outcome_unknown");
  // A higher unrelated log fence does not revise retained delivery state.
  const newer = reconcileSnapshot(timeline, snapshot(6, [request]), 3);
  expect(newer.pending?.request.delivery_state).toBe("outcome_unknown");
  expect(reconcileSnapshot(newer, snapshot(7), 2)).toBe(newer);
  expect(newer.next).toBe(8);
  expect(newer.entries).toEqual([]);
});

test("question delivery stays inert across newer pending snapshots until a real refresh", () => {
  const question = {
    id: fixtureId("QuestionId", "question"),
    message: "Choose",
    fields: [],
    delivery_state: "answering" as const,
  };
  const first = { ...snapshot(2), questions: [question] };
  let timeline = reconcileSnapshot({ ...EMPTY, next: 5 }, first, 1);
  timeline = reconcileSnapshot(
    timeline,
    {
      ...snapshot(4),
      questions: [{ ...question, delivery_state: "pending" }],
    },
    2,
  );
  expect(timeline.conversation.questions[0]?.question.delivery_state).toBe(
    "answering",
  );
  const refreshed = reduce(timeline, [
    {
      seq: 5,
      at: "2026-11-15T00:00:00Z",
      event: {
        type: "agent",
        event: {
          event: "question_asked",
          ...question,
          delivery_state: "pending",
        },
      },
    },
  ]);
  expect(refreshed.conversation.questions[0]?.question.delivery_state).toBe(
    "pending",
  );
});
