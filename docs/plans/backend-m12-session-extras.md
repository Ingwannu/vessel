# Milestone 12: session extras (N5)

Status: row 1 node implementation and integration gates passed, 2026-11-14;
V1 reported implementation PASS for 85c60716. Merge is not claimed. The results under resumed integration
below supersede the old checkpoint's unverified wire/web status.
This session owns node, wire, client, CLI, testkit and contracts only.
Native preparation below remains historical WIP for the app owners; no
native screen or page work is undertaken by this session.

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
already preserves the default-absent parent link. Schema 14 adds a bounded
latest-plan projection, updated transactionally with the plan event, so
tool-heavy conversations do not lose their plan outside the transcript tail.

- `SessionSpec.side_question_of: Option<SessionId>`: additive parent link,
  absent normally. A side is always a new structured harness conversation,
  never a resume of the parent's native session id. No nested sides.
- `session.side_open { session: SessionId } -> SessionInfo` (`operate`,
  protocol 19): open/reuse the newest unarchived side (including an ended or interrupted one) of this parent.
  Creation is serialized so concurrent clients do not launch duplicates.
  Inherit current profile, model, choices, place and cwd. Never change the
  parent or interrupt its turn. An ended structured parent is allowed.
- `session.side_reset { session: SessionId, side: SessionId } -> SessionInfo`
  (`operate` plus read, protocol 19): under the same creation gate, verify
  the expected side belongs to the parent and is still the selected side;
  stop/delete it, then open a replacement. A stale expected side is
  conflict, so a late reset never deletes a newer side. Failed deletion
  creates no replacement; retry/open resolves the resulting state. If the
  old side was deleted and replacement creation fails, the old side stays
  deleted and side_open retries creation; no rollback is promised.
- `session.side_prompt { session: SessionId, question: PromptText } -> Delivered`
  (`operate`, protocol 19): the named session must be a side. Read its
  parent's latest state and at most 128 trailing records, keep at most
  12 message fragments of 1,200 characters and 9,000 characters total.
  Independently read current status, attention and the latest projected
  plan (20 steps maximum); include current branch/place when available.
  Add quoted context and start a side turn (refuse if busy).
  Question text uses the existing nonblank 16 KiB bounded PromptText.
- All three operations are for people (owner/devices, including delegated
  people with intersected scopes); agent principals are refused. All three
  need read as well as operate because they read parent conversation.
  Read-only devices can read existing sides using normal session reads,
  but cannot create, ask, clear or answer an approval.
- Normal creation with a side link follows the same parent validation:
  structured mode, no resume, same inherited configuration and current
  profile. It goes through the same serialized open/reuse operation.
  Caller-supplied harness/profile/mode/resume/choices cannot override it.
  It cannot manufacture a link to a terminal or another side. Schedules
  and saved prompt templates refuse side specs; imports clear the local
  parent link. All calls accepting a nested side spec require protocol 19.
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
- One bump from protocol 18 to 19 for this entire track; minimum version
  unchanged. Older apps ignore the optional spec field and can still open
  a side as a normal conversation for reading, cancellation and approval;
  ordinary prompts are refused with instructions to update or use the CLI.
  Older nodes reject the new calls;
  a call using the spec field also requires protocol 19.
- CLI: `vessel session side SESSION [QUESTION]` opens/reuses a side and
  optionally asks; prints its id for `attach`, `kill`, or delete. Use this
  same side command with the side id for every later question, not send.
  A repeated call with a side id asks in that side (refreshing context).
- App: `link::side_question` uses typed calls; `view::side_question`
  owns states/status/command parsing, tested with GOAT's cases; its own
  `areas/side_question` panel follows GOAT's surface, with bilingual
  words and a single conversation-owned registration seam.

## Necessary differences and limits

Row 1 uses GOAT's fresh-thread fallback and seeds a bounded live snapshot.
The integrated base now has session.fork, but registers a failed provider
fork as a session before its result is known. Transparent fallback would
need a second lifecycle transaction with retention/delete/concurrent open;
ADR 0045 records why this slice keeps fresh-start semantics explicit.
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
  it can open again. With no archive changes, one unarchived side exists after either order.
  A person may restore an older archived side; discovery still chooses the
  newest. Reset always creates fresh state and never resurrects that older
  side as the replacement.
- Ended/interrupted sides remain discoverable, like GOAT's newest
  undeleted side. Opening never silently starts another paid process.
  Explicit resume uses the existing lifecycle and preserves the side id;
  it works only while the parent exists and after scope/person checks.
- Removing a parent does not cascade-delete a side's log. Each side still
  has normal independent retention/deletion rules. A retained side stays
  readable, but new questions/resume fail not_found when its parent is
  gone. Reset also cannot create without that parent.
- A parent's profile/model/choices change does not restart an existing
  side. Start over inherits the new current configuration. Restart leaves
  the link intact and sides interrupted; the same explicit-resume rule
  applies. Export/import cannot move a node-local parent link elsewhere.
- Snapshot is at most 24 KiB in UTF-8, INCLUDING metadata, role labels,
  separators and truncation notice. Append on character boundaries.
  Question is at most 16 KiB; the complete content is capped at 48 KiB,
  including serialized request envelope, JSON escaping, instructions and
  markers (below the existing 128 KiB conversation prompt budget). A huge individual event cannot enlarge it.
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

## Row 1: implementation refinement before review

The side control carries the visible PromptText and a bounded transient
PromptContext together. AgentInput sends the prefix to ACP but emits
TurnStarted with the question alone; ordinary prompts and attachments use
the existing path. Context cannot remain for another command on refusal or
cancellation. The final serialized ACP request, including escaping and
request envelope, is checked against 48 KiB before sending. Native slash
commands are refused explicitly instead of being moved behind the prefix
(GOAT SideChatNote bypasses its prefix for those commands). The existing
restart/profile-move replay boundary drops generated UserMessageChunk until
Ready. A new native import is an ordinary session and may replay the
provider-stored context.

The fake agent has an opt-in load-replay fixture, FAKE_ACP_REPLAY_USER.
Tests distinguish question-only persisted events from exact provider input,
and distinguish an agent echoing its input from a generated user message.
The baseline exact canary rerun passed: 1 passed, 0 failed, 35 filtered out,
12.96 seconds; its feature-specific cold compilation took 9m 08s. A prior
attempt was interrupted before tests ran after edits were discovered during
compilation; changes were set aside and the completed isolated run used
the contract commit's unchanged tracked source. Neither result is a clean
whole-workspace gate for this implementation.

Native session.export transfers provider files, not Vessel's event log.
Generated context can be persisted by the provider in those files and
carried by native export/import; those files stay opaque and unchanged.
Question-only user messages describe Vessel's TurnStarted/read/subscribe
and CLI event output. This is not a context-free native-export guarantee.

New side titles follow GOAT's `Side chat: {parent title}`. Vessel uses its
existing automatic-title constructor, so the derived single-line title is
bounded to 80 characters; an untitled parent uses Untitled. The app's
panel labels are translated separately from this persisted session title.

The 9,000-character transcript allowance includes role labels, separators
and clipping marks. Individual messages keep a complete truncation mark
inside their 1,200-character allowance; a remainder too small for that
mark is skipped. Large events are read under the store's existing page
allowance and clipped without walking their complete text.

Call::since also requires protocol 19 for fleet.forward/relay/deliver
carrying these calls, including side-linked session.create and templates.
Otherwise an older intermediary could discard the optional parent link
before a newer target saw it. Direct and wrapped forms share version tests;
old-hop routing for existing pre-19 calls stays unchanged.

## Row 1: app integration still pending

The panel discovers an existing side through the normal read-only session
list before asking side_open to create one. This preserves a read-only
device's ability to read an existing side without making an operate call.
Creation, questions, reset and explicit resume still use their guarded
paths. The panel has its own conversation state and uses the shared
conversation row renderer; it does not copy tool or approval rendering.

The composer intercept must cover both /side in an ordinary parent and
ordinary input when a full side session is shown. The latter uses the
same dedicated Start-only ingress; it must never queue, steer or resume
implicitly. Unknown session metadata holds input until the target is
known, and refusals keep the draft. These view rules and their English and
Korean words are prepared; the transport, drawing, fake-fleet captures and
native whole-workspace gate are still outstanding.

## Integration verification update

The first implementation gate passed root formatting and whole-workspace
clippy. Tests passed all 37 CLI and 187 node unit cases, then stopped at
152/154 node end-to-end cases; later workspace tests did not run. One
new exact fake-provider expectation used a newline where the fake joins
content blocks with a space. That expectation is corrected without
weakening the complete payload assertion. Native command refusal also
recognizes the leading BOM that JavaScript trimStart accepts.

The existing reinstalled-member test failed again in its one permitted
isolated run (24.95 seconds). This is not a load-only pass: the integrator
identified the restart timestamp race and supplied bce3dd9. The report may
precede the timestamp captured after restart; its grant refusal is expected.
The test is left unchanged here and the lead integration supplies that fix.
A clean whole gate on the integrated tree remains required.

The lead integration at 74dfde8 supplies protocol 12's search and parking
and the existing test race fixes. This track now makes its single bump
from 12 to 13; schema 14 appends the side-plan projection after the search
index migration. A side prompt wakes a live parked side through the same
actor wake path as an ordinary prompt, with load replay suppressed. A
failed wake fails that command without retaining its generated context.
Explicit resume is still required when the side actually ended.

The app preparation also reuses machine_book::Edit and Name. Its local
target transition preserves the parent when a successful rename moves the
book entry and closes only that machine's local panel on forget. The
actual post-success app hook and its Machines registration are not built
yet; a failed edit must invoke no hook. Separate side reads carry machine,
parent, side and non-reused request identities, so old answers cannot
restore a panel after navigation, reset or a name change. Transport and
view preparation are not yet registered or compiled as an app slice.

The first integrated whole test completed 66 test binaries with 1,447
passed, one failed and eight pre-existing ignored tests. It stopped at
the approvals migration fixture: simulating schema 9 left latest_plan in
the database, so migration 14 correctly refused its duplicate column. All
rewind fixtures now remove this column before replaying old migrations.
The later workspace tests were not run in that attempt. Node unit tests
187/187 and end-to-end tests 159/159 passed in that run, including the
known reinstalled-member race fix; this is not the final clean gate.

Self-review also corrects two reference boundaries before the next gate.
Transcript messages are selected newest-first under both character and
remaining UTF-8 byte budgets, then rendered chronologically; otherwise
the total-byte cap could discard the latest multibyte messages. A stored
placed parent already carries a resolved cwd: a side clears that derived
cwd and resolves the inherited place through ordinary creation checks.
Tests cover a project subdirectory and a worktree through open/reset,
unchanged parent logs, and preservation of the parent's worktree.

## Credit-window checkpoint: resume here

The cleanup instruction permits finishing the current node slice only.
No additional branch merge, app slice or kit gate is started during cleanup.
Rows 2 through 6 have reference notes only; row 1 is not a finished vertical
feature until the native screen and review are complete.

Committed before this checkpoint:

- 0633dc3: contract documentation, passed by V1 for the contract only.
- 117deb38: integration of the coordinator-specified lead commit 74dfde8e,
  including search/parking, protocol 12 and the known test race fixes.
- a44ab316: node-only WIP checkpoint, with the passing root gate below;
  TypeScript/web verification was not complete at that commit.
- e9645adf: control-only WIP checkpoint of inline_status. Native whole gate
  and captures are unrun; no control implementation approval or adoption.
- c709a82d: pre-existing link/view/text preparation checkpoint. Unregistered,
  uncompiled and untested as a native feature; no screen or app hooks.
- ed788696: generated protocol declarations and verification checkpoint.
  Declaration tests and web lint/typecheck/417 tests/bundle build passed;
  fresh bundled Rust binaries and web E2E remain unrun.

The node implementation is checkpointed separately and remains WIP. Its source pin
covers 69 crate files. Root formatting and whole clippy passed (3m 03s).
The final whole workspace test passed: 1,691 passed, zero failed and ten
pre-existing ignored tests across 109 test binaries/doc-test sections.
The 7m 39s compilation rebuilt the changed crates; source hashes stayed
unchanged through the run. Node unit tests passed 188/188 and end-to-end
cases 160/160, including multibyte newest-message selection, placed-parent
open/reset, failed wake and replay isolation. The earlier integrated
attempt's old-schema fixture failure remains recorded separately.
TypeScript generation passed both declaration tests after the node WIP
checkpoint. On those declarations, web lint, typecheck, all 417 tests in
81 files (maxWorkers=2) and the production bundle build passed. The bundle
is a local development artifact, not a release or deployment.

NOT RUN: embedding that bundle into fresh Rust CLI/node binaries and web
E2E. The TypeScript command alone waited 616 seconds for a build slot;
cleanup time did not allow another such queue plus build and E2E. The
backend and generated-wire commits therefore stay WIP and are not ready
to merge. Do not label the web gate complete. Generated proto.ts retains
the generator's canonical line-end whitespace (already present in the
baseline); do not hand-edit it away from the declaration check. V1 has the contract pass only; implementation review is pending. Resume
notification carries the WIP hashes and logs for its queue, without asking
for a new review slice during cleanup. Once all remaining checks pass, V1
reviews the immutable node diff and routes it to the coordinator and node
integrator. No code approval is implied by
its earlier contract approval.

Native preparation remains separate and unverified: inline_status in kit,
side_question in view and link, and bilingual side_question text. No side
screen exists, the transport is not registered, and no native gate or
capture has run. The control's reservation is not a review pass. The link
preparation expects its Command/Event/State registrations; it must not be
advertised as usable until those are connected and tested. Request and
stream results must match machine, parent, side and non-reused ticket.
The view uses a caught-up timeline instead of stale session metadata for
submission state. None of these preparation changes belongs in the node
implementation commit.

Other tracks' pending seams:

- Lead: side_question::slot and side_question::intercept are reserved,
  but registration waits for reviewed N5 functions. Intercept covers both
  /side in an ordinary parent and normal text in a full side session.
  Ended sides require explicit resume; the full-session composer must not
  claim that sending implicitly resumes one.
- C: shared timeline Seat and request-card Place seams were supplied in
  3d712a0e and 5360d38f. They are not in the integrated baseline above.
  C later reported e8f49fec: Shell::acted hands answers only to the main
  ConversationArea. The lead must add routing to the side area's answered
  method; Followup::ReadWaiting must ask Act::Conversation at its own seat.
  Read the latest C and lead source on resumption rather than applying
  these older call shapes blindly. Do not copy the renderer.
- Lead later supplied 1b517995: Link::submit returns Result<(), Unsent>
  with Rejected::Full/Closed and the original command, while send remains.
  Shell::command settles a rejected command on the next update through
  crate::unsent::settle. Registering the new side Command also needs an
  exhaustive settle arm: answer a pending key with its event, otherwise
  use Settle::Say. This was not merged during cleanup.
- E2: a post-success book-edit call is reserved. Supply one pub(crate)
  hook taking machine_book::Edit/Name, plus a gated hash. Rename moves the
  current conversation and target and invalidates or moves old request
  identities. Forget clears local state and cannot allow late replies to
  reopen it; it must never stop/delete remote work. Failed edits call
  nothing and MakeDefault is a no-op. Pure target cases are prepared;
  the actual app hook does not exist yet.
- V1: inline dot/label control reserved to N5, using existing dot6,
  11/16 normal text, gap4, and separate semantic text/dot roles. No capsule.
  E2's typed dot6/8 work keeps the existing dot6 call. Control-only review
  and real long-header captures in both sizes, languages and appearances
  remain outstanding.

Next: build the Rust CLI/node/testkit binaries with the verified web bundle
required and run web E2E. Do not rerun the passed root gate on unchanged
Rust source. If source fixes are needed, run the gate on that changed tree.
Then submit the completed immutable node slice for implementation review.
After the node slice, on resumption integrate the shared conversation seams, gate/review the
control-only change, register the side transport and view, implement the
screen and the book-edit hook, run the native whole gate and fake-fleet
captures, inspect every capture and report the app slice to V1. Start no
second feature before row 1's complete pass is reported.

No fake fleet or virtual display was started by this track. Tests used
isolated fake agents. Cleanup found no remaining build/test/fake processes
under this worktree or its own target directory; no process needed a stop.
The preview node, services, permissions and account configuration were not
changed. No repository push or release build was performed. Local web/dist
and web/node_modules are untracked artifact symlinks and must not be added;
keep both local cargo configurations and the supplied fonts uncommitted.


## Node-only resumption (2026-11-14)

The resumed assignment supersedes the earlier cleanup stop and the native
integration next steps above. Integrate the protocol-18 node line first;
side methods and side-linked specs move together to provisional protocol 19.
MIN_PROTOCOL_VERSION remains 2; the plan projection remains schema 14.
There are three new calls (side_open, side_prompt, side_reset), one optional
SessionSpec field (side_question_of), no new public events or scopes, and
no provider-native history rewrite. New scopes elsewhere on the merged line
are outside this feature. All older hops must preserve this version gate.

Merge compatibility keeps the creation lease alive across the same shared
fork-capable launch path. Saved-template byte limits and side-spec refusals
both apply. New fake replay message ids coexist with the context-replay
fixture. The new test-script rule uses Sandbox::executable; this slice's
side fixtures write data/configuration only, not executable scripts.

A parent rewind clears the latest-plan projection in the same append and
bounds transcript selection to messages after that rewind. Valid earlier
turns are also omitted; the snapshot explicitly claims partial context.
Generic fork of a side is unsupported: use side_reset to start over.
Regressions exercise the actual fake provider request and real SQLite,
including a plan replacement in the same append as a rewind.

The native app work belongs to the app workers now. A screen needs to call
side_open with the parent's id, then read/follow that returned id and send
every question through side_prompt (Start only; no queue/steer/auto-resume).
An ended side needs explicit session.resume. Reset calls side_reset with
parent and expected side id, and replaces its local target only on success.
Read/operate refusals and provider failures must remain visible. Native
export/import can contain provider-stored context; Vessel's user-event log
contains only the question. The unregistered transport/view WIP is reference
material, not a compiled or approved dependency for those screens.

Integration verification will be recorded separately from the old checkpoint.
The first targeted run stopped at compilation: its new store regression used
FromStr for HarnessSessionId, an opaque provider id constructed with new.
That test fixture is corrected; no production identifier parser changed.
The repository's existing Playwright pairing-dialog regression is explicitly
permitted for its own isolated headless browser. It is run unchanged, with
trace, screenshots and video disabled; synthetic keyboard input into a shared
display remains forbidden. Installing web dependencies replaced the local
node_modules symlink; its contents were moved to the build volume with the
operator's compare-before-removal helper, then the symlink restored. No
service account, mode or runtime configuration was changed.

The unverified native preparation from e9645adf and c709a82d is removed from
the resumed node delivery tree, without deleting those historical commits.
App owners can recover or consult those exact checkpoints independently;
they are not approved dependencies. This removes only N5's prepared status
control and unregistered side transport/view/text, not the lead's integrated
conversation or machine-book changes. No new native control or screen is
implemented or gated in this node-only session.
The dependency cache must retain the final directory name node_modules:
Node resolves package dependencies by that name, even behind the checkout's
symlink. An initial renamed leaf caused a module-resolution preflight error;
putting it under a private install directory with the canonical leaf fixed
resolution. A full-browser existence preflight initially missed the already
cached headless browser. A redundant full-browser download timed out; its
own verified installer process was stopped after the exact pinned headless
browser was found and successfully launched. E2E uses that existing matching
browser unchanged, with no alternate executable or test configuration.

The first node rewind regression then failed on its fixture's missing turn
anchor: the fake's permission-tool turn intentionally has no message id.
It now uses the existing plain completed-message fixture used by the fork
suite; the whole provider-request assertion and production checks remain.
The store's two targeted tests passed and are not repeated unchanged.
The corrected real-node rewind regression passed: one passed, zero failed,
184 filtered out. Together with the two passing real-SQLite regressions it
confirms the exact provider request omits discarded history and plans,
while a generic side fork is refused without changing side discovery.
These are targeted results, not the whole integration gate.

### Resumed integration: root gate

On the fixed protocol-18 integration snapshot plus this feature's provisional
19: root fmt --all --check passed; whole-workspace clippy --all-targets with
-D warnings passed (2m 52s compilation); cargo test --workspace passed cleanly.
The whole run completed 111 test/doc-test sections: 1,797 passed, zero failed,
ten existing ignored. No test was ignored or timeout raised by this slice.
Node unit tests were 216/216, node E2E 185/185, CLI integration 39/39.
The cold test compilation took 9m 49s. Nested fake-node builds ran before
those fixtures' own deadlines; no timeout-only rerun was needed in this run.
The 1,052-file root source manifest remained identical through the gate.
TypeScript generation and the remaining web/bundled-binary/E2E stages are
reported separately below; root success alone is not their approval.

TypeScript declaration generation passed both tests (proto and web).
Against those generated files, web lint and typecheck passed, all 417 tests
in 81 files passed (two workers), and the local web bundle build passed.
The next verification uses VESSEL_WEB_REQUIRE_BUNDLE=1 to build fresh debug
CLI/node/testkit binaries, then runs the unchanged whole Playwright E2E
suite against its own fake fleet. This is not a release or deployment.


### Resumed integration: complete gate and review handoff

The required-bundle debug build passed, recompiling vessel-web with the
newly verified bundle. The unchanged whole Playwright E2E suite passed:
62 passed in 3.3 minutes, against its isolated fake fleet. Traces, automatic
screenshots and video were disabled. All integration-gate stages passed.
No unchanged-tree gate rerun was performed. The root source pin stayed
unchanged through declaration generation, web checks and binary/E2E stages.
The earlier targeted fixture failures above remain distinct from this
clean whole run. Generated declarations retain canonical generator output.

This is a node slice for V1's implementation review, not native/UI approval
or full GOAT parity. Fresh isolated sessions with labelled bounded context
remain a deliberate alternative to GOAT's preferred completed-turn fork;
ADR 0045 records the asynchronous-fork fallback lifecycle tradeoff on the
new base. Slash questions remain explicitly unsupported. Rewinds use the
post-rewind partial tail and invalidate the plan until another report.
Provider-native exports can still carry generated context; no opaque file
is rewritten. A future direct driver must preserve atomic context placement
and byte checks or refuse the command, never silently drop its context.

At that handoff, the next node contract was row 2 (ADR 0046), then delivered
files/HTML (row 4, ADR 0047). The later continuation order below supersedes
that sequence. Row 3's terminal
surface and rows 5/6 are outside this resumed session's assigned work.
The app owners implement the screen using the call guide above; no native
screen, control or page implementation was added here. The old native WIP
checkpoints remain recoverable, but are absent from this delivery tree.

Cleanup after verification found no remaining build/test/fake processes for
this worktree or target directory. No owned fleet or virtual display remains.
No preview node/unit/port, real-provider prompt, service permission, account
configuration, push or release operation was used. All local commands ran
as the checkout's ordinary user, preserving existing ownership and modes.
At that handoff, implementation review was still owed. V1 subsequently
reported implementation PASS for 85c60716; node integration and app/UI
approval are separate and are not claimed by that decision.

## Row 2: independent-model client composition (ADR 0046)

Reference behaviour is confirmed in GOAT's useMultipleModelSend.ts,
chatViewSend.ts, useComposerSend.ts and composerSend.logic.ts/tests (paths
under apps/web/src/components/chat). A person opens a new Git-project draft,
chooses its base branch, shift-selects models, submits one text/file snapshot,
and receives independent background threads/worktrees. Provider validation
and common attachment readiness happen before dispatch. Runtime partial
failure keeps successes; failed selections/draft are restored only if the
original draft is unchanged, otherwise offered through Restore prompt.
An uncertain request retains a thread identity and blocks automatic retry;
Open thread or explicit Allow retry is required. Continue/drop is per thread.
There is no server comparison-room resource in that implementation.

Decision: modify/reuse the existing typed client composition, with bounded
prepare/send phases and per-target outcomes, rather than invent a server
batch registry. ADR 0046 defines types, authority, limits, uncertainty,
attachment snapshot copying, older nodes and tests. There are no new wire
calls/fields/events, schema or protocol changes for this row. A native
comparison surface is the app owners' work and can consume these outcomes.
V1 accepted the corrected contract at ff4cfb29. Local implementation and
whole-gate results are recorded below; V1 implementation review remains
pending. The gate for 85c60716 is separate row 1 evidence.

Row 2's prepared handle retains one explicit destination NodeId. All calls,
including chunk copy and readiness/setup reads, use that destination through
the ordinary typed route; member ids must never fall through to a home.
The contract bounds RPC/readiness/setup waits and treats a mutating deadline
as uncertain, not permission to resend. No existing test timeout is changed.


### Row 2 contract review: three accepted boundary corrections

V1 returned a1ad971e/a896ac17 before implementation. The composition choice
was accepted; the three findings are accepted without changing node guards:

1. Delegated worktree reads and exhausted local wait slots can remove wait.
   The first no-progress reply returns the same WaitingSetup/WaitingReady
   state without an automatic reissue. The sole job enforces at least one
   second before another observation; earlier continuation is local-only.
   Setup/readiness observation budgets remain finite. Tests must use an
   actual member/relay and a local exhausted-slot connection, not an ideal
   long-poll mock, for both setup and event/readiness paths.
2. Attachment reads are bounded to 45 seconds, begin/write/finish to 120.
   Whole copy is at most thirty minutes per target, with 120 seconds of no
   acknowledged-byte progress ending a copy. The exact phase, known upload,
   acknowledged offset, attempted byte range and resource trail survive
   deadlines/lost replies/cancellation. Unknown mutations are UncertainCopy;
   incomplete copies never prompt or automatically resend. Existing upload
   expiry is not used as a request timeout. Stalled reads and lost begin,
   write and finish have separate tests.
3. One nonclone client ComparisonJob owns target transitions and the frozen
   common snapshot; start no longer consumes mixed pending ownership.
   Passive snapshots cannot restart work. Source binding requires Ready
   only initially; later pending targets use those same stored refs/text
   after the source is Started or ended, or fail if it is removed/changed.
   Dropping a method reply future retains the owner/operation. Explicit
   cancel or owner drop stops later phases and turns an admitted unresolved
   session-create/upload/prompt into typed uncertainty, never a restored
   waiting continuation. Ready+WaitingSetup, later identical bytes, sent-once
   successes and cancellation during continued session-create are required.

The corrected API details and outcome table live in ADR 0046. Shared input
is validated before binding/first prompts; a rejected candidate is unbound,
whereas a bound snapshot cannot be replaced by a newer draft. The CLI must
cancel before printing its final report if exiting with pending work; no
executable continuation is promised across CLI processes. No server registry
or durable idempotency claim is introduced. GOAT's cited SSH fanout fixture
is predispatch refusal coverage only; positive fanout is proved by the new
real-node/fake-agent tests when implemented.

The docs-only ff4cfb29 revision changed no production code, generated wire,
gate, fleet, service or permissions. The unchanged 85c60716 whole-gate results remain
separate; no code gate is rerun for this documentation-only correction.
Row 1 implementation PASS is V1's reported decision, not app/UI approval.
V1 accepted ff4cfb29 as contract-only PASS. Row 2 implementation verification
and its immutable implementation review remain pending.

V1 confirmed the authority-phase wording correction during implementation:
operate-only fails read/forward preflight; read-only can pass reads and then
the first normal worktree.create is Forbidden before resources. Both must
leave zero worktrees. Existing guards remain unchanged; the contract PASS
is unchanged and is not current implementation/test approval.


### Row 2 implementation and targeted evidence

The client owns the comparison in `vessel-client::comparison`: bounded input
and strict manifest decoding, one nonclone worker, independent preparation,
advisory setup/readiness observations, immutable common input and chunked
copy, and passive typed recovery reports. The worker uses one active
selection, within the contract's maximum of two. Acknowledged model turns
run independently. No node comparison registry, RPC, wire field/event,
schema migration or protocol bump is added; the branch remains protocol 19.
The CLI consumer is `session compare`; its source upload and single-send
share one streaming file reader. Serde is an existing workspace dependency,
now directly used for the CLI's typed recovery report.

An earlier targeted run passed Clippy for client/CLI/node with all targets,
5 client input/error tests, 20 real-node tests and 1 actual CLI test. This
is targeted evidence only. The initial whole root gate passed fmt/Clippy,
then stopped at two existing SSH-start handshake timeouts (203 Node E2E
passed, including all 20 comparison cases). Their required isolated retries
both passed: token refusal 1.24 s, login-profile startup 14.04 s. The
remaining whole-gate stages were not executed by that failed run. A final
changed-tree gate is owed after the recovery/preflight corrections below.
The real-node cases cover:

- One pinned commit even while the requested branch advances; independent
  models/worktrees; provider receipt of common text and readable file links,
  identical stored bytes and distinct destination attachment ids.
- Busy Start-only refusal, independent bad-model failure, failed setup,
  no initial source from another job, common-media validation before any
  prompt and an explicitly corrected unbound candidate.
- Actual member/relay setup reads with wait removed, local exhausted wait
  shares, and exhausted delegated readiness reads; the same pending state,
  monotonic eligibility and no immediate extra RPC or replacement session.
- Ready plus WaitingSetup/NeedsApproval, first target Started, then the same
  original pending target receiving the original stored bytes; no resend
  or recreation of the successful target.
- Lost begin/write/finish replies, stalled byte-read, accepted-byte progress
  deadline, exact known upload/offset/attempt trail and no incomplete prompt.
  Owner cancellation/drop and dropped method futures preserve admitted
  uncertainty or finish the original operation exactly once. A cancelled
  continued create cannot recover its previous executable continuation.
- Same-length source mutation, missing source and missing empty source;
  remaining copies fail without replacing the snapshot or resending success.
- Real negotiated protocol 9/10 peers, zero/count/choice/encoded-size and
  unknown-field bounds. Snapshots are not deserializable and jobs are not
  cloneable (compile-fail API examples are included in the full doc gate).

Authority is reported at the actual boundaries confirmed by V1. Operate-only
fails read/forward preflight. Read-only reaches its first normal guarded
worktree.create and is Forbidden before any resource exists. Agent
credentials are refused by the existing git.log preflight guard, so this
helper is unavailable to them. The normal ninth-agent-worktree Limit after
eight is verified independently; neither test grants or bypasses authority.

One early readiness fixture incorrectly assumed every delegated session.read
removed wait. Its grouped run timed out at the unchanged 20-second step
limit; the required exact isolated retry also timed out (20.80 s). This was
a fixture error, not a load-only failure: ordinary remote event long polling
is intentionally supported. The corrected fixture fills the actual shared
wait quota; all final targeted cases pass. Earlier CLI fixtures also used a
noncanonical harness JSON name and a stale project default branch; both
were corrected to the actual canonical wire name and explicit main ref.
No timeout was increased, no test was ignored and no product guard changed.

App integration is outside this slice. A transport owner retains
`NodeClient::comparison(destination, plan)` outside its panel, calls prepare,
uploads once to a Ready source through ordinary bounded upload calls, then
binds `ComparisonInput` with finished `SourceAttachments`. It follows
`updates()`/snapshot, keeps pending targets through start, and uses
continue_target on that same owner after setup/readiness or external
approval. It follows each returned ordinary SessionId separately. Cancel or
drop stops new phases; passive reports cannot restore an executable owner.
Local source uploads also retain their own phase/known upload/offset trail;
the CLI implementation demonstrates cancellation reporting. No native or
page screen, draft-recovery interaction, registration or UI approval is
claimed. Only the two existing side-question test-helper visibility declarations
were widened to reuse its real scoped-device fixture; no product seam in
another area was changed.


Two additional source-review findings were corrected before final
verification. An acknowledged finish id must remain in recovery reports even
when returned metadata is rejected; the CLI source upload follows the same
rule. A successful harness.options RPC with no structured capabilities means
that adapter is unavailable, so shared preflight must stop before any
worktree/session is created. These retain existing node semantics and add no
wire field or authority. Regression and final-gate results follow below when
complete; the earlier source pin and failed whole run remain separate.


The two corrections passed targeted verification: all 22 comparison Node E2E
cases, 5 client input/error cases and the real CLI case, plus client/CLI/node
all-target Clippy. The finish-mismatch fixture withholds a real node's
acknowledgement after the stored file exists, changes only returned metadata,
and then reads the actual stored bytes by the retained id; no destination
prompt or retry occurs. The unavailable-provider fixture installs a fake
Codex terminal without its adapter and confirms the successful capabilities-
absent response stops all selections before any worktree/session creation.

The final source pin covers 1,662 root source/build-input files; manifest
SHA-256 is 762b9e60ba5b48674c00907a27288502dc60742d7d4a68c6c2516694c48aa1ef.
The original failed gate/pin remain separate. The final changed-tree whole
root/web gate completed successfully on 2026-11-14 at 10:59 UTC. Rust test
threads were set to one for resource scheduling; safety limits and assertions
were unchanged. No test was newly ignored.

### Row 2 final gate and handoff

The detached final gate passed all stages: fmt, workspace all-target Clippy
with warnings denied, workspace Rust tests (1,827 passed, zero failed, ten
existing ignored), generated TypeScript checks (two passed), web lint and
typecheck, web unit tests (417 passed in 81 files), the production bundle,
bundled CLI/node/testkit binaries and Playwright E2E (62 passed in 3.5 min).
The Rust total includes both compile-fail comparison API examples; its Node
E2E suite passed all 207 cases, including all 22 comparison cases and both
SSH cases that timed out in the original whole run. The actual changed
client/node/CLI crates were recompiled. No automatic screenshots, traces or
video were recorded; no native/page screen or UI approval is claimed.

The master log is row2-final-gate.log, with one row2-final-<stage>.log for
each stage. The source manifest above covers 1,662 root source/build inputs
and matched byte-for-byte after TypeScript generation and the complete
final gate, including its file list. There is no generated wire delta. The
first failed whole run and exact retries remain historical evidence above;
they were not silently relabelled as a pass.

The CLI/API guide above is the screen handoff. The implementation is ready
for V1's immutable-diff review; contract PASS is not implementation approval.
No new protocol call, field, event, scope, store migration or protocol bump
is added by row 2. The current branch protocol 19/schema 14 belong to row 1
and its integration base.

The gate ended normally. All five saved fake-fleet PIDs were absent after
teardown, and no binary from this slice's target directory remained. No
manual fleet or virtual display was created. Commands ran as the checkout's
ordinary user with existing ownership/modes preserved; no live service,
preview node/port, real-provider prompt, permission change, push or release
operation was used.

### Continuation order

Finish and submit this row's node/client/CLI slice before starting another.
The next assigned node contract is startup recovery, following Paseo's
`agents.resumeOnStartup` and `agents.resumePrompt` behaviour. Read the
reference restart-recovery module and its actual defaults before proposing
the contract in allocated ADR 0059. Define which
in-turn sessions qualify, excluding parked, archived and person-stopped
sessions, ordering and a startup concurrency bound, and observable failures
when an agent cannot resume. Clean-stop and kill recovery need fake-agent
tests. This is a node.settings feature, not an app screen.

Delivered files/HTML (row 4, ADR 0047) remains pending after that newly
prioritized contract. App screens, page work, conversation/composer
registration and the old native WIP are outside this session's scope.

### Resume checkpoint after the row 2 submission

Implementation and local gate: af6d166df9fb0ce12caae54002497ceafea27aa4.
V1 received its immutable implementation review request, including the
reference paths, full logs, source pin, earlier failures and exact retries,
authority boundaries and screen API handoff. V1 subsequently accepted af6d166d's client/CLI implementation, independently
checking the selected source manifest, immutable diff and complete logs.
Node integration, native/UI work and whole vertical-feature parity remain
separate. No unchanged-tree gate rerun was requested or performed. The intentionally untracked web dependency/bundle
links and both ignored Cargo build configurations must remain uncommitted.

Next after review: ADR 0059 startup recovery contract and reference defaults,
then its node/settings/CLI/fake-agent slice; delivered files/HTML ADR 0047
remains later. No app screen or other branch merge has been started.
No owned build, fake fleet or display remains running at this checkpoint.


## Startup recovery: contract first (ADR 0059)

Row 2 client/CLI implementation af6d166d received V1 PASS; the review does
not pass this new contract or app work. The continuation now starts with
startup recovery, using allocated ADR 0059. No new production source,
settings value, service or agent runtime has been changed for it.

The reference sources read whole are Paseo's
packages/server/src/server/agent/restart-recovery/index.ts and index.test.ts,
and packages/app/src/screens/settings/restart-recovery-card.tsx. Their
bootstrap integration, config defaults/reloadable fields and lifecycle doc
were also checked. What a person does in that reference:

1. Connect to a host and open Settings, that host's Agents section.
2. Turn on Resume agents after restart. It starts off by default.
3. Optionally choose Edit, change Resume prompt, and Save. The exact nonblank
   text is saved; Cancel leaves the old text. The default is Resume working.
4. Leave an agent actually working in a turn, then stop/restart the daemon
   or encounter a crash. Startup selects that saved running agent before
   normalizing statuses and reopens its existing provider conversation.
5. The daemon sends the configured prompt. Each agent progresses separately;
   a broken or stuck provider does not block a working one. Failures are
   logged, with no automatic recovery retry loop or exactly-once promise.
6. Turning the switch off consumes pending intent. Turning it on later does
   not revive those old agents. Idle/initializing, archived, private helpers,
   owned executions and archived-workspace agents are excluded.

[Modification/reuse decision] Existing node settings, the serial store
writer and per-session durable log lane, normal resume claims, launcher,
current device-grant lookup and Start-only delivery are the reusable assets.
A standalone scheduler/Owner launch path would duplicate lifecycle and risk
reviving a person's stopped work. No external API/package is needed. The
missing part is a durable active-turn/stop-generation projection, including
parked state, plus bounded startup coordination. Raw process Running or a
retained event tail is insufficient. ADR 0059 records the typed settings,
protocol/schema additions (including an explicit read flag so an older
forwarding hop cannot hide recovery settings), authority, phase ownership, stop barriers,
partial settings failure, manual takeover, deadlines, reporting and tests.

Necessary reference differences are explicit: at most two active attempts;
finite attempt admission; bounded prompt; current creator grant revalidation;
no startup action for a forwarded origin whose remote authority is not
recoverable, helper/agent/unattended work, or legacy rows with no active-turn
projection. Session marks are the available archive boundary on this node
line. This is a proposed node contract, with no implementation result.

Next: submit this immutable docs revision to V1 before coding. No unchanged
row 2 gate is rerun for these docs. After accepted contract, implement only
wire/node/store/session/client/CLI/testkit/generated types and run changed-
tree gates using detached commands and build slots. Report screen calls to
the app owners through the normal review path; do not draw or register a
screen. Files/HTML ADR 0047 stays after this new priority. No owned build,
fake fleet or virtual display remains running at this checkpoint.

Contract handoff: docs-only d732de4459e570ec8daf175a915e19fbbcdcd894 was
submitted to V1 with both document hashes and the complete reference,
compatibility, authority, lifecycle and evidence boundaries. Startup
recovery code has not started; the next action is to resolve that contract
review, then implement its accepted form. No additional gate was run for
the documentation-only handoff.

### ADR 0059 contract return and revision

V1 returned d732de44's contract for two boundaries before any recovery code.
The defaults/reference, consume-before-dispatch and bounded ownership
direction were retained. This revision makes both obligations explicit:

1. Reuse Settings' existing merge/write/apply engine inside one bounded,
   tracked node-owned lane (one active change, eight queued). Once admitted,
   an operation owns its saved-settings ordering through the actual file
   write, effective Off/generation invalidation and intent cleanup. RPC
   cancellation, dropped replies and disconnects cannot abandon it. A
   provisional admission barrier covers an unresolved Off save; file failure
   leaves old policy/intent intact, while post-save cleanup failure retains
   effective/saved Off and a cleanup obligation. A queued On cannot overtake
   it or revive the old batch. Shutdown drains admitted settings operations.
2. An initial creator lookup does not authorize a prompt after loading.
   Read current device grants and check their revision/revocation epoch at
   every new launch and subsequent prompt admission after waits. Serialize
   the final grant/policy/Off/stop-generation/ownership checks and enqueue
   with invalidations. Cached Guard principals and synthetic socket contexts
   are not fresh authorization. Already admitted effects retain their
   explicit in-flight boundary; a denied later phase sends no prompt, with
   cleanup fenced to the exact run the attempt still owns.

Required regressions now include RPC cancellation/drop during a gated real
settings write and post-save cleanup, cleanup failure with queued On and
shutdown/queue capacity; and real fake-provider load/ready gates followed
by revoke, grant reduction, Off or manual stop. Releasing readiness must
not send another prompt or kill a manually taken-over run. The final
admission race is tested separately from effects already in flight.

Only the contract/plan are revised. V1 contract re-review is required before
implementation; neither this revision nor row 2's existing PASS asserts
these new behaviours exist. No production source, runtime settings, grant,
service or permission was changed, and no unchanged af6d166d gate was rerun.


### ADR 0059 implementation checkpoint

V1 accepted revised contract 396e2a84, closing both documentation returns;
this is contract-only PASS. The implementation is in progress and has not
received implementation or whole-gate approval.

Existing Settings, normal resume preparation, current device grants, actor
input and the SQLite writer are extended rather than duplicated. The code
now includes the additive policy fields and CLI, a bounded node-owned
settings worker, active-turn/stop-generation projections, ordered internal
checkpoints, exact-generation reopen, fresh authority at launch and prompt,
and two-attempt startup coordination. No new RPC/scope/public event or
external dependency is added. Protocol 20 and schema 15 remain provisional.
README, ARCHITECTURE, PROTOCOL and ADR 0059 describe the resulting paths and
partial failure/ownership boundaries. Tests use fake providers only.

Targeted checks so far are development evidence, not a final result. The
initial behavioral run passed six settings cases, then two of three startup
cases; its non-timeout failure followed an ended subscription before the new
run was visible. The positive restart tests now wait at the actual provider
load barrier before following/releasing the new run. Later targeted Clippy
runs exposed and corrected missing collector adaptations, operation-context
types, strict-lint findings and a split-test module path. Latest behavior,
generated types and the final frozen-tree whole gate remain pending.

The next test run explicitly uses a private SSD temporary directory and
records host load/memory before and after. Build output and logs remain on
the build volume. Earlier targeted runs had no TMPDIR override and already
used the operating system temporary directory; no HDD-to-SSD speedup is
claimed for them. Deadlines and assertions are unchanged.

Screen handoff (pending, outside this node slice): read the peer capability,
then `node.settings { include_restart_recovery: true }`; patch the existing
`node.set_settings.restart_recovery` object with independently optional
`enabled`/`prompt`. Preserve exact custom text while off, distinguish an
unsupported peer from Off, and surface saved-but-not-applied/cleanup errors.
Settings Debug redacts prompt content; an actual continuation is an ordinary
persisted user turn. No native/page area, control or registration is added.

Development review also found a late-generation race: a claimed task that
first runs after Off/On must still use the original boot generation. That
generation is to be captured once by the boot owner, with a deferred-attempt
regression; taking a new snapshot at task execution would revive invalidated
work. A default-prompt test also needed to answer the fake provider's actual
permission before expecting TurnEnded. Exact default/reference-newline/
multibyte provider echoes now replace that incomplete wait. Neither finding
is treated as a whole-gate pass.

Focused recovery-unit evidence on the intermediate implementation: 10 tests
passed, 0 failed, 219 filtered out, in 38.63 seconds after a 5m39s compile
and a 1,226-second slot wait. This covers actual settings cancellation/
cleanup, final Off/stop prompt admission, missing-folder independence and
three provider failure modes. It is not the final tree or whole-workspace
gate: the boot-generation correction, remaining integration tests, generated
types and full gate are still pending. No deadline or assertion was relaxed.

The second focused invocation did not execute its integration tests: an
incorrect test Reply variant failed compilation and was corrected. The
10-test unit result above remains separate. Boot generation is now captured
once by the boot owner. Typed stop/input admission also closes the durable
ACK-to-effect gap; new held-journal-ACK fake-provider regressions cover kill
and cancel. These final changes still need the whole changed-tree gate.

The first final-gate attempt passed both required prebuilds, TypeScript's
two generator tests and formatting, then stopped at Clippy: a test import
path, one missing semicolon and a documentation backtick were corrected.
It did not reach workspace tests or web checks. An explicit failed-Off file
replacement regression now also checks that the old On generation/admission
and pending intent survive a failed save. The next changed-tree gate remains
required; no lint/deadline/assertion was weakened.

Before the second final-gate attempt left its build queue, source review
found another required exclusion: Imported display metadata can replace a
delegated session's Remote marker. Creation now records Person only for an
authenticated local Owner/Device with no unattended owner, and a real service
creation/projection regression contrasts local and delegated imports. This
correction was installed while the gate log still contained only the slot
wait, before any prebuild or source-manifest freeze. The import/file-copy
RPC authority path is not claimed tested by that internal creation fixture.

The second final-gate attempt passed prebuilds, TypeScript's two tests and
fmt, then Clippy found two unnecessary test type qualifications. They were
removed without changing assertions or production behaviour. Workspace tests
and web checks had not run. The next gate collects all stage results while
retaining a nonzero overall status for any failure; this is not a waiver of
Clippy or any required check.


### Recovery whole-run fixture corrections

The third frozen-tree whole run passed fmt, all-target workspace Clippy,
TypeScript's two tests, web lint/typecheck, 417 web tests, web build, bundled
binaries and all 62 web E2Es. The 1,693 selected source files stayed identical
before/after generation and after E2E. Rust returned 1,871 passed, seven
failed, ten existing ignored across 112 result sections: this is a failed
whole gate, not implementation approval. Its logs/manifests are retained
separately as recovery-attempt3 in the scratch evidence folder.

All seven failures were new fixtures. Three exact provider echo assertions
expected fake-small although structured_spec selects fake-large and recovery
correctly preserves it. The exact full-text comparison now uses that selected
model; whitespace and multibyte assertions are unchanged. The Store new-turn
fixture omitted the actor's Running transition after reopen; it now appends
Running before TurnStarted, keeping Starting excluded in production.

The fleet-settings test incorrectly assumed node.settings was forwardable.
It already is host-only administration. No method allowlist or grant is
expanded: the actual member envelope must refuse both read and mutation
even with Admin, leaving the setting unchanged. A separate host-paired
opaque-relay device exercises successful Read/Admin and Read-only mutation
refusal. A nested envelope's version requirement remains a compatibility
check, not a forwarding permission. Screen callers must use that host's
paired connection, not call_on.

Both stop-journal timeout cases were run exactly once alone on the unchanged
source and failed at the same 20-second barrier (21.89s cancel, 20.25s kill);
before/after load was 6.95/7.31, so these are not load exceptions. A newer
prompt reserves the sole control slot before waiting for input admission;
a PrivateDir control-lane barrier therefore deadlocks the fixture itself.
The test now reuses the independent rename response channel, acknowledged
by the real actor after higher-priority queued input. The held journal ACK,
no-TurnStarted check, kill refusal, subsequent cancel/start and exact event
counts remain. No deadline, channel capacity or production guard changed.
The corrected tree subsequently passed the targeted checks and the complete
changed-tree gate recorded below.


### Independent correction: side-session completion notification

A subsequently reported reset race was confirmed at the actual end broadcast:
hand_over notified before runtime.remove/forget, so a waiter could consume
its only wake while still finding Live. The unchanged ten-second wait
already subscribes before checking and rechecks after every notification.
The correction publishes after both removals, with no retry or timeout
change. A synchronous receiver-wake regression forces the old observation
using a real fake provider and actual persist::run. It fails on the assigned
standalone base a365d295 in 3.15 seconds, seeing both live entries Present.

The standalone correction and whole gate are kept separate from recovery;
the same product correction and exact regression are included here so a
later combined tree has one identical test rather than competing variants.
The standalone correction is committed as cdea77d820b38d291c6ddab03fd04437751749cf
on base a365d295. Its author-run full root gate passed (1,964 tests, zero
failures, 12 existing ignored), and it was sent to the node integrator,
coordinator and V1. Its selected 1,715 source inputs matched the committed
Git blobs. V1 subsequently passed its source and Linux root gate; node-head
integration is separate. This recovery tree's final gate is recorded below,
and no native/page scope is added.


### Startup recovery: final gate and node handoff

The final changed-tree gate ended successfully on 2026-11-14 at 21:24 UTC.
It passed root fmt, workspace all-target Clippy with warnings denied,
workspace tests including doc-tests (1,880 passed, zero failed, ten existing
ignored across 112 result sections), both generated declaration checks,
web lint/typecheck, 417 web tests in 81 files, production build, required
bundled CLI/node/testkit binaries and all 62 browser E2Es (3.4 minutes).
Node unit tests passed 234 cases and Node E2E passed 223, including the 16
restart cases. Earlier root failures, the exact failed stop-fixture retries
and the corrected targeted checks remain historical evidence above; they
are not alternative whole-gate approvals. No test was newly ignored, and
no assertion, deadline or channel capacity changed.

The one outer build slot held every sequential stage. The same environment
prebuilt fake binaries and vesseld before tests, used SSD temporary data,
and ran Rust test harnesses with one test thread. The changed proto,
session, store, node, CLI and testkit crates actually recompiled. Load was
19.34 before Rust tests, 21.78 before browser E2E and 21.71 at gate end.
The 1,694 selected source/build inputs matched before/after TypeScript and
at the end: SHA-256
a3a81588be6bcf0d79ff67e609f0b5a180dbb8a9e7f8e157f28cab4cece5fa37.
The selection excludes docs, native assets, ignored local settings and
dependency/output folders; it is not a full environment attestation.
Logs are recovery-final-gate.log and recovery-final-<stage>.log, with
recovery-final-source{,-after-typescript,-after}.sha256. Authored whitespace
checks pass. The canonical generated proto.ts retains seven ts-rs trailing
whitespace warnings; it was regenerated and not manually reformatted.

The process-kill test explicitly kills and waits on its owned child node,
then checks the same native conversation and one recovery prompt. One
verified fake-provider cleanup raced its exit and printed No such process;
the guard checked that the captured start identity no longer matched. It
was not a failed node-kill assertion or a deadline exception.

Screen callers use the host's own paired connection (direct or opaque
relay), not fleet forwarding. Read node.settings with
include_restart_recovery=true and check feature capability; absence on an
older peer is unavailable, not confirmed Off. Patch
node.set_settings.restart_recovery.enabled and/or .prompt; leaving either
out preserves it, and Off retains the exact accepted prompt. Read remains
Read and mutation remains Admin. A failed/lost reply can follow a saved
change: re-read effective state and show partial-save diagnostics rather
than assuming it was unsent or unchanged. Existing session read/follow
shows ordinary status, the actual resume prompt and bounded failures.
There is no new RPC, public event or permission scope. This branch uses
named recovery feature protocol 20 and store schema 15 provisionally;
integration must renumber around the current node line's additions. The
settings response/patch contain PromptText and are Clone rather than Copy.

The already-reviewed side hand-over correction is byte-identical in this
combined implementation; its standalone cdea77d8 and V1 pass remain separate.
This implementation is based on contract commit 396e2a84 (production from
the row 2 line), not a replacement for a combined latest-node gate. Submit
the immutable root delta from that base to V1; only V1 forwards accepted
recovery code to the integrator. No native/page work, live preview, real
agent prompt, service permission change, release or push is included.

Changed files retain their ordinary checkout account ownership, and commands
ran as that account. The owned executable,
SSD-temporary executable/cwd scan was empty after the gate. No manual fleet
or virtual display was started. Ignored build configurations and the two
intentionally untracked web dependency/bundle links remain local.

Next remains delivered files/HTML (ADR 0047), after this slice's immutable
review handoff. UI settings, side surfaces and other app integration belong
to the app tracks and are not asserted complete by these node/CLI checks.
