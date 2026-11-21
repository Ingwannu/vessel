// A terminal notice's click asks for that terminal's tab (ADR 0114
// "Terminal notices"). The click navigates to a session whose workspace
// holds the terminal; the workspace, once it lists the terminal, takes the
// request and selects its tab.
//
// [Decision Log]
// - Purpose: Paseo's tap opens the terminal tab (`session-context.tsx:
//   658-677`). The page keeps each workspace's tabs in that workspace's own
//   state (`use-layout.ts`, per scope), which a notification handler has no
//   handle on.
// - Choice: one pending request at a time, by machine and terminal, taken
//   by the first workspace on that machine that lists the terminal. A newer
//   click replaces an older request.
// - Alternative: a route search parameter. It would change the session
//   route's shape for every link, where this touches only the workspace.

import type { NodeKey } from "../wire/bridge";
import type { TerminalId } from "../wire/proto";

let pending: { node: NodeKey; terminal: TerminalId } | null = null;

/** Ask the next workspace on `node` that lists `terminal` to show it. */
export function requestTerminalTab(node: NodeKey, terminal: TerminalId): void {
  pending = { node, terminal };
}

/** The terminal a workspace on `node` listing `listed` should show now, if
 * one was asked for; the request is then spent. */
export function takeTerminalTab(
  node: NodeKey,
  listed: readonly TerminalId[],
): TerminalId | null {
  const asked = pending;
  if (asked === null) {
    return null;
  }
  if (asked.node !== node || !listed.includes(asked.terminal)) {
    return null;
  }
  pending = null;
  return asked.terminal;
}
