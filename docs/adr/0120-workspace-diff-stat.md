# ADR 0120: Workspace diff stat

Status: proposed (impl K, 2026-11-20; TRACKER 819). The coordinator
assigned 0120 and approved the plan: the count is served, not stored on
the registry row; one watcher per directory; Paseo's comparison,
including `origin/<branch>` while the checkout is already on the base.
The protocol is drafted 37 (`DIFF_STAT_PROTOCOL` in
`vessel_proto::registry`). The constant stays `None` until the
integrator assigns the number at merge. This record does not add a
protocol version section.

## Context

Paseo's workspace hover card shows a diff stat after the pull-request
badge (`workspace-hover-card.tsx`, `DiffStat`): additions and deletions
for the checkout, compact and lowercase (`1.2k`), hidden when the stat
is null. The count is `git diff --shortstat` from the merge base when
the branch is not the local base, and from `origin/<branch>` when it
is, if that ref exists. Detached HEAD, a missing comparison ref, a
failed read, and `{0, 0}` are null.

Vessel's registry row is durable identity. Putting the count on
`WorkspaceDescriptor` would publish it through `workspace.registry` and
bump `RegistryRevision` on a file save. A save is not a registry edit.
The page sidebar reads workspaces from `workspace.registry`, so the
count cannot live only on a client cache that nothing wakes.

## Decision

`workspace.list` and `workspace.get` serve a `ServedWorkspace`: the
descriptor, flattened, plus optional `diff_stat` (`{additions,
deletions}`, both `u32`) and `stat_revision`. Both requests gain
`changed_from`. Old JSON omits the new request field; clients that do
not know the response fields ignore them.

The count is not written to the registry and does not change
`RegistryRevision`. A separate generation starts at 0. `workspace.list`
and `workspace.get` carry the same generation on every row of one
answer. It increases only when a stored count changes, and the first
stored value increases it even when the count is null, so a waiter on
0 wakes. The same value does not increase it. The counter does not
wrap.

A client that passes the generation it already has waits until the
generation changes, up to the existing 30 second cap, in a wait slot,
the same way `workspace.registry` waits. The first call, with no
`changed_from`, waits only while a requested folder has not been
counted yet. Delegated principals note interest and do not wait, as
they do not wait on the registry. Nothing on the page or the app polls
on a timer. An empty list still carries the generation, so the next
call can wait even though that answer had no row to hang the number
on.

While `DIFF_STAT_PROTOCOL` is `None`, list and get do not attach the
fields, do not wait, and do not watch. `Reply::for_protocol` also
strips the fields for a peer below the assigned version, so assigning
the constant is what turns the feature on.

One recursive watch per canonical directory, shared by every workspace
in that directory, dropped 60 seconds after the last list or get that
named it. At most 1000 directories; past that a further folder is
answered as null and is not watched. Writes settle for one second.
Access events and paths under `node_modules`, `target`, `.git/objects`,
and `.git/logs` do not dirty the count. A directory that is not a git
checkout, or whose watch did not start, is counted again after 15
seconds and is not watched. The book lock is not held across an await.
The watch task holds the workspaces and the book, not the service that
owns both.

The count reuses the base-scope numstat (`Git::changes`, untracked
files capped at 500, the file list capped at 5000). A missing line
count (a binary file) adds zero. The sum saturates. `{0, 0}` is stored
as null. On the workspace base, or when the workspace has no base, the
comparison is `origin/<branch>` and only when that ref resolves;
otherwise the count is null. A detached HEAD is null. `refs/heads/main`
is the same branch as `main`. `refs/remotes/origin/main` is not.

## Consequences

The page hover card is the other half of 819, on this branch after the
generated wire is updated: Paseo's card, wide and pointer only, nothing
while dragging, for the rows whose data exists today (name, pull
request, diff stat, host, copyable branch and folder, checks). The page
long-polls `workspace.list` `changed_from` and merges `diff_stat` by
id. It does not read the count from the registry snapshot.

The native card is the 810 seam. 810's head is `31714d88e` on
`backend/m12-session-hover-card`, not on a package base yet. That
branch is not edited here. Once the lead adopts 810, the row is
`HoverDetails.diff` after `pull`, filled from the served stat into
`vessel_app_view::workspace::diff_rows::Counts { added, removed }`,
hidden when the stat is null.

Sidebar trailing "diff", the composer Changes pill, and the archive
warning are not part of 819.

A list or get waits on one generation for every watched directory, so
a change in one folder wakes a read of another. That is the cost of
not inventing a second revision per workspace.
