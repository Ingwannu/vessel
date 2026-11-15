# Canonical timeline consumer checkpoint

This is a proto/client-only producer extracted over node44 ede20014. It contains
no agent, actor, store, node, CLI or web activation. `TIMELINE_PARITY` is the sole
pending boundary and remains `None`; the three pending Unary methods refuse
through the existing negotiated preflight before upload or send. No protocol or
schema number is assigned here. Final runtime, TS and whole-package proof are
separate from this checkpoint's narrow evidence.

The shared contract is ADR0084. Modules: timeline owns dedicated tool bodies,
scopes/provenance, permission actions, question lifetime/presentation and receipts;
conversation owns authoritative projection and generic answer validation; event
and rpc compatibility traverse outgoing copies; client identifies submissions and
exposes the typed response calls. Existing serde/RawJson, UUID ids, typed options
and connection preflight are reused. No dependency or second store is introduced.

Consumers use `ClientMessageId::random()` once before preparing content. Current
`SubmitPrompt {session,message,content,delivery}` and
`PromptReceipt {message,delivered}` describe prompt delivery only. The separately
coordinated command/SteerOrInterrupt amendment is **not attached or activated in
this checkpoint**. A caller retains payload/identity after `OutcomeUnknown` and
must not retry under a fresh id. TurnStarted and Steered echo
`client_message_id`; queued prompts retain it. A durable receipt is not a claim
that provider execution is exactly once.

Tool identity is `(ToolCallId, ToolCallScope)`, never session-wide raw id alone.
ProviderTurn includes actual thread+turn, SessionTurn uses the started EventSeq,
and Record is explicitly unassociated. PermissionId is independent action
authority. Known detail survives absent/unknown updates. Notification, Compaction,
Plugin and Error are explicit AgentEvent variants. PlanEntry has optional id and
active_form. The finite plugin body is opaque display data, never executable code.

PermissionRequest.presentation and options.action describe actual offered option
ids. Rich RespondPermission/RespondQuestion replies distinguish committed Settled,
Vanished/Conflict, typed no-effect refusal and uncertain delivery. Conversation's
optional `through` is an exclusive committed watermark; consumers must reconcile
against it, not infer settlement from turn completion. Session-lifetime questions
survive TurnEnded. Field presentation carries custom pairing, preview, empty-text,
placeholder and dismiss semantics. `fill_headers` and `trim_ecmascript` keep
CLI/native answer parsing consistent without provider-specific client parsing.

Compatibility preserves seq/time/cursors. Fresh canonical references are projected
once at an unsupported outgoing edge. Read-only Forwarded.reference_origin is
bound from actual node connection ingress; modern links preserve legacy downgrade,
old links re-derive it after decode. It never enters a request or confers authority.
See timeline-reference-provenance.md. Runtime Links wiring is intentionally absent
from this producer; the main package owns it before any feature activation.

Strict compilation, tests and V3 status must be supplied for this exact immutable
commit before consumer handover. Prepared tests and source review are not green
execution evidence. Native renderers may be developed behind the inactive gate;
actual provider/receipt/question behavior still belongs to the node package.

V2 attachment keeps PermissionRequest.tool_call optional actual identity and adds
plan_proposal plus PermissionDeliveryState. A bounded PermissionReference retains
new approval actionability across inbox full-card degradation. Before old-client
decoding, tool-less/tagged/admitted requests become an inert log, are omitted from
snapshots, and become Unknown in full/pointer inbox entries. Ordinary tool Some
serialization stays byte-identical. Only proto values/projection/tests and docs
are included here; actor/store/provider and new command receipt attachment are
excluded. This source remains staged until exact strict, TS, tests and V3 review.
