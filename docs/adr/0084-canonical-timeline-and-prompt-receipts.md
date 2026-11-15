# ADR 0084: Canonical timeline and identified prompt delivery

Status: proposed, 2026-11-15. Contract first; no implementation is claimed.
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
text rather than untyped error objects. Provenance preserves sender identity,
optional remote identity/title and message identity using typed external ids;
it never grants that sender authority. ACP/provider decoders own all field
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
before making a receipt. Store admission is atomic on `(session, message)`.
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

## Versioning, activation and older peers

Use three `Option<ProtocolVersion>` constants, all `None` until integration:
`TIMELINE_DETAILS`, `PROMPT_RECEIPTS`, `DURABLE_QUESTIONS`. Documentation on each
names its required consumers (native app, web, CLI and inbox/forwarding). Calls
use the shared required-protocol accessor and, after the announced method-table
head, pending rows. No parallel registry. Call registration waits for that head.

The central event adaptation must apply even when legacy `since()` would allow
the enclosing event. New item variants become an inert `HarnessLog` stand-in
with the same sequence; optional details/identities/presentation are stripped;
Canceled becomes the legacy Failed status. Before DURABLE_QUESTIONS is assigned,
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
and typed validated identities; no claim to persist ordinary live approvals.
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
