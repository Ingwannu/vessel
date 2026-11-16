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


Queued comparison/history work revalidates the complete named root snapshot
again after obtaining its execution permit. Registry changes while waiting
refuse the original operation rather than substituting its new path/base.
The same fresh snapshot is consumed by the existing protected root open.
This validation does not promise an atomic filesystem snapshot against other
processes. The original node tracker remains the cleanup owner.

## Workspace metadata generation purpose (coordinator ruling, 2026-11-15)

### Context and existing assets

The complete-response decoder in vessel-llm is not a generation service: it does
not select routes, read credentials, admit or account for a workspace operation.
The title producer's private caller is session-bound and remains frozen under
its separate review. Its single-attempt policy from ADR0062 is unchanged.
Pretending a workspace request is a title, creating a temporary session, or
borrowing another session's admission/usage would misrepresent both ownership
and accounting.

Paseo's server/session/checkout/git-metadata-generator.ts:63–83,125–228 uses a
structured commit message or PR title/body, with maxRetries2 passed to the
structured response loop. The latter is an initial call plus up to two JSON or
schema correction calls **per candidate**, followed by the configured candidate
fallback policy. It is not two HTTP retries. Prompt patches are limited to
120,000 JavaScript string units for commits and200,000 for PR text; message and
PR title have a maximum72 JavaScript string units (UTF-16), not UTF-8 bytes or
Unicode scalar counts. Prompt/schema tests must distinguish these units.

### Decision and writer boundary

- One Models-owned workspace metadata seam owns admission, route/provider
  selection and accounting. The titles producer owns that shared seam and sends
  it through node-head/V3 review. No second key reader, router, provider stack,
  credential discovery or real-agent wrapper is introduced by C3/C4.
- G1 owns the authorized-root commit-message and PR-text callers above that
  announced seam. They retain original root/cancellation/correlation and use
  the existing protected diff reader and validated commit/forge text types.
  No private title-module visibility is widened to bypass the producer review.
- Workspace purposes use initial plus at most two schema corrections for each
  eligible candidate. Transport retry and schema correction are separate typed
  policies. Titles retain exactly one attempt; no setting silently changes it.
- Every actual attempt must be attributed to the workspace purpose and its
  original operation, with known/unknown usage and cancellation retained as the
  shared seam offers. No synthetic or arbitrary session receives ModelCall data.
- Candidate order/eligibility, credential access and paid-call admission remain
  the single Models owner's rules. The root caller cannot append providers,
  choose keys or replay after cancellation. A model returns wording, never a
  command, a path, an authority grant or a replacement repository.

### Alternatives and consequences

Extracting the frozen title driver would couple a root request to session title
application and its session-only lifetime. A new generic client/key stack would
duplicate route and secrecy policy. Treating the missing seam as successful
fallback generation would conceal an unimplemented feature. These alternatives
are rejected.

The reusable seam must be announced before the root callers are wired. Until
then generation is unavailable, not a fallback-only parity claim. Product
fallback text is allowed only after an actual classified structured-generation
failure, as in the reference; unrelated root/admission/I/O failures propagate.
Fallback origin and diagnostics remain explicit in typed results. PR fallback
branding must name Vessel, not falsely attribute this product's output to Paseo.
No real provider or installed agent is used by tests.

Required caller regressions include valid initial output, two corrected schema
failures, exhaustion without a fourth same-candidate call, candidate fallback
order, no HTTP retry disguised as correction, UTF-16 boundaries, classified
fallback versus propagated root errors, cancellation without another paid call,
and workspace-scoped accounting without a session. Original-root validation and
normal repository mutation reservation still apply before a generated commit;
no atomic filesystem snapshot against external edits is claimed.

## Merge cleanup implementation boundary (still unimplemented)

Paseo checkout-git.ts:3866–4035 operates in the worktree holding the base when
there is one, otherwise temporarily checks out the base in the current tree.
Merge-to-base restores the original branch only for the same checkout;
merge-from-base requires a clean checkout by default and chooses the most-ahead
usable base. Both detect conflicts and attempt merge abort. The source catches
and ignores some abort/restore failures; Vessel must retain those failures.

The existing project-wide Git writer reservation and same-snapshot root resolver
are reused. A worktree path printed by Git is not new filesystem authority:
the node must resolve a matching admitted project/worktree before running a
mutation there. An unregistered external checkout cannot silently become the
target. This is a safety refusal, not support for all arbitrary Git worktrees.
Pinned-base mismatch and qualified-missing base retain the same admission
distinction as the comparison/history reader; no new fallback resolver.

The node task tracker owns checkout/merge/cleanup through result publication,
even when the RPC caller disappears. Cancellation stops the mutation command,
then bounded cleanup uses the same original runner/root and retained reservation;
it must not inherit an already-cancelled command token and instantly skip
restoration. Node shutdown still waits for this tracked cleanup. No second
process pool, temporary session or detached untracked cleanup is introduced.

A failed operation and failed cleanup must both survive in a typed result.
Never report a clean refusal or success when abort/restore could not be proved;
the app must keep the original sent intent/receipt and explain the unresolved
repository state. The current reservation does not fence external CLI/agent
processes or registry lifecycle edits, so root snapshots are revalidated after
waiting and before each checkout mutation. It is not an atomic filesystem lease.

Before wiring, port the actual checkout-git.test.ts merge fixtures: base already
checked out elsewhere, restoration of a temporary checkout, same branch no-op,
dirty target, local/remote base choice, normal merge and squash conflicts. Add
Vessel regressions for unregistered targets, moved queued roots, caller drop,
node cancellation, abort failure, restore failure and project-wide writer/cache
invalidation. Fake commands use the existing testkit executable helper. These
are required tests, not executed claims. No merge RPC, method row, protocol
assignment or successful merge implementation is added by this amendment.

## Purpose-specific text units and prompt boundaries

Source rechecked: `packages/server/src/server/session/checkout/git-metadata-generator.ts`
(commit schema63–71, patch budgets81–82, caller139–187 and truncation247–252),
plus `utils/build-metadata-prompt.ts` (style replacement and fixed contract).
These are planned caller constraints, not an implemented generation service.

- The72 message/title schema limit is JavaScript UTF-16 code units. It is not
 72 Unicode scalar values, graphemes or bytes:36 supplementary characters fit,
 37 do not. Root callers validate the generated value against the typed native
 message/title too. A schema correction counts as another model attempt, never
 an HTTP retry; title generation remains private and one attempt.
- Patch budgets are120000 UTF-16 units for commit text and200000 for PR text.
 They must not accidentally inherit the title producer's smaller scalar input
 budget. The ONE Models owner will supply purpose-specific workspace budgets.
 Root callers still bound file-list/style overhead and retain truncation facts;
 a patch cap alone is not a bound on the entire prompt or its token count.
- Rust strings cannot retain an unpaired UTF-16 surrogate created by JS slice.
 If a patch boundary bisects a supplementary character, native preparation
 stops before that character, at most one unit short; it never corrupts UTF-8.
 This is a deliberate representation difference, not byte-exact JS slicing.
- An allowed nonblank project style replaces the default style wholesale. It
 never replaces the fixed purpose contract, JSON shape, root/path admission or
 safety rules. Generated values carry wording only, never command/root authority.
- No second provider/key stack, borrowed/fake session, private title caller or
 fallback-only parity. GKS confirms the workspace service/API is still pending;
 G1 owns bounded original diff/style input, schema and72-unit validation and
 root/write revalidation only after the checked seam exists. No paid provider,
 real agent, runtime call, protocol assignment or generation test is claimed.

Required future fixtures include BMP/supplementary boundaries at72 units,
120000/200000 patch boundaries with a supplementary character, style replacement
versus default (fixed contract preserved), initial plus two schema corrections
per candidate, provider fallback versus HTTP retry, and original-root/cancel/
usage settlement. These obligations do not relabel title-service results.

## File-call invalidation follows physical completion

The checked C5 base supplies one bounded tracked file task. Legacy write,
create-directory, rename, delete and C5 duplicate reuse it; do not nest another
comparison execution queue. Their owned-future constructors capture the original
registered-root snapshot before admission. A common private same-snapshot open
supports ordinary non-Git files while the Git wrapper retains its Git check.
Registry facts are checked around disk opening, without claiming a RootLease.

The existing comparison Mutation guard belongs INSIDE that actual retained file
future. Dropping the RPC waiter or timing out delivery is not physical completion
and cannot end its post-operation invalidation or release its physical permit.
Errors invalidate too. This joins RPC changes only; external/agent edits still
need the separately planned repository observer. Checkpoint restore is pending
its owner's actual worktree/project and blocking-worker join, with no new hook
claimed here. No automatic replay or known-no-effect inference from timeout.
