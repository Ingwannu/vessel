# ADR 0121: terminals belong to the workspace folder

Status: accepted by the coordinator on 2026-11-20 with two rulings (a project's list holds its project places only;
a worktree's removal ends its terminals only when the caller acknowledges it). Implementer T2. Protocol 40
(`WORKSPACE_TERMINALS_PROTOCOL`; drafted 39, assigned 40 at its node-line merge after the pull request pane's 39).
Source: the node review of TRACKER 817 (R1: closing a finished agent's tab ended a worktree's dev server).

## Context

- Paseo keys a terminal by its workspace folder (`cwd` plus `workspaceId`). It kills a workspace's terminals only
  when the workspace itself is archived (`workspace-archive-service.ts` 517-555, `killTerminalsForWorkspace` 589).
  Closing or archiving one agent never touches them.
- Vessel (ADR 0031) has two places:
  - `Session {session}`: beside a session, in its folder. Ended by the session's deletion
    (`Terminals::end_session`) and by its archive (`put_away` → `end_beside`).
  - `Project {project, dir?}`: in a project folder or a folder under its root. Ended by the project's removal.
- A worktree workspace and an unregistered-folder workspace have no place of their own. Clients (817 page, native
  P1) put their shells beside the session in view, so archiving that agent kills the dev server running in the
  workspace's terminal. After that, the archived place is refused for new shells.

## Decision

1. **Two new places.**
   - `TerminalPlace::Worktree { worktree: WorktreeId }`: the worktree's folder, resolved as
     `SessionPlace::Worktree`. It counts against the worktree's project, like a project place. Its life is tied
     to the worktree (ruling 2, Paseo's `killTerminalsForWorkspace` with a safety acknowledgement):
     `worktree.remove` gains an explicit "end its terminals" acknowledgement. With it, the node ends the live
     terminals in the worktree (kill, wait `CLOSE_WAIT`) and forgets them with the worktree; without it (and from
     every older client) removal is refused while a live terminal is in it, as today (`NodeService::occupied`,
     "close them first"). Clients confirm first with Paseo's terminal warning and the count. A live session in the
     worktree refuses the removal whatever is asked, so the node leaves the shells running then (ending them would
     lose a dev server for nothing). Otherwise the terminals end before the removal's own checks: a removal then
     kept for work it would lose leaves them ended (the person agreed to end them).
   - `TerminalPlace::Folder { path }`: an absolute folder that is no project's and no worktree's (an
     unregistered-folder workspace).
     - Resolved: an absolute, existing directory, kept canonical, so a list by the same folder finds it. The
       projects' refused roots do not apply: they guard what a file panel may read, and a home folder is a usual
       place for a shell (a person can already start a session there). That includes the node's own data folder:
       a shell there is no more than a shell that `cd`s into it, which any shell can.
     - Counts against the node's limits and a per-folder limit equal to the project's (16).
     - Ends only when closed, killed or the node stops: no session or project owns it.
   - Agents keep `Session` only, as today (`agent_may_open`).
2. **Archive and delete no longer reach them.** `end_beside` and `end_session` select `Session` places only (they
   already do); the new places are never beside a session.
3. **Listing.**
   - `terminal.list { place: Worktree }` matches that worktree.
   - `{ place: Folder }` matches the same canonical path.
   - `{ place: Project }` matches project places only (ruling 1): a worktree is its own workspace, as in Paseo,
     so a project workspace does not list its worktrees' shells even though they count against the project.
4. **Clients.**
   - Page (W, 817): a project workspace uses `Project` (unchanged), a worktree workspace `Worktree`, an
     unregistered folder `Folder {cwd}`.
   - Native: the session's workspace (`SessionInfo.workspace`: project, worktree) picks the same, with `Folder` for
     a session outside projects.
   - The anchor-session coupling and the page's archive warning about terminals go away.
5. **Version rule.**
   - `Call::since`: creating or listing with a new place needs the new protocol.
   - Answers to older clients leave out terminals with a new place (`rpc/older/places.rs`, as titles are cut):
     a client that cannot decode the place does not see the terminal. Every call naming one by id
     (`terminal_for`: get, read, subscribe, send, rename, kill, close) answers not_found below the protocol.

## Alternatives considered

- **Keep `Session` and skip `end_beside` for shells:** the session archive is the person putting the work away;
  services beside a session should still stop. It also leaves the refused-archived-place problem.
- **One `Folder` place for worktrees too:** loses the worktree's lifetime (removal) and its project count; a
  worktree id survives a rename of its folder's parent, a path does not.
- **Re-home existing terminals:** a terminal's place is fixed at creation; clients create new shells in the new
  places, and old session-placed shells keep their old life.

## Consequences

- Protocol: new `TerminalPlace` variants; older clients do not see those terminals (stated), and the minimum
  protocol stays 2.
- Node: place resolution, the limits table (per folder), worktree removal ending its terminals, list matching, the
  older-reply filter, tests (archive keeps worktree and folder shells; removal ends worktree shells; older clients
  do not see them; limits).
- Page and native consumers follow. Stated difference: an unregistered folder's terminals outlive every session in
  it until closed (Paseo's archived folder workspace would kill them; Vessel has no folder workspace record to
  archive).

## As built: the page (TRACKER 829)

- `web/src/data/workspace/terminal-place.ts` holds the place rule as pure functions.
  - From protocol 40: a project workspace gets `Project`, a worktree workspace `Worktree`, and a session outside
    projects `Folder {cwd}`.
  - Below 40, it is today's `Session` beside the first live unarchived agent, else the first unarchived one
    (an ended agent still hosts a shell); with every agent archived there is no place and "New terminal" is
    disabled.
  - A session that left its folder to the node (no `cwd`) keeps the `Session` place, since the page cannot name
    that folder.
- The workspace's shells:
  - They are listed by their place, and the node does the matching.
  - Shells placed beside the workspace's sessions are read on the same 2 s tick, alongside the place's list
    (`readStrip`). Agents still open them: the MCP `create_terminal` asks for `Session` (decision 1, agents keep
    `Session`), and so do clients below 40 and shells opened before the upgrade. The node's `Worktree` and
    `Project` lists leave them out. A first build read them once when the workspace opened, on the premise that no
    new ones are made; the node review (R1) showed the premise false, and the coordinator withdrew that ruling.
    Each tick replaces the list, so a shell an agent opens shows within a tick and one that ends leaves the strip.
  - A node row (the coordinator's) is to give an agent's shell its workspace's place, as Paseo does
    (`paseo-tools.ts:2512`); until then the page reads both.
- "New terminal" needs no agent from 40. Its "Unarchive an agent…" words remain for nodes below 40.
- The archive warning counts only shells placed beside the archived agents (`endedByArchive`). A test pins that
  `Worktree`, `Folder` and `Project` shells are never counted.
- Worktree removal:
  - The dialog counts the live terminals at or under the worktree's folder, whatever their place, which is what
    `end_in` ends, and counts them again on the press that ends them: a count that changed stops the press and
    the warning says the new one.
  - A live session is "in it" by the node's own test, a `cwd` at or under the worktree's folder, as well as by
    its workspace, so one started there by folder (the CLI) keeps the dialog from asking to end the terminals.
  - With terminals and no live session, Paseo's bulk-close terminal line with the count sits above the usual
    buttons as the kit's Alert (sm, warning), and the removal sends `end_terminals: true`.
  - With a live session, it is as before.

