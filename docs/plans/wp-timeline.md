# Conversation stream parity

Status: stream skeleton committed; approvals and questions handed to their package owner. Updated 2026-11-15T05:47:13Z.

## Reference and asset check

Reference snapshot: Paseo 0.15.4. Paths below are relative to `packages/`.
Read `protocol/src/agent-types.ts:215-487`, `app/src/agent-stream/view.tsx:1513-1738`,
`app/src/components/question-form-card-core.ts:30-177`,
`app/src/components/tool-call-details.tsx:147-178`, and the timeline/messaging survey.
The survey provides the remaining source pointers below; pointers marked survey-only
must be opened before their corresponding implementation is changed.

Decision: **modify/reuse** `spikes/native/view/src/timeline/`, `link/src/sending.rs`
and `app/src/areas/conversation/`; they already own folding, stable identity,
virtual rows, requests, queues and optimistic messages. Existing UI kit controls are reused; the bounded diff adds the already
transitive `similar` library as an explicit view dependency. Standard serde decoders and existing output/Markdown/lines
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
- Effects: additional typed render models and the explicit diff dependency; features requiring
  unavailable canonical fields stay hidden until the node contract is present.

## Validation and resume point

Done: item/status/detail inventory; current/native-node boundary survey.
Next: skeleton commit; typed display and plan outcome; question interaction fixes;
consume reviewed node contract; captures once; whole native gate once; one review.
Not yet verified: compilation, runtime, captures, real provider behavior.

### Question form implementation

Paseo source read directly: `components/question-form-card.tsx:383-420`
(single choice advances but final step requires Submit),
`components/question-form-card-core.ts:82-177` (custom answer and dismiss rules).
Question Markdown uses the existing kit renderer; the actual selected Submit or
Dismiss action spins while both disable. Late text callbacks cannot mutate a
submitted answer. Inline refusal preserves drafts and allows a retry.
Tests cover single-select advance/deselect, multi-select staying put, no auto-submit
on final step, invalid indices, and actual retained text input across navigation
and locale change. Default values supplied by generic typed fields remain honored
(a deliberate difference from Paseo's empty selections; do not discard provider defaults).

Skeleton narrow run: native-fonts rc 0; clippy found one doc backtick omission in
plan_card.rs, fixed with the question implementation. This was a narrow check,
not the package gate. No whole gate or capture has been run yet.

### Scope correction

The canonical package table now assigns this plan **A7 only**: all timeline kinds,
tool details, plan/todo/compaction, and turn footer. The approvals/questions package
owns pending cards, actions, mode policy, question forms and CLI permit; composer
owns send behavior. Prior question changes have been handed off as a patch and
removed here with a forward commit. Their narrow clippy passed; no test or capture
claim transfers with the handoff. The stream exposes one reusable Plan Markdown
renderer so the pending approval card and settled plan have one body.

### Plan renderer decision

Reference read directly: `components/plan-card.tsx:10-101`. The shared
`app::areas::conversation::plan_card::card` receives text, outcome, optional title /
description / footer, typed expansion and an explicit toggle callback. Caller owns
actions; viewing or expanding never sends an approval. `PlanDisclosures` keys local
choices by tool identity and outcome: text updates retain disclosure, a changed
decision resets it. A permission transition invalidates the earlier proposal row
so virtual-list measurement does not leave a collapsed-height box around Markdown.
A pending plan is hidden in-stream while the approval dock owns it, and is excluded
from tool groups. Legacy detection is restricted to SwitchMode + typed input.plan;
canonical Plan detail will replace that fallback when available. The 44-point phone
header is a touch-target accessibility difference from Paseo's 24-point pointer row.

### Diff presentation decision (before implementation)

Reference read directly: `utils/tool-call-parsers.ts:1-240`; old/new text uses a
line edit script with unchanged context and preserved blank lines; unified diff
headers and add/remove/context lines retain their meaning. Existing native
`timeline/diff.rs` instead removed every nonempty old line and added the whole new
file; `workspace/diff_rows.rs` consumes already parsed node hunks and cannot diff
agent text. Reuse the existing timeline output contract, extend its side enum, and
use `similar` 2.7.0 without default features for the bounded line edit script.
`similar` is Apache-2.0, approximately 508 KiB of unpacked source, already in the
native lockfile through another dependency; the direct dependency adds no new
package or transitive dependency. Standard Rust has no diff algorithm; a fresh
quadratic LCS table or hand-written Myers implementation duplicates a tested
library and adds review cost. Use bounded input and output line counts; no I/O,
wall-clock timeout, new crate edge or provider-specific matching in the screen.
Difference: word-level syntax segments remain a later enhancement; line changes
and blank/context lines must be correct first.

### Turn collapse skeleton

Read `agent-stream/turn-collapse/model.ts:1-150` directly. Closed turns fold the
work before the last assistant message into the loaded trigger; trigger, last reply
and footer remain visible. A live newest turn and a turn with only reasoning do
not fold. Earlier assistant prose is foldable; partial history without a loaded
trigger does not invent one. `TurnFolds` publishes a single host/range map for rows,
find and prompt reveal. Preferences will use the existing Appearance/PreferenceChange
path: reasoning (default collapsed), Full / Collapse tool calls / Collapse turns,
then the outline setting. No row is exposed before its behavior consumes it.

### Turn footer correction

Read `components/message.tsx:587-682` and `components/assistant-turn-footer-label.ts`
directly. A missing visible start uses the completion timestamp; copy and duration
stop at the previous response boundary. The pure TurnFooter model pins partial
history and resumed-response cases. Desktop hover swaps to the timestamp; compact
tap reveals it for the reference's 3,000 ms. Both captions reserve the longer label
width. Repeated taps use a fresh local token so a stale expiry cannot hide a newer
reveal. Footer state is display-only and sends no node request.

Narrow view test run: 846 passed / 1 failed (About dependency inventory omitted
the newly direct `similar` dependency); the inventory and its English/Korean role
were then corrected without changing its assertion. Plan tests also passed 3/3
on an immutable source snapshot. The package whole gate is still pending.

## Resume point after the package-base merge

Merged the published native package base `8d5513e4` as `54a7f739`. Shared link
registration is now in command.rs/event.rs/dispatch.rs; shell.rs and link/lib.rs
stay registration-only. About inventory conflicts retain the base's slice and
role table while including the new diff library. A later footer narrow check
found test-only DateTime arithmetic and an unused import; both are corrected.

Implemented: item inventory, plan decision/disclosure model and shared Markdown
card; bounded line/context/unified diff; closed-turn fold model; response-bounded
copy and partial-history timestamp with compact reveal. Model tests for these
passed in the narrow view run; the dependency inventory failure is retained and
its attribution fix still needs a clean run. No full gate or capture is claimed.

Next: finish the canonical tool body/display adapter, consume the reviewed
canonical event/receipt contract, and connect turn/reasoning preferences once the
Chat translation base arrives. Pending approval and question forms are owned by
the approvals package; assistant link/image/copy actions by the composer package.
Plugin items retain opaque data and show an explicit unavailable-contribution
fallback until the plugin runtime supplies its validated renderer.

### Shared timeline glyph decision

Existing kit `Glyph` owns registration, paths, vectors and raster tests. The
coordinator assigned W10 the four missing glyphs, so the native renderers can
reuse one asset source instead of keeping unrelated icon fallbacks. Wrench, Eye,
MicVocal and Scissors append to the inventory; existing vectors are unchanged.
Source is Lucide itself at immutable revision `55782beff3a0133df79c417eb68f96e8f19e2404`,
not Paseo's files. The ISC notice is retained in `kit/licenses/lucide.txt` and the
only adaptation is the existing kit stroke width. No control or screen asset is
introduced. Pending canonical tool rendering keeps its explicit fallback until
its typed consumer is integrated.

### Turn projection and virtualization

Paseo `agent-stream/turn-collapse/model.ts:76-159` retains the trigger and final
reply, never folds an open latest turn, and keeps compaction visible. Native
`TurnFolds` computes disjoint work ranges from the same complete followed log;
`Rows::fold_turns` filters only mounted rows. Human steering remains visible
inside a native turn: its authoritative `TurnStarted`/`TurnEnded` records retain
one response boundary. A closed turn can remove rows earlier than the footer
that just arrived, so `Rows::first_difference` identifies the first measurement
that must be replaced. Two regression tests pin entry identity, that earlier
invalidation, reopening and human-steer visibility. Compaction retention waits
for the reviewed canonical event contract rather than inventing a placeholder.

### Reasoning default and explicit disclosure

E2 approved `Appearance.reasoning: ReasoningExpansion::{Collapsed,Expanded}`
(default Collapsed), persisted through the existing lenient Preferences and
PreferenceChange pipeline. Settings startup/change applies it to the session
and fork origin; embedded conversations call `ConversationArea::set_reasoning`
before ingesting records. The Chat control waits for L's shared getter base.
Paseo `agent-stream/presentation.ts:177-185` and its regression at
`presentation.test.ts:747-754` keep automatically expanded reasoning outside a
collapsed tool group. `Folds::with_reasoning` does the same. Storing a toggled
bit against a changing default inverted the person's choice when a running tool
completed; explicit disclosure values now retain the actual choice across tool
completion and default changes. Plan outcome disclosures remain independently
owned by the shared Plan renderer contract. Tests cover local lenient fallback,
roundtrip/change isolation, group exclusion and explicit-choice persistence.

### Turn disclosure native integration

The same `ConversationArea` now owns turn disclosure on all seats. Summary rows
use the existing work-summary renderer, expand to the complete original entries
and draw a shared-token indent rail. `reveal_turn(entry)` opens the owning turn
before existing find/record row lookup. Record ingestion compares old/new row
projections so a footer that folds earlier work invalidates the earlier rows.
Origin conversations inherit the same display and reasoning defaults.
The existing capture override accepts `VESSEL_APP_CONVERSATION=turns` for proving
this integration while the shared translated Chat display adapter is pending;
no extra settings store, node call or untranslated Settings row is introduced.
Normal Full/CollapseTools behavior remains available through the existing choice.
V1 recorded custody of the four glyph names at7368d1d8, with no collision and
without implying a geometry/licence/raster/gate/UI review.

Each of the four attributed vectors also embeds that notice as SVG metadata,
so an asset distributed without the source checkout retains its notice. The
existing raster test checks that metadata survives the AssetSource boundary.
