# ADR 0087: git panel runtime and observation

Status: proposed

## Purpose and intent

Provide the node behaviour needed by the three git panel targets in Paseo
0.15.4: distinct Uncommitted/Committed/All comparisons, classified history,
merge to/from base, generated commits, branch-owned stashes and live status/diff
updates. The existing git runner and root validation remain authoritative.

## Existing implementation and constraints

`git.status`, `git.changes`, `git.diff`, `git.log`, branch/write/stash calls
already resolve WorkspaceRoot before I/O. Legacy DiffScope::Base includes
working changes; it cannot represent committed-only. Commit requires a message;
there are no merge-to/from-base or generated-commit calls. Stash entries have
commit/index/message/time but no owning branch. Reads are bounded/paged and
git execution protects against repository-controlled helper configuration.
New protocol registration waits for Quality B; no provisional integer or
second method table is introduced. Registry identity belongs to ADR 0082.

## Alternatives

- Reimplement git through shell commands in the app: breaks authorization,
  paths, credential redaction and platform behaviour.
- Relabel Base as Committed: returns the wrong changes.
- Poll every individual file and status from every panel: duplicate work,
  lost invalidation, transient false empty states and high process counts.
- A handwritten temporary-file allocator would duplicate collision-safe names,
  exclusive creation and cleanup. The workspace-pinned tempfile dependency
  already used in tests provides the private index lease in production too;
  it keeps MIT OR Apache-2.0 licensing and introduces no git/parser engine.
  Existing process ownership and the shared filesystem observer remain reused.

## Choice

One node-owned runtime per validated checkout, with a shared comparison cache
and bounded observation demand. Typed query keys include root, comparison,
optional base and whitespace policy. Uncommitted drops irrelevant base state;
Committed resolves merge-base..HEAD; All resolves merge-base..worktree including
untracked paths without altering the person's index. Existing paged file diffs
remain usable; additions preserve binary/too-large/refusal information.

Pending method families (names validated against the single table at wiring):

| Call | Scope | Result |
|---|---|---|
| git.compare | Read | comparison summary with revision/coverage |
| git.compare_file | Read | bounded typed file hunks for that comparison |
| git.observe | Read | cancellable revision-based observation snapshot |
| git.history | Read | branch/base/remote classification and changed files |
| git.merge_to_base | Operate | typed outcome/conflicts, original checkout restored |
| git.merge_from_base | Operate | typed outcome/conflicts, failed merge aborted |
| git.commit_generated | Operate | bounded internal generation + add-all commit outcome |
| git.branch_stashes | Read | owned branch metadata with pinned entry identity |

Observation reuses the node's bounded cancellable watch pattern, not another
WebSocket transport. A lost connection ends demand; revisions/correlation reject
late replies. Deep diff changes invalidate bodies even when line counts match.
Status-only branch changes invalidate history and dependent forge state.

Paseo limits are named once beside their owners: 150 ms diff debounce; 64-entry
LRU; 15 s freshness and 2 s read suppression; 4 snapshot computations and 2
observation setups; 5–60 s degraded polling; 30–300 s recovery; 180 s passive
fetch without interactive authentication. No timing sleep is test coordination.

Generated commit input uses the bounded uncommitted diff, the existing internal
structured-generation service, a 72-character message and two retries. Failure
uses Paseo's documented "Update files" fallback. Fake generation tests never
invoke an installed real agent. A model may not choose commands, paths or roots.

## Safety and platform boundaries

All new reads/writes resolve WorkspaceRoot and effective scope before cache/I/O.
No absolute or escaping client path is accepted; renames retain both paths for
discard. No raw git stderr decides UI state. Existing configuration isolation,
credential redaction, execution deadlines and repository serialization remain.
Merge targets are resolved/validated under the same worktree ownership rules;
conflict cleanup cannot act on another checkout. Passive fetch cannot prompt.
Each correlated rejection settles a typed failure, including unsent/full/closed.
No service files or platform-specific signals are introduced. Private index
scratch entries are created owner-only (0700 directory/0600 file on Unix);
existing repository/index permissions are never changed.

## Compatibility and costs

Calls are pending until the integrator assigns admission in the one method
table. Old calls retain their existing semantics; older clients see no new
fields without protocol gating. New clients get an explicit unavailable state
on old nodes rather than a false Committed view or an unbounded pending read.
Method and older-peer round-trip tests cover direct/nested/forwarded paths at
wiring. Root and native gates run once for the completed package (A14).

The chosen bounded watch transport differs from Paseo's push-event envelope;
observable ordering, cancellation, freshness and ownership are preserved and
tested. Runtime/cache ownership adds state on the node but avoids per-panel
pollers and duplicate subprocesses. No stronger atomic filesystem snapshot is
claimed than the git reads can provide; changed revisions supersede old data.


## Comparison operation ownership

The library exposes only complete comparison reads, not the private index path
or a cloneable lease. Its summary and body entry points always finish cleanup
on a blocking worker, retaining both failures if a read and cleanup fail.
An explicitly bound cancellation token reaches every git helper command.
The node must run this future on its existing TaskTracker until it returns:
client disappearance drops only the reply receiver, never the cleanup future.
Node shutdown cancels git commands and waits for tracked cleanup. The separate
comparison queue admits at most64 reads and executes4 snapshots; it does not
replace the runner's process pools. Cancellation/shutdown and resource-bound
checks are required before pending method registration is called ready.


## Comparison cache ownership

The first cache implementation retains summaries and lazy typed body pages in
one64-entry completed LRU. Active requests have a separate64 reservation bound and
four execution slots. Tokio watch shares outcomes while the existing node task
tracker owns execution through cleanup; no connection owns that future. A forced
read names Refresh, Mutation or Recovery, retires sibling pages and awaits an
older cancelled build before another root admission and a shared fresh build.
Original callers can receive their own old snapshot; it never re-enters cache.
Root resolution precedes every hit and every retry. Existing git writes invalidate
before and after via a retained guard, including errors. File-system observation
is still separate work; cache freshness is not proof of a push subscription.


## History payload and identity

Classified history extends the protected runner instead of the legacy paged
log decoder: all workspace commits plus ten fork ancestors are one bounded read,
with explicit failure on output limits. File summaries/bodies remain lazy
immutable comparisons (an API-shape departure, not a loss of commit metadata).
The Git-observed abbreviation is a display-only4–64 digit value distinct from
CommitId, so core.abbrev does not become request authority. Publication is
observed reachability from all remote refs, without network access. Strict
metadata errors retain their causes instead of dropping commits or guessing time.

The node's pending git.history adapter shares the existing retained job admission
and takes base plus project_default from the same resolved authority snapshot.
Missing qualified base and output bounds are typed outcomes, distinct from an
empty complete history. Neither legacy log paging nor a new per-client worker
pool is used. Registration stays pending until package admission.

### Mutation ordering

Git RPC writes share a bounded project reservation across registered worktrees.
A fresh root snapshot after waiting must match the original directory/project;
queued work cannot follow a moved record. The weak lock directory carries only
coordination handles. This does not serialize external CLI/agent processes or
project/worktree lifecycle calls, and does not claim filesystem sandboxing.
Multi-command merge cleanup must additionally live in the node task tracker
until restoration completes, even if its requesting connection disappears.
