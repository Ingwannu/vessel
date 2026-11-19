// When the page tells the person about a session it is not showing
// (DESIGN.md §7.7): a session starts to need them, or finishes its work.
//
// [Decision Log]
// - Purpose: the owner asked for the browser's notification, a sound and
//   the tab's title when a session starts to wait on the user or finishes
//   while the page is not in front (all three references notify; GOAT's
//   page is the one that does it from a browser).
// - Constraint: what a session is doing is the board's placement the
//   node reports (protocol 8, ADR 0019); the page sees it by polling each
//   machine's list. A notice is a change between two polls, never a
//   state: a session already waiting when the page first sees it is not
//   news (GOAT: "first sighting … never notify").
// - Choice: "needs you" is `needsYou` (blocked: a permission or a
//   question waits); "finished" is a session that was working and now is
//   neither working nor waiting (its turn ended, or it exited). A session
//   of an older node, placed from its status, needs nobody and finishes
//   when it ends.
// - Cost: a poll misses what starts and ends between two polls; the
//   polls are two seconds apart, and slower while the tab is hidden
//   (the browser throttles timers), so a notice can come late.

import type { NodeKey } from "../wire/bridge";
import type { SessionInfo } from "../wire/proto";
import { placement } from "./board";
import { isWorking, needsYou } from "./phrase";

/** Why the page tells the person. */
export type NoticeKind = "needs-you" | "finished";

/** One thing to tell. */
export interface Notice {
  kind: NoticeKind;
  node: NodeKey;
  session: SessionInfo;
}

/** What a session was doing, as far as notices care. */
type Doing = "needs-you" | "working" | "resting";

function doing(session: SessionInfo): Doing {
  const placed = placement(session);
  if (needsYou(placed)) {
    return "needs-you";
  }
  if (isWorking(placed)) {
    return "working";
  }
  const live =
    session.status.state === "starting" || session.status.state === "running";
  // An older node's session has no placement: running is working.
  return placed.phrase.from === "status" && live ? "working" : "resting";
}

/** The notices between two polls of `node`'s sessions. Sessions not in
 * `before` are first sightings and say nothing. */
export function notices(
  node: NodeKey,
  before: readonly SessionInfo[],
  after: readonly SessionInfo[],
): Notice[] {
  const was = new Map(before.map((session) => [session.id, doing(session)]));
  const found: Notice[] = [];
  for (const session of after) {
    const then = was.get(session.id);
    if (then === undefined) {
      continue;
    }
    const now = doing(session);
    if (now === "needs-you" && then !== "needs-you") {
      found.push({ kind: "needs-you", node, session });
    } else if (
      now === "resting" &&
      then === "working" &&
      // A failure is not announced (Paseo never notifies an error, ADR
      // 0114): it stays on the card and in the inbox.
      session.status.state !== "failed"
    ) {
      found.push({ kind: "finished", node, session });
    }
  }
  return found;
}

/** Where the person is looking: whether the page is in front (visible
 * and focused), and the session it shows. */
export interface Seen {
  front: boolean;
  shown: { node: NodeKey; id: string } | null;
}

/** The notices to tell (ADR 0114, Paseo's `notifyAgentAttention`): all
 * but one for the session shown while the page is in front. In front on
 * another session, a notice is still told. */
export function told(found: readonly Notice[], seen: Seen): Notice[] {
  return found.filter(
    (notice) =>
      !(
        seen.front &&
        seen.shown !== null &&
        seen.shown.node === notice.node &&
        seen.shown.id === notice.session.id
      ),
  );
}

/** A notice's title, in GOAT's words with Vessel's "session". */
export function noticeTitle(kind: NoticeKind): string {
  switch (kind) {
    case "needs-you":
      return "Session needs you";
    case "finished":
      return "Session finished";
  }
}

/** The tab's title with `count` notices not yet seen: "(2) Vessel". */
export function titleWith(base: string, count: number): string {
  return count > 0 ? `(${String(count)}) ${base}` : base;
}
