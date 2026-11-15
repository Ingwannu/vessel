# Per-turn file checkpoints: implementation plan

ADR 0077 passed design review at `a852bde3`; acceptance is a contract decision,
not a product/runtime pass. The node branch consumes the reviewed integration
head `ede20014` (Quality B and gate 44); pending rows live at the end of
`rpc/table.rs`. The separate app branch consumes package base 6 `18eedad4`
and fixed-lock update `dd13ce37`; no app feature is implemented yet.

## References and existing assets

The exact reference/source mapping and stated differences live in
[ADR 0077](../adr/0077-per-turn-file-checkpoints.md). Paseo's files mode and
Claude SDK rewindFiles define files-only behavior; GOAT supplies turn-linked
changed-files presentation. Existing node snapshots recover a removed worktree
into a fresh one. Existing session.rewind changes conversation and leaves files.
Neither is a fallback for this feature.

Decision: extend session/workspace infrastructure with focused checkpoint
modules. Reuse RelPath, Git probing, regular-file defenses, bounded channels,
existing SHA-256 and SQLite. Strict enumeration refuses invalid paths rather
than reusing an existing lossy listing. Existing rustix supplies Linux fd-based
xattr refusal and rename exchange; the dependency stays dev-only during A10
staging. No provider SDK, tool-call-count inference or new crate edge.

## Current implementation and boundaries

- proto/checkpoint: validating images/metadata/limits/ids, effective permission
  policy, typed changes and errors, unary requests/responses. FILE_CHECKPOINTS
  remains None and PROTOCOL_VERSION is unchanged. The five canonical pending
  rows generate client/guard/envelope plumbing. No independent method table.
- node/checkpoint: test-only bounded capture owner, worker/root/quota leases and
  deadline fencing. Timeout acknowledges without joining blocked I/O; capacity
  remains occupied until actual cleanup. Candidate persistence and durable Ready
  activation are separate phases; uncertain activation retains the archive.
  The real store/coordinator assembly still needs connecting. Real actor
  prompt/TurnEnded hooks exist, but the node does not install their bridge yet.
- node/service/checkpoints: registered methods explicitly refuse as unavailable.
  Guarded authorization and agent refusal remain in front of the seam. Ordinary
  human forwarding preserves destination checks. Pending calls refuse locally
  and at dispatch; no feature capability is advertised.
- store/checkpoint: typed operations on the production writer and one schema append
  after this base's title-origin schema. The migration runner is unchanged;
  the integrator adjusts its one schema constant in merge order. Restore
  admission reloads the capture's full key/session and frozen effective target.
  Durable per-path intent and compare-and-advance phases constrain terminal
  outcomes; human-principal-bound change pages share the frozen target/expiry, enforce
  bounded offsets and refuse expired or consumed previews; admission enforces
  the five-minute lifetime and four active previews per human principal; restart pins interrupted operations for recovery. A test-only adapter uses the existing writer, savepoint and durable reply;
  restore admission and all Pending path intents share that savepoint. Its
  real-writer commit/restart/failure-injection tests remain unverified. A durable garbage table transfers retired capture reservations out of session
  FKs; cleanup acknowledgement releases them, and active previews/recovery block
  deletion. This is staged under the existing writer delete savepoint; retention
  and the real filesystem garbage collector still need connecting. Public node
  assembly and full physical quota reconciliation remain owed.
- workspace/checkpoint: test-only strict enumeration, rooted nofollow reads,
  private immutable raw blobs, bounded streaming and canonical fingerprints.
  Files are fully staged before per-path mutation; rename exchange retains
  original inodes, no-replace creates avoid clobbering, and guarded rollback
  retains conflicting external edits. Apply and undo also compare inode identity,
  refusing an equal-content external replacement. Unsupported metadata on roots
  and intermediate directories refuses before file mutation. The node must
  durably journal each step
  and verify the entire covered scope before claiming success. Linux is staged;
  FD mount IDs also reject mount crossings on the same device; actual private
  fixture and missing-capability tests are written but not yet verified. Other
  platforms remain unavailable. Full enumeration/ignore/root revalidation
  at operation boundaries and crash-restart assembly remain owed.
- session/checkpoint: test-only bounded acknowledgement prototype. Actual queue,
  steer, prompt-delivery and persisted-turn-end integration remains owed.

## Validation and remaining work

Five-crate all-target clippy passed on `fa83036e`; checkpoint-filtered behavior
tests passed on `79b3171b` (27 passed, zero failed, six ignored). These precede
new filesystem restore and pending RPC registration and do not verify them.
New tests cover effective permissions, guarded restore/undo, non-root owner,
execute-only metadata, absent recreation, later-created deletion, path-journal
phases and forged session/worktree/boundary/generation admission. They remain
unverified until their exact compiled run completes. No package gate, code
review, permission/platform/crash recovery or native capture pass is claimed.

Next: compile the new mechanisms, regenerate TypeScript, run real migration
rollback tests, finish writer/coordinator assembly and durable Ready reconciliation,
then authenticated list/preview assembly, physical quota/retention and real session/
workspace exclusion. Replace ignored obligations only after implementation.
After node completion: one whole node gate and V3 review; after consuming the
reviewed node contract, native view/link/words/screen, complete confirmation,
result/status flow, captures, one native gate and V4 review. Reuse kit controls;
shared app/link roots receive only designated registration lines.

The ownership amendment to ADR 0077 persists the complete human principal,
including delegated origin; a delivery DeviceId alone cannot authorize a
plan. Local token/socket owners remain supported; direct/delegated agents
are refused. Cross-origin sharing and local-owner retry tests accompany the
amendment in the node package review.

Recent scoped evidence: on `9a9c3882`, checkpoint-filtered tests passed 51 with
zero failures and six ignored obligations. This includes real-writer savepoint
rollback/restart, complete human principal isolation, inode checks and global
reservation timeout fencing. It precedes owner shutdown draining, durable
garbage and mount-ID changes. Protocol/web TypeScript declarations regenerated;
the broader alias check found the baseline LimitPercent alias-of-alias
classification and its one-line correction is isolated. Generated wire files
retain the canonical generator's formatting; manual Rust sources pass whitespace
checks independently. New node/app baseline integration follows the coordinator's
announced `6c782cb6`, including the exact security e2e race fix `c5da5305`.

Resume: merged the reviewed integration head and retained all five pending rows
in its extracted canonical table. The generated handler receiver is retained
by the unavailable service seam until real orchestration is ready. Latest
scoped clippy exposed only that unused receiver; its correction is not yet
verified. The separate TypeScript registry check reached its short-lane limit
while compiling; it is not a passing result. Capture evidence, physical quota,
production assembly and recovery remain the next implementation work.

Physical observations now have validating persistent types distinct from content
and desired metadata. A preview stores root, ignore-policy and per-path full/stat
and stable object fingerprints; admission compares that exact evidence. The
filesystem capture samples observations around its content reads. Equal-byte
inode replacement therefore changes evidence without changing the content image.
Ignore-policy discovery and crash-journal staging identities still need wiring;
the supplied policy digest is not yet evidence of automatic discovery.

The staged shared-writer adapter now selects FULL WAL synchronization for each
batch containing checkpoint work, outside its transaction, then restores the
prior policy. This closes the NORMAL-commit versus durable-file-rename ordering
gap without changing ordinary event batches. Tests inspect real connection
policy and rollback/reopen behavior; they are not power-loss hardware proof.

Metadata listing uses a bounded timestamp/capture-id keyset, with cursor lookup
bound to the requested session before reading any page. Tied timestamps remain
deterministic; unavailable boundaries retain their typed reason. A running
candidate reports temporary Busy rather than implying a ready image. The store
never reads blobs to render marks; the node must still enforce session visibility.
W11 confirms its start/archive lease is not a file-writer lock; lifecycle
exclusion is being aligned with the archive owner before actual restore wiring.

The canonical root admission owner is workspace::exclusion, keyed by the
FD-derived PhysicalRoot independent of WorkspaceId and path aliases. RAII token
release has a direct notification, not a fallible bounded release message.
Queue/deadline/lost-reply/drain/recovery-pin tests accompany it. The lifecycle
owner will consume this seam for actual backing mutations after the producer is
green and reviewed; no separate cwd or workspace-id lock is introduced. Shared
writer and actor call-site wiring remain owed; the gate alone is not exclusion.

Scoped validation: the eight changed root crates' all-target clippy (including
proto TS derivation) passed on `c580d9eb`. This includes physical-observation
capture and the rebased pending handler seam, but precedes journal FULL-sync,
metadata pagination and root admission. The new later behavior tests remain
queued; no whole gate or production activation is claimed.

A complete capture scope now discovers local ancestor ignore files, the actual
Git administrative exclude file, explicit core.excludesFile and an edge-supplied
HOME/XDG default. It hashes bounded policy bytes without archiving external
policy inputs, then re-enumerates paths and re-discovers/re-hashes policy before
acceptance. Git subprocess cancellation uses the same capture cancellation token.
The synchronous assembly runs on the already admitted blocking worker and uses
an explicit runtime handle only for Git I/O; it creates no extra filesystem
worker. New isolated HOME/XDG fixtures and a fake Git wrapper exercise policy
changes and a file appearing only at the second enumeration. They remain
unverified until the next scoped run. The binary default-path adapter is owed.

On `dad520f2`, checkpoint-filtered tests passed 66, zero failed, six ignored;
this includes real FULL-sync policy admission/rollback, garbage reservations,
non-root permissions and full observation association. It precedes metadata
listing/root admission/full-scope policy discovery and is not product proof.

Capture publication now carries a validated ObservedSnapshot end to end rather
than dropping its physical and policy evidence after reading bytes. Ready rows
require both image and evidence. Preview persistence additionally requires the
selected capture and current preview to have the same PhysicalRoot and ignore
policy digest; current root/file access observations still freeze separately
for stale admission. These new storage and runner changes need scoped proof.

RecoveryRequired is no longer a permanently unresolvable metadata pin. A separate
recovery writer command may commit RolledBack only after a whole observed before
image matches content/metadata, root/access/ignore policy and every original
object identity. It cannot report Restored or accept an equal-content replacement
inode. Resolution is idempotent for the same durable rollback decision; different
terminal decisions refuse. The caller must hold root exclusion through actual
verification and the FULL-sync acknowledgement. Startup filesystem replay and
post-resolution physical garbage collection still need production assembly.

The root-admission scoped clippy on `a8a3e67b` found four local lint defects
(unused qualification, identical writer error arms and two unused results).
They are corrected without weakening checks; the corrected head still needs
its compiled run. Its written behavior cases are not passing evidence yet.

First production writer connection: Linux file write/create-directory/rename/
delete now use the common root gate and retained TaskTracker operation. An FD
pins root identity; normal file APIs are reused. Caller cancellation does not
cancel admitted blocking work or release its lease. Root startup/shutdown is
connected to Workspaces and the node stop token. Recovery clearing remains a
staged operation until real checkpoint startup/replay wiring; no unresolved
checkpoint objects are created by the unavailable product service. Remaining
Git/agent/terminal/script/lifecycle writers are not yet fenced. These changes
have written actual Workspace fixtures but have not yet passed compilation.

On `fd884246`, checkpoint-filtered tests passed 79, zero failed, six ignored.
This verifies the staged root admission cases and full Git/ignore capture scope;
it precedes capture-evidence persistence, recovery resolution and production file
writer connection. The latest eight-crate clippy is queued on `3fb22a1f`.

The retained writer facade is now run_write with a typed directory/repository
requirement. All eight Git mutators consume it, preserving branch metadata
updates. Its operation cancellation token follows caller disconnect and node
shutdown; a scoped Git clone passes that token into the existing command group
stop/reap path. The owner awaits actual completion before lease release, including
noncancellable file I/O. Written regression fixtures cover denied real file/Git
mutations, cancellation isolation in a saturated Git pool and retained response
loss. No process-reaping runtime proof or whole gate is claimed yet.

The scoped clippy on `3fb22a1f` stopped on the generation ceiling's rustdoc
`SQLite` spelling; it did not reach proof of the later writer connection.
That lint is corrected without an allowance. Cooperative cancellation now also
refuses a not-yet-started write after preflight; already running blocking I/O
still drains under its lease. The latest product writer slice needs its own
compiled result before being offered to other consumers.

The `9da7666c` all-target clippy reached the new production writers and found
local lint issues in staged code (an unnecessary qualification, two unchecked
set insert results, one missing unit semicolon and the unexercised Recovered
writer command). The latter now has a real shared-writer recovery/retry/reopen
fixture instead of a dead-code allowance. These corrections need their next
scoped compiled run. `65b9fcec` checkpoint-filtered tests passed 87 with zero
failures and six ignored obligations, including actual file denial and retained
I/O after caller cancellation; this precedes the Git writer connection.

## Resume point: terminal-start admission slice

Terminal creation now acquires the shared physical-root admission and retains it
in the existing task tracker through shell startup, synchronous registry entry,
agent watches and placement validation. FD parent ancestry maps a nested or
parent-alias CWD to a registered managed worktree root rather than treating that
subdirectory as independent. New root metadata probes use two worker permits;
the blocking closure retains its permit after caller cancellation. These changes
are connected source, not compiled or runtime-verified evidence. The unused
shell-opening import and redundant platform guard were corrected in this slice.

Latest verified behavior remains `65b9fcec`: 87 checkpoint-filtered tests passed,
zero failed, six ignored obligations. The separate run on `fe0c868b` is already
in progress and must be collected without resubmission. It precedes this terminal
slice. The latest Git clippy result was a failure; its reported local corrections
are committed, but no passing clippy result for them is claimed.

Scripts and services (including automatic restart), session create/resume/wake/
move start admission, actual per-turn actor hooks, pause/queue behavior and
restore/controller assembly remain incomplete. Physical quota/garbage/retention,
startup reconciliation and native checkpoint marks/confirmation/result UI also
remain owed. No whole gate or code review has run. FILE_CHECKPOINTS remains None;
the full checkpoint feature is not complete and this producer is not ready for
consumer adoption. Continue from the retained source and collect the existing
run before scheduling another meaningful scoped suite.

### Retained session starts

Session creation (including import/fork through creation) and ended-session
resume now move both the physical `StartGuard` and the worktree `Starting`
count into the existing retained session task. Startup revalidates the opened
folder before launching. A store failure drains the failed process before
releasing admission; successful startup releases only after runtime/durability
registration. The default harness home is resolved through the existing profile
configuration, and the selected canonical folder becomes the launch folder.
Root revalidation uses the same bounded metadata pool as initial admission.
This reuses existing task/process ownership rather than introducing a second
process owner. Automatic script restarts and live actor wake/move paths still
need their own admission; this slice does not establish complete root exclusion.

The preceding fe0c868b remote suite completed with 92 passed, 0 failed and 6
ignored. Terminal-start commit 78bd7d16 has a registered remote clippy request;
its result and this session-start slice's compile/runtime proof remain pending.
New session fixtures cover replacement before spawn and guard release after
actual registration or failed-store process cleanup. They have not yet run.

### Script process attempts

The existing script actor now accepts a generic admission callback without a
node dependency. The node uses it for every actual spawn: first run, automatic
service retry and manual restart. A registration receipt opens only after the
registry and placement check succeed, so an actor cannot spawn in the interval
before its handle is visible. Admission is released after process publication;
active processes remain visible and must be refused by the restore controller.
An approval recheck follows admission because waiting can outlive approval.
Stopping a pending attempt cancels admission before spawn. Existing library
callers keep the original unadmitted entry point; the node uses only the admitted
one. Three isolated process fixtures cover publication-before-release, reacquire
on retry plus cancellation while waiting, and refusal without command execution.
This slice is source connected; remote compilation and execution remain owed.

### Actor input/queue pause

The real structured actor now owns an internal files-only pause. A successful
receipt requires readiness, no running turn or pending human answer, and no
other pause. It changes the input route, refuses queued and newly arriving
controls (including move/rewind/wake), disables automatic queue advance and
idle parking, and restores the prior route when the RAII receipt drops. An
explicit resume waits for that actor acknowledgement. It neither emits a
conversation rewind nor alters the held queue or the parked state. The node
must hold physical exclusion before requesting it and release physical
exclusion before allowing queued delivery to resume. Lost receipts automatically
release the pause; lifecycle kill still wins. The controller and durable
recovery ownership are not connected yet, so no RPC capability is enabled.
Real fake-agent fixtures cover held-queue/input refusal and busy/parked behavior;
a closed receipt fixture covers automatic release. Remote proof is pending.

Remote terminal clippy 78bd7d16 and merged session-start suite 27f2a27c were both
cancelled rc143 while queued, before compilation. These are not compiler results;
the build-tools owner has been asked for the recovery/retry disposition.

A deterministic workspace fixture now holds every real root-metadata worker,
cancels all requesting tasks, and proves new starts are refused as MetadataBusy
until the held I/O is released. Shutdown refuses new probes independently and
drains the original workers. This fixture is not included in the currently
registered 448e2550 remote source; its execution remains owed.

The build-tools owner confirmed the simultaneous queued rc143 results were an
operator-approved build-VM restart. Original logs are retained. The obsolete runs
are superseded once by immutable 448e2550 in fresh output folders: the eight-crate
all-target clippy (with proto TypeScript feature) and the workspace/session/node
library+integration suite, both in the remote ordinary lane. Neither is a PASS
until terminal logs are collected; no duplicate or local compile was started.

### Setup, teardown and worktree removal admission

Existing retained setup tasks now take the canonical physical admission before
commands, retain it through actual command completion and root verification,
and release it after that postcondition. Existing removal/retire takes one
physical admission inside its retained task and borrows it for teardown rather
than reacquiring. The claim spans inspection, snapshot-if-needed, teardown,
Git removal and verification that no filesystem entry remains. A pre-existing
missing folder still follows the prior metadata-only removal path. Refused
admission settles the logical running claim instead of stranding it. These are
source connections; they are not a reviewed lifecycle producer yet. Physical
live-occupant validation and the new archive workflow adapters remain owed.

The fixture holds the real root gate across actual setup/removal calls, proves
setup cannot run and removal cannot delete during that hold, then repeats both
after release. Its remote run is still owed. The session failed-store fixture
now waits for its first, successful process's handover before closing the store,
so that assertion does not depend on unrelated pending persistence output.

### Durable checkpoint event foundation

A before-turn capture needs an already committed anchor; guessing the next
provider `turn_started` sequence is unsafe because incidental agent output can
arrive while the capture runs. `CheckpointEvent::PromptAccepted` supplies that
metadata anchor. `TurnAssociated` later binds it to the actual provider turn
without changing provider events. `FilesRestored` is reserved for a verified,
durably completed restoration. These are typed, bounded metadata and downgrade
to harmless logs while FILE_CHECKPOINTS is None. No producer is connected yet.
Conversation, attention, cost, search and CLI/MCP projections now explicitly
handle this event family. Regression fixtures preserve conversation identity,
the retained plan and existing inbox branch attention, including reopen. These
new fixtures, TypeScript regeneration and native consumers are not yet verified.

### Bounded actor-to-node hook bridge

The public session bridge now carries only session, immutable Boundary and the
original acknowledgement deadline. The node assigns worktree/generation and
must reject abandoned requests before reserving resources. A typed receipt
validates session/boundary correlation and reports prompt sent/refused or the
after-turn boundary's completion; lost receipts never become a success. Queue
and acknowledgement time share one deadline, and a completion-time clock check
prevents an already-ready reply winning against an expired timer. This replaces
the earlier private acknowledgement prototype. It is not connected to actual
actor delivery yet; that explicit ignored proof obligation remains. Three bridge
fixtures cover unavailable capture with normal delivery, held acknowledgement
expiry/late reply, and wrong association/lost delivery receipt. Remote execution
is still owed, and no node filesystem capability is activated by this bridge.

### Actual actor hook producers

New actors can now receive the bridge through SessionRuntime's explicit builder;
the node does not install it yet. Real new delivery (including queued delivery,
interrupt's replacement and a side turn) emits an accepted-prompt anchor after
ordered driver readiness/idle checks, asks for its before-image, and reports
actual provider delivery. Injected steers do not take this path. Actual provider
TurnStarted is linked to the anchor even when incidental output changed sequence
numbers. Published TurnEnded requests its after-image before queue advancement;
a completion pumped during another delivery is settled before that next prompt.
TurnFailed has no completed TurnEnded anchor and does not invent an after-image.

Accepted-anchor durable-channel capacity and node acknowledgement share the same
capture deadline. A congested channel, unavailable capture or expired node reply
cannot unconditionally join filesystem work or create an unbounded capture wait.
Kill remains observed. Real fake-agent fixtures cover queued-vs-injected steering,
completion ordering/association and actual prompt release on a held node deadline.
These are unrun source fixtures. The node still must wait for SQL durability of
the anchor/TurnEnded before starting real filesystem work; the remaining ignored
storage-ordering obligation is explicit. No product checkpoint controller is live.

The 448e2550 all-target remote clippy reached compilation and stopped on five
local findings: four unnecessary Future qualifications (Rust 2024 prelude) and a
pause-refusal method whose receiver was used only by test controls. The former
now use Future directly; pause refusal is an associated operation and closes
unavailable test-only control replies while paused. No lint exemption was added.
The unchanged earlier source's registered test job remains queued; a latest-head
clippy will cover the subsequent actor/event/lifecycle work as well.

### Private preview/restore reservations (staged)

The same single checkpoint migration now includes private-work allocations.
Reservation classes derive conservative byte envelopes from the existing image
cap (two images for preview work, four for restoration stages/before-images and
journal); callers cannot choose a zero-byte envelope for those classes. One SQL
accounting equation includes captures, private allocations and durable garbage.
Reservations are bound to the full capture key and human principal. An unfinished
private reservation pins session retirement. Once its worker is actually done,
a published plan or terminal/recovery journal can attach the retained allocation;
unpublished physical cleanup leaves an accounted, non-reusable id tombstone.
Retirement transfers private charges into the parent capture's garbage charge,
so metadata deletion alone never frees capacity.

Real-writer and SQL fixtures cover quota exhaustion, source/owner rejection,
non-recycled cleaned ids, unfinished-worker pins and garbage charge retention.
They have not run. Node allocation admission/cleanup callbacks and public store
facades are not wired, capture reservations still need minimum/actual-byte policy,
and plan expiry/retention/GC/recovery assembly remains unfinished. These staged
primitives do not establish an operational storage quota.

The registered 448e2550 test run stopped in its nested node-binary prebuild on a
missing TerminalHandle import and its resulting inference error; runtime tests
were never reached. The import is restored in the current source. Its four
Future warnings were already corrected by 420dcf66. Original failed logs remain
available; no result is reported as a runtime pass.

### Capture escrow settlement (staged)

The product command reserves a named fixed capture escrow; the arbitrary-size
reservation command/constructor are now test-only accounting models. Settlement
requires a matching immutable boundary in Ready or Unavailable, never Candidate.
After actual private-worker cleanup, Ready keeps raw payload plus bounded entry
and serialized-manifest allowances; Unavailable keeps its metadata reservation.
Settlement is idempotent, refuses an undersized escrow instead of inventing extra
capacity, and checks metadata byte length before loading/decoding it. This is a
reservation budget, not a promise to cap the entire database/WAL or filesystem's
physical usage. A real-writer fixture covers candidate refusal, Ready settlement
and retry, plus unavailable cleanup. It has not run. Actual worker/cleanup and
controller wiring, per-plan settlement/expiry and GC remain owed.


### Ended-target metadata contract (coordinator ruling, staged)

Reuse the existing store writer/log append rather than start an actor for ended
sessions. The checkpoint restore row now stores its metadata EventSeq; append
validates the stored Restored/checkpoint/path-count binding and records the marker
in the same savepoint. EndedMetadata derives its event from the authoritative row,
refuses live targets, and treats an existing marker as a completed retry. Live
actor append uses the same projection. Checkpoint-bearing append batches select
FULL sync as well as explicit checkpoint writes. No run/status/ended_at/queue or
conversation mutation is introduced. The shared Status comment is copied only
from de2908cd's approved event.rs comment hunk; no lifecycle branch was merged.

New real-writer fixtures cover ended status/ended_at/held queue and harness-id
preservation, FilesRestored after Status in history, replay after store reopen,
live-path refusal and ordinary append projection/duplicate rejection. They are
unrun. Store operations and projection remain staged under cfg(test), and node
controller/live pause event delivery are still unwired; FILE_CHECKPOINTS is None.


Wire-generation evidence: immutable 636dbe57044c6ff7cc4f3962017f70e891d0190a,
wp-checkpoints-node-20261115T112757Z-636dbe57, finished rc0 at 11:48 UTC.
web-ci, nested-fakes and nested-vesseld prebuilds passed; generator tests 4/0/0.
The generated proto.ts was copied from that exact verified VM checkout before
reusing its owned label. No wire declaration was edited manually. This proves
generation/prebuild for that source, not later store/actor runtime behavior.


### Owned live-pause metadata (source, not verified)

The pause guard owns a private one-slot metadata channel; the general session
handle/control channel has no result-publish command. It accepts only typed
FilesRestored identities/count, publishes under the actor's normal sequence
allocator, preserves input/queue exclusion, and returns the same sequence for an
identical retry on that pause. A different result is refused. Actor publication
is not SQL durability: the node controller must await the restore row's matching
EventSeq before releasing the physical lease and resuming. A new fake-actor test
covers one result, retry/refusal, conversation identity, and held queue until
resume. It has not run. The node restore controller remains unwired.


### Effective account preflight (staged)

[Decision Log]
- Purpose: private storage must belong to the actual executing service account;
  comparing blobs only to the archive folder owner could accept a foreign tree.
- Existing asset: Linux checkpoint Root/Archive/Stage already open FD-relative,
  refuse ACL/xattr metadata and validate modes; they lacked an effective UID check.
- Alternatives: a caller-supplied UID could diverge from process credentials, and
  account names do not establish the kernel identity.
- Choice: reuse the already resolved rustix dev dependency's process feature for
  geteuid; keep it dev-only while these filesystem mechanisms are staged.
- Consequences: project roots and private folders refuse foreign owners; private
  directories require exactly 0700 including absence of special bits. Root/group
  ownership and ACLs are never rewritten. Captured ordinary permissions remain
  subject to the frozen effective desired-image policy.

Isolated fixtures require the actual non-root build UID, test its own 0700 folder,
read only foreign-owned root metadata, and refuse 0750/setgid/sticky folders. Only
temporary fixture modes change. They are unrun; no production account/permission
flow is claimed. Root safety, archive safety and actual service operation remain
separate from source inspection.


### Startup and retention (staged SQL path)

Startup invalidates all candidate generations and drains bounded batches of
Prepared→RecoveryRequired transitions under FULL sync before session retention.
It never promotes a candidate or frees archive bytes. Ended-session pruning now
has a staged checkpoint-aware path: the established retention order/cap remains,
while busy workers, active previews, incomplete recovery and Restored rows still
awaiting FilesRestored metadata stay additionally pinned. Eligible session rows
and capture/private reservations move to independent garbage in the same
transaction. Startup retention now also uses a transaction, so transfer and
parent deletion cannot split across a crash. Filesystem garbage cleanup remains
separate and unwired; no reservation is released by metadata deletion alone.

SQL fixtures cover rollback of garbage transfer+session deletion, startup fencing
and recovery pins before pruning, and an expired successful restore remaining
pinned until its event marker is stored. They are unrun. Store checkpoint
operations/startup/pruning still use cfg(test) staging; the production startup
retention transaction is active but the checkpoint-specific path is not enabled.


### Production store boundary (source; not yet compiled)

Quality B and the local schema predecessor are already consumed, so the store
checkpoint module, error family, closed CheckpointWrite command and typed reads
now compile outside cfg(test). Commands use the sole existing writer, bounded
queue, savepoints and FULL sync; there is no second DB connection/writer. Public
records carry typed identities/images rather than SQL/JSON. Internal reads use
the existing blocking read pool and expose selected immutable evidence, bound
preview/change pages, owned operation/path journal/event marker, garbage and
bounded recovery/unpublished pages for startup reconciliation. These Rust APIs
are trusted node/storage seams, not RPC authorization or provider interfaces.

ReserveCapture now proves the exact PromptAccepted/Agent TurnEnded boundary is
already present in SQL before it can reserve/read files. Preview publication
requires its private reservation to predate allocation; restore admission reserves
its fixed private envelope and every Pending path inside the same savepoint.
Identical Admit retries do not charge a second envelope. Arbitrary-byte Reserve
remains cfg(test), never a product escape hatch around fixed capture admission.
The startup/pruning/result projection now use this production store boundary.
Filesystem/controller/GC ownership is still unwired and unreviewed; gate remains
None, so no call/capability claims a restored image.

A public integration fixture compiles the actual non-test store API and exercises
boundary refusal/success, Ready selection, missing preview reservation refusal,
owned preview/pages, idempotent admission, result-marker commit and ended reopen
retry. It uses synthetic file evidence and performs no filesystem restoration;
it has not run. Unit fixture changes reserve the same private envelopes as the
production writer. Current source needs VM compilation and behavioral checks.


### Capture guard uses the physical root (staged coordinator)

The separate bounded capture-worker guard is now keyed by the actual physical
root passed by the opened-FD admission bridge, independent of WorktreeId. The
runner forwards that root to its read callback for FD-root revalidation. A second
logical worktree alias cannot start another worker after the first actor deadline
while the first read/cleanup remains held. Existing small admission models have
an explicitly cfg(test)-only logical→synthetic-root helper; product capture input
accepts a PhysicalRoot and never derives its lock from WorktreeId.

The held blocking-read barrier now retries through a different logical worktree
with the same physical key and expects Busy, then verifies no late publication.
This is an unrun coordinator barrier, not proof of real FD/node assembly. Common
root exclusion may be released after invalidating a read-only candidate, as its
existing contract says; that does not release this separate worker/root/quota
guard. Thus a stalled read does not force the next prompt/wake to wait for it.
