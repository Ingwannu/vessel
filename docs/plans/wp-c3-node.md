# C3 git runtime

Status: unadmitted runtime implementation. No new method has an assigned protocol
number. ADR0087 defines the contract. Announced82ae7c91 is merged as e103946f;
latest owned source af78f4aed includes the shared-project cache correction and
queued-root test lint fixes. Its exact changed-source four-crate all-target Clippy passed (60s, actual Node
Checking); later6e history fixture execution remains queued. Earlier860
library proof stays on860, e103 Clippy is red on the corrected test lints, and
no final whole package gate/review has passed.

## Structure and decisions

`vessel-proto::workspace::panel` defines comparison/whitespace/page identity
without changing legacy DiffScope or its admitted calls. `git::diff::scope`
implements committed-only resolution; `git::diff::all_index` implements All through a
private index. Both reuse the protected runner and typed summary/file parsers.
The shared readers accept a node-owned optional index; legacy scopes leave it
absent. File explorer state and workspace registry identity are separate.

Reference: `utils/checkout-git.ts:3508–3609` distinguishes base..HEAD from
base..working files and returns no comparison without a base. All restores
untracked paths present in the comparison tree into a private copy of the
current index, avoiding a false deletion plus a second added entry. Summary
and file reads use the same private index. Repeated cacheinfo arguments avoid
extending the runner with stdin; its literal-path environment already protects
path arguments. Requested invalid bases never fall back to the default.

The scratch parent is supplied by the node and must be absolute. The library
must not read environment/home to choose one. It reuses the existing regular
file opener (no final symlink or FIFO), an exclusive 0600 destination in a
0700 private directory, and a 64 MiB streaming copy budget. This budget is an
explicit safety difference from the reference's unbounded index copy. Argv is
bounded by 128 paths and 16 KiB of conservatively escaped arguments. Clipped
ls-files output or an incomplete restoration set is a typed failure, never
false completeness. UTF-8/object/path decoding preserves its original source.

The existing workspace-pinned tempfile dependency is now a normal dependency
for the complete comparison read API; ADR0087 and ARCHITECTURE record its
license, alternative and tradeoff. Each entry point awaits private-index cleanup
on success and failure. Bound cancellation reaches all helper commands. Node
TaskTracker ownership and its bounded execution queue retain active comparison
and history work; the node cache coalesces completed/active comparison payloads.
The runtime table rows remain pending with GIT_PANEL=None, so no new RPC is
admitted. The separate values-only leaf194/3fde has V3 corrected scoped prerequisite PASS
on82ae, with staged rows, None and exactly two existing policy deny arms. It
contains no runtime/cache. Integrator batch12 a1add2e8 includes it; the integrator
owns its missing generated TS before announcing gate49. No runtime number is
assigned by that values handoff.

## Verification boundaries

- Initial contract and committed resolver at 3611a6b4: protocol/workspace narrow
  clippy green; library tests 382 passed, 0 failed, 1 ignored.
- All/private-index at 1bf60f28 passed all-target workspace clippy, then its
  library tests returned 110 passed, 1 failed, 1 ignored. The failing actual
  filesystem assertion found a 0755 scratch directory, because tempfile's
  directory default is 0777 masked by the process umask. The corrected builder
  requests 0700 at creation, before copying any index bytes; no process-wide
  umask or post-creation chmod is used. Its changed-source check is pending.
- Logs: `wp-git-node-narrow-20261115T072022Z-1bf60f28` (clippy),
  `wp-git-node-narrow-20261115T073212Z-1bf60f28` (tests). Both compiled the
  changed workspace crate. These are staged library checks, not node admission.
- Post-base submissions of 4331b5b2/ea58824d ended before registration during a
  build-helper lock fault. They have no step evidence and were not rerun as
  successes. Registered earlier red/green runs remain unchanged.
- No new RPC admission, live node, UI, real forge, OS cleanup or complete parity claim.

## Resume point

Finish current narrow checks and run comparison/private-index library tests.
Tests cover mixed staged/unstaged/untracked files, unborn HEAD, base errors,
index bytes, recreated base files (identical/modified), literal path names,
staged rename preservation, argument/copy budgets, modes and no-follow access.
Then implement the owned runtime/cancellation boundary and register pending
Quality B rows in the single method table. Complete revision/cache/watch,
classified history, branch stash metadata, merge-to/from-base and generated
commit behaviour. Root whole gate and V3 review occur once the node package is
complete; app package verification remains separate.


Latest executed staged result at614d8026: workspace library111 passed,0 failed,
1 ignored, including actual Unix0700/0600 assertions. Run
`wp-git-node-narrow-20261115T074017Z-614d8026`. This is the fixed private-mode
source, not proof of the later complete-read entry points or their node caller.
The complete-read API adds real-git tests for paired summary/body cleanup and
cancellation after index preparation; those changed-source tests are pending.


## Owned comparison caller and pending registration

The next coherent source connects complete comparison reads to the node's
existing root authority, TaskTracker and shutdown token. Its separate admission
bound is64 including queued reads, with4 executing snapshots. A dropped reply
receiver never drops a running cleanup future. Shutdown refuses queued work,
cancels every executing git helper and waits for their cleanup. Barrier tests
cover receiver loss, execution/admission limits, queued cancellation and cleanup
ordering; real repositories cover node-level comparison semantics, invalid roots
and unchanged index bytes. These new tests have not been executed yet.

`git.compare` and `git.compare_file` have Read/unary rows in the one Quality B
table, pending `workspace::GIT_PANEL = None`. They route real workspace methods;
no dummy version, admission claim or stub success is added. NoBase, TooLarge
and a genuinely empty Ready remain distinct typed values. Partial summary
coverage is retained. The client in-memory transport test asserts no frame is
sent while pending. Agent access is explicitly refused like the existing git
calls. Request/reply, older projection, fake handler and alias metadata changes
are the minimal exhaustive registration consequences.

The previous complete-read narrow check (0a391e7f) returned only an unquoted
DiffScope name in documentation. The current source quotes it and needs its own
changed-source check. Revision/cache/observation/history and write policy are
still incomplete, and the package is neither built nor passed. The web wire
must be regenerated from the protocol generator before package closure.

The first pending-registration check at6e9db435 stopped in protocol compilation:
three exhaustive forwarding/field-admission/older-reply matches needed explicit
comparison arms. The remaining method sample and bridge classification were
found in the same audit and updated; no wildcard or broadened fallback is used.
A file-body request also rejects the relative root before any job is admitted.

An isolated real-git permission probe found another private-index issue before
claiming runtime readiness: git replaces an index via a lock file and can widen
its mode to0664 despite a0600 copy (even with sharedRepository=false). The
private-only update-index command now uses the documented numeric
`-c core.sharedRepository=0600`; a separate probe kept0600. This does not alter
repository config or the real index. The actual protected-reader regression
now uses a group-sharing repository and asserts0700/0600 after summary/body
reads and unchanged real-index mode/bytes. It remains unexecuted in this tree.

At a9301381 the full selected all-target narrow check compiled the production
protocol, workspace, RPC, client and node code. It returned nine test-only
unused-result errors (tracker close booleans and fixture git stdout). Tracker
closure is now asserted and quiet setup outputs are explicitly acknowledged.
No lint allowance or production behaviour was changed by this correction.
Those fixes and the group-sharing permission regression still need their
changed-source narrow check and execution. Before the eventual whole root
gate, merge exactly the reviewed base security-test race fix c5da5305; do not
edit that test locally or relabel its base failure as feature evidence.


## Latest execution and common bases

Selected all-target clippy at17257659 passed for node/proto/workspace/client/RPC
(`wp-git-node-narrow-20261115T081013Z-17257659`). Library execution in
`wp-git-node-narrow-20261115T081315Z-17257659` returned608 passed/2 failed:
client80/0 and node248/0 passed (including the owned job barriers and real node
comparison cases); protocol280/2 stopped the remaining crate execution. The
forwardable-set expectation omitted the two intentionally forwardable Read
calls and is corrected here. The other failure is the base protocol-10 prefix
family assertion assigning every git.* name version10, including pending
methods. The integrator has announced its explicit family-list correction for
gate40; wait for that reviewed head and put these names in its later-family
list. Do not invent versions or weaken that base assertion locally.

The group-sharing/private-index cancellation library cases have not yet run:
Cargo stopped at protocol before workspace/RPC library execution. No claim is
made from compilation or the separate raw git permission probes.

Both trees merged reviewed base3(6c782cb6), including c5da5305; node merge
5f760ac48 and app mergeaf28ae83. The app also takes base4(3cd0d3fc), whose root
is unchanged. Future package evidence must identify its combined source rather
than relabel earlier immutable narrow runs. New comparison base selection
work is separate and still unverified.

## Comparison branch selection

A further read of checkout-git.ts:1704–1735 and its real-repository tests found
that comparison and history intentionally choose bases differently. The new
comparison resolver now prefers origin for unqualified names (including short
origin/main), preserves exact refs/heads or refs/remotes streams, and refuses a
missing/malformed requested ref without substituting a local one. Explicit
commit bases keep Vessel's existing typed extension. Legacy DiffScope calls
are unchanged. Decoding retains the comparison error's UTF-8/object sources.

New real-git cases model stale/diverged local main, an observed origin ref,
working changes, qualified local/remote selections, and a remote pointing at a
non-commit object. They assert both committed/All file sets and unchanged real
index bytes. No network or real agent is needed. These additions require their
changed-source check and execution; old17257659 results do not cover them.

## Gate40 integration

Merged ba45a6e3 after the prior narrow check completed. The two pending Read
comparison rows now live at the end of rpc/table.rs, and their names are in
WORKSPACE_FAMILIES_LATER; their protocol remains unassigned. Both architecture
additions are retained. This closes the old prefix-family fixture conflict
without assigning a protocol or changing a legacy call. Gate40's published
whole results are base evidence only; the combined package still needs checks.

## Recorded base admission

The existing roots::Resolved now carries pinned_base beside its unchanged base
fallback, captured under the same state lock. Projects have no pinned ref;
worktrees retain their recorded name before the project fallback. W11 confirmed
this small addition has no overlap with its registry work. Its first caller is
the comparison service, which admits a display alias or exact ref, then retains
the recorded stream. A different qualified ref returns BaseMismatch with both
typed facts before reserving work; neither ref is labelled correct. Immutable
commit and uncommitted comparisons retain their separate meanings. This follows
checkout-git.ts resolveOperationBaseRef/isSameBaseRef rather than a second root
resolver or a symmetric label comparison.

Real-node fixtures use a worktree created from refs/remotes/upstream/main and
assert the same-snapshot facts, a successful main display alias, and summary/body
refusals even after the job tracker stops. Wire tests retain both identities.
This changed tree still requires narrow checks and execution. Prior base-selection
clippy at5f726c04 passed; it does not cover this admission addition.

## Shared comparison cache and owned delivery

The node now retains one bounded cache for typed summaries and file pages. The
pure state follows checkout-diff-cache.ts and its own test fixtures: completed
LRU64, active ownership independent of eviction, freshness15s, minimum read gap2s,
forced-read reason, cancellation followed by cleanup, shared retry, and prevention
of stale/failed completion repopulation. Root admission runs before a cache hit
and again after waiting for an invalidated build. The existing four-worker/64-job
Jobs tracker remains the only execution owner; dropping reply receivers does not
drop preparation, subprocess cancellation or private-index cleanup.

This is an API-shape departure: Vessel's existing lazy typed file pages share the
same64-payload budget with summaries, instead of storing a whole text patch. File
path and hunk continuation are cache identity, and a forced part refresh retires
its sibling parts. There is no second patch parser. Arc shares result/error
ownership with original typed error sources; the workspace wire adapter delegates
to that original error rather than flattening it into a string or a false empty
result. Existing writes retain a guard which invalidates both before and after
their work, including partial failure/cancellation. File-system observation/push
and the remaining history/generated-write work have not landed yet; this cache
alone does not prove live observation or full C3 parity.

Deterministic barriers cover shared active builds, cancellation cleanup, dropped
receivers, failed-read retry, and reads during a write guard. Real repositories
cover explicit refresh, a commit invalidating cached work, and revalidation of a
moved cached root. The cache and new typed GitReadReason are new changed-source
work and still require their narrow checks and execution. The previous
f539a620 node/proto/workspace all-target clippy passed; its selected execution is
still queued and does not include this cache.


Selected f539a620 execution (wp-git-node-narrow-20261115T091000Z-f539a620)
finished552 passed/1 failed: Node254/0, protocol298/1, then Cargo stopped before
workspace. All real-node recorded-base summary/body admission cases passed.
The remaining pinned-forwardable assertion still expected the two new methods
beside git.changes, although gate40 integration correctly moved their table rows
to the end. This change moves only those two expected entries to the table's end;
no membership/version/permission is weakened. Its original red log is retained.
The next changed-source suite must execute that regression and the previously
unreached workspace cases along with the new cache tests.


Cache all-target clippy at36121311 stopped in the new source: the immutable-cache
fixture needed an explicit payload type; CacheError had narrower visibility than
the existing crate-visible WorkspaceError payload; one method-name doc needed
code markup; the child module needed named imports; borrowed lifecycle references
were passed by value without declaring their Copy semantics. This correction
retains the same algorithm, makes error payload visibility match its containing
error (and no wider), names imports and derives Copy only for the two borrowed
lifecycle capabilities. No lint allowance or source-chain flattening is added.
Original log: wp-git-node-narrow-20261115T093141Z-36121311. New execution remains
unverified until the changed-source checks finish.

The5e990a26 all-target check reached the new cache and returned one remaining
test-helper lint: extracting a build ID did not consume the generic decision.
The helper now borrows that decision explicitly; production ownership is unchanged.
The original red log remains node-comparison-cache-fixed-clippy. Execution of
cache and previously unreached workspace cases is still pending.


## Complete classified history core

The new panel history reader follows checkout-git.ts2585–2695/1736–1774 and
checkout-git.commits.test.ts: all workspace commits (not the legacy200 cap),
ten ancestors starting at the true fork, exact current metadata, and publication
against every observed remote ref. A most-ahead local/origin tie keeps local;
comparison's separate origin preference remains unchanged. A shared strict ref
reader removes duplicate subprocess admission without changing comparison errors.
Missing qualified refs refuse; only stale unqualified names may use the same
snapshot's project default. Detached/unborn checkouts have no branch history.

The proposed wire values keep location/publication as enums with Unknown decode
fallbacks; observed abbreviations have a distinct4–64 digit display type and
requests continue using full IDs. Strict NUL metadata avoids record-separator
subjects, dropped bad parents, truncated subjects and false epoch timestamps.
Per-commit file lists remain lazy immutable comparison reads rather than repeated
history payloads, a deliberate API-shape difference. Legacy git.log is unchanged.
Real-repository cases use the reference's shapes, including205 workspace commits,
a remote other than origin, diverged/tied bases, missing bases, detached/unborn
HEAD and core.abbrev4. These new library/value additions are uncompiled and
unexecuted until their own changed-source narrow check; they are not a registered
ready git.history call, actor integration or package completion.

The node tree merged announced head d432ef0f at f8a583d77. Only the architecture
appendix conflicted; both the comparison-cache note and all incoming speech,
connection-lifetime/process/system-context notes are retained. The app tree did
not take this node head. Prior narrow evidence keeps its original source hashes.

## Owned classified history call

The node now wires git.history as a pending Read/unary row at the table's end,
with the same GIT_PANEL=None admission. It resolves root/base/project_default
from one state snapshot (W11's approved roots seam), then uses the existing
four-executing/64-admitted Jobs owner. Dropping delivery cannot own or abandon
the read; shutdown reaches all protected Git commands. No second process pool,
cache, filesystem resolver or transport is added. History has explicit Ready,
BaseUnavailable(exact qualified ref) and TooLarge outcomes; malformed metadata
retains a typed source failure. Its thirty-second UI freshness is separate from
the comparison cache, which is not silently reused as a history cache.

Reference-derived core fixtures and real-node worktree tests cover retained
qualified refs, same-snapshot defaults, a deleted qualified base, a moved root
and shutdown admission. Protocol/client fixtures pin pending Read/unary, exact
outcome serialization, future classification preservation, unchanged legacy log
admission, and two unsent pending requests against the same memory peer. They
are new source and remain unexecuted; the queued325fec9d Clippy covers only the
previous core, not this wiring. No protocol number is assigned and generated
TypeScript still awaits the package gate/artifact regeneration.

The simultaneous10:35:42 cancelled143 runs were confirmed by build tools as
infrastructure interruption from an operator-authorized build-VM restart, not
source/compiler failures. Their logs stay intact and are not counted as tests.
The new325fec9d core Clippy is the meaningful replacement for the interrupted
old node run. No duplicate old run is submitted. All Rust compilation stays on
the remote build VM; local checks here only inspect text and boundaries.

### Exact history-wiring test attempt

Run wp-git-node-narrow-20261115T110153Z-ba94e9e6 completed web-ci and
nested-fakes, then nested-vesseld failed compilation with E0277: the new
ReadGitHistory request lacked its explicit guard::Target declaration. The
library tests never started. Add it to the existing no-session-target list:
this is a registered-root read under its method Read scope and workspace agent
policy, not a session ownership operation. The node still revalidates the root.
A new changed-source remote run is required; no previous result is relabeled.

## Git RPC mutation reservation

The existing index/commit/fetch/push/pull/switch/rename/stash entry points now
share a project-keyed mutation reservation, covering that project's registered
worktrees. At most64 writers wait/execute. A weak mutex directory retains no
closed project; only coordination metadata is recoverable after poisoning, never
authority or repository facts. No map guard is held across await. Waiting checks
node shutdown; after acquiring the project lease, the node re-resolves the
original root and refuses a changed directory/project rather than retargeting.

This is a prerequisite for a multi-command checkout/merge/restore operation,
not that operation's implementation. It serializes Git RPC writes only: external
CLI/agents and project/worktree lifecycle operations are not globally locked.
No filesystem isolation or atomic snapshot is claimed. The lock ordering is
project reservation, then a fresh short registry snapshot; the registry lock is
released before I/O. Existing mutation-cache invalidation remains inside it.

Added unexecuted source tests cover FIFO for one project, independent projects,
64-slot refusal, shutdown while waiting, weak cleanup and a real registered-root
replacement after queue admission. No sleeps/polling timers coordinate these;
a test-only Notify exposes the actual admission boundary. Production has no
notify counter. The current860 remote suite is prior-source and excludes these.

Mutation admission now captures a named registry-only root Snapshot before the
64-slot reservation; no filesystem work precedes capacity admission. The old
eight-field tuple is extracted to named project/worktree snapshot helpers,
retaining all base/default/pinned facts from the same lock and the old mapping.
The post-wait resolve remains the only filesystem authorization and is compared
with that original candidate path/project. No snapshot is a substitute for it.


## Queued read authority correction

Source inspection found that comparison/history workers captured a resolved path
before waiting for their execution permit. Revalidating only on the initial
cache probe did not revalidate the target when queued work eventually began.
The operation now retains a weak reference to the existing Workspaces owner and
the original named registry snapshot; it opens again after execution admission.
All recorded facts, including owner repository, pinned base and project default,
must still match. A changed snapshot is a typed conflict, never a new target.
The open consumes that same fresh snapshot without another registry lookup.
The mutation reservation uses the same helper rather than a path-only check.

This remains per-call validation, not filesystem isolation or an atomic snapshot
against an external process. No new registry, resolver, connection or process
pool is added. Weak ownership does not extend a dropped workspace runtime; the
existing node tracker still retains started work through cleanup. A root-only
history read now captures registry facts before admission and performs no folder
I/O until it has an execution permit.

New unexecuted real-owner tests hold all four existing execution permits, wait
on the actual submission boundary, change a registered path or default branch,
and release the permit. Comparison and history must both settle GitTargetChanged
(with the comparison's existing shared typed error wrapper), not return an old
snapshot or follow the replacement. Test-only permit/Notify access adds no
production wakeup, map or second queue. Prior860 and c840 remote runs exclude
this correction; changed-source remote checks are still required.

The V3 values-leaf review also applies to the runtime branch's family fixture:
unassigned git.compare, git.compare_file and git.history stay outside
WORKSPACE_FAMILIES_LATER. The shared family guard and the package's explicit
Read/unary/None fixtures both remain active until assignment. This changes only
the test guard, not node admission or a pending runtime row.

## Exact history-guard library result

Run wp-git-node-narrow-20261115T115802Z-860e28cc completed web-ci,
nested-fakes, nested-vesseld and cargo with rc0/no skipped steps. All four
changed crates actually compiled. Library counts are Client80/0/0, Node284/0/0,
Proto301/0/0 and Workspace131/0/1: total796 passed, zero failed, one ignored.
This executes the comparison/cache/private-index/history and pending-client
fixtures, including 0700/0600 permissions and cancelled cleanup on that exact
source. It does not execute later c840 write reservations,21ea queued-root
corrections,45b family-guard correction or the ede20014 merge. It is not a whole
root/native gate, staged-values-leaf validation, admission, real forge or UI pass.
Those later source checks retain their separate queued runs.

The coordinator accepted the purpose-specific generation split: the titles
producer owns one Models workspace admission/routing/accounting seam (V3/node
head); G1 owns authorized-root commit/PR callers above the announced API. The
ADR0087 amendment preserves title one-attempt policy and gives workspace
purposes initial plus up to two schema corrections per eligible candidate.
No fake/borrowed session, second provider/key stack or fallback-only success is
allowed. This is a documented contract; no reusable service is available yet,
and no generation/paid-call test or parity claim has been made.

## Exact mutation-reservation narrow finding

Run wp-git-writes-narrow-20261115T122538Z-c840a586 returned cargo101/E0061
in the new registered-root test: vessel_testkit::within requires both a Duration
and its future. No mutation-reservation execution passed in that run. Correct
that test and the later queued-read fixture to use named60-second safety nets
and consume the Result; add a named10-second safety net to the pure lease tests.
No sleeps coordinate any fixture: real mutex futures and Notify still control
ordering. Only a missing timeout argument/test safety-net behavior changes;
production reservation and root fencing are unchanged. Already queued21e/3be
runs retain their earlier source, including this known test-only compile defect.

## Shared-project ref invalidation

Source review found fetch/push/commit invalidation was keyed only by the calling
WorkspaceRoot, leaving cached Committed/All results of a sibling worktree fresh
for15s after shared refs changed. Cache keys now retain the ProjectId from the
SAME resolved authority snapshot. Working/index mutations remain root-local;
History/Repository invalidation matches that project, including active builds
which stay owned until cancellation cleanup. No path/alias lookup, second
registry or authority grant is introduced. Unknown external CLI changes still
need the pending observation lifecycle; this does not claim watch parity.

The existing before/after mutation guard carries both root and project. Pure
fixtures cover all three change classes against Uncommitted/Committed/All/fixed
Commit, active and completed sibling entries and a different project. Actual
changed-source checks are pending; earlier860/e103 evidence is not relabeled.

The e103 changed-source all-target Clippy reached Node test compilation and
returned three queued-root fixture lints (two semicolons and an open error
wildcard). The fixture now asserts the exact expected direct/shared error by
its closed Reader kind. No lint is suppressed. The new sibling-cache scope is
included in the next meaningful Node Clippy source; e103 itself is not green.

C5 connection: W13 confirmed it has no competing comparison-cache mutation hook
or git watcher join. A before/after guard around file writes/create/rename/delete,
including partial-error invalidation, is one pending post-C3 integration join.
No C5 file was changed, no moving runtime adopted, and no shared watch is claimed.

## Values approval and active source assignments, 2026-11-15

V3 row196 corrected scoped prerequisite PASS applies to exact19460b8c4a4702f4c30de88a8250bac9c2676de6
plus Markdown3fdeed6a, on announced82ae. Actual all-target strict Proto/Client/Node
Clippy0 and actual Client80+Proto322=402/0/0 were independently audited. It is
staged values only: no NodeService handler, guard target, runtime/cache or protocol
number. Integrator batch12 a1add2e8 merges that leaf and owns TS regeneration;
only its later announced head may be adopted. Do not replace this branch's real
pending runtime/handlers with the staged leaf accidentally during that merge.

Current runtimeaf78f4aed is distinct. Its strict four-root-crate Clippy run
wp-git-writes-narrow-20261115T135839Z-af78f4ae is queued. Existing exacte103
workspace-test selection wp-git-root-fence-tests-20261115T133413Z-e103946f is
running: its web/fake-binary/node-binary prerequisite stages passed, but no final
library result is available yet. It excludes later shared-project cache scope
and fixture lint corrections. Keep those exact source assignments; neither is a
whole root gate or a runtime review pass. No additional local Cargo was started.

## Actual e103 history fixture failure

The exact e103 workspace selection executed66 tests:65 passed,1 failed,0 ignored
(run wp-git-root-fence-tests-20261115T133413Z-e103946f). All three prerequisite
stages passed and the changed Node/Workspace crates actually compiled. The sole
failure was the history fixture expecting a moved-folder Conflict AFTER cancelling
shutdown. Current owned-job admission correctly refuses Closed before queueing disk
validation; no production failure or runtime fix is inferred from this test error.

The corrected fixture first moves the root while live and asserts its exact
Conflict detail, then cancels shutdown and asserts Closed even with the root still
missing. Both requirements remain covered without weakening an assertion or
changing production ordering. The new source also includes the later sibling-cache
scope and still needs execution. Prior e103 remains65/1; no red is relabeled green.

## Exact shared-project correction Clippy result

wp-git-writes-narrow-20261115T135839Z-af78f4ae finished cargo0 in60s on exact
af78f4aedb1fdbff0adbc97fe18310c38f792dd9. The command selects Node/Proto/Workspace/
Client all targets with -D warnings; actual changed Node Checking is in the log.
This closes the known e103 fixture lints and compiles the later project-key/cache
scope. It does not execute that scope's regression or later6e927990 fixture,
assign a number, admit a runtime, or replace the required whole package gate.
Full spec/log artifact digests are retained in the track's external evidence
packet af78-root-narrow.json; no unchanged rerun is needed.
