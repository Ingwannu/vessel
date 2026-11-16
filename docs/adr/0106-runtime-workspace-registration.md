# 0106: Runtime workspace registration

Status: proposed (node carrier for the paused W11, coordinator ruling 2026-11-16).
Extends ADR 0082 (independent workspace registry). TRACKER rows 241 and 780.

## Purpose and intent

Before this decision, nothing in production created a registry workspace. Only
tests called `put_registry_workspace` with a new record. So `workspace.registry`
was always empty, `workspace.label.assign` could only answer `workspace_not_found`
(ADR 0086), and every C row, label, script and service consumer had nothing to
show. Paseo registers a workspace when work starts in a directory or worktree,
and at daemon start from the agents it already has. This ADR copies that onto
Vessel's registry, projects and worktrees.

## Reference (Paseo 0.15.4)

- `server/workspace-registry-bootstrap.ts` 48-210 `bootstrapWorkspaceRegistries`:
  - groups existing non-archived agents whose directory still exists by
    workspace directory;
  - makes one workspace per directory, created at the oldest agent's time;
  - backfills each agent's `workspaceId`
    (`migrations/backfill-workspace-id.migration.ts`).
- `session/workspace-provisioning/workspace-provisioning-service.ts`:
  - 356-375 `findOrCreateWorkspaceForDirectory`: the oldest active foreground
    workspace at an equivalent path; otherwise unarchive one; otherwise create.
  - 238-270 `createWorkspaceForDirectory`.
  - 272- `createWorkspaceForWorktree`.
  - 377-389 `resolveOrCreateWorkspaceIdForCreateAgent`: a created worktree's
    workspace, then a requested id, then a new workspace.
- `bootstrap.ts` 1114-1118: requests that arrive with a path and no workspace id
  ("old clients / CLI") resolve the workspace by path (`resolveWorkspaceIdForPath`).
- `session.ts` 7005-7030 (open project): find or create by directory.
- Adding a project registers no workspace (TRACKER 239).

## Existing assets and constraints

- The W11 registry store:
  - `put_registry_workspace` with revision zero creates a record;
  - `claim_registry_start`, `complete_registry_start` and `fail_registry_start`
    form the start lease that fences a resource against archive;
  - `set_registry_membership` records ownership of an existing resource;
  - `registry_changes` wakes `workspace.registry` observers;
  - capacity limits are `MAX_WORKSPACES` and `MAX_RESOURCES`.
- Sessions resolve a place into `SessionInfo.workspace` (`SessionWorkspace { project,
  worktree }`). A terminal is placed beside a session or in a project. The
  runtime accepts a preallocated `SessionId` (`LaunchOptions.id`).
- `workspace/registry_fence.rs` refuses project removal and worktree removal
  while any registry descriptor or reservation names the project. W11 added it
  so that legacy cleanup cannot delete a backing the registry owns.
- `WorkspaceBacking::Local` is an unowned directory that is retained on archive.
  `Managed { worktree }` is a worktree the registry created and removes after its
  last reference. Vessel's existing worktrees are created and removed by the
  legacy worktree service, not by the registry.

## Decision

[Decision Log]
- Choice 1, which places register.
  - A project root and an existing Vessel worktree each map to a workspace
    whose `path` is that directory and whose backing is `Local`.
  - Sessions with no project place (a plain cwd) are not registered here.
    Vessel projects are explicit, and Paseo's automatic project per directory
    (`findOrCreateProjectForDirectory`) is a separate decision with visible UI
    effects (every agent directory becoming a project).
- Choice 2, when.
  - A root session created with a place joins the place's workspace through
    find-or-create, before spawn. It claims a start lease on a preallocated
    `SessionId`, completes the lease after registration, and fails it on a
    proven no-spawn error.
  - A session created by an agent joins its creator session's workspace when
    that session has one (Paseo's `callerWorkspaceId` inheritance).
  - A terminal placed beside a session joins that session's workspace. A
    terminal placed in a project joins that root's workspace through
    find-or-create.
  - `worktree.create` registers the worktree's workspace at once.
  - Project add, clone and init register none (TRACKER 239).
- Choice 3, find-or-create, not fresh-per-create.
  - Vessel's `session.create` carries no workspace id. Paseo's path-only
    requests ("old clients / CLI") resolve by path, and its open-project path
    finds or creates.
  - Fresh creation of a workspace for an explicitly new one is W3's workspace
    create schema (TRACKER 277-278). That request will carry the intent, and
    it then follows `resolveOrCreateWorkspaceIdForCreateAgent`.
  - The rule is the oldest Active, foreground workspace with the same project
    and the same path, ties broken by id; otherwise create one. An archived
    workspace at the path is not restored: restore is an explicit act
    (W21/W11).
- Choice 4, the startup bootstrap (every start, idempotent; Paseo runs it when
  its registry files are missing, and Vessel's tables exist but were never
  written).
  1. Every live worktree without an Active workspace at its path gets one.
  2. Every stored or live session with a place, not archived, whose directory
     still exists, and with no membership is grouped by placement. Each
     placement joins its find-or-create workspace, created at the earliest
     member's creation time. Each session records membership through
     `set_registry_membership` (no spawn happens, so no lease).
  3. Retained terminals follow the same terminal rule.
  - Nothing registered before is changed. A failed write fails node start, as
    Paseo's bootstrap error stops the daemon.
- Choice 5, the legacy-removal fence.
  - The fence keeps refusing what the registry owns: `Managed` backings and
    reservations in progress.
  - A `Local` workspace owns no directory, so it no longer blocks legacy
    project or worktree removal.
  - Instead, under the project admission that removal already holds, removal
    archives the `Local` Active workspaces it strands, by CAS to
    `Archived { at }`. Records and historical memberships stay, as Paseo's
    project removal archives its workspaces (TRACKER 258) and worktree removal
    archives the worktree's workspace.
  - Removal still refuses live sessions and terminals first, so an archived
    workspace has no running member. A pending start lease refuses the
    removal (`conflict`) instead of being archived around it.
- Rejected alternatives:
  - Registering with `Managed` backing would make legacy removal refuse
    forever, because the legacy service owns those worktrees.
  - A cwd heuristic at read time gives no durable identity, nothing for labels
    to attach to, and no archive fence.
  - Deleting workspace records on removal loses label assignments and
    history, and Paseo keeps archived records.
  - A fresh workspace per session would give one row per session, with no
    client able to choose otherwise until W3's create schema lands.
- Advantages, costs and effects:
  - The registry, labels and the page's "group by workspace" mode get real
    data.
  - Legacy removal keeps working, now also archiving its workspaces.
  - Each create and each terminal start adds one or two registry writes in
    the existing writer.
  - Capacity errors (`RegistryFull`) refuse the create before spawn.
  - No new crate edge, dependency, wire method or schema change. The registry
    tables exist since schema 20.

## Not decided here

The following stay with their owners:
- workspace archive and restore RPCs (W21/W11);
- W3's explicit workspace create, isolation and setup;
- moving a session between workspaces;
- an automatic project per directory;
- per-workspace titles from the first prompt (Paseo `workspace-auto-name.ts`).
