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
  Proposed docs `4de3f335` are not a consumer-ready implementation.
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
