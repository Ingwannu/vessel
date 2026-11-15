# ADR 0077: File checkpoints at turn boundaries, independently of conversation rewind

Status: proposed, 2026-11-15. Design only; no implementation or protocol
number is approved by this document. Extends ADRs 0027 and 0052 without
changing their existing calls. Implementation waits for review and the
central node/app baseline merge. No runtime setting or dependency changes
are made by this draft.

## Purpose and intent

A person can restore the files before a prompt or after a completed turn,
see every affected path before confirming, and see the actual outcome.
Files-only restoration keeps the conversation, model context and prompt
draft. A checkpoint is local recovery data, not a commit, an agent message
id, a conversation fork, or a backup of everything the agent can reach.

## Existing implementation and constraints

References read for this decision, Paseo source at `1c00b365c`:

- `packages/app/src/components/rewind/use-rewind-capabilities.ts:4,24-60`
  separates conversation, files and both, with independent capabilities.
- `packages/app/src/components/rewind/rewind-menu.tsx:68-81,98-144`
  locks pending actions and displays a warning. It does not implement a
  separate confirmation listing the changed paths.
- `packages/app/src/components/rewind/use-rewind-agent-mutation.ts:32-58`
  sends the selected mode, avoids a conversation reload for files-only,
  and displays errors. `packages/client/src/daemon-client.ts:3947-3977`
  correlates the request and rejects a response whose `ok` is false.
- `packages/server/src/server/agent/providers/claude/rewind.ts:142-168`
  calls `query.rewindFiles(messageId, { dryRun: false })`, rejects a false
  `canRewind`, and performs files before conversation in the combined mode.
- That provider's `agent.ts:3114-3125,3161-3185,3244-3258,3706-3707`
  enables SDK file checkpointing, reports optional changed-file statistics,
  and marks a query stale after transport failure. Its reference is the
  user's message, not Vessel's end-of-turn ACP message.

The [Claude SDK checkpoint guide](https://code.claude.com/docs/en/agent-sdk/file-checkpointing)
was read on the date above. It describes prompt-start restore points,
backups for the main conversation's file editing tools, files-only rewind,
and removal of created files. Shell-only and subagent changes have limited
coverage; directory layout is not restored. Linked or unsafe paths can be
skipped and reported by the SDK. These are SDK semantics, not evidence that
an ACP adapter implements file rewind. No SDK or real agent was executed.

The broader checkpoint presentation also appears in GOAT:
`apps/web/src/components/chat/TimelineAssistantRows.tsx:297-320` draws a
turn's changed files, and `chat/useTimelineRewind.ts:180-233` refuses a busy
thread, confirms, calls rollback and displays failure. GOAT's action also
rewinds thread state. This ADR takes only its turn-associated presentation.

Vessel node source inspected at `242076b5`:

- `vessel-proto/src/workspace/worktree.rs:355-370` describes a removed
  worktree snapshot, with no session or turn association.
- `vessel-node/src/workspace/worktrees/snapshots.rs` lists/deletes those
  refs; `worktrees/start.rs:88-112` recovers one into a new worktree.
- `vessel-workspace/src/git/worktree.rs:280-390,438-453` stores a temporary
  index as a Git commit and restores it in a fresh checkout. Its single
  ref per worktree cannot be treated as a sequence of turn checkpoints.
- `vessel-node/src/service/forking.rs:47-66,153-182` rewinds conversation
  and counts subsequent editing tool calls. That count is not a file list;
  files remain changed. ADR 0052 deliberately has no ACP file restoration.
- `vessel-workspace/src/paths.rs:1-23` already records the same-user
  path-swap threat boundary; `regular.rs` refuses nonregular reads and
  final symlinks on Unix. Hardened Git enumeration and `RelPath` validation
  are reusable. They do not constitute a transactional filesystem.

Crate paths above are relative to `crates/`. The native app's old root
baseline lacks later node contracts. No package independently merges or
reimplements the central baseline update.

## Alternatives considered

1. Relabel `session.rewind` or recover a removed-worktree snapshot: rejected,
   because neither restores the existing folder at a selected turn.
2. Use Claude SDK backups/CLI and infer per-provider file formats: closer to
   Paseo internally, but breaks the ACP boundary of ADR 0003, excludes other
   agents, and makes a provider's private backups a node storage contract.
3. Intercept editing tool calls: insufficient. ACP updates report work and
   need not precede filesystem writes; shell edits are not comprehensively
   observable there. No snapshot can be reconstructed from an event count.
4. Reuse Git snapshot commits for every turn: possible for Git trees, but
   puts historical contents into the user's Git object database and couples
   retention to Git's reachability. The deletion-recovery ref stays separate.
5. **Chosen: bounded node-owned file manifests at turn boundaries**, with
   preview and durable restore results. Extend existing session/workspace
   infrastructure; introduce no second provider driver or general backup tool.

## Decision: capture and coverage

Initial support is structured sessions in a node-managed Git worktree whose
root identity and repository link still match the node's record. Ordinary
project folders, imported historical turns, terminal sessions, nested mounts,
and roots shared with another live session are explicitly unsupported for
capture. This restriction is a stated difference from Paseo's provider mode.
A capability names this scope; the app never advertises a universal backup.

For each real new turn, the node captures `BeforeTurn { prompt: EventSeq }`
after accepting/logging the initiating prompt but before delivering it to the
agent. Steers/tool results joined to a running turn do not make another
boundary. Queued prompts capture when actually starting, not on enqueue.
After persisting `TurnEnded`, it captures `AfterTurn { ended: EventSeq }`
before releasing the next prompt. This point needs no `TurnPoint.message`:
file recovery is independent of whether ACP supports conversation forks.
Interrupted/crashed turns retain their before point; they get no fabricated
after point. Empty changes may share blobs, never an invented anchor.

The session actor awaits a typed capture acknowledgement from node orchestration;
it gains no dependency on `vessel-workspace` or `vessel-store`. A capture
failure yields `Unavailable { reason }` and the prompt/turn continues. The UI
then shows no restorable checkpoint at that boundary. The deadline bounds the
actor's wait, including admission/queue time, not arbitrary kernel I/O.

Each request carries a fresh `CaptureId`, immutable session/root/boundary
identity and `CaptureGeneration`. A nonblocking coordinator owns its lease:
`Waiting -> Ready` or `Waiting -> Unavailable`, exactly once. At the deadline
it invalidates the generation, requests cooperative cancellation, and sends
`Unavailable { CaptureTimedOut }` without joining the worker or waiting for
filesystem cleanup, fsync or storage acknowledgement. The actor can deliver
the prompt/next turn at that point. A persisted unavailable record may follow;
absence of a ready record is already non-restorable. The coordinator checks
the clock on completion too, so timer/completion scheduling cannot admit a
result whose deadline has elapsed. Duplicate or superseded completions cannot
change the terminal lease state.

The worker may only enumerate/read and write its private candidate staging;
it cannot publish ready metadata, mutate project files, or transfer its result
to a later turn. Only the coordinator's publication path may accept a fully
durable candidate for its still-live matching lease before deadline. A pending
publication is fenced by that lease as well: an expired lease stays unavailable
in reads even if a storage completion arrives later; such rows/artifacts remain
unaccepted candidates for reconciliation, never independently readable ready
records. Startup treats unfinished publication leases as unavailable, not as
permission to promote candidates. Publication/timeout races are a required
storage integration test, not an assumption about interrupting a commit.

A timed-out worker retains its bounded worker permit, quota reservation, root
capture-admission guard and private staging until it actually exits and cleanup
finishes. That guard excludes another capture of the same root, not prompt
execution or other workspace mutations: after timeout this read-only worker
has no workspace mutation lock. Further captures there return `Busy` rather
than accumulating waiters. Other roots share the fixed worker pool and bounded
queue. A stuck worker can exhaust capacity; capture becomes unavailable while
sessions continue. No replacement worker or unbounded detached task is spawned
for it. Completed obsolete work is discarded under its original identity;
cleanup cannot delete a newer job's data or release another job's reservation.
No claim is made that cancellation forcibly stops regular-file I/O, or that
worker cleanup/shutdown has bounded latency. Crash before accepted durable
publication means unavailable; orphan reconciliation keeps quota accounting
until the old process is gone and its private staging can be reclaimed safely.

Each ready manifest covers raw bytes and ordinary permission bits of regular
files from hardened Git enumeration: tracked paths plus nonignored untracked
paths. Staged content is not substituted for the file on disk. Missing tracked
files are explicit `Absent` entries. The before/after manifests preserve dirty
initial state, including uncommitted files; the index, HEAD and refs are untouched.
Binary files are byte-preserved; no mandatory text diff or line statistics.
Capture bounds actual streamed bytes, not just initial metadata. Compare open
file identity/size/mtime before and after each read and re-enumerate the covered
set before publication; any detected mutation is `CaptureUnstable`. This is a
quiescent cooperative snapshot, not a filesystem-wide atomic instant; an
uncooperative writer can evade observations just as it can race restoration.

Excluded are `.git` (including its actual resolved directory), ignored files,
node data/credentials, sockets/devices/FIFOs, symlinks, hard links, nested Git
repositories, submodule contents and mount crossings. A managed worktree may
live under node data: only its registered worktree subtree is allowed, never
its ancestors or siblings. An excluded entry in the enumerated candidate set
makes the whole checkpoint unavailable with a reason, rather than silently
publishing an incomplete ready image. Ignored paths are outside the stated
coverage and are never copied or deleted. Changing ignore rules later cannot
silently expand coverage: preview compares the frozen manifest scope and
current enumeration and refuses a changed ignore-policy digest.

An archive contains project contents and can therefore contain project secrets
that were themselves tracked or nonignored. It receives the node data store's
private protection, never logs contents or exports them through the list API,
and is not synced/exported with conversation history. This feature does not
claim to detect every secret. Restoring affects all covered changes since the
point, including editor/shell changes; it does not claim agent attribution.

## Storage, bounds and retention

Use `<node-data>/checkpoints/` with random validated ids, immutable manifests
and SHA-256-addressed blobs, never a path supplied by a client. SQLite stores
session/root/anchor associations, state, quota reservations and restore results.
Create archive/staging directories with owner-only traversal and files with
owner-only read/write, independently of inherited umask. Validate existing
archive ownership/modes before use; do not broaden a credential directory.
Blobs are scoped by workspace identity, not a cross-user content oracle. Reuse
existing SHA-256, SQLite, serde, UUID and workspace I/O facilities. No new crate
or dependency edge is proposed. Reference counts and publication are node-owned.

Files are copied to private staging, hashed and synced; the manifest is
published only after referenced blobs are durable, then its ready record is
committed. A crash may leave unreferenced objects, never a ready dangling
manifest. Startup reconciles staging and reference counts before GC. Session
logs contain metadata/status only, not filesystem bytes or blob names.

Proposed defaults are deliberately bounded; these are product choices, not
Paseo defaults. Each becomes one named constant at the owning type; errors and
tests derive their bounds from it, with checked arithmetic and quota reservation.

| Constant | Proposed value | Meaning |
| --- | --- | --- |
| `CHECKPOINT_FILE_BYTES` | 16 MiB | Maximum raw bytes in one covered file |
| `CHECKPOINT_IMAGE_BYTES` | 128 MiB | Maximum sum of bytes in a manifest |
| `CHECKPOINT_PATHS` | 2,048 | Maximum entries in a manifest or restore plan |
| `CHECKPOINT_PATH_BYTES` | 1,024 | Maximum UTF-8 bytes in a relative path |
| `CHECKPOINTS_PER_SESSION` | 40 | Retained ready boundary images per session |
| `CHECKPOINT_NODE_BYTES` | 2 GiB | Total blobs, manifests, staging and journals |
| `CHECKPOINT_CAPTURE_DEADLINE` | 30 seconds | Maximum actor wait for capture acknowledgement |
| `CHECKPOINT_IO_CHUNK` | 64 KiB | Streaming buffer; not a file-sized allocation |
| `CHECKPOINT_PAGE_ENTRIES` | 100 | Maximum list/preview page entries |
| `CHECKPOINT_PREVIEWS_PER_DEVICE` | 4 | Concurrent pinned preview plans |
| `CHECKPOINT_PREVIEW_LIFETIME` | 5 minutes | Preview validity and pin duration |
| `CHECKPOINT_RESTORE_DEADLINE` | 60 seconds | Deadline before entering recovery |
| `CHECKPOINT_WORKERS` | 2 | Running workers per node, including timed-out workers awaiting cleanup |
| `CHECKPOINT_QUEUE` | 8 | Pending capture/restore jobs per node |

A full queue returns `Busy`; no unbounded sender task is spawned. Admission
reserves worst-case staging and a restore's before image before file writes.
An over-limit candidate fails entirely, naming the limit. No skipped large file
can still result in a ready checkpoint. Expiration prunes oldest unpinned
images for a session. Active previews, in-flight captures, restore journals
and recovery data are pinned. If no safe pruning fits, capture becomes
unavailable/restore refuses `StorageFull`. Session deletion removes unpinned
images; a pending recovery blocks deletion of its data. Old list cursors report
expiration explicitly. No new node settings key is required by this draft.

## Preview, confirmation and restore semantics

New additive methods, provisional names, number given at merge:

| Method | Scope | Typed request and response |
| --- | --- | --- |
| `checkpoint.list` | Read | SessionId + optional CheckpointCursor -> bounded CheckpointPage, capability/unavailable reasons |
| `checkpoint.preview` | Operate | SessionId + CheckpointId -> RestorePlanId, boundary, generation, counts, expiry and first ChangePage |
| `checkpoint.changes` | Operate | RestorePlanId + optional ChangeCursor -> remaining ChangePage |
| `checkpoint.restore` | Operate | SessionId + RestorePlanId + RestoreId -> RestoreOutcome |
| `checkpoint.restore_status` | Operate | SessionId + RestoreId -> durable RestoreOutcome |

Preview is read-only with respect to project files, but requires operate
because the resulting confirmation authorizes a destructive operation. The
node persists/pins its plan before replying. Each page is bound to the same
plan digest and expiry; it lists `RelPath` and `Create | Replace | Delete |
Permissions`, old/new byte sizes and optional binary classification. A rename
is a delete plus create, not an unreliable inferred move. No file contents,
absolute paths or agent-supplied statistics are used. The app lists all pages
in its confirmation dialog; incomplete/failed pagination disables Confirm.
The dialog states that conversation remains and later covered editor/shell
edits will also be replaced. Cancel makes no project write.

The server plan binds principal/device, session, root identity, checkpoint,
manifest and ignore digests, current covered tree digest, file identities,
permissions, effective desired image/fingerprint, change list and expiry. The
client cannot submit an arbitrary path, destination, script, force flag or
edited change list. A plan belonging
to another caller/session is forbidden. Replay cannot transplant it.

Restore reauthenticates, then serializes against all node-mediated mutations
of that workspace (including Git/file RPCs, setup, teardown, removal, capture,
new prompts, side questions, scripts and terminals). A running agent, child,
script or PTY in that root is `WorkspaceBusy`; do not stop one implicitly.
Lock ordering is workspace before checkpoint store; no global lock held over
I/O. Re-resolve the registered root and verify every current digest/identity
against preview before any project write. A mismatch is `StalePreview` and
requires a new dialog; no force overwrite. Pin the checkpoint and reserve
recovery space in the same admission step.

Create the private before image and durable journal first. Stage replacement
bytes on the destination filesystem, verify their hashes and metadata, and
keep staging outside the enumerated worktree in private node storage on the
same filesystem, then replace regular files by atomic per-file rename; deletes move the original
into a private same-filesystem recovery directory. Refuse cross-device moves.
The selected manifest is capture evidence; it is not the verification image.
Preview computes one `EffectiveDesiredImage` by applying the metadata policy to
that manifest and the validated current tree. Each `DesiredEntry` is `Absent`
or a regular file with checkpoint content digest/size and effective owner,
group and mode. Existing files keep their current owner/group and read/write
bits; only executable bits come from the checkpoint. Absent restored files
receive the node account's owner/group, private owner read/write bits and the
checkpoint executable bits. Captured read/write bits, owner/group, setuid/setgid,
ACLs, xattrs and Windows streams are deliberately not replayed. Existing unsafe
special bits/ACL/ownership or a mode that would make verification impossible
are `UnsupportedMetadata` before any write; no recursive chmod/chown or index
rewrite. All covered present files receive the same effective-mode calculation,
so an executable-only change is a real approved permission operation.

Preview persists the effective entries and their fingerprint, then derives
its complete change list from current versus effective entries. The dialog
states which permissions are retained/private rather than restored verbatim.
Admission revalidates that current tree; a change requires `StalePreview`, not
recomputing a different target under an old confirmation. Journal desired
fingerprints, staged permissions, final verification and no-op classification
all use this exact frozen effective image. Captured metadata differences that
policy intentionally does not replay cannot themselves force a write or a
failure. A current tree already matching that image yields verified `Unchanged`;
otherwise verified content or executable-bit changes yield `Restored`.

For example, checkpoint mode `0644` and a current `0600` file produce desired
mode `0600`: restore different content as `Restored`, or report `Unchanged` if
content/executable bits already match. A now-absent checkpoint `0644` file is
recreated privately at `0600` and may truthfully return `Restored`. If executable
bits differ, replay them with the retained current read/write bits and verify
that resulting mode. These examples describe Unix modes, not extra constants.
Create needed ordinary parent directories privately; never delete directories
recursively. Preexisting and unrelated empty directories are left alone and
are not part of file-image equality.

Before each path mutation, recheck containment, parent identity and regular-file
identity. Reject links/hard links, special files and unsupported names, including
links introduced since capture. Reuse root/regular-file protections and the
same-user threat boundary of ADR 0027: this is not a sandbox against an
uncooperative local process. Cooperative node locks exclude node writers;
external editors/descendants can still race between check and rename. Document
that boundary in the confirmation, and never claim atomic whole-tree restore
or universal protection from external writers. A detected race stops work.

Journal `Prepared -> Applying -> Verifying -> Complete` with each path's before
and effective desired fingerprint. Success requires a final scan equal to the
approved effective image over the entire covered scope, the durable result
committed, and a `FilesRestored` metadata event appended. Replacements/deletions
only apply to
the approved scope union; a later nonignored created file is deleted only when
it appeared in the approved plan. Ignored files stay outside the operation.
`Unchanged` is a distinct verified no-op result, not a made-up changed count.

An I/O error/cancellation after writes attempts rollback only for paths still
matching this restore's own written fingerprint; never overwrite a new external
edit while rolling back. Verified rollback returns `RolledBack`; otherwise
persist `RecoveryRequired` with RestoreId and per-path applied/pending/conflict
states, keep journal/before image, block further writes by the node to this
workspace, and offer status. Startup performs the same journal reconciliation
before admitting writers. It does not blindly replay a restore into a changed
folder. Multi-file restoration is recoverable, not an atomic transaction.

`RestoreOutcome` is an exhaustive enum: `InProgress { phase }`, `Restored {
changed: PathCount }`, `Unchanged`, `RolledBack { reason }`, or
`RecoveryRequired { reason, paths }`. Recovery path states are paginated with
the same bounded cursor rules as changes. No unbounded vector crosses the wire.

RestoreId is client-minted and durably unique within the session; bind it to
caller and plan. An identical retry returns stored/in-progress outcome without
reapplying; reuse for different arguments is `RestoreIdConflict`. Lost replies
are resolved with status, including after node restart. Result records live
until session deletion (subject to pending-recovery protection); before a new
operation, reserve their metadata inside the node quota. Unknown ids return
`RestoreNotFound`, never imply success. Preview is consumed by admission, but
same-id retries still find their durable result. No combined conversation/files
operation is added: sequential composition could partially succeed, so ADR 0052
and its conversation action remain separate.

## Typed failures and compatibility

`vessel-proto::checkpoint` owns validated CheckpointId, RestorePlanId, RestoreId,
CheckpointCursor, ChangeCursor, CaptureId, CaptureGeneration, ImageBytes,
FileBytes, PathCount, ContentDigest, EffectiveDesiredImage, DesiredEntry,
WorkspaceGeneration, CheckpointBoundary, CheckpointState, ChangeKind and
RestoreOutcome. Reuse SessionId, EventSeq, RelPath and permission/scopes types.
No String/boolean/numeric sentinel carries a closed kind or domain value.
`Ready`, `Unavailable { reason }` and `Expired` are explicit; no retroactive
capture is labeled as an old turn. Wire decoders reject malformed ids/bounds.

Extend `ErrorDetail` additively with `Checkpoint { reason: CheckpointError }`;
map crate `thiserror` sources once, with redacted display. The reason enum has
a decoding-edge `Unknown` for newer peers, not string branching in the app.

| Existing ErrorCode | Proposed typed reasons |
| --- | --- |
| Forbidden | NotOwner, NotDevice, PlanOwnerMismatch, ProtectedRoot |
| NotFound | SessionNotFound, CheckpointNotFound, RestoreNotFound |
| Unsupported | UnsupportedScope, UnsupportedPlatform, UnsafeEntry, UnsupportedMetadata, NoHistoricalCheckpoint |
| Conflict | WorkspaceBusy, Busy, RootChanged, IgnorePolicyChanged, CaptureUnstable, CheckpointExpired, PreviewExpired, StalePreview, RestoreIdConflict, RecoveryPending |
| InvalidRequest | InvalidAnchor, InvalidCursor, InvalidPath, LimitExceeded |
| Unavailable | CaptureTimedOut, StorageFull, ShuttingDown |
| Internal | StorageIo, CorruptCheckpoint |

A failure after applying begins is a durable `RestoreOutcome::RolledBack` or
`RecoveryRequired`, not an ordinary error implying nothing changed. Source
errors stay internal and never expose file contents, credential paths or
absolute node paths. Errors carry typed limits/counts where needed. Initial
Unix implementation must test permission behavior under a non-root account.
Windows returns `UnsupportedPlatform` until equivalent link, metadata and
journal semantics are implemented/tested; no pretend success or global setting
change. No signals, systemd, launchd, helper server or external credentials.

Use the shared unassigned-feature mechanism:
`FILE_CHECKPOINTS: Option<ProtocolVersion> = None` until the integrator assigns
it. New calls' `required_protocol()` refuses locally and at dispatch while
unassigned or when the peer is older. Leave `PROTOCOL_VERSION` unchanged on
this branch. New metadata events downgrade per peer to a harmless existing log
record; new fields are absent on older direct, nested/list and forwarded paths.
Old clients retain conversation rewind and removed-worktree recovery unchanged.
New clients against old nodes show files checkpoints unsupported, never fall
back to `session.rewind`. Checkpoint storage is not included in session
export/import or fleet sync; moved/imported turns have no local file image.

Only an authenticated human device with scope and session visibility may
preview/restore. Delegated agent principals are refused even when they can
operate a session, preventing self-confirmed destructive restore. Read/list
uses the existing session visibility rules and carries metadata only. Every
restore/status lookup rechecks ownership after an id lookup. Node credentials
never enter agent environments or archive manifests.

## Structure and data flow

| Owner | Proposed responsibility, no new crate edge |
| --- | --- |
| `vessel-proto/src/checkpoint/` | Validated DTOs, errors, protocol gate, downgraded event metadata |
| `vessel-workspace/src/checkpoint/` | Candidate enumeration, bounded immutable image I/O, safe path operations, fingerprints and restore steps; no sessions/store/auth knowledge |
| `vessel-store/src/checkpoint/` | Associations, quota reservations, durable plans/journals/results, retention and schema migration |
| `vessel-node/src/checkpoint/` | Bounded coordinator, auth/ownership, workspace exclusion, publication/recovery; delegates filesystem work off async workers |
| `vessel-session` | Typed turn-boundary handshake only; node owns checkpoint policy and I/O |
| native `view/link/app` | Pure state, typed calls, existing kit dialog/status rendering, respectively; later work |

Flow: accepted prompt -> node capture -> agent turn -> persisted TurnEnded ->
node capture -> available metadata. Human selects boundary -> preview pages ->
confirmation -> durable restore -> verify -> outcome/event -> status in app.
Schema number and migration are assigned on the central node line. These are
proposed modules only: the architecture table and ADR 0001 are updated when
implementation adds APIs/ownership, not falsely documented as implemented now.

## Why this choice; differences and consequences

Files-only behavior follows Paseo and the SDK. Provider-independent manifests
keep ACP as the only agent transport and cover raw in-scope shell edits too.
After-turn points additionally make “restore files to here” unambiguous; the
before-turn point preserves Paseo's user-message meaning. A confirmation with
actual paths, non-overridable stale checks and a durable status are deliberate
additions. Ready checkpoints refuse unsafe entries instead of adopting the
SDK's skip behavior. No implication of full Paseo parity is made: initial
scope/platform support is narrower, combined mode is absent, quotas may make
a point unavailable, and external filesystem writers remain a stated boundary.

The costs are a bounded actor wait at capture boundaries, potentially stalled
but admission-bounded I/O workers, private historical copies,
extra disk reservations for safe recovery, and a per-file journal rather than
instant atomic tree replacement. Retention can expire old points. File restore
can make conversation assumptions stale; the UI states this and the agent is
not silently rewound or prompted. No provider binary, browser or network is
needed for checkpoint operation.

## Required evidence before enabling the capability

- Fakes prove before/after timing, queued/steered distinctions, missing ACP
  ids, aborted turns and crashes never publishing a wrong ready boundary.
  Hold read/metadata/fsync and publication with barriers, advance an injected
  clock past the capture deadline and observe prompt delivery before releasing
  I/O. Late completion must leave that exact boundary unavailable, never publish
  into a later generation, and clean up only its own staging. Saturate all
  retained worker permits/reservations, issue more prompts and prove bounded
  jobs, queue, memory and reserved bytes; new captures are unavailable/Busy,
  prompts continue, and no capacity is returned before actual cleanup.
- Dirty tracked and untracked bytes, deletions, binary/mode changes, files
  created later, changed ignore policy and ignored outputs preserve the
  promised scope; HEAD/index/refs and conversation/draft stay unchanged.
- Boundaries and queue/quota admission fail without partial ready manifests;
  pinned previews and recovery objects survive retention and session deletion.
- Preview pagination, owner/device mismatch, forged paths/ids, stale content,
  replaced roots, symlinks/hard links/FIFOs, parent swaps, nested repositories,
  non-UTF-8 paths and unsupported permissions fail as typed outcomes.
- A non-root account proves existing owner/group/read-write bits remain valid,
  restored/new files are readable, private archives are not world-readable,
  and later captures/restarts preserve those properties. Cover checkpoint
  `0644` versus current `0600` with both changed and equal content, executable
  bit changes with retained read/write bits, and an absent file recreated
  privately. Assert preview effective mode, journal target, exact final mode
  and `Restored`/`Unchanged` together; never relax modes to make raw-manifest
  equality pass, or return RolledBack solely for an intentionally unreplayed bit.
- Barriers/fault injection cover each journal stage, ENOSPC, cancellation,
  reconnect/restart, duplicate RestoreId, concurrent node writers and external
  edit conflicts. Recovery never deletes the last before image or claims all
  files restored when verification failed; no sleeps or real agents in tests.
- Protocol tests cover unassigned/old/new clients, direct/list/nested/forwarded
  events and errors, generated TypeScript, and unsupported Windows behavior.
  Native tests/captures follow only after node review and the central baseline.

## Resume point

Draft only. No storage schema, RPC registration, checkpoint filesystem or UI
exists from this ADR. The first review returned two defects; the revised
contract freezes an effective metadata-policy target and separates actor
capture timeout from retained I/O-worker cleanup. Required evidence above is
still future work, including the permission and held-I/O cases. Next: revised
contract review, then central implementation/baseline/schema/protocol assignment.
Product code remains held.
