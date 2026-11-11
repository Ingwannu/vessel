import { expect, test } from "vitest";

import { outline, preview } from "./outline";
import type { Entry } from "./timeline-model";

const prompt = (seq: number, text: string): Entry => ({
  kind: "prompt",
  key: `p${String(seq)}`,
  seq,
  at: "",
  text,
  replayed: false,
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
