// What the sidebar lists under "Machines" (agent-orchestrator's project
// tree, with machines where it has projects): each machine with its state
// and its live sessions, by title, with the mark that says whether the
// session waits on its owner or its agent is working.

import type { NodeKey } from "../wire/bridge";
import type { SessionInfo } from "../wire/proto";
import { isWorking, needsYou, placement } from "./board";
import type { FleetState } from "./fleet";
import { isLive, nodeState, type StateTone } from "./format";
import { nodeLabel } from "./nodes";
import { sessionName } from "./titles";

/** The mark a session's row leads with. */
export type SessionMark = "needs-you" | "working" | "idle";

/** A live session's row. */
export interface SidebarSession {
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
  /** The first of them, newest first. */
  sessions: readonly SidebarSession[];
  /** How many more there are than `sessions` shows. */
  more: number;
}

/** How many of a machine's live sessions the sidebar lists before it
 * links to the board for the rest: the sidebar is a way to switch, not a
 * second board. */
export const SIDEBAR_SESSIONS = 5;

/** A session's mark, from its node's own placement (protocol 8); one of
 * an older node is idle, since nothing says what it is doing. */
export function sessionMark(session: SessionInfo): SessionMark {
  const placed = placement(session);
  if (needsYou(placed)) {
    return "needs-you";
  }
  return isWorking(placed) ? "working" : "idle";
}

/** Every machine of the book, in its order, with its live sessions. */
export function sidebarMachines(
  state: FleetState,
  limit: number = SIDEBAR_SESSIONS,
): SidebarMachine[] {
  return (state.book ?? []).map((node) => {
    const status = state.statuses[node.key];
    const named = nodeState(status);
    const live = (state.sessions[node.key] ?? [])
      .filter(isLive)
      .slice()
      .sort((a, b) => b.created_at.localeCompare(a.created_at));
    return {
      key: node.key,
      label: nodeLabel(node, status),
      tone: named.tone,
      word: named.word,
      connected: status?.state === "connected",
      live: live.length,
      sessions: live.slice(0, limit).map((session) => ({
        id: session.id,
        title: sessionName(session),
        mark: sessionMark(session),
      })),
      more: Math.max(0, live.length - limit),
    };
  });
}
