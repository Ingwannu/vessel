# Milestone 12: session extras (N5)

Status: reference and contract pass, 2026-11-13. No implementation claimed.
This track takes each row through node, wire, CLI, tests, link, view and
screen before starting the next. Node and app slices are reviewed separately.

## Reference acceptance lists

Paths below are relative to each public reference repository. References
are read for behaviour; their implementation is not copied.

### 1. A side question

GOAT `apps/web/src/components/sideChat/SideChatSurface.tsx`,
`sideChat.logic.ts`, `sideChat.logic.test.ts`, and
`apps/server/src/orchestration-v2/SideChatParentContext.ts`:

1. Open a normal thread; send `/side` or `/side <question>`.
2. A side-chat tab opens beside the main conversation. Its header names
   the parent, shows its current status, and explains that nothing said
   here is added to the main thread.
3. Reuse the newest unarchived side chat, or start one with the parent's
   model and working directory. GOAT first tries a completed-turn fork;
   if no anchor is available it starts fresh.
4. Send a question. The server adds a bounded snapshot of the parent's
   latest messages, status, and plan to each side prompt. The original
   running turn continues, its conversation and context untouched.
5. Continue the side conversation, open it as a full thread, or start
   over (delete the old side and create another).
6. Starting and failed states appear in the side panel; a failure offers
   Try again. A side chat cannot itself open another side chat.

Agent-orchestrator has independent side chats under Developer mode:
`backend/internal/service/chat/side.go` and
`frontend/src/renderer/components/chat/IndependentSideChats.tsx`.
Paseo has no equivalent feature in its inventory or session/composer
sources. The screen follows GOAT, with existing Vessel kit controls.

### 2. One prompt, several models

GOAT `apps/web/src/components/chat/useMultipleModelSend.ts` and
`chatViewSend.ts`:

1. Choose a Git project and base branch, write one prompt, and shift-click
   several models in the picker.
2. Send once. Each model starts its own thread and worktree with the same
   submitted prompt and attachments; each result can be opened.
3. Successful starts remain running if another model fails. Failed
   selections stay selected and the prompt is restored, or a Restore
   prompt toast appears after navigation. Continue or drop each thread.

The explicit comparison surface requested for Vessel must reuse those
threads, not mix several providers into one conversation. Its precise
layout is recorded with ADR 0046 after row 1 is reviewed.

### 3. One session, conversation or terminal

Agent-orchestrator `frontend/src/renderer/components/SessionInterfaceSwitch.tsx`
and `backend/internal/session_manager/interface_transition.go`:

1. In a supported session choose Chat or Terminal.
2. If work is running choose finish then switch, stop and switch, or
   terminate and switch.
3. Keep the session and worktree identity, resume the native conversation
   in the other interface, and roll back if the new interface fails.

GOAT and Paseo do not offer that same-session transition. Vessel's existing
terminal renderer remains the conversation track's; this track supplies
the node operation and pure view state, then gives that track the call.

### 4. Delivered files and HTML

Agent-orchestrator `backend/internal/service/session/artifact_registration.go`,
`frontend/src/renderer/components/ArtifactsView.tsx`, `SessionArtifactsView.tsx`:

1. An agent registers a file result (`ao artifact add`).
2. Open the session's artifacts gallery, inspect a file, open/share it,
   and view or restore a previous version.
3. Render an HTML result (`ao render`, HTML render/preview tools) inline.

GOAT `apps/server/src/htmlRender/HtmlRender.ts` renders HTML replies as
sandboxed pages. Vessel's native HTML boundary is decided in ADR 0047
before implementing a viewer; no delivered script gets app privileges.

### 5. A reviewing agent

Agent-orchestrator `backend/internal/ports/reviewer.go` and
`backend/internal/adapters/reviewer/registry.go` supply a separate reviewer
harness. A person chooses a reviewer, starts review on a change, then reads
its findings and comments. GOAT's opt-in
`PullRequestReviewGuideService.ts` is a guide rather than general PR-agent
review. Exact PR actions and source files are traced before this row is
implemented; no parity claim is made from the registry alone.

### 6. Spread sessions by load

GOAT `apps/web/src/hooks/useLoadBalancedEnvironment.ts`,
`components/settings/LoadBalancingSettings.tsx`, and
`packages/client-runtime`'s load-balancing model:

1. Configure each environment as Prefer, Less often, or Manual.
2. Start a thread with automatic placement; the client requests resources
   for eligible machines and chooses using fresh samples (15 seconds).
3. A stale or failed sample is not silently treated as idle; choose a
   machine explicitly if automatic placement cannot choose.
4. Explicit machine selection remains available. Limits, pressure and
   draining are traced in the server before defining Vessel's score.

## Row 1: contract (ADR 0045)

Decision: extend existing session creation, log streaming, prompting,
permissions and deletion, with a durable parent link in the stored spec.
No dependency or separate agent transport is needed. The stored spec
already preserves the default-absent parent link. Schema 13 adds a bounded
latest-plan projection, updated transactionally with the plan event, so
tool-heavy conversations do not lose their plan outside the transcript tail.

- `SessionSpec.side_question_of: Option<SessionId>`: additive parent link,
  absent normally. A side is always a new structured harness conversation,
  never a resume of the parent's native session id. No nested sides.
- `session.side_open { session: SessionId } -> SessionInfo` (`operate`,
  protocol 12): open/reuse the newest unarchived side (including an ended or interrupted one) of this parent.
  Creation is serialized so concurrent clients do not launch duplicates.
  Inherit current profile, model, choices, place and cwd. Never change the
  parent or interrupt its turn. An ended structured parent is allowed.
- `session.side_reset { session: SessionId, side: SessionId } -> SessionInfo`
  (`operate` plus read, protocol 12): under the same creation gate, verify
  the expected side belongs to the parent and is still the selected side;
  stop/delete it, then open a replacement. A stale expected side is
  conflict, so a late reset never deletes a newer side. Failed deletion
  creates no replacement; retry/open resolves the resulting state. If the
  old side was deleted and replacement creation fails, the old side stays
  deleted and side_open retries creation; no rollback is promised.
- `session.side_prompt { session: SessionId, question: PromptText } -> Delivered`
  (`operate`, protocol 12): the named session must be a side. Read its
  parent's latest state and at most 128 trailing records, keep at most
  12 message fragments of 1,200 characters and 9,000 characters total.
  Independently read current status, attention and the latest projected
  plan (20 steps maximum); include current branch/place when available.
  Add quoted context and start a side turn (refuse if busy).
  Question text uses the existing nonblank 16 KiB bounded PromptText.
- Both operations are for people (owner/devices, including delegated
  people with intersected scopes); agent principals are refused. Both
  need read as well as operate because they read parent conversation.
  Read-only devices can read existing sides using normal session reads,
  but cannot create, ask, clear or answer an approval.
- Normal creation with a side link follows the same parent validation:
  structured mode, no resume, same inherited configuration and current
  profile. It goes through the same serialized open/reuse operation.
  Caller-supplied harness/profile/mode/resume/choices cannot override it.
  It cannot manufacture a link to a terminal or another side. Schedules
  and saved prompt templates refuse side specs; imports clear the local
  parent link. All calls accepting a nested side spec require protocol 12.
- General session.send Prompt, session.prompt (including tracked/told
  prompts and scheduled targets), and queue changes are refused for sides
  with a typed unsupported error naming session.side_prompt. This check
  lives in Sessions, beneath the service and all automation ingress.
  Dedicated side_prompt is the only prompt ingress and does not queue.
  This avoids freezing context while a question waits in a queue.
- Read, subscribe, approvals/questions, cancel, resume and delete use the
  existing side SessionId. Reset is the dedicated serialized side_reset operation; normal deletion
  still works independently. Neither side input nor side output is ever copied to parent.
- Errors: `not_found` for a missing parent/side; `unsupported` for a
  terminal parent or nested side; `invalid_request` for a non-side prompt
  or invalid question; `forbidden` for insufficient scopes or any agent;
  `conflict` for a busy side or existing lifecycle conflict; ordinary
  create/start errors retain their existing typed wire codes.
- Fleet forwarding is allowed through the usual scope guard and encrypted
  relay. Parent snapshots and side replies are application data only;
  the relay gains no interpretation or new authority.
- One bump from protocol 11 to 12 for this entire track; minimum version
  unchanged. Older apps ignore the optional spec field and can still open
  a side as a normal conversation for reading, cancellation and approval;
  ordinary prompts are refused with instructions to update or use the CLI.
  Older nodes reject the new calls;
  a call using the spec field also requires protocol 12.
- CLI: `vessel session side SESSION [QUESTION]` opens/reuses a side and
  optionally asks; prints its id for `attach`, `kill`, or delete. Use this
  same side command with the side id for every later question, not send.
  A repeated call with a side id asks in that side (refreshing context).
- App: `link::side_questions` uses typed calls; `view::side_questions`
  owns states/status/command parsing, tested with GOAT's cases; its own
  `areas/side_questions` panel follows GOAT's surface, with bilingual
  words and a single conversation-owned registration seam.

## Necessary differences and limits

Vessel's base has no completed-turn fork call yet. Like GOAT's fallback,
row 1 starts a fresh native conversation and seeds a bounded live snapshot.
It must say it has recent context, not claim a complete history fork.
The transcript is a partial tail, labelled as such. Current plan, status
and attention are independent projections, never guessed from that tail.
Vessel has no goal projection: no goal is claimed or fabricated. Plans
from before the schema upgrade are unavailable until the agent next
updates them; the snapshot explicitly says when no plan was recorded. Side questions are isolated conversation
context, not an operating-system sandbox: the side agent has the same
normal project tools/permissions as another session started there.

## Row 1: lifecycle and byte bounds

- Open is serialized with ordinary creation naming a side link. Reset uses
  side_reset with the expected old id and the same gate: stop and delete
  the old side before replacement. If a
  concurrent open retained a now-deleted id, its read returns not_found;
  it can open again. Only one unarchived side exists after either order.
- Ended/interrupted sides remain discoverable, like GOAT's newest
  undeleted side. Opening never silently starts another paid process.
  Explicit resume uses the existing lifecycle and preserves the side id;
  it works only while the parent exists and after scope/person checks.
- Parent deletion or retention never removes the side's existing log.
  It stays readable, but new questions/resume fail not_found and explain
  that the parent is gone. Reset also cannot create without that parent.
- A parent's profile/model/choices change does not restart an existing
  side. Start over inherits the new current configuration. Restart leaves
  the link intact and sides interrupted; the same explicit-resume rule
  applies. Export/import cannot move a node-local parent link elsewhere.
- Snapshot is at most 24 KiB in UTF-8, INCLUDING metadata, role labels,
  separators and truncation notice. Append on character boundaries.
  Question is at most 16 KiB; the complete content is capped at 48 KiB,
  including envelope instructions and markers (below the existing 128 KiB
  conversation prompt budget). A huge individual event cannot enlarge it.
- Plan persistence is at most 20 entries, each clipped to 1,200 characters.
  Snapshot's total byte cap can clip that further, with an explicit marker.
  Read/plan refresh waits for the observed parent's records to be durable;
  later parent changes are naturally a later snapshot, not a false atomic
  claim. A failed or busy side never mutates the parent log.

## Contract review (2026-11-13)

V1 returned five findings before implementation: generic prompt bypasses,
plan loss outside a raw tail, ordinary-create races and nested-version
checks, missing lifecycle decisions, and unspecified UTF-8 total bounds.
All five are accepted above. Tests must cover those boundaries with fake
agents, including delegated-agent refusal, read-only and operate-only
principals, concurrent creation/reset, a plan followed by more than 128
tool events, multibyte text, huge events, parent deletion and restart.
No implementation approval is implied by this contract review.

## Verification and review

Contract-only baseline, before any source changes:

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed,
  finished in 3m 04s.
- `cargo test --workspace`: failed. Cold compilation took 10m 16s;
  CLI integration tests ended with 35 passed and 1 failed (63.11s):
  `canary::no_secret_value_appears_where_it_does_not_belong` hit its
  existing 60-second safety timeout. The workspace after that test binary
  did not run. This is not a clean whole-workspace result.
- The coordinator permitted this docs-only contract commit with the
  baseline failure recorded, and requested one isolated rerun of that
  exact test. Its result is recorded with the implementation slice. No
  timeout was raised and no test was ignored.
- V1 read both contract revisions against GOAT and passed the final
  contract only, independently verifying both document hashes. Code,
  testkit evidence and the app remain unreviewed.

No feature is reported complete. Captures and gate log paths are supplied
in review messages, outside the public repository.
