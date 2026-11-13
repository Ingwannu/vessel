import { expect, test } from "vitest";

import { outline, preview, sameOutline } from "./outline";
import type { Entry } from "./timeline-model";

const prompt = (seq: number, text: string): Entry => ({
  kind: "prompt",
  key: `p${String(seq)}`,
  seq,
  at: "",
  text,
  replayed: false,
  attachments: [],
  turn: seq,
});
const message = (seq: number, text: string): Entry => ({
  kind: "message",
  key: `m${String(seq)}`,
  seq,
  at: "",
  text,
});

test("one item per prompt, with the turn's last reply", () => {
  expect(
    outline([
      prompt(1, "Fix  the\nbuild"),
      message(2, "Looking."),
      message(3, "**Done**: it builds."),
      prompt(4, "Now the tests"),
    ]),
  ).toEqual([
    { key: "p1", prompt: "Fix the build", reply: "Done: it builds." },
    { key: "p4", prompt: "Now the tests", reply: null },
  ]);
});

test("a long prompt is cut with an ellipsis", () => {
  const cut = preview("x".repeat(200), 120);
  expect(Array.from(cut)).toHaveLength(120);
  expect(cut.endsWith("…")).toBe(true);
  expect(preview("short", 120)).toBe("short");
});

test("a reply is read once per entry, and an unchanged outline is the same", () => {
  const reply = message(2, "**Done**: it builds.");
  const entries = [prompt(1, "Fix the build"), message(9, "Looking."), reply];
  const first = outline(entries);
  // The same entries again, one more record after them: the same outline.
  const again = outline([...entries, message(3, "**Done**: it builds.")]);
  expect(sameOutline(first, outline(entries))).toBe(true);
  expect(again[0]?.reply).toBe("Done: it builds.");
  expect(sameOutline(first, again)).toBe(true);
  expect(sameOutline(first, outline([...entries, prompt(4, "More")]))).toBe(
    false,
  );
  expect(
    sameOutline(
      first,
      outline([prompt(1, "Fix the build"), message(5, "No.")]),
    ),
  ).toBe(false);
});
