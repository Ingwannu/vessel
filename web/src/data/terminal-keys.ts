// The virtual key bar under a page terminal at compact widths (P4): Paseo's
// `terminal-virtual-keyboard.ts`, its pane's sticky modifiers, and the
// bytes its keys send, as the native app's `view/src/terminal/key_bar.rs`
// and `keys.rs` encode them.
//
// [Decision Log]
// - Purpose: on a phone, Esc, Tab, the arrows and Ctrl/Alt/Shift chords
//   are not on the soft keyboard. Paseo draws a bar of them under its
//   terminal when compact.
// - Paseo 0.15.4:
//   - three layouts (25-82), picked by the bar's width (84-97);
//   - shown on its mobile app, and elsewhere when compact (99-104);
//   - Paste only on its mobile app (121-123), so not on the web;
//   - modifiers are sticky: toggled, then applied to the next key, from
//     the bar or typed, then cleared (`terminal-pane.tsx` 795-830,
//     1052-1083; `utils/terminal-keys.ts` 204-263).
// - Choice: the tables, widths and sticky rule as data and pure functions.
//   The keys are encoded as native's `keys.rs` encodes them (Paseo's
//   `encodeTerminalKeyInput`), so both surfaces send the same bytes. xterm
//   cannot be asked to type a named key with modifiers.
// - Cost: a second copy of a small table of escape sequences, pinned by
//   tests that use native's cases.

/** A key the bar sends (Paseo's `TERMINAL_VIRTUAL_KEY_BUTTONS`). */
export type BarKey = "esc" | "tab" | "up" | "down" | "left" | "right" | "enter";

/** A sticky modifier. */
export type Modifier = "ctrl" | "alt" | "shift";

/** One control of the bar. */
export type Control =
  | { type: "key"; key: BarKey }
  | { type: "modifier"; modifier: Modifier }
  | { type: "paste" }
  | { type: "keyboardToggle" };

/** Each key's label (`terminal-key-dispatch.ts` 20-30). */
export const KEY_LABELS: Readonly<Record<BarKey, string>> = {
  esc: "Esc",
  tab: "Tab",
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
  enter: "Enter",
};

/** Each modifier's label (`terminal-pane.tsx` 119-123). */
export const MODIFIER_LABELS: Readonly<Record<Modifier, string>> = {
  ctrl: "Ctrl",
  shift: "Shift",
  alt: "Alt",
};

const key = (name: BarKey): Control => ({ type: "key", key: name });
const mod = (name: Modifier): Control => ({ type: "modifier", modifier: name });
const PASTE: Control = { type: "paste" };
const TOGGLE: Control = { type: "keyboardToggle" };

/** Two rows (`TERMINAL_VIRTUAL_KEYBOARD_ROWS`). */
export const ROWS: readonly (readonly Control[])[] = [
  [key("esc"), key("tab"), mod("ctrl"), key("up"), mod("shift"), TOGGLE],
  [mod("alt"), PASTE, key("left"), key("down"), key("right"), key("enter")],
];

/** One wide row (`TERMINAL_VIRTUAL_KEYBOARD_WIDE_ROWS`). */
export const WIDE_ROWS: readonly (readonly Control[])[] = [
  [
    key("esc"),
    key("tab"),
    mod("ctrl"),
    mod("alt"),
    mod("shift"),
    key("left"),
    key("down"),
    key("up"),
    key("right"),
    PASTE,
    TOGGLE,
    key("enter"),
  ],
];

/** Three columns (`TERMINAL_VIRTUAL_KEYBOARD_THREE_COLUMN_ROWS`). */
export const THREE_COLUMN_ROWS: readonly (readonly Control[])[] = [
  [key("esc"), key("tab"), mod("ctrl")],
  [mod("alt"), mod("shift"), TOGGLE],
  [PASTE, key("up"), key("enter")],
  [key("left"), key("down"), key("right")],
];

/** From this width, and not compact, one row. */
export const SINGLE_ROW_MIN_WIDTH = 660;
/** Under this width, three columns. */
export const THREE_COLUMN_MAX_WIDTH = 300;
/** The widest the rows grow, centred. */
export const KEY_BAR_MAX_WIDTH = 880;

/** The rows for a bar `width` wide (0: not measured yet). */
export function keyBarRows(
  width: number,
  compact: boolean,
): readonly (readonly Control[])[] {
  if (width > 0 && width < THREE_COLUMN_MAX_WIDTH) {
    return THREE_COLUMN_ROWS;
  }
  return !compact && width >= SINGLE_ROW_MIN_WIDTH ? WIDE_ROWS : ROWS;
}

/** A stable name for a control's element (Paseo's test ids). */
export function controlId(control: Control): string {
  switch (control.type) {
    case "key":
      return `terminal-key-${control.key}`;
    case "modifier":
      return `terminal-key-${control.modifier}`;
    case "paste":
      return "terminal-paste";
    case "keyboardToggle":
      return "terminal-keyboard-toggle";
  }
}

/** The modifiers set on the bar. */
export interface Sticky {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
}

export const NO_STICKY: Sticky = { ctrl: false, alt: false, shift: false };

export function hasSticky(sticky: Sticky): boolean {
  return sticky.ctrl || sticky.alt || sticky.shift;
}

/** A named key, or a character. */
type Pressed =
  | { named: "enter" | "tab" | "backspace" | "escape" }
  | { named: "up" | "down" | "left" | "right" }
  | { char: string };

/** xterm's modifier parameter: 1, plus 1 Shift, 2 Alt, 4 Control. */
function param(mods: Sticky): number {
  return 1 + (mods.shift ? 1 : 0) + (mods.alt ? 2 : 0) + (mods.ctrl ? 4 : 0);
}

function altPrefixed(sequence: string, mods: Sticky): string {
  return mods.alt ? `\x1b${sequence}` : sequence;
}

/** `text` with its ASCII letters upper-cased and nothing else changed (Rust's
 * `to_ascii_uppercase`): `toUpperCase` would make "ß" "SS" (P4w node review
 * note 2). */
function asciiUpper(text: string): string {
  return text.replace(/[a-z]/g, (letter) =>
    String.fromCharCode(letter.charCodeAt(0) - 32),
  );
}

/** A character's upper form's first character, as native's `printable`
 * (`to_uppercase().next()`): one character in, one out. */
function shifted(character: string): string {
  const [first] = character.toUpperCase();
  return first ?? character;
}

/** Control with a character, as xterm's (native `keys.rs` `control`). */
function control(character: string): string {
  const upper = asciiUpper(character);
  const code = upper.codePointAt(0) ?? 0;
  if (upper.length === 1 && upper >= "A" && upper <= "Z") {
    return String.fromCharCode(code - 64);
  }
  const symbols: Readonly<Record<string, number>> = {
    " ": 0x00,
    "@": 0x00,
    "2": 0x00,
    "[": 0x1b,
    "3": 0x1b,
    "\\": 0x1c,
    "4": 0x1c,
    "]": 0x1d,
    "5": 0x1d,
    "^": 0x1e,
    "6": 0x1e,
    _: 0x1f,
    "/": 0x1f,
    "7": 0x1f,
    "8": 0x7f,
    "?": 0x7f,
  };
  const symbol = symbols[upper];
  if (symbol !== undefined) {
    return String.fromCharCode(symbol);
  }
  return code < 0x80 ? String.fromCharCode(code & 0x1f) : character;
}

function arrow(last: string, mods: Sticky, application: boolean): string {
  const p = param(mods);
  if (p === 1) {
    return application ? `\x1bO${last}` : `\x1b[${last}`;
  }
  return `\x1b[1;${String(p)}${last}`;
}

const ARROWS = { up: "A", down: "B", right: "C", left: "D" } as const;

/** The bytes a program reads for `pressed` with `mods`, its arrows in the
 * program's cursor mode (native `keys.rs` `encode`). */
export function encodeKey(
  pressed: Pressed,
  mods: Sticky,
  applicationCursor: boolean,
): string {
  if ("char" in pressed) {
    const shown = mods.shift ? shifted(pressed.char) : pressed.char;
    return mods.ctrl
      ? altPrefixed(control(shown), mods)
      : altPrefixed(shown, mods);
  }
  switch (pressed.named) {
    case "enter":
      return "\r";
    case "tab":
      return mods.shift && !mods.alt && !mods.ctrl
        ? "\x1b[Z"
        : altPrefixed("\t", mods);
    case "backspace":
      return altPrefixed("\x7f", mods);
    case "escape":
      return "\x1b";
    case "up":
    case "down":
    case "left":
    case "right":
      return arrow(ARROWS[pressed.named], mods, applicationCursor);
  }
}

const BAR_KEYS: Readonly<Record<BarKey, Pressed>> = {
  esc: { named: "escape" },
  tab: { named: "tab" },
  up: { named: "up" },
  down: { named: "down" },
  left: { named: "left" },
  right: { named: "right" },
  enter: { named: "enter" },
};

/** A bar key with the set modifiers. */
export function pressKey(
  barKey: BarKey,
  sticky: Sticky,
  applicationCursor: boolean,
): string {
  return encodeKey(BAR_KEYS[barKey], sticky, applicationCursor);
}

/** Text typed while modifiers are set (Paseo's
 * `resolvePendingModifierDataInput` with `mapTerminalDataToKey`): one
 * character naming a key, or printable ASCII, goes as that key with them;
 * anything else as typed. `null` when none is set. The caller clears the
 * modifiers whenever this is not `null`. */
export function typedWith(
  text: string,
  sticky: Sticky,
  applicationCursor: boolean,
): string | null {
  if (!hasSticky(sticky)) {
    return null;
  }
  // Paseo's `data.length !== 1`: only one UTF-16 unit maps to a key (the
  // mapped set is ASCII), anything longer goes as typed.
  if (text.length !== 1) {
    return text;
  }
  const named: Record<string, Pressed> = {
    "\r": { named: "enter" },
    "\n": { named: "enter" },
    "\t": { named: "tab" },
    "\x7f": { named: "backspace" },
    "\b": { named: "backspace" },
    "\x1b": { named: "escape" },
  };
  const pressed = named[text];
  if (pressed !== undefined) {
    return encodeKey(pressed, sticky, applicationCursor);
  }
  const code = text.codePointAt(0) ?? 0;
  if (code >= 0x20 && code <= 0x7e) {
    return encodeKey({ char: text }, sticky, applicationCursor);
  }
  return text;
}
