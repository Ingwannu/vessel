# Canonical conversation protocol package

Updated: 2026-11-15. Status: original contract accepted; pending runtime/consumer source and shared amendments are in progress. Exact runtime gates, final review and activation remain owed.
ADR: [0084](../adr/0084-canonical-timeline-and-prompt-receipts.md).

## Assets, reuse and alternatives

Decision: modify/reuse. Extend vessel-proto agent/conversation, vessel-agent
ACP/direct decoder edges, vessel-session delivery/projection, vessel-store
conversation persistence, vessel-node authorized service and vessel-client.
No new crate, external API, dependency or architecture edge. Existing typed
ids/answers, RawJson, serde, SHA-256, SQLite and actor sequencing suffice.
A client-only parser/cache duplicates semantics and cannot provide restart
idempotency. Provider-specific arbitrary JSON stays at its decoding edge.

## Reference behavior read

Paseo 0.15.4, paths relative to packages:

- protocol/src/agent-types.ts:215-364: all eleven detail kinds, provenance,
  running/completed/failed/canceled states; :367-409 canonical items;
  :447-487 permission kinds, actions and response payloads.
- server/src/server/message-receipts/index.ts:8-76: canonical sorted-key hash,
  per-agent/message receipt serialization, pending-before-send, completed replay,
  conflict and uncertain refusal. Our own store does not read Paseo receipt files.
- server/src/server/agent/providers/codex/async-questions.ts:8-179: async item
  schema, stable identity, pending independent of completed tool, response
  preparation/commit, serialization and history-retention cleanup.
- server/src/server/agent/agent-timeline-content.ts:4-28: plugin payload limit is
  serialized UTF-8 bytes; the wider timeline truncation policy is separate.
- Surveys read: parts/C-timeline.md, parts/D-messaging.md, gaps.md first sections;
  CATALOGUE.md and CONNECTIONS.md links inspected for cross-area ownership.

- protocol/src/agent-message.ts:4-14: provenance event/sender/text, no separate
  message id in this structure.
- server/src/server/session.ts:1360-1370 and :5181-5220: capability filtering
  and direct modern permission acknowledgements.
- server/src/server/agent/providers/codex-app-server-agent.ts:4797-4823:
  async answer dispatch targets exact active foreground turn, otherwise follow-up.
- app/src/components/question-form-card-core.ts:30-177: custom/option pairing,
  preview, allow-empty and dismiss behavior.

These pointers were opened after the initial contract draft. The local driver
schema is pinned to Codex 0.166.0; unsupported incoming kinds must not be invented.

## Current ownership and planned modules

- proto: timeline/{item,detail,permission,identity,compat}, conversation receipts
  and question presentation/lifetime; existing event and RPC adapters delegate.
- agent: typed ACP/provider decoding modules; common external detail normalization;
  Codex async questions independent of the blocking approval responder collection.
- session: identity through delivery tickets/queue, durable question projection
  and serialized answer routing; never parse provider JSON here.
- store: schema migration allocated by integrator; atomic receipts and durable
  question records. Keep data under existing private store ownership/modes.
- node: authorized service plumbing only after the shared method-table head;
  no bypass of ownership, scope intersection or existing input validation.
- client: identified prompt and rich permission response calls with capability
  refusal. Consumers adopt these types after contract review.

## Differences and activation

The ADR lists every deliberate difference and bidirectional old-peer behavior.
The single TIMELINE_PARITY constant remains None; no provisional protocol
number. One migration holds all tables, newest schema plus one locally and
re-pointed by integration. New events use the shared optional required-protocol
shape from the lifecycle prerequisite. Whole package root gate only at completion; narrow per-crate checks
while implementing. Schema number and shared method-table head are dependencies.
Node fake fixtures only; no real agent, preview, production config or service.
No native UI files are owned by this package.

## Resume point

Done: fresh worktree from the announced node head, source/survey inventory,
existing assets checked, ADR and contract written.
Done: contract revision passed after retained-state capacity finding.
ADR now fixes finite per-session/node count+byte reservations, capacity refusal
before effects, replay-at-capacity and session-removal-only accounting release.
Standalone timeline value modules now define canonical tool/item/provenance,
permission presentation, question lifetime/presentation, admission budgets and
identified prompt request/reply. Existing events/calls/DTOs are unchanged.
 Integration ruling: one optional protocol constant;
one newest-plus-one migration, re-pointed at merge; shared event accessor.
Quality B head is still awaited.
Next: narrow value checks; share exact types/hash with consumers; event/field
adapters, driver fixtures, persistence/state machines, typed client/service
wiring once the shared head is announced; regenerate TS; one complete root gate.
Checks: first narrow clippy found two documentation-markup errors, corrected.
Narrow clippy/tests pending on current skeleton. Runtime, complete gate and UI
behavior remain unverified.

## Staged scalar choices

Typed detail URLs reuse HttpUrl, rejecting credential-bearing/non-HTTP(S) targets;
a provider lacking a valid target retains the opaque Unknown body. Plugin schema
versions are positive typed integers, a deliberate resource/typing floor beyond
Paseo's permissive number payload. Search durations use the existing DurationMs
unit; the provider decoder will normalize seconds rather than expose two units.
Compaction retains Paseo's Loading/Completed contract; ACP failed/canceled
compaction needs an explicit failure-notice mapping before advertising support.

## Retained receipt service staging

`vessel-store/src/timeline_admission` now uses the existing single writer for
receipt admission and settlement. Its SQLite operations enforce
writer-transaction admission, fingerprint canonicalization, exact-key replay
before capacity, fixed envelope reservation and accepted-result CAS. SQL
triggers own per-session/node accounting; startup checks it against retained
rows and encoded envelopes. One newest-plus-one migration installs all package
tables; the integrator re-points its index at merge. Only receipt storage is
connected to the public Store API; node dispatch and question claims remain staged.
Tests are written for replay/conflict, authenticated creator and nested JSON
canonicalization, session byte saturation, each count/byte predicate, retained
state after log pruning, committed session deletion, restart/corruption and
concurrent same-key/final-slot admissions. They have not yet run. Durable
question and node/provider dispatch work remain unimplemented.

The staged question store now preserves provider-source deduplication separately
from node-minted request ids, validates typed answers before a durable claim,
retains the exact foreground target, and only settles a claim after an explicit
accepted result. A known-not-sent refusal can restore Pending; restart turns
unacknowledged admitted answers into OutcomeUnknown. Tests are written for
deduplication, resolution/tombstones, invalid/concurrent answers, refusal retry
and uncertain recovery, but are not yet executed. This is storage behavior
only: no actor or real provider steering is connected or claimed.

## Quality B and value follow-up

Merged the announced Quality B node head under the standard merge policy;
no conflicts. Method since/shape and pending-row registration now come from
the shared table. The shared AgentEvent optional accessor is owned elsewhere
and will be consumed rather than duplicated.

Standalone value clippy passed on 96ed8b0b. Its queued local unit test was
canceled before execution under the updated build-line ruling; rc143 is a
cancellation, not passing or failing test evidence. Narrow checks now move to
the remote runner. The follow-up adds safe caller UUID allocation, typed
admission bytes, a stable task-id value and known-detail-preserving updates.

## Optional field integration (runtime still pending)

Existing tool calls/updates now carry canonical body/failure/provenance and a
Canceled status; approvals carry presentation/action semantics; task entries
carry stable id and active_form; questions carry lifetime/delivery state, field
presentation and choice previews; queued prompts and canonical prompt echoes
carry client_message_id. Legacy production constructors still emit defaults.
New metadata is masked by feature-aware output adapters, including event pages,
forwarded replies, conversation/queue snapshots and inbox/live entry recovery.
The masking composes with the pre-existing attachment downgrade for old peers.

Codex tool/control code was split without behavior changes before adding empty
fields: two previously oversized actor/responder files are now below the file
budget. Existing fixture expectations remain unchanged. Inbox tool updates keep
known canonical bodies when updates supply None/Unknown; oversize bodies keep
the existing explicit InSession degradation. New activation, provider mappings,
node acknowledgements and question lifecycle remain owed.

The optional-field adapter keeps a no-change path for ordinary legacy events
and snapshots rather than cloning tool bodies for each subscriber. Durable
question delivery state is projected from the store's authoritative claim state;
providers cannot assert Answering/OutcomeUnknown on a new offer. These source
follow-ups remain unverified until the queued narrow checks and storage tests run.

## Non-tool event integration

The shared optional AgentEvent requirement accessor is merged with the newer
head's page masking and title-origin rules. Notification(TimelineNotification),
Compaction(Compaction), Plugin(PluginTimelineItem), and Error { message } are
explicit AgentEvent variants. None use a second cursor/projection union. Their
requirement is TIMELINE_PARITY, still None, so outgoing records become readable
HarnessLog stand-ins without changing sequence/timestamp/page position. Opaque
plugin data is never copied to the legacy stand-in. These events do not settle
a turn or a pending human request. Provider emissions remain pending.

Canonical detail bodies and inbox tool snapshots use Box internally to keep
legacy event/reply enums small; serde and generated consumer shapes are
unchanged. Strict clippy found the enum growth, ambiguous default access and
unconsumed optional-key lookup expressions in the earlier integration; this
follow-up corrects those source defects. Combined compilation remains owed.

## Identified delivery wiring

The pending session.submit_prompt method reuses Operate scope, existing session
ownership and forwarded-call admission. The node resolves stored upload
references, reads retained receipts even after a provider stops, validates fresh
content through the existing provider encoders, and only then asks the store
writer for admission. A committed fresh token permits one existing actor
delivery. Failure after reservation remains OutcomeUnknown; there is no unkeyed
fallback. Queue entries retain the same ClientMessageId through start, steer,
interrupt, fallback and restart. Driver command metadata emits the id directly
in TurnStarted/Steered; identical prompt text is never used to correlate it.

Tests are written for public committed-store/restart behavior and same-text
distinct-id echoes through fake ACP/Codex processes. They remain unverified.
Generic question presentation now validates unique field identities and optional
text companions of choice fields, and enforces explicit empty policies while
retaining legacy form semantics. Provider conversion and async lifecycle work
are still owed.

## Rich permission acknowledgement

RespondPermission carries session/permission plus PermissionResponse(outcome,
modified_input?, message?, interrupt Continue/Interrupt). RespondPermissionReply
is Settled(permission,outcome,seq), Vanished, Conflict, Refused(typed reason), or
OutcomeUnknown. The existing actual option id remains authoritative. The node
waits for the matching resolution record and its persistence watermark; reading
next_seq immediately after the driver answer is insufficient.

The current ACP and Codex decision protocols do not carry arbitrary edited
input or denial explanations, and the direct Claude driver exposes no permission
responder. These extra fields and unbound interrupt intent therefore refuse at
the provider boundary before any decision/turn effects. Selecting an actually
offered provider action remains supported. This is a documented provider
capability difference; no rich value is ignored or reported as accepted. ACP
responder/send failures and Codex closed output now return failure instead of
emitting a false accepted resolution. Fake rich-field refusal tests are written;
end-to-end resolution/race/persistence proof remains owed.

The durable question SQL service now uses public commit-acknowledged Store
operations and startup converts abandoned Answering claims to OutcomeUnknown.
Only typed known-not-sent refusals restore Pending; uncertainty/settlement never
release replay reservations. Writer dispatch is split into a retained-admission
module so central dispatch stays within its existing function/file budgets.
The session callback/foreground dispatch bridge is still unconnected, and these
new public storage operations have not yet passed their actual tests.

## Durable question actor bridge (not yet verified)

The node supplies QuestionPersistence and opaque one-effect answer leases to
SessionRuntime. There is no session-to-store dependency and no second mutable
question registry. Provider async agentMessage items decode at the Codex edge;
the actor reserves their stable tool/item source before publishing QuestionAsked.
Retained questions are restored on Ready even when provider history no longer
contains the item. Normal turn/session cleanup and bounded inbox degradation
retain Session lifetime and delivery state. Ordinary blocking request eviction
cannot evict a durable request. Unsupported/uncertain requests are never made
falsely actionable to older peers.

Answer claims capture the actor foreground before admission. A turn target only
steers that same turn; PromptRequired refuses without falling back to a new turn.
An idle target may start one follow-up. Decline needs no provider prompt. Only an
observed acceptance can settle the retained claim and publish its resolution;
known-not-sent restores Pending, while lost/uncertain execution remains protected.
The sibling respond_question method/committed acknowledgement and authoritative
snapshot watermark are still being connected. No checked runtime claim is made.

Claude Task/Agent inputs now share canonical::sub_agent_input, one serde decoder
returning SubAgentDetail for the canonical launch marker and provider child
routing. The native opaque wire representation remains unchanged; child routers
must call this helper rather than inspect or clone another JSON-tree parser.

## Question response and reconciliation seam

The pending session.respond_question call and NodeClient method now take
RespondQuestion { session, question, answer: QuestionAnswer }. Replies are
Settled { question, outcome, seq }, Vanished, Conflict, Answering, OutcomeUnknown,
or Refused { question, reason }. Refusals distinguish InvalidAnswer, TooLarge,
Unsupported, TurnChanged and Unavailable. Blocking responses share the exact
committed-resolution wait; durable responses use the retained actor lease.

Conversation.through is an optional exclusive log watermark, applied with its
state under the publisher's existing lock. Stored snapshots read next_seq with
the projection columns. Node snapshots replace session-lifetime requests with
the retained store's authoritative Pending/Answering/OutcomeUnknown set; a claim
can lead its later timeline event, never permit another dispatch. Older peers
receive neither the watermark nor actionable unassigned async requests. Clients
can reconcile snapshot retirement against this watermark instead of inventing
resolution records from RPC replies or treating a known stale pending card as
authoritative forever.

vessel_client::compose::fill_headers converts the reference CLI's header/text
answer object through canonical field title/name, offered labels and declared
custom companions, then uses Question::check. Ambiguous headers/comma-joined
multi-select values refuse rather than guessing; callers retain the canonical
field-keyed QuestionAnswer route for those cases. No provider JSON decoding or
fabricated prompt message is added to clients. Public-store restart/claim tests
and pure Codex async parser fixtures are written; actual execution remains owed.

Blocking Codex user-input forms now preserve options when isOther/allowOther is
present, instead of replacing the choice with plain text. One validated optional
text companion represents custom input. Preview, placeholder, explicit empty
policy, multi-select and dismiss label are typed at this provider edge. Responses
remove companion keys and encode custom/selected values under the original
provider question id; custom-only answers cannot become a spurious extra provider
field. Secret forms and unrepresentable complete forms still refuse before being
shown. The existing wire-response regression is updated to prove that behavior;
no passing provider/runtime test result is claimed yet.

### Provider mapping and runtime regression slice

Known native Claude tool names now select typed argument decoders at the provider
edge; ACP categories decode only matching argument structures, and native Codex
items map their command, diff and search fields directly. Unrecognized inputs stay
opaque. A finite output-only snapshot enricher retains at most 128 entries / 512
KiB, with 64 KiB per canonical body; it never evicts identity associations or
merges a reused unscoped id across turns. Generic output remains present even
when canonical enrichment cannot fit. No new dependency is needed: existing serde
codecs and dedicated protocol types provide the boundary.

Private fake Codex scenarios now cover asynchronous session-lifetime questions
while active or idle, including repeated native source identity. Node regression
fixtures use the actual store callback and session actor, and check prompt replay,
canonical echo count, durable question resolution and the exclusive conversation
watermark. These tests are added, not yet passing evidence. Plan action capabilities
and opaque provider outputs must continue to be reported according to actual
native support; no text label grants a permission or substitutes a selected id.

### Shared question whitespace

[Decision Log]
- Purpose: question acceptance and native/CLI presentation must agree with Paseo's
  `String.trim()`, including FEFF as blank and NEL (0085) as content.
- Existing asset: `workspace::trim_project_text` already implemented these
  semantics for project inputs. Rust's general `str::trim` has a different set.
- Choice: extract the shared primitive as `vessel_proto::trim_ecmascript`, keep
  the project API as a facade, and use it for generic question validation and
  provider answer formatting. No provider-name parsing or new dependency.
- Why: one explicit TC39 whitespace set prevents native/node/CLI disagreement;
  all specified characters, excluded Unicode characters and interior preservation
  have regression fixtures. This is text normalization only, with no wire change.
- Primary specification: [TC39 TrimString](https://tc39.es/ecma262/multipage/text-processing.html#sec-trimstring)
  and [WhiteSpace / LineTerminator](https://tc39.es/ecma262/multipage/ecmascript-language-lexical-grammar.html#sec-white-space).

### Native observation producers (execution proof owed)

Claude system status/compact_boundary now have dedicated typed schemas and a
Translator-owned outstanding marker; repeated compacting statuses do not open
extra rows. A boundary retains explicit trigger and pre-token metadata. Unlike
Paseo's fallback to auto, absent/unrecognised trigger stays unreported. Typed
native API retries/permission denials become warning notifications; Codex's
thread-owned retry errors become warnings and non-retry errors become Error.
No question/permission transport or plan-mode transition is changed by these hooks.

Known TodoWrite arguments produce Plan snapshots with optional native id and
active_form preserved; invalid/unknown statuses fall back to the ordinary raw tool
row. The 1024-task ceiling is a finite display-state floor beyond Paseo; it drops
only the extra canonical snapshot, preserving the existing original tool event.
ACP/Codex plans have no fabricated stable ids or active forms when unreported.

Provider detail enrichment now refuses to associate unscoped old arguments after
a new turn, even before a reused announcement arrives. Explicit scopes retain
late-update evidence. Added count/byte/body saturation tests prove no eviction or
capacity-induced association. These tests remain unexecuted until remote evidence.
Primary sources: app/src/utils/tool-call-parsers.ts:270-339; Claude agent.ts:4764-4817,
6247-6273; native Codex error notification schema. No real provider was launched.

Codex root contextCompaction observations now have an isolated typed producer,
without adding a second root ThreadItem mapper or touching native RPC transport.
Item started/completed and thread/compacted channels pair completion once; the
actual owned root turn's terminal notification closes outstanding compression
rows. Child thread traffic is excluded. Absent trigger/token metadata stays absent;
W22's future accepted manual RPC may supply an explicit trigger separately.
The producer retains at most128 observations and bounded1024-byte item ids plus
validated turn ids, with no eviction. Saturation refuses new extra display markers
while allowing already opened ones to complete. This is a stated resource floor;
it does not imply provider command admission or an autonomous turn capability.
Prepared fixtures cover both channel orders, child exclusion, owned terminal
closure and saturation; actual execution remains owed. Primary reference:
Codex provider compaction handling:5790-5829,6280-6349,6408-6425 and native
schema ContextCompactionItem (`app-server.json` followed by this driver).


### Web consumer source checkpoint (not activated or tested)

The web fold now retains canonical scoped tool evidence, question presentation and
lifetime, explicit notice/error/compaction/plugin entries and stable task metadata.
Tool identity is a structural tagged tuple so native id text cannot collide with a
scoped map key. Known detail survives absent/opaque updates. Compaction completion
upgrades the earliest outstanding loading row without moving its record position.
Duplicate question state reports update one card; uncertain answers stay disabled.

Semantic tool bodies have one exhaustive renderer; plugin data is a bounded opaque
inspection fallback, never plugin execution. Generic declared custom pairing/order,
preview/placeholder/empty/dismiss semantics precede legacy form compatibility.
Permission outcomes absent from actual options display unknown, not allowed. The
existing display/update object sharing remains, with tool state extracted into a
small module to keep the web300-line budget. No new dependency or frontend queue.

These changes are source only until the VM-generated declarations land and the
web gate runs. New typed response calls, authoritative snapshot-through recovery
and keyed prepared-submission integration still need wiring; no old fire-and-forget
reply is treated as a durable receipt. Pending protocol None prevents activation.

### Trusted plugin observation append (source only)

SessionHandle::append_plugin(PluginTimelineItem) now orders a bounded canonical
observation through the existing structured-session control channel, returning
its real EventSeq. It does not call a provider, wake a parked process or create
a turn. This is actor admission only: the trusted node broker must recheck its
actual target grant and wait for the existing persistence watermark past seq
before acknowledging success. There is no new RPC, plugin runtime or independent
plugin store. W16's append remains unsupported until its broker and this seam are
checked together. A private fake fixture awaits the real store barrier, reads the
exact committed Plugin record and checks that no turn is running; execution owed.

Blocking native Codex question DTOs now keep actual threadId/turnId from the pinned
ToolRequestUserInputParams schema. Their related-tool scope is explicit when both
are reported. Otherwise the ordinary actor association map may retain a unique
known old scope or an unassociated Record, never whichever turn happens to run.
No request/answer authority is inferred from that display scope. This complements
the existing exact native async scope requirement and durable SourceKey storage.

The rich permission/question node reply budget now covers driver dispatch as well
as waiting for the committed resolution. Timeout returns OutcomeUnknown and leaves
the existing in-flight actor/driver ownership intact; it never cancels a possible
provider effect, releases a durable claim or automatically resends. The durable
question stays Answering until a late real settlement, process loss or restart
recovery classifies it. This bounds a connection call slot even if the provider
stays connected without acknowledging, without admitting a second provider command.

The W22 isolated plan carrier116ac4005 is staged as199b3d0e6. Until explicit
synthetic-permission lifetime/decision state and acknowledgements are attached,
the session treats a PlanProposal carrier as unavailable and publishes only a
static error, never an approval card. All driver plan decision arms are Unsupported.
This closes exhaustive source paths without claiming an implemented plan workflow.

### Current strict feedback corrections

The later strict consumer run (wp-timeline-consumer-ts-20261115T145012Z-7fe21e03)
reached proto tests and found two ambiguous Default trait spellings; those fixtures
now name PermissionDeliveryState::Pending explicitly without changing assertions.
The web preflight strict run reached the agent library and found existing staged
carrier semicolons/docs, the internal Command::NativeCommand name suffix, and
unchecked compaction observation indexing/arithmetic. Internal command dispatch
now uses Native (public AgentInput and all wire names stay the same). Compaction
uses checked collection access and explicit saturating finite counters under the
existing 128-observation admission cap; it adds no eviction or guessed attribution.
Other pending-state constructors receive the same explicit enum value. These
are corrections to failed immutable snapshots, not claims those snapshots passed.
Current-source strict/tests and the enclosing package gate remain required.

The TS generator run wp-timeline-ts-next-20261115T145012Z-4e2ea316 compiled the
current proto/agent/session/store/node/client/web libraries, then executed the
three selected TS tests: two passed; declaration generation refused the new
GoalObjective alias before writing fresh wire bytes. GoalObjective is now
classified once as Alias::Content: a user-authored objective is carried text,
not a minted handle or an action capability. This classification becomes reachable
through the unified SubmitPrompt intent; predecessor unregistered command values
could not prove this generator path. No failed artifact bytes were applied.

Strict run wp-timeline-core-next-20261115T152106Z-9b2bb06f next reached integration
fixture helpers: the two durable-question builders are explicitly test-only,
matching their integration target, and the snapshot byte-bound fixture uses
checked division. The existing assertions and boundary values are unchanged;
no product lint allowance is introduced. That immutable run stays failed.

The next TS run wp-timeline-ts-next-20261115T152922Z-d1ab0843 executed proto3/0
and generated the bridge declarations, then its registry completeness check
reported two stale primitive registrations: LimitPercent is an alias of the
already classified FiniteNumber, and internal-only PlanText is not wire-reachable.
Their redundant registry rows are removed; no limits type/field changes. The
complete generated artifact commit, status and SHA256SUMS were verified, and
only the generated proto/bridge patch was applied. Current generation equality
and browser typechecks/tests remain required; the original run stays failed.

The same correction checkpoint preserves dispatch semantics while nesting the
closed provider-result patterns, borrowing classified errors exhaustively,
using let-else before interruption, and extracting the unchanged live event
publication branch out of the actor output dispatcher. No lint waiver or new
provider capability is introduced.

W4's exact-label audit found that the header codec trimmed before matching,
which could select title A when the offered title was actually space-A-space.
Matching now preserves raw offered values/titles before any normalization;
only freeform/custom text is trimmed. Multi-select preserves raw component
edges and refuses representations that would require guessing normalized labels
or delimiter-bearing options. Canonical field-keyed answers remain exact. The
older FEFF-wrapped non-offered label fixture now correctly becomes custom Left,
while FEFF blank refusal and U+0085 custom content remain asserted. Two new
raw-label/value/CSV regressions are authored; prior five-case and e98 strict
passes do not prove this changed codec.

### Announced node 9c150c6e union

Merged the announced gate50c head (all18 steps0; Rust2549/0/12), including the
checked minimal body picks, without rebasing. The canonical node method order,
subagent/workspace registry declarations, schema19/20 and diagnostics/cleanup
changes are retained. W23's three unassigned rows follow the head rows, its
single admission migration is now the one append after schema20, and rollback
fixtures remove that append plus the head's registry/detach appends before
replaying their original named predecessor. The head's ratchet remains the
ceiling; already divided files are removed and smaller measured entries lowered.
The richer timeline module retains its existing helpers/identity/scope/compat
attachments in addition to the checked common body vocabulary.

The generated wire deliberately remains the verified d1ab artifact until this
new union is regenerated on the VM. It is not a hand-merged generated file or
current equality proof. Previous immutable checks remain scoped to their own
heads, and current root compilation/tests/generation are owed. No final package
whole gate was running during this A8 merge. No recovery/notice implementation
has started before joint contract row251's verdict.

### Native asynchronous messages without questions

The pinned native 0.169 schema permits AgentMessage.delivery=async with null or
absent questions, and an empty array still fails Paseo's question ItemSchema.
Such items now keep their ordinary assistant text instead of becoming a form
error that discards the body. Invalid explicit forms stay non-actionable while
safe ordinary text still falls through the normal item mapper. The existing
AgentSetup.questions policy also gates asynchronous actionable forms, as it
already gates blocking user-input/elicitation requests. No version/launch offer
or provider capability is inferred from a delivery label.

Question titles now use the shared ECMAScript trim at this native decoding edge,
matching Paseo async-questions.ts:8-18: FEFF is blank, NEL remains content. Choice
labels/values retain their raw spelling. Pure fixtures cover null/empty forms and
these title cases. Two private fake scenarios and a driver regression cover
ordinary async text, and the existing no-watcher case now also refuses an async
form without losing its text. These authored fixtures are not actual native
transport/version or real-provider proof; exact new execution remains owed.


## Resumed receiving union

The announced node gate57 base is merged, preserving its command-kind producer,
same-connection membership reader, private diagnostics and voice values. The
existing single timeline compatibility traversal keeps command metadata masking
inside its existing event/snapshot branches; no second reference pass is added.
Canonical Some(tool) approval assertions in the Codex process and inbox fixtures
now express the optional actual-tool field without weakening their identity
checks. These fixtures have not yet executed on this receiving union. Generated
wire remains the incoming base bytes until the current combined generator runs;
its formatting is never hand-edited.


The gate58b receiving merge preserves the original early-stop LaunchAdmission
carrier and the canonical runtime factory beside node setup. Its optional
QuestionPersistence attachment retains the timeline owner's existing durable
question bridge; the base factory regression passes None and keeps its original
stop-token assertions. The duplicate older factory is removed. Attachment
bootstrap moves unchanged beside the runtime factory to keep node assembly
within its shrinking budget (636, formerly651); no channel limit, loader/sweep
ordering or caller changes beyond qualification. No process-lifetime observation
is promoted into an execution/grant fence by this join. Exact receiving checks
remain owed; source movement and predecessor green are not merged-head proof.

The capture assertion probe retains its one actual handle in a single-assignment
cell so it remains observable after unwinding. The retained fixture still drains
the actual runtime and checks the original typed panic and ended handle; this
fixture correction changes no capture or effect-admission semantics.
