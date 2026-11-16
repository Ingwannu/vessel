# 0106: Runtime workspace registration

Status: accepted (node carrier for the paused W11, coordinator rulings 2026-11-16).
Extends ADR 0082 (independent workspace registry). TRACKER rows 241 and 780.

## Purpose and intent

Before this decision, nothing in production created a registry workspace. Only
tests called `put_registry_workspace` with a new record. So `workspace.registry`
was always empty, `workspace.label.assign` could only answer `workspace_not_found`
(ADR 0086), and every C row, label, script and service consumer had nothing to
show. Paseo registers a workspace when work starts in a directory or worktree,
and at daemon start from the agents it already has. This ADR copies that onto
Vessel's registry for project roots; worktrees wait for their own backing
decision.

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

Coordinator rulings of 2026-11-16 shape this decision: project roots only,
derived membership, and "option A narrowed" for the fence. The node reviewer's
early read raised the capacity and fence questions.

[Decision Log]
- Choice 1, which places register.
  - Only a project root registers. It maps to a workspace whose `path` is the
    project root and whose backing is `Local`.
  - Worktrees are not registered here. `WorkspaceDescriptor::root()` maps
    `Local` to the project root, so a worktree needs its own backing decision
    (W3 / W21).
  - A session in a project subfolder (`SessionPlace::Project { dir }`) belongs
    to the root's workspace. This is a declared difference: Paseo keys a
    workspace by the agent's own directory.
  - Sessions with no project place (a plain cwd) are not registered.
    - Vessel projects are explicit.
    - Paseo's automatic project per directory
      (`findOrCreateProjectForDirectory`) is a separate decision with visible UI
      effects.
- Choice 2, when.
  - A root workspace is found or created, before spawn, when any of these
    happens:
    - `session.create` with a project place;
    - `terminal.create` placed in a project;
    - `script.run` in a project place.
  - A session or script placed beside a session is not registered again, because
    its root was registered when that session was.
  - Project add, clone and init register nothing (TRACKER 239).
  - Registration never refuses the caller's start. A failure is logged and the
    start goes on, and that includes `RegistryFull` at the live cap. Bookkeeping
    must never refuse a live create.
  - No start lease. W11's lease fences a resource that a registry launch
    started, and nothing here launches through the registry.
- Choice 3, find-or-create.
  - Vessel's `session.create` carries no workspace id. Paseo's path-only
    requests ("old clients / CLI") resolve by path, and its open-project path
    finds or creates.
  - The rule is the oldest Active, foreground, `Local` workspace of the same
    project at the same path, with ties broken by id. Otherwise one is created.
  - The lookup and the insert share one writer savepoint, so two concurrent
    starts at one root create one workspace. The caller passes the fresh id.
  - Declared difference: an archived workspace at the path is not restored.
    Paseo unarchives it when its project is live. Here, restore is an explicit
    act (W21/W11), and project removal is the only thing that archives.
- Choice 4, membership derived from stored placement.
  - No membership row per session or terminal. Under the lifetime caps of ADR
    0082, rows like that would eventually refuse every create.
  - `workspace.registry` derives each member: a placed session that is not
    archived belongs to its project's oldest Active, foreground `Local`
    workspace (Paseo's `agent.workspaceId`), unless an explicit row already
    names it.
  - Derived members are not revisioned. A session start or archive changes
    them without a registry write, so a long-polling client does not wake for
    them.
  - A client that groups live sessions takes the membership rule from the
    session's own `SessionInfo.workspace` (project, no worktree) and gets the
    workspace from the registry. This is how Paseo's client groups agents by
    their workspace id from the agent stream.
- Choice 5, the startup bootstrap. It runs once per project, every start, and
  is idempotent.
  - It covers every project that has placed, non-archived sessions and no
    registry record in any lifecycle.
  - Each such project's root gets a workspace created at the earliest such
    session's time, as `bootstrapWorkspaceRegistries` does.
  - A project with an archived record is skipped, so removal is not undone at
    the next start.
  - A missing root is skipped, as Paseo skips an agent whose directory is gone.
  - A failure is logged and never fails node start. Paseo stops its daemon
    instead; the ruling above decides this.
- Choice 6, the legacy-removal fence ("option A narrowed").
  - A `Local` workspace that is Active or Archived owns no directory, so it no
    longer blocks legacy project or worktree removal.
  - Project removal archives the project's own `Local` Active workspaces, by
    CAS to `Archived { at }`, in the same store write that forgets the project.
    Records stay, as Paseo's project removal archives its workspaces (TRACKER
    258).
  - Still blocked:
    - `Managed` backings;
    - reservations in any stage;
    - a record that is Archiving or Restoring.
  - Removal still refuses live sessions and terminals first.
  - This reverses W11's "every retained lifecycle blocks". W11's pinned test
    was rewritten deliberately, and the reversal is recorded for W11's resume
    (TRACKER, the ledger, `docs/plans/workspace-legacy-removal-fence.md`).
- Choice 7, capacity. ADR 0082's amendment applies:
  - archived records do not count toward the live caps;
  - archived `Local` records beyond `MAX_ARCHIVED_WORKSPACES` are pruned oldest
    first;
  - a live create is never refused because of them.
- Rejected alternatives:
  - A membership row per session or terminal: lifetime row caps make creation
    refuse for good (node reviewer, coordinator ruling).
  - Registering worktrees as `Local`: `root()` would point them at the project
    root.
  - Registering worktrees as `Managed`: legacy removal would refuse for good,
    because the legacy service owns those worktrees.
  - A cwd heuristic at read time: it gives no durable identity, nothing for
    labels to attach to, and no archive fence.
  - Deleting workspace records on removal: it loses label assignments and
    history, and Paseo keeps archived records.
  - A fresh workspace per session: one workspace per session, with no client
    able to choose otherwise until W3's create schema lands.
- Advantages, costs and effects:
  - The registry, labels and the page's "group by workspace" mode get real
    data.
  - Legacy removal keeps working, and now also archives its workspaces.
  - Each start at a project root adds one writer round trip, which reads; the
    first start at a root also inserts.
  - No new crate edge, dependency, wire method or schema change. The registry
    tables exist since schema 20.

## Verification

- Store (`registry/tests/placement.rs`):
  - one workspace per root, and a later start finds it (the node test also
    runs two starts at once);
  - archived records are not restored;
  - removal archives `Local` Active only, with one revision;
  - archived records never refuse a live create and are pruned oldest first;
  - derived members;
  - the one-shot bootstrap list.
- Store fence (`registry/tests/project_reference.rs`): owned backings and
  transitions fence, and `Local` Active and Archived do not.
- Node:
  - `workspace/tests/registration.rs`: find-or-create; an unknown project; at
    the live cap the start goes on;
  - `workspace/tests/registry_fence.rs`: removal archives and keeps the records;
    transitions still block.
- e2e (`tests/e2e/registration.rs`):
  - adding a project registers nothing;
  - two sessions at a root share one workspace as derived members;
  - removal archives and the snapshot still reads with its project gone;
  - node start registers stored sessions' roots at the earliest session's time
    and skips a missing root.

## Not decided here

The following stay with their owners:
- workspace archive and restore RPCs (W21/W11);
- W3's explicit workspace create, isolation and setup, and worktree workspaces;
- moving a session between workspaces;
- an automatic project per directory;
- per-workspace titles from the first prompt (Paseo `workspace-auto-name.ts`);
- revisioned derived membership, or paging the registry snapshot past about
  90,000 placed sessions (ADR 0082 amendment).
