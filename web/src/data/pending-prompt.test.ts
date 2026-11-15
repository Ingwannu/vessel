import { expect, test } from "vitest";
import { fixtureId } from "../testing/ids";
import type { AgentEvent, AttachmentRef } from "../wire/proto";
import { reduce } from "./timeline";
import { EMPTY } from "./timeline-model";
import {
  pendingPromptKey,
  visiblePendingPrompt,
  type PendingPrompt,
} from "./pending-prompt";

const at = "2026-11-15T00:00:00Z";
const message = fixtureId("ClientMessageId", "mine");
const file: AttachmentRef = {
  id: fixtureId("AttachmentId", "file"),
  name: "image.png",
  kind: "image",
  mime: "image/png",
  size: 3,
};
const pending: PendingPrompt = {
  message,
  text: "same",
  attachments: [file],
  at,
  state: "uncertain",
};
function fold(event: AgentEvent) {
  return reduce(EMPTY, [{ seq: 17, at, event: { type: "agent", event } }]);
}
function prompt(client = message): AgentEvent {
  return {
    event: "turn_started",
    client_message_id: client,
    content: [
      { type: "text", text: "same" },
      { type: "attachment", attachment: file },
    ],
  };
}

test("exact keyed prompt and steer echoes hide only their own local overlay", () => {
  for (const event of [
    prompt(),
    {
      event: "steered",
      client_message_id: message,
      content: [
        { type: "text", text: "same" },
        { type: "attachment", attachment: file },
      ],
    } satisfies AgentEvent,
  ]) {
    const timeline = fold(event);
    expect(visiblePendingPrompt(timeline, pending)).toBeUndefined();
    expect(pending.state).toBe("uncertain");
    expect(timeline.next).toBe(18);
  }
  expect(pendingPromptKey(pending)).toBe("pending:mine");
});

test("legacy, another identity and conflicting same-id bodies never consume an echo", () => {
  expect(
    visiblePendingPrompt(
      fold({
        event: "turn_started",
        content: [
          { type: "text", text: "same" },
          { type: "attachment", attachment: file },
        ],
      }),
      pending,
    ),
  ).toBe(pending);
  expect(
    visiblePendingPrompt(
      fold(prompt(fixtureId("ClientMessageId", "other"))),
      pending,
    ),
  ).toBe(pending);
  expect(
    visiblePendingPrompt(fold(prompt()), { ...pending, text: "different" }),
  ).toEqual({ ...pending, text: "different" });
  const changed = {
    ...pending,
    attachments: [{ ...file, name: "different.png" }],
  };
  expect(visiblePendingPrompt(fold(prompt()), changed)).toBe(changed);
});

test("native command echo never reconciles a prompt or fabricates a turn", () => {
  const timeline = fold({
    event: "command_accepted",
    client_message_id: message,
    content: [{ type: "text", text: "same" }],
    status: "compact_started",
  });
  expect(visiblePendingPrompt(timeline, pending)).toBe(pending);
  expect(timeline.turns).toBe(0);
  expect(timeline.inTurn).toBe(false);
});
