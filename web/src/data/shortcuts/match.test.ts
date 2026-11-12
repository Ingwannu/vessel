import { expect, test } from "vitest";

import { type Chord, parseBinding } from "./keys";
import { match, SEQUENCE_MS } from "./match";

function chord(text: string): Chord {
  const binding = parseBinding(text);
  if (binding === null) throw new Error(text);
  return binding[0];
}

test("a chord runs its action", () => {
  expect(match(chord("Mod+K"), null, {}, 0, "other")).toEqual({
    kind: "run",
    action: "palette",
    index: null,
  });
  expect(match(chord("/"), null, {}, 0, "other")).toMatchObject({
    action: "palette",
  });
});

test("a sequence waits for its second key, within the time", () => {
  const first = match(chord("G"), null, {}, 100, "other");
  expect(first.kind).toBe("wait");
  if (first.kind !== "wait") return;
  expect(
    match(chord("S"), first.pending, {}, 100 + SEQUENCE_MS, "other"),
  ).toMatchObject({ action: "go-board" });
  expect(
    match(chord("S"), first.pending, {}, 101 + SEQUENCE_MS, "other"),
  ).toEqual({
    kind: "none",
  });
});

test("Alt and a digit go to that session", () => {
  expect(match(chord("Alt+3"), null, {}, 0, "other")).toEqual({
    kind: "run",
    action: "jump-session",
    index: 3,
  });
  expect(match(chord("Alt+0"), null, {}, 0, "other")).toEqual({ kind: "none" });
});

test("an override replaces the defaults; an empty one unsets", () => {
  expect(
    match(chord("Mod+J"), null, { find: ["Mod+J"] }, 0, "other"),
  ).toMatchObject({
    action: "find",
  });
  expect(match(chord("Mod+F"), null, { find: ["Mod+J"] }, 0, "other")).toEqual({
    kind: "none",
  });
  expect(
    match(chord("Mod+B"), null, { "toggle-sidebar": [] }, 0, "other"),
  ).toEqual({
    kind: "none",
  });
});
