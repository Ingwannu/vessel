// What the shown workspace's active tab is (claims row 3, and its node
// review's note 2): the workspace writes it; presence reads it for its
// focus (`presence.ts`). The workspace's tab state is its own
// (`use-layout.ts`), and presence is mounted by the shell, so the two meet
// here.
//
// Paseo focuses an agent only while an agent's tab is active, and a
// terminal while a terminal's is (`workspace-screen.tsx:1436-1456`): with
// Files, Changes, Diff, a pull request or a draft in front, its focused
// agent is null. So `aside` tells presence the person looks at neither.

import type { NodeKey } from "../wire/bridge";
import type { TerminalId } from "../wire/proto";

export type ShownTab =
  | { node: NodeKey; kind: "agent" }
  | { node: NodeKey; kind: "terminal"; terminal: TerminalId }
  | { node: NodeKey; kind: "aside" }
  | null;

let shown: ShownTab = null;
const listeners = new Set<() => void>();

function same(a: ShownTab, b: ShownTab): boolean {
  if (a === null || b === null) {
    return a === b;
  }
  if (a.node !== b.node || a.kind !== b.kind) {
    return false;
  }
  return (
    a.kind !== "terminal" || b.kind !== "terminal" || a.terminal === b.terminal
  );
}

/** The workspace's active tab, or `null` when no workspace is shown. */
export function setShownTab(next: ShownTab): void {
  if (same(next, shown)) {
    return;
  }
  shown = next;
  for (const listener of listeners) {
    listener();
  }
}

/** The active tab now. */
export function shownTab(): ShownTab {
  return shown;
}

/** Call `listener` on each change; gives back the unsubscribe. */
export function subscribeShownTab(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
