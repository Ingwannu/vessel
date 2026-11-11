// What each shortcut does, registered by the parts of the page that can
// do it (DESIGN.md §7.1): the palette opens itself, a session's view
// finds in it, the shell moves between sessions. The one listener
// (`listener.ts`) asks here.
//
// [Decision Log]
// - Purpose: one place listens for keys (milestone 10 had a listener per
//   component, three of them), and an action means something only where
//   its part is on screen (find, in a session).
// - Choice: a stack of handlers per action; the most recently mounted
//   one answers, so a dialog's or a page's own handler wins over the
//   shell's while it is shown. A handler says whether it acted, so a key
//   nobody acts on is left to the browser (Mod+F outside a session is the
//   browser's find).
// - Not a listener per component: registering is a Map entry, no event
//   listener is added.

import { useEffect, useRef } from "react";

import type { ShortcutAction } from "../../data/shortcuts/registry";
import { Store } from "../../data/store";

/** What an action does; `index` is the session number for "session 1 to
 * 9". Answers whether it acted. */
export type ShortcutHandler = (index: number | null) => boolean;

const stacks = new Map<ShortcutAction, ShortcutHandler[]>();

/** Whether Settings is recording a key: then no shortcut runs. */
export const recording = new Store(false);

/** Run `action`'s current handler; whether one acted. */
export function runShortcut(
  action: ShortcutAction,
  index: number | null,
): boolean {
  const handler = stacks.get(action)?.at(-1);
  return handler === undefined ? false : handler(index);
}

/** Let `handler` answer `action` while the component is mounted (and
 * `enabled`); the latest `handler` is used without registering again. */
export function useShortcut(
  action: ShortcutAction,
  handler: ShortcutHandler,
  enabled = true,
): void {
  const latest = useRef(handler);
  useEffect(() => {
    latest.current = handler;
  });
  useEffect(() => {
    if (!enabled) {
      return undefined;
    }
    const entry: ShortcutHandler = (index) => latest.current(index);
    const stack = stacks.get(action) ?? [];
    stack.push(entry);
    stacks.set(action, stack);
    return () => {
      const at = stack.indexOf(entry);
      if (at !== -1) {
        stack.splice(at, 1);
      }
    };
  }, [action, enabled]);
}
