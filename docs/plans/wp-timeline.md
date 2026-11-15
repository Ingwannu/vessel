# Conversation stream parity

Status: survey complete; implementation skeleton next. Updated 2026-11-15T05:47:13Z.

## Reference and asset check

Reference snapshot: Paseo 0.15.4. Paths below are relative to `packages/`.
Read `protocol/src/agent-types.ts:215-487`, `app/src/agent-stream/view.tsx:1513-1738`,
`app/src/components/question-form-card-core.ts:30-177`,
`app/src/components/tool-call-details.tsx:147-178`, and the timeline/messaging survey.
The survey provides the remaining source pointers below; pointers marked survey-only
must be opened before their corresponding implementation is changed.

Decision: **modify/reuse** `spikes/native/view/src/timeline/`, `link/src/sending.rs`
and `app/src/areas/conversation/`; they already own folding, stable identity,
virtual rows, requests, queues and optimistic messages. No new dependency or UI
kit control is needed. Standard serde decoders and existing output/Markdown/lines
controls suffice. A second stream model would duplicate sequencing and stale-answer
protection; a general JSON renderer alone loses tool semantics.

## Complete item inventory against the starting native app

| Paseo canonical item | Native source / behavior at start | Remaining difference |
|---|---|---|
| user_message | timeline/agent.rs TurnStarted + UserMessageChunk; bubbles::person; pending tail | Missing client message identity / receipt; optimistic matching uses text |
| assistant_message | MessageChunk merges adjacent text; bubbles::message Markdown | No per-message provider identity; reveal and bubble already exist |
| reasoning | ThoughtChunk merges adjacent text; entries::thought disclosure | Uses plain output; no independent autoExpandReasoning preference |
| tool_call | ToolView indexed by ToolCallId; partial update fields retained; tool::card | Generic input/output boxes; need typed detail, canceled status, provenance |
| todo | Plan entries + plan_activities -> plan::row / task markers | Missing stable task id / activeForm in wire |
| error | TurnFailed -> failure alert, duplicate failure reply suppressed | No distinct canonical error item |
| notification | None | No severity-bearing event contract |
| compaction | None | No loading/completed/trigger/preTokens contract |
| plugin | None | No plugin identity/version/data contract; client plugin renderer belongs to plugin package |

Native-only Ready, Status, Parking, Profile, Model and Forked entries are quiet
metadata; TurnEnded is the turn footer; Permission and Question are control records
with their actionable card docked above the composer. These are deliberate Vessel
representations, not additional Paseo canonical item kinds. Files: timeline/entry.rs,
app conversation/entries.rs, footer.rs, permission.rs, question.rs, plan.rs.

## Every tool detail and status

| Paseo detail | Starting native behavior | Work / boundary |
|---|---|---|
| shell | Execute + raw input JSON / text output | Decode typed command (string or argv); terminal body |
| read | Read + raw input/output | Keep line offset / path when canonical contract arrives |
| edit | Edit + ToolCallContent::Diff; all old then all new | Structured diff body already exists; canonical unified diff needs mapping |
| write | Edit / Other, not distinct | Needs node detail discriminant |
| search | Search + raw JSON | Canonical query, paths, web results and annotations needed |
| fetch | Fetch + raw JSON | Canonical URL / result needed |
| worktree_setup | No canonical detail | Node marker; bootstrap UI owned by workspaces |
| sub_agent | No canonical detail | Node child identity; child navigation owned by lifecycle |
| plain_text | ContentBlock::Text or raw string | Canonical label/icon needed |
| plan | Permission parser reads raw input.plan; tool card is generic JSON | Preserve plan Markdown and outcome in stream |
| unknown | json_text(input/output) | Retain opaque fallback; no arbitrary tree inspection |

Native statuses Pending/InProgress/Completed/Failed retain update fields. Paseo
running/completed/failed/canceled needs explicit canceled and explicit failure text.
Paseo reference: components/tool-call-details.tsx, components/plan-card.tsx,
components/message.tsx:2251; survey C §§1.2-1.3,14.5-14.7.

## Messaging, questions, approvals and gates

- Queue: existing node-owned PromptQueue; edit (revision checked), remove, move,
  clear/resume, steer and interrupt. Difference from Paseo: its human queue is
  local volatile state; Vessel deliberately shares the queue across clients.
  Reference: composer/actions.ts and runtime/host-runtime.ts:2563 (survey D §7).
- Send behavior: existing session/delivery.rs uses steer by default, interrupts
  when unsupported and changes Queue to Interrupt behind a pending permission.
  Cancel names the running turn's EventSeq. Keep these capability gates.
- Receipt: tail.rs has one text-matched optimistic row; no clientMessageId in
  prompt/turn echo, no idempotency key/conflict/outcome-unknown receipt API.
  Reference: composer/submission/model.ts and server/src/server/message-receipts/.
- Permission: provider option ids/order/scopes retained; sending disables buttons;
  refusal stays inline. Plan request reads input.plan, shell reads command, edit
  uses diff. Missing explicit kind, title/description, plan metadata and provider
  action intent (implement / implement_resume / dismiss). Mode currently only a
  SwitchMode tool category. No arbitrary mode change may be invented by a button.
- Questions: native typed fields, per-step drafts, radio/checkbox, custom-answer
  pairing for Claude, Next/Submit, validation, stale answer protection already exist.
  Missing auto-advance single-select, Markdown question text, spinner; preview,
  placeholder, allowEmpty, dismissLabel and generic custom companion need wire.
- Nonblocking question: must remain pending after turn completion and reconnect;
  answer steers into the identified current turn or starts a follow-up when idle.
  No such delivery/lifetime/persistence contract exists in the current node.
- Gates: existing abilities.steer + per-request action ids remain authoritative.
  New event support must use assigned feature constants and typed Unsupported
  behavior against older peers; never infer support from provider display names.
  Notification/plugin event feature negotiation and projected timeline capabilities
  are Paseo-specific; Vessel's sequence-based durable log remains authoritative.

## Node handoff requirements (separate protocol work)

1. Typed additive canonical timeline detail (all 11 variants), explicit canceled,
   error/notification severity, compaction identity/status/trigger/token count,
   plugin identity/version/opaque bounded payload; tool provenance for agent messages.
2. Explicit permission kind + display text/description + plan Markdown + provider
   action id/behavior/intent. Answer must preserve id and never fabricate a mode.
3. Question display fields, generic choice/custom grouping, allow-empty semantics,
   nonblocking lifetime + authoritative pending snapshot + provider persistence.
4. Prompt request client id echoed in canonical human row, stable send receipt,
   conflict and uncertain outcome distinguished; no retry that duplicates a prompt.
5. Capabilities / since constants assigned by the integrator; old peers refuse the
   unsupported action by type. No provisional numeric protocol/schema assignment.

No `crates/` modifications in this package tree. New node contracts are implemented
in the separate timeline protocol package (ADR 0084) and consumed after review.

## Structure and decision log

- Purpose: all existing stream entries remain readable and actionable, with typed
  presentation shared by normal chat and embedded side conversations.
- Constraint: the native base is being brought forward to protocol 23; root changes
  are a separate reviewed package. Shared shell/link files receive registration only.
- Alternatives: rewrite the stream; parse provider-specific names in GPUI; invent
  missing wire calls. Each duplicates ownership or misrepresents support.
- Choice: extend view timeline types + pure presentation rules; existing link calls
  and conversation area render them. Unknown payloads retain their original text.
- Why: one folding and identity path; testable semantics without a window; finite
  node request deadlines remain owned by link.
- Effects: additional typed render models, no new dependencies; features requiring
  unavailable canonical fields stay hidden until the node contract is present.

## Validation and resume point

Done: item/status/detail inventory; current/native-node boundary survey.
Next: skeleton commit; typed display and plan outcome; question interaction fixes;
consume reviewed node contract; captures once; whole native gate once; one review.
Not yet verified: compilation, runtime, captures, real provider behavior.
