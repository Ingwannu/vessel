import { describe, expect, test } from "vitest";

import {
  type Binding,
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

function binding(text: string): Binding {
  const parsed = parseBinding(text);
  if (parsed === null) throw new Error(text);
  return parsed;
}

describe("the grammar the bridge keeps", () => {
  test("bindings that parse, and come back as written", () => {
    const accepted = [
      "Mod+K",
      "Alt+[",
      "G S",
      "/",
      "Shift+/",
      "Mod+Shift+B",
      "Ctrl+Alt+Shift+Meta+F12",
      "Mod+=",
      "Mod+\\",
    ];
    for (const text of accepted) {
      expect(bindingText(binding(text))).toBe(text);
    }
  });

  test("text that is not a binding", () => {
    const refused = [
      "",
      "k",
      "?",
      "Plus",
      "Shift+Mod+K",
      "Mod+",
      "Mod+Mod+K",
      "A B C",
      "Mod+K ",
      "Hyper+K",
      "x".repeat(40),
    ];
    for (const text of refused) {
      expect(parseBinding(text), text).toBeNull();
    }
  });
});

describe("a key press as a chord", () => {
  test("Command is Mod on a Mac; Control is Mod elsewhere", () => {
    const k = { code: "KeyK" };
    expect(chordOf(press("k", { ...k, metaKey: true }), "mac")).toMatchObject({
      mod: true,
      key: "K",
    });
    expect(chordOf(press("k", { ...k, ctrlKey: true }), "mac")).toMatchObject({
      mod: false,
      ctrl: true,
    });
    expect(chordOf(press("k", { ...k, ctrlKey: true }), "other")).toMatchObject(
      { mod: true, ctrl: false },
    );
  });

  test("the key is its position, whatever it types", () => {
    const option = { altKey: true };
    expect(
      chordOf(press("“", { ...option, code: "BracketLeft" }), "mac")?.key,
    ).toBe("[");
    expect(chordOf(press("¡", { ...option, code: "Digit1" }), "mac")?.key).toBe(
      "1",
    );
    expect(chordOf(press("Process", { code: "KeyN" }), "other")?.key).toBe("N");
    expect(chordOf(press("ㅜ", { code: "KeyN" }), "other")?.key).toBe("N");
  });

  test("Shift is always written; a modifier alone is nothing", () => {
    expect(
      chordOf(press("?", { code: "Slash", shiftKey: true }), "other"),
    ).toMatchObject({ key: "/", shift: true });
    expect(
      chordOf(press("Shift", { code: "ShiftLeft", shiftKey: true }), "other"),
    ).toBeNull();
    expect(chordOf(press("ArrowUp", { code: "ArrowUp" }), "other")?.key).toBe(
      "Up",
    );
    expect(chordOf(press("n", { code: "" }), "other")?.key).toBe("N");
  });
});

describe("shown as each platform writes it", () => {
  test("glyphs on a Mac, names elsewhere", () => {
    const [chord] = binding("Mod+Shift+B");
    expect(chordCaps(chord, "mac")).toEqual(["⇧", "⌘", "B"]);
    expect(chordCaps(chord, "other")).toEqual(["Ctrl", "Shift", "B"]);
    expect(bindingWords(binding("Mod+Shift+B"), "other")).toBe("Ctrl+Shift+B");
    expect(bindingWords(binding("G S"), "mac")).toBe("G then S");
  });

  test("platforms", () => {
    expect(platformOf("MacIntel", "")).toBe("mac");
    expect(platformOf("Linux x86_64", "Mozilla/5.0 (X11; Linux)")).toBe(
      "other",
    );
  });
});

test("a sequence overlaps the key it starts with", () => {
  expect(overlaps(binding("G"), binding("G S"), "other")).toBe(true);
  expect(overlaps(binding("G S"), binding("G M"), "other")).toBe(false);
});

test("Mod is resolved before comparing", () => {
  expect(overlaps(binding("Mod+K"), binding("Ctrl+K"), "other")).toBe(true);
  expect(overlaps(binding("Mod+K"), binding("Ctrl+K"), "mac")).toBe(false);
  expect(overlaps(binding("Mod+K"), binding("Meta+K"), "mac")).toBe(true);
});
