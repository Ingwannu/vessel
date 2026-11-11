// Changing an action's keys in Settings (agent-orchestrator's
// `KeyboardShortcutsContent`): whether a recorded chord may be a shortcut,
// whether it clashes, and the edits a change sends to the bridge.
//
// [Decision Log]
// - Purpose: the owner asked that a clash be refused. agent-orchestrator,
//   whose page this follows, refuses a key a fixed action holds, and for
//   one another action holds asks "Shortcut already in use … Reassign it
//   to …?": the key then moves, so two actions never share one.
//   Vessel does the same; GOAT only warns, Paseo does not check.
// - A recorded key needs Ctrl, Alt or Command with it (agent-orchestrator's
//   rule: a bare letter would fire while typing elsewhere); the keys a
//   browser keeps for itself or that edit text are refused with the
//   reason, so a binding the page could never see is not saved.
// - A reassignment is one `prefer` change with two edits, so a tab never
//   sees the key on both actions or on neither.

import type { Platform } from "./platform";
import { type Binding, bindingText, type Chord, overlaps } from "./keys";
import {
  bindingsOf,
  definition,
  fixed,
  jumpOf,
  MAX_KEYS,
  type Overrides,
  SHORTCUT_ACTIONS,
  type ShortcutAction,
} from "./registry";

/** Why a recorded chord cannot be a shortcut, in agent-orchestrator's
 * words; `null` when it can. */
export function refusal(chord: Chord, platform: Platform): string | null {
  if (!(chord.mod || chord.ctrl || chord.alt || chord.meta)) {
    return platform === "mac"
      ? "Use Control, Option or Command with another key."
      : "Use Ctrl or Alt with another key.";
  }
  const primary = chord.mod && !chord.ctrl && !chord.alt && !chord.meta;
  if (primary && ["N", "T", "W", "Q", "Tab"].includes(chord.key)) {
    return "The browser keeps that shortcut for itself.";
  }
  if (primary && !chord.shift && /^[1-9]$/.test(chord.key)) {
    return "The browser keeps that shortcut for itself.";
  }
  if (
    primary &&
    !chord.shift &&
    ["A", "C", "V", "X", "Z"].includes(chord.key)
  ) {
    return "That shortcut is reserved for standard editing commands.";
  }
  return null;
}

/** What a binding would clash with. */
export type Clash =
  | { kind: "same"; action: ShortcutAction }
  | { kind: "fixed"; action: ShortcutAction }
  | { kind: "taken"; action: ShortcutAction };

/** What `binding`, given to `action`, clashes with now; `null` if
 * nothing. A sequence that starts with the binding (or the reverse)
 * clashes too: one would hide the other. */
export function clashOf(
  action: ShortcutAction,
  binding: Binding,
  overrides: Overrides,
  platform: Platform,
): Clash | null {
  if (binding.length === 1 && jumpOf(binding[0]) !== null) {
    return { kind: "fixed", action: "jump-session" };
  }
  for (const other of SHORTCUT_ACTIONS) {
    if (fixed(other)) {
      continue;
    }
    const held = bindingsOf(other, overrides).some((existing) =>
      overlaps(existing, binding, platform),
    );
    if (held) {
      return { kind: other === action ? "same" : "taken", action: other };
    }
  }
  return null;
}

/** The sentence a clash shows in the recorder or the dialog. */
export function clashText(
  clash: Clash,
  binding: string,
  to: ShortcutAction,
): string {
  switch (clash.kind) {
    case "same":
      return `${binding} is already a key of this action.`;
    case "fixed":
      return `${binding} is used by ${definition(clash.action).label}.`;
    case "taken":
      return `${binding} is assigned to ${definition(clash.action).label}. Reassign it to ${definition(to).label}?`;
  }
}

/** One action's keys after a change: the keys themselves, or back to the
 * defaults. Mirrors the bridge's `ShortcutEdit`. */
export interface ShortcutEdit {
  action: ShortcutAction;
  to: { kind: "bindings"; keys: string[] } | { kind: "default" };
}

function keysOf(bindings: readonly Binding[]): string[] {
  return bindings.slice(0, MAX_KEYS).map(bindingText);
}

/** The edits that give `action` the keys `bindings`. */
export function setKeys(
  action: ShortcutAction,
  bindings: readonly Binding[],
): ShortcutEdit[] {
  return [{ action, to: { kind: "bindings", keys: keysOf(bindings) } }];
}

/** The edits that add `binding` to `action` (`replace`: in place of the
 * key at that index, else as another key; the oldest key goes when it
 * already has two), taking it from `from` when it was another action's. */
export function assign(
  action: ShortcutAction,
  binding: Binding,
  overrides: Overrides,
  replace: number | null,
  from: ShortcutAction | null,
  platform: Platform,
): ShortcutEdit[] {
  const current = bindingsOf(action, overrides);
  const next =
    replace === null
      ? [...current, binding].slice(-MAX_KEYS)
      : current.map((existing, index) =>
          index === replace ? binding : existing,
        );
  const edits = setKeys(action, next);
  if (from !== null && from !== action) {
    const left = bindingsOf(from, overrides).filter(
      (existing) => !overlaps(existing, binding, platform),
    );
    edits.push(...setKeys(from, left));
  }
  return edits;
}

/** The edits that remove the key at `index` from `action`. */
export function removeKey(
  action: ShortcutAction,
  index: number,
  overrides: Overrides,
): ShortcutEdit[] {
  return setKeys(
    action,
    bindingsOf(action, overrides).filter((_, at) => at !== index),
  );
}

/** The edit that gives `action` its defaults again. */
export function restore(action: ShortcutAction): ShortcutEdit[] {
  return [{ action, to: { kind: "default" } }];
}
