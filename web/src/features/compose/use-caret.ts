// Where the caret goes after the composer itself rewrote its text: a `/`
// or `@` suggestion chosen, a prompt recalled with Up or Down.
//
// [Decision Log]
// - Purpose: after such a rewrite the caret belongs at a known place (after
//   the chosen command, at the start or end of a recalled prompt).
// - Before: the caret was set in the next animation frame. A selection
//   made before that frame was undone by it: a select-all followed by
//   typing, which is what Playwright's `fill` does in two steps, became a
//   caret at the old end, and the typed text was appended to the line
//   instead of replacing it ("/review /co", so `/` listed nothing:
//   conversation.spec.ts in W11's whole run, 2026-11-15). Under load a
//   frame can come much later than 16 ms, and a quick person can do the
//   same.
// - Alternatives: a test wait for the caret (the window stays in the
//   product); `flushSync` around the text change (a forced render inside
//   the key handler, for every rewrite).
// - Choice: the caret is placed in a layout effect, in the same commit
//   that puts the text in the box, before the browser handles any other
//   input; and only while the box still holds the text the caret was
//   computed for. A rewrite that leaves the text as it was (so nothing is
//   drawn) places the caret at once.
// - Cost: one ref and one effect on the composer; the effect does nothing
//   unless a rewrite is waiting.

import { type RefObject, useLayoutEffect, useRef } from "react";

interface Waiting {
  text: string;
  at: number;
}

/** `place(text, at)`: put the caret at `at` once the box shows `text`. */
export function useCaretAfterText(
  area: RefObject<HTMLTextAreaElement | null>,
): (text: string, at: number) => void {
  const waiting = useRef<Waiting | null>(null);
  useLayoutEffect(() => {
    const wanted = waiting.current;
    const box = area.current;
    if (wanted === null || box === null) {
      return;
    }
    waiting.current = null;
    if (box.value === wanted.text) {
      box.setSelectionRange(wanted.at, wanted.at);
    }
  });
  // Not wrapped in useCallback: the React Compiler memoizes it, and no
  // caller needs it stable.
  return (text: string, at: number): void => {
    const box = area.current;
    if (box !== null && box.value === text) {
      waiting.current = null;
      box.setSelectionRange(at, at);
      return;
    }
    waiting.current = { text, at };
  };
}
