// Which node is meant, and what to call it (DESIGN.md §2.5, §9).

import type { BookNode, NodeKey, NodeStatus } from "../wire/bridge";
import type { FleetState } from "./fleet";

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

/** The machine meant (DESIGN.md §2.5), the one rule every page uses:
 * the first of `wanted` (the address's, then the one last chosen), else
 * the book's default, else the first in the book; only one `accept`
 * takes. A new session needs a connected machine; Settings, one in the
 * book; a page scoped by its address, any. */
export function machineMeant(
  book: readonly BookNode[],
  wanted: readonly (NodeKey | null | undefined)[],
  accept: (key: NodeKey) => boolean = () => true,
): NodeKey | null {
  const candidates = [
    ...wanted,
    book.find((node) => node.default)?.key,
    ...book.map((node) => node.key),
  ];
  return (
    candidates.find(
      (key): key is NodeKey => key !== null && key !== undefined && accept(key),
    ) ?? null
  );
}

/** The machine a scoped page means: the address's, else the one last
 * chosen, else the default, else the first. */
export function selectedNode(
  state: FleetState,
  fromUrl: string | undefined,
): NodeKey | null {
  return machineMeant(state.book ?? [], [fromUrl, state.selected]);
}
