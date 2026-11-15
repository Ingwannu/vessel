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
