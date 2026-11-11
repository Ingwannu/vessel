// The Sessions board (DESIGN.md §6.12; ADRs 0018 and 0019): sessions as
// cards in the lanes agent-orchestrator's board has.
//
// [Decision Log]
// - Purpose: say, for each session, which lane it is in and what its card
//   reads, the same on every client.
// - Constraint: both are decided by the node (protocol 8,
//   `SessionInfo.board`), from facts only it has: whether a turn runs,
//   whether a permission waits, later the state of a pull request. The
//   page must not decide them again; milestone 8's lanes by
//   `SessionStatus` were a stand-in for a node that reported nothing more.
// - A node older than protocol 8 sends no placement. Its sessions keep
//   the one place their status implies (live: Building, ended: archive)
//   and their card reads the protocol's own status word, since nothing
//   says what the agent is doing. agent-orchestrator's client falls back
//   the same way for an old daemon.
// - Must stay true: `placement` is the only place a session gets a lane,
//   and every `DisplayStatus` and `KanbanColumn` is matched exhaustively,
//   so a value the node learns to send does not compile here until it has
//   a label, a lane and a colour.

import type {
  Activity,
  DisplayStatus,
  KanbanColumn,
  SessionInfo,
  SessionStatus,
} from "../wire/proto";
import { needsYou } from "./phrase";
import type { Row } from "./tree";

/** A lane of the board: every column but the archive, which is listed
 * apart. */
export type Lane = Exclude<KanbanColumn, "archive">;

/** The lanes in delivery order. The middle two are one review loop seen
 * from either side: validating while the fleet turns it, in review while
 * a person does. */
export const LANES: readonly Lane[] = [
  "building",
  "validating",
  "needs_review",
  "ready",
];

/** What a card reads: the node's phrase, or, from a node that sends none,
 * the session's status. */
export type Phrase =
  | { readonly from: "node"; readonly status: DisplayStatus }
  | { readonly from: "status"; readonly status: SessionStatus };

/** Where a session is on the board. */
export interface Placement {
  readonly column: KanbanColumn;
  readonly phrase: Phrase;
  /** What its agent was last seen doing; `null` from an older node. */
  readonly activity: Activity | null;
  /** When it last did something (ISO 8601): what a lane sorts by. */
  readonly at: string;
}

/** The session's placement: the node's, or the one its status implies. */
export function placement(session: SessionInfo): Placement {
  const board = session.board;
  if (board !== undefined && board !== null) {
    return {
      column: board.column,
      phrase: { from: "node", status: board.display_status },
      activity: board.activity,
      at: board.last_activity_at,
    };
  }
  const live =
    session.status.state === "starting" || session.status.state === "running";
  return {
    column: live ? "building" : "archive",
    phrase: { from: "status", status: session.status },
    activity: null,
    at: session.ended_at ?? session.created_at,
  };
}

/** A lane's heading. */
export function laneLabel(lane: Lane): string {
  switch (lane) {
    case "building":
      return "Building";
    case "validating":
      return "Validating";
    case "needs_review":
      return "In review";
    case "ready":
      return "Ready";
  }
}

/** A session on the board: its tree row (who started it) and its place. */
export interface Card extends Row {
  readonly placed: Placement;
}

/** The board: each lane's cards, and the archive. */
export interface Board {
  readonly lanes: Readonly<Record<Lane, readonly Card[]>>;
  readonly archive: readonly Card[];
}

/** `rows` dealt into lanes and the archive.
 *
 * Inside a lane the sessions that wait on their owner come first, then
 * the most recently active, as on agent-orchestrator's board. The archive
 * is newest first. The tree's order is not kept: a card names the session
 * that started it instead (ADR 0013's attribution must not depend on two
 * sessions sharing a lane). */
export function board(rows: readonly Row[]): Board {
  const lanes: Record<Lane, Card[]> = {
    building: [],
    validating: [],
    needs_review: [],
    ready: [],
  };
  const archive: Card[] = [];
  for (const row of rows) {
    const placed = placement(row.session);
    const card: Card = { ...row, placed };
    if (placed.column === "archive") {
      archive.push(card);
    } else {
      lanes[placed.column].push(card);
    }
  }
  const newest = (one: Card, two: Card) =>
    two.placed.at.localeCompare(one.placed.at);
  for (const lane of LANES) {
    // Stable: needs-you first, each part still newest first.
    lanes[lane].sort(newest);
    lanes[lane].sort(
      (one, two) => Number(needsYou(two.placed)) - Number(needsYou(one.placed)),
    );
  }
  archive.sort(newest);
  return { lanes, archive };
}

/** "just now", "6m ago", "3h ago", "5d ago": how long since `iso`, as a
 * card's footer writes it. Never weeks or months: a board is about now. */
export function sinceLabel(iso: string, now: number = Date.now()): string {
  const minutes = Math.floor((now - Date.parse(iso)) / 60_000);
  if (!Number.isFinite(minutes) || minutes < 1) {
    return "just now";
  }
  if (minutes < 60) {
    return `${minutes}m ago`;
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours}h ago`;
  }
  return `${Math.floor(hours / 24)}d ago`;
}
