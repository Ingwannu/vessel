// Standalone terminals use the workspace's registry and its own stream.
// Session-terminal input still follows the legacy session API.

import type { StreamHandlers } from "../../bridge/followers";
import type { NodeKey } from "../../wire/bridge";
import type {
  TerminalId,
  TerminalInput,
  TerminalPlace,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt } from "../outcome";

export const TERMINAL_PROTOCOL = 11;

export function workspaceTerminals(
  fleet: Fleet,
  node: NodeKey,
  place: TerminalPlace | null,
) {
  return attempt(() => fleet.bridge.call(node, "terminal.list", { place }));
}

export function openWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  place: TerminalPlace,
) {
  return attempt(() => fleet.bridge.call(node, "terminal.create", { place }));
}

export function renameWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
  title: string,
) {
  return attempt(() =>
    fleet.bridge.call(node, "terminal.rename", { terminal, title }),
  );
}

export function closeWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
) {
  return attempt(() => fleet.bridge.call(node, "terminal.close", { terminal }));
}

export function followWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
  handlers: StreamHandlers,
) {
  return fleet.bridge.followTerminal(node, terminal, 0, handlers);
}

export function sendWorkspaceTerminal(
  fleet: Fleet,
  node: NodeKey,
  terminal: TerminalId,
  input: TerminalInput,
) {
  return fleet.bridge.call(node, "terminal.send", { terminal, input });
}
