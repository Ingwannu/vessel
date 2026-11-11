// What the command palette lists (DESIGN.md §2.2): the places, the nodes
// in the book, every session the page knows, and the page's own
// commands. A pure function of the fleet's state, so what the palette
// offers is decided here and tested, and the palette only draws it.

import type { FleetState } from "./fleet";
import { isLive, sessionTitle, shortId, statusWord } from "./format";
import { nodeLabel } from "./nodes";

/** The palette's groups, in the order it lists them. */
export type PaletteGroup = "Go to" | "Nodes" | "Sessions" | "Actions";

/** What choosing an entry does. */
export type PaletteTarget =
  { kind: "route"; to: string } | { kind: "new-session"; node: string | null };

/** The places a palette entry can name, for its icon. */
export type PalettePlace =
  | "fleet"
  | "sessions"
  | "sync"
  | "devices"
  | "settings"
  | "node"
  | "terminal"
  | "structured"
  | "new";

export interface PaletteEntry {
  /** Unique in one listing. */
  id: string;
  label: string;
  detail?: string;
  /** Searched with the label (a session's full id). */
  keywords?: string;
  group: PaletteGroup;
  place: PalettePlace;
  target: PaletteTarget;
}

const PLACES: readonly PaletteEntry[] = [
  {
    id: "go-fleet",
    label: "Fleet",
    group: "Go to",
    place: "fleet",
    target: { kind: "route", to: "/fleet" },
  },
  {
    id: "go-sessions",
    label: "Sessions",
    group: "Go to",
    place: "sessions",
    target: { kind: "route", to: "/sessions" },
  },
  {
    id: "go-sync",
    label: "Sync",
    group: "Go to",
    place: "sync",
    target: { kind: "route", to: "/sync" },
  },
  {
    id: "go-devices",
    label: "Devices",
    group: "Go to",
    place: "devices",
    target: { kind: "route", to: "/devices" },
  },
  {
    id: "go-settings",
    label: "Settings",
    group: "Go to",
    place: "settings",
    target: { kind: "route", to: "/settings" },
  },
];

/** Everything the palette lists for `state`, `selected` being the node a
 * new session starts on. Live sessions come before ended ones, each
 * newest first, so the one being worked on is near the top. */
export function paletteEntries(
  state: FleetState,
  selected: string | null,
): PaletteEntry[] {
  const nodes: PaletteEntry[] = (state.book ?? []).map((node) => ({
    id: `node-${node.key}`,
    label: nodeLabel(node, state.statuses[node.key]),
    detail: node.address,
    group: "Nodes",
    place: "node",
    target: { kind: "route", to: `/fleet/${node.key}` },
  }));
  const sessions = Object.entries(state.sessions)
    .flatMap(([node, list]) => list.map((session) => ({ node, session })))
    .sort(
      (a, b) =>
        Number(isLive(b.session)) - Number(isLive(a.session)) ||
        b.session.created_at.localeCompare(a.session.created_at),
    )
    .map(({ node, session }): PaletteEntry => ({
      id: `session-${node}-${session.id}`,
      label: `${sessionTitle(session.spec.cwd)} ${shortId(session.id)}`,
      detail: `${statusWord(session.status)} · ${session.spec.cwd ?? "~"}`,
      keywords: session.id,
      group: "Sessions",
      place: session.spec.mode === "terminal" ? "terminal" : "structured",
      target: { kind: "route", to: `/sessions/${node}/${session.id}` },
    }));
  const actions: PaletteEntry[] = [
    {
      id: "new-session",
      label: "New session",
      group: "Actions",
      place: "new",
      target: { kind: "new-session", node: selected },
    },
  ];
  return [...actions, ...PLACES, ...nodes, ...sessions];
}
