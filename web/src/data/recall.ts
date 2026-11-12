// Up and Down in the composer recall what was sent before (DESIGN.md
// §6.15), from the session's own prompts: the conversation is the
// history, as in all three references, so nothing is stored for it.
//
// [Decision Log]
// - Purpose: the three references recall sent messages with the arrows;
//   the owner asked for the same.
// - Choice: Paseo's and GOAT's rules. A walk starts only from an empty
//   composer, with no modifier and the caret not in an input method's
//   composition. While a recalled message is shown unchanged, Up goes
//   older when the caret is on its first line and Down newer when it is
//   on its last (so the arrows still move within a long message);
//   Down past the newest empties the composer; Up past the oldest does
//   nothing. Editing the shown message ends the walk; history is never
//   edited. Back-to-back identical prompts are one stop.
// - The new-session page has no conversation and so no history (GOAT's
//   has none either).

import type { Entry } from "./timeline-model";

/** Where a walk through the history is: the index of the shown prompt
 * (0 the newest) and its text, to tell whether it was edited. */
export interface Walk {
  index: number;
  shown: string;
}

/** The sent prompts, newest first, back-to-back repeats as one. */
export function sentPrompts(entries: readonly Entry[]): string[] {
  const prompts: string[] = [];
  for (const entry of entries) {
    if (entry.kind !== "prompt" || entry.text.trim() === "") {
      continue;
    }
    if (prompts.at(-1) !== entry.text) {
      prompts.push(entry.text);
    }
  }
  return prompts.reverse();
}

/** What the composer holds and where its caret is. */
export interface Caret {
  text: string;
  start: number;
  end: number;
}

/** What an arrow does: show a prompt (or the empty composer, ending the
 * walk), or nothing, leaving the key to move the caret. */
export type Recalled =
  { kind: "show"; text: string; walk: Walk | null } | { kind: "pass" };

/** The arrow `key` with the composer at `caret`, `walk` the walk so far
 * (or `null`), over `prompts` (newest first). */
export function recall(
  key: "up" | "down",
  caret: Caret,
  walk: Walk | null,
  prompts: readonly string[],
): Recalled {
  if (caret.start !== caret.end) {
    return { kind: "pass" };
  }
  const walking = walk !== null && caret.text === walk.shown;
  if (!walking) {
    if (key === "down" || caret.text !== "") {
      return { kind: "pass" };
    }
    const newest = prompts[0];
    return newest === undefined
      ? { kind: "pass" }
      : { kind: "show", text: newest, walk: { index: 0, shown: newest } };
  }
  if (key === "up") {
    const firstLine = !caret.text.slice(0, caret.start).includes("\n");
    const older = prompts[walk.index + 1];
    if (!firstLine || older === undefined) {
      return { kind: "pass" };
    }
    return {
      kind: "show",
      text: older,
      walk: { index: walk.index + 1, shown: older },
    };
  }
  const lastLine = !caret.text.slice(caret.end).includes("\n");
  if (!lastLine) {
    return { kind: "pass" };
  }
  if (walk.index === 0) {
    return { kind: "show", text: "", walk: null };
  }
  const newer = prompts[walk.index - 1] ?? "";
  return {
    kind: "show",
    text: newer,
    walk: { index: walk.index - 1, shown: newer },
  };
}
