# Per-turn file checkpoints: implementation plan

ADR 0077 passed design review at `a852bde3`; acceptance is a contract decision,
not a product/runtime pass. The node branch consumes the reviewed integration head `ba45a6e3` (Quality B)
and the shared `Request`/`UnexpectedReply` extraction `1c8d0b93`; pending rows now live at the end of `rpc/table.rs`. The separate
app branch consumes package bases `8d5513e4` and `6036eeb8`; no app feature is implemented yet.

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
  The real store/coordinator assembly still needs connecting. No real prompt/TurnEnded wiring yet.
- node/service/checkpoints: registered methods explicitly refuse as unavailable.
  Guarded authorization and agent refusal remain in front of the seam. Ordinary
  human forwarding preserves destination checks. Pending calls refuse locally
  and at dispatch; no feature capability is advertised.
- store/checkpoint: test-only SQL operations and one production schema append
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
