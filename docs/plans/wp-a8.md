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
/tmp/work/m12-review/w4-497-shared-field-narrow-audit.json. Package base6036 has
byte-identical kit, native manifest and lock; A8 merged it at541b5392.
This is bounded editor-policy proof, not RPC/authority/OS/IME/UI parity proof.

Latest local all-four-crate narrow on6761865c exited101 for four missing named
field doc comments in the new public link Response enum. Those comments are
added before merging announced package base4. The run stopped before checking
app, so it is not a passing compile/test result for A8.
