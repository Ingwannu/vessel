# Milestone 12: session extras (N5)

Status: row 1 node implementation and integration gates passed, 2026-11-14;
implementation review is pending. The results under resumed integration
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

The next node contract is row 2 (ADR 0046), then delivered files/HTML
(row 4, ADR 0047) under the resumed three-feature assignment. Row 3's terminal
surface and rows 5/6 are outside this resumed session's assigned work.
The app owners implement the screen using the call guide above; no native
screen, control or page implementation was added here. The old native WIP
checkpoints remain recoverable, but are absent from this delivery tree.

Cleanup after verification found no remaining build/test/fake processes for
this worktree or target directory. No owned fleet or virtual display remains.
No preview node/unit/port, real-provider prompt, service permission, account
configuration, push or release operation was used. All local commands ran
as the checkout's ordinary user, preserving existing ownership and modes.
Implementation review and any resulting fixes are still owed before merge.

## Row 2: node contract proposal (ADR 0046)

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
This is a proposed contract only; implementation has not started. The gate
results for 85c60716 remain its unchanged code's evidence, not evidence for
this proposed feature. V1 contract review is required before implementation.
