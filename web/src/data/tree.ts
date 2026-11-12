// Sessions as a tree (DESIGN.md §2.4; ADR 0013): a session an agent
// started nests under the session whose agent started it, on whichever
// node either runs. The board (data/board.ts) deals these rows into lanes
// and keeps this order inside each.

import type { SessionInfo } from "../wire/proto";
import type { FleetState } from "./fleet";
import type { NodeKey } from "../wire/bridge";

/** A session with the node it runs on and its place in the tree. */
export interface Row {
  node: NodeKey;
  session: SessionInfo;
  depth: number;
  parent: SessionInfo | null;
  /** The node `parent` runs on: a child may run on another node than the
   * session that started it. */
  parentNode: NodeKey | null;
}

/** Every session of `nodes` as tree rows: roots (started by a person)
 * newest first, each followed by its agent-started children, depth first.
 * A child whose parent is not listed is a root. */
export function tree(state: FleetState, nodes: readonly NodeKey[]): Row[] {
  const all: { node: NodeKey; session: SessionInfo }[] = nodes.flatMap((node) =>
    (state.sessions[node] ?? []).map((session) => ({ node, session })),
  );
  const byId = new Map(all.map((entry) => [entry.session.id, entry]));
  const children = new Map<string, { node: NodeKey; session: SessionInfo }[]>();
  const roots: { node: NodeKey; session: SessionInfo }[] = [];
  for (const entry of all) {
    const creator = entry.session.created_by;
    if (creator.kind === "agent" && byId.has(creator.session)) {
      const list = children.get(creator.session) ?? [];
      list.push(entry);
      children.set(creator.session, list);
    } else {
      roots.push(entry);
    }
  }
  const newest = (
    one: { session: SessionInfo },
    two: { session: SessionInfo },
  ) => two.session.created_at.localeCompare(one.session.created_at);
  const rows: Row[] = [];
  const visit = (
    entry: { node: NodeKey; session: SessionInfo },
    depth: number,
    parent: { node: NodeKey; session: SessionInfo } | null,
  ) => {
    rows.push({
      ...entry,
      depth,
      parent: parent?.session ?? null,
      parentNode: parent?.node ?? null,
    });
    for (const child of (children.get(entry.session.id) ?? []).sort(newest)) {
      visit(child, depth + 1, entry);
    }
  };
  for (const root of roots.sort(newest)) {
    visit(root, 0, null);
  }
  return rows;
}
