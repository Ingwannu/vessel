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
import type { Outcome } from "../outcome";
import { isArchived } from "../marks";
import {
  WORKSPACE_TERMINALS_PROTOCOL,
  type SessionId,
  type SessionInfo,
  type TerminalInfo,
  type TerminalList,
  type TerminalPlace,
  type WorkspaceRoot,
  type WorktreeId,
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

/** The shells beside the workspace's sessions (a `Session` place). From
 * protocol 40 the page opens none, but agents still do: the MCP
 * `create_terminal` always asks for `Session` (ADR 0121: agents keep it),
 * as do clients below 40 and shells opened before the upgrade. The
 * node's `Worktree` and `Folder` lists leave them out, so the strip reads
 * them on its own every tick (the node reviewer's R1 on TRACKER 829). */
export function besideSessions(
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

/** Whether `path` is `folder` or inside it: the node's own test for what
 * is in a worktree (`Path::starts_with`, by whole components). */
function inFolder(path: string, folder: string): boolean {
  const inside = folder.endsWith("/") ? folder : `${folder}/`;
  return path === folder || path.startsWith(inside);
}

/** The live terminals a worktree's removal would end (`end_terminals`):
 * every live one whose folder is the worktree's or inside it, whatever its
 * place, since the node ends them by folder (`Terminals::end_in`). */
export function liveTerminalsIn(
  listed: readonly TerminalInfo[],
  folder: string,
): number {
  return listed.filter(
    (terminal) =>
      (terminal.status.state === "starting" ||
        terminal.status.state === "running") &&
      inFolder(terminal.folder, folder),
  ).length;
}

/** Whether a live session works in `worktree`, by the node's test: its
 * folder is the worktree's or inside it (`worktree.remove` checks every
 * live session's `cwd` that way), so a session started there by folder
 * (the CLI's `--cwd`) counts as much as one placed in the worktree. */
export function sessionInWorktree(
  sessions: readonly SessionInfo[],
  worktree: { id: WorktreeId; path: string },
): boolean {
  return sessions.some(
    (session) =>
      isLive(session) &&
      (session.workspace?.worktree === worktree.id ||
        inFolder(session.spec.cwd ?? "", worktree.path)),
  );
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

/** What the removal dialog offers, from what the node said and what the
 * page counted.
 * - `checking`: the count is still being read (or could not be read), so
 *   neither the warning nor "end the session" is said yet and the buttons
 *   wait.
 * - `endTerminals`: the buttons end the counted terminals first.
 * - `blocked`: `in_use` holds and cannot be lifted here (a live session,
 *   setup, or a node below 40). */
export function removalState(input: {
  protocol: number | null;
  inUse: boolean;
  /** `null` until counted. */
  liveTerminals: number | null;
  sessionInIt: boolean;
}): { checking: boolean; endTerminals: boolean; blocked: boolean } {
  const checking =
    speaksWorkspacePlaces(input.protocol) &&
    input.inUse &&
    !input.sessionInIt &&
    input.liveTerminals === null;
  const endTerminals = mayEndTerminals({
    ...input,
    liveTerminals: input.liveTerminals ?? 0,
  });
  return { checking, endTerminals, blocked: input.inUse && !endTerminals };
}

/** One tick of the strip's list: the workspace's shells as the node lists
 * them now. A `Worktree` or `Folder` place is read by itself (the node
 * matches it) and, alongside it, every terminal for the shells beside the
 * workspace's sessions; a `Project` or `Session` place reads every
 * terminal once, which holds both. Each tick replaces the last, so a shell an
 * agent opens shows within a tick and one that ends shows ended (the
 * strip then drops its tab). Either read failing fails the tick, which
 * keeps the last list rather than one with half its shells missing. */
export async function readStrip(
  list: (place: TerminalPlace | null) => Promise<Outcome<TerminalList>>,
  place: TerminalPlace,
  sessions: ReadonlySet<SessionId>,
): Promise<Outcome<readonly TerminalInfo[]>> {
  if (place.kind === "session" || place.kind === "project") {
    // One read of every terminal holds both a project's own places (with
    // their folders) and the shells beside its sessions: an agent in a
    // project-root workspace opens the latter too (the node reviewer's
    // follow-up to R1; 817 read the project's place alone).
    const read = await list(null);
    if (!read.ok) {
      return read;
    }
    const placed = read.value.terminals.filter((terminal) =>
      shellOf(place, terminal, sessions),
    );
    const beside =
      place.kind === "project"
        ? besideSessions(read.value.terminals, sessions).filter(
            (terminal) => !placed.some((row) => row.id === terminal.id),
          )
        : [];
    return { ok: true, value: [...placed, ...beside] };
  }
  const [own, all] = await Promise.all([list(place), list(null)]);
  if (!own.ok) {
    return own;
  }
  if (!all.ok) {
    return all;
  }
  const placed = own.value.terminals.filter((terminal) =>
    shellOf(place, terminal, sessions),
  );
  const beside = besideSessions(all.value.terminals, sessions).filter(
    (terminal) => !placed.some((row) => row.id === terminal.id),
  );
  return { ok: true, value: [...placed, ...beside] };
}
