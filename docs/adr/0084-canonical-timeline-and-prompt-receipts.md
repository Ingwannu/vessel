# ADR 0084: Canonical timeline and identified prompt delivery

Status: accepted, 2026-11-15 (contract review). Value types are staged;
provider, session, store and call activation remain unimplemented.
Protocol and store schema numbers are assigned at integration. This decision
keeps `PROTOCOL_VERSION` unchanged. The companion plan is
`docs/plans/wp-timeline-node.md`.

## Purpose and intent

Preserve the information a person needs to read an agent's work, approve its
actual proposed action, reconcile a submitted message, and answer a question
that outlives the turn which asked it. All clients use the same typed
contract. A renderer must not rediscover provider semantics from tool names
or inspect opaque tool arguments.

Paseo 0.15.4 defines the eleven tool detail kinds, canceled status and
inter-agent provenance in `packages/protocol/src/agent-types.ts:215-364`,
canonical items at 367-409, and permission kinds/actions at 447-487. Its
receipt state machine is `packages/server/src/server/message-receipts/index.ts:8-76`.
Its Codex async question state machine is
`packages/server/src/server/agent/providers/codex/async-questions.ts:8-179`.
The plan records further reference paths and what is still survey-only.

## Existing implementation and constraints

`vessel-proto/src/agent.rs` already owns `AgentEvent`, tool calls, updates,
permission option identities and selected outcomes. `conversation/question.rs`
already owns typed fields, typed answers and validation. `vessel-agent` maps
ACP and direct harness formats into those values. `vessel-session` sequences
and projects events; `vessel-store` persists them; `vessel-rpc` adapts replies
and events for the negotiated peer. Extend these owners, without a new crate
or dependency edge.

Current tool calls have categories and opaque arguments, but no canonical
detail. Permission requests have only a tool id and choices. Questions hold
live provider responders and disappear at turn end. `session.prompt` has no
client message identity. Adding an ignored optional id there would let an
old node accept a retry twice, so identified delivery needs a distinct call.

## Alternatives considered

1. Parse provider names and arbitrary JSON in each client: duplicated mappings,
   inconsistent authorization labels, and no exhaustiveness when a kind changes.
2. Replace the timeline with Paseo's exact JSON types: breaks existing peers,
   loses Vessel's durable sequence cursor, and imports untyped metadata.
3. Extend the existing events and projections, with a separately gated identified
   prompt call and durable nonblocking records: chosen.
4. A separate browser-only receipt cache or async question map: cannot survive
   node restart or coordinate two clients and cannot close the uncertainty window.

## Chosen contract

### Canonical items and tools

Add `AgentEvent::Notification(TimelineNotification)`, `Compaction(Compaction)`
and `Plugin(PluginTimelineItem)`, and a distinct `Error { message }` for a
canonical timeline error which does not itself terminate the turn.
`NotificationLevel` is Info/Warning/Error. `CompactionStatus` is
Loading/Completed; optional `CompactionTrigger` is Auto/Manual and optional
`pre_tokens` reuses `TokenCount`. Compaction updates the current compaction
row within its turn; the event cursor remains its transport identity.

`PluginTimelineItem` has typed item id, plugin id, contribution kind, positive
version and `PluginTimelineData`. The data wraps `RawJson`, validates its
compact UTF-8 serialization against the single named 64 KiB bound, and offers
no tree access. The plugin runtime owns interpreting that extension payload;
core code never executes it or grants permissions through it. Identifiers
are nonempty, control-free bounded newtypes with one named bound each.

Add optional `detail: ToolCallDetail`, `failure: ToolFailure` and
`agent_message: AgentMessageProvenance` to tool calls and updates. An absent
update field retains the previous value. Initial optional fields remain absent
on older records. `ToolCallDetail` has exactly Shell, Read, Edit, Write, Search,
Fetch, WorktreeSetup, SubAgent, PlainText, Plan and Unknown. Each variant holds
a dedicated struct rather than mixing unrelated optional fields. Shell keeps
command/cwd/output/exit code; Read keeps path/content/offset/limit; Edit keeps
path/old/new/unified diff; Write keeps path/content; Search keeps query, source,
content, file paths, web results, annotations, counts, timing, truncation and
mode; Fetch keeps URL/prompt/result/status/bytes/timing; WorktreeSetup keeps
path/branch/log/ordered command results; SubAgent keeps type/description/child
session/log/ordered actions; PlainText keeps label/text/typed icon; Plan keeps
Markdown. Unknown keeps optional opaque input/output. Paths, URLs, names,
counts, indices, byte lengths, timings and exit codes have typed owners.
Existing equivalents are reused; external ids are not assumed to be node ids.

`ToolCallStatus::Canceled` is separate from Failed. `ToolFailure` holds display
text rather than untyped error objects. Provenance follows `protocol/src/agent-message.ts:4-14`: typed event
Message/Finished/Errored/PermissionRequired/Closed, sender identity, optional
remote identity/title and message text. It never grants that sender authority. ACP/provider decoders own all field
inspection. Unknown tool inputs stay opaque; missing detail is not fabricated.

### Permissions and question presentation

Add optional `presentation: PermissionPresentation` to `PermissionRequest`.
It has kind Tool/Plan/Question/Mode/Other, optional title and description,
optional canonical detail, and typed plan Markdown. Keep request/tool/option
ids and option order. Add optional `action: PermissionAction` to each
`PermissionOption`: Allow/Deny behavior, Primary/Secondary/Danger appearance,
and optional Implement/ImplementResume/Dismiss intent. The existing option id
is the action id; the answer must select an actually offered option.

A plan approval never means an invented mode switch. Only a provider mapping
that supports that exact option/action may execute it; otherwise refuse before
mutating state. Unrecognized metadata remains opaque passthrough only if no
behavior depends on it. Parsed plan text belongs in the typed presentation.

Add `PermissionResponse` for rich answers, preserving selected option id,
optional modified input, denial message and interrupt intent. Modified input
is `RawJson` only until the owning provider's decoding edge validates its known
schema. Unsupported edits or permission-policy updates are typed refusals,
never ignored and never reported as accepted. The ordinary option-only path
remains valid. A new gated `session.respond_permission` call returns only after
the provider accepted and the corresponding resolution was durably published;
a stale request, concurrent answer, invalid edit and uncertain acknowledgement
are distinct typed outcomes. Read/Operate checks and session ownership remain
those of existing conversation/answer calls, including forwarded scope
intersection. No new privilege is conferred by descriptive metadata.

Extend questions with optional typed presentation: per-field choice preview,
placeholder, explicit empty-answer policy, dismiss label, and a reference to a
custom-text companion field. Validate companion identity, type and membership
at construction. A question with options and custom text is not downgraded to
text-only. Existing field-keyed typed answers stay authoritative; the adapter
maps them to provider question ids/headers. Client drafts are not server state.

### Identified prompts and receipts

New gated `session.submit_prompt`: `SubmitPrompt { session, message:
ClientMessageId, content, delivery }`, returning `SubmitPromptReply` with
Accepted(PromptReceipt), KeyConflict or OutcomeUnknown. A receipt preserves the
message id and exact `Delivered` result, including a queued entry id. The
caller creates the id before sending and retains it for retries. The client
refuses unsupported/unassigned calls before sending; it never falls back to
an unkeyed prompt automatically. The original `session.prompt` is unchanged.

The authorized node path validates the entire prompt and session ownership
before making a receipt. All new tables use one migration, newest schema plus one on the branch; the
integrator re-points this migration at merge. Store admission is atomic on
`(session, message)`.
A canonical fingerprint covers validated content, delivery and authenticated
creator (not request/transport ids). A different fingerprint conflicts;
completed returns the stored result; pending returns OutcomeUnknown and
never sends again. Admission commits Pending before provider/queue work, and
commits Accepted only after the normal delivery acknowledgement. Failure after
admission retains Pending: it may have reached the provider. Concurrent same-key
requests cannot race past admission. No expiration can turn a retained id back
into a new send; receipt deletion follows session deletion, not log pruning.

`TurnStarted` and `Steered` gain optional `client_message_id`; queued entries
carry it until their canonical echo. Thread the identity through the existing
actor delivery path rather than matching text or a provider's latest event.
Completed receipts survive restarts. Failure to persist acceptance remains
uncertain; no success is synthesized. Do not log content, fingerprints or
opaque inputs. Storage keeps fingerprints and outcomes in the private node
store using the existing account and permissions.

### Nonblocking questions

Add `QuestionLifetime::Turn` (absent/default) and `Session` to `Question`.
Session-lifetime questions survive normal turn completion and failed turns;
ordinary blocking responders keep current cancellation behavior. Their durable
record contains question, source item identity and resolution, not a live
provider promise. Codex `agentMessage` with `delivery: async` produces a
completed tool item plus an independently pending Session question; repeated
provider items deduplicate by source item identity. Persist the question
before publishing it as actionable. No intrinsic response expiry.

Answers are serialized with normal foreground mutations. Validate all fields
first. If a foreground turn exists, steer against its exact captured turn id;
a changed turn refuses and retains the question. If idle, start the standard
follow-up prompt. Commit resolution only after acceptance; dismissal commits
without sending a prompt. Record an answer admission before dispatch so a crash
or lost acknowledgement cannot resend the answer silently: that record becomes
an explicit uncertain, non-actionable state until authoritative reconciliation.
Two simultaneous answers cannot both dispatch. Session snapshots and inbox
show authoritative pending/uncertain state on reconnect; offline client caches
must not synthesize actionable requests. Resume reconstructs only these durable
questions, never ordinary SDK promises. Provider-history retention removes
orphan identities with explicit resolution records. Explicit deletion removes
session-owned records; moving/reloading rehydrates them rather than cancels them.

### Retained admission budgets and refusal

Retained receipt and question state has one finite owner in the store writer.
The named limits are a deliberate resource floor beyond Paseo's unbounded
receipt map/files and async record map. They bound logical retained state;
SQLite page/WAL overhead remains subject to the existing store lifecycle and
is not claimed to equal the logical byte charge.

The canonical definitions are in
`crates/vessel-proto/src/timeline/admission.rs`: `MAX_SESSION_ADMISSIONS`,
`MAX_NODE_ADMISSIONS`, `MAX_SESSION_ADMISSION_BYTES`,
`MAX_NODE_ADMISSION_BYTES`, `MAX_RECEIPT_STATE_BYTES`,
`MAX_QUESTION_STATE_BYTES`, and `ADMISSION_ROW_CHARGE_BYTES`.
Every receipt and source question identity consumes its session and node
record budgets; each reserves its complete type envelope plus row charge.
Settled and uncertain records still consume their reservation. The definitions
carry the accepted contract values and are the sole source for implementation,
error messages and tests. Limits are
not settings and an untrusted caller cannot raise them. Input decoders refuse
a record which cannot fit its per-record envelope before admission. Receipt
fingerprints are fixed-size; the stored receipt never retains prompt content.
Question bounds include every typed field, option/preview and answer value.
An answer exceeding its reserved envelope is rejected before dispatch.

Store admission first looks up the exact `(session, message)` or
`(session, source-question-id)` key. Existing keys retain their replay/conflict/
uncertain/settled semantics even at capacity and require no new reservation.
For a new key, the writer transaction checks both count budgets and both byte
budgets and reserves the entire type-specific envelope plus row charge before
inserting Pending. This worst-case reservation covers accepted/result growth
and the question's answer admission without a later capacity race after an
external effect. All checks, counters and insert commit together. Independent
clients/sessions cannot overbook the last slot.

An identified prompt that cannot reserve capacity returns typed
`AdmissionCapacity { scope: Session|Node, resource: Records|Bytes }` and does no
provider, queue or notification work. A new provider-origin async question
which cannot reserve is not published as actionable: emit a typed capacity
notice/error through the ordinary event path; do not create an untracked
responder or an answer which a client could falsely complete. Existing duplicate
source items still reconcile to their retained state. Unsupported rich edits
and oversize question answers refuse before any provider effect independently
of capacity.

A reservation never expires and is not released by log pruning, an accepted
reply, a resolved/dismissed question, provider-history retention or an uncertain
outcome. Those transitions retain a compact replay tombstone within the original
reservation. Only committed removal of the owning session (explicit deletion or
the store's authorized whole-session retention) cascades these rows and releases
its accounting in the same transaction. A caller cannot use a deleted session
id to obtain another send; normal session existence/ownership checks precede
admission. A new session has a distinct identity and a distinct idempotency scope.

On restart, derive/verify accounting from the retained reservation rows before
admission is enabled. Missing/inconsistent accounting refuses new admission with
a typed store integrity error; it never resets counters or deletes keys to make
space. Process crashes and failed transactions retain either the complete
reservation/Pending state or no new admission. The existing WAL store is not a
claim of persistence through host power loss.

Tests must saturate every per-session/per-node count/byte boundary, race fresh
ids for the last reservation, retry accepted and pending ids while full,
restart while full and after uncertain answer admission, and perform each
permitted/disallowed cleanup transition. Assert zero provider/queue effects on
capacity refusal, no replay eviction or resend, unchanged outcomes for old keys,
and exact accounting release only with committed whole-session removal.

## Versioning, activation and older peers

Use one `TIMELINE_PARITY: Option<ProtocolVersion> = None` until integration.
Its documentation names the required consumers (native app, web, CLI and
inbox/forwarding). All package fields, events and calls share this gate. Calls
use the shared required-protocol accessor and, after the announced method-table
head, pending rows. No parallel registry. Call registration waits for that head.

Events use the shared `SessionEvent::required_protocol() ->
Option<ProtocolVersion>` shape introduced by the lifecycle prerequisite. None
means unassigned, never a sentinel or the current version. The central event
adaptation must apply even when the enclosing legacy variant is supported. New item variants become an inert `HarnessLog` stand-in
with the same sequence; optional details/identities/presentation are stripped;
Canceled becomes the legacy Failed status. Before TIMELINE_PARITY is assigned,
no product driver emits session-lifetime requests. Older peers must not receive
a falsely actionable version of a nonblocking/uncertain question: its event is
an inert stand-in and it is omitted from actionable snapshot/inbox lists.

Apply the same adapter to direct replies, lists/pages, nested/forwarded replies,
live subscription events and paged log reads. Durable local events retain full
data. Tests exercise `None`, synthetic assigned threshold minus one, threshold
and newer; incoming new calls refuse through client and dispatch with no work.
Old nodes refuse the distinct method rather than silently ignoring an id.

## Why this choice and its effects

One durable timeline and one actor admission path preserve cursor, ordering,
first-answer and existing authorization invariants. Dedicated value modules
avoid growing `agent.rs` beyond its budget. No new dependency is needed; serde,
RawJson, SHA-256, SQLite and the actor runtime already exist in their owners.
No process, network endpoint, credential scope or background agent is added.
The contract and store are platform-neutral; a driver lacking an upstream
feature returns Unsupported on every platform rather than guessing behavior.

Differences from Paseo are explicit: SQLite receipts instead of individual JSON
files, no compatibility with Paseo's disk layout; typed named errors instead
of opaque objects; durable shared queue instead of its device-local human
queue; typed refusals rather than ignoring unknown edits; bounded plugin data
and typed validated identities; finite reserved retained-admission budgets;
public credential-free HTTP(S) detail URLs and positive plugin schema versions
(where Paseo timeline payloads accept arbitrary numbers); no claim to persist ordinary live approvals.
An uncertainty window remains between provider acceptance and durable commit;
the design prevents automatic duplication, not exactly-once execution inside
an external provider. Extending all exhaustive consumers and regenerating
TypeScript is a cost, and is part of this package's gate.

## Validation obligations

Pure value/serde tests for every detail, malformed identities, plugin byte
boundary (including multibyte text), option intent and question companions.
Recorded ACP/direct-driver fixtures exercise observed mappings, cancellation,
notifications/compaction and async item recognition. Fake session tests cover
question survival, restart, duplicate receive/answer, stale active-turn refusal,
idle follow-up, failed/uncertain delivery and owner/scope refusal. Store tests
cover concurrent receipt admission, conflict, accepted replay, pending replay,
restart and session cleanup. In-memory RPC tests prove old-peer adaptation and
no dispatch for unassigned calls. One complete root gate on the final package,
with TypeScript regeneration and web consumers; no real agent is run. Native
rendering and screenshots are separate consumer evidence, not node gate proof.

### Tool attribution when native ids repeat

`ToolCall.scope` and `ToolCallUpdate.scope` pair `ToolCallId` with a typed
`ToolCallScope`: `ProviderTurn { thread: ExternalThreadId, turn: ExternalTurnId }` when the native frame
supplies its conversation and turn (a new conversation may reuse a turn id), `SessionTurn { started: EventSeq }` for an announcement the
actor observes inside a canonical turn, and `Record { seq: EventSeq }` for an
unassociated observation. `PermissionRequest.tool_scope` identifies the exact
snapshot proposed for approval. The inbox cache keys by session, id and scope.
A late update of an earlier turn cannot change a newer approval's details.

The actor retains unambiguous id associations without eviction, at most 1024
entries and 1 MiB charged by actual id UTF-8 bytes plus a conservative scope/map
reservation. Reusing an id in different scopes permanently marks it ambiguous
for this actor. Absent, ambiguous or over-budget associations yield an isolated
`Record` update, never an inference from the current turn. Explicit provider
scope remains usable at capacity. This finite display-state bound is a deliberate
difference: preserving an update without attaching it is preferable to presenting
it as evidence about different work. Old peers receive a readable log stand-in
for such unassociated updates. Other scoped announcements, updates and permission
references are encoded into one collision-free legacy tool identity before scope
fields are removed. The codec tags the scope kind and uses delimiter-safe base64
components for provider conversation, turn and raw id (or the numbered local
scope). Unscoped raw ids beginning with the reserved namespace are themselves
escaped in a separate tagged branch, so an arbitrary provider spelling cannot
collide with a scoped identity. All direct/page/nested/live/inbox adapters use
this same outgoing-copy codec; the complete durable identity is never rewritten.
No raw-id legacy fold can apply a late A update to the B announcement. These observations never select a permission option or authorize work.

### Provider-owned plan decisions (W22 / W23 boundary)

A non-wire `AgentOutput::PlanProposal` is the sole producer seam for synthetic
plan approval. Its bounded `PlanText` (64 KiB normalized UTF-8) and exact
`PlanProposalKey { thread, turn, source }` come from W22's actual successful root
turn. `PlanProposalSource::CompletedItem { item }` preserves a validated native
item id; `LegacyTurnPlan` explicitly has no native item id. Legacy fallback is
permitted only while the provider reports plan collaboration mode. No title,
Markdown heuristic, foreign/failed/interrupted turn or fabricated item id creates
an approval. W23 assigns the human-facing `PermissionId` and maps only its exact
owned offered option ids to `PlanDecision`.

W22 owns collaboration mode, the actual implement/resume transition and provider
implementation prompt. `DecidePlan` echoes the complete proposal key and an
explicit `Dismiss`, `Implement` or `ImplementResume`. Resume is offered only from
`PlanResumeAvailability::Available`. `PlanDecisionReply::Accepted` echoes both key
and decision only after native acceptance. Typed pre-effect refusal preserves the
pending state; ambiguous external effects return `OutcomeUnknown` and cannot be
resent under that proposal. Only a matching accepted result permits W23 to emit
and durably acknowledge permission resolution. Plan requests outlive their source
turn, while loss of their live provider owner cannot leave an actionable card.
This source contract does not claim that the driver or actor implementation has
passed; their scoped tests and combined V3 review remain required.

A queued prompt's `Accepted` receipt also waits for the actor's already-published
queue change to reach the durable watermark before acceptance is stored. The
receipt writer and event persist task are independent: an actor acknowledgement
alone cannot prove that a queue entry survives a crash. A persistence failure or
bounded wait expiry leaves the committed Pending receipt uncertain, with its
reservation and replay protection intact. It cannot authorize another send.

Durable async question source identity likewise includes the actual provider
conversation and turn (`Question.tool_scope`) together with its native tool/item
id. The store's typed source key is shared by lookup, claim and update; a reused
raw item id in another turn cannot replay an earlier question's settlement.
Native async offers lacking a stable actual turn origin refuse before becoming
actionable. The legacy unscoped source path remains available to existing typed
store callers, with the same retained-state budget; no raw source text grants
permission or caller authority. A restart retains the complete typed source key.

The stateless legacy identity codec is not sufficient across repeated fleet
masking. The integrator's approved direction is the connection-derived,
downgrade-only origin described in
[the concrete shared wire proposal](../plans/timeline-reference-provenance.md).
That proposal requires V3 shared-wire review before runtime wiring; current
stateless projection code is not a merge-ready compatibility claim.

## Joint keyed command and steer-fallback amendment (review pending)

The dormant proposal in [keyed-command-and-delivery](../plans/keyed-command-and-delivery.md)
adds actual prompt dispositions and native command acceptance to the same identified
pipeline, without changing legacy Delivery/Delivered or fabricating turns. W23 owns
receipt/actor semantics, W12b pure intent/catalog production, W22 native transport.
The current attached SubmitPrompt/PromptReceipt remain prompt-only until the shared
amendment and producer capability path are reviewed and implemented. No new number,
method or second receipt store is allocated by the dormant values.

### Standalone and synthetic plan permission attachment

A permission's optional `tool_call` denotes only an actual native item. Legacy
turn plans have no item id; assigning one would join unrelated tool display and
approval context. `Some(actual_id)` retains the previous JSON string encoding.
A tool-less mode/plan request uses `None`; `PermissionId` remains its response
identity. This reuses the existing permission channel instead of creating a
second approval authority.

Only `plan_proposal: Some(actual_key)` identifies the synthetic provider proposal
whose successful root turn has already completed. Presentation kind `Plan` alone
does not change ordinary ExitPlanMode lifetime. The provider owns actual proposal
production, exact-key validation and native transitions; the actor owns human
request identity, selected-action mapping and persistence acknowledgement.
`Pending`, `Answering` and `OutcomeUnknown` are separate typed delivery states.
Only a new explicit Pending response can be admitted. Accepted implement/resume
means the native follow-up was accepted. Dismiss never implies a running turn.
Stale/turn-changed refusals withdraw a lost capability; unsupported/resume-unavailable/
invalid-state refusals apply no action and retain the Pending proposal. Possible
external effects remain uncertain and cannot trigger automatic replay.

Before old-peer decoding, a tool-less request, tagged synthetic request or an
admitted/uncertain request becomes an inert log item, is omitted from the
conversation permission snapshot, and becomes Unknown in the inbox. Full-card
budget degradation retains a bounded `PermissionReference` (proposal key and
delivery state) so it cannot bypass that rule as an InSession pointer. Ordinary
legacy-compatible pointers omit this metadata and keep their original wire shape.
SQL snapshot degradation preserves the same metadata. No provider label or
client-supplied field creates a proposal or an action capability.

This checkpoint attaches values, compatibility and constructor adaptations only;
positive producer transport, actor decision lifetime, native follow-up acceptance
and deterministic runtime proof remain pending. It does not enable plan approval.

### Unified command acceptance attachment (staged)

The one keyed SubmitPrompt carries SubmissionDelivery plus optional native_command.
PromptReceipt.accepted distinguishes Prompt{actual disposition} and
Command{closed status, committed echo seq}. CommandAccepted echoes the same
clientMessageId and original content without inventing a turn. NotAdmitted has
no retained key and precedes reservation; Refused protects a committed no-effect
actor result and replays it. Intent and original text both enter the canonical
fingerprint. Actual live input offers are checked before reservation and again
by the serialized actor, never recovered from Conversation.commands. Native
command policy is Start only and does not cancel an existing turn. Every current
provider remains unoffered/Unsupported; provider launch/version/autonomous
lifecycle proof and positive transport remain separate work. The bounded actor
pump drains outputs while awaiting provider acknowledgement. All accepted echoes
and queue state pass the ordinary persistence watermark before receipt settlement;
uncertain sends retain their reserved key. This extends the reviewed receipt
contract but is not an activation or runtime verification claim.

### Synthetic plan owner implementation checkpoint

The session actor now owns synthetic proposal presentation and exact selected
option-to-PlanDecision mapping through its existing bounded control channel.
Ordinary rich responses use that same serialized owner and continue to the
provider's permission input; synthetic responses use the dedicated decide_plan
carrier. Current-run proposal tombstones have MAX_PLAN_KEYS=128 and
MAX_PLAN_KEY_BYTES=262144 (actual validated key bytes plus named overhead), never
evicted on settlement. One current request contains at most MAX_PLAN_TEXT_BYTES
plus a bounded key and three fixed options. A new driver run retires this finite
owner and never reconstructs it from logs; duplicates within a run never renew
permission authority. Capacity failure emits an inert error, not an approval.
This finite run-local owner is a deliberate resource floor beyond Paseo.

The actor requires an observed root turn (including a fast turn whose initial
prompt acknowledgement has not arrived), ignores replay/move proposals, retains
only explicitly tagged requests across successful EndTurn, and withdraws pending
requests before a new actual turn or driver retirement. An Answering proposal
stays inert while its provider decides whether an intervening real turn belongs
to its implementation follow-up. Only an exact matching provider Accepted may
settle that option and increment the real accepted-follow-up counter; Dismiss
never increments it. Mismatch, lost acknowledgement and possible effects remain
OutcomeUnknown and cannot admit another response. Ownership loss cannot resolve
or mutate a newer proposal accidentally. Resume remains producer-advertised.

No-effects plan refusal is carried as PermissionResponseRefusal::PlanDecision;
StaleProposal/TurnChanged instead withdraw the expired capability with Cancelled,
which consumers must not report as an accepted implementation. Other typed
refusals restore Pending for an explicit corrected/different decision. A process
restart restores no plan authority from its historical card. The node's existing
exact-resolution persistence barrier still owns the final settled acknowledgement.
Prepared state, capacity, option mapping, snapshot and degraded inbox lifetime
tests are unexecuted. Positive provider producer/transition fakes remain W22's
proof; current drivers still return Unsupported. This is staged implementation,
not a package/consumer/provider readiness declaration.

### Snapshot coverage and pruned client history

Live conversation reads now wait for the captured actor projection's exclusive
through watermark to reach the ordinary persistence barrier (bounded timeout).
Durable questions are overlaid from their retained current state; they may be
newer than that lower coverage fence and never grant action authority by display.
A client applies authoritative absence only after its stream has reached that
fence, only to earlier observed requests, and never overwrites a newer real
request refresh or a known resolution. Older snapshots and old-node responses
without through cannot retire cards. Covered absence changes availability only;
it does not fabricate a cancellation/answer outcome or a log record.

A snapshot may provide a pending request whose announcement has been pruned.
The web client keeps that as a dock request, without inventing an EventSeq or
historical timeline row. An actual later request/settlement retires that separate
snapshot attachment through the same exact request id. Questions use the same
coverage rules and keep their original fields/drafts while non-pending responses
remain disabled. Reconnect/missing-history reads refresh the bounded projection;
there is no polling loop or client shadow queue. Tests for covered absence,
stale/ahead snapshots and pruned-announcement settlement are prepared, not yet
executed. This is client reconciliation and never substitutes for provider or
node response validation.

The coverage watermark is not a durable delivery revision. Retained question
claims can change without advancing that watermark. Browser snapshot reads use
per-owner monotonically increasing read tickets; an older completion cannot
replace a newer issued read, even with equal coverage. Snapshot reconciliation
may move Pending to Answering/OutcomeUnknown but cannot reopen admitted state
from a Pending snapshot, including a snapshot with a higher unrelated log fence.
An actual ordered request refresh after no-effect refusal can restore Pending;
known settlement/loss retires the card. Local admitted/uncertain response fences
remain independent of ordinary read presence. These are conservative client
rules, not additional response authority or a new wire revision. New regressions
are prepared for equal-watermark updates, stale read completion, pruned request
state and actual canonical reset; execution remains owed.

Replay lookup binds the canonical serialization of the original submitted
content, including attachment references and metadata, before fresh-work
attachment resolution. Accepted/Pending outcomes must still resolve when an old
attachment's metadata is no longer present. Only a fresh identity resolves those
references against trusted store metadata before validation/admission/provider
work; that trusted resolved content is dispatched, never client-supplied file
metadata. Changing submitted reference metadata under an existing key conflicts
rather than being silently normalized into a different request. A seeded retained
Accepted/Pending/conflict regression with unavailable attachment metadata checks
that ordering and that fresh unavailable references cannot dispatch. It is
prepared but unexecuted; this is not external execution exactly-once proof.

CLI permission listing currently has no guaranteed original provider operation
name in retained presentation. PermissionPresentation.title is a request title,
which is semantically distinct from Paseo AgentPermissionRequest.name. A client
uses a safe generic category when no actual retained operation name is supplied;
it must not reconstruct that name from a display label, pruned tool record or
opaque JSON. This display difference does not delay the minimal body vocabulary
and adds no new response authority or RPC.

### Read-only recovery amendment (contract review before implementation)

The coordinator approved this recovery amendment separately from the local web
preflight: the latter still adds no node RPC. SubmitPrompt remains read-or-admit
and is never named or used as read-only observation. There is one additional
ObservePromptReceipt method, staged under the existing TIMELINE_PARITY None, with
a deny policy line until reviewed wiring/number assignment at integration.

The proposed public request is ObservePromptReceipt { request: SubmitPrompt }.
The complete original request is fingerprinted with the authenticated Creator;
clients cannot supply Creator or select an internal receipt namespace. An
ordinary authorized session lookup precedes the same owner's read-only lookup.
Attachment references are not reloaded or normalized for observation, and a
stored/archived session is not restarted. No record, budget reservation, queue
entry, provider work or watcher is created by observation, including when full.

ObservePromptReceiptReply is a closed status union:
- Unobserved { message }: no matching row was found. This is not proof of no
  effects, and never authorizes resend, a new id or legacy fallback.
- OutcomeUnknown { message }: a retained Pending/uncertain key remains protected.
- KeyConflict { message }: the original fingerprint/Creator does not match.
- Refused { message, reason: SubmissionRefusal }: the same retained no-effect
  refusal, unchanged by later capabilities.
- Accepted { receipt: PromptReceipt, binding: PromptReceiptBinding }: the original
  immutable acceptance plus a separately retained authoritative target binding.

PromptReceiptBinding is a closed union: Unobserved; Queued { entry: QueueEntryId };
Turn { started: EventSeq }; Withdrawn { entry: QueueEntryId }; and
Command { echo: EventSeq }. Queued names the exact accepted entry; it is not a
claim that no provider work has happened or permission to resend it. Unobserved
means no exact target is proved. Withdrawn requires an explicit owning-actor
no-send removal, never absence from a later queue snapshot or a dropped ticket.
A command binds its actual canonical echo and never a model turn.

Direct/joined/replacement acceptance obtains its actual start sequence through
the existing actor TurnTicket observation. Fast completion must not erase that
binding. Queued acceptance keeps the exact entry immediately and uses the same
actor ticket when that entry actually starts or joins a turn. No turn is inferred
from counters, identical content, a guessed next event, provider labels or a
prunable history scan. An unrelated later turn cannot replace an earlier binding.
The initial accepted receipt remains immutable even when a queued target's
separate binding later becomes Turn or Withdrawn.

The existing single store writer owns the additional bounded binding metadata.
Canonical publication/persistence and its binding must commit together (or leave
an explicit Unobserved/uncertain target); a crash must not fabricate a bound turn.
Only trusted live actor correlation may advance it. Imported/replayed display
records and client-supplied ids do not create such a correlation. Exact duplicate
binding writes are idempotent; a different target for a settled binding refuses
rather than overwrites. Log pruning cannot erase retained replay/binding state.
Authorized whole-session deletion remains the sole accounting-release path.
The existing receipt reservation must cover the worst-case binding envelope;
startup verifies it rather than dropping old keys to repair accounting.

Implementation owes missing-row no-effects, same-key query while saturated,
original Creator/fingerprint conflict, pruned attachment/history, fast-turn,
queue-overtake/removal, stale callback, restart/fault and old-peer tests. The
observer never turns an uncertain external operation into claimed exactly-once
execution. Current source does not yet implement this amendment. Public/internal
notice separation approved for ADR0033 remains a distinct joint contract: W18
first supplies the exact bounded NoticeKey and captured parent/grant. No internal
namespace field or new TimelineNotification body id is introduced here.

### Joint ADR0033 private notice admission attachment (contract only)

The aligned ADR0033 source is the docs-only amendment through 69a682be1. Its
NoticeKey is { owner: NodeId, intent: FinishIntentId, checkpoint }, where intent
is a node-minted UUID and checkpoint is closed NeedsYou { request: PermissionId
or QuestionId, source: EventSeq }, Terminal { source: EventSeq }, or the singleton
ObservationUnavailable. Owner is the actual local receipt owner; exact target
node/session, source execution and captured parent/grant live in the immutable
intent. Public SubmitPrompt/ObservePromptReceipt cannot select this namespace.
The authenticated internal producer never defaults to Creator::Owner.

The shared retained owner reserves named envelopes before intent storage or its
original dispatch: 512 encoded bytes maximum notice key, 16 KiB encoded captured
payload including its randomized fence, 32 KiB complete notice state, 64 KiB full
canonical original finish-registration request, and 128 KiB complete non-payload
intent state. The 64 KiB request bound is a deliberate internal registration
floor; it is not a bound on ordinary generic SubmitPrompt. Reserve one intent
plus NEEDS_YOU_MAX+1 complete checkpoint envelopes/row charges AND record slots
atomically against the existing session/node count and byte ceilings. Capturing
ObservationUnavailable irrevocably ends observation, so that final slot is for
terminal OR unavailable. Its pending delivery can remain uncertain but cannot
later add another terminal checkpoint. Existing-key lookup works at capacity.
No captured field may outgrow its already reserved worst-case envelope.

Only committed authorized whole-parent-session removal releases the intent,
checkpoint and receipt reservations. Retention qualifies only by doing that same
whole removal. Acknowledgement, ordinary pruning, replacement, grant revocation
and uncertainty retain tombstones/charges. Startup verifies encoded bounds and
derived accounting instead of resetting or forgetting replay protection. These
are logical budgets, not a SQLite/WAL physical-byte or power-loss guarantee.

Private admission returns/replays the exact canonical parent notice EventSeq AND
QueueEntryId for the same key/fingerprint; later actual-turn binding is separate
from that immutable acceptance. KeyConflict never overwrites or queues effects;
Capacity is fresh no-admission; Unobserved/OutcomeUnknown never prove no effect.
Captured payload/fence/source is immutable before enqueue, queue+event+receipt
commit atomically or through an equivalent recoverable transaction in this same
owner, and a failed acknowledgement/removal commit never creates another notice.
No second receipt store, watcher or delivery queue is introduced.

Current parent execution and captured Read/Operate ceiling intersected with the
current grant/parentage are rechecked after awaits at atomic admission. Serialized
intent is not a capability. Only the lifecycle owner's authoritative recovery of
the same committed execution may rebind it; explicit resume, person stop, archive,
detach or a new execution cannot do so. Current scopes/is_live snapshots alone
are insufficient. Replacement/removal uses the exact captured intent id, so a
late old callback cannot retire or mutate its replacement. Lack of that real
shared fence leaves admission unsupported, never an Owner fallback.

Canonical TimelineNotification stays {level,message}, identified by its own
EventRecord.seq. Trusted internal provenance links later queue delivery to that
notice sequence, so consumers do not append an extra ordinary-user bubble while
still counting a real provider turn. Notice enqueue itself invents no turn.

The coordinator assigned the missing common execution-incarnation fence to this
package. Its proposed exact Agents/metadata-lease/actor insertion boundary is in
docs/plans/timeline-execution-fence.md and requires V3 review before shared
registrations. Transition call sites stay with GKS lifecycle and W2 detach. No
private positive admission is enabled by that proposal or the correlation DTOs.

The exact proposed display-only queue/turn/chunk provenance join is recorded in
docs/plans/timeline-notice-provenance.md for shared-wire review before code. It
keeps the existing notice body and sequence identity, excludes public selectors,
and masks internal content for legacy readers without fabricating lifecycle.
This provenance is display/control correlation, never caller authority; any
additive client carrier remains under the same None gate and its exact typed
wire join requires review before activation. No new SessionEvent presentation
union, notification body id, opaque payload inspection or OS/Inbox owner follows.
A pruned snapshot without the original source EventSeq cannot invent NeedsYou;
use the retained exact checkpoint or explicit unavailable/uncertain outcome.

The private fence, durable binding, read-only method and atomic notice join are
implementation obligations, not existing APIs or runtime proof. V3 reviews this
joint contract before code. Actual restart/fault, saturation, supersession,
parent/grant revocation, queued-turn, old-peer/forwarded and renderer regressions
remain required before either package claims durable finish-notification parity.
