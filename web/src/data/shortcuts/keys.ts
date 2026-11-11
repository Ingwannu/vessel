// A shortcut's keys: how a binding is written (the bridge keeps that text,
// `wire/prefs.rs`'s `KeyBinding`), how a key press becomes a chord, and
// how a binding is shown on each platform.
//
// [Decision Log]
// - Purpose: one grammar for the registry's defaults, the person's
//   overrides kept by the bridge, the listener's matching and the
//   Settings page's key caps, so the four cannot drift.
// - Constraint: the page runs on a Mac and elsewhere; the same action is
//   Command+K on one and Control+K on the other. A key's character
//   depends on the layout and on Option (a Mac's Option+[ types "“").
// - Choice: `Mod` names Command on a Mac and Control elsewhere
//   (agent-orchestrator's "primary", GOAT's `mod`); `Ctrl` and `Meta` name
//   the physical keys when a binding means them. A letter, a digit, a
//   bracket or a backquote is read from the key's position (`code`) when
//   Alt is held or for the brackets and backquote (agent-orchestrator's
//   rule), and from its character otherwise. A printable character that
//   is not a letter or a digit carries its Shift in itself (`?`, not
//   `Shift+/`), as Paseo shows it. A binding is one chord or two in
//   sequence (`G S`, Vessel's own "go to" keys; Paseo's chords).
// - Display: a Mac's glyphs in Apple's order (⌃ ⌥ ⇧ ⌘) with no separator;
//   elsewhere "Ctrl+Shift+K" (agent-orchestrator's `shortcutBindingKeys`).

import type { Platform } from "./platform";

/** One step of a binding: its modifiers and its key. */
export interface Chord {
  mod: boolean;
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
  /** A letter in upper case, a digit, another printable character, or a
   * name from `NAMED`. */
  key: string;
}

/** A binding: one chord, or two pressed one after the other. */
export type Binding = readonly [Chord] | readonly [Chord, Chord];

/** The keys written by name. */
const NAMED: ReadonlySet<string> = new Set([
  "Enter",
  "Escape",
  "Tab",
  "Space",
  "Backspace",
  "Delete",
  "Up",
  "Down",
  "Left",
  "Right",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "Plus",
  ...Array.from({ length: 12 }, (_, index) => `F${String(index + 1)}`),
]);

/** The modifiers, in the order a binding writes them. */
const MODIFIERS = ["Mod", "Ctrl", "Alt", "Shift", "Meta"] as const;

/** The longest binding the bridge keeps. */
export const MAX_BINDING = 32;

function isKey(key: string): boolean {
  if (NAMED.has(key)) {
    return true;
  }
  if (key.length !== 1) {
    return false;
  }
  const code = key.charCodeAt(0);
  // Printable ASCII but space and "+"; letters in upper case only.
  return code > 0x20 && code < 0x7f && key !== "+" && !/[a-z]/.test(key);
}

function parseChord(text: string): Chord | null {
  const parts = text.split("+");
  const key = parts.pop();
  if (key === undefined || !isKey(key)) {
    return null;
  }
  const chord: Chord = {
    mod: false,
    ctrl: false,
    alt: false,
    shift: false,
    meta: false,
    key,
  };
  let order = 0;
  for (const part of parts) {
    const at = MODIFIERS.findIndex((name) => name === part);
    if (at < order) {
      return null;
    }
    order = at + 1;
    switch (MODIFIERS[at]) {
      case "Mod":
        chord.mod = true;
        break;
      case "Ctrl":
        chord.ctrl = true;
        break;
      case "Alt":
        chord.alt = true;
        break;
      case "Shift":
        chord.shift = true;
        break;
      case "Meta":
        chord.meta = true;
        break;
      case undefined:
        return null;
    }
  }
  return chord;
}

/** `text` as a binding, if it is one: the bridge's grammar exactly. */
export function parseBinding(text: string): Binding | null {
  if (text.length === 0 || text.length > MAX_BINDING) {
    return null;
  }
  const steps = text.split(" ");
  const first = steps[0] === undefined ? null : parseChord(steps[0]);
  if (first === null) {
    return null;
  }
  if (steps.length === 1) {
    return [first];
  }
  const second =
    steps.length === 2 && steps[1] !== undefined ? parseChord(steps[1]) : null;
  return second === null ? null : [first, second];
}

function chordText(chord: Chord): string {
  const names = [
    chord.mod ? "Mod" : null,
    chord.ctrl ? "Ctrl" : null,
    chord.alt ? "Alt" : null,
    chord.shift ? "Shift" : null,
    chord.meta ? "Meta" : null,
  ].filter((name) => name !== null);
  return [...names, chord.key].join("+");
}

/** A binding as the bridge keeps it. */
export function bindingText(binding: Binding): string {
  return binding.map(chordText).join(" ");
}

/** Whether two chords are the same keys. */
export function sameChord(a: Chord, b: Chord): boolean {
  return (
    a.mod === b.mod &&
    a.ctrl === b.ctrl &&
    a.alt === b.alt &&
    a.shift === b.shift &&
    a.meta === b.meta &&
    a.key === b.key
  );
}

/** Whether pressing one of the two would also start or be the other: the
 * same keys, or one a sequence that begins with the other. */
export function overlaps(a: Binding, b: Binding): boolean {
  return (
    sameChord(a[0], b[0]) &&
    (a.length === 1 || b.length === 1 || sameChord(a[1], b[1]))
  );
}

/** Whether a chord holds a modifier that makes it a command rather than
 * text (so it may fire while a field has focus). */
export function commanding(chord: Chord): boolean {
  return chord.mod || chord.ctrl || chord.alt || chord.meta;
}

/** What the listener reads of a key press. */
export interface KeyPress {
  key: string;
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

/** `event.key` names that differ from a binding's. */
const RENAMED: Readonly<Record<string, string>> = {
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  " ": "Space",
  "+": "Plus",
  Esc: "Escape",
};

/** Positions read from `code`: brackets and the backquote always. */
const POSITIONAL: Readonly<Record<string, string>> = {
  BracketLeft: "[",
  BracketRight: "]",
  Backquote: "`",
};

/** The key of a press, or `null` for a modifier alone. */
function keyOf(press: KeyPress): string | null {
  const positional = POSITIONAL[press.code];
  if (positional !== undefined) {
    return positional;
  }
  if (press.altKey) {
    const letter = /^Key([A-Z])$/.exec(press.code)?.[1];
    const digit = /^Digit([0-9])$/.exec(press.code)?.[1];
    if (letter !== undefined || digit !== undefined) {
      return letter ?? digit ?? null;
    }
  }
  const renamed = RENAMED[press.key] ?? press.key;
  if (NAMED.has(renamed)) {
    return renamed;
  }
  if (renamed.length !== 1) {
    return null;
  }
  return renamed.toUpperCase();
}

/** A key press as a chord on `platform`; `null` for a modifier alone or a
 * key no binding can name. */
export function chordOf(press: KeyPress, platform: Platform): Chord | null {
  const key = keyOf(press);
  if (key === null || !isKey(key)) {
    return null;
  }
  // A printable sign carries its Shift ("?" is Shift+/ on most layouts).
  const sign = key.length === 1 && !/[A-Z0-9]/.test(key);
  const mac = platform === "mac";
  return {
    mod: mac ? press.metaKey : press.ctrlKey,
    ctrl: mac ? press.ctrlKey : false,
    alt: press.altKey,
    shift: press.shiftKey && !sign,
    meta: mac ? false : press.metaKey,
    key,
  };
}

/** How a key is shown. */
const SHOWN: Readonly<Record<string, string>> = {
  Up: "↑",
  Down: "↓",
  Left: "←",
  Right: "→",
  Plus: "+",
  Escape: "Esc",
};

/** One chord's caps on `platform`: a Mac's glyphs (⌃⌥⇧⌘ then the key),
 * else the names, as separate caps. */
export function chordCaps(chord: Chord, platform: Platform): string[] {
  const key = SHOWN[chord.key] ?? chord.key;
  if (platform === "mac") {
    return [
      chord.ctrl ? "⌃" : null,
      chord.alt ? "⌥" : null,
      chord.shift ? "⇧" : null,
      chord.mod || chord.meta ? "⌘" : null,
      key,
    ].filter((cap) => cap !== null);
  }
  return [
    chord.mod || chord.ctrl ? "Ctrl" : null,
    chord.alt ? "Alt" : null,
    chord.shift ? "Shift" : null,
    chord.meta ? "Win" : null,
    key,
  ].filter((cap) => cap !== null);
}

/** A binding in words for a sentence ("⌘K", "Ctrl+Shift+B", "G then S"). */
export function bindingWords(binding: Binding, platform: Platform): string {
  const joiner = platform === "mac" ? "" : "+";
  return binding
    .map((chord) => chordCaps(chord, platform).join(joiner))
    .join(" then ");
}
