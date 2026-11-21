import { describe, expect, it } from "vitest";

import {
  type Control,
  encodeKey,
  keyBarRows,
  NO_STICKY,
  pressKey,
  ROWS,
  type Sticky,
  THREE_COLUMN_ROWS,
  typedWith,
  WIDE_ROWS,
} from "./terminal-keys";

const set = (mods: Partial<Sticky>): Sticky => ({ ...NO_STICKY, ...mods });

describe("the key bar's rows", () => {
  it("follow Paseo's widths", () => {
    expect(keyBarRows(0, true)).toBe(ROWS);
    expect(keyBarRows(299, true)).toBe(THREE_COLUMN_ROWS);
    expect(keyBarRows(300, true)).toBe(ROWS);
    expect(keyBarRows(659, false)).toBe(ROWS);
    expect(keyBarRows(660, false)).toBe(WIDE_ROWS);
    expect(keyBarRows(880, true)).toBe(ROWS);
  });

  it("hold the same twelve controls in every table", () => {
    const sorted = (table: readonly (readonly Control[])[]) =>
      table
        .flat()
        .map((control) => JSON.stringify(control))
        .sort();
    expect(sorted(ROWS)).toHaveLength(12);
    expect(sorted(WIDE_ROWS)).toEqual(sorted(ROWS));
    expect(sorted(THREE_COLUMN_ROWS)).toEqual(sorted(ROWS));
  });
});

describe("the keys' bytes, as native's keys.rs", () => {
  it("encodes control letters and symbols as xterm", () => {
    const ctrl = (char: string) =>
      encodeKey({ char }, set({ ctrl: true }), false);
    expect(ctrl("c")).toBe("\x03");
    expect(ctrl("C")).toBe("\x03");
    expect(ctrl("a")).toBe("\x01");
    expect(ctrl(" ")).toBe("\x00");
    expect(ctrl("[")).toBe("\x1b");
    expect(ctrl("/")).toBe("\x1f");
    expect(ctrl("?")).toBe("\x7f");
  });

  it("upper-cases only ASCII for Control, and Shift gives one character (P4w note 2)", () => {
    expect(encodeKey({ char: "ß" }, set({ ctrl: true }), false)).toBe("ß");
    expect(encodeKey({ char: "ß" }, set({ shift: true }), false)).toBe("S");
    expect(encodeKey({ char: "é" }, set({ shift: true }), false)).toBe("É");
  });

  it("prefixes Alt and shifts characters", () => {
    expect(encodeKey({ char: "x" }, set({ alt: true }), false)).toBe("\x1bx");
    expect(encodeKey({ char: "x" }, set({ shift: true }), false)).toBe("X");
    expect(
      encodeKey({ char: "c" }, set({ ctrl: true, alt: true }), false),
    ).toBe("\x1b\x03");
  });

  it("encodes the bar's named keys", () => {
    expect(pressKey("esc", NO_STICKY, false)).toBe("\x1b");
    expect(pressKey("enter", set({ ctrl: true }), false)).toBe("\r");
    expect(pressKey("tab", NO_STICKY, false)).toBe("\t");
    expect(pressKey("tab", set({ shift: true }), false)).toBe("\x1b[Z");
    expect(pressKey("tab", set({ alt: true }), false)).toBe("\x1b\t");
    expect(pressKey("up", NO_STICKY, false)).toBe("\x1b[A");
    expect(pressKey("up", NO_STICKY, true)).toBe("\x1bOA");
    expect(pressKey("up", set({ ctrl: true }), false)).toBe("\x1b[1;5A");
    expect(pressKey("up", set({ ctrl: true }), true)).toBe("\x1b[1;5A");
    expect(pressKey("left", set({ ctrl: true, shift: true }), false)).toBe(
      "\x1b[1;6D",
    );
    expect(pressKey("right", NO_STICKY, false)).toBe("\x1b[C");
    expect(pressKey("down", NO_STICKY, false)).toBe("\x1b[B");
  });
});

describe("text typed while modifiers are set", () => {
  it("is as typed with none set", () => {
    expect(typedWith("c", NO_STICKY, false)).toBeNull();
  });

  it("is the key with them for one mapped character", () => {
    expect(typedWith("c", set({ ctrl: true }), false)).toBe("\x03");
    expect(typedWith("x", set({ alt: true }), false)).toBe("\x1bx");
    expect(typedWith("\r", set({ ctrl: true }), false)).toBe("\r");
    expect(typedWith("\x7f", set({ alt: true }), false)).toBe("\x1b\x7f");
  });

  it("is as typed for anything else", () => {
    expect(typedWith("ab", set({ ctrl: true }), false)).toBe("ab");
    expect(typedWith("한", set({ ctrl: true }), false)).toBe("한");
  });
});
