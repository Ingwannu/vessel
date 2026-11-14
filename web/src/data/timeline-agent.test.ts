import { expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type { AgentEvent, EventRecord, SessionEvent } from "../wire/proto";
import { reduce } from "./timeline";
import { EMPTY } from "./timeline-model";

const at = "2026-11-12T12:00:00Z";

function records(events: readonly SessionEvent[]): EventRecord[] {
  return events.map((event, seq) => ({ seq, at, event }));
}

function agent(event: AgentEvent): SessionEvent {
  return { type: "agent", event };
}

const Q = fixtureId("QuestionId", "q-1");

test("a question is an entry that becomes its outcome, in place", () => {
  const asked = reduce(
    EMPTY,
    records([
      agent({ event: "turn_started", content: [{ type: "text", text: "go" }] }),
      agent({ event: "question_asked", id: Q, message: "Which?", fields: [] }),
    ]),
  );
  expect(asked.entries.map((entry) => entry.kind)).toEqual([
    "prompt",
    "question",
  ]);
  expect(asked.attention).toBe(1);
  expect(asked.turn).toBe(0);
  const settled = reduce(asked, [
    {
      seq: 2,
      at,
      event: agent({
        event: "question_resolved",
        id: Q,
        outcome: { outcome: "declined" },
      }),
    },
  ]);
  expect(settled.entries).toHaveLength(2);
  const entry = settled.entries[1];
  expect(entry?.kind === "question" ? entry.outcome : null).toEqual({
    outcome: "declined",
  });
  // The prompt's entry is the same object: only what changed renders.
  expect(settled.entries[0]).toBe(asked.entries[0]);
});

test("the turn Stop names, and what asks for attention", () => {
  const ended = reduce(
    EMPTY,
    records([
      agent({ event: "turn_started", content: [] }),
      agent({ event: "message_chunk", content: { type: "text", text: "x" } }),
      agent({ event: "turn_ended", reason: "end_turn" }),
    ]),
  );
  expect(ended.turn).toBeNull();
  expect(ended.attention).toBe(2);
  const failed = reduce(ended, [
    {
      seq: 3,
      at,
      event: agent({ event: "turn_started", content: [] }),
    },
    { seq: 4, at, event: agent({ event: "turn_failed", message: "no" }) },
  ]);
  expect(failed.attention).toBe(4);
  expect(failed.turn).toBeNull();
});
