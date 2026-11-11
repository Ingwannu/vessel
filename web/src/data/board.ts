// The Sessions board (DESIGN.md §6.12; ADR 0018): sessions as cards in
// lanes, one lane per reason a session is where it is.
//
// [Decision Log]
// - Purpose: an operator supervising several agents needs "what is
//   running, what stopped and why" at a glance; a table sorted by start
//   time makes them read every row's status badge to find out.
// - Constraint: the lanes must be facts the node reports (DESIGN.md §1.1,
//   "the daemon is the product"). Today `SessionInfo` carries only
//   `SessionStatus`; it does not say whether a structured session is in a
//   turn or waiting on a permission answer.
// - Alternatives: lanes by attention (working / needs you / idle), as
//   agent-orchestrator's board has. The page could only get those by
//   replaying every live session's event log, which costs a full log read
//   per card and is a guess for terminal sessions. Rejected until the node
//   reports turn and permission state itself (a protocol change, listed as
//   a follow-up in the ADR).
// - Chosen: one lane per `SessionStatus`, with `starting` in the running
//   lane (it lasts seconds and would be an almost always empty lane).
// - Must stay true: `laneOf` is the only place a status becomes a lane,
//   and it matches exhaustively, so a new status does not compile until it
//   has a lane. When the node reports activity, lanes change here and
//   nowhere else.

import type { SessionStatus } from "../wire/proto";
import type { Row } from "./tree";

/** A lane of the board, named by the protocol's own status word. */
export type Lane = "running" | "interrupted" | "failed" | "exited";

/** The lanes left to right: live work first, then what stopped and may
 * need a look, then what finished. */
export const LANES: readonly Lane[] = [
  "running",
  "interrupted",
  "failed",
  "exited",
];

/** How many cards a lane shows before "Show more": ended lanes grow
 * without bound, since the node keeps every session's log. */
export const LANE_PAGE = 20;

/** The lane a status belongs to. */
export function laneOf(status: SessionStatus): Lane {
  switch (status.state) {
    case "starting":
    case "running":
      return "running";
    case "interrupted":
      return "interrupted";
    case "failed":
      return "failed";
    case "exited":
      return "exited";
  }
}

/** A lane's heading. */
export function laneLabel(lane: Lane): string {
  switch (lane) {
    case "running":
      return "Running";
    case "interrupted":
      return "Interrupted";
    case "failed":
      return "Failed";
    case "exited":
      return "Exited";
  }
}

/** The status-dot class of a lane's heading (ui/parts `StatusDot`). */
export function laneTone(lane: Lane): string {
  switch (lane) {
    case "running":
      return "running";
    case "interrupted":
      return "interrupt";
    case "failed":
      return "danger";
    case "exited":
      return "offline";
  }
}

/** A session on the board: its tree row, and how many of its ancestors
 * sit directly above it in the same lane (0 to 2). */
export interface Card extends Row {
  indent: 0 | 1 | 2;
}

/** `rows` (in tree order, `data/tree.ts`) dealt into lanes.
 *
 * Tree order is kept inside a lane, so a session an agent started still
 * follows the session that started it when both are in the lane, indented
 * under it. When its parent is in another lane the card is not indented;
 * the card's own "agent of" line still names the parent (ADR 0013's
 * attribution must not depend on two sessions sharing a status). */
export function board(rows: readonly Row[]): Record<Lane, Card[]> {
  const lanes: Record<Lane, Card[]> = {
    running: [],
    interrupted: [],
    failed: [],
    exited: [],
  };
  // Parents come before their children in tree order, so a parent's
  // indent is known when its child is placed.
  const placed = new Map<string, { lane: Lane; indent: 0 | 1 | 2 }>();
  for (const row of rows) {
    const lane = laneOf(row.session.status);
    const above = row.parent === null ? undefined : placed.get(row.parent.id);
    const indent = above?.lane !== lane ? 0 : above.indent === 0 ? 1 : 2;
    placed.set(row.session.id, { lane, indent });
    lanes[lane].push({ ...row, indent });
  }
  return lanes;
}
