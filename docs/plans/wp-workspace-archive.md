# Workspace archive and recovery

Status: staged; identity types are reviewed, registry persistence is a separate dependency.

## Reference and existing assets

Paseo 0.15.4 `packages/server/src/server/workspace-archive-service.ts:136-205`
stops setup, marks archiving, archives exact workspace contents, then considers
backing deletion. Lines 515-564 select live/stored agents by workspace ID and
continue after individual teardown failures. Last-reference and ownership checks
precede disk removal; a failure can leave an archived directory behind.
`server/session/workspace-recovery/workspace-recovery-service.ts:95-185`
reinspects project/directory availability, unarchives an existing directory or
recreates a recorded worktree, then unarchives the workspace. It does not resume
its agents or rerun setup as part of recovery.

Existing assets: `WorkspaceId`/`WorkspaceOperationId` are the canonical UUID
identities; `RemoveWorktreeOutcome` carries retained/removed backing evidence.
The session archive and terminal shutdown paths already exist. They lack a
workspace-wide ownership/start-admission fence, so applying them by cwd would
stop unrelated resources and race provider startup.

## Decision Log

- Purpose and intent: archive only resources admitted into one durable workspace;
  recovery keeps that workspace identity and never silently starts its agents.
- Constraints: registry/store dependency is being reviewed independently; no
  duplicate registry DTO, UUID implementation or path-based owner lookup is added.
- Alternatives: infer ownership from cwd; archive metadata before draining leases;
  retry failed create/restore blindly; add an external lifecycle library.
- Choice: modify existing resource lifecycle through a small generic backend
  boundary. Associated claim/plan types keep registry revision/ownership opaque
  until its reviewed persistence adapter can be connected. Real workflow rules
  are exercised against an in-memory backend under A10 staging.
- Why: an exact ownership claim precedes shutdown, late actor registrations are
  settled before member enumeration, and backing cleanup is unreachable after
  an unproven shutdown. Existing Tokio and standard types are sufficient.
- Advantages, costs and effects: testable sequencing and retained typed causes;
  final product wiring and registry/physical-filesystem checks remain required.

## Staging and lifecycle requirements

The module is test-only until the registry adapter, request/response schema,
existing RPC guards and consumer wiring are complete. The pending method table
is available; protocol availability remains None until integration assigns it.
No new wire method is invented in the native app before the node implementation.

Archive claims persist Archiving before any stop. The backend waits through the
registry watcher until all earlier start leases settle; uncertain leases require
authoritative runtime reconciliation. Member admission is rejected while a claim
is active. Every member is drained even when another fails, but a partial drain
never permits backing removal or a success state. Final metadata CAS rereads the
revision after leases settle. Restore claims persist Restoring before any backing
creation; concurrent restores cannot create two trees. Final activation follows
backing and original-subdirectory validation, with sessions still archived.

Backing cleanup must retain local/unowned or still-referenced directories. The
existing worktree removal safety outcome remains the one evidence type. A timed
out backend operation must retain its durable claim while physical work is still
uncertain; cancellation is never proof that no side effect happened.

## Differences from Paseo

A failed drain retains Archiving instead of declaring a fully archived workspace
while a member may still run. This safety floor makes partial outcomes explicit.
The workflow has one finite safety deadline, shared by all its awaits; expiration
reports uncertainty and leaves the non-active durable claim for reconciliation.
Vessel uses its existing worktree retention/snapshot safety rules rather than
silently forcing removal of dirty work. A restore never reruns setup or resumes
archived sessions, matching the reference.

## Resume point

Done: exact reviewed identities merged after Quality B; reference and asset audit.
In progress: staged archive/recovery workflows and deterministic fake regressions.
Next: adopt the reviewed registry/store dependency, implement its backend adapter,
register pending typed RPCs, connect native recovery UI, then root/native gates and
real fake-fleet filesystem/process checks. No runtime workspace parity is claimed.


## Recovery wire foundation

`vessel_proto::workspace` reserves one `WORKSPACE_LIFECYCLE = None` threshold
and defines `InspectWorkspaceRecovery`, authoritative recoverable/unavailable
inspection, and an exact-identity restore report with stage/source failures or
uncertainty. Future action/state/reason/result values decode explicitly as Unknown;
clients must offer update/refetch rather than choosing a path-based fallback.
These are DTOs only: no Request implementation, method row, client call or app
command is advertised before the registry-backed node handler exists. Reuse the
registry producer's ArchiveWorkspace/ArchiveWorkspaces/RestoreWorkspace requests
when that reviewed dependency lands. No duplicate registry identity is introduced.

The workflow's internal RecoveryAction records a known operation that actually
settled; the wire action additionally admits a future Unknown value for older
clients. Conversion belongs at the node RPC edge, without widening the internal
executable action set. Pending registration and TypeScript reachability are part
of the later wiring commit, not evidence supplied by DTO roundtrips.

Narrow evidence: staged node workflow 12 passed/0 failed at afdcb766; node
all-target clippy passed at the same source. These do not cover a real registry,
filesystem/process shutdown or registered workspace RPC.


## Atomic session archive admission (CLI consumer)

Deletion shares the same admission mutex as archive and resume publication.
Waiting for an old ended run's persistence happens before admission; deletion
then re-finds the target under the mutex and keeps it through the store commit.
A stored snapshot outside that critical section cannot authorize deleting a
newly published resumed run. The admitted delete runs as node-owned tracked work
so a cancelled RPC waiter cannot interrupt its commit. The new regression holds
the actual publication mutex, observes tracked deletion without any row removal,
cancels its waiter, then releases admission and checks the committed deletion.
This test is written but has not executed. Physical worktree/terminal cleanup
admission remains separate and is not established by this session-row guard.
The regression drives the real delete future using its own registered wakes;
resume observes a test-only notification after the gated runtime exists and
before publication admission. Neither test spins on registry state or sleeps to
assume that an operation reached the barrier. Product admission is unchanged.

W17 identified a separate A2 node gap: a CLI snapshot preflight cannot safely
implement archive-without-force, because a prompt can be admitted before the stop.
`session_archive::ArchiveSession { session, condition: IfIdle | Force }` therefore
has a pending unary Operate row under the same A2 threshold. It rejects missing
or unknown conditions rather than defaulting to Force. The node returns only its
acknowledged SessionId and committed archived_at; the client supplies no timestamp.
It reuses session.mark's Operate scope and exact session ownership rule, including
agent-owned children. The method is forwardable under that same envelope guard;
it does not add an MCP tool or bypass ownership.

Existing actor assets are reused: the structured control mailbox already orders
prompts and queue changes, and Stopper already owns graceful escalation. Archive
IfIdle checks readiness/active turn at that actor boundary. Acceptance closes
control intake and marks the input route Stopping before acknowledging; the stop
token also prevents automatic queue advance. Force deliberately cancels an active
turn. A TUI cannot prove an idle turn, so IfIdle is Unsupported there. Ended
stored sessions are eligible; an already committed archive is an explicit Conflict.

The node holds the existing marking/resume/detach mutex across actor admission,
end and store mark in a tracked task, avoiding both a second parent/admission lock
and cancellation between stop and persistence. Conditional descendant cascade
still awaits reviewed W2 parentage; this pending call must not be assigned a
protocol or advertised as parity before that integration and its root gate.
No registry/schema migration is needed for this actor admission.


Forced stop follows the existing kill cancellation path so it can interrupt a
profile move instead of waiting behind that move's mailbox operation. The actor
closes its control channel and sets the explicit internal Stopping route when
stop begins; direct inputs then return Ended throughout the grace period. IfIdle
still checks at the prompt mailbox before any stop. Queue advance also observes
the stop token, so an admitted archive/kill cannot start the next queued turn.


Legacy session.mark Archive shares that admission core with explicit Force,
retaining its idempotent first timestamp. The conditional session.archive method
chooses an explicit already-archived refusal instead. This removes the old
stop-before-marking-lock gap without creating a second stop/parentage mutex.

Physical root exclusion is a second authority, distinct from registry lifecycle
and start leases. W9 owns `workspace::exclusion` and its physical root identity;
the actual adapter will consume its checked producer rather than key a lock by
WorkspaceId or cwd. Lock order is durable Archiving/Restoring claim, then root
admission, then physical cleanup/restore and verified postconditions. A guard
belongs to actual I/O until it settles, including after caller timeout; partial
outcomes leave the durable non-active claim. This seam is planned, with no
production adapter or root-exclusion implementation adopted in this package yet.

## Conditional finished-child contract (pending node method)

Paseo `subagents/archive-finished.ts:38-52,167-199` selects managed idle/error,
not archived and still attached to the expected parent; `select.ts:74-99`
includes managed sessions across all servers. ArchiveChildSession therefore
retains full parent/child SessionRef, routes to child.node and compares current
parent locally under Sessions.marking. No distributed parent lock is claimed.
Its outcome distinguishes committed node timestamp from ParentChanged,
AlreadyArchived and NotFinished skips. Future outcomes/skips remain Unknown.
The shared snapshot predicate uses runtime Activity::Idle or Failed status,
not a PR display phrase; absent activity and Starting/Exited/Interrupted are
not proof of a managed idle/error row. Final live actor IfIdle admission still
fences a prompt racing the snapshot. A live TUI cannot prove a turn boundary
and will retain the existing explicit Unsupported refusal rather than force.

The checked W2 current-parent core is now staged on the node tree. Its one
committed-parent lookup is reused under Sessions.marking (pub(super), with W2's
agreement); no second ancestry algorithm or lock is introduced. The pending
session.archive_child method has typed Request/Reply, client and guarded handler.
The child's full NodeId must equal the receiving node before mutation. This is a
person's archive-finished action, refused to local and delegated agents. The
tracked task checks current parent, durable mark and shared finished predicate
under marking, then uses actor IfIdle admission and the existing cancellation-safe
service completion. A prompt admitted after snapshot selection becomes NotFinished;
TUI Unsupported remains an explicit refusal. Detach cannot publish between parent
validation and stop/mark completion. Unknown future outcomes never count as success.

The method's feature threshold remains None, so pending admission blocks wire use
until integration. Call admission additionally requires W2 SUBAGENT_RELATIONS;
assigning the A2 method alone cannot enable an unlanded parentage dependency.
Real fake-backed tests cover committed timestamps, repeated
archive skip, full remote parent mismatch, a committed detach ahead of its live
publication and active turns without forced stop. Exact 6042fac0f executed 30
archive-filter tests with no failures (node 23, proto 7), including these child
and service guards. Strict changed-source clippy, fresh TypeScript declarations,
the whole gate and final V3 evidence remain owed. The ordinary ArchiveSession remains
the path for ended stored sessions.

Node archive unit tests prepare their fake harness fixture before entering the
named safety deadline. The deadline then covers actor readiness, persistence,
repeat/refusal checks and cleanup, so a broken admission or handover reports a
bounded failure rather than holding the build slot indefinitely. These tests use
only the existing fake runtime and temporary store.

Archive completion (forgetting the forge watch and ending beside terminals) now
runs in the same tracked admitted task as the archive mark, for both legacy mark
and conditional archive service calls. Dropping the RPC waiter cannot cancel
that cleanup or release resume/detach admission early. Service-owned Arc handles
stay in its completion object; Sessions receives a statically dispatched
completion, with no service manager field or new task registry. Completion must
not recursively acquire the marking lock. A gated fake completion regression
aborts its caller and proves cleanup continues while a subsequent mark remains
blocked until completion. Existing terminal close-timeout semantics remain;
workspace backing cleanup still requires its separate verified drain contract.

Stored-session IfIdle admission takes marking before find, not after a caller's
ended snapshot. Resume may prepare a gated runtime first, but publishes its
durability entry and store reopening under that same lock. If resume wins, archive
re-finds the live run and the actor refuses an active/starting turn without Force.
If archive wins, a later explicit resume is ordered after its durable mark. Added
fake-backed regressions observe a real gated resume held behind stored archive
completion, then the existing explicit-resume unarchive transition, and an active
resumed run after a stale ended snapshot. These new
tests are written, not executed; the earlier 30-test result does not cover them.

Real recovery additionally depends on W11's pending canonical admission/backing
generation extension. Archived→Restoring advances it atomically, even when an
existing directory is only unarchived; checked overflow refuses before effects.
A partial/restart retry keeps the claimed generation. New Active run/start
admissions must match it and reject old Ready/plans/routes instead of resuming
implicitly. W9's actual FD PhysicalRoot remains a separate physical authority.
No new generation type, workspace ID, local counter or stronger fake-store claim
is implemented here; the production adapter waits for the reviewed producer.

## Strict terminal drain: actual completion versus logical status

Only skipping spawn entirely can certify that no child existed. A spawn error
may follow partial child/pipe setup (`MissingPipe`, `NoPid` or `NotLeader`), so
any attempted spawn that fails retains unconfirmed process, descendant and I/O
facts. A later service retry cannot erase that uncertainty. The actual missing
program regression verifies this conservative boundary; refusal before any spawn
remains the separate positively observed no-child case.

[Decision Log]
- Purpose and intent: a workspace member stop must not authorize backing cleanup
  merely because a close RPC returned or a terminal status became ended.
- Existing assets and constraints: TerminalHandle::ended observes metadata.
  Shell output drain has a deadline, session/group sweeps log failures, script
  pumps previously aborted without joining, and a command-file removal follows final
  status publication. ProcessLifetime revocation is explicitly not cleanup proof.
- Alternatives: reuse legacy close/ended, infer exit from a PID, or add a second
  terminal registry. Each loses actual runner postconditions or ownership.
- Choice: extend the existing terminal runner's shared state with a single typed
  completion observation, issued only after its owned cleanup finishes. Keep
  process exit, descendant scope, output drain and auxiliary I/O facts distinct.
  Unsupported or unobserved facts stay Unknown; timeout or task loss cannot mint
  a clean receipt. Strict node membership drain retains the original handle and
  reports incomplete instead of forgetting it on timeout.
- Why: the actual runner owns those resources and can preserve a reason that
  legacy status currently discards. No client, NodeId cache, process snapshot or
  file path can manufacture that observation.
- Advantages, costs and effects: legacy close/status behavior stays compatible.
  Physical deletion remains unavailable whenever any required cleanup fact is
  unproved. In particular, a signal sent to descendants or a revoked process
  observation alone cannot certify their exit. This is a planned extension;
  deletion authority still requires every actual postcondition below.

The first source implementation adds `TerminalHandle::cleanup()`, separate from
ended() and ProcessLifetime. The actual actor alone owns the completion sender;
return publishes immutable process/descendants/I/O/auxiliary facts, and sender
loss is Abandoned. Final status keeps its legacy ordering. In particular a
script's command-file cleanup now completes before the new report, even though
the status was already final. No registry or wire method is added.

The strict workspace helper is still inside the test-only archive module. It
checks the original admitted TerminalId, requests stop on that exact retained
handle, and waits under the existing close deadline. It never forgets a handle
on timeout, abandonment or unconfirmed facts. The outer workflow retains its
durable claim and cannot use such a result to delete backing.

This is deliberately incomplete for spawned processes: existing group sweeps and
PTY cleanup do not expose enough proof, so those facts stay Unconfirmed. Script
pipe workers now have one owned Pumps guard; normal completion joins actual EOF,
and deadline/owner loss aborts them. Deadline cancellation is joined before the
run proceeds but remains incomplete. Read failure, receiver loss or join failure
cannot certify drained output; wait errors remain unconfirmed process exit.
Facts accumulate across service attempts, so a later clean attempt cannot erase
earlier uncertainty. A completed actor alone does not certify cleanup. A run
that never successfully spawned can certify no child/I/O only after auxiliary
cleanup also succeeds. Real descendant and I/O acknowledgements remain required
before normal terminal-backed workspace deletion can be connected. New component
and actual-runner regressions are written, not executed; no safety acceptance or
consumer-ready producer is claimed.
