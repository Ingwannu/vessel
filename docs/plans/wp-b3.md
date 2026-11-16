# Per-turn file checkpoints: implementation plan

ADR 0077 passed design review at `a852bde3`. That is contract acceptance, not an
implementation or runtime pass. The full node package is incomplete and consumes
the announced `d5bfde87` base. Its five unary calls stay pending in the canonical
RPC table, `FILE_CHECKPOINTS` stays `None`, and production restore returns Busy.
No checkpoint app UI is implemented or advertised.

## Existing assets and reference mapping

[ADR 0077](../adr/0077-per-turn-file-checkpoints.md) records the exact inspected
Paseo, GOAT, Claude SDK and immutable node source locations. Paseo's files rewind
mode preserves the conversation; Claude's `rewindFiles` has provider tool-based
coverage; GOAT supplies turn-associated changed-files presentation. Vessel's
proposed full managed-worktree scope is a deliberate provider-independent
extension, not a claim that the SDK already implements it.

Existing `worktree.snapshots` / `drop_snapshot` recover removed worktrees into a
new worktree. Existing `session.rewind` changes conversation and leaves files.
Neither is checkpoint restoration or a fallback when restoration is unavailable.
The survey reference is `parts/A-lifecycle.md#Rewind`, files mode; conversation
rewind remains W2's separate tracker row and renderer.

[Decision Log]
- Purpose: restore the covered files at an accepted prompt/turn boundary without
  changing conversation, queues, session lifecycle or provider history.
- Existing assets: the typed protocol/RPC table, session actor, store writer and
  startup pass, TaskTracker, Git/RelPath, regular-file protections and hash types.
- Alternatives: conversation rewind and removed-worktree snapshots have different
  semantics; delegating to one provider SDK excludes other providers and cannot
  establish the selected complete filesystem scope.
- Choice: focused checkpoint modules in the existing crates; existing SQLite,
  bounded channels, SHA-256 and Linux rustix/nix filesystem mechanisms. No new
  provider engine or parallel RPC table.
- Why: each layer owns its existing authority. Filesystem code cannot authenticate
  a caller or declare durable success; the node cannot invent filesystem proof;
  the actor alone owns prompt ordering and its pause.
- Costs: bounded workers keep reservations after acknowledgement deadlines. Unsafe
  metadata, stale roots, unavailable cleanup proofs and uncertain writes refuse
  or retain recovery data. Linux is staged first; other platforms are unavailable.

## Source responsibilities and data flow

| Owner | Responsibility | Boundary |
|---|---|---|
| `vessel-proto::checkpoint` | Validating ids/limits, boundary/image/evidence/effective target, typed changes/errors/outcomes, five unary contracts | Pending capability; no assigned wire number |
| `vessel-store::checkpoint` | Capture publication, owner-bound plans, per-file intent/stage evidence, durable results/event marker, accounting and retention | Existing writer/savepoint; SQL does not prove physical cleanup |
| `vessel-workspace::checkpoint` | Strict enumeration/ignore policy, opened roots, private immutable archives, staging, guarded apply/undo and verification | No auth/session/store authority |
| `vessel-node::checkpoint` | Bounded capture coordinator/workers, deadlines, preview/restore controller, recovery and collector | Accepted work remains owned through actual I/O and durable acknowledgement |
| `vessel-session::checkpoint` | Prompt/turn handshake, owned actor pause, checkpoint metadata projection | StartupHooks remain unwired pending both cleanup producers |
| `workspace::exclusion` | Sole physical-root admission and opaque borrowed lease | Separate minimal producer review is not yet accepted |
| Native Timeline owner | Future marks, full changed-files confirmation and result/uncertainty state | No checkpoint app implementation yet |

Node bootstrap preserves the canonical original early-stop token when attaching
staged hooks. Workspace construction lives with the other setup helpers so the
node assembly stays within the announced shrinking size budget. Diagnostic error
projection matches checkpoint details explicitly while retaining safe display.

## Capture, publication and coverage

Initial coverage is structured sessions in a node-managed Git worktree. Ordinary
project directories, historical imported turns, terminal sessions, unsupported
mounts and roots shared with another live session are not capture support claims.
Root admission nevertheless includes ordinary project writers; exclusion and
capture eligibility are independent.

The actor persists/uses the exact accepted prompt or actual TurnStarted/TurnEnded
sequence. Steers are not new checkpoints; queued prompts capture only at delivery.
A distinct CaptureId, boundary, root and generation fence publication. A deadline
acknowledges unavailable without joining blocked filesystem/storage work. The
actual worker retains its bounded permit, private namespace, reservation and
capture guard until real settlement. Late candidates cannot become Ready through
worker completion or startup. The read-only capture guard does not authorize
workspace mutation or block prompt execution after its acknowledgement deadline.

Strict enumeration covers the configured Git/global/repository ignore policy and
refuses unsupported paths/metadata or excess limits rather than silently skipping
files. Immutable raw blobs and canonical evidence are private. Enumeration and
root/ignore/file evidence are checked at operation boundaries; same-user external
writers and descriptor-relative race limitations remain explicit in ADR 0077.

## Preview and restoration invariants

A preview freezes one EffectiveDesiredImage after current owner/group/read-write
retention and checkpoint execute-bit policy. An absent restored file is private.
Captured access bits that are deliberately not replayed cannot create a false
change, failure or mismatch. Preview changes, stale checks, staging, journal,
verification and Unchanged/Restored classification all use that same target.

Plans bind the complete authenticated human principal: local owner, device, or
human delegated through the exact delivering device/origin node/origin principal.
Direct and delegated agents cannot confirm restoration. Two delegated origins
cannot share a plan; the local token/socket owner remains supported.

RestoreId retries do not reapply files. Durable per-path intent precedes mutation;
all desired files are staged before the first write. Rename exchange retains the
actual originals, no-replace creation refuses external replacements, and rollback
only undoes verified objects still owned by this operation. Conflicts retain
recovery data. An unknown or lost storage acknowledgement never releases a pin as
if rollback or cleanup had succeeded.

Ended targets need no agent start/resume: FilesRestored is storage-only metadata
outside a run, with its EventSeq atomically recorded on the restore row. Status,
ended_at, conversation and queue stay unchanged. Live projection requires the
owned actor pause. A client may see checkpoint metadata after the last runtime
lifecycle Status. Retry/startup projection must not duplicate the marker.

## Quota, retention and restart

The source reserves fixed envelopes before private filesystem allocation. The
single SQL accounting equation includes captures, preview/restore allocations,
metadata tombstones and garbage outside session foreign keys. Active workers,
plans and recovery journals pin their data. Only actual cleanup plus the existing
writer's durable acknowledgement frees the corresponding physical reservation.

Rotation expires oldest unpinned images. Session deletion/retention transfers
obsolete payload charges to typed garbage transactionally; it does not pretend
files were removed. Completed payloads may retire after their verified durable
outcome and required metadata marker, while idempotency metadata remains charged.

Startup fences previous-process capture/allocation work separately from current
workers. Ready archives require verification; interrupted mutations remain
recovery-required. The keyset orphan scan preserves its saved cursor even after
prior-page rows settle. Its SQL-only pagination regression does not prove process
quiescence or filesystem cleanup. The serial collector shares the real bounded
worker budget and retains failed cleanup capacity conservatively.

The additive checkpoint migration follows the current base's newest schema in
W3's canonical migration sequence. The integrator assigns the final number at
merge. Keep `migrate()` unchanged and all checkpoint tables in that one SQL append.
Generated TypeScript must come from the build-machine generator before the final
whole gate; do not hand-edit the generated declarations.

## Shared dependencies and remaining physical admission

The minimal RootLease leaf is isolated from this full feature. Its first source
review returned ordinary project-subdirectory admission and async fixture cleanup.
The correction includes actual project and worktree roots from one registry
snapshot, retains the nearest selected opened FD, and compares parent/child
opened scopes in both directions without locking siblings or inventing a path
key. Relative input refuses before ambient-cwd canonicalization. Its current
producer is not yet accepted or adoptable; do not import it from this full branch.
After exact-leaf checks and V3/integrator announcement, merge that same producer
and retain one canonical RootGate implementation, including this package's
recovery-pin behavior.

Production restore stays Busy until BOTH announced cleanup proofs are consumed
under its exact borrowed RootLease:

- GKS lifecycle owns agent wait failures, actual process/descendant settlement,
  output drain, ended-agent proof and forgotten-agent debt (ADR 0103).
- W17 owns terminal root binding, terminal registry/forgotten debt and its composed
  root-bound proof receiving lifecycle's agent evidence. W21's runtime cleanup
  foundation is a separate dependency, not registry absence or revocation proof.

No status, PID/cwd, registry Ready/absence or ProcessLifetime is quiescence proof.
Do not wire StartupHooks to bypass these dependencies. Scripts/services including
restart, every start/restart/cleanup path, lifecycle physical effects and all
writers must join the reviewed admission/proof contract before activation.
W11's registry identities and logical leases remain canonical and independent of
physical roots. Lifecycle takes its durable claim before root admission; no
waiting for a logical claim while holding a physical lease.

G1's future comparison-cache invalidation must retain the original typed
WorkspaceRoot::Worktree and ProjectId from the same lookup. Its Mutation guard
belongs inside retained physical I/O through rollback/recovery/owner loss, not
around the RPC waiter. No second resolver or metadata-only retry invalidation is
requested; the checked reusable G1 producer remains pending.

## Native implementation contract

The app consumes only announced package bases; its current tree is on base 13.1.
Do not merge a node head or unreviewed checkpoint carrier into it.

Checkpoints will be a concrete nested Timeline field, not another resident/ticket
registry. A7 moves the entire original Area in TimelineSnapshot and must combine
the actual checkpoint hold with local readiness; ordinary Ready cannot overwrite
loading/confirmation/pending/uncertain state. Use the canonical Timeline operation
kind with a distinct opaque ticket for each checkpoint operation.

Input/confirmation requires the full current SessionContext. Queued replies retain
the original guard/context, FollowBinding and ticket, including hidden owners;
never refresh them to the selected successor. W9's exact pending binding/ticket
and checkpoint/prompt/turn correlation must match BEFORE canonical consume, then
mutate only after consume in the same synchronous UI update. A7's proposed
`conversation_settle_if_scoped` remains staged until its checked owner packet.

BeforeTurn anchors PromptAccepted, association supplies the actual TurnStarted,
and AfterTurn names its exact TurnEnded. Do not attach an unassociated image to a
nearby row or guessed index. Resolve original EntryKey/source before any numeric
position. A7 will own one agreed optional action slot after checked producers;
leave shared footer/rewind files untouched. Files eligibility never uses
rewind_point/provider rewind support or Act::Rewind, and never changes composer
or conversation.

The dialog must load every bounded change page before enabling Confirm and show
changed files, scope, irreversible later covered edits and preserved conversation.
Retain the same RestoreId/plan through disconnect/reconnect and uncertain results;
status is read-only, while the same-id retry may finish missing metadata. Use
existing kit controls and L-owned EN/KO words.

## Current verification and next steps

All results below are scoped evidence, not a package completion claim:

| Source/result | Disposition |
|---|---|
| Full feature `791531f1`, ordinary 012048 | Failed: missing exhaustive Checkpoint detail in CLI diagnostics; fixed in source |
| Full feature `c1e564328`, ordinary 025629 | Four-crate all-target strict queued; current runtime suite still owed |
| Minimal leaf `5cc56ccf` | Strict/quality/eight tests green; V3 source RETURNED, not adoptable |
| Minimal leaf `1aef57a8` | All-target strict green; Node3/7 tests failed before real fixture bodies; Workspace tests not reached |
| Revised minimal leaf `cb62b5211` | Test-only retained store/workspace setup phases separated, no limit increase; ordinary strict queued |
| Historical full feature `fe0c868b` | 92 passed/0 failed/6 ignored before later source; not current proof |
| Native checkpoint package | No product UI, gate, inspected captures or V4 pass |

The failed leaf setup run was at recorded load above the build-machine saturation
threshold. Keep it red. Shorter named phase instrumentation preserves actual
cleanup and will identify late store/workspace setup; serial case execution may
avoid concurrent SQLite migration/fsync contention without changing the internal
2-worker concurrency probes, product admission or total case limits.

Next: collect existing queued runs without duplicates. Exact-leaf strict then
actual 12 units and quality evidence precede V3 revision on its existing review
row. Any failed priority correction returns to ordinary lanes. Keep full-feature
results separate. Run the current checkpoint libraries (proto/workspace/store/
session/node) after current compilation, including permission, publication race,
actual cleanup, rollback/restart, quota/retention and ended metadata fixtures.
Map/remove empty ignored skeleton obligations only after the corresponding real
implementation and test evidence exist; activation stays unavailable meanwhile.

After producer dependencies and full node implementation: generated wire, final
whole root gate and one V3 package review/integration. Then implement native
view/link/words/screen, complete dialog/result/uncertainty flow, the native gate,
inspected 1280x900/390x844 (and 800x600 where relevant), dark/light EN/KO captures,
and V4 review. No real provider/service, permissions/configuration changes or
live customer flow have been exercised by this package's private fixtures.
