import { describe, expect, it } from "vitest";

import type {
  AgentEvent,
  EventRecord,
  PromptQueue,
  Question,
  SessionEvent,
} from "../wire/proto";
import { EMPTY_CONVERSATION, waitingQuestions } from "./conversation";
import { reduce } from "./timeline";
import { EMPTY } from "./timeline-model";

const at = "2026-11-11T12:00:00Z";

function records(events: readonly SessionEvent[]): EventRecord[] {
  return events.map((event, seq) => ({ seq, at, event }));
}

function agent(event: AgentEvent): SessionEvent {
  return { type: "agent", event };
}

const QUESTION: Question = {
  id: "q-1",
  message: "Which approach?",
  fields: [
    {
      name: "question_1",
      required: true,
      kind: "choice",
      options: [{ value: "a", title: "Rewrite" }],
      multiple: false,
    },
  ],
};

const HELD: PromptQueue = {
  entries: [
    {
      id: "e-1",
      content: [{ type: "text", text: "next" }],
      queued_at: at,
      revision: 0,
      by: { kind: "owner" },
    },
  ],
  held: "cancelled",
};

describe("the conversation's model", () => {
  it("keeps the last queue, commands and context the node logged", () => {
    const timeline = reduce(
      EMPTY,
      records([
        { type: "queue_changed", queue: { entries: [] } },
        {
          type: "commands_changed",
          commands: [{ name: "compact", description: "Compact" }],
        },
        { type: "usage_changed", usage: { used: 10, size: 100 } },
        { type: "queue_changed", queue: HELD },
        { type: "usage_changed", usage: { used: 40, size: 100 } },
      ]),
    );
    expect(timeline.conversation.queue).toEqual(HELD);
    expect(timeline.conversation.commands).toEqual([
      { name: "compact", description: "Compact" },
    ]);
    expect(timeline.conversation.usage).toEqual({ used: 40, size: 100 });
    // None of them is a timeline entry.
    expect(timeline.entries).toEqual([]);
    expect(timeline.next).toBe(5);
  });

  it("follows a question from asked to answered", () => {
    const asked = reduce(
      EMPTY,
      records([agent({ event: "question_asked", ...QUESTION })]),
    );
    expect(waitingQuestions(asked.conversation)).toEqual([
      { seq: 0, question: QUESTION, outcome: null },
    ]);
    const answered = reduce(asked, [
      {
        seq: 1,
        at,
        event: agent({
          event: "question_resolved",
          id: "q-1",
          outcome: {
            outcome: "answered",
            values: { question_1: { type: "choice", value: "a" } },
          },
        }),
      },
    ]);
    expect(waitingQuestions(answered.conversation)).toEqual([]);
    expect(answered.conversation.questions[0]?.outcome?.outcome).toBe(
      "answered",
    );
  });

  it("keeps steers, the agent's abilities and a turn's tokens", () => {
    const timeline = reduce(
      EMPTY,
      records([
        agent({
          event: "ready",
          harness_session: "s",
          abilities: { steer: true, resume: true, image: true, audio: false },
        }),
        agent({ event: "turn_started", content: [] }),
        agent({
          event: "steered",
          content: [{ type: "text", text: "also the tests" }],
        }),
        agent({
          event: "turn_ended",
          reason: "end_turn",
          usage: { total: 30, input: 20, output: 10 },
        }),
      ]),
    );
    const conversation = timeline.conversation;
    expect(conversation.abilities?.steer).toBe(true);
    expect(conversation.steers).toEqual([
      { seq: 2, at, content: [{ type: "text", text: "also the tests" }] },
    ]);
    expect(conversation.turnUsage).toEqual({
      total: 30,
      input: 20,
      output: 10,
    });
    // The timeline still has the turn, and the steer is not one of its
    // entries yet (drawing it is the next track's).
    expect(timeline.entries.map((entry) => entry.kind)).toEqual([
      "ready",
      "prompt",
      "turn_ended",
    ]);
  });

  it("starts empty, and names an attachment in a prompt's text", () => {
    expect(EMPTY.conversation).toEqual(EMPTY_CONVERSATION);
    const timeline = reduce(
      EMPTY,
      records([
        agent({
          event: "turn_started",
          content: [
            { type: "text", text: "look at " },
            {
              type: "attachment",
              attachment: {
                id: "a-1",
                name: "shot.png",
                kind: "image",
                mime: "image/png",
                size: 3,
              },
            },
          ],
        }),
      ]),
    );
    const prompt = timeline.entries[0];
    expect(prompt?.kind === "prompt" ? prompt.text : null).toBe(
      "look at [shot.png]",
    );
  });
});
