// Which session holds a terminal a notice names (app review of claims row 3,
// note 2; the app's `vessel_app_link::terminal_holder` holds the Decision
// Log). A session whose workspace holds the terminal's place as the page
// reads it comes first (`holds`). For a folder place no session names as a
// path, each folder session's folder is listed in turn: the node matches a
// folder place canonically, as it keeps it (ADR 0121), and the first that
// lists the terminal is the holder. Paseo's daemon names the workspace in
// the notification itself (`session-context.tsx:658-677`, `workspaceId`).

import type { Fleet } from "../../data/fleet";
import { attempt } from "../../data/outcome";
import {
  folderSessions,
  holds,
  type TerminalNotice,
} from "../../data/terminal-notices";
import type { NodeKey } from "../../wire/bridge";
import type { SessionId, SessionInfo, TerminalInfo } from "../../wire/proto";

/** A folder's terminals on the notice's machine, matched as the node keeps
 * folders; `null` when the call fails. */
export type ListFolder = (
  path: string,
) => Promise<readonly TerminalInfo[] | null>;

/** The session of `sessions` (the notice's machine's) that holds
 * `notice`'s terminal, or `null` when none does. */
export async function holderOf(
  sessions: readonly SessionInfo[],
  notice: TerminalNotice,
  listFolder: ListFolder,
): Promise<SessionId | null> {
  const found = sessions.find((session) => holds(session, notice.place));
  if (found !== undefined || notice.place.kind !== "folder") {
    return found?.id ?? null;
  }
  for (const candidate of folderSessions(sessions)) {
    const terminals = await listFolder(candidate.path);
    if (terminals?.some((info) => info.id === notice.terminal) === true) {
      return candidate.session;
    }
  }
  return null;
}

/** [`ListFolder`] through the fleet's bridge, on `node`. */
export function folderLister(fleet: Fleet, node: NodeKey): ListFolder {
  return async (path) => {
    const listed = await attempt(() =>
      fleet.bridge.call(node, "terminal.list", {
        place: { kind: "folder", path },
        changed_from: null,
      }),
    );
    return listed.ok ? listed.value.terminals : null;
  };
}
