import { describe, expect, test } from "vitest";

import {
  assign,
  clashOf,
  clashText,
  refusal,
  removeKey,
  restore,
} from "./edit";
import { type Binding, parseBinding } from "./keys";

function binding(text: string): Binding {
  const parsed = parseBinding(text);
  if (parsed === null) throw new Error(text);
  return parsed;
}

describe("what a recorded key may be", () => {
  test("a modifier is needed, and the browser's own keys are refused", () => {
    expect(refusal(binding("K")[0], "other")).toBe(
      "Use Ctrl or Alt with another key.",
    );
    expect(refusal(binding("Mod+T")[0], "mac")).toBe(
      "The browser keeps that shortcut for itself.",
    );
    expect(refusal(binding("Mod+2")[0], "other")).toBe(
      "The browser keeps that shortcut for itself.",
    );
    expect(refusal(binding("Mod+C")[0], "other")).toBe(
      "That shortcut is reserved for standard editing commands.",
    );
    expect(refusal(binding("Mod+Shift+K")[0], "other")).toBeNull();
  });
});

describe("clashes", () => {
  test("with another action, with a fixed one, with itself", () => {
    expect(clashOf("find", binding("Mod+K"), {})).toEqual({
      kind: "taken",
      action: "palette",
    });
    expect(clashOf("find", binding("Alt+4"), {})).toEqual({
      kind: "fixed",
      action: "jump-session",
    });
    expect(clashOf("find", binding("Mod+F"), {})).toEqual({
      kind: "same",
      action: "find",
    });
    expect(clashOf("find", binding("G"), {})).toEqual({
      kind: "taken",
      action: "go-board",
    });
    expect(clashOf("find", binding("Mod+J"), {})).toBeNull();
  });

  test("the sentences", () => {
    expect(
      clashText({ kind: "taken", action: "palette" }, "Ctrl+K", "find"),
    ).toBe(
      "Ctrl+K is assigned to Open the command palette. Reassign it to Find in the session?",
    );
  });
});

describe("edits", () => {
  test("a reassignment moves the key in one change", () => {
    expect(assign("find", binding("Mod+K"), {}, 0, "palette")).toEqual([
      { action: "find", to: { kind: "bindings", keys: ["Mod+K"] } },
      { action: "palette", to: { kind: "bindings", keys: ["/"] } },
    ]);
  });

  test("another key, at most two; removing one; the defaults again", () => {
    expect(assign("find", binding("Mod+J"), {}, null, null)).toEqual([
      { action: "find", to: { kind: "bindings", keys: ["Mod+F", "Mod+J"] } },
    ]);
    expect(assign("palette", binding("Mod+J"), {}, null, null)[0]?.to).toEqual({
      kind: "bindings",
      keys: ["/", "Mod+J"],
    });
    expect(removeKey("palette", 0, {})).toEqual([
      { action: "palette", to: { kind: "bindings", keys: ["/"] } },
    ]);
    expect(restore("find")).toEqual([
      { action: "find", to: { kind: "default" } },
    ]);
  });
});
