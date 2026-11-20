// Where a workspace's shells live (ADR 0121, TRACKER 829): Paseo keys a
// terminal by its workspace folder, so archiving one agent never ends a
// dev server another tab started.
//
// [Decision Log]
// - Purpose: from protocol 40 a worktree workspace opens its shells in
//   the worktree (`Worktree`) and an unregistered folder in the folder
//   (`Folder`), so neither needs an unarchived agent and neither ends
//   when an agent is archived. A project workspace keeps `Project`.
// - Constraint: a node below 40 cannot decode the new places (it refuses
//   them), so there the page keeps 817's rule: `Session` beside the first
//   live, unarchived session of the workspace, or no place at all.
// - Alternative: keep `Session` and only skip the end on archive (the
//   ADR's first alternative): an archived agent's place is refused for new
//   shells, so the workspace would still need an agent to open one.
// - Matching: the node lists a `Folder` place by its canonical path and a
//   `Worktree` place by id, so the page asks for the place it uses and
//   never compares paths itself.

import { isLive } from "../format";
import { isArchived } from "../marks";
import {
  WORKSPACE_TERMINALS_PROTOCOL,
  type SessionId,
  type SessionInfo,
  type TerminalInfo,
  type TerminalPlace,
  type WorkspaceRoot,
} from "../../wire/proto";

/** Whether a node at `protocol` takes the workspace places of ADR 0121. */
export function speaksWorkspacePlaces(protocol: number | null): boolean {
  return protocol !== null && protocol >= WORKSPACE_TERMINALS_PROTOCOL;
}

/** The session a workspace's shells sit beside below protocol 40: its
 * first live, unarchived session, else its first unarchived one (an ended
 * agent still hosts a shell for checking its work; archive ends them). */
export function sessionHost(
  sessions: readonly SessionInfo[],
): SessionId | null {
  const open = sessions.filter((session) => !isArchived(session));
  return (open.find(isLive) ?? open[0])?.id ?? null;
}

/** The place a workspace opens its shells in, or `null` when it has none
 * (below protocol 40, a workspace whose agents are all archived). */
export function workspaceTerminalPlace(input: {
  protocol: number | null;
  root: WorkspaceRoot | null;
  /** The workspace's folder when it has no root: the folder of the
   * session it was opened from (`null` when the session left it to the
   * node, its user's home, which the page cannot name). */
  folder: string | null;
  /** `sessionHost` of the workspace's sessions. */
  host: SessionId | null;
}): TerminalPlace | null {
  const { root } = input;
  if (root?.kind === "project") {
    return { kind: "project", project: root.project };
  }
  if (speaksWorkspacePlaces(input.protocol)) {
    if (root !== null) {
      return { kind: "worktree", worktree: root.worktree };
    }
    if (input.folder !== null) {
      return { kind: "folder", path: input.folder };
    }
  }
  // Below 40, and a folder the page cannot name: beside a session, which
  // every protocol takes.
  return input.host === null ? null : { kind: "session", session: input.host };
}

/** Whether `terminal`, listed for `place`, is a shell of the workspace.
 * A `Project` list holds every project place of the project (the root and
 * its folders), so the folder must match too; `Worktree` and `Folder`
 * lists are already exactly the place. */
export function shellOf(
  place: TerminalPlace,
  terminal: TerminalInfo,
  sessions: ReadonlySet<SessionId>,
): boolean {
  if (terminal.kind.kind !== "shell") {
    return false;
  }
  const at = terminal.place;
  switch (place.kind) {
    case "project":
      return (
        at.kind === "project" &&
        at.project === place.project &&
        (at.dir ?? null) === (place.dir ?? null)
      );
    case "worktree":
      return at.kind === "worktree" && at.worktree === place.worktree;
    case "folder":
      return at.kind === "folder";
    case "session":
      return at.kind === "session" && sessions.has(at.session);
  }
}

/** The shells opened beside the workspace's sessions before its node took
 * the workspace places (ADR 0121): still running, still ended by their
 * session's archive, so still shown and counted. */
export function olderShells(
  listed: readonly TerminalInfo[],
  sessions: ReadonlySet<SessionId>,
): TerminalInfo[] {
  return listed.filter(
    (terminal) =>
      terminal.kind.kind === "shell" &&
      terminal.place.kind === "session" &&
      sessions.has(terminal.place.session),
  );
}

/** The live terminals a worktree's removal would end (`end_terminals`):
 * every live one whose folder is the worktree's or inside it, whatever its
 * place, since the node ends them by folder (`Terminals::end_in`). */
export function liveTerminalsIn(
  listed: readonly TerminalInfo[],
  folder: string,
): number {
  const inside = folder.endsWith("/") ? folder : `${folder}/`;
  return listed.filter(
    (terminal) =>
      (terminal.status.state === "starting" ||
        terminal.status.state === "running") &&
      (terminal.folder === folder || terminal.folder.startsWith(inside)),
  ).length;
}

/** Whether a removal kept `in_use` may ask the node to end the terminals
 * that keep it (ADR 0121): the node takes it, some live terminal is there,
 * and no live session is (the node keeps a worktree with a live session
 * whatever is asked, and leaves its shells running then). */
export function mayEndTerminals(input: {
  protocol: number | null;
  inUse: boolean;
  liveTerminals: number;
  sessionInIt: boolean;
}): boolean {
  return (
    speaksWorkspacePlaces(input.protocol) &&
    input.inUse &&
    input.liveTerminals > 0 &&
    !input.sessionInIt
  );
}

/** The strip's shells: the place's, then the older ones the one read at
 * opening found and the place's list does not hold. With none found the
 * strip is the place's list alone. (What was closed here has already left
 * both: `forget` drops it from each.) */
export function stripShells(
  placed: readonly TerminalInfo[],
  older: readonly TerminalInfo[],
): readonly TerminalInfo[] {
  const extra = older.filter(
    (terminal) => !placed.some((row) => row.id === terminal.id),
  );
  return extra.length === 0 ? placed : [...placed, ...extra];
}
