// What the sidebar lists (agent-orchestrator's project tree, with machines
// where it has projects): sessions by machine or by what they are doing,
// in the order the person chose, with the mark that says whether a
// session waits on its owner or its agent is working.
//
// [Decision Log] The sidebar's view (milestone 11, track W)
// - Purpose: the owner asked for the references' sidebar grouping,
//   sorting and hiding. Paseo's "Display" menu groups by project or by
//   status and sorts by creation or by the last prompt; GOAT sorts by the
//   last user message or creation; all three leave archived sessions out.
// - Choice: grouping by machine (Vessel's tree, the default) or by status
//   (Paseo's buckets in the board's words: Needs you, Working, Idle,
//   Ended); sorting by creation (the default) or by last activity (the
//   node's `last_activity_at`, ADR 0019); ended sessions hidden unless
//   asked for, as now. Kept with the bridge (`data/prefs.ts`).
// - The flat order the sidebar shows is also what "session 1 to 9" and
//   "next/previous session" walk (`sidebarOrder`), so a key goes where
//   the eye already is.

import type { NodeKey } from "../wire/bridge";
import type { SessionInfo } from "../wire/proto";
import { placement } from "./board";
import { isWorking, needsYou } from "./phrase";
import type { FleetState } from "./fleet";
import { isLive, type StateTone } from "./format";
import { machineState } from "./machines";
import { nodeLabel } from "./nodes";
import { sessionName } from "./titles";

/** The mark a session's row leads with. */
export type SessionMark = "needs-you" | "working" | "idle" | "ended";

/** How the sidebar groups sessions. */
export type SidebarGrouping = "machine" | "status";

/** How it orders them, newest first either way. */
export type SidebarSort = "created" | "activity";

/** What the person chose (`data/prefs.ts` keeps it). */
export interface SidebarView {
  group: SidebarGrouping;
  sort: SidebarSort;
  /** Whether ended sessions are listed. */
  ended: boolean;
}

/** The sidebar as milestone 10 drew it. */
export const DEFAULT_VIEW: SidebarView = {
  group: "machine",
  sort: "created",
  ended: false,
};

/** A session's row. */
export interface SidebarSession {
  node: NodeKey;
  id: string;
  title: string;
  mark: SessionMark;
}

/** A machine's row and the sessions under it. */
export interface SidebarMachine {
  key: NodeKey;
  label: string;
  tone: StateTone;
  /** Its state in a word, for the dot's name. */
  word: string;
  connected: boolean;
  /** How many live sessions it has. */
  live: number;
  /** The first of the listed ones. */
  sessions: readonly SidebarSession[];
  /** How many more there are than `sessions` shows. */
  more: number;
}

/** One status group and its sessions (grouping by status). */
export interface SidebarStatus {
  mark: SessionMark;
  label: string;
  sessions: readonly (SidebarSession & { machine: string })[];
  more: number;
}

/** How many of a machine's sessions the sidebar lists before it links to
 * the board for the rest: the sidebar is a way to switch, not a second
 * board. */
export const SIDEBAR_SESSIONS = 5;

/** How many of a status group's (agent-orchestrator's section limit). */
export const SIDEBAR_STATUS_SESSIONS = 10;

/** A session's mark, from its node's own placement (protocol 8); one of
 * an older node is idle while it runs, since nothing says what it is
 * doing. */
export function sessionMark(session: SessionInfo): SessionMark {
  if (!isLive(session)) {
    return "ended";
  }
  const placed = placement(session);
  if (needsYou(placed)) {
    return "needs-you";
  }
  return isWorking(placed) ? "working" : "idle";
}

/** When `session` last did something, for sorting by activity. */
function activeAt(session: SessionInfo): string {
  return (
    session.board?.last_activity_at ?? session.ended_at ?? session.created_at
  );
}

/** `sessions` as the view lists them: ended ones only when asked for,
 * newest first by the chosen time. */
export function listed(
  sessions: readonly SessionInfo[],
  view: SidebarView,
): SessionInfo[] {
  const key =
    view.sort === "activity"
      ? activeAt
      : (session: SessionInfo) => session.created_at;
  return sessions
    .filter((session) => view.ended || isLive(session))
    .sort((a, b) => key(b).localeCompare(key(a)));
}

function row(node: NodeKey, session: SessionInfo): SidebarSession {
  return {
    node,
    id: session.id,
    title: sessionName(session),
    mark: sessionMark(session),
  };
}

/** Every machine of the book, in its order, with its listed sessions. */
export function sidebarMachines(
  state: FleetState,
  view: SidebarView = DEFAULT_VIEW,
  limit: number = SIDEBAR_SESSIONS,
): SidebarMachine[] {
  return (state.book ?? []).map((node) => {
    const status = state.statuses[node.key];
    const named = machineState(status);
    const all = state.sessions[node.key] ?? [];
    const shown = listed(all, view);
    return {
      key: node.key,
      label: nodeLabel(node, status),
      tone: named.tone,
      word: named.word,
      connected: status?.state === "connected",
      live: all.filter(isLive).length,
      sessions: shown.slice(0, limit).map((session) => row(node.key, session)),
      more: Math.max(0, shown.length - limit),
    };
  });
}

/** The status groups, in the order they are listed. */
const STATUSES: readonly { mark: SessionMark; label: string }[] = [
  { mark: "needs-you", label: "Needs you" },
  { mark: "working", label: "Working" },
  { mark: "idle", label: "Idle" },
  { mark: "ended", label: "Ended" },
];

/** Every listed session of every machine, grouped by what it is doing;
 * a group with none is left out. */
export function sidebarStatuses(
  state: FleetState,
  view: SidebarView,
  limit: number = SIDEBAR_STATUS_SESSIONS,
): SidebarStatus[] {
  const everything = (state.book ?? []).flatMap((node) =>
    (state.sessions[node.key] ?? []).map((session) => ({
      node: node.key,
      machine: nodeLabel(node, state.statuses[node.key]),
      session,
    })),
  );
  const order = new Map(
    listed(
      everything.map((item) => item.session),
      view,
    ).map((session, index) => [session, index]),
  );
  const shown = everything
    .filter((item) => order.has(item.session))
    .sort((a, b) => (order.get(a.session) ?? 0) - (order.get(b.session) ?? 0));
  return STATUSES.map(({ mark, label }) => {
    const rows = shown
      .filter((item) => sessionMark(item.session) === mark)
      .map((item) => ({
        ...row(item.node, item.session),
        machine: item.machine,
      }));
    return {
      mark,
      label,
      sessions: rows.slice(0, limit),
      more: Math.max(0, rows.length - limit),
    };
  }).filter((group) => group.sessions.length > 0);
}

/** The sessions as the sidebar shows them, top to bottom: what "session
 * 1 to 9" counts and "next" and "previous" walk. */
export function sidebarOrder(
  state: FleetState,
  view: SidebarView,
): SidebarSession[] {
  return view.group === "machine"
    ? sidebarMachines(state, view).flatMap((machine) => machine.sessions)
    : sidebarStatuses(state, view).flatMap((group) => group.sessions);
}

/** The session `by` places from `current` in `order`, wrapping; the
 * first (or last) when `current` is not in it. */
export function stepSession(
  order: readonly SidebarSession[],
  current: { node: string; id: string } | null,
  by: 1 | -1,
): SidebarSession | null {
  if (order.length === 0) {
    return null;
  }
  const at = order.findIndex(
    (session) => session.node === current?.node && session.id === current.id,
  );
  if (at === -1) {
    return (by === 1 ? order[0] : order.at(-1)) ?? null;
  }
  return order[(at + by + order.length) % order.length] ?? null;
}
