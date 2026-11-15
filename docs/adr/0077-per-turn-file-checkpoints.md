# ADR 0077: File checkpoints at turn boundaries, independently of conversation rewind

Status: accepted as a design contract, 2026-11-15, on `a852bde3`.
No implementation, protocol number, dependency or call registration is
approved by this document. Extends ADRs 0027 and 0052 without changing
their existing calls. Implementation awaits central ownership assignment
and the node/app baseline merge. No runtime setting or dependency changes
are made by this document.

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
| `CHECKPOINT_PREVIEWS_PER_OWNER` | 4 | Concurrent pinned preview plans per complete human principal |
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

Completed private stage/preview payloads may retire only after actual worker
settlement, verified durable outcome and the required result-event marker. Their
result/plan/idempotency metadata remains conservatively charged until session
retention, including after the selected image expires; an expired image cannot
authorize a new restore. In-flight and recovery journals retain their full pin.
A lost completion acknowledgement permits metadata-only retry under the exact
original canonical recovery lease; it never permits reapplying files or clearing
another operation's root pin. This clarification accompanies the implementation
for node contract review; it is not runtime acceptance.


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

An ended target does not need a process or resumed conversation to restore files.
The existing store writer appends `FilesRestored` as storage-only metadata outside
any run, preserving status, ended_at, conversation identity and the complete queue.
A final Status remains the last runtime-produced lifecycle record of a run;
checkpoint metadata may follow an ended run (shared wording owned by lifecycle,
from de2908cd's comment hunk). Live targets publish through the owned actor pause.
Both append paths validate the durable Restored outcome, selected CheckpointId and
changed-path count, and record the event's EventSeq on the restore row in the same
transaction as the event. A retry/restart reads that marker before publishing;
it cannot append another event for the same RestoreId. An ended append checks the
stored live flag in the writer transaction and refuses if a run has resumed. A
verified Unchanged, rollback or recovery-required outcome cannot emit FilesRestored.
The log append carrying checkpoint metadata uses FULL synchronous durability,
including the live actor path, before acknowledging the metadata commit.


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
| Forbidden | NotOwner, NotHuman, PlanOwnerMismatch, ProtectedRoot |
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

Only an authenticated human principal (owner, device or delegated human)
with scope and session visibility may
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

Contract accepted, with node implementation subsequently assigned on `d954fa3a`.
The branch now contains validated value types and test-only capture/storage/
actor seams, plus one appended checkpoint table migration. This does not
advertise a file restore capability: pending unary calls are registered after Quality B, FILE_CHECKPOINTS
is None and actual session wiring/restore execution are not complete. The plan
records narrow-check failures and remaining proof. No whole package gate,
permission, held-I/O, publication-race or crash-recovery pass is implied here.
Final schema/protocol integration belongs to the integrator; the app waits for
the centrally supplied app base.

### Staged filesystem implementation note

The first test-only filesystem implementation also reuses the already-resolved
`rustix` safe wrappers used by store attachments, through a workspace dev
dependency with its existing fs feature (MIT/Apache-2.0; no new resolved crate
or version). Existing `nix` regular-file checks do not expose the handle-based
xattr enumeration needed to refuse ACL metadata before replacing files. Only
Linux is staged initially; other platform implementations remain unavailable
until equivalent metadata/containment tests exist. No release dependency or
advertised platform capability is enabled by this staged addition.

### Candidate acknowledgement and Ready activation

The staged runner explicitly separates durable candidate persistence from Ready
activation. The coordinator's in-time acceptance can acknowledge the actor
without waiting for activation I/O. Only that accepted path invokes activation;
Ready remains a durable store state, never inferred from the internal actor
acknowledgement. Expired/restarted rows fail the exact-key conditional update.
An uncertain activation retains its archive for reconciliation because a lost
storage acknowledgement cannot prove that Ready was not committed. The bounded
worker/root reservation lasts through activation and actual cleanup. Requests
carry the original deadline across the bridge, and the caller also bounds its
wait including queue time. A dropped actor reply cannot authorize activation.
The runtime adapter therefore retains a separate advancement receipt after
candidate acknowledgement: only actual PromptSent or TurnComplete permits
activation. PromptRefused or a lost receipt discards the candidate. The
bridge's successful internal reply is never substituted for actor acceptance.
These are staged mechanisms and tests, not proof of real runtime integration.

### Human-principal ownership amendment

A local token/socket owner is a permitted human client. Persist CheckpointOwner
as LocalOwner, Device, or DelegatedHuman { delivering device, origin node,
Owner | Device }. Direct and delegated agents are refused; clients cannot
supply this identity in request parameters. Binding only the delivering
DeviceId would let different humans or origin nodes through one home share
confirmation plans and restore status. The complete authenticated identity
therefore binds both plan and operation, including every pagination/status
lookup and identical-id retry. Per-owner preview admission uses that same
identity. Scopes and revocation remain the existing per-call admission checks.
This wording replaces the earlier ambiguous human-device wording; the package
code review also reviews this amendment and its cross-origin/local-owner tests.

### Staged garbage and owner lifetime

Retired captures transfer their reservation into durable garbage independent
of the deleted session's foreign key. The existing writer's deletion savepoint
performs that transfer with metadata deletion; cleanup acknowledgement releases
capacity only after actual worker handles and private files are gone. Startup
reconciliation and the real filesystem collector remain implementation work.
Closing the last capture client drains admitted workers before the owner drops
their root guards or reservations. Caller disappearance is not I/O completion.
Linux staging also checks FD mount IDs, since device identity alone cannot
distinguish same-filesystem bind mounts; missing mount identity refuses support.

### Journal durability through the shared writer

The event log normally uses WAL with `synchronous=NORMAL`; a normal commit
acknowledgement alone is insufficient before a durable filesystem rename.
Checkpoint writes remain on the existing serialized writer. Before opening a
batch transaction containing checkpoint work, that writer temporarily selects
`FULL` synchronous mode, commits all path intent in the same transaction and
restores the connection's previous mode before acknowledging callers. It never
changes synchronous mode inside a transaction. Ordinary event-only batches keep
the existing cost and policy. A failed sync-policy admission applies no work and
answers every queued caller with the source-preserving commit error. A commit
or policy-restoration error refuses an affirmative acknowledgement; ambiguous
commits are resolved by the existing operation identity/status, never by replay.
Tests inspect the actual connection policy during commit and after rollback.
This proves policy selection, not power-cut behavior of an arbitrary device.

### Physical-root admission and lifecycle ordering

Logical WorkspaceId and worktree identity never replace physical exclusion.
The filesystem derives a separate PhysicalRoot from the opened directory's
filesystem device and inode; access-mode or logical-membership changes do not
mint another lock. Aliases of that same directory share admission. A single
bounded node owner grants opaque RAII RootLease values. A dropped or undelivered
lease cancels its token and notifies the owner directly, so releasing a lease
cannot fail because a bounded request queue is full. The owner prunes cancelled
leases before admission, including an immediate next request.

Archive/restore lifecycle ordering is durable workspace state claim, then
physical-root admission, then physical mutations and postcondition verification,
then lease release and lifecycle settlement. No caller holds a root lease while
waiting for a workspace lifecycle claim. A writer retains its lease while I/O
is in flight even when its caller deadline expires. A read-only capture can
release this common lease at deadline only after its publication lease has been
invalidated; it still retains its separate bounded worker, root capture guard,
reservation and staging until actual cleanup. Startup restores every unresolved
physical root into RecoveryPending admission before starting agents or writers.
Only verified physical rollback plus durable journal resolution clears that pin.
The in-memory gate alone is not restart reconciliation or proof that all callers
have been wired through it.

Recovery resolution is a separate journal command. It requires an observed whole
before image with the same physical root/access/ignore policy and original
object identity for every path; equal bytes at another inode do not resolve a
conflict. Only then may the shared FULL-sync writer atomically mark the path
journal restored and commit RolledBack. Generic terminal updates cannot promote
RecoveryRequired to Restored. A repeat of the same recorded rollback result is
idempotent; a different terminal decision is refused. The root lease stays held
through this acknowledgement, before unpinning root admission or garbage.
Capture generations are also bounded at the exact JavaScript integer ceiling,
which fits SQLite's signed INTEGER and prevents a protocol fence being rounded.
Exhaustion refuses a new generation rather than reusing or wrapping a lease.

### Normal file writers consume the same root authority

[Decision Log]
- Purpose: a restore lease must exclude ordinary node file mutations too.
- Existing implementation and constraint: file helpers already use blocking I/O;
  an RPC future can be dropped while that I/O still runs. Holding a root lease
  only in that RPC future would release exclusion too early.
- Alternatives: a lease scoped to the RPC await; a global filesystem mutex; a
  new worker framework. The first loses authority on disconnect, the second
  serializes unrelated roots and risks actor stalls, the third duplicates tasks.
- Choice: reuse the node TaskTracker to own admitted file operations and their
  lease through actual I/O and root postcondition verification. The caller waits
  on a separate response channel. Losing that response never aborts the writer.
- Why: this gives existing file APIs the same physical-root authority while
  keeping unrelated roots independent and preserving the blocking I/O boundary.
- Effects: Linux file write/create-directory/rename/delete consume the common
  gate. The owner acknowledgement has a named five-second deadline; Busy refuses
  before mutation. Non-Linux ordinary file behavior is retained while checkpoint
  restoration remains unavailable. A retained directory FD pins the inode and
  detects root-name replacement. Node shutdown closes new admission and drains
  admitted leases. Git, agent, terminal, script and lifecycle writers still need
  connecting before checkpoint restore can be activated; this slice alone does
  not establish root-wide exclusion.

Git mutations use the same retained operation owner and root lease, including
index/discard, commit, fetch/push/pull, switch, branch rename and stash. A typed
repository requirement is checked before admission. Cooperative Git cancellation
is scoped to the retained clone: disconnect or node shutdown requests the
existing subprocess-group stop/reap path, while the owner still awaits the
operation before releasing admission. Regular blocking file I/O may ignore that
cancellation and retains its lease until completion. The original Git handle
and unrelated requests are not cancelled by one scoped clone. Read-only Git
status/diff/history remain outside writer admission.

### Start admission and metadata-worker lifetime

A nested CWD and a parent-directory alias must use the registered managed root's
physical admission identity. The filesystem layer therefore walks opened parent
FDs, with a named ancestry bound, before selecting a known managed root. This
uses the already-resolved safe OwnedFd API; no dependency or unsafe code is added.
Root metadata discovery has two worker permits, retained inside the blocking
closure until it actually finishes. Losing an RPC does not admit replacement
workers while old kernel I/O remains blocked.

Shell startup is a retained node task through registration, agent watches and
placement validation. Its root guard and existing worktree starting reservation
survive caller cancellation until those steps settle. This connection alone does
not fence script restarts, every session/agent launch or restore; those call sites
and actual actor quiescence remain required before checkpoint activation. The
terminal-start slice has not yet received compiled/runtime verification.

Session-start ownership refinement: use the existing node-retained creation and
resume tasks to own physical admission and worktree start counts together.
Dropping the requesting RPC must not drop either guard before the process is
registered or, on persistence failure, actually stopped. Revalidation shares
the bounded metadata worker pool; it does not start an unbounded second pool.
The ordinary process owner remains the runtime, with its existing group cleanup.
Automatic script restarts, actor wake/move and lifecycle cleanup remain separate
required consumers before checkpoint restoration can be enabled.

Script/service admission belongs to each process attempt, including automatic
retries. Holding admission only around the RPC's initial registry insertion
would miss later spawns. The session crate therefore receives a generic async
admission callback and does not learn node/workspace types. The node withholds
a registration receipt until registry insertion and placement verification;
only then may the callback claim the physical root. Approval is checked again
following admission. The process is published before releasing its hold, so
restore must then refuse the live terminal rather than depend on a lifetime
observation or PID. This adds no dependency edge or wire capability.

Actor pause refinement: retain a non-cloneable RAII receipt in the restore owner.
The actor admits it only while idle/ready with no waiting answer, switches its
route to a typed files-paused state, refuses all new controls, and prevents queue
advance and automatic idle parking. Its prior route and existing queue hold are
preserved verbatim. Release is signalled by drop, with an explicit acknowledged
resume for controllers; an undeliverable receipt releases itself. Lifecycle kill
remains available. Physical exclusion must be acquired before actor admission
and released only after the filesystem/durable postcondition, before actor
resume permits a queued prompt to acquire that root. A recovery-required owner
must retain both holds until verified reconciliation; RPC cancellation alone
must never drop ownership of in-flight filesystem work.

Lifecycle ownership refinement: setup acquires physical admission inside its
retained task, not around the initiating RPC. Removal/retire acquires once after
the existing logical worktree claim, retains it through teardown and actual Git
removal, and checks the final filesystem absence before release. Teardown
borrows that same admission; it must not acquire the root a second time. Setup
and teardown validate the admitted root before and after command completion.
Busy admission unwinds the logical claim. An already missing folder permits
metadata-only retirement and performs no physical mutation. These connections
do not replace physical live-occupant checks or durable workspace lifecycle
claims; those remain required before checkpoint controller activation.

Durable anchor refinement: a real new prompt receives its own typed
`CheckpointEvent::PromptAccepted` log record before capture. Its durable sequence
is the immutable BeforeTurn anchor. Once the provider's TurnStarted is actually
published, a typed TurnAssociated record binds those two positions. This avoids
predicting sequence numbers while agent outputs can arrive and avoids duplicating
provider prompt events. FilesRestored carries only operation/checkpoint identities
and a bounded path count after durable verification. This closed SessionEvent
family uses FILE_CHECKPOINTS (None until assignment), downgrades for all existing
peers, and does not affect conversation projection, attention or cost accounting.

Actor producer refinement: real new delivery uses the bridge after ordered driver
readiness/idle observations; steers injected into a running turn do not create a
boundary. A completed TurnEnded pumped during delivery is settled before another
prompt, not after its files may have changed. TurnFailed does not invent an
AfterTurn anchor. Accepted-anchor channel admission and node acknowledgement
share the original capture deadline; a full durable event channel can leave
capture unavailable rather than hold delivery for an unbounded capture wait.
The node must still await actual SQL durability of accepted/ended anchors before
filesystem capture. The session bridge alone does not establish that ordering.

Private-work quota refinement: preview/restore allocations are durable, bound to
the immutable capture association and full human principal, and reserve a fixed
conservative envelope before filesystem work. The global equation includes those
allocations alongside captures and garbage. Unfinished private workers pin
retirement; attachment requires a published plan or terminal/recovery journal
and the actual worker's completion. Unpublished cleanup keeps a positively
accounted id tombstone to prevent delayed callbacks releasing a recycled object.
Parent retirement transfers private charges into durable garbage until physical
cleanup acknowledgement. SQL cannot prove kernel I/O completion: the node must
retain the worker/object ownership through it. Those controller/GC adapters are
not yet connected. This adds a table within the one unshipped checkpoint migration,
not another schema number; the migration runner stays unchanged.

Capture settlement refinement: production reservation has a fixed named escrow;
only unit accounting models accept arbitrary byte budgets. After the matching
worker's actual cleanup, Ready remains charged for its payload and conservative
entry/manifest allowances; Unavailable retains metadata. Candidate cannot settle,
and a computed charge larger than its escrow refuses rather than growing or
returning unreserved capacity. Metadata length is bounded before row decoding.
This accounts checkpoint reservations, not the total physical database/WAL size.
Node callbacks still must prove actual completion before invoking settlement.


### Implementation boundary update: Linux worker API

The store boundary now compiles on the production writer and the Linux workspace
mechanisms expose an admitted synchronous worker API. The existing rustix safe
fs/process wrappers are a Linux normal dependency, not a new version or provider
SDK. This implementation decision belongs to the package and does not derive
dependency approval from this ADR's design acceptance. The node still must own
physical admission, bounded workers, auth, durable staged evidence, cleanup and
startup recovery; FILE_CHECKPOINTS remains None. Reconstructed recovery may undo
verified written objects but cannot resume forward file application. These source
changes await compilation, runtime proof and the package's V3 review.
