# 0046: One prompt to several independent sessions

Date: 2026-11-14. Status: proposed contract, implementation not started.

## Context and reference

GOAT's `apps/web/src/components/chat/useMultipleModelSend.ts` starts a
background thread and required worktree per selected model. It composes
existing thread-start operations; it does not create a server comparison
room or combine providers in one conversation. `chatViewSend.ts` retains
uncertain submissions and offers Open thread or an explicit Allow retry.
`useComposerSend.ts` validates all selections and common attachments before
dispatch. `composerSend.logic.ts` and its tests restrict this to a new Git
thread, a base branch, at least one selection and a capable server.

Successful starts survive another model's failure. Failed selections and
the submitted draft are restored only when the original draft is still
unchanged; otherwise restoration is a separate user action. A person can
open, continue or discard each independent thread. Paseo and
agent-orchestrator have no matching row in the feature table.

Vessel already has typed project/git reads, approved worktree setup,
structured session creation, verified chunked attachments, Start-only
prompt delivery and normal per-session lifecycle controls. Duplicating
those paths in a new batch daemon would introduce a second authority and
failure lifecycle without a reference requiring one.

## Decision

Compose the existing node calls in `vessel-client::comparison`, with a CLI
consumer. No new RPC, event, scope, stored comparison room or protocol/schema
bump. The track's current protocol remains 19 for side questions; this
composition needs protocol 10, or 11 for an added agent, as its underlying
calls already declare. Normal clients still see ordinary worktrees and
sessions. A native comparison screen belongs to the app owners.

One owned client job prepares independent worktrees/sessions and sends their
job-bound common input. It can retain pending selections while others start.
Typed per-selection outcomes retain every resource, including uncertainty.
Never automatically retry a mutating call, silently start without setup,
queue/steer the initial prompt, resume an existing session or delete another
selection's successful work. A failed comparison is not an atomic rollback.

### Input bounds and independent preparation

- `ComparisonPlan`: one ProjectId, one explicit WorktreeBase and 1–8
  distinct structured targets. One target is valid for retrying a failed
  selection; zero is rejected before any call. A target carries HarnessRef,
  optional ProfileName/ModelId and OptionChoice values, not SessionSpec:
  no terminal, resume, cwd, place or side link. At most 32 choices and
  16 KiB encoded target metadata; at most 128 KiB for the complete typed CLI
  manifest. Unknown manifest fields are refused. A bounded SelectionIndex
  keeps input order independent of completion order.
- `NodeClient::comparison(destination: NodeId, plan)` constructs one
  nonclone ComparisonJob owner before any remote work begins. The job has
  one bounded command slot, at most two active selections and one latest
  immutable snapshot with at most eight entries. An observer can clone a
  passive snapshot, never an executable continuation. No snapshot can be
  deserialized or converted into a replacement owner.
- `job.prepare()` is the initial preparation command, admitted once;
  repeated prepare returns the existing checkpoint, with no new calls.
  It runs shared read-only preflight: connection/destination
  protocol, Git project and provider/profile availability. All calls use
  call_on for that retained destination, including attachment and event
  reads; a member's ids must not fall through to the home. Resolve the base
  once through git.log (limit one), then use that same CommitId for every
  worktree even if the branch advances. Do not copy uncommitted files.
- Each selection creates its own random-named branch/worktree through
  worktree.create with setup Run, observes that setup, then creates an
  ordinary structured session there. NeedsApproval/Refused/failed setup is
  explicit. No automatic approval, setup skip or start-anyway fallback.
  Metadata/provider/readiness calls have 45-second deadlines; mutating
  worktree/session/prompt calls have 120-second deadlines. These are new
  client-operation guards, not changes to existing test safety timeouts.
- A checkpoint can contain Ready, WaitingSetup, WaitingReady, NeedsApproval,
  Started, Failed, Uncertain and Cancelled entries together. Known branch,
  worktree and session identities stay with each entry. A waiting state
  stores its existing observation cursor and exact next phase. A previously
  acknowledged NeedsApproval permits another worktree attempt only after
  explicit approval: that outcome says no worktree was made. An uncertain
  creation never becomes NeedsApproval/Waiting and cannot be retried.

### Observation when the node does not honor wait

Long polling is advisory. worktree.output removes wait for every delegated
principal; the shared wait-slot guard can remove it locally too. RPC client
configuration supplies only a handshake timeout, not a read deadline. The
composition must neither weaken those restrictions nor assume an empty
Running response waited thirty seconds.

Setup reads request wait <=30 seconds and have a 45-second response deadline;
event/readiness observations have the same per-call deadline, within a
45-second readiness observation budget. Setup observations have a ten-minute
budget per explicit prepare/continue operation. Progress means an increased
output/event cursor or a changed typed lifecycle/setup state; repeated
Running with no new bytes/records is not progress. Do not accumulate output.

The first no-progress response returns the SAME WaitingSetup/WaitingReady
entry and ends that operation's observation. It sends no immediate follow-up
request, regardless of how quickly the reply arrived. A deadline with no
mutation in flight also returns that waiting entry with its resources and
cursor. The job retains a monotonic next-eligible observation time per target and
phase: at least one second after a no-progress reply, preserved across
method calls. Calling
continue_target before then returns the passive waiting snapshot without
network I/O. There is no automatic no-progress polling loop in the helper.
The app can offer Continue or use the ordinary setup viewer; it must not
reconstruct a new comparison to wait. Readiness uses the same rule and
continues observing the SAME SessionId, never creates a replacement session.

`job.continue_target(index)` acts only on that owner's waiting or explicitly
approved entry. It continues its stored cursor/phase; completion can create
its session only if creation has not yet been attempted. Before ANY mutation
await the worker atomically replaces that continuation with an in-flight
record. A dropped response cannot restore the former waiting continuation.

### One job-bound input and partial sending

`job.start(input)` validates then binds common input ONCE and dispatches
only currently Ready entries. It does not consume the job or any pending
entry. The returned snapshot still contains WaitingSetup/WaitingReady/NeedsApproval with their
original continuations. A second start cannot replace input or resend a
Started/Uncertain target; it returns AlreadyBound. Once input is bound,
continuing a pending target to Ready validates its capabilities, copies that
same input and dispatches it once. A target that later fails validation does
not change another selection's Started outcome.

Common input is optional existing PromptText (16 KiB) and at most sixteen
finished attachment references, with the ordinary per-file/media and total
100 MiB session bounds. Text absent requires attachments. Upload local files
once to a Ready source session through the ordinary upload calls. At first
binding only, that source must be one of this job's Ready targets on this
destination. Validate common input against currently Ready sessions before
any prompt; normal node guards and lower advertised limits still apply.
The worker owns the candidate during those reads even if the method's reply
future is dropped. A shared validation failure/deadline returns InputRejected
with no prompt and leaves input unbound, so an explicit corrected start can
validate again using the same resources. Once Bound, input is never replaced.
If no eligible source exists, attachment input stays unbound and no prompt
is sent; the caller retains its draft while preparation continues.

The job then owns an immutable CommonSnapshot: destination, optional text,
source SessionId and finished attachment references/metadata. It is no longer
conditional on the source target remaining Ready. It stays valid when that
target is Started or ended, so a later-ready selection reads the ORIGINAL
stored snapshot, not new local files, a new draft, or a re-upload under the
same filename. Normal attachment reads verify recorded size/SHA-256 and file
identity. Explicit deletion, retention or modification of the source fails
remaining copies with SourceUnavailable/Changed; there is no silent substitute
or resend of a successful target. No new storage lease or retention privilege
is promised. Text-only input needs no source session.

### Attachment deadlines, progress and uncertain effects

Copy snapshots through attachment.read/begin/write/finish, with at most two
active selections and one <=1 MiB byte chunk per active copy, sliced to the
destination's advertised write limit. Never buffer a complete file. Check
source totals, expected size, empty/oversized chunks, accepted offsets and
finished reference metadata. A mismatch is terminal for that target; an
incomplete copy can never proceed to its prompt.

Every attachment.read has a 45-second response deadline. Every begin,
write and finish has a 120-second response deadline. A target's whole copy
phase, across all common files, has a thirty-minute budget. Between begin
acknowledgement and its last write acknowledgement, 120 seconds without a
positive increase of ACKNOWLEDGED destination bytes ends that copy. A
successful read alone is not accepted-byte progress. Begin and finish are
bounded separately and remain inside the whole-copy budget. Expiry of the
node's idle upload is not a client RPC deadline.

Before sending, publish one typed in-flight record containing SelectionIndex,
destination, branch, known WorktreeId/SessionId, source SessionId/AttachmentId,
attachment index, phase, known UploadId, last acknowledged offset and attempted
byte range. Missing identities are explicit Option values, never guessed.
After a valid response the same worker records the acknowledgement before
issuing another operation. The outcomes are:

| Boundary that stops | Required outcome and retained recovery data |
| --- | --- |
| Source read stalled/failed, no mutation in flight | FailedCopy(Read), source offset and all known destination upload/resources; no more writes or prompt |
| Begin reply lost/deadline/cancel after admission | UncertainCopy(Begin), session/worktree/source/phase, UploadId unknown when not received; no second begin |
| Write reply lost/deadline/cancel after admission | UncertainCopy(Write), known UploadId, acknowledged offset and attempted range; do not infer acceptance or resend |
| Finish reply lost/deadline/cancel after admission | UncertainCopy(Finish), known UploadId and offsets, finished AttachmentId unknown when not received; no prompt |
| Copy/no-progress budget ends between calls | FailedCopy with exact phase/resources and acknowledged offset; no mutation replay or prompt |
| Copy/no-progress budget ends during a mutation | The corresponding UncertainCopy state above |

Disconnect or an unexpected reply likewise makes an admitted mutation
uncertain. A local definitive NotInProtocol/RequestIdsExhausted refusal is
unsent; a valid node refusal is recorded as a terminal typed failure with
its phase/resources and does not promise rollback of partial uploads. No
mutation is retried automatically, even though ordinary attachment.write
can acknowledge a repeated piece idempotently. Partial uploads retain their
existing expiry semantics. These terminal copy states expose recovery data,
not an executable upload-resume token. Finite read/copy failure preserves
other targets, including those already Started.

After all attachments of one target are confirmed finished, ordinary
session.prompt Delivery::Start sends the same visible text and bytes with
that session's own attachment ids. At most two dispatches run at once. A
busy target never falls through to queue, steer, interrupt or resume. A lost
prompt response is UncertainPrompt with its known SessionId: inspect/open
that session before any new explicit send.

### Owner lifetime and cancellation

The worker owns all mutable target states, frozen common input and in-flight
records. Methods borrow the sole nonclone ComparisonJob owner and return
passive snapshots; their reply futures do not own the operation. Dropping a
prepare/start/continue response future while keeping the job owner does NOT
abort or restart work. The worker records its eventual result, and repeated
commands see InFlight/Started/Uncertain rather than issuing it twice. The app
keeps this owner in transport state outside the panel through navigation.

Explicit job.cancel, or dropping the owner itself, prevents further phases
and calls. A read-only wait can become Cancelled with its known resources;
an admitted unresolved mutation becomes Uncertain in its recorded phase,
including continued session-create and upload mutations. Already Started
entries remain Started. The worker cannot reconstruct a fresh continuation
from a passive snapshot, an old WaitingSetup entry or an uncertain result.
Cancellation never kills a turn a person independently started, deletes a
worktree, or rolls back another selection. Normal parking, retention and
explicit per-resource kill/archive/delete/removal remain authoritative.

A lost session-create reply, timeout or cancellation after admission retains
its worktree/branch and optional known SessionId, with no create-capable
continuation. Repeated continue_target is refused for that entry. A caller
may inspect ordinary session/worktree lists; a new attempt requires a new
explicit action acknowledging possible duplicate work. This is client-job
ownership, not durable server idempotency. Process termination loses the
executable owner; retained resource reports can aid inspection but must not
be deserialized into automatic mutation retries. No new server registry is
introduced.

### Authority and older peers

Every request uses the same caller and ordinary node guards. Shared
project/git/provider reads need read, creation/uploads/prompt need operate;
read-only or operate-only callers fail preflight without a worktree. Agent
callers retain their existing ownership, depth and resource limits; the
helper adds no tool, grant or alternate principal. Fleet users compose the
same typed calls on the selected destination through the existing route;
no comparison payload or credential is interpreted by a relay.

A node before protocol 10 is refused before mutations; an added-agent
selection needs its existing protocol-11 support. There is no optional new
wire field an old node could drop. Older apps may read and continue the
resulting normal sessions. No generated TypeScript delta is expected.

## Alternatives and why

- New server comparison registry and batch RPC: not present in GOAT's
  behaviour; duplicates creation/setup/prompt guards and requires durable
  cancellation/idempotency/retention policy. Rejected for this feature.
- Several providers in one session: cannot continue/discard independently
  and makes history ambiguous. Rejected.
- One shared worktree: agents would edit each other's files. Rejected.
- Reread each local attachment for each model: concurrent edits could send
  different bytes. Use the existing stored snapshot and chunked reads.
- Full attachment buffering: ordinary files can be 50 MiB and a session
  100 MiB. Chunked copy reuses existing integrity and bounded memory.

The preparation barrier differs from GOAT's single bootstrap because
Vessel uploads belong to a live session rather than an environment asset.
It exposes preparation progress and starts only after common input is
ready; successful targets still survive other preparation/dispatch failures.
No automatic rollback can safely remove worktrees whose setup may have
created files. The 8-target/two-worker/16-KiB-text bounds deliberately reuse
ordinary bounded startup inputs and keep one send from flooding a machine;
normal single-session prompt limits are unchanged.

## CLI, screen and verification

The CLI reads a bounded typed `--targets FILE` manifest, a project/base,
optional prompt and repeatable `--attach FILE`. It prepares, uploads once,
then dispatches and prints every target's status and known ids. The process
retains its job owner until completion or an explicit stop. If it exits with
pending/partial/uncertain work, it cancels the job first and prints the final
non-executable resource report, then exits nonzero. It does not promise an
executable continuation across CLI processes; ordinary resource commands or
a new explicitly acknowledged attempt provide recovery. No successful
session is hidden by another target's error. File arguments are never
executed. Existing session/log/kill/archive/delete/worktree commands remain
the continuation and discard operations.

The app retains the same nonclone job owner and submitted draft by request
identity, starts Ready selections and continues pending ones in that job, follows each returned session through the shared renderer,
and restores only failed selections when that same draft remains unchanged.
A newer draft is never overwritten; uncertainty offers Open session, not
an automatic second send. No screen is implemented by this node slice.

Tests must use real nodes and fake agents: independent models/worktrees at
one pinned base, identical text/file bytes with different attachment ids,
partial failures, unapproved/failed/slow setup, readonly/operate-only and
agent-limit refusal, busy Start-only refusal, cancelled/disconnected calls
without automatic retry, changed/missing attachment source, zero/duplicate/
over-budget inputs and bounded concurrency. Carry GOAT's new-Git-thread,
base, zero-selection and uncertain-submission fixtures. The cited SSH
browser fanout case proves predispatch refusal only, not positive fanout.
Positive delivery/partial-failure evidence must come from new real-node
fake-agent cases.

Additional required boundaries from contract review:

- Actual member/relay setup reads with delegated wait removed, and local
  exhausted wait slots: no-progress yields the same waiting entry; immediate
  continuation sends no RPC. Repeat for readiness/event observation.
- Stalled source byte-read, lost begin/write/finish response and copy/progress
  deadlines: exact phase/upload/offset/resource data remains, no automatic
  resend and no incomplete target prompt. Include cancellation in each
  admitted mutation and a definitive unsent refusal.
- Ready plus WaitingSetup (and NeedsApproval): start Ready once, continue
  the original pending entry, send identical bytes from the same source
  after that source is already Started, and never recreate/resend successes.
- Cancel during continued session-create: retain UncertainCreate and refuse
  another create from its former continuation. Separately drop the method's
  response future while retaining the owner: the original operation must
  settle once. Dropping/cancelling the owner ends further work and keeps
  Started results unchanged; passive snapshots cannot restart it.

Whole root gate is required after implementation; no code or implementation
approval exists for this contract yet.
