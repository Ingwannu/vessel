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
   - Agents keep `Session` only, as today (`agent_may_open`). Amended by amendment 1 (TRACKER 832): an agent still
     names a session, and its shell opens in that session's workspace.
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

## Amendment 1 (TRACKER 832, 2026-11-20): an agent's shells in its workspace

Accepted by the coordinator on 2026-11-20 with the life ruling below. Implementer T2. No new protocol number: the
request and the places are 40's.

### Context

- Since this ADR the clients list a workspace's terminals by its place (page 817, native C1/P1). Decision 1's last
  bullet left agents on `Session`, so a shell an agent opened showed in no workspace's tabs. It also ended with the
  agent (`end_agents_of`), which brought back R1 for an agent's dev server.
- Paseo gives an agent's terminal the caller agent's workspace and mints one for its cwd only when the agent has
  none (`packages/server/src/server/agent/tools/paseo-tools.ts` 895-913, `resolveTerminalWorkspaceId`; its
  `create_terminal` 2492-2526; the cwd 873-893, `resolveScopedCwd`). A workspace lists its terminals by
  `workspaceId` and cwd (`packages/app/src/screens/workspace/terminals/use-workspace-terminals.ts` 75-100). They end
  only with the workspace's archive (`workspace-archive-service.ts` 517-555, 589), never with the agent.

### Decision

1. **Placement.** An agent still names a session in `terminal.create { place: session }`: its own (the MCP tool's
   default) or one it may operate. That session stays the anchor of the agent's rights, and ADR 0031's checks are
   unchanged: both profiles set `agent_terminals`, the agent may operate the session, the session is live. For a
   caller at 40 or later, the node places the shell in that session's workspace (`agent_shell_place`):
   - `worktree { worktree }` when the session runs in a worktree;
   - `project { project, dir }` when it runs in a project;
   - otherwise `folder { path }`: its cwd, kept canonical;
   - a session outside projects with no cwd keeps `session`: the page keeps such a session's shells beside it,
     since it cannot name that folder, so a folder place would show in no workspace's tabs (node review R1).
   Resolution, counts, the worktree hold and the after-start check are decision 1's for that place. People name
   their places themselves and are never moved.
2. **Life (the coordinator's ruling: Paseo's).** Such a shell lives with its place, as a person's shell there does:
   - it ends when closed or killed, with `worktree.remove { end_terminals: true }`, with the project's removal, or
     with the node;
   - `end_agents_of` ends only shells beside a session: that agent's own, and any agent's beside the ending session;
   - the agent's use ends with its session (`may_use`: only the opening agent, whose credential ends with it);
   - the 4-per-agent limit still counts its live shells.
   The folder limit (16) and the worktree's `end_terminals` path bound what stays behind.
3. **Older callers.** An agent call below 40 keeps the `session` place it named: it could not decode a workspace place
   in the answer, and such a shell keeps the old life. A person's client below 40 no longer lists an agent's
   workspace shell, by decision 5's filter (stated).
4. **Existing shells.** Session-placed shells are not moved (this ADR's rejected re-homing). In practice the node
   restart that installs this ends every live terminal on that node.

### Alternatives considered

- **The MCP tool sends the workspace place.** The node would then have to tell which session's workspace an agent
  may open in. The session it names already says that, and keeps the rights check per session.
- **Keep ending an agent's shells with its session.** That is a stated difference from Paseo, and it brings R1 back
  for an agent's dev server.

### Consequences

- Node:
  - `service/terminals/place.rs` `agent_shell_place`;
  - `create_terminal` resolves the mapped place;
  - `terminals/registry/ending.rs` `end_agents_of` is limited to Session places.
- MCP: `create_terminal`'s and the server's descriptions say "in your session's workspace". The request is unchanged.
- Tests (`tests/e2e/agent_workspace_terminals.rs`, `tests/e2e/terminals.rs`, `vessel-mcp/tests/terminals.rs`):
  - an agent in a worktree gets a `worktree` shell, listed there and not beside the session;
  - it outlives the session's archive and ends with `worktree.remove { end_terminals: true }`;
  - a folder shell outlives the archive while an older call's `session` shell ends;
  - the session-outside-projects case is a canonical `folder`, and a session with no cwd keeps `session`;
  - the rights and the 4-per-agent limit are unchanged.
