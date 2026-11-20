# ADR 0120: Workspace diff stat

Status: proposed (impl K, 2026-11-20; TRACKER 819). The coordinator
assigned 0120 and approved the plan, then ruled on the node review that
a folder is recounted every 15 seconds while a client is interested,
with no recursive OS watch. The count is served, not stored on the
registry row. The comparison is Paseo's, including `origin/<branch>`
while the checkout is already on the base. The node integrator assigned
38 on 2026-11-20, after shared terminals took 37 (`DIFF_STAT_PROTOCOL`,
`PROTOCOL_VERSION` 38). The integrator confirms the number when this
branch merges.

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

A peer below 38 is not served the fields. List and get do not attach
them, do not wait, and do not count for that peer. `Reply::for_protocol`
strips the fields, so the constant is what turns the feature on.

A folder is counted when a served client lists or gets it, and again
every 15 seconds while that interest lasts. Interest ends 60 seconds
after the last such request; the slot and its task then go. There is
no recursive directory watch. One inotify instance per folder would
spend the node's watch budget, and the walk would include trees the
count ignores. At most four counts run at once, under the shared git
pool of eight local slots, so a person's status, diff, or commit still
has room. A count that finds git busy, or that times out, keeps the
last stored value and tries again on the next pass. The first answer,
when nothing is stored yet, is null, so that read is not left waiting.
A folder that cannot be resolved is null, as one that is not a git
checkout is. Null is also stored when there is no comparison (detached,
or `origin/<branch>` does not resolve) or both sides are zero. At most
1000 folders; past that a further folder is answered as
null and is not counted. The book lock is not held across an await.
The count task holds the workspaces and the book, not the service that
owns both.

Unstaged edits can take up to 15 seconds to show, where Paseo shows
them after a 1 second debounce. A stage or a commit waits for the same
recount. The node's file observer builds one watch per request and
drops it, so there is no shared instance a count can add `.git/HEAD`
and `.git/index` to without a new inotify thread per folder.

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

A list or get waits on one generation for every counted folder, so a
change in one folder wakes a read of another. That is the cost of not
inventing a second revision per workspace.
