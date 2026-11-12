import { expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type { AttachmentRef, PromptQueue, QueuedPrompt } from "../wire/proto";
import { heldBy, holdLine, queueRows, withText } from "./queue-view";

const IMAGE: AttachmentRef = {
  id: fixtureId("AttachmentId", "a1"),
  name: "shot.png",
  kind: "image",
  mime: "image/png",
  size: 2048,
};

function entry(id: string, text: string): QueuedPrompt {
  return {
    id: fixtureId("QueueEntryId", id),
    content: [{ type: "text", text }],
    queued_at: "2026-11-12T00:00:00Z",
    revision: 1,
    by: { kind: "owner" },
  };
}

test("rows know their neighbours, so a move names the entry to go before", () => {
  const queue: PromptQueue = {
    entries: [entry("e1", "one"), entry("e2", "two"), entry("e3", "three")],
  };
  const rows = queueRows(queue);
  expect(rows.map((row) => row.text)).toEqual(["one", "two", "three"]);
  expect(rows[0]).toMatchObject({ first: true, before: null });
  expect(rows[1]?.before).toBe(fixtureId("QueueEntryId", "e1"));
  // Moving the first down puts it before the third.
  expect(rows[0]?.afterNext).toBe(fixtureId("QueueEntryId", "e3"));
  // Moving the second down puts it last.
  expect(rows[1]).toMatchObject({ afterNext: null, last: false });
  expect(rows[2]?.last).toBe(true);
  expect(queueRows(null)).toEqual([]);
});

test("an edit replaces the words and keeps the files", () => {
  const content = [
    { type: "text" as const, text: "old" },
    { type: "attachment" as const, attachment: IMAGE },
  ];
  expect(withText(content, "new")).toEqual([
    { type: "text", text: "new" },
    { type: "attachment", attachment: IMAGE },
  ]);
  expect(withText(content, "")).toEqual([
    { type: "attachment", attachment: IMAGE },
  ]);
  const rows = queueRows({
    entries: [{ ...entry("e1", "x"), content, by: { kind: "owner" } }],
  });
  expect(rows[0]?.attachments).toEqual([IMAGE]);
  expect(rows[0]?.text).toBe("old");
});

test("a held queue says why", () => {
  expect(heldBy({ entries: [] })).toBeNull();
  expect(heldBy({ entries: [], held: "cancelled" })).toBe("cancelled");
  expect(holdLine("cancelled")).toBe("Paused after you stopped the agent");
  expect(holdLine("ended")).toBe("Paused: the session ended");
});
