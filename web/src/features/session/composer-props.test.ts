import { expect, test } from "vitest";

import { liveSession } from "../../data/fixtures/machines";
import { reduce } from "../../data/timeline";
import { EMPTY, type Timeline } from "../../data/timeline-model";
import { fixtureId } from "../../testing/ids";
import type { NodeKey } from "../../wire/bridge";
import type { AgentEvent, EventRecord, SessionInfo } from "../../wire/proto";
import { type ComposerProps, sameComposerProps } from "./composer-props";

const NODE: NodeKey = fixtureId("NodeKey", "laptop");
const SESSION: SessionInfo = liveSession;

let seq = 0;
function agent(event: AgentEvent): EventRecord {
  const record: EventRecord = {
    seq,
    at: "2026-11-12T12:00:00Z",
    event: { type: "agent", event },
  };
  seq += 1;
  return record;
}

function props(timeline: Timeline): ComposerProps {
  return { node: NODE, label: "laptop", session: SESSION, timeline };
}

const started = reduce(EMPTY, [
  agent({ event: "turn_started", content: [{ type: "text", text: "fix it" }] }),
]);

test("a frame of the agent's words does not draw the composer again", () => {
  const more = reduce(started, [
    agent({ event: "message_chunk", content: { type: "text", text: "on it" } }),
    agent({ event: "thought_chunk", content: { type: "text", text: "hm" } }),
  ]);
  expect(more).not.toBe(started);
  expect(sameComposerProps(props(started), props(more))).toBe(true);
});

test("a new prompt, a replayed one, the turn's end and options do", () => {
  const prompt = reduce(started, [
    agent({ event: "turn_ended", reason: "end_turn" }),
  ]);
  expect(sameComposerProps(props(started), props(prompt))).toBe(false);
  const replayed = reduce(prompt, [
    agent({
      event: "user_message_chunk",
      content: { type: "text", text: "an earlier prompt" },
    }),
  ]);
  expect(sameComposerProps(props(prompt), props(replayed))).toBe(false);
  const options = reduce(replayed, [
    {
      seq: replayed.next,
      at: "2026-11-12T12:00:00Z",
      event: { type: "options_changed", options: [] },
    },
  ]);
  expect(sameComposerProps(props(replayed), props(options))).toBe(false);
});

test("a record after which the login may be another does", () => {
  const woken = reduce(started, [
    { seq: started.next, at: "2026-11-12T12:00:00Z", event: { type: "woken" } },
  ]);
  expect(woken.binding).toBe(started.next);
  expect(sameComposerProps(props(started), props(woken))).toBe(false);
});

test("another session or machine draws it again", () => {
  expect(
    sameComposerProps(props(started), { ...props(started), label: "gpu-box" }),
  ).toBe(false);
  expect(
    sameComposerProps(props(started), {
      ...props(started),
      session: { ...SESSION },
    }),
  ).toBe(false);
});
