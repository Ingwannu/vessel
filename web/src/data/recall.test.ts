import { expect, test } from "vitest";

import { type Caret, recall, sentPrompts } from "./recall";
import type { Entry } from "./timeline-model";

function prompt(seq: number, text: string): Entry {
  return {
    kind: "prompt",
    key: `p${String(seq)}`,
    seq,
    at: "",
    text,
    replayed: false,
    attachments: [],
    turn: seq,
  };
}

const HISTORY = sentPrompts([
  prompt(1, "first"),
  { kind: "message", key: "m2", seq: 2, at: "", text: "reply" },
  prompt(3, "second\nline two"),
  prompt(4, "second\nline two"),
  prompt(5, "third"),
]);

function at(text: string, caret = 0): Caret {
  return { text, start: caret, end: caret };
}

test("the session's prompts, newest first, repeats as one", () => {
  expect(HISTORY).toEqual(["third", "second\nline two", "first"]);
});

test("Up in an empty composer shows the newest; Up again goes older", () => {
  const first = recall("up", at(""), null, HISTORY);
  expect(first).toEqual({
    kind: "show",
    text: "third",
    walk: { index: 0, shown: "third" },
  });
  if (first.kind !== "show") return;
  const second = recall("up", at("third"), first.walk, HISTORY);
  expect(second).toMatchObject({ text: "second\nline two" });
});

test("in a multi-line prompt the arrows move the caret until an edge line", () => {
  const walk = { index: 1, shown: "second\nline two" };
  expect(recall("up", at("second\nline two", 9), walk, HISTORY)).toEqual({
    kind: "pass",
  });
  expect(recall("up", at("second\nline two", 3), walk, HISTORY)).toMatchObject({
    text: "first",
  });
  expect(recall("down", at("second\nline two", 3), walk, HISTORY)).toEqual({
    kind: "pass",
  });
  expect(
    recall("down", at("second\nline two", 9), walk, HISTORY),
  ).toMatchObject({ text: "third" });
});

test("Down past the newest empties the composer; Up past the oldest does nothing", () => {
  expect(
    recall("down", at("third"), { index: 0, shown: "third" }, HISTORY),
  ).toEqual({ kind: "show", text: "", walk: null });
  expect(
    recall("up", at("first"), { index: 2, shown: "first" }, HISTORY),
  ).toEqual({ kind: "pass" });
});

test("text of one's own, or an edited recall, is not a walk", () => {
  expect(recall("up", at("draft"), null, HISTORY)).toEqual({ kind: "pass" });
  expect(
    recall("up", at("third!"), { index: 0, shown: "third" }, HISTORY),
  ).toEqual({ kind: "pass" });
  expect(recall("down", at(""), null, HISTORY)).toEqual({ kind: "pass" });
  expect(recall("up", { text: "", start: 0, end: 0 }, null, [])).toEqual({
    kind: "pass",
  });
});
