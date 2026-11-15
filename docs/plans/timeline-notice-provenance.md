# Proposed private notice prompt provenance (row251, no implementation)

The approved notice body remains TimelineNotification {level,message}, identified
by its EventRecord.seq. A separate private receipt retains that same sequence and
the exact queue entry. This proposal supplies the missing typed link on later
queue-to-turn delivery, so an internal notice does not also appear as an ordinary
user message. It creates no new presentation union, method, store or feature.

[Decision Log]
- Purpose: count the real provider turn while rendering the captured notice once.
- Constraint: a queue entry may start much later; its text/title cannot prove
  origin, and client message identities belong to a separate public namespace.
- Alternatives: infer from text or use a public ClientMessageId for the notice.
- Choice: optional typed PromptProvenance::InternalNotice {notice: EventSeq},
  attached by the node's trusted admission owner only.
- Why: canonical notice sequence is already durable presentation identity; an
  explicit source link avoids another body id or fake model-turn acceptance.
- Consequences: this is display/correlation evidence, never admission authority.
  Old peers lose internal content while retaining actual turn/sequence state.

## Shared shape and producer boundary

`PromptProvenance` is a closed snake-case tagged enum with one initial variant:
`InternalNotice { notice: EventSeq }`. Ordinary input is None (omitted), preserving
existing JSON. The optional field is named `provenance` on QueuedPrompt and on
AgentEvent::TurnStarted, Steered and UserMessageChunk. The chunk carries the same
source explicitly, so a late chunk cannot be appended to an earlier user row
merely because the current internal prompt row was suppressed.

Public SubmitPrompt, PromptSession, queue creation/edit requests and first-input
creation requests have NO provenance selector. Unknown input keys cannot supply
the runtime capability or choose the private receipt namespace. Existing ordinary
producer literals use None. Only the same-key private notice owner can associate
its committed canonical notice sequence with the exact entry it admitted under
the current execution/grant fence. Providers never parse this from native input
JSON or a label: the session/driver carries the node's typed prompt context.
Imports/replays do not manufacture a private association.

The original private acceptance (notice sequence +queue id) remains immutable.
When a person explicitly edits a queued notice's content, the queue owner clears
its internal provenance before publishing the edit. Its later prompt is ordinary
edited user input; preserving the old tag would falsely hide that human change.
Send-now/reorder without a content edit preserves the association. These are
typed queue-owner operations, never caller-supplied source strings.

## Projection and compatibility

Supporting consumers always process the actual TurnStarted/Steered lifecycle and
turn count. An internal provenance link suppresses only the ordinary prompt/steer
row and its explicitly tagged user chunks. The canonical Notification remains
one literal-text alert; no inferred next turn, synthetic acceptance, busy counter
or fabricated sequence is added. Ordinary None input keeps existing behavior.

The field is gated by the SAME TIMELINE_PARITY Option, still None. The existing
single compatibility traversal visits direct/live/owned/borrowed, pages,
conversation queues and recursive fleet envelopes. Before an unsupported peer
sees an internal prompt/chunk, clear its content and omit provenance, retaining
the real event discriminator, seq/time/cursor and turn state. Strip the internal
queued payload and tag in its queue snapshot too. The canonical Notification's
existing fixed inert fallback remains. This deliberately may leave a legacy
reader's empty prompt placeholder; it must never display captured internal output
as ordinary user words. No second rewriting pre-pass or version is introduced.

Already downgraded/replayed input has no source and cannot regain it by spelling,
sequence guesses or parsing a prefix. A serialized provenance value can only
affect presentation; it cannot mint NoticeKey admission, Creator, a private fence
or any mutation capability. Actor admission still validates the actual target
and grants independently, and no public method accepts a provenance answer.

The bounded private receipt/intent envelopes already reserve the correlation
sequence; no new retained identity or cleanup path is created. Tests must cover
real queued delivery, late tagged chunks, explicit edit clearing, one alert/no
duplicate ordinary row, lifecycle counts, both owned/borrowed frames and actual
two-hop old/new queue/event projections, with seq/cursors preserved. Source and
runtime review are owed before implementation/activation, including integrator
acceptance of these additive shared fields. Positive private admission remains
Unsupported until all row251 fences/transactions/consumer proof exist.

Integrator shared-wire acceptance is conditional. Before implementation or positive
private records, pin inert request-key tests for SubmitPrompt, PromptSession,
CreateSession/first-input and QueueChange; none may gain a source selector.
Explicitly clear provenance at external record ingress: session and memory import,
fleet-delivered transfers and provider/history replay. A foreign export must not
hide ordinary user words by claiming a local internal source. Reading this node's
own retained canonical records is not a new foreign producer.

The following node-internal readers are OWED before private production: generated
first-input titles (ADR0062), session.search indexing, inbox/push previews,
vessel-mcp tool text, CLI/web transcripts and export snapshots for older importers.
Wire masking alone does not satisfy them. A7 native projection must also preserve
the hidden actual turn boundary for grouping/copy/footer logic when suppressing
its ordinary prompt row. No fabricated Prompt, turn or new presentation union
can replace that boundary. Positive private admission remains Unsupported while
any required reader or ingress join is owed. The one store migration repoints to
22 after lifecycle's listing schema21; numbering stays the integrator's merge order.


## First source implementation checkpoint

The agreed optional field is now on the three canonical user-input events and
QueuedPrompt, with default/omitted ordinary serialization and the one existing
TIMELINE_PARITY gate. Existing producers explicitly supply None. The existing
single compatibility traversal clears both a tagged payload and its field; queue
entries keep identity/order and event records keep real lifecycle/seq/time/cursors.
Encoded Forward/Relay/Deliver fixtures cover an old middle and modern ends, plus
modern canonical preservation and the None staging boundary. Public submit,
creation/first-input and queue change fixtures cannot select the output field.

StructuredActor clears associations from every provider output before replay or
publication handling. Session/memory/fleet import moves native provider files,
not local canonical records; their replay enters this same boundary. Canonical
local retained reads keep their association. A committed user content edit clears
the queue association; rejected/stale edits restore it, and move/send-now retain
it. Search treats tagged input as a run boundary without indexing its words.

This checkpoint does NOT attach a new positive notice producer. Exact changed
compile/tests/TS/review are still owed. Title generation, inbox/push previews,
MCP tool text, CLI/web/native transcript/group/copy/footer and export-snapshot
readers remain owed before positive private production. Their existing readers
are not claimed qualified by outgoing wire masking. The execution/grant capsule
and atomic notice/queue/receipt transaction also remain unavailable; private
admission stays Unsupported. No second body id, method, number or store is added.

The next source checkpoint supplies shared typed reader leaves:
AgentEvent::clear_internal_notice_words and
QueuedPrompt::clear_internal_notice_words. They preserve the modern association
and identity while suppressing duplicate ordinary input words. The existing
compatibility traversal uses these same leaves, then removes legacy metadata;
there is no second recursive privacy walk. MCP's owning consumer may use these
leaves on its existing typed results once the producer is qualified.

CLI streaming ends a hidden input boundary without printing its body or a user
prefix; queue previews retain the actual id/revision with a fixed internal-notice
caption. The automatic first-input title branch refuses tagged input. A native
ACP history fixture includes malicious direct and _meta associations and requires
ordinary words with no local association. New typed-leaf/CLI/queue fixtures are
authored; exact execution and generated-title/inbox/push/export/web/native reader
qualification remain separate obligations before positive private production.

## Proposed trusted input-echo carrier (before shared input edits)

The first field implementation deliberately enables no producer. A remaining
causal boundary must be explicit before attaching the field: queue advancement
currently passes content and ClientMessageId through AgentInput and the driver
emits an ordinary AgentOutput::Event. Unconditionally scrubbing foreign events
would also erase a field copied through that path. Recovering it from the next
turn, content, a native id prefix or an unrelated current turn would be unsafe.

[Decision Log]
- Purpose: preserve a locally admitted queue entry's exact presentation origin
  through its own accepted input echo, without trusting provider/history JSON.
- Existing constraint: AgentOutput::Event intentionally includes foreign native
  updates and replay, which must remain unable to assert a local association.
- Alternatives: a side map guessed from the next event/content; trusting every
  incoming tag; or dropping the link after queue delivery. Each loses causality
  or lets foreign words disappear from the transcript.
- Proposed choice: one non-serialized AgentOutput::InputEcho(InputEcho) carrier.
  InputEcho has private fields, closed Started/Steered constructors inside the
  driver crate and an into_event consumer. It cannot be decoded from JSON or
  hold Ready/control/tool/question/receipt/plugin events. It is presentation
  evidence from the accepted input owner, never queue or permission authority.
- The actor passes the already-admitted queue entry's origin through a dedicated
  internal AgentInput method. Legacy/public prompt APIs delegate with None; public
  node Submit/Prompt/Create/QueueChange requests gain no origin selector. Their
  ClientMessageId does not become an internal notice key.
- Native/ACP prompt and steer implementations construct InputEcho only from that
  exact typed input argument after actual acceptance. Foreign wire decoders and
  history replay keep using Event and cannot construct InputEcho from metadata.
  No provider RPC/content schema change or version inference is needed.
- StructuredActor scrubs Event as today; InputEcho is consumed as its exact
  canonical Started/Steered event without that foreign scrub. The queue's normal
  start/send-now path must carry the original typed origin, while a committed
  user content edit already clears it. Canonical local retained reads preserve
  the link; UserMessageChunk remains foreign/untagged unless a separately
  qualified local retained producer exists.
- Failure before acceptance produces no input echo; possible-send uncertainty
  stays uncertain and cannot produce a fabricated turn or authorize retry.

This is a proposal awaiting the shared input-owner and V3 source-contract check,
not implemented or a reason to enable private admission. Planned ownership is
W23's common input carrier/actor propagation, preserving W22's plan decision,
native-command transport and actual provider lifecycle ownership. Tests must
exercise accepted queued input through real private fake drivers, malicious
foreign tags, failed/uncertain acceptance, direct/steered input and legacy None
APIs. The existing single outgoing compatibility traversal remains the only wire
mask; the same TIMELINE_PARITY=None still controls the output fields. No new
number, method, receipt namespace or authority registry is proposed.


Exact e2c9 provenance proto strict stopped on two newly authored fixture lints:
a helper always returned Some, and a small fixture sequence used unchecked
addition. The helper now returns the closed value and its callers wrap the
optional field explicitly; fixture numbering uses checked_add. No assertion,
product bound or wire shape changes. The failed strict remains failed and the
separate e2c9 runtime result does not prove this corrected source.
