# Per-turn file checkpoints: implementation plan

ADR 0077 passed design review at `a852bde3`; acceptance is a contract decision,
not a product/runtime pass. The node branch consumes the reviewed integration
head `82ae7c91` (Quality B and gate 46g); pending rows live at the end of
`rpc/table.rs`. The separate app branch consumes package base 8 `91407173`
through app-line merge `da48212c`; no app feature is implemented yet.

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
xattr refusal, rename exchange and effective UID checks; after adding the
admitted-worker API it is a Linux normal dependency, with no new version. No provider SDK, tool-call-count inference or new crate edge.

## Current implementation and boundaries

- proto/checkpoint: validating images/metadata/limits/ids, effective permission
  policy, typed changes and errors, unary requests/responses. FILE_CHECKPOINTS
  remains None and PROTOCOL_VERSION is unchanged. The five canonical pending
  rows generate client/guard/envelope plumbing. No independent method table.
- node/checkpoint: Linux bounded capture owner, worker/root/quota leases and
  deadline fencing. Timeout acknowledges without joining blocked I/O; capacity
  remains occupied until actual cleanup. Candidate persistence and durable Ready
  activation are separate phases; uncertain activation retains the archive.
  Linux bootstrap wires the real SQL/filesystem bridge; actual actor
  prompt/TurnEnded hooks activate only after FILE_CHECKPOINTS is assigned. Compilation/runtime verification of that connection
  is pending; restore, expiry/GC and recovery controllers remain incomplete.
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
  the five-minute lifetime and four active previews per human principal; restart pins interrupted operations for recovery. The production storage API uses the existing writer, savepoint and durable reply;
  restore admission and all Pending path intents share that savepoint. Its
  real-writer commit/restart/failure-injection tests remain unverified. A durable garbage table transfers retired capture reservations out of session
  FKs; cleanup acknowledgement releases them, and active previews/recovery block
  deletion. Startup and writer retention transfer accounting transactionally;
  the real filesystem garbage collector still needs connecting. Node controller
  assembly and full physical quota reconciliation remain owed.
- workspace/checkpoint: production Linux admitted-worker API, strict enumeration, rooted nofollow reads,
  private immutable raw blobs, bounded streaming and canonical fingerprints.
  Files are fully staged before per-path mutation; rename exchange retains
  original inodes, no-replace creates avoid clobbering, and guarded rollback
  retains conflicting external edits. Apply and undo also compare inode identity,
  refusing an equal-content external replacement. Unsupported metadata on roots
  and intermediate directories refuses before file mutation. The node must
  durably journal each step
  and verify the entire covered scope before claiming success. The Linux library is active;
  FD mount IDs also reject mount crossings on the same device; actual private
  fixture and missing-capability tests are written but not yet verified. Other
  platforms remain unavailable. Full enumeration/ignore/root revalidation
  at operation boundaries and crash-restart assembly remain owed.
- session/checkpoint: production bounded hook bridge at actual prompt/TurnEnded
  boundaries, plus owned actor pause/result metadata. Injected steers create no
  checkpoint; queued prompts capture only on delivery. Node bridge installation
  and a SQL-durability barrier before filesystem reads remain owed.

## Validation and remaining work

Earlier fe0c868b checkpoint tests passed 92/0/6; generator/prebuild source636
passed 4/0/0. Those are scoped evidence for their immutable heads, not current
runtime proof. Clippy420 failed seven session lints, corrected in9fe6c1d1; its
current ten-crate replacement is queued. The older56eff914 behavior run remains
queued; the current tree includes later storage/permission/recovery changes.
No package gate, code review, current permission/platform/crash recovery or
native capture pass is claimed. Original failed/interrupted logs are retained.

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

New actors receive the bridge through SessionRuntime's explicit builder;
Linux bootstrap now installs it (see the bootstrap section below). Real new delivery (including queued delivery,
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


### Durable staged-object evidence (filesystem mechanism, not DB wiring)

New bounded storage-only proto StagedPath/StagedPaths carry stable original and
written ObjectFingerprints for exactly the approved changed paths. They are not
RPC types and add no TypeScript declaration/protocol assignment. Workspace keeps
full observations for fresh admission, exports canonical staged-object evidence,
and reconstructs recovery expectations from that evidence instead of accepting
whatever inode currently occupies the path. A reconstructed operation refuses
forward apply; it can classify Original/Desired/Conflict and perform guarded undo.
Stable hashes retain device/inode/owner/group/nlink while excluding rename-time
changes. Fresh before-write checks still compare the full original observation.

Two isolated filesystem fixtures drop/reopen handles and deserialize evidence:
rollback of an applied replacement succeeds only for the expected written inode;
an equal-content/equal-mode external replacement stays a conflict and keeps its
bytes. These tests are unrun. Store path-journal persistence of this evidence and
actual crash/startup controller assembly are still the next work; serialized
fixture reconstruction is not a process-crash/recovery pass.


### Store staged-object barrier (production storage seam)

One nullable staged-evidence field joins the same checkpoint_paths table in the
single package migration; no additional schema number is assigned. Staged commits
the complete bounded approved path set inside the existing writer's savepoint,
checking captured original object and target presence, refusing changed retries.
Advance to Applied is impossible before that path's staged evidence exists;
terminal Restored requires all applied paths to have evidence. Reads retain the
objects through restart for workspace recovery reconstruction. A real-writer
trigger fixture fails the second path, checks that none committed, retries once
after removing the trigger, rejects altered object evidence and reopens the store.
The public non-test storage fixture also uses this barrier. All remain unrun.
Consumed plans no longer count as active preview authority or independently pin
retirement: their restore/worker/metadata obligations provide the required pins.


### VM lint result 420dcf66 and corrections

wp-checkpoints-node-narrow-20261115T111842Z-420dcf66 finished rc1, cargo101
at 12:19 UTC after 121 seconds of compilation. Seven session findings:
single-pattern matches (2), receipt result missing Errors/double must_use,
acknowledge missing must_use, structured output over100 lines, and a no-effect
underscore binding in the test-only pause refusal. These are corrected by using
if/if-let, documenting the actual receipt error, retaining the advancement
receiver explicitly, and delegating the package's turn association handling to
its checkpoint module. The established closed output dispatcher remains together
and shrinks below the prior function length. No lint allowance or size ratchet is
added. This old source stopped before later node/store changes and establishes
no runtime pass; its exact failed logs remain in actor-hooks-clippy.


### Linux admitted-worker filesystem boundary (source; uncompiled)

Root/Archive/IgnoreDefaults/Restore expose the existing filesystem mechanisms to
the node without accepting RPC paths or owning auth/storage/session state. Tree
opens against the actual admitted PhysicalRoot, capture preserves the opened-FD
scope, and apply/undo recheck stable root authority before and after a mutation.
Directory mtimes changed by our own renames do not mint another authority. Recovery
reopens expected stable objects and refuses forward apply. Source errors remain
owned and typed; public reason classification does not expose file paths/bytes.

The already resolved rustix fs/process features move from dev-only to Linux normal
dependency for safe FD/xattr/rename/UID operations; no new version or Vessel crate
edge. The API exists only on Linux like the existing physical FD seam; node
adapters must explicitly return UnsupportedPlatform elsewhere when activated.
FILE_CHECKPOINTS stays None on every platform. Test-only convenience methods are
not product bypasses. The public-worker fixture executes on spawn_blocking with
isolated Git configuration, checks a wrong physical root, captures/restore/reopens
rollback evidence and asserts the actual non-root UID. It is unrun, and models its
evidence commit without a store, so it is not end-to-end restore/crash proof.


### Ended runtime cache visibility

An ended handle can remain briefly in Sessions until persistence handover. Its
ring is sealed; it cannot override a later stored FilesRestored page, and get/list
retain a newer stored EventSeq instead of overwriting it with the stale ended
SessionInfo. The existing read module owns that preference and get. The file
budget check caught the added list branch pushing sessions.rs over500; moving its
query method into the existing read module restores the budget without raising a
ratchet. A real ended fake-runtime fixture checks page selection while handover
is held, using an explicitly modeled store page; store tests separately validate
the actual result-event binding and restart transaction. This is unrun and not a
combined end-to-end restore/subscribe proof.


### Hydrated recovery admission and replacement folders

Workspaces now reads all bounded recovery pages and their owner-bound plans before
starting RootGate. The same gate stores physical roots plus the managed worktree
namespaces those unresolved rows belong to. A folder replaced with another inode
cannot bypass recovery by presenting a new physical key for the same worktree.
This namespace is an additional fail-closed recovery condition, never the key of
a file lock. Lease.root remains the actual opened-FD identity. Independent logical
identities sharing a physical root still conflict through that physical key.

A single metadata helper opens the requested directory and registered worktrees,
finds the nearest physical managed ancestor and all names bound there. Actual file/
Git writes and starts pass that evidence to the sole gate. Pre/post writer checks
now use the bounded retained metadata pool and verify that the admitted physical
root is still in the directory's FD ancestry, including subdirectory starts.
Only missing unrelated roots are skipped; other metadata failures refuse safely.
A gate fixture checks old physical key blocking, new inode/same managed id blocking,
and a genuinely independent root's admission. It has not run. Runtime pin/guarded
recovery-claim/completion APIs and the actual restore controller remain owed; no
raw production force-unlock or new lock owner is introduced by this slice.


### VM behavior compile result 56eff914

wp-checkpoints-node-20261115T114927Z-56eff914 finished rc1 at12:40 UTC.
web-ci/nested-fakes/nested-vesseld passed; cargo stopped compiling store unit tests
on E0308, the CheckpointWrite apply error conversion already corrected in69ed6d563.
The emitted inbox fixture qualification warning is now corrected too. No behavior
test executed in this run, and the preceding node prebuild is not runtime proof.
Current storage/FS/admission source remains ahead of that failed immutable head.


### Recovery settlement is per operation (controller seam still staged)

RecoveryRoots now retains RestoreId→WorktreeId per actual physical root. Startup
hydrates those exact identities. Resolving one operation cannot clear another
operation or aliased worktree's pin. The staged claim/pin/resolve controller seam
uses an uncloneable physical lease and compares its retained token by identity;
late callbacks from a dropped/replaced lease are refused. A recovery lease remains
owned after settlement until its physical work/postcondition has ended and the
holder drops it. Idempotent settlement on that same held lease reuses the result.
Recovery claims can enter an already-counted pinned root even at the root-count
limit, avoiding a full-registry deadlock.

The old test-only raw root clear is removed. Controller-facing recovery mutation
APIs stay cfg(test) until the real restore controller consumes them; the ordinary
production writer/start gate and startup identity index remain active. Tests cover
two restore pins on one physical root, wrong operation, stale token, shared root
blocking and continued physical ownership after resolution. They are unrun, and
provide no actual SQL/whole-filesystem recovery proof by themselves.


### VM lint result 9fe6c1d1

wp-checkpoints-node-narrow-20261115T122206Z-9fe6c1d1 finished rc1/cargo101
at12:57 UTC after163 seconds. Store lib stopped on one similar_names lint in
capture escrow settlement (charged/changed); the SQL affected-row count is now
updated. The run did not complete the later dependency graph, current worker API
or recovery interfaces, and no runtime pass follows from it. Exact logs remain
in production-store-clippy. No lint allowance is added.


### Private storage creation (cleanup not yet wired)

The Linux worker Storage API opens the service's already-private data directory
and uses relative no-follow directory handles for checkpoints/worktree/capture,
immutable blobs, preview before-images and restore stages. Only validated ids and
fixed component names form descendants. Existing folders are validated for actual
effective UID, exact0700, unsupported attributes and mount identity, never chmodded
into acceptance. Newly created directories get a no-follow0700 chmod independent
of umask, followed by child/parent sync; immutable Archive handles reuse opened
folders instead of traversing a pathname again. Stage location keeps its opened
folder owned while the existing restore worker API opens its stage.

All creation requires the node's prior durable escrow and retained worker. The
Storage API itself does not authenticate or release quota and does not delete on
Drop. Filesystem GC/cleanup callbacks remain next work; no operational quota or
cleanup proof follows from creation. Isolated fixtures check non-reuse, private
UID/mode, symlink refusal and preserving an existing0750 directory. The umask0777
case runs in a one-test child with an explicit helper selector, verifies the helper
actually created storage, and never changes the parent/service umask. The helper
is intentionally ignored in normal runs and explicitly executed by its parent.
These tests are unrun; chmodat/no-follow behavior on supported kernels remains a
VM/platform obligation. No live permissions or runtime configuration changed.

### Capture cleanup is independent of publication (source; checks pending)

An unavailable capture may still own blocked read I/O and private writes. The
same migration now records active/settled worker cleanup independently of the
candidate/ready/unavailable state. Session deletion and retention refuse active
workers; only CaptureSettled after actual cleanup reduces escrow and permits
garbage transfer. Startup invalidation cannot manufacture cleanup completion.
Pure SQL fixtures explicitly model no worker; there is no production bypass.
The writer regression covers timeout, refused deletion/no garbage, cleanup
acknowledgement, then successful deletion with retained metadata accounting.
This is source evidence until the next immutable remote test completes.

### Private garbage cleanup mechanism (source; remote verification pending)

Storage::clean_capture consumes only typed worktree/capture identities after the
node proves worker/pin retirement. A bounded pass traverses the fixed private
layout through no-follow FDs; only canonical blob, plan, restore and stage-slot
names are accepted. Opened UID, regular-file/link count, mount and attributes
are checked before unlink, with directory identity checks and parent fsync.
Pending keeps the charge; Complete requires synced namespace absence. Unknown
entries, symlinks, hardlinks, mount crossings and I/O errors retain accounting.
No project paths, recursive path deletion, chmod-on-reuse or Drop cleanup.
Fixtures cover repeated bounded passes, retry after absence, sibling isolation
and refusal without changing linked outside bytes. Runtime GC assembly remains
outstanding, and these new Linux fixtures have not executed yet.

### Actual actor advancement gates Ready (source; remote verification pending)

The capture runner now returns AcceptedCapture after the in-time durable
candidate, retaining a separate receipt through the actor bridge. Only actual
PromptSent/TurnComplete permits Ready activation. Refused delivery or dropped
receipt discards; candidate acknowledgement alone cannot make Ready. The worker
and its physical/quota admission remain occupied throughout that wait. A
three-outcome barrier regression checks no early activation and Keep/Discard
cleanup. This closes the adapter boundary before real bridge assembly; it does
not claim that assembly is already installed.

### Real capture backend assembly (source; not installed at bootstrap)

The staged Linux backend now binds the bounded runner to the existing store
writer and managed-worktree filesystem implementation. ReserveCapture proves
the exact durable anchor before private namespace creation or project reads.
The blocking worker resolves the registered WorktreeId, refuses an active
script, verifies the supplied opened physical identity, and captures with
explicit global-ignore defaults. Candidate and Ready remain separate store
barriers; actual actor advancement gates Ready. Discard retains the original
typed failure, invalidates before physical cleanup, and settles escrow only
after synced absence. Lost reserve/activation acknowledgement retains the
worker and archive for reconciliation. No second writer or extra I/O pool.
The integration fixture uses a real isolated managed worktree and sole writer,
with no provider process: accepted delivery retains readable Ready evidence;
refused delivery exposes Unavailable and removes the private capture. Both
are unrun until the next remote suite. Bootstrap/turn bridge, recovery and
restore controllers remain incomplete; FILE_CHECKPOINTS stays None.

### Linux bootstrap and first-anchor durability (source; unverified remotely)

Linux startup now installs CheckpointHooks and starts one bounded bridge with
the existing sessions, workspaces, sole writer, task tracker and shutdown token.
The original actor deadline covers registration, persistence, root metadata,
admission and candidate acknowledgement. Initial actor hooks may precede the
node durability table: a pre-registered Notify waiter prevents lost wakeups,
then persist::wait_until proves the exact next sequence before filesystem work.
Runtime visibility alone never proves a committed boundary. Registered worktree
identity comes from the session's resolved workspace or its validated place,
then an opened physical guard is revalidated and held through actual delivery.
The separate capture worker retains its own root/quota permit until actual
cleanup. Full principal authorization/restore remains in the pending handlers.
Other platforms install no capture hooks and advertise no file checkpoint RPC.

Bridge work is bounded by the existing request queue and two handlers. Shutdown
closes admission, drains handlers and waits for actual I/O; a cleanup error
keeps serving-time admission and durable escrow pinned, while shutdown can exit
once the I/O returned, leaving reconciliation work persisted. A blocked syscall
cannot use this failure path. Git's default global-ignore fallback is resolved
once at the node environment edge; unsafe defaults refuse capture.

Node runtime construction moved intact into node/setup beside its other setup
helpers to fit the file budget; its ratchet decreases from 691 to 687. Session
limit-association reads moved into sessions/read for the same reason. No limit
was raised. Registration/writer barrier and real-node/fake-ACP turn fixtures
were added, including stored image reads after node shutdown/reopen. They have
not executed yet. Runtime Busy timing, startup GC/recovery and the full node
gate remain owed. No runtime capability, product completion or review is claimed.

### Previous-process capture reconciliation identity (source; unverified)

Store startup now marks prior active workers as orphaned in the same FULL
transaction that invalidates candidates. New runtime captures remain active;
the bounded internal orphan keyset scan cannot return them. Orphaned ready
rows retain their exact published checkpoint identity and must have all private
payloads verified before settlement; unavailable rows require actual private
namespace cleanup. Retirement pins both active and orphaned workers. This
prevents a background startup collector from sweeping a new live capture.
The SQL fixture checks retained/discard identities and excludes a new worker;
it models physical cleanup acknowledgement explicitly, without claiming I/O.
Archive verification reuses the existing bounded hashing loop, now checking
opened private blob mount identity and metadata too. The actual-worker fixture
checks valid retained payloads and refuses later corruption. Node collection
and shared capture/restore/GC worker admission remain the next integration step.

### Shared checkpoint I/O admission (source; unverified)

The Linux capture backend now receives a node-owned Workers budget rather than
inventing a second pool. Capture jobs retain its opaque lease in both the owner
and actual blocking closures, including cleanup. Losing the async owner cannot
release capacity while kernel work is held; failed cleanup retains the owner
copy until serving ends. Restore and collection must consume this same budget.
The barrier fixture combines an external collector lease with held capture I/O,
then loses the model owner and proves the actual closure still exhausts the
shared budget until it returns. No real process or filesystem is cancelled by
that fixture. Remote execution and actual collector wiring remain outstanding.

### Collected shared-worker baseline clippy evidence

Run wp-checkpoints-node-narrow-20261115T130022Z-bfb7559c finished rc1,
cargo101 after16s. Five workspace-library lint diagnostics stopped it: test-only
AtFlags import, missing method-name backticks twice, the then-production-unused
owned archive path, and the public generic Error type name. The first three
source sites are corrected; the public error is CheckpointFsError (internal
scoped aliases remain short). Private directory creation already changed owned
archive-path forwarding/consumption after that tested head; the replacement
checks that newer source rather than claiming the old diagnostic is green.
No runtime tests executed in this clippy run. The ordinary test run remains
queued and is not duplicated.

### Connected private collector (source; remote verification pending)

Linux bootstrap now starts one serial collector with the same Workers instance
as capture. Every pass scans bounded previous-process orphan metadata and
durable garbage, never active runtime captures. It verifies retained Ready
archives or removes discarded/retired namespaces on an admitted blocking
worker, then awaits CaptureSettled/GarbageCleaned on the existing writer.
Only successful actual I/O and its durable acknowledgement release accounting.
A failed physical object retains its opaque lease until serving ends; a bounded
held set prevents repeated admissions for that same object. Busy/ambiguous
store work defers until the next named interval with quota still charged.
Shutdown stops new collection and waits for actual in-flight I/O; no kernel
cancellation or filesystem atomicity is claimed. Global-ignore configuration
does not gate private cleanup. The managed-worktree backend fixture now also
retires a real capture, proves garbage exists, collects its actual directory,
and verifies synced absence plus the writer's accounting removal. It closes
archive handles first. This fixture is unrun; orphan restart integration,
preview expiry/image rotation and restore recovery remain owed.

### Physical launch binding and the pending activation gate (source; unverified)

Node registration retains the physical root supplied by the actual held startup
admission through create/resume. This is launch evidence, not another root lock
or WorkspaceId. After taking the canonical physical guard, capture checks the
launch binding and rejects a replaced cwd inode or another live session sharing
that physical root. Unknown live bindings conservatively yield Busy. Root and
durability visibility publish together, and handover removes both; registry
locks are never nested with runtime handle locks. The real fake-process fixture
checks shared-root refusal and stable evidence after a cwd pathname is reused.
Internal wake/move/fork start admission still needs its own complete bridge.

Bootstrap now gates hook installation on the single FILE_CHECKPOINTS assignment.
While it remains None, ordinary turns emit no new checkpoint anchor events and
all pending RPCs stay unavailable. This avoids changing baseline history before
central activation. The real wire/bootstrap fixture explicitly remains ignored
until that assignment, rather than being counted as a pass. Private runtime
and backend fixtures can install the existing typed hooks for pre-activation
verification. The collector remains wired for already-journaled private work.

### Private runtime integration fixture before wire activation (unrun)

The node's session fixture can install the existing typed CheckpointHooks
explicitly, independently of the production FILE_CHECKPOINTS gate. The new
fixture starts a real isolated managed Git worktree, actual node session
registration/persistence, the production bridge/backend/collector, and only the
fake ACP provider. It sends two real turns, shuts down actors and drains tracked
work before inspecting stored before/after images. The normal wire fixture
remains explicitly ignored pending version assignment; this private fixture
covers runtime ordering without inventing a production activation setting.
No results are claimed until the immutable remote test actually executes.

### Collected worker-API test attempt

Run wp-checkpoints-node-20261115T124207Z-89b77cab reached the ordinary slot
at13:57 UTC after4442s queued. web-ci and nested-fakes passed; nested-vesseld
failed cargo101 after132s. Its root error was the missing SessionInfo import in
sessions/read, followed by23 inference cascades in callers of get(). The import
was already corrected in4184aa0d6; current source retains it. The old AtFlags
warning was also corrected before the replacement. No behavior tests executed.
The replacement runs the current runtime/GC/physical-binding fixtures rather
than presenting the older compile attempt as their verification.

### Preview selected/current union (source; unrun)

The admitted Tree preview read extends current coverage with explicit absent
observations for selected files that disappeared. An existing selected name
outside today's Git enumeration refuses UnsupportedScope; historical coverage
cannot authorize reading/deleting a now-ignored file. Current image and evidence
therefore cover the same union used by effective desired files. Final verification
must use the whole frozen desired image as its union, including approved deleted
paths, so a missing file is compared as Absent rather than omitted. The actual
filesystem fixture covers deleted untracked files, private creation metadata,
and refusal without replacing currently ignored data. The node account defaults
come from actual effective UID/GID and private0600, not project or checkpoint
ownership. No runtime controller claim follows from this worker mechanism.

### Consecutive-boundary publication handoff (source; unrun)

A watch contains at most the two admitted physical roots and their bounded
Reading/Publishing/Unavailable states. A production bridge request waits for a
prior accepted capture's activation/cleanup inside its existing bounded handler
and original actor deadline, before acquiring the common root. It creates no
new read, waiter task or timeout budget. Expired reads and cleanup faults remain
Busy; a stuck kernel job never permits a replacement worker. After actual actor
advancement, the bridge drops its common root before allowing Ready activation,
so the next availability notification cannot race that old common guard. The
separate capture root/worker/quota ownership still lasts through actual cleanup.
The held-publication regression checks a following boundary waits with only one
read, honors an already-expired deadline, and reads only after actual release.
The real two-turn runtime fixture remains owed remotely.

### Confirmed reservation refusal versus ambiguity (source; unrun)

ReserveCapture can return StorageFull from quota admission or StaleCapture from
its boundary proof before INSERT. Neither path has begun a filesystem callback.
The runner now distinguishes these confirmed no-reservation outcomes, returns
the typed error, and releases its memory/root/worker admission after the no-I/O
cleanup acknowledgement. Other storage/commit failures remain uncertain and keep
recovery ownership. This classification applies only to the reserve phase, never
to a candidate or Ready publication error. The repeated-refusal fixture checks
more attempts than worker slots with no file read; the real managed backend
also refuses an uncommitted anchor, creates no checkpoint directory, then accepts
a valid capture. This prevents ordinary quota exhaustion permanently occupying
both worker slots without weakening ambiguous-commit handling.

### Collected turn-bridge clippy attempt

Run wp-checkpoints-node-narrow-20261115T134727Z-c70c6ca4 finished rc1 at
14:33 UTC, cargo101 after15s. Earlier public filesystem error/path/import lints
no longer appeared. Two diagnostics stopped this attempt: a missing unit-result
semicolon in archive verification, and an unconsumed pause Signal in the lost
receipt regression. The source now terminates the unit statement and explicitly
asserts Released for that signal. No runtime tests ran; later dependency graph
and newer producer/collector/physical-binding source still require checks.

### Replacement-start actor seam (source; node consumer staged)

StartupHooks provides bounded queue-inclusive admission for the one start_agent
path shared by wake, move, fork and rewind. Before any spawn, a granted permit
becomes Starting. Synchronous spawn failure reports NotStarted; ownership and
runtime publication report Registered before handshake processing. A dropped
granted permit never began spawn, while Starting plus sender loss is unknown
and must keep node exclusion. Registered grants no descendant/I/O cleanup proof.
The node must bind W11's canonical lifecycle/restart claim and the existing
physical RootLease before granting; that consumer is not installed yet and no
local replacement registry/lock is introduced. Initial creation/resume still
uses its existing retained node admission.

Restart::Admission preserves typed refusal. A move that already linked the
target profile retains its conversation claim on this failure just as it does
for spawn/handshake failure; a failed pre-move wake keeps its existing behavior.
Protocol fixtures distinguish dropped pre-spawn permission from unknown begun
spawn, terminal progress after sender loss, lost node observer and queue-time
expiry. The fake ACP wake fixture holds admission, refuses without publishing
a replacement, then grants and observes the actual admitted wake/next turn.
These tests are unrun. Wire version/schema and runtime Config remain unchanged;
there is no production activation option or consumer-ready claim.

### Controller preview assembly (source; unverified)

[Decision Log]
- Purpose: connect the frozen files-only confirmation to actual managed files and
  the existing sole store writer, without letting an RPC caller own worker I/O.
- Existing assets: capture Tree/Storage, opened-root admission, principal-bound
  Plan/changes, the shared checkpoint semaphore and node TaskTracker already
  provide the required boundaries; no new filesystem or database dependency.
- Choice: extend those assets. One controller is wired into NodeService and
  shares the capture/collector worker pool. Preview reserves private quota before
  opening its current-image archive, validates selected blobs and ignore policy,
  captures the selected/current union and publishes the effective desired image.
- Reason: a disconnected/timed-out caller cannot release a blocked worker. The
  tracker retains the job through actual cleanup and durable accounting. An
  uncertain publication never deletes possibly referenced bytes; a failed
  cleanup or uncertain accounting retains its worker slot for reconciliation.
- Tradeoff: the response deadline does not cancel a database command already
  submitted; an inaccessible late plan can remain pinned until expiry/collection.
  Restore execution, stale-confirmation checks and allocation collection remain
  incomplete. FILE_CHECKPOINTS remains None, so the public gate stays closed.

Private cleanup now supports one typed preview or restore namespace with the
same bounded, no-follow, synced-absence checks as whole-capture retirement.
Regression obligations include a real bridge/fake-provider preview that leaves
current bytes untouched, principal isolation, and idempotent namespace cleanup
without removing siblings. These added tests have not run.

Run wp-checkpoints-node-20261115T140208Z-14bc5dd4 finished rc1 at 14:59 UTC,
with cargo101 in nested-vesseld: cleanup referred to a removed key binding.
No tests executed. The corrected worktree/capture arguments and three reported
warning fixes are included here. This is not runtime or permission evidence.

### Retained restore execution (source; unverified)

[Decision Log]
- Purpose: make a confirmed files-only plan executable without replaying a prompt
  or starting an ended target, while retaining truthful crash/failure outcomes.
- Existing assets: canonical RootGate, opened Tree/Restore, per-path store intents,
  stable staged-object evidence, whole-image recovery validation and storage-only
  FilesRestored already implement the required mechanisms. They are extended.
- Choice: read-only scope observation hashes through the existing bounded loop
  without growing archives. Under the held root, fresh admission rechecks the
  full preview image/evidence and expiry, pins recovery, and commits Admit before
  stage creation. All stage evidence commits before first apply; each applied
  postcondition advances its journal. Final verification uses the whole frozen
  effective target. Failure fences RecoveryRequired before guarded undo; only
  the store's whole original-image/object verification can report RolledBack.
- Why: neither cancellation nor a lost SQL acknowledgement proves whether files
  changed. A retry reads its durable operation before expiry and never reapplies
  a terminal success. Uncertain terminal acknowledgement is reloaded before undo.
- Tradeoff: the retained tracker enforces cancellation even after RPC caller loss,
  then continues polling actual I/O/cleanup. A response deadline never force-drops
  kernel work. Partial/conflicting state retains private escrow and the physical
  and worktree recovery pins. The original writer can settle only its own held
  verified pin; settlement does not drop the physical guard early.

**Runtime admission remains closed.** W21 confirms no reviewed terminal
quiescence producer exists, including forgotten-but-unconfirmed runners. Registry
absence, ended flags, cwd, PID and ProcessLifetime are not cleanup proof. The
production controller therefore returns Busy before restore admission. Only the
private fixture that never constructs terminal runners installs an explicit
cfg(test) admission. That fixture now covers stale confirmation before Admit,
ended status/ended_at preservation, FilesRestored after Status, idempotent retry
that preserves a later external edit, and a default-controller Busy assertion.
These are source obligations, not passing runtime evidence. Live target pause,
canonical replacement-start admission, terminal cleanup-debt adoption, automatic
restore reconciliation and preview/image retention are still unfinished.

Run wp-checkpoints-node-narrow-20261115T150311Z-4d997fe4 finished rc1 at
15:23 UTC (cargo101). It reached session integration support and node source:
three new test suites lacked the established cfg(test) support-module marker;
node diagnostics covered the old long preview function, unused session lookup
results, two doc backticks, lazy Option fallback, needless Result wrapping,
PreviewRead borrowing and startup root-evidence/RAII naming. The long function
and async restore were already replaced by the retained execution slice; this
followup fixes the remaining diagnostics without new lint suppressions. Root
launch evidence now uses an explicit conversion from the whole held Admission,
retaining the placement guard through registration. No test/runtime pass follows
from this lint correction.

### Rotation, private expiry and restart projection (source; unverified)

The existing single checkpoint migration now permits Expired capture marks and
Orphaned private reservations. No migration-runner or schema-number change is
introduced. Collect is a bounded command on the sole writer: it retains 40 ready
unpinned images per session, active worker/preview/restore pins remain additional,
and older payload charges move atomically into durable garbage while boundary
and cursor tombstones survive. Capacity pressure plans enough safe reclamation
for the fixed capture envelope; already pending garbage prevents over-pruning.
Actual quota admission still counts that garbage until physical cleanup completes.

Expired, unconsumed, Attached preview plans can retire independently. The plan
loses authority before GC, its allocation id retains a metadata charge, and its
payload charge stays in typed preview garbage. Pending child garbage pins whole
capture/session retirement, avoiding concurrent parent/child namespace cleanup.
Session deletion merges an existing whole-capture garbage charge atomically.
All restore journals remain pinned in this slice; terminal journal/stage payload
retirement is still unfinished, and StorageFull remains truthful meanwhile.

Startup fences prior-process Reserved allocations as Orphaned; current new
workers remain Reserved and cannot enter that scan. The node data lock establishes
that the old private worker ended, independently of publication. Referenced
orphans retain their full charge and become Attached; unpublished private ids
move to typed preview/restore garbage without freeing bytes. Restore verifies
its referenced current-image archive before staging. This reconciliation neither
promotes a capture nor treats elapsed time as I/O completion.

After the existing interrupt-live startup pass, pending completed restore metadata
is replayed through the existing store transaction before retention/admission.
FilesRestored and its restore-row EventSeq commit under FULL sync together;
startup failure rolls back both. Added SQL regressions cover pins, the 40-image
boundary, pressure accounting, old versus current private workers, no late plan
publication, atomic session deletion and failed startup marker replay. These
checks have not run. Runtime restore admission stays Busy awaiting W17's reviewed
terminal quiescence producer and the canonical startup consumers.

Run wp-checkpoints-node-20261115T150311Z-4d997fe4 completed rc1 at 15:31 UTC.
Nested vesseld compiled, but the selected tests did not run: workspace test code
referenced undeclared serde_json. This followup uses saved typed evidence instead;
codec roundtrip tests stay in proto/store, which own JSON, with no new dependency.
The reported test-only qualification/unused-result warnings are also corrected.


### Completed payload collection and completion retry (source; unverified)

[Decision Log]
- Purpose: free terminal stage/preview payloads without forgetting an old
  RestoreId outcome or turning a lost result-marker ack into repeated file I/O.
- Constraint: completed result/plan metadata has foreign-key and idempotency
  obligations beyond the raw image's retention lifetime.
- Choice: only settled Attached private allocations with a complete outcome and
  durable required result marker retire. Their payload charges move to typed
  garbage; exact encoded metadata, conservative copies and per-row overhead
  remain charged on non-reusable allocation ids. Completed plans/results remain
  in SQL, while expired images stop authorizing new restoration.
- Why: deleting result rows would let a reconnecting client mistake a retry for a
  new operation; retaining worst-case stage escrow indefinitely would exhaust
  quota despite completed cleanup. Pending child garbage still pins parent GC.
- Tradeoff: in-flight/recovery journals remain fully pinned; metadata lives until
  session retention. SQL accounting tests model physical acknowledgements and do
  not prove namespace cleanup or real runtime outcomes.

The same canonical RootGate now supports lookup of a pending operation's original
physical/worktree pin. A completed retry uses that identity to acquire its exact
recovery lease, publishes metadata only, and settles the pin. It never infers
write authority from a point-in-time lookup or replays file changes. The private
real-worktree/fake-provider fixture injects a failed result-marker trigger after
file success, checks that admission stays fenced, then retries publication and
checks root admission releases without file reapply. A later external edit is
preserved. The new writer regression retains the completed result through payload
collection, image expiration and database reopen. All additions remain unrun.

### Actual writer ownership (source; unverified)

The restore blocking closure now owns an Arc of its actual StartGuard in addition
to the shared worker lease. The async owner and the closure therefore both retain
physical exclusion through real mutation/postconditions. Lost async ownership
cannot release another writer while kernel I/O still runs; the operation/pin and
quota remain available for reconciliation. A barrier regression aborts only the
pure test waiter and proves the canonical gate stays Busy until the real blocking
closure drops its lease. This is an ownership test, not descendant or kernel-I/O
cancellation proof. Live restore remains closed; its eventual owned actor pause
must likewise survive actual worker I/O if its async coordinator disappears.

StartGuard exposes a borrowed opaque root_lease() for W17's future lifetime-bound
quiescence contract. Its physical() accessor uses that exact same lease; there is
no copyable lease id or additional root lock. This is unverified source and no
consumer-ready hash or external adoption is announced.


### Refused-boundary metadata (source; unverified)

Root/worker admission can refuse before a filesystem candidate exists. After
acknowledging the actor's error, the retained bounded bridge now records a small
Unavailable marker when the exact managed boundary/SQL anchor is known. The
writer checks that anchor and quota, creates no private namespace, and never
promotes it to Ready. Existing active/ready rows remain under their original
worker's ownership and are unchanged by this best-effort metadata command.
Missing/unsupported scope, absent anchors or exhausted metadata capacity can
still leave no mark; the app must treat missing availability as unavailable.
The actor never waits for this trailing write past its capture deadline.
The regression checks durable-anchor refusal, stable single charging, no Ready
publication, no filesystem I/O and restart persistence. It remains unrun.

Run wp-checkpoints-node-narrow-20261115T153918Z-2f7778ad finished rc1 at
16:03 UTC (cargo101, eight seconds). It stopped at a private parked() test API,
a needless preview-observation move and startup doc formatting. Tests now use
SessionInfo/watch_info, the observation is borrowed, and docs use ordinary field
wording. No product API or lint suppression was added to satisfy the fixture.

A separate isolated SQLite compatibility probe executed the exact extracted
rotation/completed/garbage SQL: rank/40 boundary, marker/worker/recovery pins,
consumed-plan FK retention, Expired state and garbage merge passed. This verifies
SQL compatibility only; it is not Rust execution, actual namespace cleanup,
permissions, runtime parity or a replacement for the queued remote suites.
