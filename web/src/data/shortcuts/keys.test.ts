import { describe, expect, test } from "vitest";

import {
  bindingText,
  bindingWords,
  chordCaps,
  chordOf,
  type KeyPress,
  overlaps,
  parseBinding,
} from "./keys";
import { platformOf } from "./platform";

function press(key: string, extra: Partial<KeyPress> = {}): KeyPress {
  return {
    key,
    code: "",
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    ...extra,
  };
}

describe("the grammar the bridge keeps", () => {
  test("bindings that parse, and come back as written", () => {
    for (const text of [
      "Mod+K",
      "Alt+[",
      "G S",
      "/",
      "Mod+Shift+B",
      "Ctrl+Alt+Shift+Meta+F12",
      "Mod+Plus",
    ]) {
      const binding = parseBinding(text);
      expect(binding, text).not.toBeNull();
      if (binding !== null) expect(bindingText(binding)).toBe(text);
    }
  });

  test("text that is not a binding", () => {
    for (const text of [
      "",
      "k",
      "Shift+Mod+K",
      "Mod+",
      "Mod+Mod+K",
      "A B C",
      "Mod+K ",
      "Hyper+K",
      "x".repeat(40),
    ]) {
      expect(parseBinding(text), text).toBeNull();
    }
  });
});

describe("a key press as a chord", () => {
  test("Command is Mod on a Mac; Control is Mod elsewhere", () => {
    expect(chordOf(press("k", { metaKey: true }), "mac")).toMatchObject({
      mod: true,
      key: "K",
    });
    expect(chordOf(press("k", { ctrlKey: true }), "mac")).toMatchObject({
      mod: false,
      ctrl: true,
    });
    expect(chordOf(press("k", { ctrlKey: true }), "other")).toMatchObject({
      mod: true,
      ctrl: false,
    });
  });

  test("Option's character is not the key: its position is", () => {
    expect(
      chordOf(press("“", { altKey: true, code: "BracketLeft" }), "mac")?.key,
    ).toBe("[");
    expect(
      chordOf(press("¡", { altKey: true, code: "Digit1" }), "mac")?.key,
    ).toBe("1");
    expect(
      chordOf(press("å", { altKey: true, code: "KeyA" }), "mac")?.key,
    ).toBe("A");
  });

  test("a sign carries its Shift; a letter keeps it; a modifier alone is nothing", () => {
    expect(chordOf(press("?", { shiftKey: true }), "other")).toMatchObject({
      key: "?",
      shift: false,
    });
    expect(chordOf(press("A", { shiftKey: true }), "other")).toMatchObject({
      key: "A",
      shift: true,
    });
    expect(chordOf(press("Shift", { shiftKey: true }), "other")).toBeNull();
    expect(chordOf(press("ArrowUp"), "other")?.key).toBe("Up");
  });
});

describe("shown as each platform writes it", () => {
  const binding = parseBinding("Mod+Shift+B");
  test("glyphs on a Mac, names elsewhere", () => {
    if (binding === null) throw new Error("no binding");
    expect(chordCaps(binding[0], "mac")).toEqual(["⇧", "⌘", "B"]);
    expect(chordCaps(binding[0], "other")).toEqual(["Ctrl", "Shift", "B"]);
    expect(bindingWords(binding, "other")).toBe("Ctrl+Shift+B");
    const sequence = parseBinding("G S");
    if (sequence === null) throw new Error("no sequence");
    expect(bindingWords(sequence, "mac")).toBe("G then S");
  });

  test("platforms", () => {
    expect(platformOf("MacIntel", "")).toBe("mac");
    expect(platformOf("Linux x86_64", "Mozilla/5.0 (X11; Linux)")).toBe(
      "other",
    );
  });
});

test("a sequence overlaps the key it starts with", () => {
  const g = parseBinding("G");
  const gs = parseBinding("G S");
  const gm = parseBinding("G M");
  if (g === null || gs === null || gm === null) throw new Error("no binding");
  expect(overlaps(g, gs)).toBe(true);
  expect(overlaps(gs, gm)).toBe(false);
});
