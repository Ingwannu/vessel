import { expect, test } from "vitest";

import {
  type EnterPress,
  sendKeyLabel,
  sends,
  sendsOther,
} from "./compose-keys";

function enter(extra: Partial<EnterPress> = {}): EnterPress {
  return {
    key: "Enter",
    shiftKey: false,
    altKey: false,
    metaKey: false,
    ctrlKey: false,
    isComposing: false,
    ...extra,
  };
}

test("Enter mode: Enter and Mod+Enter send; Shift or Alt starts a line", () => {
  expect(sends(enter(), "enter")).toBe(true);
  expect(sends(enter({ metaKey: true }), "enter")).toBe(true);
  expect(sends(enter({ shiftKey: true }), "enter")).toBe(false);
  expect(sends(enter({ altKey: true }), "enter")).toBe(false);
});

test("Mod+Enter mode: only Mod+Enter sends", () => {
  expect(sends(enter(), "mod_enter")).toBe(false);
  expect(sends(enter({ ctrlKey: true }), "mod_enter")).toBe(true);
  expect(sends(enter({ ctrlKey: true, shiftKey: true }), "mod_enter")).toBe(
    false,
  );
});

test("an input method's Enter never sends; another key never does", () => {
  expect(sends(enter({ isComposing: true }), "enter")).toBe(false);
  expect(sends({ ...enter(), key: "a" }, "enter")).toBe(false);
});

test("labels", () => {
  expect(sendKeyLabel("enter", "⌘")).toBe("Enter");
  expect(sendKeyLabel("mod_enter", "Ctrl")).toBe("Ctrl+Enter");
});

test("the other way: Mod+Enter, or Mod+Shift+Enter where Mod+Enter sends", () => {
  const press = {
    key: "Enter",
    shiftKey: false,
    altKey: false,
    metaKey: false,
    ctrlKey: true,
    isComposing: false,
  };
  expect(sendsOther(press, "enter")).toBe(true);
  expect(sendsOther(press, "mod_enter")).toBe(false);
  expect(sendsOther({ ...press, shiftKey: true }, "mod_enter")).toBe(true);
  expect(sendsOther({ ...press, ctrlKey: false }, "enter")).toBe(false);
  expect(sendsOther({ ...press, isComposing: true }, "enter")).toBe(false);
});
