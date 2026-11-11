// The page's one key listener (DESIGN.md §7.1): a key press becomes a
// chord (data/shortcuts/keys.ts), the chord an action
// (data/shortcuts/match.ts, with the person's overrides kept by the
// bridge), and the action runs its handler (handlers.ts).
//
// Rules (track W's contract): a chord with Mod, Ctrl, Alt or Meta fires
// while typing in a field; one without (`N`, `/`, `G S`) stands down. No
// key fires while an input method composes, while Settings records a
// key, or after another handler took the event. In a terminal off a Mac
// nothing fires: the terminal owns Control.

import { useEffect } from "react";

import { prefs } from "../../data/prefs";
import { chordOf, commanding } from "../../data/shortcuts/keys";
import { match, type Pending } from "../../data/shortcuts/match";
import { currentPlatform } from "../../data/shortcuts/platform";
import { recording, runShortcut } from "./handlers";

/** Whether `target` is in the agent's terminal. */
function inTerminal(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(".xterm") !== null;
}

/** Whether `target` takes typed text. */
export function typing(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLElement && target.isContentEditable) ||
    inTerminal(target)
  );
}

/** Listen for the page's shortcuts. Mounted once, by the shell. */
export function useShortcutListener(): void {
  useEffect(() => {
    const platform = currentPlatform();
    let pending: Pending | null = null;
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.isComposing || recording.get()) {
        return;
      }
      const chord = chordOf(event, platform);
      if (chord === null) {
        return;
      }
      if (inTerminal(event.target) && platform !== "mac") {
        return;
      }
      if (typing(event.target) && !commanding(chord)) {
        pending = null;
        return;
      }
      const overrides = prefs.get().values.shortcuts;
      const matched = match(chord, pending, overrides, Date.now(), platform);
      pending = null;
      switch (matched.kind) {
        case "wait":
          pending = matched.pending;
          return;
        case "none":
          return;
        case "run":
          if (runShortcut(matched.action, matched.index)) {
            event.preventDefault();
          }
          return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, []);
}
