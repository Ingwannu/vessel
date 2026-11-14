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

The client has two explicit phases: prepare independent worktrees/sessions,
then send their common prompt and copied attachment snapshots. Typed per-
selection outcomes retain every created resource, including uncertainty.
Never automatically retry a mutating call, silently start without setup,
queue/steer the initial prompt, resume an existing session or delete another
selection's successful work. A failed comparison is not an atomic rollback.

### Bounded contract

- `ComparisonPlan`: one ProjectId, one explicit WorktreeBase and 1–8
  distinct structured targets. One target is valid for retrying a failed
  selection; zero is rejected before any call. A target carries HarnessRef,
  optional ProfileName/ModelId and OptionChoice values, not an arbitrary
  SessionSpec: no terminal, resume, cwd, place or side link. At most 32
  choices and 16 KiB encoded target metadata; at most 128 KiB for the CLI's
  complete typed target manifest. A stable bounded SelectionIndex identifies
  outcomes in input order, independent of asynchronous completion.
- `NodeClient::prepare_comparison(plan)`: shared read-only preflight checks
  the protocol, Git project and selected provider/profile availability.
  Resolve the chosen base once with `git.log` (limit one); all new worktrees
  use that same CommitId, even if the branch advances during the operation.
  Do not copy the project's uncommitted files. Create a new random-named
  branch/worktree for each target through `worktree.create`, with setup Run.
  Follow its existing setup output before creating the ordinary structured
  session in that place. NeedsApproval/Refused/failed setup is an explicit
  outcome; no automatic approval, skip or start-anyway fallback.
- Prepared outcomes are Ready with session/worktree ids, NeedsApproval,
  Failed with phase/resources/reason, WaitingSetup with its same worktree,
  or Uncertain with phase/resources and the generated branch name. There
  are at most two in-flight selections; no unbounded task or output list.
  Individual RPC/readiness waits are bounded. A setup still running after
  ten minutes returns WaitingSetup, not a fabricated failure or new worktree.
  Continuing that typed handle observes the same setup and creates its
  session at most once; it never repeats worktree creation.
- `NodeClient::start_comparison(prepared, input)`: consumes the prepared
  handle once. Input is an optional existing bounded PromptText (16 KiB)
  plus at most 16 finished attachment references from a Ready source
  session in that prepared result. Text absent requires attachments. The
  original input stays with the caller for draft recovery. Validate common
  input against the ready sessions' advertised limits/abilities before any
  prompt; normal node checks remain authoritative.
- Upload the person's files once to the chosen Ready source using normal
  attachment calls. Copy its immutable stored attachment snapshot to the
  other ready sessions with `attachment.read/begin/write/finish`, one piece
  at a time. The store verifies recorded size/SHA-256 and file identity;
  never reopen a mutable local path per model. Check totals, empty/oversized
  chunks and returned metadata; mismatch/changed/missing source is an
  explicit failure. Memory is at most one existing 1 MiB chunk per active
  copy, sliced to the receiving node's chunk limit. Aggregate common input
  remains within the ordinary 100 MiB session/16-attachment limit, and each
  destination enforces its own lower limits. Partial uploads retain the
  existing expiry behaviour; no new cleanup privilege is introduced.
- Each ready target receives the same visible text and attachment bytes,
  with its own session-scoped attachment ids, through ordinary
  `session.prompt` Delivery::Start. At most two dispatches at once. Return
  ordered Started, Failed or Uncertain outcomes, each retaining its
  session/worktree when known. A busy session never falls through to queue,
  steer, interrupt or automatic resume. A lost prompt response is uncertain:
  open that known session and inspect its log before any explicit resend.
- No comparison-wide rollback or destructive cleanup. Every session and
  worktree created is reported and remains governed by normal limits,
  parking, retention and explicit kill/archive/delete/removal. In particular
  a cleanup must not kill a turn a person independently started in a
  prepared session. Navigation must not abort/restart the owner's job;
  app transport retains it outside the panel, as GOAT retains route state.
  Process termination can leave uncertainty; this is not durable server
  idempotency. Branch/worktree/session identifiers are the recovery trail.

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
then dispatches and prints every target's status and known ids. Partial or
uncertain results exit nonzero after printing all outcomes; no successful
session is hidden by another target's error. File arguments are never
executed. Existing session/log/kill/archive/delete/worktree commands remain
the continuation and discard operations.

The app calls the same two phases, retains the job and submitted draft by
request identity, follows each returned session through the shared renderer,
and restores only failed selections when that same draft remains unchanged.
A newer draft is never overwritten; uncertainty offers Open session, not
an automatic second send. No screen is implemented by this node slice.

Tests must use real nodes and fake agents: independent models/worktrees at
one pinned base, identical text/file bytes with different attachment ids,
partial failures, unapproved/failed/slow setup, readonly/operate-only and
agent-limit refusal, busy Start-only refusal, cancelled/disconnected calls
without automatic retry, changed/missing attachment source, zero/duplicate/
over-budget inputs and bounded concurrency. Carry GOAT's new-Git-thread,
base, zero-selection and uncertain-submission fixtures. Whole root gate is
required after implementation; no code or implementation approval exists
for this contract yet.
