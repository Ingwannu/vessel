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

export function openWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  place: TerminalPlace,
): Promise<Outcome<TerminalInfo>> {
  return attempt(() => fleet.bridge.call(node, "terminal.create", { place }));
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
