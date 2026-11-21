// A terminal's agent asking for the person, or done: a notice, claimed by its
// exact attention revision before it shows (ADR 0089; ADR 0114 "Terminal
// notices"). The claims follow-up's third row, page part; the app's
// `vessel_app_view::notices::terminals` holds the same rules.
//
// [Decision Log]
// - Purpose: Paseo tells one client "Terminal finished" or "Terminal needs
//   input" with the terminal's name, and a tap opens the terminal
//   (`websocket-server.ts:213-215,2955-3044`; the app's
//   `session-context.tsx:658-677`).
// - Constraint: behind ADR 0089's constant. Until it is assigned the node
//   fills no terminal's attention and no machine admits claims (`claimsOn`),
//   so nothing here runs.
// - Source: the machine's whole terminal list (`terminal.list`, no place),
//   waited on with ADR 0116's `changed_from`. Once assigned, the node moves
//   that list's revision when a terminal's attention changes.
// - A connection's first list tells nothing (attention already there was an
//   earlier event). Later, a terminal at Finished or Needs input at a
//   revision not told before is a notice; a failed terminal never is.
// - Only an admitted claim shows: a terminal has no inbox to fall back on.
// - The text names the terminal's title, never its command (ADR 0089).

import type { NodeKey } from "../wire/bridge";
import type {
  ClearTerminalAttention,
  NotificationSource,
  SessionInfo,
  TerminalAttentionRevision,
  TerminalId,
  TerminalInfo,
  TerminalPlace,
} from "../wire/proto";
import type { NoticeKind } from "./notices";
import { sessionRoot } from "./workspace/root";

/** A terminal that started to need the person, or finished. */
export interface TerminalNotice {
  kind: NoticeKind;
  node: NodeKey;
  terminal: TerminalId;
  revision: TerminalAttentionRevision;
  place: TerminalPlace;
  /** What a person called it; empty when untitled. */
  title: string;
}

/** What a claim names: this exact revision, never just the terminal. */
export function terminalSource(notice: TerminalNotice): NotificationSource {
  return {
    target: "terminal",
    terminal: notice.terminal,
    revision: notice.revision,
  };
}

/** The revision and kind `info` asks attention at, if it does. */
function asking(
  info: TerminalInfo,
): { revision: TerminalAttentionRevision; kind: NoticeKind } | null {
  const attention = info.attention ?? null;
  if (info.status.state === "failed" || attention === null) {
    return null;
  }
  const { activity, revision } = attention;
  if (activity.state !== "attention") {
    return null;
  }
  switch (activity.reason) {
    case "finished":
      return { revision, kind: "finished" };
    case "needs_input":
      return { revision, kind: "needs-you" };
    case "unknown":
      return null;
  }
}

/** One connection's terminal lists, as read: what it has told of. `null`
 * before its first list. */
export type Watched = ReadonlyMap<TerminalId, TerminalAttentionRevision> | null;

/** A new list of `node`'s terminals: the notices it brings, and what the
 * connection has now seen. The first list brings none. */
export function readTerminals(
  watched: Watched,
  node: NodeKey,
  list: readonly TerminalInfo[],
): { notices: TerminalNotice[]; watched: Watched } {
  const now = new Map<TerminalId, TerminalAttentionRevision>();
  for (const info of list) {
    if (info.attention !== null && info.attention !== undefined) {
      now.set(info.id, info.attention.revision);
    }
  }
  if (watched === null) {
    return { notices: [], watched: now };
  }
  const notices = list.flatMap((info): TerminalNotice[] => {
    const asked = asking(info);
    if (asked === null || watched.get(info.id) === asked.revision) {
      return [];
    }
    return [
      {
        kind: asked.kind,
        node,
        terminal: info.id,
        revision: asked.revision,
        place: info.place,
        title: info.title ?? "",
      },
    ];
  });
  return { notices, watched: now };
}

/** What viewing `info` clears: its attention at the exact revision shown,
 * while it asks for the person. A stale view never clears a newer one. */
export function toClear(info: TerminalInfo): ClearTerminalAttention | null {
  const attention = info.attention ?? null;
  if (attention?.activity.state !== "attention") {
    return null;
  }
  return { terminal: info.id, revision: attention.revision };
}

/** Whether `session`'s workspace holds `place`, so opening the session
 * shows the terminal among its tabs (`workspace/terminal-place.ts`): the
 * session itself, its project or worktree, or its folder when it has no
 * root. */
export function holds(session: SessionInfo, place: TerminalPlace): boolean {
  const root = sessionRoot(session);
  switch (place.kind) {
    case "session":
      return session.id === place.session;
    case "project":
      return root?.kind === "project" && root.project === place.project;
    case "worktree":
      return root?.kind === "worktree" && root.worktree === place.worktree;
    case "folder":
      return root === null && session.spec.cwd === place.path;
  }
}

/** A terminal notice's title, in Paseo's words. */
export function terminalNoticeTitle(kind: NoticeKind): string {
  switch (kind) {
    case "needs-you":
      return "Terminal needs input";
    case "finished":
      return "Terminal finished";
  }
}
