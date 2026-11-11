// Which node is meant, and what to call it (DESIGN.md §2.5, §9).

import type { BookNode, NodeKey, NodeStatus } from "../wire/bridge";
import type { Harness, SessionInfo } from "../wire/proto";
import type { FleetState } from "./fleet";
import { harnessName, isLive, nodeState } from "./format";

/** A node's name for display: the book's, else (this machine's node) the
 * name it gives itself once connected. */
export function nodeLabel(
  node: BookNode,
  status: NodeStatus | undefined,
): string {
  if (node.key !== "_here") {
    return node.label;
  }
  return status?.state === "connected" ? status.node.name : node.label;
}

/** The label of `key`, from the book. */
export function labelOf(state: FleetState, key: NodeKey): string {
  const node = state.book?.find((candidate) => candidate.key === key);
  return node === undefined ? key : nodeLabel(node, state.statuses[key]);
}

/** The node a scoped page means: the URL's, else the last chosen, else the
 * book's default, else this machine's. */
export function selectedNode(
  state: FleetState,
  fromUrl: string | undefined,
): NodeKey | null {
  if (fromUrl !== undefined) {
    return fromUrl;
  }
  if (state.selected !== null) {
    return state.selected;
  }
  const book = state.book ?? [];
  return (book.find((node) => node.default) ?? book[0])?.key ?? null;
}

/** How this client reaches `node`, in the page's words. */
export function reach(node: BookNode): string {
  switch (node.transport) {
    case "ws":
      return "direct";
    case "ssh":
      return "ssh";
    case "paired":
      return "relay";
  }
}

/** A node's facts for its row (GOAT's machine row: one line joined with
 * " · "): what it runs on and how it is reached, then, while connected,
 * its harnesses, its live sessions and its version; else where it is and
 * its state. */
export function nodeFacts(
  node: BookNode,
  status: NodeStatus | undefined,
  harnesses: readonly Harness[] | undefined,
  sessions: readonly SessionInfo[] | undefined,
): string[] {
  if (status?.state !== "connected") {
    return [node.address, reach(node), nodeState(status).word];
  }
  const info = status.node;
  const live = (sessions ?? []).filter(isLive).length;
  return [
    `${info.os}/${info.arch}`,
    reach(node),
    (harnesses ?? []).map((harness) => harnessName(harness.kind)).join(", "),
    `${String(live)} live`,
    info.version,
  ];
}
