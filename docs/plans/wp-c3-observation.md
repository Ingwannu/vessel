# C3 observation join: design, not an installed observer

Status: observation runtime unimplemented. The current node comparison/cache/file-mutation tests do
not prove external-change observation, subscriptions or background fetch. Keep
those tracker rows missing until their actual implementation and checks exist.

[Decision Log]
- Purpose: external working-tree/index/ref changes must invalidate the existing
  comparison cache and update the same authoritative status projection.
- Existing assets: Workspaces owns root snapshots, stop token and TaskTracker;
  comparison::Reader owns bounded active/completed reads and root/project
  invalidation. Git owns protected commands and common-repository assets.
  vessel-workspace already owns notify8.2.2 for C5's finite file observation.
- Reuse decision: extend these owners. C5's observe-file contract registers one
  file parent, completes within30s and invalidates a saved-file receipt; it is
  not the recursive checkout/shared-Git-metadata lifecycle C3 requires. W13
  confirmed it owns no repository observer/cache bus. No second file controller,
  root resolver, provider/router or comparison cache is justified.
- Rejected shortcut: periodic full diff reads in every pane. They duplicate
  work, miss shared-worktree ref scope and do not implement reference recovery.
- Consequences: observation needs its own bounded registration lifetime while
  reusing node tracked shutdown. Idle registrations must not occupy the finite
  file-work or Git read/write execution permits. Physical cleanup remains
  owned after delivery disappears; no new arbitrary worker pool is implied.

## Source boundaries checked

Paseo packages/server/src/server/workspace-git-service.ts and
git-metadata-event-rules.ts own the reference lifecycle/classification.
checkout-diff-manager.ts consumes it; it is not a second metadata watcher.
Current Vessel roots::Snapshot/Resolved preserve original root/project/base
facts. They are not an atomic filesystem lease; observation cannot upgrade
that into RootLease or infer a replacement root from cwd/session/alias.

Reference values to preserve: workspace refresh debounce1000ms; ignored-dir
refresh quiet300/max2000ms and known-directory cap50000; subscribe deadline10s;
observation re-ensure60s; degraded polling5s doubling while quiet to60s; recovery
30s up to300s with4 bounded backoff steps, continuing recovery. Background fetch
interval180s, fetch only when origin exists, and a noninteractive bounded120s Git command;
metadata echo suppression5000ms. These are source facts, not current behavior.

Working-tree change is root-local and invalidates Uncommitted/All consumers.
Repository metadata is shared by the authoritative project's worktrees. Event
routing must preserve the ordered lock-ignore rule; worktree index vs HEAD,
config.worktree and Paseo metadata; common local/remote refs, packed refs,
reftable/config; administrative/objects/logs/hooks ignores and derived pruning.
A malformed or namespace-only ref event needs a typed structural fallback, not
an invented valid branch or a silently stale cache. Do not expose raw path/ref
strings as executable Git arguments. A new pure `vessel_workspace::git::metadata` decoder now owns this ordered
inventory. It derives pruning from the same rules, preserves typed validated
ref names and reports namespace-only events explicitly; malformed ref names
retain `InvalidText`. Native separators normalize at this decoding edge.
Its source/tests are newly authored and unverified; no native watcher, Node
cache invalidation callback, subscription or first runtime consumer is wired.

## Joins and tests still owed

Registration must precede its initial snapshot. Coalescing/dedup must be bounded,
retain errors and have no lost-change window. Native metadata paths come from
the SAME resolved checkout/common-dir authority, never a caller path. Root
retirement/shutdown must stop admission immediately and retain actual observer
cleanup until finished. A late old generation cannot install a successor value.

The selected-connection Unary boundary is checked; a real streaming/follow
consumer still needs the ONE G4 lease/follow producer. No alias fallback, second
connector, fake push via a pane timer or successful-read-as-write-authority.

Tests need real temporary Git worktrees, file/index/ref changes and atomic
renames; source rule/namespace/pruning cases, shared-project vs unrelated-root
scope, registration-before-read races, sticky errors and recovery, bounded
coalescing and original-root retirement. Control time through paused Tokio time
or explicit events; wall-clock sleeps do not coordinate fixtures. Include lost
delivery and shutdown through actual native cleanup. No real remote/credential,
provider, agent, OS opener or live checkout is needed for these tests.

The decoder installs no watcher, RPC, protocol number, default
handler, polling loop or fetch. It does not extend the V3 values-only approval
or borrow the announced node gate as proof of the current C3 runtime.
