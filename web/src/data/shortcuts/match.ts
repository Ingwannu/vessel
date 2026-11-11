// Which action a key press runs (DESIGN.md §7.1): one chord, or the second
// of a sequence (`G S`) pressed within `SEQUENCE_MS` of the first.
//
// A pure function of the press, what was pressed before, the overrides
// and the time, so the one listener (`features/shortcuts`) only reads the
// event and runs what this answers.

import { type Binding, type Chord, sameChord } from "./keys";
import {
  bindingsOf,
  jumpOf,
  type Overrides,
  SHORTCUT_ACTIONS,
  type ShortcutAction,
} from "./registry";

/** How long the second key of a sequence may wait (Paseo's chord
 * timeout). */
export const SEQUENCE_MS = 1500;

/** The first key of a sequence, waiting for its second. */
export interface Pending {
  chord: Chord;
  at: number;
}

/** What a press does. */
export type Matched =
  | { kind: "run"; action: ShortcutAction; index: number | null }
  | { kind: "wait"; pending: Pending }
  | { kind: "none" };

function every(overrides: Overrides): [ShortcutAction, Binding][] {
  return SHORTCUT_ACTIONS.flatMap((action) =>
    bindingsOf(action, overrides).map((binding): [ShortcutAction, Binding] => [
      action,
      binding,
    ]),
  );
}

/** What pressing `chord` at `now` does, `pending` being the first key of a
 * sequence pressed before (or `null`). */
export function match(
  chord: Chord,
  pending: Pending | null,
  overrides: Overrides,
  now: number,
): Matched {
  const bound = every(overrides);
  if (pending !== null && now - pending.at <= SEQUENCE_MS) {
    for (const [action, binding] of bound) {
      if (
        binding.length === 2 &&
        sameChord(binding[0], pending.chord) &&
        sameChord(binding[1], chord)
      ) {
        return { kind: "run", action, index: null };
      }
    }
  }
  const jump = jumpOf(chord);
  if (jump !== null) {
    return { kind: "run", action: "jump-session", index: jump };
  }
  for (const [action, binding] of bound) {
    if (binding.length === 1 && sameChord(binding[0], chord)) {
      return { kind: "run", action, index: null };
    }
  }
  const starts = bound.some(
    ([, binding]) => binding.length === 2 && sameChord(binding[0], chord),
  );
  return starts
    ? { kind: "wait", pending: { chord, at: now } }
    : { kind: "none" };
}
