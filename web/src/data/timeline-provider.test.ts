import { describe, expect, it } from "vitest";

import { fixtureId } from "../testing/ids";
import type { AgentEvent, EventRecord } from "../wire/proto";
import { reduce } from "./timeline";
import { EMPTY } from "./timeline-model";

const at = "2026-11-07T12:00:00Z";

function agent(seq: number, event: AgentEvent): EventRecord {
  return { seq, at, event: { type: "agent", event } };
}

describe("provider child observations in a parent log", () => {
  it("advances the parent cursor without changing its conversation or turn", () => {
    const parent = reduce(EMPTY, [
      agent(0, {
        event: "turn_started",
        content: [{ type: "text", text: "parent prompt" }],
      }),
      agent(1, {
        event: "message_chunk",
        content: { type: "text", text: "parent answer" },
      }),
      agent(2, {
        event: "plan",
        entries: [
          { content: "parent plan", priority: "high", status: "in_progress" },
        ],
      }),
    ]);
    const child = fixtureId("ProviderChildId", "actual-provider-task");
    const activities: AgentEvent[] = [
      {
        event: "turn_started",
        content: [{ type: "text", text: "child prompt" }],
      },
      {
        event: "message_chunk",
        content: { type: "text", text: "child answer" },
      },
      { event: "plan", entries: [] },
      { event: "turn_ended", reason: "end_turn" },
      { event: "turn_failed", message: "child failure" },
    ];
    const observations: AgentEvent[] = [
      { event: "provider_child", kind: "supported" },
      {
        event: "provider_child",
        kind: "descriptor",
        descriptor: { id: child },
      },
      ...activities.map((activity): AgentEvent => ({
        event: "provider_child",
        kind: "activity",
        child,
        activity,
      })),
      { event: "provider_child", kind: "gap", reason: "ingestion_budget" },
    ];
    const records = observations.map((event, index) =>
      agent(parent.next + index, event),
    );
    const folded = reduce(parent, records);
    expect(folded.next).toBe(parent.next + observations.length);
    expect({ ...folded, next: parent.next }).toEqual(parent);
    expect(reduce(folded, records)).toBe(folded);

    const continued = reduce(folded, [
      agent(folded.next, {
        event: "turn_ended",
        reason: "end_turn",
      }),
    ]);
    expect(continued.inTurn).toBe(false);
    expect(continued.attention).toBe(folded.next);
    expect(continued.entries.at(-1)?.kind).toBe("turn_ended");
  });
});
