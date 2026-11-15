# One keyed command and prompt acceptance pipeline

Status: dormant values and joint contract amendment; not attached to SubmitPrompt,
not an implemented actor/driver path. Coordinator assigned W23 the shared receipt
and atomic policy, W12b pure intents/catalog/composer producer, W22 actual provider
transport. All three agree this shape; V3 shared review and narrow execution remain
owed. TIMELINE_PARITY stays the one None boundary; no new number or method.

[Decision Log]
- Purpose: /compact and /goal can succeed without a model turn; steer can safely
  fall back to interruption only when the serialized owner knows steering is absent.
- Existing assets: SubmitPrompt ClientMessageId/fingerprint/admission, current
  delivery actor and bounded control channel, W12 native_commands parser/status,
  actual producer capabilities and existing committed watermark.
- Choice: extend that keyed ingress and accepted kind; keep legacy Delivery,
  Delivered and turn tickets unchanged. Client-side read/interrupt and fabricated
  TurnStarted/Steered would lose atomicity or create false busy/optimistic rows.
- Cost: new retained terminal Refused state and explicit actual disposition;
  final actor/provider/fault tests are required before activation.

Planned attachment (not present in the current SubmitPrompt): replace its delivery
with SubmissionDelivery, add optional native_command:NativeCommandIntent, and
replace PromptReceipt.delivered with accepted:SubmissionAcceptedKind. Original
content, exact intent, policy and creator are in the same existing fingerprint.
Exact old keys resolve before new capacity admission, including stable Refused.
No external action after a new capacity refusal. Pending remains uncertain after
possibly sent mutating work; no automatic resend, fresh-id retry or interruption.

A native command requires Start selector, one original text block and no attachment
or appended context. The shared parser must match that exact original text and
intent, and the actual current run must advertise its NativeCommandKind, before
reservation. The provider rechecks its actual version/launch/state before any RPC.
Goals stay unavailable until real launch flags, version and matched root autonomous
turn lifecycle are all implemented and tested. A global manifest/version alone
cannot advertise a current run's native capability.

The planned common AgentInput carrier returns NativeCommandDispatch Accepted
{status}, Refused {reason}, or OutcomeUnknown; W22 owns all actual RPC adapters.
Known refusal before mutation is durable/replayed under the same key. Once a
mutating RPC may have been sent, timeout/malformed/lost response remains Pending
and returns OutcomeUnknown. No raw provider diagnostic or objective in outcomes.

After actual command acceptance the serialized actor emits CommandAccepted
{client_message_id,content,status}. Node waits for that exact echo to commit before
storing Accepted Command{status,echo}. It never calls Desk.started_one, changes a
turn ticket, queues or cancels the existing turn. A later actual native autonomous
turn has its own independently validated lifecycle; it is not inferred from this
acceptance. A crash after native send but before committed acceptance stays unknown.

SteerOrInterrupt examines the actor's held active turn and current steer capability
in the same control order. Idle starts once. Offered steering follows the one
steer attempt; a possibly sent/ambiguous failure does not fall back. Absent steering
chooses cancel-and-wait on the exact held turn, then starts the same message once.
Validation/loading precedes cancel. Actual accepted result distinguishes Started,
Steered, Queued, InterruptedAndStarted{interrupted:EventSeq}. A failure after cancel
is uncertain, never a no-mutation Refused. Queue/cancel waiting remains bounded and
cannot be overtaken by a second client command.

Required deterministic proof: command acceptance with active/idle turn leaves turn
counters unchanged; capability/parser refusal causes no RPC; possibly-sent loss
retains uncertainty across restart; echo persistence precedes acceptance replay;
concurrent same-key call dispatches once; steer available/absent/racing-end outcomes
are exact; invalid attachment refuses before cancel; cancel succeeds/send fails
never claims no mutation. No real agent is used.

Primary references: Paseo server/session.ts:8329-8391 (caller behavior defaults),
agent/agent-manager.test.ts:944 and1501 (unavailable/missing steer replaces once),
agent/provider-registry-wrap.test.ts:229 (ambiguous failure does not interrupt),
providers/codex-app-server-agent.ts:5166-5287 (native commands without a model turn).
Finite durable reservations/refusal tombstones and explicit transport uncertainty
are deliberate Vessel safety floors; external execution is not claimed exactly once.

## Actor preparation checkpoint (unverified, wire unchanged)

SessionHandle::submit_identified now sends one ordered submission through the
existing control channel. Prompt policies reuse send_as/start_loaded/cancel_and_wait;
no second queue, transport or client capability decision exists. The typed fallback
loads/validates before cancel, observes the actual cancellation reason, and reports
InterruptedAndStarted only for a real canceled held turn. Natural completion before
cancel yields Started. Failure after a possible cancel stays OutcomeUnknown. The
queue's existing interruption suppression and legacy tickets are unchanged.

Native intent currently returns explicit Unsupported before any provider operation;
there is no advertised native capability or simulated command success. W22's checked
transport must replace this refusal after the common carrier/metadata review.
The current wire SubmitPrompt/receipt are still prompt-only and do not call this new
entry point until the joint attachment is finished. Private fake tests cover steer
available/absent and unsupported native intent with exact message echoes; execution
and failure/race/saturation proof remain owed.

The common internal provider carrier is now prepared as
AgentInput::native_command(intent) → NativeCommandDispatch::{Accepted{status},
Refused{reason},OutcomeUnknown}. Its Command variant uses the existing bounded
input queue and a correlated reply. All current ACP/Claude/Codex arms explicitly
return Unsupported without an RPC; W22 owns positive transport branches and
current-run capability proof. This source checkpoint does not remove the actor's
unsupported guard or attach a wire request/event. One in-memory queue fixture
checks the separate correlated result; it is not native transport evidence.

NativeCommandOffers is a closed live-input capability set, read through
AgentInput::native_command_offers and SessionHandle's current input route. All
current drivers answer None. Retired/parked/closed input cannot offer anything;
no restored Conversation.commands value is consulted. The node must query this
before fresh admission and the actual dispatch rechecks the current run again.
W12b's optional AgentCommand.native_command is UI metadata only. A small queue
fixture checks live correlation and retirement; execution remains owed.

Attachment checkpoint: SubmitPrompt now directly carries SubmissionDelivery and
optional NativeCommandIntent; PromptReceipt.accepted is SubmissionAcceptedKind.
The fingerprint still hashes the complete request and creator, including exact
original content and native intent. NotAdmitted explicitly denotes preflight
before identity reservation; Refused is a committed, replayable no-effect actor
result after a reservation. These meanings are distinct so later provider
capability changes cannot silently reinterpret a retained refusal.

The node queries the actual live input's offered kinds before reserving native
work. The serialized actor queries that same run again before dispatch, validates
Start plus exact shared text parser, and pumps outputs while waiting. Every
provider still offers None/returns Unsupported. A future positive driver returns
closed status; the actor publishes one CommandAccepted (same clientMessageId,
original content, closed status), records its actual seq, and leaves busy/tickets
alone. The node waits for the ordinary commit watermark before retaining the
acceptance. Possibly sent failures stay Pending/OutcomeUnknown. Native commands
never route through interrupt/start and never fabricate TurnStarted or Steered.

Prepared tests cover command receipt/wire echo distinction, no-offer refusal
without a turn, stable refusal restart replay, and a changed intent conflicting
with a retained identity. No positive native provider transport is claimed.

Web source fold now keeps CommandAccepted as a separate caller-identified row,
leaving turn count/running turn unchanged. Permission delivery-state refreshes
replace the existing exact PermissionId row instead of duplicating it; Answering
and OutcomeUnknown disable action controls. Generated TS and execution of these
web regressions are still owed. Native A7/A8 remain separate consumer owners.

The original native command text also has a named finite envelope:
MAX_NATIVE_COMMAND_ORIGINAL_BYTES = 64KiB objective bound +256 prefix/whitespace
bytes. This bounds the retained canonical echo even when normalization would
remove arbitrarily large outer whitespace. Refusal precedes reservation and
dispatch, with no truncation; it is an explicit resource floor beyond Paseo.
The typed SessionError::Moving emitted by SessionHandle::order before enqueuing
is settled as a durable Moving refusal; generic transport/provider failures
remain uncertain and cannot select an interrupt/retry fallback.
