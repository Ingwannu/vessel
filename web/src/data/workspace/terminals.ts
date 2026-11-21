// Standalone terminals use the workspace's registry and its own stream.
// Session-terminal input still follows the legacy session API.

import type { StreamHandlers } from "../../bridge/followers";
import type { NodeKey } from "../../wire/bridge";
import type {
  TerminalId,
  TerminalInput,
  TerminalInfo,
  TerminalList,
  TerminalPlace,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

export const TERMINAL_PROTOCOL = 11;

export function workspaceTerminals(
  fleet: Fleet,
  node: NodeKey,
  place: TerminalPlace | null,
): Promise<Outcome<TerminalList>> {
  return attempt(() => fleet.bridge.call(node, "terminal.list", { place }));
}

/** The terminals this tab opened: their pane is the "opened" view of
 * ADR 0073's duty, answering a live query before anyone types (a program
 * started there, nvim say, asks at once). Kept for the tab's life. */
const openedHere = new Set<TerminalId>();

/** Whether this tab opened `terminal`. */
export function openedInThisTab(terminal: TerminalId): boolean {
  return openedHere.has(terminal);
}

export function rememberOpenedTerminal(terminal: TerminalId): void {
  openedHere.add(terminal);
}

/** Open a shell in `place`, or terminal profile `profile`'s program (ADR
 * 0117; no prompt, so its placeholder arguments are left out). */
export async function openWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  place: TerminalPlace,
  profile: string | null = null,
): Promise<Outcome<TerminalInfo>> {
  const result = await attempt(() =>
    fleet.bridge.call(
      node,
      "terminal.create",
      profile === null ? { place } : { place, profile: { id: profile } },
    ),
  );
  if (result.ok) {
    rememberOpenedTerminal(result.value.id);
  }
  return result;
}

export function renameWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
  title: string,
): Promise<Outcome<TerminalInfo>> {
  return attempt(() =>
    fleet.bridge.call(node, "terminal.rename", { terminal, title }),
  );
}

export function readWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
): Promise<Outcome<TerminalInfo>> {
  return attempt(() => fleet.bridge.call(node, "terminal.get", { terminal }));
}

export function closeWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
): Promise<Outcome<null>> {
  return attempt(() => fleet.bridge.call(node, "terminal.close", { terminal }));
}

export function followWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
  handlers: StreamHandlers,
): () => void {
  return fleet.bridge.followTerminal(node, terminal, 0, handlers);
}

export function sendWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
  input: TerminalInput,
): Promise<null> {
  return fleet.bridge.call(node, "terminal.send", { terminal, input });
}
