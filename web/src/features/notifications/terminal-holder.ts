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
import type { SessionId } from "../../wire/proto";

/** The session that holds `notice`'s terminal, or `null` when none does. */
export async function holderOf(
  fleet: Fleet,
  notice: TerminalNotice,
): Promise<SessionId | null> {
  const sessions = fleet.store.get().sessions[notice.node] ?? [];
  const found = sessions.find((session) => holds(session, notice.place));
  if (found !== undefined || notice.place.kind !== "folder") {
    return found?.id ?? null;
  }
  for (const candidate of folderSessions(sessions)) {
    const listed = await attempt(() =>
      fleet.bridge.call(notice.node, "terminal.list", {
        place: { kind: "folder", path: candidate.path },
        changed_from: null,
      }),
    );
    if (
      listed.ok &&
      listed.value.terminals.some((info) => info.id === notice.terminal)
    ) {
      return candidate.session;
    }
  }
  return null;
}
