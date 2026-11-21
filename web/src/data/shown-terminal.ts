// The terminal whose tab the shown workspace has active (claims row 3). The
// workspace writes it; presence reads it for `terminal` focus
// (`presence.ts`). The workspace's tab state is its own (`use-layout.ts`),
// and presence is mounted by the shell, so the two meet here.

import type { NodeKey } from "../wire/bridge";
import type { TerminalId } from "../wire/proto";

export type ShownTerminal = { node: NodeKey; terminal: TerminalId } | null;

let shown: ShownTerminal = null;
const listeners = new Set<() => void>();

/** The workspace's active terminal tab, or `null` when it shows none. */
export function setShownTerminal(next: ShownTerminal): void {
  if (next?.node === shown?.node && next?.terminal === shown?.terminal) {
    return;
  }
  shown = next;
  for (const listener of listeners) {
    listener();
  }
}

/** The terminal shown now. */
export function shownTerminal(): ShownTerminal {
  return shown;
}

/** Call `listener` on each change; gives back the unsubscribe. */
export function subscribeShownTerminal(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
