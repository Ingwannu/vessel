# Conversation stream parity

Current integration boundaries and exact evidence are indexed in
[the acceptance map](wp-timeline-readiness.md); the sections below retain the
source survey, implementation rationale and earlier scoped execution history.

Status: A7 source includes announced app base8e `82ca07a7` (mergec0d3cc052).
Actuale879 pacing/kernel App114 + Kit208 =322 passed/0 failed/2 existing ignored;
later Markdown helper consumption and this base merge are outside that source.
View stricta415 passed; View tests1025/1 failed only the About registry, now fixed
and awaiting its next immutable run. Full App/Kit strict still waits for S2's
canonical glyph inventory in announced base9. W23 rich fields/echo/receipts,
W12 live resident cutover, K syntax and scoped Markdown host actions remain held.
A8 approvals/questions belong to W4. Minimal resident preparation has V1 row252
bounded staged source/narrow PASS; its line merge9652a72b is not yet green.
No whole gate, captures or V4 package pass. Updated 2026-11-15T16:48Z.

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

No W10-authored `crates/` edits. Node contracts belong to the separate timeline
protocol package (ADR 0084); only its committed standalone DTO dependency is
consumed here so far, as described below. Runtime/field activation remains pending.

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

Source implemented: shared outcome-scoped Plan body, bounded line/unified diffs,
turn and tool grouping, explicit reasoning defaults/disclosures, completion
footer/copy boundaries, compact tool/group sheets and stable reading bookmarks.
Protocol-dependent detail/item/receipt adapters remain staged until W23's
combined source-consumable hash; resident adapters await W12's producer API.

Scoped evidence: native view library880tests passed (the subsequently added
precise context-boundary and distinct-file summary tests were not in that run);
kit asset/raster/notice tests3passed. Compact/bookmark adapter at `f8aaaa1d`
passed strict all-target native app/view/kit Clippy in remote run
`wp-timeline-narrow-20261115T081535Z-f8aaaa1d`; this excludes subsequent base
merges and Chat controls. The controls at `a6a6dfe48` passed fonts but failed
strict Clippy on a test fixture's get/unwrap idiom in run
`wp-timeline-narrow-20261115T084001Z-a6a6dfe4`; the fixture is corrected without
changing product behavior. L separately passed the exact Chat EN/KO wording and
data shape at `a6a6dfe48` (retained wording-review result.json);
this is not timeline behavior or package approval. No whole package gate, captures
or V4 review yet. Real providers and new
wire activation remain unverified; no real agent was run.

The corrected Chat slice `e4ff42120` passed strict all-target app/view/kit Clippy
in `wp-timeline-narrow-20261115T084410Z-e4ff4212`. Subsequent standalone-group
state and source-bound changes need their own execution evidence. The local
conversation-state run failed because base4 added three embedded Mono faces;
the existing pinned `fonts.sh` regenerated those assets in this worktree's
private artifact cache, and a fresh state run is queued. No font source was copied
from another worker and no production file permissions changed.

Next: finish source-green narrow/model checks; consume the shared Chat, canonical
protocol, Markdown callback and resident seams; then owned fake-fleet/GPUI
captures once, one whole native gate and one V4 package review.

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

### Shared Plan title localization

The new decision titles live in `locales/{en,ko}/timeline-plan.ftl`, following
`locales/README.md`, rather than extending legacy bilingual Rust tables. The
shared renderer receives cached `catalog.timeline_plan()` words; its text,
outcome, disclosure and footer ownership are unchanged. An exhaustive adapter
and EN/KO whole-title test cover all five outcomes. Permission wording remains
A8's. Existing translator generation supplies these getters without a second
word table or render-time resource lookup.

Turn summary fallback copies Paseo's `turnSummary.details` whole EN/KO values
(`summary-line.tsx:23`, `resources/{en,ko}.ts:2105/2090`) through isolated
`timeline-turns.ftl`. Prose-only folded turns never render a blank button.
The single timeline post-entry hook delegates summary/rail rendering to
`conversation/turns.rs`; no duplicate list or work-summary control is added.

Turn projection moves the previous row map instead of cloning the full mounted
index on each stream batch. Disclosure splices preserve already mounted tail
rows and their bookkeeping; only a full list reset drops/rebuilds the tail.
This avoids duplicate pending/working rows after a turn disclosure.

A GPUI state regression drives actual `VirtualList` splice bookkeeping through
an open turn, completion that hides earlier rows, find-style reveal, disclosure
close and working-footer removal. It asserts stable final-answer entry identity
and exactly one mounted tail throughout, including the brief interval when
external running state has not caught up with the completion record.

Local glyph tests initially found an ambiguous SharedString assertion and
missing generated fonts; the assertion now names SharedString, and the exact
pinned font packages are prepared in the private artifact cache through the
existing font script. The native ignore rule also covers a generated-font
symlink, preserving the workspace's HDD artifact policy without shipping it.

The L-approved Chat reasoning label/hint are prepared in isolated
`chat-reasoning.ftl` resources with EN/KO whole-sentence checks. They do not
publish a second Chat display adapter; the control still waits for the shared
Chat base. `set_reasoning` is idempotent so applying the current preference to a
resident or embedded conversation never discards its scroll position.

Coordinator assigned the multi-session resident resource registry to W12. A7
will adapt ConversationArea/Seat behind W12's V1-reviewed keyed-context contract;
no parallel SessionView/composer manager is introduced here. Until that contract
lands, this package's existing single-session and embedded rows remain its
consumer boundary.

### Failed response footer

Paseo `agent-stream/layout.ts:75-110` finds an assistant-message or tool fallback
for a completed footer; it does not require successful completion. Native now
keeps that footer for a failed response with observed assistant/tool work.
`TurnCompletion::{Ended(StopReason),Failed}` retains the distinction without
inventing a stop reason. The existing failure card says why; the footer keeps
factual timing/copy, suppresses any provider reply already withheld as duplicate
failure text, and never borrows an earlier response. An error-only partial log
without a response/tool host gets no synthetic footer. Regression tests pin
failed response boundaries, explicit failure state and copied content.

Base 2 (`6036eeb8`) merged as `f3ef6446`: Machine.node identity and shared
field events/editability are consumed unchanged. A narrow check then caught
missing per-key translator comments, an ambiguous asset-test string type and
an app test attempting JSON decoding without a dependency. These are corrected:
every new Fluent key owns context, asset expectations name SharedString, and
the app fixture constructs typed events without adding a JSON dependency.

### Settled tool outcomes and turn-scoped identity

Paseo `types/stream.ts:1147-1163` keeps failure sticky and completion stronger
than late running updates. Native tool updates and repeat announcements now do
the same; pending cannot regress an observed running call. The canceled case
awaits the reviewed enum variant. Tests cover terminal outcome preservation.
W23 confirmed that whole-session ToolCallId uniqueness is not established:
Codex drops provider turnId and reused/late Claude ids are not constrained.
Canonical turn scope is a node gap now owned by W23. Native must consume the
announced typed scope when available; inferring a late update's turn from only
the latest local EventSeq would be an unsupported claim. The legacy identity
fallback remains explicitly unscoped until then.
W23 also confirmed the compaction reducer rule: completion upgrades earliest
outstanding loading, even nonadjacent; loading always creates a row. Its node
comment is being corrected, not a new competing presentation union.

### Context warning boundary

W5 identified the context-ring threshold difference. Paseo
`components/context-window-meter.tsx:63-69` uses danger strictly above90%,
warning from70%. `timeline/meter.rs` now compares the original token ratio by
u128 cross multiplication, leaving exact90% warning and detecting90.001% as
danger even when the displayed whole percent still reads90. This reuses the
existing context model; no quota meter or second store is added. Boundary tests
cover70%, exact90%, just above90% and maximum-width token counts without floats.

### Stable reading bookmark

Existing VirtualList exposes its logical top row and exact pixel offset, but
only reset/reveal methods; changing Chat display/defaults reset the reader to
the end. `ListBookmark` now captures value data only, preserving detached versus
following state and fractional offsets. Native rebases its row using EntryKey
(including origin entries and folded hosts) after rebuilding rows, then restores
that bookmark. This reuses GPUI's existing follow/scroll APIs and retains no view
or transcript clone. It is also the exact scroll-value seam needed by W12's
resident snapshot contract. Kit regressions cover remapped rows, empty lists and
resuming at the new end; owner context still controls when restoring is allowed.

### Compact tool details

Paseo `message.tsx:2251`, `tool-call-sheet.tsx` and the surveyC14.5 open a compact
tool/group in a bottom sheet, leaving forced-expanded reasoning inline. Native
now uses the existing kit dialog's compact bottom-sheet path, the same tool
body/output controls, and the same per-entry disclosure state for group members.
The sheet is mounted outside virtual rows, so newly streamed output cannot
unmount it. Plan cards remain inline. Empty settled details use the exact EN/KO
`toolCallDetails.empty` sentence; pending empty bodies use shared skeleton shapes.
The dialog callback now accepts a clonable typed-context closure on its existing
render path, preserving every existing function-pointer caller. A local checked
generation prevents an old Escape/scrim/close callback from closing a reopened
sheet, and clear preserves that counter. This is UI lifetime fencing, not a
substitute for W12's resident/transport admission. Its snapshot must move the
counter along with other retained controller identities. Context adaptation and
hidden-resource focus rules remain part of the coordinated resident cutover.

### File counts in activity summaries

Paseo `tool-calls/detail-level/overview/model.ts` counts distinct read/edited
paths. Native group and turn summaries now deduplicate the typed reported file
locations instead of counting repeated reads of one path as several files.
Missing legacy locations retain per-call counts: no path or shared identity is
invented from titles/opaque input. Canonical Read/Edit/Write paths will join this
same interpretation after W23's typed detail fields are consumable. One
regression covers repeated and different file paths in both group and turn
summaries; the iterator path does not allocate an intermediate tool list.

Compact group inline expansion is derived from that conversation's single
SheetState rather than adding a field to the shared Paint struct; A8 permission
and question consumers keep their existing Paint construction unchanged.

### Consumed Chat controls after base4

Base4 `3cd0d3fc` merged as `e6c1d9ab`; README conflict retained both existing
settings audit notes and timeline glyph/font notes. L's Chat adapter is now
consumed directly. `ConversationDisplay::CollapseTurns` is the third picker
choice and new-document default, matching Paseo storage.ts:163; explicitly
saved Full stays Full. Existing lenient field serialization and PreferenceChange
store remain the only path. Chat rows are reasoning, display, then W1's outline
when that owner lands. No second display label table is introduced.

Both collapsed modes group active tool work. Whole-turn expansion reveals the
original grouped members individually, and a closed turn keeps its trigger and
last reply. Added projection regression prevents a tool group from concealing
members inside an opened turn. The existing capture override selects these same
consumed modes instead of a separate feature path. Reasoning switch words are
L-approved cached FTL getters; whole EN/KO labels and all three picker choices
are checked, including preference roundtrip and explicit old Full retention.

Paseo `tool-calls/detail-level/grouping.ts:90-117` keeps a lone tool as the
original row; only runs of at least two groupable members get a group. Native
`Folds` now applies that same threshold (including thought/tool pairs), rather
than replacing one call with a summary. Auto-expanded reasoning splits runs;
the isolated tools on either side therefore remain individual disclosures.

Paseo `tool-calls/expansion.ts:7-14,43-46` also pre-opens a future group when
the person explicitly opens its lone host call. `remember_standalone` now
records that intent in the existing `open_runs`; running-status defaults do not
open a group. A call within an existing group keeps its own independent choice.
Display/reasoning changes preserve those group choices for the same entry
identities. A native list-state regression covers single-call arrival, explicit
open, group formation, inner-call close and reasoning-default change.

Ordinary tool disclosure now starts closed even while running, matching
`components/message.tsx:2276` (`defaultExpanded ?? false`) and its normal
`view.tsx` call site. This corrects the earlier page-derived automatic expansion;
Plan outcome and reasoning preference defaults remain separately consumed.
Explicitly opened work stays open when completion arrives; a native regression
uses the real record ingestion/default boundary to verify that transition.

### Source bounds for draft forks

[Decision Log]
- Purpose: W2 must reject a projected prefix when a previously placed tool row
  has incorporated a source record after the selected anchor.
- Constraint: `Entry.seq` is the first record, and `rev` counts mutations; neither
  identifies the last source record, particularly when records have gaps.
- Alternatives: infer a cursor from revision, or replay an independent projection.
- Choice: additive `Entry.through: EventSeq`, the exclusive watermark of the last
  contributing or invalidating record. Push/change use the fold's already advanced
  cursor, preserving the original entry key and first-record sequence.
- Why: the existing single projection knows the exact record at every mutation;
  consumers can conservatively reject a bound crossing their selected cursor.
- Effects: this is not a contiguous contribution range or historical reconstruction;
  invalidation may advance the bound even without changing visible text. Replayed
  overlap does not advance it. Tests cover late tool updates and merged text with
  source gaps. No protocol call or node contract is invented.

### Canonical DTO presentation skeleton

Consumed W23's committed standalone values `96ed8b0b` in development merge
`057532a5f`. This adds no ToolCall/event fields, call registration or activation;
the wider runtime checkpoint is still not merged. Pure `tool_display` and
`tool_sections` now exhaust all eleven canonical variants using those exact DTOs.
They preserve read offsets (including an explicit zero), distinguish absent
replacement from deletion, prefer supplied unified patches, and retain opaque
Input/Output formatting. Setup/child markers do not offer empty details. Core
does not decode provider-specific JSON or execute reported commands/URLs.

`ToolDetailSource` is the narrow typed renderer seam: the current legacy
projection reports no canonical body/failure; in-memory fixtures supply the
canonical DTOs. W23's checked field/scope handoff will replace the compatibility
implementation directly. This is source-contract work, not activated canonical
provider rendering. The category labels use one cached Fluent enum adapter;
the source's hardcoded English categories gain complete EN/KO labels here.

The existing tool card/body now accepts `ToolDetailSource`, retaining the legacy
producer path while typed in-memory fixtures exercise the canonical path. The
same kit output/numbered lines and shared diff control render command/output,
read offsets, writes, replacement/unified diffs, prose and opaque Input/Output.
Plan uses the already shared Plan renderer. Semantic icons use only shared kit
assets. Marker/empty-settled rows do not advertise a dead disclosure; explicit
failure text remains readable, and empty running details retain the loading UI.
The scoped file-open hook remains absent until W13/W12/G4 provide the authorized
context; a provider path never becomes file access authority in A7.

Read/unified previews keep the named line bound, a localized truncation notice,
and full-text copy. The kit's existing `version_of` now accepts typed Hash values
without changing string call sites; `(text, Option<DetailOffset>)` distinguishes
write/no numbering from read/explicit zero without allocating cache-key strings.
One cache regression covers that edge. These bodies have no syntax highlighting
beyond the shared output controls; this remains an explicit presentation gap.
Strict canonical-model run `wp-timeline-narrow-20261115T085643Z-a66f1bf5` reached
the app and failed only a documentation backtick lint, now corrected. The new
native body/typed fake affordance tests still need a checked run.

W2 requested the entry-source prerequisite without the unreviewed A7 screen
stack. Isolated bridge `20ae7edac` (parent base4) contains seven files/98 lines;
its exact extraction has diff/size evidence only, while original `9fbda65af`
passed all 1,059 view all-target tests. W2 owns its exact integration check.

### Shared dialog actual-close adoption

Announced app line `074ade16` merged as `a7d08bc12`. The kit dialog conflict was
only the equivalent captured-closure variable naming; the incoming G4 renderer
is retained exactly, including its public closure API and weak return-focus seam.
Compact tool sheets now acknowledge `DialogState::dismissed` only after their
generation matched and the selection was actually removed. The screen-local
strong prior-focus handle is removed. Stale dismissals return without consuming
the shared focus cycle, and newer focus cannot be stolen by old sheet cleanup.
The existing local generation regression is preserved with the narrower state.

Row mutations moved unchanged from the 497-line `fold.rs` into `append.rs` before
the remaining canonical item adapters are added. Admission/public queries stay
in `fold.rs`; there is still one push/change path and the same exclusive source
bound. `touch` remains with its private file index; no size record is raised.


### Notice and context-maintenance source adapter

The canonical standalone DTOs now have explicit view variants and one renderer
per kind. Notifications use the existing severity Alert and a thin selectable
literal-prose wrapper over the existing text engine. Escaped bare-pre HTML keeps
provider markup literal and preserves logical whitespace; the same global
selection adapter supplies Plain copy, including its standard paragraph separator.
HTML line-ending normalization remains a limitation. No engine/Markdown API or
selection manager is introduced. Plugin data remains opaque; its localized
unavailable card identifies the plugin/kind and has no action. This is a fallback,
not installed-plugin rendering (W16 owns that future adapter).

Compaction follows Paseo `message-compaction-label.ts`: loading precedes explicit
trigger, positive pre-token count rounded to integer thousands, then completion.
Every loading starts a marker; completion upgrades the earliest non-rewound
outstanding marker across intervening rows and retains unreported metadata.
Markers remain visible when turn work is collapsed. A paired plugin/item id
updates its original row through the same append/change path. Direct typed
projection fixtures cover this source behavior without inventing wire events.
Actual AgentEvent activation is held for W23's checked runtime/contract handoff.

### User-message continuity and exact source navigation

[Decision Log]
- Purpose: a diagnostic log between replayed user chunks must not split one
  prompt, and searching its later chunk must reveal that same prompt.
- Existing constraint: appending only to the last row splits across HarnessLog;
  first/through bounds overlap when an older tool receives a late update.
- Alternatives: infer containment from spans, or create another projection.
- Choice: retain the current user-stream row across the canonical prompt-index
  metadata classifier; true content/status/terminal/rewind boundaries end it.
  TurnStarted and Steered establish new streams. Steer chunks preserve its key.
  Each entry retains exact later source membership alongside its conservative
  exclusive through bound. Navigation prefers an exact creating record, then an
  explicitly contributing record, with the existing earlier-row fallback for a
  record that has no visible contribution.
- Why: the sole projection sees both admission and mutation; conservative bounds
  alone cannot distinguish a gap or an unrelated late update. No provider message
  id or session-wide tool-id uniqueness is assumed.
- Effects: one optional Vec per entry records changed source sequences, growing
  with the loaded log; unchanged rows allocate no membership storage. The existing
  find scheduling/cache remains unchanged. Its searchable-kind classifier moved
  to a dedicated module to preserve the file budget. Real ingestion regressions
  cover chunks across HarnessLog/Woken, Steered, invisible Terminal boundaries,
  overlapping late tool spans, search start and native list reveal. Shared W1
  controller identity/state is outside this change.


### Checked baseline and shared-source union

Fixed clean `c6f0c0c63` passed view all-target tests: 1,072 passed, no failures
or ignored tests, including the identified pending-message ledger. It does not
cover the subsequent notice/chunk-source changes in `0e9085da1`, whose strict
Clippy is queued separately. The shared Plan prerequisite `9e53f43fd` merged as
`1c6a65397`: its renderer body is unchanged, and its canonical PlanOutcome /
PlanExpansion now have one definition in plan_types. App line `9e18cf0d` merged
as `6e0485987`, preserving all existing glyphs plus the four owned additions
(96 total). It contains exact W12 producer9480/e46 and no live resident resource
map/cutover. No W10-authored node implementation is introduced. From this point
all Rust compilation, tests and gates run on the prescribed build VM.


L reviewed the exact0e9085da EN/KO timeline-items and tool-category data with
complete coverage and no variable mismatch. The one requested correction changes
Korean Thinking to 추론: it is a state-independent category, so 생각 중 would
incorrectly imply a completed tool is still running. The exhaustive EN/KO enum
label regression follows that correction. This is wording/data review only;
number formatting, renderer behavior, package gates and captures remain separate.


### Moving read-only provider-child projections

W2 retains bounded canonical records and hands off each accepted rebuilt Timeline
by ownership (`take_projection`); the existing conversation rows own the moved
projection. No second fold, cloned transcript, managed child SessionId, prompt,
approval dock or lifecycle action is introduced. The shared installation helper
is first consumed by the existing fork-origin renderer. Its guarded public
`replace_readonly_projection` adapter and tests are staged until W2's actual
caller; remove its two cfg(test) annotations with that integration.

Explicit entry/run/turn choices and the visible reading anchor rebase by EntryKey
when an earlier sparse parent page shifts indices. Missing choices are dropped,
default choices stay implicit, and tool-sheet generation is preserved on clear.
The receiver must have no managed shown/info identity. W12's sole resident owner
must first validate the parent binding, read generation and UI holds; this helper
is presentation only and grants no input/network authority. Scoped Seat routing
and old-callback rejection remain held for that producer. W1's Prompts, Pointer
and query objects are not replaced. Read-only pending plans remain visible because
there is no approval dock to host them; their body has no action footer.

Source cross-check: Paseo `types/stream.ts:1551-1562` projects canonical error as
an error-level Notification. Future W23 AgentEvent::Error wiring will use that
same alert path; it must not synthesize TurnFailed or a completion footer.


Validation update: the VM restart canceled both queued source0e and merged325
jobs with rc143 before compilation. Their replacements on6df reached real
compilation: view all-target tests found two missing exhaustive fixture helper
arms, fixed97db0fd6; strict Clippy found the equivalent-body Steer text arm,
merged with the Message/Thought arm in07bd40d3. These are scoped failed checks,
not passing gates. Replacement checks retain immutable source and separate logs.
No new local Cargo/Rustc process was started after the VM-only rule.


Seat now also exposes borrowed `get`/`get_mut` methods, used by A7 render/actions
instead of reading function-pointer fields directly. The current legacy readers
and Copy behavior remain compatible while W4 owns its permission/question callsite
migration. This is a mechanical access boundary, not resident activation: the
future keyed branch must resolve both methods through W12's same validated
SessionContext and must never fall back to the selected conversation.


### Literal tool and reasoning prose

Paseo `tool-call-details.tsx:277,366,497` renders plain_text and an unknown
input string without output in the selectable UI face; search/fetch output uses
its code surface. ToolSection now distinguishes Prose from Text accordingly.
The existing formatting edge recognizes only top-level JSON string/null values,
without reading arbitrary provider fields. Empty opaque values add no section.
Reasoning, these prose details and explicit tool errors now reuse kit::plain_text
rather than an unselectable Div/string or a console surface. The shared domain
prose container wraps and scrolls at the source's 300-point default height. The
existing engine retains selection/copy; no Markdown interpretation is introduced.


The merged97db model run compiled and reached 989 passing/one failing library
test. The only failure was the new compaction sentence expectation omitting the
shared Fluent resolver's bidi isolation around its string argument. The expected
whole EN/KO sentences now retain those U+2068/U+2069 controls, consistent with
existing locale tests; no formatter or production message was weakened/changed.


Canonical Plan detection now goes through the same generic `plan_text` seam in
all existing grouping/disclosure/dock-hiding/sheet eligibility paths, rather than
only in the tool body. It borrows canonical Markdown, respects a canonical
non-Plan even when old input resembles a plan, and decodes legacy input only
when no detail was supplied. This prepares field adoption without activating it.
A typed fake regression covers canonical Plan with broad kind Other/no raw input,
canonical non-Plan precedence, empty canonical Plan and unchanged legacy fallback.
The shared Plan card renderer and approval semantics are unchanged.


W2's staged child projection_revision changes per accepted page, including a
same-query prepend, and its navigation-lifetime ticket source does not reset on
A→B→A reopen. The future keyed Seat must capture that revision so queued old
numeric-row callbacks cannot target a different entry after replacement.
Read-only replacement also rebases an open detail sheet by key and group choices
to the group's new lead. Removed targets enter the existing actual-dismissal path,
preserving dialog focus restoration and generation fencing rather than dropping
an open sheet silently. These UI rules have additional native regressions; actual
child context/callback integration remains held for checked W2/W12 producers.


The same selectable failure helper now serves canonical and legacy detail bodies.
An explicit failure supplied without a detail body is not lost behind the legacy
empty/loading branch, and it is not inferred from ordinary output text.


### Inter-agent provenance presentation

Paseo `tool-call-display.ts:153` and
`paseo-tool-presentation/agent-message.ts:21` prioritize typed agentMessage over
ordinary tool names/details. ToolDetailSource now requires that optional typed
provenance. The current compatibility projection reports None until W23's field
handoff, while fakes can exercise all five delivery events. Native uses the shared
host mark (its own asset), event label, title-or-external-id summary, and literal
localized sender line plus full message body. External host/agent ids grant no
local navigation or node authority; there is no invented sender-open action.

The summary scan/output use the source's 480/120 UTF-16 bounds, normalize shared
whitespace, append an ellipsis when capped, and keep Rust scalar boundaries rather
than emitting a dangling surrogate. Only the summary is shortened; the body stays
literal/selectable. One cached enum adapter supplies complete EN/KO event labels.
Canonical Plan detection also respects provenance priority. This is source/typed
fixture work pending checked node fields, runtime and final native validation.


### Latest source and verification boundaries

`ba743c98` passed strict all-target app/view/kit Clippy in
`wp-timeline-narrow-20261115T105149Z-ba743c98`; it includes the first guarded
read-only projection adapter, but excludes later prose, canonical Plan precedence,
open-sheet rebasing, provenance and base6 changes. Its helper was handed off as
checked source, with GPUI/package review still explicitly pending.

App base6 `18eedad4` merged as `8173f6073`; only the shared app line was imported,
not the moving node branch. W2's strict-checked five-file canonical child identity
values `0560af73` merged as `4dd2dd7a4`, without methods/events/runtime. Shared line
`dd13ce378` (including its Cargo.lock correction) then merged as `94d80c9c5`.
There are still no W10-authored node implementation changes.

L accepted the exact `76fb22fa` four timeline FTL files, including corrected 추론
and six delivery/sender keys. This is wording/data only, separate from renderer,
authority, number formatting or gates. The current provenance Clippy and newer
model/app/kit tests are queued on fixed commits. The old56 app/kit attempt failed
only the missing InteractiveElement import, already fixed in629404149; it did
not execute those tests and is not a runtime pass. The replacement uses the
current immutable94 source. Whole gate, captures and V4 remain owed.


Fixed `fcddc1c8` now passed all 1,131 view all-target tests in
`wp-timeline-models-20261115T110133Z-fcddc1c8`. This includes exact source
membership/user-chunk continuity and the earlier canonical item/prose model. It
excludes later canonical Plan/provenance changes, open-sheet rebasing and base6.
The pending app/kit replacement and newer strict run remain separate evidence.


### Staged concrete resident owner adapters

Consumed W12's exact isolated1d93a10d resource producer, with its literal cfg(test)
module/storage boundaries unchanged. Only the shared app line, W1 checked search
extraction and that staged owner are included; no second live manager is enabled.
A7's own resident module is likewise test-only. Its borrowed view/input accessors
use the same full-context resources/resources_mut pair; input additionally requires
current Visible/Focused metadata. Records use original FollowBinding through
receive (including hidden fresh records); operation completion settles a Timeline
ticket before its owner closure can mutate. Actual G4 lease validation is still
required before these receivers; the adapters do not authorize or launch I/O.

The legacy session guard now delegates to one identity-free fold_records method;
the scoped receiver reaches that same fold only after admission. There is no
fabricated Shown/session identity, cloned transcript or selected-global fallback.
A typed snapshot moves the concrete Area unchanged, retaining rows, scroll,
disclosures and controller identity. It is not serialization or a claim of byte
memory reclamation. W1's navigation hold is an explicit input; active find/edit,
overlays and own in-flight work refuse transfer and return the original object.
A8/lifecycle owners must independently vote Ready in W12's same owner before any
retirement. Concrete ticket-correlated fork/rewind/read retirement UI transitions
and the scoped Seat/render/follow cutover remain pending the coordinated carriers;
no placeholder uncertainty marker is claimed as that behavior.

Staged GPUI regressions populate two real timeline resources, apply hidden data,
reject old-context input, consume one effect once, reject old-follow records, and
preserve the other timeline/global. A separate moved-snapshot regression retains
actual disclosure and scroll data and refuses a navigation hold. Product storage,
raw follow and permission/question handlers remain untouched until W12 cutover.

Snapshot data additionally carries the original ResidentToken. Restore compares
window RegistryId and the full authenticated node/session address; a mismatch
returns the unchanged snapshot. A later epoch of the same admitted resource may
reuse the local data, but the comparison does not admit or refresh that token.
This prevents accidentally installing a retained transcript or GPUI entities in
another session or window. Old transport callbacks still require their original
binding and fail the registry fence after retirement. These are staged owner
adapters and source regressions, not a live resource cutover verdict.

### Borrowed canonical detail body for A8

[Decision Log]
- Purpose: a PermissionPresentation.detail must keep provider precedence without
  manufacturing a ToolView or timeline entry for a pending request.
- Constraint: the earlier shared body read loading/failure through ToolDetailSource,
  coupling the caller to a timeline model despite already having a canonical DTO.
- Alternatives: duplicate A8 rendering, construct a synthetic log row, or extend
  the existing drawing boundary with only explicit owner presentation state.
- Choice: tool_detail::borrowed_body(&Paint, &ElementId, BorrowedDetail, Window, Context)
  renders the same sections/code/diff/prose/failure controls. BorrowedDetail holds
  &ToolCallDetail, DetailContentState::{Loading,Ready}, and Option<&ToolFailure>.
- Reason: A7's existing ToolDetailSource body delegates to this same function;
  A8 supplies its PermissionId-derived ElementId and actual loading state. No
  approval semantics, capability, request lifecycle or node method is inferred.
- Tradeoff: this remains an internal conversation API using the existing Paint.
  Plan approval keeps the shared Plan card; this body owns no Plan disclosure.
  Sections/loading/failure derive structural NamedChild IDs from the caller's
  ElementId, rather than stringifying a potentially colliding identity. No A8
  files changed. Actual canonical field attachment still waits W23's checked slice.

The queued 76fb narrow job was verified queued and cancelled once per the 12:1x
stale-lock instruction: it predates dd13ce37. Terminal rc143 is an authorized
queue cancellation, not a compiler result. Its single replacement contains the
fixed lock and current coherent owner source; existing 94d80 app/kit tests remain
queued and are not duplicated.
A retained fork-origin Area also votes recursively before snapshot transfer, so
an active overlay/effect in that embedded conversation cannot be skipped by a
clean parent. The staged regression exercises an actual retained child resource;
no product eviction or follow path is enabled by this additional vote.

### Staged owned-context Seat successor

W12 confirmed the adoption point: a Clone enum owns the complete SessionContext;
main and derived origin lookup resolve only the sole SessionResidents. The staged
ResidentSeat constructors main(context)/origin(context), get(&Shell), and
get_mut(&mut Shell) return None for missing/stale parents or absent origins.
Mutation additionally requires current Visible/Focused metadata. Cloned row
contexts remain strict and never refresh themselves into a successor. No main
SESSION constant, copy decomposition, identity cache or selected fallback exists
in this staged type. Product Place/Seat and A8 handlers remain unchanged until
the same coordinated W12 storage/follow/main-render union; this is not another
resource manager or a live parallel rendering path.

The staged populated-resource regression exercises a cloned captured Seat,
independent parent/origin rows, and rejection of both after presentation change.
Child Seat adoption has an agreed concrete owner rather than a new manager:
W2 moves its existing SubagentArea into SessionResources.subagents once. Its
readonly_for(_mut)(ProviderChildRef, projection_revision Ticket) validates the
canonical pane identity, installed readonly_source and exact revision. A child
Seat retains all three parent context/source/revision and validates the parent
before those getters and BEFORE numeric row access. Retargeted or inactive child
queries return None. That isolated W2 getter and field move are not present in
this A7 source yet, so no live child Seat claim is made.

Actual 94d80 app/kit all-target run wp-timeline-state-tests-20261115T113834Z-94d80c9c
failed compilation before tests at 12:34 UTC: two Option::filter callbacks passed
&&ToolView into generic plan_text(&impl ToolDetailSource). Both A7 callbacks now
explicitly dereference the filter borrow. No test counts or GPUI pass are claimed
from that failed run; its retained literal-and-projection-tests-2 evidence retains
the exact compiler log. The corrected current-head run also covers the staged
resident/Seat regression source omitted by that earlier snapshot.

W12's proposed explicit Subagents owner was chosen over a generic A7 child
snapshot placeholder: W12 owns the canonical owner/ALL and operation mapping
addition. W2 owns its real query/track/navigation/detach/dialog/uncertainty holds
and metadata snapshot; A7's Timeline vote covers main+origin with W1 navigation.
Neither writes a second last-writer Timeline vote for the other's resource.
The sole registry must collect both real snapshots and Ready votes before drop.

W2 can reuse the SAME staged Conversation TimelineSnapshot for its readonly Area:
into_timeline_snapshot(parent ResidentToken, navigation vote) moves the real Area
including list/bookmark/disclosures/find resources. A child with shown/info None
remains readonly; no fabricated child SessionId is allocated. The W2 snapshot
retains canonical child ref, accepted projection revision and actual query ticket
source alongside that Area snapshot; A7 does not invent or duplicate those fields.
Restore checks original parent window/node/session; W2 additionally restores its
matching child identity/revision and retains its dispatch fences. This helper
never reinstates an old transport operation. The public crate reexport remains
literal cfg(test) until the coordinated product owner/caller cutover.

### Canonical tool-scope activation requirements (waiting checked ADR0084 DTOs)

Existing EntryKey::Tool and tools/PlanDisclosures maps use raw ToolCallId. W23's
source contract explicitly does not promise whole-session uniqueness, so merely
copying detail fields into ToolView would leave cross-turn updates, Plan outcomes
and rebased sheet choices unsound. The same activation must change projection
identity, permission lookup and local disclosure together, using canonical
ToolCallScope paired with the raw ID. Provider scope includes BOTH actual thread
and turn; Record scope is an unassociated observation, never a current-turn guess.
This is a consumer design requirement, not a claim that those DTOs are adopted.

A scope-aware internal lookup must also keep conservative older-peer behavior:
an unscoped update can use only one unambiguous retained announcement. A repeated
unscoped ID permanently loses that association; missing/ambiguous updates remain
separate record-keyed observations, never whichever turn is currently running.
An update before an announcement cannot retroactively manufacture attribution.
Identical EventSeq replay remains skipped by the existing projection watermark.
The authoritative typed scope is not inferred from tool names or provider JSON.

All dependent consumers must migrate together: ToolView/source fields and update
precedence, append/find_tool/public tool_call, PermissionRequest.tool_scope title
and Plan outcome resolution, PlanDisclosures' entry identity, and EntryKey-based
sheet/run/bookmark rebasing. A permission lacking scope can resolve only an
unambiguous retained source, not the latest same-name tool. Canonical known detail
survives Unknown/absent updates; completed/canceled status cannot regress to late
progress. Explicit failure/provenance stay separate from raw output.

Required regressions on the checked DTO union: same raw ID in two provider turns
with a late update to the first; same turn ID in a new thread; Record orphan
isolation; unique older announcement/update versus repeated ambiguous ID; update
before announcement; scoped Plan permission/disclosure isolation; retained sheet
and source cursor/fork boundary staying on the original entry. This remains
unimplemented field/scope activation until W23's exact checked source handoff.
No local copy of ToolCallScope, node association tracker or numeric gate is added.

Final capture dependency: the approved shared root binaries predate ADR0084.
They remain valid for legacy native screenshots, but cannot prove receipt echoes,
canonical fields/scopes or new event variants that their decoder does not carry.
After runtime/gate activation, those scenarios require the matching checked root
fake/node export (or an explicitly reviewed fixture producer), alongside the
native app from the final frozen source. No legacy Delivered is presented as a
canonical receipt, and no direct UI injection/demo route is added to hide that
dependency. The final fleet/capture run has not started.

A8's pending approval is proposed work, so the borrowed renderer's state was
clarified to DetailContentState::{Loading,Ready}. It controls only the empty-body
loading placeholder; Ready means the content can be displayed, including a
proposal, and never claims execution, completion or approval. A8 uses Ready for
reported proposed content while its outer card remains Pending. The same section
renderer has no success styling derived from this state. This amendment is source
only until the next meaningful checked handoff.

The staged owner factory can use
ConversationArea::with_presentation(ConversationDisplay, ReasoningExpansion, App).
It reuses the existing Area constructor and display predicates, then seeds an
empty timeline's folds/turns/reasoning from the kept Chat choices. It has no
context allocator, selected-session lookup, follow or second preference store.
The same populated registry fixture now creates both actual areas with Full and
Expanded, verifying these saved choices survive admission instead of silently
using constructor defaults. W12 still supplies the scoped factory after capacity
admission; B2's real scoped composer constructor remains a separate prerequisite.

[Decision Log: staged resident owner adapters — 수정 사용]
- Purpose and intent: preserve the actual conversation's UI/input identity while
  resolving a session's rows and replies through W12's single admitted resource.
- Existing implementation and constraints: the Area already owns its Timeline,
  GPUI list/controllers and transient state. W12 supplies canonical Context,
  binding, tickets, resources and retirement. Current product globals remain
  until one coordinated cutover; A8 and W2 own their separate concrete resources.
- Alternatives considered: serialized reconstruction loses live controller
  identity; a new registry duplicates admission; Copy function pointers cannot
  capture full Context. No external library addresses this ownership boundary.
- Choice: extend existing Rust/GPUI resources with borrowed scoped accessors,
  Box-owned snapshot transfer, a Clone full-context Seat and an empty-resource
  constructor seeded from the existing typed Chat choices.
- Reason for choice: Rust ownership moves the original entities once and can
  return them unchanged on Hold/mismatch. Existing predicates/settings and
  controls remain the source of behavior; the registry stays the only admission
  minter. No unsafe, new dependency, store or context allocator is introduced.
- Benefits, costs and effects: concrete controllers/disclosures/bookmarks survive;
  stale window/session/claim operations refuse. A Box snapshot retains memory
  rather than proving cold-byte reclamation. Actual wire authority, retirement
  UI transitions, child field move and product rendering remain coordinated
  dependencies. All new owner/Seat/snapshot/factory APIs stay literal cfg(test)
  until actual callers and product storage/follow land together.

Readonly projection/find cutover limit: retaining the Area object is not enough
to rebase a live FindBar's numeric matches or incremental cursor. The current
install helper resets FindCache but does not own W1's live search generation.
W2 must hold protected navigation during replacement or consume W1's scoped
projection-refresh hook preserving its field/query/focus identity. A deferred or
refused install must not expose the pane's newly accepted revision over the old
installed Area. Both owners have been informed; the temporary child frame has
no scoped find, and the final keyed finder/installation handshake is still owed.
No global find_grew fallback, dropped input or default-recreated controller is
claimed as that migration.

Actual 0ce app/kit and strict runs completed 13:12/13:13 UTC without executing
tests: both found duplicate connection_epoch in the merged W1/W12 Machine test
literal. Exactly the duplicate fixture field is removed; no W1 controller or
search algorithm changes. Strict also reported the already-fixed qualified Seat
test import and ToolView rustdoc plus needless ownership in borrowed_body.
The helper now borrows &ElementId, and BorrowedDetail is Clone+Copy of borrowed
DTO/failure refs and content-loading state. Child IDs still own structural parent
clones; no borrowed element identity escapes. The corrected current-source
checks remain owed, and the old exact logs remain in resident-seat-tests and
resident-seat-narrow; no GPUI/test pass is inferred from their compilation.

W1 confirmed the current safe contract: Hold/refuse installation whenever its
navigation_held() is true; no scoped finder refresh is claimed yet. The staged
public wrapper now requires an explicit OwnerReadiness navigation argument and
also refuses an actually open FindBar. Refusal returns the original moved
projection without changing rows/disclosures/scroll/cache/controllers. W2 retains
that returned projection and keeps its installed revision unchanged until an
actual install; the helper never updates W2 metadata. A staged regression checks
the returned prefix plus exact old rows/disclosure/anchor under a navigation Hold.
This is a source amendment, not a checked W1 scoped finder implementation.

Executed exact189652504 view all-target tests completed 13:31 UTC:
1153 passed, 0 failed, 0 ignored; run
wp-timeline-models-20261115T124844Z-18965250, evidence
the retained provenance-plan-models/native-cargo.log. This includes the
Plan/provenance additions after fcdd. App/kit/owner-body successors remain queued;
this is neither the later app API source nor a whole/native/capture/V4 pass.

[Decision Log: remaining diff and code presentation — 수정 사용]
- Purpose: the source edit renderer marks changed words and syntax in read/write/
  edit content (tool-call-parsers.ts:55-203 and diff-highlight.ts).
- Existing assets/constraints: bounded timeline DiffLine and similar line edits;
  kit output's existing virtual list, line tones, copy and scroll; view workspace
  lexer/catalog; shared kit range sanitizer. Existing Line/Lines callers stay valid.
- Alternatives: another diff/editor control or a language/color catalog would
  duplicate existing behavior. Reuse the virtual renderer with opt-in decorations.
- Choice: bounded adjacent remove/add word-range computation matching the source's
  ASCII word/nonword runs, plus optional output emphasis. Reuse existing range
  validation. Syntax consumes W13/K's existing lexer and reserved mapping/resolver
  packet when checked, with no new filesystem operation or path authority.
- Reason: state, virtualization, scrolling and copy stay in the existing control;
  semantic ranges/styles are added at the presentation boundary. Standard Rust
  vectors/slices and existing similar suffice, with no external dependency.
- Effects/tradeoffs: intraline matching has a named finite token/work budget and
  leaves ordinary whole-line diff available when that budget is exhausted. Syntax
  adapter consumption remains pending K's checked source; no duplicate seven-arm
  mapping/color table is added. This section records the design, not implemented
  or tested intraline/syntax parity.

Intraline implementation source now uses the source's adjacent removed/added pair
and ASCII word/nonword runs with the same LCS deletion tie-break. It emits changed
UTF-8 ranges without rebuilding text. Guards are 16,384 bytes / 256 tokens per
line pair and 1,048,576 LCS cells across one diff; exhaustion keeps whole-line tones.
Unified patches retain their existing source behavior (no synthesized word pairs).

Kit output retains Line/Lines and old lines_box/output_box/diff_line signatures.
The opt-in AnnotatedLines/annotated_lines_box adds ranges over that same cached
virtual list, scroll handle, row painter and copy payload. Invalid/partial Unicode
ranges are sanitized by the existing command-range validator, moved once to a
shared private text_ranges module with the old path reexported. No validator,
selection manager, language catalog or output controller is duplicated. The
existing diff wash is layered on changed word ranges, without new theme colors.
Syntax color remains pending K's checked adapter/resolver packet.

Staged source regressions cover reference word grouping/tie-break, Korean/emoji
byte boundaries, exact adjacent-pair behavior, insertion/deletion, shared work
exhaustion and malformed/stale cache ranges preserving text/copy. These new tests
have not executed yet; the preceding 1153-view pass predates this implementation.
The public word-highlights boundary independently caps annotation rows to the
existing MAX_LINES and stops tokenization when its shared work budget reaches
zero. An oversized caller keeps its original text/rows; annotation lookup past
the cap returns no emphasis. This does not rely only on the current diff_lines
caller already being bounded. The additional cap regression is source only and
postdates the queued9ec word-model run.

Executed exact3a10a0ecc app/kit all-target tests completed 14:02 UTC:
271 passed, 0 failed, 2 existing ignored. App101 and Kit170 include the actual
populated scoped hidden/stale/duplicate/Seat cases, moved snapshot/origin Hold,
borrowed-detail identity and literal HTML-engine selection/copy regression.
Evidence: retained owner-body-tests-2/native-cargo.log. It excludes
later readonly-navigation amendment, intraline implementation and base8.

Its separate strict run failed only a 164-line test function and an ignored bool
in the same test. Fixture setup and origin assertions are now small helpers and
the settled fold result is asserted. No suppression or check reduction added.

Official app package base8 91407173 (root82ae7c91, native1431/0/2) is merged in
5449216a1. Conflicts were additive README text and shared glyph declarations/
paths/drawing: retain upstream Gauge/PackagePlus and all four A7 Lucide glyphs
with geometry/licence unchanged. The complete merged inventory is99. W2's event
accessor prerequisite arrived only through this official app base, not a copied
accessor or direct node-head adoption. W23's portable checked values cut is still
pending; no canonical field/receipt runtime activation is inferred from base8.

### Portable borrowed-body boundary extraction

The single body implementation now lives in conversation::tool_body, separate
from ToolDetailSource/header, provenance wrapper and compact sheet lifetime.
tool_detail reexports BorrowedDetail, DetailContentState and borrowed_body, keeping
A8's agreed interface unchanged. Timeline/provenance/legacy failure wrappers call
that same kernel; sheet loading delegates its empty presentation there too.
The structural-ID test moves with its implementation rather than being copied.

This extraction prepares a minimal portable native prerequisite for W4 on base8.
It does not declare a ready packet: W4 has no ToolCallDetail, so the canonical
standalone-values dependency must be identified/approved by W23 under the current
integrator disposition. No proto source, ToolView, Entry or response is fabricated.
The native-only source cut still needs exact narrow validation before handoff.

The kernel is now session-neutral: BodyPaint borrows only catalogs, legacy
conversation words, tokens and secondary ink; rendering accepts App rather than
a Shell context. The existing public tool_detail wrappers project their Paint and
preserve A8's agreed interface. Computed diff drawing moves once into this same
kernel; the legacy tool wrapper delegates, so canonical edits and older tool
content cannot drift through separate diff renderers. The moved gap uses existing
spacing::S1 and the old tool.rs px-step ratchet decreases5 to4; no budget is raised.
The source-only portable packet must include this exact common kernel and its
existing output/diff/word/locale prerequisites, with no header, resident, request
controller, node-field or RPC activation.

The borrowed facade's cx parameter is generalized to App (existing Shell contexts
still coerce directly); this matches the session-neutral kernel. A headless
ProposedBody render fixture now calls the actual public borrowed API with a Shell
detail DTO describing proposed material and content Ready. It creates neither a
ToolView nor any log Entry/session identity and performs an actual draw. This
new fixture is source only, not executed evidence yet. The ordinary timeline
wrappers retain their previous callback context and call the same function.

### Base8 bounded execution and shared path table

Exact cfa857548 app/kit all-target tests completed 14:46 UTC, run
wp-timeline-state-tests-20261115T140823Z-cfa85754: App102 + Kit185 =287 passed,
0 failed, 2 existing ignored. This includes the navigation Hold, bounded output
emphasis/cache regression and base8 union. It predates the shared kernel
extraction and proposed-material draw fixture, so it does not validate those.
Exact9ec010f3 view all-target tests: 1157 passed, 0 failed, 0 ignored, run
wp-timeline-models-20261115T134356Z-9ec010f3; the independent annotation-row cap
regression added later is not covered by that earlier run.

The cfa strict run failed Glyph::path at101/100 after additive shared glyphs and
new test idioms (get-first/single-range literal). Assertions now use first() and
slice::from_ref without reducing coverage. Path data moves to a declarative
exhaustive table generating the same constant function: every previous path,
geometry and inventory order stays unchanged, with no lint suppression. A7 is
the sole author of this shared seam. The lead requires a separate path-only
commit on the next green announced line, plus exact strict/kit tests, before
shared adoption; the four A7 glyph additions then remain separate table rows.
The current A7 preparation is not that shared-line approved packet.

Native-only borrowed-body source is separately isolated on official base8 as
717f4f58c, without crates changes. Both modules remain literal cfg(test) until
an actual A8 caller and legacy computed-diff delegation land together. W23's
minimal d596 body-values prerequisite is source-passed but still awaits exact
checks and integrator adoption; no field/event/RPC/runtime activation follows.

Shared path ownership correction (14:55 UTC): the lead withdrew A7's table
ownership because the coordinator had already assigned the single inventory to
S2. S2's existing e7f3a244 inventory macro is the pending canonical seam. The A7
paths.rs preparation is withdrawn; no shared-line path packet was delivered.
Keep the original exhaustive path function until S2's reviewed, green seam is
announced, then append only A7's four entries there. The two newly queued44ccd
narrow/test runs are obsolete and canceled while still queued; retained143 logs
are neither a compiler failure nor a pass. Their replacements wait for that seam
rather than spending build slots on a withdrawn implementation.

The shared-kernel extraction also removes the legacy diff's now-unused fluent
builder import and keeps its bounded-preview documentation independent of a
removed local MAX_LINES import. No stale body import or Rustdoc link is kept in
the activation artifact. The artifact is source-only and cannot activate A8 alone.

Portable-body dependency audit found the empty-JSON predicate still referenced
header-only tool_display. The existing predicate is moved once into tool_sections
and imported by the header model, rather than importing the entire header/
provenance/source-trait graph into A8 or copying a second JSON classifier. Existing
unknown null/string/object tests cover that unchanged formatting rule. This is
native-only source; no protocol decoder, field or approval action is added.

W1's source10a8a905 recursively aggregates navigation_held/retired through every
existing Origin::Shown level. It remains unexecuted/literal cfg(test) on W1's
branch, not adopted here. When that owner's checked source joins the resident
union, the external navigation vote passed to this snapshot must use that same
recursive query; A7 still owns its separate loading/overlay/pending checks.

The extracted computed-diff body retains its required FluentBuilder import at
its new call site. A source scan against the pinned GPUI trait definitions found
that `.when` belongs to this separate trait, not InteractiveElement/Styled. The
old queued draw-test source lacks that moved import and is superseded only if
still queued; no source pass is claimed from a canceled run.

### Paced assistant and reasoning text (independent A7 gap)

[Decision Log — 수정 사용]
- Purpose: preserve Paseo's display-only text reveal, not raw coalesced arrival
  jumps. Read app/agent-stream/text-reveal.ts and hooks/use-revealed-text.ts in
  full, plus layout.ts live-head phase and message.tsx find bypass.
- Existing assets/constraints: the timeline owns full text/revisions and exact
  entry identity; GPUI use_keyed_state/request_animation_frame already own mounted
  output state. unicode-segmentation1.13.3 is already in the native lock. Existing
  composer Reveal is a scalar surface tween, not a text/grapheme policy.
- Alternatives: duplicate transcript strings or a per-session timer map would
  burden snapshots/retirement; a byte/Unicode-scalar cut can split Korean/emoji;
  a second renderer would break copy and find semantics.
- Choice: pure view TextReveal counter/FrameClock with Rust Duration and UTF-16
  release units, plus conformant grapheme boundaries from the already-locked
  segmentation crate. Mounted-row GPUI state stores counters/version/epoch only,
  never another message. The source150ms backlog horizon, max250ms elapsed,
  60Hz frame commits, first-sight full hydration/remount, completion flush and
  streaming trailing-ZWJ/odd-regional-indicator hold are preserved. Rust str
  cannot contain a dangling UTF-16 surrogate, so no surrogate repair is needed.
- Why: canonical full text, outline, global search and reply copy stay unchanged;
  only the displayed prefix changes. Existing keyed state dies on unmount, and
  frames are requested only while a drawn row has backlog. No detached task or
  new session manager is introduced. Reduced motion and the selected find result
  complete immediately through the native motion/accessibility policy.
- Effects/tradeoffs: one already-transitive dependency becomes direct in view;
  no new dependency version or node contract. UTF-16 counting is cached per text
  revision, and grapheme scanning is confined to mounted streaming text. Native
  keeps its uncapped full-text rule. This section records intended implementation,
  not an executed pacing/latency/UI pass.

Pacing implementation source now connects entries to the mounted-row helper for
assistant text and expanded reasoning. The caller derives Streaming only from
the same area's current Timeline::turn and the entry's creating sequence; history
and completed/status-closed turns are Complete. Both normal rows and compact
run sheets use that phase. No A8 Paint field or permission/question file changed.
Selected find matches and the existing reduced-motion policy complete the prefix.
The Markdown version includes canonical revision plus visible byte length, so a
frame's reveal updates its cached text without changing the canonical revision.
Reply copy/outline/find ingestion retain the original full entry text.

GPUI keyed state holds only revision/counters/clock origin, not another String,
map, session resource or detached timer. New growth schedules its first frame
without immediately revealing a burst. Unmounted rows release this state and
rehydrate whole on remount. The fixture exercises the real keyed-state/frame
request helper, growth, completion and reduced motion; pure tests cover 60Hz,
UTF-16 arithmetic, Korean/Indic/combining/flag/ZWJ boundaries and monotonic progress.
Those new app tests remain unexecuted. The previous287 pass predates pacing.

Official c3429952 (native1450/0/2) is merged as f5783cb63; conflict resolutions
keep the canonical W12 resident foundation/minter and six-owner vocabulary,
retain exactly one Machine.connection_epoch initializer and both README sections.
The resident module stays literal cfg(test); no product resource/follow path is
enabled. Its real lazy visibility/same-claim synchronization remains W12's
coordinated cutover. View-only exact d2ea72814 strict and tests are submitted as
separate immutable runs; no current result is presumed.
Readonly history explicitly uses Complete even when its bounded projection ends
inside an open turn. A provider-child/origin page must not hide its final regional
indicator forever waiting for a live delta it does not follow. The source guard
uses the existing managed shown identity, without creating a child SessionId.
A GPUI regression folds an intentionally open readonly snapshot and verifies full
fragment visibility. The eventual resident hydration still owns managed identity;
no selected-session or guessed-provider fallback is added by this phase rule.

The body draw run wp-timeline-state-tests-20261115T151424Z-3304d050 failed before
compilation because the author incorrectly selected --lib for the vessel-app
binary package. No tests ran and no kernel result can be inferred. Its retained
logs are preserved. Corrected App/Kit all-target selection on exactbab3c11cc is
submitted once as wp-timeline-state-tests-20261115T153700Z-bab3c11c, covering the
kernel, new pacing and canonical base union; results remain pending.

W18 owns durable finish subscriptions/existing ADR0033 notice watching; A7 owns
the notification item body, and W7 retains OS/Inbox. W23/W18 agree that the body
stays TimelineNotification{level,message} and canonical EventRecord.seq remains
its entry identity. Trusted NoticeKey/source, captured parent/grant lifetime and
internal receipt admission remain their node contract. A7 adds neither a body-id
field, label parser, parallel event union nor duplicate user bubble. Positive
notice runtime and event activation still await that reviewed producer.


Minimal resident preparation is isolated on announced2b806402 as7ae881e59,
without the full A7 presentation branch, node types or layout changes. Its
with_display(existing ConversationDisplay, App) initializer is now shared by the
full with_presentation wrapper here; reasoning/turn state remains with A7. Both
variants move the entire original Area and use one caller-supplied W1 aggregate
navigation vote. Origin recursion checks local A7 protection without introducing
a new navigation Ready. W1 recursive metadata/hover/query hook10a8a905 is still
source-only/unexecuted, not copied into that packet. W12 requested the bounded
checked packet for its next core union; source checks/tests are still owed.

A8 coordination: Conversation.through covers actor-event membership only. A high
unrelated watermark cannot clear Answering/Unknown/Unconfirmed. Owner request/read
tickets and authoritative current pending delivery metadata remain with W4;
resolved log beats stale Live. Original context/lease/binding admission must occur
before the owner changes that state. This timeline adapter creates no A8 read
registry and does not infer no-effect from a transcript watermark.


Exactbab3 App/Kit all-target run completed16:04 UTC with App112 passed,
1 failed, 2 ignored; Kit execution was not reached after the App failure. The
actual proposed-body Ready/Loading draw, readonly fragment, scoped owner and
snapshot cases passed individually. This is partial runtime evidence, not a
whole App/Kit pass. The new pacing fixture incorrectly assumed its explicit draw
was the first after notify; GPUI had already rendered once, correctly producing
a partial prefix. The fixture now records actual render order and supplies a
deterministic frame clock through the same helper, instead of guessing paint
counts/wall time. It retains the first-arrival assertion and strengthens the
outstanding-callback bound to exactly one.

The renderer now advances only on a real frame wake, not an unrelated render.
FrameWake::{Idle,Scheduled,Ready} coalesces requests, and the callback holds only
a weak mounted state. Completion/reduced motion makes an already queued callback
inert, without waking the row; after that bounded callback drains, no work remains.
This matches the source's one frame handle without detached session timers. The
new exact callback/regression source is not executed yet. Pacing view strict's
firstd2 run found type complexity and integer/NFC/slice test idioms; WordChanges
names the existing return value, clock remainder is checked, the frame interval
uses nearest-nanosecond precision, and escaped jamo preserve test bytes. No lint
suppression, weakened Unicode assertion or new dependency version is introduced.


Exacta415 view all-target tests reached1025 passes/1 failure at16:21 UTC; all
six text-reveal regressions passed. The sole failure was the unchanged About
manifest-order regression: the newly direct unicode-segmentation dependency was
not listed in BUNDLED. Its existing registry SPDX expression MIT OR Apache-2.0
is verified and the actual About inventory is now updated in manifest order,
using the existing text-editing category. The test is not weakened; remaining
integration suites did not run after that failure. This is not a green full run.

The verified portable Markdown leaf f4c01f618 is merged without its author's
moving branch or any host/field/App/node additions. The reply's single existing
Markdown call now uses chat_markdown_with, preserving find marks, structural id,
revision/visible-length version and global selection. MarkdownCodeAware copy
preserves source markup and raw single-code selections through the SAME engine.
Actions remains None until actual scoped host helpers/root authorization/image
acquisition are checked; no guessed route, inferred-link authority or lightbox
is invented here. Owner f4 standalone Kit strict is0; actual195 Kit tests belong
to its earlier40aa source and remain prior-source evidence, not current A7 proof.

Exacte879 App/Kit all-target run
wp-timeline-state-tests-20261115T161517Z-e879b506 completed16:40 UTC with
App114/0/2 and Kit208/0/0 (322/0/2 overall). All three pacing regressions and
the actual proposed-body Ready/Loading facade draw passed. This is execution of
the frame-wake correction, not a latency/capture or current whole-source proof.
The owner additionally reports actual Kit212/0/0 atd068 with all17 f4 Markdown
Rust blobs byte-identical; that supports the shared helper only. Its A7 call,
scoped host/root/image authority and final UI acceptance remain separate.

### Spoken user-message presentation

[Decision Log — 수정 사용]
- Purpose: match `app/src/agent-stream/presentation.ts:57-72` and its tests at
  `presentation.test.ts:505-568`, read in full. Complete current/legacy voice
  envelopes show only spoken words; unrelated XML and quoted/incomplete examples
  remain literal. Assistant messages are never unwrapped.
- Existing assets and constraints: user bubbles already receive canonical Prompt
  text; the shared searchable extractor supplies local finder offsets, and its
  existing is_space exactly implements the reference's ECMAScript whitespace.
- Alternatives: rewriting Entry text would discard source content needed by
  replay/fork/plugins. A second transcript/cache or a new regex dependency adds
  ownership and cost to a small anchored display rule.
- Choice: presented_prompt borrows the displayed slice with standard string
  operations and the existing whitespace predicate. Bubble and local Prompt/Steer
  search use this one helper; the existing trailing copy/rewind text also receives
  this slice, as `message.tsx:533-541` supplies its presented message to both
  controls. Entry text/key/revision/attachments stay intact; rewind's protocol
  boundary and owner state are unchanged. Empty displayed text has no trailing
  copy/rewind row, matching the reference's hasText gate for attachment-only or
  whitespace-only spoken messages.
- Why: find marks index the exact words actually shown, including trimmed Unicode
  boundaries. Closing tags embedded in examples require a valid whole suffix
  before recognition, matching the source's anchored lazy capture.
- Effects: no I/O, new dependency, voice-mode permission or provider parsing.
  This does not claim to repair node global-search hits or bounded metadata
  previews built before a complete body exists; W1 owns those projections and
  will consume the shared helper only after a checked union. Local tests cover
  source identity, assistant examples, hidden instruction terms, find offsets,
  optional legacy suffix, invalid XML and exact whitespace. Execution is owed.

### Mounted pacing source isolation

[Decision Log — 수정 사용]
- Purpose: a local EventSeq/revision is not unique across sessions. Reusing a
  mounted row's keyed reveal state could display an old session's partial length
  on the first render of another session with the same numeric row key.
- Existing assets and constraints: GPUI use_keyed_state retains state only across
  consecutive frames, and its existing with_id scopes keyed children structurally.
  The managed Area already has Shown route/session identity; readonly projections
  always display Complete and need no reveal state.
- Choice: pass one borrowed Presentation (phase plus existing Shown) from the
  actual Area into both row renderers. Scope the same keyed helper under distinct
  route and SessionId components; do not concatenate delimiters, hash identity,
  allocate a session map/counter or rewrite canonical entries. Readonly bodies
  return their full byte length before allocating mounted reveal state.
- Why: a source switch drops the previous mounted state and its weak frame
  callback cannot wake the new row. A GPUI regression switches between equal row
  numbers/revisions while the old source has backlog and one callback outstanding.
- Effects: display identity grants no input/network authority. W12 still owns
  admission, authenticated node/resident/claim, lazy visible rendering and transport
  retirement. Shown has no authenticated NodeId: alias retarget with a reused
  SessionId is not fenced by this legacy display key. The actual resident body
  must use the canonical ResidentToken (registry/address/epoch) structurally,
  never a guess from the latest Fleet. This patch is not that live cutover. Its new source-switch
  regression is not covered by the earlier e879322-pass run; execution is owed.
