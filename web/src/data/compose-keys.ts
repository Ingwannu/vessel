// Which key sends a prompt (DESIGN.md §6.15): Enter, or Mod+Enter, as the
// person chose in Settings › General (agent-orchestrator's "Chat send
// key", Paseo's "Send shortcut", GOAT's "Send shortcut").
//
// Shift+Enter always starts a line, in both modes, as in all three. With
// `enter`, Mod+Enter sends too (track A's steering may take it later);
// with `mod_enter`, Enter starts a line and only Mod+Enter sends. A key
// that ends an input method's composition never sends.

import type { SendKey } from "../wire/bridge";

/** What the composer reads of an Enter press. */
export interface EnterPress {
  key: string;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  ctrlKey: boolean;
  isComposing: boolean;
}

/** Whether `press` sends, in `mode`. */
export function sends(press: EnterPress, mode: SendKey): boolean {
  if (press.key !== "Enter" || press.isComposing || press.shiftKey) {
    return false;
  }
  const mod = press.metaKey || press.ctrlKey;
  switch (mode) {
    case "enter":
      return mod || !press.altKey;
    case "mod_enter":
      return mod;
  }
}

/** The send key in Settings' words, `primary` being "⌘" on a Mac and
 * "Ctrl" elsewhere (Paseo's labels). */
export function sendKeyLabel(mode: SendKey, primary: string): string {
  switch (mode) {
    case "enter":
      return "Enter";
    case "mod_enter":
      return `${primary}+Enter`;
  }
}
