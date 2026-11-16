# A8: permissions, questions and CLI permit

Status: first question frame built; response correlation under narrow validation. Owner W4, PLAN section 7. This replaces the earlier A5a
assignment; its unverified skeleton is parked on `backend/m12-wp-side`
at `fc7f3139` and is not part of this branch.

## Reference read and contract

Paseo 0.15.4, source snapshot `1c00b365c9e0b74c1580eba33d905d693125ae21`:

- `packages/app/src/agent-stream/view.tsx:1513-1740`: provider actions
  precede fallback actions; exact selected action identity; plan Markdown;
  selected spinner and disabled sibling actions; inline question failure.
- `packages/app/src/components/question-form-card-core.ts:30-179`:
  initially empty selections; radio/checkbox/custom-answer encoding;
  empty-text dismiss submits rather than denies; custom dismiss label.
- `question-form-card.tsx:365-565`: single-choice navigation never submits
  the last answer; custom/selected values replace or append by cardinality;
  controls hold during submission and preserve drafts on failure.
- `packages/client/src/daemon-client.ts:6237-6263`: response confirmation
  matches agent plus request, bypasses offline queue, waits 15 seconds.
  This is a response deadline, never request expiry or an automatic denial.
- `packages/server/src/server/agent/permission-response.ts:22-42`:
  provider-owned follow-up prompt starts a replacement run after approval.
- `agent/providers/claude/agent.ts:1193-1227`: implement versus
  implement_resume are distinct provider actions; the UI must preserve ids.
- `packages/cli/src/commands/permit/{index,allow,deny}.ts`:
  list/allow/deny; allow without request selects all; deny requires request
  or explicit all; input JSON validates before connecting; success prints
  only after acknowledgement; unconfirmed outcome asks for reconciliation.

The five assigned survey sections in `gaps.md` are the complete spec,
including nonblocking Codex questions, persistence, cancellation and fan-out.
Further reference paths will be recorded as their runtime paths are read.

## Existing assets and audit

| Surface | Existing owner/code | Finding and required work |
| --- | --- | --- |
| Permission tool/plan body | `view::timeline::permission`, `conversation/permission.rs` | Already renders provider option ids/names, tool detail and input plan. Presentation kind, primary/secondary/danger and intent await ADR 0084. Keep the explicit approval-field destructuring guard. |
| Question radio/checkbox/custom values | `timeline/question.rs`, `question_step.rs` | Typed fields and node validation already exist. Add source-tested navigation, empty defaults for the native form, presentation/preview/dismiss support. |
| Question visual states | `conversation/question.rs`, `question/card.rs` | Existing retry retains drafts; lacked single-select advance, Markdown question and per-action spinner. Reuse W10's parked patch, audit and test independently. |
| Response boundary | `link::session`, `Place::answer`, `ConversationArea::answered` | Existing generic Act answer identifies only session and kind, not request; late failures can target a newer question in the same session. Add request/ticket correlation, bounded acknowledgement and safe reconciliation. |
| Durable/nonblocking request runtime | ADR 0084, W23 | W23 owns proto/node changes and provider mappings. Consume reviewed typed contract; do not duplicate or invent wire fields/calls. |
| Resolved Plan rendering | A7, W10 | W10 owns shared `conversation::plan_card` renderer; A8 supplies approval outcome and footer. No copied Plan renderer. |
| CLI | Existing session answer and selector; W17 output seam | Add permit command/parser, list, asymmetric selection, rich response and acknowledged output. Reuse reviewed W17 output helpers and W23 contract. |

## Decision Log

- Purpose and intent: preserve the exact request a person answers, all
  provider-defined choices, retryable local drafts and authoritative
  cross-client resolution; do not turn a timeout into a denial or resend.
- Existing implementation and constraints: most controls and typed form
  rules already exist. Node base integration is central Step 0. Current
  optional metadata cannot express the surveyed rich presentation/runtime.
- Alternatives: duplicate question/card reducers; parse opaque input in
  each screen; infer implementation actions from labels; send async
  question answers as unrelated messages. These lose identity or provider
  policy and are rejected.
- Choice: modify/reuse existing view/card code, add a small correlated
  response boundary, consume W23 ADR 0084 and W10's Plan renderer. W17
  owns common CLI output. Standard Rust and existing dependencies suffice.
- Why: each behavior has one owner, while existing pending/request cards
  remain usable against older nodes without pretending rich data exists.
- Advantages, costs and effects: narrow shared registrations; separate
  node-contract and native evidence; rich behavior refuses unsupported
  nodes before effects, without silently downgrading the user's answer.

## Coordination and differences

- W23 (`17459095`) owns the shared proto/node permission/question fields,
  rich response acknowledgement and durable async runtime under ADR 0084.
  ADR contract `b5b131bd` passed V3 source review (row113), and standalone
  types `96ed8b0b` are committed source contracts. Attached metadata, rich receipt
  and runtime proof still need W23's checked consumer hash; none is treated as
  consumer-ready from source review alone.
- W10 (`9dc6bb3a`) owns A7 timeline and shared Plan body. His A8 patch
  `bef67fa7..95206de1` has a reported clippy pass only; tests/captures are
  unverified. The saved patch omits its `step_after_pick` re-export, which
  this branch must add explicitly.
- Existing ACP field-keyed answers remain authoritative; the provider
  adapter maps those to question header/id strings. The app must never
  manufacture unrelated prompt messages as a fallback.
- Existing bounded in-card scroll remains a stated responsive difference
  from Paseo's page-scrolling card; controls must stay reachable at 390
  and 800x600. No hidden incoming metadata may become an actionable grant.

## Validation

Per sub-feature: narrow changed-crate clippy and tests through cargo-slot,
with `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. Unit transition tables,
in-memory node responses, delayed/out-of-order cross-client resolution,
timeout without automatic retry, retained input and exact provider ids.
At package end: private fake fleet and authenticated Xvfb, inspected EN/KO
dark/light desktop/phone/800x600 captures; one whole native gate and V4
review. CLI/root changes get their applicable root gate and review. No
real agents, live preview, public pushes or production changes.

## Correlated response implementation

`view::responses::Registry` bounds window-wide admissions to 32. Its monotonic
RequestId ticket includes MachineKey, SessionId and the typed permission/question
id. `areas::responses` retains the originating conversation Seat. A reply only
changes a still-waiting, still-sending card with the complete matching identity;
a same-session successor, cancellation, navigation or duplicate reply is ignored.
The existing retained question TextField becomes read-only during submission;
inline refusal preserves its drafts, caret and selected step for an explicit retry.

`link::responses` uses the existing session.send typed input without offline
queueing. The 15-second acknowledgement deadline yields Unconfirmed and triggers
an authoritative conversation reread; it never sends a denial or retry. A bounded
semaphore retains its permit through result delivery. Capacity refusal applies
backpressure to the actor instead of spawning unbounded refusal tasks. Provider
error text is not rendered; typed categories select embedded EN/KO Fluent words.
This legacy acknowledgement is not the new ADR 0084 durable resolution receipt;
that richer call remains pending W23's reviewed consumer contract.

The in-memory framed RPC tests check exact provider option preservation, waiting
for acknowledgement, safe refusal, uncertainty without retransmission, and local
capacity/offline refusal. App tests cover same-session successor/cancellation,
wrong-target replies and duplicate acknowledgement after a manual retry. These
are tests to run, not yet a green runtime claim.

Narrow evidence: app-only clippy on the question frame passed locally before the
base merge. Remote be9cd023 run wp-side-narrow-20261115T063710Z-be9cd023 failed in
view test targets on two unused qualifications; fixed in the next commit. No
whole gate, inspected captures, durable rich response or V4 PASS is claimed.

## Retained editor identity

Coordinator approved the shared kit extraction on 2026-11-15: 7a3e3ed1 moves
Handlers/listen without behavior changes, lowering field.rs ratchet694→635.
Its narrow kit check compiled the extraction but failed an earlier A8 editor
regression's missing semicolon; the follow-up fixes that test. The opt-in
TextField::with_source callback supplies FieldId from the actual engine entity;
all existing constructors retain their signatures. Question input handlers
require that source to own the current waiting question. Rendered buttons also
retain a request/step cursor, so old option/tab/submit clicks cannot affect a
successor. A real keyboard test checks source propagation, and the app regression
proves stale typing/Enter cannot alter or submit a replacement question while
its own current editor can submit normally. No copied input engine is added.


Offline handling retains local drafts while `dock::requests` hides cached cards
unless the shown machine is connected and its conversation stream is not lost.
The answer controller repeats this availability check before admission; reconnect
alone never queues or replays the retained answer. This is the native equivalent
of Paseo's stripping pendingPermissions from offline replica state, without
throwing away an unsent editor draft.

Reference clarification from agent-stream/view.tsx:1673 and :1821-1900: only
`variant === primary` changes action text emphasis. Secondary/danger use the
same muted bordered button in this snapshot; do not invent a red danger style.
The exact action identity/behavior/intent still belongs to the provider contract.
Question option labels and descriptions use compact Markdown (question-form-card
.tsx:151-163); option previews are plain wrapped text on surface0, shown for all
options before selection (:163-178). Rich metadata integration must preserve
those distinctions.


## Failure isolation and shared editor seam

An answer failure is now a typed, exact-request entry in `responses::Failures`,
with independent permission/question slots. Generic Conversation/Info/Rename act
failures cannot erase it. This fixes a queue-full/read-failure race where the
reconciliation read replaced the old generic error, leaving a form marked
sending with no visible retry. The controller clears only that request's failure
on retry/ack. Cards translate retained categories at render time, so language
changes do not retain a stale-language transport sentence. A failed refresh and
duplicate old ack after retry are explicitly tested.

The native lead requested Editability as a shared consumer seam. Independent
branch backend/m12-wp-permissions-kit at base8d5513e4 has8eac02e8 (unchanged
Handlers/listen extraction; normalized source-body equality recorded in scratch)
and497dd62e (Editability-only API/root export and Line+Lines keyboard tests).
V1 source review requested in queue134. Neither shared commit is consumer-green
until its narrow Clippy/tests complete; the A8 package still gets its single V4
review at the end. A8 will merge the lead's package base and use that shared seam.

Narrow5d2b7054 run wp-side-narrow-20261115T065645Z-5d2b7054 failed on missing
per-message Fluent translator context and a FieldId documentation backtick; both
are fixed in the next commit. The newer source also corrects generated plain
message method calls and test unused results found during inspection. No passing
compile/test claim is made from a build that stopped in translation generation.


## Remaining parity validation matrix

These rows describe the fill-in checks; they are **not passing test claims**.

| Scenario | Required behavior | Boundary / owner |
| --- | --- | --- |
| Provider-defined plan/mode actions | Show all labels in provider order, preserve exact option id and implement vs implement_resume intent; only primary changes emphasis in this Paseo snapshot | W23 metadata + A8 permission view + W10 Plan body |
| Plan approval | Durable accepted resolution precedes replacing run with provider-owned follow-up; reject/cancel resets disclosure according to outcome | W23 runtime + A8 outcome seam |
| Question radio | Empty initial state; selecting one clears custom answer and advances one step, never submits final step | Existing typed view + source-aware A8 callbacks |
| Question multi-select | Checkbox order and selected labels retained while custom text appends; typed values stay provider adapter-owned | Question view + W23 provider mapping |
| Question preview | Every option preview visible for comparison as wrapped plain text; label/description compact Markdown | W23 option preview + shared Markdown renderer |
| Explicit custom companion | Use metadata FieldName pairing, not a guessed provider suffix; choice-only questions get no invented free-text answer | W23 field presentation + A8 step reducer |
| Empty answer / Dismiss | All free-text allowEmpty questions encode empty accepted answers on Dismiss; mixed/choice forms decline; custom dismiss label does not independently change semantics | W23 empty policy + A8 explicit pressed-action spinner |
| Failure / retry | Keep drafts/caret/step; unrelated metadata read failures cannot erase request failure; old callbacks/replies cannot affect successor | A8 request correlation and retained input |
| Durable async question | Completed provider message/turn does not withdraw session-lifetime request; active exact-turn steer or idle follow-up; restart does not resend uncertain answer | W23 runtime / fake Codex tests + A8 authoritative delivery state |
| Cross-client resolution / cancel | All observing clients fold one exact resolution; sender without subscription still receives durable receipt; cancellation removes actionable form, no human-answer TTL | W23 fan-out + A8 pending fold |
| Offline / reconnect | Cached cards not actionable; preserve local draft; no queued answer and no replay on reconnect | A8 availability admission + direct response port |
| CLI malformed input / missing scope | Refuse JSON/deny missing scope before connecting; allow omitted request means all; ambiguous prefix refuses before effects | Staged CLI parser/selection |
| CLI complete list / exact recheck | Last page must succeed; empty final list is valid; all selected current forms and source identities preflight before effects | Staged CLI pages/source |
| CLI modified input / refusal | Forward scalar/null JSON for provider validation; unsupported edits/message/interrupt refuse before that request's effects, never downgrade to legacy input | W23 rich receipt + CLI call |
| CLI batch acknowledgement | Print only acknowledged allowed/denied results, report possible partial completion and uncertainty without automatic retry | W23 receipt + W17 output helpers |

W23 confirmed deterministic fake-provider runtime tests will cover plan options,
rich refusals, async active/idle/restart. Existing seams are vessel-testkit
fake-acp/fake-codex and vessel-agent integration tests. Native captures may add
private test-only typed seeds after the exact reviewed hash, without changing
root provider adapters. Runtime fixtures and names are still owed with that hash.


Question text readiness/encoding now mirrors Paseo's otherTexts.trim(): whitespace
alone does not enable Next/Submit; encoding trims outer whitespace while retained
editor text/caret remain unchanged. The narrow trim helper matches ECMAScript's
WhiteSpace + LineTerminator set, including FEFF and excluding0085, rather than
silently taking Rust's different Unicode White_Space set. Primary reference:
[ECMA-262 TrimString](https://tc39.es/ecma262/multipage/text-processing.html#sec-trimstring)
and [White Space/Line Terminators](https://tc39.es/ecma262/multipage/ecmascript-language-lexical-grammar.html#sec-white-space).
Unit fixtures cover ASCII/Unicode blanks, inner text retention and encoded values.


Shared Editability8eac02e8+497dd62e is now V1 SOURCE + REQUESTED NARROW PROOF
PASS: exact kit all-target Clippy0 and real keyboard test1/0/0 (Line+Lines in one
case), independently audited specs/TSV/log hashes. Audit:
shared-field narrow audit w4-497-shared-field-narrow-audit.json. Package base6036 has
byte-identical kit, native manifest and lock; A8 merged it at541b5392.
This is bounded editor-policy proof, not RPC/authority/OS/IME/UI parity proof.

Latest local all-four-crate narrow on6761865c exited101 for four missing named
field doc comments in the new public link Response enum. Those comments are
added before merging announced package base4. The run stopped before checking
app, so it is not a passing compile/test result for A8.


## Base4 narrow follow-up

Merged the lead's common captured-dialog seam074ade16 after the base4 narrow
finished. Exactd91e035f all-four-crate Clippy reached app and failed on the new
ActAnswer.terminal test field and a needless owned reply argument. The former is
explicitly None for an ordinary conversation; response handling borrows its reply.
No actual A8 tests have passed from that stopped run.

Question Markdown uses the existing retained renderer. Its keyed state must be
unique to a retained form across simultaneous cards: a fixed question-prompt key
would reuse one renderer for different prompts in the same window. The existing
FieldId exposes a namespaced ElementId from its real input entity, preserving
the renderer through frames and steps while separating successor/parallel forms.
This reuses GPUI identity and does not create a second session/resident registry.


Option label/description parity reuses kit::markdown rather than a new renderer
or external Markdown dependency. Reference: question-form-card.tsx80-180 renders
compact Markdown for both; description interactions are claimed separately from
the option click. Native descriptions stop bubbling a click to the choice row.
Rich text cache keys retain the real form identity and distinguish step/option
parts. Preview needs W23's retained preview metadata and remains pending.

The GPUI regression draws the actual production option row and verifies a
description click leaves the step/draft unchanged, then a Markdown-label click
selects the real option and advances. It has no real link, node or URL opener.
The test is pending execution, not proof from source inspection.

Coordinator approved kit::markdown_label as a thin sibling of markdown using
the same TextView::markdown engine, selectable(false); existing selectable
Markdown and all callers are unchanged. The label/body distinction matches
Paseo question-form-card.tsx154. W12b owns link/image behavior; notify it with
the resulting hash. The regression must click actual rendered label text, not
row padding, so a selectable renderer cannot accidentally satisfy the test.


Plan disclosure is keyed to the exact PermissionId instead of one bool retained
across every approval in an area. A successor opens expanded; a queued chevron
from a resolved/canceled request cannot fold the new plan. The existing single
visible-plan slot retains only its folded request id, so no second disclosure
manager or unbounded map is needed. The GPUI controller regression covers repeat
toggle, successor and stale chevron order. Shared W10 Plan rendering stays the
consumer seam; its final typed outcome/renderer integration is still pending.


## Shared Plan first caller

Consumed W10's isolated renderer prerequisite9e53f43f as221885a6e; renderer
source is byte-identical to its previously strict-green A7 file, while the
standalone split and this new actual caller still require narrow checks. A8
registers conversation::plan_card and supplies the exact permission-keyed local
disclosure, Pending outcome and its existing action footer. The renderer owns
one Plan Markdown body and surface; A8 owns answer admission and no plan text
can authorize an action. The dock's whole-card height cap remains because it
shares vertical space with the composer; duplicate private Markdown/chevron
rendering and the old inner-plan cap are removed. Approved/rejected/canceled
stream outcomes remain A7-owned and no receipt is fabricated as a timeline log.


Snapshot reconciliation skeleton reuses the existing Live form list and EventSeq
without inventing an event or maintaining a second pending-request registry.
responses::retain_request keeps a record unless a snapshot's exclusive through
watermark covers its sequence and omits the exact typed request. None (old peer),
a boundary equal to the request sequence and older snapshots are conservative.
The helper is not wired until W23's checked Conversation.through arrives; the
owner must also refuse replacing a newer snapshot with an older one. Table tests
cover both request kinds and covered/uncovered/retained/successor identities.

W23 explicitly confirmed native Codex plan collaboration/implement/resume plus
provider-owned follow-up remains an unimplemented node prerequisite. DTO action
fixtures and card removal do not close it. Native will preserve offered option
ids and never infer an implementation prompt from a label. W23 is coordinating
that prerequisite; A8 remains incomplete until it is implemented or explicitly
scoped by the operator. W12 producer1720 was withdrawn before adoption; no pane
or resident producer has been brought into this branch.


Exact62c library runwp-side-narrow-20261115T092542Z-62c17767 compiled and ran
kit128/0/0, then link64pass/1fail (cargo stopped before view). Failure was the
no-retry probe treating the normal 24-byte ClientFrame::Cancel for the timed-out
RPC as a second input. The corrected test decodes the framed protocol and permits
at most one Cancel naming the original RPC id; every new input, different-id
control frame or repeated Cancel still fails. EOF or a bounded no-frame interval
ends the probe. Product timeout behavior is unchanged. App tests are bin targets
and were excluded by the --lib filter; no app runtime pass is claimed.


## Resident cutover adapter (awaiting the self-contained producer)

Existing assets are the retained TextField/source callbacks, ConversationArea's
form state, the response single-flight policy, W12's sole resident registry and
G4's authenticated connection lease. No second session/pane identity map is
introduced. The withdrawn1720 producer is not adopted.

The intended owner adapter uses a retained question-form entity carrying the
canonical SessionContext and emits typed form events back to Shell. Existing
TextField::with_source can target that entity, so immutable callback function
pointers do not need a new toolkit context/closure API or a global-selected-area
swap. Shell must resolve and validate the canonical resident before a form event
can affect its draft; moving/hiding the panel keeps its real input entity.

Request submission must use canonical FollowBinding/OperationTicket admission
and G4's actual connection lease; the present request registry's local correlation
policy cannot become an independent identity or authority manager. The exact
producer resource getters/retirement API must land before wiring this adapter.
A retired reply can release only its own admission, never clear a new draft,
close a pane or resend an answer.

Until a concrete, safely restorable request snapshot exists, active forms, IME,
in-flight answers and uncertain effects return the producer's typed Hold and
block eviction. A pending node request is retained beyond the provider turn
according to its declared lifetime. No snapshot is a clone/swap of globally
selected conversation state, and no hidden form claims focus or read ownership.
This section is an implementation boundary, not runtime/visibility evidence.


The legacy plan fallback must not interpret arbitrary tool input.plan as a plan
approval: doing so selected the plan-only body and hid an execute command or edit
detail. Until explicit permission presentation is available, only the existing
typed ToolKind::SwitchMode can supply that fallback. No provider tool-name match
or raw JSON tree is added. Regression fixtures put a plan argument alongside a
real execute/edit request and require ordinary tool rendering; the existing
mode-switch plan fixture remains the positive case. This is display classification,
not a new global approval policy or a synthesized provider implementation action.


Exact1bd native suitewp-side-narrow-20261115T100217Z-1bd7640e compiled current
dependencies but stopped before test execution because the real option-row Probe
used IntoElement as a return trait without importing its name (the parent uses
an anonymous trait import). The test imports the trait explicitly. No additional
runtime pass is claimed from this compile failure.

## Multiple pending requests

Paseo renders every pending request, while the inherited native dock and
ConversationArea retained only one question/permission operation. Reuse the
existing view step reducers, real source-aware TextFields and response port,
with one local retained form per exact QuestionId and per-request sending/failure
state. This local draft store is not a second node request/identity registry;
the timeline/live projection remains authoritative. Main conversation displays
all pending cards, with bounded scrolling above the composer; an explicit
request-scoped drawing entry point preserves an Inbox detail's exact selection.
No provider input parser or wire method is added by this extension.

The multiple-request frame retains real fields in question::Forms keyed by
QuestionId; its source-aware callbacks resolve that field's owner and every
production action takes an explicit question id. Single-form test probes alone
keep convenience wrappers; there is no first-request fallback in production.
Question validation, transport failure, sending and plan disclosure are scoped
independently. Typed pending-permission iteration reuses the fold's existing
index and does not scan history or create a second projection.

Place::response_scope defaults to ResponseScope::All for the main conversation;
Inbox can return One(Request) for an exact detail. The shared requests slot
limits the whole stack to half the viewport and scrolls, keeping the composer
reachable. Request state is pruned after real event/snapshot changes; drafts
remain live until their exact request no longer waits. The pending source still
needs W23's exclusive projection watermark before omission may retire log-backed
cards. A real two-editor keyboard/Enter test and independent plan/failure tests
cover sibling isolation; execution/strict proof is still pending.


## Canonical unary connection adoption

Merged package base6 through the app line, including G4's checked sole selected
connection producer. Response targets now carry its immutable ConnectionContext
from the selected Machine snapshot. Link calls only State::selected_connection
and SelectedConnection::call with the typed SendInput; it has no raw-client
lookup/retry/alias search. Original context travels with the request/ticket and
late contexts cannot be installed by the UI. Single-flight identity is the node,
session and typed request even if another alias/reconnect is selected.

These remain legacy call acknowledgements plus authoritative reread until W23's
reviewed rich receipts arrive. The real framed tests now admit through the
shared actor producer and add before-poll/after-send retirement cases. This
unary edge is not a live stream or full SessionResources adoption claim. W12
is adding the mandatory original-ticket retirement hook (Sending to Uncertain,
fields retained before replacement attach); that checked resource cutover is
still required before a consumer-ready A8 handoff.


Simultaneous permission cards must also isolate GPUI element state, not just
response maps. Both ordinary and plan outer scroll roots now use their exact
PermissionId instead of the inherited shared `permit-card` id. Descendant
button/detail ids remain scoped beneath that root. This reuses GPUI's normal
identity tree; there is no new UI state registry. A distinct/stable root identity
regression accompanies the fix; actual remote execution is still owed.

The two-editor Enter fixture explicitly opens its owning session before typing.
Without a shown session the response boundary correctly refuses to admit any
answer, so the intended exact-second-question offline result would never occur.
The test retains the real session boundary instead of bypassing it to manufacture
an expected error. This correction has source/static evidence only until the
queued native suite can execute.


Producer-consumption notes from the pinned Paseo source: PermissionActionButton
uses `variant === "primary"` solely for foreground vs muted styling
(agent-stream/view.tsx:1480-1511,1673). Secondary and danger do not add a red
button, and visual primary must not choose CLI grant/denial policy. The existing
CardLook::{Primary,Muted} therefore covers these provider variants without a
second renderer or a new shared kit option. Icon/response direction follows
explicit action behavior; identity remains the offered option id.

Question previews are visible for every option before selection, wrap as prose,
and are nonselectable (question-form-card.tsx:163-180). They must not be implemented
as selected-only output or reuse the current horizontally scrolling output box,
whose copy region, horizontal layout and fixed line virtualization have different
semantics. Consume the existing wrapped-text engine through the kit when the
reviewed ChoiceOption.preview producer becomes available; description selection
continues to claim its pointer event independently of the option label.


Local validation navigation now consumes the existing typed InvalidAnswer field
identity directly. Parsing a `Display` sentence to choose a step
coupled behavior to human wording and duplicated a decoder outside its owning
edge. The view exhaustively matches the current error enum and verifies the
field belongs to the offered form; the app still presents its validation message.
A regression puts a different field name inside explanatory text and requires the
typed field to win. No node or wire change is involved; new producer error variants
will deliberately require an exhaustive consumer update.


Queued option events are checked against the current offered values before the
pure pick reducer changes any draft or clears custom text. Previously any string
could become a choice, and every non-`true` string silently became Boolean false.
The current form's options (or the standard Boolean decoder) now determine
admission. Single/multiple choice and malformed Boolean regressions preserve the
previous draft on refusal; no response RPC is needed to reject a stale UI event.


The shared response boundary now validates the exact offered option or canonical
Question::check result before reserving a response ticket or queuing a command.
This protects every place using response_at, including a queued event from an old
schema, rather than relying on only the drawing handler to have validated input.
It follows the same log-first/exact-live fallback as the card. Invalid input gets
an exact-request safe failure and leaves the admission registry empty. Real Shell
regressions cover an unoffered permission id and a wrong-type Boolean answer.


## Staged concrete Requests owner

Adopted W12's isolated exact1d93 source (with its declared W1 constructor
prerequisite), retaining literal cfg(test) on the sole resource producer. The A8
resident_requests adapter stays literal cfg(test) too. It stores bounded original
binding/ticket/request correlation inside its existing ConversationArea; it does
not introduce another admission table or resource manager. The only admission
path validates the owning pending/sending request before calling the sole
SessionResidents::admit_operation(Request); settlement consumes that same ticket
before a caller can apply an exact-request result.

The mandatory retirement callback matches original binding plus opaque Requests
ticket, marks the owner effect Uncertain, retains real form fields/actions and
sets only that request's safe inline failure. A replacement follow cannot admit a
second response while this uncertain effect remains. Pending forms, including
untouched forms, conservatively Hold the resource bundle; no clean form snapshot
or dehydration is claimed. Live scoped factories, input/focus/keyboard routing,
network delivery leases, rich settlement reconciliation and product cutover
remain the coordinated producer/consumer work. The global product path is still
the only active runtime; this staged adapter is not live authority proof.

The staged form regressions retain an actual TextField entity, its text/drafts
and exact input source across hide/rebegin; they reject original-attempt late
settlement, readmission while uncertain, foreign Requests tickets and duplicate
settlement. Requests is made the only Hold before testing refused eviction, so
unrelated default owner holds cannot make the test pass. These tests do not
simulate keyboard/focus/IME or prove live scoped callbacks; those remain explicit
cutover requirements. Current execution evidence is still pending.

Exact cc120 native Clippy run wp-side-narrow-20261115T110400Z-cc120e9e completed
with rc101 after actual dependency compilation. Its only reported source error
was duplicate must_use on pending_permissions' Iterator return; Iterator already
carries the contract, so the redundant attribute is removed. The failure stopped
view compilation and is not app/link execution or a package pass.


The typed link Response port now redacts answer/option payloads in Debug while
retaining exact request correlation. Ask and the enclosing Command derive Debug,
so redacting only reply errors would still expose private question text through
actor diagnostics. A regression formats the actual Command::Respond for both an
opaque permission option and a typed text answer with a private canary. The typed
payload remains available to the wire call; only diagnostics are reduced.


Requests readiness uses the same effective log/live projection as the cards,
not raw snapshot array length. An exact resolution already present in the log
wins over an older live snapshot; its leftover local field must not keep a clean
resident permanently held. A real owned form plus Asked/Cancelled records tests
this boundary without mutating the snapshot or manufacturing a resolution.
Uncertain admitted effects remain a separate Hold until authoritative settlement.


The isolated shared FieldId producer dcbdbb018 now has V1 bounded source adoption
and exact requested Kit Clippy PASS; its four source blobs match prior Kit128
keyboard evidence. Lead and G2 received the immutable producer/evidence packet.
G2 subsequently delivered the selected multiline producer through580a7781,
with V1 bounded source/requested Kit Clippy and171 tests passed. G3 is the next
sole SearchField writer; W4 makes no concurrent field.rs edit. This shared prerequisite pass
is separate from the A8 package's latest still-pending all-target native checks.


The reconnect admission regression now uses one real ConnectionEpochs producer
to mint actual successive epochs for the same alias and a second alias. It tests
both node/session/request single-flight and refusal to settle with a changed
context, followed by a distinct retry ticket only after original settlement.
The previous alias-only fixture did not actually exercise a reconnect despite
its test name. This strengthens scoped evidence without changing product logic.


Exact7ff all-target Clippy reached Link and reported dispatcher length101/100
plus two test get_unwrap lints; no whole/pass inference follows that failure.
Reuse the existing prefs module for its own off-thread save/warning boundary:
move the SavePrefs body unchanged into prefs::persist and retain one exhaustive
dispatch arm. Task creation, await order, warnings and persistence behavior stay
identical. This avoids a dispatch lint waiver, another persistence module or any
size-record increase. No new test behavior is introduced by this extraction.

The two exact peer-retirement fixtures use test-only HashMap indexing instead of
get().unwrap(), as the reported get_unwrap lint requires. Fixture identity and
all before-poll/after-send retirement assertions remain unchanged; no product
indexing, retry policy or suppression attribute is added.


Official package base8 (91407173) is merged. The sole kit export conflict retains
both the shared button design additions and A8 Markdown label sibling. Owned
native Rust format, size, dependency boundary and token checks pass after the
merge; the current all-target Clippy run is pinned to its pre-merge feature head,
so it cannot establish this union or overall runtime acceptance. Shared field
proof, request-owner source and final package evidence remain distinct.

## Uncertain delivery is a retained response fence

W23 confirmed that Conversation.through covers actor-event membership, not
all durable delivery changes: equal-through and even a higher unrelated event
can accompany stale Pending after Answering/OutcomeUnknown. Per-owner read
correlation remains necessary, and a retained admitted/uncertain/settled response
cannot be reopened by ordinary snapshot presence alone.

Reuse the existing request-scoped failure owner as the current uncertainty hold;
no second registry is added. Unconfirmed keeps both question and permission
actions disabled, retains the actual question editor/drafts, rejects queued local
callbacks and refuses direct response_at admission before allocating a new ticket.
The spinner stops in favor of the translated uncertainty message. A known
no-effect refusal permits explicit correction; exact settlement/authority loss
may retire the old request. A duplicate success for an already consumed ticket
cannot clear uncertainty. New regression source exercises direct admission plus
stale same-request projection and the retained question editor; VM execution is
owed. Rich receipt classification and coordinated read-ticket ingress remain
pending their checked producers; no snapshot or timeline event is fabricated.

The staged Requests admission now additionally requires the original captured
ConnectionContext as an argument and compares its NodeId to the canonical
FollowBinding resident address before minting any operation. The same alias,
session id and request id cannot bridge a node-book retarget. A regression uses
the real context producer, checks OwnerConflict, retires the refused attempt and
requires zero admitted operations before the correct source can admit once.
Existing uncertainty also refuses this staged path before ticket allocation.
This parameter is identification, not a lease capability: product cutover still
requires the original same-lease DeliveryGuard and exact binding on data ingress,
with the sole resident minter/carrier. No source node is inferred from Shown or
the current alias, no second identity cache is added, and this remains test-only
adapter source with execution owed.

Retired Requests effects now release their local Uncertain Hold only when actual
records resolve that exact typed request. Other requests' resolution records,
ordinary snapshots and late transport replies cannot release it. Sending effects
retain their outstanding canonical operation until its original settlement; the
record hook only removes already-retired uncertainty and cannot consume another
owner's ticket. The hook is test-only alongside the staged resident resources.
A fixture admits records through the sole registry's fresh binding, retains Hold
for an unrelated resolution, then accepts the exact resolution and becomes Ready
while preserving the actual local editor. This is staged source, with execution
owed; live lease-checked ingress and coordinated product cutover remain required.

Run wp-side-narrow-20261115T134424Z-87a4d876 rebuilt the actual four native
crates, then failed in App on missing StatefulInteractiveElement for the stack,
GPUI debug_selector closures, one unnecessary card_at qualification, a test-only
SessionPlace import and one old single-form fixture call missing its request id.
The corrections use the existing GPUI/kit API and explicit-id controller; no
lint suppression or alternate renderer is introduced. This run has no whole or
runtime pass and predates base8 plus uncertainty/resident deltas.

## Input source belongs to the displayed question step

Paseo QuestionOtherInput captures qIndex in its onChange callback
(question-form-card.tsx:324-330). One retained native engine reused across every
step cannot provide that origin through FieldId alone: an old page's delayed
Change/Enter would target the replacement page's current step. Reuse the existing
kit constructor to replace only that page's editor when navigating to a different
step. Its new actual engine identity rejects callbacks from the previous step;
no independent id counter, subscription engine or shared-field change is added.
Drafts remain keyed by the canonical field name and restore when returning.
The current field survives ordinary repaint, session hiding and follow retirement.

A focused input transfers focus to the replacement. While its input method still
has marked text, navigation is held until composition completes, rather than
discarding marked input through a programmatic text replacement. This is an
explicit native navigation policy; OS IME behavior is unverified. The new source
regression captures a previous-step FieldId, navigates, sends stale Change/Enter,
and requires unchanged replacement drafts/no submission before restoring the
original page's draft. Runtime execution and native IME evidence remain owed.

Cursor also includes the actual FieldId alongside request and step. Returning
from step A to B to A must not reactivate clicks captured by A's previous render;
the reopened input supplies a new source identity without a parallel visit counter.
The same regression verifies that the old cursor remains stale after return.

Reselecting the active step also avoids programmatic text replacement while
composing, and unchanged text is not reapplied. A page refresh must not reset
selection or marked text merely because the same draft is already displayed.
This uses the same kit text/composition boundary; no new editor hooks are added.

The multi-editor keyboard regression separates fixture construction and retained
field assertions into small test helpers. Its real focus, typed input, Enter,
sibling-isolation and retained-engine assertions stay unchanged; the split keeps
the test within the same function budget as product code without a lint waiver.
This is source preparation, not a new test execution claim.

Run wp-side-narrow-20261115T143210Z-975c9de5 finished with actual native
cargo rc101. App reported a sibling test helper's private method, the viewport
arithmetic lint, two deferred callback semicolons and a collapsible validation
branch. The fixture now names its retained form directly, the existing pixel
conversion uses the same one-half ratio through mul_add, and callback/validation
formatting follows the strict lint rules. No lint waiver or size record changed.
This failed run does not prove runtime behavior or the later input-visit changes.

Exact345e Clippy ended rc101 with nine App-test lints: five unit-callback
semicolons already corrected in a2cd, a label-hit coordinate calculation, two
Rustdoc code names and explicit PromptQueue default construction. The remaining
fixes preserve the actual Markdown label hit (left+4 pixels inside the glyph,
not its surrounding whitespace) and all assertions. No runtime pass follows
from this failed strict run; current source still needs its exact VM checks.

Announced package base c3429952 is merged after its native1450/0/2 gate. Four
conflicts were earlier adopted W1/W12 source only: connection-epoch field order,
the literal resource gate comment, resident minter registration/storage and the
six-owner list. The newly reviewed canonical versions replace those old producer
blobs; A8 forms, uncertainty and request-owner adapters are retained. Resource
module/Areas storage remain test-only. This union does not enable product resident
storage, guarded follow, runtime workspace lookup or durable response receipts.

The uncertainty retry classification matches every Problem variant explicitly.
A future error variant must choose its policy at compile time instead of silently
becoming retryable through a negative comparison against Unconfirmed. Existing
variant behavior and independent admission/provider checks remain unchanged.

Run wp-side-narrow-20261115T152659Z-acca7c63 compiled the updated union,
then stopped before executing tests: the announced line boxes Command::SavePrefs,
while A8's earlier behavior-preserving persist extraction still accepted Prefs.
The private helper now accepts the exact existing Box<Prefs> and moves that same
box into the existing blocking save closure. No unboxing/extra allocation, save
ordering, path or failure behavior changes. The original test run has no runtime
counts or pass claim; its compile failure requires a corrected-head test run.

## Actual narrow native runtime proof

Exact730e2db5, run wp-side-narrow-20261115T155809Z-730e2db5: actual fonts
and native cargo rc0,1492 passed/0 failed/2 existing ignored. All four crates
were compiled; App binary tests were included (no --lib). Logs confirm actual
multi-field keyboard/Enter targeting, Markdown label-versus-description clicks,
stale step source/cursor refusal, retained uncertainty, staged source-node checks
and exact-record retirement recovery, plus selected-connection before/after-send
fences. Evidence packet: a8-native-730-evidence.json in the worker handoff bundle.

The two existing ignored cases require private capture fixtures (pairing browser
words and terminal states); they are not new A8 exclusions. This is a narrow
runtime suite, including baseline tests, not a whole gate or review. It proves
neither full rich receipt/provider activation nor live resident/lease ingress,
OS IME, visual captures or private CLI behavior. Those remain incomplete.

## Official base 82ca07a7 union

The full A8 tree now merges announced package base 8e, after the native730
runtime run completed and before any final whole-package gate. The only source
conflict was two identical preference-save extractions. The union keeps the
line's canonical dispatch::save_prefs helper and removes A8's private
prefs::persist copy; the same existing boxed preferences and awaited blocking
save/warning body remain. The Respond arm remains in the one dispatch table.
The resource/minter staging attributes remain unchanged. This union requires
fresh narrow verification; native730's1492/0/2 is prior-source evidence.

## Coordinated Requests cutover contract

The portable owner prerequisite on base82ca is a staged kernel, not a second
live response manager. Its new canonical view::requests::Request identity must
replace this branch's private response identity through a re-export. The actual
question fields, drafts and page cursors remain in ConversationArea; W12 alone
adds SessionResources.requests and its readiness/retirement/ingress joins.
The older literal-test RequestEffects adapter is replaced, not enabled beside
this kernel. Product global response Registry retirement remains a coordinated
migration requirement; the portable producer does not perform it.

The product mutation carrier must retain original ConnectionContext plus the
exact FollowBinding and sole OperationTicket across queue, call and reply.
Transport's internal RPC request number is not another UI admission manager.
Local callbacks resolve the SAME token and claim to the full CURRENT
Visible/Focused context; network replies never refresh their captured context.
Before effects, the owner checks original node, exact current request and its
actual offered option or retained FieldId. Prepare, sole ticket allocation and
Sending commit are one synchronous operation, without an await. A failed mint
leaves the editor and Sending state untouched.

On return, the owner verifies its exact correlation before canonical settle
consumes the ticket; only its successful callback may mutate the original form.
A typed no-effect refusal may release Sending for correction. An uncertain
wire outcome retains the owner fence and editor and cannot automatically retry.
Retirement converts its exact Request tickets to Uncertain before replacement
launch. Only an admitted exact resolution can reconcile a retired uncertainty;
a snapshot's Pending state or omission, even with a higher unrelated through,
cannot authorize resubmission. Full rich receipt/projection activation still
waits W23's checked producer and W12's actual original-lease carrier.

This contract is preparation for the coordinated consumer. It adds no live
resource field, no new allocator, no transport call and no product staging
promotion. Current staged primitive tests are not authority, populated bundle,
keyboard/IME, whole-package or user-flow proof.

## Official base 9 and frozen portable prerequisite

Main A8 merges announced db42d9c5 (native1510/0/2 baseline, root9c150c6e)
without conflicts, preserving A8's own registration and FieldId exports. This
brings only the checked body/identity vocabulary; full permission/question rich
fields, receipts and provider runtime are still absent and are not activated.
The newer shared body renderer remains held for its own checked handoff.

By the coordinator's explicit selected-producer ruling, the separate portable
Requests packet remains frozen on82ca at3eaa330f0 for its queued strict and four
GPUI tests. It does not move to base9 before those results. W12 and the lead
verify the consumer union on their current line when adopting that exact packet.
This main A8 union has no new narrow or whole-gate proof yet; baseline totals do
not replace own changed-source verification.

### Rich-state eligibility remains a consumer amendment

The frozen portable kernel is checked against the base82 request DTOs, which
have no authoritative delivery-state field. When W23's full rich contract lands,
the real consumer must require canonical Pending BEFORE ticket admission.
Answering and OutcomeUnknown are inspectable but never actionable, regardless
of the absence of a local operation. The current kernel's old-shape checks and
four primitive fixtures do not prove this rule. The full request/card schema
guards and eligibility amendment belong in that same checked activation; no
state may be inferred from labels, timeline completion or current alias.

## Announced base10 resident preparation

Main A8 merges announced3e7820e3 (baseline1535/0/2), preserving both the checked
A7 resident preparation and A8's existing own response registration. The only
merge conflict was their adjacent module registrations. The imported resident
module/re-exports stay literal cfg(test); no global/resource/follow or lease
staging is promoted. Legacy Seat/Place remain as the current product API.
This merge is not adoption of the full unreviewed A7 branch or its body cut,
and it does not import the selected Requests producer into product globals.
A8's own stale-source and exact-resolution intake hooks remain present.
Fresh combined-source checking remains owed before claiming this union works.

## Selected portable Requests prerequisite result

V1 row258 finalized bounded staged source plus requested narrow PASS for
executable3eaa330f0 and docs-only successord18624d69, on82ca. Exact strict run
wp-side-narrow-20261115T170412Z-3eaa330f is0 (View/App requested; fresh Kit/App,
Viewcached). Actual App-bin runtime run
wp-side-narrow-20261115T180719Z-3eaa330f is4/0/0/105filtered, test-threads1,
with fresh View/Link/Kit/App compilation. The initialc3d API-error run remains
negative history, not test evidence. This is separate from full-A8 source730's
1492 prior-runtime result; current main A8 has not inherited a whole-package pass.

W12 selected only this13path native prerequisite into its resource-owner prep
union, with all staging gates retained and its own combined compile/source
checks still owed. W12 owns actual bundle field, factory, readiness vote and
atomic admission/retirement/ingress installation. Main A8's older staged adapter
has not been promoted or silently substituted. No populated resource lifetime,
original-lease authority, painting, keyboard/IME, live wire, rich-state admission,
CLI mutation, captures or complete A8 acceptance follows from the four cases.

## Resume on package base13

Main A8 merges announced2ebeedf9 (baseline native1614/0/2 and app wasm32 clean)
after all owned earlier narrow runs completed. Three conflicts were adjacent
registration additions only: preserve A8 Respond/Responded and the line's
History command/event/handler. Both paths keep their existing bodies; no
resource/Seat/body staging is promoted and no node head is merged directly.
The selected primitive producer and caller companion remain separate, frozen
artifacts. Fresh combined A8 narrow checking is required; old730 runtime and
base13's own totals do not certify this source union.
