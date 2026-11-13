// A session's marks and the actions on them (milestone 11, Track A-page;
// ADR 0026): pinned, archived, unread; pin, archive, bring back, start
// again, delete, mark read or unread; and when the page marks a session
// read on its own.
//
// [Decision Log]
// - Purpose: agent-orchestrator, GOAT and Paseo pin, archive, restore
//   and delete sessions and show which ones the person has not seen.
//   Vessel's node keeps those marks (protocol 10), so every client agrees.
// - Constraint: what is offered is what the node takes: marks need
//   protocol 10; start again needs an ended structured session whose
//   agent had a session to load and did not say it cannot resume; delete
//   needs an ended session. `read` names the record the reader has not
//   seen, and the node counts unread by records asking for attention.
// - Choice: the actions as a pure list per session (the title bar's menu
//   and the archive draw from it); `read` sent when the session is in
//   front and has shown a record that asks for attention at or after the
//   last `through` sent, or the list says unread, never more than once a
//   second; after Mark unread, not until a record comes after it (else
//   the open page would undo the person's mark at once).

import type { KeptReason, SessionDeleted, SessionInfo } from "../wire/proto";
import { hasConversation } from "./delivery";
import { isLive } from "./format";

/** Whether the node keeps `session` archived. */
export function isArchived(session: SessionInfo): boolean {
  const at = session.marks?.archived_at;
  return at !== undefined && at !== null;
}

export function isPinned(session: SessionInfo): boolean {
  return session.marks?.pinned === true;
}

export function isUnread(session: SessionInfo): boolean {
  return session.marks?.unread === true;
}

/** Whether `session` can be started again: ended, with a conversation,
 * whose agent had a session to load and did not say it cannot. A
 * session stored before protocol 10 has no recorded abilities: the node's
 * answer decides. */
export function canStartAgain(session: SessionInfo): boolean {
  if (isLive(session) || session.spec.mode !== "structured") {
    return false;
  }
  const loaded = session.harness_session;
  if (loaded === undefined || loaded === null) {
    return false;
  }
  return session.abilities?.resume !== false;
}

/** An action on a session's marks or life. */
export type MarkAction =
  | "pin"
  | "unpin"
  | "mark-unread"
  | "mark-read"
  | "archive"
  | "unarchive"
  | "start-again"
  | "delete";

/** The actions `session` offers on a connection speaking `protocol`, in
 * the menu's order. */
export function markActions(
  session: SessionInfo,
  protocol: number | null,
): MarkAction[] {
  if (!hasConversation(protocol)) {
    return [];
  }
  const actions: MarkAction[] = [isPinned(session) ? "unpin" : "pin"];
  actions.push(isUnread(session) ? "mark-read" : "mark-unread");
  if (canStartAgain(session)) {
    actions.push("start-again");
  }
  actions.push(isArchived(session) ? "unarchive" : "archive");
  if (!isLive(session)) {
    actions.push("delete");
  }
  return actions;
}

/** What the page knows when it decides whether to mark read. */
export interface ReadView {
  /** The session is in front: its page shown, the tab visible and
   * focused. */
  front: boolean;
  /** The list says the session is unread. */
  unread: boolean;
  /** The last record that asks for attention (`Timeline.attention`). */
  attention: number;
  /** The record after the last one shown (`Timeline.next`). */
  next: number;
  /** The last `through` sent; -1 before one. */
  sent: number;
  /** The `next` when the person marked it unread, if they did: no read
   * until a record comes after it. */
  heldAt: number | null;
}

/** The `through` to send now, or `null` for none. */
export function readThrough(view: ReadView): number | null {
  if (!view.front || view.next <= view.sent || view.next === 0) {
    return null;
  }
  if (view.heldAt !== null && view.next <= view.heldAt) {
    return null;
  }
  const asked =
    (view.unread && view.heldAt === null) ||
    view.attention >= Math.max(0, view.sent);
  return asked ? view.next : null;
}

/** Why a worktree was kept, in the page's words. */
export function keptReason(reason: KeptReason): string {
  switch (reason.kind) {
    case "uncommitted":
      return `${String(reason.files)} uncommitted ${reason.files === 1 ? "file" : "files"}`;
    case "unique_commits":
      return `${String(reason.commits)} ${reason.commits === 1 ? "commit" : "commits"} no other branch has`;
    case "operation":
      return `a ${reason.operation.replace("_", " ")} in progress`;
    case "submodules":
      return "submodules";
    case "flagged_files":
      return "files git was told to keep";
    case "worktree_refs":
      return "refs of its own";
    case "in_use":
      return "another live session works in it";
    case "inspection_failed":
      return "git could not say what it holds";
    case "unknown":
      return "a reason this page does not know";
  }
}

/** What a delete's answer says became of the session's worktree, or
 * `null` when it had none. */
export function deletedLine(deleted: SessionDeleted): string | null {
  const worktree = deleted.worktree;
  if (worktree === undefined || worktree === null) {
    return null;
  }
  switch (worktree.outcome) {
    case "removed":
      return "Its worktree was removed.";
    case "kept":
      return `Its worktree was kept: ${worktree.reasons.map(keptReason).join(", ")}.`;
  }
}

/** The protocol from which archiving a session also ends the terminals
 * and services beside it (the node's join of wave 2's first half). */
const ENDS_BESIDE = 11;

/** What the archive's confirmation says it does: stop the agent first on
 * `machine` when the session is live, end what runs beside it on a node
 * that has terminals (`protocol`, `null` when not known), and that it can
 * be restored. */
export function archiveWords(
  live: boolean,
  machine: string,
  protocol: number | null,
): string {
  const stops = live ? `Archiving stops the agent on ${machine} first. ` : "";
  const beside =
    protocol !== null && protocol >= ENDS_BESIDE
      ? "The terminals and services running beside it end too; their output is kept. "
      : "";
  return `${stops}${beside}You can always restore it from the Archive section later.`;
}
